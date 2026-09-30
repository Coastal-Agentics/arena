//! Headless match runner: N matches -> JSON summary on stdout. Source of truth for CI.
//!
//! Match `i` uses seed `seed + i` (wrapping), so `--matches 1 --seed S+i` reproduces it.
//! Team 0 is the built-in `Chaser`, team 1 the built-in `Wanderer` placeholder policy.

use clap::Parser;
use engine::bots::{Chaser, Wanderer};
use engine::{EndReason, Match, MatchConfig, Replay};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "engine-cli",
    version,
    about = "Coastal Agentics Arena: run headless duels between the built-in bots and print JSON results"
)]
struct Args {
    /// Number of matches to run.
    #[arg(long, default_value_t = 10)]
    matches: u32,
    /// RNG seed; the same seed reproduces the same matches.
    #[arg(long, default_value_t = 42)]
    seed: u64,
    /// Optional directory to write one JSON replay per match (`match-<seed>.json`).
    #[arg(long)]
    replay_dir: Option<PathBuf>,
}

#[derive(Serialize, Debug, PartialEq)]
struct MatchResult {
    /// Decimal string: JavaScript cannot represent every u64 exactly.
    #[serde(with = "engine::json_u64")]
    seed: u64,
    /// Winning team (0 = Chaser, 1 = Wanderer) or null for a draw.
    winner: Option<u8>,
    ticks: u32,
    reason: EndReason,
    /// Final state hash; identical across runs with the same seed.
    hash: String,
}

#[derive(Serialize, Debug)]
struct Summary {
    matches: u32,
    #[serde(with = "engine::json_u64")]
    seed: u64,
    results: Vec<MatchResult>,
}

fn play(seed: u64) -> (MatchResult, Replay) {
    let mut m = Match::new(MatchConfig::duel(), seed);
    let mut a = Chaser;
    let mut b = Wanderer::new(seed ^ 0x5eed);
    let o = m.run(&mut [&mut a, &mut b]);
    let replay = m.replay();
    (
        MatchResult {
            seed,
            winner: o.winner,
            ticks: o.ticks,
            reason: o.reason,
            hash: replay.final_hash.clone(),
        },
        replay,
    )
}

fn run(args: &Args) -> Result<Summary, String> {
    if let Some(dir) = &args.replay_dir {
        std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    }
    let mut results = Vec::with_capacity(args.matches as usize);
    for i in 0..args.matches {
        let seed = args.seed.wrapping_add(i as u64);
        let (res, replay) = play(seed);
        if let Some(dir) = &args.replay_dir {
            let path = dir.join(format!("match-{seed}.json"));
            std::fs::write(&path, replay.to_json())
                .map_err(|e| format!("write {}: {e}", path.display()))?;
        }
        results.push(res);
    }
    Ok(Summary {
        matches: args.matches,
        seed: args.seed,
        results,
    })
}

fn main() {
    let args = Args::parse();
    match run(&args) {
        Ok(summary) => println!(
            "{}",
            serde_json::to_string(&summary).expect("summary serializes")
        ),
        Err(e) => {
            eprintln!("engine-cli: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(matches: u32, seed: u64) -> Args {
        Args {
            matches,
            seed,
            replay_dir: None,
        }
    }

    #[test]
    fn help_uses_current_branding() {
        use clap::CommandFactory;
        let help = Args::command().render_help().to_string();
        assert!(help.starts_with("Coastal Agentics Arena: "), "{help}");
        assert!(!help.contains("Starscream"), "{help}");
    }

    #[test]
    fn same_seed_same_json() {
        let a = serde_json::to_string(&run(&args(5, 42)).unwrap()).unwrap();
        let b = serde_json::to_string(&run(&args(5, 42)).unwrap()).unwrap();
        assert_eq!(a, b);
        assert!(a.starts_with(r#"{"matches":5,"seed":"42","results":[{"seed":"42","#));
    }

    #[test]
    fn seeds_are_js_safe_strings() {
        let s = run(&args(2, u64::MAX)).unwrap();
        let v: serde_json::Value = serde_json::to_value(&s).unwrap();
        assert_eq!(v["seed"], "18446744073709551615");
        assert_eq!(v["results"][0]["seed"], "18446744073709551615");
        assert_eq!(v["results"][1]["seed"], "0"); // wrapping_add
    }

    #[test]
    fn match_i_is_reproducible_alone() {
        let all = run(&args(4, 100)).unwrap();
        let third = run(&args(1, 102)).unwrap();
        assert_eq!(all.results[2], third.results[0]);
    }

    #[test]
    fn written_replay_verifies() {
        let dir = std::env::temp_dir().join(format!("engine-cli-test-{}", std::process::id()));
        let mut a = args(1, 7);
        a.replay_dir = Some(dir.clone());
        let s = run(&a).unwrap();
        let json = std::fs::read_to_string(dir.join("match-7.json")).unwrap();
        let r = Replay::from_json(&json).unwrap();
        let m = r.verify().unwrap();
        assert_eq!(format!("{:016x}", m.state_hash()), s.results[0].hash);
        std::fs::remove_dir_all(dir).ok();
    }
}
