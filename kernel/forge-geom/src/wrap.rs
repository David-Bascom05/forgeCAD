//! Flat curves wrapped round a cylinder, exact by construction: a line, parabola or circle arc
//! drawn in (s, h), s the length round and h the height along the axis, laid on a cylinder of
//! radius `unroll` (so s turns the angle s / unroll) and carried out to radius `radius` along the
//! same rays. Text wrapped round a cylinder has these for its edges and rails, and ruled walls
//! between two of them; a hole across a sheet metal bend has a wrapped circle (an ellipse in
//! angle and height, its wall a right conoid; research/sheet-metal/), and a spline cut across one
//! its rational Bezier pieces (Copy, so a wrapped curve stays one).

use crate::curve::{CurveDeriv, CurveGeom, Domain1, Parabola, ProcCurve, ProceduralCurve};
use crate::intersect::{Contact, CurveHit, CurveSurface};
use crate::surface::Surface;
use forge_math::{Box3, Frame, Interval, Point2, Point3, Vec2, Vec3, dmath, tol};
use std::sync::Arc;

/// A curve in (s, h), by the fraction t along it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Flat {
    Line {
        a: Point2,
        b: Point2,
    },
    /// The parabola (in its frame's xy, read as (s, h)) over the parameter range.
    Parabola {
        curve: Parabola,
        range: Interval,
    },
    /// The circle arc from angle `start` through `sweep` (at most a whole turn).
    Arc {
        center: Point2,
        radius: f64,
        start: f64,
        sweep: f64,
    },
    /// A rational Bezier piece of degree `degree` (at most 3): points and weights.
    Bezier {
        degree: u8,
        pts: [Point2; 4],
        w: [f64; 4],
    },
}

impl Flat {
    /// A rational Bezier piece from homogeneous points [s w, h w, _, w] (a spline's, as
    /// `Spline::bezier_pieces` gives them); None past degree 3 or for a weight that is not positive.
    pub fn bezier(h: &[[f64; 4]]) -> Option<Flat> {
        let n = h.len().checked_sub(1)?;
        if !(1..=3).contains(&n) || h.iter().any(|q| q[3].is_nan() || q[3] <= 0.0) {
            return None;
        }
        let mut pts = [Point2::ORIGIN; 4];
        let mut w = [0.0; 4];
        for (i, q) in h.iter().enumerate() {
            pts[i] = Point2::new(q[0] / q[3], q[1] / q[3]);
            w[i] = q[3];
        }
        Some(Flat::Bezier {
            degree: n as u8,
            pts,
            w,
        })
    }

    /// Point and first two derivatives at fraction t.
    pub fn at(&self, t: f64) -> (Point2, Vec2, Vec2) {
        match *self {
            Flat::Line { a, b } => (a.lerp(b, t), b - a, Vec2::ZERO),
            Flat::Parabola { curve, range } => {
                let w = range.width();
                let d = curve.deriv(range.lo + w * t);
                (
                    Point2::new(d.p.x, d.p.y),
                    Vec2::new(d.d1.x, d.d1.y) * w,
                    Vec2::new(d.d2.x, d.d2.y) * (w * w),
                )
            }
            Flat::Arc {
                center,
                radius,
                start,
                sweep,
            } => {
                let (sn, cs) = dmath::sin_cos(start + sweep * t);
                (
                    Point2::new(center.x + radius * cs, center.y + radius * sn),
                    Vec2::new(-sn, cs) * (radius * sweep),
                    Vec2::new(-cs, -sn) * (radius * sweep * sweep),
                )
            }
            Flat::Bezier { degree, pts, w } => bezier_at(degree as usize, &pts, &w, t),
        }
    }

