//! [`RacingRules`]: the race state, car physics, collisions, gates and laps, placings,
//! the end of a race, the hash and the reward (racing.md "Cars", "How a race ends",
//! "Scoring", "Determinism and replays").
//!
//! # Order inside one step
//! 1. Every car stores its progress as last tick's progress.
//! 2. Each car that is still racing drives with its action (a finished car drives with
//!    the default action): steer, then throttle on the forward speed, then grip on the
//!    sideways speed, then the speed cap, then it moves. Cars are processed in index
//!    order, but none reads another car here.
//! 3. Collisions, up to [`COLLISION_PASSES`] passes, each one: car pairs (all pairs
//!    from the same positions, applied together; each car moves half the overlap,
//!    except that a car already pushed off a wall this tick doesn't move back into
//!    that wall, and the other car takes that part), then walls (each car against every
//!    wall segment in order). Finished cars are ghosts: they still hit walls but not
//!    cars. A car moving into a wall loses that speed; the 40% loss along the wall
//!    applies at most once per car per tick.
//! 4. Each racing car's move (old position → new position) is tested against its next
//!    gate only.
//! 5. The tick counter, progress and places are updated.
//!
//! No random draws happen during a race. The RNG is used once, in `init`, to shuffle
//! the grid unless the config fixes it.

use crate::config::{CarParams, Physics, RacingConfig, MAX_CARS};
use crate::end::RaceEnd;
use crate::track::TrackGeom;
use engine::angle::{dir, Heading};
use engine::generic::{MatchRng, Outcome, Rules, StateHasher};
use engine::{Vec2, DT};
use serde::{Deserialize, Serialize};

/// Collision passes per tick (car pairs then walls, repeated until nothing overlaps).
pub const COLLISION_PASSES: usize = 64;
/// Overlapping cars are pushed apart to `2 × radius + CONTACT_SKIN`, so float rounding
/// can't leave them a hair inside each other.
pub const CONTACT_SKIN: f32 = 0.01;

/// One car's controls for one tick, each in [−1, 1]. Positive steer turns
/// counter-clockwise; negative throttle brakes, then reverses once stopped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RaceAction {
    /// −1 (full brake / reverse) to 1 (full throttle).
    pub throttle: f32,
    /// −1 (full right, clockwise) to 1 (full left, counter-clockwise).
    pub steer: f32,
}

/// One car's dynamic state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Car {
    /// Position (u).
    pub pos: Vec2,
    /// Velocity (u/s).
    pub vel: Vec2,
    /// Heading (BAU).
    pub heading: Heading,
    /// Grid slot it started from.
    pub slot: u8,
    /// Index of the next gate to cross.
    pub next_gate: u32,
    /// Forward gate crossings that counted, the start crossing of gate 0 included.
    pub crossings: u32,
    /// Laps completed.
    pub laps: u32,
    /// Tick count at which the car finished, if it has.
    pub finish_tick: Option<u32>,
    /// Race progress now (see [`RacingRules::progress`]).
    pub progress: f32,
    /// Race progress at the end of the previous tick (the reward's baseline).
    pub last_progress: f32,
    /// Current place (1 = leading); equal values share a place.
    pub place: u8,
}

impl Car {
    /// Whether the car has crossed the start line.
    pub fn started(&self) -> bool {
        self.crossings > 0
    }
    /// Whether the car has finished (it is then inactive and a ghost).
    pub fn finished(&self) -> bool {
        self.finish_tick.is_some()
    }
    /// Gates passed since the start crossing.
    pub fn gates_passed(&self) -> u32 {
        self.crossings.saturating_sub(1)
    }
}

/// The race state: cars, the derived track geometry, and the tick.
#[derive(Clone, Debug, PartialEq)]
pub struct RaceState {
    /// One per car, indexed like the config's cars.
    pub cars: Vec<Car>,
    /// Walls, gates and arc lengths, derived from the config's track at the start.
    pub geom: TrackGeom,
    /// Ticks simulated so far.
    pub tick: u32,
    /// Tick count when the first car finished.
    pub first_finish: Option<u32>,
}

