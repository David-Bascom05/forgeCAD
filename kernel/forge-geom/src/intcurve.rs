//! Curves defined as "where surface A meets surface B", for pairs with no
//! closed form (a sphere and a cylinder meet in a quartic space curve).
//!
//! The curve keeps both surfaces and a dense chain of points lying on both,
//! with the curve's direction at each. The parameter runs along the chain
//! (point i at t = i); an evaluation starts from the cubic through the chain
//! (Hermite, from those directions: within about 1e-6 mm) and takes one Newton
//! step onto both surfaces, so the chain only guides — the curve is exact.

use crate::surface::Surface;
use forge_math::{Box3, Frame, Interval, Point2, Point3, Vec3, tol};
use std::sync::OnceLock;

#[derive(Clone, Debug)]
pub struct IntCurve {
    pub a: Surface,
    pub b: Surface,
    /// Points on both surfaces, in order along the curve.
    pub pts: Vec<Point3>,
    /// The curve's unit direction at each point, along increasing t.
    pub tans: Vec<Vec3>,
    /// The last point connects back to the first.
    pub closed: bool,
    /// Largest distance from a chain segment to the true curve between its ends.
    pub sag: f64,
    /// `reach`, found on first use.
    reach: OnceLock<f64>,
    /// Each chain point's parameters on each surface that can start its value there
    /// (`Procedural::value_near`), found on first use.
    hints: OnceLock<[Option<Vec<Point2>>; 2]>,
}

impl PartialEq for IntCurve {
    fn eq(&self, o: &IntCurve) -> bool {
        (&self.a, &self.b, &self.pts, &self.tans, self.closed, self.sag)
            == (&o.a, &o.b, &o.pts, &o.tans, o.closed, o.sag)
    }
}

/// Pulls `p` onto both surfaces: the minimum-norm Newton step for the two
/// implicit equations, repeated. None when the surfaces are tangent there.
pub fn refine(a: &Surface, b: &Surface, mut p: Point3) -> Option<Point3> {
    for _ in 0..40 {
        let ((ga, na), (gb, nb)) = (a.implicit_gradient(p), b.implicit_gradient(p));
        if ga.abs() <= 1e-3 * tol::LINEAR && gb.abs() <= 1e-3 * tol::LINEAR {
            return Some(p);
        }
        // dp = J^T (J J^T)^-1 (-g) with J = [na; nb]
        let (aa, ab, bb) = (na.dot(na), na.dot(nb), nb.dot(nb));
        let det = aa * bb - ab * ab;
        if det <= 1e-12 * aa * bb {
            return None;
        }
        let la = (-ga * bb + gb * ab) / det;
        let lb = (-gb * aa + ga * ab) / det;
        p += na * la + nb * lb;
    }
    let ok = a.implicit(p).abs() <= tol::LINEAR && b.implicit(p).abs() <= tol::LINEAR;
    ok.then_some(p)
}

/// One minimum-norm Newton step of `p` towards both surfaces, from their values and gradients
/// there (p itself where they are tangent).
fn newton_step(p: Point3, (ga, na): (f64, Vec3), (gb, nb): (f64, Vec3)) -> Point3 {
    let (aa, ab, bb) = (na.dot(na), na.dot(nb), nb.dot(nb));
    let det = aa * bb - ab * ab;
    if det <= 1e-12 * aa * bb {
        return p;
    }
    p + na * ((-ga * bb + gb * ab) / det) + nb * ((-gb * aa + ga * ab) / det)
}

/// Whether a Newton step from values this small lands within `refine`'s 1e-8: what it leaves is
/// about the curvature times their square, 1e-12 times a curvature of up to 1e4.
fn settled(a: f64, b: f64) -> bool {
    a.abs() <= 1e-6 && b.abs() <= 1e-6
}

/// Unit direction of the curve at p (na x nb), or None where the surfaces are tangent.
pub fn direction(a: &Surface, b: &Surface, p: Point3) -> Option<Vec3> {
    a.gradient(p).cross(b.gradient(p)).normalize().map(|u| u.get())
}

impl IntCurve {
    pub fn new(
        a: Surface,
        b: Surface,
        pts: Vec<Point3>,
        tans: Vec<Vec3>,
        closed: bool,
        sag: f64,
    ) -> IntCurve {
        IntCurve {
            reach: OnceLock::new(),
            a,
            b,
            pts,
            tans,
            closed,
            sag,
            hints: OnceLock::new(),
        }
    }

