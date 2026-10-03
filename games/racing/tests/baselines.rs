//! R2 (racing.md "Scripted baselines", "Acceptance"): the Follower, Cutter and
//! Blocker. Always on: each finishes alone on 1,000 seeds, the 4-car numbers on a
//! smaller seed set, the reward-sum and no-overlap checks with the baselines driving,
//! the catalog ids, and driver determinism. The full acceptance run (the numbers in
//! BALANCE.md) is ignored in debug builds:
//! `cargo test -p racing --release --test baselines -- --ignored`.

use engine::generic::{Policy, Rules};
use racing::balance::{field, mirror, mixed, mixed_lineups, par_map, run, solo, threads, Field};
use racing::catalog::catalog;
use racing::drivers::Behavior;
use racing::*;

const G: f64 = 9.0;

#[test]
fn every_baseline_finishes_all_three_laps_alone_on_1000_seeds() {
    for b in Behavior::ALL {
        let s = solo(b, Setup::BALANCED, 1000);
        assert_eq!(s.finished, 1000, "{b:?}: {s:?}");
        assert!((20.0..=30.0).contains(&s.median_s), "{b:?}: {s:?}");
    }
}

fn check_field(name: &str, f: &Field) {
    assert!(
        f.finished as f64 >= 0.95 * f.entries as f64,
        "{name}: {}/{} finished",
        f.finished,
        f.entries
    );
    assert!(
        (20.0..=40.0).contains(&f.median_s),
        "{name}: median {}",
        f.median_s
    );
    assert!(
        (f.capped as f64) < 0.05 * f.races as f64,
        "{name}: {} capped",
        f.capped
    );
}

#[test]
fn four_car_races_finish_in_time_on_20_seeds() {
    for b in Behavior::ALL {
        check_field(b.name(), &field(&mirror(b, 20)));
    }
    let m = field(&mixed(20));
    assert_eq!(m.races, 240);
    assert_eq!(
        m.entries_by_driver, [320; 3],
        "every driver drives as often"
    );
    check_field("mixed", &m);
}

#[test]
#[ignore = "the BALANCE.md acceptance run; use --release"]
fn acceptance_targets_hold_on_200_seeds() {
    let fs = solo(Behavior::Follower, Setup::BALANCED, 1000);
    let cs = solo(Behavior::Cutter, Setup::BALANCED, 1000);
    assert!(
        cs.median_s < fs.median_s,
        "Cutter {cs:?} vs Follower {fs:?}"
    );
    for b in Behavior::ALL {
        let f = field(&mirror(b, 200));
        check_field(b.name(), &f);
        let top = *f.wins_by_slot.iter().max().unwrap();
        assert!(
            top * 100 <= 60 * f.races,
            "{b:?} mirror slots {:?}",
            f.wins_by_slot
        );
    }
    let m = field(&mixed(200));
    check_field("mixed", &m);
    let top = *m.wins_by_driver.iter().max().unwrap();
    assert!(
        top * 100 <= 70 * m.races,
        "mixed wins {:?}",
        m.wins_by_driver
    );
}

