//! Tracks as data (racing.md "Track"): a closed centreline and one width, and the
//! geometry derived from them: mitred walls, checkpoint gates, the start grid and the
//! segment maths (circle-vs-segment, rays, gate crossings) that stays in this crate
//! until another game needs it (game-system.md §3).

use engine::angle::Heading;
use engine::Vec2;
use serde::{Deserialize, Serialize};

/// A track as stored in the config (and so covered by `setup_hash`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Track {
    /// Display name, e.g. `"Ring"`.
    pub name: String,
    /// Canvas size the track is drawn on (u); positions in the flat view are scaled by it.
    pub size: [f32; 2],
    /// The closed centreline, counter-clockwise (Y-up), one gate per point. Point 0 is
    /// the start/finish line.
    pub centreline: Vec<[f32; 2]>,
    /// Track width (u); the walls are the centreline offset by half of it on each side.
    pub width: f32,
    /// Grid slots behind the line, slot 0 first.
    pub grid: Vec<[f32; 2]>,
    /// Heading of every car on the grid (BAU).
    pub grid_heading: Heading,
}

impl Track {
    /// The first track, "Ring" (racing.md table): 9 points on the 800 × 600 canvas,
    /// width 120, gate 0 at (400, 100), four grid slots facing +X.
    pub fn ring() -> Self {
        Self {
            name: "Ring".to_string(),
            size: [800.0, 600.0],
            centreline: vec![
                [400.0, 100.0],
                [550.0, 100.0],
                [700.0, 250.0],
                [700.0, 350.0],
                [550.0, 500.0],
                [250.0, 500.0],
                [100.0, 350.0],
                [100.0, 250.0],
                [250.0, 100.0],
            ],
            width: 120.0,
            grid: vec![[370.0, 120.0], [370.0, 80.0], [340.0, 120.0], [340.0, 80.0]],
            grid_heading: 0,
        }
    }
}

/// A straight wall segment from `a` to `b`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    /// Start point.
    pub a: Vec2,
    /// End point.
    pub b: Vec2,
}

impl Segment {
    /// The point of the segment closest to `p`.
    #[inline]
    pub fn closest_point(&self, p: Vec2) -> Vec2 {
        let ab = self.b - self.a;
        let len_sq = ab.length_squared();
        if len_sq == 0.0 {
            return self.a;
        }
        let t = ((p - self.a).dot(ab) / len_sq).clamp(0.0, 1.0);
        self.a + ab * t
    }

    /// Distance along the ray `origin + t * dir` (unit `dir`) to this segment, if the
    /// ray hits it at `t >= 0`.
    #[inline]
    pub fn ray_hit(&self, origin: Vec2, dir: Vec2) -> Option<f32> {
        let e = self.b - self.a;
        let denom = dir.perp_dot(e);
        if denom == 0.0 {
            return None;
        }
        let w = self.a - origin;
        let t = w.perp_dot(e) / denom;
        let s = w.perp_dot(dir) / denom;
        (t >= 0.0 && (0.0..=1.0).contains(&s)).then_some(t)
    }
}

/// A checkpoint gate: from the inner wall corner to the outer wall corner at one
/// centreline point. `normal` is the direction of travel through it (unit): the
/// bisector of the incoming and outgoing centreline directions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gate {
    /// Inner wall corner.
    pub inner: Vec2,
    /// Outer wall corner.
    pub outer: Vec2,
    /// The centreline point (the gate's midpoint for a mitred corner is not exactly
    /// this point, see [`Gate::mid`]).
    pub point: Vec2,
    /// Forward direction through the gate (unit).
    pub normal: Vec2,
}

impl Gate {
    /// Midpoint of the gate line.
    #[inline]
    pub fn mid(&self) -> Vec2 {
        (self.inner + self.outer) * 0.5
    }

    /// Signed side of `p`: < 0 behind the gate line, >= 0 on or past it.
    #[inline]
    pub fn side(&self, p: Vec2) -> f32 {
        (p - self.inner).dot(self.normal)
    }

    /// Whether the move `from → to` crosses this gate forwards: `from` strictly behind
    /// the line, `to` on or past it, and the crossing point between the two corners.
    pub fn crossed_forward(&self, from: Vec2, to: Vec2) -> bool {
        let s0 = self.side(from);
        let s1 = self.side(to);
        if !(s0 < 0.0 && s1 >= 0.0) {
            return false;
        }
        // Crossing point on the gate line, as a fraction along inner → outer.
        let t = s0 / (s0 - s1);
        let x = from + (to - from) * t;
        let g = self.outer - self.inner;
        let u = (x - self.inner).dot(g) / g.length_squared();
        (0.0..=1.0).contains(&u)
    }
}

