//! GATE-003 M1: a seeded genetic algorithm over the scripted policies' numbers and the
//! loadout (`docs/plans/GATE-003-learning-tanks.md`).
//!
//! - **Genome:** a [`Behavior`], its tunable numbers ([`gene_infos`]) and one of the 19
//!   valid loadouts. Gen 0 contains the three shipped policies exactly ([`Genome::scripted`]).
//! - **Fitness:** every candidate plays the three scripted policies (3/3/3) and the
//!   hall of fame (the last [`Config::hall_of_fame`] champions) on
//!   [`Config::train_seeds`] fixed seeds, from **both** spawn sides. Win = 1, draw =
//!   0.25, loss = 0, averaged over all those matches; ties go to more damage dealt,
//!   then to the lower population index.
//! - **Held-out check:** the champion plays the three Gen 0 policies on seeds
//!   `HELDOUT_SEED_BASE..+n`, both sides; they never appear in training
//!   (`TRAIN_SEED_BASE..+train_seeds`). Win rate = wins / matches.
//! - **Determinism:** all randomness is ChaCha8 seeded from `(Config::seed,
//!   generation)`; matches run in parallel but results are folded in a fixed order, so
//!   the thread count never changes a result. Same config → same champions and hashes.

pub mod genome;

pub use genome::{gene_infos, GeneInfo, Genome};

use crate::loadout::Loadout;
use crate::matchup::{BLUE_SALT, ORANGE_SALT};
use crate::policies::Behavior;
use crate::rules;
use engine::Match;
use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde_json::{json, Value};
use std::thread;

/// Points for a win.
pub const WIN_POINTS: f64 = 1.0;
/// Points for a draw (low, so stalling to the 120 s cap does not pay).
pub const DRAW_POINTS: f64 = 0.25;
/// Points for a loss.
pub const LOSS_POINTS: f64 = 0.0;
/// The fitness function, as published with every result.
pub const FITNESS_TEXT: &str = "mean over matches of (win 1, draw 0.25, loss 0); opponents: \
Charger, Kiter and Sniper at defaults and 3/3/3, plus the last 8 champions; fixed training \
seeds, both spawn sides; ties: more damage dealt, then lower population index";

/// Training seeds are `TRAIN_SEED_BASE + i`.
pub const TRAIN_SEED_BASE: u64 = 1_000_000;
/// Held-out seeds are `HELDOUT_SEED_BASE + i` (disjoint from training).
pub const HELDOUT_SEED_BASE: u64 = 9_000_000_000;
/// Held-out seeds used by the acceptance check (charter: 1,000).
pub const HELDOUT_SEEDS: u64 = 1_000;
/// M1 acceptance: Gen N wins at least this share of held-out matches vs Gen 0.
pub const ACCEPT_WIN_RATE: f64 = 0.65;
/// Above this share vs the scripted field, a champion is held from promotion.
pub const HOLD_WIN_RATE: f64 = 0.70;
/// Version of the JSON files written by this module.
pub const FORMAT: u32 = 1;

/// GA settings. Stored in the state file, so a resumed run keeps them.
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// Run seed: drives Gen 0 mutants and all breeding.
    pub seed: u64,
    /// Candidates per generation.
    pub population: usize,
    /// Best candidates copied unchanged into the next generation.
    pub elite: usize,
    /// Tournament size for parent selection.
    pub tournament: usize,
    /// Training seeds per opponent (each played from both sides).
    pub train_seeds: u64,
    /// Hall-of-fame size (most recent champions).
    pub hall_of_fame: usize,
    /// Chance that each gene mutates.
    pub gene_rate: f64,
    /// Mutation step: standard deviation as a share of the gene's range.
    pub sigma: f64,
    /// Chance that the loadout moves one point between two stats.
    pub loadout_rate: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            seed: 1,
            population: 32,
            elite: 4,
            tournament: 3,
            train_seeds: 8,
            hall_of_fame: 8,
            gene_rate: 0.25,
            sigma: 0.1,
            loadout_rate: 0.3,
        }
    }
}