    /// The chain with each step that turns by more than about 10 degrees, on a bend tight for a ball of
    /// radius `r` about the curve, halved until none does: the fixed step of a trace follows the curve's
    /// reach over its box, not its tightest bend, and a chain too coarse there has the nearest points of
    /// a ball's surface in the wrong place. Self if no step does.
    pub fn refined(self, r: f64) -> IntCurve {
        let mut pts = self.pts.clone();
        for _ in 0..6 {
            let n = pts.len();
            if !(2..=20000).contains(&n) {
                break;
            }
            let dirs: Vec<Option<Vec3>> = pts.iter().map(|&p| direction(&self.a, &self.b, p)).collect();
            let segs = if self.closed { n } else { n - 1 };
            let mut next = Vec::with_capacity(n);
            let mut added = false;
            for i in 0..segs {
                let (p, q) = (pts[i], pts[(i + 1) % n]);
                next.push(p);
                // the angle over the step's length is the curvature
                let turns = matches!((dirs[i], dirs[(i + 1) % n]), (Some(u), Some(v))
                    if u.dot(v).abs() < 0.985 && u.cross(v).length() * r > 0.25 * p.distance(q));
                let m = p.lerp(q, 0.5);
                if turns
                    && let Some(x) = refine(&self.a, &self.b, m)
                    && x.distance(m) <= p.distance(q)
                    && x.distance(p) > 0.0
                    && x.distance(q) > 0.0
                {
                    next.push(x);
                    added = true;
                }
            }
            if !self.closed {
                next.push(pts[n - 1]);
            }
            pts = next;
            if !added {
                break;
            }
        }
        if pts.len() == self.pts.len() {
            return self;
        }
        let step = pts.windows(2).map(|w| w[0].distance(w[1])).fold(0.0, f64::max);
        finish(&self.a, &self.b, pts, self.closed, step)
    }

    /// The chain carried on past its end (or its start) to `p`, a point on both surfaces, by walking
    /// on along them, with how many segments now come before the old chain. None if the branch doesn't
    /// lead there.
    pub fn carried_to(&self, p: Point3, at_end: bool) -> Option<(IntCurve, usize)> {
        if self.closed {
            return None;
        }
        let (from, out) = if at_end {
            (*self.pts.last()?, *self.tans.last()?)
        } else {
            (*self.pts.first()?, -*self.tans.first()?)
        };
        let left = from.distance(p);
        if left <= tol::LINEAR {
            return None;
        }
        let on = towards_touch(&self.a, &self.b, from, p, left / 16.0)?;
        if (on.pts[1] - from).dot(out) <= 0.0 {
            return None;
        }
        let sag = self.sag.max(on.sag);
        let n = on.pts.len() - 1;
        let joined = if at_end {
            (
                [&self.pts[..], &on.pts[1..]].concat(),
                [&self.tans[..], &on.tans[1..]].concat(),
            )
        } else {
            let pts: Vec<Point3> = on.pts[1..].iter().rev().chain(&self.pts).copied().collect();
            let back = on.tans[1..].iter().rev().map(|&t| -t);
            let tans: Vec<Vec3> = back.chain(self.tans.iter().copied()).collect();
            (pts, tans)
        };
        let curve = IntCurve::new(self.a.clone(), self.b.clone(), joined.0, joined.1, false, sag);
        Some((curve, if at_end { 0 } else { n }))
    }

    pub fn segments(&self) -> usize {
        if self.closed {
            self.pts.len()
        } else {
            self.pts.len() - 1
        }
    }

    /// t on the domain: wrapped if closed, clamped if open. In range it is t itself, without
    /// the remainder (a library call on wasm).
    fn wrap(&self, t: f64, hi: f64) -> f64 {
        if (0.0..hi).contains(&t) {
            t
        } else if self.closed {
            t.rem_euclid(hi)
        } else {
            t.clamp(0.0, hi)
        }
    }

    pub fn domain_hi(&self) -> f64 {
        self.segments() as f64
    }

    /// The chain segment holding t: its end samples' indices and t's fraction along it.
    fn locate(&self, t: f64) -> (usize, usize, f64) {
        let n = self.pts.len();
        let seg = self.segments();
        let t = self.wrap(t, seg as f64);
        let i = (t.floor() as usize).min(seg.saturating_sub(1));
        (i, (i + 1) % n, t - i as f64)
    }

    /// The cubic through the chain at t: point, first and second derivative.
    fn hermite(&self, t: f64) -> (Point3, Vec3, Vec3) {
        let (i, j, f) = self.locate(t);
        let (p0, p1) = (self.pts[i], self.pts[j]);
        let chord = p1 - p0;
        let len = chord.length();
        let (m0, m1) = (self.tans[i] * len, self.tans[j] * len);
        let (f2, f3) = (f * f, f * f * f);
        let p = p0 + chord * (3.0 * f2 - 2.0 * f3) + m0 * (f3 - 2.0 * f2 + f) + m1 * (f3 - f2);
        let d1 = chord * (6.0 * f - 6.0 * f2) + m0 * (3.0 * f2 - 4.0 * f + 1.0) + m1 * (3.0 * f2 - 2.0 * f);
        let d2 = chord * (6.0 - 12.0 * f) + m0 * (6.0 * f - 4.0) + m1 * (6.0 * f - 2.0);
        (p, d1, d2)
    }

    /// The cubic's point alone (as `hermite`'s).
    #[inline(always)]
    fn hermite_point(&self, t: f64) -> Point3 {
        let (i, j, f) = self.locate(t);
        let (p0, p1) = (self.pts[i], self.pts[j]);
        let chord = p1 - p0;
        let len = chord.length();
        let (m0, m1) = (self.tans[i] * len, self.tans[j] * len);
        let (f2, f3) = (f * f, f * f * f);
        p0 + chord * (3.0 * f2 - 2.0 * f3) + m0 * (f3 - 2.0 * f2 + f) + m1 * (f3 - f2)
    }

    pub fn eval(&self, t: f64) -> Point3 {
        self.onto(self.hermite_point(t), t)
    }

