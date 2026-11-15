//! Flight-mode machine and the stair-hop mission.
//!
//! Modes are the five the host shows: ground, deploying, flight, landing, stowing.
//! The stair-hop script underneath is the phase list in `validate.py`:
//! drive → brake → deploy → climb → transit → descend → touchdown → stow →
//! roll → settle → done. A physics sim with estimated parameters; the script
//! is hand-tuned against the MuJoCo model, not against hardware.
//!
//! Same-iteration fall-through matches the Python script:
//! * deploy can enter climb, and the climb target is integrated on that same tick;
//! * descend can enter touchdown on the tick the altitude condition trips, the
//!   controller still runs (so the altitude integrator moves), then thrust is
//!   overwritten to zero. Battery energy on that tick uses the controller's
//!   thrust, as `validate.py` does (`bat.step` sees `thrusts`, after `ctrl` was
//!   zeroed).

use crate::control::{Plant, Setpoint};
use crate::math::{quat_to_rpy, yaw_of};
use crate::obs::{Obs, CTRL_LEN, PILOT_FOLD, PILOT_LAND, PILOT_TAKEOFF};
use crate::params::{FOLD_DEPLOY, FOLD_STOW, WHEEL_RAD_S};

/// Landing height of the staircase in `tbd.xml`, m.
pub const STAIR_TOP: f64 = 0.68;
/// World x where the landing deck starts, m. `LAND_X0` in `validate.py`.
pub const LAND_X0: f64 = 1.92;
/// Hover x over the deck the script flies to, m.
pub const XLAND: f64 = 2.30;
/// Climb ceiling, m (`STAIR_TOP + 0.32`).
pub const CLIMB_Z: f64 = STAIR_TOP + 0.32;
/// Scripted ground speed toward the stairs, rad/s at the wheel actuators.
const DRIVE_RAD_S: f64 = 24.0;
/// Manual flight speed cap, m/s. Same cruise the script uses in transit.
const MANUAL_SPEED: f64 = 0.7;
/// Manual climb and descend rate at full `pilot_z`, m/s.
const MANUAL_CLIMB: f64 = 0.9;
/// Manual yaw-rate stick scale, rad/s at full deflection.
const MANUAL_YAW_RATE: f64 = 1.2;
/// Altitude the manual mission holds after takeoff, m.
const MANUAL_HOLD_Z: f64 = 0.6;

/// The five modes a host should label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlightMode {
    /// Arms stowed, wheels may be driven.
    Ground,
    /// Fold command going to deployed. JSON name `deploying`.
    Deploying,
    /// Cascade controller in closed loop.
    Flight,
    /// Descending, or down with thrust cut, waiting to fold.
    Landing,
    /// Fold command going back to stowed. JSON name `stowing`.
    Stowing,
}

impl FlightMode {
    /// Stable name for JSON and the HUD.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ground => "ground",
            Self::Deploying => "deploying",
            Self::Flight => "flight",
            Self::Landing => "landing",
            Self::Stowing => "stowing",
        }
    }
}

/// Stair-hop script phase. Manual flight reports the mode name instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScriptPhase {
    Drive,
    Brake,
    Deploy,
    Climb,
    Transit,
    Descend,
    Touchdown,
    Stow,
    Roll,
    Settle,
    Done,
}

impl ScriptPhase {
    fn as_str(self) -> &'static str {
        match self {
            Self::Drive => "drive",
            Self::Brake => "brake",
            Self::Deploy => "deploy",
            Self::Climb => "climb",
            Self::Transit => "transit",
            Self::Descend => "descend",
            Self::Touchdown => "touchdown",
            Self::Stow => "stow",
            Self::Roll => "roll",
            Self::Settle => "settle",
            Self::Done => "done",
        }
    }

    fn mode(self) -> FlightMode {
        match self {
            Self::Drive | Self::Brake | Self::Roll | Self::Settle | Self::Done => {
                FlightMode::Ground
            }
            Self::Deploy => FlightMode::Deploying,
            Self::Climb | Self::Transit | Self::Descend => FlightMode::Flight,
            Self::Touchdown => FlightMode::Landing,
            Self::Stow => FlightMode::Stowing,
        }
    }
}

