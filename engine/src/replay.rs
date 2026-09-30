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

use crate::policy::Action;
use crate::sim::{Match, MatchConfig, Outcome};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Bumped whenever the replay format or sim semantics change incompatibly.
/// v2: swept projectile collision + simultaneous movement (v1 replays no longer
/// reproduce), seed written as a decimal string (numbers still accepted on read).
/// v3: adds `setup_hash` (seed + config); sim semantics unchanged from v2.
/// v4: the config may carry per-tank params (`TankSpawn::params`) and stationary
/// accuracy (`TankParams::projectile_spread_still`); both are omitted from JSON when
/// unset, and a config without them plays exactly as in v2/v3.
pub const REPLAY_FORMAT: u32 = 4;

/// Oldest format [`Replay::from_json`] still reads. Format 2 has the same sim semantics
/// as 3 and 4 but no `setup_hash`, so verifying a v2 replay does not cover its config.
/// Formats 2 and 3 cannot use the v4 config fields.
pub const OLDEST_READABLE_FORMAT: u32 = 2;

/// FNV-1a (64-bit) of the match setup: the seed's 8 little-endian bytes, then the
/// config serialized with `serde_json` (compact, struct field order).
///
/// The config is hashed in its canonical serialized form, so every field is covered
/// (including fields added later) and JSON formatting of a file doesn't matter: a
/// replay that is re-indented, or writes `16` for `16.0`, hashes the same.
///
/// ```
/// use engine::replay::setup_hash;
/// use engine::MatchConfig;
///
/// let duel = MatchConfig::duel();
/// let mut open = duel.clone();
/// open.arena.obstacles.clear();
/// assert_eq!(setup_hash(7, &duel), setup_hash(7, &duel.clone()));
/// assert_ne!(setup_hash(7, &duel), setup_hash(7, &open));
/// assert_ne!(setup_hash(7, &duel), setup_hash(8, &duel));
/// ```
pub fn setup_hash(seed: u64, config: &MatchConfig) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let config = serde_json::to_vec(config).expect("config serializes");
    for &b in seed.to_le_bytes().iter().chain(config.iter()) {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// A recorded match.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Replay {
    /// Format of the writer: [`REPLAY_FORMAT`] for new replays. [`Replay::from_json`]
    /// accepts [`OLDEST_READABLE_FORMAT`] through [`REPLAY_FORMAT`].
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
    /// [`setup_hash`] of `seed` and `config`, 16 lowercase hex digits. Required from
    /// format 3; absent (`None`) in format 2 files, and then not checked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setup_hash: Option<String>,
}

/// Why a replay failed to load or reproduce.
#[derive(Debug, PartialEq)]
pub enum ReplayError {
    /// The JSON did not parse into a [`Replay`] (message from `serde_json`).
    Json(String),
    /// The `format` field is outside [`OLDEST_READABLE_FORMAT`]..=[`REPLAY_FORMAT`];
    /// carries the value found.
    Format(u32),
    /// A format 3 (or later) replay has no `setup_hash`.
    MissingSetupHash,
    /// The config uses a field that the replay's (older) format does not have:
    /// `tanks[].params` or `params.projectile_spread_still` in a format 2 or 3 file.
    FieldNotInFormat {
        /// The replay's `format`.
        format: u32,
        /// The config field that needs a newer format.
        field: &'static str,
    },
    /// The seed or config differs from what was recorded (`setup_hash` mismatch).
    SetupMismatch {
        /// Recorded `setup_hash`.
        expected: String,
        /// [`setup_hash`] of the replay's seed and config as loaded.
        got: String,
    },
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
            ReplayError::MissingSetupHash => write!(f, "replay has no setup_hash"),
            ReplayError::FieldNotInFormat { format, field } => {
                write!(f, "replay format {format} has no {field} (needs format 4)")
            }
            ReplayError::SetupMismatch { expected, got } => write!(
                f,
                "setup hash mismatch (seed or config edited): expected {expected}, got {got}"
            ),
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
    /// Snapshot a match (finished or not): its config, seed, action history, outcome,
    /// current state hash and setup hash, tagged with [`REPLAY_FORMAT`] and the engine
    /// version.
    pub fn from_match(m: &Match) -> Self {
        Self {
            format: REPLAY_FORMAT,
            engine_version: crate::version().to_string(),
            seed: m.seed(),
            config: m.config().clone(),
            actions: m.history().to_vec(),
            outcome: m.outcome(),
            final_hash: format!("{:016x}", m.state_hash()),
            setup_hash: Some(format!("{:016x}", setup_hash(m.seed(), m.config()))),
        }
    }

    /// Serialize to compact JSON (see `docs/engine/replay-format.md`).
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("replay serializes")
    }

    /// Parse JSON and check the format: [`OLDEST_READABLE_FORMAT`] through
    /// [`REPLAY_FORMAT`] load; format 3 and later must carry a `setup_hash`, and
    /// formats 2 and 3 must not use the v4 config fields (per-tank `params`,
    /// `projectile_spread_still`). Does not re-simulate; call [`Replay::verify`] for that.
    ///
    /// Older replays keep their `format` (a format 2 replay loads with
    /// `setup_hash: None`). To upgrade one, verify it and re-record the result:
    /// `r.verify()?.replay()`.
    pub fn from_json(s: &str) -> Result<Self, ReplayError> {
        let r: Replay = serde_json::from_str(s).map_err(|e| ReplayError::Json(e.to_string()))?;
        if !(OLDEST_READABLE_FORMAT..=REPLAY_FORMAT).contains(&r.format) {
            return Err(ReplayError::Format(r.format));
        }
        if r.format >= 3 && r.setup_hash.is_none() {
            return Err(ReplayError::MissingSetupHash);
        }
        if r.format < 4 {
            let c = &r.config;
            let field = if c.tanks.iter().any(|t| t.params.is_some()) {
                Some("tanks[].params")
            } else if c.params.projectile_spread_still.is_some() {
                Some("params.projectile_spread_still")
            } else {
                None
            };
            if let Some(field) = field {
                return Err(ReplayError::FieldNotInFormat {
                    format: r.format,
                    field,
                });
            }
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

    /// Check the replay against its recording: first the `setup_hash` (if present)
    /// against the seed and config, then re-simulate and compare the outcome and the
    /// final state hash. Returns the re-simulated match on success.
    pub fn verify(&self) -> Result<Match, ReplayError> {
        if let Some(expected) = &self.setup_hash {
            let got = format!("{:016x}", setup_hash(self.seed, &self.config));
            if &got != expected {
                return Err(ReplayError::SetupMismatch {
                    expected: expected.clone(),
                    got,
                });
            }
        }
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
