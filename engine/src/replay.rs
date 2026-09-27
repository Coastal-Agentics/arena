//! Replays: config + seed + per-tick actions. Playback re-simulates, so a replay is
//! both a viewer input and a determinism check (the final state hash must match).

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
    pub format: u32,
    pub engine_version: String,
    /// Serialized as a decimal string (JS-safe); a JSON number is also accepted.
    #[serde(with = "crate::json_u64")]
    pub seed: u64,
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
    Json(String),
    Format(u32),
    OutcomeMismatch {
        expected: Option<Outcome>,
        got: Option<Outcome>,
    },
    HashMismatch {
        expected: String,
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

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("replay serializes")
    }

    pub fn from_json(s: &str) -> Result<Self, ReplayError> {
        let r: Replay = serde_json::from_str(s).map_err(|e| ReplayError::Json(e.to_string()))?;
        if r.format != REPLAY_FORMAT {
            return Err(ReplayError::Format(r.format));
        }
        Ok(r)
    }

    /// Re-simulate the whole replay and return the resulting match.
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

    pub fn state(&self) -> &Match {
        &self.state
    }

    pub fn is_finished(&self) -> bool {
        self.cursor >= self.replay.actions.len()
    }
}
