//! The car setup, racing's "loadout" (racing.md "Car setup"): Power, Top speed and
//! Grip, each level 1–5, exactly 9 points in total, so the same 19 valid setups as the
//! tanks. Levels map onto [`CarParams`], which is what the config stores.

use crate::config::CarParams;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Lowest level of a stat.
pub const MIN_LEVEL: u8 = 1;
/// Highest level of a stat.
pub const MAX_LEVEL: u8 = 5;
/// Points every car spends across the three stats.
pub const BUDGET: u8 = 9;
/// Stat keys, in order (the Nyborg library and catalog use these).
pub const STAT_KEYS: [&str; 3] = ["power", "top_speed", "grip"];

/// Acceleration (u/s²) by Power level 1..=5.
pub const POWER: [f32; 5] = [180.0, 210.0, 240.0, 280.0, 320.0];
/// Top speed (u/s) by Top speed level 1..=5.
pub const TOP_SPEED: [f32; 5] = [200.0, 220.0, 240.0, 255.0, 270.0];
/// Share of the sideways speed removed per tick, by Grip level 1..=5.
pub const GRIP: [f32; 5] = [0.15, 0.20, 0.25, 0.30, 0.35];

/// A valid setup: each stat in `1..=5`, summing to [`BUDGET`]. Text form `P-T-G`,
/// e.g. `3-3-3`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Setup {
    /// Power level.
    pub power: u8,
    /// Top speed level.
    pub top_speed: u8,
    /// Grip level.
    pub grip: u8,
}

/// Why three levels are not a setup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetupError {
    /// A stat is outside `1..=5`.
    LevelOutOfRange {
        /// The stat key.
        stat: &'static str,
        /// The level given.
        level: u8,
    },
    /// The levels don't sum to [`BUDGET`].
    WrongTotal(u32),
    /// Text isn't `P-T-G`.
    Syntax(String),
}

impl fmt::Display for SetupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LevelOutOfRange { stat, level } => {
                write!(f, "{stat} level {level} is outside {MIN_LEVEL}..={MAX_LEVEL}")
            }
            Self::WrongTotal(t) => write!(f, "stats sum to {t}, must be exactly {BUDGET}"),
            Self::Syntax(s) => write!(f, "setup {s:?} is not P-T-G (e.g. 3-3-3)"),
        }
    }
}

impl std::error::Error for SetupError {}

impl Setup {
    /// The middle setup, 3/3/3.
    pub const BALANCED: Setup = Setup {
        power: 3,
        top_speed: 3,
        grip: 3,
    };

    /// A checked setup.
    pub fn new(power: u8, top_speed: u8, grip: u8) -> Result<Self, SetupError> {
        for (stat, level) in STAT_KEYS.iter().zip([power, top_speed, grip]) {
            if !(MIN_LEVEL..=MAX_LEVEL).contains(&level) {
                return Err(SetupError::LevelOutOfRange { stat, level });
            }
        }
        let total = power as u32 + top_speed as u32 + grip as u32;
        if total != BUDGET as u32 {
            return Err(SetupError::WrongTotal(total));
        }
        Ok(Self {
            power,
            top_speed,
            grip,
        })
    }

    /// All 19 valid setups, ordered by (power, top speed, grip).
    pub fn all() -> Vec<Setup> {
        let mut v = Vec::with_capacity(19);
        for p in MIN_LEVEL..=MAX_LEVEL {
            for t in MIN_LEVEL..=MAX_LEVEL {
                if let Ok(s) = Setup::new(p, t, BUDGET.saturating_sub(p + t)) {
                    v.push(s);
                }
            }
        }
        v
    }

    /// The car params for this setup (the level tables).
    pub fn params(&self) -> CarParams {
        CarParams {
            power: POWER[(self.power - 1) as usize],
            top_speed: TOP_SPEED[(self.top_speed - 1) as usize],
            grip: GRIP[(self.grip - 1) as usize],
        }
    }
}

impl Default for Setup {
    fn default() -> Self {
        Self::BALANCED
    }
}

impl fmt::Display for Setup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}-{}", self.power, self.top_speed, self.grip)
    }
}

impl std::str::FromStr for Setup {
    type Err = SetupError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('-').collect();
        let bad = || SetupError::Syntax(s.to_string());
        if parts.len() != 3 {
            return Err(bad());
        }
        let n = |i: usize| parts[i].parse::<u8>().map_err(|_| bad());
        Setup::new(n(0)?, n(1)?, n(2)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nineteen_setups_on_the_tank_budget() {
        let all = Setup::all();
        assert_eq!(all.len(), 19);
        let mut brute = 0;
        for p in 0..=6u8 {
            for t in 0..=6u8 {
                for g in 0..=6u8 {
                    if Setup::new(p, t, g).is_ok() {
                        brute += 1;
                    }
                }
            }
        }
        assert_eq!(brute, 19);
        assert!(all.contains(&Setup::BALANCED));
        assert_eq!(Setup::new(5, 3, 2), Err(SetupError::WrongTotal(10)));
        assert!(matches!(
            Setup::new(6, 2, 1),
            Err(SetupError::LevelOutOfRange { stat: "power", .. })
        ));
        assert_eq!("2-5-2".parse::<Setup>().unwrap().to_string(), "2-5-2");
        assert!("2-5".parse::<Setup>().is_err());
    }

    #[test]
    fn level_tables_are_the_spec_starting_values() {
        let p = Setup::BALANCED.params();
        assert_eq!((p.power, p.top_speed, p.grip), (240.0, 240.0, 0.25));
        assert_eq!(STAT_KEYS, ["power", "top_speed", "grip"]);
        // Fastest setup: 4.5 u/tick, under the car radius of 12 (no tunnelling).
        assert_eq!(TOP_SPEED[4] / 60.0, 4.5);
    }
}