    /// Each surface's value and gradient at p, and its parameters there where it was started
    /// from `uv`.
    fn side(&self, k: usize, p: Point3, uv: Option<Point2>) -> ((f64, Vec3), Option<Point2>) {
        let s = if k == 0 { &self.a } else { &self.b };
        if let (Surface::Procedural(pr), Some(uv)) = (s, uv)
            && let Some((f, g, at)) = pr.value_near(p, Some(uv))
        {
            return ((f, g), Some(at));
        }
        (s.implicit_gradient(p), None)
    }

    /// Where each surface that can start from them has parameters near chain point t.
    fn hint(&self, t: f64) -> [Option<Point2>; 2] {
        let near = |s: &Surface| match s {
            Surface::Procedural(pr) => self
                .pts
                .iter()
                .map(|&p| pr.value_near(p, None).map(|v| v.2))
                .collect::<Option<Vec<Point2>>>(),
            _ => None,
        };
        let [ha, hb] = self.hints.get_or_init(|| [near(&self.a), near(&self.b)]);
        let (i, j, f) = self.locate(t);
        let at = |h: &Option<Vec<Point2>>| h.as_ref().map(|h| h[i].lerp(h[j], f));
        [at(ha), at(hb)]
    }

    /// The cubic's point at t taken onto both surfaces.
    #[inline(always)]
    fn onto(&self, p: Point3, t: f64) -> Point3 {
        let unhinted = |h: &[Option<Vec<Point2>>; 2]| h[0].is_none() && h[1].is_none();
        if (matches!(self.a, Surface::Procedural(_)) || matches!(self.b, Surface::Procedural(_)))
            && !self.hints.get().is_some_and(unhinted)
        {
            return self.onto_from_chain(p, t);
        }
        let (va, vb) = (self.a.implicit_gradient(p), self.b.implicit_gradient(p));
        let q = newton_step(p, va, vb);
        if settled(va.0, vb.0) {
            return q;
        }
        // one step lands where the cubic was close; where it wasn't, finish the job
        let tight = |p: Point3| {
            self.a.implicit(p).abs() <= 1e-3 * tol::LINEAR && self.b.implicit(p).abs() <= 1e-3 * tol::LINEAR
        };
        if tight(q) {
            q
        } else {
            refine(&self.a, &self.b, q).unwrap_or(q)
        }
    }

    /// `onto`, each procedural surface that can searching from the chain's parameters there.
    #[inline(never)]
    fn onto_from_chain(&self, p: Point3, t: f64) -> Point3 {
        let [ha, hb] = self.hint(t);
        let ((va, ua), (vb, ub)) = (self.side(0, p, ha), self.side(1, p, hb));
        let q = newton_step(p, va, vb);
        if settled(va.0, vb.0) {
            return q;
        }
        // and from where p's search ended for q
        let level = |k: usize, uv: Option<Point2>| match uv {
            Some(_) => self.side(k, q, uv).0.0,
            None => [&self.a, &self.b][k].implicit(q),
        };
        if level(0, ua).abs() <= 1e-3 * tol::LINEAR && level(1, ub).abs() <= 1e-3 * tol::LINEAR {
            q
        } else {
            refine(&self.a, &self.b, q).unwrap_or(q)
        }
    }

    /// First and second derivative with respect to t (the chain spacing sets its scale).
    fn derivs(&self, t: f64) -> (Point3, Vec3, Vec3) {
        let (p, d1, d2) = self.hermite(t);
        (self.onto(p, t), d1, d2)
    }

    /// The guiding cubic's derivative at t, no surfaces asked (`derivs`' first derivative).
    pub fn guide_tangent(&self, t: f64) -> Vec3 {
        self.hermite(t).1
    }

    /// Bound on |dP/dt| over the curve.
    pub fn speed_bound(&self) -> f64 {
        let n = self.pts.len();
        (0..self.segments())
            .map(|i| self.pts[i].distance(self.pts[(i + 1) % n]))
            .fold(0.0, f64::max)
            * 1.5
            + 2.0 * self.sag
    }

    /// A point of the curve lies this near a chain sample: half the longest chord and the sag.
    pub fn reach(&self) -> f64 {
        *self.reach.get_or_init(|| {
            let n = self.pts.len();
            let longest = (0..self.segments())
                .map(|i| self.pts[i].distance_sq(self.pts[(i + 1) % n]))
                .fold(0.0, f64::max);
            0.5 * longest.sqrt() + self.sag
        })
    }

    /// The squared distance from p to the nearest chain sample.
    pub fn nearest_sample_sq(&self, p: Point3) -> f64 {
        self.pts
            .iter()
            .map(|q| q.distance_sq(p))
            .fold(f64::INFINITY, f64::min)
    }

