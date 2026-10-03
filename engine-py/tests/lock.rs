//! This crate is its own workspace (the root one excludes it), so it has its own
//! `Cargo.lock`. Every crate both lockfiles share must be the same version, so the
//! engine and games in the wheel build exactly as in engine-cli and engine-wasm
//! (e.g. the same `serde_json`, which writes the replay JSON). To fix a mismatch,
//! copy the root `Cargo.lock` here and run `cargo update -p engine-py` (or any build).

use std::collections::BTreeMap;

fn packages(path: &str) -> BTreeMap<String, Vec<String>> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut name = None;
    for line in text.lines() {
        if let Some(n) = line.strip_prefix("name = ") {
            name = Some(n.trim_matches('"').to_string());
        } else if let (Some(v), Some(n)) = (line.strip_prefix("version = "), name.take()) {
            out.entry(n)
                .or_default()
                .push(v.trim_matches('"').to_string());
        }
    }
    out
}

#[test]
fn shared_crates_have_the_root_versions() {
    let dir = env!("CARGO_MANIFEST_DIR");
    let ours = packages(&format!("{dir}/Cargo.lock"));
    let root = packages(&format!("{dir}/../Cargo.lock"));
    assert!(ours.contains_key("pyo3") && !root.contains_key("pyo3"));
    let mut shared = 0;
    for (name, versions) in &ours {
        if let Some(r) = root.get(name) {
            assert_eq!(versions, r, "{name}: engine-py/Cargo.lock vs Cargo.lock");
            shared += 1;
        }
    }
    for c in [
        "engine",
        "tank",
        "racing",
        "game-catalog",
        "serde",
        "serde_json",
        "rand_chacha",
        "glam",
    ] {
        assert!(
            ours.contains_key(c) && root.contains_key(c),
            "{c} in both lockfiles"
        );
    }
    assert!(shared >= 15, "{shared} shared crates");
}
