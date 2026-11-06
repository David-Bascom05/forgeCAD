//! The drafting curve: lines, rays, construction lines, circles, arcs, ellipses and elliptical
//! arcs, polylines of line and arc segments, and NURBS splines, in the XY plane; and evaluating
//! them.
//!
//! Every curve has a parameter running over its `domain`:
//! - a line, 0 at its start to 1 at its end;
//! - a ray or construction line, the distance from its base point along its (unit) direction;
//! - a circle, the angle (counter-clockwise from +x), 0..2pi;
//! - an arc or elliptical arc, the angle travelled from its start in its direction, 0..|sweep|
//!   (for an ellipse, the eccentric angle);
//! - a polyline, segment k over k..k + 1 (an arc segment by the fraction of its sweep);
//! - a spline, its knots.
//!
//! Lines, arcs, ellipses and an open polyline's end segments also have an extension past their
//! ends: parameters outside the domain (`point_ext`).

use super::geom::{Affine, Bbox, PI, TAU, Tol, V2, solve2, v2, wrap};
use super::nurbs::Nurbs;
use forge_math::dmath;
use std::sync::OnceLock;

/// A circular arc: centre `c`, radius `r`, from angle `start` through the signed `sweep`
/// (positive counter-clockwise, |sweep| <= 2pi).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Arc {
    pub c: V2,
    pub r: f64,
    pub start: f64,
    pub sweep: f64,
}

/// An ellipse or elliptical arc: centre `c`, semi-major axis `major`, the minor axis `ratio` times
/// as long a quarter turn counter-clockwise from it; from eccentric angle `start` through the
/// signed `sweep` (2pi for the whole ellipse).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ellipse {
    pub c: V2,
    pub major: V2,
    pub ratio: f64,
    pub start: f64,
    pub sweep: f64,
}

/// A polyline: vertices, the bulge after each (tan of a quarter of the arc's signed sweep; 0
/// straight), and closed or not. `widths` (start, end per vertex) is empty or one per vertex.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Poly {
    pub pts: Vec<V2>,
    pub bulges: Vec<f64>,
    pub widths: Vec<[f64; 2]>,
    pub const_width: f64,
    pub closed: bool,
}

/// A polyline segment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg {
    Line(V2, V2),
    Arc(Arc),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Curve {
    Line {
        a: V2,
        b: V2,
    },
    /// From `at` along the unit `dir`.
    Ray {
        at: V2,
        dir: V2,
    },
    /// Through `at` both ways along the unit `dir`.
    Xline {
        at: V2,
        dir: V2,
    },
    Circle {
        c: V2,
        r: f64,
    },
    Arc(Arc),
    Ellipse(Ellipse),
    Poly(Poly),
    /// A DWG SPLINE's own NURBS.
    Spline(Nurbs),
    /// The exact offset of an ellipse or spline (which no entity holds): fitted to a SPLINE only
    /// when written.
    Offset(Offset),
}

/// `base(t) + d * n(t)`, n the unit normal a quarter turn counter-clockwise of the tangent (so a
/// positive `d` is on the left). The parameter is the base's.
#[derive(Clone, Debug, PartialEq)]
pub struct Offset {
    pub base: Box<Curve>,
    pub d: f64,
}

impl Offset {
    pub fn point(&self, t: f64) -> V2 {
        let (p, d1, _) = self.base.derivs(t);
        p + d1.unit_or_zero().perp() * self.d
    }
    /// B'(1 - d k), k the base's signed curvature.
    pub fn deriv(&self, t: f64) -> V2 {
        let (_, d1, d2) = self.base.derivs(t);
        let l2 = d1.len2();
        if l2 == 0.0 {
            return d1;
        }
        let k = d1.cross(d2) / (l2 * l2.sqrt());
        d1 * (1.0 - self.d * k)
    }
    /// Sample parameters dense enough to separate roots.
    pub fn samples(&self) -> Vec<f64> {
        self.base.samples()
    }
}

/// The 10-point Gauss-Legendre rule.
fn gl10() -> &'static [(f64, f64)] {
    static R: OnceLock<Vec<(f64, f64)>> = OnceLock::new();
    R.get_or_init(|| forge_math::poly::gauss_legendre(10))
}

/// The integral of `f` over `[a, b]`, adaptive Gauss-Legendre to a relative `tol`.
pub(crate) fn integrate(f: &dyn Fn(f64) -> f64, a: f64, b: f64, tol: f64) -> f64 {
    fn gl(f: &dyn Fn(f64) -> f64, a: f64, b: f64) -> f64 {
        let (h, m) = (0.5 * (b - a), 0.5 * (a + b));
        gl10().iter().map(|&(x, w)| w * f(m + h * x)).sum::<f64>() * h
    }
    fn rec(f: &dyn Fn(f64) -> f64, a: f64, b: f64, whole: f64, tol: f64, depth: u32) -> f64 {
        let m = 0.5 * (a + b);
        let (l, r) = (gl(f, a, m), gl(f, m, b));
        if depth == 0 || (l + r - whole).abs() <= tol * (l + r).abs().max(1e-300) {
            l + r
        } else {
            rec(f, a, m, l, tol, depth - 1) + rec(f, m, b, r, tol, depth - 1)
        }
    }
    if b <= a {
        return 0.0;
    }
    rec(f, a, b, gl(f, a, b), tol, 24)
}

impl Arc {
    pub fn dir(&self) -> f64 {
        if self.sweep < 0.0 { -1.0 } else { 1.0 }
    }
    pub fn angle_at(&self, t: f64) -> f64 {
        self.start + self.dir() * t
    }
    pub fn end_angle(&self) -> f64 {
        self.start + self.sweep
    }
    pub fn point(&self, t: f64) -> V2 {
        self.c + V2::polar(self.angle_at(t), self.r)
    }
    pub fn deriv(&self, t: f64) -> V2 {
        V2::polar(self.angle_at(t), self.r).perp() * self.dir()
    }
    pub fn start_pt(&self) -> V2 {
        self.c + V2::polar(self.start, self.r)
    }
    pub fn end_pt(&self) -> V2 {
        self.c + V2::polar(self.end_angle(), self.r)
    }
    pub fn len(&self) -> f64 {
        self.sweep.abs()
    }
    pub fn is_empty(&self) -> bool {
        self.sweep == 0.0
    }
    /// The parameter (angle travelled from the start) of an angle, in [0, 2pi).
    pub fn param_of_angle(&self, a: f64) -> f64 {
        wrap(self.dir() * (a - self.start))
    }
    /// The arc from three points (start, through, end); None when they are in a line.
    pub fn from_3_points(a: V2, m: V2, b: V2) -> Option<Arc> {
        let (d1, d2) = (m - a, b - a);
        let det = 2.0 * d1.cross(d2);
        let s = d1.len().max(d2.len());
        if det.abs() <= 1e-12 * s * s {
            return None;
        }
        let c = a + v2(
            d2.y * d1.len2() - d1.y * d2.len2(),
            d1.x * d2.len2() - d2.x * d1.len2(),
        ) / det;
        let r = c.dist(a);
        let (sa, ea) = ((a - c).angle(), (b - c).angle());
        let ccw = d1.cross(d2) < 0.0;
        // a, m, b counter-clockwise when m is to the left going a to b ... the turn from a to m to b
        let sweep = if (m - a).cross(b - m) > 0.0 {
            // a left turn: counter-clockwise
            wrap(ea - sa)
        } else {
            -wrap(sa - ea)
        };
        let _ = ccw;
        Some(Arc {
            c,
            r,
            start: sa,
            sweep,
        })
    }
    /// The arc of a polyline segment from `a` to `b` with `bulge`.
    pub fn from_bulge(a: V2, b: V2, bulge: f64) -> Option<Arc> {
        if bulge == 0.0 || a == b {
            return None;
        }
        let d = b - a;
        let sweep = 4.0 * dmath::atan(bulge);
        let c = a.mid(b) + d.perp() * ((1.0 - bulge * bulge) / (4.0 * bulge));
        Some(Arc {
            c,
            r: c.dist(a),
            start: (a - c).angle(),
            sweep,
        })
    }
    /// tan(sweep / 4).
    pub fn bulge(&self) -> f64 {
        let (s, c) = dmath::sin_cos(self.sweep / 4.0);
        s / c
    }
    pub fn reversed(&self) -> Arc {
        Arc {
            start: self.end_angle(),
            sweep: -self.sweep,
            ..*self
        }
    }
    /// The piece from parameter t0 to t1 (either may lie past the ends, round the circle).
    pub fn sub(&self, t0: f64, t1: f64) -> Arc {
        Arc {
            start: self.angle_at(t0),
            sweep: self.dir() * (t1 - t0),
            ..*self
        }
    }
    /// The same arc counter-clockwise.
    pub fn ccw(&self) -> Arc {
        if self.sweep < 0.0 { self.reversed() } else { *self }
    }
    pub fn bbox(&self) -> Bbox {
        let mut b = Bbox::of([self.start_pt(), self.end_pt()]);
        for k in 0..4 {
            let a = k as f64 * PI / 2.0;
            if self.param_of_angle(a) <= self.len() {
                b.add(self.c + V2::polar(a, self.r));
            }
        }
        b
    }
}