    pub fn closest(&self, p: Point3, range: Interval) -> f64 {
        let seg = self.segments();
        let (lo, hi) = (
            range.lo.max(if self.closed { f64::NEG_INFINITY } else { 0.0 }),
            range.hi,
        );
        let hi = if self.closed { hi } else { hi.min(seg as f64) };
        // nearest chain sample in range, then golden-section on the two neighbouring segments
        let mut best = (f64::INFINITY, lo);
        let start = lo.ceil() as i64;
        let end = hi.floor() as i64;
        let n = self.pts.len() as i64;
        for k in start..=end {
            let d = self.pts[k.rem_euclid(n) as usize].distance_sq(p);
            if d < best.0 {
                best = (d, k as f64);
            }
        }
        for t in [lo, hi] {
            let d = self.eval(t).distance_sq(p);
            if d < best.0 {
                best = (d, t);
            }
        }
        // Newton on (C(t) - p) . C'(t) = 0 along the cubic, on the segments either side
        let near = best.1;
        let mut out = near;
        let mut out_d = self.eval(near).distance_sq(p);
        for (s0, s1) in [((near - 1.0).max(lo), near), (near, (near + 1.0).min(hi))] {
            if s1 <= s0 {
                continue;
            }
            let mut t = self.on_cubic(p, s0, s1);
            // then against the exact curve (the cubic's nearest point sits a hair off it)
            for _ in 0..3 {
                let d1 = self.hermite(t).1;
                let g = (self.eval(t) - p).dot(d1);
                let h = d1.dot(d1);
                if h <= 0.0 {
                    break;
                }
                t = (t - g / h).clamp(s0, s1);
            }
            let d = self.eval(t).distance_sq(p);
            if d < out_d {
                (out, out_d) = (t, d);
            }
        }
        out
    }

    /// The cubic's point nearest p between s0 and s1, by Newton from the middle.
    fn on_cubic(&self, p: Point3, s0: f64, s1: f64) -> f64 {
        let mut t = 0.5 * (s0 + s1);
        for _ in 0..12 {
            let (c, d1, d2) = self.hermite(t);
            let g = (c - p).dot(d1);
            let h = d1.dot(d1) + (c - p).dot(d2);
            if h <= 0.0 {
                break;
            }
            let next = (t - g / h).clamp(s0, s1);
            let done = (next - t).abs() <= 1e-13;
            t = next;
            if done {
                break;
            }
        }
        t
    }

    /// The parameter nearest p along the guiding cubic, for a caller that finishes on the exact
    /// curve: no evaluation of the surfaces at all. The sample's tangent says which side the
    /// foot is on.
    pub fn closest_on_cubic(&self, p: Point3) -> f64 {
        let (mut best, mut near) = (f64::INFINITY, 0usize);
        for (k, q) in self.pts.iter().enumerate() {
            let d = q.distance_sq(p);
            if d < best {
                (best, near) = (d, k);
            }
        }
        let k = near as f64;
        let (s0, s1) = if (p - self.pts[near]).dot(self.tans[near]) >= 0.0 {
            (k, k + 1.0)
        } else {
            (k - 1.0, k)
        };
        if !self.closed && (s0 < 0.0 || s1 > self.domain_hi()) {
            return k;
        }
        self.on_cubic(p, s0, s1)
    }

    /// The guiding cubic's point at t: within about 1e-6 of the curve, no surfaces asked.
    pub fn guide(&self, t: f64) -> Point3 {
        self.hermite_point(t)
    }

    pub fn bbox(&self, range: Interval) -> Box3 {
        let mut b = Box3::from_points([self.eval(range.lo), self.eval(range.hi)]);
        let n = self.pts.len();
        let (s, e) = (range.lo.ceil() as i64, range.hi.floor() as i64);
        for k in s..=e {
            b.include(self.pts[(k.rem_euclid(n as i64)) as usize]);
        }
        b.grow(self.sag + tol::LINEAR)
    }

    pub fn transformed(&self, f: &Frame) -> IntCurve {
        use crate::surface::SurfaceGeom;
        IntCurve::new(
            self.a.transformed(f),
            self.b.transformed(f),
            self.pts.iter().map(|&p| f.point_to_world(p)).collect(),
            self.tans.iter().map(|&v| f.vec_to_world(v)).collect(),
            self.closed,
            self.sag,
        )
    }
}

