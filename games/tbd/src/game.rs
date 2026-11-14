//! One TBD run: controller, battery, mode machine, mission, wind.
//!
//! [`TbdGame::step`] is one physics tick. The host owns MuJoCo. Replaying the
//! same seed, config and observation log reproduces the actuator commands, the
//! battery, the mission and the wind force.

use crate::battery::Battery;
use crate::control::{ControlOutput, FlightController};
use crate::mission::{self, plant_of, stair_score, FlightMode};
use crate::obs::{clip_ctrl, driving_flag, Obs, StepError, CTRL_LEN};
use crate::params::{DT_MODEL, FOLD_STOW};
use crate::wind::Wind;
use serde::{Deserialize, Serialize};

/// Which mission [`TbdGame`] runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MissionId {
    /// Scripted hop onto the staircase landing. The default.
    #[default]
    StairHop,
    /// Stick and sentinel control. See the observation layout.
    Manual,
}

/// Run configuration. Parsed from the JSON passed to [`TbdGame::from_json`].
///
/// Unknown fields are rejected. Missing fields take the defaults: `dt` 0.002 s
/// (the `tbd.xml` timestep), mission `stair_hop`, wind on, `max_time_s` 16
/// (the script's horizon in `validate.py`).
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Seconds per [`TbdGame::step`]. Must be finite and positive.
    #[serde(default = "default_dt")]
    pub dt: f64,
    /// `stair_hop` or `manual`.
    #[serde(default)]
    pub mission: MissionId,
    /// Seeded wind disturbance. The controller does not cancel it.
    #[serde(default = "default_true")]
    pub wind: bool,
    /// Stop advancing once simulated time reaches this, seconds.
    #[serde(default = "default_max_time")]
    pub max_time_s: f64,
}

fn default_dt() -> f64 {
    DT_MODEL
}
fn default_true() -> bool {
    true
}
fn default_max_time() -> f64 {
    16.0
}

impl Default for Config {
    fn default() -> Self {
        Self {
            dt: default_dt(),
            mission: MissionId::StairHop,
            wind: true,
            max_time_s: default_max_time(),
        }
    }
}

impl Config {
    fn validate(&self) -> Result<(), ConfigError> {
        if !(self.dt.is_finite() && self.dt > 0.0) {
            return Err(ConfigError::Dt);
        }
        if !(self.max_time_s.is_finite() && self.max_time_s > 0.0) {
            return Err(ConfigError::MaxTime);
        }
        Ok(())
    }
}

