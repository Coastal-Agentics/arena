//! Tank Arena balance experiments (docs/design/tank-balance-2026-10.md): why the
//! nightly Gen 499 champion (charger-3-4-2, evolved params) wins 99.35% against Gen 0,
//! and what small rule or catalog change would bring it back into the 70% build cap.
//!
//! Nothing here changes the rules on `main`: every variant is applied inside this
//! binary, either as per-tank `TankParams` (the level tables) or as a change to the
//! genome a tank plays (a gene cap). The `baseline` variant is today's rules exactly
//! (checked against `tank::evolve::duel` at start-up).
//!
//! ```text
//! cargo run -p tank --release --example balance_variants -- diagnose [seeds]
//! cargo run -p tank --release --example balance_variants -- round-robin [seeds] [--retune] [variant…]
//! cargo run -p tank --release --example balance_variants -- list
//! ```
//!
//! Defaults: 500 seeds per pairing, each played from both sides (1,000 games). Win
//! rate = wins / games; draws count as not winning. Deterministic: same code, same
//! numbers on any thread count.

use engine::{Action, Match, Observation, Policy, TankParams};
use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde_json::Value;
use std::fmt::Write as _;
use tank::evolve::{duel, gene_infos, nudge_loadout, par_map, Genome};
use tank::loadout::FIRE_COOLDOWN;
use tank::matchup::{BLUE_SALT, ORANGE_SALT};
use tank::rules::{config, Mode};
use tank::{Behavior, Loadout, Preset};

/// Seeds for these experiments, away from training (1e6) and held-out (9e9) seeds.
const SEED_BASE: u64 = 20_000_000;

/// The nightly Gen 499 champion, copied from `web/data/evolution/champion.json` on `main`
/// at `c538b47` (the nightly rewrites that file, so the experiment keeps its own copy).
const CHAMPION_499: &str = r#"{"commit":"71f7c6cdef1a8273bfe1af0cf1e492a83e977893","config":{"elite":4,"gene_rate":0.25,"hall_of_fame":8,"loadout_rate":0.3,"population":32,"seed":"1","sigma":0.1,"tournament":3,"train_seeds":8},"fitness_function":"mean over matches of (win 1, draw 0.25, loss 0); opponents: Charger, Kiter and Sniper at defaults and 3/3/3, plus the last 8 champions; fixed training seeds, both spawn sides; ties: more damage dealt, then lower population index","format":1,"generation":499,"genome":{"behavior":"charger","loadout":"3-4-2","params":{"aim_tol":0.128,"dodge_chance":0.645,"dodge_horizon":60.0,"dodge_margin":11.738,"route_margin":0.522,"stall.min_speed":0.619,"stall.reverse_ticks":5,"stall.stuck_ticks":3,"steer_tol":0.589,"stop_dist":20.0,"weave_deg":0.0,"weave_jitter":12,"weave_period":120,"weave_until":12.916}},"held_out":{"digest":"623218a3ca642a64","matches":6000,"seed_base":"9000000000","seeds":1000,"status":"experimental","vs_wdl":{"charger":[2000,0,0],"kiter":[1985,0,15],"sniper":[1976,2,22]},"win_rate":0.9935,"wins":5961},"label":"charger-3-4-2","promotion":"win rate vs Gen 0 >= 65% to promote; above 70% the champion is held from promotion and labeled experimental, and the CoS tells Nye","training":{"damage":0.9883,"fitness":0.7642,"matches":176,"seed_base":"1000000","vs_scripted":1.0}}"#;
const CHAMPION_M1: &str = include_str!("../tests/fixtures/m1-champion.json");
const CHAMPION_PRE_DODGE: &str =
    include_str!("../../../web/data/evolution/archive/pre-dodge-2026-10-02/champion.json");

// ---------------------------------------------------------------------------------
// Variants

/// A rules variant: the params a loadout plays with, and a change to the genome a
/// tank plays (identity for table-only variants).
struct Variant {
    key: &'static str,
    title: &'static str,
    params: fn(Loadout, &TankParams) -> TankParams,
    genome: fn(&Genome) -> Genome,
    /// Minimum firing range (u, centre to centre; 0 = none): a tank's fire command is
    /// ignored on a tick that starts with an enemy tank closer than this. Simulated
    /// exactly by [`MinRange`] around every policy, since the decision only reads the
    /// tick's observation.
    min_range: f32,
}

/// The minimum-firing-range rule as a wrapper: drops `fire` when the nearest enemy
/// (observations list enemies nearest first) is closer than `range`.
struct MinRange {
    inner: Box<dyn Policy>,
    range: f32,
}

