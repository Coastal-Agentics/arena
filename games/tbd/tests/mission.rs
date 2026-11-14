//! Mode machine, stair-hop score and seeded wind. No MuJoCo: the observation
//! is scripted so each phase's exit condition is met on purpose.

use tbd::{Config, MissionId, TbdGame, OBS_LEN, PILOT_FOLD, PILOT_LAND, PILOT_TAKEOFF, STAIR_TOP};

fn plant(pos: [f64; 3], vel: [f64; 3], fold: f64) -> [f64; OBS_LEN] {
    let mut o = [0.0; OBS_LEN];
    o[6] = 1.0;
    o[11] = fold;
    o[12] = pos[0];
    o[13] = pos[1];
    o[14] = pos[2];
    o[15] = vel[0];
    o[16] = vel[1];
    o[17] = vel[2];
    o
}

fn game(mission: MissionId, wind: bool) -> TbdGame {
    TbdGame::new(
        7,
        Config {
            dt: 0.1,
            mission,
            wind,
            max_time_s: 30.0,
            // The script tests the mode machine. The physics switches have their own tests.
            voltage_sag: false,
            ram_drag: false,
            ground_effect: false,
            fold_rate_limit: false,
            ..Config::default()
        },
    )
    .unwrap()
}

fn until(g: &mut TbdGame, phase: &str, obs: [f64; OBS_LEN]) {
    for _ in 0..80 {
        if g.state().phase == phase {
            return;
        }
        g.step(&obs).unwrap();
    }
    panic!("never reached {phase}, stuck in {}", g.state().phase);
}

#[test]
fn stair_hop_walks_the_script_and_scores() {
    let mut g = game(MissionId::StairHop, false);
    let stow = tbd::params::FOLD_STOW;
    let c = g.step(&plant([0.0, 0.0, 0.0], [0.0; 3], stow)).unwrap();
    assert_eq!(g.state().phase, "drive");
    assert_eq!(g.state().mode, "ground");
    assert_eq!(c[4], 24.0);
    assert_eq!(c[5], 24.0);
    assert!((c[8] - stow).abs() < 1e-9);

    // Pilot channels do not steer the script.
    let mut bumped = plant([0.1, 0.0, 0.0], [0.0; 3], stow);
    bumped[18] = 50.0;
    bumped[19] = -50.0;
    let c = g.step(&bumped).unwrap();
    assert_eq!(c[4], 24.0);

    g.step(&plant([0.42, 0.0, 0.0], [0.0; 3], stow)).unwrap();
    assert_eq!(g.state().phase, "brake");
    assert_eq!(g.state().ctrl[4], 0.0);
    let early = g.state().score;
    assert!(early > 0.0 && early < 100.0, "partial score {early}");

    until(&mut g, "deploy", plant([0.42, 0.0, 0.0], [0.0; 3], stow));
    assert_eq!(g.state().mode, "unfolding");
    assert!(g.state().ctrl[8].abs() < 1e-9);

    until(&mut g, "climb", plant([0.42, 0.0, 0.2], [0.0; 3], 0.03));
    assert_eq!(g.state().mode, "flight");
    assert!(g.state().ctrl[0] > 0.0, "collective is on");

    // Over the stairs, so clearance is measured, then up to the ceiling.
    g.step(&plant([1.0, 0.0, 1.0], [0.0; 3], 0.0)).unwrap();
    assert!(g.state().min_clearance_m.is_some());
    until(&mut g, "transit", plant([0.5, 0.0, 1.0], [0.0; 3], 0.0));
    until(&mut g, "descend", plant([2.30, 0.0, 1.0], [0.0; 3], 0.0));
    until(
        &mut g,
        "touchdown",
        plant([2.30, 0.0, STAIR_TOP], [0.0; 3], 0.0),
    );
    assert_eq!(g.state().mode, "landing");
    until(&mut g, "stow", plant([2.30, 0.0, STAIR_TOP], [0.0; 3], 0.0));
    assert!(g.state().landed_on_landing);
    assert_eq!(g.state().mode, "folding");
    until(
        &mut g,
        "roll",
        plant([2.30, 0.0, STAIR_TOP], [0.0; 3], stow),
    );
    assert_eq!(g.state().mode, "ground");
    // The stow → roll tick still commands zero drive, matching validate.py's
    // if/elif. The next tick is the roll.
    g.step(&plant([2.30, 0.0, STAIR_TOP], [0.0; 3], stow))
        .unwrap();
    assert_eq!(g.state().phase, "roll");
    assert_eq!(g.state().ctrl[4], 24.0);
    until(
        &mut g,
        "done",
        plant([2.40, 0.0, STAIR_TOP], [0.0; 3], stow),
    );
    let end = g.state();
    assert!(end.completed && end.over);
    assert!(end.score > 500.0, "score {}", end.score);
    let tick = end.tick;
    let ctrl = end.ctrl;
    g.step(&plant([0.0; 3], [0.0; 3], stow)).unwrap();
    assert_eq!(g.state().tick, tick);
    assert_eq!(g.state().ctrl, ctrl);
}

