//! R1 (racing.md "Acceptance", "Determinism and replays"): the race rules on the Ring:
//! physics, gates and laps, the end of a race, placings, the reward, the flat view,
//! determinism and replays, and the exploit tests. Cars are driven by test-only
//! drivers (`common`); the baselines come in R2.

mod common;

use common::{v, LineDriver, Wild};
use engine::angle::from_degrees;
use engine::generic::{EndReason, Flat, Policy, ReplayError, Rules};
use racing::config::{CarParams, Physics};
use racing::end::FINISHED_REASON;
use racing::flat::*;
use racing::*;

const G: usize = 9;

fn line_drivers(cfg: &RacingConfig) -> Vec<LineDriver> {
    (0..cfg.cars.len())
        .map(|i| {
            let mut d = LineDriver::new(cfg);
            d.offset = [0.0, 20.0, -20.0, 10.0][i];
            d
        })
        .collect()
}

fn run_line(cfg: &RacingConfig, seed: u64) -> Race {
    let mut m = Race::new(cfg.clone(), seed);
    let mut ds = line_drivers(cfg);
    let mut ps: Vec<&mut dyn Policy<RacingRules>> = ds
        .iter_mut()
        .map(|d| d as &mut dyn Policy<RacingRules>)
        .collect();
    m.run(&mut ps);
    m
}

fn four() -> RacingConfig {
    RacingConfig::ring_setups(&[Setup::BALANCED; 4])
}

fn solo() -> RacingConfig {
    RacingConfig::ring_setups(&[Setup::BALANCED])
}

/// A started car (next gate 1) placed by hand, for the gate tests.
fn state_with_car(
    cfg: &RacingConfig,
    pos: engine::Vec2,
    heading: u16,
    crossings: u32,
) -> RaceState {
    let mut s = RaceState::start(cfg, &[0]);
    let c = &mut s.cars[0];
    c.pos = pos;
    c.heading = heading;
    c.crossings = crossings;
    c.next_gate = crossings % G as u32;
    c.laps = crossings.saturating_sub(1) / G as u32;
    s
}

fn drive(cfg: &RacingConfig, s: &mut RaceState, a: RaceAction, ticks: u32) -> Vec<RaceEvent> {
    let mut all = Vec::new();
    for _ in 0..ticks {
        let mut ev = Vec::new();
        RacingRules::advance(cfg, s, &[a], &mut ev);
        all.extend(ev);
    }
    all
}

const GO: RaceAction = RaceAction {
    throttle: 1.0,
    steer: 0.0,
};
const BACK: RaceAction = RaceAction {
    throttle: -1.0,
    steer: 0.0,
};

// ---------------------------------------------------------------- physics

#[test]
fn a_standing_car_cannot_turn_and_speed_is_capped() {
    let cfg = solo();
    let mut s = RaceState::start(&cfg, &[0]);
    let h0 = s.cars[0].heading;
    drive(
        &cfg,
        &mut s,
        RaceAction {
            throttle: 0.0,
            steer: 1.0,
        },
        30,
    );
    assert_eq!(s.cars[0].heading, h0, "no spinning in place");
    assert_eq!(s.cars[0].pos, v(370.0, 120.0));
    // Full throttle on the bottom straight: 240 u/s² reaches 240 u/s in one second.
    let mut s = state_with_car(&cfg, v(260.0, 100.0), 0, 1);
    drive(&cfg, &mut s, GO, 60);
    assert!((s.cars[0].vel.x - 240.0).abs() < 1e-3, "{}", s.cars[0].vel);
    drive(&cfg, &mut s, GO, 5);
    assert!(s.cars[0].vel.length() <= 240.0);
    // Coasting loses 60 u/s per second; braking stops, then reverses up to 60 u/s.
    let v0 = s.cars[0].vel.x;
    drive(&cfg, &mut s, RaceAction::default(), 30);
    assert!((s.cars[0].vel.x - (v0 - 30.0)).abs() < 1e-2);
    drive(&cfg, &mut s, BACK, 120);
    assert!((s.cars[0].vel.x + 60.0).abs() < 1e-3, "{}", s.cars[0].vel);
}