impl Policy for MinRange {
    fn act(&mut self, obs: &Observation) -> Action {
        let mut a = self.inner.act(obs);
        if let Some(e) = obs.enemies.first() {
            if e.dist_sq < self.range * self.range {
                a.fire = false;
            }
        }
        a
    }
}

fn same(g: &Genome) -> Genome {
    g.clone()
}

/// Today's tables ([`Loadout::apply`]).
fn tables_now(l: Loadout, base: &TankParams) -> TankParams {
    l.apply(base)
}

fn idx(level: u8) -> usize {
    (level - 1) as usize
}

/// A: reload no longer comes from Speed: every tank reloads in 45 ticks (the old 3/3/3
/// value); Speed buys movement only.
fn tables_reload_flat(l: Loadout, base: &TankParams) -> TankParams {
    TankParams {
        fire_cooldown: 45,
        ..l.apply(base)
    }
}

/// B: reload moves from Speed to Attack: the fire-cooldown table is indexed by the
/// Attack level, so Speed buys movement only and Attack buys damage per second.
fn tables_reload_on_attack(l: Loadout, base: &TankParams) -> TankParams {
    TankParams {
        fire_cooldown: FIRE_COOLDOWN[idx(l.attack())],
        ..l.apply(base)
    }
}

/// Speed's reload table flattened by half: 50/47/45/43/41 instead of 64/54/45/37/31.
const RELOAD_HALF: [u32; 5] = [52, 48, 45, 42, 39];
fn tables_reload_half(l: Loadout, base: &TankParams) -> TankParams {
    TankParams {
        fire_cooldown: RELOAD_HALF[idx(l.speed())],
        ..l.apply(base)
    }
}

/// Defense table steeper at the low end.
const HP_STEEP: [i32; 5] = [400, 520, 650, 800, 960];
fn tables_hp_steep(l: Loadout, base: &TankParams) -> TankParams {
    TankParams {
        max_hp: HP_STEEP[idx(l.defense())],
        ..l.apply(base)
    }
}

/// Speed table flattened (movement only): 105/112/120/127/135 u/s and the matching turn.
const SPEED_FLAT: [f32; 5] = [105.0, 112.0, 120.0, 127.0, 135.0];
const TURN_FLAT: [u16; 5] = [318, 341, 364, 387, 410];
fn tables_speed_flat(l: Loadout, base: &TankParams) -> TankParams {
    TankParams {
        max_speed: SPEED_FLAT[idx(l.speed())],
        turn_rate: TURN_FLAT[idx(l.speed())],
        ..l.apply(base)
    }
}

fn set_gene(g: &Genome, name: &str, v: f32) -> Genome {
    let mut g = g.clone();
    if let Some(i) = gene_infos(g.behavior).iter().position(|x| x.name == name) {
        g.genes[i] = v;
    }
    g
}
fn gene(g: &Genome, name: &str) -> Option<f32> {
    gene_infos(g.behavior)
        .iter()
        .position(|x| x.name == name)
        .map(|i| g.genes[i])
}
fn cap_gene(g: &Genome, name: &str, hi: f32) -> Genome {
    match gene(g, name) {
        Some(v) if v > hi => set_gene(g, name, hi),
        _ => g.clone(),
    }
}

/// C: the Charger's dodge strength cap drops from 0.65 to 0.40 (scripted Charger too).
fn genome_charger_dodge_040(g: &Genome) -> Genome {
    if g.behavior == Behavior::Charger {
        cap_gene(g, "dodge_chance", 0.40)
    } else {
        g.clone()
    }
}

fn floor_gene(g: &Genome, name: &str, lo: f32) -> Genome {
    match gene(g, name) {
        Some(v) if v < lo => set_gene(g, name, lo),
        _ => g.clone(),
    }
}

/// C': Charger dodge cap 0.50.
fn genome_charger_dodge_050(g: &Genome) -> Genome {
    if g.behavior == Behavior::Charger {
        cap_gene(g, "dodge_chance", 0.50)
    } else {
        g.clone()
    }
}

/// D'': stop-distance floor 80 u.
fn genome_stop_floor_80(g: &Genome) -> Genome {
    floor_gene(g, "stop_dist", 80.0)
}

/// D + C': floor 60 and Charger dodge cap 0.50.
fn genome_stop60_dodge050(g: &Genome) -> Genome {
    genome_charger_dodge_050(&genome_stop_floor_60(g))
}

/// E: stop floor 60 and the Charger's steer tolerance capped at 0.3 (scripted 0.2).
fn genome_stop60_steer03(g: &Genome) -> Genome {
    cap_gene(&genome_stop_floor_60(g), "steer_tol", 0.3)
}

/// E'': stop floor 60 and steer tolerance capped at the shipped 0.2.
fn genome_stop60_steer02(g: &Genome) -> Genome {
    cap_gene(&genome_stop_floor_60(g), "steer_tol", 0.2)
}

