//! Tank Arena's build catalog and validator, in the shared [`game_catalog`] shape:
//! what the Customizer shows (`catalogJson("tank")`) and what every match started from
//! a build is checked against (game-system.md §4; nyborg-library.md §4–5).
//!
//! Every number comes from [`crate::loadout`]'s tables, so the catalog can't drift from
//! the rules. Every path that starts a match from a build or config shares one level
//! and budget check ([`game_catalog::check_levels`] on [`catalog`]): [`validate_build`]
//! for build JSON, [`validate_spec`] for URL-query tanks and [`check_config`] for raw
//! configs, so an imported or edited build can't beat the budget.

use crate::loadout::{
    Loadout, Preset as LoadoutPreset, BUDGET, DAMAGE, FIRE_COOLDOWN, MAX_HP, MAX_LEVEL, MAX_SPEED,
    MIN_LEVEL, TURN_RATE,
};
use crate::matchup::TankSpec;
use crate::policies::Behavior;
use engine::{MatchConfig, TankParams};
pub use game_catalog::{
    BehaviorRef, Build, BuildError, Catalog, LevelValues, Levels, Num, Preset, Stat, Valid,
};

/// This game's id in `games()`, `catalogJson(game)` and a Nyborg's `builds`.
pub const GAME: &str = "tank";
/// Version of the tank rules a build's levels refer to. Bump it whenever a level's
/// meaning changes (a table, the budget or the stat keys).
pub const RULES_VERSION: u64 = 1;
/// Points one level of any stat costs.
pub const COST_PER_LEVEL: u8 = 1;
/// The stat keys, in catalog order: Attack, Speed, Defense.
pub const STAT_KEYS: [&str; 3] = ["attack", "speed", "defense"];
const STAT_LABELS: [&str; 3] = ["Attack", "Speed", "Defense"];

/// A loadout's levels by stat key.
pub fn levels(l: Loadout) -> Levels {
    Levels(STAT_KEYS.into_iter().zip(l.levels()).collect())
}

/// The default build: 3/3/3 with the first scripted behavior.
pub fn default_build() -> Build {
    Build {
        rules_version: RULES_VERSION,
        levels: levels(Loadout::DEFAULT),
        behavior: BehaviorRef::Scripted {
            id: Behavior::ALL[0].key(),
        },
    }
}

/// `catalogJson("tank")`: budget, stats with their per-level values (damage; top
/// speed, turn rate and reload ticks; max HP), scripted behaviors, presets and the
/// default build, all from the loadout tables.
pub fn catalog() -> Catalog {
    let per_level = |i: usize| -> Vec<(&'static str, Num)> {
        match i / 5 {
            0 => vec![("damage", DAMAGE[i % 5].into())],
            1 => vec![
                ("max_speed", MAX_SPEED[i % 5].into()),
                ("turn_rate", TURN_RATE[i % 5].into()),
                ("fire_cooldown", FIRE_COOLDOWN[i % 5].into()),
            ],
            _ => vec![("max_hp", MAX_HP[i % 5].into())],
        }
    };
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
                    values: per_level(s * 5 + level as usize - 1),
                })
                .collect(),
        })
        .collect();
    Catalog {
        game: GAME,
        rules_version: RULES_VERSION,
        budget: BUDGET as u32,
        stats,
        behaviors: Behavior::ALL.iter().map(|b| b.key()).collect(),
        presets: LoadoutPreset::ALL
            .iter()
            .map(|p| Preset {
                id: p.name().to_ascii_lowercase().replace(' ', "_"),
                label: p.name(),
                levels: levels(p.loadout()),
            })
            .collect(),
        default_build: default_build(),
    }
}

