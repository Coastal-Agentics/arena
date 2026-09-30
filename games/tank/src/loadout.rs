//! The 9-point stat budget (SPEC "Tanks"): Attack, Speed and Defense levels 1–5 that
//! always sum to 9, their mapping onto [`TankParams`], presets, hits-to-kill, and the
//! Customize tab's triangle snap.
//!
//! Level 3 in every stat is exactly [`TankParams::default`], so a 3/3/3 match is the
//! default match.

use engine::TankParams;
use std::fmt;
use std::str::FromStr;

/// Lowest level of a stat.
pub const MIN_LEVEL: u8 = 1;
/// Highest level of a stat.
pub const MAX_LEVEL: u8 = 5;
/// Points every tank spends across Attack + Speed + Defense.
pub const BUDGET: u8 = 9;

/// `projectile_damage` by Attack level 1..=5.
pub const DAMAGE: [i32; 5] = [12, 16, 20, 24, 28];
/// `max_speed` (units per second) by Speed level 1..=5.
pub const MAX_SPEED: [f32; 5] = [90.0, 105.0, 120.0, 135.0, 150.0];
/// `turn_rate` (BAU per tick) by Speed level 1..=5.
pub const TURN_RATE: [u16; 5] = [273, 318, 364, 410, 455];
/// `max_hp` by Defense level 1..=5.
pub const MAX_HP: [i32; 5] = [60, 80, 100, 120, 140];

/// A valid build: each stat in `1..=5`, summing to [`BUDGET`]. Construct with
/// [`Loadout::new`] (checked) or take one from [`Loadout::ALL`] / [`Preset`].
///
/// Text form is `A-S-D`, e.g. `5-3-1` (used in match URLs).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Loadout {
    attack: u8,
    speed: u8,
    defense: u8,
}

/// Why a stat triple is not a loadout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadoutError {
    /// A stat is outside `1..=5`.
    LevelOutOfRange {
        /// `"attack"`, `"speed"` or `"defense"`.
        stat: &'static str,
        /// The offending level.
        level: u8,
    },
    /// The levels are in range but don't sum to [`BUDGET`].
    WrongTotal(u32),
    /// Text isn't `A-S-D` with three small integers.
    Syntax(String),
}

impl fmt::Display for LoadoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LevelOutOfRange { stat, level } => {
                write!(
                    f,
                    "{stat} level {level} is outside {MIN_LEVEL}..={MAX_LEVEL}"
                )
            }
            Self::WrongTotal(t) => write!(f, "stats sum to {t}, must be exactly {BUDGET}"),
            Self::Syntax(s) => write!(f, "loadout {s:?} is not A-S-D (e.g. 3-3-3)"),
        }
    }
}

impl std::error::Error for LoadoutError {}

const fn l(attack: u8, speed: u8, defense: u8) -> Loadout {
    Loadout {
        attack,
        speed,
        defense,
    }
}

impl Loadout {
    /// The scripted default, 3/3/3 (= [`TankParams::default`]).
    pub const DEFAULT: Loadout = l(3, 3, 3);

    /// All 19 valid loadouts, ordered by Attack, then Speed (ascending). This order is
    /// the snap tie-break order.
    pub const ALL: [Loadout; 19] = [
        l(1, 3, 5),
        l(1, 4, 4),
        l(1, 5, 3),
        l(2, 2, 5),
        l(2, 3, 4),
        l(2, 4, 3),
        l(2, 5, 2),
        l(3, 1, 5),
        l(3, 2, 4),
        l(3, 3, 3),
        l(3, 4, 2),
        l(3, 5, 1),
        l(4, 1, 4),
        l(4, 2, 3),
        l(4, 3, 2),
        l(4, 4, 1),
        l(5, 1, 3),
        l(5, 2, 2),
        l(5, 3, 1),
    ];

