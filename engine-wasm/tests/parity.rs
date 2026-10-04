//! Native side of the native-vs-wasm parity check: every pinned replay in
//! `tests/parity/` must verify natively ([`engine::Replay::verify`]) and match
//! `tests/parity/manifest.json`. `scripts/check-parity.mjs` checks the same files and
//! manifest against the committed `web/pkg`. Regenerate with
//! `cargo run -p engine-wasm --example parity_fixtures` (only on a bump of the format
//! Tank Arena writes, `TankRules::WRITES_FORMAT`, or a deliberate rule change).

use engine::{Replay, Rules, TankRules};
use serde_json::Value;
use std::path::PathBuf;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/parity")
}

fn manifest() -> Vec<Value> {
    let text = std::fs::read_to_string(dir().join("manifest.json")).expect("read manifest");
    let v: Value = serde_json::from_str(&text).expect("manifest is JSON");
    v["fixtures"].as_array().expect("fixtures array").clone()
}

#[test]
fn every_fixture_verifies_and_matches_the_manifest() {
    let fixtures = manifest();
    assert!(!fixtures.is_empty());
    let mut failures = Vec::new();
    for f in &fixtures {
        let file = f["file"].as_str().expect("file");
        let json = std::fs::read_to_string(dir().join(file)).expect("read fixture");
        let r = Replay::from_json(&json).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert_eq!(
            r.format,
            TankRules::WRITES_FORMAT,
            "{file}: pinned fixtures use the format Tank Arena writes"
        );
        let m = match r.verify() {
            Ok(m) => m,
            Err(e) => {
                failures.push(format!("{file}: Replay::verify: {e}"));
                continue;
            }
        };
        // The same summary the wasm side computes (engine_wasm::check_replay), compared
        // field by field with the manifest.
        let got = serde_json::to_value(engine_wasm::check_replay(&json).unwrap()).unwrap();
        assert_eq!(got["ticks"], m.tick());
        for key in [
            "format",
            "seed",
            "tanks",
            "ticks",
            "outcome",
            "final_hash",
            "setup_hash",
        ] {
            if got[key] != f[key] {
                failures.push(format!(
                    "{file}: {key}: manifest {} != native {}",
                    f[key], got[key]
                ));
            }
        }
        if !got["verify_error"].is_null() {
            failures.push(format!("{file}: verify_error {}", got["verify_error"]));
        }
        if f["bytes"] != json.len() {
            failures.push(format!(
                "{file}: bytes: manifest {} != file {}",
                f["bytes"],
                json.len()
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "parity fixtures:\n{}",
        failures.join("\n")
    );
}

#[test]
fn manifest_lists_every_fixture_file() {
    let mut listed: Vec<String> = manifest()
        .iter()
        .map(|f| f["file"].as_str().unwrap().to_string())
        .collect();
    let mut on_disk: Vec<String> = std::fs::read_dir(dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.ends_with(".json") && n != "manifest.json")
        .collect();
    listed.sort();
    on_disk.sort();
    assert_eq!(listed, on_disk);
}

#[test]
fn a_tampered_fixture_fails() {
    // Reverse tank 0's first recorded throttle in the 2v2 fixture: it drives the other
    // way on tick 0, so the replay must stop verifying.
    let file = "arena-2v2-tick-limit.json";
    let json = std::fs::read_to_string(dir().join(file)).unwrap();
    let mut v: Value = serde_json::from_str(&json).unwrap();
    let t = &mut v["actions"][0][0]["throttle"];
    let flipped = if t.as_f64().unwrap() > 0.0 { -1.0 } else { 1.0 };
    *t = flipped.into();
    let c = engine_wasm::check_replay(&v.to_string()).unwrap();
    assert!(c.verify_error.is_some(), "tampered replay still verifies");
}

/// Replay format 5 (the `game` envelope) leaves Tank Arena's committed format 4 files
/// alone: each still loads with no `game` or `rules_version`, verifies, and writes back
/// byte for byte.
#[test]
fn committed_format_4_fixtures_load_verify_and_round_trip() {
    assert_eq!(engine::replay::REPLAY_FORMAT, 5);
    for f in &manifest() {
        let file = f["file"].as_str().expect("file");
        let json = std::fs::read_to_string(dir().join(file)).expect("read fixture");
        let r = Replay::from_json(&json).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert_eq!(
            (r.format, &r.game, r.rules_version),
            (4, &None, None),
            "{file}"
        );
        let m = r.verify().unwrap_or_else(|e| panic!("{file}: {e}"));
        assert_eq!(r.to_json(), json.trim_end(), "{file}: bytes round-trip");
        assert_eq!(
            m.replay().to_json(),
            json.trim_end(),
            "{file}: re-recorded bytes"
        );
    }
}