/// Every branch of the intersection of two surfaces inside `region`.
///
/// Seeds come from an octree over the region: a cell can hold no point of
/// the intersection if either surface's distance at its centre exceeds the
/// cell's half-diagonal (both implicits are 1-Lipschitz). Each seed is pulled
/// onto both surfaces and traced both ways along na x nb, with a Newton
/// correction at every step, until the branch leaves the region or closes.
/// Seeds already lying on a traced branch are skipped. None if the surfaces
/// are tangent somewhere the tracing needs to pass.
pub fn trace(a: &Surface, b: &Surface, region: Box3) -> Option<Vec<IntCurve>> {
    let size = region.diagonal().max(1e-6);
    let feature = |s: &Surface| match s {
        Surface::Procedural(p) => p.feature(),
        _ => f64::INFINITY,
    };
    let leaf = (size / 64.0)
        .min(feature(a))
        .min(feature(b))
        .max(10.0 * tol::LINEAR);
    let step = leaf * 0.5;
    let mut seeds: Vec<Point3> = Vec::new();
    // cells are cubes, each held as its circumscribed sphere: the root is the cube on the
    // region's longest side (the region's own half diagonal leaves its long corners out)
    let half = 0.5
        * region
            .x
            .width()
            .max(region.y.width())
            .max(region.z.width())
            .max(1e-6);
    // a point on both faces lies in a patch of each that overlaps one of the other's
    let near = |own: Option<&[Box3]>, other: Option<&[Box3]>| {
        own.map(|ps| {
            ps.iter()
                .filter(|p| p.overlaps(&region) && other.is_none_or(|qs| qs.iter().any(|q| q.overlaps(p))))
                .copied()
                .collect::<Vec<Box3>>()
        })
    };
    let (pa, pb) = (near(a.patches(), b.patches()), near(b.patches(), a.patches()));
    if pa.as_ref().is_some_and(Vec::is_empty) || pb.as_ref().is_some_and(Vec::is_empty) {
        return Some(Vec::new());
    }
    let reaches = |ps: &Option<Vec<Box3>>, c: Point3, r: f64| {
        ps.as_ref().is_none_or(|ps| {
            ps.iter().any(|p| {
                let d = |i: Interval, x: f64| (i.lo - x).max(x - i.hi).max(0.0);
                let (dx, dy, dz) = (d(p.x, c.x), d(p.y, c.y), d(p.z, c.z));
                dx * dx + dy * dy + dz * dz <= r * r
            })
        })
    };
    let boxed = pa.is_some() || pb.is_some();
    // the cheap surface first: most cells are off it
    let (first, second) = if matches!(a, Surface::Procedural(_)) {
        (b, a)
    } else {
        (a, b)
    };
    // the root cube stands well clear of a flat region; a point of the curve in the region lies
    // in a cell that touches it
    let grown = region.grow(step);
    let touches = |c: Point3, r: f64| {
        let d = |i: Interval, x: f64| (i.lo - x).max(x - i.hi).max(0.0);
        let (dx, dy, dz) = (d(grown.x, c.x), d(grown.y, c.y), d(grown.z, c.z));
        dx * dx + dy * dy + dz * dz <= r * r
    };
    // a surface that can name points of the curve spares the octree
    let named = match (a, b) {
        (Surface::Procedural(p), other) | (other, Surface::Procedural(p)) => {
            // its steps a fraction of a small round's radius: a narrow loop on one has sharp tips
            let round = match other {
                Surface::Cylinder(c) => c.radius,
                Surface::Sphere(s) => s.radius,
                Surface::Cone(c) => c.radius,
                _ => f64::INFINITY,
            };
            let fine = leaf.min(0.5 * round).max(10.0 * tol::LINEAR);
            p.seeds(other, grown, fine).map(|s| (s, fine))
        }
        _ => None,
    };
    let (leaf, step) = named.as_ref().map_or((leaf, step), |n| (n.1, 0.5 * n.1));
    let named = named.map(|n| n.0);
    let octree = named.is_none();
    seeds.extend(named.into_iter().flatten());
    let mut stack = if octree {
        vec![(region.center(), half * 3f64.sqrt())]
    } else {
        Vec::new()
    };
    while let Some((c, r)) = stack.pop() {
        if !touches(c, r)
            || (boxed && (!reaches(&pa, c, r) || !reaches(&pb, c, r)))
            || first.implicit(c).abs() > r
            || !match second {
                Surface::Procedural(s) => s.within(c, r),
                s => s.implicit(c).abs() <= r,
            }
        {
            continue;
        }
        if 2.0 * r <= leaf {
            seeds.push(c);
            continue;
        }
        let h = r * 0.5;
        let o = h / 3f64.sqrt();
        for dx in [-o, o] {
            for dy in [-o, o] {
                for dz in [-o, o] {
                    stack.push((c + Vec3::new(dx, dy, dz), h));
                }
            }
        }
    }
    let mut curves: Vec<IntCurve> = Vec::new();
    // cells (leaf-sized) around every traced point: a seed in one needs no refining, it lies
    // on a branch already traced (most seeds do, the octree leaves line every branch)
    let key = |p: Point3| {
        let f = |x: f64, lo: f64| (((x - lo) / leaf).floor() as i64 + 2) as u64 & 0x1f_ffff;
        (f(p.x, region.x.lo) << 42) | (f(p.y, region.y.lo) << 21) | f(p.z, region.z.lo)
    };
    let mut covered: Vec<u64> = Vec::new();
    let cover = |c: &IntCurve, covered: &mut Vec<u64>| {
        let start = covered.len();
        for &p in &c.pts {
            for dx in [-leaf, 0.0, leaf] {
                for dy in [-leaf, 0.0, leaf] {
                    for dz in [-leaf, 0.0, leaf] {
                        covered.push(key(p + Vec3::new(dx, dy, dz)));
                    }
                }
            }
        }
        // a short list is sorted again whole; a long one (a part with many branches) has this merged in
        if start < 4096 {
            covered.sort_unstable();
            covered.dedup();
            return;
        }
        let mut new = covered.split_off(start);
        new.sort_unstable();
        let old = std::mem::take(covered);
        covered.reserve(old.len() + new.len());
        let (mut i, mut j) = (0, 0);
        while i < old.len() || j < new.len() {
            let x = if j == new.len() || (i < old.len() && old[i] <= new[j]) {
                i += 1;
                old[i - 1]
            } else {
                j += 1;
                new[j - 1]
            };
            if covered.last() != Some(&x) {
                covered.push(x);
            }
        }
    };
    for s in seeds {
        if covered.binary_search(&key(s)).is_ok() {
            continue;
        }
        let Some(p0) = refine(a, b, s) else { continue };
        if !grown.contains(p0) {
            continue;
        }
        if curves.iter().any(|c| near_chain(c, p0, step)) {
            continue;
        }
        let forward = march(a, b, p0, 1.0, step, &grown)?;
        if forward.1 {
            let mut pts = forward.0;
            pts.pop(); // the closing point repeats the first
            curves.push(finish(a, b, pts, true, step));
            cover(curves.last().expect("just pushed"), &mut covered);
            continue;
        }
        let (mut back, closed) = march(a, b, p0, -1.0, step, &grown)?;
        // forward stopped short at a kink the way back got past: the way back is the whole loop if it
        // ran over forward's way, and joining the two would run round it twice (a forward that never
        // left its seed says nothing: back may only have come by the seed; forward's last few points
        // may have wandered off at the kink)
        let over = |p: &&Point3| back.iter().any(|q| q.distance(**p) <= 1.5 * step);
        if closed && forward.0.len() > 2 && 4 * forward.0.iter().filter(over).count() >= 3 * forward.0.len() {
            back.pop();
            curves.push(finish(a, b, back, true, step));
            cover(curves.last().expect("just pushed"), &mut covered);
            continue;
        }
        back.reverse();
        back.pop();
        back.extend(forward.0);
        if back.len() >= 2 {
            curves.push(finish(a, b, back, false, step));
            cover(curves.last().expect("just pushed"), &mut covered);
        }
    }
    Some(curves)
}

