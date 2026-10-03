//! Race runners and summaries for the balance report (`examples/balance.rs`, which
//! writes `games/racing/BALANCE.md`) and the R2 acceptance tests. Deterministic: the
//! same seeds give the same numbers on any thread count.

use crate::config::RacingConfig;
use crate::drivers::Behavior;
use crate::end::RaceEnd;
use crate::rules::RacingRules;
use crate::setup::Setup;
use crate::Race;
use engine::generic::Policy;

/// One finished race.
#[derive(Clone, Debug, PartialEq)]
pub struct RaceResult {
    /// Ticks the race lasted.
    pub ticks: u32,
    /// Why it ended.
    pub end: RaceEnd,
    /// Winning car, if any.
    pub winner: Option<usize>,
    /// Per car: finish tick, if it finished.
    pub finish: Vec<Option<u32>>,
    /// Per car: final place.
    pub places: Vec<u8>,
    /// Per car: grid slot.
    pub slots: Vec<u8>,
}

/// Run one race of `drivers` (car `i` drives `drivers[i]`) on `config`.
pub fn run(config: &RacingConfig, drivers: &[Behavior], seed: u64) -> RaceResult {
    let mut m = Race::new(config.clone(), seed);
    let mut ps: Vec<Box<dyn Policy<RacingRules>>> = drivers
        .iter()
        .enumerate()
        .map(|(i, b)| b.driver(config, i, seed))
        .collect();
    let o = {
        let mut refs: Vec<&mut dyn Policy<RacingRules>> = ps
            .iter_mut()
            .map(|p| p.as_mut() as &mut dyn Policy<RacingRules>)
            .collect();
        m.run(&mut refs)
    };
    let s = m.state();
    RaceResult {
        ticks: o.ticks,
        end: RacingRules::race_end(config, s, o.ticks).expect("race is over"),
        winner: o.winner.map(usize::from),
        finish: s.cars.iter().map(|c| c.finish_tick).collect(),
        places: s.cars.iter().map(|c| c.place).collect(),
        slots: s.cars.iter().map(|c| c.slot).collect(),
    }
}

/// Run `f(seed)` for every seed in `seeds` on `threads` threads, results in seed order.
pub fn par_map<T: Send>(seeds: &[u64], threads: usize, f: impl Fn(u64) -> T + Sync) -> Vec<T> {
    let threads = threads.max(1);
    let chunk = seeds.len().div_ceil(threads).max(1);
    std::thread::scope(|sc| {
        let f = &f;
        let hs: Vec<_> = seeds
            .chunks(chunk)
            .map(|c| sc.spawn(move || c.iter().map(|&s| f(s)).collect::<Vec<T>>()))
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    })
}

/// Threads to use: the machine's parallelism, at most 16.
pub fn threads() -> usize {
    std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .min(16)
}

/// Median of `v` (mean of the middle two for even lengths); 0 for an empty slice.
pub fn median(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

/// Solo summary for one driver and setup.
#[derive(Clone, Debug, PartialEq)]
pub struct Solo {
    /// Races run.
    pub races: usize,
    /// Races finished (all laps).
    pub finished: usize,
    /// Median finish time (s) of finished races.
    pub median_s: f64,
    /// Best and worst finish time (s).
    pub best_s: f64,
    /// Worst finish time (s).
    pub worst_s: f64,
}

/// `driver` alone on the Ring with `setup`, seeds `0..n` (the seed only moves the
/// driver's jitter).
pub fn solo(driver: Behavior, setup: Setup, n: u64) -> Solo {
    let cfg = RacingConfig::ring_setups(&[setup]);
    let seeds: Vec<u64> = (0..n).collect();
    let rs = par_map(&seeds, threads(), |s| run(&cfg, &[driver], s));
    let mut t: Vec<f64> = rs
        .iter()
        .filter_map(|r| r.finish[0])
        .map(|t| t as f64 / 60.0)
        .collect();
    let finished = t.len();
    let best = t.iter().cloned().fold(f64::INFINITY, f64::min);
    let worst = t.iter().cloned().fold(0.0, f64::max);
    Solo {
        races: rs.len(),
        finished,
        median_s: median(&mut t),
        best_s: best,
        worst_s: worst,
    }
}

/// A 4-car field summary.
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    /// Races run.
    pub races: usize,
    /// Car-races run (races × cars).
    pub entries: usize,
    /// Car-races that finished.
    pub finished: usize,
    /// Median race length (s).
    pub median_s: f64,
    /// Races that hit the tick cap.
    pub capped: usize,
    /// Wins by grid slot.
    pub wins_by_slot: [usize; 4],
    /// Wins by driver ([`Behavior::ALL`] order).
    pub wins_by_driver: [usize; 3],
    /// Entries by driver (how many car-races each drove).
    pub entries_by_driver: [usize; 3],
    /// Races with a winner.
    pub decided: usize,
}

