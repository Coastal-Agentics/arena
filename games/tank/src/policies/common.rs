//! Shared policy pieces (SPEC "Scripted policies", "Shared"): lead aim, the fire rule,
//! stall recovery, and steering helpers.

use crate::los;
use engine::angle::{dir, turn_toward, Heading};
use engine::{Action, Observation, Rect, TankObs, TankParams, Vec2, TICK_HZ};

/// Shell speed in units per tick (360 u/s at 60 Hz = 6). Fixed for every loadout.
pub fn shell_speed_per_tick() -> f32 {
    TankParams::default().projectile_speed / TICK_HZ as f32
}

/// Lead aim point relative to the observer: `L = rel + vel · (dist / 6)`, with `dist`
/// from `f32::sqrt` (IEEE-754 correctly rounded, so deterministic everywhere).
pub fn lead_point(target: &TankObs) -> Vec2 {
    let dist = target.dist_sq.sqrt();
    target.rel + target.vel * (dist / shell_speed_per_tick())
}

/// BAU per radian, for the small-angle turret scaling below.
const BAU_PER_RADIAN: f32 = 65536.0 / std::f32::consts::TAU;

/// Turret command toward `aim` (a relative vector) with tolerance `tol`. Returns
/// `(turret_turn, aligned)`.
///
/// Direction and the "aligned" test are exactly `turn_toward(turret, aim, tol)`. Near
/// alignment (target ahead, error under one full turret step) the command is scaled to
/// the remaining error, `sin(err) · BAU/rad / turret_turn_rate`, so the turret can
/// settle inside tolerances narrower than one 546-BAU (3°) step: the sniper's 0.02 is
/// ±1.1°, and a bang-bang turret would oscillate around it forever.
pub fn aim_turret(turret: Heading, aim: Vec2, tol: f32) -> (f32, bool) {
    let side = turn_toward(turret, aim, tol);
    if side == 0 {
        return (0.0, true);
    }
    let len_sq = aim.length_squared();
    let d = dir(turret);
    if d.dot(aim) > 0.0 && len_sq > 0.0 {
        let sin_err = d.perp_dot(aim) / len_sq.sqrt();
        let step = TankParams::default().turret_turn_rate as f32;
        let cmd = sin_err * BAU_PER_RADIAN / step;
        if cmd.abs() < 1.0 {
            return (cmd, false);
        }
    }
    (side as f32, false)
}

/// Aim at `target` with lead and decide whether to fire: aligned within `tol`, gun
/// ready (`cooldown == 0`) and line of sight. Returns `(turret_turn, fire)`.
pub fn aim_and_fire(obs: &Observation, target: &TankObs, tol: f32) -> (f32, bool) {
    let (turret_turn, aligned) = aim_turret(obs.me.turret, lead_point(target), tol);
    let fire = aligned && obs.me.cooldown == 0 && los::has_los(obs, target);
    (turret_turn, fire)
}

/// Drive along world direction `want` with hull tolerance `tol`, using whichever end of
/// the hull is closer: forward (throttle `+1`) when `want` is ahead or beside, reverse
/// (`-1`, steering the tail) when it is behind. Returns `(throttle, turn)`.
///
/// Reversing is as fast as driving forward in the engine, so a direction flip (the
/// kiter's strafe flip, the sniper's evade) takes effect at once instead of after a
/// 90-tick U-turn.
pub fn drive_along(heading: Heading, want: Vec2, tol: f32) -> (f32, f32) {
    if want.length_squared() == 0.0 {
        return (0.0, 0.0);
    }
    if dir(heading).dot(want) >= 0.0 {
        (1.0, turn_toward(heading, want, tol) as f32)
    } else {
        (-1.0, turn_toward(heading, -want, tol) as f32)
    }
}

/// Stall recovery parameters (shared by all three policies).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StallParams {
    /// Speed (units per tick) below which a tank that asked to move counts as stuck.
    pub min_speed: f32,
    /// Consecutive stuck ticks that trigger recovery.
    pub stuck_ticks: u32,
    /// Ticks to reverse (turning `+1`) once triggered.
    pub reverse_ticks: u32,
}

