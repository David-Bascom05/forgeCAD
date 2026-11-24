//! The drop-cutter interface the strategies are written against.
//!
//! The primitive itself (exact drop-cutter of flat, ball and bull-nose cutters against the
//! lowered B-rep) is CAM-3AXIS's; the strategies here need only two certified answers from it:
//!
//! - [`DropCutter::drop`]: at (x, y), an enclosure of the cutter-location height: the lowest tip
//!   z at which the cutter, axis vertical, touches nothing of the part (plus stock to leave);
//! - [`DropCutter::drop_seg`]: over the segment from a to b in xy, an enclosure of the *maximum*
//!   of that height. A cutting move from a to b at height z is free of gouges iff z is at least
//!   that maximum (for a line in xy at constant z; a sloped line is split until each piece's
//!   chord is above the bound, see `chord_clear`).
//!
//! Both return `None` where no face is under the tool's footprint: the cutter can go down
//! freely there, and a strategy treats it as outside its region.
//!
//! [`Analytic`] implements both exactly for a ball cutter over horizontal planes and spheres
//! (dome test parts with closed-form answers); it is the reference the strategy tests use.

use forge_cam_geom::ival::Ival;

/// A certified enclosure of a height: lo ≤ the true value ≤ hi.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Band {
    pub lo: f64,
    pub hi: f64,
}

impl Band {
    pub fn of(i: Ival) -> Band {
        Band { lo: i.lo, hi: i.hi }
    }
    pub fn width(&self) -> f64 {
        self.hi - self.lo
    }
    /// The larger of two maxima.
    pub fn max(self, o: Band) -> Band {
        Band {
            lo: self.lo.max(o.lo),
            hi: self.hi.max(o.hi),
        }
    }
}

/// `Sync`: strategies sample it from several threads (forge_math::par).
pub trait DropCutter: Sync {
    /// The cutter-location height at (x, y), or `None` with nothing under the tool.
    fn drop(&self, x: f64, y: f64) -> Option<Band>;
    /// The maximum cutter-location height along the xy segment a → b, or `None` with nothing
    /// under the tool anywhere along it.
    fn drop_seg(&self, a: [f64; 2], b: [f64; 2]) -> Option<Band>;
    /// Whether the CL height is *proven* at most `z` all along the xy segment a → b (true with
    /// nothing under the tool). The decision form of `drop_seg`, which may stop early: false
    /// means a point above z was found or the proof ran out of budget.
    fn seg_below(&self, a: [f64; 2], b: [f64; 2], z: f64) -> bool;
    /// Whether the CL height is *proven* at most the chord's height plus `slack` all along the
    /// 3D segment a → b (z linear in the xy parameter). With slack ≤ the op's inner tolerance this
    /// is the generator's no-gouge test for a sloped cutting move.
    fn seg3_below(&self, a: [f64; 3], b: [f64; 3], slack: f64) -> bool;
    /// The xy box the part occupies, grown by the tool radius: outside it `drop` is `None`.
    fn extent(&self) -> [[f64; 2]; 2];
}

/// The band of the maximum CL height along a → b from the trait's own tests: the lower end the
/// larger of the ends' lower ends (points of the segment); the upper end the lowest height at
/// which the level move is proven clear (`seg_below`), found by stepping up from the ends' upper
/// ends and then bisecting to `tol`. None with nothing under the tool at either end.
pub fn seg_band(dc: &dyn DropCutter, a: [f64; 2], b: [f64; 2], tol: f64) -> Option<Band> {
    let da = dc.drop(a[0], a[1]);
    let db = dc.drop(b[0], b[1]);
    let (lo, start) = match (da, db) {
        (None, None) => return None,
        (Some(p), None) | (None, Some(p)) => (p.lo, p.hi),
        (Some(p), Some(q)) => (p.lo.max(q.lo), p.hi.max(q.hi)),
    };
    if dc.seg_below(a, b, start) {
        return Some(Band { lo, hi: start });
    }
    let mut l = start;
    let mut step = tol.max(1e-6);
    let mut h = start + step;
    let mut k = 0;
    while !dc.seg_below(a, b, h) {
        l = h;
        step *= 2.0;
        h = start + step;
        k += 1;
        if k > 60 {
            return Some(Band {
                lo,
                hi: f64::INFINITY,
            });
        }
    }
    let mut k = 0;
    while h - l > tol && k < 60 {
        let m = 0.5 * (l + h);
        if dc.seg_below(a, b, m) {
            h = m;
        } else {
            l = m;
        }
        k += 1;
    }
    Some(Band { lo, hi: h })
}