    /// Checked constructor.
    ///
    /// ```
    /// use tank::Loadout;
    /// assert!(Loadout::new(5, 3, 1).is_ok());
    /// assert!(Loadout::new(5, 3, 2).is_err()); // 10 points
    /// assert!(Loadout::new(6, 2, 1).is_err()); // level 6
    /// ```
    pub fn new(attack: u8, speed: u8, defense: u8) -> Result<Self, LoadoutError> {
        for (stat, level) in [("attack", attack), ("speed", speed), ("defense", defense)] {
            if !(MIN_LEVEL..=MAX_LEVEL).contains(&level) {
                return Err(LoadoutError::LevelOutOfRange { stat, level });
            }
        }
        let total = attack as u32 + speed as u32 + defense as u32;
        if total != BUDGET as u32 {
            return Err(LoadoutError::WrongTotal(total));
        }
        Ok(l(attack, speed, defense))
    }

    /// Attack level (1–5).
    pub fn attack(self) -> u8 {
        self.attack
    }
    /// Speed level (1–5).
    pub fn speed(self) -> u8 {
        self.speed
    }
    /// Defense level (1–5).
    pub fn defense(self) -> u8 {
        self.defense
    }
    /// `[attack, speed, defense]`.
    pub fn levels(self) -> [u8; 3] {
        [self.attack, self.speed, self.defense]
    }

    /// Damage per hit (`projectile_damage`).
    pub fn damage(self) -> i32 {
        DAMAGE[(self.attack - 1) as usize]
    }
    /// Top speed in units per second (`max_speed`).
    pub fn max_speed(self) -> f32 {
        MAX_SPEED[(self.speed - 1) as usize]
    }
    /// Hull turn rate in BAU per tick (`turn_rate`).
    pub fn turn_rate(self) -> u16 {
        TURN_RATE[(self.speed - 1) as usize]
    }
    /// Hit points (`max_hp`).
    pub fn max_hp(self) -> i32 {
        MAX_HP[(self.defense - 1) as usize]
    }

    /// This loadout's [`TankParams`]: damage, speed, turn rate and HP from the level
    /// tables; everything else (radius, turret, cooldown, shells) is the shared default.
    ///
    /// ```
    /// use tank::Loadout;
    /// assert_eq!(Loadout::DEFAULT.params(), engine::TankParams::default());
    /// let p = Loadout::new(5, 3, 1).unwrap().params();
    /// assert_eq!((p.projectile_damage, p.max_hp), (28, 60));
    /// ```
    pub fn params(self) -> TankParams {
        TankParams {
            max_speed: self.max_speed(),
            turn_rate: self.turn_rate(),
            max_hp: self.max_hp(),
            projectile_damage: self.damage(),
            ..TankParams::default()
        }
    }

    /// Shots this loadout needs to destroy `defender` (⌈HP / damage⌉).
    ///
    /// ```
    /// use tank::{Loadout, Preset};
    /// let (gc, br) = (Preset::GlassCannon.loadout(), Preset::Brawler.loadout());
    /// assert_eq!(gc.hits_to_kill(br), 5); // 28 dmg vs 120 HP
    /// assert_eq!(br.hits_to_kill(gc), 3); // 24 dmg vs 60 HP
    /// ```
    pub fn hits_to_kill(self, defender: Loadout) -> u32 {
        hits_to_kill(self.damage(), defender.max_hp())
    }

    /// Barycentric weights `[attack, speed, defense]` of this loadout's exact point on
    /// the Customize triangle (the inverse of `stat = 1 + 6 · weight`).
    pub fn weights(self) -> [f32; 3] {
        self.levels().map(|v| (v as f32 - 1.0) / 6.0)
    }

    /// Customize-triangle snap (SPEC "Customize tab"). `w` are barycentric weights for
    /// the Attack, Speed and Defense corners; they are clamped at 0 and normalised, so
    /// any input (even outside the triangle) gives a valid loadout. Each stat is
    /// `1 + 6 · weight`; the result is the nearest of the 19 loadouts by squared
    /// distance, exact ties going to lower Attack, then lower Speed.
    ///
    /// ```
    /// use tank::Loadout;
    /// let third = 1.0 / 3.0;
    /// assert_eq!(Loadout::snap([third, third, third]), Loadout::DEFAULT);
    /// assert_eq!(Loadout::snap([1.0, 0.0, 0.0]).to_string(), "5-2-2");
    /// ```
    pub fn snap(w: [f32; 3]) -> Loadout {
        let w = w.map(|x| if x.is_finite() && x > 0.0 { x } else { 0.0 });
        let sum: f32 = w.iter().sum();
        let w = if sum > 0.0 {
            w.map(|x| x / sum)
        } else {
            [1.0 / 3.0; 3]
        };
        let target = w.map(|x| 1.0 + 6.0 * x);
        let mut best = Loadout::ALL[0];
        let mut best_d = f32::INFINITY;
        for cand in Loadout::ALL {
            let d: f32 = cand
                .levels()
                .iter()
                .zip(target)
                .map(|(&v, t)| (v as f32 - t) * (v as f32 - t))
                .sum();
            // Strictly less: ALL is sorted by (attack, speed), so ties keep the lower.
            if d < best_d {
                best_d = d;
                best = cand;
            }
        }
        best
    }
}