    /// Boxes of s and h over fractions t.
    pub fn bbox(&self, t: Interval) -> (Interval, Interval) {
        match *self {
            Flat::Line { a, b } => (t * (b.x - a.x) + a.x, t * (b.y - a.y) + a.y),
            Flat::Parabola { curve, range } => {
                let w = range.width();
                let b = curve.bbox(Interval::new(range.lo + w * t.lo, range.lo + w * t.hi));
                (b.x, b.y)
            }
            Flat::Arc {
                center,
                radius,
                start,
                sweep,
            } => {
                let a = t * sweep + start;
                (a.cos() * radius + center.x, a.sin() * radius + center.y)
            }
            Flat::Bezier { degree, pts, .. } => {
                let (mut s, mut h) = (
                    Interval::new(pts[0].x, pts[0].x),
                    Interval::new(pts[0].y, pts[0].y),
                );
                for p in &pts[1..=degree as usize] {
                    s = s.hull(&Interval::new(p.x, p.x));
                    h = h.hull(&Interval::new(p.y, p.y));
                }
                (s, h)
            }
        }
    }

    /// Coordinates s and h as polynomials in t, lowest power first; None for an arc or a Bezier.
    pub fn poly(&self) -> Option<[[f64; 3]; 2]> {
        Some([self.coord(0)?, self.coord(1)?])
    }

    /// Coordinate `c` (0: s, 1: h) as a polynomial in t.
    fn coord(&self, c: usize) -> Option<[f64; 3]> {
        let pick = |v: Point2| if c == 0 { v.x } else { v.y };
        match *self {
            Flat::Line { a, b } => Some([pick(a), pick(b) - pick(a), 0.0]),
            Flat::Parabola { curve, range } => {
                // o + x k u^2 + y u, u = lo + w t
                let f = &curve.frame;
                let at = |v: Vec3| if c == 0 { v.x } else { v.y };
                let (o, x, y) = (
                    at(f.origin.to_vec()),
                    at(f.x().get()) / (4.0 * curve.focal),
                    at(f.y().get()),
                );
                let (lo, w) = (range.lo, range.width());
                Some([o + y * lo + x * lo * lo, (y + 2.0 * x * lo) * w, x * w * w])
            }
            Flat::Arc { .. } | Flat::Bezier { .. } => None,
        }
    }

    /// The fractions over `range` where coordinate `c` is `v`; on an arc, where the cos (s) or
    /// sin (h) of its angle has a value: twice a turn, once where it touches.
    pub(crate) fn level(&self, c: usize, v: f64, range: Interval) -> Vec<forge_math::poly::Root> {
        if let Flat::Bezier { degree, pts, w } = *self {
            // sum of w_i (c_i - v) B_i(t), a polynomial of the piece's degree, in powers of t
            let n = degree as usize;
            let mut b = [0.0; 4];
            for i in 0..=n {
                b[i] = w[i] * (if c == 0 { pts[i].x } else { pts[i].y } - v);
            }
            return roots(&bernstein_to_power(n, &b), range);
        }
        let Flat::Arc {
            center,
            radius,
            start,
            sweep,
        } = *self
        else {
            let Some(mut p) = self.coord(c) else {
                return vec![];
            };
            p[0] -= v;
            return roots(&p, range);
        };
        let q = (v - if c == 0 { center.x } else { center.y }) / radius;
        let touching = (q.abs() - 1.0).abs() <= 4.0 * f64::EPSILON;
        if q.abs() > 1.0 && !touching {
            return vec![];
        }
        // s: a = -+acos q; h: a = pi/2 -+ acos q
        let a = dmath::acos_clamped(q);
        let (b0, b1) = if c == 0 {
            (-a, a)
        } else {
            (dmath::FRAC_PI_2 - a, dmath::FRAC_PI_2 + a)
        };
        let turn = dmath::TAU / sweep.abs();
        let mut out = Vec::new();
        for b in if touching { vec![b0] } else { vec![b0, b1] } {
            let t0 = (b - start) / sweep;
            let k0 = ((range.lo - t0) / turn).ceil() as i64;
            let k1 = ((range.hi - t0) / turn).floor() as i64;
            out.extend((k0..=k1).map(|k| forge_math::poly::Root {
                x: t0 + turn * k as f64,
                touching,
            }));
        }
        out.sort_by(|a, b| a.x.total_cmp(&b.x));
        out
    }