/// E': E and the dodge look-ahead capped at 30 ticks.
fn genome_stop60_steer03_h30(g: &Genome) -> Genome {
    cap_gene(&genome_stop60_steer03(g), "dodge_horizon", 30.0)
}

/// Top of Speed's reload table flattened: 64/54/45/41/38.
const RELOAD_TOP: [u32; 5] = [64, 54, 45, 41, 38];
fn tables_reload_top(l: Loadout, base: &TankParams) -> TankParams {
    TankParams {
        fire_cooldown: RELOAD_TOP[idx(l.speed())],
        ..l.apply(base)
    }
}

/// D: the Charger's stop distance can't go below 60 u (the scripted value); today's
/// gene floor is 20 u, which is two tanks touching (radius 16 each).
fn genome_stop_floor_60(g: &Genome) -> Genome {
    floor_gene(g, "stop_dist", 60.0)
}

/// D': stop-distance floor 40 u.
fn genome_stop_floor_40(g: &Genome) -> Genome {
    floor_gene(g, "stop_dist", 40.0)
}

fn variants() -> Vec<Variant> {
    vec![
        Variant {
            key: "baseline",
            title: "Today's rules",
            params: tables_now,
            genome: same,
            min_range: 0.0,
        },
        Variant {
            key: "reload-flat",
            title: "Reload fixed at 45 ticks for every build (Speed buys movement only)",
            params: tables_reload_flat,
            genome: same,
            min_range: 0.0,
        },
        Variant {
            key: "reload-on-attack",
            title: "Reload table moves from Speed to Attack",
            params: tables_reload_on_attack,
            genome: same,
            min_range: 0.0,
        },
        Variant {
            key: "reload-half",
            title: "Speed's reload table flattened by half (52/48/45/42/39)",
            params: tables_reload_half,
            genome: same,
            min_range: 0.0,
        },
        Variant {
            key: "hp-steep",
            title: "Defense table steeper (400/520/650/800/960)",
            params: tables_hp_steep,
            genome: same,
            min_range: 0.0,
        },
        Variant {
            key: "speed-flat",
            title: "Speed table flattened (105–135 u/s)",
            params: tables_speed_flat,
            genome: same,
            min_range: 0.0,
        },
        Variant {
            key: "charger-dodge-040",
            title: "Charger dodge strength cap 0.65 → 0.40",
            params: tables_now,
            genome: genome_charger_dodge_040,
            min_range: 0.0,
        },
        Variant {
            key: "charger-dodge-050",
            title: "Charger dodge strength cap 0.65 → 0.50",
            params: tables_now,
            genome: genome_charger_dodge_050,
            min_range: 0.0,
        },
        Variant {
            key: "stop-floor-80",
            title: "Charger stop-distance gene floor 20 → 80 u",
            params: tables_now,
            genome: genome_stop_floor_80,
            min_range: 0.0,
        },
        Variant {
            key: "stop60+dodge050",
            title: "Stop-distance floor 60 u and Charger dodge cap 0.50",
            params: tables_now,
            genome: genome_stop60_dodge050,
            min_range: 0.0,
        },
        Variant {
            key: "stop60+steer03",
            title: "Stop-distance floor 60 u and steer tolerance cap 0.3",
            params: tables_now,
            genome: genome_stop60_steer03,
            min_range: 0.0,
        },
        Variant {
            key: "stop60+steer02",
            title: "Stop-distance floor 60 u and steer tolerance cap 0.2 (both the shipped values)",
            params: tables_now,
            genome: genome_stop60_steer02,
            min_range: 0.0,
        },
        Variant {
            key: "stop60+steer03+h30",
            title: "Stop floor 60 u, steer cap 0.3, dodge look-ahead cap 30",
            params: tables_now,
            genome: genome_stop60_steer03_h30,
            min_range: 0.0,
        },
        Variant {
            key: "reload-top",
            title: "Top of Speed's reload table flattened (… 45/41/38)",
            params: tables_reload_top,
            genome: same,
            min_range: 0.0,
        },
        Variant {
            key: "reload-top+stop60",
            title: "Reload top flattened and stop-distance floor 60 u",
            params: tables_reload_top,
            genome: genome_stop_floor_60,
            min_range: 0.0,
        },
        Variant {
            key: "min-range-40",
            title: "No firing with an enemy closer than 40 u (tanks touch at 32 u)",
            params: tables_now,
            genome: same,
            min_range: 40.0,
        },
        Variant {
            key: "min-range-50",
            title: "No firing with an enemy closer than 50 u",
            params: tables_now,
            genome: same,
            min_range: 50.0,
        },
        Variant {
            key: "min-range-50+stop60",
            title: "Minimum firing range 50 u and stop-distance floor 60 u",
            params: tables_now,
            genome: genome_stop_floor_60,
            min_range: 50.0,
        },
        Variant {
            key: "min-range-40+stop60",
            title: "Minimum firing range 40 u and stop-distance floor 60 u",
            params: tables_now,
            genome: genome_stop_floor_60,
            min_range: 40.0,
        },
        Variant {
            key: "min-range-45+stop60",
            title: "Minimum firing range 45 u and stop-distance floor 60 u",
            params: tables_now,
            genome: genome_stop_floor_60,
            min_range: 45.0,
        },
        Variant {
            key: "stop-floor-60",
            title: "Charger stop-distance gene floor 20 → 60 u",
            params: tables_now,
            genome: genome_stop_floor_60,
            min_range: 0.0,
        },
        Variant {
            key: "stop-floor-40",
            title: "Charger stop-distance gene floor 20 → 40 u",
            params: tables_now,
            genome: genome_stop_floor_40,
            min_range: 0.0,
        },
    ]
}

