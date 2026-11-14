//! TBD, a physics sim with estimated parameters.
//!
//! TBD is the working name. The MuJoCo model (`assets/tbd/tbd.xml`) is the
//! physics. This crate is the game logic that sits beside it: the cascade
//! flight controller, the ground → unfolding → flight → landing → folding
//! mode machine, the battery model, seeded wind, and the stair-hop mission.
//! It does not link MuJoCo. The browser calls [`TbdGame::step`] once per
//! physics tick through `WasmTbd`. A native runner can do the same with
//! `mujoco-rs` behind a feature that the default build does not turn on.
//!
//! Numbers come from `assets/tbd/PARAMS.md` and the reference controller in
//! `assets/tbd/reference/`. Nothing in them was measured on hardware.
//!
//! # Flat arrays
//!
//! One tick is [`OBS_LEN`] floats in and [`CTRL_LEN`] floats out. The layout
//! matches `tbd.xml` sensor order, then the free-joint position and linear
//! velocity, then two pilot channels. Commands match `<actuator>` order.
//! Field names live in [`OBS_NAMES`] and [`CTRL_NAMES`]. The full table is on
//! the [`obs`] module.
//!
//! # Reproducibility
//!
//! Wind uses `ChaCha8Rng::seed_from_u64`, the same generator and seeding as
//! the engine's `MatchRng`. Each enabled tick draws four `u64`s, independent
//! of the observation values. The same seed, config and observation log
//! reproduce the commands, the battery, the mission and `wind_force_n`.
//! After the mission ends, [`TbdGame::step`] returns the last command and
//! does not draw again.
//!
//! # Host loop
//!
//! The host reads MuJoCo, calls [`TbdGame::step`], writes the returned
//! command into `ctrl`, writes [`StateView::wind_force_n`] into
//! `xfrc_applied` on body `tbd`, then steps physics. `dt` in the config must
//! be the model timestep (0.002 s unless the host changed it).

#![warn(missing_docs)]

pub mod battery;
pub mod control;
pub mod math;
pub mod mission;
pub mod obs;
pub mod params;
mod wind;

pub use battery::Battery;
pub use control::{mix, ControlOutput, FlightController, Plant, Setpoint};
pub use game::{Config, ConfigError, MissionId, StateView, TbdGame};
pub use math::{quat_to_rpy, wrap};
pub use mission::{stair_score, FlightMode, CLIMB_Z, LAND_X0, STAIR_TOP, XLAND};
pub use obs::{
    pack_observation, Obs, StepError, CTRL_LEN, CTRL_NAMES, OBS_LEN, OBS_NAMES, PILOT_FOLD,
    PILOT_LAND, PILOT_TAKEOFF, SENSOR_LEN, SENSOR_SPEC,
};

mod game;
