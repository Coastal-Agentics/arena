//! The build catalog every game fills in, and the one build validator (Nyborg builds;
//! game-system.md §4, nyborg-library.md §4–5).
//!
//! A **build** is the per-game part of a Nyborg: levels per stat plus a behavior, never
//! resolved numbers and never its look.
//!
//! ```json
//! {"rules_version": 1,
//!  "levels": {"attack": 5, "speed": 3, "defense": 1},
//!  "behavior": {"kind": "scripted", "id": "kiter"}}
//! ```
//!
//! `behavior` can also be `{"kind": "champion", "ref": "tank/nightly/gen-99"}`; only its
//! shape is checked here, because the loader resolves the ref.
//!
//! # What a game crate provides
//!
//! Plain data and two functions, no traits (Tank Arena's are in `tank::catalog`):
//!
//! ```ignore
//! pub const GAME: &str = "racing";
//! pub const RULES_VERSION: u64 = 1;
//!
//! /// Built from the game's own level tables, so no number is written twice.
//! pub fn catalog() -> game_catalog::Catalog;
//!
//! /// `game_catalog::validate(&catalog(), json)`, then the game's own resolved
//! /// params for the levels (anything `Serialize`).
//! pub fn validate_build(json: &str) -> Result<game_catalog::Valid<Params>, Vec<game_catalog::BuildError>>;
//! ```
//!
//! `engine-wasm` lists each game once (id, `catalog`, and `validate_build` through
//! [`validation_json`]) and dispatches on the game id. This crate and the game crates
//! stay out of `engine/`, which is game-agnostic.

use serde::ser::{SerializeMap, Serializer};
use serde::Serialize;

/// A game's build catalog: what the Customizer shows (`catalogJson(game)`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Catalog {
    /// Game id (`"tank"`).
    pub game: &'static str,
    /// Version of the rules the levels refer to. Bump it whenever a level's meaning
    /// changes (a table, the budget or the stat keys); builds saved with another
    /// version then fail with `rules_version_mismatch`.
    pub rules_version: u64,
    /// Points every build must spend, exactly.
    pub budget: u32,
    /// The stats, in display order (also the order of [`Levels`]).
    pub stats: Vec<Stat>,
    /// Scripted behavior ids, as a build's `{"kind": "scripted", "id"}` names them.
    pub behaviors: Vec<&'static str>,
    /// Ready-made level sets.
    pub presets: Vec<Preset>,
    /// A valid build (a new Nyborg's, or one whose saved build is gone).
    pub default_build: Build,
}

/// One stat.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Stat {
    /// Key in a build's `levels` (`"attack"`).
    pub key: &'static str,
    /// Customizer label (`"Attack"`).
    pub label: &'static str,
    /// Lowest level (every stat gets at least this).
    pub min: u8,
    /// Highest level.
    pub max: u8,
    /// Points one level costs.
    pub cost_per_level: u8,
    /// What each level resolves to: `values[i]` is level `min + i`.
    pub values: Vec<LevelValues>,
}

/// What one level of a stat resolves to, as `{"level": 1, "<name>": value, ...}`.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelValues {
    /// The level.
    pub level: u8,
    /// Named values in display order (`("max_hp", 460.into())`).
    pub values: Vec<(&'static str, Num)>,
}

impl Serialize for LevelValues {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(self.values.len() + 1))?;
        m.serialize_entry("level", &self.level)?;
        for (k, v) in &self.values {
            m.serialize_entry(k, v)?;
        }
        m.end()
    }
}

/// A table value: an integer or an `f32` (written as JSON numbers, `120.0` for floats).
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Num {
    /// An integer.
    Int(i64),
    /// A float.
    F32(f32),
}

macro_rules! num_from_int {
    ($($t:ty),*) => {$(
        impl From<$t> for Num {
            fn from(v: $t) -> Self {
                Num::Int(v as i64)
            }
        }
    )*};
}
num_from_int!(u8, u16, u32, i32, i64);

impl From<f32> for Num {
    fn from(v: f32) -> Self {
        Num::F32(v)
    }
}

/// One preset.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Preset {
    /// Snake-case id (`"glass_cannon"`).
    pub id: String,
    /// Label (`"Glass Cannon"`).
    pub label: &'static str,
    /// Its levels.
    pub levels: Levels,
}

/// Levels by stat key, in the catalog's stat order; written as a JSON object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Levels(pub Vec<(&'static str, u8)>);

impl Levels {
    /// The level for `key`, if it is a stat.
    pub fn get(&self, key: &str) -> Option<u8> {
        self.0.iter().find(|(k, _)| *k == key).map(|&(_, l)| l)
    }
}

impl Serialize for Levels {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            m.serialize_entry(k, v)?;
        }
        m.end()
    }
}

/// A build's behavior: `{"kind": "scripted", "id"}` or `{"kind": "champion", "ref"}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BehaviorRef {
    /// A scripted behavior, by one of the catalog's `behaviors`.
    Scripted {
        /// The id.
        id: &'static str,
    },
    /// A published champion, by ref (resolved by the loader, not here).
    Champion {
        /// The ref.
        #[serde(rename = "ref")]
        reference: String,
    },
}