impl Config {
    /// JSON object with every field.
    pub fn to_json(&self) -> Value {
        json!({
            "seed": self.seed.to_string(),
            "population": self.population,
            "elite": self.elite,
            "tournament": self.tournament,
            "train_seeds": self.train_seeds,
            "hall_of_fame": self.hall_of_fame,
            "gene_rate": self.gene_rate,
            "sigma": self.sigma,
            "loadout_rate": self.loadout_rate,
        })
    }

    /// Parse [`Config::to_json`] output.
    pub fn from_json(v: &Value) -> Result<Self, String> {
        let u = |k: &str| {
            v.get(k)
                .and_then(Value::as_u64)
                .ok_or_else(|| format!("config.{k}"))
        };
        let f = |k: &str| {
            v.get(k)
                .and_then(Value::as_f64)
                .ok_or_else(|| format!("config.{k}"))
        };
        let seed = v
            .get("seed")
            .and_then(Value::as_str)
            .ok_or("config.seed")?
            .parse()
            .map_err(|e| format!("config.seed: {e}"))?;
        let c = Self {
            seed,
            population: u("population")? as usize,
            elite: u("elite")? as usize,
            tournament: u("tournament")? as usize,
            train_seeds: u("train_seeds")?,
            hall_of_fame: u("hall_of_fame")? as usize,
            gene_rate: f("gene_rate")?,
            sigma: f("sigma")?,
            loadout_rate: f("loadout_rate")?,
        };
        c.validate()?;
        Ok(c)
    }

    /// Reject settings the GA cannot run with.
    pub fn validate(&self) -> Result<(), String> {
        if self.population < 3 || self.elite >= self.population || self.tournament == 0 {
            return Err("need population >= 3, elite < population, tournament >= 1".into());
        }
        if self.train_seeds == 0 {
            return Err("need at least one training seed".into());
        }
        Ok(())
    }
}

/// One duel's result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Duel {
    /// Winning team (0 = blue, 1 = orange), `None` for a draw.
    pub winner: Option<u8>,
    /// Match length in ticks.
    pub ticks: u32,
    /// Final [`Match::state_hash`].
    pub hash: u64,
    /// Final hit points by tank id.
    pub hp: [i32; 2],
}

/// Play `blue` (tank 0) against `orange` (tank 1) on `seed`, with the same policy
/// jitter salts as [`crate::MatchSpec`], so scripted genomes replay shared matches.
pub fn duel(blue: &Genome, orange: &Genome, seed: u64) -> Duel {
    let setup = rules::duel(blue.loadout, orange.loadout);
    let mut m = Match::new(setup.config, seed);
    let mut a = blue.build(seed ^ BLUE_SALT);
    let mut b = orange.build(seed ^ ORANGE_SALT);
    let o = m.run(&mut [a.as_mut(), b.as_mut()]);
    let t = m.tanks();
    Duel {
        winner: o.winner,
        ticks: o.ticks,
        hash: m.state_hash(),
        hp: [t[0].hp, t[1].hp],
    }
}

/// A candidate's view of one match.
#[derive(Clone, Copy, Debug)]
struct Game {
    points: f64,
    win: bool,
    draw: bool,
    /// Damage dealt as a share of the opponent's max HP.
    damage: f64,
    duel: Duel,
}

fn play(cand: &Genome, opp: &Genome, seed: u64, side: usize) -> Game {
    let d = if side == 0 {
        duel(cand, opp, seed)
    } else {
        duel(opp, cand, seed)
    };
    let (win, draw) = match d.winner {
        None => (false, true),
        Some(t) => (t as usize == side, false),
    };
    let points = if win {
        WIN_POINTS
    } else if draw {
        DRAW_POINTS
    } else {
        LOSS_POINTS
    };
    let max = opp.loadout.max_hp() as f64;
    let left = d.hp[1 - side].max(0) as f64;
    Game {
        points,
        win,
        draw,
        damage: (max - left) / max,
        duel: d,
    }
}