    /// The curve's implicit value along the curve whose coordinates are the polynomials `g`
    /// (as `poly` gives them): zero where the two meet. A Bezier piece has none (NaN): the wall
    /// meets refuse it before asking.
    pub fn meet_poly(&self, g: [[f64; 3]; 2]) -> [f64; 5] {
        let mut out = [0.0; 5];
        match *self {
            Flat::Line { a, b } => {
                // (q - a) x (b - a)
                let (ds, dh) = (b.x - a.x, b.y - a.y);
                for i in 0..3 {
                    out[i] = g[0][i] * dh - g[1][i] * ds;
                }
                out[0] -= a.x * dh - a.y * ds;
            }
            Flat::Parabola { curve, .. } => {
                // X - k Y^2 in the parabola's frame
                let f = &curve.frame;
                let local = |e: Vec3| {
                    let mut p = [0.0; 3];
                    for i in 0..3 {
                        p[i] = g[0][i] * e.x + g[1][i] * e.y;
                    }
                    p[0] -= f.origin.x * e.x + f.origin.y * e.y;
                    p
                };
                let (x, y) = (local(f.x().get()), local(f.y().get()));
                let k = 1.0 / (4.0 * curve.focal);
                for i in 0..3 {
                    out[i] += x[i];
                    for j in 0..3 {
                        out[i + j] -= k * y[i] * y[j];
                    }
                }
            }
            Flat::Arc { center, radius, .. } => {
                // (s - cx)^2 + (h - cy)^2 - r^2
                let (mut x, mut y) = (g[0], g[1]);
                x[0] -= center.x;
                y[0] -= center.y;
                for i in 0..3 {
                    for j in 0..3 {
                        out[i + j] += x[i] * x[j] + y[i] * y[j];
                    }
                }
                out[0] -= radius * radius;
            }
            // no implicit: the meets that need one refuse a Bezier before asking (`crossings`)
            Flat::Bezier { .. } => out = [f64::NAN; 5],
        }
        out
    }

    /// The fraction t of a point on the curve.
    pub fn param(&self, q: Point2) -> f64 {
        match *self {
            Flat::Line { a, b } => (q - a).dot(b - a) / (b - a).length_sq(),
            Flat::Parabola { curve, range } => {
                let f = &curve.frame;
                let y = f.y().get();
                let u = (q.x - f.origin.x) * y.x + (q.y - f.origin.y) * y.y;
                (u - range.lo) / range.width()
            }
            // the turn nearest the arc's middle
            Flat::Arc {
                center, start, sweep, ..
            } => {
                let mid = start + 0.5 * sweep;
                let a = dmath::atan2(q.y - center.y, q.x - center.x);
                (a + dmath::TAU * ((mid - a) / dmath::TAU).round() - start) / sweep
            }
            Flat::Bezier { .. } => bezier_param(self, q),
        }
    }

    /// The fraction of a point on the curve if it lies within `range`: on an arc, at any turn.
    pub(crate) fn param_in(&self, q: Point2, range: Interval) -> Option<f64> {
        let t = self.param(q);
        let turn = match *self {
            Flat::Arc { sweep, .. } => dmath::TAU / sweep.abs(),
            _ => return range.contains(t).then_some(t),
        };
        [t, t - turn, t + turn].into_iter().find(|&t| range.contains(t))
    }

    /// How far past each end the curve can be carried and stay one turn at most.
    pub fn reach(&self) -> f64 {
        match *self {
            Flat::Line { .. } | Flat::Parabola { .. } => 0.25,
            Flat::Arc { sweep, .. } => (0.5 * (dmath::TAU / sweep.abs() - 1.0)).clamp(0.0, 0.25),
            // a piece of a spline is not carried past its ends
            Flat::Bezier { .. } => 0.0,
        }
    }

    /// Bounds on |d/dt| and |d2/dt2| over fractions t (the speed is linear in t, or constant).
    fn bounds(&self, t: Interval) -> (f64, f64) {
        if let Flat::Bezier { degree, pts, w } = *self {
            return bezier_bounds(degree as usize, &pts, &w);
        }
        let (_, v0, a) = self.at(t.lo);
        let (_, v1, _) = self.at(t.hi);
        (v0.length().max(v1.length()), a.length())
    }
}

