//! Controller-side battery model. Not part of the MuJoCo file.
//!
//! Port of `Battery` in `tbd_control.py` / `tbd_control.js`. Hover power scales
//! with thrust to the 1.5, from momentum theory. Estimated, not measured.

use crate::params::{AVIONICS_W, G, G_PER_W, MASS, PACK_WH, USABLE, WHEEL_W};

/// Pack state. `driving` on [`Battery::step`] is the reference's flag: `1` while
/// the wheels are commanded, `0` otherwise (not a speed fraction).
#[derive(Clone, Debug)]
pub struct Battery {
    wh_used: f64,
    last_w: f64,
    /// Hover thrust of one duct, N.
    th: f64,
    /// Hover power of one duct, W.
    ph: f64,
}

impl Default for Battery {
    fn default() -> Self {
        Self::new()
    }
}

impl Battery {
    /// Full pack.
    pub fn new() -> Self {
        let th = MASS * G / 4.0;
        let ph = (th / G * 1000.0) / G_PER_W;
        Self {
            wh_used: 0.0,
            last_w: AVIONICS_W,
            th,
            ph,
        }
    }

    /// Clear the coulomb count. Power-at-hover constants stay.
    pub fn reset(&mut self) {
        self.wh_used = 0.0;
        self.last_w = AVIONICS_W;
    }

    /// Instantaneous electrical power, watts.
    pub fn power(&self, thrusts_n: &[f64], driving: f64) -> f64 {
        let mut p = 0.0;
        for &t in thrusts_n {
            let ratio = t.max(0.0) / self.th;
            p += self.ph * ratio.powf(1.5);
        }
        p + AVIONICS_W + driving * 4.0 * WHEEL_W
    }

    /// Integrate power over `dt` seconds. Returns the state of charge, percent.
    pub fn step(&mut self, dt: f64, thrusts_n: &[f64], driving: f64) -> f64 {
        self.last_w = self.power(thrusts_n, driving);
        self.wh_used += self.last_w * dt / 3600.0;
        self.pct()
    }

    /// State of charge, percent, floored at 0.
    pub fn pct(&self) -> f64 {
        (100.0 * (1.0 - self.wh_used / (PACK_WH * USABLE))).max(0.0)
    }

    /// Energy taken from the pack, Wh.
    pub fn wh_used(&self) -> f64 {
        self.wh_used
    }

    /// Power from the last [`Battery::step`], or avionics-only before the first step.
    pub fn last_w(&self) -> f64 {
        self.last_w
    }

    /// Estimated hover endurance, minutes, at the 2.3 gf/W figure.
    pub fn hover_minutes(&self) -> f64 {
        PACK_WH * USABLE / (4.0 * self.ph + AVIONICS_W) * 60.0
    }
}
