//! Regenerate the native-vs-wasm parity fixtures in `engine-wasm/tests/parity/`:
//!
//! ```text
//! cargo run -p engine-wasm --example parity_fixtures
//! ```
//!
//! Plays each match below natively, writes its replay (Tank Arena's format, 4) and
//! rewrites `manifest.json` with the expected ticks, outcome, final hash and setup hash.
//! The output is deterministic: running it twice gives byte-identical files.
//!
//! Only regenerate on a bump of the format Tank Arena writes (`TankRules::WRITES_FORMAT`),
//! a deliberate rule change, or a refresh after scripted-policy changes (the `arena-*`
//! files), and say so in the PR (see `docs/engine/determinism.md`, "Native-vs-wasm
//! parity"). The replays store actions, not policies, so a policy change in `games/tank`
//! does not invalidate them; it only means a regenerated file would differ.

use engine::{Match, MatchConfig, Policy, Replay, TankParams};
use serde::Serialize;
use std::path::Path;
use tank::{rules, Behavior, Chaser, Loadout, MatchSpec, Wanderer};

/// One fixture: file name, why it exists, and the match to record.
struct Fixture {
    file: &'static str,
    why: &'static str,
    play: fn() -> Match,
}

/// Chaser (team 0) vs Wanderer (team 1), seeded as `engine-cli` does.
fn chaser_vs_wanderer(config: MatchConfig, seed: u64) -> Match {
    let mut m = Match::new(config, seed);
    let mut a = Chaser;
    let mut b = Wanderer::new(seed ^ 0x5eed);
    m.run(&mut [&mut a, &mut b]);
    m
}

/// A Tank Arena match exactly as `WasmMatch.tank(query)` plays it.
fn arena_query(query: &str) -> Match {
    let spec = MatchSpec::from_query(query).expect("valid query");
    let (mut m, [mut a, mut b]) = spec.start();
    m.run(&mut [a.as_mut(), b.as_mut()]);
    m
}

/// Tank Arena policies, one per tank; policy `i` is seeded `seed ^ (0x9a_0000 + i)`.
fn arena_policies(config: MatchConfig, seed: u64, behaviors: &[Behavior]) -> Match {
    let mut m = Match::new(config, seed);
    let mut ps: Vec<Box<dyn Policy>> = behaviors
        .iter()
        .enumerate()
        .map(|(i, b)| b.build(seed ^ (0x9a_0000 + i as u64)))
        .collect();
    let mut refs: Vec<&mut dyn Policy> = ps
        .iter_mut()
        .map(|p| p.as_mut() as &mut dyn Policy)
        .collect();
    m.run(&mut refs);
    m
}