/// `validateBuild("tank", json)`: [`game_catalog::validate`] against [`catalog`], with
/// the resolved [`TankParams`] the tank plays with ([`Loadout::params`]).
pub fn validate_build(json: &str) -> Result<Valid<TankParams>, Vec<BuildError>> {
    let catalog = catalog();
    let build = game_catalog::validate(&catalog, json)?;
    let params = loadout(&build.levels).params();
    Ok(Valid {
        game: GAME,
        points: build.points(&catalog),
        build,
        params,
    })
}

/// The loadout of levels that passed [`game_catalog::check_levels`] on [`catalog`].
fn loadout(levels: &Levels) -> Loadout {
    let [a, s, d] = STAT_KEYS.map(|k| levels.get(k).unwrap_or(0));
    Loadout::new(a, s, d).expect("checked levels are a loadout")
}

/// A valid build's tank, or why it can't play yet (a champion needs its genome, which
/// the loader resolves).
pub fn tank_spec(valid: &Valid<TankParams>) -> Result<TankSpec, String> {
    let id = valid
        .scripted()
        .ok_or("a champion behavior needs its genome; the loader resolves it")?;
    let behavior = Behavior::ALL
        .into_iter()
        .find(|b| b.key() == id)
        .expect("validated scripted id");
    Ok(TankSpec::new(behavior, loadout(&valid.build.levels)))
}

/// The same level and budget check for a tank from a URL query (`kiter-5-3-1`), so a
/// link can't start a match a build couldn't.
pub fn validate_spec(spec: &TankSpec) -> Result<(), Vec<BuildError>> {
    let wanted = spec.loadout.levels().map(|l| Some(l as i64));
    game_catalog::check_levels(&catalog(), &wanted).map(|_| ())
}

