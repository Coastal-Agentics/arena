//! Tank Arena game rules on top of the Coastal Agentics Arena engine.
//!
//! Implements `games/tank/SPEC.md` (GATE-002, approved, with its 2026-09-30 amendment):
//! * [`bots`]: the placeholder `Chaser` and `Wanderer` used by `engine-cli` and the
//!   viewer's built-in-bot match;
//! * [`loadout`]: the 9-point Attack/Speed/Defense budget, the 19 valid loadouts,
//!   presets, the level → [`engine::TankParams`] mapping, hits-to-kill and the
//!   Customize-triangle snap;
//! * [`rules`]: the pillar arena, fixed mirrored spawns (duel, 2v2, FFA-4), the
//!   7200-tick limit, and per-tank loadouts through `TankSpawn::params`;
//! * [`los`]: line of sight, from `TankObs::los` between tanks and an obstacle test for
//!   other points;
//! * [`policies`]: the charger, kiter and sniper scripted policies with their params;
//! * [`matchup`]: a shareable duel (seed, behaviors, loadouts) and its URL query form.
//! * [`evolve`]: the GATE-003 M1 genetic algorithm over those params and the loadout
//!   (`docs/plans/GATE-003-learning-tanks.md`; CLI: `examples/evolve.rs`).
//!
//! The tank entity, `TankParams`, `TankSpawn`, the step rules and the tank
//! `Observation`/`Action` still live in `engine/`: ADR-014 keeps them there unless a
//! second Rust game appears (see ADR-009 and ADR-014).

pub mod bots;
pub mod evolve;
pub mod loadout;
pub mod los;
pub mod matchup;
pub mod policies;
pub mod rules;

pub use bots::{Chaser, Wanderer};
pub use loadout::{Loadout, LoadoutError, Preset};
pub use matchup::{MatchSpec, TankSpec};
pub use policies::{Behavior, Charger, ChargerParams, Kiter, KiterParams, Sniper, SniperParams};

/// Name of this game.
pub const GAME_NAME: &str = "Tank Arena";

/// Tick rate inherited from the engine.
pub fn tick_hz() -> u32 {
    engine::TICK_HZ
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_engine_tick_rate() {
        assert_eq!(tick_hz(), 60);
        assert_eq!(GAME_NAME, "Tank Arena");
    }
}
