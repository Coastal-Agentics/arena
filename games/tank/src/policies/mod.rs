//! The three scripted Tank Arena policies (SPEC "Scripted policies"), their params
//! structs (the Phase 3 evolution genome), and a [`Behavior`] picker for configs, the
//! CLI and the viewer.
//!
//! Intended triangle: kiter > charger > sniper > kiter.

pub mod charger;
pub mod common;
pub mod kiter;
pub mod sniper;

pub use charger::{Charger, ChargerParams};
pub use kiter::{Kiter, KiterParams};
pub use sniper::{Sniper, SniperParams};

use engine::Policy;
use std::fmt;
use std::str::FromStr;

/// Which scripted policy drives a tank.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Behavior {
    /// [`Charger`]: the rusher.
    Charger,
    /// [`Kiter`]: the dancer.
    Kiter,
    /// [`Sniper`]: the camper.
    Sniper,
}

impl Behavior {
    /// All behaviors, in picker order.
    pub const ALL: [Behavior; 3] = [Behavior::Charger, Behavior::Kiter, Behavior::Sniper];

    /// Lower-case URL name (`charger`, `kiter`, `sniper`).
    pub fn key(self) -> &'static str {
        match self {
            Self::Charger => "charger",
            Self::Kiter => "kiter",
            Self::Sniper => "sniper",
        }
    }

    /// Display name (`Charger`).
    pub fn name(self) -> &'static str {
        match self {
            Self::Charger => "Charger",
            Self::Kiter => "Kiter",
            Self::Sniper => "Sniper",
        }
    }

    /// How the behavior was made (SPEC "Training indicator"). Every policy is
    /// hand-written today; Phase 3 evolution will report generation and fitness.
    pub fn training(self) -> &'static str {
        "Scripted"
    }

    /// A fresh policy with default params and its timing jitter seeded from `seed`
    /// (see [`crate::MatchSpec::policies`] for how a match derives it).
    pub fn build(self, seed: u64) -> Box<dyn Policy> {
        match self {
            Self::Charger => Box::new(Charger::seeded(ChargerParams::default(), seed)),
            Self::Kiter => Box::new(Kiter::seeded(KiterParams::default(), seed)),
            Self::Sniper => Box::new(Sniper::seeded(SniperParams::default(), seed)),
        }
    }
}

impl fmt::Display for Behavior {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key())
    }
}

impl FromStr for Behavior {
    type Err = String;
    /// Case-insensitive: `charger`, `Kiter`, `SNIPER`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let k = s.trim().to_ascii_lowercase();
        Behavior::ALL
            .into_iter()
            .find(|b| b.key() == k)
            .ok_or_else(|| format!("unknown behavior {s:?} (expected charger, kiter or sniper)"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{config, Mode};
    use engine::{Match, Outcome};

    fn play(a: Behavior, b: Behavior, seed: u64) -> (Outcome, u64) {
        let mut m = Match::new(config(Mode::Duel), seed);
        let (mut pa, mut pb) = (a.build(seed), b.build(seed ^ 1));
        let o = m.run(&mut [pa.as_mut(), pb.as_mut()]);
        (o, m.state_hash())
    }

    #[test]
    fn parse_names() {
        for b in Behavior::ALL {
            assert_eq!(b.key().parse::<Behavior>().unwrap(), b);
            assert_eq!(b.name().parse::<Behavior>().unwrap(), b);
            assert_eq!(b.training(), "Scripted");
        }
        assert!("wanderer".parse::<Behavior>().is_err());
    }

    #[test]
    fn every_pairing_is_deterministic_and_terminates() {
        for a in Behavior::ALL {
            for b in Behavior::ALL {
                let x = play(a, b, 42);
                assert_eq!(x, play(a, b, 42), "{a} vs {b}");
                assert!(x.0.ticks <= 7200);
            }
        }
    }

    #[test]
    fn policies_fight() {
        // Over a few seeds, every non-mirror pairing produces hits and some decisive
        // results: nobody sits in a corner forever.
        for a in Behavior::ALL {
            for b in Behavior::ALL {
                if a == b {
                    continue;
                }
                let decided = (0..6).filter(|&s| play(a, b, s).0.winner.is_some()).count();
                assert!(decided > 0, "{a} vs {b}: all draws");
            }
        }
    }
}