/// Geometry derived from a [`Track`]: built once (in `init`, and by policies) and
/// never on a tick.
#[derive(Clone, Debug, PartialEq)]
pub struct TrackGeom {
    /// Centreline points.
    pub points: Vec<Vec2>,
    /// One gate per centreline point.
    pub gates: Vec<Gate>,
    /// Wall segments: the inner loop (9 for Ring) then the outer loop.
    pub walls: Vec<Segment>,
    /// Direction of centreline segment `i` (point `i` → point `i + 1`), unit.
    pub seg_dir: Vec<Vec2>,
    /// Length of centreline segment `i`.
    pub seg_len: Vec<f32>,
    /// Arc length from point 0 to point `i` along the centreline (`cum[n]` is the lap).
    pub cum: Vec<f32>,
    /// Half the width.
    pub half_width: f32,
    /// Canvas size.
    pub size: Vec2,
}

impl TrackGeom {
    /// Derive walls, gates and arc lengths. Assumes a valid track (see
    /// [`crate::RacingConfig::validate`]).
    pub fn new(track: &Track) -> Self {
        let n = track.centreline.len();
        let points: Vec<Vec2> = track
            .centreline
            .iter()
            .map(|p| Vec2::new(p[0], p[1]))
            .collect();
        let mut seg_dir = Vec::with_capacity(n);
        let mut seg_len = Vec::with_capacity(n);
        let mut cum = Vec::with_capacity(n + 1);
        cum.push(0.0);
        for i in 0..n {
            let d = points[(i + 1) % n] - points[i];
            let len = d.length();
            seg_dir.push(d / len);
            seg_len.push(len);
            cum.push(cum[i] + len);
        }
        let h = track.width * 0.5;
        let mut inner = Vec::with_capacity(n);
        let mut outer = Vec::with_capacity(n);
        let mut gates = Vec::with_capacity(n);
        for i in 0..n {
            let d0 = seg_dir[(i + n - 1) % n];
            let d1 = seg_dir[i];
            // Left normals point to the infield for a counter-clockwise centreline.
            let n0 = d0.perp();
            let n1 = d1.perp();
            // Mitre: the offset lines meet at p ± h (n0 + n1) / (1 + n0·n1).
            let m = (n0 + n1) * (h / (1.0 + n0.dot(n1)));
            let p = points[i];
            inner.push(p + m);
            outer.push(p - m);
            let normal = (d0 + d1).normalize();
            gates.push(Gate {
                inner: p + m,
                outer: p - m,
                point: p,
                normal,
            });
        }
        let mut walls = Vec::with_capacity(2 * n);
        for i in 0..n {
            walls.push(Segment {
                a: inner[i],
                b: inner[(i + 1) % n],
            });
        }
        for i in 0..n {
            walls.push(Segment {
                a: outer[i],
                b: outer[(i + 1) % n],
            });
        }
        Self {
            points,
            gates,
            walls,
            seg_dir,
            seg_len,
            cum,
            half_width: h,
            size: Vec2::new(track.size[0], track.size[1]),
        }
    }

    /// Number of gates (= centreline points).
    #[inline]
    pub fn gate_count(&self) -> usize {
        self.points.len()
    }

    /// Lap length along the centreline.
    #[inline]
    pub fn lap_length(&self) -> f32 {
        self.cum[self.points.len()]
    }

    /// Fraction (0–1, clamped) of the way along the centreline segment that ends at
    /// gate `next`, for a car at `p`: the projection onto that segment.
    #[inline]
    pub fn fraction_to(&self, next: usize, p: Vec2) -> f32 {
        let n = self.points.len();
        let i = (next + n - 1) % n;
        ((p - self.points[i]).dot(self.seg_dir[i]) / self.seg_len[i]).clamp(0.0, 1.0)
    }

    /// The point at arc length `s` along the centreline (wraps around the lap).
    pub fn point_at(&self, s: f32) -> Vec2 {
        let lap = self.lap_length();
        let mut s = s % lap;
        if s < 0.0 {
            s += lap;
        }
        let n = self.points.len();
        let mut i = 0;
        while i + 1 < n && self.cum[i + 1] <= s {
            i += 1;
        }
        self.points[i] + self.seg_dir[i] * (s - self.cum[i])
    }