// ---------------------------------------------------------------------------------
// Entrants and matches

#[derive(Clone)]
struct Entrant {
    name: String,
    genome: Genome,
    preset: bool,
}

fn champion(json: &str) -> Genome {
    let v: Value = serde_json::from_str(json).expect("champion json");
    let g = &v["genome"];
    match Genome::from_json(g) {
        Ok(g) => g,
        Err(_) => {
            // An older genome (before the dodge_chance gene): fill missing genes from
            // the scripted defaults, i.e. today's cap.
            let b: Behavior = g["behavior"].as_str().unwrap().parse().unwrap();
            let mut out = Genome::scripted(b);
            out.loadout = g["loadout"].as_str().unwrap().parse().unwrap();
            for (i, info) in gene_infos(b).iter().enumerate() {
                if let Some(x) = g["params"][info.name].as_f64() {
                    out.genes[i] = x as f32;
                }
            }
            out.repair();
            out
        }
    }
}

fn scripted(b: Behavior, l: Loadout) -> Genome {
    let mut g = Genome::scripted(b);
    g.loadout = l;
    g
}

/// The field: every preset with each scripted behavior, plus the evolved champions.
fn field() -> Vec<Entrant> {
    let mut v = Vec::new();
    for p in Preset::ALL {
        for b in Behavior::ALL {
            v.push(Entrant {
                name: format!("{} {} ({})", p.name(), b.name(), p.loadout()),
                genome: scripted(b, p.loadout()),
                preset: true,
            });
        }
    }
    for (name, json) in [
        ("Gen 499 champion", CHAMPION_499),
        ("M1 Scout champion", CHAMPION_M1),
        ("Pre-dodge Glass Cannon champion", CHAMPION_PRE_DODGE),
    ] {
        let g = champion(json);
        v.push(Entrant {
            name: format!("{name} ({})", g.label()),
            genome: g,
            preset: false,
        });
    }
    v
}

/// One duel under `v`: `a` is Blue (tank 0), `b` Orange. Returns the winner.
fn play(v: &Variant, a: &Genome, b: &Genome, seed: u64) -> Option<u8> {
    let mut cfg = config(Mode::Duel);
    let base = cfg.params.clone();
    for (spawn, g) in cfg.tanks.iter_mut().zip([a, b]) {
        let p = (v.params)(g.loadout, &base);
        spawn.params = (p != base).then_some(p);
    }
    let (ga, gb) = ((v.genome)(a), (v.genome)(b));
    let mut m = Match::new(cfg, seed);
    let wrap = |p: Box<dyn Policy>| -> Box<dyn Policy> {
        if v.min_range > 0.0 {
            Box::new(MinRange {
                inner: p,
                range: v.min_range,
            })
        } else {
            p
        }
    };
    let mut pa = wrap(ga.build(seed ^ BLUE_SALT));
    let mut pb = wrap(gb.build(seed ^ ORANGE_SALT));
    m.run(&mut [pa.as_mut(), pb.as_mut()]).winner
}

#[derive(Clone, Copy, Default, Debug)]
struct Wdl {
    w: u32,
    d: u32,
    l: u32,
}
impl Wdl {
    fn games(&self) -> u32 {
        self.w + self.d + self.l
    }
    fn rate(&self) -> f64 {
        100.0 * self.w as f64 / self.games().max(1) as f64
    }
    fn add(&mut self, o: Wdl) {
        self.w += o.w;
        self.d += o.d;
        self.l += o.l;
    }
}

