//! Headless match runner. Phase 0: prints placeholder JSON.

use clap::Parser;
use serde::Serialize;

#[derive(Parser, Debug)]
#[command(
    name = "engine-cli",
    version,
    about = "Run headless Starscream matches"
)]
struct Args {
    /// Number of matches to run.
    #[arg(long, default_value_t = 10)]
    matches: u32,
    /// RNG seed; the same seed reproduces the same matches.
    #[arg(long, default_value_t = 42)]
    seed: u64,
}

#[derive(Serialize, Debug)]
struct Summary {
    matches: u32,
    seed: u64,
    results: Vec<serde_json::Value>,
}

fn run(args: &Args) -> Summary {
    Summary {
        matches: args.matches,
        seed: args.seed,
        results: Vec::new(),
    }
}

fn main() {
    let args = Args::parse();
    let _ = engine::TICK_HZ;
    let summary = run(&args);
    println!(
        "{}",
        serde_json::to_string(&summary).expect("summary serializes")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_json_shape() {
        let s = run(&Args {
            matches: 10,
            seed: 42,
        });
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, r#"{"matches":10,"seed":42,"results":[]}"#);
    }
}
