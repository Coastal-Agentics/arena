//! The evolution genome (GATE-003 M1): which scripted policy, its tunable numbers, and
//! a loadout. Every number has published bounds (the tables below, which include the
//! three stall-recovery settings every policy shares); values are quantized to 0.001 (whole
//! numbers for tick counts) so they round-trip exactly through JSON.

use crate::loadout::Loadout;
use crate::policies::{Behavior, Charger, ChargerParams, Kiter, KiterParams, Sniper, SniperParams};
use engine::Policy;
use serde_json::{json, Map, Value};

/// Float genes are multiples of `1 / QUANT`.
pub const QUANT: f64 = 1000.0;

/// One tunable number: its name (`stall.min_speed` for nested fields), bounds, and
/// whether it is a whole number of ticks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeneInfo {
    /// Field name, as in the params struct.
    pub name: &'static str,
    /// Lowest allowed value.
    pub lo: f32,
    /// Highest allowed value.
    pub hi: f32,
    /// A `u32` field (ticks); otherwise an `f32`.
    pub int: bool,
}

struct Gene<P> {
    info: GeneInfo,
    get: fn(&P) -> f32,
    set: fn(&mut P, f32),
}

macro_rules! fg {
    ($t:ty, $name:literal, $($f:ident).+, $lo:expr, $hi:expr) => {
        Gene {
            info: GeneInfo { name: $name, lo: $lo, hi: $hi, int: false },
            get: |p: &$t| p.$($f).+,
            set: |p: &mut $t, v| p.$($f).+ = v,
        }
    };
}

macro_rules! ug {
    ($t:ty, $name:literal, $($f:ident).+, $lo:expr, $hi:expr) => {
        Gene {
            info: GeneInfo { name: $name, lo: $lo as f32, hi: $hi as f32, int: true },
            get: |p: &$t| p.$($f).+ as f32,
            set: |p: &mut $t, v| p.$($f).+ = v as u32,
        }
    };
}

macro_rules! stall_genes {
    ($t:ty) => {
        [
            fg!($t, "stall.min_speed", stall.min_speed, 0.05, 1.0),
            ug!($t, "stall.stuck_ticks", stall.stuck_ticks, 3, 60),
            ug!($t, "stall.reverse_ticks", stall.reverse_ticks, 5, 60),
        ]
    };
}

const CHARGER: [Gene<ChargerParams>; 14] = {
    let [s0, s1, s2] = stall_genes!(ChargerParams);
    [
        fg!(
            ChargerParams,
            "steer_tol",
            steer_tol,
            0.05,
            CHARGER_STEER_TOL_MAX
        ),
        fg!(
            ChargerParams,
            "stop_dist",
            stop_dist,
            CHARGER_STOP_DIST_MIN,
            300.0
        ),
        fg!(ChargerParams, "aim_tol", aim_tol, 0.005, 0.2),
        fg!(ChargerParams, "route_margin", route_margin, 0.0, 40.0),
        fg!(ChargerParams, "weave_deg", weave_deg, 0.0, 60.0),
        ug!(ChargerParams, "weave_period", weave_period, 5, 120),
        ug!(ChargerParams, "weave_jitter", weave_jitter, 0, 60),
        fg!(ChargerParams, "weave_until", weave_until, 0.0, 400.0),
        fg!(ChargerParams, "dodge_horizon", dodge.horizon, 0.0, 60.0),
        fg!(ChargerParams, "dodge_margin", dodge.margin, 0.0, 20.0),
        fg!(
            ChargerParams,
            "dodge_chance",
            dodge.chance,
            0.0,
            DODGE_CHANCE_MAX[0]
        ),
        s0,
        s1,
        s2,
    ]
};

