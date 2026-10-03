//! Tank Arena on the generic core: config, entities, and the tank rules ([`TankRules`]).
//!
//! A [`Match`] (`generic::Match<TankRules>`) is built from a [`MatchConfig`] and a `u64`
//! seed, then advanced one fixed tick ([`crate::DT`] seconds) at a time with
//! [`Match::step`] (explicit actions) or [`Match::step_policies`] / [`Match::run`]
//! (policies). The loop itself is generic ([`crate::generic`]); see [`TankRules`] for
//! the exact order of operations inside a tank tick.

use crate::angle::{self, Heading};
use crate::arena::{circles_overlap, segment_circle_entry, Arena, Rect};
pub use crate::generic::{EndReason, Outcome};

use crate::generic::{self, MatchRng, Rules, StateHasher};
use crate::policy::{
    Action, Observation, ProjectileObs, SelfObs, TankObs, WallObs, MAX_OBSERVED_PROJECTILES,
    MAX_OBSERVED_TANKS,
};
use crate::{DT, TICK_HZ};
use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Per-tank tunables. Speeds are per second; the sim scales by `DT`.
/// Turn rates are BAU per tick (65536 BAU = one turn).
///
/// [`MatchConfig::params`] is the shared set; a [`TankSpawn::params`] overrides it for
/// one tank (all fields except `radius`). [`MatchConfig::tank_params`] resolves the
/// set a tank actually uses.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TankParams {
    /// Collision radius of every tank (tanks are circles). Always taken from
    /// [`MatchConfig::params`]; a per-tank value is ignored, so collisions stay symmetric.
    pub radius: f32,
    /// Speed at `throttle = 1.0`, in units per second.
    pub max_speed: f32,
    /// Hull turn at `turn = 1.0`, in BAU per tick.
    pub turn_rate: u16,
    /// Turret turn at `turret_turn = 1.0`, in BAU per tick.
    pub turret_turn_rate: u16,
    /// Starting (and maximum) hit points.
    pub max_hp: i32,
    /// Ticks between shots.
    pub fire_cooldown: u32,
    /// Projectile speed in units per second.
    pub projectile_speed: f32,
    /// Projectile lifetime in ticks.
    pub projectile_ttl: u32,
    /// HP removed from a tank by one hit.
    pub projectile_damage: i32,
    /// Max random deviation of a shot, in BAU either side (drawn from the match RNG).
    pub projectile_spread: u16,
    /// Optional stationary accuracy: the spread used instead of `projectile_spread`
    /// when the tank fires on a tick in which it did not move (its applied velocity
    /// that tick is exactly zero, e.g. throttle 0 or blocked; turning in place counts
    /// as still). `None` (the default): always `projectile_spread`. Omitted from JSON
    /// when `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projectile_spread_still: Option<u16>,
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
            projectile_spread_still: None,
        }
    }
}

/// Where a tank starts. `None` fields are drawn from the match RNG.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TankSpawn {
    /// Team id. Tanks on the same team are allies; the last team with living tanks wins.
    pub team: u8,
    /// Start position (circle centre). `None`: random clear spot from the match RNG.
    /// Explicit positions are used as given (not checked against walls or obstacles).
    #[serde(default)]
    pub pos: Option<Vec2>,
    /// Start heading (the turret starts aligned with it). `None`: random from the match RNG.
    #[serde(default)]
    pub heading: Option<Heading>,
    /// Per-tank params. `None` (the default): [`MatchConfig::params`]. When set, it
    /// replaces the shared set as a whole (no field-by-field merge): every field
    /// applies to this tank except `radius`, which stays shared. Omitted from JSON when
    /// `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<TankParams>,
}

/// Everything needed (with a seed) to reproduce a match.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatchConfig {
    /// Arena bounds and obstacles.
    pub arena: Arena,
    /// One spawn per tank; the index is the tank id.
    pub tanks: Vec<TankSpawn>,
    /// Tunables for every tank without its own [`TankSpawn::params`]; `radius` is
    /// shared by all tanks.
    pub params: TankParams,
    /// Match ends in a draw after this many ticks.
    pub max_ticks: u32,
}

impl MatchConfig {
    /// 1v1 in an 800x600 arena with two obstacles, random spawns, 2-minute limit.
    ///
    /// ```
    /// use engine::{MatchConfig, TICK_HZ};
    /// let c = MatchConfig::duel();
    /// assert_eq!((c.arena.size.x, c.arena.size.y), (800.0, 600.0));
    /// assert_eq!(c.arena.obstacles.len(), 2);
    /// assert_eq!(c.tanks.len(), 2);
    /// assert_eq!(c.max_ticks, 120 * TICK_HZ); // 7200 ticks
    /// ```
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

