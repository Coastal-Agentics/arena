//! The race config: track, cars and their params, laps, tick cap and the physics
//! constants. Everything a race needs besides the seed, so `setup_hash` covers the
//! track too (racing.md "Determinism and replays").

use crate::setup::Setup;
use crate::track::Track;
use engine::angle::Heading;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Most cars in one race (the Ring grid has 4 slots; the flat view has 3 opponent slots).
pub const MAX_CARS: usize = 4;

/// One car's params, as stored in the config. [`Setup::params`] maps levels onto these.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CarParams {
    /// Acceleration under full throttle (u/s²).
    pub power: f32,
    /// Top speed (u/s).
    pub top_speed: f32,
    /// Share of the sideways speed removed per tick (0–1).
    pub grip: f32,
}

impl Default for CarParams {
    fn default() -> Self {
        Setup::BALANCED.params()
    }
}

/// Physics shared by every car (racing.md "Cars").
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Physics {
    /// Collider radius (u).
    pub radius: f32,
    /// Turn rate at full steer and full steering factor (BAU/tick).
    pub turn_rate: Heading,
    /// Speed at which the steering factor reaches 1 (u/s); 0 at a standstill.
    pub full_steer_speed: f32,
    /// Steering factor at top speed (it falls linearly from 1 at `full_steer_speed`).
    pub top_speed_steer: f32,
    /// Braking deceleration under full negative throttle while moving forward (u/s²).
    pub brake: f32,
    /// Coasting deceleration at zero throttle (u/s²).
    pub coast: f32,
    /// Top reversing speed (u/s).
    pub reverse_speed: f32,
    /// Share of the speed along a wall kept when scraping it (0.6 = loses 40%).
    pub wall_keep: f32,
}

impl Default for Physics {
    fn default() -> Self {
        Self {
            radius: 12.0,
            turn_rate: 364,
            full_steer_speed: 60.0,
            top_speed_steer: 0.5,
            brake: 480.0,
            coast: 60.0,
            reverse_speed: 60.0,
            wall_keep: 0.6,
        }
    }
}

/// Everything (with the seed) that reproduces a race.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RacingConfig {
    /// The track (Ring by default).
    pub track: Track,
    /// One entry per car; car `i` is agent `i` and team `i`.
    pub cars: Vec<CarParams>,
    /// Shared physics.
    pub physics: Physics,
    /// Laps to finish.
    pub laps: u32,
    /// Tick cap (truncation).
    pub max_ticks: u32,
    /// Ticks after the winner finishes before the race ends anyway.
    pub finish_window: u32,
    /// Fixed grid: `grid[i]` is car `i`'s slot. `None` shuffles the first `cars.len()`
    /// slots with the match RNG (Fisher–Yates, one `next_u32` per car).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid: Option<Vec<u8>>,
}

impl RacingConfig {
    /// A 3-lap Ring race for the given cars, shuffled grid, 3,600-tick cap.
    pub fn ring(cars: Vec<CarParams>) -> Self {
        Self {
            track: Track::ring(),
            cars,
            physics: Physics::default(),
            laps: 3,
            max_ticks: 3600,
            finish_window: 600,
            grid: None,
        }
    }

    /// A Ring race for these setups.
    pub fn ring_setups(setups: &[Setup]) -> Self {
        Self::ring(setups.iter().map(Setup::params).collect())
    }

    /// The same race with a fixed grid (`grid[i]` = car `i`'s slot).
    pub fn with_grid(mut self, grid: Vec<u8>) -> Self {
        self.grid = Some(grid);
        self
    }

