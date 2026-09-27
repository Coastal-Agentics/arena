//! Tank Arena game rules on top of the Starscream engine.
//!
//! Phase 0 stub. The Tank Designer-Developer writes `SPEC.md` first (Phase 1),
//! then implements rules once Nye approves the spec.

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