const KITER: [Gene<KiterParams>; 15] = {
    let [s0, s1, s2] = stall_genes!(KiterParams);
    [
        fg!(KiterParams, "min_dist", min_dist, 80.0, 450.0),
        fg!(KiterParams, "max_dist", max_dist, 120.0, 600.0),
        fg!(KiterParams, "bend_deg", bend_deg, 0.0, 75.0),
        fg!(KiterParams, "wall_margin", wall_margin, 20.0, 150.0),
        ug!(KiterParams, "flip_every", flip_every, 30, 600),
        ug!(KiterParams, "flip_jitter", flip_jitter, 0, 200),
        ug!(
            KiterParams,
            "wall_flip_cooldown",
            wall_flip_cooldown,
            5,
            120
        ),
        fg!(KiterParams, "steer_tol", steer_tol, 0.05, 0.6),
        fg!(KiterParams, "aim_tol", aim_tol, 0.005, 0.2),
        fg!(KiterParams, "dodge_horizon", dodge.horizon, 0.0, 60.0),
        fg!(KiterParams, "dodge_margin", dodge.margin, 0.0, 20.0),
        fg!(
            KiterParams,
            "dodge_chance",
            dodge.chance,
            0.0,
            DODGE_CHANCE_MAX[1]
        ),
        s0,
        s1,
        s2,
    ]
};

const SNIPER: [Gene<SniperParams>; 20] = {
    let [s0, s1, s2] = stall_genes!(SniperParams);
    [
        fg!(SniperParams, "wall_margin", wall_margin, 30.0, 150.0),
        fg!(SniperParams, "grid_step", grid_step, 15.0, 60.0),
        fg!(
            SniperParams,
            "obstacle_clearance",
            obstacle_clearance,
            0.0,
            30.0
        ),
        ug!(SniperParams, "replan_every", replan_every, 10, 120),
        fg!(SniperParams, "replan_gain", replan_gain, 0.0, 250.0),
        fg!(SniperParams, "arrive_radius", arrive_radius, 3.0, 40.0),
        fg!(SniperParams, "aim_tol", aim_tol, 0.005, 0.2),
        fg!(SniperParams, "evade_dist", evade_dist, 80.0, 450.0),
        ug!(SniperParams, "evade_ticks", evade_ticks, 10, 180),
        ug!(SniperParams, "evade_jitter", evade_jitter, 0, 60),
        ug!(SniperParams, "blind_ticks", blind_ticks, 20, 400),
        fg!(SniperParams, "peek_offset", peek_offset, 0.0, 40.0),
        fg!(SniperParams, "steer_tol", steer_tol, 0.05, 0.6),
        fg!(SniperParams, "route_margin", route_margin, 0.0, 40.0),
        fg!(SniperParams, "dodge_horizon", dodge.horizon, 0.0, 60.0),
        fg!(SniperParams, "dodge_margin", dodge.margin, 0.0, 20.0),
        fg!(
            SniperParams,
            "dodge_chance",
            dodge.chance,
            0.0,
            DODGE_CHANCE_MAX[2]
        ),
        s0,
        s1,
        s2,
    ]
};

/// Upper bound of the `dodge_chance` gene (dodge strength) per behavior, in
/// [`Behavior::ALL`] order (Charger, Kiter, Sniper): each is that policy's shipped
/// strength. Evolution may make a tank dodge less often than its scripted policy, never
/// more. A bound below a shipped value would put Gen 0 out of bounds. A uniform cap would
/// have to be at least the Kiter's 0.95, and in the 2026-10-02 sweep that capped nothing
/// (seed 1: 98.3% vs Gen 0, uncapped 95.5%). These bounds gave the weakest champions
/// that still clear 65% (88.3% seed 1, 71.1% seed 2). Evidence: field note
/// `docs/fieldnotes/2026-10-02-dodge-cap.md`.
pub const DODGE_CHANCE_MAX: [f32; 3] = [0.65, 0.95, 0.61];

/// Lower bound of the Charger's `stop_dist` gene: its shipped 60 u. Below 32 u (two
/// radii) an evolved Charger never stops driving into its target; see
/// `docs/design/tank-balance-2026-10.md`. Like [`DODGE_CHANCE_MAX`], evolution may make
/// a Charger more careful than its scripted self, never more reckless.
pub const CHARGER_STOP_DIST_MIN: f32 = 60.0;

/// Upper bound of the Charger's `steer_tol` gene: its shipped 0.2 (was 0.6).
pub const CHARGER_STEER_TOL_MAX: f32 = 0.2;

/// Kiter's range band keeps at least this width (`max_dist >= min_dist + KITER_BAND`).
pub const KITER_BAND: f32 = 20.0;

fn infos<P>(t: &[Gene<P>]) -> Vec<GeneInfo> {
    t.iter().map(|g| g.info).collect()
}
fn read<P>(t: &[Gene<P>], p: &P) -> Vec<f32> {
    t.iter().map(|g| (g.get)(p)).collect()
}
fn write<P: Default>(t: &[Gene<P>], genes: &[f32]) -> P {
    let mut p = P::default();
    for (g, &v) in t.iter().zip(genes) {
        (g.set)(&mut p, v);
    }
    p
}

