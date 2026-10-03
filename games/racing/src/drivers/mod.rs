//! The scripted baselines, Gen 0 for racing (racing.md "Scripted baselines"):
//! [`Follower`], [`Cutter`] and [`Blocker`]. Their params are the genes R4's
//! evolution will tune, as the tank policies' are.
//!
//! All three are a [`Pilot`]: pure pursuit of a point a set distance ahead on a
//! driving [`Line`], and a speed limit from the corners ahead (brake to the corner
//! speed in time). They differ in the line and in what they do about traffic.
//! The seed only feeds each driver's own jitter (a few percent on its look-ahead and
//! corner speed, drawn once when it is built), never the match RNG.

mod line;

pub use line::Line;

use crate::config::RacingConfig;
use crate::obs::Observation;
use crate::rules::{RaceAction, RacingRules};
use crate::track::TrackGeom;
use engine::generic::Policy;
use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

/// Steering gain: steer = gain × sin(angle to the target point), clamped.
pub const STEER_GAIN: f32 = 3.0;
/// How far ahead (u) the drivers look for corners.
pub const CORNER_HORIZON: f32 = 320.0;

/// The three scripted drivers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Behavior {
    /// [`Follower`]: the safe middle.
    Follower,
    /// [`Cutter`]: the racing line, fastest alone.
    Cutter,
    /// [`Blocker`]: steals places in traffic.
    Blocker,
}

impl Behavior {
    /// All three, in catalog order.
    pub const ALL: [Behavior; 3] = [Behavior::Follower, Behavior::Cutter, Behavior::Blocker];

    /// Catalog id (`follower`, `cutter`, `blocker`).
    pub const fn key(self) -> &'static str {
        match self {
            Self::Follower => "follower",
            Self::Cutter => "cutter",
            Self::Blocker => "blocker",
        }
    }

    /// Display name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Follower => "Follower",
            Self::Cutter => "Cutter",
            Self::Blocker => "Blocker",
        }
    }

    /// The behavior for a catalog id.
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|b| b.key() == key)
    }

    /// A driver with default params for car `car`, jittered by `seed`.
    pub fn driver(
        self,
        config: &RacingConfig,
        car: usize,
        seed: u64,
    ) -> Box<dyn Policy<RacingRules>> {
        match self {
            Self::Follower => Box::new(Follower::new(config, FollowerParams::default(), car, seed)),
            Self::Cutter => Box::new(Cutter::new(config, CutterParams::default(), car, seed)),
            Self::Blocker => Box::new(Blocker::new(config, BlockerParams::default(), car, seed)),
        }
    }
}

/// Jitter: one factor in `1 ± amount` per draw, from the driver's own stream.
#[derive(Clone, Debug, PartialEq)]
pub struct Jitter(ChaCha8Rng);

impl Jitter {
    fn new(seed: u64, car: usize) -> Self {
        Self(ChaCha8Rng::seed_from_u64(
            seed ^ (car as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15),
        ))
    }
    fn unit(&mut self) -> f32 {
        (self.0.next_u32() >> 8) as f32 / (1u32 << 24) as f32 // [0, 1)
    }
    fn factor(&mut self, amount: f32) -> f32 {
        1.0 + amount * (2.0 * self.unit() - 1.0)
    }
}

/// Share of the physics brake the drivers plan with (a safety margin).
pub const PLAN_BRAKE: f32 = 0.85;
/// A car on the line closer ahead than this (u) and not pulling away gets passed.
pub const PASS_DISTANCE: f32 = 70.0;
/// Side clearance (u) when passing, beyond two radii.
pub const PASS_CLEARANCE: f32 = 6.0;
/// Jitter on the drivers' look-ahead (±share), drawn once per driver.
pub const LOOK_JITTER: f32 = 0.15;
/// Jitter on the drivers' corner speed (±share), drawn once per driver.
pub const SPEED_JITTER: f32 = 0.12;
/// Corner-to-corner pace variation (±share), drawn per corner.
pub const CORNER_JITTER: f32 = 0.08;
/// Chance per corner of a lift (a small mistake).
pub const MISTAKE_CHANCE: f32 = 0.1;
/// Corner speed factor of a lift.
pub const MISTAKE_SLOW: f32 = 0.8;

