//! Quaternion and angle helpers, matching `tbd_control.py` / `tbd_control.js`.

use crate::params::G;

/// Wrap `a` into `(-π, π]`.
///
/// Python's `%` and the JavaScript reference both return a non-negative remainder,
/// so this does too: `(a + π) mod 2π − π`, with a negative remainder folded up.
pub fn wrap(a: f64) -> f64 {
    let tau = std::f64::consts::TAU;
    let mut x = (a + std::f64::consts::PI) % tau;
    if x < 0.0 {
        x += tau;
    }
    x - std::f64::consts::PI
}

/// Roll, pitch, yaw (radians) from a `(w, x, y, z)` quaternion.
///
/// Same ZYX conversion as `quat_to_rpy` / `quatToRpy` in the reference controller.
pub fn quat_to_rpy(q: [f64; 4]) -> [f64; 3] {
    let [w, x, y, z] = q;
    let roll = (2.0 * (w * x + y * z)).atan2(1.0 - 2.0 * (x * x + y * y));
    let pitch = (2.0 * (w * y - z * x)).clamp(-1.0, 1.0).asin();
    let yaw = (2.0 * (w * z + x * y)).atan2(1.0 - 2.0 * (y * y + z * z));
    [roll, pitch, yaw]
}

/// Yaw (radians) of a `(w, x, y, z)` quaternion.
pub fn yaw_of(q: [f64; 4]) -> f64 {
    quat_to_rpy(q)[2]
}

/// Body-to-world rotation, row-major, from a `(w, x, y, z)` quaternion.
pub fn quat_to_rot(q: [f64; 4]) -> [f64; 9] {
    let [w, x, y, z] = q;
    let (xx, yy, zz) = (x * x, y * y, z * z);
    let (xy, xz, yz) = (x * y, x * z, y * z);
    let (wx, wy, wz) = (w * x, w * y, w * z);
    [
        1.0 - 2.0 * (yy + zz),
        2.0 * (xy - wz),
        2.0 * (xz + wy),
        2.0 * (xy + wz),
        1.0 - 2.0 * (xx + zz),
        2.0 * (yz - wx),
        2.0 * (xz - wy),
        2.0 * (yz + wx),
        1.0 - 2.0 * (xx + yy),
    ]
}

/// World-frame horizontal acceleration rotated into the yaw frame, then turned
/// into a desired pitch (forward) and roll (left). Shared by the controller so
/// tests can lock the tilt direction.
pub fn tilt_from_horizontal(ax: f64, ay: f64, yaw: f64) -> (f64, f64) {
    let (cy, sy) = (yaw.cos(), yaw.sin());
    let axb = cy * ax + sy * ay;
    let ayb = -sy * ax + cy * ay;
    let pitch = axb.atan2(G);
    let roll = -ayb.atan2(G);
    (roll, pitch)
}
