//! Verify replay files natively, outside Python: `cargo run --release --example
//! verify_replay -- FILE...`. Each file is loaded as its game (format 5 `"game"`:
//! racing; no `"game"`: Tank Arena, format 4) and re-simulated with
//! `Replay::verify`. Prints `ok <file> <final_hash>`, and exits 1 on any failure.
//!
//! CI uses it on the replays the Python determinism tests write
//! (`SALTMARSH_ARENA_REPLAY_DIR`), so "the Python `final_hash` equals Rust's
//! `Replay::verify`" is checked by a binary with no Python in it.

use engine::generic::{Replay, Rules};
use engine::TankRules;
use racing::RacingRules;

fn verify<R: Rules>(json: &str) -> Result<String, String>
where
    R::Config: Clone,
{
    let m = Replay::<R>::from_json(json)
        .and_then(|r| r.verify())
        .map_err(|e| e.to_string())?;
    Ok(format!("{:016x}", m.state_hash()))
}

fn main() {
    let files: Vec<String> = std::env::args().skip(1).collect();
    if files.is_empty() {
        eprintln!("usage: verify_replay FILE...");
        std::process::exit(2);
    }
    let mut failed = false;
    for f in &files {
        let result = std::fs::read_to_string(f)
            .map_err(|e| e.to_string())
            .and_then(|json| {
                let v: serde_json::Value =
                    serde_json::from_str(&json).map_err(|e| e.to_string())?;
                match v.get("game").and_then(|g| g.as_str()) {
                    None => verify::<TankRules>(&json),
                    Some(g) if g == RacingRules::GAME => verify::<RacingRules>(&json),
                    Some(g) => Err(format!("unknown game {g:?}")),
                }
            });
        match result {
            Ok(h) => println!("ok {f} {h}"),
            Err(e) => {
                println!("FAIL {f}: {e}");
                failed = true;
            }
        }
    }
    if failed {
        std::process::exit(1);
    }
}
