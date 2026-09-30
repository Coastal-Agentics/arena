//! Coastal Agentics Arena engine: deterministic fixed-timestep 2D simulation core
//! (Tank Arena first).
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
//! stats, optional stationary accuracy), shared in [`MatchConfig::params`] or per tank in
//! [`TankSpawn::params`]; [`MatchConfig`] (arena, obstacles, spawns, tick limit);
//! [`Arena::segment_clear`] (line of sight, also reported as [`TankObs::los`]); and
//! per-step [`Event`]s (fired/hit/destroyed). Projectiles are generic straight-line shots with
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
    use replay::REPLAY_FORMAT;
    use serde_json::json;

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

    /// Re-serialize a replay's JSON after editing it as a `serde_json::Value`.
    fn edit(json: &str, f: impl FnOnce(&mut serde_json::Value)) -> String {
        let mut v: serde_json::Value = serde_json::from_str(json).unwrap();
        f(&mut v);
        v.to_string()
    }

    #[test]
    fn edited_config_fails_verification() {
        // Regression: in format 2, deleting the obstacles from the seed-7 duel still
        // verified, because nothing in that match touches them and the state hash
        // doesn't cover the config.
        let m = play(7);
        let json = m.replay().to_json();
        let no_obstacles = edit(&json, |v| v["config"]["arena"]["obstacles"] = json!([]));
        let r = Replay::from_json(&no_obstacles).expect("still parses");
        assert!(r.config.arena.obstacles.is_empty());
        // The gap is real: the edited config re-simulates to the same outcome and state...
        let edited = r.play();
        assert_eq!(edited.outcome(), m.outcome());
        assert_eq!(edited.state_hash(), m.state_hash());
        // ...but verification now catches the edit.
        assert!(
            matches!(r.verify(), Err(ReplayError::SetupMismatch { .. })),
            "{:?}",
            r.verify().err()
        );

        // Any other config field, and the seed, are covered too.
        type Edit = (&'static str, fn(&mut serde_json::Value));
        let edits: [Edit; 7] = [
            ("arena size", |v| {
                v["config"]["arena"]["size"] = json!([801.0, 600.0])
            }),
            ("tank param", |v| {
                v["config"]["params"]["max_hp"] = json!(101)
            }),
            ("spread", |v| {
                v["config"]["params"]["projectile_spread"] = json!(0)
            }),
            ("spawn", |v| v["config"]["tanks"][0]["heading"] = json!(0)),
            ("team", |v| v["config"]["tanks"][1]["team"] = json!(2)),
            ("max_ticks", |v| v["config"]["max_ticks"] = json!(7201)),
            ("seed", |v| v["seed"] = json!("8")),
        ];
        for (what, f) in edits {
            let r = Replay::from_json(&edit(&json, f)).expect(what);
            assert!(
                matches!(r.verify(), Err(ReplayError::SetupMismatch { .. })),
                "{what}: {:?}",
                r.verify().err()
            );
        }
    }

    #[test]
    fn setup_hash_ignores_json_formatting() {
        let json = play(7).replay().to_json();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        let pretty = serde_json::to_string_pretty(&v).unwrap();
        // `16` instead of `16.0`, and an explicit `null` spawn left out, parse to the
        // same config, so they hash the same.
        let reworded = pretty
            .replace("\"radius\": 16.0", "\"radius\": 16")
            .replace("\"pos\": null,", "");
        assert_ne!(reworded, pretty);
        Replay::from_json(&reworded)
            .unwrap()
            .verify()
            .expect("verifies");
    }

    #[test]
    fn format_2_replays_still_load_and_upgrade() {
        let m = play(7);
        let v4 = m.replay().to_json();
        assert!(v4.contains(r#""format":4"#), "{v4}");
        let v2 = edit(&v4, |v| {
            v["format"] = json!(2);
            v.as_object_mut().unwrap().remove("setup_hash");
        });
        let r = Replay::from_json(&v2).expect("format 2 loads");
        assert_eq!((r.format, r.setup_hash.as_deref()), (2, None));
        let out = r.to_json();
        assert!(
            out.contains(r#""format":2"#) && !out.contains("setup_hash"),
            "{out}"
        );
        let back = r
            .verify()
            .expect("v2 verifies (outcome and final hash only)");
        assert_eq!(back.state_hash(), m.state_hash());
        // Documented limit: without a setup hash, v2 can't see config edits.
        let v2_edited = edit(&v2, |v| v["config"]["arena"]["obstacles"] = json!([]));
        assert!(Replay::from_json(&v2_edited).unwrap().verify().is_ok());
        // Upgrade: verify, then re-record.
        let up = back.replay();
        assert_eq!(up.format, REPLAY_FORMAT);
        assert_eq!(up.setup_hash, m.replay().setup_hash);
    }

    #[test]
    fn replay_format_bounds() {
        let json = play(7).replay().to_json();
        let missing = edit(&json, |v| {
            v.as_object_mut().unwrap().remove("setup_hash");
        });
        assert_eq!(
            Replay::from_json(&missing),
            Err(ReplayError::MissingSetupHash)
        );
        for f in [0, 1, 5] {
            let other = edit(&json, |v| v["format"] = json!(f));
            assert_eq!(Replay::from_json(&other), Err(ReplayError::Format(f)));
        }
    }

    #[test]
    fn documented_hashes_are_unchanged() {
        // Values quoted in docs/engine (engine-cli.md, replay-format.md). Default
        // configs play exactly as before the per-tank params and stationary accuracy
        // asks, and serialize the same, so setup hashes are unchanged too.
        let cases: [(u64, u32, &str, &str); 4] = [
            (42, 447, "03722b5e86d38fac", "-"),
            (7, 276, "51234f61b02b5784", "0b24ce74f45e9a27"),
            (101, 274, "baf3fcb2cbb76c06", "9cfd58498bbe3f85"),
            (u64::MAX, 532, "f1d983e88de5d020", "-"),
        ];
        for (seed, ticks, hash, setup) in cases {
            for _ in 0..2 {
                let m = play(seed);
                assert_eq!(
                    m.outcome().map(|o| (o.winner, o.ticks)),
                    Some((Some(1), ticks))
                );
                let r = m.replay();
                assert_eq!(r.final_hash, hash, "seed {seed}");
                if setup != "-" {
                    assert_eq!(r.setup_hash.as_deref(), Some(setup), "seed {seed}");
                }
            }
        }
    }

    /// Glass Cannon (A/S/D 5/3/1) vs Brawler (4/1/4), mapped with the spec's table.
    /// The mapping itself is game rules and belongs to `games/tank`; this is test data.
    fn loadout_config() -> MatchConfig {
        let mut c = MatchConfig::duel();
        c.tanks[0].params = Some(TankParams {
            projectile_damage: 28,
            max_hp: 60,
            ..Default::default()
        });
        c.tanks[1].params = Some(TankParams {
            projectile_damage: 24,
            max_speed: 90.0,
            turn_rate: 273,
            max_hp: 120,
            ..Default::default()
        });
        c
    }

    fn play_config(config: MatchConfig, seed: u64) -> Match {
        let mut m = Match::new(config, seed);
        m.run(&mut [&mut Chaser, &mut Wanderer::new(seed ^ 0x5eed)]);
        m
    }

    #[test]
    fn default_params_per_tank_play_like_shared_params() {
        // Spec acceptance: 3/3/3 vs 3/3/3 is bit-identical to the default match.
        for seed in [0, 7, 42, u64::MAX] {
            let mut c = MatchConfig::duel();
            for t in &mut c.tanks {
                t.params = Some(TankParams::default());
            }
            let (a, b) = (play(seed), play_config(c, seed));
            assert_eq!(a.outcome(), b.outcome());
            assert_eq!(a.history(), b.history());
            assert_eq!(a.state_hash(), b.state_hash());
            // Different config, so a different setup hash.
            assert_ne!(a.replay().setup_hash, b.replay().setup_hash);
        }
    }

    #[test]
    fn per_tank_params_are_deterministic_and_change_the_match() {
        let mut changed = 0;
        for seed in [1, 7, 42] {
            let a = play_config(loadout_config(), seed);
            let b = play_config(loadout_config(), seed);
            assert_eq!(a.state_hash(), b.state_hash());
            assert_eq!(a.history(), b.history());
            assert_eq!(a.outcome(), b.outcome());
            changed += (a.state_hash() != play(seed).state_hash()) as u32;
        }
        assert_eq!(changed, 3, "a loadout change alters the sim");
    }

    #[test]
    fn per_tank_params_replay_roundtrip_and_verify() {
        let mut cfg = loadout_config();
        cfg.params.projectile_spread_still = Some(128);
        cfg.tanks[1]
            .params
            .as_mut()
            .unwrap()
            .projectile_spread_still = Some(128);
        let m = play_config(cfg, 42);
        let json = m.replay().to_json();
        assert!(json.contains(r#""format":4"#), "{json}");
        assert!(json.contains(r#""params":{"radius""#), "{json}");
        assert!(json.contains(r#""projectile_spread_still":128"#), "{json}");
        let r = Replay::from_json(&json).expect("parses");
        assert_eq!(r.config.tanks[1].params.as_ref().unwrap().max_hp, 120);
        let back = r.verify().expect("reproduces");
        assert_eq!(back.state_hash(), m.state_hash());
        assert_eq!(back.outcome(), m.outcome());
        assert_eq!(back.tank_params(1), m.tank_params(1));
        assert_eq!(Replay::from_json(&back.replay().to_json()).unwrap(), r);
        // The setup hash covers the per-tank params and stationary accuracy.
        type Edit = (&'static str, fn(&mut serde_json::Value));
        let edits: [Edit; 5] = [
            ("per-tank hp", |v| {
                v["config"]["tanks"][1]["params"]["max_hp"] = json!(121)
            }),
            ("per-tank damage", |v| {
                v["config"]["tanks"][0]["params"]["projectile_damage"] = json!(27)
            }),
            ("drop per-tank params", |v| {
                v["config"]["tanks"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("params");
            }),
            ("add per-tank params", |v| {
                v["config"]["tanks"][0]["params"] = v["config"]["params"].clone()
            }),
            ("still spread", |v| {
                v["config"]["tanks"][1]["params"]["projectile_spread_still"] = json!(0)
            }),
        ];
        for (what, f) in edits {
            let r = Replay::from_json(&edit(&json, f)).expect(what);
            assert!(
                matches!(r.verify(), Err(ReplayError::SetupMismatch { .. })),
                "{what}: {:?}",
                r.verify().err()
            );
        }
    }

    #[test]
    fn format_3_replays_still_load_and_verify() {
        let m = play(7);
        let v3 = edit(&m.replay().to_json(), |v| v["format"] = json!(3));
        let r = Replay::from_json(&v3).expect("format 3 loads");
        assert_eq!(r.format, 3);
        assert_eq!(r.setup_hash, m.replay().setup_hash, "same config bytes");
        assert!(r.to_json().contains(r#""format":3"#));
        let back = r.verify().expect("format 3 verifies, setup hash included");
        assert_eq!(back.state_hash(), m.state_hash());
        assert_eq!(back.replay().format, REPLAY_FORMAT);
        // Formats 2 and 3 have no per-tank params or stationary accuracy.
        let v4 = play_config(loadout_config(), 7).replay().to_json();
        for format in [2, 3] {
            let old = edit(&v4, |v| v["format"] = json!(format));
            assert_eq!(
                Replay::from_json(&old),
                Err(ReplayError::FieldNotInFormat {
                    format,
                    field: "tanks[].params"
                })
            );
            let still = edit(&v3, |v| {
                v["format"] = json!(format);
                v["config"]["params"]["projectile_spread_still"] = json!(128);
            });
            assert_eq!(
                Replay::from_json(&still),
                Err(ReplayError::FieldNotInFormat {
                    format,
                    field: "params.projectile_spread_still"
                })
            );
        }
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
