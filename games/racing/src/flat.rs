//! The 43-float view (`Flat`, racing.md "Flat encoding") and its index table.
//! Every value is scaled and then clamped to [−1, 1]. "Car frame" is (forward, left)
//! relative to the car's heading.

use crate::config::RacingConfig;
use crate::obs::{Observation, OPPONENT_SLOTS, RAY_DEGREES, RAY_RANGE};
use crate::rules::{RaceAction, RaceState, RacingRules};
use engine::angle::dir;
use engine::generic::{Flat, Rules};

/// Length of one encoded observation.
pub const OBS_LEN: usize = 43;
/// Length of one encoded action: `[throttle, steer]`.
pub const ACTION_LEN: usize = 2;

/// 0–1: speed forward, sideways ÷ the car's top speed.
pub const SPEED: usize = 0;
/// 2–3: heading cos, sin.
pub const HEADING: usize = 2;
/// 4–5: position, 2·p/size − 1.
pub const POSITION: usize = 4;
/// 6–14: wall distance along the 9 rays ÷ [`RAY_RANGE`].
pub const RAYS: usize = 6;
/// 15–16: next gate's midpoint, car frame ÷ [`DIST_SCALE`].
pub const NEXT_GATE: usize = 15;
/// 17–18: track direction at the next gate relative to the heading, cos and sin.
pub const NEXT_DIR: usize = 17;
/// 19–20: the gate after next, car frame ÷ [`DIST_SCALE`].
pub const GATE_AFTER: usize = 19;
/// 21: laps done ÷ laps.
pub const LAPS: usize = 21;
/// 22: race progress ÷ (laps × gates).
pub const PROGRESS: usize = 22;
/// 23: (place − 1) ÷ (cars − 1); 0 racing alone.
pub const PLACE: usize = 23;
/// 24–41: opponent slots, nearest first.
pub const OPPONENTS: usize = 24;
/// Floats per opponent slot: present, position (2, car frame), relative velocity (2,
/// car frame), ahead in the race.
pub const OPPONENT_SLOT: usize = 6;
/// 42: tick ÷ tick cap.
pub const TICK: usize = 42;
/// Distance scale for gate and opponent positions (u).
pub const DIST_SCALE: f32 = 300.0;

const _: () = assert!(OPPONENTS + OPPONENT_SLOTS * OPPONENT_SLOT == TICK);
const _: () = assert!(RAYS + RAY_DEGREES.len() == NEXT_GATE);
const _: () = assert!(TICK + 1 == OBS_LEN);

#[inline]
fn c1(x: f32) -> f32 {
    x.clamp(-1.0, 1.0)
}

/// Write an observation into `out` (at least [`OBS_LEN`] long).
pub fn encode(config: &RacingConfig, state: &RaceState, o: &Observation, out: &mut [f32]) {
    let top = o.params.top_speed;
    let g = &state.geom;
    let n = g.gate_count();
    out[SPEED] = c1(o.speed / top);
    out[SPEED + 1] = c1(o.slide / top);
    let d = dir(o.heading);
    out[HEADING] = c1(d.x);
    out[HEADING + 1] = c1(d.y);
    out[POSITION] = c1(2.0 * o.pos.x / g.size.x - 1.0);
    out[POSITION + 1] = c1(2.0 * o.pos.y / g.size.y - 1.0);
    for (k, r) in o.rays.iter().enumerate() {
        out[RAYS + k] = c1(r / RAY_RANGE);
    }
    let next = &g.gates[o.next_gate];
    let after = &g.gates[(o.next_gate + 1) % n];
    let rel = o.to_car_frame(next.mid() - o.pos) / DIST_SCALE;
    out[NEXT_GATE] = c1(rel.x);
    out[NEXT_GATE + 1] = c1(rel.y);
    out[NEXT_DIR] = c1(next.normal.dot(d));
    out[NEXT_DIR + 1] = c1(d.perp_dot(next.normal));
    let rel = o.to_car_frame(after.mid() - o.pos) / DIST_SCALE;
    out[GATE_AFTER] = c1(rel.x);
    out[GATE_AFTER + 1] = c1(rel.y);
    out[LAPS] = c1(o.laps as f32 / o.laps_total as f32);
    out[PROGRESS] = c1(o.progress / (o.laps_total as f32 * n as f32));
    out[PLACE] = if o.cars > 1 {
        c1((o.place - 1) as f32 / (o.cars - 1) as f32)
    } else {
        0.0
    };
    for (s, opp) in o.opponents.iter().enumerate() {
        let b = OPPONENTS + s * OPPONENT_SLOT;
        match opp {
            Some(p) => {
                let rp = o.to_car_frame(p.pos - o.pos) / DIST_SCALE;
                let rv = o.to_car_frame(p.vel - o.vel) / (2.0 * top);
                out[b] = 1.0;
                out[b + 1] = c1(rp.x);
                out[b + 2] = c1(rp.y);
                out[b + 3] = c1(rv.x);
                out[b + 4] = c1(rv.y);
                out[b + 5] = if p.ahead { 1.0 } else { 0.0 };
            }
            None => out[b..b + OPPONENT_SLOT].fill(0.0),
        }
    }
    out[TICK] = c1(o.tick as f32 / config.max_ticks as f32);
}

impl Flat for RacingRules {
    const OBS_LEN: usize = OBS_LEN;
    const ACTION_LEN: usize = ACTION_LEN;

    /// # Panics
    /// If `out.len()` is not [`OBS_LEN`].
    fn encode_obs(
        config: &RacingConfig,
        state: &RaceState,
        agent: usize,
        tick: u32,
        out: &mut [f32],
    ) {
        assert_eq!(out.len(), OBS_LEN, "observation buffer length");
        let o = Observation::new(config, state, agent, tick);
        encode(config, state, &o, out);
    }

    /// `[throttle, steer]`, clamped to [−1, 1] with NaN as 0 (the same as `sanitize`).
    ///
    /// # Panics
    /// If `input.len()` is not [`ACTION_LEN`].
    fn decode_action(input: &[f32]) -> RaceAction {
        assert_eq!(input.len(), ACTION_LEN, "action buffer length");
        RacingRules::sanitize(RaceAction {
            throttle: input[0],
            steer: input[1],
        })
    }
}