/// Runs one race of `lineup` stepping by hand; checks every tick that no car overlaps
/// a wall or another (unfinished) car, and at the end that each car's rewards sum to
/// final progress ÷ gates plus its bonus.
fn checked_race(cfg: &RacingConfig, lineup: &[Behavior], seed: u64) {
    let n = lineup.len();
    let r = cfg.physics.radius;
    let geom = TrackGeom::new(&cfg.track);
    let mut m = Race::new(cfg.clone(), seed);
    let mut ps: Vec<Box<dyn Policy<RacingRules>>> = lineup
        .iter()
        .enumerate()
        .map(|(i, b)| b.driver(cfg, i, seed))
        .collect();
    let mut sums = vec![0.0f64; n];
    while !m.is_over() {
        let mut refs: Vec<&mut dyn Policy<RacingRules>> = ps
            .iter_mut()
            .map(|p| p.as_mut() as &mut dyn Policy<RacingRules>)
            .collect();
        m.step_policies(&mut refs);
        for (a, s) in sums.iter_mut().enumerate() {
            *s += m.reward(a) as f64;
        }
        let cars = &m.state().cars;
        for (i, a) in cars.iter().enumerate() {
            assert!(
                geom.on_track(a.pos) && geom.wall_distance(a.pos) >= r - 1e-3,
                "seed {seed} tick {} car {i} in a wall",
                m.tick()
            );
            for b in &cars[i + 1..] {
                if !a.finished() && !b.finished() {
                    assert!(
                        (a.pos - b.pos).length() >= 2.0 * r - 1e-3,
                        "seed {seed} tick {}",
                        m.tick()
                    );
                }
            }
        }
    }
    for (a, c) in m.state().cars.iter().enumerate() {
        let bonus = if c.finished() {
            RacingRules::finish_bonus(c.place, n) as f64
        } else {
            0.0
        };
        let want = c.progress as f64 / G + bonus;
        assert!(
            (sums[a] - want).abs() < 1e-4,
            "seed {seed} car {a}: {} vs {want}",
            sums[a]
        );
    }
}

#[test]
fn baseline_races_never_overlap_and_rewards_sum_to_progress_plus_bonus() {
    let [f, c, b] = Behavior::ALL;
    let mut lineups: Vec<Vec<Behavior>> = mixed_lineups().to_vec();
    lineups.extend([vec![f; 4], vec![c; 4], vec![b; 4], vec![c, f], vec![b]]);
    let seeds: Vec<u64> = (0..24).collect();
    par_map(&seeds, threads(), |seed| {
        for lineup in &lineups {
            let cfg = RacingConfig::ring_setups(&vec![Setup::BALANCED; lineup.len()]);
            checked_race(&cfg, lineup, seed);
        }
    });
}

#[test]
fn the_catalog_lists_exactly_the_three_drivers() {
    let keys: Vec<&str> = Behavior::ALL.iter().map(|b| b.key()).collect();
    assert_eq!(keys, racing::catalog::BEHAVIORS);
    let c = catalog();
    assert_eq!(c.behaviors, keys);
    for b in Behavior::ALL {
        assert_eq!(Behavior::from_key(b.key()), Some(b));
    }
    assert_eq!(Behavior::from_key("rammer"), None);
}

#[test]
fn drivers_are_deterministic_and_the_seed_moves_their_jitter() {
    let [f, c, b] = Behavior::ALL;
    let lineup = [f, c, b, f];
    let cfg = RacingConfig::ring_setups(&[Setup::BALANCED; 4]);
    let a = run(&cfg, &lineup, 5);
    assert_eq!(a, run(&cfg, &lineup, 5));
    // Replays of baseline races verify and are byte-identical across runs.
    let race = |seed| {
        let mut m = Race::new(cfg.clone(), seed);
        let mut ps: Vec<Box<dyn Policy<RacingRules>>> = lineup
            .iter()
            .enumerate()
            .map(|(i, d)| d.driver(&cfg, i, seed))
            .collect();
        let mut refs: Vec<&mut dyn Policy<RacingRules>> = ps
            .iter_mut()
            .map(|p| p.as_mut() as &mut dyn Policy<RacingRules>)
            .collect();
        m.run(&mut refs);
        m
    };
    let (m1, m2) = (race(9), race(9));
    assert_eq!(m1.replay().to_json(), m2.replay().to_json());
    let v = RaceReplay::from_json(&m1.replay().to_json())
        .unwrap()
        .verify()
        .unwrap();
    assert_eq!(v.state_hash(), m1.state_hash());
    assert_eq!(m1.replay().game.as_deref(), Some(RacingRules::GAME));
    // Solo, the seed only moves the driver's jitter: different seeds, different times.
    let solo_cfg = RacingConfig::ring_setups(&[Setup::BALANCED]);
    let times: std::collections::BTreeSet<u32> = (0..10)
        .map(|s| run(&solo_cfg, &[c], s).finish[0].unwrap())
        .collect();
    assert!(times.len() > 5, "{times:?}");
}