/// Line following and corner braking, shared by the three drivers.
#[derive(Clone, Debug, PartialEq)]
pub struct Pilot {
    /// Per-corner pace: drawn from the driver's own stream each time a new corner
    /// comes up ([`CORNER_JITTER`], and a [`MISTAKE_SLOW`] lift with chance
    /// [`MISTAKE_CHANCE`]).
    pub rng: Jitter,
    /// The corner the current pace applies to.
    pub corner: usize,
    /// Current per-corner pace factor.
    pub pace: f32,
    /// The line followed.
    pub line: Line,
    /// Look-ahead along the line (u).
    pub look_ahead: f32,
    /// Speed for a 45° turn of the line (u/s).
    pub corner_speed: f32,
    /// Be at corner speed this far before the corner vertex (u).
    pub brake_margin: f32,
    /// Planning deceleration (u/s²).
    pub decel: f32,
}

impl Pilot {
    /// The sideways offset (u) that passes a slower car close ahead on the line, if
    /// there is one: to whichever side of it has more room, kept within `max_offset`.
    pub fn pass_offset(&self, o: &Observation, offset: f32, max_offset: f32, radius: f32) -> f32 {
        let (s, _) = self.line.project(o.pos);
        for opp in o.opponents.iter().flatten() {
            let rel = o.to_car_frame(opp.pos - o.pos);
            let gaining = o.to_car_frame(o.vel - opp.vel).x > -5.0;
            if rel.x <= -radius || rel.x > PASS_DISTANCE || !gaining {
                continue;
            }
            let (os, lateral) = self.line.project(opp.pos);
            let ahead = (os - s).rem_euclid(self.line.length());
            if ahead > PASS_DISTANCE && ahead < self.line.length() - radius {
                continue;
            }
            let gap = 2.0 * radius + PASS_CLEARANCE;
            if (lateral - offset).abs() >= gap {
                continue; // already clear of our line
            }
            let left = lateral + gap;
            let right = lateral - gap;
            let room_left = max_offset - left;
            let room_right = right + max_offset;
            return if room_left >= room_right {
                left.min(max_offset)
            } else {
                right.max(-max_offset)
            };
        }
        offset
    }

    /// The action that follows the line at `offset` (u, positive = left).
    pub fn drive(&mut self, o: &Observation, offset: f32) -> RaceAction {
        let (s, _) = self.line.project(o.pos);
        let corner = self.line.next_turn(s);
        if corner != self.corner {
            self.corner = corner;
            self.pace = self.rng.factor(CORNER_JITTER);
            if self.rng.unit() < MISTAKE_CHANCE {
                self.pace *= MISTAKE_SLOW;
            }
        }
        let target = self.line.point_at(s + self.look_ahead, offset);
        let rel = o.to_car_frame(target - o.pos);
        let len = rel.length().max(1e-3);
        let steer = if rel.x < 0.0 {
            if rel.y >= 0.0 {
                1.0
            } else {
                -1.0
            }
        } else {
            (STEER_GAIN * rel.y / len).clamp(-1.0, 1.0)
        };
        let limit = self.line.speed_limit(
            s,
            CORNER_HORIZON,
            self.corner_speed * self.pace,
            self.decel,
            self.brake_margin,
        );
        let throttle = if o.speed > limit {
            -1.0
        } else if o.speed > limit - 2.0 {
            0.0
        } else {
            1.0
        };
        RaceAction { throttle, steer }
    }
}

/// [`Follower`] params (genes).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FollowerParams {
    /// Look-ahead along the centreline (u).
    pub look_ahead: f32,
    /// Corner speed for a 45° turn (u/s).
    pub corner_speed: f32,
    /// Braking distance margin: at corner speed this far before the corner (u).
    pub brake_distance: f32,
}

