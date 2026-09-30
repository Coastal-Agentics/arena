//! Tank-only observation/action interface. Sports will get its own (or a generalized) pair later.
//!
//! Each tick a [`Policy`] receives an [`Observation`] for its tank (built by
//! [`crate::Match::observe`]) and returns an [`Action`]. The sim clamps the action and
//! records it in the match history, which is what a [`crate::Replay`] stores.

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
    /// This tank's id.
    pub id: usize,
    /// This tank's team.
    pub team: u8,
    /// Position (circle centre).
    pub pos: Vec2,
    /// Velocity actually applied on the last tick, in units per tick.
    pub vel: Vec2,
    /// Hull heading.
    pub heading: Heading,
    /// Turret heading (world frame).
    pub turret: Heading,
    /// Current hit points.
    pub hp: i32,
    /// [`crate::TankParams::max_hp`].
    pub max_hp: i32,
    /// Ticks until the gun can fire again (0 = ready).
    pub cooldown: u32,
    /// [`crate::TankParams::radius`].
    pub radius: f32,
}

/// Another tank as seen by the observer. Lists are sorted nearest first (ties by id).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TankObs {
    /// The other tank's id.
    pub id: usize,
    /// The other tank's team.
    pub team: u8,
    /// Its position.
    pub pos: Vec2,
    /// `pos - me.pos`.
    pub rel: Vec2,
    /// `rel.length_squared()`.
    pub dist_sq: f32,
    /// Its velocity on the last tick, in units per tick.
    pub vel: Vec2,
    /// Its hull heading.
    pub heading: Heading,
    /// Its turret heading (world frame).
    pub turret: Heading,
    /// Its hit points.
    pub hp: i32,
}

/// A projectile in flight (any team, including the observer's own shots).
/// Sorted nearest first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectileObs {
    /// Its position.
    pub pos: Vec2,
    /// `pos - me.pos`.
    pub rel: Vec2,
    /// `rel.length_squared()`.
    pub dist_sq: f32,
    /// Its velocity in units per tick.
    pub vel: Vec2,
    /// Team of the tank that fired it.
    pub owner_team: u8,
}

/// Distances from the observer's centre to each arena edge.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WallObs {
    /// `pos.x` (distance to `x = 0`).
    pub left: f32,
    /// `arena_size.x - pos.x`.
    pub right: f32,
    /// `pos.y` (distance to `y = 0`).
    pub bottom: f32,
    /// `arena_size.y - pos.y`.
    pub top: f32,
}

/// Everything a tank policy sees on one tick.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    /// [`crate::Match::tick`] when the observation was built (ticks simulated so far).
    pub tick: u32,
    /// The observing tank.
    pub me: SelfObs,
    /// Living enemy tanks, nearest first, at most `MAX_OBSERVED_TANKS`.
    pub enemies: Vec<TankObs>,
    /// Living teammates (excluding self), nearest first, at most `MAX_OBSERVED_TANKS`.
    pub allies: Vec<TankObs>,
    /// Projectiles in flight, nearest first, at most `MAX_OBSERVED_PROJECTILES`.
    pub projectiles: Vec<ProjectileObs>,
    /// Distances from the observer's centre to the four arena edges.
    pub walls: WallObs,
    /// [`crate::Arena::size`].
    pub arena_size: Vec2,
    /// All arena obstacles (a copy of [`crate::Arena::obstacles`]).
    pub obstacles: Vec<Rect>,
}

/// Cap on enemies/allies listed in an observation.
pub const MAX_OBSERVED_TANKS: usize = 4;
/// Cap on projectiles listed in an observation.
pub const MAX_OBSERVED_PROJECTILES: usize = 8;

/// A tank controller. Must be deterministic given its own state and the observations
/// (use a seeded RNG inside the policy if it needs randomness).
///
/// Any `FnMut(&Observation) -> Action` closure is a policy:
///
/// ```
/// use engine::{Action, Match, MatchConfig, Observation};
///
/// let mut sit = |_: &Observation| Action::default();
/// let mut spin = |_: &Observation| Action { turn: 1.0, ..Default::default() };
/// let mut m = Match::new(MatchConfig::duel(), 1);
/// let o = m.run(&mut [&mut sit, &mut spin]);
/// assert_eq!(o.winner, None); // nobody shoots: draw at the tick limit
/// ```
pub trait Policy {
    /// Choose this tick's action from the observation.
    fn act(&mut self, obs: &Observation) -> Action;
}

impl<F: FnMut(&Observation) -> Action> Policy for F {
    fn act(&mut self, obs: &Observation) -> Action {
        self(obs)
    }
}
