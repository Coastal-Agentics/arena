//! GATE-003 M1 acceptance (`docs/plans/GATE-003-learning-tanks.md` §7).
//!
//! `fixtures/m1-champion.json` is the `champion.json` written by
//! `cargo run -p tank --release --example evolve -- run --out DIR --seed 1 --generations 100`.
//!
//! - In CI: the pinned champion replays **all 1,000 held-out seeds** against Gen 0 (the
//!   scripted defaults) with exactly the recorded wins, draws, losses and digest, and
//!   wins at least 65%; plus a small two-run determinism check of the GA itself.
//! - Ignored (run with `cargo test -p tank --release --test evolve_m1 -- --ignored`):
//!   the full 100-generation run from seed 1 reproduces that champion and its hashes.

use serde_json::Value;
use tank::evolve::{self, Config, Genome, State};

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/m1-champion.json")).expect("fixture is JSON")
}

fn threads() -> usize {
    std::thread::available_parallelism().map_or(4, |n| n.get())
}

#[test]
fn pinned_champion_beats_gen0_on_1000_unseen_seeds() {
    let f = fixture();
    let g = Genome::from_json(&f["genome"]).expect("fixture genome is valid");
    assert!(tank::Loadout::ALL.contains(&g.loadout));
    let h = evolve::held_out(&g, evolve::HELDOUT_SEEDS, threads());
    assert_eq!(h.games(), 6_000, "1,000 seeds x 3 opponents x 2 sides");
    assert_eq!(
        h.to_json(),
        f["held_out"],
        "replay differs from the recorded run"
    );
    assert!(
        h.win_rate() >= evolve::ACCEPT_WIN_RATE,
        "Gen {} wins {:.1}% vs Gen 0",
        f["generation"],
        100.0 * h.win_rate()
    );
    // Above 70% vs the scripted field: held from promotion (resolved Q2).
    assert_eq!(h.status(), "experimental");
}

#[test]
fn same_seed_gives_byte_identical_files() {
    let cfg = Config {
        seed: 11,
        population: 6,
        elite: 2,
        tournament: 2,
        train_seeds: 1,
        hall_of_fame: 2,
        ..Config::default()
    };
    let run = |threads| {
        let mut s = State::new(cfg.clone()).unwrap();
        let hist: Vec<String> = (0..2)
            .map(|_| s.step(threads).to_json().to_string())
            .collect();
        let champ = s.hall_of_fame.last().unwrap().clone();
        let h = evolve::held_out(&champ, 3, threads);
        (s.to_json().to_string(), hist, h.to_json().to_string())
    };
    assert_eq!(run(1), run(3));
}

#[test]
#[ignore = "full M1 run: ~15 CPU-minutes; use --release"]
fn full_run_from_seed_1_reproduces_the_pinned_champion() {
    let f = fixture();
    let cfg = Config::from_json(&f["config"]).unwrap();
    assert_eq!(cfg, Config::default());
    let gens = f["generation"].as_u64().unwrap() as u32 + 1;
    let mut s = State::new(cfg).unwrap();
    let mut last = None;
    for _ in 0..gens {
        last = Some(s.step(threads()));
    }
    let g = last.unwrap();
    assert_eq!(g.champion, Genome::from_json(&f["genome"]).unwrap());
    let h = evolve::held_out(&g.champion, evolve::HELDOUT_SEEDS, threads());
    assert_eq!(h.to_json(), f["held_out"]);
    assert!(h.win_rate() >= evolve::ACCEPT_WIN_RATE);
}