impl Default for FollowerParams {
    fn default() -> Self {
        Self {
            look_ahead: 70.0,
            corner_speed: 163.0,
            brake_distance: 20.0,
        }
    }
}

/// Steers at a point on the centreline a set distance ahead and brakes to a corner
/// speed before each corner.
#[derive(Clone, Debug, PartialEq)]
pub struct Follower {
    /// Its params after jitter.
    pub params: FollowerParams,
    pilot: Pilot,
    max_offset: f32,
    radius: f32,
}

impl Follower {
    /// For car `car` of `config`, jittered by `seed`.
    pub fn new(config: &RacingConfig, params: FollowerParams, car: usize, seed: u64) -> Self {
        let geom = TrackGeom::new(&config.track);
        let mut j = Jitter::new(seed, car);
        let params = FollowerParams {
            look_ahead: params.look_ahead * j.factor(LOOK_JITTER),
            corner_speed: params.corner_speed * j.factor(SPEED_JITTER),
            ..params
        };
        let pilot = Pilot {
            rng: j,
            corner: usize::MAX,
            pace: 1.0,
            line: Line::centreline(&geom),
            look_ahead: params.look_ahead,
            corner_speed: params.corner_speed,
            brake_margin: params.brake_distance,
            decel: config.physics.brake * PLAN_BRAKE,
        };
        Self {
            params,
            pilot,
            max_offset: max_offset(config, &geom),
            radius: config.physics.radius,
        }
    }
}

/// Largest sideways offset from the line a driver takes (u).
fn max_offset(config: &RacingConfig, geom: &TrackGeom) -> f32 {
    geom.half_width - config.physics.radius - 6.0
}

impl Policy<RacingRules> for Follower {
    fn act(&mut self, o: &Observation) -> RaceAction {
        let off = self.pilot.pass_offset(o, 0.0, self.max_offset, self.radius);
        self.pilot.drive(o, off)
    }
}

/// [`Cutter`] params (genes).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CutterParams {
    /// Distance kept from the inner wall at each apex (u, beyond the radius).
    pub apex_margin: f32,
    /// Corner entry speed for a 45° turn of the racing line (u/s).
    pub entry_speed: f32,
    /// How wide the line runs at corner entry and exit (u from the centreline
    /// towards the outer wall).
    pub exit_point: f32,
}

impl Default for CutterParams {
    fn default() -> Self {
        Self {
            apex_margin: 46.0,
            entry_speed: 128.0,
            exit_point: 90.0,
        }
    }
}

/// The racing line: an apex line that runs `apex_margin` off the inner wall at each
/// corner ([`Line::apex`]), looked along `exit_point` ahead, braking later than the
/// Follower (at the corner vertex itself). It doesn't barge: a car just ahead in its
/// path, or alongside, makes it lift ([`CUTTER_GAP`]), and it only pulls out to pass
/// (half the Follower's offset) with [`CUTTER_PASS_ROOM`] of straight ahead.
#[derive(Clone, Debug, PartialEq)]
pub struct Cutter {
    /// Its params after jitter.
    pub params: CutterParams,
    pilot: Pilot,
    max_offset: f32,
    radius: f32,
}

impl Cutter {
    /// For car `car` of `config`, jittered by `seed`.
    pub fn new(config: &RacingConfig, params: CutterParams, car: usize, seed: u64) -> Self {
        let geom = TrackGeom::new(&config.track);
        let mut j = Jitter::new(seed, car);
        let params = CutterParams {
            entry_speed: params.entry_speed * j.factor(SPEED_JITTER),
            ..params
        };
        let look_ahead = params.exit_point * j.factor(LOOK_JITTER);
        let pilot = Pilot {
            rng: j,
            corner: usize::MAX,
            pace: 1.0,
            line: Line::apex(&geom, config.physics.radius + params.apex_margin),
            look_ahead,
            corner_speed: params.entry_speed,
            brake_margin: 0.0,
            decel: config.physics.brake * PLAN_BRAKE,
        };
        Self {
            params,
            pilot,
            max_offset: max_offset(config, &geom),
            radius: config.physics.radius,
        }
    }
}

