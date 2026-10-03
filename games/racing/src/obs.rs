//! The rich observation for scripted policies (racing.md "Observation and action").
//! Fixed-size, so observing allocates nothing; [`crate::flat`] encodes it into the
//! 43-float view.

use crate::config::{CarParams, RacingConfig, MAX_CARS};
use crate::rules::RaceState;
use engine::angle::{dir, from_degrees, Heading};
use engine::Vec2;

/// Ray angles from the heading, in degrees (negative = clockwise, to the right).
pub const RAY_DEGREES: [i32; 9] = [-90, -60, -35, -15, 0, 15, 35, 60, 90];
/// Ray length cap (u): a ray reads this when no wall is within range.
pub const RAY_RANGE: f32 = 300.0;
/// Opponent slots in the observation (nearest first).
pub const OPPONENT_SLOTS: usize = MAX_CARS - 1;

/// Another car still racing, as seen by the observer.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OpponentObs {
    /// Its car index.
    pub car: u8,
    /// Position (world).
    pub pos: Vec2,
    /// Velocity (world).
    pub vel: Vec2,
    /// Race progress.
    pub progress: f32,
    /// Its place is better than the observer's.
    pub ahead: bool,
}

/// What one car sees on one tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Observation {
    /// Observer's car index.
    pub car: usize,
    /// Number of cars in the race.
    pub cars: usize,
    /// Observer's params.
    pub params: CarParams,
    /// Position (world).
    pub pos: Vec2,
    /// Velocity (world).
    pub vel: Vec2,
    /// Heading (BAU).
    pub heading: Heading,
    /// Speed along the heading (u/s; negative when reversing).
    pub speed: f32,
    /// Sideways speed, positive to the left (u/s).
    pub slide: f32,
    /// Next gate index.
    pub next_gate: usize,
    /// Laps completed.
    pub laps: u32,
    /// Laps to finish.
    pub laps_total: u32,
    /// Race progress (gates passed + fraction).
    pub progress: f32,
    /// Current place (1 = leading).
    pub place: u8,
    /// Whether the observer has crossed the start line.
    pub started: bool,
    /// Whether the observer has finished.
    pub finished: bool,
    /// Wall distance along each of [`RAY_DEGREES`], capped at [`RAY_RANGE`].
    pub rays: [f32; 9],
    /// Other cars still racing, nearest first (ties: lower car index); `None` = empty.
    pub opponents: [Option<OpponentObs>; OPPONENT_SLOTS],
    /// Ticks simulated so far.
    pub tick: u32,
    /// The race's tick cap.
    pub max_ticks: u32,
}

impl Observation {
    /// Build `agent`'s observation from the state.
    pub fn new(config: &RacingConfig, state: &RaceState, agent: usize, tick: u32) -> Self {
        let me = &state.cars[agent];
        let d = dir(me.heading);
        let mut rays = [0.0; 9];
        for (r, deg) in rays.iter_mut().zip(RAY_DEGREES) {
            let rd = dir(me.heading.wrapping_add(from_degrees(deg)));
            *r = state.geom.ray(me.pos, rd, RAY_RANGE);
        }
        // Nearest first, ties to the lower index: insertion into a fixed array.
        let mut cand: [(f32, usize); MAX_CARS] = [(f32::INFINITY, usize::MAX); MAX_CARS];
        let mut k = 0;
        for (j, c) in state.cars.iter().enumerate() {
            if j == agent || c.finished() {
                continue;
            }
            let d2 = (c.pos - me.pos).length_squared();
            let mut at = k;
            while at > 0 && cand[at - 1].0 > d2 {
                cand[at] = cand[at - 1];
                at -= 1;
            }
            cand[at] = (d2, j);
            k += 1;
        }
        let mut opponents = [None; OPPONENT_SLOTS];
        for (slot, &(_, j)) in opponents.iter_mut().zip(cand.iter().take(k)) {
            let c = &state.cars[j];
            *slot = Some(OpponentObs {
                car: j as u8,
                pos: c.pos,
                vel: c.vel,
                progress: c.progress,
                ahead: c.place < me.place,
            });
        }
        Self {
            car: agent,
            cars: state.cars.len(),
            params: config.cars[agent],
            pos: me.pos,
            vel: me.vel,
            heading: me.heading,
            speed: me.vel.dot(d),
            slide: me.vel.dot(d.perp()),
            next_gate: me.next_gate as usize,
            laps: me.laps,
            laps_total: config.laps,
            progress: me.progress,
            place: me.place,
            started: me.started(),
            finished: me.finished(),
            rays,
            opponents,
            tick,
            max_ticks: config.max_ticks,
        }
    }

    /// `v` (world) in this car's frame: (forward, left).
    #[inline]
    pub fn to_car_frame(&self, v: Vec2) -> Vec2 {
        let d = dir(self.heading);
        Vec2::new(v.dot(d), v.dot(d.perp()))
    }
}
