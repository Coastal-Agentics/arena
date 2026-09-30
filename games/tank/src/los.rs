//! Line of sight (SPEC "Observation": `los: bool` per tank; engine ask #2).
//!
//! Tank-to-tank sight is the engine's `TankObs::los` ([`has_los`]), which is
//! `Arena::segment_clear` between the two centres: obstacles are closed (grazing an
//! edge or corner blocks), running along an arena wall is clear, tanks never block.
//!
//! Policies also need sight lines from points that aren't tanks (the sniper scoring
//! candidate spots, route planning). An observation carries the obstacles but not the
//! `Arena`, so [`segment_clear`] tests the obstacles directly with the same slab test
//! ([`engine::Rect::segment_entry`]). For segments inside the arena it agrees with
//! `Arena::segment_clear` (checked by a test), without cloning the arena per query.

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

/// Whether the observer can see `target`: the engine's `TankObs::los`.
pub fn has_los(_obs: &Observation, target: &TankObs) -> bool {
    target.los
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
    fn segment_clear_agrees_with_the_engine_inside_the_arena() {
        let arena = config(Mode::Duel).arena;
        // Deterministic LCG over points inside the arena, plus pillar-corner cases.
        let mut x: u64 = 0x5eed;
        let mut next = || {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (x >> 40) as f32 / (1u64 << 24) as f32
        };
        let mut pts: Vec<Vec2> = (0..400)
            .map(|_| Vec2::new(next() * 800.0, next() * 600.0))
            .collect();
        for o in &arena.obstacles {
            pts.extend([
                o.min,
                o.max,
                Vec2::new(o.min.x, o.max.y),
                Vec2::new(o.max.x, o.min.y),
            ]);
        }
        pts.extend([Vec2::ZERO, Vec2::new(800.0, 0.0), Vec2::new(0.0, 600.0)]);
        for w in pts.windows(2) {
            assert_eq!(
                segment_clear(&arena.obstacles, w[0], w[1]),
                arena.segment_clear(w[0], w[1]),
                "{} -> {}",
                w[0],
                w[1]
            );
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
        assert_eq!(
            has_los(&obs, &obs.enemies[0]),
            segment_clear(&obs.obstacles, obs.me.pos, obs.enemies[0].pos)
        );
    }
}