/// The Cutter won't barge: a car closer ahead than this (u, centre to centre) and in
/// its path makes it lift instead of passing, unless the next corner is at least
/// [`CUTTER_PASS_ROOM`] away (then it passes on the straight).
pub const CUTTER_GAP: f32 = 40.0;
/// Straight needed (u to the next corner) before the Cutter passes.
pub const CUTTER_PASS_ROOM: f32 = 130.0;

impl Policy<RacingRules> for Cutter {
    fn act(&mut self, o: &Observation) -> RaceAction {
        let (s, _) = self.pilot.line.project(o.pos);
        if self.pilot.line.to_next_turn(s) >= CUTTER_PASS_ROOM {
            let off = self
                .pilot
                .pass_offset(o, 0.0, self.max_offset * 0.5, self.radius);
            if off != 0.0 {
                return self.pilot.drive(o, off);
            }
        }
        let mut a = self.pilot.drive(o, 0.0);
        for opp in o.opponents.iter().flatten() {
            let rel = o.to_car_frame(opp.pos - o.pos);
            let closing = o.to_car_frame(o.vel - opp.vel).x;
            if rel.x > -self.radius
                && rel.x < CUTTER_GAP
                && rel.y.abs() < 2.0 * self.radius + PASS_CLEARANCE
                && closing > 0.0
            {
                a.throttle = a.throttle.min(0.0);
            }
        }
        a
    }
}

/// [`Blocker`] params (genes): the Follower's, plus the block.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BlockerParams {
    /// The line and braking it drives.
    pub follower: FollowerParams,
    /// A car behind closer than this (u) and gaining is blocked.
    pub block_distance: f32,
    /// Share (0–1) of that car's sideways offset the Blocker moves across to cover.
    pub block_strength: f32,
}

impl Default for BlockerParams {
    fn default() -> Self {
        Self {
            follower: FollowerParams {
                corner_speed: 164.0,
                ..FollowerParams::default()
            },
            block_distance: 70.0,
            block_strength: 0.8,
        }
    }
}

/// Drives the Follower's line; when a car is close behind and gaining, it moves
/// across to cover that car's line.
#[derive(Clone, Debug, PartialEq)]
pub struct Blocker {
    /// Its params after jitter.
    pub params: BlockerParams,
    pilot: Pilot,
    max_offset: f32,
    radius: f32,
}

impl Blocker {
    /// For car `car` of `config`, jittered by `seed`.
    pub fn new(config: &RacingConfig, params: BlockerParams, car: usize, seed: u64) -> Self {
        let f = Follower::new(config, params.follower, car, seed);
        Self {
            params: BlockerParams {
                follower: f.params,
                ..params
            },
            pilot: f.pilot,
            max_offset: f.max_offset,
            radius: f.radius,
        }
    }

    /// The sideways offset to drive at: cover the nearest car behind that is within
    /// `block_distance`, gaining, and behind in the race.
    fn block_offset(&self, o: &Observation) -> f32 {
        for opp in o.opponents.iter().flatten() {
            let rel = o.to_car_frame(opp.pos - o.pos);
            let closing = o.to_car_frame(opp.vel - o.vel).x;
            if rel.x < 0.0
                && rel.length() < self.params.block_distance
                && closing > 0.0
                && !opp.ahead
            {
                let (_, lateral) = self.pilot.line.project(opp.pos);
                return (lateral * self.params.block_strength)
                    .clamp(-self.max_offset, self.max_offset);
            }
        }
        0.0
    }
}

impl Policy<RacingRules> for Blocker {
    fn act(&mut self, o: &Observation) -> RaceAction {
        let off = match self.block_offset(o) {
            0.0 => self.pilot.pass_offset(o, 0.0, self.max_offset, self.radius),
            b => b,
        };
        self.pilot.drive(o, off)
    }
}