/// `a` against each of `opps` over `seeds` seeds × both sides, from `a`'s view.
fn versus(v: &Variant, a: &Genome, opps: &[Genome], seeds: u64) -> Vec<Wdl> {
    let jobs: Vec<(usize, u64, u8)> = (0..opps.len())
        .flat_map(|o| (0..seeds).flat_map(move |s| [(o, s, 0u8), (o, s, 1u8)]))
        .collect();
    let res = par_map(&jobs, threads(), |&(o, s, side)| {
        let seed = SEED_BASE + s;
        let w = if side == 0 {
            play(v, a, &opps[o], seed)
        } else {
            play(v, &opps[o], a, seed)
        };
        match w {
            None => 0i8,
            Some(t) if t == side => 1,
            Some(_) => -1,
        }
    });
    let mut out = vec![Wdl::default(); opps.len()];
    for (&(o, _, _), r) in jobs.iter().zip(res) {
        match r {
            1 => out[o].w += 1,
            0 => out[o].d += 1,
            _ => out[o].l += 1,
        }
    }
    out
}

fn threads() -> usize {
    std::thread::available_parallelism().map_or(4, |n| n.get())
}

/// The baseline variant must be today's rules: same results as `tank::evolve::duel`.
fn check_baseline(f: &[Entrant]) {
    let v = &variants()[0];
    for (i, a) in f.iter().enumerate() {
        let b = &f[(i + 5) % f.len()];
        for seed in [SEED_BASE, SEED_BASE + 1] {
            assert_eq!(
                play(v, &a.genome, &b.genome, seed),
                duel(&a.genome, &b.genome, seed).winner,
                "baseline differs from tank::evolve::duel: {} vs {}",
                a.name,
                b.name
            );
        }
    }
}

// ---------------------------------------------------------------------------------
// Re-tuning: what would evolution find under a variant?

/// Seeds for re-tuning fitness (disjoint from the report seeds).
const RETUNE_SEED_BASE: u64 = 30_000_000;
/// Seeds per Gen 0 opponent per fitness evaluation (both sides).
const RETUNE_SEEDS: u64 = 40;
/// Hill-climb steps per start.
const RETUNE_STEPS: u32 = 300;

/// Evolution's fitness (win 1, draw 0.25, loss 0) against Gen 0, under `v`.
fn fitness(v: &Variant, g: &Genome) -> f64 {
    let opps = gen0();
    let jobs: Vec<(usize, u64, u8)> = (0..3)
        .flat_map(|o| (0..RETUNE_SEEDS).flat_map(move |s| [(o, s, 0u8), (o, s, 1u8)]))
        .collect();
    let pts = par_map(&jobs, threads(), |&(o, s, side)| {
        let seed = RETUNE_SEED_BASE + s;
        let w = if side == 0 {
            play(v, g, &opps[o], seed)
        } else {
            play(v, &opps[o], g, seed)
        };
        match w {
            None => 0.25,
            Some(t) if t == side => 1.0,
            Some(_) => 0.0,
        }
    });
    pts.iter().sum::<f64>() / pts.len() as f64
}

fn gauss(r: &mut ChaCha8Rng) -> f64 {
    let u = |r: &mut ChaCha8Rng| ((r.next_u32() >> 8) as f64 + 0.5) / (1u32 << 24) as f64;
    let (a, b) = (u(r), u(r));
    (-2.0 * a.ln()).sqrt() * (std::f64::consts::TAU * b).cos()
}

/// Hill-climb from `start` (as clamped by `v`): mutate a quarter of the genes by
/// 10% of their range (and the loadout 30% of the time, to a neighbour), keep the
/// mutant if its fitness is at least as good. Evolution's GA in miniature, enough to
/// show whether a variant just clamps today's champion or actually closes the hole.
fn retune(v: &Variant, start: &Genome, seed: u64) -> (Genome, f64) {
    let mut r = ChaCha8Rng::seed_from_u64(seed);
    let mut best = (v.genome)(start);
    let mut best_f = fitness(v, &best);
    let infos = gene_infos(best.behavior);
    for _ in 0..RETUNE_STEPS {
        let mut g = best.clone();
        let mut changed = false;
        for (i, info) in infos.iter().enumerate() {
            if (r.next_u32() % 4) == 0 {
                let range = (info.hi - info.lo) as f64;
                g.genes[i] = (g.genes[i] as f64 + 0.1 * range * gauss(&mut r)) as f32;
                changed = true;
            }
        }
        if r.next_u32() % 10 < 3 {
            g.loadout = nudge_loadout(g.loadout, &mut r);
            changed = true;
        }
        if !changed {
            continue;
        }
        g.repair();
        let g = (v.genome)(&g);
        let f = fitness(v, &g);
        if f >= best_f {
            best = g;
            best_f = f;
        }
    }
    (best, best_f)
}

// ---------------------------------------------------------------------------------
// Reports

fn gen0() -> Vec<Genome> {
    Behavior::ALL.map(Genome::scripted).to_vec()
}