#[test]
fn steering_factor_and_grip() {
    let ph = Physics::default();
    let p = Setup::BALANCED.params();
    assert_eq!(RacingRules::steer_factor(0.0, &p, &ph), 0.0);
    assert_eq!(RacingRules::steer_factor(30.0, &p, &ph), 0.5);
    assert_eq!(RacingRules::steer_factor(60.0, &p, &ph), 1.0);
    assert_eq!(RacingRules::steer_factor(240.0, &p, &ph), 0.5);
    assert_eq!(RacingRules::steer_factor(300.0, &p, &ph), 0.5);
    // Sideways speed loses `grip` of itself per tick (with no throttle on it).
    let mut car = RaceState::start(&solo(), &[0]).cars[0];
    car.vel = v(0.0, 100.0);
    RacingRules::drive(&mut car, RaceAction::default(), &p, &ph);
    assert!((car.vel.y - 75.0).abs() < 1e-4, "{}", car.vel);
}

#[test]
fn scraping_a_wall_costs_speed_and_never_leaves_the_car_inside() {
    let cfg = solo();
    // Bottom straight, aimed 30° into the outer wall (y = 40).
    let mut s = state_with_car(&cfg, v(250.0, 60.0), from_degrees(-30), 1);
    s.cars[0].vel = engine::angle::dir(from_degrees(-30)) * 200.0;
    let ev = drive(&cfg, &mut s, RaceAction::default(), 20);
    assert!(ev.iter().any(|e| matches!(e, RaceEvent::Wall { car: 0 })));
    let c = s.cars[0];
    assert!(c.pos.y >= 40.0 + 12.0 - 1e-3, "{}", c.pos);
    assert!(c.vel.y >= -1e-3, "no speed into the wall");
    assert!(
        c.vel.length() < 200.0 * 0.6 + 1.0,
        "scrape cost 40%: {}",
        c.vel
    );
}

// ---------------------------------------------------------------- gates, laps, exploits

#[test]
fn reversing_over_the_line_does_not_count_a_lap() {
    let cfg = solo();
    // End of lap 1 with every other gate passed (next gate 0), but just past the line.
    let mut s = state_with_car(&cfg, v(405.0, 100.0), 0, G as u32);
    s.cars[0].next_gate = 0;
    drive(&cfg, &mut s, BACK, 40);
    assert!(s.cars[0].pos.x < 400.0, "reversed back over the line");
    assert_eq!((s.cars[0].crossings, s.cars[0].laps), (G as u32, 0));
    // Driving forwards over it now completes the lap, once.
    let mut ev = Vec::new();
    while s.cars[0].pos.x <= 405.0 {
        ev.extend(drive(&cfg, &mut s, GO, 1));
    }
    assert_eq!((s.cars[0].crossings, s.cars[0].laps), (G as u32 + 1, 1));
    assert_eq!(
        ev.iter()
            .filter(|e| matches!(e, RaceEvent::Lap { .. }))
            .count(),
        1
    );
    // Back and forth over the line again: nothing more (gate 1 is next).
    rock_over_the_line(&cfg, &mut s, 3);
    assert_eq!((s.cars[0].crossings, s.cars[0].laps), (G as u32 + 1, 1));
    // The start crossing is the same: a car that rocks over gate 0 has started once.
    let mut s = RaceState::start(&cfg, &[0]);
    drive(&cfg, &mut s, GO, 30);
    assert_eq!(s.cars[0].crossings, 1);
    rock_over_the_line(&cfg, &mut s, 3);
    assert_eq!(s.cars[0].crossings, 1);
}

/// Reverse until the car is behind gate 0, then drive until it is past it, `times` times.
fn rock_over_the_line(cfg: &RacingConfig, s: &mut RaceState, times: usize) {
    for _ in 0..times {
        while s.cars[0].pos.x >= 395.0 {
            drive(cfg, s, BACK, 1);
        }
        while s.cars[0].pos.x <= 405.0 {
            drive(cfg, s, GO, 1);
        }
    }
}

#[test]
fn gates_crossed_out_of_order_do_not_count() {
    let cfg = solo();
    // Started (next gate 1), placed just behind gate 3 (700, 350) heading up the right side.
    let mut s = state_with_car(&cfg, v(700.0, 345.0), from_degrees(90), 1);
    s.cars[0].vel = v(0.0, 200.0);
    let p0 = s.cars[0].progress;
    let ev = drive(&cfg, &mut s, GO, 10);
    assert!(s.cars[0].pos.y > 350.0);
    assert_eq!((s.cars[0].crossings, s.cars[0].next_gate), (1, 1));
    assert!(!ev.iter().any(|e| matches!(e, RaceEvent::Gate { .. })));
    // Progress is still measured towards gate 1 (clamped), so skipping earns nothing.
    assert!(s.cars[0].progress <= 1.0 && p0 <= 1.0);
    // Gate 0 again while gate 1 is next: nothing either.
    let mut s = state_with_car(&cfg, v(395.0, 100.0), 0, 1);
    drive(&cfg, &mut s, GO, 10);
    assert_eq!(s.cars[0].crossings, 1);
}

