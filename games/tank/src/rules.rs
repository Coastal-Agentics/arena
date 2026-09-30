//! Tank Arena match setup (SPEC "Arena", "Match end"): the pillar arena, fixed mirrored
//! spawns for duel, 2v2 and FFA-4, the 7200-tick limit, and per-tank loadouts.
//!
//! The end rules themselves (last team standing wins, simultaneous wipe or time limit =
//! draw, no friendly fire, tanks block, walls clamp) are the engine's current `Match`
//! behaviour; this module only builds the [`MatchConfig`].
//!
//! Loadouts go through the engine's per-tank params (engine ask #5): each tank whose
//! loadout differs from the shared params gets `TankSpawn::params`, built on top of
//! the shared `MatchConfig::params` ([`Loadout::apply`]). A 3/3/3 tank keeps
//! `params: None`, so an all-3/3/3 config is byte-for-byte the default config.

use crate::loadout::Loadout;
use engine::angle::{from_degrees, Heading};
use engine::{MatchConfig, TankSpawn, Vec2, TICK_HZ};

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
        params: None,
    }
}

/// The Tank Arena config for `mode` with every tank at 3/3/3: the engine's duel arena
/// (800×600, two pillars), fixed spawns, shared params = the 3/3/3 loadout
/// ([`engine::TankParams::default`] with 650 HP), [`MAX_TICKS`].
pub fn config(mode: Mode) -> MatchConfig {
    let base = MatchConfig::duel();
    MatchConfig {
        tanks: match mode {
            Mode::Duel => duel_spawns().to_vec(),
            m => corner_spawns(m),
        },
        params: Loadout::DEFAULT.params(),
        max_ticks: MAX_TICKS,
        ..base
    }
}

/// A built match config and the loadouts it encodes.
#[derive(Clone, Debug, PartialEq)]
pub struct Setup {
    /// Config to hand to [`engine::Match::new`].
    pub config: MatchConfig,
    /// Loadouts by tank id.
    pub loadouts: Vec<Loadout>,
}

/// Apply per-tank loadouts (indexed by tank id) to a config: tank `i` plays with
/// `loadouts[i].apply(&config.params)`, set as its `TankSpawn::params` unless that
/// equals the shared params (then it stays `None`).
pub fn with_loadouts(mut config: MatchConfig, loadouts: &[Loadout]) -> Setup {
    assert_eq!(
        loadouts.len(),
        config.tanks.len(),
        "one loadout per tank spawn"
    );
    for (spawn, l) in config.tanks.iter_mut().zip(loadouts) {
        let p = l.apply(&config.params);
        spawn.params = (p != config.params).then_some(p);
    }
    Setup {
        config,
        loadouts: loadouts.to_vec(),
    }
}

/// Duel config with Blue (tank 0) and Orange (tank 1) loadouts.
///
/// ```
/// use tank::{rules, Loadout};
/// let s = rules::duel(Loadout::DEFAULT, Loadout::DEFAULT);
/// assert_eq!(s.config, rules::config(rules::Mode::Duel));
/// let gc = rules::duel("5-3-1".parse().unwrap(), Loadout::DEFAULT).config;
/// assert_eq!(gc.tank_params(0).max_hp, 460);
/// assert_eq!(gc.tank_params(1).max_hp, 650);
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
        assert_eq!(c.params, Loadout::DEFAULT.params());
        assert_eq!(c.params.max_hp, 650);
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
        assert_eq!(s.config, config(Mode::Duel));
        assert!(s.config.tanks.iter().all(|t| t.params.is_none()));
    }

    #[test]
    fn loadouts_become_per_tank_params() {
        let gc: Loadout = "5-3-1".parse().unwrap();
        let br: Loadout = "4-1-4".parse().unwrap();
        let s = duel(gc, br);
        assert_eq!(s.loadouts, vec![gc, br]);
        assert_eq!(s.config.tank_params(0), gc.params());
        assert_eq!(s.config.tank_params(1), br.params());
        let m = Match::new(s.config, 1);
        assert_eq!((m.tanks()[0].hp, m.tanks()[1].hp), (460, 790));
        assert_eq!(m.tank_params(0).projectile_damage, 29);
        assert_eq!(m.tank_params(1).max_speed, 90.0);
        assert_eq!(m.tank_params(1).fire_cooldown, 64);
        // Observations carry each tank's own max_hp.
        let o = m.observe(0);
        assert_eq!((o.me.max_hp, o.enemies[0].max_hp), (460, 790));
    }

    #[test]
    fn loadouts_build_on_the_shared_params() {
        // The pitfall: a per-tank set replaces the shared one as a whole, so it must
        // carry the shared non-stat fields (here a stationary spread) along.
        let mut c = config(Mode::Duel);
        c.params.projectile_spread_still = Some(128);
        let s = with_loadouts(c, &["2-5-2".parse().unwrap(), Loadout::DEFAULT]);
        assert_eq!(s.config.tank_params(0).projectile_spread_still, Some(128));
        assert_eq!(s.config.tank_params(0).max_speed, 150.0);
        assert_eq!(s.config.tanks[1].params, None);
    }
}
