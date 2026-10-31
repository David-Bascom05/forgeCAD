//! Curves defined as "where surface A meets surface B", for pairs with no
//! closed form (a sphere and a cylinder meet in a quartic space curve).
//!
//! The curve keeps both surfaces and a dense chain of points lying on both,
//! with the curve's direction at each. The parameter runs along the chain
//! (point i at t = i); an evaluation starts from the cubic through the chain
//! (Hermite, from those directions: within about 1e-6 mm) and takes one Newton
//! step onto both surfaces, so the chain only guides — the curve is exact.

use crate::curve::{CurveDeriv, Domain1, ProcCurve, ProceduralCurve};
use crate::surface::Surface;
use forge_math::{Box3, Frame, Interval, Point2, Point3, Vec3, tol};
use std::sync::{Arc, OnceLock};

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
            a,
            b,
            pts,
            tans,
            closed,
            sag,
            hints: OnceLock::new(),
        }
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

    fn chain_point(&self, t: f64) -> Point3 {
        let (i, j, f) = self.locate(t);
        self.pts[i].lerp(self.pts[j], f)
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
    pub(crate) fn hermite_point(&self, t: f64) -> Point3 {
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
        let q = newton_step(p, self.a.implicit_gradient(p), self.b.implicit_gradient(p));
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
    pub fn derivs(&self, t: f64) -> (Point3, Vec3, Vec3) {
        let (p, d1, d2) = self.hermite(t);
        (self.onto(p, t), d1, d2)
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
        for k in start..=end {
            let d = self.chain_point(k as f64).distance_sq(p);
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

/// An intersection curve given a smooth parameter: its chain parameter only joins once
/// differentiably at the chain points. In stretches over which the curve turns less than 30
/// degrees, u is the distance along the stretch's chord, the point solved exactly onto both
/// surfaces and the plane square to the chord at that distance: as smooth as the surfaces
/// within a stretch, and only the speed jumps where stretches meet.
#[derive(Debug)]
pub struct ChordCurve {
    pub ic: IntCurve,
    stretches: Vec<Stretch>,
    /// u at the end (a closed curve's period).
    len: f64,
}

#[derive(Debug, Clone)]
struct Stretch {
    u0: f64,
    anchor: Point3,
    dir: Vec3,
    /// (t, u) at the chain points over the stretch, increasing.
    knots: Vec<(f64, f64)>,
    /// A bound on |dP/du| (1 / the least cosine between the curve and the chord).
    speed: f64,
}

impl ChordCurve {
    /// None where the chain doubles back within a stretch.
    pub fn new(ic: IntCurve) -> Option<ChordCurve> {
        let n = ic.pts.len();
        let hi = ic.domain_hi();
        let mut stretches = Vec::new();
        let (mut t0, mut u0) = (0.0, 0.0);
        while t0 < hi {
            let tan0 = ic.tans[t0 as usize % n];
            let mut t1 = t0 + 1.0;
            while t1 < hi && ic.tans[t1 as usize % n].dot(tan0) >= 0.866 {
                t1 += 1.0;
            }
            let (anchor, end) = (ic.pts[t0 as usize % n], ic.pts[t1 as usize % n]);
            let dir = (end - anchor).normalize()?.get();
            let knots: Vec<(f64, f64)> = (t0 as usize..=t1 as usize)
                .map(|k| (k as f64, u0 + dir.dot(ic.pts[k % n] - anchor)))
                .collect();
            if knots.windows(2).any(|w| w[1].1 <= w[0].1) {
                return None;
            }
            let cos = (t0 as usize..=t1 as usize)
                .map(|k| ic.tans[k % n].dot(dir))
                .fold(1.0, f64::min);
            if cos <= 0.1 {
                return None;
            }
            u0 = knots[knots.len() - 1].1;
            stretches.push(Stretch {
                u0: knots[0].1,
                anchor,
                dir,
                knots,
                speed: 1.25 / cos,
            });
            t0 = t1;
        }
        Some(ChordCurve {
            ic,
            stretches,
            len: u0,
        })
    }

    pub fn curve(self) -> crate::Curve {
        crate::Curve::Procedural(ProcCurve(Arc::new(self)))
    }

    /// u moved by periods into [0, len) on a closed curve.
    fn wrap(&self, u: f64) -> f64 {
        if self.ic.closed && !(0.0..self.len).contains(&u) {
            u.rem_euclid(self.len)
        } else {
            u
        }
    }

    fn stretch(&self, u: f64) -> &Stretch {
        let i = self
            .stretches
            .partition_point(|s| s.u0 <= u)
            .clamp(1, self.stretches.len());
        &self.stretches[i - 1]
    }

    /// The chain parameter about where u is (between the knots, linearly), whole turns of a
    /// closed curve carried over.
    fn t_of(&self, u: f64) -> f64 {
        let w = self.wrap(u);
        let k = &self.stretch(w).knots;
        let i = k.partition_point(|x| x.1 <= w).clamp(1, k.len() - 1);
        let ((ta, ua), (tb, ub)) = (k[i - 1], k[i]);
        let t = ta + (tb - ta) * (w - ua) / (ub - ua);
        if self.ic.closed {
            t + self.ic.domain_hi() * ((u - w) / self.len).round()
        } else {
            t
        }
    }

    /// u of the curve's point at chain parameter t.
    fn u_of(&self, t: f64) -> f64 {
        let hi = self.ic.domain_hi();
        let w = if self.ic.closed {
            t.rem_euclid(hi)
        } else {
            t.clamp(0.0, hi)
        };
        let i = self
            .stretches
            .partition_point(|s| s.knots[0].0 <= w)
            .clamp(1, self.stretches.len());
        let st = &self.stretches[i - 1];
        let u = st.u0 + st.dir.dot(self.ic.eval(w) - st.anchor);
        if self.ic.closed {
            u + self.len * ((t - w) / hi).round()
        } else {
            u
        }
    }

    /// The point at u: from the chain's, Newton onto both surfaces and the stretch's plane.
    fn point(&self, u: f64) -> (Point3, &Stretch) {
        let w = self.wrap(u);
        let st = self.stretch(w);
        let s = w - st.u0;
        // the chain's cubic is within about 1e-6 of the curve: Newton doubles the digits per
        // step, so once a step is below 1e-8 of the size the next would be below rounding
        let mut p = self.ic.hermite_point(self.t_of(w));
        let scale = 1.0 + (p - Point3::ORIGIN).abs_max();
        for _ in 0..8 {
            let ((fa, ga), (fb, gb)) = (self.ic.a.implicit_gradient(p), self.ic.b.implicit_gradient(p));
            let fs = st.dir.dot(p - st.anchor) - s;
            // Cramer on rows ga, gb, dir: the step solves ga.d = -fa, gb.d = -fb, dir.d = -fs
            let (bc, ca, ab) = (gb.cross(st.dir), st.dir.cross(ga), ga.cross(gb));
            let det = ga.dot(bc);
            if det.abs() <= 1e-14 {
                break;
            }
            let step = (bc * -fa + ca * -fb + ab * -fs) / det;
            p += step;
            if step.abs_max() <= 1e-8 * scale {
                break;
            }
        }
        (p, st)
    }
}

impl ProceduralCurve for ChordCurve {
    fn point(&self, u: f64) -> Point3 {
        ChordCurve::point(self, u).0
    }
    fn deriv1(&self, u: f64) -> (Point3, Vec3) {
        let (p, st) = ChordCurve::point(self, u);
        let t = self.ic.a.gradient(p).cross(self.ic.b.gradient(p));
        (p, t / t.dot(st.dir))
    }
    fn deriv(&self, u: f64) -> CurveDeriv {
        let (p, st) = self.point(u);
        let (a, b) = (&self.ic.a, &self.ic.b);
        let (ga, gb) = (a.gradient(p), b.gradient(p));
        // along the curve at unit speed along dir: d1 = T / (T . dir), T = ga x gb
        let t = ga.cross(gb);
        let k = t.dot(st.dir);
        let d1 = t / k;
        let dt = a.gradient_deriv(p, d1).cross(gb) + ga.cross(b.gradient_deriv(p, d1));
        CurveDeriv {
            p,
            d1,
            d2: (dt - d1 * dt.dot(st.dir)) / k,
        }
    }
    fn exact_d1(&self) -> bool {
        true
    }
    fn domain(&self) -> Domain1 {
        Domain1 {
            lo: 0.0,
            hi: self.len,
            period: self.ic.closed.then_some(self.len),
        }
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        let (tl, th) = (
            self.t_of(range.lo).floor() - 1.0,
            self.t_of(range.hi).ceil() + 1.0,
        );
        let tr = if self.ic.closed {
            Interval::new(tl, th.max(tl))
        } else {
            Interval::new(tl.max(0.0), th.min(self.ic.domain_hi()))
        };
        let t = self.ic.closest(p, tr);
        // the chain parameter's answer moved to u
        // (round a whole closed curve, free to run past the ends, then put back in the range)
        let whole = self.ic.closed && range.width() >= self.len;
        let fit = |u: f64| if whole { u } else { u.clamp(range.lo, range.hi) };
        let mut u = self.u_of(t);
        if self.ic.closed {
            u += self.len * ((range.mid() - u) / self.len).round();
        }
        let mut u = fit(u);
        // then Newton on (P - p) . P' = 0
        for _ in 0..8 {
            let d = self.deriv(u);
            let g = (d.p - p).dot(d.d1);
            let h = d.d1.dot(d.d1) + (d.p - p).dot(d.d2);
            if h <= 0.0 {
                break;
            }
            let next = fit(u - g / h);
            let done = (next - u).abs() <= 1e-14 * (1.0 + u.abs());
            u = next;
            if done {
                break;
            }
        }
        if whole {
            range.lo + (u - range.lo).rem_euclid(self.len)
        } else {
            u
        }
    }
    fn bbox(&self, range: Interval) -> Box3 {
        let (tl, th) = (self.t_of(range.lo), self.t_of(range.hi));
        let b = self.ic.bbox(Interval::new(tl, th.max(tl)));
        b.hull(&Box3::from_points([
            self.point(range.lo).0,
            self.point(range.hi).0,
        ]))
    }
    fn transformed(&self, f: &Frame) -> ProcCurve {
        ProcCurve(Arc::new(ChordCurve {
            ic: self.ic.transformed(f),
            stretches: self
                .stretches
                .iter()
                .map(|s| Stretch {
                    anchor: f.point_to_world(s.anchor),
                    dir: f.vec_to_world(s.dir),
                    ..s.clone()
                })
                .collect(),
            len: self.len,
        }))
    }
    fn speed_bound(&self, range: Interval) -> f64 {
        if self.ic.closed && range.width() >= self.len {
            return self.stretches.iter().map(|s| s.speed).fold(0.0, f64::max);
        }
        let (lo, hi) = (self.wrap(range.lo), self.wrap(range.lo) + range.width());
        self.stretches
            .iter()
            .enumerate()
            .filter(|(i, s)| {
                let end = self.stretches.get(i + 1).map_or(self.len, |n| n.u0);
                (s.u0 <= hi && end >= lo) || (self.ic.closed && s.u0 + self.len <= hi)
            })
            .map(|(_, s)| s.speed)
            .fold(0.0, f64::max)
    }
    fn segments(&self, range: Interval, chord: f64) -> usize {
        // a chord of parameter length h sags at most h^2 max|P''| / 8
        let m = 16;
        let acc = (0..=m)
            .map(|i| {
                self.deriv(range.lo + range.width() * i as f64 / m as f64)
                    .d2
                    .length()
            })
            .fold(0.0, f64::max)
            * 1.5
            + f64::MIN_POSITIVE;
        ((range.width() / (8.0 * chord.max(tol::LINEAR) / acc).sqrt()).ceil() as usize).max(1)
    }
    fn kinks(&self, range: Interval) -> Vec<f64> {
        let mut out = Vec::new();
        let base = if self.ic.closed {
            self.len * (range.lo / self.len).floor()
        } else {
            0.0
        };
        let mut k = 0;
        loop {
            let s = &self.stretches[k % self.stretches.len()];
            let u = base + s.u0 + self.len * (k / self.stretches.len()) as f64;
            if u >= range.hi || (!self.ic.closed && k >= self.stretches.len()) {
                break;
            }
            if u > range.lo {
                out.push(u);
            }
            k += 1;
        }
        out
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
    let mut stack = vec![(region.center(), half * 3f64.sqrt())];
    while let Some((c, r)) = stack.pop() {
        if (boxed && (!reaches(&pa, c, r) || !reaches(&pb, c, r)))
            || a.implicit(c).abs() > r
            || b.implicit(c).abs() > r
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
    let grown = region.grow(step);
    let mut curves: Vec<IntCurve> = Vec::new();
    // cells (leaf-sized) around every traced point: a seed in one needs no refining, it lies
    // on a branch already traced (most seeds do, the octree leaves line every branch)
    let key = |p: Point3| {
        let f = |x: f64, lo: f64| (((x - lo) / leaf).floor() as i64 + 2) as u64 & 0x1f_ffff;
        (f(p.x, region.x.lo) << 42) | (f(p.y, region.y.lo) << 21) | f(p.z, region.z.lo)
    };
    let mut covered: Vec<u64> = Vec::new();
    let cover = |c: &IntCurve, covered: &mut Vec<u64>| {
        for &p in &c.pts {
            for dx in [-leaf, 0.0, leaf] {
                for dy in [-leaf, 0.0, leaf] {
                    for dz in [-leaf, 0.0, leaf] {
                        covered.push(key(p + Vec3::new(dx, dy, dz)));
                    }
                }
            }
        }
        covered.sort_unstable();
        covered.dedup();
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
        let mut back = march(a, b, p0, -1.0, step, &grown)?.0;
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

fn near_chain(c: &IntCurve, p: Point3, step: f64) -> bool {
    c.pts.iter().any(|q| q.distance(p) <= 1.5 * step)
}

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
    let mut pts = vec![p0];
    let mut p = p0;
    // a helix winds many times its box's size
    let max = (64.0 * bounds.diagonal() / step) as usize + 16;
    for k in 0..max {
        // the way the chain was going: where the surfaces touch on the curve their normals' cross
        // product vanishes and may turn over, and the chain carries on regardless
        let along = pts
            .len()
            .checked_sub(2)
            .and_then(|i| (p - pts[i]).normalize())
            .map(|u| u.get());
        let d = match (direction(a, b, p), along) {
            (Some(d), Some(u)) if d.dot(u) * sign < 0.0 => -d * sign,
            (Some(d), _) => d * sign,
            (None, Some(u)) => u,
            (None, None) => return None,
        };
        // a predictor along the tangent, then back onto both surfaces in the plane across it;
        // a step that lands behind, short or far from its guess jumped (a corner of a surface's
        // projection, another sheet) or stalled, and is retried shorter; at a touch, it steps
        // over along the chain
        let ahead = |d: Vec3, h: f64, least: f64| {
            let guess = p + d * h;
            refine(a, b, guess).filter(|&q| (q - p).dot(d) > least * h && q.distance(guess) <= h)
        };
        let mut h = step;
        let q = loop {
            if let Some(q) = ahead(d, h, 0.5) {
                break q;
            }
            if h > step / 64.0 {
                h *= 0.5;
                continue;
            }
            if let Some(q) =
                along.and_then(|u| [1.0, 2.0, 4.0].into_iter().find_map(|m| ahead(u, step * m, 0.5)))
            {
                break q;
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
                break q;
            }
            return None;
        };
        if k > 2 && q.distance(p0) <= 0.75 * step {
            pts.push(p0);
            return Some((pts, true));
        }
        pts.push(q);
        if !bounds.contains(q) {
            return Some((pts, false));
        }
        p = q;
    }
    None
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
    fn chord_curve_is_on_both_with_its_derivatives() {
        let cyl = Surface::Cylinder(Cylinder {
            frame: Frame::WORLD,
            radius: 2.0,
        });
        let sph = Surface::Sphere(Sphere {
            frame: Frame::WORLD.with_origin(Point3::new(0.5, 0.0, 1.0)),
            radius: 4.0,
        });
        let region = Box3::from_points([Point3::new(-6.0, -6.0, -6.0), Point3::new(6.0, 6.0, 8.0)]);
        for ic in trace(&cyl, &sph, region).unwrap() {
            let c = ChordCurve::new(ic).unwrap();
            let len = c.len;
            assert!(
                c.stretches.len() >= 8 && c.stretches.len() < c.ic.pts.len(),
                "{}",
                c.stretches.len()
            );
            for i in 0..=97 {
                let u = len * i as f64 / 97.0 + 0.123;
                let d = ProceduralCurve::deriv(&c, u);
                assert!(cyl.distance(d.p) <= 1e-12 && sph.distance(d.p) <= 1e-12, "{u}");
                let h = 1e-5;
                let (a, b) = (
                    ProceduralCurve::deriv(&c, u + h),
                    ProceduralCurve::deriv(&c, u - h),
                );
                if c.kinks(Interval::new(u - 2.0 * h, u + 2.0 * h)).is_empty() {
                    assert!(((a.p - b.p) / (2.0 * h) - d.d1).length() < 1e-8, "d1 at {u}");
                    assert!(((a.d1 - b.d1) / (2.0 * h) - d.d2).length() < 1e-5, "d2 at {u}");
                }
                let q = d.p + Vec3::new(0.01, -0.02, 0.015);
                let t = c.closest(q, Interval::new(0.0, len));
                let near = ProceduralCurve::deriv(&c, t);
                assert!(
                    (q - near.p).dot(near.d1).abs() < 1e-9,
                    "closest at {u}: t {t} res {} dist {} kinks {:?}",
                    (q - near.p).dot(near.d1),
                    near.p.distance(d.p),
                    c.kinks(Interval::new(u - 0.5, u + 0.5))
                );
                assert!(near.p.distance(d.p) < 0.1, "closest at {u}");
            }
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