/// Why a config was rejected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// `config_json` was not an object this crate accepts.
    Json(String),
    /// `dt` was missing a finite positive value.
    Dt,
    /// `max_time_s` was missing a finite positive value.
    MaxTime,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(s) => write!(f, "config json: {s}"),
            Self::Dt => write!(f, "dt must be a finite number greater than 0"),
            Self::MaxTime => write!(f, "max_time_s must be a finite number greater than 0"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// HUD / trace snapshot. [`TbdGame::state_json`] serializes this.
///
/// `wind_force_n` is the estimated world-frame force the host should write
/// into MuJoCo `xfrc_applied` on body `tbd` before `mj_step` (torque left at
/// zero). `score` follows [`stair_score`] for `stair_hop` and stays 0 for
/// `manual`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StateView {
    /// Steps taken. 0 before the first step.
    pub tick: u64,
    /// Simulated time after the last step, seconds.
    pub time_s: f64,
    /// Decimal seed, so a JSON number is not required to hold every `u64`.
    pub seed: String,
    /// `ground`, `unfolding`, `flight`, `landing` or `folding`.
    pub mode: &'static str,
    /// `stair_hop` or `manual`.
    pub mission: &'static str,
    /// Script phase (`drive`, `climb`, …) or, in manual, the same word as `mode`.
    pub phase: &'static str,
    /// True once the mission finishes or `max_time_s` is reached.
    /// Further [`TbdGame::step`] calls return the last command and do not draw wind.
    pub over: bool,
    /// Pack state of charge, percent.
    pub battery_pct: f64,
    /// Electrical power from the last step, watts.
    pub power_w: f64,
    /// Energy taken from the pack, Wh.
    pub wh_used: f64,
    /// Estimated hover endurance at 2.3 gf/W, minutes.
    pub hover_minutes_est: f64,
    /// Seeded wind velocity, world frame, m/s.
    pub wind_world_mps: [f64; 3],
    /// Estimated wind force, world frame, N.
    pub wind_force_n: [f64; 3],
    /// Mission points. See [`stair_score`].
    pub score: f64,
    /// Mission clock, seconds (same as `time_s`).
    pub timer_s: f64,
    /// Stair-hop script reached `done`.
    pub completed: bool,
    /// Touchdown height cleared `STAIR_TOP - 0.01`.
    pub landed_on_landing: bool,
    /// Peak |roll| or |pitch| during flight, degrees.
    pub max_tilt_deg: f64,
    /// Minimum body-origin clearance over the stairs, m. Null if never measured.
    pub min_clearance_m: Option<f64>,
    /// `100 - battery_pct`.
    pub battery_pct_used: f64,
    /// Body-origin position from the last observation, m.
    pub pos: [f64; 3],
    /// Fold angle from the last observation, rad.
    pub fold_rad: f64,
    /// Configured timestep, seconds.
    pub dt: f64,
    /// Last actuator command, [`CTRL_LEN`] values.
    pub ctrl: [f64; CTRL_LEN],
}

/// Flight assist plus mission for one seed.
#[derive(Clone, Debug)]
pub struct TbdGame {
    seed: u64,
    config: Config,
    fc: FlightController,
    battery: Battery,
    wind: Wind,
    mission: mission::Mission,
    tick: u64,
    time_s: f64,
    over: bool,
    mode: FlightMode,
    phase: &'static str,
    last_ctrl: [f64; CTRL_LEN],
    last_pos: [f64; 3],
    last_fold: f64,
    last_output: Option<ControlOutput>,
}

impl TbdGame {
    /// Start from a checked config.
    pub fn new(seed: u64, config: Config) -> Result<Self, ConfigError> {
        config.validate()?;
        Ok(Self::from_parts(seed, config))
    }

    /// `config_json` is a [`Config`] object. An empty string is the default config.
    ///
    /// A JSON array is rejected. Serde would otherwise accept `[]` as "every
    /// field defaulted", which is not a config document.
    pub fn from_json(seed: u64, config_json: &str) -> Result<Self, ConfigError> {
        let config = if config_json.trim().is_empty() {
            Config::default()
        } else {
            let value: serde_json::Value =
                serde_json::from_str(config_json).map_err(|e| ConfigError::Json(e.to_string()))?;
            if !value.is_object() {
                return Err(ConfigError::Json(
                    "config must be a JSON object".to_string(),
                ));
            }
            serde_json::from_value(value).map_err(|e| ConfigError::Json(e.to_string()))?
        };
        Self::new(seed, config)
    }

    fn from_parts(seed: u64, config: Config) -> Self {
        let (mission, mode, phase) = match config.mission {
            MissionId::StairHop => (mission::Mission::stair(), FlightMode::Ground, "drive"),
            MissionId::Manual => (mission::Mission::manual(), FlightMode::Ground, "ground"),
        };
        let mut last_ctrl = [0.0; CTRL_LEN];
        last_ctrl[8] = FOLD_STOW;
        Self {
            seed,
            wind: Wind::new(seed, config.wind),
            config,
            fc: FlightController::new(),
            battery: Battery::new(),
            mission,
            tick: 0,
            time_s: 0.0,
            over: false,
            mode,
            phase,
            last_ctrl,
            last_pos: [0.0; 3],
            last_fold: FOLD_STOW,
            last_output: None,
        }
    }

    /// The config this run was built with.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// True once the mission finishes or time runs out.
    pub fn is_over(&self) -> bool {
        self.over
    }