/// Summarize races of the given lineups.
pub fn field(results: &[(Vec<Behavior>, RaceResult)]) -> Field {
    let mut f = Field {
        races: results.len(),
        entries: 0,
        finished: 0,
        median_s: 0.0,
        capped: 0,
        wins_by_slot: [0; 4],
        wins_by_driver: [0; 3],
        entries_by_driver: [0; 3],
        decided: 0,
    };
    let mut len = Vec::with_capacity(results.len());
    for (lineup, r) in results {
        f.entries += r.finish.len();
        f.finished += r.finish.iter().filter(|t| t.is_some()).count();
        if r.end == RaceEnd::TickLimit {
            f.capped += 1;
        }
        len.push(r.ticks as f64 / 60.0);
        for b in lineup {
            f.entries_by_driver[idx(*b)] += 1;
        }
        if let Some(w) = r.winner {
            f.decided += 1;
            f.wins_by_slot[r.slots[w] as usize] += 1;
            f.wins_by_driver[idx(lineup[w])] += 1;
        }
    }
    f.median_s = median(&mut len);
    f
}

fn idx(b: Behavior) -> usize {
    Behavior::ALL.iter().position(|&x| x == b).unwrap()
}

/// `n` mirror races of `driver` (4 cars, 3-3-3, grid shuffled by seed).
pub fn mirror(driver: Behavior, n: u64) -> Vec<(Vec<Behavior>, RaceResult)> {
    let cfg = RacingConfig::ring_setups(&[Setup::BALANCED; 4]);
    let lineup = vec![driver; 4];
    let seeds: Vec<u64> = (0..n).collect();
    par_map(&seeds, threads(), |s| {
        (lineup.clone(), run(&cfg, &lineup, s))
    })
}

/// The mixed lineups: Follower, Cutter, Blocker and a fourth car that is each of the
/// three in turn.
pub fn mixed_lineups() -> [Vec<Behavior>; 3] {
    let [f, c, b] = Behavior::ALL;
    [vec![f, c, b, f], vec![f, c, b, c], vec![f, c, b, b]]
}

/// Mixed 4-car races, seeds `0..n`: every seed runs each of the 3 lineups on each of
/// the 4 grid rotations (car `i` on slot `(i + r) % 4`), so 12 races per seed.
pub fn mixed(n: u64) -> Vec<(Vec<Behavior>, RaceResult)> {
    let seeds: Vec<u64> = (0..n).collect();
    let per_seed = par_map(&seeds, threads(), |s| {
        let mut out = Vec::with_capacity(12);
        for lineup in mixed_lineups() {
            for r in 0..4u8 {
                let grid = (0..4u8).map(|i| (i + r) % 4).collect();
                let cfg = RacingConfig::ring_setups(&[Setup::BALANCED; 4]).with_grid(grid);
                out.push((lineup.clone(), run(&cfg, &lineup, s)));
            }
        }
        out
    });
    per_seed.into_iter().flatten().collect()
}
