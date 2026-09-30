//! Arena geometry and collision primitives. Overlap tests use squared distances; the
//! swept segment tests use one IEEE-754 `sqrt` (correctly rounded on every target, so
//! still bit-deterministic) to find the entry time.
//!
//! Coordinates are world units in a Y-up frame: the arena spans `(0, 0)` (bottom-left)
//! to [`Arena::size`] (top-right).

use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Axis-aligned rectangle given by its min and max corners.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    /// Bottom-left corner (smallest x and y).
    pub min: Vec2,
    /// Top-right corner (largest x and y).
    pub max: Vec2,
}

impl Rect {
    /// Rectangle from its `min` and `max` corners (not reordered or validated).
    ///
    /// ```
    /// use engine::{Rect, Vec2};
    /// let r = Rect::new(Vec2::new(10.0, 10.0), Vec2::new(20.0, 20.0));
    /// assert!(r.contains(Vec2::new(10.0, 15.0))); // edges count as inside
    /// assert!(!r.contains(Vec2::new(21.0, 15.0)));
    /// ```
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    /// True if point `p` is inside (or on the edge of) the rectangle.
    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    /// Closest point on/in the rectangle to `p`.
    pub fn closest_point(&self, p: Vec2) -> Vec2 {
        p.clamp(self.min, self.max)
    }

    /// True if a circle at `c` with radius `r` overlaps this rectangle (touching is not overlap).
    pub fn overlaps_circle(&self, c: Vec2, r: f32) -> bool {
        (c - self.closest_point(c)).length_squared() < r * r
    }

    /// Earliest `t` in `[0, 1]` at which the segment `p0 -> p0 + d` touches this
    /// (closed) rectangle, or `None`. Slab test; `Some(0.0)` if `p0` is inside.
    pub fn segment_entry(&self, p0: Vec2, d: Vec2) -> Option<f32> {
        let mut t_min = 0.0f32;
        let mut t_max = 1.0f32;
        for axis in 0..2 {
            let (o, v, lo, hi) = (p0[axis], d[axis], self.min[axis], self.max[axis]);
            if v == 0.0 {
                if o < lo || o > hi {
                    return None;
                }
            } else {
                let (mut t1, mut t2) = ((lo - o) / v, (hi - o) / v);
                if t1 > t2 {
                    std::mem::swap(&mut t1, &mut t2);
                }
                t_min = t_min.max(t1);
                t_max = t_max.min(t2);
                if t_min > t_max {
                    return None;
                }
            }
        }
        Some(t_min)
    }
}

/// Bounded rectangular arena spanning `(0,0)` to `size`, with optional obstacles.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Arena {
    /// Width (`x`) and height (`y`) in world units.
    pub size: Vec2,
    /// Solid axis-aligned obstacles. Tanks cannot enter them; projectiles die on them.
    /// Defaults to empty when missing from JSON.
    #[serde(default)]
    pub obstacles: Vec<Rect>,
}

