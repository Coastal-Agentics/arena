//! Tank-only observation/action interface. Sports will get its own (or a generalized) pair later.

use crate::angle::Heading;
use crate::arena::Rect;
use glam::Vec2;
use serde::{Deserialize, Serialize};

/// What a tank controller outputs each tick. Analog fields are clamped to `[-1, 1]` by the sim.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Action {
    /// Forward (+) / reverse (-) drive, fraction of max speed.
    pub throttle: f32,
    /// Hull turn: + counter-clockwise, - clockwise, fraction of max turn rate.
    pub turn: f32,
    /// Turret turn relative to the world, same sign convention as `turn`.
    pub turret_turn: f32,
    /// Fire the main gun if the cooldown allows.
    pub fire: bool,
}

impl Action {
    /// Copy with analog fields clamped to `[-1, 1]` (NaN becomes 0).
    pub fn clamped(self) -> Self {
        fn c(v: f32) -> f32 {
            if v.is_nan() {
                0.0
            } else {
                v.clamp(-1.0, 1.0)
            }
        }
        Self {
            throttle: c(self.throttle),
            turn: c(self.turn),
            turret_turn: c(self.turret_turn),
            fire: self.fire,
        }
    }
}

/// The observing tank's own state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SelfObs {
    pub id: usize,
    pub team: u8,
    pub pos: Vec2,
    pub vel: Vec2,
    pub heading: Heading,
    pub turret: Heading,
    pub hp: i32,
    pub max_hp: i32,
    /// Ticks until the gun can fire again (0 = ready).
    pub cooldown: u32,
    pub radius: f32,
}

/// Another tank as seen by the observer. Lists are sorted nearest first (ties by id).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TankObs {
    pub id: usize,
    pub team: u8,
    pub pos: Vec2,
    /// `pos - me.pos`.
    pub rel: Vec2,
    /// `rel.length_squared()`.
    pub dist_sq: f32,
    pub vel: Vec2,
    pub heading: Heading,
    pub turret: Heading,
    pub hp: i32,
}

/// A projectile in flight. Sorted nearest first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectileObs {
    pub pos: Vec2,
    pub rel: Vec2,
    pub dist_sq: f32,
    pub vel: Vec2,
    pub owner_team: u8,
}

/// Distances from the observer's centre to each arena edge.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WallObs {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
}

/// Everything a tank policy sees on one tick.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub tick: u32,
    pub me: SelfObs,
    /// Living enemy tanks, nearest first, at most `MAX_OBSERVED_TANKS`.
    pub enemies: Vec<TankObs>,
    /// Living teammates (excluding self), nearest first, at most `MAX_OBSERVED_TANKS`.
    pub allies: Vec<TankObs>,
    /// Projectiles in flight, nearest first, at most `MAX_OBSERVED_PROJECTILES`.
    pub projectiles: Vec<ProjectileObs>,
    pub walls: WallObs,
    pub arena_size: Vec2,
    pub obstacles: Vec<Rect>,
}

/// Cap on enemies/allies listed in an observation.
pub const MAX_OBSERVED_TANKS: usize = 4;
/// Cap on projectiles listed in an observation.
pub const MAX_OBSERVED_PROJECTILES: usize = 8;

/// A tank controller. Must be deterministic given its own state and the observations
/// (use a seeded RNG inside the policy if it needs randomness).
pub trait Policy {
    fn act(&mut self, obs: &Observation) -> Action;
}

impl<F: FnMut(&Observation) -> Action> Policy for F {
    fn act(&mut self, obs: &Observation) -> Action {
        self(obs)
    }
}
