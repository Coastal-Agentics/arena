//! Trivial placeholder policies (for the CLI smoke run, tests, and the viewer).
//! Real tank AI lives in `games/tank` later.

use crate::angle::turn_toward;
use crate::policy::{Action, Observation, Policy};
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
