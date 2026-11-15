//! Voltage sag, ram drag, ground effect and the fold rate. Estimated parameters.

use tbd::params::{FOLD_STOW, R_CELL_OHM, T_MAX, V_REF};
use tbd::physics::{
    duct_height_from_range, ground_effect_ratio, mass_flow, open_circuit_voltage, ram_drag,
    soc_from_wh, terminal_voltage, thrust_voltage_scale, FoldSlew,
};
use tbd::{Battery, Config, MissionId, TbdGame, OBS_LEN};

fn near(a: f64, b: f64, tol: f64, what: &str) {
    let d = (a - b).abs();
    assert!(d <= tol, "{what}: {a} vs {b} (delta {d})");
}

#[test]
fn sag_lowers_thrust_as_the_pack_drains_and_hover_is_about_19_a() {
    let bat = Battery::new();
    let hover = tbd::physics::hover_thrust_per_duct();
    let thrusts = [hover; 4];
    let power = bat.power(&thrusts, 0.0);
    // Mid pack: open circuit halfway from 4.2 V to 3.5 V per cell.
    let soc = 0.5;
    let v_oc = open_circuit_voltage(soc);
    let (v, i) = terminal_voltage(v_oc, power, 6.0 * R_CELL_OHM);
    assert!(
        (18.0..20.5).contains(&i),
        "hover current {i} A at {v} V, power {power} W"
    );
    let fresh = thrust_voltage_scale(open_circuit_voltage(1.0), V_REF);
    let empty_v = terminal_voltage(open_circuit_voltage(0.0), power, 6.0 * R_CELL_OHM).0;
    let drained = thrust_voltage_scale(empty_v, V_REF);
    assert!(
        fresh > drained,
        "fresh {fresh} should out-thrust empty {drained}"
    );
    assert!(drained < 0.85, "empty scale {drained}");
    // A lower state of charge is a lower thrust scale at the same power.
    let high = {
        let (v, _) = terminal_voltage(open_circuit_voltage(0.8), power, 6.0 * R_CELL_OHM);
        thrust_voltage_scale(v, V_REF)
    };
    let low = {
        let (v, _) = terminal_voltage(open_circuit_voltage(0.2), power, 6.0 * R_CELL_OHM);
        thrust_voltage_scale(v, V_REF)
    };
    assert!(high > low);
    let _ = soc_from_wh(bat.wh_used());
}

#[test]
fn game_step_thrust_drops_when_sag_is_on() {
    let fly = |sag: bool| {
        let mut g = TbdGame::new(
            1,
            Config {
                dt: 0.1,
                mission: MissionId::Manual,
                wind: false,
                max_time_s: 30.0,
                voltage_sag: sag,
                ram_drag: false,
                ground_effect: false,
                fold_rate_limit: false,
                ..Config::default()
            },
        )
        .unwrap();
        let mut obs = [0.0; OBS_LEN];
        obs[6] = 1.0;
        obs[11] = FOLD_STOW;
        obs[18] = tbd::PILOT_TAKEOFF;
        g.step(&obs).unwrap();
        obs[18] = 0.0;
        obs[11] = 0.03;
        for _ in 0..8 {
            g.step(&obs).unwrap();
            if g.state().mode == "flight" {
                return g.state().ctrl[0];
            }
        }
        panic!("never flew");
    };
    let with = fly(true);
    let without = fly(false);
    assert!(
        with < without,
        "sag thrust {with} should be below unsagged {without}"
    );
    assert!(with > 0.0 && without <= 1.0);
}

#[test]
fn ground_effect_is_about_7_percent_at_liftoff_and_gone_at_1_m() {
    let liftoff = ground_effect_ratio(0.037);
    near(liftoff, 1.07, 0.015, "liftoff ratio");
    let high = ground_effect_ratio(1.0);
    near(high, 1.0, 0.002, "1 m ratio");
    // Rangefinder on the ground sits ~14 mm up; the duct plane is 37 mm.
    let h = duct_height_from_range(0.014).unwrap();
    near(h, 0.037, 1e-9, "duct height");
    near(
        ground_effect_ratio(h),
        liftoff,
        1e-12,
        "range maps to liftoff",
    );
    assert!(duct_height_from_range(-1.0).is_none());
}

#[test]
fn ram_drag_is_2_to_3_n_at_4_mps() {
    let t = tbd::physics::hover_thrust_per_duct();
    let (force, torque) = ram_drag(
        [t; 4],
        [1.0, 0.0, 0.0, 0.0],
        [4.0, 0.0, 0.0],
        [0.0; 3],
        [0.0; 3],
    );
    let mag = (force[0] * force[0] + force[1] * force[1] + force[2] * force[2]).sqrt();
    assert!(
        (2.0..3.0).contains(&mag),
        "ram drag {mag} N, force {force:?}"
    );
    assert!(force[0] < 0.0, "drag opposes +X airspeed");
    // Ducts sit below the body COM, so a rearward force makes a pitch moment.
    assert!(torque[1].abs() > 1e-4, "pitch moment {torque:?}");
    assert!(mass_flow(0.0) == 0.0);
    // T_MAX is the unsagged duct limit the scale multiplies toward.
    assert!(t < T_MAX);
}

#[test]
fn fold_command_takes_about_0_19_s() {
    let mut slew = FoldSlew::new(true);
    let dt = 0.001;
    let mut t = 0.0;
    while slew.command() > 1e-3 {
        slew.step(0.0, dt);
        t += dt;
        assert!(t < 0.5, "fold did not finish");
    }
    assert!(
        (0.17..0.21).contains(&t),
        "fold command time {t} s, ended at {}",
        slew.command()
    );
    let mut jump = FoldSlew::new(false);
    assert!(jump.step(0.0, dt).abs() < 1e-15);
}