    /// The params tank `id` plays with: its [`TankSpawn::params`] if set, else
    /// [`MatchConfig::params`]. `radius` is always the shared `params.radius`.
    ///
    /// Panics if `id` is out of range.
    ///
    /// ```
    /// use engine::{MatchConfig, TankParams};
    /// let mut c = MatchConfig::duel();
    /// c.tanks[1].params = Some(TankParams { max_hp: 140, radius: 99.0, ..Default::default() });
    /// assert_eq!(c.tank_params(0), c.params);
    /// assert_eq!(c.tank_params(1).max_hp, 140);
    /// assert_eq!(c.tank_params(1).radius, c.params.radius); // radius stays shared
    /// ```
    pub fn tank_params(&self, id: usize) -> TankParams {
        match &self.tanks[id].params {
            Some(p) => TankParams {
                radius: self.params.radius,
                ..p.clone()
            },
            None => self.params.clone(),
        }
    }
}

/// A tank entity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tank {
    /// Index into [`Match::tanks`] and into each tick's action list.
    pub id: usize,
    /// Team id, from the spawn.
    pub team: u8,
    /// Circle centre.
    pub pos: Vec2,
    /// Velocity in units per tick (after collision; blocked axes are zeroed).
    pub vel: Vec2,
    /// Hull heading.
    pub heading: Heading,
    /// Turret heading in the world frame (not relative to the hull).
    pub turret: Heading,
    /// Hit points. Not clamped: stays negative after an overkill hit.
    pub hp: i32,
    /// Ticks until the gun can fire again (0 = ready).
    pub cooldown: u32,
    /// False once `hp <= 0`. Dead tanks stay in the list but do not move, fire or block.
    pub alive: bool,
}

/// A generic projectile: travels in a straight line, dies on walls/obstacles/ttl,
/// damages the first enemy tank it overlaps.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Projectile {
    /// Id of the tank that fired it.
    pub owner: usize,
    /// Team of the tank that fired it; projectiles pass through tanks of this team.
    pub team: u8,
    /// Current position.
    pub pos: Vec2,
    /// Units per tick.
    pub vel: Vec2,
    /// Remaining lifetime in ticks.
    pub ttl: u32,
    /// HP removed on hit.
    pub damage: i32,
}

/// Things that happened during the last step, for viewers and rule layers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    /// Tank `tank` fired a projectile this tick.
    Fired {
        /// Id of the tank that fired.
        tank: usize,
    },
    /// A projectile fired by `owner` hit tank `target`.
    Hit {
        /// Id of the tank that was hit.
        target: usize,
        /// Id of the tank that fired the projectile.
        owner: usize,
        /// HP removed.
        damage: i32,
    },
    /// Tank `tank` reached `hp <= 0` this tick.
    Destroyed {
        /// Id of the destroyed tank.
        tank: usize,
    },
}

/// A running Tank Arena match: the generic [`generic::Match`] with [`TankRules`]
/// (ADR-014 step B1). Create with [`Match::new`], drive with [`Match::step`] or
/// [`Match::run`], inspect with the accessors. Works identically headless and in wasm.
///
/// Policies come from game crates (e.g. `tank::Chaser`); any
/// `FnMut(&Observation) -> Action` closure or function works too.
///
/// ```
/// use engine::angle::turn_toward;
/// use engine::{Action, Match, MatchConfig, Observation};
///
/// // Drive at the nearest enemy and fire when the turret is on it.
/// fn charge(obs: &Observation) -> Action {
///     let Some(e) = obs.enemies.first() else {
///         return Action::default();
///     };
///     let aim = turn_toward(obs.me.turret, e.rel, 0.08);
///     Action {
///         throttle: 1.0,
///         turn: turn_toward(obs.me.heading, e.rel, 0.2) as f32,
///         turret_turn: aim as f32,
///         fire: aim == 0,
///     }
/// }
///
/// let mut m = Match::new(MatchConfig::duel(), 42);
/// let outcome = m.run(&mut [&mut charge, &mut charge]);
/// assert_eq!(outcome.ticks, m.tick());
///
/// // Same seed, same policies: the same match, bit for bit.
/// let mut again = Match::new(MatchConfig::duel(), 42);
/// assert_eq!(again.run(&mut [&mut charge, &mut charge]), outcome);
/// assert_eq!(again.state_hash(), m.state_hash());
/// ```
pub type Match = generic::Match<TankRules>;

/// Tank Arena's dynamic state ([`TankRules`]' `State`): the tanks, the projectiles in
/// flight, and each tank's resolved params. Read it through [`Match::tanks`],
/// [`Match::projectiles`] and [`Match::tank_params`], or [`generic::Match::state`].
#[derive(Clone, PartialEq)]
pub struct TankState {
    /// `config.tank_params(id)` for every tank, resolved once at init.
    params: Vec<TankParams>,
    tanks: Vec<Tank>,
    projectiles: Vec<Projectile>,
    /// Working memory for [`TankRules::step`]; not part of the match state.
    scratch: StepScratch,
}

impl std::fmt::Debug for TankState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TankState")
            .field("params", &self.params)
            .field("tanks", &self.tanks)
            .field("projectiles", &self.projectiles)
            .finish()
    }
}

