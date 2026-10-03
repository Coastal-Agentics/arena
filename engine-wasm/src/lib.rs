//! Browser bindings for the Coastal Agentics engine: create a duel from a seed and a bot
//! pairing (optionally with a custom `MatchConfig`, e.g. per-tank params), step it, and
//! read the state as JSON. Used by `web/arena.html`.
//!
//! Two kinds of duel:
//! * **Tank Arena** ([`Viewer::tank`], JS `WasmMatch.tank(query)`): the `games/tank`
//!   rules, loadouts and charger/kiter/sniper policies from a URL query
//!   (`seed=42&blue=kiter-5-3-1&orange=charger-4-1-4`);
//! * **Built-in bots** ([`Viewer::new`] / [`Viewer::with_config`]): the placeholder
//!   [`tank::Chaser`]/[`tank::Wanderer`], `MatchConfig::duel` by default, identical to
//!   `engine-cli`.
//!
//! Separately, [`check_replay`] (JS `checkReplayJson`) re-simulates a replay file for the
//! native-vs-wasm parity check (`scripts/check-parity.mjs`, `tests/parity.rs`).
//!
//! **Builds** (the per-game part of a Nyborg): `games()`, `catalogJson(game)`,
//! `defaultBuild(game)` and `validateBuild(game, buildJson)` dispatch on the game id
//! through `GAMES`, one entry per game crate in the shared [`game_catalog`] shape
//! ([`tank::catalog`] today). Every way to start a match from a build or config goes
//! through the same check: [`Viewer::from_builds`] (JS `WasmMatch.fromBuilds`) validates
//! each build, [`Viewer::tank`] checks each tank of the query, and
//! [`Viewer::with_config`] rejects per-tank params that aren't a valid build.
//!
//! The sim logic lives in [`Viewer`] (plain Rust, unit-tested natively); the
//! `#[wasm_bindgen]` [`WasmMatch`] wrapper only converts errors to JS.
//!
//! Coordinates are the engine's: Y-up, origin bottom-left. Angles in the JSON are
//! radians (counter-clockwise from +X), converted from integer headings for display
//! only; the sim itself never sees them.

use engine::{Action, EndReason, Heading, Match, MatchConfig, Policy, Vec2};
use serde::Serialize;
use tank::catalog;
use tank::{loadout, Behavior, Chaser, Loadout, MatchSpec, Preset, TankSpec, Wanderer};
use wasm_bindgen::prelude::*;

/// Built-in bots the viewer can pit against each other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BotKind {
    /// [`tank::Chaser`].
    Chaser,
    /// [`tank::Wanderer`], seeded from the match seed (see [`Viewer::new`]).
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
    /// This tank's own `TankParams::max_hp` (`Match::tank_params`), so HP bars stay
    /// right when spawns carry per-tank params.
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

/// `[{"code", "key"}, ...]` for an error message.
fn errors_json(errors: &[game_catalog::BuildError]) -> String {
    to_json(&errors)
}

fn to_json(v: &impl serde::Serialize) -> String {
    serde_json::to_string(v).expect("catalog JSON serializes")
}

fn parse_seed(seed: &str) -> Result<u64, String> {
    seed.trim()
        .parse()
        .map_err(|e| format!("seed must be a decimal u64: {e}"))
}

fn radians(h: Heading) -> f32 {
    h as f32 * (std::f32::consts::TAU / 65536.0)
}

/// A duel between two built-in bots, steppable from a render loop.
pub struct Viewer {
    m: Match,
    bots: Vec<Box<dyn Policy>>,
    setup: Option<SetupView>,
}

/// One tank's setup in [`SetupView`].
#[derive(Serialize, Debug, PartialEq)]
pub struct TankSetupView {
    /// Behavior key (`kiter`).
    pub behavior: &'static str,
    /// Behavior display name (`Kiter`).
    pub name: &'static str,
    /// How the behavior was made (SPEC "Training indicator"): `Scripted` today.
    pub training: &'static str,
    /// Loadout, `A-S-D`.
    pub loadout: String,
}

/// A Tank Arena duel's setup; serialized by [`WasmMatch::setup_json`].
#[derive(Serialize, Debug, PartialEq)]
pub struct SetupView {
    /// Canonical URL query that reproduces this match.
    pub query: String,
    /// Seed as a decimal string.
    pub seed: String,
    /// Tank 0 (Blue) then tank 1 (Orange).
    pub tanks: Vec<TankSetupView>,
}