    /// Restore the seed state: integrator, battery, mission, wind stream, clock.
    pub fn reset(&mut self) {
        *self = Self::from_parts(self.seed, self.config.clone());
    }

    /// One physics tick. Returns the 9 actuator commands.
    ///
    /// After [`TbdGame::is_over`] this returns the last command and does not
    /// advance the clock, the battery or the wind stream.
    pub fn step(&mut self, obs: &[f64]) -> Result<[f64; CTRL_LEN], StepError> {
        if self.over {
            return Ok(self.last_ctrl);
        }
        if self.time_s >= self.config.max_time_s {
            self.over = true;
            return Ok(self.last_ctrl);
        }
        let obs = Obs::parse(obs)?;
        let cmd = self.mission.advance(&obs, self.config.dt, self.time_s);
        if cmd.reset_fc {
            self.fc.reset();
        }
        let mut ctrl = [0.0; CTRL_LEN];
        let mut thrusts = [0.0; 4];
        if let Some(set) = cmd.flight {
            let out = self.fc.step(self.config.dt, &plant_of(&obs), &set);
            if !cmd.zero_thrust {
                ctrl[0..4].copy_from_slice(&out.u);
            }
            thrusts = out.f;
            self.last_output = Some(out);
        } else {
            self.last_output = None;
        }
        mission::write_axes(&mut ctrl, &cmd);
        let ctrl = clip_ctrl(ctrl);
        self.wind.sample(self.config.dt);
        let driving = driving_flag(&ctrl);
        // On the descend → touchdown tick the script bills the controller thrust
        // even though the actuator command was zeroed.
        let zeros = [0.0; 4];
        let billed: &[f64] = if cmd.flight.is_some() {
            &thrusts
        } else {
            &zeros
        };
        self.battery.step(self.config.dt, billed, driving);
        self.mode = cmd.mode;
        self.phase = cmd.phase;
        self.last_ctrl = ctrl;
        self.last_pos = obs.pos;
        self.last_fold = obs.fold;
        self.time_s += self.config.dt;
        self.tick += 1;
        if cmd.completed || self.time_s >= self.config.max_time_s {
            self.over = true;
        }
        Ok(ctrl)
    }

    /// Snapshot for the HUD, the trace and `state_json`.
    pub fn state(&self) -> StateView {
        let scored = self.mission.score_input(self.last_pos[0]);
        let battery_pct = self.battery.pct();
        let score = match self.config.mission {
            MissionId::StairHop => stair_score(
                scored.landed,
                self.time_s,
                scored.max_tilt_deg,
                100.0 - battery_pct,
                scored.min_clearance_m,
                scored.progress_x,
            ),
            MissionId::Manual => 0.0,
        };
        StateView {
            tick: self.tick,
            time_s: self.time_s,
            seed: self.seed.to_string(),
            mode: self.mode.as_str(),
            mission: self.mission.name(),
            phase: self.phase,
            over: self.over,
            battery_pct,
            power_w: self.battery.last_w(),
            wh_used: self.battery.wh_used(),
            hover_minutes_est: self.battery.hover_minutes(),
            wind_world_mps: self.wind.velocity(),
            wind_force_n: self.wind.force(),
            score,
            timer_s: self.time_s,
            completed: scored.completed,
            landed_on_landing: scored.landed,
            max_tilt_deg: scored.max_tilt_deg,
            min_clearance_m: scored.min_clearance_m,
            battery_pct_used: 100.0 - battery_pct,
            pos: self.last_pos,
            fold_rad: self.last_fold,
            dt: self.config.dt,
            ctrl: self.last_ctrl,
        }
    }

    /// [`StateView`] as one JSON object.
    pub fn state_json(&self) -> String {
        serde_json::to_string(&self.state()).expect("state serializes")
    }

    /// Last cascade output, if the mission ran the controller on the last step.
    pub fn last_control(&self) -> Option<ControlOutput> {
        self.last_output
    }
}