/// Gene table of a behavior, in genome order.
pub fn gene_infos(b: Behavior) -> Vec<GeneInfo> {
    match b {
        Behavior::Charger => infos(&CHARGER),
        Behavior::Kiter => infos(&KITER),
        Behavior::Sniper => infos(&SNIPER),
    }
}

/// Round to the gene's grid and clamp to its bounds.
pub fn quantize(info: &GeneInfo, v: f64) -> f32 {
    let v = v.clamp(info.lo as f64, info.hi as f64);
    if info.int {
        v.round() as f32
    } else {
        ((v * QUANT).round() / QUANT) as f32
    }
}

/// A candidate tank: behavior, its numbers (in [`gene_infos`] order) and a loadout.
#[derive(Clone, Debug, PartialEq)]
pub struct Genome {
    /// Which scripted policy's code drives it.
    pub behavior: Behavior,
    /// Its numbers, one per [`gene_infos`] entry, always within bounds and quantized.
    pub genes: Vec<f32>,
    /// One of the 19 valid 9-point splits.
    pub loadout: Loadout,
}

impl Genome {
    /// Gen 0: a scripted policy exactly as shipped (default params, 3/3/3).
    pub fn scripted(behavior: Behavior) -> Self {
        let genes = match behavior {
            Behavior::Charger => read(&CHARGER, &ChargerParams::default()),
            Behavior::Kiter => read(&KITER, &KiterParams::default()),
            Behavior::Sniper => read(&SNIPER, &SniperParams::default()),
        };
        Self {
            behavior,
            genes,
            loadout: Loadout::DEFAULT,
        }
    }

    /// `kiter-4-3-2` (behavior and loadout, as in match URLs).
    pub fn label(&self) -> String {
        format!("{}-{}", self.behavior, self.loadout)
    }

    /// Enforce bounds, the grid, and cross-field rules (Kiter's band).
    pub fn repair(&mut self) {
        let infos = gene_infos(self.behavior);
        for (g, i) in self.genes.iter_mut().zip(&infos) {
            *g = quantize(i, *g as f64);
        }
        if self.behavior == Behavior::Kiter {
            // genes[0] = min_dist, genes[1] = max_dist
            let hi = infos[1].hi;
            if self.genes[0] > hi - KITER_BAND {
                self.genes[0] = hi - KITER_BAND;
            }
            if self.genes[1] < self.genes[0] + KITER_BAND {
                self.genes[1] = quantize(&infos[1], (self.genes[0] + KITER_BAND) as f64);
            }
        }
    }

    /// The policy, with timing jitter seeded from `seed` (as [`Behavior::build`]).
    pub fn build(&self, seed: u64) -> Box<dyn Policy> {
        match self.behavior {
            Behavior::Charger => Box::new(Charger::seeded(write(&CHARGER, &self.genes), seed)),
            Behavior::Kiter => Box::new(Kiter::seeded(write(&KITER, &self.genes), seed)),
            Behavior::Sniper => Box::new(Sniper::seeded(write(&SNIPER, &self.genes), seed)),
        }
    }

    /// Charger params, if this is a Charger genome.
    pub fn charger_params(&self) -> Option<ChargerParams> {
        (self.behavior == Behavior::Charger).then(|| write(&CHARGER, &self.genes))
    }
    /// Kiter params, if this is a Kiter genome.
    pub fn kiter_params(&self) -> Option<KiterParams> {
        (self.behavior == Behavior::Kiter).then(|| write(&KITER, &self.genes))
    }
    /// Sniper params, if this is a Sniper genome.
    pub fn sniper_params(&self) -> Option<SniperParams> {
        (self.behavior == Behavior::Sniper).then(|| write(&SNIPER, &self.genes))
    }

    /// Readable JSON: `{"behavior", "loadout", "params": {name: value}}`.
    pub fn to_json(&self) -> Value {
        let mut params = Map::new();
        for (i, &g) in gene_infos(self.behavior).iter().zip(&self.genes) {
            params.insert(i.name.to_string(), gene_value(i, g));
        }
        json!({
            "behavior": self.behavior.key(),
            "loadout": self.loadout.to_string(),
            "params": params,
        })
    }