impl Ellipse {
    pub fn minor(&self) -> V2 {
        self.major.perp() * self.ratio
    }
    pub fn dir(&self) -> f64 {
        if self.sweep < 0.0 { -1.0 } else { 1.0 }
    }
    pub fn is_full(&self) -> bool {
        self.sweep.abs() >= TAU
    }
    pub fn angle_at(&self, t: f64) -> f64 {
        self.start + self.dir() * t
    }
    pub fn at_angle(&self, a: f64) -> V2 {
        let (s, c) = dmath::sin_cos(a);
        self.c + self.major * c + self.minor() * s
    }
    pub fn point(&self, t: f64) -> V2 {
        self.at_angle(self.angle_at(t))
    }
    pub fn deriv(&self, t: f64) -> V2 {
        let (s, c) = dmath::sin_cos(self.angle_at(t));
        (self.minor() * c - self.major * s) * self.dir()
    }
    pub fn deriv2(&self, t: f64) -> V2 {
        let (s, c) = dmath::sin_cos(self.angle_at(t));
        -(self.major * c + self.minor() * s)
    }
    pub fn param_of_angle(&self, a: f64) -> f64 {
        wrap(self.dir() * (a - self.start))
    }
    pub fn a(&self) -> f64 {
        self.major.len()
    }
    pub fn b(&self) -> f64 {
        self.major.len() * self.ratio.abs()
    }
    /// The eccentric angle of a point on (or near) the ellipse.
    pub fn angle_of(&self, p: V2) -> f64 {
        let (u, w) = self.local(p);
        dmath::atan2(w / self.b(), u / self.a())
    }
    /// A point in the axes' frame.
    pub fn local(&self, p: V2) -> (f64, f64) {
        let ux = self.major.unit_or_zero();
        let d = p - self.c;
        (d.dot(ux), d.dot(ux.perp()))
    }
    pub fn reversed(&self) -> Ellipse {
        Ellipse {
            start: self.start + self.sweep,
            sweep: -self.sweep,
            ..*self
        }
    }
    /// The ellipse `c + u cos(th) + v sin(th)` for th from `start` through `sweep` (u and v any
    /// conjugate semi-diameters) in the standard form.
    pub fn from_conjugate(c: V2, u: V2, v: V2, start: f64, sweep: f64) -> Option<Ellipse> {
        let th0 = 0.5 * dmath::atan2(2.0 * u.dot(v), u.len2() - v.len2());
        let (s0, c0) = dmath::sin_cos(th0);
        let mut uu = u * c0 + v * s0;
        let mut vv = v * c0 - u * s0;
        let mut shift = th0;
        if vv.len() > uu.len() {
            (uu, vv) = (vv, -uu);
            shift += PI / 2.0;
        }
        let a = uu.len();
        if a == 0.0 || !a.is_finite() {
            return None;
        }
        let ratio = vv.len() / a;
        // th = phi + shift; the standard form's minor axis is a quarter turn counter-clockwise
        let s = if uu.cross(vv) < 0.0 { -1.0 } else { 1.0 };
        let mut st = s * (start - shift);
        let full = sweep.abs() >= TAU;
        if full {
            st = 0.0;
        }
        Some(Ellipse {
            c,
            major: uu,
            ratio,
            start: st,
            sweep: if full { TAU } else { s * sweep },
        })
    }
    pub fn bbox(&self) -> Bbox {
        let (u, v) = (self.major, self.minor());
        let mut b = if self.is_full() {
            Bbox::EMPTY
        } else {
            Bbox::of([self.point(0.0), self.point(self.sweep.abs())])
        };
        for th in [dmath::atan2(v.x, u.x), dmath::atan2(v.y, u.y)] {
            for a in [th, th + PI] {
                if self.is_full() || self.param_of_angle(a) <= self.sweep.abs() {
                    b.add(self.at_angle(a));
                }
            }
        }
        b
    }
}

impl Seg {
    pub fn start(&self) -> V2 {
        match self {
            Seg::Line(a, _) => *a,
            Seg::Arc(a) => a.start_pt(),
        }
    }
    pub fn end(&self) -> V2 {
        match self {
            Seg::Line(_, b) => *b,
            Seg::Arc(a) => a.end_pt(),
        }
    }
    /// At fraction f (outside 0..1 on the extension).
    pub fn point(&self, f: f64) -> V2 {
        match self {
            Seg::Line(a, b) => a.lerp(*b, f),
            Seg::Arc(a) => a.point(f * a.len()),
        }
    }
    /// d/df.
    pub fn deriv(&self, f: f64) -> V2 {
        match self {
            Seg::Line(a, b) => *b - *a,
            Seg::Arc(a) => a.deriv(f * a.len()) * a.len(),
        }
    }
    pub fn len(&self) -> f64 {
        match self {
            Seg::Line(a, b) => a.dist(*b),
            Seg::Arc(a) => a.r * a.len(),
        }
    }
    pub fn bulge(&self) -> f64 {
        match self {
            Seg::Line(..) => 0.0,
            Seg::Arc(a) => a.bulge(),
        }
    }
    pub fn curve(&self) -> Curve {
        match *self {
            Seg::Line(a, b) => Curve::Line { a, b },
            Seg::Arc(a) => Curve::Arc(a),
        }
    }
    /// The piece from fraction f0 to f1.
    pub fn sub(&self, f0: f64, f1: f64) -> Seg {
        match self {
            Seg::Line(..) => Seg::Line(self.point(f0), self.point(f1)),
            Seg::Arc(a) => Seg::Arc(a.sub(f0 * a.len(), f1 * a.len())),
        }
    }
    pub fn reversed(&self) -> Seg {
        match self {
            Seg::Line(a, b) => Seg::Line(*b, *a),
            Seg::Arc(a) => Seg::Arc(a.reversed()),
        }
    }
}

impl Poly {
    pub fn new(pts: Vec<V2>, bulges: Vec<f64>, closed: bool) -> Poly {
        let n = pts.len();
        let mut bulges = bulges;
        bulges.resize(n, 0.0);
        Poly {
            pts,
            bulges,
            widths: vec![],
            const_width: 0.0,
            closed,
        }
    }
    pub fn nseg(&self) -> usize {
        let n = self.pts.len();
        if n < 2 {
            0
        } else if self.closed {
            n
        } else {
            n - 1
        }
    }
    pub fn bulge(&self, k: usize) -> f64 {
        self.bulges.get(k).copied().unwrap_or(0.0)
    }
    pub fn seg(&self, k: usize) -> Seg {
        let n = self.pts.len();
        let (a, b) = (self.pts[k % n], self.pts[(k + 1) % n]);
        match Arc::from_bulge(a, b, self.bulge(k)) {
            Some(arc) => Seg::Arc(arc),
            None => Seg::Line(a, b),
        }
    }
    pub fn segs(&self) -> Vec<Seg> {
        (0..self.nseg()).map(|k| self.seg(k)).collect()
    }
    /// The segment and fraction of a parameter (clamped to a segment for outside parameters: the
    /// first segment before 0, the last after the end).
    pub fn locate(&self, t: f64) -> (usize, f64) {
        let n = self.nseg();
        if n == 0 {
            return (0, 0.0);
        }
        let k = if t < 0.0 {
            0
        } else {
            (t.floor() as usize).min(n - 1)
        };
        (k, t - k as f64)
    }
    /// From segments end to end (each starting where the last ended).
    pub fn from_segs(segs: &[Seg], closed: bool) -> Poly {
        let mut pts = vec![];
        let mut bulges = vec![];
        for s in segs {
            pts.push(s.start());
            bulges.push(s.bulge());
        }
        if !closed && let Some(s) = segs.last() {
            pts.push(s.end());
            bulges.push(0.0);
        }
        Poly::new(pts, bulges, closed)
    }
    fn width_at(&self, k: usize, f: f64) -> f64 {
        // the width along segment k at fraction f
        if self.widths.is_empty() {
            return 0.0;
        }
        let w = self.widths[k % self.widths.len()];
        w[0] + (w[1] - w[0]) * f
    }
}