/// The branch where surfaces `a` and `b` meet from `x` to `p`, `p` a point where they touch (two
/// rolling-ball blends meet so, tangent to one face): no tracing from seeds finds its way into such a
/// point, so the chain walks from `x` towards it and ends there. An evaluation still takes a Newton
/// step onto both surfaces wherever they have not quite touched.
pub fn towards_touch(a: &Surface, b: &Surface, x: Point3, p: Point3, step: f64) -> Option<IntCurve> {
    let mut pts = vec![x];
    let mut cur = x;
    for _ in 0..4096 {
        let left = cur.distance(p);
        if left <= 1.5 * step {
            break;
        }
        let d = direction(a, b, cur)?;
        let d = if d.dot(p - cur) < 0.0 { -d } else { d };
        let h = step;
        let q = refine(a, b, cur + d * h)?;
        if (q - cur).dot(d) <= 0.5 * h || q.distance(p) >= left {
            return None;
        }
        pts.push(q);
        cur = q;
    }
    pts.push(p);
    if pts.len() < 2 {
        return None;
    }
    Some(finish(a, b, pts, false, step))
}

fn near_chain(c: &IntCurve, p: Point3, step: f64) -> bool {
    c.pts.iter().any(|q| q.distance(p) <= 1.5 * step)
}

/// A chain step turns through less than 30 degrees (past that, what follows the chain loses its way:
/// mass walks stretches turning less than that), down to TURN_LEAST of the step.
pub(crate) const TURN_COS: f64 = 0.866;
pub(crate) const TURN_LEAST: f64 = 1.0 / 1024.0;

/// Points from p0 in direction `sign`, until leaving `bounds` (true) or
/// closing back on p0 (returns closed = true).
fn march(
    a: &Surface,
    b: &Surface,
    p0: Point3,
    sign: f64,
    step: f64,
    bounds: &Box3,
) -> Option<(Vec<Point3>, bool)> {
    // a chain that crawls into a cone's apex or retraces itself finds no way on: its first return closes it
    let mut back = None;
    walk(a, b, p0, sign, step, bounds, &mut back).or_else(|| back.map(|pts| (pts, true)))
}

