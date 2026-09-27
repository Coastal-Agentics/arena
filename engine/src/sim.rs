//! Match lifecycle: config, entities, fixed-step update, end conditions.

use crate::angle::{self, Heading};
use crate::arena::{circles_overlap, segment_circle_entry, Arena, Rect};
use crate::policy::{
    Action, Observation, Policy, ProjectileObs, SelfObs, TankObs, WallObs,
    MAX_OBSERVED_PROJECTILES, MAX_OBSERVED_TANKS,
};
use crate::replay::Replay;
use crate::{DT, TICK_HZ};
use glam::Vec2;
use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

/// Per-tank tunables. Speeds are per second; the sim scales by `DT`.
/// Turn rates are BAU per tick (65536 BAU = one turn).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TankParams {
    pub radius: f32,
    pub max_speed: f32,
    pub turn_rate: u16,
    pub turret_turn_rate: u16,
    pub max_hp: i32,
    /// Ticks between shots.
    pub fire_cooldown: u32,
    pub projectile_speed: f32,
    /// Projectile lifetime in ticks.
    pub projectile_ttl: u32,
    pub projectile_damage: i32,
    /// Max random deviation of a shot, in BAU either side (drawn from the match RNG).
    pub projectile_spread: u16,
}

impl Default for TankParams {
    fn default() -> Self {
        Self {
            radius: 16.0,
            max_speed: 120.0,
            turn_rate: 364,        // ~2 deg/tick = 120 deg/s
            turret_turn_rate: 546, // ~3 deg/tick = 180 deg/s
            max_hp: 100,
            fire_cooldown: 45,
            projectile_speed: 360.0,
            projectile_ttl: 120,
            projectile_damage: 20,
            projectile_spread: 256, // ~1.4 deg
        }
    }
}

/// Where a tank starts. `None` fields are drawn from the match RNG.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TankSpawn {
    pub team: u8,
    #[serde(default)]
    pub pos: Option<Vec2>,
    #[serde(default)]
    pub heading: Option<Heading>,
}

/// Everything needed (with a seed) to reproduce a match.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatchConfig {
    pub arena: Arena,
    pub tanks: Vec<TankSpawn>,
    pub params: TankParams,
    /// Match ends in a draw after this many ticks.
    pub max_ticks: u32,
}

impl MatchConfig {
    /// 1v1 in an 800x600 arena with two obstacles, random spawns, 2-minute limit.
    pub fn duel() -> Self {
        Self {
            arena: Arena::new(800.0, 600.0)
                .with_obstacle(Rect::new(Vec2::new(250.0, 200.0), Vec2::new(300.0, 400.0)))
                .with_obstacle(Rect::new(Vec2::new(500.0, 200.0), Vec2::new(550.0, 400.0))),
            tanks: vec![
                TankSpawn {
                    team: 0,
                    ..Default::default()
                },
                TankSpawn {
                    team: 1,
                    ..Default::default()
                },
            ],
            params: TankParams::default(),
            max_ticks: 120 * TICK_HZ,
        }
    }
}

/// A tank entity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tank {
    pub id: usize,
    pub team: u8,
    pub pos: Vec2,
    /// Velocity in units per tick (after collision; blocked axes are zeroed).
    pub vel: Vec2,
    pub heading: Heading,
    pub turret: Heading,
    pub hp: i32,
    pub cooldown: u32,
    pub alive: bool,
}

/// A generic projectile: travels in a straight line, dies on walls/obstacles/ttl,
/// damages the first enemy tank it overlaps.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Projectile {
    pub owner: usize,
    pub team: u8,
    pub pos: Vec2,
    /// Units per tick.
    pub vel: Vec2,
    pub ttl: u32,
    pub damage: i32,
}

/// Things that happened during the last step, for viewers and rule layers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    Fired {
        tank: usize,
    },
    Hit {
        target: usize,
        owner: usize,
        damage: i32,
    },
    Destroyed {
        tank: usize,
    },
}

/// Why a match ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndReason {
    /// Exactly one team has living tanks.
    LastStanding,
    /// Everyone died on the same tick (draw).
    AllDestroyed,
    /// `max_ticks` reached (draw).
    TickLimit,
}