    /// Distance from `p` to the nearest wall, along the ray `dir` (unit), capped at
    /// `max`.
    pub fn ray(&self, p: Vec2, dir: Vec2, max: f32) -> f32 {
        let mut best = max;
        for w in &self.walls {
            if let Some(t) = w.ray_hit(p, dir) {
                if t < best {
                    best = t;
                }
            }
        }
        best
    }

    /// Smallest distance from `p` to any wall segment.
    pub fn wall_distance(&self, p: Vec2) -> f32 {
        let mut best = f32::INFINITY;
        for w in &self.walls {
            let d = (p - w.closest_point(p)).length();
            if d < best {
                best = d;
            }
        }
        best
    }

    /// Whether `p` lies on the track surface (between the inner and outer wall loops).
    pub fn on_track(&self, p: Vec2) -> bool {
        let n = self.points.len();
        let inside = |off: usize| {
            // Even-odd test against the loop of walls[off..off + n].
            let mut c = false;
            for w in &self.walls[off..off + n] {
                let (a, b) = (w.a, w.b);
                if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x
                {
                    c = !c;
                }
            }
            c
        };
        inside(n) && !inside(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-3
    }

    #[test]
    fn ring_matches_the_spec_table() {
        let g = TrackGeom::new(&Track::ring());
        assert_eq!(g.gate_count(), 9);
        assert!((g.lap_length() - 1648.528).abs() < 0.01, "{}", g.lap_length());
        // Gate 0: from the infield edge (y 160) to the outer wall (y 40) at x 400.
        assert!(close(g.gates[0].inner, Vec2::new(400.0, 160.0)));
        assert!(close(g.gates[0].outer, Vec2::new(400.0, 40.0)));
        assert!(close(g.gates[0].normal, Vec2::X));
        // Outer walls at y 40/560 and x 40/760; infield edges at 160/440 and 160/640.
        let ys: Vec<f32> = g.walls.iter().map(|w| w.a.y).collect();
        let xs: Vec<f32> = g.walls.iter().map(|w| w.a.x).collect();
        for v in [40.0, 560.0, 160.0, 440.0] {
            assert!(ys.iter().any(|y| (y - v).abs() < 1e-3), "y {v}");
        }
        for v in [40.0, 760.0, 160.0, 640.0] {
            assert!(xs.iter().any(|x| (x - v).abs() < 1e-3), "x {v}");
        }
        for s in Track::ring().grid {
            let p = Vec2::new(s[0], s[1]);
            assert!(g.on_track(p) && g.wall_distance(p) > 12.0, "{p}");
        }
        assert!(!g.on_track(Vec2::new(400.0, 300.0)), "infield is solid");
        assert!(!g.on_track(Vec2::new(20.0, 20.0)));
    }

    #[test]
    fn gate_crossing_needs_forwards_and_between_the_corners() {
        let g = TrackGeom::new(&Track::ring()).gates[0];
        assert!(g.crossed_forward(Vec2::new(398.0, 100.0), Vec2::new(401.0, 100.0)));
        assert!(!g.crossed_forward(Vec2::new(401.0, 100.0), Vec2::new(398.0, 100.0)));
        assert!(!g.crossed_forward(Vec2::new(398.0, 200.0), Vec2::new(401.0, 200.0)));
        assert!(g.crossed_forward(Vec2::new(399.0, 100.0), Vec2::new(400.0, 100.0)));
        assert!(!g.crossed_forward(Vec2::new(400.0, 100.0), Vec2::new(402.0, 100.0)));
    }

    #[test]
    fn rays_and_points() {
        let g = TrackGeom::new(&Track::ring());
        let p = Vec2::new(400.0, 100.0);
        assert!((g.ray(p, Vec2::Y, 300.0) - 60.0).abs() < 1e-3);
        assert!((g.ray(p, -Vec2::Y, 300.0) - 60.0).abs() < 1e-3);
        // Straight ahead from the line: the outer diagonal at x = 400 + 175 + 60·tan 45°.
        assert!((g.ray(p, Vec2::X, 300.0) - 234.853).abs() < 1e-2);
        assert_eq!(g.ray(p, Vec2::X, 200.0), 200.0);
        assert!(close(g.point_at(0.0), p));
        assert!(close(g.point_at(150.0), Vec2::new(550.0, 100.0)));
        assert!(close(g.point_at(g.lap_length() + 10.0), Vec2::new(410.0, 100.0)));
        assert!((g.fraction_to(1, Vec2::new(475.0, 130.0)) - 0.5).abs() < 1e-6);
    }
}