impl Default for Loadout {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl fmt::Display for Loadout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}-{}", self.attack, self.speed, self.defense)
    }
}

impl FromStr for Loadout {
    type Err = LoadoutError;
    /// Parses `A-S-D` (e.g. `5-3-1`) and validates the budget.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.trim().split('-').collect();
        let bad = || LoadoutError::Syntax(s.to_string());
        if parts.len() != 3 {
            return Err(bad());
        }
        let mut v = [0u8; 3];
        for (slot, p) in v.iter_mut().zip(&parts) {
            if p.is_empty() || p.len() > 2 || !p.bytes().all(|b| b.is_ascii_digit()) {
                return Err(bad());
            }
            *slot = p.parse().map_err(|_| bad())?;
        }
        Loadout::new(v[0], v[1], v[2])
    }
}

/// ⌈`hp` / `damage`⌉ for positive values (the number of hits to reach `hp <= 0`).
pub fn hits_to_kill(damage: i32, hp: i32) -> u32 {
    assert!(damage > 0 && hp > 0, "damage and hp must be positive");
    ((hp + damage - 1) / damage) as u32
}

/// Named builds from the spec.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Preset {
    /// 3/3/3, the scripted default.
    Balanced,
    /// 5/3/1.
    GlassCannon,
    /// 4/1/4.
    Brawler,
    /// 2/5/2.
    Scout,
}

impl Preset {
    /// Every preset, in the order the viewer shows them.
    pub const ALL: [Preset; 4] = [
        Preset::Balanced,
        Preset::GlassCannon,
        Preset::Brawler,
        Preset::Scout,
    ];

