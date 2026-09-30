//! Preliminary policy-triangle check at 3/3/3 (SPEC: every pairing 55–80% over 200
//! mirrored seeds). Not the balance report: prints win/loss/draw per pairing.
//!
//! `cargo run -p tank --release --example triangle -- [seeds]`

use tank::{Behavior, Loadout, MatchSpec, TankSpec};

fn main() {
    let n: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(200);
    let pairs = [
        (Behavior::Kiter, Behavior::Charger),
        (Behavior::Charger, Behavior::Sniper),
        (Behavior::Sniper, Behavior::Kiter),
    ];
    for (a, b) in pairs {
        let (mut w, mut l, mut d) = (0, 0, 0);
        let mut ticks = Vec::new();
        for seed in 0..n {
            let m = MatchSpec {
                seed,
                blue: TankSpec::new(a, Loadout::DEFAULT),
                orange: TankSpec::new(b, Loadout::DEFAULT),
            };
            // Mirrored: a plays blue, then orange, with the same seed.
            for (spec, a_team) in [(m, 0u8), (m.swapped(), 1u8)] {
                let (o, _) = spec.run();
                ticks.push(o.ticks);
                match o.winner {
                    Some(t) if t == a_team => w += 1,
                    Some(_) => l += 1,
                    None => d += 1,
                }
            }
        }
        ticks.sort_unstable();
        let games = w + l + d;
        println!(
            "{a:>7} vs {b:<7} {w:>4} W {l:>4} L {d:>4} D  win {:5.1}%  (decisive {:5.1}%)  median {:.1}s",
            100.0 * w as f64 / games as f64,
            100.0 * w as f64 / (w + l).max(1) as f64,
            ticks[ticks.len() / 2] as f64 / 60.0
        );
    }
}
