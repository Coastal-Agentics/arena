//! Estimated propulsion effects from the physics review.
//!
//! Each one is a switch on [`crate::Config`]. Defaults are on. Nothing here was
//! measured on this vehicle. Sources and the ESTIMATE marks are in
//! `assets/tbd/PARAMS.md`.
//!
//! * Voltage sag: `V = V_oc(soc) − I R`, thrust scales with `(V / V_ref)²`.
//! * Ram drag: `F ≈ ṁ V_air` at each duct, opposite the in-plane airspeed.
//! * Ground effect: Cheeseman and Bennett, `1 / (1 − (R / 4h)²)`.
//! * Fold rate: the command slews at the ES08MD II no-load arm rate.

use crate::math::quat_to_rot;
use crate::params::{
    AIR_DENSITY, DUCT_RADIUS, DUCT_Z, FOLD_RATE_RAD_S, FOLD_STOW, N_CELLS, PACK_WH, RANGE_SITE_Z,
    R_CELL_OHM, T_MAX, V_CELL_EMPTY, V_CELL_FULL, V_REF, X_ARM, X_SIGN, Y_ARM, Y_SIGN,
};

/// Body-shell centre of mass in the body frame, m. `tbd.xml` inertial on body `tbd`.
/// MuJoCo applies `xfrc_applied` at that point, so ram-drag torque is about it.
pub const BODY_COM_Z: f64 = 0.055;

/// Lowest duct height used in the ground-effect formula, m. `R/2`.
pub fn min_ground_effect_height() -> f64 {
    DUCT_RADIUS / 2.0
}

/// Open-circuit pack voltage, volts. `soc` is 1 full and 0 empty, across the
/// whole pack (not the 80% usable window). Linear from 4.2 V to 3.5 V per cell.
pub fn open_circuit_voltage(soc: f64) -> f64 {
    let soc = soc.clamp(0.0, 1.0);
    let cell = V_CELL_EMPTY + soc * (V_CELL_FULL - V_CELL_EMPTY);
    N_CELLS as f64 * cell
}

/// Full-pack state of charge from energy already taken out of the pack.
pub fn soc_from_wh(wh_used: f64) -> f64 {
    (1.0 - wh_used / PACK_WH).clamp(0.0, 1.0)
}

/// Terminal voltage and current for a power draw.
///
/// Solves `V = V_oc − (P / V) R`. The root kept is the one nearer `V_oc`.
/// Returns `(volts, amps)`.
pub fn terminal_voltage(v_oc: f64, power_w: f64, r_pack_ohm: f64) -> (f64, f64) {
    let power = power_w.max(0.0);
    if r_pack_ohm <= 0.0 {
        let v = v_oc.max(0.0);
        let i = if v > 1e-9 { power / v } else { 0.0 };
        return (v, i);
    }
    let disc = (v_oc * v_oc - 4.0 * power * r_pack_ohm).max(0.0);
    let v = (0.5 * (v_oc + disc.sqrt())).max(0.0);
    let i = if v > 1e-9 { power / v } else { 0.0 };
    (v, i)
}

/// `(V / V_ref)²`. Thrust at a given command scales by this.
pub fn thrust_voltage_scale(volts: f64, v_ref: f64) -> f64 {
    if v_ref <= 0.0 {
        return 1.0;
    }
    let r = volts.max(0.0) / v_ref;
    r * r
}

/// Pack resistance, ohms, from the per-cell value.
pub fn pack_resistance_ohm(r_cell_ohm: f64) -> f64 {
    N_CELLS as f64 * r_cell_ohm.max(0.0)
}

/// Default per-cell resistance, ohms (7.5 mΩ).
pub fn default_r_cell_ohm() -> f64 {
    R_CELL_OHM
}

/// `T_IGE / T_OGE` for a rotor of radius [`DUCT_RADIUS`] at height `h` metres.
///
/// Cheeseman and Bennett. `h` is clamped to at least `R/2` so the denominator
/// stays at least 0.75. An open-rotor formula applied to a duct: an estimate.
pub fn ground_effect_ratio(h_m: f64) -> f64 {
    let h = h_m.max(min_ground_effect_height());
    let x = DUCT_RADIUS / (4.0 * h);
    1.0 / (1.0 - x * x)
}

/// Duct-disk height above the ground from `range_down`, m.
///
/// The rangefinder site sits at z = 14 mm in the body; the duct plane is at
/// 37 mm. A negative range (MuJoCo miss) returns `None` and the caller uses 1.
pub fn duct_height_from_range(range_down_m: f64) -> Option<f64> {
    if range_down_m < 0.0 {
        None
    } else {
        Some(range_down_m + (DUCT_Z - RANGE_SITE_Z))
    }
}

/// Disk area of one duct, m². `π R²` with `R = 0.038` (Gemfan D76).
pub fn disk_area() -> f64 {
    std::f64::consts::PI * DUCT_RADIUS * DUCT_RADIUS
}

/// Induced velocity at thrust `t_n`, m/s. Zero at zero thrust.
pub fn induced_velocity(t_n: f64) -> f64 {
    if t_n <= 1e-8 {
        0.0
    } else {
        (t_n / (2.0 * AIR_DENSITY * disk_area())).sqrt()
    }
}

/// Mass flow through one duct, kg/s. `ṁ = T / v_i`.
pub fn mass_flow(t_n: f64) -> f64 {
    let vi = induced_velocity(t_n);
    if vi <= 1e-9 {
        0.0
    } else {
        t_n / vi
    }
}