/// Something that happened during the last step (for viewers; never in the hash).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RaceEvent {
    /// The car crossed the start line (its race began).
    Started {
        /// Car index.
        car: u8,
    },
    /// The car crossed its next gate.
    Gate {
        /// Car index.
        car: u8,
        /// Gate index.
        gate: u8,
    },
    /// The car completed a lap.
    Lap {
        /// Car index.
        car: u8,
        /// Laps completed.
        lap: u32,
    },
    /// The car finished.
    Finished {
        /// Car index.
        car: u8,
        /// Its place.
        place: u8,
    },
    /// The car hit a wall (once per car per tick).
    Wall {
        /// Car index.
        car: u8,
    },
    /// Two cars touched (once per pair per tick).
    Contact {
        /// Lower car index.
        a: u8,
        /// Higher car index.
        b: u8,
    },
}

/// Racing's rules: game id [`crate::GAME`], rules version [`crate::RULES_VERSION`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RacingRules {}

impl RaceState {
    /// The state at tick 0 with car `i` on grid slot `slots[i]` (no RNG).
    ///
    /// # Panics
    /// If the config is invalid ([`RacingConfig::validate`]) or `slots` doesn't have
    /// one valid slot per car.
    pub fn start(config: &RacingConfig, slots: &[u8]) -> Self {
        if let Err(e) = config.validate() {
            panic!("{e}");
        }
        assert_eq!(slots.len(), config.cars.len(), "one grid slot per car");
        let geom = TrackGeom::new(&config.track);
        let cars = slots
            .iter()
            .map(|&s| {
                let p = config.track.grid[s as usize];
                Car {
                    pos: Vec2::new(p[0], p[1]),
                    vel: Vec2::ZERO,
                    heading: config.track.grid_heading,
                    slot: s,
                    next_gate: 0,
                    crossings: 0,
                    laps: 0,
                    finish_tick: None,
                    progress: 0.0,
                    last_progress: 0.0,
                    place: 1,
                }
            })
            .collect();
        let mut s = Self {
            cars,
            geom,
            tick: 0,
            first_finish: None,
        };
        RacingRules::update_progress(config, &mut s);
        for c in &mut s.cars {
            c.last_progress = c.progress;
        }
        s
    }
}

impl RacingRules {
    /// Race progress of a car: gates passed plus the fraction of the way to the next
    /// gate (projected onto that centreline segment, clamped 0–1). 0 before the start
    /// line; exactly `laps × gates` once finished. A function of the state only.
    pub fn progress(config: &RacingConfig, geom: &TrackGeom, car: &Car) -> f32 {
        let total = config.laps * geom.gate_count() as u32;
        if car.finished() {
            return total as f32;
        }
        if !car.started() {
            return 0.0;
        }
        car.gates_passed() as f32 + geom.fraction_to(car.next_gate as usize, car.pos)
    }

    /// Steering factor k(|u|): 0 at a standstill, 1 at `full_steer_speed`, then down
    /// linearly to `top_speed_steer` at the car's top speed.
    #[inline]
    pub fn steer_factor(speed: f32, params: &CarParams, ph: &Physics) -> f32 {
        if speed <= ph.full_steer_speed {
            speed / ph.full_steer_speed
        } else {
            let f =
                ((speed - ph.full_steer_speed) / (params.top_speed - ph.full_steer_speed)).min(1.0);
            1.0 - (1.0 - ph.top_speed_steer) * f
        }
    }

    /// One car's drive for one tick (steps 2 of the step order), without collisions.
    pub fn drive(car: &mut Car, a: RaceAction, params: &CarParams, ph: &Physics) {
        let d0 = dir(car.heading);
        let k = Self::steer_factor(car.vel.dot(d0).abs(), params, ph);
        let delta = (a.steer * ph.turn_rate as f32 * k) as i16;
        car.heading = car.heading.wrapping_add_signed(delta);
        let d = dir(car.heading);
        let l = d.perp();
        let mut u = car.vel.dot(d);
        let mut w = car.vel.dot(l);
        let top = params.top_speed;
        if a.throttle > 0.0 {
            if u < top {
                u = (u + a.throttle * params.power * DT).min(top);
            } else {
                u = (u - ph.coast * DT).max(top);
            }
        } else if a.throttle < 0.0 {
            let t = -a.throttle;
            if u > 0.0 {
                u = (u - t * ph.brake * DT).max(0.0);
            } else if u > -ph.reverse_speed {
                u = (u - t * params.power * DT).max(-ph.reverse_speed);
            } else {
                u = (u + ph.coast * DT).min(-ph.reverse_speed);
            }
        } else if u > 0.0 {
            u = (u - ph.coast * DT).max(0.0);
        } else {
            u = (u + ph.coast * DT).min(0.0);
        }
        w *= 1.0 - params.grip;
        let mut v = d * u + l * w;
        let sp2 = v.length_squared();
        if sp2 > top * top {
            v *= top / sp2.sqrt();
        }
        car.vel = v;
        car.pos += v * DT;
    }