impl Arena {
    /// Empty arena of the given width and height.
    ///
    /// ```
    /// use engine::{Arena, Rect, Vec2};
    /// let a = Arena::new(100.0, 50.0)
    ///     .with_obstacle(Rect::new(Vec2::new(40.0, 0.0), Vec2::new(60.0, 20.0)));
    /// assert_eq!(a.obstacles.len(), 1);
    /// assert!(a.circle_in_bounds(Vec2::new(20.0, 25.0), 10.0));
    /// assert_eq!(a.clamp_circle(Vec2::new(-5.0, 60.0), 10.0), Vec2::new(10.0, 40.0));
    /// ```
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            size: Vec2::new(width, height),
            obstacles: Vec::new(),
        }
    }

    /// Builder: add an obstacle and return the arena.
    pub fn with_obstacle(mut self, r: Rect) -> Self {
        self.obstacles.push(r);
        self
    }

    /// True if a circle is fully inside the arena bounds.
    pub fn circle_in_bounds(&self, c: Vec2, r: f32) -> bool {
        c.x >= r && c.y >= r && c.x <= self.size.x - r && c.y <= self.size.y - r
    }

    /// Clamp a circle centre so the circle stays inside the arena.
    pub fn clamp_circle(&self, c: Vec2, r: f32) -> Vec2 {
        Vec2::new(
            c.x.clamp(r, (self.size.x - r).max(r)),
            c.y.clamp(r, (self.size.y - r).max(r)),
        )
    }

    /// True if a circle overlaps any obstacle.
    pub fn circle_hits_obstacle(&self, c: Vec2, r: f32) -> bool {
        self.obstacles.iter().any(|o| o.overlaps_circle(c, r))
    }

    /// True if a point is outside the arena or inside an obstacle.
    pub fn point_blocked(&self, p: Vec2) -> bool {
        p.x < 0.0
            || p.y < 0.0
            || p.x > self.size.x
            || p.y > self.size.y
            || self.obstacles.iter().any(|o| o.contains(p))
    }

    /// Line-of-sight test: true if the segment from `a` to `b` stays inside the arena
    /// and touches no obstacle. The same geometry that stops a projectile
    /// ([`Arena::segment_blocked_at`]): obstacles are closed, so a segment that only
    /// grazes an obstacle's edge or corner is blocked, while the arena boundary itself
    /// counts as inside, so a segment running along a wall is clear. Only arena geometry
    /// is considered (no tanks or other entities). A zero-length segment is clear unless
    /// its point is blocked.
    ///
    /// ```
    /// use engine::{Arena, Rect, Vec2};
    /// let a = Arena::new(100.0, 100.0)
    ///     .with_obstacle(Rect::new(Vec2::new(40.0, 40.0), Vec2::new(60.0, 60.0)));
    /// assert!(!a.segment_clear(Vec2::new(10.0, 50.0), Vec2::new(90.0, 50.0))); // through it
    /// assert!(a.segment_clear(Vec2::new(10.0, 10.0), Vec2::new(90.0, 10.0))); // below it
    /// ```
    pub fn segment_clear(&self, a: Vec2, b: Vec2) -> bool {
        self.segment_blocked_at(a, b - a).is_none()
    }

    /// Earliest `t` in `[0, 1]` at which the segment `p0 -> p0 + d` leaves the arena
    /// or touches an obstacle (the swept form of [`Arena::point_blocked`]), or `None`.
    pub fn segment_blocked_at(&self, p0: Vec2, d: Vec2) -> Option<f32> {
        let mut best: Option<f32> = None;
        let mut take = |t: f32| {
            let t = t.max(0.0);
            if best.is_none_or(|b| t < b) {
                best = Some(t);
            }
        };
        let p1 = p0 + d;
        for axis in 0..2 {
            // `d[axis]` is non-zero whenever `p1` is outside but `p0` is not; if `p0` is
            // already outside, the division yields <= 0 (or NaN, handled below).
            if p1[axis] < 0.0 || p0[axis] < 0.0 {
                take(if d[axis] < 0.0 {
                    -p0[axis] / d[axis]
                } else {
                    0.0
                });
            }
            let hi = self.size[axis];
            if p1[axis] > hi || p0[axis] > hi {
                take(if d[axis] > 0.0 {
                    (hi - p0[axis]) / d[axis]
                } else {
                    0.0
                });
            }
        }
        for o in &self.obstacles {
            if let Some(t) = o.segment_entry(p0, d) {
                take(t);
            }
        }
        best
    }
}

/// Earliest `t` in `[0, 1]` at which the segment `p0 -> p0 + d` enters the open circle
/// at `c` with radius `r` (touching is not a hit, matching [`circles_overlap`]), or
/// `None`. `Some(0.0)` if `p0` is already inside.
pub fn segment_circle_entry(p0: Vec2, d: Vec2, c: Vec2, r: f32) -> Option<f32> {
    let f = p0 - c;
    let cc = f.length_squared() - r * r;
    if cc < 0.0 {
        return Some(0.0);
    }
    let a = d.length_squared();
    let b = f.dot(d);
    if a == 0.0 || b >= 0.0 {
        return None; // not moving, or moving away from the centre
    }
    let disc = b * b - a * cc;
    if disc <= 0.0 {
        return None; // misses or only grazes
    }
    let t = (-b - disc.sqrt()) / a;
    (t < 1.0).then_some(t.max(0.0))
}

