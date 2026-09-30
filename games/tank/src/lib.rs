//! Tank Arena game rules on top of the Coastal Agentics Arena engine.
//!
//! Implements `games/tank/SPEC.md` (GATE-002, approved):
//! * [`loadout`]: the 9-point Attack/Speed/Defense budget, the 19 valid loadouts,
//!   presets, the level → [`engine::TankParams`] mapping, hits-to-kill and the
//!   Customize-triangle snap;
//! * [`rules`]: the pillar arena, fixed mirrored spawns (duel, 2v2, FFA-4), the
//!   7200-tick limit, and per-tank loadouts (stubbed until engine ask #5);
//! * [`los`]: line of sight (a stand-in until engine ask #2);
//! * [`policies`]: the charger, kiter and sniper scripted policies with their params;
//! * [`matchup`]: a shareable duel (seed, behaviors, loadouts) and its URL query form.
//!
//! The tank entity, `TankParams`, `TankSpawn`, the step rules and the tank
//! `Observation`/`Action` still live in `engine/` (see ADR-009).

pub mod loadout;
pub mod los;
pub mod matchup;
pub mod policies;
pub mod rules;

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