/// Net ram drag on the four ducts.
///
/// `thrust_n` is FL FR RL RR. `vel_world` is the body-origin velocity.
/// `omega_body` is the gyro. `wind_world` is the air velocity.
/// Force and torque are world frame, torque about the body COM, so they can
/// be added to `xfrc_applied` on body `tbd`.
pub fn ram_drag(
    thrust_n: [f64; 4],
    quat_wxyz: [f64; 4],
    vel_world: [f64; 3],
    omega_body: [f64; 3],
    wind_world: [f64; 3],
) -> ([f64; 3], [f64; 3]) {
    let r = quat_to_rot(quat_wxyz);
    let n = [r[2], r[5], r[8]];
    let mut force = [0.0; 3];
    let mut torque = [0.0; 3];
    for i in 0..4 {
        let r_body = [X_SIGN[i] * X_ARM, Y_SIGN[i] * Y_ARM, DUCT_Z];
        let omega_cross_r = cross(omega_body, r_body);
        let v_rot = mat_vec(&r, omega_cross_r);
        let v_point = [
            vel_world[0] + v_rot[0],
            vel_world[1] + v_rot[1],
            vel_world[2] + v_rot[2],
        ];
        let air = [
            v_point[0] - wind_world[0],
            v_point[1] - wind_world[1],
            v_point[2] - wind_world[2],
        ];
        let vn = air[0] * n[0] + air[1] * n[1] + air[2] * n[2];
        let in_plane = [air[0] - vn * n[0], air[1] - vn * n[1], air[2] - vn * n[2]];
        let mdot = mass_flow(thrust_n[i].max(0.0));
        let f = [
            -mdot * in_plane[0],
            -mdot * in_plane[1],
            -mdot * in_plane[2],
        ];
        let r_com = [r_body[0], r_body[1], r_body[2] - BODY_COM_Z];
        let r_world = mat_vec(&r, r_com);
        let tau = cross(r_world, f);
        for k in 0..3 {
            force[k] += f[k];
            torque[k] += tau[k];
        }
    }
    (force, torque)
}

/// Hover thrust of one duct at the estimated mass, N. Used by the sag check.
pub fn hover_thrust_per_duct() -> f64 {
    crate::params::MASS * crate::params::G / 4.0
}

/// Command slew for the fold actuator. MuJoCo's position actuator has no rate
/// limit; the XML `kp` is the hold. This limits the command instead.
#[derive(Clone, Debug)]
pub struct FoldSlew {
    cmd: f64,
    enabled: bool,
}

impl FoldSlew {
    /// Start stowed.
    pub fn new(enabled: bool) -> Self {
        Self {
            cmd: FOLD_STOW,
            enabled,
        }
    }

    /// Move `cmd` toward `target` by at most `rate * dt`. Returns the command.
    pub fn step(&mut self, target: f64, dt: f64) -> f64 {
        if !self.enabled {
            self.cmd = target;
            return self.cmd;
        }
        let max_step = FOLD_RATE_RAD_S * dt.max(0.0);
        let err = target - self.cmd;
        let step = err.clamp(-max_step, max_step);
        self.cmd += step;
        self.cmd
    }

    /// Current command, rad.
    pub fn command(&self) -> f64 {
        self.cmd
    }
}

/// Scale four normalized thrust commands by voltage sag and ground effect.
///
/// `u` is the mixer output, 0..=1 of [`T_MAX`] at [`V_REF`]. Returns the
/// commands to send (still 0..=1), the newtons MuJoCo will actually produce,
/// and `(pack volts, pack amps, ground-effect ratio)`.
pub fn scale_thrust(
    u: [f64; 4],
    wh_used: f64,
    power_of: impl Fn([f64; 4]) -> f64,
    r_cell_ohm: f64,
    voltage_sag: bool,
    ground_effect: f64,
) -> ([f64; 4], [f64; 4], f64, f64, f64) {
    let ge = if ground_effect.is_finite() {
        ground_effect.max(0.0)
    } else {
        1.0
    };
    let v_oc = open_circuit_voltage(soc_from_wh(wh_used));
    let r_pack = pack_resistance_ohm(r_cell_ohm);
    let mut volts = v_oc;
    let mut amps = 0.0;
    let mut out_u = u;
    let mut out_f = [0.0; 4];
    // Two passes: thrust sets power, power sets voltage, voltage sets thrust.
    for _ in 0..2 {
        let sag = if voltage_sag {
            thrust_voltage_scale(volts, V_REF)
        } else {
            1.0
        };
        let mult = sag * ge;
        for i in 0..4 {
            out_u[i] = (u[i] * mult).clamp(0.0, 1.0);
            out_f[i] = out_u[i] * T_MAX;
        }
        let power = power_of(out_f);
        let (v, i_amp) = terminal_voltage(v_oc, power, if voltage_sag { r_pack } else { 0.0 });
        volts = v;
        amps = i_amp;
    }
    (out_u, out_f, volts, amps, ge)
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn mat_vec(m: &[f64; 9], v: [f64; 3]) -> [f64; 3] {
    [
        m[0] * v[0] + m[1] * v[1] + m[2] * v[2],
        m[3] * v[0] + m[4] * v[1] + m[5] * v[2],
        m[6] * v[0] + m[7] * v[1] + m[8] * v[2],
    ]
}