    /// Display name ("Glass Cannon").
    pub fn name(self) -> &'static str {
        match self {
            Self::Balanced => "Balanced",
            Self::GlassCannon => "Glass Cannon",
            Self::Brawler => "Brawler",
            Self::Scout => "Scout",
        }
    }

    /// The preset's loadout.
    pub fn loadout(self) -> Loadout {
        match self {
            Self::Balanced => l(3, 3, 3),
            Self::GlassCannon => l(5, 3, 1),
            Self::Brawler => l(4, 1, 4),
            Self::Scout => l(2, 5, 2),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exactly_19_valid_loadouts_and_all_is_complete_and_sorted() {
        let mut found = Vec::new();
        for a in 0..=9u8 {
            for s in 0..=9u8 {
                for d in 0..=9u8 {
                    if let Ok(x) = Loadout::new(a, s, d) {
                        found.push(x);
                    }
                }
            }
        }
        assert_eq!(found.len(), 19);
        assert_eq!(
            found,
            Loadout::ALL.to_vec(),
            "ALL is every valid triple, sorted"
        );
        for x in Loadout::ALL {
            assert_eq!(x.levels().iter().map(|&v| v as u32).sum::<u32>(), 9);
        }
    }

    #[test]
    fn validation_errors() {
        assert_eq!(
            Loadout::new(0, 4, 5),
            Err(LoadoutError::LevelOutOfRange {
                stat: "attack",
                level: 0
            })
        );
        assert_eq!(
            Loadout::new(3, 6, 0),
            Err(LoadoutError::LevelOutOfRange {
                stat: "speed",
                level: 6
            })
        );
        assert_eq!(Loadout::new(3, 3, 4), Err(LoadoutError::WrongTotal(10)));
        assert_eq!(Loadout::new(1, 1, 1), Err(LoadoutError::WrongTotal(3)));
    }

    #[test]
    fn presets_match_spec() {
        let got: Vec<(&str, String)> = Preset::ALL
            .iter()
            .map(|p| (p.name(), p.loadout().to_string()))
            .collect();
        assert_eq!(
            got,
            [
                ("Balanced", "3-3-3"),
                ("Glass Cannon", "5-3-1"),
                ("Brawler", "4-1-4"),
                ("Scout", "2-5-2"),
            ]
            .map(|(n, s)| (n, s.to_string()))
        );
        for p in Preset::ALL {
            assert!(Loadout::ALL.contains(&p.loadout()));
        }
        assert_eq!(Preset::Balanced.loadout(), Loadout::DEFAULT);
    }

    #[test]
    fn level_tables_map_to_tank_params() {
        // SPEC "Tanks" table, row by row.
        let expect = [
            (1, 12, 90.0, 273, 60),
            (2, 16, 105.0, 318, 80),
            (3, 20, 120.0, 364, 100),
            (4, 24, 135.0, 410, 120),
            (5, 28, 150.0, 455, 140),
        ];
        let base = TankParams::default();
        for (lvl, dmg, spd, turn, hp) in expect {
            let a = *Loadout::ALL.iter().find(|x| x.attack() == lvl).unwrap();
            let s = *Loadout::ALL.iter().find(|x| x.speed() == lvl).unwrap();
            let d = *Loadout::ALL.iter().find(|x| x.defense() == lvl).unwrap();
            assert_eq!(a.params().projectile_damage, dmg);
            assert_eq!((s.params().max_speed, s.params().turn_rate), (spd, turn));
            assert_eq!(d.params().max_hp, hp);
            assert_eq!(DAMAGE[lvl as usize - 1], dmg);
            assert_eq!(MAX_SPEED[lvl as usize - 1], spd);
            assert_eq!(TURN_RATE[lvl as usize - 1], turn);
            assert_eq!(MAX_HP[lvl as usize - 1], hp);
        }
        for x in Loadout::ALL {
            let p = x.params();
            assert_eq!(p.projectile_damage, x.damage());
            assert_eq!(p.max_speed, x.max_speed());
            assert_eq!(p.turn_rate, x.turn_rate());
            assert_eq!(p.max_hp, x.max_hp());
            // Everything else is fixed for everyone (SPEC "Fixed for everyone").
            assert_eq!(p.radius, 16.0);
            assert_eq!(p.turret_turn_rate, 546);
            assert_eq!(p.fire_cooldown, 45);
            assert_eq!(p.projectile_speed, 360.0);
            assert_eq!(p.projectile_ttl, 120);
            assert_eq!(p.projectile_spread, 256);
            assert_eq!(
                TankParams {
                    max_speed: base.max_speed,
                    turn_rate: base.turn_rate,
                    max_hp: base.max_hp,
                    projectile_damage: base.projectile_damage,
                    ..p.clone()
                },
                base
            );
        }
        assert_eq!(Loadout::DEFAULT.params(), base);
    }

    #[test]
    fn speed_scales_turn_rate_with_max_speed() {
        // Turn rate is the default 364 scaled by the same factor as speed, rounded.
        for (s, t) in MAX_SPEED.iter().zip(TURN_RATE) {
            let scaled = 364.0 * s / 120.0;
            assert!((scaled - t as f32).abs() <= 0.5, "{s} -> {t} vs {scaled}");
        }
    }

    /// SPEC "Hits-to-kill" table, verbatim: rows Attack 1..5, columns Defense 1..5.
    const HITS_TO_KILL: [[u32; 5]; 5] = [
        [5, 7, 9, 10, 12],
        [4, 5, 7, 8, 9],
        [3, 4, 5, 6, 7],
        [3, 4, 5, 5, 6],
        [3, 3, 4, 5, 5],
    ];

    #[test]
    fn hits_to_kill_table_matches_spec() {
        for (ai, row) in HITS_TO_KILL.iter().enumerate() {
            for (di, &want) in row.iter().enumerate() {
                assert_eq!(
                    hits_to_kill(DAMAGE[ai], MAX_HP[di]),
                    want,
                    "attack {} vs defense {}",
                    ai + 1,
                    di + 1
                );
            }
        }
        // Via loadouts: every attacker/defender pair uses its own attack and the
        // opponent's defense.
        for a in Loadout::ALL {
            for d in Loadout::ALL {
                assert_eq!(
                    a.hits_to_kill(d),
                    HITS_TO_KILL[a.attack() as usize - 1][d.defense() as usize - 1]
                );
            }
        }
        // The default match: 5 hits (the engine test `projectile_hits_and_kills`).
        assert_eq!(Loadout::DEFAULT.hits_to_kill(Loadout::DEFAULT), 5);
        // Readout example from the spec.
        let (gc, br) = (Preset::GlassCannon.loadout(), Preset::Brawler.loadout());
        assert_eq!((gc.hits_to_kill(br), br.hits_to_kill(gc)), (5, 3));
    }

    #[test]
    fn hits_to_kill_is_the_real_sim_count() {
        // Shots actually needed in the engine equal the table, for every damage/HP pair.
        for dmg in DAMAGE {
            for hp in MAX_HP {
                let mut left = hp;
                let mut n = 0;
                while left > 0 {
                    left -= dmg;
                    n += 1;
                }
                assert_eq!(hits_to_kill(dmg, hp), n);
            }
        }
    }

    #[test]
    fn text_round_trip_and_rejects() {
        for x in Loadout::ALL {
            assert_eq!(x.to_string().parse::<Loadout>().unwrap(), x);
        }
        assert_eq!(" 5-3-1 ".parse::<Loadout>().unwrap().levels(), [5, 3, 1]);
        for bad in [
            "", "5-3", "5-3-1-0", "a-b-c", "5--3", "+5-3-1", "5-3-2", "9-0-0", "005-3-1",
        ] {
            assert!(bad.parse::<Loadout>().is_err(), "{bad:?}");
        }
    }

    #[test]
    fn snap_corners_centre_and_exact_points() {
        let third = 1.0 / 3.0;
        assert_eq!(Loadout::snap([third, third, third]), Loadout::DEFAULT);
        assert_eq!(Loadout::snap([1.0, 0.0, 0.0]).levels(), [5, 2, 2]);
        assert_eq!(Loadout::snap([0.0, 1.0, 0.0]).levels(), [2, 5, 2]);
        assert_eq!(Loadout::snap([0.0, 0.0, 1.0]).levels(), [2, 2, 5]);
        // Each loadout's own point snaps to itself.
        for x in Loadout::ALL {
            assert_eq!(Loadout::snap(x.weights()), x, "{x}");
        }
        // Garbage in, valid loadout out.
        for w in [
            [0.0, 0.0, 0.0],
            [f32::NAN, 1.0, 0.0],
            [-5.0, 2.0, 2.0],
            [f32::INFINITY, 0.0, 0.0],
            [1e30, 1e30, 1e30],
        ] {
            assert!(Loadout::ALL.contains(&Loadout::snap(w)), "{w:?}");
        }
        // Unnormalised weights are normalised first.
        assert_eq!(Loadout::snap([2.0, 2.0, 2.0]), Loadout::DEFAULT);
    }

    #[test]
    fn snap_tie_breaks_lower_attack_then_lower_speed() {
        // Target (3.5, 3.5, 2): equidistant from 3-4-2 and 4-3-2 → lower Attack wins.
        let w = [2.5 / 6.0, 2.5 / 6.0, 1.0 / 6.0];
        assert_eq!(Loadout::snap(w).to_string(), "3-4-2");
        // Target (3, 3.5, 2.5): 3-3-3 vs 3-4-2 → lower Speed wins.
        let w = [2.0 / 6.0, 2.5 / 6.0, 1.5 / 6.0];
        assert_eq!(Loadout::snap(w).to_string(), "3-3-3");
    }

    #[test]
    fn snap_grid_reaches_all_19_and_nothing_else() {
        let mut seen = std::collections::BTreeSet::new();
        let n = 300;
        for i in 0..=n {
            for j in 0..=(n - i) {
                let (a, s) = (i as f32 / n as f32, j as f32 / n as f32);
                let x = Loadout::snap([a, s, 1.0 - a - s]);
                assert!(Loadout::ALL.contains(&x));
                seen.insert(x);
            }
        }
        assert_eq!(seen.len(), 19);
    }
}
