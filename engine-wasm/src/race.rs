//! Racing in the browser: [`RaceViewer`] (plain Rust, unit-tested natively) and its
//! `#[wasm_bindgen]` handle [`WasmRace`], shaped like [`crate::WasmMatch`] so a viewer
//! can reuse the same loop: `fromBuilds`, `step(n)`, `tick`, `isOver`, `stateJson`,
//! `outcomeJson`, `stateHash`, `setupJson`, plus `trackJson` (static geometry, draw
//! once) and `replayJson` (format 5). [`check_race_replay`] (JS `checkRaceReplayJson`)
//! verifies a racing replay, e.g. one written from Python by `engine-py`.
//!
//! Coordinates are the engine's: Y-up, origin bottom-left, world units. Angles are
//! radians counter-clockwise from +X, as in the tank state.

use crate::{errors_json, parse_seed, radians};
use engine::generic::{setup_hash, EndReason, Policy, Rules};
use racing::drivers::Behavior;
use racing::{Race, RaceReplay, RacingConfig, RacingRules};
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// One car in [`RaceStateView`].
#[derive(Serialize, Debug, PartialEq)]
pub struct CarView {
    /// Car index: the build's position in `fromBuilds` (also its team).
    pub id: usize,
    /// Centre, world units.
    pub pos: PointView,
    /// Heading, radians CCW from +X.
    pub heading: f32,
    /// Speed (length of the velocity), units per second.
    pub speed: f32,
    /// Laps completed (0 until the first full lap after the start crossing).
    pub lap: u32,
    /// Laps to finish.
    pub laps_total: u32,
    /// Index into `trackJson().gates` of the next gate to cross.
    pub next_gate: u32,
    /// Whether the car has crossed the start line yet.
    pub started: bool,
    /// Whether the car has finished. It then gets no throttle and coasts to a stop, as a
    /// ghost: it still hits walls, not cars.
    pub finished: bool,
    /// Tick it finished on, or null.
    pub finish_tick: Option<u32>,
    /// Current place, 1 = leading (final once finished).
    pub placing: u8,
    /// Grid slot it started from.
    pub slot: u8,
}

/// A point, world units.
#[derive(Serialize, Debug, PartialEq, Clone, Copy)]
pub struct PointView {
    /// x.
    pub x: f32,
    /// y.
    pub y: f32,
}

/// A race result in [`RaceStateView`] and `outcomeJson`.
#[derive(Serialize, Debug, PartialEq)]
pub struct RaceOutcomeView {
    /// Winning car (the first finisher), or null (nobody finished).
    pub winner: Option<u8>,
    /// Tick the race ended on.
    pub ticks: u32,
    /// `finished` or `tick_limit`.
    pub reason: EndReason,
    /// Final place per car, by car index (1 = winner).
    pub placings: Vec<u8>,
    /// Finish tick per car, by car index (null = did not finish).
    pub finish_ticks: Vec<Option<u32>>,
}

/// Everything the race viewer draws for one frame; serialized by `stateJson`.
#[derive(Serialize, Debug, PartialEq)]
pub struct RaceStateView {
    /// Ticks simulated so far.
    pub tick: u32,
    /// Tick cap.
    pub max_ticks: u32,
    /// True once the race has ended.
    pub over: bool,
    /// Cars in build order.
    pub cars: Vec<CarView>,
    /// Null while racing.
    pub outcome: Option<RaceOutcomeView>,
}

/// One gate in [`TrackView`]: the line from the inner wall to the outer wall.
#[derive(Serialize, Debug, PartialEq)]
pub struct GateView {
    /// Inner-wall endpoint `[x, y]`.
    pub inner: [f32; 2],
    /// Outer-wall endpoint `[x, y]`.
    pub outer: [f32; 2],
    /// True for gate 0, the start/finish line.
    pub start_finish: bool,
}

