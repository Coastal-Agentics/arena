//! Sniper (SPEC policy 3): hold a far spot with a sight line, fire only when precisely
//! aligned, back off from rushers, peek around a pillar when blind.

use super::common::{
    action, aim_and_fire, drive_along, waypoint, DodgeParams, Jitter, Reflex, Stall, StallParams,
};
use crate::los;
use engine::{Action, Observation, Policy, Rect, Vec2};

/// Sniper numbers (part of the Phase 3 evolution genome).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SniperParams {
    /// Candidate spots keep at least this distance from every arena edge.
    pub wall_margin: f32,
    /// Spacing of the candidate-spot grid, in units.
    pub grid_step: f32,
    /// Extra clearance (beyond the tank radius) a candidate spot keeps from obstacles.
    pub obstacle_clearance: f32,
    /// Re-score candidate spots this often, in ticks (at once if the spot loses sight).
    pub replan_every: u32,
    /// Move to a newly scored spot only if it is at least this much farther from the
    /// target than the current one (units), so the sniper settles instead of drifting.
    pub replan_gain: f32,
    /// Close enough to the spot to stop (throttle 0), in units.
    pub arrive_radius: f32,
    /// Turret alignment tolerance for firing.
    pub aim_tol: f32,
    /// Evade when the target is closer than this, in units.
    pub evade_dist: f32,
    /// Evade duration, in ticks.
    pub evade_ticks: u32,
    /// Each evade lasts `evade_ticks` ± this many ticks (seeded).
    pub evade_jitter: u32,
    /// Ticks without line of sight before moving along the nearest pillar to peek.
    pub blind_ticks: u32,
    /// How far outside a pillar corner the peek point sits (beyond the tank radius).
    pub peek_offset: f32,
    /// Hull steering tolerance.
    pub steer_tol: f32,
    /// Extra clearance when routing around obstacles (see [`waypoint`]).
    pub route_margin: f32,
    /// Dodge reflex: look-ahead, threshold and strength (see [`DodgeParams`]).
    pub dodge: DodgeParams,
    /// Stall recovery.
    pub stall: StallParams,
}

impl Default for SniperParams {
    fn default() -> Self {
        Self {
            wall_margin: 60.0,
            grid_step: 20.0,
            obstacle_clearance: 8.0,
            replan_every: 30,
            replan_gain: 80.0,
            arrive_radius: 10.0,
            aim_tol: 0.025,
            evade_dist: 238.0,
            evade_ticks: 60,
            evade_jitter: 15,
            blind_ticks: 120,
            peek_offset: 12.0,
            steer_tol: 0.2,
            route_margin: 12.0,
            dodge: DodgeParams {
                horizon: 15.0,
                margin: 5.4,
                chance: 0.61,
            },
            stall: StallParams::default(),
        }
    }
}

/// Relocates to the point on its own half (≥ 60 u from walls) that maximises distance
/// to `enemies[0]` while keeping line of sight, then holds still (throttle 0) and fires
/// with tolerance 0.025. If the target comes within 238 u it drives away perpendicular
/// to `rel` for 60 ticks. After 120 ticks without sight it moves along the nearest
/// pillar edge until it sees the target. Dodges enemy shells first (look-ahead 15
/// ticks, threshold 5.4 u, strength 0.61).
///
/// "Its half" is the half of the arena (split at `x = width / 2`) it first observes
/// itself in.
#[derive(Clone, Debug, PartialEq)]
pub struct Sniper {
    /// Tunables.
    pub params: SniperParams,
    stall: Stall,
    reflex: Reflex,
    left_half: Option<bool>,
    spot: Option<Vec2>,
    replan_in: u32,
    blind: u32,
    evading: u32,
    evade_dir: Vec2,
    rng: Jitter,
}

impl Sniper {
    /// A sniper with the given numbers (jitter seed 0).
    pub fn new(params: SniperParams) -> Self {
        Self::seeded(params, 0)
    }

