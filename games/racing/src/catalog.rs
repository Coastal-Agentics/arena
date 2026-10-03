//! Racing's build catalog and validator, in the shared [`game_catalog`] shape (as
//! `tank::catalog`): what the Customizer shows (`catalogJson("racing")`) and what every
//! race started from a build is checked against.
//!
//! Every number comes from [`crate::setup`]'s tables; `catalogJson` ships the committed
//! `games/racing/catalog.json`, which a test keeps equal to [`catalog`]. Build JSON
//! ([`validate_build`]) and raw configs ([`check_config`]) obey one level and budget
//! rule ([`game_catalog::check_levels`] on [`RULES`]); `Setup::new` accepts exactly
//! those levels (tested), so a config of the 19 setups needs no second check.
//!
//! The behaviors are the R2 scripted drivers ([`crate::drivers::Behavior`]):
//! `follower`, `cutter`, `blocker`.

use crate::config::{CarParams, RacingConfig};
use crate::drivers::Behavior;
use crate::setup::{Setup, BUDGET, GRIP, MAX_LEVEL, MIN_LEVEL, POWER, STAT_KEYS, TOP_SPEED};
pub use crate::{GAME, RULES_VERSION};
pub use game_catalog::{
    BehaviorRef, Build, BuildError, Catalog, LevelValues, Levels, Num, Preset, Rules, Stat,
    StatRule, Valid,
};

/// What a valid racing build resolves to: the car's params.
pub type RaceParams = CarParams;

/// Points one level of any stat costs.
pub const COST_PER_LEVEL: u8 = 1;
/// Scripted driver ids, in catalog order ([`Behavior::ALL`]).
pub const BEHAVIORS: [&str; 3] = [
    Behavior::ALL[0].key(),
    Behavior::ALL[1].key(),
    Behavior::ALL[2].key(),
];

const fn stat_rule(key: &'static str) -> StatRule {
    StatRule {
        key,
        min: MIN_LEVEL,
        max: MAX_LEVEL,
        cost_per_level: COST_PER_LEVEL,
    }
}

/// What a racing build is checked against: the stat keys, levels 1–5 at one point
/// each, exactly [`BUDGET`] points, and the scripted driver ids.
pub const RULES: Rules = Rules {
    game: GAME,
    rules_version: RULES_VERSION,
    budget: BUDGET as u32,
    stats: &[
        stat_rule(STAT_KEYS[0]),
        stat_rule(STAT_KEYS[1]),
        stat_rule(STAT_KEYS[2]),
    ],
    behaviors: &BEHAVIORS,
};

/// `catalogJson("racing")`: [`catalog`] as JSON, committed as
/// `games/racing/catalog.json` so wasm carries a string instead of the code that
/// writes it. A test keeps the file equal to `serde_json::to_string(&catalog())`;
/// regenerate it with `cargo run -q -p racing --example catalog_json > games/racing/catalog.json`.
pub const CATALOG_JSON: &str = include_str!("../catalog.json");

/// `defaultBuild("racing")`: [`default_build`] as JSON (a test keeps them equal).
pub const DEFAULT_BUILD_JSON: &str = r#"{"rules_version":1,"levels":{"power":3,"top_speed":3,"grip":3},"behavior":{"kind":"scripted","id":"follower"}}"#;
const STAT_LABELS: [&str; 3] = ["Power", "Top speed", "Grip"];
/// Presets: (id, label, setup).
pub const PRESETS: [(&str, &str, Setup); 4] = [
    ("balanced", "Balanced", Setup::BALANCED),
    (
        "sprinter",
        "Sprinter",
        Setup {
            power: 5,
            top_speed: 2,
            grip: 2,
        },
    ),
    (
        "speedster",
        "Speedster",
        Setup {
            power: 2,
            top_speed: 5,
            grip: 2,
        },
    ),
    (
        "carver",
        "Carver",
        Setup {
            power: 2,
            top_speed: 2,
            grip: 5,
        },
    ),
];

/// A setup's levels by stat key.
pub fn levels(s: Setup) -> Levels {
    Levels(
        STAT_KEYS
            .into_iter()
            .zip([s.power, s.top_speed, s.grip])
            .collect(),
    )
}