/// Static track geometry; serialized by `trackJson`. Draw it once.
#[derive(Serialize, Debug, PartialEq)]
pub struct TrackView {
    /// Track name (`Ring`).
    pub name: String,
    /// World bounds: x in `0..width`, y in `0..height`.
    pub width: f32,
    /// See `width`.
    pub height: f32,
    /// Centreline, a closed polyline (the last point joins the first; not repeated).
    pub centreline: Vec<[f32; 2]>,
    /// Half the road width.
    pub half_width: f32,
    /// Inner wall, a closed polyline (vertex `i` is gate `i`'s inner end).
    pub inner: Vec<[f32; 2]>,
    /// Outer wall, a closed polyline (vertex `i` is gate `i`'s outer end).
    pub outer: Vec<[f32; 2]>,
    /// Gates in race order; gate 0 is the start/finish line.
    pub gates: Vec<GateView>,
    /// Index of the start/finish gate (always 0).
    pub start_finish_gate: usize,
    /// Car radius, world units.
    pub car_radius: f32,
    /// Laps to finish.
    pub laps: u32,
    /// Tick cap.
    pub max_ticks: u32,
    /// Ticks per second (60), so race time = tick / tick_hz.
    pub tick_hz: u32,
    /// Seconds per tick (1/60).
    pub seconds_per_tick: f32,
}

/// One car's setup in [`RaceSetupView`].
#[derive(Serialize, Debug, PartialEq)]
pub struct CarSetupView {
    /// Behavior key (`follower`, `cutter`, `blocker`).
    pub behavior: &'static str,
    /// Display name (`Follower`).
    pub name: &'static str,
    /// How the behavior was made: `Scripted`.
    pub training: &'static str,
    /// Levels `power-top_speed-grip` (`3-3-3`).
    pub setup: String,
}

/// A race's setup; serialized by `setupJson`.
#[derive(Serialize, Debug, PartialEq)]
pub struct RaceSetupView {
    /// Seed as a decimal string.
    pub seed: String,
    /// Cars in build order.
    pub cars: Vec<CarSetupView>,
}

/// A race between scripted drivers, steppable from a render loop.
pub struct RaceViewer {
    m: Race,
    drivers: Vec<Box<dyn Policy<RacingRules>>>,
    /// One tick's actions, reused so a step allocates nothing.
    actions: Vec<racing::RaceAction>,
    setup: RaceSetupView,
}

impl RaceViewer {
    /// A Ring race from a JSON array of 1–4 racing builds: car `i` plays
    /// `builds[i]`, validated with [`racing::catalog::validate_build`]; each needs a
    /// scripted behavior. Drivers are seeded as in `racing::balance::run` and engine-py.
    pub fn from_builds(seed: &str, builds_json: &str) -> Result<Self, String> {
        let seed = parse_seed(seed)?;
        let builds: Vec<serde_json::Value> = serde_json::from_str(builds_json)
            .map_err(|e| format!("builds must be a JSON array of builds: {e}"))?;
        if builds.is_empty() || builds.len() > racing::MAX_CARS {
            return Err(format!(
                "a race takes 1 to {} builds, got {}",
                racing::MAX_CARS,
                builds.len()
            ));
        }
        let mut setups = Vec::with_capacity(builds.len());
        let mut behaviors = Vec::with_capacity(builds.len());
        for (i, b) in builds.iter().enumerate() {
            let valid = racing::catalog::validate_build(&b.to_string())
                .map_err(|e| format!("car {i}: invalid build {}", errors_json(&e)))?;
            let id = valid.scripted().ok_or_else(|| {
                format!("car {i}: a champion behavior needs its genome; the loader resolves it")
            })?;
            setups.push(racing::catalog::setup(&valid.build.levels));
            behaviors.push(Behavior::from_key(id).expect("validated scripted id"));
        }
        let config = RacingConfig::ring_setups(&setups);
        config.validate().map_err(|e| e.to_string())?;
        let drivers = behaviors
            .iter()
            .enumerate()
            .map(|(i, b)| b.driver(&config, i, seed))
            .collect();
        let setup = RaceSetupView {
            seed: seed.to_string(),
            cars: behaviors
                .iter()
                .zip(&setups)
                .map(|(b, s)| CarSetupView {
                    behavior: b.key(),
                    name: b.name(),
                    training: "Scripted",
                    setup: s.to_string(),
                })
                .collect(),
        };
        Ok(Self {
            actions: Vec::with_capacity(setups.len()),
            m: Race::new(config, seed),
            drivers,
            setup,
        })
    }