fn presets_field() -> Vec<Genome> {
    field()
        .into_iter()
        .filter(|e| e.preset)
        .map(|e| e.genome)
        .collect()
}

fn diag_row(out: &mut String, v: &Variant, name: &str, g: &Genome, seeds: u64) {
    let g0 = versus(v, g, &gen0(), seeds);
    let pf = versus(v, g, &presets_field(), seeds);
    let mut a = Wdl::default();
    g0.iter().for_each(|x| a.add(*x));
    let mut b = Wdl::default();
    pf.iter().for_each(|x| b.add(*x));
    let per: Vec<String> = g0.iter().map(|x| format!("{:.1}", x.rate())).collect();
    writeln!(
        out,
        "| {name} | {} | {:.1}% | {} | {:.1}% |",
        g.label(),
        a.rate(),
        per.join(" / "),
        b.rate()
    )
    .unwrap();
    eprintln!("  {name}: gen0 {:.1}% field {:.1}%", a.rate(), b.rate());
}

fn diag_header(out: &mut String, title: &str) {
    writeln!(out, "\n### {title}\n").unwrap();
    writeln!(
        out,
        "| Tank | Genome | vs Gen 0 | vs Charger / Kiter / Sniper | vs 12 preset tanks |\n|---|---|---|---|---|"
    )
    .unwrap();
}

fn diagnose(seeds: u64) -> String {
    let base = &variants()[0];
    let champ = champion(CHAMPION_499);
    let mut out = String::new();
    writeln!(
        out,
        "## Diagnosis ({seeds} seeds per pairing, both sides)\n\n\"vs Gen 0\" is the nightly's held-out test (scripted Charger, Kiter and Sniper at 3-3-3) on this harness's seeds; \"vs 12 preset tanks\" is every preset with every scripted behavior."
    )
    .unwrap();

    let v: Value = serde_json::from_str(CHAMPION_499).unwrap();
    let h = tank::evolve::held_out(&champ, 1000, threads());
    writeln!(
        out,
        "\nThe Gen 499 champion (copied from `web/data/evolution/champion.json` at `c538b47`) replays its nightly held-out test exactly: {} of {} ({:.2}%), digest `{:016x}` (file: {} wins, digest `{}`).",
        h.wins(),
        h.games(),
        100.0 * h.win_rate(),
        h.digest,
        v["held_out"]["wins"],
        v["held_out"]["digest"].as_str().unwrap_or("?")
    )
    .unwrap();
    diag_header(
        &mut out,
        "1. The real champion, and its build with other behaviors",
    );
    diag_row(&mut out, base, "Gen 499 champion", &champ, seeds);
    for b in Behavior::ALL {
        diag_row(
            &mut out,
            base,
            &format!("Scripted {} at 3-4-2", b.name()),
            &scripted(b, champ.loadout),
            seeds,
        );
    }

    diag_header(&mut out, "2. The champion's params on every loadout");
    for l in Loadout::ALL {
        let mut g = champ.clone();
        g.loadout = l;
        diag_row(
            &mut out,
            base,
            &format!("Champion params at {l}"),
            &g,
            seeds,
        );
    }

    let infos = gene_infos(Behavior::Charger);
    let def = Genome::scripted(Behavior::Charger);
    diag_header(
        &mut out,
        "3. Champion with one gene put back to the scripted default",
    );
    for (i, info) in infos.iter().enumerate() {
        let mut g = champ.clone();
        g.genes[i] = def.genes[i];
        diag_row(
            &mut out,
            base,
            &format!("{} {} → {}", info.name, champ.genes[i], def.genes[i]),
            &g,
            seeds,
        );
    }
    diag_header(
        &mut out,
        "4. Scripted Charger 3-4-2 with one champion gene added",
    );
    for (i, info) in infos.iter().enumerate() {
        let mut g = scripted(Behavior::Charger, champ.loadout);
        g.genes[i] = champ.genes[i];
        diag_row(
            &mut out,
            base,
            &format!("{} {} → {}", info.name, def.genes[i], champ.genes[i]),
            &g,
            seeds,
        );
    }
    diag_header(&mut out, "5. Champion with one gene group put back");
    let groups: [(&str, &[&str]); 5] = [
        (
            "dodge (horizon, margin, chance)",
            &["dodge_horizon", "dodge_margin", "dodge_chance"],
        ),
        (
            "approach (stop_dist, steer_tol, route_margin)",
            &["stop_dist", "steer_tol", "route_margin"],
        ),
        (
            "weave (deg, period, jitter, until)",
            &["weave_deg", "weave_period", "weave_jitter", "weave_until"],
        ),
        ("aim_tol", &["aim_tol"]),
        (
            "stall (3)",
            &[
                "stall.min_speed",
                "stall.stuck_ticks",
                "stall.reverse_ticks",
            ],
        ),
    ];
    for (name, genes) in groups {
        let mut g = champ.clone();
        for n in genes {
            g = set_gene(&g, n, gene(&def, n).unwrap());
        }
        diag_row(&mut out, base, name, &g, seeds);
    }
    diag_header(&mut out, "6. Champion's dodge strength and look-ahead");
    for c in [0.0, 0.25, 0.4, 0.5, 0.65] {
        diag_row(
            &mut out,
            base,
            &format!("dodge_chance {c}"),
            &set_gene(&champ, "dodge_chance", c),
            seeds,
        );
    }
    for h in [0.0, 15.0, 23.0, 30.0, 45.0, 60.0] {
        diag_row(
            &mut out,
            base,
            &format!("dodge_horizon {h}"),
            &set_gene(&champ, "dodge_horizon", h),
            seeds,
        );
    }

    diag_header(
        &mut out,
        "7. Rule ablations (each applied to every tank; Gen 0 is 3-3-3)",
    );
    for v in variants().iter().skip(1) {
        diag_row(&mut out, v, v.title, &champ, seeds);
    }
    out
}

