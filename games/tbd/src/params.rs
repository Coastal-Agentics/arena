//! Estimated plant and controller numbers from `assets/tbd/reference/tbd_control.py`
//! and `assets/tbd/PARAMS.md`.
//!
//! Nothing here was measured on hardware. TBD is a physics sim with estimated parameters.

/// Gravity, m/s².
pub const G: f64 = 9.81;
/// All-up mass, kg.
pub const MASS: f64 = 0.970;
/// Max thrust of one duct, N.
pub const T_MAX: f64 = 5.5;
/// Yaw reaction torque per newton of thrust, N·m/N.
pub const K_YAW: f64 = 0.012;
/// Fore-aft arm of a duct centre from the body origin, m.
pub const X_ARM: f64 = 0.055;
/// Lateral arm of a duct centre from the body origin, m.
pub const Y_ARM: f64 = 0.1115;
/// Composite inertia estimate `(Ix, Iy, Iz)`, kg·m². This is the controller's
/// model, not the inertia written into `tbd.xml`.
pub const INERTIA: [f64; 3] = [0.0046, 0.0040, 0.0075];
/// Attitude loop natural frequency, rad/s.
pub const ATT_WN: f64 = 14.0;
/// Attitude loop damping ratio.
pub const ATT_ZETA: f64 = 0.9;
/// Yaw proportional gain (on the wrapped yaw error, inside the inertia scale).
pub const YAW_KP: f64 = 16.0;
/// Yaw rate gain.
pub const YAW_KD: f64 = 8.0;
/// Horizontal position gain, 1/s².
pub const POS_KP: f64 = 5.0;
/// Horizontal velocity gain, 1/s.
pub const POS_KD: f64 = 4.0;
/// Altitude proportional gain.
pub const ALT_KP: f64 = 8.0;
/// Altitude rate gain.
pub const ALT_KD: f64 = 5.0;
/// Altitude integral gain.
pub const ALT_KI: f64 = 2.0;
/// Altitude integrator clamp, m·s.
pub const ALT_I_LIMIT: f64 = 2.0;
/// Desired tilt cap, rad (about 23°).
pub const MAX_TILT: f64 = 0.40;
/// Horizontal acceleration cap, m/s².
pub const MAX_ACC: f64 = 4.0;
/// Hover efficiency, gram-force per watt.
pub const G_PER_W: f64 = 2.3;
/// Avionics draw, W.
pub const AVIONICS_W: f64 = 6.0;
/// Pack energy, Wh (6S 850 mAh).
pub const PACK_WH: f64 = 18.9;
/// Fraction of the pack counted as usable.
pub const USABLE: f64 = 0.80;
/// Draw of one wheel motor while driving, W. The battery adds four of these
/// when the driving flag is on.
pub const WHEEL_W: f64 = 1.0;
/// Model timestep in `tbd.xml`, seconds. One [`crate::TbdGame::step`] is one tick.
pub const DT_MODEL: f64 = 0.002;
/// Wheel actuator limit, rad/s (`ctrlrange` on the drive actuators).
pub const WHEEL_RAD_S: f64 = 26.0;
/// Fold stowed angle, rad. The literal `1.5708` from `tbd.xml` (keyframe and
/// the fold actuator's upper limit). It is not `π/2`.
#[allow(clippy::approx_constant)]
pub const FOLD_STOW: f64 = 1.5708;
/// Fold deployed angle, rad.
pub const FOLD_DEPLOY: f64 = 0.0;
/// Air density used for the estimated wind force, kg/m³. Matches `tbd.xml`.
pub const AIR_DENSITY: f64 = 1.225;
/// Estimated drag area (Cd · A) for the wind disturbance only, m². Not measured.
pub const WIND_DRAG_AREA: f64 = 0.08;
/// Cells in series. Tattu 6S.
pub const N_CELLS: u32 = 6;
/// Open-circuit volts per cell at full charge.
pub const V_CELL_FULL: f64 = 4.2;
/// Open-circuit volts per cell at empty. Landing-voltage end of the sag curve.
pub const V_CELL_EMPTY: f64 = 3.5;
/// Voltage the 5.5 N placeholder is referenced to, volts. 6 × 4.2.
pub const V_REF: f64 = N_CELLS as f64 * V_CELL_FULL;
/// Per-cell resistance, ohms. 7.5 mΩ. ESTIMATE, unsourced. Plausible band 5–10 mΩ.
pub const R_CELL_OHM: f64 = 0.0075;
/// Prop disk radius, m. Gemfan D76 is 76 mm across.
pub const DUCT_RADIUS: f64 = 0.038;
/// Duct plane height in the body frame, m. Fold-hinge height.
pub const DUCT_Z: f64 = 0.037;
/// Rangefinder site height in the body frame, m (`tbd.xml` site `down`).
pub const RANGE_SITE_Z: f64 = 0.014;
/// Fold command rate, deg/s at the arm. About 90° in 0.19 s.
pub const FOLD_RATE_DEG_S: f64 = 465.0;
/// [`FOLD_RATE_DEG_S`] in rad/s.
pub const FOLD_RATE_RAD_S: f64 = FOLD_RATE_DEG_S * std::f64::consts::PI / 180.0;
/// Motor thrust lag in `tbd.xml`, seconds. Kept at 40 ms.
pub const MOTOR_TAU_S: f64 = 0.040;

/// Yaw-reaction sign per duct, FL FR RL RR. Matches `tbd.xml` actuator gear.
pub const SPIN: [f64; 4] = [1.0, -1.0, -1.0, 1.0];
/// Sign of the fore-aft arm, FL FR RL RR.
pub const X_SIGN: [f64; 4] = [1.0, 1.0, -1.0, -1.0];
/// Sign of the lateral arm, FL FR RL RR.
pub const Y_SIGN: [f64; 4] = [1.0, -1.0, 1.0, -1.0];

/// Duct order shared by the mixer, the observation notes and `tbd.xml`.
pub const DUCTS: [&str; 4] = ["FL", "FR", "RL", "RR"];
