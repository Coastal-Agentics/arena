//! Charger (SPEC policy 1): drive straight at the target and trade shots up close.

use super::common::{
    action, aim_and_fire, cos_sin, dodge, drive_along, waypoint, Jitter, Stall, StallParams,
    DODGE_HORIZON,
};
use engine::angle::turn_toward;
use engine::{Action, Observation, Policy, Vec2};

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
    /// Zig-zag while closing: steer this many degrees left/right of the target line
    /// (0 = straight at it). Radial approaches are the easiest shots in the game.
    pub weave_deg: f32,
    /// Ticks per zig or zag.
    pub weave_period: u32,
    /// Each zig lasts `weave_period` ± this many ticks (seeded).
    pub weave_jitter: u32,
    /// Stop weaving (charge straight) once closer than this, in units.
    pub weave_until: f32,
    /// Dodge enemy shells that would hit within this many ticks (0 = never dodge).
    pub dodge_horizon: f32,
    /// Extra miss distance (beyond the radius) that still counts as a threat.
    pub dodge_margin: f32,
    /// Stall recovery.
    pub stall: StallParams,
}

impl Default for ChargerParams {
    fn default() -> Self {
        Self {
            steer_tol: 0.2,
            stop_dist: 60.0,
            aim_tol: 0.05,
            route_margin: 12.0,
            weave_deg: 20.0,
            weave_period: 30,
            weave_jitter: 10,
            weave_until: 150.0,
            dodge_horizon: DODGE_HORIZON,
            dodge_margin: 4.0,
            stall: StallParams::default(),
        }
    }
}

/// Steers at `enemies[0]` (`tol 0.2`, routing around a pillar in the way, weaving ±20°
/// every 30 ticks until within 150 u) at full throttle until it is within 60 u, then
/// holds; lead-aims with tolerance 0.05 and fires when aligned, ready and in sight.
#[derive(Clone, Debug, PartialEq)]
pub struct Charger {
    /// Tunables.
    pub params: ChargerParams,
    stall: Stall,
    rng: Jitter,
    zig: bool,
    zig_left: u32,
}

impl Charger {
    /// A charger with the given numbers (jitter seed 0).
    pub fn new(params: ChargerParams) -> Self {
        Self::seeded(params, 0)
    }

    /// A charger whose weave timing comes from `seed`.
    pub fn seeded(params: ChargerParams, seed: u64) -> Self {
        let mut rng = Jitter::new(seed);
        let zig = rng.coin();
        Self {
            params,
            stall: Stall::default(),
            rng,
            zig,
            zig_left: 0,
        }
    }
}

impl Default for Charger {
    fn default() -> Self {
        Self::new(ChargerParams::default())
    }
}

impl Policy for Charger {
    fn act(&mut self, obs: &Observation) -> Action {
        let p = self.params;
        if self.zig_left == 0 {
            self.zig = !self.zig;
            self.zig_left = self.rng.around(p.weave_period, p.weave_jitter);
        }
        self.zig_left -= 1;
        let Some(target) = obs.enemies.first() else {
            self.stall.record(0.0);
            return Action::default();
        };
        let (turret_turn, fire) = aim_and_fire(obs, target, p.aim_tol);
        let (stalled, _) = self.stall.update(&p.stall, obs.me.vel);
        let threat = (p.dodge_horizon > 0.0)
            .then(|| dodge(obs, p.dodge_horizon, p.dodge_margin))
            .flatten();
        let (throttle, turn) = if stalled {
            Stall::recovery()
        } else if let Some(away) = threat {
            drive_along(obs.me.heading, away, p.steer_tol)
        } else {
            let goal = waypoint(obs, target.pos, obs.me.radius, p.route_margin);
            let mut to = goal - obs.me.pos;
            if p.weave_deg != 0.0 && target.dist_sq > p.weave_until * p.weave_until {
                let (c, s) = cos_sin(if self.zig { p.weave_deg } else { -p.weave_deg });
                to = Vec2::new(to.x * c - to.y * s, to.x * s + to.y * c);
            }
            let steer = turn_toward(obs.me.heading, to, p.steer_tol) as f32;
            let close = target.dist_sq < p.stop_dist * p.stop_dist;
            (if close { 0.0 } else { 1.0 }, steer)
        };
        self.stall.record(throttle);
        action(throttle, turn, turret_turn, fire)
    }
}
