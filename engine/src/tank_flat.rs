//! Tank Arena's fixed-size `f32` view ([`Flat`] for [`TankRules`]): the GATE-003 §6
//! layout described in `docs/design/tank-refit.md`, for training tools and bindings.
//!
//! [`Flat::encode_obs`] writes [`OBS_LEN`] = 176 values and [`Flat::decode_action`]
//! reads [`ACTION_LEN`] = 4. Neither allocates, and neither changes the match: the loop
//! never calls them, and the rich [`Observation`](crate::Observation) for Rust policies
//! is unchanged.
//!
//! **Layout** (offsets below; every value is clamped to `[-1, 1]` after scaling, and a
//! NaN, e.g. from a zero-size arena or a zero `max_hp`, becomes 0):
//!
//! | Block | Offset | Floats | Features, in order |
//! |---|---|---|---|
//! | Self | [`SELF`] = 0 | 11 | pos x, y · vel x, y · heading cos, sin · turret cos, sin · hp/max_hp · max_hp · cooldown |
//! | Enemies | [`ENEMIES`] = 11 | 4 × 12 | present · rel x, y · vel x, y · heading cos, sin · turret cos, sin · hp/max_hp · max_hp · los |
//! | Allies | [`ALLIES`] = 59 | 4 × 12 | as enemies |
//! | Projectiles | [`PROJECTILES`] = 107 | 8 × 6 | present · rel x, y · vel x, y · is-enemy |
//! | Walls | [`WALLS`] = 155 | 4 | left, right, bottom, top distances |
//! | Tick | [`TICK`] = 159 | 1 | tick |
//! | Obstacles | [`OBSTACLES`] = 160 | 4 × 4 | min x, min y, max x, max y |
//!
//! **Scaling.** Positions (own and obstacle corners) are `2·p / arena.size − 1` per
//! axis. Relative positions are `rel / arena.size` and wall distances are divided by
//! the arena width (left, right) or height (bottom, top). The tick is divided by
//! `config.max_ticks`. All of these use the match's own `config` (Blitzwing,
//! 2026-10-03), so a custom map stays in range. Speeds, hp and cooldown use fixed
//! constants: [`TANK_VEL_SCALE`], [`MAX_HP_SCALE`], [`COOLDOWN_SCALE`] and
//! [`PROJECTILE_VEL_SCALE`]. hp is `hp / own max_hp`. Flags are 0 or 1. cos and sin come
//! from the engine's heading table ([`angle::dir`]), so the output is the same natively
//! and in wasm.
//!
//! **Order and ties** (the same as [`Rules::observe`](crate::Rules::observe)'s lists, computed without
//! allocating):
//! - Tank slots are nearest first by squared distance, and equal distance goes to the
//!   lower tank id. Only living tanks other than this one are listed, at most 4
//!   enemies and 4 allies.
//! - Shell slots are nearest first, and equal distance keeps the projectile list order.
//!   That is older shells first, then shells fired on the same tick by lower tank id.
//!   At most 8, including this tank's own (is-enemy = 0).
//! - Obstacle slots are nearest first by squared distance from the tank's centre to the
//!   nearest point of the rectangle (0 inside it), and equal distance goes to the lower
//!   index in `Arena::obstacles`. At most 4.
//! - Empty slots are all zeros (present = 0).
//!
//! **Inactive agents:** a destroyed tank's observation is all zeros.
//!
//! **Frames:** relative positions and velocities are in the world frame (x right, y
//! up), not rotated into the hull frame, as in the v1 table.
//!
//! **Actions:** throttle, turn, turret turn, and fire = value `> 0`. Input from a
//! binding is untrusted, so [`Flat::decode_action`] clamps every value to `[-1, 1]` and
//! maps NaN to 0 itself; the loop's [`Rules::sanitize`](crate::Rules::sanitize) still
//! clamps again before recording.
//!
//! **Native vs wasm:** the encoding uses only table trig and basic f32 arithmetic, so it
//! is the same natively and in wasm. Nothing exports it to wasm or Python yet, so no
//! parity check runs. That check lands with the first wasm or Python consumer
//! (game-system.md M4/M5).

use crate::angle;
use crate::generic::Flat;
use crate::policy::{Action, MAX_OBSERVED_PROJECTILES, MAX_OBSERVED_TANKS};
use crate::sim::{MatchConfig, TankRules, TankState};
use glam::Vec2;