#[test]
fn a_lap_needs_every_gate_in_order() {
    let cfg = solo();
    let m = run_line(&cfg, 3);
    let c = m.state().cars[0];
    assert_eq!(c.laps, 3);
    assert_eq!(c.crossings, 3 * G as u32 + 1);
    assert_eq!(c.progress, 27.0);
    assert_eq!(m.outcome().unwrap().reason, FINISHED_REASON);
}

#[test]
fn rewards_sum_to_final_progress_over_gates_plus_the_bonus() {
    for (cfg, seeds) in [(solo(), 0..3u64), (four(), 0..6u64)] {
        for seed in seeds {
            for wild in [false, true] {
                let n = cfg.cars.len();
                let mut m = Race::new(cfg.clone(), seed);
                let mut ld = line_drivers(&cfg);
                let mut wd: Vec<Wild> = (0..n as u64).map(|i| Wild::new(seed * 8 + i)).collect();
                let mut sums = vec![0.0f64; n];
                while !m.is_over() {
                    let mut ps: Vec<&mut dyn Policy<RacingRules>> = Vec::new();
                    for (i, (l, w)) in ld.iter_mut().zip(wd.iter_mut()).enumerate() {
                        // With `wild`, cars 2 and 3 drive at random (most never finish).
                        if wild && i >= 2 {
                            ps.push(w)
                        } else {
                            ps.push(l)
                        }
                    }
                    m.step_policies(&mut ps);
                    for (a, s) in sums.iter_mut().enumerate() {
                        *s += m.reward(a) as f64;
                    }
                }
                for (a, c) in m.state().cars.iter().enumerate() {
                    let bonus = if c.finished() {
                        RacingRules::finish_bonus(c.place, n) as f64
                    } else {
                        0.0
                    };
                    let want = c.progress as f64 / G as f64 + bonus;
                    assert!(
                        (sums[a] - want).abs() < 1e-4,
                        "seed {seed} wild {wild} car {a}: {} vs {want}",
                        sums[a]
                    );
                }
            }
        }
    }
}

#[test]
fn reward_is_progress_gained_and_the_bonus_lands_once() {
    let cfg = four();
    let mut m = Race::new(cfg.clone(), 1);
    let mut ds = line_drivers(&cfg);
    let mut bonus_ticks = [0u32; 4];
    while !m.is_over() {
        let before: Vec<f32> = m.state().cars.iter().map(|c| c.progress).collect();
        let mut ps: Vec<&mut dyn Policy<RacingRules>> = ds
            .iter_mut()
            .map(|d| d as &mut dyn Policy<RacingRules>)
            .collect();
        m.step_policies(&mut ps);
        for (a, c) in m.state().cars.iter().enumerate() {
            let base = (c.progress - before[a]) / G as f32;
            let r = m.reward(a);
            if (r - base).abs() > 1e-6 {
                assert_eq!(c.finish_tick, Some(m.tick()));
                assert!((r - base - RacingRules::finish_bonus(c.place, 4)).abs() < 1e-6);
                bonus_ticks[a] += 1;
            }
            if c.finished() && c.finish_tick != Some(m.tick()) {
                assert_eq!(r, 0.0, "a finished car earns nothing more");
            }
        }
    }
    let places: Vec<u8> = m.state().cars.iter().map(|c| c.place).collect();
    for (a, &b) in bonus_ticks.iter().enumerate() {
        // Last place's bonus is 0, so it doesn't show.
        assert_eq!(b, u32::from(places[a] < 4), "car {a}");
    }
    assert_eq!(RacingRules::finish_bonus(1, 4), 1.0);
    assert_eq!(RacingRules::finish_bonus(4, 4), 0.0);
    assert_eq!(RacingRules::finish_bonus(1, 1), 1.0);
}

