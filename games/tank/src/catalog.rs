//! Tank Arena's build catalog and validator: what the Customizer shows and what
//! every match started from a build is checked against (game-system.md, "catalog";
//! nyborg-library.md §4–5).
//!
//! A **build** is the per-game part of a Nyborg:
//!
//! ```json
//! {"rules_version": 1,
//!  "levels": {"attack": 5, "speed": 3, "defense": 1},
//!  "behavior": {"kind": "scripted", "id": "kiter"}}
//! ```
//!
//! `behavior` can also be `{"kind": "champion", "ref": "tank/nightly/gen-99"}`; only
//! its shape is checked here, because the loader resolves the ref. A Nyborg's `look`
//! never reaches this module or the engine.
//!
//! Every number comes from [`crate::loadout`]'s tables, so the catalog can't drift
//! from the rules. Every path that starts a match from a build ([`validate_build`],
//! [`validate_spec`] for URL queries, [`check_config`] for raw configs) shares one
//! level and budget check, so an imported or edited build can't beat the budget.

use crate::loadout::{
    Loadout, Preset, BUDGET, DAMAGE, FIRE_COOLDOWN, MAX_HP, MAX_LEVEL, MAX_SPEED, MIN_LEVEL,
    TURN_RATE,
};
use crate::matchup::TankSpec;
use crate::policies::Behavior;
use engine::{MatchConfig, TankParams};
use serde::Serialize;

/// This game's id in `games()`, `catalogJson(game)` and a Nyborg's `builds`.
pub const GAME: &str = "tank";
/// Version of the tank rules a build's levels refer to. Bump it whenever a level's
/// meaning changes (a table, the budget or the stat keys); saved builds with another
/// version then fail with `rules_version_mismatch` and must be rebuilt.
pub const RULES_VERSION: u64 = 1;
/// Points one level of any stat costs.
pub const COST_PER_LEVEL: u8 = 1;
/// The stat keys, in catalog order: Attack, Speed, Defense.
pub const STAT_KEYS: [&str; 3] = ["attack", "speed", "defense"];
const STAT_LABELS: [&str; 3] = ["Attack", "Speed", "Defense"];

/// Why a build is invalid. Every error names the key it failed on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BuildError {
    /// `wrong_game`, `rules_version_mismatch`, `unknown_key`, `out_of_range`,
    /// `over_budget`, `under_budget`, `unknown_behavior` or `invalid_json`.
    pub code: &'static str,
    /// The key it failed on: a stat key, `levels` (budget), `behavior`,
    /// `rules_version`, `game`, or empty for `invalid_json`.
    pub key: String,
}

impl BuildError {
    fn new(code: &'static str, key: &str) -> Self {
        Self {
            code,
            key: key.to_string(),
        }
    }
}

/// A build's behavior, after validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuildBehavior {
    /// One of the scripted policies.
    Scripted(Behavior),
    /// A published champion, by ref (resolved by the loader, not here).
    Champion(String),
}

/// A valid build.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidBuild {
    /// The levels, as a checked [`Loadout`].
    pub loadout: Loadout,
    /// The behavior.
    pub behavior: BuildBehavior,
}

impl ValidBuild {
    /// Points spent (always [`BUDGET`] for a valid build).
    pub fn points(&self) -> u32 {
        self.loadout.levels().iter().map(|&l| l as u32).sum::<u32>() * COST_PER_LEVEL as u32
    }
}

/// A stat's levels by key, as in a build's `levels`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Levels {
    /// Attack level.
    pub attack: u8,
    /// Speed level.
    pub speed: u8,
    /// Defense level.
    pub defense: u8,
}

impl From<Loadout> for Levels {
    fn from(l: Loadout) -> Self {
        let [attack, speed, defense] = l.levels();
        Self {
            attack,
            speed,
            defense,
        }
    }
}

/// A build's `behavior` as JSON: `{"kind": "scripted", "id"}` or
/// `{"kind": "champion", "ref"}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BehaviorJson {
    /// A scripted behavior by id.
    Scripted {
        /// `charger`, `kiter` or `sniper`.
        id: &'static str,
    },
    /// A champion by ref.
    Champion {
        /// The ref the loader resolves.
        #[serde(rename = "ref")]
        reference: String,
    },
}

impl From<&BuildBehavior> for BehaviorJson {
    fn from(b: &BuildBehavior) -> Self {
        match b {
            BuildBehavior::Scripted(b) => Self::Scripted { id: b.key() },
            BuildBehavior::Champion(r) => Self::Champion {
                reference: r.clone(),
            },
        }
    }
}

