//! Headless TBD runner.
//!
//! TBD is a physics sim with estimated parameters. The working name is TBD.
//! This binary loads `assets/tbd/tbd.xml`, steps MuJoCo, and asks `games/tbd`
//! for actuator commands once per tick. It is behind the `native` cargo
//! feature: a default `cargo test --workspace` does not compile or link MuJoCo.
//!
//! mujoco-rs 7.1 links MuJoCo 3.17. The model file and the browser build are
//! MuJoCo 4.0.1. Traces are tolerance-level, not bit-exact, across those builds.
//! See `tbd-native/README.md`.

use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;

/// Shown when this binary was built without `--features native`.
pub fn feature_missing_message() -> &'static str {
    "tbd-native was built without the `native` feature, so it cannot load MuJoCo. \
Rebuild with `cargo run -p tbd-native --features native -- --seed 1 --out trace.json` \
after setting MUJOCO_DOWNLOAD_DIR (mujoco-rs 7.1 downloads MuJoCo 3.17). \
See tbd-native/README.md. The model file and the browser path are MuJoCo 4.0.1. \
This is a physics sim with estimated parameters."
}

#[derive(Parser, Debug)]
#[command(
    name = "tbd-native",
    version,
    about = "TBD headless stair-hop (physics sim with estimated parameters). Needs --features native."
)]
struct Args {
    /// Wind seed. The same seed and model reproduce the trace.
    #[arg(long, default_value_t = 1)]
    seed: u64,
    /// `stair_hop` (default) or `manual` (holds the ground; no stick).
    #[arg(long, default_value = "stair_hop")]
    scenario: String,
    /// Disable the seeded wind disturbance.
    #[arg(long)]
    no_wind: bool,
    /// Stop at this many seconds. Default 16, the stair-hop horizon.
    #[arg(long)]
    max_time: Option<f64>,
    /// Path to `tbd.xml`. Default searches the repo and `$TBD_XML`.
    #[arg(long)]
    xml: Option<PathBuf>,
    /// Write the trace JSON here. The summary is always printed to stdout.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Record every physics tick. The default keeps 50 Hz plus phase changes.
    #[arg(long)]
    full: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();
    match execute(&args) {
        Ok(summary) => {
            println!("{summary}");
            ExitCode::SUCCESS
        }
        #[cfg(not(feature = "native"))]
        Err(RunError::MissingFeature) => {
            eprintln!("tbd-native: {}", feature_missing_message());
            ExitCode::from(2)
        }
        #[cfg(feature = "native")]
        Err(RunError::Run(e)) => {
            eprintln!("tbd-native: {e}");
            ExitCode::from(1)
        }
    }
}

#[derive(Debug)]
enum RunError {
    #[cfg(not(feature = "native"))]
    MissingFeature,
    #[cfg(feature = "native")]
    Run(String),
}

fn execute(args: &Args) -> Result<String, RunError> {
    #[cfg(feature = "native")]
    {
        native::run(args).map_err(RunError::Run)
    }
    #[cfg(not(feature = "native"))]
    {
        let _ = args;
        Err(RunError::MissingFeature)
    }
}

#[cfg(feature = "native")]
mod native {
    use super::Args;
    use std::path::{Path, PathBuf};
    use tbd::{Config, MissionId, TbdGame, CTRL_LEN, SENSOR_LEN};

