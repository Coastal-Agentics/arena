//! Flat observation in, flat actuator command out.
//!
//! The arrays follow `assets/tbd/tbd.xml`. Sensors are concatenated in file
//! order; the free-joint position and linear velocity follow, because the
//! cascade needs them and they are not in `<sensor>`. Two pilot channels close
//! the 20. Actuator commands are `nu = 9`, in `<actuator>` order.
//!
//! ```text
//! observation (20 f64), one physics tick
//! index  name          unit     source in tbd.xml
//! 0      gyro_x        rad/s    sensor gyro (site imu), x
//! 1      gyro_y        rad/s    sensor gyro, y
//! 2      gyro_z        rad/s    sensor gyro, z
//! 3      accel_x       m/s²     sensor accelerometer (site imu), x
//! 4      accel_y       m/s²     sensor accelerometer, y
//! 5      accel_z       m/s²     sensor accelerometer, z
//! 6      quat_w        1        sensor framequat (site imu)
//! 7      quat_x        1        sensor framequat
//! 8      quat_y        1        sensor framequat
//! 9      quat_z        1        sensor framequat
//! 10     range_down    m        sensor rangefinder range_down; MuJoCo writes -1 on a miss
//! 11     fold_angle    rad      sensor jointpos fold_angle (joint fold_FL)
//! 12     pos_x         m        free joint root, qpos[0]
//! 13     pos_y         m        free joint root, qpos[1]
//! 14     pos_z         m        free joint root, qpos[2]
//! 15     vel_x         m/s      free joint root, qvel[0] (world)
//! 16     vel_y         m/s      free joint root, qvel[1]
//! 17     vel_z         m/s      free joint root, qvel[2]
//! 18     pilot_x       1        host stick, see below
//! 19     pilot_yaw     1        host stick, see below
//! 20     pilot_y       1        manual strafe, body left; 0 if the array is 20 long
//! 21     pilot_z       1        manual climb rate; 0 if the array is 20 long
//!
//! control (9 f64), copied straight into MjData.ctrl
//! index  name          range           actuator
//! 0      thrust_FL     0 .. 1          thrust_FL
//! 1      thrust_FR     0 .. 1          thrust_FR
//! 2      thrust_RL     0 .. 1          thrust_RL
//! 3      thrust_RR     0 .. 1          thrust_RR
//! 4      drive_FL      -26 .. 26 rad/s drive_FL
//! 5      drive_FR      -26 .. 26 rad/s drive_FR
//! 6      drive_RL      -26 .. 26 rad/s drive_RL
//! 7      drive_RR      -26 .. 26 rad/s drive_RR
//! 8      fold          0 .. 1.5708 rad fold (0 deployed, 1.5708 stowed)
//! ```
//!
//! The imu site is aligned with the body, so the gyro matches the body rates
//! the reference controller reads from `qvel[3:6]`, and the frame quaternion
//! matches the free-joint quaternion. `pos` and `vel` are the body origin, not
//! the imu site (the site sits 60 mm above the origin).
//!
//! Pilot channels are used by the `manual` mission only. The stair-hop mission
//! ignores them (they still count as input-log data). Inside `[-1, 1]`,
//! `pilot_x` is forward drive on the ground and forward speed in flight;
//! `pilot_yaw` is skid steer on the ground and a yaw-rate command in flight.
//! Positive `pilot_x` commands both wheel sides positive, which on this model
//! drives toward +X (the stairs), matching `drive(24, 24)` in `validate.py`.
//! Positive `pilot_yaw` speeds the right side and slows the left (nose left).
//!
//! Sentinels, checked before the stick (a sentinel tick does not also drive):
//! * `pilot_x >= 2` requests takeoff (ground → deploying → flight), and while
//!   landing and still airborne it returns to flight;
//! * `pilot_x <= -2` requests landing;
//! * `pilot_yaw >= 2` requests stowing. If it is already held at touchdown,
//!   stowing starts on that tick.
//!
//! `pilot_y` and `pilot_z` are manual-only, inside ±1. `pilot_y` +1 strafes
//! left (body +Y) at 0.7 m/s. `pilot_z` +1 climbs at 0.9 m/s and −1 descends
//! at 0.9 m/s. The stair-hop mission ignores all four sticks.

use crate::params::{FOLD_STOW, WHEEL_RAD_S};