/// Final result of a match.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    /// Winning team, or `None` for a draw.
    pub winner: Option<u8>,
    pub ticks: u32,
    pub reason: EndReason,
}

/// A running match. Create with [`Match::new`], drive with [`Match::step`] or
/// [`Match::run`], inspect with the accessors. Works identically headless and in wasm.
#[derive(Clone, Debug)]
pub struct Match {
    config: MatchConfig,
    seed: u64,
    rng: ChaCha8Rng,
    tick: u32,
    tanks: Vec<Tank>,
    projectiles: Vec<Projectile>,
    events: Vec<Event>,
    outcome: Option<Outcome>,
    history: Vec<Vec<Action>>,
}

fn rand_unit(rng: &mut ChaCha8Rng) -> f32 {
    (rng.next_u32() >> 8) as f32 * (1.0 / (1u32 << 24) as f32)
}

impl Match {
    /// Start a match: spawns tanks (random spawns come from the seeded RNG).
    pub fn new(config: MatchConfig, seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let r = config.params.radius;
        let mut tanks: Vec<Tank> = Vec::with_capacity(config.tanks.len());
        for (id, spawn) in config.tanks.iter().enumerate() {
            let pos = match spawn.pos {
                Some(p) => p,
                None => {
                    let a = &config.arena;
                    let mut p = a.clamp_circle(a.size * 0.5, r);
                    for _ in 0..1000 {
                        let cand = Vec2::new(
                            r + rand_unit(&mut rng) * (a.size.x - 2.0 * r),
                            r + rand_unit(&mut rng) * (a.size.y - 2.0 * r),
                        );
                        let clear = !a.circle_hits_obstacle(cand, r * 1.5)
                            && tanks
                                .iter()
                                .all(|t| !circles_overlap(t.pos, r * 3.0, cand, r * 3.0));
                        if clear {
                            p = cand;
                            break;
                        }
                    }
                    p
                }
            };
            let heading = spawn
                .heading
                .unwrap_or_else(|| (rng.next_u32() >> 16) as Heading);
            tanks.push(Tank {
                id,
                team: spawn.team,
                pos,
                vel: Vec2::ZERO,
                heading,
                turret: heading,
                hp: config.params.max_hp,
                cooldown: 0,
                alive: true,
            });
        }
        let mut m = Self {
            config,
            seed,
            rng,
            tick: 0,
            tanks,
            projectiles: Vec::new(),
            events: Vec::new(),
            outcome: None,
            history: Vec::new(),
        };
        m.outcome = m.check_end();
        m
    }