impl Default for StallParams {
    /// SPEC: `|vel| < 0.2` for 10 ticks → reverse 20 ticks turning +1.
    fn default() -> Self {
        Self {
            min_speed: 0.2,
            stuck_ticks: 10,
            reverse_ticks: 20,
        }
    }
}

/// Stall detector: call [`Stall::update`] at the start of each `act`, and
/// [`Stall::record`] with the throttle actually returned.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stall {
    last_throttle: f32,
    stuck: u32,
    reversing: u32,
}

impl Stall {
    /// Feed this tick's observed velocity. Returns true if recovery is active this tick
    /// (the caller should output `throttle = -1, turn = +1`). `just_triggered` tells a
    /// caller (the kiter) that a new stall began this tick.
    pub fn update(&mut self, p: &StallParams, vel: Vec2) -> (bool, bool) {
        if self.reversing > 0 {
            self.reversing -= 1;
            return (true, false);
        }
        let slow = vel.length_squared() < p.min_speed * p.min_speed;
        if self.last_throttle != 0.0 && slow {
            self.stuck += 1;
        } else {
            self.stuck = 0;
        }
        if self.stuck >= p.stuck_ticks {
            self.stuck = 0;
            // This tick is the first of `reverse_ticks`.
            self.reversing = p.reverse_ticks.saturating_sub(1);
            return (true, true);
        }
        (false, false)
    }

    /// Record the throttle this tick's action used.
    pub fn record(&mut self, throttle: f32) {
        self.last_throttle = throttle;
    }

    /// The recovery action's hull part: reverse, turning counter-clockwise.
    pub fn recovery() -> (f32, f32) {
        (-1.0, 1.0)
    }
}

/// Obstacle-aware steering goal: `goal` itself if the straight path is clear for a tank
/// of radius `radius` (plus half of `margin`), else the corner of an obstacle (grown by
/// `radius + margin`) that is straight-line reachable and minimises
/// `|me → corner| + |corner → goal|`. One hop is enough for the two-pillar arena; with
/// no reachable corner it returns `goal` and stall recovery takes over.
///
/// The spec's "steer at target" taken literally leaves a charger grinding along a
/// pillar face at a crawl for seconds (fast enough not to count as a stall); this
/// keeps the intent (close fast) around cover.
pub fn waypoint(obs: &Observation, goal: Vec2, radius: f32, margin: f32) -> Vec2 {
    let me = obs.me.pos;
    let grow = |r: &Rect, by: f32| Rect::new(r.min - Vec2::splat(by), r.max + Vec2::splat(by));
    let test: Vec<Rect> = obs
        .obstacles
        .iter()
        .map(|r| grow(r, radius + margin * 0.5))
        .collect();
    let clear = |a: Vec2, b: Vec2| crate::los::segment_clear(&test, a, b);
    if clear(me, goal) {
        return goal;
    }
    let mut best: Option<(f32, Vec2)> = None;
    for r in &obs.obstacles {
        let g = grow(r, radius + margin);
        for c in [
            g.min,
            Vec2::new(g.max.x, g.min.y),
            Vec2::new(g.min.x, g.max.y),
            g.max,
        ] {
            let outside =
                c.x < 0.0 || c.y < 0.0 || c.x > obs.arena_size.x || c.y > obs.arena_size.y;
            if outside || !clear(me, c) {
                continue;
            }
            let cost = (c - me).length() + (goal - c).length();
            if best.is_none_or(|(bc, _)| cost < bc) {
                best = Some((cost, c));
            }
        }
    }
    best.map_or(goal, |(_, c)| c)
}

/// Distance to the nearest arena edge.
pub fn nearest_wall(obs: &Observation) -> f32 {
    let w = obs.walls;
    w.left.min(w.right).min(w.bottom).min(w.top)
}

