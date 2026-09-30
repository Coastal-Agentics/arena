//! Tank Arena match setup (SPEC "Arena", "Match end"): the pillar arena, fixed mirrored
//! spawns for duel, 2v2 and FFA-4, the 7200-tick limit, and per-tank loadouts.
//!
//! The end rules themselves (last team standing wins, simultaneous wipe or time limit =
//! draw, no friendly fire, tanks block, walls clamp) are the engine's current `Match`
//! behaviour; this module only builds the [`MatchConfig`].
//!
//! **Per-tank stats are stubbed.** The engine has one shared [`MatchConfig::params`]
//! today; per-tank `TankParams` is engine ask #5 (`TankSpawn.params`). Until it lands,
//! [`duel`] applies the loadouts exactly when both tanks use the same one (it becomes
//! the shared params), and otherwise runs both at 3/3/3 and reports that through
//! [`Setup::stats_applied`]. [`ENGINE_HAS_PER_TANK_PARAMS`] is the single switch.

use crate::loadout::Loadout;
use engine::angle::{from_degrees, Heading};
use engine::{MatchConfig, TankParams, TankSpawn, Vec2, TICK_HZ};

/// False until the engine supports per-tank [`TankParams`] (engine ask #5).
pub const ENGINE_HAS_PER_TANK_PARAMS: bool = false;

/// Match length cap: 120 s at 60 Hz. Reaching it is a draw.
pub const MAX_TICKS: u32 = 120 * TICK_HZ;

/// Which spawn layout to use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// 1v1, left (team 0, Blue) vs right (team 1, Orange).
    Duel,
    /// Two teams of two: left corners are team 0, right corners team 1.
    TwoVTwo,
    /// Four tanks, each its own team (0–3).
    Ffa4,
}

/// Duel spawns: `(100, 300)` facing 0°, `(700, 300)` facing 180°.
pub fn duel_spawns() -> [TankSpawn; 2] {
    [
        spawn(0, 100.0, 300.0, 0),
        spawn(1, 700.0, 300.0, from_degrees(180)),
    ]
}

/// Heading from a corner spawn to the centre `(400, 300)`: atan2(200, 300) ≈ 33.69°,
/// precomputed so config building calls no trig (checked in tests).
const CORNER_TO_CENTRE: Heading = 6133;

/// Corner spawns `(100,100)`, `(700,100)`, `(100,500)`, `(700,500)`, each facing the
/// centre. Teams: 2v2 is left (0) vs right (1); FFA gives tank `i` team `i`.
pub fn corner_spawns(mode: Mode) -> Vec<TankSpawn> {
    let c = CORNER_TO_CENTRE;
    let corners = [
        (100.0, 100.0, c),
        (700.0, 100.0, 32768 - c),
        (100.0, 500.0, 0u16.wrapping_sub(c)),
        (700.0, 500.0, 32768 + c),
    ];
    corners
        .iter()
        .enumerate()
        .map(|(i, &(x, y, h))| {
            let team = match mode {
                Mode::Ffa4 => i as u8,
                _ => (i % 2) as u8,
            };
            spawn(team, x, y, h)
        })
        .collect()
}

fn spawn(team: u8, x: f32, y: f32, heading: Heading) -> TankSpawn {
    TankSpawn {
        team,
        pos: Some(Vec2::new(x, y)),
        heading: Some(heading),
    }
}

/// The Tank Arena config for `mode` with every tank at 3/3/3: the engine's duel arena
/// (800×600, two pillars), fixed spawns, default params, [`MAX_TICKS`].
pub fn config(mode: Mode) -> MatchConfig {
    let base = MatchConfig::duel();
    MatchConfig {
        tanks: match mode {
            Mode::Duel => duel_spawns().to_vec(),
            m => corner_spawns(m),
        },
        params: TankParams::default(),
        max_ticks: MAX_TICKS,
        ..base
    }
}

/// A built match config plus what the sim will actually use for each tank.
#[derive(Clone, Debug, PartialEq)]
pub struct Setup {
    /// Config to hand to [`engine::Match::new`].
    pub config: MatchConfig,
    /// Loadouts requested, by tank id.
    pub requested: Vec<Loadout>,
    /// Loadouts the sim will actually run, by tank id (see [`Setup::stats_applied`]).
    pub applied: Vec<Loadout>,
}

impl Setup {
    /// True when every tank runs its requested loadout.
    pub fn stats_applied(&self) -> bool {
        self.requested == self.applied
    }
}