/// A ball cutter of radius `r` over a union of horizontal planes (each a z over an xy box) and
/// spheres (centre, radius): exact closed forms. Its CL height over a sphere is
/// c.z + √((ρ + r)² − d²) − r for xy distance d < ρ + r (the ball's centre is at ρ + r from the
/// sphere's); over a plane box it is z where the ball's footprint meets the box (a ball's
/// lowest point is its tip, so for a box under the footprint but not under the tip the contact
/// is on the box's edge: c.z = z + √(r² − e²) − r with e the xy distance to the box).
#[derive(Clone, Debug)]
pub struct Analytic {
    pub r: f64,
    /// (z, [xmin, ymin], [xmax, ymax]).
    pub planes: Vec<(f64, [f64; 2], [f64; 2])>,
    /// (centre, radius).
    pub spheres: Vec<([f64; 3], f64)>,
}

fn dist_box(p: [Ival; 2], lo: [f64; 2], hi: [f64; 2]) -> Ival {
    let mut s = Ival::pt(0.0);
    let mut k = 0;
    while k < 2 {
        // max(lo − p, 0, p − hi)
        let a = Ival::pt(lo[k]).sub(p[k]);
        let b = p[k].sub(Ival::pt(hi[k]));
        let e = a.max(b).max(Ival::pt(0.0));
        s = s.add(e.sqr());
        k += 1;
    }
    s.sqrt()
}

/// The height of a ball's tip touching a point-like feature at xy distance `e`, height z.
fn over_edge(r: f64, z: f64, e: Ival) -> Option<Ival> {
    let ri = Ival::pt(r);
    let disc = ri.sqr().sub(e.sqr());
    if disc.hi <= 0.0 {
        return None;
    }
    Some(Ival::pt(z).add(disc.max(Ival::pt(0.0)).sqrt()).sub(ri))
}

impl Analytic {
    /// The CL height at an xy point given as intervals (a box of points: the result encloses
    /// the CL height of every point in it).
    fn cl(&self, p: [Ival; 2]) -> Option<Ival> {
        let mut best: Option<Ival> = None;
        let mut i = 0;
        while i < self.planes.len() {
            let (z, lo, hi) = self.planes[i];
            let e = dist_box(p, lo, hi);
            if let Some(h) = over_edge(self.r, z, e) {
                best = Some(match best {
                    None => h,
                    Some(b) => b.max(h),
                });
            }
            i += 1;
        }
        let mut j = 0;
        while j < self.spheres.len() {
            let (c, rho) = self.spheres[j];
            let dx = p[0].sub(Ival::pt(c[0]));
            let dy = p[1].sub(Ival::pt(c[1]));
            let d2 = dx.sqr().add(dy.sqr());
            let big = Ival::pt(rho + self.r);
            let disc = big.sqr().sub(d2);
            if disc.hi > 0.0 {
                let h = Ival::pt(c[2])
                    .add(disc.max(Ival::pt(0.0)).sqrt())
                    .sub(Ival::pt(self.r));
                best = Some(match best {
                    None => h,
                    Some(b) => b.max(h),
                });
            }
            j += 1;
        }
        best
    }
}

impl DropCutter for Analytic {
    fn drop(&self, x: f64, y: f64) -> Option<Band> {
        self.cl([Ival::pt(x), Ival::pt(y)]).map(Band::of)
    }

    fn drop_seg(&self, a: [f64; 2], b: [f64; 2]) -> Option<Band> {
        // branch and bound on the parameter: the max over pieces of the enclosure over each
        // piece; pieces whose upper end is below the best proven lower end are dropped
        let mut stack: Vec<(f64, f64)> = vec![(0.0, 1.0)];
        let mut lo = f64::NEG_INFINITY;
        let mut hi = f64::NEG_INFINITY;
        let mut any = false;
        let mut n = 0;
        while let Some((s0, s1)) = stack.pop() {
            n += 1;
            let s = Ival::new(s0, s1);
            let px = Ival::pt(a[0]).add(Ival::pt(b[0]).sub(Ival::pt(a[0])).mul(s));
            let py = Ival::pt(a[1]).add(Ival::pt(b[1]).sub(Ival::pt(a[1])).mul(s));
            let Some(h) = self.cl([px, py]) else { continue };
            any = true;
            // the real point at s0 is on the segment; the enclosure of its CL (its coordinates
            // as intervals) has a lower end below its CL, so below the maximum: a proven lower end
            let s0i = Ival::pt(s0);
            let qx = Ival::pt(a[0]).add(Ival::pt(b[0]).sub(Ival::pt(a[0])).mul(s0i));
            let qy = Ival::pt(a[1]).add(Ival::pt(b[1]).sub(Ival::pt(a[1])).mul(s0i));
            if let Some(m) = self.cl([qx, qy]) {
                lo = lo.max(m.lo);
            }
            if h.hi <= lo || h.hi - h.lo <= 1e-9 || s1 - s0 < 1e-12 || n > 20000 {
                hi = hi.max(h.hi);
                continue;
            }
            let m = 0.5 * (s0 + s1);
            stack.push((s0, m));
            stack.push((m, s1));
        }
        if !any {
            return None;
        }
        Some(Band { lo, hi })
    }