/// A build: `{"rules_version", "levels", "behavior"}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Build {
    /// The rules version the levels refer to.
    pub rules_version: u64,
    /// The levels.
    pub levels: Levels,
    /// The behavior.
    pub behavior: BehaviorRef,
}

impl Build {
    /// Points spent under `catalog`'s costs.
    pub fn points(&self, catalog: &Catalog) -> u32 {
        catalog
            .stats
            .iter()
            .map(|s| self.levels.get(s.key).unwrap_or(0) as u32 * s.cost_per_level as u32)
            .sum()
    }
}

/// Why a build is invalid. Every error names the key it failed on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BuildError {
    /// `invalid_json`, `wrong_game`, `rules_version_mismatch`, `unknown_key`,
    /// `out_of_range`, `over_budget`, `under_budget` or `unknown_behavior`.
    pub code: &'static str,
    /// The key: a stat key, `levels` (budget), `behavior`, `rules_version`, `game`, or
    /// empty for `invalid_json`.
    pub key: String,
}

impl BuildError {
    /// An error with `code` on `key`.
    pub fn new(code: &'static str, key: &str) -> Self {
        Self {
            code,
            key: key.to_string(),
        }
    }
}

/// A valid build with the game's resolved params (`P`, e.g. `TankParams`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Valid<P> {
    /// Game id.
    pub game: &'static str,
    /// The normalized build (levels in stat order).
    #[serde(flatten)]
    pub build: Build,
    /// Points spent (always the budget).
    pub points: u32,
    /// What the agent plays with.
    pub params: P,
}

impl<P> Valid<P> {
    /// The scripted id, or `None` for a champion.
    pub fn scripted(&self) -> Option<&'static str> {
        match self.build.behavior {
            BehaviorRef::Scripted { id } => Some(id),
            BehaviorRef::Champion { .. } => None,
        }
    }
}

/// The JSON `validateBuild` returns: `{"ok": true, "game", "rules_version", "levels",
/// "behavior", "points", "params"}` or `{"ok": false, "errors": [{"code", "key"}, ...]}`.
pub fn validation_json<P: Serialize>(result: &Result<Valid<P>, Vec<BuildError>>) -> String {
    #[derive(Serialize)]
    #[serde(untagged)]
    enum View<'a, P> {
        Ok {
            ok: bool,
            #[serde(flatten)]
            valid: &'a Valid<P>,
        },
        Err {
            ok: bool,
            errors: &'a [BuildError],
        },
    }
    let view = match result {
        Ok(valid) => View::Ok { ok: true, valid },
        Err(errors) => View::Err { ok: false, errors },
    };
    serde_json::to_string(&view).expect("validation serializes")
}