impl Curve {
    pub fn line(a: V2, b: V2) -> Curve {
        Curve::Line { a, b }
    }
    pub fn circle(c: V2, r: f64) -> Curve {
        Curve::Circle { c, r }
    }
    /// A counter-clockwise arc from angle `a0` to `a1` (radians).
    pub fn arc(c: V2, r: f64, a0: f64, a1: f64) -> Curve {
        let mut sweep = wrap(a1 - a0);
        if sweep == 0.0 {
            sweep = TAU;
        }
        Curve::Arc(Arc {
            c,
            r,
            start: a0,
            sweep,
        })
    }
    pub fn poly(pts: &[V2], bulges: &[f64], closed: bool) -> Curve {
        Curve::Poly(Poly::new(pts.to_vec(), bulges.to_vec(), closed))
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Curve::Line { .. } => "line",
            Curve::Ray { .. } => "ray",
            Curve::Xline { .. } => "xline",
            Curve::Circle { .. } => "circle",
            Curve::Arc(_) => "arc",
            Curve::Ellipse(e) if e.is_full() => "ellipse",
            Curve::Ellipse(_) => "elliptical arc",
            Curve::Poly(_) => "polyline",
            Curve::Spline(_) => "spline",
            Curve::Offset(_) => "offset curve",
        }
    }

    pub fn domain(&self) -> (f64, f64) {
        match self {
            Curve::Line { .. } => (0.0, 1.0),
            Curve::Ray { .. } => (0.0, f64::INFINITY),
            Curve::Xline { .. } => (f64::NEG_INFINITY, f64::INFINITY),
            Curve::Circle { .. } => (0.0, TAU),
            Curve::Arc(a) => (0.0, a.len()),
            Curve::Ellipse(e) => (0.0, e.sweep.abs()),
            Curve::Poly(p) => (0.0, p.nseg() as f64),
            Curve::Spline(n) => n.domain(),
            Curve::Offset(o) => o.base.domain(),
        }
    }

    pub fn is_closed(&self) -> bool {
        match self {
            Curve::Circle { .. } => true,
            Curve::Ellipse(e) => e.is_full(),
            Curve::Poly(p) => p.closed,
            Curve::Spline(n) => n.closed,
            Curve::Offset(o) => o.base.is_closed(),
            _ => false,
        }
    }

    pub fn is_bounded(&self) -> bool {
        !matches!(self, Curve::Ray { .. } | Curve::Xline { .. })
    }

    /// The point at `t`; outside the domain, on the extension (lines along their direction, arcs
    /// and ellipses round, an open polyline's end segments, a spline at its end).
    pub fn point(&self, t: f64) -> V2 {
        match self {
            Curve::Line { a, b } => a.lerp(*b, t),
            Curve::Ray { at, dir } | Curve::Xline { at, dir } => *at + *dir * t,
            Curve::Circle { c, r } => *c + V2::polar(t, *r),
            Curve::Arc(a) => a.point(t),
            Curve::Ellipse(e) => e.point(t),
            Curve::Poly(p) => {
                if p.nseg() == 0 {
                    return p.pts.first().copied().unwrap_or_default();
                }
                let t = if p.closed {
                    t.rem_euclid(p.nseg() as f64)
                } else {
                    t
                };
                let (k, f) = p.locate(t);
                p.seg(k).point(f)
            }
            Curve::Spline(n) => {
                let (a, b) = n.domain();
                let t = if n.closed && (t < a || t > b) {
                    a + (t - a).rem_euclid(b - a)
                } else {
                    t.clamp(a, b)
                };
                n.point(t)
            }
            Curve::Offset(o) => o.point(t),
        }
    }

    /// d point / dt.
    pub fn deriv(&self, t: f64) -> V2 {
        match self {
            Curve::Line { a, b } => *b - *a,
            Curve::Ray { dir, .. } | Curve::Xline { dir, .. } => *dir,
            Curve::Circle { r, .. } => V2::polar(t, *r).perp(),
            Curve::Arc(a) => a.deriv(t),
            Curve::Ellipse(e) => e.deriv(t),
            Curve::Poly(p) => {
                if p.nseg() == 0 {
                    return V2::ZERO;
                }
                let t = if p.closed {
                    t.rem_euclid(p.nseg() as f64)
                } else {
                    t
                };
                let (k, f) = p.locate(t);
                p.seg(k).deriv(f)
            }
            Curve::Spline(n) => {
                let (a, b) = n.domain();
                n.derivs(t.clamp(a, b)).1
            }
            Curve::Offset(o) => o.deriv(t),
        }
    }

    /// The point, first and second derivatives.
    pub fn derivs(&self, t: f64) -> (V2, V2, V2) {
        match self {
            Curve::Circle { c, r } => {
                let u = V2::polar(t, *r);
                (*c + u, u.perp(), -u)
            }
            Curve::Arc(a) => {
                let u = V2::polar(a.angle_at(t), a.r);
                (a.c + u, u.perp() * a.dir(), -u)
            }
            Curve::Ellipse(e) => (e.point(t), e.deriv(t), e.deriv2(t)),
            Curve::Poly(p) if p.nseg() > 0 => {
                let t = if p.closed {
                    t.rem_euclid(p.nseg() as f64)
                } else {
                    t
                };
                let (k, f) = p.locate(t);
                let s = p.seg(k);
                match s {
                    Seg::Arc(a) => {
                        let l = a.len();
                        let u = V2::polar(a.angle_at(f * l), a.r);
                        (a.c + u, u.perp() * a.dir() * l, -u * (l * l))
                    }
                    Seg::Line(..) => (s.point(f), s.deriv(f), V2::ZERO),
                }
            }
            Curve::Spline(n) => {
                let (a, b) = n.domain();
                n.derivs(t.clamp(a, b))
            }
            Curve::Offset(o) => {
                let (lo, hi) = self.domain();
                let h = 1e-6 * (hi - lo);
                let (a, b) = if t + h <= hi { (t, t + h) } else { (t - h, t) };
                (o.point(t), o.deriv(t), (o.deriv(b) - o.deriv(a)) / (b - a))
            }
            _ => (self.point(t), self.deriv(t), V2::ZERO),
        }
    }

    /// The unit tangent at `t` (in the direction of increasing parameter).
    pub fn tangent(&self, t: f64) -> V2 {
        let d = self.deriv(t);
        match d.unit() {
            Some(u) => u,
            None => {
                // a cusp or a degenerate spline end: look a little along
                let (a, b) = self.domain();
                let h = 1e-7 * (b - a).min(1.0);
                let t2 = if t + h <= b { t + h } else { t - h };
                (self.point(t2.max(t)) - self.point(t2.min(t))).unit_or_zero()
            }
        }
    }

    pub fn start(&self) -> V2 {
        self.point(self.domain().0)
    }
    pub fn end(&self) -> V2 {
        self.point(self.domain().1)
    }

    /// The length from parameter `t0` to `t1` (t0 <= t1, within the domain or on the extension
    /// for lines and arcs).
    pub fn length_between(&self, t0: f64, t1: f64) -> f64 {
        if t1 <= t0 {
            return 0.0;
        }
        match self {
            Curve::Line { a, b } => a.dist(*b) * (t1 - t0),
            Curve::Ray { .. } | Curve::Xline { .. } => t1 - t0,
            Curve::Circle { r, .. } => r * (t1 - t0),
            Curve::Arc(a) => a.r * (t1 - t0),
            Curve::Ellipse(e) => {
                let f = |t: f64| e.deriv(t).len();
                integrate(&f, t0, t1, 1e-13)
            }
            Curve::Poly(p) => {
                let mut s = 0.0;
                let n = p.nseg();
                let mut t = t0;
                while t < t1 {
                    let k = if t < 0.0 {
                        0
                    } else {
                        (t.floor() as usize).min(n.saturating_sub(1))
                    };
                    let next = if k + 1 >= n { t1 } else { ((k + 1) as f64).min(t1) };
                    let next = if t < 0.0 && n > 0 {
                        next.min(1.0).min(t1)
                    } else {
                        next
                    };
                    let seg = p.seg(k);
                    s += seg.len() * (next - t);
                    if next <= t {
                        break;
                    }
                    t = next;
                }
                s
            }
            Curve::Spline(n) => {
                let br = n.breaks();
                let mut s = 0.0;
                let f = |t: f64| n.derivs(t).1.len();
                for w in br.windows(2) {
                    let (a, b) = (w[0].max(t0), w[1].min(t1));
                    if b > a {
                        s += integrate(&f, a, b, 1e-12);
                    }
                }
                s
            }
            Curve::Offset(o) => {
                let ts = o.samples();
                let f = |t: f64| o.deriv(t).len();
                let mut s = 0.0;
                for w in ts.windows(2) {
                    let (a, b) = (w[0].max(t0), w[1].min(t1));
                    if b > a {
                        s += integrate(&f, a, b, 1e-12);
                    }
                }
                s
            }
        }
    }

    pub fn length(&self) -> f64 {
        let (a, b) = self.domain();
        self.length_between(a, b)
    }

    /// The parameter `len` along the curve from its start (extension allowed for lines and arcs;
    /// clamped to the domain otherwise).
    pub fn param_at_length(&self, len: f64) -> f64 {
        let (a, b) = self.domain();
        match self {
            Curve::Line { a: p, b: q } => {
                let l = p.dist(*q);
                if l > 0.0 { len / l } else { 0.0 }
            }
            Curve::Ray { .. } | Curve::Xline { .. } => len,
            Curve::Circle { r, .. } => len / r,
            Curve::Arc(arc) => len / arc.r,
            _ => {
                let total = self.length();
                if len <= 0.0 {
                    return a;
                }
                if len >= total {
                    return b;
                }
                // bisection with secant steps on the monotonic length
                let f = |t: f64| self.length_between(a, t) - len;
                Nurbs::root(&f, a, b, 1e-14 * (b - a).abs().max(1.0))
            }
        }
    }

    /// The bounding box (infinite sides for rays and construction lines).
    pub fn bbox(&self) -> Bbox {
        match self {
            Curve::Line { a, b } => Bbox::of([*a, *b]),
            Curve::Ray { at, dir } => {
                let f = |o: f64, d: f64| {
                    if d > 0.0 {
                        (o, f64::INFINITY)
                    } else if d < 0.0 {
                        (f64::NEG_INFINITY, o)
                    } else {
                        (o, o)
                    }
                };
                let (x, y) = (f(at.x, dir.x), f(at.y, dir.y));
                Bbox {
                    min: v2(x.0, y.0),
                    max: v2(x.1, y.1),
                }
            }
            Curve::Xline { at, dir } => {
                let f = |o: f64, d: f64| {
                    if d != 0.0 {
                        (f64::NEG_INFINITY, f64::INFINITY)
                    } else {
                        (o, o)
                    }
                };
                let (x, y) = (f(at.x, dir.x), f(at.y, dir.y));
                Bbox {
                    min: v2(x.0, y.0),
                    max: v2(x.1, y.1),
                }
            }
            Curve::Circle { c, r } => Bbox {
                min: *c - v2(*r, *r),
                max: *c + v2(*r, *r),
            },
            Curve::Arc(a) => a.bbox(),
            Curve::Ellipse(e) => e.bbox(),
            Curve::Poly(p) => {
                let mut b = Bbox::of(p.pts.iter().copied());
                for s in p.segs() {
                    if let Seg::Arc(a) = s {
                        b = b.union(a.bbox());
                    }
                }
                b
            }
            Curve::Spline(n) => spline_bbox(n),
            Curve::Offset(o) => {
                let (a, b) = self.domain();
                let ts = o.samples();
                let mut bb = Bbox::of([o.point(a), o.point(b)]);
                for &t in &ts {
                    bb.add(o.point(t));
                }
                for axis in 0..2 {
                    let g = |t: f64| {
                        let d = o.deriv(t);
                        if axis == 0 { d.x } else { d.y }
                    };
                    for t in scan_roots(&g, &ts, 1e-12) {
                        bb.add(o.point(t));
                    }
                }
                bb
            }
        }
    }

    /// A box holding the curve, cheaper than `bbox` for splines (their control points') and
    /// offset curves.
    pub fn rough_bbox(&self) -> Bbox {
        match self {
            Curve::Spline(n) => n.hull_box(),
            Curve::Offset(o) => o.base.rough_bbox().grow(o.d.abs()),
            _ => self.bbox(),
        }
    }

    pub fn tol(&self) -> Tol {
        Tol::of_box(&self.rough_bbox())
    }

    /// The closest point: (parameter, point). For rays and construction lines too.
    pub fn closest(&self, p: V2) -> (f64, V2) {
        match self {
            Curve::Line { a, b } => {
                let d = *b - *a;
                let l2 = d.len2();
                let t = if l2 > 0.0 {
                    ((p - *a).dot(d) / l2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                (t, a.lerp(*b, t))
            }
            Curve::Ray { at, dir } => {
                let t = (p - *at).dot(*dir).max(0.0);
                (t, *at + *dir * t)
            }
            Curve::Xline { at, dir } => {
                let t = (p - *at).dot(*dir);
                (t, *at + *dir * t)
            }
            Curve::Circle { c, r } => {
                let t = if p == *c { 0.0 } else { wrap((p - *c).angle()) };
                (t, *c + V2::polar(t, *r))
            }
            Curve::Arc(a) => {
                let t = if p == a.c {
                    0.0
                } else {
                    a.param_of_angle((p - a.c).angle())
                };
                if t <= a.len() {
                    (t, a.point(t))
                } else {
                    let (s, e) = (a.start_pt(), a.end_pt());
                    if p.dist(s) <= p.dist(e) {
                        (0.0, s)
                    } else {
                        (a.len(), e)
                    }
                }
            }
            Curve::Ellipse(e) => {
                let ts = ellipse_feet(e, p, false);
                let mut best = (0.0, e.point(0.0));
                let mut bd = f64::INFINITY;
                for t in ts {
                    let q = e.point(t);
                    let d = q.dist(p);
                    if d < bd {
                        bd = d;
                        best = (t, q);
                    }
                }
                best
            }
            Curve::Poly(pl) => {
                let mut best = (0.0, pl.pts.first().copied().unwrap_or_default());
                let mut bd = f64::INFINITY;
                for (k, s) in pl.segs().iter().enumerate() {
                    let (t, q) = s.curve().closest(p);
                    let f = match s {
                        Seg::Line(..) => t,
                        Seg::Arc(a) => t / a.len(),
                    };
                    let d = q.dist(p);
                    if d < bd {
                        bd = d;
                        best = (k as f64 + f, q);
                    }
                }
                best
            }
            Curve::Spline(n) => spline_closest(n, p),
            Curve::Offset(o) => {
                // the offset's normals are the base's: its feet are the base's feet
                let (a, b) = self.domain();
                let mut ts = o.base.feet(p);
                if !self.is_closed() {
                    ts.extend([a, b]);
                }
                if ts.is_empty() {
                    ts.push(a);
                }
                let mut best = (a, o.point(a));
                let mut bd = f64::INFINITY;
                for t in ts {
                    let q = o.point(t);
                    if q.dist(p) < bd {
                        bd = q.dist(p);
                        best = (t, q);
                    }
                }
                best
            }
        }
    }

    pub fn dist(&self, p: V2) -> f64 {
        self.closest(p).1.dist(p)
    }

    /// The parameter of a point on the curve (its closest point's).
    pub fn param_of(&self, p: V2) -> f64 {
        self.closest(p).0
    }

    /// Parameters where the curve is perpendicular to the direction to `p` (the feet of the
    /// normals from `p`), within the domain.
    pub fn feet(&self, p: V2) -> Vec<f64> {
        let (lo, hi) = self.domain();
        match self {
            Curve::Line { .. } | Curve::Ray { .. } | Curve::Xline { .. } => {
                let (t, _) = self.closest_unbounded(p);
                if t >= lo && t <= hi { vec![t] } else { vec![] }
            }
            Curve::Circle { c, .. } => {
                if p == *c {
                    return vec![];
                }
                let a = wrap((p - *c).angle());
                vec![a, wrap(a + PI)]
            }
            Curve::Arc(a) => {
                if p == a.c {
                    return vec![];
                }
                let th = (p - a.c).angle();
                [th, th + PI]
                    .iter()
                    .map(|&x| a.param_of_angle(x))
                    .filter(|&t| t <= a.len())
                    .collect()
            }
            Curve::Ellipse(e) => ellipse_feet(e, p, true),
            Curve::Poly(pl) => {
                let mut out = vec![];
                for (k, s) in pl.segs().iter().enumerate() {
                    let c = s.curve();
                    for t in c.feet(p) {
                        let f = match s {
                            Seg::Line(..) => t,
                            Seg::Arc(a) => t / a.len(),
                        };
                        out.push(k as f64 + f);
                    }
                }
                out
            }
            Curve::Spline(n) => {
                let g = |t: f64| {
                    let (q, d, _) = n.derivs(t);
                    (q - p).dot(d)
                };
                let tol = Tol::of_box(&n.hull_box().union(Bbox::of([p])));
                let mut out = vec![];
                for t in scan_roots(&g, &spline_samples(n), 1e-14) {
                    let (q, d, _) = n.derivs(t);
                    if (q - p).dot(d.unit_or_zero()).abs() <= tol.lin * 1e3 {
                        out.push(t);
                    }
                }
                out
            }
            Curve::Offset(o) => o.base.feet(p),
        }
    }

    /// The closest point on the curve and its extension (lines as infinite lines, arcs as their
    /// circle, ellipses whole).
    pub fn closest_unbounded(&self, p: V2) -> (f64, V2) {
        match self {
            Curve::Line { a, b } => {
                let d = *b - *a;
                let l2 = d.len2();
                let t = if l2 > 0.0 { (p - *a).dot(d) / l2 } else { 0.0 };
                (t, a.lerp(*b, t))
            }
            Curve::Ray { at, dir } | Curve::Xline { at, dir } => {
                let t = (p - *at).dot(*dir);
                (t, *at + *dir * t)
            }
            Curve::Arc(a) => {
                let t = a.param_of_angle((p - a.c).angle());
                let t = if t > a.len() && t - a.len() > TAU - t {
                    t - TAU
                } else {
                    t
                };
                (t, a.point(t))
            }
            Curve::Ellipse(e) => {
                let full = Ellipse {
                    sweep: TAU * e.dir(),
                    ..*e
                };
                let (t, q) = Curve::Ellipse(full).closest(p);
                let t = if t > e.sweep.abs() && t - e.sweep.abs() > TAU - t {
                    t - TAU
                } else {
                    t
                };
                (t, q)
            }
            _ => self.closest(p),
        }
    }

    /// The piece from `t0` to `t1`. On a closed curve `t1 < t0` wraps past the end; lines, arcs,
    /// ellipses and open polylines may take parameters on their extension. None when empty.
    pub fn sub(&self, t0: f64, t1: f64) -> Option<Curve> {
        let (lo, hi) = self.domain();
        if self.is_closed() {
            let period = hi - lo;
            let mut t1 = t1;
            if t1 <= t0 {
                t1 += period;
            }
            if t1 - t0 <= 0.0 {
                return None;
            }
            return match self {
                Curve::Circle { c, r } => {
                    if t1 - t0 >= TAU {
                        return Some(self.clone());
                    }
                    Some(Curve::Arc(Arc {
                        c: *c,
                        r: *r,
                        start: t0,
                        sweep: t1 - t0,
                    }))
                }
                Curve::Ellipse(e) => {
                    if t1 - t0 >= TAU {
                        return Some(self.clone());
                    }
                    Some(Curve::Ellipse(Ellipse {
                        start: e.angle_at(t0),
                        sweep: e.dir() * (t1 - t0),
                        ..*e
                    }))
                }
                Curve::Poly(p) => Some(Curve::Poly(poly_sub(p, t0, t1))),
                Curve::Spline(n) => {
                    if t1 <= hi {
                        Some(Curve::Spline(n.sub(t0, t1)))
                    } else {
                        let a = n.sub(t0, hi);
                        let t1 = t1 - period;
                        if t1 <= lo {
                            return Some(Curve::Spline(a));
                        }
                        let b = n.sub(lo, t1);
                        if t0 >= hi {
                            return Some(Curve::Spline(b));
                        }
                        a.join(&b).map(Curve::Spline)
                    }
                }
                Curve::Offset(o) => {
                    let b = o.base.sub(t0, t1)?;
                    Some(Curve::Offset(Offset {
                        base: Box::new(b),
                        d: o.d,
                    }))
                }
                _ => None,
            };
        }
        if t1 <= t0 {
            return None;
        }
        match self {
            Curve::Line { .. } => Some(Curve::Line {
                a: self.point(t0),
                b: self.point(t1),
            }),
            Curve::Ray { dir, .. } | Curve::Xline { dir, .. } => {
                let a = self.point(t0.max(lo));
                if t1.is_infinite() {
                    if t0.is_infinite() {
                        return Some(self.clone());
                    }
                    Some(Curve::Ray { at: a, dir: *dir })
                } else if t0.is_infinite() {
                    Some(Curve::Ray {
                        at: self.point(t1),
                        dir: -*dir,
                    })
                } else {
                    Some(Curve::Line { a, b: self.point(t1) })
                }
            }
            Curve::Arc(a) => {
                if t1 - t0 >= TAU {
                    return Some(Curve::Circle { c: a.c, r: a.r });
                }
                Some(Curve::Arc(a.sub(t0, t1)))
            }
            Curve::Ellipse(e) => Some(Curve::Ellipse(Ellipse {
                start: e.angle_at(t0),
                sweep: e.dir() * (t1 - t0).min(TAU),
                ..*e
            })),
            Curve::Poly(p) => Some(Curve::Poly(poly_sub(p, t0, t1))),
            Curve::Spline(n) => {
                let (t0, t1) = (t0.max(lo), t1.min(hi));
                (t1 > t0).then(|| Curve::Spline(n.sub(t0, t1)))
            }
            Curve::Offset(o) => Some(Curve::Offset(Offset {
                base: Box::new(o.base.sub(t0, t1)?),
                d: o.d,
            })),
            Curve::Circle { .. } => None,
        }
    }

    /// The pieces between sorted parameters (inside the domain): an open curve splits into
    /// `ts.len() + 1` pieces; a closed one into `ts.len()` pieces, the last wrapping round.
    pub fn split_at(&self, ts: &[f64]) -> Vec<Curve> {
        let (lo, hi) = self.domain();
        let mut ts: Vec<f64> = ts.iter().copied().filter(|t| *t >= lo && *t <= hi).collect();
        ts.sort_by(f64::total_cmp);
        ts.dedup();
        let mut out = vec![];
        if self.is_closed() {
            if ts.is_empty() {
                return vec![self.clone()];
            }
            for k in 0..ts.len() {
                let t0 = ts[k];
                let t1 = if k + 1 < ts.len() { ts[k + 1] } else { ts[0] };
                if ts.len() == 1 {
                    // one cut opens the curve
                    if let Some(c) = self.sub(t0, t0) {
                        out.push(c);
                    }
                } else if let Some(c) = self.sub(t0, t1) {
                    out.push(c);
                }
            }
            return out;
        }
        let mut prev = lo;
        for &t in ts.iter().chain(std::iter::once(&hi)) {
            if t > prev
                && let Some(c) = self.sub(prev, t)
            {
                out.push(c);
            }
            prev = t;
        }
        out
    }

    /// The same curve run the other way.
    pub fn reversed(&self) -> Curve {
        match self {
            Curve::Line { a, b } => Curve::Line { a: *b, b: *a },
            Curve::Ray { .. } | Curve::Circle { .. } => self.clone(),
            Curve::Xline { at, dir } => Curve::Xline { at: *at, dir: -*dir },
            Curve::Arc(a) => Curve::Arc(a.reversed()),
            Curve::Ellipse(e) => Curve::Ellipse(e.reversed()),
            Curve::Poly(p) => Curve::Poly(poly_reversed(p)),
            Curve::Spline(n) => Curve::Spline(n.reversed()),
            Curve::Offset(o) => Curve::Offset(Offset {
                base: Box::new(o.base.reversed()),
                d: -o.d,
            }),
        }
    }

    /// The image under an affine map. A circle or arc under a map that is not a similarity
    /// becomes an ellipse; a polyline with arcs becomes a spline (exactly, rational).
    pub fn transformed(&self, m: &Affine) -> Option<Curve> {
        let sim = m.similarity_scale(1e-12);
        let flip = if m.is_reflection() { -1.0 } else { 1.0 };
        let out = match self {
            Curve::Line { a, b } => Curve::Line {
                a: m.apply(*a),
                b: m.apply(*b),
            },
            Curve::Ray { at, dir } => Curve::Ray {
                at: m.apply(*at),
                dir: m.vec(*dir).unit()?,
            },
            Curve::Xline { at, dir } => Curve::Xline {
                at: m.apply(*at),
                dir: m.vec(*dir).unit()?,
            },
            Curve::Circle { c, r } => match sim {
                Some(k) => Curve::Circle {
                    c: m.apply(*c),
                    r: r * k,
                },
                None => Curve::Ellipse(Ellipse::from_conjugate(
                    m.apply(*c),
                    m.vec(v2(*r, 0.0)),
                    m.vec(v2(0.0, *r)),
                    0.0,
                    TAU,
                )?),
            },
            Curve::Arc(a) => match sim {
                Some(k) => Curve::Arc(Arc {
                    c: m.apply(a.c),
                    r: a.r * k,
                    start: m.vec(V2::polar(a.start, 1.0)).angle(),
                    sweep: a.sweep * flip,
                }),
                None => Curve::Ellipse(Ellipse::from_conjugate(
                    m.apply(a.c),
                    m.vec(v2(a.r, 0.0)),
                    m.vec(v2(0.0, a.r)),
                    a.start,
                    a.sweep,
                )?),
            },
            Curve::Ellipse(e) => Curve::Ellipse(Ellipse::from_conjugate(
                m.apply(e.c),
                m.vec(e.major),
                m.vec(e.minor()),
                e.start,
                e.sweep,
            )?),
            Curve::Poly(p) => {
                let arcs = p.bulges.iter().any(|&b| b != 0.0);
                if arcs && sim.is_none() {
                    // its arcs would be elliptical: no polyline holds that
                    return None;
                }
                let k = sim.unwrap_or(1.0);
                Curve::Poly(Poly {
                    pts: p.pts.iter().map(|&q| m.apply(q)).collect(),
                    bulges: p.bulges.iter().map(|&b| b * flip).collect(),
                    widths: p.widths.iter().map(|w| [w[0] * k, w[1] * k]).collect(),
                    const_width: p.const_width * k,
                    closed: p.closed,
                })
            }
            Curve::Spline(n) => Curve::Spline(n.transformed(m)),
            Curve::Offset(o) => {
                let k = sim?;
                Curve::Offset(Offset {
                    base: Box::new(o.base.transformed(m)?),
                    d: o.d * k * flip,
                })
            }
        };
        Some(out)
    }

    /// Sample parameters for the numeric curves (splines and offsets): a few per knot span, or
    /// per eighth of a turn.
    pub fn samples(&self) -> Vec<f64> {
        match self {
            Curve::Spline(n) => spline_samples(n),
            Curve::Offset(o) => o.samples(),
            _ => {
                let (a, b) = self.domain();
                let (a, b) = (
                    if a.is_finite() { a } else { -1.0 },
                    if b.is_finite() { b } else { 1.0 },
                );
                let k = match self {
                    Curve::Poly(p) => 8 * p.nseg().max(1),
                    _ => (((b - a) / (PI / 16.0)).ceil() as usize).clamp(8, 64),
                };
                (0..=k).map(|i| a + (b - a) * i as f64 / k as f64).collect()
            }
        }
    }
}

fn poly_sub(p: &Poly, t0: f64, t1: f64) -> Poly {
    let n = p.nseg();
    let mut segs = vec![];
    let mut widths = vec![];
    let mut t = t0;
    let mut guard = 0;
    while t < t1 && guard <= 2 * n + 2 {
        guard += 1;
        let base = t.floor();
        let (k, f0, f1) = if p.closed {
            let k = (base as i64).rem_euclid(n as i64) as usize;
            (k, t - base, (t1 - base).min(1.0))
        } else if t < 0.0 {
            // the first segment's extension backwards
            (0, t, t1.min(1.0))
        } else {
            let k = (base as usize).min(n - 1);
            let f1 = if k == n - 1 {
                t1 - k as f64
            } else {
                (t1 - k as f64).min(1.0)
            };
            (k, t - k as f64, f1)
        };
        if f1 > f0 {
            let s = p.seg(k).sub(f0, f1);
            segs.push(s);
            if !p.widths.is_empty() {
                widths.push([p.width_at(k, f0), p.width_at(k, f1)]);
            }
        }
        let next = if p.closed || t >= 0.0 { k as f64 + 1.0 } else { 1.0 };
        let next = if p.closed { base + 1.0 } else { next };
        if next <= t {
            break;
        }
        t = next;
    }
    let mut out = Poly::from_segs(&segs, false);
    if !widths.is_empty() {
        let mut w: Vec<[f64; 2]> = widths.clone();
        w.push([widths.last().map_or(0.0, |x| x[1]); 2]);
        out.widths = w;
    }
    out.const_width = p.const_width;
    out
}

fn poly_reversed(p: &Poly) -> Poly {
    let n = p.pts.len();
    if n == 0 {
        return p.clone();
    }
    let segs: Vec<Seg> = p.segs().iter().rev().map(|s| s.reversed()).collect();
    let mut out = if p.closed {
        // keep the first vertex first
        let mut s = segs;
        s.rotate_right(1);
        let mut q = Poly::from_segs(&s, true);
        q.pts[0] = p.pts[0];
        q
    } else {
        Poly::from_segs(&segs, false)
    };
    if !p.widths.is_empty() {
        let ns = p.nseg();
        let mut w = vec![[0.0; 2]; n];
        for k in 0..ns {
            // segment k (from vertex k) becomes segment ns-1-k, its widths swapped
            let src = p.widths[k];
            let j = if p.closed { (ns - k) % ns } else { ns - 1 - k };
            w[j] = [src[1], src[0]];
        }
        out.widths = w;
    }
    out.const_width = p.const_width;
    out
}

/// Sample parameters along a spline: a few per knot span.
pub(crate) fn spline_samples(n: &Nurbs) -> Vec<f64> {
    let br = n.breaks();
    let per = 4 * (n.degree + 1);
    let mut out = vec![];
    for w in br.windows(2) {
        for k in 0..per {
            out.push(w[0] + (w[1] - w[0]) * k as f64 / per as f64);
        }
    }
    out.push(*br.last().unwrap_or(&0.0));
    out
}

/// Roots of `g` over the sample parameters: sign changes, solved, and near-zero minima of |g|
/// (touches), polished.
pub(crate) fn scan_roots(g: &dyn Fn(f64) -> f64, ts: &[f64], rel: f64) -> Vec<f64> {
    let vals: Vec<f64> = ts.iter().map(|&t| g(t)).collect();
    let scale = vals.iter().fold(0.0f64, |m, v| m.max(v.abs())).max(1e-300);
    let mut out = vec![];
    for k in 0..ts.len() {
        if vals[k] == 0.0 {
            out.push(ts[k]);
        }
        if k + 1 < ts.len() {
            let (a, b) = (vals[k], vals[k + 1]);
            if (a < 0.0 && b > 0.0) || (a > 0.0 && b < 0.0) {
                out.push(Nurbs::root(
                    g,
                    ts[k],
                    ts[k + 1],
                    rel * (ts[k + 1] - ts[k]).abs().max(1e-300),
                ));
            }
        }
        // a local minimum of |g| that may touch zero
        if k > 0 && k + 1 < ts.len() {
            let (a, b, c) = (vals[k - 1].abs(), vals[k].abs(), vals[k + 1].abs());
            if b < a && b <= c && vals[k - 1].signum() == vals[k + 1].signum() && b < 1e-3 * scale {
                let (lo, hi) = (ts[k - 1], ts[k + 1]);
                let t = golden_min(&|t| g(t).abs(), lo, hi);
                if g(t).abs() <= 1e-9 * scale {
                    out.push(t);
                }
            }
        }
    }
    out.sort_by(f64::total_cmp);
    out.dedup_by(|a, b| (*a - *b).abs() <= 1e-12 * (1.0 + b.abs()));
    out
}

/// The minimum of a unimodal `f` over [a, b] (golden section).
pub(crate) fn golden_min(f: &dyn Fn(f64) -> f64, mut a: f64, mut b: f64) -> f64 {
    let g = 0.618_033_988_749_894_9;
    let mut c = b - g * (b - a);
    let mut d = a + g * (b - a);
    let (mut fc, mut fd) = (f(c), f(d));
    for _ in 0..120 {
        if (b - a).abs() <= 1e-15 * (a.abs() + b.abs()).max(1e-300) {
            break;
        }
        if fc < fd {
            b = d;
            d = c;
            fd = fc;
            c = b - g * (b - a);
            fc = f(c);
        } else {
            a = c;
            c = d;
            fc = fd;
            d = a + g * (b - a);
            fd = f(d);
        }
    }
    0.5 * (a + b)
}

fn spline_bbox(n: &Nurbs) -> Bbox {
    let (a, b) = n.domain();
    let mut bb = Bbox::of([n.point(a), n.point(b)]);
    let ts = spline_samples(n);
    for axis in 0..2 {
        let g = |t: f64| {
            let d = n.derivs(t).1;
            if axis == 0 { d.x } else { d.y }
        };
        for t in scan_roots(&g, &ts, 1e-12) {
            bb.add(n.point(t));
        }
    }
    for &t in &ts {
        bb.add(n.point(t));
    }
    bb
}

/// Newton on (C(t) - p) . C'(t) = 0 from `t`, kept in [lo, hi].
fn newton_foot(c: &dyn Fn(f64) -> (V2, V2, V2), p: V2, mut t: f64, lo: f64, hi: f64) -> f64 {
    for _ in 0..40 {
        let (q, d1, d2) = c(t);
        let f = (q - p).dot(d1);
        let df = d1.len2() + (q - p).dot(d2);
        if df.abs() < 1e-300 {
            break;
        }
        let nt = (t - f / df).clamp(lo, hi);
        if (nt - t).abs() <= 1e-15 * (1.0 + t.abs()) {
            t = nt;
            break;
        }
        t = nt;
    }
    t
}

fn spline_closest(n: &Nurbs, p: V2) -> (f64, V2) {
    let (lo, hi) = n.domain();
    let ts = spline_samples(n);
    let ds: Vec<f64> = ts.iter().map(|&t| n.point(t).dist(p)).collect();
    let mut cands = vec![lo, hi];
    for k in 0..ts.len() {
        let l = if k > 0 { ds[k - 1] } else { f64::INFINITY };
        let r = if k + 1 < ts.len() {
            ds[k + 1]
        } else {
            f64::INFINITY
        };
        if ds[k] <= l && ds[k] <= r {
            cands.push(ts[k]);
        }
    }
    let ev = |t: f64| n.derivs(t);
    let mut best = (lo, n.point(lo));
    let mut bd = f64::INFINITY;
    for t0 in cands {
        let t = newton_foot(&ev, p, t0, lo, hi);
        // Newton may wander; keep the better of the start and the result
        for t in [t, t0] {
            let q = n.point(t);
            let d = q.dist(p);
            if d < bd {
                bd = d;
                best = (t, q);
            }
        }
    }
    best
}

/// Parameters of an ellipse's feet of normals from `p` (all, polished) — within the arc when
/// `only_feet`; otherwise with the arc's ends added and the full ellipse's for the closest.
fn ellipse_feet(e: &Ellipse, p: V2, only_feet: bool) -> Vec<f64> {
    let (a, b) = (e.a(), e.b());
    let (x, y) = e.local(p);
    // g(th) = (b^2 - a^2) sin cos + a x sin - b y cos; with s = tan(th / 2):
    let k = b * b - a * a;
    let coef = [-b * y, 2.0 * k + 2.0 * a * x, 0.0, -2.0 * k + 2.0 * a * x, b * y];
    let mut ths: Vec<f64> = forge_math::poly::all_real_roots(&coef)
        .iter()
        .map(|r| 2.0 * dmath::atan(r.x))
        .collect();
    ths.push(PI);
    if !only_feet {
        // the quadrants: robust starts when the quartic is ill-conditioned
        ths.extend([0.0, PI / 2.0, 3.0 * PI / 2.0]);
    }
    let g = |th: f64| {
        let (s, c) = dmath::sin_cos(th);
        (
            k * s * c + a * x * s - b * y * c,
            k * (c * c - s * s) + a * x * c + b * y * s,
        )
    };
    let mut out = vec![];
    let len = e.sweep.abs();
    let scale = a * (a + x.abs() + y.abs());
    for th0 in ths {
        let mut th = th0;
        for _ in 0..30 {
            let (f, df) = g(th);
            if df.abs() < 1e-300 {
                break;
            }
            let step = f / df;
            th -= step.clamp(-0.5, 0.5);
            if step.abs() < 1e-16 {
                break;
            }
        }
        if only_feet && g(th).0.abs() > 1e-9 * scale {
            continue;
        }
        let t = e.param_of_angle(th);
        if e.is_full() || t <= len {
            out.push(if e.is_full() { t.min(TAU) } else { t });
        }
    }
    if !only_feet && !e.is_full() {
        out.extend([0.0, len]);
    }
    out.sort_by(f64::total_cmp);
    out.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
    out
}

/// The tangent points from `p` to the curve: parameters where the tangent line passes through p.
pub fn tangent_params(c: &Curve, p: V2) -> Vec<f64> {
    match c {
        Curve::Circle { c: o, r } | Curve::Arc(Arc { c: o, r, .. }) => {
            let d = p - *o;
            let l = d.len();
            if l <= *r {
                return vec![];
            }
            let base = d.angle();
            let off = dmath::acos(r / l);
            let mut out = vec![];
            for a in [base + off, base - off] {
                match c {
                    Curve::Arc(arc) => {
                        let t = arc.param_of_angle(a);
                        if t <= arc.len() {
                            out.push(t);
                        }
                    }
                    _ => out.push(wrap(a)),
                }
            }
            out
        }
        Curve::Ellipse(e) => {
            // to the unit circle, where tangency is the circle's
            let m = Affine {
                m: [[e.major.x, e.minor().x], [e.major.y, e.minor().y]],
                t: e.c,
            };
            let Some(inv) = m.inverse() else { return vec![] };
            let q = inv.apply(p);
            let l = q.len();
            if l <= 1.0 {
                return vec![];
            }
            let off = dmath::acos(1.0 / l);
            let mut out = vec![];
            for th in [q.angle() + off, q.angle() - off] {
                let t = e.param_of_angle(th);
                if e.is_full() || t <= e.sweep.abs() {
                    out.push(t);
                }
            }
            out
        }
        Curve::Poly(pl) => {
            let mut out = vec![];
            for (k, s) in pl.segs().iter().enumerate() {
                if let Seg::Arc(a) = s {
                    for t in tangent_params(&Curve::Arc(*a), p) {
                        out.push(k as f64 + t / a.len());
                    }
                }
            }
            out
        }
        Curve::Spline(n) => {
            let g = |t: f64| {
                let (q, d, _) = n.derivs(t);
                (q - p).cross(d)
            };
            scan_roots(&g, &spline_samples(n), 1e-14)
        }
        _ => vec![],
    }
}

/// Intersections of two infinite lines a + s da and b + t db: (s, t).
pub fn line_line(a: V2, da: V2, b: V2, db: V2) -> Option<(f64, f64)> {
    let d = b - a;
    solve2(da.x, -db.x, da.y, -db.y, d.x, d.y, 1e-14)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, e: f64) -> bool {
        (a - b).abs() <= e
    }

    #[test]
    fn line_and_arc_evaluate() {
        let l = Curve::line(v2(0.0, 0.0), v2(4.0, 3.0));
        assert!(close(l.length(), 5.0, 1e-12));
        assert!(l.point(0.5).dist(v2(2.0, 1.5)) < 1e-12);
        assert!(close(l.param_of(v2(4.0, 0.0)), 0.64, 1e-12));
        let a = Curve::arc(v2(1.0, 1.0), 2.0, 0.0, PI / 2.0);
        assert!(close(a.length(), PI, 1e-12));
        assert!(a.end().dist(v2(1.0, 3.0)) < 1e-12);
        assert!(a.tangent(0.0).dist(v2(0.0, 1.0)) < 1e-12);
        let (t, q) = a.closest(v2(5.0, 5.0));
        assert!(close(t, PI / 4.0, 1e-12) && close(q.dist(v2(1.0, 1.0)), 2.0, 1e-12));
        let (t, _) = a.closest(v2(1.0, -5.0));
        assert_eq!(t, 0.0);
        let bb = a.bbox();
        assert!(bb.min.dist(v2(1.0, 1.0)) < 1e-12 && bb.max.dist(v2(3.0, 3.0)) < 1e-12);
        let r = a.reversed();
        assert!(r.start().dist(a.end()) < 1e-12 && r.point(0.1).dist(a.point(PI / 2.0 - 0.1)) < 1e-12);
    }

    #[test]
    fn three_point_arc() {
        let a = Arc::from_3_points(v2(1.0, 0.0), v2(0.0, 1.0), v2(-1.0, 0.0)).unwrap();
        assert!(a.c.dist(V2::ZERO) < 1e-12 && close(a.sweep, PI, 1e-12));
        let b = Arc::from_3_points(v2(-1.0, 0.0), v2(0.0, 1.0), v2(1.0, 0.0)).unwrap();
        assert!(close(b.sweep, -PI, 1e-12));
    }

    #[test]
    fn bulge_arcs() {
        let a = Arc::from_bulge(v2(0.0, 0.0), v2(2.0, 0.0), 1.0).unwrap();
        assert!(a.c.dist(v2(1.0, 0.0)) < 1e-12 && close(a.sweep, PI, 1e-12));
        // a positive bulge goes counter-clockwise: below the chord
        assert!(a.point(PI / 2.0).dist(v2(1.0, -1.0)) < 1e-12);
        assert!(close(a.bulge(), 1.0, 1e-12));
    }

    #[test]
    fn ellipse_evaluates() {
        let e = Ellipse {
            c: v2(1.0, 2.0),
            major: v2(0.0, 3.0),
            ratio: 0.5,
            start: 0.0,
            sweep: TAU,
        };
        let c = Curve::Ellipse(e);
        // perimeter of a 3 x 1.5 ellipse
        let chords: f64 = (0..100_000)
            .map(|k| {
                e.point(TAU * k as f64 / 1e5)
                    .dist(e.point(TAU * (k + 1) as f64 / 1e5))
            })
            .sum();
        assert!(close(c.length(), chords, 1e-8), "{} {chords}", c.length());
        for p in [v2(5.0, 1.0), v2(1.2, 2.1), v2(-3.0, 9.0), v2(1.0, 2.0)] {
            let (t, q) = c.closest(p);
            // brute force
            let best = (0..200_000)
                .map(|k| e.point(TAU * k as f64 / 200_000.0).dist(p))
                .fold(f64::MAX, f64::min);
            assert!(q.dist(p) <= best + 1e-9, "{p:?}");
            assert!(e.point(t).dist(q) < 1e-12);
        }
        let bb = c.bbox();
        assert!(bb.min.dist(v2(-0.5, -1.0)) < 1e-9 && bb.max.dist(v2(2.5, 5.0)) < 1e-9);
        let arc = Curve::Ellipse(Ellipse {
            sweep: -PI / 2.0,
            ..e
        });
        assert!(arc.end().dist(v2(2.5, 2.0)) < 1e-12);
        let mid = arc.param_at_length(arc.length() / 2.0);
        assert!(close(arc.length_between(0.0, mid) * 2.0, arc.length(), 1e-9));
    }

    #[test]
    fn conjugate_diameters() {
        let u = v2(2.0, 1.0);
        let v = v2(-0.5, 1.0);
        let e = Ellipse::from_conjugate(V2::ZERO, u, v, 0.3, 1.2).unwrap();
        for k in 0..=10 {
            let th = 0.3 + 1.2 * k as f64 / 10.0;
            let (s, c) = dmath::sin_cos(th);
            let want = u * c + v * s;
            assert!(e.point(1.2 * k as f64 / 10.0).dist(want) < 1e-12);
        }
        // a reflected frame runs the other way
        let e = Ellipse::from_conjugate(V2::ZERO, v2(2.0, 0.0), v2(0.0, -1.0), 0.0, 1.0).unwrap();
        assert!(e.point(1.0).dist(v2(2.0 * dmath::cos(1.0), -dmath::sin(1.0))) < 1e-12);
    }

    #[test]
    fn polyline_pieces() {
        let p = Curve::poly(
            &[v2(0.0, 0.0), v2(10.0, 0.0), v2(10.0, 10.0)],
            &[0.0, 1.0, 0.0],
            false,
        );
        // a half circle of diameter 10
        assert!(close(p.length(), 10.0 + PI * 5.0, 1e-9));
        let s = p.sub(0.5, 1.5).unwrap();
        assert!(s.start().dist(v2(5.0, 0.0)) < 1e-12);
        assert!(s.end().dist(p.point(1.5)) < 1e-12);
        let Curve::Poly(sp) = &s else { panic!() };
        assert_eq!(sp.pts.len(), 3);
        let r = p.reversed();
        assert!(r.start().dist(v2(10.0, 10.0)) < 1e-12);
        assert!(r.point(0.5).dist(p.point(1.5)) < 1e-12);
        let sq = Curve::poly(
            &[v2(0.0, 0.0), v2(1.0, 0.0), v2(1.0, 1.0), v2(0.0, 1.0)],
            &[],
            true,
        );
        let w = sq.sub(3.5, 0.5).unwrap();
        assert!(close(w.length(), 1.0, 1e-12));
        assert!(w.start().dist(v2(0.0, 0.5)) < 1e-12 && w.end().dist(v2(0.5, 0.0)) < 1e-12);
        assert_eq!(sq.split_at(&[0.5, 2.5]).len(), 2);
        // extension past the ends
        let ext = p.sub(-0.5, 2.0).unwrap();
        assert!(ext.start().dist(v2(-5.0, 0.0)) < 1e-12);
    }

    #[test]
    fn transforms() {
        let a = Curve::arc(V2::ZERO, 1.0, 0.0, PI / 2.0);
        let m = Affine::mirror(V2::ZERO, v2(0.0, 1.0));
        let b = a.transformed(&m).unwrap();
        assert!(b.start().dist(v2(-1.0, 0.0)) < 1e-12 && b.end().dist(v2(0.0, 1.0)) < 1e-12);
        let s = Affine::scale_xy(V2::ZERO, 2.0, 1.0);
        let e = a.transformed(&s).unwrap();
        assert!(matches!(e, Curve::Ellipse(_)));
        assert!(e.start().dist(v2(2.0, 0.0)) < 1e-12 && e.end().dist(v2(0.0, 1.0)) < 1e-12);
        let p = Curve::poly(&[v2(0.0, 0.0), v2(2.0, 0.0)], &[1.0], false);
        // its arc would be elliptical
        assert!(p.transformed(&s).is_none());
        let r = p.transformed(&Affine::rotate(V2::ZERO, 1.0)).unwrap();
        assert!(close(r.length(), p.length(), 1e-12));
    }

    #[test]
    fn ellipse_offset_is_exact() {
        let e = Ellipse {
            c: v2(1.0, 0.0),
            major: v2(4.0, 1.0),
            ratio: 0.5,
            start: 0.5,
            sweep: 2.0,
        };
        let o = Curve::Offset(Offset {
            base: Box::new(Curve::Ellipse(e)),
            d: -0.3,
        });
        let base = Curve::Ellipse(e);
        for k in 0..=20 {
            let t = 2.0 * k as f64 / 20.0;
            let q = o.point(t);
            assert!(close(base.dist(q), 0.3, 1e-9));
            // the derivative matches differences
            let h = 1e-6;
            let fd = (o.point(t + h) - o.point(t - h)) / (2.0 * h);
            assert!(fd.dist(o.deriv(t)) < 1e-6);
        }
        let p = v2(3.0, 3.0);
        let (t, q) = o.closest(p);
        let best = (0..20_000)
            .map(|k| o.point(2.0 * k as f64 / 20_000.0).dist(p))
            .fold(f64::MAX, f64::min);
        assert!(q.dist(p) <= best + 1e-9 && o.point(t).dist(q) < 1e-12);
        let s = o.sub(0.5, 1.5).unwrap();
        assert!(s.start().dist(o.point(0.5)) < 1e-12 && s.end().dist(o.point(1.5)) < 1e-12);
        let r = o.reversed();
        assert!(r.point(0.25).dist(o.point(1.75)) < 1e-12);
    }
}