/// Assemble an action.
pub fn action(throttle: f32, turn: f32, turret_turn: f32, fire: bool) -> Action {
    Action {
        throttle,
        turn,
        turret_turn,
        fire,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::angle::{from_degrees, QUARTER_TURN};

    fn enemy(rel: Vec2, vel: Vec2) -> TankObs {
        TankObs {
            id: 1,
            team: 1,
            pos: rel,
            rel,
            dist_sq: rel.length_squared(),
            vel,
            heading: 0,
            turret: 0,
            hp: 100,
        }
    }

    #[test]
    fn waypoint_routes_around_a_pillar() {
        use crate::rules::{config, Mode};
        let m = engine::Match::new(config(Mode::Duel), 1);
        let obs = m.observe(0);
        // Spawn (100,300) to (400,300): the left pillar (250..300 × 200..400) is in the
        // way, so the goal is a grown corner, reachable in a straight line.
        let goal = Vec2::new(400.0, 300.0);
        let w = waypoint(&obs, goal, 16.0, 12.0);
        assert_ne!(w, goal);
        assert!(w.x < 250.0 && (w.y < 200.0 || w.y > 400.0), "{w}");
        // A clear path goes straight.
        let open = Vec2::new(200.0, 300.0);
        assert_eq!(waypoint(&obs, open, 16.0, 12.0), open);
    }

    #[test]
    fn shell_speed_is_6() {
        assert_eq!(shell_speed_per_tick(), 6.0);
    }

    #[test]
    fn lead_point_leads_by_flight_time() {
        // 300 u away: 50 ticks of flight; moving +2 u/tick in y → lead 100 u.
        let e = enemy(Vec2::new(300.0, 0.0), Vec2::new(0.0, 2.0));
        assert_eq!(lead_point(&e), Vec2::new(300.0, 100.0));
        let still = enemy(Vec2::new(300.0, 40.0), Vec2::ZERO);
        assert_eq!(lead_point(&still), still.rel);
    }

    #[test]
    fn fine_aim_settles_inside_a_tight_tolerance() {
        // Simulate the turret alone: from a range of offsets it reaches the 0.02 window
        // and stays aligned, where a ±1 bang-bang turret can skip over it.
        let step = TankParams::default().turret_turn_rate as f32;
        for start_deg in [-170, -40, -3, -1, 1, 2, 5, 90, 179] {
            let aim = Vec2::new(1.0, 0.0);
            let mut h: Heading = from_degrees(start_deg);
            let mut aligned_at = None;
            for t in 0..200 {
                let (cmd, ok) = aim_turret(h, aim, 0.02);
                if ok {
                    aligned_at = Some(t);
                    break;
                }
                assert!((-1.0..=1.0).contains(&cmd));
                h = h.wrapping_add_signed((cmd * step) as i16);
            }
            assert!(aligned_at.is_some(), "never aligned from {start_deg}°");
        }
    }

    #[test]
    fn drive_along_uses_the_nearer_end() {
        // Facing +X: target direction +X → forward; -X → reverse; +Y → forward, turn left.
        assert_eq!(drive_along(0, Vec2::new(1.0, 0.0), 0.2), (1.0, 0.0));
        assert_eq!(drive_along(0, Vec2::new(-1.0, 0.0), 0.2), (-1.0, 0.0));
        assert_eq!(drive_along(0, Vec2::new(0.0, 1.0), 0.2), (1.0, 1.0));
        assert_eq!(drive_along(QUARTER_TURN, Vec2::ZERO, 0.2), (0.0, 0.0));
    }

    #[test]
    fn stall_triggers_after_10_stuck_ticks_and_reverses_for_20() {
        let p = StallParams::default();
        let mut s = Stall::default();
        // Not asking to move: never a stall.
        for _ in 0..50 {
            assert_eq!(s.update(&p, Vec2::ZERO), (false, false));
            s.record(0.0);
        }
        let mut active = Vec::new();
        for _ in 0..40 {
            let (on, _) = s.update(&p, Vec2::ZERO);
            active.push(on);
            s.record(if on { -1.0 } else { 1.0 });
        }
        // Ticks 0..9 build the count (the first has last_throttle 0 → no count).
        let first = active.iter().position(|&a| a).unwrap();
        assert_eq!(first, 10);
        assert!(active[first..first + 20].iter().all(|&a| a));
        assert!(!active[first + 20]);
        // Moving fast resets the count.
        let mut s = Stall::default();
        s.record(1.0);
        for _ in 0..100 {
            assert!(!s.update(&p, Vec2::new(2.0, 0.0)).0);
            s.record(1.0);
        }
    }
}