    /// Advance up to `n` ticks (stops early when the race ends). Returns true if over.
    /// Each tick is `Match::step_policies` (finished cars idle, their driver isn't
    /// asked), with no allocation.
    pub fn step(&mut self, n: u32) -> bool {
        for _ in 0..n {
            if self.m.is_over() {
                break;
            }
            self.actions.clear();
            for (i, d) in self.drivers.iter_mut().enumerate() {
                self.actions
                    .push(if RacingRules::is_active(self.m.state(), i) {
                        d.act(&self.m.observe(i))
                    } else {
                        Default::default()
                    });
            }
            self.m.step(&self.actions);
        }
        self.m.is_over()
    }

    /// The underlying match.
    pub fn inner(&self) -> &Race {
        &self.m
    }

    /// The setup.
    pub fn setup(&self) -> &RaceSetupView {
        &self.setup
    }

    /// The result, once the race has ended.
    pub fn outcome(&self) -> Option<RaceOutcomeView> {
        let cars = &self.m.state().cars;
        self.m.outcome().map(|o| RaceOutcomeView {
            winner: o.winner,
            ticks: o.ticks,
            reason: o.reason,
            placings: cars.iter().map(|c| c.place).collect(),
            finish_ticks: cars.iter().map(|c| c.finish_tick).collect(),
        })
    }

    /// Snapshot of the current state for drawing.
    pub fn state(&self) -> RaceStateView {
        let laps_total = self.m.config().laps;
        RaceStateView {
            tick: self.m.tick(),
            max_ticks: self.m.config().max_ticks,
            over: self.m.is_over(),
            cars: self
                .m
                .state()
                .cars
                .iter()
                .enumerate()
                .map(|(id, c)| CarView {
                    id,
                    pos: PointView {
                        x: c.pos.x,
                        y: c.pos.y,
                    },
                    heading: radians(c.heading),
                    speed: c.vel.length(),
                    lap: c.laps,
                    laps_total,
                    next_gate: c.next_gate,
                    started: c.started(),
                    finished: c.finished(),
                    finish_tick: c.finish_tick,
                    placing: c.place,
                    slot: c.slot,
                })
                .collect(),
            outcome: self.outcome(),
        }
    }

    /// The track geometry (see [`TrackView`]).
    pub fn track(&self) -> TrackView {
        let cfg = self.m.config();
        let g = &self.m.state().geom;
        let xy = |v: engine::Vec2| [v.x, v.y];
        TrackView {
            name: cfg.track.name.clone(),
            width: g.size.x,
            height: g.size.y,
            centreline: g.points.iter().map(|&p| xy(p)).collect(),
            half_width: g.half_width,
            inner: g.gates.iter().map(|gt| xy(gt.inner)).collect(),
            outer: g.gates.iter().map(|gt| xy(gt.outer)).collect(),
            gates: g
                .gates
                .iter()
                .enumerate()
                .map(|(i, gt)| GateView {
                    inner: xy(gt.inner),
                    outer: xy(gt.outer),
                    start_finish: i == 0,
                })
                .collect(),
            start_finish_gate: 0,
            car_radius: cfg.physics.radius,
            laps: cfg.laps,
            max_ticks: cfg.max_ticks,
            tick_hz: 60,
            seconds_per_tick: 1.0 / 60.0,
        }
    }
}

/// JS handle: `WasmRace.fromBuilds("42", JSON.stringify([build, build]))`.
#[wasm_bindgen]
pub struct WasmRace(RaceViewer);

#[wasm_bindgen]
impl WasmRace {
    /// A Ring race: `seed` as a decimal string, `builds_json` a JSON array of 1–4
    /// racing builds (`{rules_version, levels, behavior}`), car `i` = `builds[i]`.
    /// Each is checked with the same validator as `validateBuild("racing", ...)`; an
    /// invalid build (`car 1: invalid build [{"code","key"}]`) or a champion behavior
    /// throws.
    #[wasm_bindgen(js_name = fromBuilds)]
    pub fn from_builds(seed: &str, builds_json: &str) -> Result<WasmRace, JsError> {
        RaceViewer::from_builds(seed, builds_json)
            .map(WasmRace)
            .map_err(|e| JsError::new(&e))
    }