/// A rational Bezier piece's point and first two derivatives at t: the homogeneous sums and the
/// quotient rule.
fn bezier_at(n: usize, pts: &[Point2; 4], w: &[f64; 4], t: f64) -> (Point2, Vec2, Vec2) {
    // Bernstein values and their first two derivatives
    let u = 1.0 - t;
    let (b, d1, d2): ([f64; 4], [f64; 4], [f64; 4]) = match n {
        1 => ([u, t, 0.0, 0.0], [-1.0, 1.0, 0.0, 0.0], [0.0; 4]),
        2 => (
            [u * u, 2.0 * u * t, t * t, 0.0],
            [-2.0 * u, 2.0 * (u - t), 2.0 * t, 0.0],
            [2.0, -4.0, 2.0, 0.0],
        ),
        _ => (
            [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t],
            [
                -3.0 * u * u,
                3.0 * u * (u - 2.0 * t),
                3.0 * t * (2.0 * u - t),
                3.0 * t * t,
            ],
            [6.0 * u, 6.0 * (3.0 * t - 2.0), 6.0 * (1.0 - 3.0 * t), 6.0 * t],
        ),
    };
    let sum = |c: &[f64; 4]| {
        let (mut p, mut q) = (Vec2::ZERO, 0.0);
        for i in 0..=n {
            p += pts[i].to_vec() * (w[i] * c[i]);
            q += w[i] * c[i];
        }
        (p, q)
    };
    let ((p0, w0), (p1, w1), (p2, w2)) = (sum(&b), sum(&d1), sum(&d2));
    let c = p0 * (1.0 / w0);
    let c1 = (p1 - c * w1) * (1.0 / w0);
    let c2 = (p2 - c1 * (2.0 * w1) - c * w2) * (1.0 / w0);
    (Point2::new(c.x, c.y), c1, c2)
}

/// Bernstein coefficients of degree n as powers of t, lowest first.
fn bernstein_to_power(n: usize, b: &[f64; 4]) -> [f64; 4] {
    match n {
        1 => [b[0], b[1] - b[0], 0.0, 0.0],
        2 => [b[0], 2.0 * (b[1] - b[0]), b[0] - 2.0 * b[1] + b[2], 0.0],
        _ => [
            b[0],
            3.0 * (b[1] - b[0]),
            3.0 * (b[0] - 2.0 * b[1] + b[2]),
            b[3] - b[0] + 3.0 * (b[1] - b[2]),
        ],
    }
}

/// Bounds on |C'| and |C''| over the whole piece. Polynomial: n and n(n - 1) times the largest
/// first and second differences of the points. Rational: C' = sum B_i' w_i (p_i - C) / W, with
/// |p_i - C| at most the points' spread D, sum |B_i'| at most 2n and W at least the least weight,
/// so |C'| <= 2n D r (r the weights' ratio); likewise |C''| <= r (4n(n - 1) D + 4n |C'|).
fn bezier_bounds(n: usize, pts: &[Point2; 4], w: &[f64; 4]) -> (f64, f64) {
    let p = &pts[..=n];
    let (lo, hi) = w[..=n]
        .iter()
        .fold((f64::INFINITY, 0.0f64), |(a, b), &x| (a.min(x), b.max(x)));
    let nf = n as f64;
    if lo == hi {
        let d1 = p.windows(2).map(|q| (q[1] - q[0]).length()).fold(0.0, f64::max);
        let d2 = p
            .windows(3)
            .map(|q| (q[2] - q[1] - (q[1] - q[0])).length())
            .fold(0.0, f64::max);
        return (nf * d1, nf * (nf - 1.0) * d2);
    }
    let spread = p
        .iter()
        .flat_map(|a| p.iter().map(move |b| a.distance(*b)))
        .fold(0.0, f64::max);
    let r = hi / lo;
    let v = 2.0 * nf * spread * r;
    (v, r * (4.0 * nf * (nf - 1.0) * spread + 4.0 * nf * v))
}

