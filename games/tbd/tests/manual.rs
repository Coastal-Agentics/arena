//! Manual sticks, the landing abort, and the held stow sentinel.

use tbd::{Config, MissionId, TbdGame, OBS_LEN, OBS_LEN_V1, PILOT_FOLD, PILOT_LAND, PILOT_TAKEOFF};

fn cfg() -> Config {
    Config {
        dt: 0.1,
        mission: MissionId::Manual,
        wind: false,
        max_time_s: 30.0,
        voltage_sag: false,
        ram_drag: false,
        ground_effect: false,
        fold_rate_limit: false,
        ..Config::default()
    }
}

fn base() -> [f64; OBS_LEN] {
    let mut o = [0.0; OBS_LEN];
    o[6] = 1.0;
    o[11] = tbd::params::FOLD_STOW;
    o
}

fn enter_flight(g: &mut TbdGame) -> [f64; OBS_LEN] {
    let mut obs = base();
    obs[18] = PILOT_TAKEOFF;
    g.step(&obs).unwrap();
    obs[18] = 0.0;
    obs[11] = 0.03;
    obs[14] = 0.6;
    for _ in 0..8 {
        g.step(&obs).unwrap();
        if g.state().mode == "flight" {
            return obs;
        }
    }
    panic!("never flew, mode {}", g.state().mode);
}

#[test]
fn twenty_long_observation_still_steps() {
    let mut g = TbdGame::new(1, cfg()).unwrap();
    let mut short = [0.0; OBS_LEN_V1];
    short[6] = 1.0;
    short[11] = tbd::params::FOLD_STOW;
    g.step(&short).unwrap();
    assert_eq!(g.state().mode, "ground");
}

#[test]
fn climb_and_strafe_change_the_flight_command() {
    let mut hold = TbdGame::new(1, cfg()).unwrap();
    let mut climb = TbdGame::new(1, cfg()).unwrap();
    let mut strafe = TbdGame::new(1, cfg()).unwrap();
    let obs = enter_flight(&mut hold);
    let mut up = enter_flight(&mut climb);
    let mut side = enter_flight(&mut strafe);
    up[21] = 1.0;
    side[20] = 1.0;
    let a = hold.step(&obs).unwrap();
    let b = climb.step(&up).unwrap();
    let c = strafe.step(&side).unwrap();
    assert_ne!(a[0..4], b[0..4], "climb rate should change thrust");
    assert_ne!(a[0..4], c[0..4], "strafe should change thrust");
}

#[test]
fn airborne_takeoff_stick_returns_to_flight() {
    let mut g = TbdGame::new(1, cfg()).unwrap();
    let mut obs = enter_flight(&mut g);
    obs[10] = -1.0;
    obs[14] = 1.0;
    obs[18] = PILOT_LAND;
    g.step(&obs).unwrap();
    assert_eq!(g.state().mode, "landing");
    obs[18] = PILOT_TAKEOFF;
    g.step(&obs).unwrap();
    assert_eq!(g.state().mode, "flight");
}

#[test]
fn held_stow_stick_folds_on_touchdown() {
    let mut g = TbdGame::new(1, cfg()).unwrap();
    let mut obs = enter_flight(&mut g);
    obs[10] = -1.0;
    obs[14] = 1.0;
    obs[18] = PILOT_LAND;
    obs[19] = PILOT_FOLD;
    g.step(&obs).unwrap();
    assert_eq!(g.state().mode, "landing");
    obs[14] = 0.01;
    obs[18] = 0.0;
    g.step(&obs).unwrap();
    assert_eq!(
        g.state().mode,
        "stowing",
        "a held stow stick folds on the touchdown tick"
    );
}

#[test]
fn climb_entry_does_not_thrust_on_the_transition_tick() {
    let mut g = TbdGame::new(
        1,
        Config {
            dt: 0.1,
            mission: MissionId::StairHop,
            wind: false,
            max_time_s: 30.0,
            voltage_sag: false,
            ram_drag: false,
            ground_effect: false,
            fold_rate_limit: false,
            ..Config::default()
        },
    )
    .unwrap();
    let stow = tbd::params::FOLD_STOW;
    let mut obs = [0.0; OBS_LEN];
    obs[6] = 1.0;
    obs[11] = stow;
    obs[12] = 0.5;
    g.step(&obs).unwrap();
    for _ in 0..20 {
        g.step(&obs).unwrap();
        if g.state().phase == "deploy" {
            break;
        }
    }
    assert_eq!(g.state().phase, "deploy");
    obs[11] = 0.03;
    obs[14] = 0.2;
    for _ in 0..20 {
        let ctrl = g.step(&obs).unwrap();
        if g.state().phase == "climb" {
            assert_eq!(ctrl[0], 0.0, "transition tick must not thrust");
            assert_eq!(g.state().mode, "flight");
            let next = g.step(&obs).unwrap();
            assert!(next[0] > 0.0, "the following tick thrusts");
            return;
        }
    }
    panic!("never climbed, phase {}", g.state().phase);
}