/// What one mission tick wants the game to write.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Command {
    pub mode: FlightMode,
    pub phase: &'static str,
    /// Run the cascade on this setpoint. `None` means zero thrust.
    pub flight: Option<Setpoint>,
    /// Call the cascade (integrator, battery thrust) but write zero thrust.
    /// Set on the descend → touchdown tick, matching `validate.py`.
    pub zero_thrust: bool,
    /// Clear the altitude integrator before the cascade runs.
    pub reset_fc: bool,
    pub drive_left: f64,
    pub drive_right: f64,
    pub fold: f64,
    /// Update tilt and stair clearance from this tick.
    pub flight_metrics: bool,
    pub completed: bool,
}

impl Command {
    fn ground(phase: &'static str, left: f64, right: f64, fold: f64) -> Self {
        Self {
            mode: FlightMode::Ground,
            phase,
            flight: None,
            zero_thrust: false,
            reset_fc: false,
            drive_left: left,
            drive_right: right,
            fold,
            flight_metrics: false,
            completed: false,
        }
    }
}

/// Running score inputs the stair-hop script fills in.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ScoreInput {
    pub landed: bool,
    pub completed: bool,
    pub max_tilt_deg: f64,
    pub min_clearance_m: Option<f64>,
    pub progress_x: f64,
}

/// Stair-hop points.
///
/// Landed on the deck (`landed_z > STAIR_TOP - 0.01`, the `validate.py` check):
///
/// ```text
/// max(0, 1000 − 25·duration_s − 4·max_tilt_deg − 8·battery_pct_used + 200·clearance_m)
/// ```
///
/// `clearance_m` is the minimum of `pos_z − step_height` while the body is over
/// the stairs (`0.55 < x < LAND_X0`), clamped to `[0, 0.5]` for the bonus. If the
/// body never entered that band the bonus is 0.
///
/// Not yet landed: `100 · clamp(x / XLAND, 0, 1)`, so a run that never hops still
/// scores how far the drive got.
pub fn stair_score(
    landed: bool,
    duration_s: f64,
    max_tilt_deg: f64,
    battery_pct_used: f64,
    min_clearance_m: Option<f64>,
    progress_x: f64,
) -> f64 {
    if !landed {
        return 100.0 * (progress_x / XLAND).clamp(0.0, 1.0);
    }
    let clear = min_clearance_m.unwrap_or(0.0).clamp(0.0, 0.5);
    (1000.0 - 25.0 * duration_s - 4.0 * max_tilt_deg - 8.0 * battery_pct_used + 200.0 * clear)
        .max(0.0)
}

/// Step riser the clearance check uses, m. Mirrors `validate.py`.
fn step_height(x: f64) -> f64 {
    if x + 0.16 <= 0.80 {
        return 0.0;
    }
    let n = ((x + 0.16 - 0.80) / 0.28).floor() + 1.0;
    0.17 * n.clamp(0.0, 4.0)
}

#[derive(Clone, Debug)]
pub(crate) struct StairHop {
    phase: ScriptPhase,
    t_phase: f64,
    target: [f64; 3],
    v_des: [f64; 3],
    max_tilt_rad: f64,
    min_clearance_m: Option<f64>,
    landed_z: Option<f64>,
}

impl StairHop {
    fn new() -> Self {
        Self {
            phase: ScriptPhase::Drive,
            t_phase: 0.0,
            target: [0.0; 3],
            v_des: [0.0; 3],
            max_tilt_rad: 0.0,
            min_clearance_m: None,
            landed_z: None,
        }
    }

    fn set(&mut self, phase: ScriptPhase, t: f64) {
        self.phase = phase;
        self.t_phase = t;
    }