fn round_robin(v: &Variant, seeds: u64, retuned: bool) -> String {
    let mut f = field();
    let mut note = String::new();
    if retuned {
        let starts = [
            ("Gen 499 champion", champion(CHAMPION_499)),
            ("M1 Scout champion", champion(CHAMPION_M1)),
            (
                "Glass Cannon Sniper",
                scripted(Behavior::Sniper, Preset::GlassCannon.loadout()),
            ),
            (
                "Balanced Kiter",
                scripted(Behavior::Kiter, Loadout::DEFAULT),
            ),
        ];
        writeln!(
            note,
            "Re-tuned tanks ({RETUNE_STEPS} hill-climb steps each against Gen 0 under this variant; training fitness = evolution's points per match):\n"
        )
        .unwrap();
        for (k, (from, start)) in starts.into_iter().enumerate() {
            let (g, fit) = retune(v, &start, 1 + k as u64);
            writeln!(
                note,
                "- from {from}: **{}**, fitness {:.3}, params `{}`",
                g.label(),
                fit,
                g.to_json()["params"]
            )
            .unwrap();
            eprintln!(
                "  {}: re-tuned from {from}: {} fitness {:.3}",
                v.key,
                g.label(),
                fit
            );
            f.push(Entrant {
                name: format!("Re-tuned from {from} ({})", g.label()),
                genome: g,
                preset: false,
            });
        }
        note.push('\n');
    }
    let n = f.len();
    let pairs: Vec<(usize, usize)> = (0..n)
        .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
        .collect();
    let jobs: Vec<(usize, u64, u8)> = (0..pairs.len())
        .flat_map(|p| (0..seeds).flat_map(move |s| [(p, s, 0u8), (p, s, 1u8)]))
        .collect();
    let res = par_map(&jobs, threads(), |&(p, s, side)| {
        let (i, j) = pairs[p];
        let seed = SEED_BASE + s;
        // side 0: i is Blue. Result from i's view.
        let w = if side == 0 {
            play(v, &f[i].genome, &f[j].genome, seed)
        } else {
            play(v, &f[j].genome, &f[i].genome, seed)
        };
        match w {
            None => 0i8,
            Some(t) if t == side => 1,
            Some(_) => -1,
        }
    });
    let mut m = vec![vec![Wdl::default(); n]; n];
    for (&(p, _, _), r) in jobs.iter().zip(res) {
        let (i, j) = pairs[p];
        match r {
            1 => {
                m[i][j].w += 1;
                m[j][i].l += 1;
            }
            0 => {
                m[i][j].d += 1;
                m[j][i].d += 1;
            }
            _ => {
                m[i][j].l += 1;
                m[j][i].w += 1;
            }
        }
    }
    let avg: Vec<f64> = (0..n)
        .map(|i| {
            let mut t = Wdl::default();
            (0..n).filter(|&j| j != i).for_each(|j| t.add(m[i][j]));
            t.rate()
        })
        .collect();
    let short = |e: &Entrant| -> String {
        if e.preset {
            let l = e.genome.loadout.to_string();
            format!("{}{}", &e.genome.behavior.name()[..1], l)
        } else {
            if e.name.starts_with("Re-tuned") {
                format!("☆{}{}", &e.genome.behavior.name()[..1], e.genome.loadout)
            } else {
                format!("★{}", e.genome.loadout)
            }
        }
    };
    let mut out = String::new();
    writeln!(out, "### `{}`: {}\n", v.key, v.title).unwrap();
    out.push_str(&note);
    // Summary first.
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| avg[b].partial_cmp(&avg[a]).unwrap());
    writeln!(
        out,
        "| Rank | Tank | Avg win % vs the field | Draws |\n|---|---|---|---|"
    )
    .unwrap();
    for (r, &i) in order.iter().enumerate() {
        let mut t = Wdl::default();
        (0..n).filter(|&j| j != i).for_each(|j| t.add(m[i][j]));
        writeln!(
            out,
            "| {} | {} | {:.1}% | {:.1}% |",
            r + 1,
            f[i].name,
            avg[i],
            100.0 * t.d as f64 / t.games().max(1) as f64
        )
        .unwrap();
    }
    let best_preset = |p: Preset| {
        (0..n)
            .filter(|&i| f[i].preset && f[i].genome.loadout == p.loadout())
            .map(|i| avg[i])
            .fold(0.0, f64::max)
    };
    let presets: Vec<String> = Preset::ALL
        .iter()
        .map(|&p| format!("{} {:.1}%", p.name(), best_preset(p)))
        .collect();
    let top = order[0];
    let low_preset = Preset::ALL
        .iter()
        .map(|&p| (p, best_preset(p)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap();
    writeln!(
        out,
        "\nTop: {} at {:.1}%. Best behavior per preset: {}. Lowest preset (best behavior): {} {:.1}%.\n",
        f[top].name,
        avg[top],
        presets.join(", "),
        low_preset.0.name(),
        low_preset.1
    )
    .unwrap();
    // The scripted counter triangle at 3-3-3 (BALANCE.md: every pairing 55–80%).
    let at = |b: Behavior| {
        (0..n)
            .find(|&i| {
                f[i].preset && f[i].genome.behavior == b && f[i].genome.loadout == Loadout::DEFAULT
            })
            .unwrap()
    };
    let (c, k, s) = (
        at(Behavior::Charger),
        at(Behavior::Kiter),
        at(Behavior::Sniper),
    );
    let tri = [m[k][c].rate(), m[c][s].rate(), m[s][k].rate()];
    let ok = tri.iter().all(|r| (55.0..=80.0).contains(r));
    writeln!(
        out,
        "Triangle at 3-3-3: Kiter > Charger {:.1}%, Charger > Sniper {:.1}%, Sniper > Kiter {:.1}% ({}).\n",
        tri[0],
        tri[1],
        tri[2],
        if ok { "holds, 55–80%" } else { "**broken**" }
    )
    .unwrap();
    // Full matrix.
    write!(
        out,
        "<details><summary>Full round robin (row's win % against column)</summary>\n\n| |"
    )
    .unwrap();
    for e in &f {
        write!(out, " {} |", short(e)).unwrap();
    }
    write!(out, "\n|---|").unwrap();
    for _ in &f {
        write!(out, "---|").unwrap();
    }
    writeln!(out).unwrap();
    for (i, row) in m.iter().enumerate() {
        write!(out, "| {} |", short(&f[i])).unwrap();
        for (j, cell) in row.iter().enumerate() {
            if i == j {
                write!(out, " – |").unwrap();
            } else {
                write!(out, " {:.0} |", cell.rate()).unwrap();
            }
        }
        writeln!(out).unwrap();
    }
    writeln!(out, "\n</details>\n").unwrap();
    eprintln!(
        "{}: top {} {:.1}%, lowest preset {} {:.1}%, triangle {:.1}/{:.1}/{:.1}",
        v.key,
        f[top].name,
        avg[top],
        low_preset.0.name(),
        low_preset.1,
        tri[0],
        tri[1],
        tri[2]
    );
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("list");
    let seeds: u64 = args.get(1).map_or(500, |s| s.parse().expect("seeds"));
    let f = field();
    check_baseline(&f);
    match cmd {
        "list" => {
            for v in variants() {
                println!("{:<24} {}", v.key, v.title);
            }
            for e in &f {
                println!("entrant: {} = {}", e.name, e.genome.to_json());
            }
        }
        "diagnose" => print!("{}", diagnose(seeds)),
        "round-robin" => {
            let retuned = args.iter().any(|a| a == "--retune");
            let want: Vec<&str> = args
                .iter()
                .skip(2)
                .map(String::as_str)
                .filter(|a| !a.starts_with("--"))
                .collect();
            let legend = "Short names: C/K/S = scripted Charger/Kiter/Sniper on that preset's loadout; ★ = an evolved champion (its loadout); ☆ = a tank re-tuned under the variant (behavior initial and loadout).";
            println!("## Round robins ({seeds} seeds per pairing, both sides)\n\n{legend}\n");
            for v in variants() {
                if want.is_empty() || want.contains(&v.key) {
                    print!("{}", round_robin(&v, seeds, retuned));
                }
            }
        }
        other => panic!("unknown command {other:?} (list, diagnose, round-robin)"),
    }
}