/// Length of the flat observation. Indexes 0..19 match the first layout;
/// 20 and 21 are the manual strafe and climb sticks. A 20-long array is still
/// accepted and those two sticks read as 0.
pub const OBS_LEN: usize = 22;
/// First observation length, without `pilot_y` and `pilot_z`. Still accepted.
pub const OBS_LEN_V1: usize = 20;
/// Length of the flat actuator command.
pub const CTRL_LEN: usize = 9;
/// Number of `<sensor>` values concatenated at the front of the observation.
pub const SENSOR_LEN: usize = 12;

/// `pilot_x` at or above this requests takeoff.
pub const PILOT_TAKEOFF: f64 = 2.0;
/// `pilot_x` at or below this requests landing.
pub const PILOT_LAND: f64 = -2.0;
/// `pilot_yaw` at or above this requests stowing.
pub const PILOT_FOLD: f64 = 2.0;

/// Observation field names, index order.
pub const OBS_NAMES: [&str; OBS_LEN] = [
    "gyro_x",
    "gyro_y",
    "gyro_z",
    "accel_x",
    "accel_y",
    "accel_z",
    "quat_w",
    "quat_x",
    "quat_y",
    "quat_z",
    "range_down",
    "fold_angle",
    "pos_x",
    "pos_y",
    "pos_z",
    "vel_x",
    "vel_y",
    "vel_z",
    "pilot_x",
    "pilot_yaw",
    "pilot_y",
    "pilot_z",
];

/// Actuator command names, index order. These are the `name` attributes in
/// `tbd.xml` `<actuator>`, in file order.
pub const CTRL_NAMES: [&str; CTRL_LEN] = [
    "thrust_FL",
    "thrust_FR",
    "thrust_RL",
    "thrust_RR",
    "drive_FL",
    "drive_FR",
    "drive_RL",
    "drive_RR",
    "fold",
];

/// Sensor elements at the front of the observation: `(xml name, width)`.
pub const SENSOR_SPEC: [(&str, usize); 5] = [
    ("gyro", 3),
    ("accel", 3),
    ("quat", 4),
    ("range_down", 1),
    ("fold_angle", 1),
];

/// Why [`crate::TbdGame::step`] rejected an observation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepError {
    /// `obs.len()` was not [`OBS_LEN`].
    Length {
        /// Expected length.
        expected: usize,
        /// Actual length.
        got: usize,
    },
    /// `obs[index]` was NaN or infinite.
    NonFinite {
        /// Index of the bad sample.
        index: usize,
    },
    /// `sensordata` passed to [`pack_observation`] was not [`SENSOR_LEN`] long.
    SensorLength {
        /// Expected length.
        expected: usize,
        /// Actual length.
        got: usize,
    },
}

impl std::fmt::Display for StepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Length { expected, got } => {
                write!(
                    f,
                    "observation length {got}, expected {expected} (or {OBS_LEN_V1})"
                )
            }
            Self::NonFinite { index } => write!(f, "observation[{index}] is not finite"),
            Self::SensorLength { expected, got } => {
                write!(f, "sensordata length {got}, expected {expected}")
            }
        }
    }
}

impl std::error::Error for StepError {}

/// One tick of plant state plus the pilot stick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Obs {
    /// Body rates, rad/s.
    pub gyro: [f64; 3],
    /// Specific force at the imu, m/s². Recorded; the cascade does not use it.
    pub accel: [f64; 3],
    /// Orientation `(w, x, y, z)`.
    pub quat: [f64; 4],
    /// Downward range, m. Negative means no return.
    pub range_down: f64,
    /// `fold_FL` angle, rad.
    pub fold: f64,
    /// Body-origin position, m.
    pub pos: [f64; 3],
    /// Body-origin linear velocity, world, m/s.
    pub vel: [f64; 3],
    /// Longitudinal stick, or a takeoff/land sentinel.
    pub pilot_x: f64,
    /// Yaw stick, or a stow sentinel.
    pub pilot_yaw: f64,
    /// Strafe stick, body left. Manual flight only.
    pub pilot_y: f64,
    /// Climb-rate stick, positive up. Manual flight only.
    pub pilot_z: f64,
}