/// The fraction of a point on a Bezier flat: the nearest of 33 samples, then Newton.
fn bezier_param(f: &Flat, q: Point2) -> f64 {
    let mut best = (f64::INFINITY, 0.0);
    for i in 0..=32 {
        let t = i as f64 / 32.0;
        let d = f.at(t).0.distance(q);
        if d < best.0 {
            best = (d, t);
        }
    }
    let mut t = best.1;
    for _ in 0..16 {
        let (p, d1, d2) = f.at(t);
        let e = p - q;
        let (g, h) = (e.dot(d1), d1.length_sq() + e.dot(d2));
        let h = if h > 0.0 { h } else { d1.length_sq() };
        let next = (t - g / h).clamp(0.0, 1.0);
        if next == t {
            break;
        }
        t = next;
    }
    t
}

/// A flat curve wrapped round the axis frame's z, its s = 0 on the frame's x.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wrap {
    pub axis: Frame,
    pub radius: f64,
    pub unroll: f64,
    pub flat: Flat,
}

impl Wrap {
    /// Point and first two derivatives at fraction t.
    pub fn at(&self, t: f64) -> (Point3, Vec3, Vec3) {
        let (q, d1, d2) = self.flat.at(t);
        let (sn, cs) = dmath::sin_cos(q.x / self.unroll);
        let (x, y, z) = (self.axis.x().get(), self.axis.y().get(), self.axis.z().get());
        let out = x * cs + y * sn;
        let round = y * cs - x * sn;
        let (r, w) = (self.radius, d1.x / self.unroll);
        (
            self.axis.origin + out * r + z * q.y,
            round * (r * w) + z * d1.y,
            out * (-r * w * w) + round * (r * d2.x / self.unroll) + z * d2.y,
        )
    }

    pub fn bbox(&self, t: Interval) -> Box3 {
        let (s, h) = self.flat.bbox(t);
        let a = s * (1.0 / self.unroll);
        let (cx, cy) = (a.cos() * self.radius, a.sin() * self.radius);
        let f = &self.axis;
        let (x, y, z, o) = (f.x().get(), f.y().get(), f.z().get(), f.origin);
        Box3 {
            x: cx * x.x + cy * y.x + h * z.x + o.x,
            y: cx * x.y + cy * y.y + h * z.y + o.y,
            z: cx * x.z + cy * y.z + h * z.z + o.z,
        }
        .grow(tol::LINEAR)
    }

    pub fn transformed(&self, f: &Frame) -> Wrap {
        Wrap {
            axis: f.compose(&self.axis),
            ..*self
        }
    }

    /// The same flat curve carried to another radius.
    pub fn at_radius(&self, radius: f64) -> Wrap {
        Wrap { radius, ..*self }
    }

    pub fn curve(self) -> crate::Curve {
        crate::Curve::Procedural(ProcCurve(Arc::new(self)))
    }

    /// Whether `f` has the same axis (origin, x and z) as this curve's.
    pub(crate) fn coaxial(&self, f: &Frame) -> bool {
        let a = &self.axis;
        a.origin.distance(f.origin) <= tol::LINEAR
            && (a.x().get() - f.x().get()).length() <= tol::ANGULAR
            && (a.z().get() - f.z().get()).length() <= tol::ANGULAR
    }

    fn bounds(&self, t: Interval) -> (f64, f64) {
        let (v, a) = self.flat.bounds(t);
        let k = self.radius / self.unroll;
        (v * (k + 1.0), k * v * v / self.unroll + a * (k + 1.0))
    }
}

/// Roots of a polynomial over `range`; none for one that is zero throughout.
pub(crate) fn roots(p: &[f64], range: Interval) -> Vec<forge_math::poly::Root> {
    let n = p.iter().rposition(|c| *c != 0.0).map_or(0, |n| n + 1);
    if n <= 1 {
        return vec![];
    }
    forge_math::poly::real_roots(&p[..n], range.lo, range.hi)
}