    /// A sniper whose evade timing comes from `seed`.
    pub fn seeded(params: SniperParams, seed: u64) -> Self {
        Self {
            params,
            stall: Stall::default(),
            reflex: Reflex::new(seed),
            left_half: None,
            spot: None,
            replan_in: 0,
            blind: 0,
            evading: 0,
            evade_dir: Vec2::ZERO,
            rng: Jitter::new(seed),
        }
    }

    /// The best spot on our half for shooting at `target_pos`, or `None` if no
    /// candidate has a sight line. Grid order is fixed (x, then y ascending) and only a
    /// strictly farther point replaces the best, so the choice is deterministic.
    pub fn best_spot(&self, obs: &Observation, target_pos: Vec2) -> Option<Vec2> {
        let p = &self.params;
        let size = obs.arena_size;
        let left = self.left_half.unwrap_or(obs.me.pos.x < size.x * 0.5);
        let (x_lo, x_hi) = if left {
            (p.wall_margin, size.x * 0.5)
        } else {
            (size.x * 0.5, size.x - p.wall_margin)
        };
        let clearance = obs.me.radius + p.obstacle_clearance;
        let mut best: Option<(f32, Vec2)> = None;
        let nx = ((x_hi - x_lo) / p.grid_step).floor() as i32;
        let ny = ((size.y - 2.0 * p.wall_margin) / p.grid_step).floor() as i32;
        for i in 0..=nx {
            for j in 0..=ny {
                let c = Vec2::new(
                    x_lo + i as f32 * p.grid_step,
                    p.wall_margin + j as f32 * p.grid_step,
                );
                if obs
                    .obstacles
                    .iter()
                    .any(|o| o.overlaps_circle(c, clearance))
                    || !los::segment_clear(&obs.obstacles, c, target_pos)
                {
                    continue;
                }
                let d = (target_pos - c).length_squared();
                if best.is_none_or(|(bd, _)| d > bd) {
                    best = Some((d, c));
                }
            }
        }
        best.map(|(_, c)| c)
    }

    /// Where to go to peek: the corner of the nearest obstacle (pushed out diagonally
    /// by radius + `peek_offset`) that can see the target, else its nearest corner.
    fn peek_point(&self, obs: &Observation, target_pos: Vec2) -> Option<Vec2> {
        let me = obs.me.pos;
        let nearest: &Rect = obs.obstacles.iter().min_by(|a, b| {
            let da = (a.closest_point(me) - me).length_squared();
            let db = (b.closest_point(me) - me).length_squared();
            da.total_cmp(&db)
        })?;
        let off = obs.me.radius + self.params.peek_offset;
        let corners = [
            Vec2::new(nearest.min.x - off, nearest.min.y - off),
            Vec2::new(nearest.max.x + off, nearest.min.y - off),
            Vec2::new(nearest.min.x - off, nearest.max.y + off),
            Vec2::new(nearest.max.x + off, nearest.max.y + off),
        ];
        let by_dist = |c: &Vec2| (*c - me).length_squared();
        let mut seeing: Vec<Vec2> = corners
            .iter()
            .copied()
            .filter(|&c| los::segment_clear(&obs.obstacles, c, target_pos))
            .collect();
        let pool = if seeing.is_empty() {
            corners.to_vec()
        } else {
            std::mem::take(&mut seeing)
        };
        pool.into_iter()
            .min_by(|a, b| by_dist(a).total_cmp(&by_dist(b)))
    }

    fn go_to(&self, obs: &Observation, point: Vec2) -> (f32, f32) {
        let to = point - obs.me.pos;
        let via = waypoint(obs, point, obs.me.radius, self.params.route_margin) - obs.me.pos;
        if to.length_squared() <= self.params.arrive_radius * self.params.arrive_radius {
            (0.0, 0.0)
        } else {
            drive_along(obs.me.heading, via, self.params.steer_tol)
        }
    }
}

impl Default for Sniper {
    fn default() -> Self {
        Self::new(SniperParams::default())
    }
}