/// `f` over `items` on up to `threads` threads; results in input order.
pub fn par_map<T: Sync, R: Send>(
    items: &[T],
    threads: usize,
    f: impl Fn(&T) -> R + Sync,
) -> Vec<R> {
    let n = threads.max(1).min(items.len().max(1));
    if n == 1 {
        return items.iter().map(f).collect();
    }
    let f = &f;
    let mut parts: Vec<Vec<(usize, R)>> = thread::scope(|s| {
        let hs: Vec<_> = (0..n)
            .map(|t| {
                s.spawn(move || {
                    (t..items.len())
                        .step_by(n)
                        .map(|i| (i, f(&items[i])))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        hs.into_iter()
            .map(|h| h.join().expect("worker panicked"))
            .collect()
    });
    let mut out: Vec<Option<R>> = (0..items.len()).map(|_| None).collect();
    for part in parts.drain(..) {
        for (i, r) in part {
            out[i] = Some(r);
        }
    }
    out.into_iter()
        .map(|r| r.expect("every index filled"))
        .collect()
}

/// The three Gen 0 opponents, in [`Behavior::ALL`] order.
pub fn scripted_field() -> [Genome; 3] {
    Behavior::ALL.map(Genome::scripted)
}

/// Fitness of one candidate.
#[derive(Clone, Debug, PartialEq)]
pub struct Score {
    /// Mean points over every training match.
    pub fitness: f64,
    /// Mean damage dealt (share of opponent max HP), the tie-break.
    pub damage: f64,
    /// Win share against the three scripted policies (training seeds).
    pub vs_scripted: f64,
    /// Matches played.
    pub matches: u32,
}

/// Summary of one evaluated generation.
#[derive(Clone, Debug, PartialEq)]
pub struct Generation {
    /// Generation index (0 = the one containing the scripted policies).
    pub index: u32,
    /// Its best candidate.
    pub champion: Genome,
    /// The champion's score.
    pub best: Score,
    /// Mean fitness of the whole population.
    pub mean_fitness: f64,
    /// Candidates per behavior, in [`Behavior::ALL`] order.
    pub mix: [usize; 3],
}

/// Everything needed to continue a run: settings, next generation index, the current
/// (not yet evaluated) population and the hall of fame.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    /// GA settings.
    pub config: Config,
    /// Index of the population below (generations evaluated so far).
    pub generation: u32,
    /// Candidates to evaluate next.
    pub population: Vec<Genome>,
    /// Most recent champions, oldest first.
    pub hall_of_fame: Vec<Genome>,
}

fn gen_rng(seed: u64, generation: u32) -> ChaCha8Rng {
    // SplitMix64 finalizer over (seed, generation): independent streams per generation,
    // so a resumed run breeds exactly as an uninterrupted one.
    let mut z = seed ^ (generation as u64 + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    ChaCha8Rng::seed_from_u64(z ^ (z >> 31))
}

fn unit(r: &mut ChaCha8Rng) -> f64 {
    (r.next_u64() >> 11) as f64 / (1u64 << 53) as f64
}

fn below(r: &mut ChaCha8Rng, n: usize) -> usize {
    (r.next_u64() % n as u64) as usize
}

/// Approximately standard normal, with no transcendental functions (Irwin–Hall sum of
/// four uniforms, rescaled), so it is identical on every platform.
fn normal(r: &mut ChaCha8Rng) -> f64 {
    let s: f64 = (0..4).map(|_| unit(r)).sum();
    (s - 2.0) * 1.732_050_807_568_877_2
}

/// Move one point from one stat to another, staying within the 19 valid splits.
pub fn nudge_loadout(l: Loadout, r: &mut ChaCha8Rng) -> Loadout {
    let lv = l.levels();
    let moves: Vec<(usize, usize)> = (0..3)
        .flat_map(|from| (0..3).map(move |to| (from, to)))
        .filter(|&(f, t)| f != t && lv[f] > 1 && lv[t] < 5)
        .collect();
    let (f, t) = moves[below(r, moves.len())];
    let mut n = lv;
    n[f] -= 1;
    n[t] += 1;
    Loadout::new(n[0], n[1], n[2]).expect("a one-point move keeps a valid split")
}

fn mutate(g: &mut Genome, c: &Config, r: &mut ChaCha8Rng) {
    let infos = gene_infos(g.behavior);
    let mut changed = false;
    for (v, i) in g.genes.iter_mut().zip(&infos) {
        if unit(r) < c.gene_rate {
            let span = (i.hi - i.lo) as f64;
            *v = genome::quantize(i, *v as f64 + normal(r) * c.sigma * span);
            changed = true;
        }
    }
    if unit(r) < c.loadout_rate {
        g.loadout = nudge_loadout(g.loadout, r);
        changed = true;
    }
    if !changed {
        let k = below(r, infos.len());
        let i = &infos[k];
        let span = (i.hi - i.lo) as f64;
        g.genes[k] = genome::quantize(i, g.genes[k] as f64 + normal(r) * c.sigma * span);
    }
    g.repair();
}

impl State {
    /// Gen 0: the three scripted policies, then mutants of them (round-robin).
    pub fn new(config: Config) -> Result<Self, String> {
        config.validate()?;
        let mut r = gen_rng(config.seed, 0);
        let field = scripted_field();
        let mut population: Vec<Genome> = field.to_vec();
        while population.len() < config.population {
            let mut g = field[population.len() % 3].clone();
            mutate(&mut g, &config, &mut r);
            population.push(g);
        }
        population.truncate(config.population);
        Ok(Self {
            config,
            generation: 0,
            population,
            hall_of_fame: Vec::new(),
        })
    }

    /// Score every candidate against the scripted field and the hall of fame.
    pub fn evaluate(&self, threads: usize) -> Vec<Score> {
        let field = scripted_field();
        let opps: Vec<&Genome> = field.iter().chain(&self.hall_of_fame).collect();
        let seeds = self.config.train_seeds;
        let mut jobs = Vec::new();
        for c in 0..self.population.len() {
            for o in 0..opps.len() {
                for s in 0..seeds {
                    for side in 0..2 {
                        jobs.push((c, o, TRAIN_SEED_BASE + s, side));
                    }
                }
            }
        }
        let games = par_map(&jobs, threads, |&(c, o, seed, side)| {
            play(&self.population[c], opps[o], seed, side)
        });
        let per = opps.len() * seeds as usize * 2;
        let scripted = 3 * seeds as usize * 2;
        games
            .chunks(per)
            .map(|g| {
                let n = g.len() as f64;
                Score {
                    fitness: g.iter().map(|x| x.points).sum::<f64>() / n,
                    damage: g.iter().map(|x| x.damage).sum::<f64>() / n,
                    vs_scripted: g[..scripted].iter().filter(|x| x.win).count() as f64
                        / scripted as f64,
                    matches: g.len() as u32,
                }
            })
            .collect()
    }

    /// Evaluate the current population, record its champion in the hall of fame, and
    /// breed the next generation.
    pub fn step(&mut self, threads: usize) -> Generation {
        let scores = self.evaluate(threads);
        let n = self.population.len();
        let mut rank: Vec<usize> = (0..n).collect();
        rank.sort_by(|&a, &b| {
            scores[b]
                .fitness
                .total_cmp(&scores[a].fitness)
                .then(scores[b].damage.total_cmp(&scores[a].damage))
                .then(a.cmp(&b))
        });
        let mut place = vec![0usize; n];
        for (p, &i) in rank.iter().enumerate() {
            place[i] = p;
        }
        let champion = self.population[rank[0]].clone();
        let mut mix = [0usize; 3];
        for g in &self.population {
            mix[g.behavior as usize] += 1;
        }
        let summary = Generation {
            index: self.generation,
            champion: champion.clone(),
            best: scores[rank[0]].clone(),
            mean_fitness: scores.iter().map(|s| s.fitness).sum::<f64>() / n as f64,
            mix,
        };

        self.hall_of_fame.push(champion);
        let excess = self
            .hall_of_fame
            .len()
            .saturating_sub(self.config.hall_of_fame);
        self.hall_of_fame.drain(..excess);

        let c = &self.config;
        let mut r = gen_rng(c.seed, self.generation + 1);
        let pick = |r: &mut ChaCha8Rng| {
            (0..c.tournament)
                .map(|_| below(r, n))
                .min_by_key(|&i| place[i])
                .expect("tournament >= 1")
        };
        let mut next: Vec<Genome> = rank[..c.elite]
            .iter()
            .map(|&i| self.population[i].clone())
            .collect();
        while next.len() < n {
            let a = pick(&mut r);
            let b = pick(&mut r);
            let (pa, pb) = (&self.population[a], &self.population[b]);
            let mut child = pa.clone();
            if a != b && pa.behavior == pb.behavior {
                for (x, y) in child.genes.iter_mut().zip(&pb.genes) {
                    if unit(&mut r) < 0.5 {
                        *x = *y;
                    }
                }
                if unit(&mut r) < 0.5 {
                    child.loadout = pb.loadout;
                }
            }
            mutate(&mut child, c, &mut r);
            next.push(child);
        }
        self.population = next;
        self.generation += 1;
        summary
    }

    /// State file JSON (compact genomes, gene names listed once per behavior).
    pub fn to_json(&self) -> Value {
        let names: serde_json::Map<String, Value> = Behavior::ALL
            .iter()
            .map(|&b| {
                let n: Vec<Value> = gene_infos(b).iter().map(|i| json!(i.name)).collect();
                (b.key().to_string(), Value::Array(n))
            })
            .collect();
        json!({
            "format": FORMAT,
            "config": self.config.to_json(),
            "generation": self.generation,
            "genes": names,
            "population": self.population.iter().map(Genome::to_compact).collect::<Vec<_>>(),
            "hall_of_fame": self.hall_of_fame.iter().map(Genome::to_compact).collect::<Vec<_>>(),
        })
    }

    /// Parse [`State::to_json`] output; the gene tables must match this build's.
    pub fn from_json(v: &Value) -> Result<Self, String> {
        if v.get("format").and_then(Value::as_u64) != Some(FORMAT as u64) {
            return Err(format!("state format must be {FORMAT}"));
        }
        for b in Behavior::ALL {
            let want: Vec<&str> = gene_infos(b).iter().map(|i| i.name).collect();
            let got: Option<Vec<&str>> = v["genes"][b.key()]
                .as_array()
                .map(|a| a.iter().filter_map(Value::as_str).collect());
            if got.as_ref() != Some(&want) {
                return Err(format!("{b}: gene table differs from this build"));
            }
        }
        let list = |k: &str| -> Result<Vec<Genome>, String> {
            v.get(k)
                .and_then(Value::as_array)
                .ok_or_else(|| format!("missing {k}"))?
                .iter()
                .map(Genome::from_compact)
                .collect()
        };
        let s = Self {
            config: Config::from_json(&v["config"])?,
            generation: v
                .get("generation")
                .and_then(Value::as_u64)
                .ok_or("missing generation")? as u32,
            population: list("population")?,
            hall_of_fame: list("hall_of_fame")?,
        };
        if s.population.len() != s.config.population {
            return Err("population size differs from config".into());
        }
        Ok(s)
    }
}

/// Wins, draws and losses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Wdl {
    /// Wins.
    pub wins: u32,
    /// Draws.
    pub draws: u32,
    /// Losses.
    pub losses: u32,
}

impl Wdl {
    /// Matches played.
    pub fn games(&self) -> u32 {
        self.wins + self.draws + self.losses
    }
}

/// A champion's held-out result against Gen 0.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeldOut {
    /// Seeds used (`HELDOUT_SEED_BASE..+seeds`), each vs each opponent from both sides.
    pub seeds: u64,
    /// Results vs Charger, Kiter, Sniper (Gen 0).
    pub per_opponent: [Wdl; 3],
    /// FNV-1a over every match's (opponent, seed, side, winner, ticks, state hash).
    pub digest: u64,
}

impl HeldOut {
    /// Total matches.
    pub fn games(&self) -> u32 {
        self.per_opponent.iter().map(Wdl::games).sum()
    }
    /// Total wins.
    pub fn wins(&self) -> u32 {
        self.per_opponent.iter().map(|w| w.wins).sum()
    }
    /// Wins / matches (equal matches per opponent, so this is also the average).
    pub fn win_rate(&self) -> f64 {
        self.wins() as f64 / self.games().max(1) as f64
    }
    /// Promotion status from the plan's rules.
    pub fn status(&self) -> &'static str {
        let w = self.win_rate();
        if w > HOLD_WIN_RATE {
            "experimental"
        } else if w >= ACCEPT_WIN_RATE {
            "promotable"
        } else {
            "below-bar"
        }
    }

    /// JSON object (the digest as a hex string).
    pub fn to_json(&self) -> Value {
        let per: serde_json::Map<String, Value> = Behavior::ALL
            .iter()
            .zip(&self.per_opponent)
            .map(|(b, w)| (b.key().to_string(), json!([w.wins, w.draws, w.losses])))
            .collect();
        json!({
            "seeds": self.seeds,
            "seed_base": HELDOUT_SEED_BASE.to_string(),
            "matches": self.games(),
            "wins": self.wins(),
            "win_rate": round4(self.win_rate()),
            "vs_wdl": per,
            "digest": format!("{:016x}", self.digest),
            "status": self.status(),
        })
    }
}

fn fnv(h: &mut u64, bytes: &[u8]) {
    for &b in bytes {
        *h ^= b as u64;
        *h = h.wrapping_mul(0x100_0000_01b3);
    }
}

/// Play `g` against the three Gen 0 policies on `seeds` held-out seeds, both sides.
pub fn held_out(g: &Genome, seeds: u64, threads: usize) -> HeldOut {
    let field = scripted_field();
    let jobs: Vec<(usize, u64, usize)> = (0..3)
        .flat_map(|o| (0..seeds).flat_map(move |s| (0..2).map(move |side| (o, s, side))))
        .collect();
    let games = par_map(&jobs, threads, |&(o, s, side)| {
        play(g, &field[o], HELDOUT_SEED_BASE + s, side)
    });
    let mut per = [Wdl::default(); 3];
    let mut digest = 0xcbf2_9ce4_8422_2325u64;
    for (&(o, s, side), x) in jobs.iter().zip(&games) {
        let w = &mut per[o];
        if x.win {
            w.wins += 1;
        } else if x.draw {
            w.draws += 1;
        } else {
            w.losses += 1;
        }
        let winner = x.duel.winner.unwrap_or(2u8);
        fnv(&mut digest, &[o as u8, side as u8, winner]);
        fnv(&mut digest, &(HELDOUT_SEED_BASE + s).to_le_bytes());
        fnv(&mut digest, &x.duel.ticks.to_le_bytes());
        fnv(&mut digest, &x.duel.hash.to_le_bytes());
    }
    HeldOut {
        seeds,
        per_opponent: per,
        digest,
    }
}

/// Round to 4 decimals (for small, stable JSON).
pub fn round4(x: f64) -> f64 {
    (x * 1e4).round() / 1e4
}

impl Generation {
    /// One history entry.
    pub fn to_json(&self) -> Value {
        json!({
            "gen": self.index,
            "best": round4(self.best.fitness),
            "mean": round4(self.mean_fitness),
            "champion": self.champion.label(),
            "vs_scripted": round4(self.best.vs_scripted),
            "mix": self.mix,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MatchSpec, TankSpec};

    fn tiny() -> Config {
        Config {
            seed: 7,
            population: 6,
            elite: 1,
            tournament: 2,
            train_seeds: 1,
            hall_of_fame: 2,
            ..Config::default()
        }
    }

    #[test]
    fn scripted_duel_matches_the_shared_match_spec() {
        let spec = MatchSpec {
            seed: 42,
            blue: TankSpec::new(Behavior::Kiter, Loadout::DEFAULT),
            orange: TankSpec::new(Behavior::Charger, Loadout::DEFAULT),
        };
        let (o, h) = spec.run();
        let d = duel(
            &Genome::scripted(Behavior::Kiter),
            &Genome::scripted(Behavior::Charger),
            42,
        );
        assert_eq!((d.winner, d.ticks, d.hash), (o.winner, o.ticks, h));
    }

    #[test]
    fn points_follow_the_published_formula() {
        assert_eq!((WIN_POINTS, DRAW_POINTS, LOSS_POINTS), (1.0, 0.25, 0.0));
        let k = Genome::scripted(Behavior::Kiter);
        for seed in [1u64, 2, 3] {
            for side in 0..2 {
                let g = play(&k, &k, seed, side);
                let want = match g.duel.winner {
                    None => 0.25,
                    Some(t) if t as usize == side => 1.0,
                    Some(_) => 0.0,
                };
                assert_eq!(g.points, want);
            }
        }
    }

    #[test]
    fn nudged_loadouts_stay_valid_and_reach_every_split() {
        let mut r = ChaCha8Rng::seed_from_u64(3);
        let mut seen = std::collections::BTreeSet::new();
        let mut l = Loadout::DEFAULT;
        for _ in 0..2_000 {
            l = nudge_loadout(l, &mut r);
            assert!(Loadout::ALL.contains(&l));
            seen.insert(l.to_string());
        }
        assert_eq!(seen.len(), 19);
    }

    #[test]
    fn par_map_order_does_not_depend_on_threads() {
        let xs: Vec<u64> = (0..97).collect();
        let one = par_map(&xs, 1, |x| x * x);
        for t in [2, 3, 8, 200] {
            assert_eq!(par_map(&xs, t, |x| x * x), one);
        }
    }

    #[test]
    fn gen0_holds_the_scripted_policies_and_valid_mutants() {
        let s = State::new(Config::default()).unwrap();
        assert_eq!(s.population.len(), 32);
        assert_eq!(s.population[..3], scripted_field());
        for g in &s.population[3..] {
            let mut r = g.clone();
            r.repair();
            assert_eq!(&r, g);
            assert_ne!(g, &Genome::scripted(g.behavior));
        }
    }

    #[test]
    fn same_seed_same_generations_any_thread_count_and_resume() {
        let run = |threads, stop_at: Option<u32>| {
            let mut s = State::new(tiny()).unwrap();
            let mut out = Vec::new();
            for _ in 0..2 {
                if stop_at == Some(s.generation) {
                    s = State::from_json(&s.to_json()).unwrap();
                }
                out.push(s.step(threads));
            }
            (out, s)
        };
        let (a, sa) = run(1, None);
        let (b, sb) = run(4, Some(1));
        assert_eq!(a, b);
        assert_eq!(sa, sb);
        assert_eq!(sa.hall_of_fame.len(), 2);
        let h = held_out(&a[1].champion, 2, 1);
        assert_eq!(h, held_out(&b[1].champion, 2, 3));
        assert_eq!(h.games(), 12);
        let other = State::new(Config { seed: 8, ..tiny() }).unwrap();
        assert_ne!(other.population, State::new(tiny()).unwrap().population);
    }

    #[test]
    fn state_json_round_trips() {
        let mut s = State::new(tiny()).unwrap();
        s.step(2);
        let back = State::from_json(&s.to_json()).unwrap();
        assert_eq!(back, s);
        let mut v = s.to_json();
        v["genes"]["kiter"][0] = json!("renamed");
        assert!(State::from_json(&v).is_err());
    }

    #[test]
    fn promotion_status_thresholds() {
        let mk = |wins, losses| HeldOut {
            seeds: 1,
            per_opponent: [
                Wdl {
                    wins,
                    draws: 0,
                    losses,
                },
                Wdl::default(),
                Wdl::default(),
            ],
            digest: 0,
        };
        assert_eq!(mk(64, 36).status(), "below-bar");
        assert_eq!(mk(65, 35).status(), "promotable");
        assert_eq!(mk(70, 30).status(), "promotable");
        assert_eq!(mk(71, 29).status(), "experimental");
    }
}
