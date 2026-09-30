//! Charger (SPEC policy 1): drive straight at the target and trade shots up close.

use super::common::{action, aim_and_fire, waypoint, Stall, StallParams};
use engine::angle::turn_toward;
use engine::{Action, Observation, Policy};

/// Charger numbers (part of the Phase 3 evolution genome).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChargerParams {
    /// Hull steering tolerance toward the target.
    pub steer_tol: f32,
    /// Stop (throttle 0) once the target is closer than this, in units.
    pub stop_dist: f32,
    /// Turret alignment tolerance for firing.
    pub aim_tol: f32,
    /// Extra clearance when routing around obstacles (see [`waypoint`]).
    pub route_margin: f32,
    /// Stall recovery.
    pub stall: StallParams,
}

impl Default for ChargerParams {
    fn default() -> Self {
        Self {
            steer_tol: 0.2,
            stop_dist: 60.0,
            aim_tol: 0.10,
            route_margin: 12.0,
            stall: StallParams::default(),
        }
    }
}

/// Steers at `enemies[0]` (`tol 0.2`, routing around a pillar in the way) at full
/// throttle until it is within 60 u, then holds; lead-aims with tolerance 0.10 and fires when aligned, ready and in sight.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Charger {
    /// Tunables.
    pub params: ChargerParams,
    stall: Stall,
}

impl Charger {
    /// A charger with the given numbers.
    pub fn new(params: ChargerParams) -> Self {
        Self {
            params,
            stall: Stall::default(),
        }
    }
}

impl Policy for Charger {
    fn act(&mut self, obs: &Observation) -> Action {
        let p = self.params;
        let Some(target) = obs.enemies.first() else {
            self.stall.record(0.0);
            return Action::default();
        };
        let (turret_turn, fire) = aim_and_fire(obs, target, p.aim_tol);
        let (stalled, _) = self.stall.update(&p.stall, obs.me.vel);
        let (throttle, turn) = if stalled {
            Stall::recovery()
        } else {
            let goal = waypoint(obs, target.pos, obs.me.radius, p.route_margin);
            let steer = turn_toward(obs.me.heading, goal - obs.me.pos, p.steer_tol) as f32;
            let close = target.dist_sq < p.stop_dist * p.stop_dist;
            (if close { 0.0 } else { 1.0 }, steer)
        };
        self.stall.record(throttle);
        action(throttle, turn, turret_turn, fire)
    }
}
