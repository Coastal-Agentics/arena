//! Line of sight (SPEC "Observation": `los: bool` per tank; engine ask #2).
//!
//! **Stand-in.** The engine has no `Arena::segment_clear` and no `TankObs::los` yet, so
//! the policies call [`has_los`], which computes it from the observation with the
//! engine's existing public slab test ([`engine::Rect::segment_entry`]). When the engine
//! ask lands, [`has_los`] becomes `target.los` and [`segment_clear`] a call to
//! `Arena::segment_clear`; no policy code changes.
//!
//! Definition: the segment between the two tank centres touches no obstacle (closed
//! rectangles, so grazing a pillar corner blocks). Tanks don't block sight, and the
//! arena walls can't (both ends are inside the arena).

use engine::{Observation, Rect, TankObs, Vec2};

/// True if the segment `a → b` touches none of `obstacles`.
///
/// ```
/// use engine::{Rect, Vec2};
/// let pillar = [Rect::new(Vec2::new(250.0, 200.0), Vec2::new(300.0, 400.0))];
/// assert!(!tank::los::segment_clear(&pillar, Vec2::new(100.0, 300.0), Vec2::new(700.0, 300.0)));
/// assert!(tank::los::segment_clear(&pillar, Vec2::new(100.0, 500.0), Vec2::new(700.0, 500.0)));
/// ```
pub fn segment_clear(obstacles: &[Rect], a: Vec2, b: Vec2) -> bool {
    let d = b - a;
    obstacles.iter().all(|o| o.segment_entry(a, d).is_none())
}

/// Whether the observer can see `target` (centre to centre, obstacles only).
pub fn has_los(obs: &Observation, target: &TankObs) -> bool {
    segment_clear(&obs.obstacles, obs.me.pos, target.pos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{config, Mode};
    use engine::Match;

    fn duel_obstacles() -> Vec<Rect> {
        config(Mode::Duel).arena.obstacles
    }

    #[test]
    fn pillars_block_the_centre_line_and_open_lanes_are_clear() {
        let o = duel_obstacles();
        let p = |x, y| Vec2::new(x, y);
        // Spawn to spawn: both pillars in the way.
        assert!(!segment_clear(&o, p(100.0, 300.0), p(700.0, 300.0)));
        // Above and below the pillars (y 200..400) is open.
        assert!(segment_clear(&o, p(100.0, 450.0), p(700.0, 450.0)));
        assert!(segment_clear(&o, p(100.0, 150.0), p(700.0, 150.0)));
        // The middle lane between the pillars is open vertically.
        assert!(segment_clear(&o, p(400.0, 20.0), p(400.0, 580.0)));
        // Grazing a corner counts as blocked (closed rectangles).
        assert!(!segment_clear(&o, p(200.0, 400.0), p(350.0, 400.0)));
        // Symmetric.
        for (a, b) in [
            (p(60.0, 60.0), p(740.0, 540.0)),
            (p(100.0, 300.0), p(400.0, 300.0)),
        ] {
            assert_eq!(segment_clear(&o, a, b), segment_clear(&o, b, a));
        }
    }

    #[test]
    fn has_los_from_an_observation() {
        let m = Match::new(config(Mode::Duel), 1);
        let obs = m.observe(0);
        assert!(
            !has_los(&obs, &obs.enemies[0]),
            "spawns are hidden by the pillars"
        );
        let mut cfg = config(Mode::Duel);
        cfg.tanks[1].pos = Some(Vec2::new(700.0, 500.0));
        cfg.tanks[0].pos = Some(Vec2::new(100.0, 500.0));
        let obs = Match::new(cfg, 1).observe(0);
        assert!(has_los(&obs, &obs.enemies[0]));
    }
}
