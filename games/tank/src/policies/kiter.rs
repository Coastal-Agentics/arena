//! Kiter (SPEC policy 2): circle-strafe at mid range and keep moving.

use super::common::{
    action, aim_and_fire, cos_sin, drive_along, nearest_wall, DodgeParams, Jitter, Reflex, Stall,
    StallParams,
};
use engine::{Action, Observation, Policy, Vec2};

/// Kiter numbers (part of the Phase 3 evolution genome).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KiterParams {
    /// Inner edge of the preferred range band, in units.
    pub min_dist: f32,
    /// Outer edge of the preferred range band, in units.
    pub max_dist: f32,
    /// Strafe bend outward (too close) or inward (too far), in degrees.
    pub bend_deg: f32,
    /// Flip strafe direction when a wall is closer than this (and we're moving at it).
    pub wall_margin: f32,
    /// Flip strafe direction at least this often, in ticks.
    pub flip_every: u32,
    /// Each timed flip comes after `flip_every` ± this many ticks (seeded).
    pub flip_jitter: u32,
    /// Minimum ticks between wall-triggered flips (stops flip-flopping in a corner).
    pub wall_flip_cooldown: u32,
    /// Hull steering tolerance.
    pub steer_tol: f32,
    /// Turret alignment tolerance for firing.
    pub aim_tol: f32,
    /// Dodge reflex: look-ahead, threshold and strength (see [`DodgeParams`]).
    pub dodge: DodgeParams,
    /// Stall recovery.
    pub stall: StallParams,
}

impl Default for KiterParams {
    fn default() -> Self {
        Self {
            min_dist: 250.0,
            max_dist: 348.0,
            bend_deg: 30.0,
            wall_margin: 60.0,
            flip_every: 190,
            flip_jitter: 40,
            wall_flip_cooldown: 30,
            steer_tol: 0.2,
            aim_tol: 0.048,
            dodge: DodgeParams {
                horizon: 11.0,
                margin: 0.66,
                chance: 0.95,
            },
            stall: StallParams::default(),
        }
    }
}

/// Holds 250–348 u from `enemies[0]` with its hull perpendicular to `rel` (circle
/// strafe at full throttle), bent 30° outward when too close and inward when too far.
/// Flips strafe direction near a wall, on a stall, and every 190 ± 40 ticks. Lead-aims
/// with tolerance 0.048. Dodges enemy shells (look-ahead 11 ticks, threshold 0.66 u,
/// strength 0.95), which overrides the strafe for that tick.
#[derive(Clone, Debug, PartialEq)]
pub struct Kiter {
    /// Tunables.
    pub params: KiterParams,
    stall: Stall,
    reflex: Reflex,
    /// `+1`: strafe counter-clockwise around the target; `-1`: clockwise.
    side: f32,
    since_flip: u32,
    since_wall_flip: u32,
    next_flip: u32,
    rng: Jitter,
}

impl Default for Kiter {
    fn default() -> Self {
        Self::new(KiterParams::default())
    }
}

impl Kiter {
    /// A kiter with the given numbers (jitter seed 0).
    pub fn new(params: KiterParams) -> Self {
        Self::seeded(params, 0)
    }

    /// A kiter whose starting strafe side and flip timing come from `seed`.
    pub fn seeded(params: KiterParams, seed: u64) -> Self {
        let mut rng = Jitter::new(seed);
        let side = if rng.coin() { 1.0 } else { -1.0 };
        let next_flip = rng.around(params.flip_every, params.flip_jitter);
        Self {
            params,
            stall: Stall::default(),
            reflex: Reflex::new(seed),
            side,
            since_flip: 0,
            since_wall_flip: u32::MAX,
            next_flip,
            rng,
        }
    }

    fn flip(&mut self) {
        self.side = -self.side;
        self.since_flip = 0;
        self.next_flip = self
            .rng
            .around(self.params.flip_every, self.params.flip_jitter);
    }
}

/// Rotate `v` counter-clockwise by the angle with the given cosine and sine.
fn rotate(v: Vec2, cos: f32, sin: f32) -> Vec2 {
    Vec2::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos)
}