    pub fn run(args: &Args) -> Result<String, String> {
        let mission = match args.scenario.as_str() {
            "stair_hop" => MissionId::StairHop,
            "manual" => MissionId::Manual,
            other => return Err(format!("unknown scenario {other:?} (stair_hop or manual)")),
        };
        let xml = find_xml(args.xml.as_deref())?;
        let model = mujoco_rs::prelude::MjModel::from_xml(&xml)
            .map_err(|e| format!("load {}: {e}", xml.display()))?;
        let mut data = mujoco_rs::prelude::MjData::new(&model);
        let dt = model.opt().timestep;
        if !(dt.is_finite() && dt > 0.0) {
            return Err(format!("model timestep {dt} is not usable"));
        }
        let max_time = args.max_time.unwrap_or(16.0);
        if !(max_time.is_finite() && max_time > 0.0) {
            return Err("max-time must be a finite number greater than 0".into());
        }
        let key = model
            .key("ground")
            .ok_or("tbd.xml has no keyframe named ground")?;
        data.reset_keyframe(key.id)
            .map_err(|e| format!("reset keyframe ground: {e}"))?;
        data.forward();

        let root = data
            .joint("root")
            .ok_or("tbd.xml has no free joint named root")?;
        let body = data.body("tbd").ok_or("tbd.xml has no body named tbd")?;
        let body_id = body.id;
        if data.ctrl().len() != CTRL_LEN {
            return Err(format!(
                "model nu is {}, games/tbd commands {CTRL_LEN}",
                data.ctrl().len()
            ));
        }
        if data.sensordata().len() != SENSOR_LEN {
            return Err(format!(
                "model nsensordata is {}, games/tbd reads {SENSOR_LEN}",
                data.sensordata().len()
            ));
        }

        let mut game = TbdGame::new(
            args.seed,
            Config {
                dt,
                mission,
                wind: !args.no_wind,
                max_time_s: max_time,
                ..Config::default()
            },
        )
        .map_err(|e| e.to_string())?;

        let mut samples = Vec::new();
        let mut last_phase = String::new();
        let stride = if args.full { 1 } else { 10 };
        let mut guard = 0u32;
        let guard_max = ((max_time / dt) as u32).saturating_add(2);

        while !game.is_over() {
            guard += 1;
            if guard > guard_max {
                return Err("stepped past max-time without the mission stopping".into());
            }
            let obs = read_obs(&data, &root)?;
            let ctrl = game.step(&obs).map_err(|e| e.to_string())?;
            data.ctrl_mut().copy_from_slice(&ctrl);
            let state = game.state();
            let force = state.wind_force_n;
            let aero = state.aero_force_n;
            let torque = state.aero_torque_nm;
            data.xfrc_applied_mut()[body_id] = [
                force[0] + aero[0],
                force[1] + aero[1],
                force[2] + aero[2],
                torque[0],
                torque[1],
                torque[2],
            ];
            let state = game.state();
            let phase_changed = state.phase != last_phase;
            if phase_changed || state.tick % stride == 0 || state.over {
                samples.push(serde_json::json!({
                    "t": state.time_s,
                    "tick": state.tick,
                    "mode": state.mode,
                    "phase": state.phase,
                    "x": state.pos[0],
                    "y": state.pos[1],
                    "z": state.pos[2],
                    "fold": state.fold_rad,
                    "battery_pct": state.battery_pct,
                    "score": state.score,
                    "wind_force_n": state.wind_force_n,
                    "ctrl": state.ctrl,
                }));
                last_phase = state.phase.to_string();
            }
            data.step();
        }

        let state = game.state();
        let summary = serde_json::json!({
            "name": "TBD",
            "note": "physics sim with estimated parameters",
            "model": xml.display().to_string(),
            "mujoco_rs": "7.1.0",
            "linked_mujoco": "3.17.0",
            "model_authored_for": "4.0.1",
            "seed": state.seed,
            "scenario": state.mission,
            "wind": !args.no_wind,
            "dt": dt,
            "tick": state.tick,
            "time_s": state.time_s,
            "phase": state.phase,
            "mode": state.mode,
            "completed": state.completed,
            "landed_on_landing": state.landed_on_landing,
            "score": state.score,
            "battery_pct": state.battery_pct,
            "max_tilt_deg": state.max_tilt_deg,
            "min_clearance_m": state.min_clearance_m,
            "final_pos": state.pos,
        });
        if let Some(path) = &args.out {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("create {}: {e}", parent.display()))?;
                }
            }
            let doc = serde_json::json!({
                "summary": summary,
                "samples": samples,
            });
            std::fs::write(
                path,
                serde_json::to_vec_pretty(&doc).expect("trace serializes"),
            )
            .map_err(|e| format!("write {}: {e}", path.display()))?;
        }
        Ok(summary.to_string())
    }

    fn read_obs<M: mujoco_rs::wrappers::mj_model::traits::ModelType>(
        data: &mujoco_rs::prelude::MjData<M>,
        root: &mujoco_rs::prelude::MjJointDataInfo,
    ) -> Result<[f64; tbd::OBS_LEN], String> {
        let pos = {
            let qpos = root.view(data).qpos;
            [qpos[0], qpos[1], qpos[2]]
        };
        let vel = {
            let qvel = root.view(data).qvel;
            [qvel[0], qvel[1], qvel[2]]
        };
        let sensor = data.sensordata().to_vec();
        tbd::pack_observation(&sensor, pos, vel, 0.0, 0.0).map_err(|e| e.to_string())
    }

    fn find_xml(explicit: Option<&Path>) -> Result<PathBuf, String> {
        if let Some(path) = explicit {
            return if path.is_file() {
                Ok(path.to_path_buf())
            } else {
                Err(format!("xml not found: {}", path.display()))
            };
        }
        if let Ok(env) = std::env::var("TBD_XML") {
            let path = PathBuf::from(&env);
            return if path.is_file() {
                Ok(path)
            } else {
                Err(format!("TBD_XML does not exist: {env}"))
            };
        }
        let candidates = [
            PathBuf::from("assets/tbd/tbd.xml"),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/tbd/tbd.xml"),
        ];
        for path in &candidates {
            if path.is_file() {
                return Ok(path.clone());
            }
        }
        Err("could not find assets/tbd/tbd.xml; pass --xml or set TBD_XML".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_feature_message_names_the_rebuild() {
        let msg = feature_missing_message();
        assert!(msg.contains("native"));
        assert!(msg.contains("MuJoCo"));
        assert!(msg.contains("tbd-native/README.md"));
        assert!(msg.contains("physics sim with estimated parameters"));
    }

    #[test]
    #[cfg(not(feature = "native"))]
    fn default_build_does_not_run_mujoco() {
        let args = Args {
            seed: 1,
            scenario: "stair_hop".into(),
            no_wind: false,
            max_time: None,
            xml: None,
            out: None,
            full: false,
        };
        assert!(matches!(execute(&args), Err(RunError::MissingFeature)));
    }

    #[test]
    #[cfg(feature = "native")]
    fn short_stair_hop_is_deterministic() {
        let dir = std::env::temp_dir().join(format!("tbd-native-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out_a = dir.join("a.json");
        let out_b = dir.join("b.json");
        let args = |out: PathBuf| Args {
            seed: 11,
            scenario: "stair_hop".into(),
            no_wind: true,
            max_time: Some(0.2),
            xml: None,
            out: Some(out),
            full: false,
        };
        let a = execute(&args(out_a.clone())).expect("run a");
        let b = execute(&args(out_b.clone())).expect("run b");
        assert_eq!(a, b);
        let doc: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&out_a).unwrap()).unwrap();
        assert_eq!(
            doc["summary"]["note"],
            "physics sim with estimated parameters"
        );
        assert!(doc["samples"].as_array().unwrap().len() > 1);
        assert_eq!(doc["summary"]["scenario"], "stair_hop");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