/// A build as JSON (`defaultBuild`, `default_build` in the catalog).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BuildJson {
    /// [`RULES_VERSION`].
    pub rules_version: u64,
    /// The levels.
    pub levels: Levels,
    /// The behavior.
    pub behavior: BehaviorJson,
}

/// One entry of `games()`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GameInfo {
    /// The game id.
    pub game: &'static str,
    /// Its rules version.
    pub rules_version: u64,
}

/// What one level of a stat resolves to. Only the fields of that stat are present.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LevelValues {
    /// The level, 1 to 5.
    pub level: u8,
    /// Attack: damage per hit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub damage: Option<i32>,
    /// Speed: top speed (units per second).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_speed: Option<f32>,
    /// Speed: turn rate (heading units per second).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_rate: Option<u16>,
    /// Speed: reload, in ticks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fire_cooldown: Option<u32>,
    /// Defense: hit points.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_hp: Option<i32>,
}

/// One stat in the catalog.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Stat {
    /// Key in a build's `levels`.
    pub key: &'static str,
    /// Customizer label.
    pub label: &'static str,
    /// Lowest level.
    pub min: u8,
    /// Highest level.
    pub max: u8,
    /// Points per level.
    pub cost_per_level: u8,
    /// Resolved values for levels `min..=max`.
    pub values: Vec<LevelValues>,
}

/// One preset in the catalog.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PresetJson {
    /// Snake-case id (`glass_cannon`).
    pub id: String,
    /// Label (`Glass Cannon`).
    pub label: &'static str,
    /// Its levels.
    pub levels: Levels,
}

/// `catalogJson("tank")`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Catalog {
    /// [`GAME`].
    pub game: &'static str,
    /// [`RULES_VERSION`].
    pub rules_version: u64,
    /// Points every build spends ([`BUDGET`]).
    pub budget: u8,
    /// Attack, Speed and Defense.
    pub stats: Vec<Stat>,
    /// Scripted behavior ids.
    pub behaviors: Vec<&'static str>,
    /// Presets.
    pub presets: Vec<PresetJson>,
    /// [`default_build`].
    pub default_build: BuildJson,
}

/// What `validateBuild` returns: `{"ok": true, ...}` with the normalized build, or
/// `{"ok": false, "errors": [...]}`.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Validation {
    /// A valid build, normalized.
    Ok {
        /// Always `true`.
        ok: bool,
        /// [`GAME`].
        game: &'static str,
        /// [`RULES_VERSION`].
        rules_version: u64,
        /// The levels.
        levels: Levels,
        /// The behavior.
        behavior: BehaviorJson,
        /// Points spent.
        points: u32,
        /// The resolved params the tank plays with ([`Loadout::params`]).
        params: TankParams,
    },
    /// An invalid build.
    Err {
        /// Always `false`.
        ok: bool,
        /// Every error found.
        errors: Vec<BuildError>,
    },
}

/// `games()`: the game ids with their rules versions. Tank only until racing (M3).
pub fn games() -> Vec<GameInfo> {
    vec![GameInfo {
        game: GAME,
        rules_version: RULES_VERSION,
    }]
}

/// The default build: 3/3/3 with the first scripted behavior.
pub fn default_build() -> BuildJson {
    BuildJson {
        rules_version: RULES_VERSION,
        levels: Loadout::DEFAULT.into(),
        behavior: BehaviorJson::Scripted {
            id: Behavior::ALL[0].key(),
        },
    }
}

/// `catalogJson("tank")`: budget, stats with their per-level values, scripted
/// behaviors, presets and the default build, all from the loadout tables.
pub fn catalog() -> Catalog {
    let none = |level: u8| LevelValues {
        level,
        damage: None,
        max_speed: None,
        turn_rate: None,
        fire_cooldown: None,
        max_hp: None,
    };
    let stat = |i: usize, values: &dyn Fn(usize, LevelValues) -> LevelValues| Stat {
        key: STAT_KEYS[i],
        label: STAT_LABELS[i],
        min: MIN_LEVEL,
        max: MAX_LEVEL,
        cost_per_level: COST_PER_LEVEL,
        values: (MIN_LEVEL..=MAX_LEVEL)
            .map(|l| values(l as usize - 1, none(l)))
            .collect(),
    };
    Catalog {
        game: GAME,
        rules_version: RULES_VERSION,
        budget: BUDGET,
        stats: vec![
            stat(0, &|i, v| LevelValues {
                damage: Some(DAMAGE[i]),
                ..v
            }),
            stat(1, &|i, v| LevelValues {
                max_speed: Some(MAX_SPEED[i]),
                turn_rate: Some(TURN_RATE[i]),
                fire_cooldown: Some(FIRE_COOLDOWN[i]),
                ..v
            }),
            stat(2, &|i, v| LevelValues {
                max_hp: Some(MAX_HP[i]),
                ..v
            }),
        ],
        behaviors: Behavior::ALL.iter().map(|b| b.key()).collect(),
        presets: Preset::ALL
            .iter()
            .map(|p| PresetJson {
                id: p.name().to_ascii_lowercase().replace(' ', "_"),
                label: p.name(),
                levels: p.loadout().into(),
            })
            .collect(),
        default_build: default_build(),
    }
}