/// Hits at fractions `ts` of a wrapped curve, in order.
fn hits(w: &Wrap, ts: impl IntoIterator<Item = (f64, bool)>) -> CurveSurface {
    let mut out: Vec<CurveHit> = ts
        .into_iter()
        .map(|(t, touching)| CurveHit {
            t,
            point: w.at(t).0,
            contact: if touching {
                Contact::Touching
            } else {
                Contact::Crossing
            },
        })
        .collect();
    out.sort_by(|a, b| a.t.total_cmp(&b.t));
    CurveSurface::Hits(out)
}

/// Where a wrapped curve meets a cylinder about its axis (on it, or nowhere) or a plane square
/// to its axis or along it (where the flat curve's height, or its turn, has a value), in closed
/// form; None for any other surface.
pub fn wrap_surface(w: &Wrap, range: Interval, s: &Surface) -> Option<CurveSurface> {
    let z = w.axis.z();
    match s {
        Surface::Cylinder(c) => {
            let off = c.frame.origin - w.axis.origin;
            if c.frame.z().cross(z.get()).length() > tol::ANGULAR
                || (off - z * z.dot(off)).length() > tol::LINEAR
            {
                return None;
            }
            Some(if (c.radius - w.radius).abs() <= tol::LINEAR {
                CurveSurface::OnSurface
            } else {
                CurveSurface::Hits(vec![])
            })
        }
        Surface::Plane(p) => {
            let n = p.frame.z().get();
            let n = (n.dot(w.axis.x().get()), n.dot(w.axis.y().get()), n.dot(z.get()));
            let c0 = p.frame.z().dot(w.axis.origin - p.frame.origin);
            if (n.0 * n.0 + n.1 * n.1).sqrt() <= tol::ANGULAR {
                // square to the axis: at height -c0 / nz
                let ts = w.flat.level(1, -c0 / n.2, range);
                return Some(hits(w, ts.into_iter().map(|r| (r.x, r.touching))));
            }
            if n.2.abs() > tol::ANGULAR {
                return None;
            }
            // along the axis: r m cos(theta - phi) = -c0
            let m = (n.0 * n.0 + n.1 * n.1).sqrt();
            let c = -c0 / (w.radius * m);
            if c.abs() > 1.0 + tol::ANGULAR {
                return Some(CurveSurface::Hits(vec![]));
            }
            let phi = dmath::atan2(n.1, n.0);
            let a = dmath::acos_clamped(c);
            let touching = (c.abs() - 1.0).abs() <= tol::ANGULAR;
            let turn = dmath::TAU * w.unroll;
            let (sr, _) = w.flat.bbox(range);
            let mut ts = Vec::new();
            for theta in if touching {
                vec![phi]
            } else {
                vec![phi - a, phi + a]
            } {
                let s0 = theta * w.unroll;
                let k0 = ((sr.lo - s0) / turn).floor() as i64;
                let k1 = ((sr.hi - s0) / turn).ceil() as i64;
                for k in k0..=k1 {
                    for r in w.flat.level(0, s0 + turn * k as f64, range) {
                        ts.push((r.x, touching || r.touching));
                    }
                }
            }
            Some(hits(w, ts))
        }
        _ => None,
    }
}

