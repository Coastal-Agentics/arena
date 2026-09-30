//! The placeholder bots `Chaser` and `Wanderer`, used by `engine-cli` and by the web
//! viewer's built-in-bot match (`engine-wasm`). They moved here from `engine::bots`
//! (ADR-014 Phase A, #19 and #22) with unchanged behaviour.
//!
//! They play the engine's random-spawn duel (`MatchConfig::duel`), not the Tank Arena
//! rules; the replay hashes quoted in `docs/engine/` come from them.

use engine::angle::turn_toward;
use engine::{Action, Observation, Policy};
use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Drives at the nearest enemy, aims the turret at it, fires when roughly aligned.
///
/// Stateless. Per tick: full throttle while the nearest enemy is more than 220 units
/// away (else stop); hull turns toward it (tolerance 0.2); turret turns toward it
/// (tolerance 0.08) and `fire` is set when the turret is within that tolerance.
/// Line of sight is not checked. With no living enemy it returns [`Action::default`].
#[derive(Clone, Debug, Default)]
pub struct Chaser;

impl Policy for Chaser {
    fn act(&mut self, obs: &Observation) -> Action {
        let Some(e) = obs.enemies.first() else {
            return Action::default();
        };
        let aim = turn_toward(obs.me.turret, e.rel, 0.08);
        let steer = turn_toward(obs.me.heading, e.rel, 0.2);
        Action {
            throttle: if e.dist_sq > 220.0 * 220.0 { 1.0 } else { 0.0 },
            turn: steer as f32,
            turret_turn: aim as f32,
            fire: aim == 0,
        }
    }
}

/// Wanders with seeded random steering, tracks the nearest enemy with its turret,
/// and fires whenever it is roughly aligned.
///
/// Always drives at throttle 0.8. It picks a hull turn of -1, 0 or +1 from its own
/// RNG and holds it for 20 to 59 ticks, then picks again. The turret and `fire` work
/// like [`Chaser`]'s (tolerance 0.08).
#[derive(Clone, Debug)]
pub struct Wanderer {
    rng: ChaCha8Rng,
    turn: f32,
    hold: u32,
}

impl Wanderer {
    /// A wanderer whose steering RNG is `ChaCha8Rng::seed_from_u64(seed)`.
    ///
    /// `engine-cli` and the web viewer seed team 1's wanderer with `match_seed ^ 0x5eed`.
    pub fn new(seed: u64) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
            turn: 0.0,
            hold: 0,
        }
    }
}

impl Policy for Wanderer {
    fn act(&mut self, obs: &Observation) -> Action {
        if self.hold == 0 {
            self.turn = (self.rng.next_u32() % 3) as f32 - 1.0;
            self.hold = 20 + self.rng.next_u32() % 40;
        }
        self.hold -= 1;
        let (aim, fire) = match obs.enemies.first() {
            Some(e) => {
                let a = turn_toward(obs.me.turret, e.rel, 0.08);
                (a as f32, a == 0)
            }
            None => (0.0, false),
        };
        Action {
            throttle: 0.8,
            turn: self.turn,
            turret_turn: aim,
            fire,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::{Match, MatchConfig};

    /// The engine-cli pairing: Chaser (team 0) vs Wanderer seeded `seed ^ 0x5eed`, on
    /// the engine's default duel.
    fn play(seed: u64) -> Match {
        let mut m = Match::new(MatchConfig::duel(), seed);
        let mut a = Chaser;
        let mut b = Wanderer::new(seed ^ 0x5eed);
        m.run(&mut [&mut a, &mut b]);
        m
    }

    #[test]
    fn documented_hashes_are_unchanged() {
        // The replay pins listed in ADR-014 and quoted in docs/engine/engine-cli.md
        // (42, 101, u64::MAX) and replay-format.md (7); engine-cli pins its own rows too.
        let cases: [(u64, u32, &str, &str); 4] = [
            (42, 447, "03722b5e86d38fac", "-"),
            (7, 276, "51234f61b02b5784", "0b24ce74f45e9a27"),
            (101, 274, "baf3fcb2cbb76c06", "9cfd58498bbe3f85"),
            (u64::MAX, 532, "f1d983e88de5d020", "-"),
        ];
        for (seed, ticks, hash, setup) in cases {
            let m = play(seed);
            assert_eq!(
                m.outcome().map(|o| (o.winner, o.ticks)),
                Some((Some(1), ticks)),
                "seed {seed}"
            );
            let r = m.replay();
            assert_eq!(r.final_hash, hash, "seed {seed}");
            if setup != "-" {
                assert_eq!(r.setup_hash.as_deref(), Some(setup), "seed {seed}");
            }
        }
    }

    #[test]
    fn smoke_run_seeds_0_to_199_are_unchanged() {
        // FNV-1a over (winner, ticks, state hash) of seeds 0..200, recorded from
        // `engine::bots` before the move, so the bots still play exactly as they did.
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for seed in 0..200 {
            let m = play(seed);
            let o = m.outcome().expect("match ends");
            let w = o.winner.unwrap_or(255);
            for b in [w]
                .into_iter()
                .chain(o.ticks.to_le_bytes())
                .chain(m.state_hash().to_le_bytes())
            {
                h = (h ^ b as u64).wrapping_mul(0x0100_0000_01b3);
            }
        }
        assert_eq!(format!("{h:016x}"), SMOKE_DIGEST);
    }

    const SMOKE_DIGEST: &str = "28ae434ec1996a74";

    #[test]
    fn wanderer_is_seeded() {
        let obs = Match::new(MatchConfig::duel(), 1).observe(1);
        let turns = |seed| {
            let mut w = Wanderer::new(seed);
            (0..200).map(|_| w.act(&obs).turn).collect::<Vec<_>>()
        };
        assert_eq!(turns(3), turns(3));
        assert_ne!(turns(3), turns(4));
    }
}
