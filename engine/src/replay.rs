//! Replays: config + seed + per-tick actions. Playback re-simulates, so a replay is
//! both a viewer input and a determinism check (the final state hash must match).
//!
//! ```
//! use engine::bots::{Chaser, Wanderer};
//! use engine::{Match, MatchConfig, Replay};
//!
//! let mut m = Match::new(MatchConfig::duel(), 7);
//! m.run(&mut [&mut Chaser, &mut Wanderer::new(7 ^ 0x5eed)]);
//! let json = m.replay().to_json();
//! let replay = Replay::from_json(&json).expect("parses");
//! let again = replay.verify().expect("reproduces");
//! assert_eq!(again.state_hash(), m.state_hash());
//! ```

use crate::policy::Action;
use crate::sim::{Match, MatchConfig, Outcome};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Bumped whenever the replay format or sim semantics change incompatibly.
/// v2: swept projectile collision + simultaneous movement (v1 replays no longer
/// reproduce), seed written as a decimal string (numbers still accepted on read).
pub const REPLAY_FORMAT: u32 = 2;

/// A recorded match.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Replay {
    /// [`REPLAY_FORMAT`] of the writer. [`Replay::from_json`] rejects any other value.
    pub format: u32,
    /// `engine` crate version of the writer (informational; not checked on load).
    pub engine_version: String,
    /// Serialized as a decimal string (JS-safe); a JSON number is also accepted.
    #[serde(with = "crate::json_u64")]
    pub seed: u64,
    /// The full match config (arena, spawns, tank params, tick limit).
    pub config: MatchConfig,
    /// One entry per tick; each is indexed by tank id.
    pub actions: Vec<Vec<Action>>,
    /// Outcome at recording time (`None` if recorded mid-match).
    pub outcome: Option<Outcome>,
    /// `Match::state_hash` after the last recorded tick, hex-encoded (JS-safe).
    pub final_hash: String,
}

/// Why a replay failed to load or reproduce.
#[derive(Debug, PartialEq)]
pub enum ReplayError {
    /// The JSON did not parse into a [`Replay`] (message from `serde_json`).
    Json(String),
    /// The `format` field is not [`REPLAY_FORMAT`]; carries the value found.
    Format(u32),
    /// Re-simulation ended with a different outcome than the recorded one.
    OutcomeMismatch {
        /// Recorded outcome.
        expected: Option<Outcome>,
        /// Outcome after re-simulating.
        got: Option<Outcome>,
    },
    /// Re-simulation ended in a different state than the recorded `final_hash`.
    HashMismatch {
        /// Recorded `final_hash`.
        expected: String,
        /// Hash after re-simulating (16 lowercase hex digits).
        got: String,
    },
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReplayError::Json(e) => write!(f, "replay json: {e}"),
            ReplayError::Format(v) => write!(f, "unsupported replay format {v}"),
            ReplayError::OutcomeMismatch { expected, got } => {
                write!(f, "outcome mismatch: expected {expected:?}, got {got:?}")
            }
            ReplayError::HashMismatch { expected, got } => {
                write!(f, "state hash mismatch: expected {expected}, got {got}")
            }
        }
    }
}

impl std::error::Error for ReplayError {}

impl Replay {
    /// Snapshot a match (finished or not): its config, seed, action history, outcome
    /// and current state hash, tagged with [`REPLAY_FORMAT`] and the engine version.
    pub fn from_match(m: &Match) -> Self {
        Self {
            format: REPLAY_FORMAT,
            engine_version: crate::version().to_string(),
            seed: m.seed(),
            config: m.config().clone(),
            actions: m.history().to_vec(),
            outcome: m.outcome(),
            final_hash: format!("{:016x}", m.state_hash()),
        }
    }

    /// Serialize to compact JSON (see `docs/engine/replay-format.md`).
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("replay serializes")
    }

    /// Parse JSON and check `format == REPLAY_FORMAT`. Does not re-simulate; call
    /// [`Replay::verify`] for that.
    pub fn from_json(s: &str) -> Result<Self, ReplayError> {
        let r: Replay = serde_json::from_str(s).map_err(|e| ReplayError::Json(e.to_string()))?;
        if r.format != REPLAY_FORMAT {
            return Err(ReplayError::Format(r.format));
        }
        Ok(r)
    }

    /// Re-simulate the whole replay and return the resulting match. Does not compare
    /// anything; see [`Replay::verify`].
    pub fn play(&self) -> Match {
        let mut m = Match::new(self.config.clone(), self.seed);
        for a in &self.actions {
            m.step(a);
        }
        m
    }

    /// Re-simulate and check the outcome and final state hash match the recording.
    pub fn verify(&self) -> Result<Match, ReplayError> {
        let m = self.play();
        if m.outcome() != self.outcome {
            return Err(ReplayError::OutcomeMismatch {
                expected: self.outcome,
                got: m.outcome(),
            });
        }
        let got = format!("{:016x}", m.state_hash());
        if got != self.final_hash {
            return Err(ReplayError::HashMismatch {
                expected: self.final_hash.clone(),
                got,
            });
        }
        Ok(m)
    }
}

/// Step-by-step playback for viewers: call [`ReplayPlayer::step`] once per frame
/// and draw [`ReplayPlayer::state`].
#[derive(Clone, Debug)]
pub struct ReplayPlayer {
    replay: Replay,
    state: Match,
    cursor: usize,
}

impl ReplayPlayer {
    /// Start playback at tick 0 (the match is created from the replay's config and seed).
    pub fn new(replay: Replay) -> Self {
        let state = Match::new(replay.config.clone(), replay.seed);
        Self {
            replay,
            state,
            cursor: 0,
        }
    }

    /// Apply the next recorded tick. Returns `false` when the replay is exhausted.
    pub fn step(&mut self) -> bool {
        match self.replay.actions.get(self.cursor) {
            Some(a) => {
                self.state.step(a);
                self.cursor += 1;
                true
            }
            None => false,
        }
    }

    /// The re-simulated match as of the last applied tick.
    pub fn state(&self) -> &Match {
        &self.state
    }

    /// True once every recorded tick has been applied.
    pub fn is_finished(&self) -> bool {
        self.cursor >= self.replay.actions.len()
    }
}