    fn advance(&mut self, obs: &Obs, dt: f64, t: f64) -> Command {
        let mut reset_fc = false;
        let tp = t - self.t_phase;
        // Flight control runs only if this tick started in the air. Entering
        // climb from deploy on this tick does not thrust until the next tick.
        let phase_at_start = self.phase;

        match self.phase {
            ScriptPhase::Drive if obs.pos[0] >= 0.42 => self.set(ScriptPhase::Brake, t),
            ScriptPhase::Brake if tp > 0.5 => self.set(ScriptPhase::Deploy, t),
            ScriptPhase::Deploy if obs.fold < 0.04 && tp > 0.4 => {
                self.target = obs.pos;
                reset_fc = true;
                self.set(ScriptPhase::Climb, t);
            }
            _ => {}
        }

        let mut flight: Option<Setpoint> = None;
        let mut flight_metrics = false;
        if matches!(
            phase_at_start,
            ScriptPhase::Climb | ScriptPhase::Transit | ScriptPhase::Descend
        ) {
            match self.phase {
                ScriptPhase::Climb => {
                    self.target[2] = (self.target[2] + 0.9 * dt).min(CLIMB_Z);
                    let vz = if self.target[2] < CLIMB_Z { 0.9 } else { 0.0 };
                    self.v_des = [0.0, 0.0, vz];
                    if obs.pos[2] > CLIMB_Z - 0.06 {
                        self.set(ScriptPhase::Transit, t);
                    }
                }
                ScriptPhase::Transit => {
                    self.target[0] = (self.target[0] + 0.7 * dt).min(XLAND);
                    let vx = if self.target[0] < XLAND { 0.7 } else { 0.0 };
                    self.v_des = [vx, 0.0, 0.0];
                    let speed = (obs.vel[0] * obs.vel[0]
                        + obs.vel[1] * obs.vel[1]
                        + obs.vel[2] * obs.vel[2])
                        .sqrt();
                    if (obs.pos[0] - XLAND).abs() < 0.03 && speed < 0.1 {
                        self.set(ScriptPhase::Descend, t);
                    }
                }
                ScriptPhase::Descend => {
                    self.target[2] = (self.target[2] - 0.35 * dt).max(STAIR_TOP - 0.05);
                    self.v_des = [0.0, 0.0, -0.35];
                    if obs.pos[2] < STAIR_TOP + 0.004 && tp > 0.5 {
                        self.set(ScriptPhase::Touchdown, t);
                    }
                }
                _ => {}
            }
            flight = Some(Setpoint {
                pos: self.target,
                vel: self.v_des,
                yaw: 0.0,
                a_ff: [0.0; 3],
            });
            flight_metrics = true;
            let [roll, pitch, _] = quat_to_rpy(obs.quat);
            self.max_tilt_rad = self.max_tilt_rad.max(roll.abs()).max(pitch.abs());
            if obs.pos[0] > 0.55 && obs.pos[0] < LAND_X0 {
                let clear = obs.pos[2] - step_height(obs.pos[0]);
                self.min_clearance_m = Some(match self.min_clearance_m {
                    Some(m) => m.min(clear),
                    None => clear,
                });
            }
        }

        let mut zero_thrust = false;
        let mut completed = false;
        let (drive_left, drive_right, fold) = match self.phase {
            ScriptPhase::Drive => (DRIVE_RAD_S, DRIVE_RAD_S, FOLD_STOW),
            ScriptPhase::Brake => (0.0, 0.0, FOLD_STOW),
            ScriptPhase::Deploy
            | ScriptPhase::Climb
            | ScriptPhase::Transit
            | ScriptPhase::Descend => (0.0, 0.0, FOLD_DEPLOY),
            ScriptPhase::Touchdown => {
                zero_thrust = true;
                let mut fold = FOLD_DEPLOY;
                if t - self.t_phase > 0.4 {
                    fold = FOLD_STOW;
                    self.landed_z = Some(obs.pos[2]);
                    self.set(ScriptPhase::Stow, t);
                }
                (0.0, 0.0, fold)
            }
            ScriptPhase::Stow => {
                if t - self.t_phase > 0.9 {
                    self.set(ScriptPhase::Roll, t);
                }
                (0.0, 0.0, FOLD_STOW)
            }
            ScriptPhase::Roll => {
                if t - self.t_phase > 1.0 {
                    self.set(ScriptPhase::Settle, t);
                    (0.0, 0.0, FOLD_STOW)
                } else {
                    (DRIVE_RAD_S, DRIVE_RAD_S, FOLD_STOW)
                }
            }
            ScriptPhase::Settle => {
                if t - self.t_phase > 0.6 {
                    self.set(ScriptPhase::Done, t);
                    completed = true;
                }
                (0.0, 0.0, FOLD_STOW)
            }
            ScriptPhase::Done => {
                completed = true;
                (0.0, 0.0, FOLD_STOW)
            }
        };

        // A phase change inside the flight block updates `mode` to wherever we
        // ended (transit after a climb exit, touchdown after a descend exit).
        let phase = self.phase;
        Command {
            mode: phase.mode(),
            phase: phase.as_str(),
            flight,
            zero_thrust,
            reset_fc,
            drive_left,
            drive_right,
            fold,
            flight_metrics,
            completed,
        }
    }

