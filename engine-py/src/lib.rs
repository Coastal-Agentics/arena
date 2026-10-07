//! Python bindings for the arena games: the `coastal-arena` wheel
//! (`docs/design/game-system.md` §6, M4; GATE-003 §6).
//!
//! The crate has two layers:
//! - **The session core** (always built, pure Rust): [`Game`] puts a game's build
//!   validation and scripted drivers behind one interface, and [`Session`] runs a
//!   [`Match`](engine::generic::Match) for training tools through the generic
//!   [`Flat`] view. It reads actions from a `&[f32]` slice, writes observations and
//!   rewards into caller-owned slices, and does frame-skip and the early stop in Rust.
//!   Tank Arena and Racing both go through it, and a new game is one [`Game`] impl.
//! - **The Python module** `coastal_arena._core` (`python` feature, pyo3 with the
//!   stable ABI, `abi3-py310`). It is off by default, so `cargo build`, `test` and
//!   `clippy --workspace` never compile pyo3 or link Python. maturin turns on
//!   `extension-module` (see `pyproject.toml`).
//!
//! Nothing here changes a game: a Python-driven match is an ordinary
//! [`Replay`](engine::generic::Replay), byte for byte what the same actions give
//! natively ([`reference_episode`] and the tests). See `docs/engine/python.md`.

#![deny(missing_docs)]

mod games;
#[cfg(feature = "python")]
mod python;
mod session;

pub use games::{GameInfo, Lineup, GAMES};
pub use session::{reference_action, reference_episode, Session};

use engine::generic::{Flat, Policy};

/// One game as the bindings see it: its [`Flat`] view, how a list of builds becomes a
/// match config, and its scripted drivers. Implemented for
/// [`TankRules`](engine::TankRules) and [`RacingRules`](racing::RacingRules).
pub trait Game: Flat + Sized + 'static
where
    Self::Config: Clone,
{
    /// Fewest builds (agents) a match takes.
    const MIN_AGENTS: usize;
    /// Most builds (agents) a match takes.
    const MAX_AGENTS: usize;

    /// The agent's name in the Python env: `"<team>_<agent>"` (tank `blue_0`,
    /// `orange_1`; racing `car_0` … `car_3`).
    fn agent_name(agent: usize) -> String;

    /// Validate each build with the game's catalog (`<game>::catalog::validate_build`)
    /// and lay out the match: agent `i` plays `builds[i]`.
    fn lineup(builds: &[&str]) -> Result<Lineup<Self>, String>;

    /// The scripted driver for `agent` with catalog behavior `id`, seeded the way the
    /// game's native runner seeds it for a match with this `seed`.
    fn scripted(
        config: &Self::Config,
        agent: usize,
        id: &'static str,
        seed: u64,
    ) -> Box<dyn Policy<Self>>;
}