    /// Compact JSON for the population file: `{"b", "l", "g": [values]}`.
    pub fn to_compact(&self) -> Value {
        let infos = gene_infos(self.behavior);
        let g: Vec<Value> = infos
            .iter()
            .zip(&self.genes)
            .map(|(i, &v)| gene_value(i, v))
            .collect();
        json!({ "b": self.behavior.key(), "l": self.loadout.to_string(), "g": g })
    }

    /// Parse [`Genome::to_json`] output. Every param must be present and in bounds.
    pub fn from_json(v: &Value) -> Result<Self, String> {
        let behavior: Behavior = str_field(v, "behavior")?.parse()?;
        let loadout: Loadout = str_field(v, "loadout")?
            .parse()
            .map_err(|e| format!("loadout: {e}"))?;
        let params = v
            .get("params")
            .and_then(Value::as_object)
            .ok_or("missing params")?;
        let infos = gene_infos(behavior);
        if params.len() != infos.len() {
            return Err(format!(
                "{behavior}: expected {} params, got {}",
                infos.len(),
                params.len()
            ));
        }
        let genes = infos
            .iter()
            .map(|i| parse_gene(i, params.get(i.name)))
            .collect::<Result<Vec<_>, _>>()?;
        Self::checked(behavior, genes, loadout)
    }

    /// Parse [`Genome::to_compact`] output.
    pub fn from_compact(v: &Value) -> Result<Self, String> {
        let behavior: Behavior = str_field(v, "b")?.parse()?;
        let loadout: Loadout = str_field(v, "l")?
            .parse()
            .map_err(|e| format!("loadout: {e}"))?;
        let g = v.get("g").and_then(Value::as_array).ok_or("missing g")?;
        let infos = gene_infos(behavior);
        if g.len() != infos.len() {
            return Err(format!("{behavior}: expected {} genes", infos.len()));
        }
        let genes = infos
            .iter()
            .zip(g)
            .map(|(i, x)| parse_gene(i, Some(x)))
            .collect::<Result<Vec<_>, _>>()?;
        Self::checked(behavior, genes, loadout)
    }

    fn checked(behavior: Behavior, genes: Vec<f32>, loadout: Loadout) -> Result<Self, String> {
        let g = Self {
            behavior,
            genes,
            loadout,
        };
        let mut r = g.clone();
        r.repair();
        if r != g {
            return Err(format!("{}: genes violate a cross-field rule", g.label()));
        }
        Ok(g)
    }
}

fn gene_value(i: &GeneInfo, g: f32) -> Value {
    if i.int {
        json!(g as u64)
    } else {
        json!((g as f64 * QUANT).round() / QUANT)
    }
}

fn parse_gene(i: &GeneInfo, v: Option<&Value>) -> Result<f32, String> {
    let x = v
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("{}: missing or not a number", i.name))?;
    // Compare on the grid: an f32 bound such as 0.05 widens to 0.0500000007 in f64.
    let tol = 0.5 / QUANT;
    if x < i.lo as f64 - tol || x > i.hi as f64 + tol {
        return Err(format!("{} = {x} is outside [{}, {}]", i.name, i.lo, i.hi));
    }
    let q = quantize(i, x);
    if (q as f64 - x).abs() > 0.5 / QUANT {
        return Err(format!("{} = {x} is off the grid", i.name));
    }
    Ok(q)
}