    fn score_input(&self, progress_x: f64) -> ScoreInput {
        ScoreInput {
            landed: self.landed_z.is_some_and(|z| z > STAIR_TOP - 0.01),
            completed: self.phase == ScriptPhase::Done,
            max_tilt_deg: self.max_tilt_rad.to_degrees(),
            min_clearance_m: self.min_clearance_m,
            progress_x,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ManualPhase {
    Ground,
    Deploying,
    Flight,
    Landing,
    Stowing,
}

impl ManualPhase {
    fn mode(self) -> FlightMode {
        match self {
            Self::Ground => FlightMode::Ground,
            Self::Deploying => FlightMode::Deploying,
            Self::Flight => FlightMode::Flight,
            Self::Landing => FlightMode::Landing,
            Self::Stowing => FlightMode::Stowing,
        }
    }

    fn as_str(self) -> &'static str {
        self.mode().as_str()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ManualFly {
    phase: ManualPhase,
    t_phase: f64,
    hold: [f64; 3],
    yaw_des: f64,
    v_des: [f64; 3],
}

impl ManualFly {
    fn new() -> Self {
        Self {
            phase: ManualPhase::Ground,
            t_phase: 0.0,
            hold: [0.0, 0.0, MANUAL_HOLD_Z],
            yaw_des: 0.0,
            v_des: [0.0; 3],
        }
    }

    fn set(&mut self, phase: ManualPhase, t: f64) {
        self.phase = phase;
        self.t_phase = t;
    }

    fn advance(&mut self, obs: &Obs, dt: f64, t: f64) -> Command {
        let takeoff = obs.pilot_x >= PILOT_TAKEOFF;
        let land = obs.pilot_x <= PILOT_LAND;
        let fold_req = obs.pilot_yaw >= PILOT_FOLD;
        let mut reset_fc = false;
        let age = t - self.t_phase;

        match self.phase {
            ManualPhase::Ground if takeoff => self.set(ManualPhase::Deploying, t),
            ManualPhase::Deploying if obs.fold < 0.04 && age > 0.4 => {
                self.hold = [obs.pos[0], obs.pos[1], MANUAL_HOLD_Z];
                self.yaw_des = yaw_of(obs.quat);
                self.v_des = [0.0; 3];
                reset_fc = true;
                self.set(ManualPhase::Flight, t);
            }
            ManualPhase::Flight if land => {
                self.hold = obs.pos;
                self.set(ManualPhase::Landing, t);
            }
            // Touchdown with the stow stick already held folds on that tick.
            ManualPhase::Landing if fold_req && low_enough(obs) => {
                self.set(ManualPhase::Stowing, t);
            }
            // Still airborne: takeoff stick returns to flight. No new edge required.
            ManualPhase::Landing if takeoff && !low_enough(obs) => {
                self.hold[2] = obs.pos[2];
                reset_fc = true;
                self.set(ManualPhase::Flight, t);
            }
            ManualPhase::Stowing if obs.fold > FOLD_STOW - 0.05 && age > 0.2 => {
                self.set(ManualPhase::Ground, t);
            }
            _ => {}
        }

        let phase = self.phase;
        let mut cmd = Command::ground(phase.as_str(), 0.0, 0.0, FOLD_STOW);
        cmd.mode = phase.mode();
        cmd.reset_fc = reset_fc;
        match phase {
            ManualPhase::Ground => {
                let (left, right) = stick_drive(obs.pilot_x, obs.pilot_yaw);
                cmd.drive_left = left;
                cmd.drive_right = right;
                cmd.fold = FOLD_STOW;
            }
            ManualPhase::Deploying => {
                cmd.fold = FOLD_DEPLOY;
            }
            ManualPhase::Flight => {
                let yaw = yaw_of(obs.quat);
                let surge = stick_axis(obs.pilot_x) * MANUAL_SPEED;
                let strafe = stick_axis(obs.pilot_y) * MANUAL_SPEED;
                let climb = stick_axis(obs.pilot_z) * MANUAL_CLIMB;
                let yaw_rate = stick_axis(obs.pilot_yaw) * MANUAL_YAW_RATE;
                let (cy, sy) = (yaw.cos(), yaw.sin());
                self.yaw_des = crate::math::wrap(self.yaw_des + yaw_rate * dt);
                self.v_des = [cy * surge - sy * strafe, sy * surge + cy * strafe, climb];
                self.hold[0] += self.v_des[0] * dt;
                self.hold[1] += self.v_des[1] * dt;
                self.hold[2] += climb * dt;
                cmd.flight = Some(Setpoint {
                    pos: self.hold,
                    vel: self.v_des,
                    yaw: self.yaw_des,
                    a_ff: [0.0; 3],
                });
                cmd.fold = FOLD_DEPLOY;
                cmd.flight_metrics = true;
            }
            ManualPhase::Landing => {
                let on_ground = low_enough(obs);
                if on_ground {
                    cmd.zero_thrust = true;
                    cmd.fold = FOLD_DEPLOY;
                } else {
                    self.hold[2] = (self.hold[2] - 0.35 * dt).max(0.0);
                    self.v_des = [0.0, 0.0, -0.35];
                    cmd.flight = Some(Setpoint {
                        pos: self.hold,
                        vel: self.v_des,
                        yaw: self.yaw_des,
                        a_ff: [0.0; 3],
                    });
                    cmd.fold = FOLD_DEPLOY;
                    cmd.flight_metrics = true;
                }
            }
            ManualPhase::Stowing => {
                cmd.fold = FOLD_STOW;
            }
        }
        cmd
    }
}

fn low_enough(obs: &Obs) -> bool {
    // range_down of 0 means the channel was left unset. A real hit is positive.
    obs.pos[2] < 0.04 || (obs.range_down > 0.0 && obs.range_down < 0.05)
}

fn stick_axis(v: f64) -> f64 {
    if v.abs() <= 1.0 {
        v
    } else {
        0.0
    }
}

fn stick_drive(pilot_x: f64, pilot_yaw: f64) -> (f64, f64) {
    // A sentinel lives outside ±1. Don't also drive on that channel this tick.
    let x = if pilot_x.abs() <= 1.0 { pilot_x } else { 0.0 };
    let yaw = if pilot_yaw.abs() <= 1.0 {
        pilot_yaw
    } else {
        0.0
    };
    scaled_drive(x, yaw)
}

fn scaled_drive(pilot_x: f64, pilot_yaw: f64) -> (f64, f64) {
    let x = pilot_x.clamp(-1.0, 1.0);
    let yaw = pilot_yaw.clamp(-1.0, 1.0);
    let left = (x - yaw).clamp(-1.0, 1.0) * WHEEL_RAD_S;
    let right = (x + yaw).clamp(-1.0, 1.0) * WHEEL_RAD_S;
    (left, right)
}

#[derive(Clone, Debug)]
pub(crate) enum Mission {
    Stair(StairHop),
    Manual(ManualFly),
}

impl Mission {
    pub(crate) fn stair() -> Self {
        Self::Stair(StairHop::new())
    }

    pub(crate) fn manual() -> Self {
        Self::Manual(ManualFly::new())
    }

    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::Stair(_) => "stair_hop",
            Self::Manual(_) => "manual",
        }
    }

    pub(crate) fn advance(&mut self, obs: &Obs, dt: f64, t: f64) -> Command {
        match self {
            Self::Stair(s) => s.advance(obs, dt, t),
            Self::Manual(m) => m.advance(obs, dt, t),
        }
    }

    pub(crate) fn score_input(&self, progress_x: f64) -> ScoreInput {
        match self {
            Self::Stair(s) => s.score_input(progress_x),
            Self::Manual(_) => ScoreInput {
                landed: false,
                completed: false,
                max_tilt_deg: 0.0,
                min_clearance_m: None,
                progress_x,
            },
        }
    }
}

/// Plant view the cascade wants, from an observation.
pub(crate) fn plant_of(obs: &Obs) -> Plant {
    Plant {
        pos: obs.pos,
        quat: obs.quat,
        vel: obs.vel,
        omega: obs.gyro,
    }
}

/// Write `cmd` into a 9-wide actuator buffer. Thrust slots stay 0; the game
/// fills them from the cascade.
pub(crate) fn write_axes(ctrl: &mut [f64; CTRL_LEN], cmd: &Command) {
    ctrl[4] = cmd.drive_left;
    ctrl[5] = cmd.drive_right;
    ctrl[6] = cmd.drive_left;
    ctrl[7] = cmd.drive_right;
    ctrl[8] = cmd.fold;
}