    /// Setup as JSON (see `RaceSetupView`).
    #[wasm_bindgen(js_name = setupJson)]
    pub fn setup_json(&self) -> String {
        serde_json::to_string(self.0.setup()).expect("setup serializes")
    }

    /// Advance up to `n` ticks; returns true once the race is over.
    pub fn step(&mut self, n: u32) -> bool {
        self.0.step(n)
    }

    /// Ticks simulated so far.
    pub fn tick(&self) -> u32 {
        self.0.inner().tick()
    }

    /// True once the race has ended.
    #[wasm_bindgen(js_name = isOver)]
    pub fn is_over(&self) -> bool {
        self.0.inner().is_over()
    }

    /// Per-frame state as JSON (see `RaceStateView`).
    #[wasm_bindgen(js_name = stateJson)]
    pub fn state_json(&self) -> String {
        serde_json::to_string(&self.0.state()).expect("state serializes")
    }

    /// Static track geometry as JSON (see `TrackView`); draw once.
    #[wasm_bindgen(js_name = trackJson)]
    pub fn track_json(&self) -> String {
        serde_json::to_string(&self.0.track()).expect("track serializes")
    }

    /// Outcome as JSON, or `"null"` while racing.
    #[wasm_bindgen(js_name = outcomeJson)]
    pub fn outcome_json(&self) -> String {
        serde_json::to_string(&self.0.outcome()).expect("outcome serializes")
    }

    /// Current state hash, 16 hex digits (the replay's `final_hash` at the end).
    #[wasm_bindgen(js_name = stateHash)]
    pub fn state_hash(&self) -> String {
        format!("{:016x}", self.0.inner().state_hash())
    }

    /// The race so far as replay JSON (format 5, `game: "racing"`).
    #[wasm_bindgen(js_name = replayJson)]
    pub fn replay_json(&self) -> String {
        self.0.inner().replay().to_json()
    }
}

/// What re-simulating a racing replay gives (like [`crate::ReplayCheck`] for tanks).
/// Every value except `format` and `seed` is recomputed.
#[derive(Serialize, Debug, PartialEq)]
pub struct RaceReplayCheck {
    /// The file's `format` (5).
    pub format: u32,
    /// `racing`.
    pub game: &'static str,
    /// The file's seed, decimal string.
    #[serde(with = "engine::json_u64")]
    pub seed: u64,
    /// Cars in the config.
    pub cars: usize,
    /// Ticks re-simulated.
    pub ticks: u32,
    /// Outcome after re-simulating, or null if the actions stop before the end.
    pub outcome: Option<RaceOutcomeView>,
    /// State hash after re-simulating.
    pub final_hash: String,
    /// `setup_hash` recomputed from the seed and config.
    pub setup_hash: String,
    /// `null` if `Replay::verify` passes, else its error.
    pub verify_error: Option<String>,
}

/// Load a racing replay and re-simulate it. Errors only if it doesn't load.
pub fn check_race_replay(json: &str) -> Result<RaceReplayCheck, String> {
    let r = RaceReplay::from_json(json).map_err(|e| e.to_string())?;
    let (m, verify_error) = match r.verify() {
        Ok(m) => (m, None),
        Err(e) => (r.play(), Some(e.to_string())),
    };
    let cars = &m.state().cars;
    Ok(RaceReplayCheck {
        format: r.format,
        game: RacingRules::GAME,
        seed: r.seed,
        cars: cars.len(),
        ticks: m.tick(),
        outcome: m.outcome().map(|o| RaceOutcomeView {
            winner: o.winner,
            ticks: o.ticks,
            reason: o.reason,
            placings: cars.iter().map(|c| c.place).collect(),
            finish_ticks: cars.iter().map(|c| c.finish_tick).collect(),
        }),
        final_hash: format!("{:016x}", m.state_hash()),
        setup_hash: format!("{:016x}", setup_hash(r.seed, &r.config)),
        verify_error,
    })
}

