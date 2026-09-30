//! GATE-003 M1 evolution loop (`docs/plans/GATE-003-learning-tanks.md`).
//!
//! ```text
//! cargo run -p tank --release --example evolve -- run --out DIR [--seed 1]
//!     [--generations 100] [--population 32] [--train-seeds 8] [--heldout 1000]
//!     [--threads N] [--minutes 30] [--resume] [--commit SHA]
//! cargo run -p tank --release --example evolve -- verify --out DIR [--threads N]
//! ```
//!
//! `run` evolves `--generations` more generations (continuing `DIR/state.json` with
//! `--resume`), then plays the latest champion against Gen 0 on the held-out seeds and
//! writes, all deterministic (no timestamps):
//!
//! - `state.json`: settings, next generation, population, hall of fame (for `--resume`);
//! - `champion.json`: latest champion, its held-out result, digest and promotion status;
//! - `history.json`: one line per generation (thinned past 2,000 entries);
//! - `genomes.json`: the champion of every 10th generation (for the viewer's slider).
//!
//! `verify` replays `champion.json`'s held-out matches and fails on any difference.

use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;
use tank::evolve::{self, Config, Genome, HeldOut, State};

/// Every generation is kept for the latest `HISTORY_FULL` entries; older ones only
/// every 10th.
const HISTORY_FULL: usize = 2_000;
/// Caps from the plan's guardrails (the per-commit 200 KB cap is checked by the
/// nightly job on the git diff).
const MAX_TOTAL_BYTES: u64 = 5_000_000;
const MAX_STATE_BYTES: u64 = 64_000;
const MAX_CHAMPION_BYTES: u64 = 16_000;

struct Args {
    cmd: String,
    out: PathBuf,
    seed: u64,
    generations: u32,
    population: usize,
    train_seeds: u64,
    heldout: u64,
    threads: usize,
    minutes: f64,
    resume: bool,
    commit: Option<String>,
}

fn parse() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let cmd = it.next().ok_or("usage: evolve run|verify --out DIR ...")?;
    let d = Config::default();
    let mut a = Args {
        cmd,
        out: PathBuf::new(),
        seed: d.seed,
        generations: 100,
        population: d.population,
        train_seeds: d.train_seeds,
        heldout: evolve::HELDOUT_SEEDS,
        threads: std::thread::available_parallelism().map_or(4, |n| n.get()),
        minutes: 30.0,
        resume: false,
        commit: None,
    };
    while let Some(k) = it.next() {
        if k == "--resume" {
            a.resume = true;
            continue;
        }
        let v = it.next().ok_or_else(|| format!("{k} needs a value"))?;
        let bad = |e: &dyn std::fmt::Display| format!("{k}: {e}");
        match k.as_str() {
            "--out" => a.out = v.into(),
            "--seed" => a.seed = v.parse().map_err(|e| bad(&e))?,
            "--generations" => a.generations = v.parse().map_err(|e| bad(&e))?,
            "--population" => a.population = v.parse().map_err(|e| bad(&e))?,
            "--train-seeds" => a.train_seeds = v.parse().map_err(|e| bad(&e))?,
            "--heldout" => a.heldout = v.parse().map_err(|e| bad(&e))?,
            "--threads" => a.threads = v.parse().map_err(|e| bad(&e))?,
            "--minutes" => a.minutes = v.parse().map_err(|e| bad(&e))?,
            "--commit" => a.commit = Some(v),
            _ => return Err(format!("unknown flag {k}")),
        }
    }
    if a.out.as_os_str().is_empty() {
        return Err("--out DIR is required".into());
    }
    Ok(a)
}

fn read_json(p: &Path) -> Result<Value, String> {
    let s = fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
    serde_json::from_str(&s).map_err(|e| format!("{}: {e}", p.display()))
}

fn write(p: &Path, s: String) -> Result<(), String> {
    fs::write(p, s + "\n").map_err(|e| format!("{}: {e}", p.display()))
}

/// A JSON array with one element per line (small, readable diffs).
fn lines(v: &[Value]) -> String {
    let body: Vec<String> = v.iter().map(|x| x.to_string()).collect();
    if body.is_empty() {
        "[]".into()
    } else {
        format!("[\n{}\n]", body.join(",\n"))
    }
}

fn load_list(p: &Path) -> Result<Vec<Value>, String> {
    if !p.exists() {
        return Ok(Vec::new());
    }
    read_json(p)?
        .as_array()
        .cloned()
        .ok_or_else(|| format!("{}: expected an array", p.display()))
}

fn thin(history: &mut Vec<Value>) {
    if history.len() <= HISTORY_FULL {
        return;
    }
    let cut = history.len() - HISTORY_FULL;
    let mut i = 0;
    history.retain(|e| {
        let keep = i >= cut || e["gen"].as_u64().is_some_and(|g| g % 10 == 0);
        i += 1;
        keep
    });
}

fn print_heldout(h: &HeldOut) {
    let [c, k, s] = h.per_opponent;
    println!(
        "held-out vs Gen 0: {} / {} wins = {:.1}% ({} seeds x 3 opponents x 2 sides) [{}]",
        h.wins(),
        h.games(),
        100.0 * h.win_rate(),
        h.seeds,
        h.status()
    );
    for (name, w) in [("charger", c), ("kiter", k), ("sniper", s)] {
        println!(
            "  vs {name:<7} W {:>4}  D {:>4}  L {:>4}",
            w.wins, w.draws, w.losses
        );
    }
    println!("  digest {:016x}", h.digest);
}