/// The one level and budget check: each level an integer in `MIN_LEVEL..=MAX_LEVEL`
/// (missing counts as out of range), then the points spent exactly [`BUDGET`].
fn check_levels(levels: [Option<i64>; 3]) -> Result<Loadout, Vec<BuildError>> {
    let mut errors = Vec::new();
    for (key, level) in STAT_KEYS.iter().zip(levels) {
        if !level.is_some_and(|l| (MIN_LEVEL as i64..=MAX_LEVEL as i64).contains(&l)) {
            errors.push(BuildError::new("out_of_range", key));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let [a, s, d] = levels.map(|l| l.unwrap_or(0) as u8);
    let spent = (a as u32 + s as u32 + d as u32) * COST_PER_LEVEL as u32;
    if spent != BUDGET as u32 {
        let code = if spent > BUDGET as u32 {
            "over_budget"
        } else {
            "under_budget"
        };
        return Err(vec![BuildError::new(code, "levels")]);
    }
    Loadout::new(a, s, d).map_err(|_| vec![BuildError::new("out_of_range", "levels")])
}

/// `validateBuild(game, buildJson)`: the build if valid, else every error found.
///
/// Order: `invalid_json` (unparsable text, or not an object), then `wrong_game` (an
/// unknown `game`, or a `game` field in the build that differs) and
/// `rules_version_mismatch` stop the check, because the levels' meaning depends on
/// them; otherwise level, budget and behavior errors are all reported. Unknown
/// top-level fields (a Nyborg's `look`, say) are skipped, never kept.
pub fn validate_build(game: &str, build: &str) -> Result<ValidBuild, Vec<BuildError>> {
    let Ok(build @ Loose::Map(_)) = serde_json::from_str::<Loose>(build) else {
        return Err(vec![BuildError::new("invalid_json", "")]);
    };
    if game != GAME || build.get("game").is_some_and(|g| g.as_str() != Some(GAME)) {
        return Err(vec![BuildError::new("wrong_game", "game")]);
    }
    if build.get("rules_version").and_then(Loose::as_i64) != Some(RULES_VERSION as i64) {
        return Err(vec![BuildError::new(
            "rules_version_mismatch",
            "rules_version",
        )]);
    }

    let mut errors = Vec::new();
    let levels = build.get("levels");
    if let Some(Loose::Map(levels)) = levels {
        for (key, _) in levels
            .iter()
            .filter(|(k, _)| !STAT_KEYS.contains(&k.as_str()))
        {
            errors.push(BuildError::new("unknown_key", key));
        }
    }
    let level = |key: &str| levels.and_then(|l| l.get(key)).and_then(Loose::as_i64);
    let loadout = check_levels(STAT_KEYS.map(level));
    if let Err(e) = &loadout {
        errors.extend(e.iter().cloned());
    }

    let behavior = build.get("behavior").and_then(|b| {
        let field = |k| b.get(k).and_then(Loose::as_str);
        match field("kind")? {
            "scripted" => Behavior::ALL
                .into_iter()
                .find(|x| Some(x.key()) == field("id"))
                .map(BuildBehavior::Scripted),
            "champion" => field("ref")
                .filter(|r| !r.trim().is_empty() && r.len() <= 200)
                .map(|r| BuildBehavior::Champion(r.to_string())),
            _ => None,
        }
    });
    if behavior.is_none() {
        errors.push(BuildError::new("unknown_behavior", "behavior"));
    }

    match (loadout, behavior) {
        (Ok(loadout), Some(behavior)) if errors.is_empty() => Ok(ValidBuild { loadout, behavior }),
        _ => Err(errors),
    }
}

/// A parsed JSON value, kept only as far as [`validate_build`] reads it: integers,
/// strings and objects (in order; a repeated key's last value wins). Floats, booleans,
/// null and arrays are `Other`. Leaner in wasm than `serde_json::Value`.
enum Loose {
    Int(i64),
    Str(String),
    Map(Vec<(String, Loose)>),
    Other,
}

impl Loose {
    fn get(&self, key: &str) -> Option<&Loose> {
        match self {
            Loose::Map(m) => m.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    fn as_i64(&self) -> Option<i64> {
        match self {
            Loose::Int(i) => Some(*i),
            _ => None,
        }
    }
    fn as_str(&self) -> Option<&str> {
        match self {
            Loose::Str(s) => Some(s),
            _ => None,
        }
    }
}

impl<'de> serde::Deserialize<'de> for Loose {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::{MapAccess, SeqAccess, Visitor};
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Loose;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a JSON value")
            }
            fn visit_i64<E>(self, v: i64) -> Result<Loose, E> {
                Ok(Loose::Int(v))
            }
            fn visit_u64<E>(self, v: u64) -> Result<Loose, E> {
                Ok(i64::try_from(v).map_or(Loose::Other, Loose::Int))
            }
            fn visit_f64<E>(self, _: f64) -> Result<Loose, E> {
                Ok(Loose::Other)
            }
            fn visit_bool<E>(self, _: bool) -> Result<Loose, E> {
                Ok(Loose::Other)
            }
            fn visit_unit<E>(self) -> Result<Loose, E> {
                Ok(Loose::Other)
            }
            fn visit_str<E>(self, v: &str) -> Result<Loose, E> {
                Ok(Loose::Str(v.to_string()))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Loose, A::Error> {
                while a.next_element::<Loose>()?.is_some() {}
                Ok(Loose::Other)
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Loose, A::Error> {
                let mut m = Vec::new();
                while let Some(entry) = a.next_entry::<String, Loose>()? {
                    m.push(entry);
                }
                Ok(Loose::Map(m))
            }
        }
        d.deserialize_any(V)
    }
}

/// What `validateBuild` returns for a result.
pub fn validation(result: &Result<ValidBuild, Vec<BuildError>>) -> Validation {
    match result {
        Ok(b) => Validation::Ok {
            ok: true,
            game: GAME,
            rules_version: RULES_VERSION,
            levels: b.loadout.into(),
            behavior: (&b.behavior).into(),
            points: b.points(),
            params: b.loadout.params(),
        },
        Err(errors) => Validation::Err {
            ok: false,
            errors: errors.clone(),
        },
    }
}

/// The same level and budget check for a tank from a URL query (`kiter-5-3-1`), so
/// a link can't start a match a build couldn't.
pub fn validate_spec(spec: &TankSpec) -> Result<ValidBuild, Vec<BuildError>> {
    let loadout = check_levels(spec.loadout.levels().map(|l| Some(l as i64)))?;
    Ok(ValidBuild {
        loadout,
        behavior: BuildBehavior::Scripted(spec.behavior),
    })
}

/// For a raw `MatchConfig` (JS `WasmMatch.withConfig`): every tank with its own
/// `params` must play exactly a valid build on top of the shared params
/// ([`Loadout::apply`]) or exactly the shared params, so a config can't hand one tank
/// more than the budget. Tanks without their own params play the shared set, which
/// they all share.
pub fn check_config(config: &MatchConfig) -> Result<(), String> {
    for (i, spawn) in config.tanks.iter().enumerate() {
        if spawn.params.is_none() {
            continue;
        }
        let params = config.tank_params(i);
        // Its own copy of the shared set is the same as none.
        let is_build = params == config.params
            || Loadout::ALL.iter().any(|l| {
                check_levels(l.levels().map(|v| Some(v as i64))).is_ok()
                    && l.apply(&config.params) == params
            });
        if !is_build {
            return Err(format!(
                "tanks[{i}].params is not a valid {BUDGET}-point build on the shared params"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn vb(game: &str, b: &Value) -> Result<ValidBuild, Vec<BuildError>> {
        validate_build(game, &b.to_string())
    }

    fn v(x: impl Serialize) -> Value {
        serde_json::to_value(x).unwrap()
    }

    fn build(levels: Value, behavior: Value) -> Value {
        json!({ "rules_version": 1, "levels": levels, "behavior": behavior })
    }
    fn scripted(id: &str) -> Value {
        json!({ "kind": "scripted", "id": id })
    }
    fn codes(r: Result<ValidBuild, Vec<BuildError>>) -> Vec<(&'static str, String)> {
        r.unwrap_err()
            .into_iter()
            .map(|e| (e.code, e.key))
            .collect()
    }

    #[test]
    fn exactly_19_builds_fit_the_budget() {
        let mut valid = Vec::new();
        for a in 0..=6 {
            for s in 0..=6 {
                for d in 0..=6 {
                    let b = build(
                        json!({"attack": a, "speed": s, "defense": d}),
                        scripted("kiter"),
                    );
                    if let Ok(v) = vb(GAME, &b) {
                        assert_eq!(v.points(), 9);
                        valid.push(v.loadout);
                    }
                }
            }
        }
        assert_eq!(valid.len(), 19);
        assert_eq!(valid, Loadout::ALL.to_vec());
    }

    #[test]
    fn catalog_comes_from_the_tables() {
        let c = v(catalog());
        assert_eq!(
            (
                c["game"].as_str(),
                c["rules_version"].as_u64(),
                c["budget"].as_u64()
            ),
            (Some("tank"), Some(1), Some(9))
        );
        let keys: Vec<&str> = c["stats"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["key"].as_str().unwrap())
            .collect();
        assert_eq!(keys, STAT_KEYS);
        let speed = &c["stats"][1];
        assert_eq!(
            (
                speed["label"].as_str(),
                speed["min"].as_u64(),
                speed["max"].as_u64(),
                speed["cost_per_level"].as_u64()
            ),
            (Some("Speed"), Some(1), Some(5), Some(1))
        );
        for i in 0..5 {
            assert_eq!(c["stats"][0]["values"][i]["damage"], json!(DAMAGE[i]));
            assert_eq!(
                c["stats"][1]["values"][i]["fire_cooldown"],
                json!(FIRE_COOLDOWN[i])
            );
            assert_eq!(c["stats"][1]["values"][i]["max_speed"], json!(MAX_SPEED[i]));
            assert_eq!(c["stats"][2]["values"][i]["max_hp"], json!(MAX_HP[i]));
            assert_eq!(c["stats"][2]["values"][i]["level"], json!(i + 1));
        }
        assert_eq!(c["behaviors"], json!(["charger", "kiter", "sniper"]));
        assert_eq!(
            c["presets"][1],
            json!({"id": "glass_cannon", "label": "Glass Cannon", "levels": {"attack": 5, "speed": 3, "defense": 1}})
        );
        assert_eq!(
            c["presets"][3],
            json!({"id": "scout", "label": "Scout", "levels": {"attack": 2, "speed": 5, "defense": 2}})
        );
        for p in c["presets"].as_array().unwrap() {
            assert!(vb(GAME, &build(p["levels"].clone(), scripted("sniper"))).is_ok());
        }
        assert_eq!(c["default_build"], v(default_build()));
        assert_eq!(v(games()), json!([{"game": "tank", "rules_version": 1}]));
    }

    #[test]
    fn default_build_is_valid_and_normalizes() {
        let b = vb(GAME, &v(default_build())).unwrap();
        assert_eq!(b.loadout, Loadout::DEFAULT);
        let j = v(validation(&Ok(b.clone())));
        assert_eq!(j["points"], json!(9));
        assert_eq!(j["behavior"], json!({"kind": "scripted", "id": "charger"}));
        assert_eq!(j["params"]["max_hp"], json!(650));
        let gc = vb(
            GAME,
            &build(
                json!({"attack": 5, "speed": 3, "defense": 1}),
                json!({"kind": "champion", "ref": "tank/nightly/gen-99"}),
            ),
        )
        .unwrap();
        assert_eq!(
            gc.behavior,
            BuildBehavior::Champion("tank/nightly/gen-99".into())
        );
        assert_eq!(
            serde_json::from_value::<engine::TankParams>(
                v(validation(&Ok(gc.clone())))["params"].clone()
            )
            .unwrap(),
            Loadout::new(5, 3, 1).unwrap().params()
        );
    }

    #[test]
    fn every_error_code_names_its_key() {
        let ok_levels = json!({"attack": 3, "speed": 3, "defense": 3});
        assert_eq!(
            codes(vb("racing", &v(default_build()))),
            [("wrong_game", "game".into())]
        );
        let mut b = v(default_build());
        b["game"] = json!("racing");
        assert_eq!(codes(vb(GAME, &b)), [("wrong_game", "game".into())]);
        let mut b = v(default_build());
        b["rules_version"] = json!(2);
        assert_eq!(
            codes(vb(GAME, &b)),
            [("rules_version_mismatch", "rules_version".into())]
        );
        b.as_object_mut().unwrap().remove("rules_version");
        assert_eq!(
            codes(vb(GAME, &b)),
            [("rules_version_mismatch", "rules_version".into())]
        );
        assert_eq!(
            codes(vb(
                GAME,
                &build(
                    json!({"attack": 6, "speed": 2, "defense": 1}),
                    scripted("kiter")
                )
            )),
            [("out_of_range", "attack".into())]
        );
        assert_eq!(
            codes(vb(
                GAME,
                &build(
                    json!({"attack": 4, "speed": 0, "defense": 2.5}),
                    scripted("kiter")
                )
            )),
            [
                ("out_of_range", "speed".into()),
                ("out_of_range", "defense".into())
            ]
        );
        assert_eq!(
            codes(vb(
                GAME,
                &build(json!({"attack": 4, "speed": 4}), scripted("kiter"))
            )),
            [("out_of_range", "defense".into())]
        );
        assert_eq!(
            codes(vb(
                GAME,
                &build(
                    json!({"attack": 5, "speed": 3, "defense": 2}),
                    scripted("kiter")
                )
            )),
            [("over_budget", "levels".into())]
        );
        assert_eq!(
            codes(vb(
                GAME,
                &build(
                    json!({"attack": 2, "speed": 2, "defense": 2}),
                    scripted("kiter")
                )
            )),
            [("under_budget", "levels".into())]
        );
        assert_eq!(
            codes(vb(
                GAME,
                &build(
                    json!({"attack": 3, "speed": 3, "defense": 3, "luck": 1}),
                    scripted("kiter")
                )
            )),
            [("unknown_key", "luck".into())]
        );
        for bad in [
            scripted("rammer"),
            scripted("Kiter"),
            json!({"kind": "champion"}),
            json!({"kind": "champion", "ref": ""}),
            json!({"kind": "learned", "id": "kiter"}),
            json!("kiter"),
        ] {
            assert_eq!(
                codes(vb(GAME, &build(ok_levels.clone(), bad.clone()))),
                [("unknown_behavior", "behavior".into())],
                "{bad}"
            );
        }
        assert_eq!(
            codes(validate_build(GAME, "{not json")),
            [("invalid_json", String::new())]
        );
        assert_eq!(
            codes(vb(GAME, &json!([1, 2]))),
            [("invalid_json", String::new())]
        );
        // Several problems at once are all reported.
        assert_eq!(
            codes(vb(
                GAME,
                &build(
                    json!({"attack": 9, "speed": 3, "defense": 3, "x": 1}),
                    scripted("nope")
                )
            ))
            .len(),
            3
        );
        let r = v(validation(&vb(
            GAME,
            &build(
                json!({"attack": 6, "speed": 2, "defense": 1}),
                scripted("kiter"),
            ),
        )));
        assert_eq!(
            r,
            json!({"ok": false, "errors": [{"code": "out_of_range", "key": "attack"}]})
        );
        // A look (or any other extra field) is ignored, never read.
        let mut b = v(default_build());
        b["look"] = json!({"hair_color": "#D9534F"});
        assert!(vb(GAME, &b).is_ok());
    }

    #[test]
    fn query_specs_and_configs_share_the_check() {
        for l in Loadout::ALL {
            let s = TankSpec::new(Behavior::Kiter, l);
            assert_eq!(validate_spec(&s).unwrap().loadout, l);
        }
        let setup = crate::MatchSpec::from_query("seed=1&blue=kiter-5-3-1&orange=charger-4-1-4")
            .unwrap()
            .setup();
        assert!(check_config(&setup.config).is_ok());
        assert!(check_config(&MatchConfig::duel()).is_ok());
        let mut tampered = setup.config.clone();
        tampered.tanks[0].params.as_mut().unwrap().max_hp = 940; // 5/3/5 in effect
        assert!(check_config(&tampered)
            .unwrap_err()
            .starts_with("tanks[0].params"));
        let mut odd = MatchConfig::duel();
        odd.tanks[1].params = Some(engine::TankParams {
            max_hp: 140,
            ..Default::default()
        });
        assert!(check_config(&odd).is_err());
        odd.tanks[1].params = Some(odd.params.clone());
        assert!(check_config(&odd).is_ok());
    }
}
