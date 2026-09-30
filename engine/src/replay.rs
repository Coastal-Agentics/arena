//! Replays: config + seed + per-tick actions. Playback re-simulates, so a replay is
//! both a viewer input and a determinism check (the final state hash must match).
//! Since format 3 a replay also records a [`setup_hash`] of its seed and config, so
//! editing the config (or seed) fails verification even when the edit happens not to
//! change what the match does.
//!
//! ```
//! use engine::{Action, Match, MatchConfig, Observation, Replay};
//!
//! // Any closure is a policy: this one circles and fires.
//! let mut circle = |_: &Observation| Action { throttle: 1.0, turn: 0.5, fire: true, ..Action::default() };
//! let mut m = Match::new(MatchConfig::duel(), 7);
//! m.run(&mut [&mut circle.clone(), &mut circle]);
//! let json = m.replay().to_json();
//! let replay = Replay::from_json(&json).expect("parses");
//! let again = replay.verify().expect("reproduces");
//! assert_eq!(again.state_hash(), m.state_hash());
//! ```

pub use crate::generic::{setup_hash, ReplayError, OLDEST_READABLE_FORMAT, REPLAY_FORMAT};

use crate::generic;
use crate::sim::TankRules;

/// A recorded Tank Arena match: [`generic::Replay`] with [`TankRules`]. The JSON shape
/// is described in `docs/engine/replay-format.md`.
pub type Replay = generic::Replay<TankRules>;

/// Step-by-step playback of a Tank Arena [`Replay`]: [`generic::ReplayPlayer`] with
/// [`TankRules`].
pub type ReplayPlayer = generic::ReplayPlayer<TankRules>;
