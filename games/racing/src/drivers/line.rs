//! A closed driving line (a polyline) and the arc-length maths the drivers share.

use crate::track::TrackGeom;
use engine::Vec2;

/// A closed polyline the drivers follow: the centreline, or the Cutter's apex line.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pts: Vec<Vec2>,
    dir: Vec<Vec2>,
    len: Vec<f32>,
    cum: Vec<f32>,
    /// |sin| of the turn at each vertex (0 = straight on).
    turn: Vec<f32>,
}

impl Line {
    /// A line through `pts` (closed, at least 3 distinct points).
    pub fn new(pts: Vec<Vec2>) -> Self {
        let n = pts.len();
        let mut dir = Vec::with_capacity(n);
        let mut len = Vec::with_capacity(n);
        let mut cum = vec![0.0];
        for i in 0..n {
            let d = pts[(i + 1) % n] - pts[i];
            let l = d.length();
            dir.push(d / l);
            len.push(l);
            cum.push(cum[i] + l);
        }
        let turn = (0..n)
            .map(|i| dir[(i + n - 1) % n].perp_dot(dir[i]).abs())
            .collect();
        Self {
            pts,
            dir,
            len,
            cum,
            turn,
        }
    }

    /// The track's centreline.
    pub fn centreline(geom: &TrackGeom) -> Self {
        Self::new(geom.points.clone())
    }

    /// The inside line: through every turning vertex's inner corner, moved `inset`
    /// towards the outer corner (straight-on vertices are skipped).
    pub fn apex(geom: &TrackGeom, inset: f32) -> Self {
        let n = geom.gate_count();
        let pts = (0..n)
            .filter(|&i| {
                let a = geom.seg_dir[(i + n - 1) % n];
                a.perp_dot(geom.seg_dir[i]).abs() > 0.05
            })
            .map(|i| {
                let g = &geom.gates[i];
                g.inner + (g.outer - g.inner).normalize() * inset
            })
            .collect();
        Self::new(pts)
    }

    /// A racing line: wide (`wide` u towards the outer wall) at every turning
    /// vertex, and through an apex `apex` u from the inner wall at the middle of every
    /// segment that joins two turning vertices (the diagonal of a two-bend corner).
    /// Straight-on vertices are skipped.
    pub fn racing(geom: &TrackGeom, wide: f32, apex: f32) -> Self {
        let n = geom.gate_count();
        let turning = |i: usize| {
            let a = geom.seg_dir[(i + n - 1) % n];
            a.perp_dot(geom.seg_dir[i]).abs() > 0.05
        };
        let mut pts = Vec::new();
        for i in 0..n {
            if turning(i) {
                let g = &geom.gates[i];
                pts.push(g.point + (g.outer - g.inner).normalize() * wide);
                let j = (i + 1) % n;
                if turning(j) {
                    let mid = (geom.points[i] + geom.points[j]) * 0.5;
                    let inward = geom.seg_dir[i].perp();
                    pts.push(mid + inward * (geom.half_width - apex));
                }
            }
        }
        Self::new(pts)
    }

    /// Lap length of the line.
    pub fn length(&self) -> f32 {
        self.cum[self.pts.len()]
    }

    fn wrap(&self, s: f32) -> f32 {
        let l = self.length();
        let s = s % l;
        if s < 0.0 {
            s + l
        } else {
            s
        }
    }

    fn segment(&self, s: f32) -> usize {
        let n = self.pts.len();
        let mut i = 0;
        while i + 1 < n && self.cum[i + 1] <= s {
            i += 1;
        }
        i
    }

    /// Arc position of the point of the line closest to `p`, and the signed lateral
    /// offset of `p` from it (positive = left of the line's direction).
    pub fn project(&self, p: Vec2) -> (f32, f32) {
        let mut best = (f32::INFINITY, 0.0, 0.0);
        for i in 0..self.pts.len() {
            let t = (p - self.pts[i]).dot(self.dir[i]).clamp(0.0, self.len[i]);
            let q = self.pts[i] + self.dir[i] * t;
            let d2 = (p - q).length_squared();
            if d2 < best.0 {
                best = (d2, self.cum[i] + t, self.dir[i].perp_dot(p - q));
            }
        }
        (best.1, best.2)
    }

    /// The point at arc position `s` (wraps), moved `offset` to the left.
    pub fn point_at(&self, s: f32, offset: f32) -> Vec2 {
        let s = self.wrap(s);
        let i = self.segment(s);
        self.pts[i] + self.dir[i] * (s - self.cum[i]) + self.dir[i].perp() * offset
    }

    /// Arc distance from `s` to the next turning vertex.
    pub fn to_next_turn(&self, s: f32) -> f32 {
        self.wrap(self.cum[self.next_turn(s)] - s)
    }

    /// Index of the next turning vertex at or after arc position `s`.
    pub fn next_turn(&self, s: f32) -> usize {
        let mut best = (f32::INFINITY, 0);
        for (i, &t) in self.turn.iter().enumerate() {
            if t < 0.05 {
                continue;
            }
            let d = self.wrap(self.cum[i] - s);
            if d < best.0 {
                best = (d, i);
            }
        }
        best.1
    }

    /// The slowest speed the line asks for between `s` and `s + horizon`: at each
    /// turning vertex `corner_speed` (for a 45° turn; sharper turns ask for less),
    /// reachable from here by braking at `decel` and arriving `margin` before it.
    pub fn speed_limit(
        &self,
        s: f32,
        horizon: f32,
        corner_speed: f32,
        decel: f32,
        margin: f32,
    ) -> f32 {
        let mut v = f32::INFINITY;
        for (i, &t) in self.turn.iter().enumerate() {
            if t < 0.05 {
                continue;
            }
            let d = self.wrap(self.cum[i] - s);
            if d > horizon {
                continue;
            }
            let vc = corner_speed * (std::f32::consts::FRAC_1_SQRT_2 / t).sqrt().min(1.5);
            let room = (d - margin).max(0.0);
            v = v.min((vc * vc + 2.0 * decel * room).sqrt());
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::track::Track;

    #[test]
    fn lines_on_the_ring() {
        let g = TrackGeom::new(&Track::ring());
        let c = Line::centreline(&g);
        assert!((c.length() - g.lap_length()).abs() < 1e-3);
        let (s, off) = c.project(Vec2::new(475.0, 130.0));
        assert!((s - 75.0).abs() < 1e-3 && (off - 30.0).abs() < 1e-3);
        let a = Line::apex(&g, 20.0);
        assert_eq!(a.pts.len(), 8, "gate 0 is straight on");
        assert!(
            a.length() < c.length() - 200.0,
            "the inside line is much shorter"
        );
        assert!(c.speed_limit(0.0, 300.0, 150.0, 400.0, 0.0) > 150.0);
        assert!((c.speed_limit(150.0, 300.0, 150.0, 400.0, 0.0) - 150.0).abs() < 1e-3);
    }
}