fn run(a: &Args) -> Result<(), String> {
    fs::create_dir_all(&a.out).map_err(|e| format!("{}: {e}", a.out.display()))?;
    let state_path = a.out.join("state.json");
    let mut state = if a.resume && state_path.exists() {
        State::from_json(&read_json(&state_path)?)?
    } else {
        if state_path.exists() {
            return Err(format!("{} exists; pass --resume", state_path.display()));
        }
        State::new(Config {
            seed: a.seed,
            population: a.population,
            train_seeds: a.train_seeds,
            ..Config::default()
        })?
    };
    let hist_path = a.out.join("history.json");
    let gen_path = a.out.join("genomes.json");
    let mut history = load_list(&hist_path)?;
    let mut genomes = load_list(&gen_path)?;

    let t0 = Instant::now();
    let mut last = None;
    for _ in 0..a.generations {
        if t0.elapsed().as_secs_f64() > a.minutes * 60.0 {
            eprintln!("time cap of {} min reached; stopping early", a.minutes);
            break;
        }
        let g = state.step(a.threads);
        println!(
            "gen {:>4}  best {:.4}  mean {:.4}  vs scripted {:>5.1}%  champion {}  mix c/k/s {:?}",
            g.index,
            g.best.fitness,
            g.mean_fitness,
            100.0 * g.best.vs_scripted,
            g.champion.label(),
            g.mix
        );
        history.push(g.to_json());
        if g.index % 10 == 0 {
            genomes.push(json!({ "gen": g.index, "genome": g.champion.to_json() }));
        }
        last = Some(g);
    }
    let evolved = t0.elapsed().as_secs_f64();
    let Some(g) = last else {
        return Err("no generation ran".into());
    };
    thin(&mut history);

    let t1 = Instant::now();
    let h = evolve::held_out(&g.champion, a.heldout, a.threads);
    let checked = t1.elapsed().as_secs_f64();
    print_heldout(&h);

    let mut champ = json!({
        "format": evolve::FORMAT,
        "generation": g.index,
        "label": g.champion.label(),
        "genome": g.champion.to_json(),
        "training": {
            "fitness": evolve::round4(g.best.fitness),
            "damage": evolve::round4(g.best.damage),
            "vs_scripted": evolve::round4(g.best.vs_scripted),
            "matches": g.best.matches,
            "seed_base": evolve::TRAIN_SEED_BASE.to_string(),
        },
        "held_out": h.to_json(),
        "config": state.config.to_json(),
        "fitness_function": evolve::FITNESS_TEXT,
        "promotion": format!(
            "win rate vs Gen 0 >= {:.0}% to promote; above {:.0}% the champion is held from \
             promotion and labeled experimental, and the CoS tells Nye",
            100.0 * evolve::ACCEPT_WIN_RATE,
            100.0 * evolve::HOLD_WIN_RATE
        ),
    });
    if let Some(c) = &a.commit {
        champ["commit"] = json!(c);
    }
    write(&state_path, state.to_json().to_string())?;
    write(
        &a.out.join("champion.json"),
        serde_json::to_string_pretty(&champ).unwrap(),
    )?;
    write(&hist_path, lines(&history))?;
    write(&gen_path, lines(&genomes))?;
    check_sizes(&a.out)?;
    println!(
        "evolved to gen {} in {:.1} s; held-out check {:.1} s; {} threads",
        g.index, evolved, checked, a.threads
    );
    Ok(())
}

fn check_sizes(dir: &Path) -> Result<(), String> {
    let size = |n: &str| fs::metadata(dir.join(n)).map_or(0, |m| m.len());
    let total: u64 = [
        "state.json",
        "champion.json",
        "history.json",
        "genomes.json",
    ]
    .iter()
    .map(|n| size(n))
    .sum();
    println!(
        "files: state {} B, champion {} B, history {} B, genomes {} B, total {} B",
        size("state.json"),
        size("champion.json"),
        size("history.json"),
        size("genomes.json"),
        total
    );
    for (n, cap) in [
        ("state.json", MAX_STATE_BYTES),
        ("champion.json", MAX_CHAMPION_BYTES),
    ] {
        if size(n) > cap {
            return Err(format!("{n} is {} B, over its {cap} B cap", size(n)));
        }
    }
    if total > MAX_TOTAL_BYTES {
        return Err(format!(
            "output is {total} B, over the {MAX_TOTAL_BYTES} B cap"
        ));
    }
    Ok(())
}

fn verify(a: &Args) -> Result<(), String> {
    let c = read_json(&a.out.join("champion.json"))?;
    let g = Genome::from_json(&c["genome"])?;
    let want = &c["held_out"];
    let seeds = want["seeds"].as_u64().ok_or("held_out.seeds")?;
    if want["seed_base"].as_str() != Some(&evolve::HELDOUT_SEED_BASE.to_string()) {
        return Err("held-out seed base differs from this build".into());
    }
    State::from_json(&read_json(&a.out.join("state.json"))?)?;
    let h = evolve::held_out(&g, seeds, a.threads);
    print_heldout(&h);
    if &h.to_json() != want {
        return Err(format!(
            "MISMATCH: recorded {want}\n          replayed {}",
            h.to_json()
        ));
    }
    check_sizes(&a.out)?;
    println!(
        "verified: {} ({} matches) replays exactly",
        g.label(),
        h.games()
    );
    Ok(())
}

fn main() -> ExitCode {
    let r = parse().and_then(|a| match a.cmd.as_str() {
        "run" => run(&a),
        "verify" => verify(&a),
        c => Err(format!("unknown command {c:?} (run or verify)")),
    });
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("evolve: {e}");
            ExitCode::FAILURE
        }
    }
}