/// Length of one tank's encoded observation: 11 + 96 + 48 + 4 + 1 + 16.
pub const OBS_LEN: usize = 176;
/// Length of one tank's encoded action: throttle, turn, turret turn, fire.
pub const ACTION_LEN: usize = 4;

/// Offset of the self block (11 floats).
pub const SELF: usize = 0;
/// Offset of the 4 enemy slots ([`TANK_SLOT`] floats each).
pub const ENEMIES: usize = 11;
/// Offset of the 4 ally slots ([`TANK_SLOT`] floats each).
pub const ALLIES: usize = ENEMIES + TANK_SLOTS * TANK_SLOT;
/// Offset of the 8 projectile slots ([`PROJECTILE_SLOT`] floats each).
pub const PROJECTILES: usize = ALLIES + TANK_SLOTS * TANK_SLOT;
/// Offset of the 4 wall distances.
pub const WALLS: usize = PROJECTILES + PROJECTILE_SLOTS * PROJECTILE_SLOT;
/// Offset of the tick.
pub const TICK: usize = WALLS + 4;
/// Offset of the 4 obstacle slots ([`OBSTACLE_SLOT`] floats each).
pub const OBSTACLES: usize = TICK + 1;

/// Tank slots per block (enemies, allies): [`MAX_OBSERVED_TANKS`].
pub const TANK_SLOTS: usize = MAX_OBSERVED_TANKS;
/// Floats per tank slot.
pub const TANK_SLOT: usize = 12;
/// Projectile slots: [`MAX_OBSERVED_PROJECTILES`].
pub const PROJECTILE_SLOTS: usize = MAX_OBSERVED_PROJECTILES;
/// Floats per projectile slot.
pub const PROJECTILE_SLOT: usize = 6;
/// Obstacle slots.
pub const OBSTACLE_SLOTS: usize = 4;
/// Floats per obstacle slot.
pub const OBSTACLE_SLOT: usize = 4;

/// Tank velocities (units per tick) are divided by this: 150 u/s ÷ 60, Tank Arena's
/// top speed (`tank::loadout::MAX_SPEED[4]`).
///
/// The level tables live in `games/tank`, which depends on `engine`, so the constants
/// are written out here; `games/tank` tests assert them against the tables.
pub const TANK_VEL_SCALE: f32 = 2.5;
/// `max_hp` is divided by this: the highest max HP (`tank::loadout::MAX_HP[4]`).
/// Asserted against the table by `games/tank` tests.
pub const MAX_HP_SCALE: f32 = 940.0;
/// `cooldown` (ticks) is divided by this: the longest reload
/// (`tank::loadout::FIRE_COOLDOWN[0]`). Asserted against the table by `games/tank` tests.
pub const COOLDOWN_SCALE: f32 = 64.0;
/// Projectile velocities (units per tick) are divided by this: 360 u/s ÷ 60, the
/// default `projectile_speed`. Asserted by `games/tank` tests.
pub const PROJECTILE_VEL_SCALE: f32 = 6.0;

const _: () = assert!(OBSTACLES + OBSTACLE_SLOTS * OBSTACLE_SLOT == OBS_LEN);

/// The `N` smallest keys seen so far, in ascending order (insertion into a fixed
/// array, no allocation). Keys must be distinct (each carries an index or id), so the
/// result equals a stable sort followed by a truncate.
struct Nearest<const N: usize> {
    keys: [(f32, usize); N],
    len: usize,
}

impl<const N: usize> Nearest<N> {
    fn new() -> Self {
        Self {
            keys: [(0.0, 0); N],
            len: 0,
        }
    }

    fn offer(&mut self, dist_sq: f32, index: usize) {
        let before =
            |a: (f32, usize), b: (f32, usize)| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).is_lt();
        let key = (dist_sq, index);
        let mut at = self.len;
        while at > 0 && before(key, self.keys[at - 1]) {
            at -= 1;
        }
        if at == N {
            return;
        }
        let end = self.len.min(N - 1);
        self.keys.copy_within(at..end, at + 1);
        self.keys[at] = key;
        self.len = (self.len + 1).min(N);
    }

    fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.keys[..self.len].iter().map(|k| k.1)
    }
}