    fn seg_below(&self, a: [f64; 2], b: [f64; 2], z: f64) -> bool {
        let mut stack: Vec<(f64, f64)> = vec![(0.0, 1.0)];
        let mut n = 0;
        while let Some((s0, s1)) = stack.pop() {
            n += 1;
            if n > 4000 {
                return false;
            }
            let s = Ival::new(s0, s1);
            let px = Ival::pt(a[0]).add(Ival::pt(b[0]).sub(Ival::pt(a[0])).mul(s));
            let py = Ival::pt(a[1]).add(Ival::pt(b[1]).sub(Ival::pt(a[1])).mul(s));
            let Some(h) = self.cl([px, py]) else { continue };
            if h.hi <= z {
                continue;
            }
            if h.lo > z || s1 - s0 < 1e-14 {
                return false;
            }
            let m = 0.5 * (s0 + s1);
            stack.push((s0, m));
            stack.push((m, s1));
        }
        true
    }

    fn seg3_below(&self, a: [f64; 3], b: [f64; 3], slack: f64) -> bool {
        let mut stack: Vec<(f64, f64)> = vec![(0.0, 1.0)];
        let mut n = 0;
        while let Some((s0, s1)) = stack.pop() {
            n += 1;
            if n > 8000 {
                return false;
            }
            let s = Ival::new(s0, s1);
            let px = Ival::pt(a[0]).add(Ival::pt(b[0]).sub(Ival::pt(a[0])).mul(s));
            let py = Ival::pt(a[1]).add(Ival::pt(b[1]).sub(Ival::pt(a[1])).mul(s));
            let pz = Ival::pt(a[2])
                .add(Ival::pt(b[2]).sub(Ival::pt(a[2])).mul(s))
                .add(Ival::pt(slack));
            let Some(h) = self.cl([px, py]) else { continue };
            // proven below on the piece: the CL's upper end under the chord's lower end
            if h.hi <= pz.lo {
                continue;
            }
            if h.lo > pz.hi || s1 - s0 < 1e-14 {
                return false;
            }
            let m = 0.5 * (s0 + s1);
            stack.push((s0, m));
            stack.push((m, s1));
        }
        true
    }

    fn extent(&self) -> [[f64; 2]; 2] {
        let mut lo = [f64::INFINITY; 2];
        let mut hi = [f64::NEG_INFINITY; 2];
        for &(_, a, b) in &self.planes {
            lo = [lo[0].min(a[0] - self.r), lo[1].min(a[1] - self.r)];
            hi = [hi[0].max(b[0] + self.r), hi[1].max(b[1] + self.r)];
        }
        for &(c, rho) in &self.spheres {
            let e = rho + self.r;
            lo = [lo[0].min(c[0] - e), lo[1].min(c[1] - e)];
            hi = [hi[0].max(c[0] + e), hi[1].max(c[1] + e)];
        }
        [lo, hi]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dome() -> Analytic {
        Analytic {
            r: 3.0,
            planes: vec![(0.0, [-50.0, -50.0], [50.0, 50.0])],
            spheres: vec![([0.0, 0.0, -10.0], 30.0)],
        }
    }

    #[test]
    fn dome_top_and_floor() {
        let d = dome();
        let t = d.drop(0.0, 0.0).unwrap();
        assert!(t.lo <= 20.0 && 20.0 <= t.hi && t.width() < 1e-12);
        // far from the dome: the floor
        let f = d.drop(45.0, 45.0).unwrap();
        assert!(f.lo <= 0.0 && 0.0 <= f.hi);
        // nothing under the tool
        assert!(d.drop(60.0, 0.0).is_none());
    }

    #[test]
    fn seg_max_is_the_dome_top() {
        let d = dome();
        let b = d.drop_seg([-20.0, 0.0], [20.0, 0.0]).unwrap();
        assert!(b.lo <= 20.0 && 20.0 <= b.hi, "{b:?}");
        assert!(b.width() < 1e-6, "{b:?}");
    }
}