impl Viewer {
    /// A [`MatchConfig::duel`] with `team0` vs `team1` (bot names for
    /// [`BotKind::parse`]). `seed` is a decimal u64 string (JS numbers can't hold
    /// every u64).
    pub fn new(seed: &str, team0: &str, team1: &str) -> Result<Self, String> {
        Self::with_config(MatchConfig::duel(), seed, team0, team1)
    }

    /// Like [`Viewer::new`], but with any config (arena, spawns, per-tank params, tick
    /// limit). `team0` drives tank 0 and `team1` tank 1; further tanks idle. A tank's
    /// own `params` must be a valid build on the shared params
    /// ([`tank::catalog::check_config`]).
    pub fn with_config(
        config: MatchConfig,
        seed: &str,
        team0: &str,
        team1: &str,
    ) -> Result<Self, String> {
        let seed = parse_seed(seed)?;
        catalog::check_config(&config)?;
        let bots = vec![
            BotKind::parse(team0)?.build(seed, 0),
            BotKind::parse(team1)?.build(seed, 1),
        ];
        Ok(Self {
            m: Match::new(config, seed),
            bots,
            setup: None,
        })
    }

    /// A Tank Arena duel from a URL query (see [`tank::MatchSpec::from_query`]):
    /// fixed spawns, per-tank loadouts, the charger/kiter/sniper policies.
    pub fn tank(query: &str) -> Result<Self, String> {
        Self::from_spec(MatchSpec::from_query(query)?)
    }

    /// A Tank Arena duel from two builds (`blue` is tank 0): each is validated with
    /// [`tank::catalog::validate_build`] and must have a scripted behavior.
    /// Plays exactly like [`Viewer::tank`] with the same seed, behaviors and levels.
    pub fn from_builds(game: &str, seed: &str, blue: &str, orange: &str) -> Result<Self, String> {
        let seed = parse_seed(seed)?;
        let tank = |side: &str, json: &str| -> Result<TankSpec, String> {
            let valid = if game == catalog::GAME {
                catalog::validate_build(json)
            } else {
                Err(vec![game_catalog::BuildError::new("wrong_game", "game")])
            };
            let valid = valid.map_err(|e| format!("{side}: invalid build {}", errors_json(&e)))?;
            catalog::tank_spec(&valid).map_err(|e| format!("{side}: {e}"))
        };
        Self::from_spec(MatchSpec {
            seed,
            blue: tank("blue", blue)?,
            orange: tank("orange", orange)?,
        })
    }

    fn from_spec(spec: MatchSpec) -> Result<Self, String> {
        for (side, t) in [("blue", &spec.blue), ("orange", &spec.orange)] {
            catalog::validate_spec(t)
                .map_err(|e| format!("{side}: invalid build {}", errors_json(&e)))?;
        }
        let (m, [a, b]) = spec.start();
        let view = |t: &tank::TankSpec| TankSetupView {
            behavior: t.behavior.key(),
            name: t.behavior.name(),
            training: t.behavior.training(),
            loadout: t.loadout.to_string(),
        };
        let setup = SetupView {
            query: spec.to_query(),
            seed: spec.seed.to_string(),
            tanks: vec![view(&spec.blue), view(&spec.orange)],
        };
        Ok(Self {
            m,
            bots: vec![a, b],
            setup: Some(setup),
        })
    }