/// `march`, where a loop closes only having left and running the way it set out; `back` is the chain as
/// far as its first return within reach, whichever way.
fn walk(
    a: &Surface,
    b: &Surface,
    p0: Point3,
    sign: f64,
    step: f64,
    bounds: &Box3,
    back: &mut Option<Vec<Point3>>,
) -> Option<(Vec<Point3>, bool)> {
    let mut pts = vec![p0];
    let mut p = p0;
    // a helix winds many times its box's size
    let max = (64.0 * bounds.diagonal() / step) as usize + 4096;
    // how far the chain has come: a chain back within reach of p0 after this has closed
    // a wall made by a loft or a sweep may run all but tangent to what it meets; analytic pairs reach here
    // only where the closed form gave up, and a round's own surfaces keep the steps their springs were fitted to
    let wall = |s: &Surface| matches!(s, Surface::Procedural(p) if !matches!(p.kind(), "offset" | "blend" | "blend fence" | "variable blend" | "corner patch"));
    let round = |s: &Surface| matches!(s, Surface::Procedural(_)) && !wall(s);
    let tips = (wall(a) || wall(b)) && !round(a) && !round(b);
    let mut come = 0.0;
    let mut d0 = None;
    // the curve's direction at p, already asked of the surfaces when p was reached
    let mut here = direction(a, b, p);
    for k in 0..max {
        // the way the chain was going: where the surfaces touch on the curve their normals' cross
        // product vanishes and may turn over, and the chain carries on regardless
        let along = pts
            .len()
            .checked_sub(2)
            .and_then(|i| (p - pts[i]).normalize())
            .map(|u| u.get());
        let d = match (here, along) {
            (Some(d), Some(u)) if d.dot(u) * sign < 0.0 => -d * sign,
            (Some(d), _) => d * sign,
            (None, Some(u)) => u,
            (None, None) => return None,
        };
        let way = match here {
            Some(t) if t.dot(d) < 0.0 => -1.0,
            _ => 1.0,
        };
        // a predictor along the tangent, then back onto both surfaces in the plane across it;
        // a step that lands behind, short or far from its guess jumped (a corner of a surface's
        // projection, another sheet) or stalled, and is retried shorter; at a touch, it steps
        // over along the chain
        let ahead = |d: Vec3, h: f64, least: f64| {
            let guess = p + d * h;
            refine(a, b, guess).filter(|&q| (q - p).dot(d) > least * h && q.distance(guess) <= h)
        };
        // a step over which the curve turns sharply (past a place where the surfaces all but touch)
        // is retried shorter too, as far as TURN_LEAST; the step that turned stands if none shorter lands
        let mut turned = None;
        let mut h = step;
        let q = loop {
            if let Some(q) = ahead(d, h, 0.5) {
                let there = direction(a, b, q);
                let cos = there.map_or(1.0, |t| t.dot(d) * way);
                // (na x nb runs on round a tip: a step clean over one lands on it running backwards)
                if !tips
                    || h <= step * TURN_LEAST
                    || cos >= TURN_COS
                    || !(all_but_touch(a.gradient(p), b.gradient(p))
                        || all_but_touch(a.gradient(q), b.gradient(q)))
                {
                    break (q, there);
                }
                // running on as before just past here, the curve passes through a point where its
                // branches cross, and takes neither turn
                if turned.is_none()
                    && ahead(d, 2.0 * h, 0.5)
                        .and_then(|x| direction(a, b, x))
                        .is_some_and(|t| t.dot(d).abs() >= TURN_COS)
                {
                    break (q, there);
                }
                // a turn that a shorter step does not lessen is the normals' rounding, not the curve's
                if let Some((last, _, _)) = turned
                    && last > 0.0
                    && cos <= last
                {
                    let (_, q, there) = turned.expect("just seen");
                    break (q, there);
                }
                turned = Some((cos, q, there));
                h *= 0.5;
                continue;
            }
            if let Some((_, q, there)) = turned {
                break (q, there);
            }
            if h > step / 64.0 {
                h *= 0.5;
                continue;
            }
            if let Some(q) =
                along.and_then(|u| [1.0, 2.0, 4.0].into_iter().find_map(|m| ahead(u, step * m, 0.5)))
            {
                break (q, direction(a, b, q));
            }
            // a procedural surface may stop at its face's rim: the branch ends there (any part
            // of it back on the face is seeded and traced from there)
            if ends(a, p, d, h) || ends(b, p, d, h) {
                return Some((pts, false));
            }
            // last, any forward step at all: near a branch point (a cone's apex on the other
            // surface) every step makes little headway, and asking half a step stalled the chain
            if let Some(q) = [1.0, 0.25, 1.0 / 16.0, 1.0 / 64.0]
                .into_iter()
                .find_map(|m| ahead(d, step * m, 0.0))
            {
                break (q, direction(a, b, q));
            }
            return stalled(pts, p);
        };
        let (q, there) = q;
        come += q.distance(p);
        // (running the other way it is passing p0 on the far arm of a narrow tip, not closing)
        let first = *d0.get_or_insert(d);
        let near = q.distance(p0) <= 0.75 * step;
        if near && k > 2 && back.is_none() {
            let mut so_far = pts.clone();
            so_far.push(p0);
            *back = Some(so_far);
        }
        if near && come > 2.0 * step && d.dot(first) > 0.0 {
            pts.push(p0);
            return Some((pts, true));
        }
        pts.push(q);
        if !bounds.contains(q) {
            return Some((pts, false));
        }
        p = q;
        here = there;
    }
    stalled(pts, p)
}

thread_local! {
    /// Where branches have stalled, while a caller is noting them: a stall then ends its branch
    /// rather than the trace.
    static STALLS: std::cell::RefCell<Option<Vec<Point3>>> = const { std::cell::RefCell::new(None) };
}

fn stalled(pts: Vec<Point3>, p: Point3) -> Option<(Vec<Point3>, bool)> {
    STALLS.with(|s| {
        s.borrow_mut().as_mut().map(|v| {
            v.push(p);
            (pts, false)
        })
    })
}

/// `trace`, where a branch that stalls ends there, with the points it stalled at. The branch's other
/// stretches are seeded and traced by themselves.
pub fn trace_past_stalls(a: &Surface, b: &Surface, region: Box3) -> Option<(Vec<IntCurve>, Vec<Point3>)> {
    STALLS.with(|s| *s.borrow_mut() = Some(Vec::new()));
    let curves = trace(a, b, region);
    let stalls = STALLS.with(|s| s.borrow_mut().take()).unwrap_or_default();
    Some((curves?, stalls))
}

/// Whether the surfaces' normals at p are near parallel, as at a curve's narrow tip, but not so near
/// that na x nb is rounding (they touch along the curve: it says nothing of the way on).
pub(crate) fn all_but_touch(na: Vec3, nb: Vec3) -> bool {
    let (x, l) = (na.cross(nb).length(), na.length() * nb.length());
    x < 0.05 * l && x > 2e-4 * l
}