impl Flat for TankRules {
    const OBS_LEN: usize = OBS_LEN;
    const ACTION_LEN: usize = ACTION_LEN;

    /// Writes tank `agent`'s observation in the layout above, or all zeros if the tank
    /// is destroyed (inactive). Allocation-free.
    ///
    /// # Panics
    /// If `out.len()` is not [`OBS_LEN`] or `agent` is out of range.
    fn encode_obs(
        config: &MatchConfig,
        state: &TankState,
        agent: usize,
        tick: u32,
        out: &mut [f32],
    ) {
        assert_eq!(out.len(), OBS_LEN, "observation buffer length");
        out.fill(0.0);
        let tanks = state.tanks();
        let me = &tanks[agent];
        if !me.alive {
            return;
        }
        let size = config.arena.size;
        let pos = |p: Vec2| 2.0 * p / size - Vec2::ONE;

        // Self.
        let p = pos(me.pos);
        let (h, t) = (angle::dir(me.heading), angle::dir(me.turret));
        let max_hp = state.tank_params(agent).max_hp as f32;
        out[SELF..SELF + 11].copy_from_slice(&[
            p.x,
            p.y,
            me.vel.x / TANK_VEL_SCALE,
            me.vel.y / TANK_VEL_SCALE,
            h.x,
            h.y,
            t.x,
            t.y,
            me.hp as f32 / max_hp,
            max_hp / MAX_HP_SCALE,
            me.cooldown as f32 / COOLDOWN_SCALE,
        ]);

        // Enemies and allies: nearest first, ties to the lower id.
        let mut enemies = Nearest::<TANK_SLOTS>::new();
        let mut allies = Nearest::<TANK_SLOTS>::new();
        for o in tanks.iter().filter(|o| o.alive && o.id != agent) {
            let dist_sq = (o.pos - me.pos).length_squared();
            if o.team == me.team {
                allies.offer(dist_sq, o.id);
            } else {
                enemies.offer(dist_sq, o.id);
            }
        }
        for (base, list) in [(ENEMIES, &enemies), (ALLIES, &allies)] {
            for (slot, id) in list.iter().enumerate() {
                let o = &tanks[id];
                let rel = (o.pos - me.pos) / size;
                let (h, t) = (angle::dir(o.heading), angle::dir(o.turret));
                let max_hp = state.tank_params(id).max_hp as f32;
                let los = config.arena.segment_clear(me.pos, o.pos);
                let at = base + slot * TANK_SLOT;
                out[at..at + TANK_SLOT].copy_from_slice(&[
                    1.0,
                    rel.x,
                    rel.y,
                    o.vel.x / TANK_VEL_SCALE,
                    o.vel.y / TANK_VEL_SCALE,
                    h.x,
                    h.y,
                    t.x,
                    t.y,
                    o.hp as f32 / max_hp,
                    max_hp / MAX_HP_SCALE,
                    los as u8 as f32,
                ]);
            }
        }

        // Projectiles: nearest first, ties keep list order.
        let shells = state.projectiles();
        let mut nearest = Nearest::<PROJECTILE_SLOTS>::new();
        for (i, pr) in shells.iter().enumerate() {
            nearest.offer((pr.pos - me.pos).length_squared(), i);
        }
        for (slot, i) in nearest.iter().enumerate() {
            let pr = &shells[i];
            let rel = (pr.pos - me.pos) / size;
            let at = PROJECTILES + slot * PROJECTILE_SLOT;
            out[at..at + PROJECTILE_SLOT].copy_from_slice(&[
                1.0,
                rel.x,
                rel.y,
                pr.vel.x / PROJECTILE_VEL_SCALE,
                pr.vel.y / PROJECTILE_VEL_SCALE,
                (pr.team != me.team) as u8 as f32,
            ]);
        }

        // Walls and tick.
        out[WALLS..WALLS + 4].copy_from_slice(&[
            me.pos.x / size.x,
            (size.x - me.pos.x) / size.x,
            me.pos.y / size.y,
            (size.y - me.pos.y) / size.y,
        ]);
        out[TICK] = tick as f32 / config.max_ticks as f32;

        // Obstacles: nearest point of each rectangle, ties to the lower index.
        let obstacles = &config.arena.obstacles;
        let mut nearest = Nearest::<OBSTACLE_SLOTS>::new();
        for (i, r) in obstacles.iter().enumerate() {
            let closest = me.pos.max(r.min).min(r.max);
            nearest.offer((me.pos - closest).length_squared(), i);
        }
        for (slot, i) in nearest.iter().enumerate() {
            let (lo, hi) = (pos(obstacles[i].min), pos(obstacles[i].max));
            let at = OBSTACLES + slot * OBSTACLE_SLOT;
            out[at..at + OBSTACLE_SLOT].copy_from_slice(&[lo.x, lo.y, hi.x, hi.y]);
        }

        for v in out.iter_mut() {
            *v = if v.is_nan() { 0.0 } else { v.clamp(-1.0, 1.0) };
        }
    }

