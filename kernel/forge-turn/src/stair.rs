//! The in-process stock as columns: S = {(ρ, z) : 0 ≤ ρ ≤ R, bottom ≤ z ≤ h(ρ)}, h a step function.
//! It starts as the bar and only ever loses the top of a column, and only where a rectangle
//! inside what a cut swept reaches that column's top: so S stays a superset of the true stock,
//! and a clearance from S (G3: rapids, holders) is sound. No booleans: an update is a few
//! comparisons, the region a staircase polygon. The exact stock (ops.rs, by booleans) is the
//! test oracle and computes the G4 leftover on demand.
//!
//! The rectangles come from a convex polygon Q inside the swept outline (ops.rs `swept_inner`):
//! over an interval [x0, x1] of x, Q's lower boundary lo(x) is convex and its upper boundary
//! hi(x) concave, so the rectangle [x0, x1] × [max(lo(x0), lo(x1)), min(hi(x0), hi(x1))] lies in
//! Q. Q's own vertices and the steps' breakpoints split the intervals; the rectangle is pulled in
//! on z by more than the slices' rounding.
use crate::profile::poly;
use forge_cam_geom::region::Region;

#[derive(Clone, Debug, PartialEq)]
pub struct Stair {
    /// Breakpoints, increasing, from 0 to R; h[i] is the top over [xs[i], xs[i + 1]].
    pub xs: Vec<f64>,
    pub h: Vec<f64>,
    pub bottom: f64,
}

/// How far a column's top stays above the bottom at least (an empty column would make the
/// staircase degenerate; leaving a sliver is a superset, so sound).
const FLOOR: f64 = 1e-6;

impl Stair {
    pub fn bar(r: f64, front: f64, bottom: f64) -> Stair {
        Stair {
            xs: vec![0.0, r],
            h: vec![front],
            bottom,
        }
    }

    fn split(&mut self, x: f64) {
        let n = self.xs.len();
        if x <= self.xs[0] || x >= self.xs[n - 1] {
            return;
        }
        if let Err(i) = self.xs.binary_search_by(|v| v.total_cmp(&x)) {
            self.xs.insert(i, x);
            let hv = self.h[i - 1];
            self.h.insert(i, hv);
        }
    }

    /// Removes [x0, x1] × [zb, zt] where it takes a column's top (h ≤ zt): there the top falls
    /// to zb. Returns whether anything changed.
    pub fn clear_rect(&mut self, x0: f64, x1: f64, zb: f64, zt: f64) -> bool {
        let n = self.xs.len();
        let (x0, x1) = (x0.max(self.xs[0]), x1.min(self.xs[n - 1]));
        if !(x0 < x1 && zb < zt) {
            return false;
        }
        self.split(x0);
        self.split(x1);
        let mut changed = false;
        for i in 0..self.h.len() {
            let (a, b) = (self.xs[i], self.xs[i + 1]);
            if a >= x0 && b <= x1 && self.h[i] <= zt {
                let nh = zb.max(self.bottom + FLOOR);
                if nh < self.h[i] {
                    self.h[i] = nh;
                    changed = true;
                }
            }
        }
        // merge equal neighbours
        let mut i = 0;
        while i + 1 < self.h.len() {
            if self.h[i] == self.h[i + 1] {
                self.h.remove(i + 1);
                self.xs.remove(i + 1);
            } else {
                i += 1;
            }
        }
        changed
    }

    /// Clears what a convex polygon q (counter-clockwise) provably holds, column by column.
    pub fn clear_poly(&mut self, q: &[[f64; 2]]) -> bool {
        if q.len() < 3 {
            return false;
        }
        let mut cuts: Vec<f64> = q.iter().map(|p| p[0]).collect();
        let (qx0, qx1) = (
            cuts.iter().copied().fold(f64::INFINITY, f64::min),
            cuts.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        );
        for &x in &self.xs {
            if x > qx0 && x < qx1 {
                cuts.push(x);
            }
        }
        cuts.sort_by(|a, b| a.total_cmp(b));
        cuts.dedup();
        let mut changed = false;
        for w in cuts.windows(2) {
            let (x0, x1) = (w[0], w[1]);
            if !(x1 > x0) {
                continue;
            }
            // the slices at the interval's own ends: z = p + t (r − p) with t in [0, 1] rounds by
            // a few ulps of the coordinates whatever the edge's slope, so the rectangle is pulled
            // in on z only
            let (a, b) = (x0, x1);
            let (Some((la, ha)), Some((lb, hb))) = (slice(q, a), slice(q, b)) else {
                continue;
            };
            let lo = la.max(lb);
            let hi = ha.min(hb);
            let t = 1e-9 * (1.0 + lo.abs() + hi.abs());
            changed |= self.clear_rect(a, b, lo + t, hi - t);
        }
        changed
    }

    /// The staircase as a region (counter-clockwise).
    pub fn region(&self) -> Region {
        let n = self.xs.len();
        let mut pts = vec![[self.xs[0], self.bottom], [self.xs[n - 1], self.bottom]];
        let mut i = self.h.len();
        while i > 0 {
            i -= 1;
            pts.push([self.xs[i + 1], self.h[i]]);
            pts.push([self.xs[i], self.h[i]]);
        }
        // drop repeated points
        let mut out: Vec<[f64; 2]> = Vec::new();
        for p in pts {
            if out.last() != Some(&p) {
                out.push(p);
            }
        }
        while out.len() > 1 && out.first() == out.last() {
            out.pop();
        }
        Region {
            loops: vec![poly(&out)],
        }
    }
}

/// The z-extent [lo, hi] of a convex polygon over the vertical line at x (f64; None off it).
fn slice(q: &[[f64; 2]], x: f64) -> Option<(f64, f64)> {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    let n = q.len();
    for i in 0..n {
        let (p, r) = (q[i], q[(i + 1) % n]);
        if (p[0] <= x && x <= r[0]) || (r[0] <= x && x <= p[0]) {
            let z = if p[0] == r[0] {
                lo = lo.min(p[1].min(r[1]));
                hi = hi.max(p[1].max(r[1]));
                continue;
            } else {
                p[1] + (x - p[0]) / (r[0] - p[0]) * (r[1] - p[1])
            };
            lo = lo.min(z);
            hi = hi.max(z);
        }
    }
    (lo <= hi).then_some((lo, hi))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_fall_only_where_a_cut_takes_their_top() {
        let mut s = Stair::bar(16.0, 2.0, -100.0);
        // a facing band across the top: every column falls to 1
        assert!(s.clear_rect(0.0, 20.0, 1.0, 2.6));
        assert_eq!(s.h, vec![1.0]);
        // a band below the top takes nothing
        assert!(!s.clear_rect(0.0, 20.0, -5.0, -3.0));
        // an axial pass: the columns 14..16 fall to -40
        assert!(s.clear_rect(14.0, 17.0, -40.0, 3.0));
        assert_eq!(s.xs, vec![0.0, 14.0, 16.0]);
        assert_eq!(s.h, vec![1.0, -40.0]);
        let r = s.region();
        let mut cx = forge_cam_geom::real::Ctx::new();
        assert!(forge_cam_geom::region::validate(&mut cx, &r).is_ok(), "{r:?}");
    }
}