/// For a raw `MatchConfig` (JS `WasmMatch.withConfig`): every tank with its own
/// `params` must play exactly a valid build on top of the shared params
/// ([`Loadout::apply`]) or exactly the shared params, so a config can't hand one tank
/// more than the budget. Tanks without their own params play the shared set, which
/// they all share.
pub fn check_config(config: &MatchConfig) -> Result<(), String> {
    let catalog = catalog();
    for (i, spawn) in config.tanks.iter().enumerate() {
        if spawn.params.is_none() {
            continue;
        }
        let params = config.tank_params(i);
        // Its own copy of the shared set is the same as none.
        let is_build = params == config.params
            || Loadout::ALL.iter().any(|l| {
                let wanted = l.levels().map(|v| Some(v as i64));
                game_catalog::check_levels(&catalog, &wanted).is_ok()
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

    fn vb(b: &Value) -> Result<Valid<TankParams>, Vec<BuildError>> {
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
        for a in 0..=6 {
            for s in 0..=6 {
                for d in 0..=6 {
                    let b = build(
                        json!({"attack": a, "speed": s, "defense": d}),
                        scripted("kiter"),
                    );
                    if let Ok(ok) = vb(&b) {
                        assert_eq!(ok.points, 9);
                        let l = loadout(&ok.build.levels);
                        assert_eq!(ok.params, l.params());
                        valid.push(l);
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
            assert_eq!(
                c["stats"][0]["values"][i],
                json!({"level": i + 1, "damage": DAMAGE[i]})
            );
            assert_eq!(
                c["stats"][1]["values"][i],
                json!({"level": i + 1, "max_speed": MAX_SPEED[i], "turn_rate": TURN_RATE[i], "fire_cooldown": FIRE_COOLDOWN[i]})
            );
            assert_eq!(
                c["stats"][2]["values"][i],
                json!({"level": i + 1, "max_hp": MAX_HP[i]})
            );
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
            assert!(vb(&build(p["levels"].clone(), scripted("sniper"))).is_ok());
        }
        assert_eq!(c["default_build"], v(default_build()));
    }

    #[test]
    fn default_build_is_valid_and_normalizes() {
        let ok = vb(&v(default_build())).unwrap();
        assert_eq!(ok.build, default_build());
        let j: Value = serde_json::from_str(&game_catalog::validation_json(&Ok(ok))).unwrap();
        assert_eq!(j["ok"], json!(true));
        assert_eq!(j["points"], json!(9));
        assert_eq!(j["behavior"], json!({"kind": "scripted", "id": "charger"}));
        assert_eq!(j["params"]["max_hp"], json!(650));
        let gc = vb(&build(
            json!({"attack": 5, "speed": 3, "defense": 1}),
            json!({"kind": "champion", "ref": "tank/nightly/gen-99"}),
        ))
        .unwrap();
        assert_eq!(
            gc.build.behavior,
            BehaviorRef::Champion {
                reference: "tank/nightly/gen-99".into()
            }
        );
        assert_eq!(gc.params, Loadout::new(5, 3, 1).unwrap().params());
        assert!(tank_spec(&gc).unwrap_err().contains("champion"));
        let kiter = vb(&build(
            json!({"defense": 1, "attack": 5, "speed": 3}),
            scripted("kiter"),
        ))
        .unwrap();
        assert_eq!(
            tank_spec(&kiter).unwrap(),
            TankSpec::new(Behavior::Kiter, Loadout::new(5, 3, 1).unwrap())
        );
        // Normalized: levels come back in stat order.
        assert_eq!(
            serde_json::to_string(&kiter.build.levels).unwrap(),
            r#"{"attack":5,"speed":3,"defense":1}"#
        );
    }

    #[test]
    fn every_error_code_names_its_key() {
        let ok_levels = json!({"attack": 3, "speed": 3, "defense": 3});
        let mut b = v(default_build());
        b["game"] = json!("racing");
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
        let lv = |l: Value| codes(vb(&build(l, scripted("kiter"))));
        assert_eq!(
            lv(json!({"attack": 6, "speed": 2, "defense": 1})),
            [("out_of_range", "attack".into())]
        );
        assert_eq!(
            lv(json!({"attack": 4, "speed": 0, "defense": 2.5})),
            [
                ("out_of_range", "speed".into()),
                ("out_of_range", "defense".into())
            ]
        );
        assert_eq!(
            lv(json!({"attack": 4, "speed": 4})),
            [("out_of_range", "defense".into())]
        );
        assert_eq!(
            lv(json!({"attack": "3", "speed": 3, "defense": 3})),
            [("out_of_range", "attack".into())]
        );
        assert_eq!(
            lv(json!({"attack": 5, "speed": 3, "defense": 2})),
            [("over_budget", "levels".into())]
        );
        assert_eq!(
            lv(json!({"attack": 2, "speed": 2, "defense": 2})),
            [("under_budget", "levels".into())]
        );
        assert_eq!(
            lv(json!({"attack": 3, "speed": 3, "defense": 3, "luck": 1})),
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
            json!({"attack": 9, "speed": 3, "defense": 3, "x": 1}),
            scripted("nope"),
        );
        assert_eq!(codes(vb(&many)).len(), 3);
        let r = validate_build(
            &build(
                json!({"attack": 6, "speed": 2, "defense": 1}),
                scripted("kiter"),
            )
            .to_string(),
        );
        assert_eq!(
            game_catalog::validation_json(&r),
            r#"{"ok":false,"errors":[{"code":"out_of_range","key":"attack"}]}"#
        );
        // A look (or any other extra field) is ignored, never read.
        let mut b = v(default_build());
        b["look"] = json!({"hair_color": "#D9534F"});
        b["game"] = json!("tank");
        assert!(vb(&b).is_ok());
    }

    #[test]
    fn query_specs_and_configs_share_the_check() {
        for l in Loadout::ALL {
            assert!(validate_spec(&TankSpec::new(Behavior::Kiter, l)).is_ok());
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
        odd.tanks[1].params = Some(TankParams {
            max_hp: 140,
            ..Default::default()
        });
        assert!(check_config(&odd).is_err());
        odd.tanks[1].params = Some(odd.params.clone());
        assert!(check_config(&odd).is_ok());
    }
}
