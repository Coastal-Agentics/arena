//! Run one Tank Arena duel from a URL query and print the outcome and final state hash
//! as JSON (the viewer's `stateHash()` at the end of the same link must match).
//!
//! `cargo run -p tank --example tank_match -- "seed=42&blue=kiter-5-3-1&orange=charger-4-1-4"`

fn main() {
    let q = std::env::args().nth(1).unwrap_or_default();
    let spec = match tank::MatchSpec::from_query(&q) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let setup = spec.setup();
    let (o, hash) = spec.run();
    println!(
        "{{\"query\":\"{}\",\"winner\":{},\"ticks\":{},\"reason\":\"{:?}\",\"hash\":\"{:016x}\",\"stats_applied\":{}}}",
        spec.to_query(),
        o.winner.map_or("null".to_string(), |w| w.to_string()),
        o.ticks,
        o.reason,
        hash,
        setup.stats_applied()
    );
}