/// The one level and budget check, for levels in `catalog`'s stat order: each an
/// integer in `min..=max` (`None`, a missing or non-integer level, is out of range),
/// then the points spent exactly the budget.
pub fn check_levels(catalog: &Catalog, levels: &[Option<i64>]) -> Result<Levels, Vec<BuildError>> {
    let mut errors = Vec::new();
    let mut out = Vec::with_capacity(catalog.stats.len());
    let mut spent = 0u32;
    for (i, stat) in catalog.stats.iter().enumerate() {
        match levels.get(i).copied().flatten() {
            Some(l) if (stat.min as i64..=stat.max as i64).contains(&l) => {
                out.push((stat.key, l as u8));
                spent += l as u32 * stat.cost_per_level as u32;
            }
            _ => errors.push(BuildError::new("out_of_range", stat.key)),
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    if spent != catalog.budget {
        let code = if spent > catalog.budget {
            "over_budget"
        } else {
            "under_budget"
        };
        return Err(vec![BuildError::new(code, "levels")]);
    }
    Ok(Levels(out))
}

/// Check a build's JSON text against `catalog`.
///
/// Order: `invalid_json` (unparsable, or not an object), then `wrong_game` (a `game`
/// field that isn't the catalog's) and `rules_version_mismatch` stop the check, since
/// the levels mean nothing without them; otherwise every `unknown_key`,
/// `out_of_range`, budget and `unknown_behavior` error is reported. Unknown top-level
/// fields (a Nyborg's `look`) are skipped, never kept. Scripted ids must match one of
/// `catalog.behaviors` exactly; a champion `ref` must be a non-empty string of at most
/// 200 bytes.
pub fn validate(catalog: &Catalog, json: &str) -> Result<Build, Vec<BuildError>> {
    let Ok(build @ Loose::Map(_)) = serde_json::from_str::<Loose>(json) else {
        return Err(vec![BuildError::new("invalid_json", "")]);
    };
    if build
        .get("game")
        .is_some_and(|g| g.as_str() != Some(catalog.game))
    {
        return Err(vec![BuildError::new("wrong_game", "game")]);
    }
    if build.get("rules_version").and_then(Loose::as_i64) != Some(catalog.rules_version as i64) {
        return Err(vec![BuildError::new(
            "rules_version_mismatch",
            "rules_version",
        )]);
    }

    let mut errors = Vec::new();
    let levels = build.get("levels");
    if let Some(Loose::Map(levels)) = levels {
        for (key, _) in levels {
            if !catalog.stats.iter().any(|s| s.key == key) {
                errors.push(BuildError::new("unknown_key", key));
            }
        }
    }
    let wanted: Vec<Option<i64>> = catalog
        .stats
        .iter()
        .map(|s| levels.and_then(|l| l.get(s.key)).and_then(Loose::as_i64))
        .collect();
    let levels = check_levels(catalog, &wanted);
    if let Err(e) = &levels {
        errors.extend(e.iter().cloned());
    }

    let behavior = build.get("behavior").and_then(|b| {
        let field = |k| b.get(k).and_then(Loose::as_str);
        match field("kind")? {
            "scripted" => catalog
                .behaviors
                .iter()
                .find(|&&id| Some(id) == field("id"))
                .map(|&id| BehaviorRef::Scripted { id }),
            "champion" => field("ref")
                .filter(|r| !r.trim().is_empty() && r.len() <= 200)
                .map(|r| BehaviorRef::Champion {
                    reference: r.to_string(),
                }),
            _ => None,
        }
    });
    if behavior.is_none() {
        errors.push(BuildError::new("unknown_behavior", "behavior"));
    }

    match (levels, behavior) {
        (Ok(levels), Some(behavior)) if errors.is_empty() => Ok(Build {
            rules_version: catalog.rules_version,
            levels,
            behavior,
        }),
        _ => Err(errors),
    }
}

/// `{"ok": false, "errors": [{"code": "wrong_game", "key": "game"}]}`, for a game id
/// nobody registered.
pub fn unknown_game_json() -> String {
    validation_json::<()>(&Err(vec![BuildError::new("wrong_game", "game")]))
}

/// A parsed JSON value, kept only as far as [`validate`] reads it: integers, strings
/// and objects (in order; a repeated key's last value wins). Floats, booleans, null and
/// arrays are `Other`. Leaner in wasm than `serde_json::Value`.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A made-up two-stat game: shows a catalog needs nothing tank-specific.
    fn toy() -> Catalog {
        let stat = |key, label| Stat {
            key,
            label,
            min: 1,
            max: 3,
            cost_per_level: 2,
            values: (1..=3)
                .map(|level| LevelValues {
                    level,
                    values: vec![("x", Num::from(level as f32 / 2.0))],
                })
                .collect(),
        };
        Catalog {
            game: "toy",
            rules_version: 7,
            budget: 8,
            stats: vec![stat("grip", "Grip"), stat("power", "Power")],
            behaviors: vec!["follower", "cutter"],
            presets: vec![],
            default_build: Build {
                rules_version: 7,
                levels: Levels(vec![("grip", 2), ("power", 2)]),
                behavior: BehaviorRef::Scripted { id: "follower" },
            },
        }
    }

    fn codes(r: Result<Build, Vec<BuildError>>) -> Vec<(&'static str, String)> {
        r.unwrap_err()
            .into_iter()
            .map(|e| (e.code, e.key))
            .collect()
    }

    #[test]
    fn a_generic_catalog_validates_and_serializes() {
        let c = toy();
        let default = serde_json::to_string(&c.default_build).unwrap();
        assert_eq!(
            default,
            r#"{"rules_version":7,"levels":{"grip":2,"power":2},"behavior":{"kind":"scripted","id":"follower"}}"#
        );
        let b = validate(&c, &default).unwrap();
        assert_eq!((b.points(&c), &b), (8, &c.default_build));
        let s = serde_json::to_string(&c.stats[0].values[0]).unwrap();
        assert_eq!(s, r#"{"level":1,"x":0.5}"#);
        let v = Valid {
            game: c.game,
            build: b,
            points: 8,
            params: [1, 2],
        };
        assert_eq!(
            validation_json(&Ok(v)),
            r#"{"ok":true,"game":"toy","rules_version":7,"levels":{"grip":2,"power":2},"behavior":{"kind":"scripted","id":"follower"},"points":8,"params":[1,2]}"#
        );
        assert_eq!(
            unknown_game_json(),
            r#"{"ok":false,"errors":[{"code":"wrong_game","key":"game"}]}"#
        );
        let bad = r#"{"rules_version":7,"levels":{"grip":3,"power":3,"luck":1},"behavior":{"kind":"scripted","id":"blocker"}}"#;
        assert_eq!(
            codes(validate(&c, bad)),
            [
                ("unknown_key", "luck".into()),
                ("over_budget", "levels".into()),
                ("unknown_behavior", "behavior".into())
            ]
        );
        let champ = r#"{"game":"toy","rules_version":7,"levels":{"grip":1,"power":3},"behavior":{"kind":"champion","ref":"toy/x"}}"#;
        assert_eq!(
            validate(&c, champ).unwrap().behavior,
            BehaviorRef::Champion {
                reference: "toy/x".into()
            }
        );
        assert_eq!(
            codes(validate(&c, &champ.replace("\"toy\"", "\"tank\""))),
            [("wrong_game", "game".into())]
        );
    }
}
