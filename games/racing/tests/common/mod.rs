//! Test-only drivers for the R1 rules tests (the real baselines are R2's drivers).
#![allow(dead_code)]

use engine::generic::Policy;
use engine::Vec2;
use racing::{Observation, RaceAction, RacingConfig, RacingRules, TrackGeom};

/// Steers at the centreline a fixed distance ahead and brakes for corners.
pub struct LineDriver {
    pub geom: TrackGeom,
    pub look: f32,
    pub corner_speed: f32,
    pub brake: f32,
    /// Lateral offset of the line (u, positive = left), to vary the traffic.
    pub offset: f32,
}

impl LineDriver {
    pub fn new(config: &RacingConfig) -> Self {
        Self {
            geom: TrackGeom::new(&config.track),
            look: 70.0,
            corner_speed: 150.0,
            brake: config.physics.brake * 0.8,
            offset: 0.0,
        }
    }

    pub fn arc(&self, o: &Observation) -> f32 {
        let n = self.geom.gate_count();
        let i = (o.next_gate + n - 1) % n;
        let f = self.geom.fraction_to(o.next_gate, o.pos);
        // Before the start line the car is on the segment that ends at gate 0.
        self.geom.cum[i] + f * self.geom.seg_len[i]
    }

    fn corner_turn(&self, i: usize) -> f32 {
        let n = self.geom.gate_count();
        let a = self.geom.seg_dir[(i + n - 1) % n];
        let b = self.geom.seg_dir[i];
        a.perp_dot(b).abs()
    }
}

impl Policy<RacingRules> for LineDriver {
    fn act(&mut self, o: &Observation) -> RaceAction {
        let n = self.geom.gate_count();
        let s = self.arc(o);
        let seg = |s: f32| {
            let lap = self.geom.lap_length();
            let s = s.rem_euclid(lap);
            let mut i = 0;
            while i + 1 < n && self.geom.cum[i + 1] <= s {
                i += 1;
            }
            i
        };
        let i = seg(s + self.look);
        let target = self.geom.point_at(s + self.look) + self.geom.seg_dir[i].perp() * self.offset;
        let rel = o.to_car_frame(target - o.pos);
        let len = rel.length().max(1e-3);
        let mut steer = (3.0 * rel.y / len).clamp(-1.0, 1.0);
        if rel.x < 0.0 {
            steer = if rel.y >= 0.0 { 1.0 } else { -1.0 };
        }
        // Distance to the next turning vertex.
        let mut d = f32::INFINITY;
        let lap = self.geom.lap_length();
        for k in 0..n {
            if self.corner_turn(k) < 0.1 {
                continue;
            }
            let dk = (self.geom.cum[k] - s).rem_euclid(lap);
            d = d.min(dk);
        }
        let allowed = (self.corner_speed * self.corner_speed + 2.0 * self.brake * d).sqrt();
        let throttle = if o.speed > allowed { -1.0 } else { 1.0 };
        RaceAction { throttle, steer }
    }
}

/// Seeded pseudo-random controls (xorshift), changed every `hold` ticks.
pub struct Wild {
    pub state: u64,
    pub hold: u32,
    pub t: u32,
    pub a: RaceAction,
}

impl Wild {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1,
            hold: 20,
            t: 0,
            a: RaceAction::default(),
        }
    }
    fn next(&mut self) -> f32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        (self.state >> 40) as f32 / (1u64 << 23) as f32 * 2.0 - 1.0
    }
}

impl Policy<RacingRules> for Wild {
    fn act(&mut self, _o: &Observation) -> RaceAction {
        if self.t % self.hold == 0 {
            self.a = RaceAction {
                throttle: self.next() * 1.2,
                steer: self.next() * 1.2,
            };
        }
        self.t += 1;
        self.a
    }
}

pub fn v(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}
