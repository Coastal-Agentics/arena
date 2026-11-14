//! Seeded wind disturbance.
//!
//! The generator is `ChaCha8Rng::seed_from_u64`, the same construction as
//! `engine::MatchRng`. `MatchRng` can only be built inside the engine crate, so
//! this crate owns its own stream and draws with `RngCore::next_u64`. A run is
//! reproducible from the seed and the step index: every tick consumes four
//! `u64`s when wind is enabled, and none when it is not. The draw count does
//! not depend on the observation.
//!
//! The force is an estimated gust load, `½ ρ (Cd·A) |w| w`, in the world frame,
//! applied by the host at the body origin. It is not a substitute for the fluid
//! drag already in `tbd.xml`, and the flight controller does not cancel it.

use crate::params::{AIR_DENSITY, WIND_DRAG_AREA};
use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Mean wind is at most this, m/s.
const MEAN_MAX_MPS: f64 = 0.8;
/// Gust target is at most this, m/s, scaled by one seed draw in `(0, 1]`.
const GUST_MAX_MPS: f64 = 2.5;
/// Mean time between gust retargets, seconds.
const GUST_HOLD_S: f64 = 0.75;
/// Gust low-pass time constant, seconds.
const GUST_TAU_S: f64 = 0.35;

/// Wind state for one run.
#[derive(Clone, Debug)]
pub struct Wind {
    rng: ChaCha8Rng,
    enabled: bool,
    mean: [f64; 3],
    gust: [f64; 3],
    gust_target: [f64; 3],
    amp: f64,
    velocity: [f64; 3],
    force: [f64; 3],
}

impl Wind {
    /// Seed the stream. When `enabled` is false the stream is still seeded (so a
    /// later inspection is stable) but [`Wind::sample`] draws nothing and the
    /// force stays zero.
    pub fn new(seed: u64, enabled: bool) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let (mean, amp) = if enabled {
            let speed = unit(&mut rng) * MEAN_MAX_MPS;
            let dir = unit(&mut rng) * std::f64::consts::TAU;
            let amp = unit(&mut rng) * GUST_MAX_MPS;
            ([dir.cos() * speed, dir.sin() * speed, 0.0], amp)
        } else {
            ([0.0; 3], 0.0)
        };
        Self {
            rng,
            enabled,
            mean,
            gust: [0.0; 3],
            gust_target: [0.0; 3],
            amp,
            velocity: [0.0; 3],
            force: [0.0; 3],
        }
    }

    /// Advance one tick. `dt` is seconds.
    pub fn sample(&mut self, dt: f64) {
        if !self.enabled {
            self.velocity = [0.0; 3];
            self.force = [0.0; 3];
            return;
        }
        let retarget = unit(&mut self.rng);
        let gx = unit(&mut self.rng) * 2.0 - 1.0;
        let gy = unit(&mut self.rng) * 2.0 - 1.0;
        let gz = unit(&mut self.rng) * 2.0 - 1.0;
        let p = (dt / GUST_HOLD_S).clamp(0.0, 1.0);
        if retarget < p {
            self.gust_target = [gx * self.amp, gy * self.amp, gz * self.amp * 0.2];
        }
        let alpha = 1.0 - (-dt / GUST_TAU_S).exp();
        for i in 0..3 {
            self.gust[i] += alpha * (self.gust_target[i] - self.gust[i]);
            self.velocity[i] = self.mean[i] + self.gust[i];
        }
        let speed = norm3(self.velocity);
        let scale = 0.5 * AIR_DENSITY * WIND_DRAG_AREA * speed;
        for i in 0..3 {
            self.force[i] = scale * self.velocity[i];
        }
    }

    /// Air velocity in the world frame, m/s. Zero until the first sample.
    pub fn velocity(&self) -> [f64; 3] {
        self.velocity
    }

    /// Estimated world-frame force, N. The host writes this into `xfrc_applied`
    /// on body `tbd` (three forces, zero torque) before `mj_step`.
    pub fn force(&self) -> [f64; 3] {
        self.force
    }
}

/// `[0, 1)` from the next `u64`, using the top 53 bits.
fn unit(rng: &mut ChaCha8Rng) -> f64 {
    let bits = rng.next_u64() >> 11;
    (bits as f64) * (1.0 / ((1u64 << 53) as f64))
}

fn norm3(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}