impl ProceduralCurve for Wrap {
    fn exact_d1(&self) -> bool {
        true
    }
    fn deriv(&self, t: f64) -> CurveDeriv {
        let (p, d1, d2) = self.at(t);
        CurveDeriv { p, d1, d2 }
    }
    fn domain(&self) -> Domain1 {
        // carried on past each end (a quarter, or less on an arc of most of a turn), so curves
        // cut from walls cross their ends
        let e = self.flat.reach();
        Domain1 {
            lo: -e,
            hi: 1.0 + e,
            period: None,
        }
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        const N: usize = 32;
        let mut best = (f64::INFINITY, range.lo);
        for i in 0..=N {
            let t = range.lo + range.width() * i as f64 / N as f64;
            let d = self.at(t).0.distance_sq(p);
            if d < best.0 {
                best = (d, t);
            }
        }
        // Newton on (C - p).C' = 0, kept inside the range while it gets closer
        let (mut t, mut f) = (best.1, best.0);
        for _ in 0..16 {
            let (q, d1, d2) = self.at(t);
            let e = q - p;
            let (g, h) = (e.dot(d1), d1.length_sq() + e.dot(d2));
            let h = if h > 0.0 { h } else { d1.length_sq() };
            let next = (t - g / h).clamp(range.lo, range.hi);
            if next == t {
                break;
            }
            let fnext = self.at(next).0.distance_sq(p);
            if fnext > f {
                break;
            }
            (t, f) = (next, fnext);
        }
        t
    }
    fn bbox(&self, range: Interval) -> Box3 {
        Wrap::bbox(self, range)
    }
    fn transformed(&self, f: &Frame) -> ProcCurve {
        ProcCurve(Arc::new(Wrap::transformed(self, f)))
    }
    fn speed_bound(&self, range: Interval) -> f64 {
        self.bounds(range).0 * (1.0 + 1e-12)
    }
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
    fn segments(&self, range: Interval, chord: f64) -> usize {
        // equal steps h keep each chord within |C''| h^2 / 8 of the curve
        let n = range.width() * (self.bounds(range).1 / (8.0 * chord.max(tol::LINEAR))).sqrt();
        (n.ceil() as usize).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::rand_frame;
    use forge_math::rng::Rng;

    fn random_wrap(r: &mut Rng) -> Wrap {
        let flat = if r.bool() {
            Flat::Line {
                a: Point2::new(r.range(-5.0, 5.0), r.range(-5.0, 5.0)),
                b: Point2::new(r.range(-5.0, 5.0), r.range(-5.0, 5.0)),
            }
        } else {
            let lo = r.range(-3.0, 1.0);
            Flat::Parabola {
                curve: Parabola {
                    frame: Frame::WORLD
                        .rotated_about_z(r.range(0.0, 6.0))
                        .with_origin(Point3::new(r.range(-3.0, 3.0), r.range(-3.0, 3.0), 0.0)),
                    focal: r.range(0.2, 3.0),
                },
                range: Interval::new(lo, lo + r.range(0.5, 3.0)),
            }
        };
        let unroll = r.range(4.0, 12.0);
        Wrap {
            axis: rand_frame(r),
            radius: unroll + r.range(-2.0, 2.0),
            unroll,
            flat,
        }
    }

    #[test]
    fn derivatives_match_finite_differences() {
        let mut r = Rng::new(611);
        for _ in 0..300 {
            let w = random_wrap(&mut r);
            let t = r.range(0.0, 1.0);
            let h = 1e-6;
            let (_, d1, d2) = w.at(t);
            let close =
                |a: Vec3, b: Vec3| assert!((a - b).length() < 1e-5 * (1.0 + b.length()), "{a:?} {b:?}");
            close((w.at(t + h).0 - w.at(t - h).0) / (2.0 * h), d1);
            close((w.at(t + h).1 - w.at(t - h).1) / (2.0 * h), d2);
        }
    }

    #[test]
    fn lies_on_its_cylinder_and_boxes_hold_it() {
        let mut r = Rng::new(612);
        for _ in 0..200 {
            let w = random_wrap(&mut r);
            let (a, b) = (r.range(0.0, 1.0), r.range(0.0, 1.0));
            let range = Interval::new(a.min(b), a.max(b));
            let bx = w.bbox(range);
            let speed = w.speed_bound(range);
            for i in 0..=20 {
                let t = range.lo + range.width() * i as f64 / 20.0;
                let (p, d1, _) = w.at(t);
                let l = w.axis.point_to_local(p);
                assert!(((l.x * l.x + l.y * l.y).sqrt() - w.radius.abs()).abs() < 1e-9);
                assert!(bx.contains(p));
                assert!(d1.length() <= speed);
            }
            let t = r.range(0.0, 1.0);
            let p = w.at(t).0;
            assert!(w.at(w.closest(p, Interval::new(0.0, 1.0))).0.distance(p) < 1e-9);
        }
    }
}