/// The default build: 3/3/3 with the first scripted driver (`follower`).
pub fn default_build() -> Build {
    Build {
        rules_version: RULES_VERSION,
        levels: levels(Setup::BALANCED),
        behavior: BehaviorRef::Scripted { id: BEHAVIORS[0] },
    }
}

/// `catalogJson("racing")`: budget 9, the three stats with their per-level values
/// (acceleration u/s², top speed u/s, grip share per tick), the drivers, presets and
/// the default build, all from the setup tables.
pub fn catalog() -> Catalog {
    let tables: [(&'static str, &[f32; 5]); 3] = [
        ("acceleration", &POWER),
        ("top_speed", &TOP_SPEED),
        ("grip", &GRIP),
    ];
    let stats = (0..3)
        .map(|s| Stat {
            key: STAT_KEYS[s],
            label: STAT_LABELS[s],
            min: MIN_LEVEL,
            max: MAX_LEVEL,
            cost_per_level: COST_PER_LEVEL,
            values: (MIN_LEVEL..=MAX_LEVEL)
                .map(|level| LevelValues {
                    level,
                    values: vec![(tables[s].0, tables[s].1[level as usize - 1].into())],
                })
                .collect(),
        })
        .collect();
    Catalog {
        game: GAME,
        rules_version: RULES_VERSION,
        budget: BUDGET as u32,
        stats,
        behaviors: BEHAVIORS.to_vec(),
        presets: PRESETS
            .iter()
            .map(|&(id, label, s)| Preset {
                id: id.to_string(),
                label,
                levels: levels(s),
            })
            .collect(),
        default_build: default_build(),
    }
}

/// `validateBuild("racing", json)`: [`game_catalog::validate`] against [`RULES`],
/// with the car's resolved params ([`Setup::params`]).
pub fn validate_build(json: &str) -> Result<Valid<RaceParams>, Vec<BuildError>> {
    let build = game_catalog::validate(&RULES, json)?;
    let params = setup(&build.levels).params();
    Ok(Valid {
        game: GAME,
        points: build.points(&RULES),
        build,
        params,
    })
}

/// The setup of levels that passed [`game_catalog::check_levels`] on [`RULES`].
pub fn setup(levels: &Levels) -> Setup {
    let [p, t, g] = STAT_KEYS.map(|k| levels.get(k).unwrap_or(0));
    match Setup::new(p, t, g) {
        Ok(s) => s,
        Err(_) => unreachable!("checked levels are a setup"),
    }
}

/// The setup whose params these are, if any (exact match against the 19).
pub fn setup_of(params: &CarParams) -> Option<Setup> {
    Setup::all().into_iter().find(|s| s.params() == *params)
}

/// For a raw [`RacingConfig`] (a future `WasmRace.withConfig`): every car must play
/// exactly one of the 19 setups (`Setup::all`, which are exactly the builds [`RULES`]
/// allows), so a config can't hand a car more than the budget.
pub fn check_config(config: &RacingConfig) -> Result<(), String> {
    for (i, c) in config.cars.iter().enumerate() {
        if setup_of(c).is_none() {
            return Err(format!("cars[{i}] is not a valid {BUDGET}-point setup"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn vb(b: &Value) -> Result<Valid<RaceParams>, Vec<BuildError>> {
        validate_build(&b.to_string())
    }
    fn v(x: impl serde::Serialize) -> Value {
        serde_json::to_value(x).unwrap()
    }
    fn build(levels: Value, behavior: Value) -> Value {
        json!({ "rules_version": 1, "levels": levels, "behavior": behavior })
    }
    fn scripted(id: &str) -> Value {
        json!({ "kind": "scripted", "id": id })
    }
    fn codes<T: std::fmt::Debug>(r: Result<T, Vec<BuildError>>) -> Vec<(&'static str, String)> {
        r.unwrap_err()
            .into_iter()
            .map(|e| (e.code, e.key))
            .collect()
    }

    #[test]
    fn exactly_19_builds_fit_the_budget() {
        let mut valid = Vec::new();
        for p in 0..=6 {
            for t in 0..=6 {
                for g in 0..=6 {
                    let b = build(
                        json!({"power": p, "top_speed": t, "grip": g}),
                        scripted("cutter"),
                    );
                    if let Ok(ok) = vb(&b) {
                        assert_eq!(ok.points, 9);
                        let s = setup(&ok.build.levels);
                        assert_eq!(ok.params, s.params());
                        valid.push(s);
                    }
                }
            }
        }
        assert_eq!(valid.len(), 19);
        assert_eq!(valid, Setup::all());
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
            (Some("racing"), Some(1), Some(9))
        );
        let keys: Vec<&str> = c["stats"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["key"].as_str().unwrap())
            .collect();
        assert_eq!(keys, STAT_KEYS);
        assert_eq!(keys, ["power", "top_speed", "grip"]);
        for (i, label) in STAT_LABELS.iter().enumerate() {
            let s = &c["stats"][i];
            assert_eq!(
                (
                    s["label"].as_str(),
                    s["min"].as_u64(),
                    s["max"].as_u64(),
                    s["cost_per_level"].as_u64()
                ),
                (Some(*label), Some(1), Some(5), Some(1))
            );
        }
        for i in 0..5 {
            assert_eq!(
                c["stats"][0]["values"][i],
                json!({"level": i + 1, "acceleration": POWER[i]})
            );
            assert_eq!(
                c["stats"][1]["values"][i],
                json!({"level": i + 1, "top_speed": TOP_SPEED[i]})
            );
            assert_eq!(
                c["stats"][2]["values"][i],
                json!({"level": i + 1, "grip": GRIP[i]})
            );
        }
        assert_eq!(c["behaviors"], json!(["follower", "cutter", "blocker"]));
        assert_eq!(
            c["presets"][2],
            json!({"id": "speedster", "label": "Speedster", "levels": {"power": 2, "top_speed": 5, "grip": 2}})
        );
        for p in c["presets"].as_array().unwrap() {
            assert!(vb(&build(p["levels"].clone(), scripted("blocker"))).is_ok());
        }
        assert_eq!(c["default_build"], v(default_build()));
        assert_eq!(
            serde_json::to_string(&default_build()).unwrap(),
            r#"{"rules_version":1,"levels":{"power":3,"top_speed":3,"grip":3},"behavior":{"kind":"scripted","id":"follower"}}"#
        );
    }

    #[test]
    fn default_build_is_valid_and_normalizes() {
        let ok = vb(&v(default_build())).unwrap();
        assert_eq!(ok.build, default_build());
        let j: Value = serde_json::from_str(&game_catalog::validation_json(&Ok(ok))).unwrap();
        assert_eq!(j["ok"], json!(true));
        assert_eq!(j["game"], json!("racing"));
        assert_eq!(j["points"], json!(9));
        assert_eq!(j["behavior"], json!({"kind": "scripted", "id": "follower"}));
        assert_eq!(
            j["params"],
            json!({"power": 240.0, "top_speed": 240.0, "grip": 0.25})
        );
        let champ = vb(&build(
            json!({"power": 2, "top_speed": 5, "grip": 2}),
            json!({"kind": "champion", "ref": "racing/nightly/gen-9"}),
        ))
        .unwrap();
        assert_eq!(champ.scripted(), None);
        assert_eq!(champ.params, Setup::new(2, 5, 2).unwrap().params());
        let cutter = vb(&build(
            json!({"grip": 1, "power": 4, "top_speed": 4}),
            scripted("cutter"),
        ))
        .unwrap();
        assert_eq!(cutter.scripted(), Some("cutter"));
        // Normalized: levels come back in stat order.
        assert_eq!(
            serde_json::to_string(&cutter.build.levels).unwrap(),
            r#"{"power":4,"top_speed":4,"grip":1}"#
        );
    }

    #[test]
    fn every_error_code_names_its_key() {
        let ok_levels = json!({"power": 3, "top_speed": 3, "grip": 3});
        let mut b = v(default_build());
        b["game"] = json!("tank");
        assert_eq!(codes(vb(&b)), [("wrong_game", "game".into())]);
        let mut b = v(default_build());
        b["rules_version"] = json!(2);
        assert_eq!(
            codes(vb(&b)),
            [("rules_version_mismatch", "rules_version".into())]
        );
        b.as_object_mut().unwrap().remove("rules_version");
        assert_eq!(
            codes(vb(&b)),
            [("rules_version_mismatch", "rules_version".into())]
        );
        let lv = |l: Value| codes(vb(&build(l, scripted("follower"))));
        assert_eq!(
            lv(json!({"power": 6, "top_speed": 2, "grip": 1})),
            [("out_of_range", "power".into())]
        );
        assert_eq!(
            lv(json!({"power": 4, "top_speed": 0, "grip": 2.5})),
            [
                ("out_of_range", "top_speed".into()),
                ("out_of_range", "grip".into())
            ]
        );
        assert_eq!(
            lv(json!({"power": 4, "top_speed": 4})),
            [("out_of_range", "grip".into())]
        );
        assert_eq!(
            lv(json!({"power": "3", "top_speed": 3, "grip": 3})),
            [("out_of_range", "power".into())]
        );
        assert_eq!(
            lv(json!({"power": 5, "top_speed": 3, "grip": 2})),
            [("over_budget", "levels".into())]
        );
        assert_eq!(
            lv(json!({"power": 2, "top_speed": 2, "grip": 2})),
            [("under_budget", "levels".into())]
        );
        assert_eq!(
            lv(json!({"power": 3, "top_speed": 3, "grip": 3, "nitro": 1})),
            [("unknown_key", "nitro".into())]
        );
        for bad in [
            scripted("rammer"),
            scripted("Cutter"),
            scripted("kiter"),
            json!({"kind": "champion"}),
            json!({"kind": "champion", "ref": ""}),
            json!({"kind": "learned", "id": "cutter"}),
            json!("cutter"),
        ] {
            assert_eq!(
                codes(vb(&build(ok_levels.clone(), bad.clone()))),
                [("unknown_behavior", "behavior".into())],
                "{bad}"
            );
        }
        assert_eq!(
            codes(validate_build("{not json")),
            [("invalid_json", String::new())]
        );
        assert_eq!(codes(vb(&json!([1, 2]))), [("invalid_json", String::new())]);
        // Several problems at once are all reported.
        let many = build(
            json!({"power": 9, "top_speed": 3, "grip": 3, "x": 1}),
            scripted("nope"),
        );
        assert_eq!(codes(vb(&many)).len(), 3);
        let r = validate_build(
            &build(
                json!({"power": 6, "top_speed": 2, "grip": 1}),
                scripted("cutter"),
            )
            .to_string(),
        );
        assert_eq!(
            game_catalog::validation_json(&r),
            r#"{"ok":false,"errors":[{"code":"out_of_range","key":"power"}]}"#
        );
        // A look (or any other extra field) is ignored, never read.
        let mut b = v(default_build());
        b["look"] = json!({"hair_color": "#D9534F"});
        b["game"] = json!("racing");
        assert!(vb(&b).is_ok());
    }

    #[test]
    fn configs_share_the_check() {
        let all = Setup::all();
        assert!(check_config(&RacingConfig::ring_setups(&all[..4])).is_ok());
        // Duplicate Nyborgs (same setup) may race each other.
        assert!(check_config(&RacingConfig::ring_setups(&[Setup::BALANCED; 4])).is_ok());
        for s in &all {
            assert_eq!(setup_of(&s.params()), Some(*s));
        }
        let mut tampered = RacingConfig::ring_setups(&[Setup::BALANCED; 2]);
        tampered.cars[1].top_speed = TOP_SPEED[4]; // 3/5/3 in effect
        assert!(check_config(&tampered).unwrap_err().starts_with("cars[1]"));
    }

    #[test]
    fn committed_json_is_current() {
        assert_eq!(
            CATALOG_JSON,
            serde_json::to_string(&catalog()).unwrap(),
            "games/racing/catalog.json is stale; regenerate it with \
             `cargo run -q -p racing --example catalog_json > games/racing/catalog.json`"
        );
        assert_eq!(
            DEFAULT_BUILD_JSON,
            serde_json::to_string(&default_build()).unwrap()
        );
        assert!(catalog().matches(&RULES));
    }

    /// `Setup::new` (so `Setup::all` and every config of setups) accepts exactly the
    /// levels [`RULES`] does.
    #[test]
    fn setups_are_exactly_the_valid_builds() {
        for p in 0..=6u8 {
            for t in 0..=6u8 {
                for g in 0..=6u8 {
                    let wanted = [p, t, g].map(|v| Some(v as i64));
                    assert_eq!(
                        Setup::new(p, t, g).is_ok(),
                        game_catalog::check_levels(&RULES, &wanted).is_ok(),
                        "{p}-{t}-{g}"
                    );
                }
            }
        }
    }
}