    /// Check the config. [`crate::RacingRules`] panics on an invalid config at
    /// `Match::new`, and replays with one are rejected at load (`check_format`).
    pub fn validate(&self) -> Result<(), ConfigError> {
        let err = |field: &'static str, message: String| Err(ConfigError { field, message });
        let ph = &self.physics;
        let pos_finite = |v: f32| v.is_finite() && v > 0.0;
        if !pos_finite(ph.radius) {
            return err(
                "physics.radius",
                format!("{} is not a positive number", ph.radius),
            );
        }
        for (field, v) in [
            ("physics.full_steer_speed", ph.full_steer_speed),
            ("physics.brake", ph.brake),
            ("physics.coast", ph.coast),
            ("physics.reverse_speed", ph.reverse_speed),
        ] {
            if !pos_finite(v) {
                return err(field, format!("{v} is not a positive number"));
            }
        }
        for (field, v) in [
            ("physics.top_speed_steer", ph.top_speed_steer),
            ("physics.wall_keep", ph.wall_keep),
        ] {
            if !(0.0..=1.0).contains(&v) {
                return err(field, format!("{v} is outside 0..=1"));
            }
        }
        if ph.reverse_speed / engine::TICK_HZ as f32 >= ph.radius {
            return err(
                "physics.reverse_speed",
                "reverse speed per tick must be below the radius".to_string(),
            );
        }
        if self.cars.is_empty() || self.cars.len() > MAX_CARS {
            return err(
                "cars",
                format!("{} cars; 1..={MAX_CARS} allowed", self.cars.len()),
            );
        }
        for c in &self.cars {
            if !pos_finite(c.power) {
                return err(
                    "cars[].power",
                    format!("{} is not a positive number", c.power),
                );
            }
            if !pos_finite(c.top_speed) || c.top_speed <= ph.full_steer_speed {
                return err(
                    "cars[].top_speed",
                    format!("{} must be above full_steer_speed", c.top_speed),
                );
            }
            // No tunnelling: a car moves less than its radius per tick.
            if c.top_speed / engine::TICK_HZ as f32 >= ph.radius {
                return err(
                    "cars[].top_speed",
                    format!(
                        "top speed per tick {} is not below the radius {}",
                        c.top_speed / engine::TICK_HZ as f32,
                        ph.radius
                    ),
                );
            }
            if !(0.0..=1.0).contains(&c.grip) {
                return err("cars[].grip", format!("{} is outside 0..=1", c.grip));
            }
        }
        if self.laps == 0 {
            return err("laps", "at least 1 lap".to_string());
        }
        if self.max_ticks == 0 {
            return err("max_ticks", "at least 1 tick".to_string());
        }
        let t = &self.track;
        if t.centreline.len() < 3 {
            return err("track.centreline", "at least 3 points".to_string());
        }
        let finite = |p: &[f32; 2]| p[0].is_finite() && p[1].is_finite();
        if !t.centreline.iter().all(finite) || !t.grid.iter().all(finite) {
            return err("track", "non-finite coordinate".to_string());
        }
        for i in 0..t.centreline.len() {
            let (a, b) = (t.centreline[i], t.centreline[(i + 1) % t.centreline.len()]);
            if a == b {
                return err("track.centreline", format!("points {i} and next coincide"));
            }
        }
        if !(t.width.is_finite() && t.width > 4.0 * ph.radius) {
            return err(
                "track.width",
                format!("{} must exceed two car widths", t.width),
            );
        }
        if !(pos_finite(t.size[0]) && pos_finite(t.size[1])) {
            return err("track.size", "must be positive".to_string());
        }
        if t.grid.len() < self.cars.len() {
            return err("track.grid", "fewer grid slots than cars".to_string());
        }
        if let Some(g) = &self.grid {
            if g.len() != self.cars.len() {
                return err("grid", "one slot per car".to_string());
            }
            for (i, &s) in g.iter().enumerate() {
                if s as usize >= t.grid.len() || g[..i].contains(&s) {
                    return err("grid", format!("slot {s} is out of range or repeated"));
                }
            }
        }
        Ok(())
    }
}

/// An invalid config: the field and why.
#[derive(Clone, Debug, PartialEq)]
pub struct ConfigError {
    /// The offending field, e.g. `"cars[].top_speed"`.
    pub field: &'static str,
    /// What is wrong with it.
    pub message: String,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid racing config: {}: {}", self.field, self.message)
    }
}

impl std::error::Error for ConfigError {}