const FIXTURES: &[Fixture] = &[
    Fixture {
        file: "cw-seed-max.json",
        why: "Chaser vs Wanderer on MatchConfig::duel at seed u64::MAX (18446744073709551615): \
              the largest seed, not representable as a JS number, so it must survive as a \
              decimal string in both engines; ends last_standing.",
        play: || chaser_vs_wanderer(MatchConfig::duel(), u64::MAX),
    },
    Fixture {
        file: "cw-all-destroyed.json",
        why: "Chaser vs Wanderer, seed 2916: both tanks die on the same tick, so the match \
              ends as an all_destroyed draw (winner null).",
        play: || chaser_vs_wanderer(MatchConfig::duel(), 2916),
    },
    Fixture {
        file: "cw-per-tank-params.json",
        why: "Per-tank params (replay format 4): the documented withConfig example, \
              Glass Cannon 28 dmg / 60 HP vs Brawler 24 dmg, 90 u/s, 273 BAU/tick, 120 HP, \
              seed 42 (docs/engine/wasm-and-web.md, determinism.md).",
        play: || {
            let mut c = MatchConfig::duel();
            let base = c.params.clone();
            c.tanks[0].params = Some(TankParams {
                projectile_damage: 28,
                max_hp: 60,
                ..base.clone()
            });
            c.tanks[1].params = Some(TankParams {
                projectile_damage: 24,
                max_speed: 90.0,
                turn_rate: 273,
                max_hp: 120,
                ..base
            });
            chaser_vs_wanderer(c, 42)
        },
    },
    Fixture {
        file: "cw-spread-still.json",
        why: "projectile_spread_still = Some(0) (format 4): a still tank's shot draws no \
              RNG, so the RNG stream depends on movement. Seed 28 ends at tick 263 instead \
              of 305 without it.",
        play: || {
            let mut c = MatchConfig::duel();
            c.params.projectile_spread_still = Some(0);
            chaser_vs_wanderer(c, 28)
        },
    },
    Fixture {
        file: "arena-charger-mirror.json",
        why: "Tank Arena rules v1 as WasmMatch.tank plays it: \
              seed=2&blue=charger-5-3-1&orange=charger-5-3-1. Loadouts as per-tank params, \
              fixed spawns, the pillar arena; ends last_standing (orange) at tick 816, the \
              winner on 54 HP.",
        play: || arena_query("seed=2&blue=charger-5-3-1&orange=charger-5-3-1"),
    },
    Fixture {
        file: "arena-sniper-vs-charger.json",
        why: "Tank Arena: seed=0&blue=sniper-5-3-1&orange=charger-5-3-1. The sniper holds \
              a spot with a sight line and fires only when it sees the target, so its shots \
              depend on the pillars; ends last_standing (blue, the sniper) at tick 1023.",
        play: || arena_query("seed=0&blue=sniper-5-3-1&orange=charger-5-3-1"),
    },
    Fixture {
        file: "arena-2v2-tick-limit.json",
        why: "Tank Arena 2v2 corner spawns (four tanks, teammates, no friendly fire), \
              charger/kiter/sniper/charger with loadouts 5-3-1, 2-5-2, 3-1-5, 4-4-1, \
              seed 1, max_ticks cut to 360 so it ends at the tick_limit (a draw) while \
              small. Two tanks (both chargers) have taken hits by then.",
        play: || {
            let loadouts: Vec<Loadout> = ["5-3-1", "2-5-2", "3-1-5", "4-4-1"]
                .iter()
                .map(|s| s.parse().expect("valid loadout"))
                .collect();
            let mut setup = rules::with_loadouts(rules::config(rules::Mode::TwoVTwo), &loadouts);
            setup.config.max_ticks = 360;
            arena_policies(
                setup.config,
                1,
                &[
                    Behavior::Charger,
                    Behavior::Kiter,
                    Behavior::Sniper,
                    Behavior::Charger,
                ],
            )
        },
    },
];

#[derive(Serialize)]
struct Outcome {
    winner: Option<u8>,
    ticks: u32,
    reason: engine::EndReason,
}

#[derive(Serialize)]
struct Entry {
    file: &'static str,
    why: &'static str,
    format: u32,
    #[serde(with = "engine::json_u64")]
    seed: u64,
    tanks: usize,
    ticks: u32,
    outcome: Option<Outcome>,
    final_hash: String,
    setup_hash: String,
    bytes: usize,
}

#[derive(Serialize)]
struct Manifest {
    about: &'static str,
    regenerate: &'static str,
    fixtures: Vec<Entry>,
}

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/parity");
    std::fs::create_dir_all(&dir).expect("create tests/parity");
    let mut fixtures = Vec::new();
    for f in FIXTURES {
        let m = (f.play)();
        let replay: Replay = m.replay();
        let json = replay.to_json();
        std::fs::write(dir.join(f.file), &json).expect("write fixture");
        fixtures.push(Entry {
            file: f.file,
            why: f.why,
            format: replay.format,
            seed: replay.seed,
            tanks: m.tanks().len(),
            ticks: m.tick(),
            outcome: m.outcome().map(|o| Outcome {
                winner: o.winner,
                ticks: o.ticks,
                reason: o.reason,
            }),
            final_hash: replay.final_hash.clone(),
            setup_hash: replay
                .setup_hash
                .clone()
                .expect("current format has setup_hash"),
            bytes: json.len(),
        });
        println!("{} {} ticks, {} bytes", f.file, m.tick(), json.len());
    }
    let manifest = Manifest {
        about: "Pinned replays for the native-vs-wasm parity check. Expected values were \
                recorded natively; engine-wasm/tests/parity.rs (native) and \
                scripts/check-parity.mjs (web/pkg) re-simulate every file and compare.",
        regenerate: "cargo run -p engine-wasm --example parity_fixtures",
        fixtures,
    };
    let mut out = serde_json::to_string_pretty(&manifest).expect("manifest serializes");
    out.push('\n');
    std::fs::write(dir.join("manifest.json"), out).expect("write manifest");
}