impl Obs {
    /// Decode a flat observation. Length [`OBS_LEN`] or [`OBS_LEN_V1`].
    pub fn parse(obs: &[f64]) -> Result<Self, StepError> {
        if obs.len() != OBS_LEN && obs.len() != OBS_LEN_V1 {
            return Err(StepError::Length {
                expected: OBS_LEN,
                got: obs.len(),
            });
        }
        for (i, v) in obs.iter().enumerate() {
            if !v.is_finite() {
                return Err(StepError::NonFinite { index: i });
            }
        }
        Ok(Self {
            gyro: [obs[0], obs[1], obs[2]],
            accel: [obs[3], obs[4], obs[5]],
            quat: [obs[6], obs[7], obs[8], obs[9]],
            range_down: obs[10],
            fold: obs[11],
            pos: [obs[12], obs[13], obs[14]],
            vel: [obs[15], obs[16], obs[17]],
            pilot_x: obs[18],
            pilot_yaw: obs[19],
            pilot_y: if obs.len() == OBS_LEN { obs[20] } else { 0.0 },
            pilot_z: if obs.len() == OBS_LEN { obs[21] } else { 0.0 },
        })
    }

    /// Encode to the flat layout.
    pub fn to_array(self) -> [f64; OBS_LEN] {
        let mut o = [0.0; OBS_LEN];
        o[0..3].copy_from_slice(&self.gyro);
        o[3..6].copy_from_slice(&self.accel);
        o[6..10].copy_from_slice(&self.quat);
        o[10] = self.range_down;
        o[11] = self.fold;
        o[12..15].copy_from_slice(&self.pos);
        o[15..18].copy_from_slice(&self.vel);
        o[18] = self.pilot_x;
        o[19] = self.pilot_yaw;
        o[20] = self.pilot_y;
        o[21] = self.pilot_z;
        o
    }
}

/// Build the flat observation from a MuJoCo `sensordata` block (file order,
/// length [`SENSOR_LEN`]), the free-joint position and linear velocity, and the
/// pilot channels.
pub fn pack_observation(
    sensordata: &[f64],
    pos: [f64; 3],
    vel: [f64; 3],
    pilot_x: f64,
    pilot_yaw: f64,
    pilot_y: f64,
    pilot_z: f64,
) -> Result<[f64; OBS_LEN], StepError> {
    if sensordata.len() != SENSOR_LEN {
        return Err(StepError::SensorLength {
            expected: SENSOR_LEN,
            got: sensordata.len(),
        });
    }
    let mut gyro = [0.0; 3];
    let mut accel = [0.0; 3];
    let mut quat = [0.0; 4];
    gyro.copy_from_slice(&sensordata[0..3]);
    accel.copy_from_slice(&sensordata[3..6]);
    quat.copy_from_slice(&sensordata[6..10]);
    Obs {
        gyro,
        accel,
        quat,
        range_down: sensordata[10],
        fold: sensordata[11],
        pos,
        vel,
        pilot_x,
        pilot_yaw,
        pilot_y,
        pilot_z,
    }
    .to_array()
    .pipe_finite()
}

trait FiniteCheck {
    fn pipe_finite(self) -> Result<Self, StepError>
    where
        Self: Sized;
}

impl FiniteCheck for [f64; OBS_LEN] {
    fn pipe_finite(self) -> Result<Self, StepError> {
        for (i, v) in self.iter().enumerate() {
            if !v.is_finite() {
                return Err(StepError::NonFinite { index: i });
            }
        }
        Ok(self)
    }
}

/// Clip a command into the actuator ranges in `tbd.xml`.
pub fn clip_ctrl(mut ctrl: [f64; CTRL_LEN]) -> [f64; CTRL_LEN] {
    for c in &mut ctrl[0..4] {
        *c = c.clamp(0.0, 1.0);
    }
    for c in &mut ctrl[4..8] {
        *c = c.clamp(-WHEEL_RAD_S, WHEEL_RAD_S);
    }
    ctrl[8] = ctrl[8].clamp(0.0, FOLD_STOW);
    ctrl
}

/// True when any wheel command is non-zero. The battery treats that as driving.
pub fn driving_flag(ctrl: &[f64; CTRL_LEN]) -> f64 {
    if ctrl[4].abs() + ctrl[5].abs() + ctrl[6].abs() + ctrl[7].abs() > 1e-9 {
        1.0
    } else {
        0.0
    }
}
