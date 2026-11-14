//! TBD in the browser. [`WasmTbd`] is a thin `#[wasm_bindgen]` handle on
//! [`tbd::TbdGame`], shaped so the page can call it once per MuJoCo tick.
//!
//! TBD is a physics sim with estimated parameters. The working name is TBD.
//! This module does not run physics: the page owns MuJoCo (`@mujoco/mujoco`
//! 4.0.1) and three.js. One `step` reads a flat observation and returns the
//! nine actuator commands. `stateJson` carries the wind force the page writes
//! into `xfrc_applied` before `mj_step`.

use tbd::{TbdGame, CTRL_LEN, OBS_LEN};
use wasm_bindgen::prelude::*;

/// JS handle: `new WasmTbd(seed, configJson)`.
///
/// `seed` is a BigInt (`1n`). wasm-bindgen 0.2.100 passes `u64` as a bigint,
/// not a Number. `configJson` is a [`tbd::Config`] object. `""` and `"{}"`
/// are the defaults: 0.002 s, mission `stair_hop`, wind on, 16 s horizon.
#[wasm_bindgen]
pub struct WasmTbd {
    inner: TbdGame,
}

#[wasm_bindgen]
impl WasmTbd {
    /// Build a run. Throws if `configJson` is not an object this crate accepts,
    /// or if `dt` / `max_time_s` are not finite and positive.
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u64, config_json: &str) -> Result<WasmTbd, JsError> {
        TbdGame::from_json(seed, config_json)
            .map(|inner| WasmTbd { inner })
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// One physics tick.
    ///
    /// `obs` is [`OBS_LEN`] floats in `tbd.xml` sensor order, then free-joint
    /// position and linear velocity, then `pilot_x` and `pilot_yaw`. Returns
    /// [`CTRL_LEN`] actuator commands in `<actuator>` order. Throws if the
    /// length is wrong or any value is not finite. After the mission is over,
    /// returns the last command and does not advance.
    pub fn step(&mut self, obs: &[f64]) -> Result<Vec<f64>, JsError> {
        self.inner
            .step(obs)
            .map(|c| c.to_vec())
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// [`tbd::StateView`] as JSON: mode, phase, battery, score,
    /// `wind_force_n`, `aero_force_n` and `aero_torque_nm` (world frame; add the
    /// forces, and use the torque, in `xfrc_applied` on body `tbd`).
    #[wasm_bindgen(js_name = stateJson)]
    pub fn state_json(&self) -> String {
        self.inner.state_json()
    }

    /// Same seed and config, back to tick 0.
    pub fn reset(&mut self) {
        self.inner.reset();
    }
}

/// Observation length (20).
#[wasm_bindgen(js_name = tbdObsLen)]
pub fn tbd_obs_len() -> u32 {
    OBS_LEN as u32
}

/// Actuator-command length (9).
#[wasm_bindgen(js_name = tbdCtrlLen)]
pub fn tbd_ctrl_len() -> u32 {
    CTRL_LEN as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use tbd::params::FOLD_STOW;

    fn hover_obs() -> Vec<f64> {
        let mut obs = vec![0.0; OBS_LEN];
        obs[6] = 1.0;
        obs[11] = FOLD_STOW;
        obs[14] = 0.2;
        obs
    }

    #[test]
    fn step_returns_nine_commands_and_json_state() {
        let mut g = WasmTbd::new(1, "{}").unwrap();
        assert_eq!(tbd_obs_len() as usize, OBS_LEN);
        assert_eq!(tbd_ctrl_len() as usize, CTRL_LEN);
        let ctrl = g.step(&hover_obs()).unwrap();
        assert_eq!(ctrl.len(), CTRL_LEN);
        assert_eq!(ctrl[4], 24.0);
        let state: serde_json::Value = serde_json::from_str(&g.state_json()).unwrap();
        assert_eq!(state["mission"], "stair_hop");
        assert_eq!(state["mode"], "ground");
        assert_eq!(state["phase"], "drive");
        assert_eq!(state["wind_force_n"].as_array().unwrap().len(), 3);
        assert_eq!(state["tick"], 1);
        g.reset();
        let again: serde_json::Value = serde_json::from_str(&g.state_json()).unwrap();
        assert_eq!(again["tick"], 0);
        // Error paths are checked on `TbdGame`. `JsError` aborts outside a
        // JS runtime, so this native test stays on the success path of the wrapper.
        assert!(tbd::TbdGame::from_json(1, r#"{"dt":-1}"#).is_err());
        assert!(g.inner.step(&[0.0; 3]).is_err());
    }

    #[test]
    fn manual_config_selects_the_stick_mission() {
        let g = WasmTbd::new(3, r#"{"mission":"manual","wind":false}"#).unwrap();
        let state: serde_json::Value = serde_json::from_str(&g.state_json()).unwrap();
        assert_eq!(state["mission"], "manual");
        assert_eq!(state["phase"], "ground");
        assert_eq!(state["wind_force_n"], serde_json::json!([0.0, 0.0, 0.0]));
    }
}
