//! Racing, game #2 of the Coastal Agentics Arena (`docs/design/racing.md`, racing v0).
//!
//! 1–4 cars race laps around a walled track; the first car home wins. Each car is its
//! own agent and team. The rules are a [`engine::generic::Rules`] +
//! [`engine::generic::Flat`] impl with the default-free [`Rules::reward`]:
//! * [`track`]: tracks as data (Ring first), mitred walls, gates, rays;
//! * [`setup`]: Power / Top speed / Grip levels on the tanks' 9-point budget;
//! * [`config`]: the race config and its validation (no tunnelling);
//! * [`rules`]: [`RacingRules`], the physics, collisions, laps, placings, end and reward;
//! * [`obs`] and [`flat`]: the rich observation and the 43/2 flat view;
//! * [`end`]: [`RaceEnd`] and the adapter onto the engine's `EndReason`;
//! * [`catalog`]: the build catalog and validator in the shared `game_catalog` shape.
//!
//! [`Rules::reward`]: engine::generic::Rules::reward

pub mod catalog;
pub mod config;
pub mod end;
pub mod flat;
pub mod obs;
pub mod rules;
pub mod setup;
pub mod track;

pub use catalog::RaceParams;
pub use config::{CarParams, ConfigError, Physics, RacingConfig, MAX_CARS};
pub use end::RaceEnd;
pub use obs::{Observation, OpponentObs};
pub use rules::{Car, RaceAction, RaceEvent, RaceState, RacingRules};
pub use setup::{Setup, SetupError, STAT_KEYS};
pub use track::{Track, TrackGeom};

/// Game id (`game` in the format 5 replay envelope and the catalog).
pub const GAME: &str = "racing";
/// Racing rules version: bumped whenever racing's sim semantics or a level's meaning
/// change (the catalog and the format 5 envelope carry it).
pub const RULES_VERSION: u64 = 1;

/// A racing match: `engine::generic::Match<RacingRules>`.
pub type Race = engine::generic::Match<RacingRules>;
/// A racing replay.
pub type RaceReplay = engine::generic::Replay<RacingRules>;
