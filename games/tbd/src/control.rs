//! Cascade flight controller.
//!
//! Position and velocity PD produce a desired tilt and a collective thrust.
//! Attitude PD on body rates produces torques. An analytic X mixer turns those
//! into four duct thrusts. This is a direct port of `FlightController` in
//! `assets/tbd/reference/tbd_control.py` (and `tbd_control.js`, which is the
//! same math with zero acceleration feedforward).

use crate::math::{quat_to_rpy, wrap};
use crate::params::{
    ALT_I_LIMIT, ALT_KD, ALT_KI, ALT_KP, ATT_WN, ATT_ZETA, G, INERTIA, K_YAW, MASS, MAX_ACC,
    MAX_TILT, POS_KD, POS_KP, SPIN, T_MAX, X_ARM, X_SIGN, YAW_KD, YAW_KP, Y_ARM, Y_SIGN,
};

/// What one controller step commands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlOutput {
    /// Normalized duct command, FL FR RL RR, each in `0..=1` (fraction of [`T_MAX`]).
    pub u: [f64; 4],
    /// Duct thrust, FL FR RL RR, newtons, each in `0..=T_MAX`.
    pub f: [f64; 4],
    /// Roll, pitch, yaw (radians) of the quaternion that was passed in.
    pub rpy: [f64; 3],
}

/// Plant state the cascade reads. Angles and rates are estimated-perfect: the
/// reference controller uses true state, and so does this port.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plant {
    /// World position of the body origin, m.
    pub pos: [f64; 3],
    /// Body orientation `(w, x, y, z)`.
    pub quat: [f64; 4],
    /// World linear velocity of the body origin, m/s.
    pub vel: [f64; 3],
    /// Body angular velocity, rad/s. The reference passes MuJoCo `qvel[3:6]`;
    /// on this model that matches the `gyro` sensor (the imu site is aligned
    /// with the body).
    pub omega: [f64; 3],
}

/// Where the cascade is trying to be.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Setpoint {
    /// Desired world position, m.
    pub pos: [f64; 3],
    /// Desired world velocity, m/s.
    pub vel: [f64; 3],
    /// Desired yaw, rad.
    pub yaw: f64,
    /// World acceleration feedforward, m/s². The JavaScript reference has no
    /// feedforward (it is zero). The Python reference takes `a_ff`.
    pub a_ff: [f64; 3],
}

impl Setpoint {
    /// Hold `pos` with zero velocity, zero yaw and no feedforward.
    pub fn hold(pos: [f64; 3]) -> Self {
        Self {
            pos,
            vel: [0.0; 3],
            yaw: 0.0,
            a_ff: [0.0; 3],
        }
    }
}

/// Thrust per duct (N) for a collective force and body torques.
///
/// Symmetric X layout, analytic inverse, then clipped to `0..=T_MAX`. Index
/// order is FL, FR, RL, RR.
pub fn mix(collective_n: f64, tx: f64, ty: f64, tz: f64) -> [f64; 4] {
    let mut f = [0.0; 4];
    for i in 0..4 {
        let xs = X_SIGN[i] * X_ARM;
        let ys = Y_SIGN[i] * Y_ARM;
        let raw =
            collective_n / 4.0 + tx / (4.0 * ys) - ty / (4.0 * xs) + tz / (4.0 * SPIN[i] * K_YAW);
        f[i] = raw.clamp(0.0, T_MAX);
    }
    f
}

/// Cascade controller. Holds the altitude integrator between steps.
#[derive(Clone, Debug)]
pub struct FlightController {
    iz: f64,
}

impl Default for FlightController {
    fn default() -> Self {
        Self::new()
    }
}

impl FlightController {
    /// Integrator at zero.
    pub fn new() -> Self {
        Self { iz: 0.0 }
    }

    /// Clear the altitude integrator. The stair-hop script does this when climb starts.
    pub fn reset(&mut self) {
        self.iz = 0.0;
    }

    /// Altitude integrator state, m·s. Clamped to ±[`ALT_I_LIMIT`].
    pub fn integrator(&self) -> f64 {
        self.iz
    }

    /// One control step. `dt` is seconds.
    pub fn step(&mut self, dt: f64, plant: &Plant, set: &Setpoint) -> ControlOutput {
        let rpy = quat_to_rpy(plant.quat);
        let [roll, pitch, yaw] = rpy;
        let mut e = [0.0; 3];
        let mut ev = [0.0; 3];
        for i in 0..3 {
            e[i] = set.pos[i] - plant.pos[i];
            ev[i] = set.vel[i] - plant.vel[i];
        }
        let mut ax = POS_KP * e[0] + POS_KD * ev[0] + set.a_ff[0];
        let mut ay = POS_KP * e[1] + POS_KD * ev[1] + set.a_ff[1];
        let n = (ax * ax + ay * ay).sqrt();
        if n > MAX_ACC {
            let s = MAX_ACC / n;
            ax *= s;
            ay *= s;
        }
        let (roll_d, pitch_d) = {
            let (roll_raw, pitch_raw) = crate::math::tilt_from_horizontal(ax, ay, yaw);
            (
                roll_raw.clamp(-MAX_TILT, MAX_TILT),
                pitch_raw.clamp(-MAX_TILT, MAX_TILT),
            )
        };
        self.iz = (self.iz + e[2] * dt).clamp(-ALT_I_LIMIT, ALT_I_LIMIT);
        let az = ALT_KP * e[2] + ALT_KD * ev[2] + ALT_KI * self.iz + set.a_ff[2];
        let collective = MASS * (G + az) / (roll.cos() * pitch.cos()).max(0.5);
        let [ix, iy, iz] = INERTIA;
        let tx =
            ix * (ATT_WN * ATT_WN * (roll_d - roll) - 2.0 * ATT_ZETA * ATT_WN * plant.omega[0]);
        let ty =
            iy * (ATT_WN * ATT_WN * (pitch_d - pitch) - 2.0 * ATT_ZETA * ATT_WN * plant.omega[1]);
        let tz = iz * (YAW_KP * wrap(set.yaw - yaw) - YAW_KD * plant.omega[2]);
        let f = mix(collective, tx, ty, tz);
        let u = [f[0] / T_MAX, f[1] / T_MAX, f[2] / T_MAX, f[3] / T_MAX];
        ControlOutput { u, f, rpy }
    }
}
