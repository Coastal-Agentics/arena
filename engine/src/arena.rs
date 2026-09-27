//! Arena geometry and collision primitives. All tests use squared distances (no `sqrt`).

use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Axis-aligned rectangle given by its min and max corners.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect {
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
}

/// Bounded rectangular arena spanning `(0,0)` to `size`, with optional obstacles.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Arena {
    pub size: Vec2,
    #[serde(default)]
    pub obstacles: Vec<Rect>,
}

impl Arena {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            size: Vec2::new(width, height),
            obstacles: Vec::new(),
        }
    }

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
}