/// Buffers [`TankRules::step`] reuses from tick to tick, so a step allocates nothing
/// once they have grown to the match's size. They hold no state between steps: a
/// clone starts empty, and every two scratches compare equal.
#[derive(Default)]
struct StepScratch {
    /// Each tank's (position, velocity) after its move, before it is applied.
    moves: Vec<(Vec2, Vec2)>,
}

impl Clone for StepScratch {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl PartialEq for StepScratch {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl TankState {
    /// All tanks, living and dead, indexed by id.
    pub fn tanks(&self) -> &[Tank] {
        &self.tanks
    }
    /// Projectiles in flight.
    pub fn projectiles(&self) -> &[Projectile] {
        &self.projectiles
    }
    /// The params tank `id` plays with ([`MatchConfig::tank_params`], resolved at init).
    /// Panics if `id` is out of range.
    pub fn tank_params(&self, id: usize) -> &TankParams {
        &self.params[id]
    }
}

impl Match {
    /// All tanks, living and dead, indexed by id.
    pub fn tanks(&self) -> &[Tank] {
        self.state().tanks()
    }
    /// Projectiles in flight.
    pub fn projectiles(&self) -> &[Projectile] {
        self.state().projectiles()
    }
    /// The params tank `id` plays with ([`MatchConfig::tank_params`], resolved at
    /// creation). Panics if `id` is out of range.
    pub fn tank_params(&self, id: usize) -> &TankParams {
        self.state().tank_params(id)
    }
}

/// The Tank Arena rules, implemented against [`Rules`]. They live in `engine` for now;
/// ADR-014 defers moving them to `games/tank` until a second Rust game exists.
///
/// Agents are tanks (agent `i` is tank `i`); an agent is active while its tank is
/// alive; actions are clamped with [`Action::clamped`].
///
/// Step order ([`Rules::step`]):
/// 1. every living tank turns and computes its move against the tick-start
///    positions of the others (axis-separated: walls clamp, obstacles and tanks
///    block the axis);
/// 2. moves are applied simultaneously; any tank whose new circle would overlap
///    another tank's new circle is held at its old position (repeated until
///    stable), so the result does not depend on tank id order;
/// 3. tanks fire in id order (spread is drawn from the match RNG in that order, one
///    draw per shot whose effective spread is non-zero; a tank that did not move
///    this tick uses [`TankParams::projectile_spread_still`] if set);
/// 4. existing projectiles sweep their whole per-tick segment: the earliest
///    contact with an enemy tank (at its post-move position) or a wall/obstacle
///    wins, ties go to the tank, then to the lower tank id;
/// 5. new shots are added, deaths applied.
///
/// Each tank moves, turns and fires with its own [`Match::tank_params`]; the radius
/// is shared. Actions for dead tanks are ignored. The end check ([`Rules::outcome`])
/// runs after every step, in this order: no tank alive (`all_destroyed`), one team left
/// with some tank of another team existing (`last_standing`), tick limit (`tick_limit`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TankRules;

fn rand_unit(rng: &mut MatchRng) -> f32 {
    (rng.next_u32() >> 8) as f32 * (1.0 / (1u32 << 24) as f32)
}

impl Rules for TankRules {
    type Config = MatchConfig;
    type State = TankState;
    type Action = Action;
    type Observation = Observation;
    type Event = Event;

