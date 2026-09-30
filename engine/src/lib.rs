//! Starscream engine: deterministic fixed-timestep 2D simulation core (Tank Arena first).
//!
//! * Fixed 60 Hz step ([`TICK_HZ`], [`DT`]); no wall clock anywhere.
//! * All randomness from a seeded `ChaCha8Rng` owned by the [`Match`]; policies bring their own.
//! * Headings are integer binary angle units with a compile-time sin/cos table ([`angle`]);
//!   the sim core calls no `sin`/`cos`/`atan2`. Overlap tests use squared distances; the
//!   swept projectile test uses `sqrt`, which IEEE-754 requires to be correctly rounded.
//! * Same seed + same actions → bit-identical state on the same platform ([`Match::state_hash`]).
//! * No OS/threads/time dependencies: builds for `wasm32-unknown-unknown`.
//!
//! Extension points for rule crates: [`TankParams`] (speeds, HP, cooldown, projectile
//! stats), [`MatchConfig`] (arena, obstacles, spawns, tick limit), and per-step
//! [`Event`]s (fired/hit/destroyed). Projectiles are generic straight-line shots with
//! swept (segment) collision, so fast shots cannot tunnel through tanks or obstacles.
//! Seeds in JSON are decimal strings ([`json_u64`]) so JavaScript reads them exactly.
//!
//! Longer-form docs (architecture, tick loop, determinism, replay format, CLI, wasm):
//! `docs/engine/` in the repository.

#![warn(missing_docs)]

pub mod angle;
pub mod arena;
pub mod bots;
pub mod json_u64;
pub mod policy;
pub mod replay;
pub mod sim;

pub use angle::Heading;
pub use arena::{Arena, Rect};
pub use glam::Vec2;
pub use policy::{Action, Observation, Policy, ProjectileObs, SelfObs, TankObs, WallObs};
pub use replay::{Replay, ReplayError, ReplayPlayer};
pub use sim::{
    EndReason, Event, Match, MatchConfig, Outcome, Projectile, Tank, TankParams, TankSpawn,
};

/// Simulation tick rate in Hz. The sim always steps at this fixed rate.
pub const TICK_HZ: u32 = 60;

/// Seconds per tick.
pub const DT: f32 = 1.0 / TICK_HZ as f32;

/// Returns the engine crate version.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;
    use bots::{Chaser, Wanderer};

    #[test]
    fn tick_rate_is_60hz() {
        assert_eq!(TICK_HZ, 60);
        assert!(!version().is_empty());
    }

    fn play(seed: u64) -> Match {
        let mut m = Match::new(MatchConfig::duel(), seed);
        let mut a = Chaser;
        let mut b = Wanderer::new(seed ^ 0x5eed);
        m.run(&mut [&mut a, &mut b]);
        m
    }

    #[test]
    fn same_seed_same_match() {
        for seed in [0, 1, 42, u64::MAX] {
            let a = play(seed);
            let b = play(seed);
            assert_eq!(a.outcome(), b.outcome());
            assert_eq!(a.state_hash(), b.state_hash());
            assert_eq!(a.history(), b.history());
            assert_eq!(a.tanks(), b.tanks());
        }
    }

    #[test]
    fn different_seeds_differ() {
        assert_ne!(play(1).state_hash(), play(2).state_hash());
    }

    #[test]
    fn replay_json_roundtrip_reproduces_match() {
        for seed in [3, 42, 1234] {
            let m = play(seed);
            let json = m.replay().to_json();
            let r = Replay::from_json(&json).expect("parses");
            let back = r.verify().expect("reproduces");
            assert_eq!(back.outcome(), m.outcome());
            assert_eq!(back.state_hash(), m.state_hash());
            assert_eq!(back.tanks(), m.tanks());
            assert_eq!(back.projectiles(), m.projectiles());
        }
    }

    #[test]
    fn replay_seed_is_a_string_and_numbers_still_load() {
        let m = play(u64::MAX);
        let json = m.replay().to_json();
        assert!(json.contains(r#""seed":"18446744073709551615""#), "{json}");
        Replay::from_json(&json)
            .unwrap()
            .verify()
            .expect("reproduces");
        // Older writers (and hand-written JSON) used a plain number: still accepted.
        let numeric = json.replace(
            r#""seed":"18446744073709551615""#,
            r#""seed":18446744073709551615"#,
        );
        let r = Replay::from_json(&numeric).expect("numeric seed parses");
        assert_eq!(r.seed, u64::MAX);
        r.verify().expect("reproduces");
    }

    #[test]
    fn tampered_replay_is_detected() {
        let m = play(42);
        let mut r = m.replay();
        for a in r.actions.iter_mut().take(200) {
            a[0].throttle = -1.0;
            a[0].fire = false;
        }
        assert!(r.verify().is_err());
    }

    #[test]
    fn replay_player_steps_to_end() {
        let m = play(9);
        let mut p = ReplayPlayer::new(m.replay());
        let mut n = 0;
        while p.step() {
            n += 1;
        }
        assert!(p.is_finished());
        assert_eq!(n, m.tick());
        assert_eq!(p.state().state_hash(), m.state_hash());
    }

    #[test]
    fn matches_terminate_with_some_winners() {
        let mut winners = 0;
        for seed in 0..10 {
            let o = play(seed).outcome().expect("ended");
            assert!(o.ticks <= MatchConfig::duel().max_ticks);
            winners += o.winner.is_some() as u32;
        }
        assert!(winners > 0, "placeholder bots should decide some matches");
    }
}