    /// The Tank Arena setup, or `None` for a built-in-bot duel.
    pub fn setup(&self) -> Option<&SetupView> {
        self.setup.as_ref()
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
                    max_hp: self.m.tank_params(t.id).max_hp,
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

    /// Like the constructor, but with a custom config: `config_json` is a
    /// `MatchConfig` as JSON (start from [`duel_config_json`] and edit it, e.g. set
    /// `tanks[i].params` for a loadout). Bad JSON or input throws a JS `Error`.
    #[wasm_bindgen(js_name = withConfig)]
    pub fn with_config(
        config_json: &str,
        seed: &str,
        team0: &str,
        team1: &str,
    ) -> Result<WasmMatch, JsError> {
        let config: MatchConfig = serde_json::from_str(config_json)
            .map_err(|e| JsError::new(&format!("config json: {e}")))?;
        Viewer::with_config(config, seed, team0, team1)
            .map(WasmMatch)
            .map_err(|e| JsError::new(&e))
    }

    /// A Tank Arena duel from a URL query (`seed=42&blue=kiter-5-3-1&orange=charger`);
    /// missing keys take defaults, unknown keys are ignored. Bad input throws.
    pub fn tank(query: &str) -> Result<WasmMatch, JsError> {
        Viewer::tank(query)
            .map(WasmMatch)
            .map_err(|e| JsError::new(&e))
    }

    /// A Tank Arena duel from two build JSONs (`{rules_version, levels, behavior}`) for
    /// `game` (`"tank"`). Each is checked with the same validator as `validateBuild`;
    /// an invalid build, or a champion behavior (the loader resolves those), throws.
    #[wasm_bindgen(js_name = fromBuilds)]
    pub fn from_builds(
        game: &str,
        seed: &str,
        blue: &str,
        orange: &str,
    ) -> Result<WasmMatch, JsError> {
        Viewer::from_builds(game, seed, blue, orange)
            .map(WasmMatch)
            .map_err(|e| JsError::new(&e))
    }

    /// Tank Arena setup as JSON (see `SetupView`), or `"null"` for built-in bots.
    #[wasm_bindgen(js_name = setupJson)]
    pub fn setup_json(&self) -> String {
        serde_json::to_string(&self.0.setup()).expect("setup serializes")
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

/// `MatchConfig::duel()` as JSON: the default config, a starting point for
/// [`WasmMatch::with_config`].
#[wasm_bindgen(js_name = duelConfigJson)]
pub fn duel_config_json() -> String {
    serde_json::to_string(&MatchConfig::duel()).expect("config serializes")
}

/// Everything the Customize tab shows, from `games/tank` (one source of truth).
#[derive(Serialize, Debug)]
pub struct CatalogView {
    /// Points per tank.
    pub budget: u8,
    /// Damage by Attack level (index 0 = level 1).
    pub damage: [i32; 5],
    /// Max speed (u/s) by Speed level.
    pub max_speed: [f32; 5],
    /// Turn rate (BAU/tick) by Speed level.
    pub turn_rate: [u16; 5],
    /// Fire cooldown (ticks between shots) by Speed level.
    pub fire_cooldown: [u32; 5],
    /// Max HP by Defense level.
    pub max_hp: [i32; 5],
    /// The 19 valid loadouts as `A-S-D`, ordered by Attack then Speed.
    pub loadouts: Vec<String>,
    /// `(name, loadout)` presets.
    pub presets: Vec<(&'static str, String)>,
    /// `(key, name, training)` behaviors.
    pub behaviors: Vec<(&'static str, &'static str, &'static str)>,
    /// The default match's canonical query.
    pub default_query: String,
}

/// The Customize tab's tables and lists.
pub fn catalog() -> CatalogView {
    CatalogView {
        budget: loadout::BUDGET,
        damage: loadout::DAMAGE,
        max_speed: loadout::MAX_SPEED,
        turn_rate: loadout::TURN_RATE,
        fire_cooldown: loadout::FIRE_COOLDOWN,
        max_hp: loadout::MAX_HP,
        loadouts: Loadout::ALL.iter().map(|l| l.to_string()).collect(),
        presets: Preset::ALL
            .iter()
            .map(|p| (p.name(), p.loadout().to_string()))
            .collect(),
        behaviors: Behavior::ALL
            .iter()
            .map(|b| (b.key(), b.name(), b.training()))
            .collect(),
        default_query: MatchSpec::default().to_query(),
    }
}

/// The Customize tab's tables and lists as JSON (see `CatalogView`).
#[wasm_bindgen(js_name = tankCatalogJson)]
pub fn tank_catalog_json() -> String {
    serde_json::to_string(&catalog()).expect("catalog serializes")
}

/// A game with a build catalog: its id, its `catalog()` and its `validate_build`
/// rendered as `validateBuild`'s JSON. Both functions live in the game's crate, in the
/// shared [`game_catalog`] shape.
struct Game {
    id: &'static str,
    catalog: fn() -> game_catalog::Catalog,
    validate_json: fn(&str) -> String,
}

/// Every game with a build catalog, in `games()` order. A new game (racing, M3) is one
/// entry here plus its crate dependency.
const GAMES: &[Game] = &[Game {
    id: tank::catalog::GAME,
    catalog: tank::catalog::catalog,
    validate_json: |json| game_catalog::validation_json(&tank::catalog::validate_build(json)),
}];

fn game(id: &str) -> Result<&'static Game, JsError> {
    GAMES
        .iter()
        .find(|g| g.id == id)
        .ok_or_else(|| JsError::new(&format!("unknown game {id:?}")))
}

/// The games and their rules versions, as JSON: `[{"game": "tank", "rules_version": 1}]`.
#[wasm_bindgen]
pub fn games() -> String {
    #[derive(Serialize)]
    struct GameInfo {
        game: &'static str,
        rules_version: u64,
    }
    let list: Vec<GameInfo> = GAMES
        .iter()
        .map(|g| GameInfo {
            game: g.id,
            rules_version: (g.catalog)().rules_version,
        })
        .collect();
    to_json(&list)
}

/// A game's build catalog as JSON (budget, stats with per-level values, scripted
/// behaviors, presets, default build); see [`game_catalog::Catalog`] and
/// [`tank::catalog::catalog`]. Throws for an unknown game.
#[wasm_bindgen(js_name = catalogJson)]
pub fn catalog_json(game_id: &str) -> Result<String, JsError> {
    Ok(to_json(&(game(game_id)?.catalog)()))
}

/// A game's default build as JSON (tank: 3/3/3, first scripted behavior). Throws for
/// an unknown game.
#[wasm_bindgen(js_name = defaultBuild)]
pub fn default_build(game_id: &str) -> Result<String, JsError> {
    Ok(to_json(&(game(game_id)?.catalog)().default_build))
}

/// Check a build: returns `{"ok": true, levels, behavior, points, params, ...}` or
/// `{"ok": false, "errors": [{"code", "key"}, ...]}` as JSON (`wrong_game` for an
/// unknown game). Never throws.
#[wasm_bindgen(js_name = validateBuild)]
pub fn validate_build(game_id: &str, build_json: &str) -> String {
    match GAMES.iter().find(|g| g.id == game_id) {
        Some(g) => (g.validate_json)(build_json),
        None => game_catalog::unknown_game_json(),
    }
}

/// Snap barycentric triangle weights (Attack, Speed, Defense corners) to a loadout,
/// returned as `A-S-D` (`tank::Loadout::snap`).
#[wasm_bindgen(js_name = snapLoadout)]
pub fn snap_loadout(attack: f32, speed: f32, defense: f32) -> String {
    Loadout::snap([attack, speed, defense]).to_string()
}

/// Canonical form of a Tank Arena URL query; throws on invalid input.
#[wasm_bindgen(js_name = canonicalTankQuery)]
pub fn canonical_tank_query(query: &str) -> Result<String, JsError> {
    MatchSpec::from_query(query)
        .map(|m| m.to_query())
        .map_err(|e| JsError::new(&e))
}

/// What re-simulating a replay file gives, for the native-vs-wasm parity check
/// (`scripts/check-parity.mjs`, `engine-wasm/tests/parity.rs`). Every value except
/// `format` and `seed` is **recomputed** from the replay's seed, config and actions, not
/// copied from the file, so comparing it with a manifest compares engines.
#[derive(Serialize, Debug, PartialEq)]
pub struct ReplayCheck {
    /// The file's `format`.
    pub format: u32,
    /// The file's seed, as a decimal string.
    #[serde(with = "engine::json_u64")]
    pub seed: u64,
    /// Number of tanks in the config.
    pub tanks: usize,
    /// Ticks re-simulated (one per recorded action list).
    pub ticks: u32,
    /// Outcome after re-simulating, or null if the actions stop before the end.
    pub outcome: Option<OutcomeView>,
    /// State hash after re-simulating, 16 lowercase hex digits.
    pub final_hash: String,
    /// `setup_hash` recomputed from the seed and config, 16 lowercase hex digits.
    pub setup_hash: String,
    /// `null` if [`engine::Replay::verify`] passes, else its error message.
    pub verify_error: Option<String>,
}

/// Load a replay (any readable format) and re-simulate it; see [`ReplayCheck`].
/// Errors only if the JSON doesn't load ([`engine::Replay::from_json`]); a replay that
/// loads but doesn't verify is reported in `verify_error`.
pub fn check_replay(json: &str) -> Result<ReplayCheck, String> {
    let r = engine::Replay::from_json(json).map_err(|e| e.to_string())?;
    let (m, verify_error) = match r.verify() {
        Ok(m) => (m, None),
        Err(e) => (r.play(), Some(e.to_string())),
    };
    Ok(ReplayCheck {
        format: r.format,
        seed: r.seed,
        tanks: m.tanks().len(),
        ticks: m.tick(),
        outcome: m.outcome().map(|o| OutcomeView {
            winner: o.winner,
            ticks: o.ticks,
            reason: o.reason,
        }),
        final_hash: format!("{:016x}", m.state_hash()),
        setup_hash: format!("{:016x}", engine::replay::setup_hash(r.seed, &r.config)),
        verify_error,
    })
}

/// Re-simulate a replay file's JSON and return a JSON [`ReplayCheck`]. Throws if the
/// JSON doesn't load as a replay.
#[wasm_bindgen(js_name = checkReplayJson)]
pub fn check_replay_json(json: &str) -> Result<String, JsError> {
    check_replay(json)
        .map(|c| serde_json::to_string(&c).expect("check serializes"))
        .map_err(|e| JsError::new(&e))
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
    fn custom_config_with_per_tank_params() {
        // The default config through JSON plays exactly like `Viewer::new`.
        let duel: MatchConfig = serde_json::from_str(&duel_config_json()).unwrap();
        let mut a = Viewer::new("42", "Chaser", "Wanderer").unwrap();
        let mut b = Viewer::with_config(duel, "42", "Chaser", "Wanderer").unwrap();
        while !a.step(50) {}
        while !b.step(50) {}
        assert_eq!(a.inner().state_hash(), b.inner().state_hash());
        // Per-tank params must be a valid build on the shared params; each TankView
        // reports its own max_hp.
        let mut c = MatchConfig::duel();
        c.tanks[1].params = Some(Loadout::new(2, 2, 5).unwrap().apply(&c.params));
        let v = Viewer::with_config(c.clone(), "42", "Chaser", "Wanderer").unwrap();
        let s = v.state();
        assert_eq!((s.tanks[0].max_hp, s.tanks[1].max_hp), (100, 940));
        assert_eq!(s.tanks[1].hp, 940);
        // A tampered config (more HP than any build gives) can't start.
        c.tanks[1].params.as_mut().unwrap().max_hp = 2000;
        let err = Viewer::with_config(c, "42", "Chaser", "Wanderer")
            .err()
            .unwrap();
        assert!(err.starts_with("tanks[1].params"), "{err}");
        assert!(Viewer::with_config(MatchConfig::duel(), "x", "Chaser", "Wanderer").is_err());
    }

    #[test]
    fn tank_duel_matches_the_tank_crate() {
        for q in [
            "seed=42&blue=kiter-5-3-1&orange=charger-4-1-4",
            "seed=7&blue=sniper&orange=kiter",
            "",
        ] {
            let mut v = Viewer::tank(q).unwrap();
            while !v.step(13) {}
            let spec = MatchSpec::from_query(q).unwrap();
            let (o, hash) = spec.run();
            assert_eq!(v.inner().outcome(), Some(o), "{q}");
            assert_eq!(v.inner().state_hash(), hash, "{q}");
            let s = v.setup().unwrap();
            assert_eq!(s.query, spec.to_query());
            assert_eq!(s.tanks[0].training, "Scripted");
        }
        let v = Viewer::tank("seed=1&blue=kiter-5-3-1&orange=charger-4-1-4").unwrap();
        let s = v.setup().unwrap();
        assert_eq!(
            (s.tanks[0].loadout.as_str(), s.tanks[1].name),
            ("5-3-1", "Charger")
        );
        let st = v.state();
        assert_eq!((st.tanks[0].max_hp, st.tanks[1].max_hp), (460, 790));
        assert!(Viewer::tank("blue=kiter-5-3-2").is_err());
        assert!(Viewer::new("1", "Chaser", "Wanderer")
            .unwrap()
            .setup()
            .is_none());
    }

    #[test]
    fn check_replay_recomputes_and_reports() {
        let mut v = Viewer::new("7", "Chaser", "Wanderer").unwrap();
        while !v.step(100) {}
        let json = v.inner().replay().to_json();
        let c = check_replay(&json).unwrap();
        assert_eq!((c.format, c.seed, c.tanks, c.ticks), (4, 7, 2, 276));
        assert_eq!(c.final_hash, "51234f61b02b5784");
        assert_eq!(c.setup_hash, "0b24ce74f45e9a27");
        assert_eq!(c.verify_error, None);
        let edited = json.replacen("\"seed\":\"7\"", "\"seed\":\"8\"", 1);
        let c = check_replay(&edited).unwrap();
        assert!(c.verify_error.unwrap().starts_with("setup hash mismatch"));
        assert!(check_replay("{}").is_err());
    }

    #[test]
    fn catalog_and_snap() {
        let c = catalog();
        assert_eq!(c.loadouts.len(), 19);
        assert_eq!(c.presets[1], ("Glass Cannon", "5-3-1".to_string()));
        assert_eq!(c.fire_cooldown, tank::loadout::FIRE_COOLDOWN);
        assert_eq!(c.max_hp[2], 650);
        assert_eq!(snap_loadout(1.0, 1.0, 1.0), "3-3-3");
        assert_eq!(snap_loadout(1.0, 0.0, 0.0), "5-2-2");
        assert_eq!(
            MatchSpec::from_query(&c.default_query).unwrap(),
            MatchSpec::default()
        );
    }

    #[test]
    fn builds_start_the_same_match_as_the_query() {
        let b = |l: &str, id: &str| {
            let [a, s, d]: [u8; 3] = l
                .split('-')
                .map(|x| x.parse().unwrap())
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();
            format!(
                r#"{{"rules_version":1,"levels":{{"attack":{a},"speed":{s},"defense":{d}}},"behavior":{{"kind":"scripted","id":"{id}"}}}}"#
            )
        };
        for (seed, blue, orange) in [
            ("42", ("5-3-1", "kiter"), ("4-1-4", "charger")),
            ("7", ("2-5-2", "sniper"), ("3-3-3", "kiter")),
        ] {
            let q = format!(
                "seed={seed}&blue={}-{}&orange={}-{}",
                blue.1, blue.0, orange.1, orange.0
            );
            let mut a = Viewer::tank(&q).unwrap();
            let mut f =
                Viewer::from_builds("tank", seed, &b(blue.0, blue.1), &b(orange.0, orange.1))
                    .unwrap();
            assert_eq!(a.setup(), f.setup());
            while !a.step(97) {}
            while !f.step(97) {}
            assert_eq!(a.inner().state_hash(), f.inner().state_hash(), "{q}");
            assert_eq!(f.inner().replay().to_json(), a.inner().replay().to_json());
        }
        let ok = b("3-3-3", "kiter");
        let err = Viewer::from_builds("tank", "1", &b("5-3-2", "kiter"), &ok)
            .err()
            .unwrap();
        assert_eq!(
            err,
            r#"blue: invalid build [{"code":"over_budget","key":"levels"}]"#
        );
        assert!(Viewer::from_builds("racing", "1", &ok, &ok)
            .err()
            .unwrap()
            .contains("wrong_game"));
        assert!(Viewer::from_builds("tank", "x", &ok, &ok).is_err());
        let champ = r#"{"rules_version":1,"levels":{"attack":3,"speed":3,"defense":3},"behavior":{"kind":"champion","ref":"tank/nightly/gen-99"}}"#;
        assert!(Viewer::from_builds("tank", "1", &ok, champ)
            .err()
            .unwrap()
            .starts_with("orange: a champion"));
    }

    #[test]
    fn build_exports_wrap_the_tank_catalog() {
        assert_eq!(games(), r#"[{"game":"tank","rules_version":1}]"#);
        let default = default_build("tank").unwrap();
        assert_eq!(
            default,
            r#"{"rules_version":1,"levels":{"attack":3,"speed":3,"defense":3},"behavior":{"kind":"scripted","id":"charger"}}"#
        );
        let ok = validate_build("tank", &default);
        assert!(ok.starts_with(r#"{"ok":true,"game":"tank","rules_version":1,"levels":{"attack":3,"speed":3,"defense":3},"behavior":{"kind":"scripted","id":"charger"},"points":9,"params":{"#), "{ok}");
        assert_eq!(
            validate_build(
                "tank",
                r#"{"rules_version":1,"levels":{"attack":5,"speed":3,"defense":2},"behavior":{"kind":"scripted","id":"kiter"}}"#
            ),
            r#"{"ok":false,"errors":[{"code":"over_budget","key":"levels"}]}"#
        );
        assert!(validate_build("tank", "nope").contains("invalid_json"));
    }

    #[test]
    fn mirror_wanderers_use_distinct_rngs() {
        let mut v = Viewer::new("5", "Wanderer", "Wanderer").unwrap();
        v.step(300);
        let s = v.state();
        assert_ne!(s.tanks[0].heading, s.tanks[1].heading);
    }
}
