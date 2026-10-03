//! `tests/fixtures/determinism.json`: native final hashes for the episodes the Python
//! tests replay (`python/tests/test_determinism.py`), so the two languages are pinned
//! to the same numbers. Regenerate with `ENGINE_PY_WRITE_FIXTURE=1 cargo test -p
//! engine-py --test fixture` (only when a game's rules change on purpose).

use engine::TankRules;
use engine_py::{reference_episode, Game};
use racing::RacingRules;
use serde_json::{json, Value};

const PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/determinism.json"
);

fn case<R: Game>(c: &Value) -> Value
where
    R::Config: Clone,
{
    let builds: Vec<String> = c["builds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b.to_string())
        .collect();
    let refs: Vec<&str> = builds.iter().map(String::as_str).collect();
    let learning: Vec<usize> = c["learning"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as usize)
        .collect();
    let max_steps = c["max_steps"].as_u64().map_or(u32::MAX, |v| v as u32);
    let m = reference_episode(
        &R::lineup(&refs).unwrap(),
        &learning,
        c["frame_skip"].as_u64().unwrap() as u32,
        c["seed"].as_u64().unwrap(),
        max_steps,
    );
    let mut out = c.clone();
    out["final_hash"] = json!(format!("{:016x}", m.state_hash()));
    out["ticks"] = json!(m.tick());
    out["outcome"] = serde_json::to_value(m.outcome()).unwrap();
    out
}

#[test]
fn native_hashes_match_the_fixture() {
    let text = std::fs::read_to_string(PATH).unwrap();
    let fixture: Value = serde_json::from_str(&text).unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert!(cases.len() >= 8);
    let got: Vec<Value> = cases
        .iter()
        .map(|c| match c["game"].as_str().unwrap() {
            "tank" => case::<TankRules>(c),
            "racing" => case::<RacingRules>(c),
            g => panic!("unknown game {g}"),
        })
        .collect();
    if std::env::var_os("ENGINE_PY_WRITE_FIXTURE").is_some() {
        let mut s = serde_json::to_string_pretty(&json!({ "note": fixture["note"], "cases": got }))
            .unwrap();
        s.push('\n');
        std::fs::write(PATH, s).unwrap();
        return;
    }
    for (c, g) in cases.iter().zip(&got) {
        assert_eq!(c, g, "fixture case {}", c["name"]);
    }
}
