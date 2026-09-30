//! Browser bindings for the Coastal Agentics engine: create a duel from a seed and a bot
//! pairing, step it, and read the state as JSON. Used by `web/arena.html`.
//!
//! The sim logic lives in [`Viewer`] (plain Rust, unit-tested natively); the
//! `#[wasm_bindgen]` [`WasmMatch`] wrapper only converts errors to JS.
//!
//! Coordinates are the engine's: Y-up, origin bottom-left. Angles in the JSON are
//! radians (counter-clockwise from +X), converted from integer headings for display
//! only; the sim itself never sees them.

use engine::bots::{Chaser, Wanderer};
use engine::{Action, EndReason, Heading, Match, MatchConfig, Policy, Vec2};
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Built-in bots the viewer can pit against each other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BotKind {
    /// [`engine::bots::Chaser`].
    Chaser,
    /// [`engine::bots::Wanderer`], seeded from the match seed (see [`Viewer::new`]).
    Wanderer,
}

impl BotKind {
    /// Parse a bot name (case-insensitive): `chaser` or `wanderer`.
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "chaser" => Ok(Self::Chaser),
            "wanderer" => Ok(Self::Wanderer),
            other => Err(format!(
                "unknown bot {other:?} (expected Chaser or Wanderer)"
            )),
        }
    }

    /// Build the policy for tank `team`. A Wanderer on team 1 is seeded with
    /// `seed ^ 0x5eed` so that Chaser vs Wanderer reproduces `engine-cli --seed <seed>`
    /// exactly; on team 0 it uses `seed ^ 0x5eed_0000`, so mirrored Wanderers differ.
    fn build(self, seed: u64, team: u8) -> Box<dyn Policy> {
        match self {
            Self::Chaser => Box::new(Chaser),
            Self::Wanderer => {
                let salt = if team == 1 { 0x5eed } else { 0x5eed_0000 };
                Box::new(Wanderer::new(seed ^ salt))
            }
        }
    }
}

/// One tank in [`StateView`].
#[derive(Serialize, Debug, PartialEq)]
pub struct TankView {
    /// Tank id.
    pub id: usize,
    /// Team id (0 = blue, 1 = orange in the viewer).
    pub team: u8,
    /// Centre x (engine units, Y-up frame).
    pub x: f32,
    /// Centre y (engine units, Y-up frame).
    pub y: f32,
    /// Hull heading, radians CCW from +X.
    pub heading: f32,
    /// Turret angle (world frame), radians CCW from +X.
    pub turret: f32,
    /// Current hit points.
    pub hp: i32,
    /// `TankParams::max_hp`.
    pub max_hp: i32,
    /// False once destroyed.
    pub alive: bool,
}

/// One projectile in [`StateView`].
#[derive(Serialize, Debug, PartialEq)]
pub struct ProjectileView {
    /// Position x.
    pub x: f32,
    /// Position y.
    pub y: f32,
    /// Velocity x, units per tick.
    pub vx: f32,
    /// Velocity y, units per tick.
    pub vy: f32,
    /// Team of the tank that fired it.
    pub team: u8,
}

