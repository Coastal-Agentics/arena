//! Integer headings and a precomputed sin/cos table.
//!
//! Headings are binary angle units (BAU): a `u16` where 65536 = one full turn,
//! 0 = +X, 16384 = +Y (counter-clockwise in a Y-up frame). Turning wraps for free.
//! The trig table is built at compile time from a Taylor series using only
//! `+ - * /`, so the sim never calls platform `sin`/`cos`/`atan2`.

use glam::Vec2;

/// A heading in binary angle units (65536 per full turn).
pub type Heading = u16;

/// One quarter turn in BAU.
pub const QUARTER_TURN: Heading = 16384;
/// Half a turn in BAU.
pub const HALF_TURN: Heading = 32768;

const TABLE_BITS: u32 = 10;
/// Number of entries in the trig table.
pub const TABLE_SIZE: usize = 1 << TABLE_BITS;
const SHIFT: u32 = 16 - TABLE_BITS;

const fn taylor_sin(x: f64) -> f64 {
    // x is in [-pi, pi]; 20 terms is far past f32 precision.
    let mut term = x;
    let mut sum = x;
    let mut n = 1;
    while n < 20 {
        let k = (2 * n) as f64;
        term = -term * x * x / (k * (k + 1.0));
        sum += term;
        n += 1;
    }
    sum
}

const fn build_sin_table() -> [f32; TABLE_SIZE] {
    const PI: f64 = std::f64::consts::PI;
    let mut t = [0.0f32; TABLE_SIZE];
    let mut i = 0;
    while i < TABLE_SIZE {
        let mut x = (i as f64) * 2.0 * PI / (TABLE_SIZE as f64);
        if x > PI {
            x -= 2.0 * PI;
        }
        t[i] = taylor_sin(x) as f32;
        i += 1;
    }
    t
}

static SIN_TABLE: [f32; TABLE_SIZE] = build_sin_table();

/// Table sine of a heading (nearest-lower table entry, ~0.35 degree resolution).
#[inline]
pub fn sin(h: Heading) -> f32 {
    SIN_TABLE[(h >> SHIFT) as usize]
}

/// Table cosine of a heading.
#[inline]
pub fn cos(h: Heading) -> f32 {
    sin(h.wrapping_add(QUARTER_TURN))
}

/// Unit direction vector for a heading.
#[inline]
pub fn dir(h: Heading) -> Vec2 {
    Vec2::new(cos(h), sin(h))
}

/// Which way to turn from `h` to face along `target` (a relative vector):
/// `1` = counter-clockwise, `-1` = clockwise, `0` = already aligned within `tolerance`
/// (tolerance is compared against the sine of the angle between them, via a cross product,
/// so no `atan2` is needed). A target directly behind returns `1`.
pub fn turn_toward(h: Heading, target: Vec2, tolerance: f32) -> i8 {
    let d = dir(h);
    let len_sq = target.length_squared();
    if len_sq == 0.0 {
        return 0;
    }
    let cross = d.perp_dot(target);
    let dot = d.dot(target);
    // |sin(angle)| <= tol  <=>  cross^2 <= tol^2 * |target|^2  (d is unit length)
    if dot > 0.0 && cross * cross <= tolerance * tolerance * len_sq {
        0
    } else if cross >= 0.0 {
        1
    } else {
        -1
    }
}

/// Convert degrees (integer) to BAU. Handy for configs and tests.
pub const fn from_degrees(deg: i32) -> Heading {
    ((deg as i64 * 65536 / 360).rem_euclid(65536)) as Heading
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cardinal_values() {
        assert!((sin(0) - 0.0).abs() < 1e-6);
        assert!((cos(0) - 1.0).abs() < 1e-6);
        assert!((sin(QUARTER_TURN) - 1.0).abs() < 1e-6);
        assert!(cos(QUARTER_TURN).abs() < 1e-6);
        assert!((sin(HALF_TURN)).abs() < 1e-6);
        assert!((cos(HALF_TURN) + 1.0).abs() < 1e-6);
        assert!((sin(3 * QUARTER_TURN) + 1.0).abs() < 1e-6);
    }

    #[test]
    fn table_matches_libm_closely() {
        // libm is fine in tests; the sim itself only uses the table.
        for i in 0..TABLE_SIZE {
            let h = (i << SHIFT) as u16;
            let x = (h as f64) * std::f64::consts::TAU / 65536.0;
            assert!((sin(h) as f64 - x.sin()).abs() < 1e-6, "i={i}");
            assert!((cos(h) as f64 - x.cos()).abs() < 1e-6, "i={i}");
        }
    }

    #[test]
    fn turn_toward_signs() {
        assert_eq!(turn_toward(0, Vec2::new(10.0, 0.0), 0.05), 0);
        assert_eq!(turn_toward(0, Vec2::new(10.0, 5.0), 0.05), 1);
        assert_eq!(turn_toward(0, Vec2::new(10.0, -5.0), 0.05), -1);
        assert_eq!(turn_toward(0, Vec2::new(-10.0, 0.0), 0.05), 1);
    }

    #[test]
    fn degrees() {
        assert_eq!(from_degrees(90), QUARTER_TURN);
        assert_eq!(from_degrees(-90), 3 * QUARTER_TURN);
        assert_eq!(from_degrees(360), 0);
    }
}