/// Re-simulate a racing replay's JSON and return a JSON `RaceReplayCheck`. Throws if
/// it doesn't load as a racing replay (a tank replay names its game).
#[wasm_bindgen(js_name = checkRaceReplayJson)]
pub fn check_race_replay_json(json: &str) -> Result<String, JsError> {
    check_race_replay(json)
        .map(|c| serde_json::to_string(&c).expect("check serializes"))
        .map_err(|e| JsError::new(&e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use racing::catalog::{setup, validate_build};

    fn build(levels: (u8, u8, u8), id: &str) -> String {
        format!(
            r#"{{"rules_version":1,"levels":{{"power":{},"top_speed":{},"grip":{}}},"behavior":{{"kind":"scripted","id":"{id}"}}}}"#,
            levels.0, levels.1, levels.2
        )
    }

    fn four() -> String {
        format!(
            "[{},{},{},{}]",
            build((3, 3, 3), "follower"),
            build((5, 2, 2), "cutter"),
            build((2, 2, 5), "blocker"),
            build((2, 5, 2), "cutter")
        )
    }

    /// The same race as `racing::balance::run` on the same config and drivers.
    #[test]
    fn race_matches_the_racing_crate() {
        let builds: Vec<serde_json::Value> = serde_json::from_str(&four()).unwrap();
        let valid: Vec<_> = builds
            .iter()
            .map(|b| validate_build(&b.to_string()).unwrap())
            .collect();
        let setups: Vec<_> = valid.iter().map(|v| setup(&v.build.levels)).collect();
        let config = RacingConfig::ring_setups(&setups);
        let behaviors: Vec<_> = valid
            .iter()
            .map(|v| Behavior::from_key(v.scripted().unwrap()).unwrap())
            .collect();
        for seed in [0u64, 7, 42] {
            let mut v = RaceViewer::from_builds(&seed.to_string(), &four()).unwrap();
            while !v.step(13) {}
            let r = racing::balance::run(&config, &behaviors, seed);
            let o = v.outcome().unwrap();
            assert_eq!(o.ticks, r.ticks);
            assert_eq!(o.winner.map(usize::from), r.winner);
            assert_eq!(o.placings, r.places);
            assert_eq!(o.finish_ticks, r.finish);
            let mut m = Race::new(config.clone(), seed);
            let mut ps: Vec<_> = behaviors
                .iter()
                .enumerate()
                .map(|(i, b)| b.driver(&config, i, seed))
                .collect();
            let mut refs: Vec<&mut dyn Policy<RacingRules>> =
                ps.iter_mut().map(|p| p.as_mut() as _).collect();
            m.run(&mut refs);
            assert_eq!(v.inner().state_hash(), m.state_hash());
        }
    }

    /// engine-py's native fixture (`engine-py/tests/fixtures/determinism.json`, the
    /// numbers the Python runs are pinned to) for its all-scripted race.
    #[test]
    fn all_scripted_race_matches_the_engine_py_fixture() {
        let f: serde_json::Value = serde_json::from_str(include_str!(
            "../../engine-py/tests/fixtures/determinism.json"
        ))
        .unwrap();
        let c = f["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "racing-all-scripted")
            .unwrap();
        let mut v =
            RaceViewer::from_builds(&c["seed"].to_string(), &c["builds"].to_string()).unwrap();
        while !v.step(64) {}
        assert_eq!(
            format!("{:016x}", v.inner().state_hash()),
            c["final_hash"].as_str().unwrap()
        );
        assert_eq!(v.inner().tick() as u64, c["ticks"].as_u64().unwrap());
        let o = serde_json::to_value(v.outcome().unwrap()).unwrap();
        for k in ["winner", "ticks", "reason"] {
            assert_eq!(o[k], c["outcome"][k], "{k}");
        }
    }

    #[test]
    fn state_track_and_stepping() {
        let mut v = RaceViewer::from_builds("42", &four()).unwrap();
        let s = v.state();
        assert_eq!((s.tick, s.over, s.cars.len()), (0, false, 4));
        assert!(s.outcome.is_none());
        assert_eq!(
            s.cars.iter().map(|c| c.id).collect::<Vec<_>>(),
            [0, 1, 2, 3]
        );
        let t = v.track();
        assert_eq!(t.name, "Ring");
        assert_eq!(t.gates.len(), t.centreline.len());
        assert_eq!(
            (t.inner.len(), t.outer.len()),
            (t.gates.len(), t.gates.len())
        );
        assert!(t.gates[0].start_finish && t.gates[1..].iter().all(|g| !g.start_finish));
        assert!(t.half_width > 0.0 && t.width > 0.0 && t.height > 0.0 && t.tick_hz == 60);
        for p in t.centreline.iter().chain(&t.inner).chain(&t.outer) {
            assert!((0.0..=t.width).contains(&p[0]) && (0.0..=t.height).contains(&p[1]));
        }
        assert!(!v.step(100));
        assert_eq!(v.inner().tick(), 100);
        assert!(v.state().cars.iter().any(|c| c.speed > 0.0));
        while !v.step(1000) {}
        let end = v.inner().tick();
        assert!(v.step(5));
        assert_eq!(v.inner().tick(), end, "no ticks past the end");
        let s = v.state();
        assert!(s.over && s.outcome.is_some());
        let o = s.outcome.unwrap();
        let mut placings = o.placings.clone();
        placings.sort();
        assert_eq!(placings, [1, 2, 3, 4]);
        // Same seed and builds, same race.
        let mut w = RaceViewer::from_builds("42", &four()).unwrap();
        while !w.step(1) {}
        assert_eq!(w.inner().state_hash(), v.inner().state_hash());
    }

    #[test]
    fn rejects_bad_input() {
        let one = build((3, 3, 3), "follower");
        let err = |b: &str| RaceViewer::from_builds("1", b).err().unwrap();
        assert!(RaceViewer::from_builds("1", &format!("[{one}]")).is_ok());
        assert!(err("[]").starts_with("a race takes 1 to 4 builds"));
        assert!(err(&format!("[{one},{one},{one},{one},{one}]")).contains("got 5"));
        assert!(err("{}").starts_with("builds must be a JSON array"));
        assert_eq!(
            err(&format!("[{one},{}]", build((5, 5, 5), "follower"))),
            r#"car 1: invalid build [{"code":"over_budget","key":"levels"}]"#
        );
        let champ = r#"[{"rules_version":1,"levels":{"power":3,"top_speed":3,"grip":3},"behavior":{"kind":"champion","ref":"racing/nightly/gen-1"}}]"#;
        assert!(err(champ).starts_with("car 0: a champion"));
        let tank = r#"[{"rules_version":1,"levels":{"attack":3,"speed":3,"defense":3},"behavior":{"kind":"scripted","id":"charger"}}]"#;
        assert!(err(tank).starts_with("car 0: invalid build"));
        assert!(RaceViewer::from_builds("-1", &format!("[{one}]")).is_err());
    }

    #[test]
    fn replay_round_trips_and_checks() {
        let mut v = RaceViewer::from_builds("9", &four()).unwrap();
        while !v.step(64) {}
        let json = v.inner().replay().to_json();
        let r: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            (r["format"].as_u64(), r["game"].as_str()),
            (Some(5), Some("racing"))
        );
        let c = check_race_replay(&json).unwrap();
        assert_eq!(c.verify_error, None);
        assert_eq!(c.final_hash, format!("{:016x}", v.inner().state_hash()));
        assert_eq!(c.outcome, v.outcome());
        assert_eq!((c.cars, c.ticks, c.seed), (4, v.inner().tick(), 9));
        // A tampered hash still loads, re-simulates, and reports the mismatch.
        let bad = json.replace(&c.final_hash, "0000000000000000");
        let b = check_race_replay(&bad).unwrap();
        assert!(b.verify_error.is_some());
        assert_eq!(b.final_hash, c.final_hash);
        // A tank replay is not a racing replay.
        let mut t = crate::Viewer::new("1", "Chaser", "Wanderer").unwrap();
        while !t.step(500) {}
        assert!(check_race_replay(&t.inner().replay().to_json()).is_err());
    }
}