    /// Spawns the tanks. Tanks are spawned in id order; a random position is retried up
    /// to 1000 times until it is clear of obstacles (by 1.5 radii) and of already-placed
    /// tanks (centres at least 6 radii apart), falling back to the arena centre. A random
    /// heading is drawn after the position. Each tank starts with its own
    /// [`TankParams::max_hp`] ([`MatchConfig::tank_params`]).
    fn init(config: &MatchConfig, rng: &mut MatchRng) -> TankState {
        let r = config.params.radius;
        let params: Vec<TankParams> = (0..config.tanks.len())
            .map(|id| config.tank_params(id))
            .collect();
        let mut tanks: Vec<Tank> = Vec::with_capacity(config.tanks.len());
        for (id, spawn) in config.tanks.iter().enumerate() {
            let pos = match spawn.pos {
                Some(p) => p,
                None => {
                    let a = &config.arena;
                    let mut p = a.clamp_circle(a.size * 0.5, r);
                    for _ in 0..1000 {
                        let cand = Vec2::new(
                            r + rand_unit(rng) * (a.size.x - 2.0 * r),
                            r + rand_unit(rng) * (a.size.y - 2.0 * r),
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
                hp: params[id].max_hp,
                cooldown: 0,
                alive: true,
            });
        }
        TankState {
            params,
            tanks,
            projectiles: Vec::new(),
            scratch: StepScratch::default(),
        }
    }

    fn agents(state: &TankState) -> usize {
        state.tanks.len()
    }

    fn is_active(state: &TankState, agent: usize) -> bool {
        state.tanks[agent].alive
    }

    fn sanitize(action: Action) -> Action {
        action.clamped()
    }

    fn step(
        config: &MatchConfig,
        state: &mut TankState,
        actions: &[Action],
        rng: &mut MatchRng,
        events: &mut Vec<Event>,
    ) {
        // No allocation once the buffers have grown: `moves` is reused scratch, new shots
        // go straight onto the projectile list, and step 4 filters that list in place.
        // Tank positions don't change until step 3, so `state.tanks[i].pos` is tank i's
        // tick-start position in steps 1 and 2 (tank i is at index i).
        let n = state.tanks.len();
        let r = config.params.radius;

        // 1. Intents against the tick-start snapshot.
        let moves = &mut state.scratch.moves;
        moves.clear();
        moves.extend(state.tanks.iter().map(|t| (t.pos, Vec2::ZERO)));
        for (i, &a) in actions.iter().enumerate() {
            if !state.tanks[i].alive {
                continue;
            }
            let params = &state.params[i];
            let t = &mut state.tanks[i];
            t.heading = t
                .heading
                .wrapping_add_signed((a.turn * params.turn_rate as f32) as i16);
            t.turret = t
                .turret
                .wrapping_add_signed((a.turret_turn * params.turret_turn_rate as f32) as i16);
            let want = angle::dir(t.heading) * (a.throttle * params.max_speed * DT);
            let mut pos = t.pos;
            let mut vel = want;
            for axis in 0..2 {
                let mut cand = pos;
                cand[axis] += want[axis];
                cand = config.arena.clamp_circle(cand, r);
                let blocked = config.arena.circle_hits_obstacle(cand, r)
                    || state
                        .tanks
                        .iter()
                        .any(|o| o.alive && o.id != i && circles_overlap(o.pos, r, cand, r));
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
                if !state.tanks[i].alive || moves[i].0 == state.tanks[i].pos {
                    continue;
                }
                let clash = (0..n).any(|j| {
                    j != i && state.tanks[j].alive && circles_overlap(moves[j].0, r, moves[i].0, r)
                });
                if clash {
                    moves[i] = (state.tanks[i].pos, Vec2::ZERO);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        // 3. Fire. New shots are appended after the `in_flight` existing ones, which
        // step 4 moves; the new ones are kept as they are.
        let in_flight = state.projectiles.len();
        for (i, &a) in actions.iter().enumerate() {
            if !state.tanks[i].alive {
                continue;
            }
            let params = &state.params[i];
            let t = &mut state.tanks[i];
            (t.pos, t.vel) = moves[i];
            if t.cooldown > 0 {
                t.cooldown -= 1;
            }
            if a.fire && t.cooldown == 0 {
                let spread = match params.projectile_spread_still {
                    Some(still) if t.vel == Vec2::ZERO => still,
                    _ => params.projectile_spread,
                } as u32;
                let dev = if spread > 0 {
                    (rng.next_u32() % (2 * spread + 1)) as i32 - spread as i32
                } else {
                    0
                };
                let h = t.turret.wrapping_add_signed(dev as i16);
                let d = angle::dir(h);
                state.projectiles.push(Projectile {
                    owner: i,
                    team: t.team,
                    pos: t.pos + d * (r + 1.0),
                    vel: d * (params.projectile_speed * DT),
                    ttl: params.projectile_ttl,
                    damage: params.projectile_damage,
                });
                t.cooldown = params.fire_cooldown;
                events.push(Event::Fired { tank: i });
            }
        }

        // 4. Existing shots in list order (`retain_mut` visits each once, in order, and
        // keeps the survivors' order); the new shots after them stay as they are.
        let tanks = &mut state.tanks;
        let mut index = 0;
        state.projectiles.retain_mut(|pr| {
            index += 1;
            if index > in_flight {
                return true;
            }
            // Swept collision over the whole tick: no tunnelling at any speed.
            let mut hit: Option<(f32, usize)> = None;
            for t in tanks.iter().filter(|t| t.alive && t.team != pr.team) {
                if let Some(tt) = segment_circle_entry(pr.pos, pr.vel, t.pos, r) {
                    if hit.is_none_or(|(bt, _)| tt < bt) {
                        hit = Some((tt, t.id));
                    }
                }
            }
            let wall = config.arena.segment_blocked_at(pr.pos, pr.vel);
            pr.pos += pr.vel;
            match (hit, wall) {
                (Some((tt, ti)), w) if w.is_none_or(|wt| tt <= wt) => {
                    tanks[ti].hp -= pr.damage;
                    events.push(Event::Hit {
                        target: ti,
                        owner: pr.owner,
                        damage: pr.damage,
                    });
                    return false;
                }
                (_, Some(_)) => return false,
                _ => {}
            }
            if pr.ttl <= 1 {
                return false;
            }
            pr.ttl -= 1;
            true
        });

        for t in state.tanks.iter_mut() {
            if t.alive && t.hp <= 0 {
                t.alive = false;
                t.vel = Vec2::ZERO;
                events.push(Event::Destroyed { tank: t.id });
            }
        }
    }

    /// `max_hp` values are each tank's own ([`Match::tank_params`]); `los` on each
    /// listed tank is [`Arena::segment_clear`] between the two tank centres (walls and
    /// obstacles block it; tanks do not). Panics if the id is out of range. Works for
    /// dead tanks too.
    fn observe(config: &MatchConfig, state: &TankState, id: usize, tick: u32) -> Observation {
        let me = &state.tanks[id];
        let arena = &config.arena;
        let mut enemies = Vec::new();
        let mut allies = Vec::new();
        for t in state.tanks.iter().filter(|t| t.alive && t.id != id) {
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
                max_hp: state.params[t.id].max_hp,
                los: false,
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
        for o in enemies.iter_mut().chain(allies.iter_mut()) {
            o.los = arena.segment_clear(me.pos, o.pos);
        }
        let mut projectiles: Vec<ProjectileObs> = state
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
        let size = config.arena.size;
        Observation {
            tick,
            me: SelfObs {
                id,
                team: me.team,
                pos: me.pos,
                vel: me.vel,
                heading: me.heading,
                turret: me.turret,
                hp: me.hp,
                max_hp: state.params[id].max_hp,
                cooldown: me.cooldown,
                radius: config.params.radius,
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
            obstacles: config.arena.obstacles.clone(),
        }
    }

    fn outcome(config: &MatchConfig, state: &TankState, tick: u32) -> Option<Outcome> {
        // The teams of the living tanks, without collecting them: the first one, then
        // whether every other living tank is on it (exactly one team left).
        let mut alive = state.tanks.iter().filter(|t| t.alive).map(|t| t.team);
        let (winner, reason) = match alive.next() {
            None => (None, EndReason::AllDestroyed),
            Some(team)
                if alive.all(|other| other == team)
                    && state.tanks.iter().any(|t| t.team != team) =>
            {
                (Some(team), EndReason::LastStanding)
            }
            _ if tick >= config.max_ticks => (None, EndReason::TickLimit),
            _ => return None,
        };
        Some(Outcome {
            winner,
            ticks: tick,
            reason,
        })
    }

    /// Per tank: `pos`, `vel`, `heading`, `turret`, `hp`, `cooldown`, `alive`; then per
    /// projectile: `owner`, `pos`, `vel`, `ttl`; in order.
    fn hash_state(state: &TankState, h: &mut StateHasher) {
        for t in &state.tanks {
            h.write_u64(t.pos.x.to_bits() as u64);
            h.write_u64(t.pos.y.to_bits() as u64);
            h.write_u64(t.vel.x.to_bits() as u64);
            h.write_u64(t.vel.y.to_bits() as u64);
            h.write_u64(t.heading as u64);
            h.write_u64(t.turret as u64);
            h.write_u64(t.hp as u32 as u64);
            h.write_u64(t.cooldown as u64);
            h.write_u64(t.alive as u64);
        }
        for p in &state.projectiles {
            h.write_u64(p.owner as u64);
            h.write_u64(p.pos.x.to_bits() as u64);
            h.write_u64(p.pos.y.to_bits() as u64);
            h.write_u64(p.vel.x.to_bits() as u64);
            h.write_u64(p.vel.y.to_bits() as u64);
            h.write_u64(p.ttl as u64);
        }
    }

    /// Formats 2 and 3 have no per-tank `params` and no `projectile_spread_still`.
    fn check_format(config: &MatchConfig, format: u32) -> Result<(), &'static str> {
        if format < 4 {
            if config.tanks.iter().any(|t| t.params.is_some()) {
                return Err("tanks[].params");
            }
            if config.params.projectile_spread_still.is_some() {
                return Err("params.projectile_spread_still");
            }
        }
        Ok(())
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
            params: None,
        }
    }

    fn with_params(mut s: TankSpawn, p: TankParams) -> TankSpawn {
        s.params = Some(p);
        s
    }

    fn fire() -> Action {
        Action {
            fire: true,
            ..Default::default()
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

    // --- Engine ask #5: per-tank params ---

    #[test]
    fn tank_params_resolve_per_spawn_with_shared_radius() {
        let own = TankParams {
            max_hp: 60,
            radius: 99.0,
            ..Default::default()
        };
        let cfg = empty_config(vec![
            at(0, 100.0, 100.0, 0),
            with_params(at(1, 300.0, 300.0, 0), own.clone()),
        ]);
        assert_eq!(cfg.tank_params(0), cfg.params);
        assert_eq!(cfg.tank_params(1).max_hp, 60);
        assert_eq!(cfg.tank_params(1).radius, 16.0, "radius stays shared");
        let m = Match::new(cfg.clone(), 1);
        assert_eq!(m.tank_params(1), &cfg.tank_params(1));
        assert_eq!(m.tanks()[0].hp, 100);
        assert_eq!(m.tanks()[1].hp, 60, "starts at its own max_hp");
    }

    #[test]
    fn per_tank_speed_turn_rate_and_radius() {
        // Tank 0: Speed 5 (150 u/s, 455 BAU/tick) and a per-tank radius that must be
        // ignored. Tank 1: shared defaults (120 u/s, 364 BAU/tick).
        let fast = TankParams {
            max_speed: 150.0,
            turn_rate: 455,
            turret_turn_rate: 600,
            radius: 50.0,
            ..Default::default()
        };
        let cfg = empty_config(vec![
            with_params(at(0, 50.0, 100.0, 0), fast),
            at(1, 50.0, 300.0, 0),
        ]);
        let mut m = Match::new(cfg, 1);
        for _ in 0..60 {
            m.step(&[drive(), drive()]);
        }
        assert!(
            (m.tanks()[0].pos.x - 200.0).abs() < 0.01,
            "{:?}",
            m.tanks()[0]
        );
        assert!(
            (m.tanks()[1].pos.x - 170.0).abs() < 0.01,
            "{:?}",
            m.tanks()[1]
        );
        // Wall clamp uses the shared radius 16, not the per-tank 50.
        for _ in 0..200 {
            m.step(&[drive(), drive()]);
        }
        assert_eq!(m.tanks()[0].pos.x, 400.0 - 16.0);
        let spin = Action {
            turn: 1.0,
            turret_turn: 1.0,
            ..Default::default()
        };
        let (h0, h1) = (m.tanks()[0].heading, m.tanks()[1].heading);
        let (t0, t1) = (m.tanks()[0].turret, m.tanks()[1].turret);
        for _ in 0..10 {
            m.step(&[spin, spin]);
        }
        assert_eq!(m.tanks()[0].heading, h0.wrapping_add(4550));
        assert_eq!(m.tanks()[1].heading, h1.wrapping_add(3640));
        assert_eq!(m.tanks()[0].turret, t0.wrapping_add(6000));
        assert_eq!(m.tanks()[1].turret, t1.wrapping_add(5460));
    }

    #[test]
    fn per_tank_damage_hp_and_gun() {
        // Glass Cannon (28 dmg, 60 HP) vs Brawler (24 dmg, 120 HP), facing each other.
        // A spawn's params replace the shared set as a whole, so spread 0 is repeated.
        let glass = TankParams {
            projectile_damage: 28,
            max_hp: 60,
            projectile_spread: 0,
            ..Default::default()
        };
        let brawler = TankParams {
            projectile_damage: 24,
            max_hp: 120,
            fire_cooldown: 30,
            projectile_speed: 480.0,
            projectile_spread: 0,
            ..Default::default()
        };
        let cfg = empty_config(vec![
            with_params(at(0, 100.0, 200.0, 0), glass),
            with_params(at(1, 300.0, 200.0, from_degrees(180)), brawler),
        ]);
        let mut m = Match::new(cfg, 1);
        m.step(&[fire(), fire()]);
        let shots = m.projectiles();
        assert_eq!(shots.len(), 2);
        assert_eq!((shots[0].damage, shots[1].damage), (28, 24));
        assert_eq!(shots[0].vel, angle::dir(0) * (360.0 * DT));
        assert_eq!(shots[1].vel, angle::dir(from_degrees(180)) * (480.0 * DT));
        assert_eq!((m.tanks()[0].cooldown, m.tanks()[1].cooldown), (45, 30));
        let mut hits = [0; 2];
        let out = loop {
            let o = m.step(&[fire(), fire()]);
            for e in m.events() {
                if let Event::Hit { target, damage, .. } = *e {
                    hits[target] += 1;
                    assert_eq!(damage, if target == 1 { 28 } else { 24 });
                }
            }
            if let Some(o) = o {
                break o;
            }
        };
        // Hits-to-kill from the spec table: 24 dmg kills 60 HP in 3; 28 kills 120 in 5.
        // The Brawler's faster gun lands its third hit first.
        assert_eq!(hits[0], 3, "{hits:?}");
        assert!(hits[1] < 5, "{hits:?}");
        assert_eq!(out.winner, Some(1));
        assert_eq!(m.tanks()[1].hp, 120 - 28 * hits[1]);
    }

    // --- Engine ask #3 (movement fairness) with per-tank speeds ---

    #[test]
    fn movement_with_per_tank_speeds_does_not_depend_on_id_order() {
        let fast = TankParams {
            max_speed: 150.0,
            ..Default::default()
        };
        let left = with_params(at(0, 100.0, 200.0, 0), fast);
        let right = at(1, 300.0, 200.0, from_degrees(180));
        let mut a = Match::new(empty_config(vec![left.clone(), right.clone()]), 1);
        let mut b = Match::new(empty_config(vec![right, left]), 1);
        for _ in 0..200 {
            a.step(&[drive(), drive()]);
            b.step(&[drive(), drive()]);
            assert_eq!(a.tanks()[0].pos, b.tanks()[1].pos);
            assert_eq!(a.tanks()[1].pos, b.tanks()[0].pos);
        }
    }

    // --- Engine ask #1 (swept hits) with a per-tank shell speed ---

    #[test]
    fn fast_per_tank_projectile_does_not_tunnel() {
        let gun = TankParams {
            projectile_speed: 100.0 * TICK_HZ as f32,
            ..Default::default()
        };
        let mut cfg = empty_config(vec![
            with_params(at(0, 100.0, 200.0, 0), gun),
            at(1, 250.0, 200.0, QUARTER_TURN),
        ]);
        cfg.arena = Arena::new(1000.0, 400.0);
        let mut m = Match::new(cfg, 1);
        m.step(&[fire()]);
        m.step(&[]);
        m.step(&[]); // 217 -> 317 passes through the tank at x = 250
        assert!(m.events().iter().any(|e| matches!(
            e,
            Event::Hit {
                target: 1,
                owner: 0,
                ..
            }
        )));
    }

    // --- Engine ask #4: optional stationary accuracy ---

    /// Signed table-bucket offset (64 BAU each) of a shot from the firing turret.
    fn shot_bucket(turret: Heading, vel: Vec2) -> i32 {
        let d = vel.normalize();
        (-8..=8)
            .find(|&k| {
                let e = angle::dir(turret.wrapping_add_signed(k as i16 * 64));
                (e - d).length_squared() < 1e-10
            })
            .expect("shot within 8 buckets of the turret")
    }

    /// Fire `n` shots from tank 0 (standing, or driving back and forth along +X/-X)
    /// and return the bucket offset of each.
    fn shot_spreads(spread_still: Option<u16>, moving: bool, n: usize) -> Vec<i32> {
        // Heading 0 is bucket-aligned, so a deviation in [-s, s] lands in bucket
        // [-s/64, s/64].
        let p = TankParams {
            projectile_spread: 256,
            projectile_spread_still: spread_still,
            fire_cooldown: 1,
            ..Default::default()
        };
        let mut cfg = empty_config(vec![at(0, 50.0, 200.0, 0), at(1, 350.0, 380.0, 0)]);
        cfg.params = p;
        cfg.max_ticks = 100_000;
        let mut m = Match::new(cfg, 99);
        let mut out = Vec::new();
        let mut tick = 0u32;
        while out.len() < n {
            let throttle = if !moving {
                0.0
            } else if (tick / 20).is_multiple_of(2) {
                1.0
            } else {
                -1.0
            };
            m.step(&[Action {
                throttle,
                fire: true,
                ..Default::default()
            }]);
            tick += 1;
            if m.events()
                .iter()
                .any(|e| matches!(e, Event::Fired { tank: 0 }))
            {
                let t = &m.tanks()[0];
                assert_eq!(t.vel == Vec2::ZERO, !moving, "tick {tick}");
                let pr = m.projectiles().last().expect("new shot is last");
                out.push(shot_bucket(t.turret, pr.vel));
            }
        }
        out
    }

    #[test]
    fn stationary_accuracy_is_optional_and_applies_only_when_still() {
        // Spec ask #4: 128 BAU when still, 256 when moving.
        let still = shot_spreads(Some(128), false, 300);
        let moving = shot_spreads(Some(128), true, 300);
        let off_still = shot_spreads(None, false, 300);
        assert!(still.iter().all(|b| b.abs() <= 2), "{still:?}");
        assert!(moving.iter().all(|b| b.abs() <= 4), "{moving:?}");
        assert!(
            moving.iter().any(|b| b.abs() > 2),
            "moving uses the full spread"
        );
        assert!(
            off_still.iter().any(|b| b.abs() > 2),
            "None: always full spread"
        );
        // Some(0) when still: dead straight, and no RNG draw for those shots.
        assert!(shot_spreads(Some(0), false, 50).iter().all(|&b| b == 0));
    }

    #[test]
    fn blocked_tank_counts_as_still() {
        // Driving into the wall: throttle 1 but the applied velocity is zero.
        let mut cfg = empty_config(vec![at(0, 400.0 - 16.0, 200.0, 0), at(1, 50.0, 50.0, 0)]);
        cfg.params.projectile_spread = 20_000;
        cfg.params.projectile_spread_still = Some(0);
        let mut m = Match::new(cfg, 3);
        m.step(&[Action {
            throttle: 1.0,
            fire: true,
            ..Default::default()
        }]);
        assert_eq!(m.tanks()[0].vel, Vec2::ZERO);
        assert_eq!(m.projectiles()[0].vel, angle::dir(0) * (360.0 * DT));
    }

    #[test]
    fn unset_new_fields_are_omitted_from_json() {
        // Keeps default configs serializing exactly as before, so their setup hashes
        // (and format 3 replays) are unchanged.
        let json = serde_json::to_string(&MatchConfig::duel()).unwrap();
        assert!(!json.contains("projectile_spread_still"), "{json}");
        assert_eq!(json.matches("\"params\"").count(), 1, "{json}");
        let back: MatchConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back, MatchConfig::duel());
    }

    // --- Engine ask #2: `los` and `max_hp` on TankObs ---

    #[test]
    fn observation_reports_max_hp_and_los() {
        // 0 observes. 1 is hidden behind the pillar. 2 is in the open. 3 (an ally)
        // stands straight behind 2: tanks don't block line of sight.
        let mut cfg = empty_config(vec![
            at(0, 50.0, 200.0, 0),
            with_params(
                at(1, 350.0, 200.0, 0),
                TankParams {
                    max_hp: 140,
                    ..Default::default()
                },
            ),
            at(1, 150.0, 50.0, 0),
            with_params(
                at(0, 250.0, 50.0, 0),
                TankParams {
                    max_hp: 60,
                    ..Default::default()
                },
            ),
        ]);
        cfg.tanks[0].params = Some(TankParams {
            max_hp: 80,
            ..Default::default()
        });
        cfg.tanks[2].pos = Some(Vec2::new(150.0, 125.0));
        cfg.arena = cfg
            .arena
            .with_obstacle(Rect::new(Vec2::new(200.0, 150.0), Vec2::new(220.0, 250.0)));
        let m = Match::new(cfg, 1);
        let o = m.observe(0);
        assert_eq!((o.me.hp, o.me.max_hp), (80, 80));
        let e: Vec<_> = o.enemies.iter().map(|e| (e.id, e.max_hp, e.los)).collect();
        assert_eq!(e, vec![(2, 100, true), (1, 140, false)]);
        let a: Vec<_> = o.allies.iter().map(|a| (a.id, a.max_hp, a.los)).collect();
        assert_eq!(a, vec![(3, 60, true)]);
        // Tank 2 is on the segment from 0 to 3, and still does not block it.
        let (p0, p2, p3) = (m.tanks()[0].pos, m.tanks()[2].pos, m.tanks()[3].pos);
        let d = p3 - p0;
        assert!(crate::arena::segment_circle_entry(p0, d, p2, 16.0).is_some());
        // LOS is symmetric here: 1 cannot see 0 either.
        assert!(!m.observe(1).enemies.iter().find(|e| e.id == 0).unwrap().los);
    }

    type BoxedPolicy = Box<dyn FnMut(&Observation) -> Action>;

    /// Plays a whole match through `TankRules` directly, counting the heap allocations
    /// of `step` and `outcome` only (observations and policies are outside the count).
    /// The buffers are pre-grown, so any allocation is a per-tick one.
    fn step_and_outcome_allocations(config: &MatchConfig, seed: u64) -> (u64, u32, u64) {
        let mut rng = MatchRng::new(seed);
        let mut state = TankRules::init(config, &mut rng);
        let n = state.tanks.len();
        state.projectiles.reserve(64);
        state.scratch.moves.reserve(n);
        let mut events: Vec<Event> = Vec::with_capacity(64);
        let mut policies: Vec<BoxedPolicy> = (0..n)
            .map(|i| -> BoxedPolicy {
                if i % 2 == 0 {
                    Box::new(crate::testing::hunter())
                } else {
                    Box::new(crate::testing::drifter(seed + i as u64))
                }
            })
            .collect();
        let mut actions = vec![Action::default(); n];
        let (mut allocations, mut tick, mut shots) = (0, 0, 0);
        loop {
            for (i, a) in actions.iter_mut().enumerate() {
                *a = if state.tanks[i].alive {
                    TankRules::sanitize(policies[i](&TankRules::observe(config, &state, i, tick)))
                } else {
                    Action::default()
                };
            }
            let (over, count) = crate::testing::allocations_in(|| {
                events.clear();
                TankRules::step(config, &mut state, &actions, &mut rng, &mut events);
                TankRules::outcome(config, &state, tick + 1).is_some()
            });
            allocations += count;
            shots += events
                .iter()
                .filter(|e| matches!(e, Event::Fired { .. }))
                .count() as u64;
            tick += 1;
            if over {
                return (allocations, tick, shots);
            }
        }
    }

    #[test]
    fn step_and_outcome_allocate_nothing_per_tick() {
        // The counter sees an allocation when there is one.
        let (v, count) = crate::testing::allocations_in(|| std::hint::black_box(vec![1u8]));
        assert_eq!((v.len(), count), (1, 1));
        let mut two_v_two = MatchConfig::duel();
        two_v_two.tanks = (0..4)
            .map(|i| TankSpawn {
                team: i % 2,
                ..Default::default()
            })
            .collect();
        two_v_two.max_ticks = 2000;
        for (config, seed) in [
            (MatchConfig::duel(), 0),
            (MatchConfig::duel(), 42),
            (two_v_two, 7),
        ] {
            let (allocations, ticks, shots) = step_and_outcome_allocations(&config, seed);
            assert!(
                ticks > 100 && shots > 5,
                "a real match: {ticks} ticks, {shots} shots"
            );
            assert_eq!(allocations, 0, "{} tanks, seed {seed}", config.tanks.len());
        }
    }

    #[test]
    fn step_scratch_is_not_state() {
        let mut m = Match::new(MatchConfig::duel(), 3);
        let fresh = m.state().clone();
        m.step(&[drive(), drive()]);
        let mut stepped = m.state().clone();
        assert_ne!(stepped, fresh);
        // A clone starts with empty scratch and still equals the original.
        assert!(stepped.scratch.moves.is_empty());
        assert_eq!(&stepped, m.state());
        stepped.scratch.moves.push((Vec2::ONE, Vec2::ONE));
        assert_eq!(&stepped, m.state());
        assert!(!format!("{stepped:?}").contains("scratch"));
    }
}
