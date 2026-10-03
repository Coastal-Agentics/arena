//! Generic replays: config + seed + per-tick actions, the setup hash and the format
//! number. See [`crate::replay`] for the Tank Arena view and an example.

use super::{Match, Outcome, Rules};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Bumped whenever the replay format or sim semantics change incompatibly.
/// v2: swept projectile collision + simultaneous movement (v1 replays no longer
/// reproduce), seed written as a decimal string (numbers still accepted on read).
/// v3: adds `setup_hash` (seed + config); sim semantics unchanged from v2.
/// v4: the config may carry per-tank params (`TankSpawn::params`) and stationary
/// accuracy (`TankParams::projectile_spread_still`); both are omitted from JSON when
/// unset, and a config without them plays exactly as in v2/v3.
/// v5: the versioned envelope for more than one game: adds `game` ([`Rules::GAME`]) and
/// `rules_version` ([`Rules::RULES_VERSION`]), both required and checked on load. New
/// games write it; a game whose replays predate it keeps writing its older format
/// ([`Rules::WRITES_FORMAT`]; Tank Arena: 4, byte-identical).
pub const REPLAY_FORMAT: u32 = 5;

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
pub fn setup_hash<C: Serialize>(seed: u64, config: &C) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let config = serde_json::to_vec(config).expect("config serializes");
    for &b in seed.to_le_bytes().iter().chain(config.iter()) {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// A recorded match under rules `R`: config, seed and per-tick actions. For Tank Arena
/// use the crate-root alias [`crate::Replay`] (`Replay<TankRules>`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct Replay<R: Rules> {
    /// Format of the writer: [`Rules::WRITES_FORMAT`] for new replays (5, or 4 for Tank
    /// Arena). [`Replay::from_json`] accepts [`OLDEST_READABLE_FORMAT`] through
    /// [`REPLAY_FORMAT`].
    pub format: u32,
    /// [`Rules::GAME`]: required from format 5, absent before (a file without it is a
    /// Tank Arena file).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game: Option<String>,
    /// [`Rules::RULES_VERSION`]: required from format 5, absent before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules_version: Option<u32>,
    /// `engine` crate version of the writer (informational; not checked on load).
    pub engine_version: String,
    /// Serialized as a decimal string (JS-safe); a JSON number is also accepted.
    #[serde(with = "crate::json_u64")]
    pub seed: u64,
    /// The full match config (Tank Arena: arena, spawns, tank params, tick limit).
    pub config: R::Config,
    /// One entry per tick; each is indexed by agent (Tank Arena: tank id).
    pub actions: Vec<Vec<R::Action>>,
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
    /// The replay is for another game: its `game` isn't [`Rules::GAME`]. `got` is `None`
    /// when the file names no game: a format 5 file without `game`, or a file older than
    /// format 5 (a Tank Arena file) loaded by rules that write format 5.
    GameMismatch {
        /// [`Rules::GAME`] of the rules loading the file.
        expected: &'static str,
        /// The file's `game`.
        got: Option<String>,
    },
    /// A format 5 replay's `rules_version` isn't [`Rules::RULES_VERSION`] (`None`:
    /// missing). The file would not replay the same under these rules.
    RulesVersionMismatch {
        /// [`Rules::GAME`].
        game: &'static str,
        /// [`Rules::RULES_VERSION`] of the rules loading the file.
        expected: u32,
        /// The file's `rules_version`.
        got: Option<u32>,
    },
    /// The `format` field is outside [`OLDEST_READABLE_FORMAT`]..=[`REPLAY_FORMAT`];
    /// carries the value found.
    Format(u32),
    /// A format 3 (or later) replay has no `setup_hash`.
    MissingSetupHash,
    /// The replay uses a field that its (older) format does not have: `game` or
    /// `rules_version` before format 5, or a config field reported by
    /// [`Rules::check_format`] (Tank Arena: `tanks[].params` or
    /// `params.projectile_spread_still` in a format 2 or 3 file).
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
            ReplayError::GameMismatch {
                expected,
                got: Some(got),
            } => write!(
                f,
                "replay is for game \"{got}\", these rules are \"{expected}\""
            ),
            ReplayError::GameMismatch {
                expected,
                got: None,
            } => write!(f, "replay names no game, these rules are \"{expected}\""),
            ReplayError::RulesVersionMismatch {
                game,
                expected,
                got: Some(got),
            } => write!(
                f,
                "replay has {game} rules_version {got}, these rules are version {expected}"
            ),
            ReplayError::RulesVersionMismatch {
                game,
                expected,
                got: None,
            } => write!(
                f,
                "replay has no rules_version, these {game} rules are version {expected}"
            ),
            ReplayError::FieldNotInFormat { format, field } => {
                let needs = if matches!(*field, "game" | "rules_version") {
                    5
                } else {
                    4
                };
                write!(
                    f,
                    "replay format {format} has no {field} (needs format {needs})"
                )
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

/// The fields of a replay file that decide how to read the rest, parsed first so a file
/// for another game fails with a clear error instead of a config parse error.
#[derive(Deserialize)]
struct Envelope {
    format: u32,
    #[serde(default)]
    game: Option<String>,
    #[serde(default)]
    rules_version: Option<u32>,
}

impl Envelope {
    /// The format range, then `game` and `rules_version` against `R`.
    fn check<R: Rules>(self) -> Result<(), ReplayError> {
        let format = self.format;
        if !(OLDEST_READABLE_FORMAT..=REPLAY_FORMAT).contains(&format) {
            return Err(ReplayError::Format(format));
        }
        if format >= 5 {
            if self.game.as_deref() != Some(R::GAME) {
                return Err(ReplayError::GameMismatch {
                    expected: R::GAME,
                    got: self.game,
                });
            }
            if self.rules_version != Some(R::RULES_VERSION) {
                return Err(ReplayError::RulesVersionMismatch {
                    game: R::GAME,
                    expected: R::RULES_VERSION,
                    got: self.rules_version,
                });
            }
        } else {
            for (present, field) in [
                (self.game.is_some(), "game"),
                (self.rules_version.is_some(), "rules_version"),
            ] {
                if present {
                    return Err(ReplayError::FieldNotInFormat { format, field });
                }
            }
            if R::WRITES_FORMAT >= 5 {
                return Err(ReplayError::GameMismatch {
                    expected: R::GAME,
                    got: None,
                });
            }
        }
        Ok(())
    }
}

impl<R: Rules> Replay<R> {
    /// Snapshot a match (finished or not): its config, seed, action history, outcome,
    /// current state hash and setup hash, tagged with the engine version and
    /// [`Rules::WRITES_FORMAT`]. From format 5 it also records [`Rules::GAME`] and
    /// [`Rules::RULES_VERSION`]; a format 4 writer (Tank Arena) leaves them out, so its
    /// files are byte-identical to before format 5.
    pub fn from_match(m: &Match<R>) -> Self {
        let v5 = R::WRITES_FORMAT >= 5;
        Self {
            format: R::WRITES_FORMAT,
            game: v5.then(|| R::GAME.to_string()),
            rules_version: v5.then_some(R::RULES_VERSION),
            engine_version: crate::version().to_string(),
            seed: m.seed(),
            config: m.config().clone(),
            actions: m.history().iter().map(<[_]>::to_vec).collect(),
            outcome: m.outcome(),
            final_hash: format!("{:016x}", m.state_hash()),
            setup_hash: Some(format!("{:016x}", setup_hash(m.seed(), m.config()))),
        }
    }

    /// Serialize to compact JSON (see `docs/engine/replay-format.md`).
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("replay serializes")
    }

    /// Parse JSON and check the format, in this order:
    /// 1. [`OLDEST_READABLE_FORMAT`] through [`REPLAY_FORMAT`] load
    ///    ([`ReplayError::Format`]);
    /// 2. the game: a format 5 file must carry `game` == [`Rules::GAME`]
    ///    ([`ReplayError::GameMismatch`]) and `rules_version` == [`Rules::RULES_VERSION`]
    ///    ([`ReplayError::RulesVersionMismatch`]). An older file must carry neither
    ///    ([`ReplayError::FieldNotInFormat`]) and loads only under rules that write a
    ///    format older than 5 ([`Rules::WRITES_FORMAT`]; Tank Arena), since it predates
    ///    the `game` field ([`ReplayError::GameMismatch`] with `got: None`);
    /// 3. the whole file parses ([`ReplayError::Json`]);
    /// 4. format 3 and later must carry a `setup_hash`;
    /// 5. [`Rules::check_format`] (Tank Arena: formats 2 and 3 must not use the v4
    ///    config fields, per-tank `params` and `projectile_spread_still`).
    ///
    /// Steps 1 and 2 read only `format`, `game` and `rules_version`, so a file for
    /// another game is reported as such, not as a config parse error. Does not
    /// re-simulate; call [`Replay::verify`] for that.
    ///
    /// Older replays keep their `format` (a format 2 replay loads with
    /// `setup_hash: None`). To upgrade one, verify it and re-record the result:
    /// `r.verify()?.replay()` (in [`Rules::WRITES_FORMAT`]).
    pub fn from_json(s: &str) -> Result<Self, ReplayError> {
        let json = |e: serde_json::Error| ReplayError::Json(e.to_string());
        serde_json::from_str::<Envelope>(s)
            .map_err(json)?
            .check::<R>()?;
        let r: Self = serde_json::from_str(s).map_err(json)?;
        if r.format >= 3 && r.setup_hash.is_none() {
            return Err(ReplayError::MissingSetupHash);
        }
        if let Err(field) = R::check_format(&r.config, r.format) {
            return Err(ReplayError::FieldNotInFormat {
                format: r.format,
                field,
            });
        }
        Ok(r)
    }

    /// Re-simulate the whole replay and return the resulting match. Does not compare
    /// anything; see [`Replay::verify`].
    pub fn play(&self) -> Match<R> {
        let mut m = Match::new(self.config.clone(), self.seed);
        for a in &self.actions {
            m.step(a);
        }
        m
    }

    /// Check the replay against its recording: first the `setup_hash` (if present)
    /// against the seed and config, then re-simulate and compare the outcome and the
    /// final state hash. Returns the re-simulated match on success.
    pub fn verify(&self) -> Result<Match<R>, ReplayError> {
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
/// and draw [`ReplayPlayer::state`]. For Tank Arena use the crate-root alias
/// [`crate::ReplayPlayer`].
pub struct ReplayPlayer<R: Rules> {
    replay: Replay<R>,
    state: Match<R>,
    cursor: usize,
}

// Written out (not derived) so the bounds are on `Replay<R>` and `Match<R>`, whose
// derives already require what the rules' associated types need.
impl<R: Rules> Clone for ReplayPlayer<R>
where
    Replay<R>: Clone,
    Match<R>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            replay: self.replay.clone(),
            state: self.state.clone(),
            cursor: self.cursor,
        }
    }
}

impl<R: Rules> fmt::Debug for ReplayPlayer<R>
where
    Replay<R>: fmt::Debug,
    Match<R>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReplayPlayer")
            .field("replay", &self.replay)
            .field("state", &self.state)
            .field("cursor", &self.cursor)
            .finish()
    }
}

impl<R: Rules> ReplayPlayer<R> {
    /// Start playback at tick 0 (the match is created from the replay's config and seed).
    pub fn new(replay: Replay<R>) -> Self {
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
    pub fn state(&self) -> &Match<R> {
        &self.state
    }

    /// True once every recorded tick has been applied.
    pub fn is_finished(&self) -> bool {
        self.cursor >= self.replay.actions.len()
    }
}