/// An obstacle in [`StateView`]: `(x, y)` is its min (bottom-left) corner.
#[derive(Serialize, Debug, PartialEq)]
pub struct RectView {
    /// Min corner x.
    pub x: f32,
    /// Min corner y.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

/// Match result in [`StateView`] (same fields as `engine::Outcome`).
#[derive(Serialize, Debug, PartialEq)]
pub struct OutcomeView {
    /// Winning team, or null for a draw.
    pub winner: Option<u8>,
    /// Tick the match ended on.
    pub ticks: u32,
    /// `last_standing`, `all_destroyed` or `tick_limit`.
    pub reason: EndReason,
}

/// Everything the viewer draws for one frame; serialized by [`WasmMatch::state_json`].
#[derive(Serialize, Debug, PartialEq)]
pub struct StateView {
    /// Ticks simulated so far.
    pub tick: u32,
    /// Tick limit (`MatchConfig::max_ticks`).
    pub max_ticks: u32,
    /// Arena width.
    pub width: f32,
    /// Arena height.
    pub height: f32,
    /// Arena obstacles.
    pub obstacles: Vec<RectView>,
    /// All tanks, living and dead.
    pub tanks: Vec<TankView>,
    /// Projectiles in flight.
    pub projectiles: Vec<ProjectileView>,
    /// Null while the match is running.
    pub outcome: Option<OutcomeView>,
}

fn radians(h: Heading) -> f32 {
    h as f32 * (std::f32::consts::TAU / 65536.0)
}

/// A duel between two built-in bots, steppable from a render loop.
pub struct Viewer {
    m: Match,
    bots: Vec<Box<dyn Policy>>,
}

impl Viewer {
    /// A [`MatchConfig::duel`] with `team0` vs `team1` (bot names for
    /// [`BotKind::parse`]). `seed` is a decimal u64 string (JS numbers can't hold
    /// every u64).
    pub fn new(seed: &str, team0: &str, team1: &str) -> Result<Self, String> {
        let seed: u64 = seed
            .trim()
            .parse()
            .map_err(|e| format!("seed must be a decimal u64: {e}"))?;
        let bots = vec![
            BotKind::parse(team0)?.build(seed, 0),
            BotKind::parse(team1)?.build(seed, 1),
        ];
        Ok(Self {
            m: Match::new(MatchConfig::duel(), seed),
            bots,
        })
    }

    /// Advance up to `n` ticks (stops early when the match ends). Returns true if over.
    pub fn step(&mut self, n: u32) -> bool {
        for _ in 0..n {
            if self.m.is_over() {
                break;
            }
            // Same as `Match::step_policies`: dead tanks idle, living ones ask their bot.
            let actions: Vec<Action> = (0..self.m.tanks().len())
                .map(|i| match self.bots.get_mut(i) {
                    Some(b) if self.m.tanks()[i].alive => b.act(&self.m.observe(i)),
                    _ => Action::default(),
                })
                .collect();
            self.m.step(&actions);
        }
        self.m.is_over()
    }

    /// The underlying engine match.
    pub fn inner(&self) -> &Match {
        &self.m
    }

    /// The result, once the match has ended.
    pub fn outcome(&self) -> Option<OutcomeView> {
        self.m.outcome().map(|o| OutcomeView {
            winner: o.winner,
            ticks: o.ticks,
            reason: o.reason,
        })
    }

    /// Snapshot of the current state for drawing.
    pub fn state(&self) -> StateView {
        let cfg = self.m.config();
        let size: Vec2 = cfg.arena.size;
        StateView {
            tick: self.m.tick(),
            max_ticks: cfg.max_ticks,
            width: size.x,
            height: size.y,
            obstacles: cfg
                .arena
                .obstacles
                .iter()
                .map(|r| RectView {
                    x: r.min.x,
                    y: r.min.y,
                    w: r.max.x - r.min.x,
                    h: r.max.y - r.min.y,
                })
                .collect(),
            tanks: self
                .m
                .tanks()
                .iter()
                .map(|t| TankView {
                    id: t.id,
                    team: t.team,
                    x: t.pos.x,
                    y: t.pos.y,
                    heading: radians(t.heading),
                    turret: radians(t.turret),
                    hp: t.hp,
                    max_hp: cfg.params.max_hp,
                    alive: t.alive,
                })
                .collect(),
            projectiles: self
                .m
                .projectiles()
                .iter()
                .map(|p| ProjectileView {
                    x: p.pos.x,
                    y: p.pos.y,
                    vx: p.vel.x,
                    vy: p.vel.y,
                    team: p.team,
                })
                .collect(),
            outcome: self.outcome(),
        }
    }
}

/// JS handle: `new WasmMatch("42", "Chaser", "Wanderer")`.
#[wasm_bindgen]
pub struct WasmMatch(Viewer);

#[wasm_bindgen]
impl WasmMatch {
    /// A duel: `seed` as a decimal string, then two bot names (`Chaser` or `Wanderer`,
    /// case-insensitive). Bad input throws a JS `Error`.
    #[wasm_bindgen(constructor)]
    pub fn new(seed: &str, team0: &str, team1: &str) -> Result<WasmMatch, JsError> {
        Viewer::new(seed, team0, team1)
            .map(WasmMatch)
            .map_err(|e| JsError::new(&e))
    }