/// Whether a procedural surface stops at p going along d: a short step on lies off it by about
/// the step's length (a curve on it would stay within the step's square).
fn ends(s: &Surface, p: Point3, d: Vec3, h: f64) -> bool {
    let Surface::Procedural(pr) = s else { return false };
    let x = p + d * h;
    pr.point(pr.project(x)).distance(x) > 0.5 * h
}

pub(crate) fn finish(a: &Surface, b: &Surface, pts: Vec<Point3>, closed: bool, step: f64) -> IntCurve {
    // measured: how far each chord's midpoint sits from the curve
    let n = pts.len();
    let segs = if closed { n } else { n - 1 };
    let sag = (0..segs)
        .map(|i| {
            let m = pts[i].lerp(pts[(i + 1) % n], 0.5);
            refine(a, b, m).map_or(step, |q| q.distance(m))
        })
        .fold(0.0, f64::max);
    // directions along the chain (na x nb, turned to follow it)
    let tans = (0..n)
        .map(|i| {
            let along = if closed || (i > 0 && i + 1 < n) {
                pts[(i + 1) % n] - pts[(i + n - 1) % n]
            } else if i == 0 {
                pts[1] - pts[0]
            } else {
                pts[i] - pts[i - 1]
            };
            let d = direction(a, b, pts[i])
                .or_else(|| along.normalize().map(|u| u.get()))
                .unwrap_or(Vec3::ZERO);
            if d.dot(along) < 0.0 { -d } else { d }
        })
        .collect();
    IntCurve::new(a.clone(), b.clone(), pts, tans, closed, 1.5 * sag + tol::LINEAR)
}

impl crate::curve::CurveGeom for IntCurve {
    fn deriv(&self, t: f64) -> crate::curve::CurveDeriv {
        let (p, d1, d2) = self.derivs(t);
        crate::curve::CurveDeriv { p, d1, d2 }
    }
    fn eval(&self, t: f64) -> Point3 {
        IntCurve::eval(self, t)
    }
    fn domain(&self) -> crate::curve::Domain1 {
        let hi = self.domain_hi();
        crate::curve::Domain1 {
            lo: 0.0,
            hi,
            period: self.closed.then_some(hi),
        }
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        IntCurve::closest(self, p, range)
    }
    fn bbox(&self, range: Interval) -> Box3 {
        IntCurve::bbox(self, range)
    }
    fn transformed(&self, f: &Frame) -> Self {
        IntCurve::transformed(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::CurveGeom;
    use crate::surface::{Cone, Cylinder, Sphere, SurfaceGeom, Torus};
    use forge_math::{Frame, UnitVec3};

    fn on_both(c: &IntCurve) {
        for i in 0..=200 {
            let t = c.domain_hi() * i as f64 / 200.0;
            let p = CurveGeom::eval(c, t);
            assert!(
                c.a.distance(p) <= tol::LINEAR && c.b.distance(p) <= tol::LINEAR,
                "{t}"
            );
        }
    }

    #[test]
    fn sphere_through_cylinder_gives_two_loops() {
        let cyl = Surface::Cylinder(Cylinder {
            frame: Frame::WORLD,
            radius: 2.0,
        });
        let sph = Surface::Sphere(Sphere {
            frame: Frame::WORLD.with_origin(Point3::new(0.5, 0.0, 1.0)),
            radius: 4.0,
        });
        let region = Box3::from_points([Point3::new(-6.0, -6.0, -6.0), Point3::new(6.0, 6.0, 8.0)]);
        let cs = trace(&cyl, &sph, region).unwrap();
        assert_eq!(cs.len(), 2, "{}", cs.len());
        for c in &cs {
            assert!(c.closed);
            on_both(c);
        }
    }

    #[test]
    fn crossing_cylinders_and_a_cone_with_a_torus() {
        let a = Surface::Cylinder(Cylinder {
            frame: Frame::WORLD,
            radius: 2.0,
        });
        let x = Frame::from_z(Point3::new(0.0, 0.5, 0.0), UnitVec3::X);
        let b = Surface::Cylinder(Cylinder {
            frame: x,
            radius: 1.2,
        });
        let region = Box3::from_points([Point3::new(-5.0, -5.0, -5.0), Point3::new(5.0, 5.0, 5.0)]);
        let cs = trace(&a, &b, region).unwrap();
        assert_eq!(cs.len(), 2);
        cs.iter().for_each(on_both);
        let cone = Surface::Cone(
            Cone::new(Frame::WORLD.with_origin(Point3::new(4.5, 0.0, -3.0)), 0.5, 0.4).unwrap(),
        );
        let torus = Surface::Torus(Torus::new(Frame::WORLD, 5.0, 1.0).unwrap());
        let region = Box3::from_points([Point3::new(-7.0, -7.0, -2.0), Point3::new(7.0, 7.0, 2.0)]);
        let cs = trace(&cone, &torus, region).unwrap();
        assert!(!cs.is_empty());
        cs.iter().for_each(on_both);
        let _ = cs[0].a.project(Point3::ORIGIN);
    }
}