impl Policy for Sniper {
    fn act(&mut self, obs: &Observation) -> Action {
        let p = self.params;
        if self.left_half.is_none() {
            self.left_half = Some(obs.me.pos.x < obs.arena_size.x * 0.5);
        }
        let Some(target) = obs.enemies.first() else {
            self.stall.record(0.0);
            return Action::default();
        };
        let (turret_turn, fire) = aim_and_fire(obs, target, p.aim_tol);
        let sees = los::has_los(obs, target);
        self.blind = if sees {
            0
        } else {
            self.blind.saturating_add(1)
        };

        let (stalled, _) = self.stall.update(&p.stall, obs.me.vel);
        let threat = self.reflex.update(obs, &p.dodge);
        let (throttle, turn) = if stalled {
            Stall::recovery()
        } else if let Some(away) = threat {
            drive_along(obs.me.heading, away, p.steer_tol)
        } else if self.evading > 0 {
            self.evading -= 1;
            drive_along(obs.me.heading, self.evade_dir, p.steer_tol)
        } else if target.dist_sq < p.evade_dist * p.evade_dist {
            // Perpendicular to rel, on the side with more room (toward the centre).
            let perp = target.rel.perp();
            let centre = obs.arena_size * 0.5 - obs.me.pos;
            self.evade_dir = if perp.dot(centre) >= 0.0 { perp } else { -perp };
            self.evading = self
                .rng
                .around(p.evade_ticks, p.evade_jitter)
                .saturating_sub(1);
            self.spot = None; // re-pick once the evade ends
            drive_along(obs.me.heading, self.evade_dir, p.steer_tol)
        } else if self.blind >= p.blind_ticks {
            match self.peek_point(obs, target.pos) {
                Some(pt) => self.go_to(obs, pt),
                None => (0.0, 0.0),
            }
        } else {
            let spot_blind = self
                .spot
                .is_some_and(|s| !los::segment_clear(&obs.obstacles, s, target.pos));
            if self.spot.is_none() || self.replan_in == 0 || spot_blind {
                if let Some(s) = self.best_spot(obs, target.pos) {
                    let better = match self.spot {
                        Some(cur) if !spot_blind => {
                            (target.pos - s).length() > (target.pos - cur).length() + p.replan_gain
                        }
                        _ => true,
                    };
                    if better {
                        self.spot = Some(s);
                    }
                }
                self.replan_in = p.replan_every;
            }
            self.replan_in = self.replan_in.saturating_sub(1);
            match self.spot {
                Some(s) => self.go_to(obs, s),
                None => (0.0, 0.0),
            }
        };
        self.stall.record(throttle);
        action(throttle, turn, turret_turn, fire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{config, Mode};
    use engine::Match;

    #[test]
    fn best_spot_is_on_its_half_clear_of_walls_and_sees_the_target() {
        let m = Match::new(config(Mode::Duel), 1);
        for id in [0, 1] {
            let obs = m.observe(id);
            let s = Sniper::default();
            let target = obs.enemies[0].pos;
            let spot = s.best_spot(&obs, target).expect("a spot exists");
            let left = id == 0;
            assert_eq!(spot.x <= 400.0, left, "{spot}");
            assert!(spot.x >= 60.0 && spot.x <= 740.0 && spot.y >= 60.0 && spot.y <= 540.0);
            assert!(los::segment_clear(&obs.obstacles, spot, target));
            // Farther than the spawn, which has no sight line.
            assert!((target - spot).length_squared() > 0.0);
        }
        // Mirror symmetry: both sides find an equally distant spot.
        let a = Sniper::default()
            .best_spot(&m.observe(0), m.tanks()[1].pos)
            .unwrap();
        let b = Sniper::default()
            .best_spot(&m.observe(1), m.tanks()[0].pos)
            .unwrap();
        let da = (m.tanks()[1].pos - a).length_squared();
        let db = (m.tanks()[0].pos - b).length_squared();
        assert_eq!(da, db, "{a} vs {b}");
    }
}
