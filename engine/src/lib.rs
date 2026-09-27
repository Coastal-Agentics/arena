//! Starscream engine: deterministic fixed-timestep 2D simulation core.
//!
//! Phase 0 stub. The Engine Lead builds the real API in Phase 1.

/// Simulation tick rate in Hz. The sim always steps at this fixed rate.
pub const TICK_HZ: u32 = 60;

/// Returns the engine crate version.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_rate_is_60hz() {
        assert_eq!(TICK_HZ, 60);
        assert!(!version().is_empty());
    }
}