/// Apply per-tank loadouts (indexed by tank id) to a config.
///
/// With today's engine (no per-tank params): if every loadout is the same it becomes the
/// shared `params`; otherwise every tank runs [`Loadout::DEFAULT`].
pub fn with_loadouts(mut config: MatchConfig, loadouts: &[Loadout]) -> Setup {
    assert_eq!(
        loadouts.len(),
        config.tanks.len(),
        "one loadout per tank spawn"
    );
    let requested = loadouts.to_vec();
    let shared = match requested.split_first() {
        Some((first, rest)) if rest.iter().all(|l| l == first) => *first,
        _ => Loadout::DEFAULT,
    };
    // Swap point for engine ask #5: set `config.tanks[i].params = Some(l.params())` for
    // every tank, and `applied = requested`.
    config.params = shared.params();
    Setup {
        config,
        applied: vec![shared; requested.len()],
        requested,
    }
}

/// Duel config with Blue (tank 0) and Orange (tank 1) loadouts.
///
/// ```
/// use tank::{rules, Loadout};
/// let s = rules::duel(Loadout::DEFAULT, Loadout::DEFAULT);
/// assert!(s.stats_applied());
/// assert_eq!(s.config, rules::config(rules::Mode::Duel));
/// ```
pub fn duel(blue: Loadout, orange: Loadout) -> Setup {
    with_loadouts(config(Mode::Duel), &[blue, orange])
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::{Match, Rect};

    #[test]
    fn duel_layout_matches_spec() {
        let c = config(Mode::Duel);
        assert_eq!(c.arena.size, Vec2::new(800.0, 600.0));
        assert_eq!(
            c.arena.obstacles,
            vec![
                Rect::new(Vec2::new(250.0, 200.0), Vec2::new(300.0, 400.0)),
                Rect::new(Vec2::new(500.0, 200.0), Vec2::new(550.0, 400.0)),
            ]
        );
        assert_eq!(c.max_ticks, 7200);
        assert_eq!(c.params, TankParams::default());
        let m = Match::new(c, 1);
        let t = m.tanks();
        assert_eq!(
            (t[0].team, t[0].pos, t[0].heading),
            (0, Vec2::new(100.0, 300.0), 0)
        );
        assert_eq!(
            (t[1].team, t[1].pos, t[1].heading),
            (1, Vec2::new(700.0, 300.0), 32768)
        );
    }

    #[test]
    fn spawns_are_mirrored_and_seed_independent() {
        let a = Match::new(config(Mode::Duel), 1);
        let b = Match::new(config(Mode::Duel), 999);
        assert_eq!(a.tanks(), b.tanks(), "fixed spawns: no RNG involved");
        for mode in [Mode::TwoVTwo, Mode::Ffa4] {
            let s = corner_spawns(mode);
            for sp in &s {
                let p = sp.pos.unwrap();
                // Mirror image through the centre exists.
                let mirror = Vec2::new(800.0, 600.0) - p;
                assert!(s.iter().any(|o| o.pos == Some(mirror)));
            }
        }
    }

    #[test]
    fn corner_spawns_face_the_centre() {
        for sp in corner_spawns(Mode::Ffa4) {
            let to_centre = Vec2::new(400.0, 300.0) - sp.pos.unwrap();
            let want = (to_centre.y as f64).atan2(to_centre.x as f64);
            let want_bau = (want / std::f64::consts::TAU * 65536.0).rem_euclid(65536.0);
            let got = sp.heading.unwrap() as f64;
            let diff = (got - want_bau).abs();
            assert!(diff.min(65536.0 - diff) <= 1.0, "{got} vs {want_bau}");
        }
    }

    #[test]
    fn team_assignment() {
        let teams = |m| corner_spawns(m).iter().map(|s| s.team).collect::<Vec<_>>();
        // Tanks 0 and 2 are on the left (x = 100), 1 and 3 on the right.
        assert_eq!(teams(Mode::TwoVTwo), vec![0, 1, 0, 1]);
        assert_eq!(teams(Mode::Ffa4), vec![0, 1, 2, 3]);
        for s in corner_spawns(Mode::TwoVTwo) {
            assert_eq!(s.team == 0, s.pos.unwrap().x < 400.0);
        }
    }

    #[test]
    fn default_duel_is_3_3_3() {
        let s = duel(Loadout::DEFAULT, Loadout::DEFAULT);
        assert!(s.stats_applied());
        assert_eq!(s.config, config(Mode::Duel));
    }

    #[test]
    fn equal_loadouts_apply_today_and_different_ones_are_flagged() {
        let gc: Loadout = "5-3-1".parse().unwrap();
        let br: Loadout = "4-1-4".parse().unwrap();
        let same = duel(gc, gc);
        assert!(same.stats_applied());
        assert_eq!(same.config.params, gc.params());
        let mixed = duel(gc, br);
        assert_eq!(mixed.requested, vec![gc, br]);
        assert_eq!(
            mixed.stats_applied(),
            ENGINE_HAS_PER_TANK_PARAMS,
            "flip this test when TankSpawn.params lands"
        );
        if !ENGINE_HAS_PER_TANK_PARAMS {
            assert_eq!(mixed.config.params, TankParams::default());
        }
    }
}