#[test]
fn no_car_ends_a_tick_overlapping_a_wall_or_another_car() {
    // 1,000 seeds: four cars, each seed mixing line drivers at different offsets with
    // random drivers, so there is traffic, contact and wall scraping. Split across
    // threads so the debug-build CI run stays short.
    let threads = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .min(16) as u64;
    let totals: Vec<(u64, u64)> = std::thread::scope(|sc| {
        let hs: Vec<_> = (0..threads)
            .map(|t| sc.spawn(move || overlap_sweep((0..1000u64).filter(|s| s % threads == t))))
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let (contacts, walls) = totals.iter().fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
    assert!(
        contacts > 1000 && walls > 1000,
        "the sweep exercised collisions: {contacts} {walls}"
    );
}

/// Runs the seeds and checks every tick; returns (contact events, wall events).
fn overlap_sweep(seeds: impl Iterator<Item = u64>) -> (u64, u64) {
    let cfg = four();
    let r = cfg.physics.radius;
    let geom = TrackGeom::new(&cfg.track);
    let eps = 1e-3;
    let (mut contacts, mut walls) = (0u64, 0u64);
    for seed in seeds {
        let mut m = Race::new(cfg.clone(), seed);
        let mut ld = line_drivers(&cfg);
        for (i, d) in ld.iter_mut().enumerate() {
            d.offset = ((seed * 7 + i as u64 * 13) % 41) as f32 - 20.0;
            d.corner_speed = 140.0 + ((seed + i as u64) % 5) as f32 * 15.0;
        }
        let mut wd: Vec<Wild> = (0..4u64).map(|i| Wild::new(seed * 4 + i)).collect();
        let wild = (seed % 4) as usize; // how many random drivers
        while !m.is_over() {
            let mut ps: Vec<&mut dyn Policy<RacingRules>> = Vec::new();
            for (i, (l, w)) in ld.iter_mut().zip(wd.iter_mut()).enumerate() {
                if i < wild {
                    ps.push(w)
                } else {
                    ps.push(l)
                }
            }
            m.step_policies(&mut ps);
            let cars = &m.state().cars;
            for e in m.events() {
                match e {
                    RaceEvent::Contact { .. } => contacts += 1,
                    RaceEvent::Wall { .. } => walls += 1,
                    _ => {}
                }
            }
            for (i, a) in cars.iter().enumerate() {
                assert!(
                    geom.on_track(a.pos) && geom.wall_distance(a.pos) >= r - eps,
                    "seed {seed} tick {} car {i} in a wall at {}",
                    m.tick(),
                    a.pos
                );
                for b in &cars[i + 1..] {
                    if a.finished() || b.finished() {
                        continue; // finished cars are ghosts
                    }
                    assert!(
                        (a.pos - b.pos).length() >= 2.0 * r - eps,
                        "seed {seed} tick {}: cars overlap: {:?}",
                        m.tick(),
                        cars.iter()
                            .map(|c| (c.pos, c.finished()))
                            .collect::<Vec<_>>()
                    );
                }
            }
        }
    }
    (contacts, walls)
}

#[test]
fn a_config_that_could_tunnel_is_rejected() {
    let mut cfg = solo();
    cfg.cars[0].top_speed = 12.0 * 60.0; // exactly the radius per tick
    let e = cfg.validate().unwrap_err();
    assert_eq!(e.field, "cars[].top_speed");
    cfg.cars[0].top_speed = 719.0;
    assert!(cfg.validate().is_ok(), "just under the radius per tick");
    cfg.physics.radius = 10.0;
    assert_eq!(cfg.validate().unwrap_err().field, "cars[].top_speed");
    // The fastest real setup is far inside the limit.
    let fastest = CarParams {
        top_speed: setup::TOP_SPEED[4],
        ..CarParams::default()
    };
    assert!(RacingConfig::ring(vec![fastest; 4]).validate().is_ok());
    // A replay carrying a bad config is rejected at load.
    let good = run_line(&solo(), 0).replay();
    let mut bad = good.clone();
    bad.config.cars[0].top_speed = 800.0;
    assert_eq!(
        RaceReplay::from_json(&bad.to_json()),
        Err(ReplayError::FieldNotInFormat {
            format: good.format,
            field: "cars[].top_speed"
        })
    );
}

#[test]
#[should_panic(expected = "top speed per tick")]
fn starting_a_race_with_a_tunnelling_config_panics() {
    let mut cfg = solo();
    cfg.cars[0].top_speed = 900.0;
    Race::new(cfg, 0);
}

#[test]
fn other_invalid_configs_name_their_field() {
    let base = four();
    type Edit = Box<dyn Fn(&mut RacingConfig)>;
    let cases: Vec<(&str, Edit)> = vec![
        ("cars", Box::new(|c| c.cars.clear())),
        ("cars", Box::new(|c| c.cars.push(CarParams::default()))),
        ("cars[].grip", Box::new(|c| c.cars[1].grip = 1.5)),
        ("cars[].power", Box::new(|c| c.cars[2].power = f32::NAN)),
        ("laps", Box::new(|c| c.laps = 0)),
        ("max_ticks", Box::new(|c| c.max_ticks = 0)),
        ("grid", Box::new(|c| c.grid = Some(vec![0, 1, 1, 2]))),
        ("grid", Box::new(|c| c.grid = Some(vec![0, 1, 2]))),
        ("track.width", Box::new(|c| c.track.width = 40.0)),
        (
            "track.centreline",
            Box::new(|c| c.track.centreline.truncate(2)),
        ),
        ("physics.wall_keep", Box::new(|c| c.physics.wall_keep = 2.0)),
    ];
    for (field, f) in cases {
        let mut c = base.clone();
        f(&mut c);
        assert_eq!(c.validate().map_err(|e| e.field), Err(field));
    }
}

// ---------------------------------------------------------------- race end, placings

#[test]
fn a_race_ends_when_every_car_has_finished_and_finished_cars_go_inactive() {
    let cfg = four();
    let mut m = Race::new(cfg.clone(), 2);
    let mut ds = line_drivers(&cfg);
    let mut saw_inactive = false;
    while !m.is_over() {
        let mut ps: Vec<&mut dyn Policy<RacingRules>> = ds
            .iter_mut()
            .map(|d| d as &mut dyn Policy<RacingRules>)
            .collect();
        m.step_policies(&mut ps);
        for (a, c) in m.state().cars.iter().enumerate() {
            assert_eq!(RacingRules::is_active(m.state(), a), !c.finished());
            saw_inactive |= c.finished() && !m.is_over();
        }
    }
    assert!(saw_inactive);
    let o = m.outcome().unwrap();
    let s = m.state();
    let last = s.cars.iter().filter_map(|c| c.finish_tick).max().unwrap();
    assert!(s.cars.iter().all(Car::finished));
    assert_eq!(o.ticks, last, "ends on the tick the last car finishes");
    assert!(last < s.first_finish.unwrap() + cfg.finish_window);
    assert_eq!(RaceEnd::from_reason(o.reason), Some(RaceEnd::Finished));
    assert!(!RaceEnd::Finished.is_truncated());
    let winner = o.winner.unwrap() as usize;
    assert_eq!(s.cars[winner].place, 1);
    assert_eq!(s.cars[winner].finish_tick, s.first_finish);
    let mut places: Vec<u8> = s.cars.iter().map(|c| c.place).collect();
    places.sort();
    assert_eq!(places, [1, 2, 3, 4]);
}

#[test]
fn the_finish_window_closes_ten_seconds_after_the_winner() {
    // Car 0 drives; car 1 sits on the grid and never starts.
    let cfg = RacingConfig::ring_setups(&[Setup::BALANCED; 2]).with_grid(vec![0, 1]);
    let mut m = Race::new(cfg.clone(), 0);
    let mut d = LineDriver::new(&cfg);
    let mut sit = |_: &Observation| RaceAction::default();
    let o = m.run(&mut [&mut d, &mut sit]);
    let t0 = m.state().first_finish.unwrap();
    assert_eq!(o.ticks, t0 + 600);
    assert_eq!(
        RacingRules::race_end(&cfg, m.state(), o.ticks),
        Some(RaceEnd::Finished)
    );
    assert_eq!(o.winner, Some(0));
    // (Car 0 may nudge it over the line at the start, but it never finishes.)
    assert!(!m.state().cars[1].finished());
    assert_eq!(m.state().cars[1].place, 2);
}

#[test]
fn the_tick_cap_is_tick_limit_even_inside_the_window() {
    // Nobody finishes: winner None.
    let cfg = RacingConfig::ring_setups(&[Setup::BALANCED; 2]);
    let mut m = Race::new(cfg.clone(), 0);
    let mut sit = |_: &Observation| RaceAction::default();
    let mut sit2 = |_: &Observation| RaceAction::default();
    let o = m.run(&mut [&mut sit, &mut sit2]);
    assert_eq!(
        (o.ticks, o.reason, o.winner),
        (3600, EndReason::TickLimit, None)
    );
    assert_eq!(
        m.state().cars[0].place,
        1,
        "equal progress shares first place"
    );
    assert_eq!(m.state().cars[1].place, 1);
    // A winner finishes, then the cap falls inside the 10 s window: still TickLimit,
    // winner = the first finisher.
    let probe = run_line(&solo(), 0);
    let t_win = probe.state().first_finish.unwrap();
    let mut cfg2 = RacingConfig::ring_setups(&[Setup::BALANCED; 2]).with_grid(vec![0, 1]);
    cfg2.max_ticks = t_win + 100;
    let mut m = Race::new(cfg2.clone(), 0);
    let mut d = LineDriver::new(&cfg2);
    let o = m.run(&mut [&mut d, &mut sit]);
    assert_eq!(o.reason, EndReason::TickLimit);
    assert_eq!(o.ticks, cfg2.max_ticks);
    assert_eq!(o.winner, Some(0));
    assert!(RaceEnd::TickLimit.is_truncated());
}

#[test]
fn placings_rank_finishers_by_tick_then_progress_and_share_ties() {
    let s = RaceState::start(&four(), &[0, 1, 2, 3]);
    let mut cars = s.cars.clone();
    cars[0].finish_tick = Some(1500);
    cars[1].finish_tick = Some(1500);
    cars[2].progress = 20.5;
    cars[3].progress = 21.0;
    assert_eq!(RacingRules::placings(&cars), [1, 1, 4, 3]);
    cars[2].progress = 21.0;
    assert_eq!(RacingRules::placings(&cars), [1, 1, 3, 3]);
    cars[1].finish_tick = Some(1499);
    assert_eq!(RacingRules::placings(&cars), [2, 1, 3, 3]);
}

#[test]
fn progress_is_zero_before_the_line_and_continuous_through_gates() {
    let cfg = solo();
    let mut m = Race::new(cfg.clone(), 0);
    assert_eq!(m.state().cars[0].progress, 0.0);
    let mut d = LineDriver::new(&cfg);
    let mut last = 0.0f32;
    while !m.is_over() {
        m.step_policies(&mut [&mut d]);
        let p = m.state().cars[0].progress;
        assert!(
            p >= last - 0.05 && p - last < 0.1,
            "tick {}: {last} -> {p}",
            m.tick()
        );
        last = p;
    }
    assert_eq!(last, 27.0);
}

// ---------------------------------------------------------------- flat view

#[test]
fn flat_lengths_layout_and_wrong_length_panics() {
    assert_eq!((RacingRules::OBS_LEN, RacingRules::ACTION_LEN), (43, 2));
    assert_eq!(
        (SPEED, HEADING, POSITION, RAYS, NEXT_GATE, NEXT_DIR),
        (0, 2, 4, 6, 15, 17)
    );
    assert_eq!(
        (GATE_AFTER, LAPS, PROGRESS, PLACE, OPPONENTS, TICK),
        (19, 21, 22, 23, 24, 42)
    );
    assert_eq!(OPPONENT_SLOT * racing::obs::OPPONENT_SLOTS, 18);
}

#[test]
#[should_panic(expected = "observation buffer length")]
fn encode_obs_panics_on_a_wrong_length_buffer() {
    let m = Race::new(solo(), 0);
    m.encode_obs(0, &mut [0.0; 42]);
}

#[test]
#[should_panic(expected = "action buffer length")]
fn decode_action_panics_on_a_wrong_length_input() {
    RacingRules::decode_action(&[0.0; 3]);
}

#[test]
fn decode_clamps_and_maps_nan_to_zero() {
    let a = RacingRules::decode_action(&[0.25, -0.5]);
    assert_eq!((a.throttle, a.steer), (0.25, -0.5));
    let a = RacingRules::decode_action(&[7.0, f32::NAN]);
    assert_eq!((a.throttle, a.steer), (1.0, 0.0));
    let a = RacingRules::decode_action(&[f32::NEG_INFINITY, -3.0]);
    assert_eq!((a.throttle, a.steer), (-1.0, -1.0));
    assert_eq!(
        RacingRules::sanitize(RaceAction {
            throttle: f32::NAN,
            steer: 2.0
        })
        .steer,
        1.0
    );
}

#[test]
fn flat_values_on_the_grid_match_the_table() {
    let cfg = four().with_grid(vec![0, 1, 2, 3]);
    let m = Race::new(cfg, 0);
    let mut o = [9.0f32; OBS_LEN];
    m.encode_obs(0, &mut o); // car 0 at (370, 120), heading 0
    assert_eq!(&o[SPEED..SPEED + 2], &[0.0, 0.0]);
    assert_eq!(&o[HEADING..HEADING + 2], &[1.0, 0.0]);
    assert!((o[POSITION] - (2.0 * 370.0 / 800.0 - 1.0)).abs() < 1e-6);
    assert!((o[POSITION + 1] - (2.0 * 120.0 / 600.0 - 1.0)).abs() < 1e-6);
    // Left ray (+90°): the infield edge at y 160 is 40 u away; right ray: y 40 is 80 u.
    assert!((o[RAYS + 8] - 40.0 / 300.0).abs() < 1e-5, "{}", o[RAYS + 8]);
    assert!((o[RAYS] - 80.0 / 300.0).abs() < 1e-5);
    // Next gate is gate 0 at (400, 100): 30 u ahead, 20 u to the right.
    assert!((o[NEXT_GATE] - 30.0 / 300.0).abs() < 1e-5);
    assert!((o[NEXT_GATE + 1] + 20.0 / 300.0).abs() < 1e-5);
    assert_eq!(&o[NEXT_DIR..NEXT_DIR + 2], &[1.0, 0.0]);
    assert!((o[GATE_AFTER] - 180.0 / 300.0).abs() < 1e-5);
    // Nearest opponent: car 2 at (340, 120), 30 u behind; then car 1 at (370, 80), 40 u
    // to the right; then car 3 at (340, 80). Nobody moves and places are all shared.
    let mut want = [0.0f32; TICK - LAPS];
    for (k, (f, l)) in [(-30.0, 0.0), (0.0, -40.0), (-30.0, -40.0)]
        .into_iter()
        .enumerate()
    {
        let b = OPPONENTS + k * OPPONENT_SLOT - LAPS;
        want[b] = 1.0;
        want[b + 1] = f / 300.0;
        want[b + 2] = l / 300.0;
    }
    assert_eq!(&o[LAPS..TICK], &want);
    assert_eq!(o[TICK], 0.0);
    // Racing alone: no opponents, place 0.
    let mut o = [9.0f32; OBS_LEN];
    Race::new(solo(), 0).encode_obs(0, &mut o);
    assert_eq!(&o[PLACE..TICK], &[0.0; TICK - PLACE]);
}

#[test]
fn every_flat_value_is_finite_and_in_range_over_whole_races() {
    let cfg = four();
    let mut buf = [0.0f32; OBS_LEN];
    let mut seen_place = false;
    for seed in 0..5u64 {
        let mut m = Race::new(cfg.clone(), seed);
        let mut ds = line_drivers(&cfg);
        let mut wd = Wild::new(seed);
        while !m.is_over() {
            for a in 0..4 {
                m.encode_obs(a, &mut buf);
                for (k, x) in buf.iter().enumerate() {
                    assert!(x.is_finite() && (-1.0..=1.0).contains(x), "index {k}: {x}");
                }
                seen_place |= buf[PLACE] > 0.0;
                assert!(
                    (buf[HEADING] * buf[HEADING] + buf[HEADING + 1] * buf[HEADING + 1] - 1.0).abs()
                        < 1e-5
                );
            }
            let (a, b) = ds.split_at_mut(1);
            let mut ps: Vec<&mut dyn Policy<RacingRules>> = vec![&mut wd, &mut a[0]];
            ps.extend(
                b.iter_mut()
                    .skip(1)
                    .map(|d| d as &mut dyn Policy<RacingRules>),
            );
            m.step_policies(&mut ps);
        }
        m.encode_obs(0, &mut buf);
        assert!((buf[TICK] - m.tick() as f32 / 3600.0).abs() < 1e-6);
    }
    assert!(seen_place);
}

// ---------------------------------------------------------------- determinism, replays

#[test]
fn same_seed_and_actions_give_the_same_hash_and_byte_identical_replays() {
    for seed in [0u64, 7, 42, u64::MAX] {
        let a = run_line(&four(), seed);
        let b = run_line(&four(), seed);
        assert_eq!(a.state_hash(), b.state_hash());
        assert_eq!(a.replay().to_json(), b.replay().to_json());
        // Re-simulating the recorded actions reproduces the hash and outcome.
        let r = RaceReplay::from_json(&a.replay().to_json()).unwrap();
        let m = r.verify().unwrap();
        assert_eq!(m.state_hash(), a.state_hash());
        assert_eq!(r, a.replay(), "replay JSON round-trips");
    }
}

#[test]
fn the_seed_only_shuffles_the_grid() {
    // Shuffled: different seeds give different grids (one next_u32 per car).
    let mut grids = std::collections::BTreeSet::new();
    for seed in 0..64u64 {
        let m = Race::new(four(), seed);
        let g: Vec<u8> = m.state().cars.iter().map(|c| c.slot).collect();
        let mut sorted = g.clone();
        sorted.sort();
        assert_eq!(sorted, [0, 1, 2, 3]);
        grids.insert(g);
    }
    assert!(grids.len() > 12, "{} distinct grids", grids.len());
    // Fixed grid: the seed changes nothing (same actions, same hash).
    let cfg = four().with_grid(vec![3, 2, 1, 0]);
    let a = run_line(&cfg, 1);
    let b = run_line(&cfg, 2);
    assert_eq!(a.state_hash(), b.state_hash());
    assert_eq!(a.state().cars[0].slot, 3);
}

#[test]
fn racing_replays_are_format_5_and_name_their_game_and_rules_version() {
    let m = run_line(&four(), 9);
    let json = m.replay().to_json();
    assert!(
        json.starts_with(r#"{"format":5,"game":"racing","rules_version":1,"#),
        "{}",
        &json[..60]
    );
    assert_eq!(<RacingRules as Rules>::GAME, racing::catalog::GAME);
    assert_eq!(
        <RacingRules as Rules>::RULES_VERSION,
        racing::catalog::RULES_VERSION
    );
    let r = RaceReplay::from_json(&json).unwrap();
    assert_eq!(r.to_json(), json, "round-trips byte for byte");
    assert_eq!(
        r.verify().unwrap().outcome().unwrap().reason,
        EndReason::Finished
    );
    // Another game, or other racing rules, is named rather than misparsed.
    assert_eq!(
        RaceReplay::from_json(&json.replace(r#""game":"racing""#, r#""game":"tank""#)),
        Err(ReplayError::GameMismatch {
            expected: "racing",
            got: Some("tank".into())
        })
    );
    assert_eq!(
        RaceReplay::from_json(&json.replace(r#""rules_version":1"#, r#""rules_version":2"#)),
        Err(ReplayError::RulesVersionMismatch {
            game: "racing",
            expected: 1,
            got: Some(2)
        })
    );
    assert!(matches!(
        engine::Replay::from_json(&json),
        Err(ReplayError::GameMismatch {
            expected: "tank",
            ..
        })
    ));
    // A tank format 4 file is refused by racing, and still loads and verifies as tank.
    let tank = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../engine-wasm/tests/parity/cw-seed-max.json"
    ))
    .unwrap();
    assert_eq!(
        RaceReplay::from_json(&tank),
        Err(ReplayError::GameMismatch {
            expected: "racing",
            got: None
        })
    );
    let t = engine::Replay::from_json(&tank).unwrap();
    assert_eq!(t.format, 4);
    assert_eq!(
        format!("{:016x}", t.verify().unwrap().state_hash()),
        "f1d983e88de5d020"
    );
}

#[test]
fn a_tampered_replay_fails_to_verify() {
    let m = run_line(&four(), 5);
    let mut r = m.replay();
    let t = r.actions.len() / 2;
    r.actions[t][1].steer = -r.actions[t][1].steer + 0.5;
    assert!(matches!(
        r.verify(),
        Err(ReplayError::HashMismatch { .. } | ReplayError::OutcomeMismatch { .. })
    ));
    let mut r = m.replay();
    r.config.laps = 2;
    assert!(matches!(r.verify(), Err(ReplayError::SetupMismatch { .. })));
}

#[test]
fn hash_covers_the_spec_fields() {
    let cfg = solo();
    let s0 = RaceState::start(&cfg, &[0]);
    let h = |s: &RaceState| {
        let mut h = engine::generic::StateHasher::new();
        RacingRules::hash_state(s, &mut h);
        h.finish()
    };
    let base = h(&s0);
    let edits: [fn(&mut Car); 8] = [
        |c| c.pos.x += 0.001,
        |c| c.vel.y = 1.0,
        |c| c.heading = 1,
        |c| c.crossings = 1,
        |c| c.laps = 1,
        |c| c.finish_tick = Some(9),
        |c| c.last_progress = 0.5,
        |c| c.pos.y -= 0.001,
    ];
    for (k, e) in edits.iter().enumerate() {
        let mut s = s0.clone();
        e(&mut s.cars[0]);
        assert_ne!(h(&s), base, "edit {k}");
    }
}

#[test]
fn identity_constants() {
    assert_eq!(racing::GAME, "racing");
    assert_eq!(racing::RULES_VERSION, 1);
    assert_eq!(STAT_KEYS, ["power", "top_speed", "grip"]);
    assert_eq!(Setup::all().len(), 19);
}
