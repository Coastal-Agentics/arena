//! A shareable duel: seed plus each tank's behavior and loadout, with the URL query
//! format from the spec (SPEC "Customize tab"):
//! `seed=42&blue=kiter-5-3-1&orange=charger-4-1-4`.
//!
//! A tank is `<behavior>-<attack>-<speed>-<defense>`; a bare behavior (`kiter`) means
//! 3/3/3. Missing keys take the defaults below; unknown keys (the viewer's `tab`,
//! `speed`, `t`, `paused`) are ignored. [`MatchSpec::to_query`] always writes the full
//! canonical form, so an encoded link replays exactly.

use crate::loadout::Loadout;
use crate::policies::Behavior;
use crate::rules::{self, Setup};
use engine::{Match, Outcome, Policy};
use std::fmt;
use std::str::FromStr;

/// One tank's behavior and loadout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TankSpec {
    /// Which scripted policy drives it.
    pub behavior: Behavior,
    /// Its stats.
    pub loadout: Loadout,
}

impl TankSpec {
    /// Shorthand constructor.
    pub fn new(behavior: Behavior, loadout: Loadout) -> Self {
        Self { behavior, loadout }
    }
}

impl fmt::Display for TankSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.behavior, self.loadout)
    }
}

impl FromStr for TankSpec {
    type Err = String;
    /// `kiter-5-3-1`, or `kiter` for 3/3/3. Behavior is case-insensitive.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let (b, l) = match s.split_once('-') {
            Some((b, l)) => (b, Some(l)),
            None => (s, None),
        };
        let behavior: Behavior = b.parse()?;
        let loadout = match l {
            Some(l) => l.parse::<Loadout>().map_err(|e| e.to_string())?,
            None => Loadout::DEFAULT,
        };
        Ok(Self { behavior, loadout })
    }
}

/// Everything that determines a Tank Arena duel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MatchSpec {
    /// Match seed (drives shell spread).
    pub seed: u64,
    /// Tank 0, team 0, spawns left.
    pub blue: TankSpec,
    /// Tank 1, team 1, spawns right.
    pub orange: TankSpec,
}

impl Default for MatchSpec {
    /// `seed=42&blue=kiter-3-3-3&orange=charger-3-3-3`.
    fn default() -> Self {
        Self {
            seed: 42,
            blue: TankSpec::new(Behavior::Kiter, Loadout::DEFAULT),
            orange: TankSpec::new(Behavior::Charger, Loadout::DEFAULT),
        }
    }
}

impl MatchSpec {
    /// Parse a URL query (with or without the leading `?`). Values are plain ASCII, so
    /// no percent-decoding is needed; a `%` anywhere in a used value is an error.
    ///
    /// ```
    /// use tank::MatchSpec;
    /// let m = MatchSpec::from_query("?seed=42&blue=kiter-5-3-1&orange=charger-4-1-4").unwrap();
    /// assert_eq!(m.to_query(), "seed=42&blue=kiter-5-3-1&orange=charger-4-1-4");
    /// assert_eq!(MatchSpec::from_query(&m.to_query()).unwrap(), m);
    /// ```
    pub fn from_query(q: &str) -> Result<Self, String> {
        let mut m = Self::default();
        for pair in q
            .trim_start_matches('?')
            .split('&')
            .filter(|p| !p.is_empty())
        {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            match k {
                "seed" => {
                    m.seed = v
                        .trim()
                        .parse()
                        .map_err(|e| format!("seed must be a decimal u64: {e}"))?
                }
                "blue" => m.blue = v.parse().map_err(|e| format!("blue: {e}"))?,
                "orange" => m.orange = v.parse().map_err(|e| format!("orange: {e}"))?,
                _ => {}
            }
        }
        Ok(m)
    }

    /// Canonical query: `seed=<u64>&blue=<tank>&orange=<tank>`.
    pub fn to_query(&self) -> String {
        format!(
            "seed={}&blue={}&orange={}",
            self.seed, self.blue, self.orange
        )
    }

    /// Match config with both loadouts (see [`rules::with_loadouts`]).
    pub fn setup(&self) -> Setup {
        rules::duel(self.blue.loadout, self.orange.loadout)
    }