    pub fn config(&self) -> &MatchConfig {
        &self.config
    }
    pub fn seed(&self) -> u64 {
        self.seed
    }
    pub fn tick(&self) -> u32 {
        self.tick
    }
    pub fn tanks(&self) -> &[Tank] {
        &self.tanks
    }
    pub fn projectiles(&self) -> &[Projectile] {
        &self.projectiles
    }
    /// Events produced by the most recent step.
    pub fn events(&self) -> &[Event] {
        &self.events
    }
    pub fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }
    pub fn is_over(&self) -> bool {
        self.outcome.is_some()
    }
    /// Actions applied so far, one `Vec` (indexed by tank id) per tick.
    pub fn history(&self) -> &[Vec<Action>] {
        &self.history
    }

    /// Build the observation for tank `id`.
    pub fn observe(&self, id: usize) -> Observation {
        let me = &self.tanks[id];
        let p = &self.config.params;
        let mut enemies = Vec::new();
        let mut allies = Vec::new();
        for t in self.tanks.iter().filter(|t| t.alive && t.id != id) {
            let rel = t.pos - me.pos;
            let o = TankObs {
                id: t.id,
                team: t.team,
                pos: t.pos,
                rel,
                dist_sq: rel.length_squared(),
                vel: t.vel,
                heading: t.heading,
                turret: t.turret,
                hp: t.hp,
            };
            if t.team == me.team {
                allies.push(o);
            } else {
                enemies.push(o);
            }
        }
        let by_dist =
            |a: &TankObs, b: &TankObs| a.dist_sq.total_cmp(&b.dist_sq).then(a.id.cmp(&b.id));
        enemies.sort_by(by_dist);
        allies.sort_by(by_dist);
        enemies.truncate(MAX_OBSERVED_TANKS);
        allies.truncate(MAX_OBSERVED_TANKS);
        let mut projectiles: Vec<ProjectileObs> = self
            .projectiles
            .iter()
            .map(|pr| {
                let rel = pr.pos - me.pos;
                ProjectileObs {
                    pos: pr.pos,
                    rel,
                    dist_sq: rel.length_squared(),
                    vel: pr.vel,
                    owner_team: pr.team,
                }
            })
            .collect();
        projectiles.sort_by(|a, b| a.dist_sq.total_cmp(&b.dist_sq));
        projectiles.truncate(MAX_OBSERVED_PROJECTILES);
        let size = self.config.arena.size;
        Observation {
            tick: self.tick,
            me: SelfObs {
                id,
                team: me.team,
                pos: me.pos,
                vel: me.vel,
                heading: me.heading,
                turret: me.turret,
                hp: me.hp,
                max_hp: p.max_hp,
                cooldown: me.cooldown,
                radius: p.radius,
            },
            enemies,
            allies,
            projectiles,
            walls: WallObs {
                left: me.pos.x,
                right: size.x - me.pos.x,
                bottom: me.pos.y,
                top: size.y - me.pos.y,
            },
            arena_size: size,
            obstacles: self.config.arena.obstacles.clone(),
        }
    }

    /// Advance one fixed 1/60 s tick. `actions[i]` drives tank `i`; missing entries
    /// mean "do nothing"; actions for dead tanks are ignored. Returns the outcome once
    /// the match has ended (further calls are no-ops).
    ///
    /// Step order:
    /// 1. every living tank turns and computes its move against the tick-start
    ///    positions of the others (axis-separated: walls clamp, obstacles and tanks
    ///    block the axis);
    /// 2. moves are applied simultaneously; any tank whose new circle would overlap
    ///    another tank's new circle is held at its old position (repeated until
    ///    stable), so the result does not depend on tank id order;
    /// 3. tanks fire in id order (spread is drawn from the match RNG in that order);
    /// 4. existing projectiles sweep their whole per-tick segment: the earliest
    ///    contact with an enemy tank (at its post-move position) or a wall/obstacle
    ///    wins, ties go to the tank, then to the lower tank id;
    /// 5. new shots are added, deaths applied, end conditions checked.
    pub fn step(&mut self, actions: &[Action]) -> Option<Outcome> {
        if self.outcome.is_some() {
            return self.outcome;
        }
        self.events.clear();
        let n = self.tanks.len();
        let recorded: Vec<Action> = (0..n)
            .map(|i| actions.get(i).copied().unwrap_or_default().clamped())
            .collect();
        let params = self.config.params.clone();
        let r = params.radius;
        let mut spawned = Vec::new();

        // 1. Intents against the tick-start snapshot.
        let old: Vec<Vec2> = self.tanks.iter().map(|t| t.pos).collect();
        let mut moves: Vec<(Vec2, Vec2)> = old.iter().map(|&p| (p, Vec2::ZERO)).collect();
        for (i, &a) in recorded.iter().enumerate() {
            if !self.tanks[i].alive {
                continue;
            }
            let t = &mut self.tanks[i];
            t.heading = t
                .heading
                .wrapping_add_signed((a.turn * params.turn_rate as f32) as i16);
            t.turret = t
                .turret
                .wrapping_add_signed((a.turret_turn * params.turret_turn_rate as f32) as i16);
            let want = angle::dir(t.heading) * (a.throttle * params.max_speed * DT);
            let mut pos = old[i];
            let mut vel = want;
            for axis in 0..2 {
                let mut cand = pos;
                cand[axis] += want[axis];
                cand = self.config.arena.clamp_circle(cand, r);
                let blocked = self.config.arena.circle_hits_obstacle(cand, r)
                    || self
                        .tanks
                        .iter()
                        .any(|o| o.alive && o.id != i && circles_overlap(old[o.id], r, cand, r));
                if blocked {
                    vel[axis] = 0.0;
                } else {
                    vel[axis] = cand[axis] - pos[axis];
                    pos = cand;
                }
            }
            moves[i] = (pos, vel);
        }

        // 2. Apply simultaneously; hold back tanks whose new positions collide.
        // Tick-start positions never overlap, so this converges (at worst everyone holds).
        loop {
            let mut changed = false;
            for i in 0..n {
                if !self.tanks[i].alive || moves[i].0 == old[i] {
                    continue;
                }
                let clash = (0..n).any(|j| {
                    j != i && self.tanks[j].alive && circles_overlap(moves[j].0, r, moves[i].0, r)
                });
                if clash {
                    moves[i] = (old[i], Vec2::ZERO);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        // 3. Fire.
        for (i, &a) in recorded.iter().enumerate() {
            if !self.tanks[i].alive {
                continue;
            }
            let t = &mut self.tanks[i];
            (t.pos, t.vel) = moves[i];
            if t.cooldown > 0 {
                t.cooldown -= 1;
            }
            if a.fire && t.cooldown == 0 {
                let spread = params.projectile_spread as u32;
                let dev = if spread > 0 {
                    (self.rng.next_u32() % (2 * spread + 1)) as i32 - spread as i32
                } else {
                    0
                };
                let h = t.turret.wrapping_add_signed(dev as i16);
                let d = angle::dir(h);
                spawned.push(Projectile {
                    owner: i,
                    team: t.team,
                    pos: t.pos + d * (r + 1.0),
                    vel: d * (params.projectile_speed * DT),
                    ttl: params.projectile_ttl,
                    damage: params.projectile_damage,
                });
                t.cooldown = params.fire_cooldown;
                self.events.push(Event::Fired { tank: i });
            }
        }

        let mut keep = Vec::with_capacity(self.projectiles.len() + spawned.len());
        for mut pr in std::mem::take(&mut self.projectiles) {
            // 4. Swept collision over the whole tick: no tunnelling at any speed.
            let mut hit: Option<(f32, usize)> = None;
            for t in self.tanks.iter().filter(|t| t.alive && t.team != pr.team) {
                if let Some(tt) = segment_circle_entry(pr.pos, pr.vel, t.pos, r) {
                    if hit.is_none_or(|(bt, _)| tt < bt) {
                        hit = Some((tt, t.id));
                    }
                }
            }
            let wall = self.config.arena.segment_blocked_at(pr.pos, pr.vel);
            pr.pos += pr.vel;
            match (hit, wall) {
                (Some((tt, ti)), w) if w.is_none_or(|wt| tt <= wt) => {
                    self.tanks[ti].hp -= pr.damage;
                    self.events.push(Event::Hit {
                        target: ti,
                        owner: pr.owner,
                        damage: pr.damage,
                    });
                    continue;
                }
                (_, Some(_)) => continue,
                _ => {}
            }
            if pr.ttl <= 1 {
                continue;
            }
            pr.ttl -= 1;
            keep.push(pr);
        }
        keep.extend(spawned);
        self.projectiles = keep;

        for t in self.tanks.iter_mut() {
            if t.alive && t.hp <= 0 {
                t.alive = false;
                t.vel = Vec2::ZERO;
                self.events.push(Event::Destroyed { tank: t.id });
            }
        }

        self.history.push(recorded);
        self.tick += 1;
        self.outcome = self.check_end();
        self.outcome
    }

    /// Query each living tank's policy (`policies[i]` drives tank `i`) and step once.
    pub fn step_policies(&mut self, policies: &mut [&mut dyn Policy]) -> Option<Outcome> {
        let actions: Vec<Action> = (0..self.tanks.len())
            .map(|i| match policies.get_mut(i) {
                Some(p) if self.tanks[i].alive => p.act(&self.observe(i)),
                _ => Action::default(),
            })
            .collect();
        self.step(&actions)
    }

    /// Run to completion with the given policies.
    pub fn run(&mut self, policies: &mut [&mut dyn Policy]) -> Outcome {
        loop {
            if let Some(o) = self.step_policies(policies) {
                return o;
            }
        }
    }

    fn check_end(&self) -> Option<Outcome> {
        let mut teams: Vec<u8> = self
            .tanks
            .iter()
            .filter(|t| t.alive)
            .map(|t| t.team)
            .collect();
        teams.sort_unstable();
        teams.dedup();
        let (winner, reason) = match teams.len() {
            0 => (None, EndReason::AllDestroyed),
            1 if self.tanks.iter().any(|t| t.team != teams[0]) => {
                (Some(teams[0]), EndReason::LastStanding)
            }
            _ if self.tick >= self.config.max_ticks => (None, EndReason::TickLimit),
            _ => return None,
        };
        Some(Outcome {
            winner,
            ticks: self.tick,
            reason,
        })
    }

    /// FNV-1a hash of the full simulation state (bit patterns, so any drift shows).
    pub fn state_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut eat = |v: u64| {
            for b in v.to_le_bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
        };
        eat(self.tick as u64);
        for t in &self.tanks {
            eat(t.pos.x.to_bits() as u64);
            eat(t.pos.y.to_bits() as u64);
            eat(t.vel.x.to_bits() as u64);
            eat(t.vel.y.to_bits() as u64);
            eat(t.heading as u64);
            eat(t.turret as u64);
            eat(t.hp as u32 as u64);
            eat(t.cooldown as u64);
            eat(t.alive as u64);
        }
        for p in &self.projectiles {
            eat(p.owner as u64);
            eat(p.pos.x.to_bits() as u64);
            eat(p.pos.y.to_bits() as u64);
            eat(p.vel.x.to_bits() as u64);
            eat(p.vel.y.to_bits() as u64);
            eat(p.ttl as u64);
        }
        h
    }

    /// Package this match (so far) as a replay.
    pub fn replay(&self) -> Replay {
        Replay::from_match(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::angle::{from_degrees, QUARTER_TURN};

    fn empty_config(spawns: Vec<TankSpawn>) -> MatchConfig {
        MatchConfig {
            arena: Arena::new(400.0, 400.0),
            tanks: spawns,
            params: TankParams {
                projectile_spread: 0,
                ..Default::default()
            },
            max_ticks: 600,
        }
    }

    fn at(team: u8, x: f32, y: f32, h: Heading) -> TankSpawn {
        TankSpawn {
            team,
            pos: Some(Vec2::new(x, y)),
            heading: Some(h),
        }
    }

    fn drive() -> Action {
        Action {
            throttle: 1.0,
            ..Default::default()
        }
    }

    #[test]
    fn tank_moves_forward_at_max_speed() {
        let mut m = Match::new(
            empty_config(vec![at(0, 100.0, 100.0, 0), at(1, 300.0, 300.0, 0)]),
            1,
        );
        for _ in 0..60 {
            m.step(&[drive()]);
        }
        let t = &m.tanks()[0];
        assert!((t.pos.x - 220.0).abs() < 0.01, "x={}", t.pos.x);
        assert!((t.pos.y - 100.0).abs() < 0.01);
    }

    #[test]
    fn wall_clamps_position() {
        let mut m = Match::new(
            empty_config(vec![at(0, 350.0, 200.0, 0), at(1, 50.0, 50.0, 0)]),
            1,
        );
        for _ in 0..120 {
            m.step(&[drive()]);
        }
        let t = &m.tanks()[0];
        assert_eq!(t.pos.x, 400.0 - 16.0);
        assert_eq!(t.vel.x, 0.0);
    }

    #[test]
    fn obstacle_blocks_movement() {
        let mut cfg = empty_config(vec![at(0, 100.0, 200.0, 0), at(1, 50.0, 50.0, 0)]);
        cfg.arena = cfg
            .arena
            .with_obstacle(Rect::new(Vec2::new(200.0, 150.0), Vec2::new(220.0, 250.0)));
        let mut m = Match::new(cfg, 1);
        for _ in 0..120 {
            m.step(&[drive()]);
        }
        let t = &m.tanks()[0];
        assert!(t.pos.x <= 200.0 - 16.0 && t.pos.x > 180.0, "x={}", t.pos.x);
    }

    #[test]
    fn tanks_do_not_overlap() {
        // Drive head-on into each other.
        let mut m = Match::new(
            empty_config(vec![
                at(0, 100.0, 200.0, 0),
                at(1, 300.0, 200.0, from_degrees(180)),
            ]),
            1,
        );
        for _ in 0..200 {
            m.step(&[drive(), drive()]);
        }
        let [a, b] = [&m.tanks()[0], &m.tanks()[1]];
        assert!((a.pos - b.pos).length_squared() >= 32.0 * 32.0);
        assert!(b.pos.x - a.pos.x < 40.0);
    }

    #[test]
    fn slides_along_wall_on_free_axis() {
        // Heading up-right into the top wall keeps moving right.
        let mut m = Match::new(
            empty_config(vec![
                at(0, 100.0, 380.0, from_degrees(45)),
                at(1, 50.0, 50.0, 0),
            ]),
            1,
        );
        let x0 = m.tanks()[0].pos.x;
        for _ in 0..30 {
            m.step(&[drive()]);
        }
        let t = &m.tanks()[0];
        assert_eq!(t.pos.y, 400.0 - 16.0);
        assert!(t.pos.x > x0 + 30.0);
    }

    #[test]
    fn turning_and_turret() {
        let mut m = Match::new(
            empty_config(vec![at(0, 100.0, 100.0, 0), at(1, 300.0, 300.0, 0)]),
            1,
        );
        let a = Action {
            turn: 1.0,
            turret_turn: -1.0,
            ..Default::default()
        };
        for _ in 0..10 {
            m.step(&[a]);
        }
        assert_eq!(m.tanks()[0].heading, 3640);
        assert_eq!(m.tanks()[0].turret, 0u16.wrapping_sub(5460));
    }

    #[test]
    fn projectile_hits_and_kills() {
        let mut m = Match::new(
            empty_config(vec![
                at(0, 100.0, 200.0, 0),
                at(1, 300.0, 200.0, QUARTER_TURN),
            ]),
            1,
        );
        let fire = Action {
            fire: true,
            ..Default::default()
        };
        let mut hits = 0;
        let out = loop {
            let o = m.step(&[fire]);
            hits += m
                .events()
                .iter()
                .filter(|e| matches!(e, Event::Hit { target: 1, .. }))
                .count();
            if let Some(o) = o {
                break o;
            }
        };
        assert_eq!(hits, 5);
        assert_eq!(out.winner, Some(0));
        assert_eq!(out.reason, EndReason::LastStanding);
        assert!(!m.tanks()[1].alive);
        assert!(m.step(&[fire]).is_some(), "stepping after end is a no-op");
        assert_eq!(m.tick(), out.ticks);
    }

    #[test]
    fn fast_projectile_does_not_tunnel_through_tank() {
        // 100 units/tick: from the muzzle at x=117 the shot's tick end points are
        // x=217, 317, 417... Tank 1 at x=250 (radius 16) contains none of them, so an
        // end-point-only check would let the shot pass straight through. Sweeping hits.
        let mut cfg = empty_config(vec![
            at(0, 100.0, 200.0, 0),
            at(1, 250.0, 200.0, QUARTER_TURN),
        ]);
        cfg.arena = Arena::new(1000.0, 400.0);
        cfg.params.projectile_speed = 100.0 * TICK_HZ as f32;
        let muzzle = 100.0 + 16.0 + 1.0;
        for k in 1..=8 {
            let x: f32 = muzzle + 100.0 * k as f32;
            assert!((x - 250.0).abs() >= 16.0, "end point {x} would already hit");
        }
        let mut m = Match::new(cfg, 1);
        let fire = Action {
            fire: true,
            ..Default::default()
        };
        m.step(&[fire]); // spawns at the muzzle; its first move is next tick
        assert_eq!(m.projectiles().len(), 1);
        m.step(&[]); // 117 -> 217: short of the tank
        assert!(m.events().is_empty());
        assert_eq!(m.projectiles().len(), 1);
        m.step(&[]); // 217 -> 317: passes through x = 250
        assert!(
            m.events().iter().any(|e| matches!(
                e,
                Event::Hit {
                    target: 1,
                    owner: 0,
                    ..
                }
            )),
            "events: {:?}",
            m.events()
        );
        assert!(m.projectiles().is_empty());
        assert_eq!(m.tanks()[1].hp, 100 - 20);
    }

    #[test]
    fn fast_projectile_does_not_tunnel_through_obstacle() {
        // Thin wall between the tanks. The shot's end points (217, 317) straddle it and
        // 317 lies inside tank 1 (x = 320), so an end-point check would hit through the
        // wall. Sweeping finds the wall first (t ~ 0.33) before the tank (t ~ 0.87).
        let mut cfg = empty_config(vec![
            at(0, 100.0, 200.0, 0),
            at(1, 320.0, 200.0, QUARTER_TURN),
        ]);
        cfg.arena = cfg
            .arena
            .with_obstacle(Rect::new(Vec2::new(250.0, 100.0), Vec2::new(255.0, 300.0)));
        cfg.params.projectile_speed = 100.0 * TICK_HZ as f32;
        let mut m = Match::new(cfg, 1);
        let fire = Action {
            fire: true,
            ..Default::default()
        };
        m.step(&[fire]);
        for _ in 0..5 {
            m.step(&[]);
            assert!(!m.events().iter().any(|e| matches!(e, Event::Hit { .. })));
        }
        assert!(m.projectiles().is_empty());
        assert_eq!(m.tanks()[1].hp, 100);
    }

    #[test]
    fn movement_does_not_depend_on_tank_id_order() {
        // The same head-on duel with the ids swapped ends in the same positions.
        let left = at(0, 100.0, 200.0, 0);
        let right = at(1, 300.0, 200.0, from_degrees(180));
        let mut a = Match::new(empty_config(vec![left.clone(), right.clone()]), 1);
        let mut b = Match::new(empty_config(vec![right, left]), 1);
        for _ in 0..200 {
            a.step(&[drive(), drive()]);
            b.step(&[drive(), drive()]);
        }
        assert_eq!(a.tanks()[0].pos, b.tanks()[1].pos);
        assert_eq!(a.tanks()[1].pos, b.tanks()[0].pos);
        let gap = a.tanks()[1].pos.x - a.tanks()[0].pos.x;
        assert!((32.0..40.0).contains(&gap), "gap={gap}");
    }

    #[test]
    fn projectile_dies_on_wall_and_cooldown_applies() {
        let mut m = Match::new(
            empty_config(vec![
                at(0, 100.0, 100.0, from_degrees(180)),
                at(1, 300.0, 300.0, 0),
            ]),
            1,
        );
        let fire = Action {
            fire: true,
            ..Default::default()
        };
        m.step(&[fire]);
        assert_eq!(m.projectiles().len(), 1);
        m.step(&[fire]);
        assert_eq!(m.projectiles().len(), 1, "cooldown blocks a second shot");
        for _ in 0..20 {
            m.step(&[]);
        }
        assert!(m.projectiles().is_empty());
    }

    #[test]
    fn tick_limit_is_a_draw() {
        let mut cfg = empty_config(vec![at(0, 100.0, 100.0, 0), at(1, 300.0, 300.0, 0)]);
        cfg.max_ticks = 10;
        let mut m = Match::new(cfg, 1);
        let mut out = None;
        for _ in 0..10 {
            out = m.step(&[]);
        }
        let out = out.expect("ended");
        assert_eq!(out.winner, None);
        assert_eq!(out.reason, EndReason::TickLimit);
        assert_eq!(out.ticks, 10);
    }

    #[test]
    fn random_spawns_are_seeded_and_valid() {
        let a = Match::new(MatchConfig::duel(), 7);
        let b = Match::new(MatchConfig::duel(), 7);
        let c = Match::new(MatchConfig::duel(), 8);
        assert_eq!(a.tanks(), b.tanks());
        assert_ne!(a.tanks(), c.tanks());
        for t in a.tanks() {
            assert!(a.config().arena.circle_in_bounds(t.pos, 16.0));
            assert!(!a.config().arena.circle_hits_obstacle(t.pos, 16.0));
        }
    }

    #[test]
    fn observation_sorts_enemies() {
        let m = Match::new(
            empty_config(vec![
                at(0, 100.0, 100.0, 0),
                at(1, 300.0, 300.0, 0),
                at(1, 150.0, 100.0, 0),
                at(0, 100.0, 300.0, 0),
            ]),
            1,
        );
        let o = m.observe(0);
        assert_eq!(
            o.enemies.iter().map(|e| e.id).collect::<Vec<_>>(),
            vec![2, 1]
        );
        assert_eq!(o.allies.len(), 1);
        assert_eq!(o.walls.right, 300.0);
    }
}