/// Desired hull direction: the tangent to the circle around the target (`side` picks
/// the way round), bent `bend_deg` outward when closer than `min_dist` and inward when
/// farther than `max_dist`.
fn strafe_direction(p: &KiterParams, rel: Vec2, side: f32) -> Vec2 {
    let tangent = rel.perp() * side;
    let dist_sq = rel.length_squared();
    let bend = if dist_sq < p.min_dist * p.min_dist {
        1.0 // outward
    } else if dist_sq > p.max_dist * p.max_dist {
        -1.0 // inward
    } else {
        return tangent;
    };
    let (c, s) = cos_sin(p.bend_deg);
    let ccw = rotate(tangent, c, s);
    let cw = rotate(tangent, c, -s);
    // The rotation that points more away from the target is "outward".
    let away = -rel;
    let outward_is_ccw = ccw.dot(away) >= cw.dot(away);
    if (bend > 0.0) == outward_is_ccw {
        ccw
    } else {
        cw
    }
}

impl Policy for Kiter {
    fn act(&mut self, obs: &Observation) -> Action {
        let p = self.params;
        let Some(target) = obs.enemies.first() else {
            self.stall.record(0.0);
            return Action::default();
        };
        self.since_flip = self.since_flip.saturating_add(1);
        self.since_wall_flip = self.since_wall_flip.saturating_add(1);
        let (turret_turn, fire) = aim_and_fire(obs, target, p.aim_tol);
        let (stalled, new_stall) = self.stall.update(&p.stall, obs.me.vel);
        let threat = self.reflex.update(obs, &p.dodge);
        if new_stall {
            self.flip();
        }
        if stalled {
            let (throttle, turn) = Stall::recovery();
            self.stall.record(throttle);
            return action(throttle, turn, turret_turn, fire);
        }
        if self.since_flip >= self.next_flip {
            self.flip();
        }

        let mut want = strafe_direction(&p, target.rel, self.side);
        // Wall flip: close to a wall and heading into it.
        if nearest_wall(obs) < p.wall_margin && self.since_wall_flip >= p.wall_flip_cooldown {
            let w = obs.walls;
            let into = (w.left < p.wall_margin && want.x < 0.0)
                || (w.right < p.wall_margin && want.x > 0.0)
                || (w.bottom < p.wall_margin && want.y < 0.0)
                || (w.top < p.wall_margin && want.y > 0.0);
            if into {
                self.flip();
                self.since_wall_flip = 0;
                want = strafe_direction(&p, target.rel, self.side);
            }
        }
        if let Some(away) = threat {
            want = away;
        }
        let (throttle, turn) = drive_along(obs.me.heading, want, p.steer_tol);
        self.stall.record(throttle);
        action(throttle, turn, turret_turn, fire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strafe_is_tangent_in_band_and_bends_out_or_in() {
        let p = KiterParams::default();
        let rel = Vec2::new(300.0, 0.0);
        for side in [1.0, -1.0] {
            let t = strafe_direction(&p, rel, side);
            assert_eq!(t.dot(rel), 0.0, "in band: pure tangent");
            let near = strafe_direction(&p, Vec2::new(200.0, 0.0), side);
            assert!(
                near.dot(Vec2::new(200.0, 0.0)) < 0.0,
                "too close: bends away"
            );
            let far = strafe_direction(&p, Vec2::new(400.0, 0.0), side);
            assert!(far.dot(Vec2::new(400.0, 0.0)) > 0.0, "too far: bends in");
            // 30° bend: the radial component is sin 30° of the length.
            let r = -near.normalize().dot(Vec2::X);
            assert!((r - 0.5).abs() < 0.01, "{r}");
        }
    }

    #[test]
    fn bend_uses_the_table_trig() {
        let (c, s) = cos_sin(30.0);
        assert!(
            (c - 0.866_025_4).abs() < 0.01 && (s - 0.5).abs() < 0.01,
            "{c} {s}"
        );
    }
}