fn str_field<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    v.get(k)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing {k}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_cover_every_field_and_defaults_are_in_bounds() {
        assert_eq!(gene_infos(Behavior::Charger).len(), 14);
        assert_eq!(gene_infos(Behavior::Kiter).len(), 15);
        assert_eq!(gene_infos(Behavior::Sniper).len(), 20);
        for b in Behavior::ALL {
            let g = Genome::scripted(b);
            let mut r = g.clone();
            r.repair();
            assert_eq!(
                r, g,
                "{b}: defaults must already be on the grid and in bounds"
            );
            let mut names: Vec<_> = gene_infos(b).iter().map(|i| i.name).collect();
            names.sort();
            names.dedup();
            assert_eq!(names.len(), g.genes.len(), "{b}: duplicate gene name");
        }
    }

    #[test]
    fn dodge_chance_is_capped_at_the_shipped_strength() {
        for (i, b) in Behavior::ALL.into_iter().enumerate() {
            let infos = gene_infos(b);
            let k = infos.iter().position(|g| g.name == "dodge_chance").unwrap();
            assert_eq!(infos[k].hi, DODGE_CHANCE_MAX[i], "{b}");
            assert!(DODGE_CHANCE_MAX[i] < 1.0, "{b}: capped below the maximum");
            assert_eq!(
                Genome::scripted(b).genes[k],
                DODGE_CHANCE_MAX[i],
                "{b}: the cap is the shipped dodge strength"
            );
        }
    }

    #[test]
    fn scripted_genome_is_exactly_the_shipped_policy() {
        assert_eq!(
            Genome::scripted(Behavior::Charger).charger_params(),
            Some(ChargerParams::default())
        );
        assert_eq!(
            Genome::scripted(Behavior::Kiter).kiter_params(),
            Some(KiterParams::default())
        );
        assert_eq!(
            Genome::scripted(Behavior::Sniper).sniper_params(),
            Some(SniperParams::default())
        );
    }

    #[test]
    fn each_gene_reaches_its_field() {
        // Setting gene i to a new value changes the params struct (no dead table rows).
        for b in Behavior::ALL {
            let base = Genome::scripted(b);
            for (i, info) in gene_infos(b).iter().enumerate() {
                let mut g = base.clone();
                g.genes[i] = if g.genes[i] == info.hi {
                    info.lo
                } else {
                    info.hi
                };
                g.repair();
                let changed = match b {
                    Behavior::Charger => g.charger_params() != base.charger_params(),
                    Behavior::Kiter => g.kiter_params() != base.kiter_params(),
                    Behavior::Sniper => g.sniper_params() != base.sniper_params(),
                };
                assert!(changed, "{b}.{}", info.name);
            }
        }
    }

    #[test]
    fn json_round_trips_exactly() {
        for b in Behavior::ALL {
            let mut g = Genome::scripted(b);
            for (k, x) in g.genes.iter_mut().enumerate() {
                *x += 0.123 * (k as f32 + 1.0);
            }
            g.loadout = Loadout::ALL[b as usize * 5];
            g.repair();
            assert_eq!(Genome::from_json(&g.to_json()).unwrap(), g);
            assert_eq!(Genome::from_compact(&g.to_compact()).unwrap(), g);
        }
    }

    #[test]
    fn genes_at_their_bounds_round_trip() {
        // Regression: 0.05 (steer_tol's lower bound) was rejected as below 0.05f32.
        for b in Behavior::ALL {
            for pick_hi in [false, true] {
                let mut g = Genome::scripted(b);
                for (x, i) in g.genes.iter_mut().zip(gene_infos(b)) {
                    *x = if pick_hi { i.hi } else { i.lo };
                }
                g.repair();
                assert_eq!(
                    Genome::from_json(&g.to_json()).unwrap(),
                    g,
                    "{b} hi={pick_hi}"
                );
                assert_eq!(Genome::from_compact(&g.to_compact()).unwrap(), g);
            }
        }
    }

    #[test]
    fn grid_values_survive_f32_and_f64() {
        // Every float gene value is k/1000; check the JSON path is exact for all of them
        // up to the largest bound.
        let info = GeneInfo {
            name: "x",
            lo: 0.0,
            hi: 600.0,
            int: false,
        };
        for k in 0..=600_000u32 {
            let q = quantize(&info, k as f64 / QUANT);
            assert_eq!(parse_gene(&info, Some(&gene_value(&info, q))).unwrap(), q);
        }
    }

    #[test]
    fn bad_json_is_rejected() {
        let good = Genome::scripted(Behavior::Kiter).to_json();
        let mut v = good.clone();
        v["params"]["min_dist"] = json!(10.0);
        assert!(Genome::from_json(&v).is_err(), "out of bounds");
        let mut v = good.clone();
        v["params"]["max_dist"] = json!(255.0);
        assert!(Genome::from_json(&v).is_err(), "band rule");
        let mut v = good.clone();
        v["loadout"] = json!("5-3-2");
        assert!(Genome::from_json(&v).is_err(), "10-point loadout");
        let mut v = good;
        v["params"].as_object_mut().unwrap().remove("aim_tol");
        assert!(Genome::from_json(&v).is_err(), "missing param");
    }
}