    /// Fresh policies, by tank id.
    pub fn policies(&self) -> [Box<dyn Policy>; 2] {
        [self.blue.behavior.build(), self.orange.behavior.build()]
    }

    /// A new match (not stepped yet) and its policies.
    pub fn start(&self) -> (Match, [Box<dyn Policy>; 2]) {
        (Match::new(self.setup().config, self.seed), self.policies())
    }

    /// Play to the end: outcome and final state hash.
    pub fn run(&self) -> (Outcome, u64) {
        let (mut m, [mut a, mut b]) = self.start();
        let o = m.run(&mut [a.as_mut(), b.as_mut()]);
        (o, m.state_hash())
    }

    /// The same match with sides swapped (for mirrored evaluation).
    pub fn swapped(&self) -> Self {
        Self {
            seed: self.seed,
            blue: self.orange,
            orange: self.blue,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{config, Mode};

    #[test]
    fn tank_spec_parse_and_print() {
        let t: TankSpec = "Kiter-5-3-1".parse().unwrap();
        assert_eq!(t.to_string(), "kiter-5-3-1");
        assert_eq!(
            "sniper".parse::<TankSpec>().unwrap().to_string(),
            "sniper-3-3-3"
        );
        for bad in [
            "",
            "kiter-",
            "kiter-5-3",
            "kiter-5-3-2",
            "tank-3-3-3",
            "kiter-6-2-1",
        ] {
            assert!(bad.parse::<TankSpec>().is_err(), "{bad:?}");
        }
    }

    #[test]
    fn query_round_trip_every_behavior_and_loadout() {
        for b in Behavior::ALL {
            for l in Loadout::ALL {
                let m = MatchSpec {
                    seed: u64::MAX - l.attack() as u64,
                    blue: TankSpec::new(b, l),
                    orange: TankSpec::new(Behavior::Sniper, Loadout::ALL[18 - l.speed() as usize]),
                };
                assert_eq!(MatchSpec::from_query(&m.to_query()).unwrap(), m);
            }
        }
    }

    #[test]
    fn query_defaults_extras_and_errors() {
        assert_eq!(MatchSpec::from_query("").unwrap(), MatchSpec::default());
        let m = MatchSpec::from_query("tab=customize&seed=7&speed=4&t=600&paused=1").unwrap();
        assert_eq!(m.seed, 7);
        assert_eq!(m.blue, MatchSpec::default().blue);
        for bad in [
            "seed=-1",
            "seed=abc",
            "blue=kiter-5-3-2",
            "orange=wanderer",
            "seed=4%32",
        ] {
            assert!(MatchSpec::from_query(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn url_round_trip_reproduces_the_match() {
        let m = MatchSpec::from_query("seed=42&blue=kiter-5-3-1&orange=charger-4-1-4").unwrap();
        let back = MatchSpec::from_query(&m.to_query()).unwrap();
        assert_eq!(m.run(), back.run());
    }

    #[test]
    fn default_3_3_3_is_the_default_match() {
        // SPEC acceptance: 3/3/3 vs 3/3/3 is bit-identical to the default match.
        let spec = MatchSpec::default();
        let mut plain = Match::new(config(Mode::Duel), spec.seed);
        let (mut a, mut b) = (spec.blue.behavior.build(), spec.orange.behavior.build());
        let o = plain.run(&mut [a.as_mut(), b.as_mut()]);
        assert_eq!(spec.run(), (o, plain.state_hash()));
    }

    #[test]
    fn every_loadout_change_alters_the_sim() {
        // SPEC acceptance: same seed, any one tank's loadout changed → different match.
        let base = MatchSpec::default();
        let (_, h0) = base.run();
        for l in Loadout::ALL.into_iter().filter(|&l| l != Loadout::DEFAULT) {
            for side in 0..2 {
                let mut m = base;
                if side == 0 {
                    m.blue.loadout = l;
                } else {
                    m.orange.loadout = l;
                }
                assert_ne!(m.run().1, h0, "{}", m.to_query());
            }
        }
    }
}