    /// Step 3: collisions between racing cars and against the walls.
    fn collide(config: &RacingConfig, state: &mut RaceState, events: &mut Vec<RaceEvent>) {
        let ph = &config.physics;
        let r = ph.radius;
        let n = state.cars.len();
        let inner_walls = state.geom.gate_count();
        let mut scraped = [false; MAX_CARS];
        let mut slowed = [false; MAX_CARS];
        // The last wall normal each car was pushed off this tick.
        let mut pinned: [Option<Vec2>; MAX_CARS] = [None; MAX_CARS];
        let mut touched = [[false; MAX_CARS]; MAX_CARS];
        for _ in 0..COLLISION_PASSES {
            let mut any = false;
            let mut dp = [Vec2::ZERO; MAX_CARS];
            let mut dv = [Vec2::ZERO; MAX_CARS];
            let min = 2.0 * r;
            for i in 0..n {
                for j in i + 1..n {
                    let (a, b) = (&state.cars[i], &state.cars[j]);
                    if a.finished() || b.finished() {
                        continue;
                    }
                    let d = b.pos - a.pos;
                    let dist2 = d.length_squared();
                    if dist2 >= min * min {
                        continue;
                    }
                    let dist = dist2.sqrt();
                    let nrm = if dist > 0.0 { d / dist } else { Vec2::X };
                    // Halfway each. A car already pushed off a wall this tick can't
                    // move back into it: that part of its share goes to the other car.
                    let push = min + CONTACT_SKIN - dist;
                    let mut pi = -nrm * (push * 0.5);
                    if let Some(w) = pinned[i] {
                        pi -= w * pi.dot(w).min(0.0);
                    }
                    let mut pj = nrm * (push * 0.5);
                    if let Some(w) = pinned[j] {
                        pj -= w * pj.dot(w).min(0.0);
                    }
                    let (gi, gj) = (-pi.dot(nrm), pj.dot(nrm));
                    let short = push - gi - gj;
                    if short > 0.0 {
                        match (pinned[i], pinned[j]) {
                            (Some(_), None) => pj += nrm * short,
                            (None, Some(_)) => pi -= nrm * short,
                            _ => {}
                        }
                    }
                    dp[i] += pi;
                    dp[j] += pj;
                    let closing = (a.vel - b.vel).dot(nrm);
                    if closing > 0.0 {
                        dv[i] -= nrm * (0.5 * closing);
                        dv[j] += nrm * (0.5 * closing);
                    }
                    if !touched[i][j] {
                        touched[i][j] = true;
                        events.push(RaceEvent::Contact {
                            a: i as u8,
                            b: j as u8,
                        });
                    }
                    any = true;
                }
            }
            for (i, c) in state.cars.iter_mut().enumerate() {
                c.pos += dp[i];
                c.vel += dv[i];
            }
            for (i, c) in state.cars.iter_mut().enumerate() {
                for (wi, w) in state.geom.walls.iter().enumerate() {
                    let cp = w.closest_point(c.pos);
                    let d = c.pos - cp;
                    let dist2 = d.length_squared();
                    if dist2 >= r * r {
                        continue;
                    }
                    let dist = dist2.sqrt();
                    let nrm = if dist > 0.0 {
                        d / dist
                    } else {
                        // Exactly on the wall line: push to the track side.
                        let p = (w.b - w.a).normalize().perp();
                        if wi < inner_walls {
                            -p
                        } else {
                            p
                        }
                    };
                    c.pos = cp + nrm * r;
                    pinned[i] = Some(nrm);
                    let vn = c.vel.dot(nrm);
                    if vn < 0.0 {
                        c.vel -= nrm * vn;
                        if !slowed[i] {
                            slowed[i] = true;
                            c.vel *= ph.wall_keep;
                        }
                    }
                    if !scraped[i] {
                        scraped[i] = true;
                        events.push(RaceEvent::Wall { car: i as u8 });
                    }
                    any = true;
                }
            }
            if !any {
                break;
            }
        }
    }