    /// `[throttle, turn, turret_turn, fire]`. Each value is clamped to `[-1, 1]` with
    /// NaN mapped to 0, then fire = value `> 0` (so NaN means no fire). The match loop's
    /// [`Rules::sanitize`](crate::Rules::sanitize) clamps again before recording.
    ///
    /// # Panics
    /// If `input.len()` is not [`ACTION_LEN`].
    fn decode_action(input: &[f32]) -> Action {
        assert_eq!(input.len(), ACTION_LEN, "action buffer length");
        let v = |x: f32| if x.is_nan() { 0.0 } else { x.clamp(-1.0, 1.0) };
        Action {
            throttle: v(input[0]),
            turn: v(input[1]),
            turret_turn: v(input[2]),
            fire: v(input[3]) > 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::{Arena, Rect};
    use crate::sim::{Match, TankParams, TankSpawn};
    use crate::testing::{allocations_in, drifter, hunter};
    use crate::Observation;

    fn encode(m: &Match, agent: usize) -> [f32; OBS_LEN] {
        let mut out = [f32::NAN; OBS_LEN];
        m.encode_obs(agent, &mut out);
        out
    }

    #[test]
    fn lengths_and_offsets() {
        assert_eq!((TankRules::OBS_LEN, TankRules::ACTION_LEN), (176, 4));
        assert_eq!(
            [SELF, ENEMIES, ALLIES, PROJECTILES, WALLS, TICK, OBSTACLES],
            [0, 11, 59, 107, 155, 159, 160]
        );
    }

    /// Hunter vs drifter on the duel, stopped mid-match with shells in flight.
    fn mid_match(seed: u64, ticks: u32) -> Match {
        let mut m = Match::new(MatchConfig::duel(), seed);
        let (mut a, mut b) = (hunter(), drifter(seed));
        for _ in 0..ticks {
            m.step_policies(&mut [&mut a, &mut b]);
        }
        m
    }

    #[test]
    fn encode_and_decode_allocate_nothing() {
        let m = mid_match(42, 200);
        assert!(!m.projectiles().is_empty() || m.is_over());
        let mut out = [0.0; OBS_LEN];
        let input = [0.5, -0.25, 1.0, 0.1];
        let (action, count) = allocations_in(|| {
            m.encode_obs(0, &mut out);
            TankRules::encode_obs(m.config(), m.state(), 1, m.tick(), &mut out);
            TankRules::decode_action(&input)
        });
        assert_eq!(count, 0);
        assert!(action.fire);
    }

    #[test]
    fn decode_reads_four_values_and_thresholds_fire() {
        let a = TankRules::decode_action(&[1.0, -1.0, 0.5, 0.0]);
        assert_eq!(
            (a.throttle, a.turn, a.turret_turn, a.fire),
            (1.0, -1.0, 0.5, false)
        );
        assert!(TankRules::decode_action(&[0.0, 0.0, 0.0, 1e-6]).fire);
        assert!(!TankRules::decode_action(&[0.0, 0.0, 0.0, f32::NAN]).fire);
        // Untrusted input is clamped, NaN to 0.
        let a = TankRules::decode_action(&[7.0, f32::NAN, f32::NEG_INFINITY, 3.0]);
        assert_eq!(
            (a.throttle, a.turn, a.turret_turn, a.fire),
            (1.0, 0.0, -1.0, true)
        );
    }

    #[test]
    #[should_panic(expected = "action buffer length")]
    fn decode_checks_the_length() {
        TankRules::decode_action(&[0.0; 3]);
    }

    #[test]
    fn spot_checks_on_a_hand_built_match() {
        // 400 × 200 arena, one obstacle; blue 0 and 2 against red 1, no shells yet.
        let spawn = |team, x, y, heading| TankSpawn {
            team,
            pos: Some(Vec2::new(x, y)),
            heading: Some(heading),
            ..Default::default()
        };
        let tough = TankParams {
            max_hp: 470,
            ..Default::default()
        };
        let mut config = MatchConfig {
            arena: Arena::new(400.0, 200.0)
                .with_obstacle(Rect::new(Vec2::new(180.0, 20.0), Vec2::new(220.0, 60.0))),
            tanks: vec![
                spawn(0, 100.0, 150.0, 0),
                spawn(1, 300.0, 150.0, angle::HALF_TURN),
                spawn(0, 100.0, 50.0, angle::QUARTER_TURN),
            ],
            params: TankParams::default(),
            max_ticks: 1000,
        };
        config.tanks[1].params = Some(tough);
        let m = Match::new(config, 1);
        let o = encode(&m, 0);
        // Self: pos 2·p/size − 1, heading 0 → (1, 0), full hp, max_hp 100/940.
        assert_eq!(&o[SELF..SELF + 4], &[-0.5, 0.5, 0.0, 0.0]);
        assert_eq!(&o[SELF + 4..SELF + 6], &[1.0, 0.0]);
        assert_eq!(o[SELF + 8], 1.0);
        assert_eq!(o[SELF + 9], 100.0 / 940.0);
        assert_eq!(o[SELF + 10], 0.0);
        // Enemy slot 0: tank 1, rel (200, 0) / (400, 200), facing −x, 470/940, clear LOS.
        let e = &o[ENEMIES..ENEMIES + TANK_SLOT];
        assert_eq!(&e[..3], &[1.0, 0.5, 0.0]);
        assert_eq!(&e[5..7], &[-1.0, angle::sin(angle::HALF_TURN)]);
        assert_eq!((e[9], e[10], e[11]), (1.0, 0.5, 1.0));
        assert!(o[ENEMIES + TANK_SLOT..ALLIES].iter().all(|&v| v == 0.0));
        // Ally slot 0: tank 2, rel (0, −100) / (400, 200), clear LOS.
        let a = &o[ALLIES..ALLIES + TANK_SLOT];
        assert_eq!(&a[..3], &[1.0, 0.0, -0.5]);
        assert_eq!(a[11], 1.0);
        // No shells: all 8 slots empty.
        assert!(o[PROJECTILES..WALLS].iter().all(|&v| v == 0.0));
        // Walls ÷ width, width, height, height; tick ÷ max_ticks.
        assert_eq!(&o[WALLS..TICK], &[0.25, 0.75, 0.75, 0.25]);
        assert_eq!(o[TICK], 0.0);
        // Obstacle slot 0 like pos; the other three empty.
        let expect = [-0.1, -0.8, 0.1, -0.4];
        for (got, want) in o[OBSTACLES..OBSTACLES + 4].iter().zip(expect) {
            assert!((got - want).abs() < 1e-6, "{got} vs {want}");
        }
        assert!(o[OBSTACLES + 4..].iter().all(|&v| v == 0.0));
        // Tank 2 sees tank 1 too: the line between them passes above the pillar.
        let o2 = encode(&m, 2);
        assert_eq!((o2[ENEMIES], o2[ENEMIES + 11]), (1.0, 1.0));
    }

    /// The same encoding built from [`Rules::observe`]'s (allocating) lists: the
    /// fixed-array selection must give the same slots, in the same order.
    fn from_observation(config: &MatchConfig, obs: &Observation) -> [f32; OBS_LEN] {
        let mut out = [0.0; OBS_LEN];
        let size = config.arena.size;
        let me = &obs.me;
        let p = 2.0 * me.pos / size - Vec2::ONE;
        let (h, t) = (angle::dir(me.heading), angle::dir(me.turret));
        out[..11].copy_from_slice(&[
            p.x,
            p.y,
            me.vel.x / 2.5,
            me.vel.y / 2.5,
            h.x,
            h.y,
            t.x,
            t.y,
            me.hp as f32 / me.max_hp as f32,
            me.max_hp as f32 / 940.0,
            me.cooldown as f32 / 64.0,
        ]);
        for (base, list) in [(ENEMIES, &obs.enemies), (ALLIES, &obs.allies)] {
            for (slot, o) in list.iter().enumerate() {
                let rel = o.rel / size;
                let (h, t) = (angle::dir(o.heading), angle::dir(o.turret));
                let at = base + slot * 12;
                out[at..at + 12].copy_from_slice(&[
                    1.0,
                    rel.x,
                    rel.y,
                    o.vel.x / 2.5,
                    o.vel.y / 2.5,
                    h.x,
                    h.y,
                    t.x,
                    t.y,
                    o.hp as f32 / o.max_hp as f32,
                    o.max_hp as f32 / 940.0,
                    o.los as u8 as f32,
                ]);
            }
        }
        for (slot, pr) in obs.projectiles.iter().enumerate() {
            let rel = pr.rel / size;
            let at = PROJECTILES + slot * 6;
            out[at..at + 6].copy_from_slice(&[
                1.0,
                rel.x,
                rel.y,
                pr.vel.x / 6.0,
                pr.vel.y / 6.0,
                (pr.owner_team != me.team) as u8 as f32,
            ]);
        }
        let w = &obs.walls;
        out[WALLS..TICK].copy_from_slice(&[
            w.left / size.x,
            w.right / size.x,
            w.bottom / size.y,
            w.top / size.y,
        ]);
        out[TICK] = obs.tick as f32 / config.max_ticks as f32;
        let mut order: Vec<usize> = (0..obs.obstacles.len()).collect();
        let d = |r: &Rect| (me.pos - me.pos.max(r.min).min(r.max)).length_squared();
        order.sort_by(|&a, &b| d(&obs.obstacles[a]).total_cmp(&d(&obs.obstacles[b])));
        for (slot, &i) in order.iter().take(4).enumerate() {
            let r = &obs.obstacles[i];
            let (lo, hi) = (
                2.0 * r.min / size - Vec2::ONE,
                2.0 * r.max / size - Vec2::ONE,
            );
            let at = OBSTACLES + slot * 4;
            out[at..at + 4].copy_from_slice(&[lo.x, lo.y, hi.x, hi.y]);
        }
        out.map(|v| if v.is_nan() { 0.0 } else { v.clamp(-1.0, 1.0) })
    }

    #[test]
    fn matches_the_observation_lists_over_whole_matches() {
        let mut crowded = MatchConfig::duel();
        // 10 tanks on two teams (5 enemies, 4 allies: both lists overflow) and 8 obstacles.
        crowded.tanks = (0..10)
            .map(|i| TankSpawn {
                team: (i % 2) as u8,
                ..Default::default()
            })
            .collect();
        for i in 0..6 {
            let x = 60.0 + 120.0 * i as f32;
            crowded.arena = crowded
                .arena
                .with_obstacle(Rect::new(Vec2::new(x, 40.0), Vec2::new(x + 30.0, 70.0)));
        }
        for (config, seed) in [(MatchConfig::duel(), 3), (crowded, 9)] {
            let n = config.tanks.len();
            let mut m = Match::new(config.clone(), seed);
            let mut hunters: Vec<_> = (0..n).step_by(2).map(|_| hunter()).collect();
            let mut drifters: Vec<_> = (1..n)
                .step_by(2)
                .map(|i| drifter(seed + i as u64))
                .collect();
            let mut checked = 0;
            while !m.is_over() {
                for agent in 0..n {
                    let got = encode(&m, agent);
                    if m.tanks()[agent].alive {
                        assert_eq!(got, from_observation(&config, &m.observe(agent)));
                        checked += 1;
                    } else {
                        assert!(got.iter().all(|&v| v == 0.0), "inactive: all zeros");
                    }
                }
                let (mut h, mut d) = (hunters.iter_mut(), drifters.iter_mut());
                let mut refs: Vec<&mut dyn crate::Policy> = (0..n)
                    .map(|i| -> &mut dyn crate::Policy {
                        if i % 2 == 0 {
                            h.next().unwrap()
                        } else {
                            d.next().unwrap()
                        }
                    })
                    .collect();
                m.step_policies(&mut refs);
            }
            assert!(checked > 200);
        }
    }
}