/// True if two circles overlap (touching is not overlap).
pub fn circles_overlap(a: Vec2, ra: f32, b: Vec2, rb: f32) -> bool {
    let r = ra + rb;
    (a - b).length_squared() < r * r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circle_rect_overlap() {
        let r = Rect::new(Vec2::new(10.0, 10.0), Vec2::new(20.0, 20.0));
        assert!(r.overlaps_circle(Vec2::new(15.0, 15.0), 1.0)); // inside
        assert!(r.overlaps_circle(Vec2::new(8.0, 15.0), 3.0)); // side
        assert!(!r.overlaps_circle(Vec2::new(5.0, 15.0), 3.0)); // clear
        assert!(!r.overlaps_circle(Vec2::new(7.0, 7.0), 4.0)); // corner: dist ~4.24
        assert!(r.overlaps_circle(Vec2::new(7.0, 7.0), 4.5));
    }

    #[test]
    fn bounds_and_clamp() {
        let a = Arena::new(100.0, 50.0);
        assert!(a.circle_in_bounds(Vec2::new(50.0, 25.0), 10.0));
        assert!(!a.circle_in_bounds(Vec2::new(5.0, 25.0), 10.0));
        assert_eq!(
            a.clamp_circle(Vec2::new(-5.0, 60.0), 10.0),
            Vec2::new(10.0, 40.0)
        );
        assert!(a.point_blocked(Vec2::new(-0.1, 1.0)));
        assert!(!a.point_blocked(Vec2::new(1.0, 1.0)));
    }

    #[test]
    fn circles() {
        assert!(circles_overlap(Vec2::ZERO, 5.0, Vec2::new(9.0, 0.0), 5.0));
        assert!(!circles_overlap(Vec2::ZERO, 5.0, Vec2::new(10.0, 0.0), 5.0));
    }

    #[test]
    fn segment_vs_circle() {
        let c = Vec2::new(50.0, 0.0);
        // Straight through: enters at x = 40 of a 0..100 segment.
        assert_eq!(
            segment_circle_entry(Vec2::ZERO, Vec2::new(100.0, 0.0), c, 10.0),
            Some(0.4)
        );
        // Ends before reaching the circle.
        assert_eq!(
            segment_circle_entry(Vec2::ZERO, Vec2::new(30.0, 0.0), c, 10.0),
            None
        );
        // Passes beside it; grazing (tangent) is not a hit.
        assert_eq!(
            segment_circle_entry(Vec2::new(0.0, 10.0), Vec2::new(100.0, 0.0), c, 10.0),
            None
        );
        assert!(
            segment_circle_entry(Vec2::new(0.0, 9.0), Vec2::new(100.0, 0.0), c, 10.0).is_some()
        );
        // Starting inside hits at t = 0; moving away from outside does not.
        assert_eq!(
            segment_circle_entry(c, Vec2::new(100.0, 0.0), c, 10.0),
            Some(0.0)
        );
        assert_eq!(
            segment_circle_entry(Vec2::new(70.0, 0.0), Vec2::new(100.0, 0.0), c, 10.0),
            None
        );
    }

    #[test]
    fn segment_vs_arena() {
        let a = Arena::new(100.0, 100.0)
            .with_obstacle(Rect::new(Vec2::new(40.0, 40.0), Vec2::new(60.0, 60.0)));
        // Tunnels straight through the obstacle: blocked on entry at x = 40.
        assert_eq!(
            a.segment_blocked_at(Vec2::new(10.0, 50.0), Vec2::new(80.0, 0.0)),
            Some(0.375)
        );
        // Leaves the arena through the right wall at x = 100.
        assert_eq!(
            a.segment_blocked_at(Vec2::new(80.0, 10.0), Vec2::new(40.0, 0.0)),
            Some(0.5)
        );
        // Clear path.
        assert_eq!(
            a.segment_blocked_at(Vec2::new(10.0, 10.0), Vec2::new(20.0, 20.0)),
            None
        );
    }

    #[test]
    fn segment_clear_line_of_sight() {
        let a = Arena::new(100.0, 100.0)
            .with_obstacle(Rect::new(Vec2::new(40.0, 40.0), Vec2::new(60.0, 60.0)))
            // Touches the right wall.
            .with_obstacle(Rect::new(Vec2::new(90.0, 80.0), Vec2::new(100.0, 90.0)));
        let v = Vec2::new;
        // (a, b, clear?) — each case is also checked in the reverse direction.
        let cases = [
            ("through the obstacle", v(10.0, 50.0), v(90.0, 50.0), false),
            ("beside it", v(10.0, 30.0), v(90.0, 30.0), true),
            // y = x + 20 touches only the top-left corner (40, 60): grazing blocks.
            ("grazes a corner", v(20.0, 40.0), v(60.0, 80.0), false),
            ("just misses the corner", v(20.0, 40.5), v(60.0, 80.5), true),
            ("runs along an edge", v(10.0, 60.0), v(90.0, 60.0), false),
            (
                "just above the edge",
                v(10.0, 60.001),
                v(90.0, 60.001),
                true,
            ),
            ("ends at the obstacle", v(10.0, 50.0), v(40.0, 50.0), false),
            ("stops short", v(10.0, 50.0), v(39.99, 50.0), true),
            ("along the bottom wall", v(0.0, 0.0), v(100.0, 0.0), true),
            ("along the left wall", v(0.0, 5.0), v(0.0, 95.0), true),
            (
                "arena diagonal crosses it",
                v(0.0, 0.0),
                v(100.0, 100.0),
                false,
            ),
            ("leaves the arena", v(50.0, 10.0), v(101.0, 10.0), false),
            ("starts outside", v(-1.0, 10.0), v(50.0, 10.0), false),
            (
                "into the wall obstacle",
                v(50.0, 85.0),
                v(95.0, 85.0),
                false,
            ),
            (
                "under the wall obstacle",
                v(50.0, 75.0),
                v(100.0, 75.0),
                true,
            ),
            ("zero length, open", v(10.0, 10.0), v(10.0, 10.0), true),
            ("zero length, inside", v(50.0, 50.0), v(50.0, 50.0), false),
        ];
        for (what, p, q, clear) in cases {
            assert_eq!(a.segment_clear(p, q), clear, "{what}");
            assert_eq!(a.segment_clear(q, p), clear, "{what} (reversed)");
        }
        // Empty arena: everything inside is visible.
        let open = Arena::new(100.0, 100.0);
        assert!(open.segment_clear(v(0.0, 0.0), v(100.0, 100.0)));
    }
}