#[test]
fn same_seed_and_obs_log_reproduces_wind_and_commands() {
    let cfg = Config {
        dt: 0.002,
        mission: MissionId::StairHop,
        wind: true,
        max_time_s: 16.0,
        ..Config::default()
    };
    let mut a = TbdGame::new(42, cfg.clone()).unwrap();
    let mut b = TbdGame::new(42, cfg.clone()).unwrap();
    let mut other = TbdGame::new(43, cfg).unwrap();
    let obs = plant([0.0, 0.0, 0.2], [0.1, 0.0, 0.0], tbd::params::FOLD_STOW);
    let mut log = Vec::new();
    for _ in 0..40 {
        let ca = a.step(&obs).unwrap();
        let cb = b.step(&obs).unwrap();
        assert_eq!(ca, cb);
        assert_eq!(a.state_json(), b.state_json());
        log.push(ca);
    }
    assert_ne!(
        a.state().wind_force_n,
        [0.0; 3],
        "wind should have produced a force"
    );
    let mut diverged = false;
    for _ in 0..40 {
        other.step(&obs).unwrap();
        if other.state().wind_world_mps != a.state().wind_world_mps {
            diverged = true;
            break;
        }
    }
    assert!(diverged, "a different seed should not copy the wind");

    a.reset();
    assert_eq!(a.state().tick, 0);
    assert!(!a.is_over());
    for (i, cmd) in log.iter().enumerate() {
        assert_eq!(&a.step(&obs).unwrap(), cmd, "replay step {i}");
    }
}

#[test]
fn wind_off_stays_zero_and_matches_commands() {
    let mut on = TbdGame::new(
        9,
        Config {
            wind: true,
            ..Config::default()
        },
    )
    .unwrap();
    let mut off = TbdGame::new(
        9,
        Config {
            wind: false,
            ..Config::default()
        },
    )
    .unwrap();
    let obs = plant([0.2, 0.0, 0.3], [0.0; 3], 1.57);
    for _ in 0..15 {
        assert_eq!(on.step(&obs).unwrap(), off.step(&obs).unwrap());
        assert_eq!(off.state().wind_force_n, [0.0; 3]);
    }
    assert_ne!(on.state().wind_force_n, [0.0; 3]);
}

#[test]
fn manual_takeoff_land_and_fold() {
    let mut g = game(MissionId::Manual, false);
    assert_eq!(g.state().mode, "ground");
    let mut stick = plant([0.0, 0.0, 0.0], [0.0; 3], tbd::params::FOLD_STOW);
    stick[18] = 1.0;
    let c = g.step(&stick).unwrap();
    assert!(c[4] > 0.0 && c[5] > 0.0);

    stick[18] = PILOT_TAKEOFF;
    g.step(&stick).unwrap();
    assert_eq!(g.state().mode, "unfolding");
    assert!(g.state().ctrl[8].abs() < 1e-12);

    let mut open = plant([0.0, 0.0, 0.05], [0.0; 3], 0.03);
    open[18] = 0.0;
    until(&mut g, "flight", open);
    assert!(g.state().ctrl[0] > 0.0);

    open[14] = 0.6;
    open[18] = PILOT_LAND;
    g.step(&open).unwrap();
    assert_eq!(g.state().mode, "landing");

    let mut down = plant([0.0, 0.0, 0.01], [0.0; 3], 0.0);
    down[19] = PILOT_FOLD;
    until(&mut g, "folding", down);
    assert!((g.state().ctrl[8] - tbd::params::FOLD_STOW).abs() < 1e-9);
    let mut stowed = down;
    stowed[11] = tbd::params::FOLD_STOW;
    stowed[19] = 0.0;
    until(&mut g, "ground", stowed);
}

#[test]
fn rejects_bad_observation_and_config() {
    let mut g = TbdGame::from_json(1, "").unwrap();
    assert_eq!(g.config().mission, MissionId::StairHop);
    assert!(g.config().wind);
    assert!((g.config().dt - 0.002).abs() < 1e-15);
    let err = g.step(&[0.0; 3]).unwrap_err();
    assert!(err.to_string().contains("length"));
    let mut obs = [0.0; OBS_LEN];
    obs[6] = f64::NAN;
    assert!(g.step(&obs).unwrap_err().to_string().contains("not finite"));
    assert!(TbdGame::from_json(1, "{\"dt\":0}").is_err());
    assert!(TbdGame::from_json(1, "{\"nope\":1}").is_err());
    let err = TbdGame::from_json(1, "[]").unwrap_err();
    assert!(err.to_string().contains("JSON object"), "{err}");
    let packed = tbd::pack_observation(&[0.0; 12], [1.0, 2.0, 3.0], [0.0; 3], 0.0, 0.0).unwrap();
    assert_eq!(&packed[12..15], &[1.0, 2.0, 3.0]);
    assert!(tbd::pack_observation(&[0.0; 4], [0.0; 3], [0.0; 3], 0.0, 0.0).is_err());
}