    /// Advance up to `n` ticks; returns true once the match is over.
    pub fn step(&mut self, n: u32) -> bool {
        self.0.step(n)
    }

    /// Ticks simulated so far.
    pub fn tick(&self) -> u32 {
        self.0.inner().tick()
    }

    /// True once the match has ended.
    #[wasm_bindgen(js_name = isOver)]
    pub fn is_over(&self) -> bool {
        self.0.inner().is_over()
    }

    /// Full render state as JSON (see `StateView`).
    #[wasm_bindgen(js_name = stateJson)]
    pub fn state_json(&self) -> String {
        serde_json::to_string(&self.0.state()).expect("state serializes")
    }

    /// Outcome as JSON, or `"null"` while the match is running.
    #[wasm_bindgen(js_name = outcomeJson)]
    pub fn outcome_json(&self) -> String {
        serde_json::to_string(&self.0.outcome()).expect("outcome serializes")
    }

    /// Current state hash as 16 hex digits; at the end of a Chaser vs Wanderer match
    /// it equals `engine-cli`'s `hash` for the same seed.
    #[wasm_bindgen(js_name = stateHash)]
    pub fn state_hash(&self) -> String {
        format!("{:016x}", self.0.inner().state_hash())
    }
}

/// Engine crate version.
#[wasm_bindgen(js_name = engineVersion)]
pub fn engine_version() -> String {
    engine::version().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bots_and_rejects_bad_input() {
        assert_eq!(BotKind::parse(" chaser ").unwrap(), BotKind::Chaser);
        assert_eq!(BotKind::parse("Wanderer").unwrap(), BotKind::Wanderer);
        assert!(BotKind::parse("sniper").is_err());
        assert!(Viewer::new("-1", "Chaser", "Wanderer").is_err());
        assert!(Viewer::new("abc", "Chaser", "Wanderer").is_err());
        assert!(Viewer::new("18446744073709551615", "Chaser", "Wanderer").is_ok());
    }

    #[test]
    fn chaser_vs_wanderer_matches_headless_run() {
        for seed in [0u64, 42, 1234] {
            let mut v = Viewer::new(&seed.to_string(), "Chaser", "Wanderer").unwrap();
            while !v.step(7) {}
            let mut m = Match::new(MatchConfig::duel(), seed);
            let (mut a, mut b) = (Chaser, Wanderer::new(seed ^ 0x5eed));
            let o = m.run(&mut [&mut a, &mut b]);
            assert_eq!(v.inner().outcome(), Some(o));
            assert_eq!(v.inner().state_hash(), m.state_hash());
        }
    }

    #[test]
    fn state_shape_and_stepping() {
        let mut v = Viewer::new("42", "Wanderer", "Chaser").unwrap();
        let s = v.state();
        assert_eq!((s.width, s.height), (800.0, 600.0));
        assert_eq!(s.obstacles.len(), 2);
        assert_eq!(s.tanks.len(), 2);
        assert!(s.outcome.is_none());
        assert!(!v.step(30));
        assert_eq!(v.state().tick, 30);
        let json = serde_json::to_string(&v.state()).unwrap();
        assert!(json.contains(r#""outcome":null"#), "{json}");
        while !v.step(1000) {}
        let s = v.state();
        let o = s.outcome.expect("ended");
        assert_eq!(o.ticks, s.tick);
        assert!(serde_json::to_string(&v.outcome())
            .unwrap()
            .contains("reason"));
    }

    #[test]
    fn mirror_wanderers_use_distinct_rngs() {
        let mut v = Viewer::new("5", "Wanderer", "Wanderer").unwrap();
        v.step(300);
        let s = v.state();
        assert_ne!(s.tanks[0].heading, s.tanks[1].heading);
    }
}