    /// Recompute every car's progress and the places.
    fn update_progress(config: &RacingConfig, state: &mut RaceState) {
        for i in 0..state.cars.len() {
            let p = Self::progress(config, &state.geom, &state.cars[i]);
            state.cars[i].progress = p;
        }
        let places = Self::placings(&state.cars);
        for (c, p) in state.cars.iter_mut().zip(places) {
            c.place = p;
        }
    }

    /// Places: finishers by finish tick, then everyone else by progress; equal values
    /// share a place (1, 1, 3). Entry `i` is car `i`'s place; unused entries are 0.
    pub fn placings(cars: &[Car]) -> [u8; MAX_CARS] {
        let ahead = |a: &Car, b: &Car| match (a.finish_tick, b.finish_tick) {
            (Some(x), Some(y)) => x < y,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => a.progress > b.progress,
        };
        let mut out = [0u8; MAX_CARS];
        for (i, a) in cars.iter().enumerate() {
            out[i] = 1 + cars.iter().filter(|b| ahead(b, a)).count() as u8;
        }
        out
    }

    /// Step without the RNG (racing never draws during a race). [`Rules::step`] calls
    /// this; tests can call it on a hand-built state.
    pub fn advance(
        config: &RacingConfig,
        state: &mut RaceState,
        actions: &[RaceAction],
        events: &mut Vec<RaceEvent>,
    ) {
        let n = state.cars.len();
        let mut old = [Vec2::ZERO; MAX_CARS];
        for (i, c) in state.cars.iter_mut().enumerate() {
            c.last_progress = c.progress;
            old[i] = c.pos;
            let a = if c.finished() {
                RaceAction::default()
            } else {
                actions
                    .get(i)
                    .copied()
                    .map(Self::sanitize)
                    .unwrap_or_default()
            };
            Self::drive(c, a, &config.cars[i], &config.physics);
        }
        Self::collide(config, state, events);
        let gates = state.geom.gate_count() as u32;
        let total = config.laps * gates;
        let tick = state.tick + 1;
        let mut finished_now = [false; MAX_CARS];
        for i in 0..n {
            let c = &mut state.cars[i];
            if c.finished() {
                continue;
            }
            let g = state.geom.gates[c.next_gate as usize];
            if !g.crossed_forward(old[i], c.pos) {
                continue;
            }
            let gate = c.next_gate;
            c.crossings += 1;
            c.next_gate = (c.next_gate + 1) % gates;
            if c.crossings == 1 {
                events.push(RaceEvent::Started { car: i as u8 });
                continue;
            }
            events.push(RaceEvent::Gate {
                car: i as u8,
                gate: gate as u8,
            });
            if gate == 0 {
                c.laps += 1;
                events.push(RaceEvent::Lap {
                    car: i as u8,
                    lap: c.laps,
                });
            }
            if c.gates_passed() >= total {
                c.finish_tick = Some(tick);
                finished_now[i] = true;
                state.first_finish.get_or_insert(tick);
            }
        }
        state.tick = tick;
        Self::update_progress(config, state);
        for (i, &f) in finished_now.iter().enumerate().take(n) {
            if f {
                events.push(RaceEvent::Finished {
                    car: i as u8,
                    place: state.cars[i].place,
                });
            }
        }
    }

    /// Why the race is over, if it is, in racing's terms. The tick cap wins over the
    /// finish window (it is still `TickLimit` then).
    pub fn race_end(config: &RacingConfig, state: &RaceState, tick: u32) -> Option<RaceEnd> {
        if tick >= config.max_ticks {
            return Some(RaceEnd::TickLimit);
        }
        let t0 = state.first_finish?;
        let all = state.cars.iter().all(Car::finished);
        (all || tick >= t0.saturating_add(config.finish_window)).then_some(RaceEnd::Finished)
    }

    /// The winner's team (car index): the first finisher; on a shared first place the
    /// lowest car index. `None` if nobody has finished.
    pub fn winner(state: &RaceState) -> Option<u8> {
        let t0 = state.first_finish?;
        state
            .cars
            .iter()
            .position(|c| c.finish_tick == Some(t0))
            .map(|i| i as u8)
    }

    /// The finish bonus for `place` among `cars`: (cars − place) ÷ (cars − 1), 1 when
    /// racing alone.
    pub fn finish_bonus(place: u8, cars: usize) -> f32 {
        if cars <= 1 {
            1.0
        } else {
            (cars - place as usize) as f32 / (cars - 1) as f32
        }
    }
}

impl Rules for RacingRules {
    type Config = RacingConfig;
    type State = RaceState;
    type Action = RaceAction;
    type Observation = crate::obs::Observation;
    type Event = RaceEvent;

    /// Validates the config (panics if invalid), then places the cars: the config's
    /// fixed grid, or the first `cars` slots shuffled by Fisher–Yates with one
    /// `next_u32` per car (from the last car down to car 0; `j = r·(i+1) >> 32`).
    fn init(config: &RacingConfig, rng: &mut MatchRng) -> RaceState {
        if let Err(e) = config.validate() {
            panic!("{e}");
        }
        let n = config.cars.len();
        let slots: Vec<u8> = match &config.grid {
            Some(g) => g.clone(),
            None => {
                let mut s: Vec<u8> = (0..n as u8).collect();
                for i in (0..n).rev() {
                    let r = rng.next_u32() as u64;
                    let j = ((r * (i as u64 + 1)) >> 32) as usize;
                    s.swap(i, j);
                }
                s
            }
        };
        RaceState::start(config, &slots)
    }

    fn agents(state: &RaceState) -> usize {
        state.cars.len()
    }

    fn is_active(state: &RaceState, agent: usize) -> bool {
        state.cars.get(agent).is_some_and(|c| !c.finished())
    }

    /// Clamps both controls to [−1, 1]; NaN becomes 0.
    fn sanitize(a: RaceAction) -> RaceAction {
        let c = |x: f32| if x.is_nan() { 0.0 } else { x.clamp(-1.0, 1.0) };
        RaceAction {
            throttle: c(a.throttle),
            steer: c(a.steer),
        }
    }

    fn step(
        config: &RacingConfig,
        state: &mut RaceState,
        actions: &[RaceAction],
        _rng: &mut MatchRng,
        events: &mut Vec<RaceEvent>,
    ) {
        Self::advance(config, state, actions, events);
    }

    fn observe(
        config: &RacingConfig,
        state: &RaceState,
        agent: usize,
        tick: u32,
    ) -> crate::obs::Observation {
        crate::obs::Observation::new(config, state, agent, tick)
    }

    fn outcome(config: &RacingConfig, state: &RaceState, tick: u32) -> Option<Outcome> {
        Self::race_end(config, state, tick).map(|end| Outcome {
            winner: Self::winner(state),
            ticks: tick,
            reason: end.reason(),
        })
    }

    /// Per car, in index order: position, velocity, heading, gates passed (counted
    /// crossings), laps, finish tick (`u64::MAX` if none) and last tick's progress.
    fn hash_state(state: &RaceState, h: &mut StateHasher) {
        h.write_u64(state.cars.len() as u64);
        for c in &state.cars {
            h.write_u64(c.pos.x.to_bits() as u64);
            h.write_u64(c.pos.y.to_bits() as u64);
            h.write_u64(c.vel.x.to_bits() as u64);
            h.write_u64(c.vel.y.to_bits() as u64);
            h.write_u64(c.heading as u64);
            h.write_u64(c.crossings as u64);
            h.write_u64(c.laps as u64);
            h.write_u64(c.finish_tick.map_or(u64::MAX, u64::from));
            h.write_u64(c.last_progress.to_bits() as u64);
        }
    }

    /// Racing configs need format 4 or later (format 5 once M3a lands) and must be
    /// valid; the field named is the first invalid one.
    fn check_format(config: &RacingConfig, format: u32) -> Result<(), &'static str> {
        if format < 4 {
            return Err("racing config");
        }
        config.validate().map_err(|e| e.field)
    }

    /// (Progress now − progress last tick) ÷ gates per lap, plus the finish bonus on
    /// the tick the car finishes. Summed over a race it is final progress ÷ gates plus
    /// the bonus. Never recorded.
    fn reward(_config: &RacingConfig, state: &RaceState, _e: &[RaceEvent], agent: usize) -> f32 {
        let c = &state.cars[agent];
        let mut r = (c.progress - c.last_progress) / state.geom.gate_count() as f32;
        if state.tick > 0 && c.finish_tick == Some(state.tick) {
            r += Self::finish_bonus(c.place, state.cars.len());
        }
        r
    }
}
