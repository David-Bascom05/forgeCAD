//! Smooth lofts: surfaces through matched points of three or more exact section curves, a cubic
//! spline across the sections at each point. With the knots fixed for the whole loft, the spline
//! is a sum of weights times data, S(t, s) = sum_k w_k(s) C_k(t) (+ w(s) D for a given end
//! slope D), the weights the natural (or end-clamped) cubic spline's: curvature-continuous
//! through every middle section. Points and derivatives come straight from the section curves.
//!
//! The surface is carried on past the face (half a span past each end in s, up to a quarter past
//! each side in t, as far as it stays regular); the implicit value is the signed distance to its
//! nearest point and projection searches all of it, as for ruled surfaces.

use crate::curve::{CurveDeriv, Domain1, ProcCurve, ProceduralCurve};
use crate::ruled::{Rail, principal_max};
use crate::screw::{fresh_id, polish, remembered};
use crate::surface::{Domain2, Proc, Procedural, ShapeBounds, SurfDeriv, Surface};
use forge_math::{Box2, Box3, Frame, Interval, Point2, Point3, Vec3, tol};
use std::sync::Arc;

/// The most data a spline sums: sections, and a slope at each end.
const MAX: usize = 16;

/// Each datum's weight in a value and its first two derivatives.
type Weights = [[f64; 3]; MAX];

/// A cubic spline's weights through values at fixed knots, natural at an end or with a given
/// slope there.
#[derive(Clone, Debug)]
pub struct Spline {
    knots: Vec<f64>,
    /// The second derivatives at the knots for each unit datum: the values, then the start's
    /// and the end's slope where given.
    m: Vec<Vec<f64>>,
    slopes: usize,
}

impl Spline {
    /// None unless the knots increase, or for more than 14 of them.
    pub fn new(knots: Vec<f64>, start_slope: bool, end_slope: bool) -> Option<Spline> {
        let n = knots.len();
        if n < 2
            || n + 2 > MAX
            || knots
                .windows(2)
                .any(|w| w[1] - w[0] <= tol::LINEAR || w[1].is_nan())
        {
            return None;
        }
        let h: Vec<f64> = knots.windows(2).map(|w| w[1] - w[0]).collect();
        // the tridiagonal system's rows: (below, diagonal, above)
        let mut rows = vec![(0.0, 1.0, 0.0); n];
        if start_slope {
            rows[0] = (0.0, 2.0 * h[0], h[0]);
        }
        if end_slope {
            rows[n - 1] = (h[n - 2], 2.0 * h[n - 2], 0.0);
        }
        for i in 1..n - 1 {
            rows[i] = (h[i - 1], 2.0 * (h[i - 1] + h[i]), h[i]);
        }
        let slopes = usize::from(start_slope) + usize::from(end_slope);
        let m = (0..n + slopes)
            .map(|d| {
                // the right side for unit datum d
                let y = |i: usize| if d == i { 1.0 } else { 0.0 };
                let mut r = vec![0.0; n];
                for (i, ri) in r.iter_mut().enumerate().take(n - 1).skip(1) {
                    *ri = 6.0 * ((y(i + 1) - y(i)) / h[i] - (y(i) - y(i - 1)) / h[i - 1]);
                }
                if start_slope {
                    let d0 = if d == n { 1.0 } else { 0.0 };
                    r[0] = 6.0 * ((y(1) - y(0)) / h[0] - d0);
                }
                if end_slope {
                    let d1 = if d == n + slopes - 1 { 1.0 } else { 0.0 };
                    r[n - 1] = 6.0 * (d1 - (y(n - 1) - y(n - 2)) / h[n - 2]);
                }
                thomas(&rows, r)
            })
            .collect();
        Some(Spline { knots, m, slopes })
    }

    pub fn knots(&self) -> &[f64] {
        &self.knots
    }

    fn data(&self) -> usize {
        self.knots.len() + self.slopes
    }

    /// The span holding s (the end spans carried on past the ends).
    fn span(&self, s: f64) -> usize {
        let n = self.knots.len();
        self.knots.partition_point(|&k| k <= s).clamp(1, n - 1) - 1
    }

    /// Each datum's weight in the value and the first two derivatives at s.
    fn weights(&self, s: f64) -> Weights {
        let mut out = [[0.0; 3]; MAX];
        let i = self.span(s);
        let (k0, k1) = (self.knots[i], self.knots[i + 1]);
        let h = k1 - k0;
        let (a, b) = ((k1 - s) / h, (s - k0) / h);
        for (d, o) in out.iter_mut().enumerate().take(self.data()) {
            let (mi, mj) = (self.m[d][i], self.m[d][i + 1]);
            let (yi, yj) = (f64::from(d == i), f64::from(d == i + 1));
            *o = [
                a * yi + b * yj + ((a * a * a - a) * mi + (b * b * b - b) * mj) * h * h / 6.0,
                (yj - yi) / h + (-(3.0 * a * a - 1.0) * mi + (3.0 * b * b - 1.0) * mj) * h / 6.0,
                a * mi + b * mj,
            ];
        }
        out
    }

    /// Boxes of each datum's weight (`order` 0) or its derivative (1, 2, 3) over s.
    fn weight_box(&self, s: Interval, order: usize) -> Vec<Interval> {
        let mut out = vec![Interval::EMPTY; self.data()];
        let n = self.knots.len();
        let mut lo = s.lo;
        loop {
            let i = self.span(lo);
            let hi = if i + 2 < n {
                s.hi.min(self.knots[i + 1])
            } else {
                s.hi
            };
            let (k0, k1) = (self.knots[i], self.knots[i + 1]);
            let h = k1 - k0;
            let si = Interval::new(lo, hi.max(lo));
            let a = (Interval::point(k1) - si) * (1.0 / h);
            let b = (si - Interval::point(k0)) * (1.0 / h);
            for (d, o) in out.iter_mut().enumerate() {
                let (mi, mj) = (self.m[d][i], self.m[d][i + 1]);
                let (yi, yj) = (f64::from(d == i), f64::from(d == i + 1));
                let w = match order {
                    0 => a * yi + b * yj + ((a * a * a - a) * mi + (b * b * b - b) * mj) * (h * h / 6.0),
                    1 => {
                        Interval::point((yj - yi) / h)
                            + ((a * a * 3.0 + (-1.0)) * (-mi) + (b * b * 3.0 + (-1.0)) * mj) * (h / 6.0)
                    }
                    2 => a * mi + b * mj,
                    _ => Interval::point((mj - mi) / h),
                };
                *o = o.hull(&w);
            }
            if hi >= s.hi {
                return out;
            }
            lo = hi;
        }
    }
}

/// Solves a tridiagonal system (rows of below, diagonal, above).
fn thomas(rows: &[(f64, f64, f64)], mut r: Vec<f64>) -> Vec<f64> {
    let n = r.len();
    let mut c = vec![0.0; n];
    let (mut b0, c0, _) = (rows[0].1, rows[0].2, 0.0);
    c[0] = c0 / b0;
    r[0] /= b0;
    for i in 1..n {
        let (a, b, cc) = rows[i];
        b0 = b - a * c[i - 1];
        c[i] = cc / b0;
        r[i] = (r[i] - a * r[i - 1]) / b0;
    }
    for i in (0..n - 1).rev() {
        r[i] -= c[i] * r[i + 1];
    }
    r
}

/// A guide curve: a chain of lines and arcs, by arc length from its start (carried straight
/// on past its ends).
#[derive(Clone, Debug)]
pub struct GuidePath {
    /// Each piece, where it starts along the guide, and its length.
    pieces: Vec<(Rail, f64, f64)>,
}

impl GuidePath {
    /// The chain of `pieces` (each from where the one before ends).
    pub fn new(pieces: &[Rail]) -> GuidePath {
        let mut at = 0.0;
        GuidePath {
            pieces: pieces
                .iter()
                .map(|&r| {
                    let len = r.length();
                    at += len;
                    (r, at - len, len)
                })
                .collect(),
        }
    }

    pub fn length(&self) -> f64 {
        self.pieces.last().map_or(0.0, |p| p.1 + p.2)
    }

    fn piece(&self, p: f64) -> &(Rail, f64, f64) {
        let i = self
            .pieces
            .partition_point(|x| x.1 <= p)
            .clamp(1, self.pieces.len())
            - 1;
        &self.pieces[i]
    }

    /// Point and first two derivatives at arc length p.
    pub fn at(&self, p: f64) -> (Point3, Vec3, Vec3) {
        let &(r, s0, len) = self.piece(p);
        let (q, d1, d2) = r.at((p - s0) / len);
        (q, d1 / len, d2 / (len * len))
    }

    fn bbox(&self, p: Interval) -> Box3 {
        let mut b = Box3::EMPTY;
        let n = self.pieces.len();
        for (i, &(r, s0, len)) in self.pieces.iter().enumerate() {
            // the end pieces carry on past the guide's ends
            let lo = if i == 0 { p.lo } else { p.lo.max(s0) };
            let hi = if i + 1 == n { p.hi } else { p.hi.min(s0 + len) };
            if lo <= hi {
                b = b.hull(&r.bbox(Interval::new((lo - s0) / len, (hi - s0) / len)));
            }
        }
        b
    }

    /// The arc lengths where the guide crosses the plane through `o` square to unit `n`.
    pub fn crossings(&self, o: Point3, n: Vec3) -> Vec<f64> {
        let mut out = Vec::new();
        for &(r, s0, len) in &self.pieces {
            let ts: Vec<f64> = match r {
                Rail::Line { a, b } => {
                    let (da, db) = (n.dot(a - o), n.dot(b - o));
                    if da == db { vec![] } else { vec![da / (da - db)] }
                }
                Rail::Arc { circle, start, sweep } => {
                    // n.(c + r cos a x + r sin a y - o) = 0: A cos a + B sin a = C
                    let f = &circle.frame;
                    let (ca, cb, cc) = (
                        circle.radius * n.dot(f.x().get()),
                        circle.radius * n.dot(f.y().get()),
                        -n.dot(f.origin - o),
                    );
                    let m = (ca * ca + cb * cb).sqrt();
                    if m <= 0.0 || cc.abs() > m {
                        vec![]
                    } else {
                        let base = forge_math::dmath::atan2(cb, ca);
                        let off = forge_math::dmath::acos_clamped(cc / m);
                        let per = forge_math::dmath::TAU / sweep.abs();
                        [base + off, base - off]
                            .into_iter()
                            .map(|a| {
                                // the turn of the angle that falls on the arc, if any
                                let t = (a - start) / sweep;
                                t - per * (t / per).floor()
                            })
                            .collect()
                    }
                }
                Rail::Parabola { .. } | Rail::Wrap(_) => unreachable!("guide paths are lines and arcs"),
            };
            out.extend(
                ts.into_iter()
                    .filter(|t| (-1e-12..=1.0 + 1e-12).contains(t))
                    .map(|t| s0 + t.clamp(0.0, 1.0) * len),
            );
        }
        out.sort_by(f64::total_cmp);
        out.dedup_by(|a, b| (*a - *b).abs() <= 1e-9);
        out
    }

    fn transformed(&self, f: &Frame) -> GuidePath {
        GuidePath {
            pieces: self
                .pieces
                .iter()
                .map(|&(r, s0, len)| (r.transformed(f), s0, len))
                .collect(),
        }
    }
}

/// How much of a guide's pull a wall takes across it: all of it along its start (t = 0), along
/// its end (t = 1), or everywhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Along {
    Start,
    End,
    Whole,
}

/// A guide's pull on a skinned surface: E(s) = G(p(s)) - (the spline through `vals` and the end
/// slopes), p(s) the spline through the arc lengths `p` where the guide crosses the sections. It
/// vanishes at every section, so a wall edge carrying the whole of it runs along the guide.
#[derive(Clone, Debug)]
pub struct Shift {
    path: Arc<GuidePath>,
    p: Vec<f64>,
    vals: Vec<Point3>,
    map: Arc<Spline>,
    along: Along,
}

impl Shift {
    /// The pull of `path` (crossing the sections at arc lengths `p`, where the surface's own
    /// spline has the points `vals`).
    pub fn new(
        path: Arc<GuidePath>,
        p: Vec<f64>,
        vals: Vec<Point3>,
        knots: &[f64],
        along: Along,
    ) -> Option<Shift> {
        Some(Shift {
            path,
            p,
            vals,
            map: Arc::new(Spline::new(knots.to_vec(), false, false)?),
            along,
        })
    }

    fn with(&self, along: Along) -> Shift {
        Shift {
            along,
            ..self.clone()
        }
    }

    /// The weight across the wall at t, and its derivative.
    fn beta(&self, t: f64) -> (f64, f64) {
        match self.along {
            Along::Start => (1.0 - t, -1.0),
            Along::End => (t, 1.0),
            Along::Whole => (1.0, 0.0),
        }
    }

    /// E and its first two derivatives, given the surface's weights `w` and the map's `m` there
    /// (every guide of a loft shares the map's knots), and the end slopes.
    fn at(&self, w: &Weights, m: &Weights, slopes: &[Vec3]) -> [Vec3; 3] {
        let n = self.p.len();
        let (mut p, mut p1, mut p2) = (0.0, 0.0, 0.0);
        for (mk, &pk) in m.iter().zip(&self.p) {
            p += mk[0] * pk;
            p1 += mk[1] * pk;
            p2 += mk[2] * pk;
        }
        let (g, g1, g2) = self.path.at(p);
        let mut e = [g.to_vec(), g1 * p1, g2 * (p1 * p1) + g1 * p2];
        for (wk, v) in w.iter().zip(&self.vals) {
            for (i, ei) in e.iter_mut().enumerate() {
                *ei -= v.to_vec() * wk[i];
            }
        }
        for (j, &d) in slopes.iter().enumerate() {
            for (i, ei) in e.iter_mut().enumerate() {
                *ei -= d * w[n + j][i];
            }
        }
        e
    }

    /// A box holding E over s, from the values' weights `w` there.
    fn bbox(&self, s: Interval, w: &[Interval], slopes: &[Vec3]) -> Box3 {
        let m = self.map.weight_box(s, 0);
        let n = self.p.len();
        let mut p = Interval::point(self.p[0]);
        for (mk, &pk) in m.iter().zip(&self.p) {
            p = p + *mk * (pk - self.p[0]);
        }
        // measured from the first section's point: the weights sum to one
        let g0 = self.vals[0];
        let gb = self.path.bbox(p);
        let mut b = Box3 {
            x: gb.x - Interval::point(g0.x),
            y: gb.y - Interval::point(g0.y),
            z: gb.z - Interval::point(g0.z),
        };
        for (wk, v) in w.iter().zip(&self.vals) {
            let v = *v - g0;
            b.x = b.x - *wk * v.x;
            b.y = b.y - *wk * v.y;
            b.z = b.z - *wk * v.z;
        }
        for (j, &d) in slopes.iter().enumerate() {
            b.x = b.x - w[n + j] * d.x;
            b.y = b.y - w[n + j] * d.y;
            b.z = b.z - w[n + j] * d.z;
        }
        b
    }

    fn transformed(&self, f: &Frame) -> Shift {
        Shift {
            path: Arc::new(self.path.transformed(f)),
            vals: self.vals.iter().map(|&v| f.point_to_world(v)).collect(),
            ..self.clone()
        }
    }
}

/// The data a skinned surface or rail sums: section curves (or points), end slopes, and guides'
/// pulls.
#[derive(Clone, Debug)]
struct Data {
    rails: Vec<Rail>,
    slopes: Vec<Vec3>,
    spline: Arc<Spline>,
    shifts: Vec<Shift>,
    poly: Option<Poly>,
}

/// The surface summed once per span: A(s) + B(s) cos a + D(s) sin a + E(s) t, a = start + sweep t,
/// each a cubic in the fraction b along the span (where every rail is a line or an arc turning
/// through the same angles, and no guide pulls on it).
#[derive(Clone, Debug)]
struct Poly {
    start: f64,
    sweep: f64,
    arcs: bool,
    /// Per span, [A, B, D, E] by powers of b.
    spans: Vec<[[Vec3; 4]; 4]>,
}

impl Poly {
    fn new(rails: &[Rail], slopes: &[Vec3], spline: &Spline, shifts: &[Shift]) -> Option<Poly> {
        if !shifts.is_empty() {
            return None;
        }
        let mut angles = None;
        for r in rails {
            match *r {
                Rail::Line { .. } => {}
                Rail::Arc { start, sweep, .. } => {
                    if angles.is_some_and(|a| a != (start, sweep)) {
                        return None;
                    }
                    angles = Some((start, sweep));
                }
                _ => return None,
            }
        }
        let (start, sweep) = angles.unwrap_or((0.0, 0.0));
        let k = &spline.knots;
        let spans = (0..k.len() - 1)
            .map(|i| {
                let q = (k[i + 1] - k[i]) * (k[i + 1] - k[i]) / 6.0;
                let mut out = [[Vec3::ZERO; 4]; 4];
                // datum d's weight over the span, in powers of b (as `Spline::weights`)
                let weight = |d: usize| {
                    let (mi, mj) = (spline.m[d][i], spline.m[d][i + 1]);
                    let (yi, yj) = (f64::from(d == i), f64::from(d == i + 1));
                    [yi, yj - yi - q * (2.0 * mi + mj), 3.0 * q * mi, q * (mj - mi)]
                };
                let mut add = |part: usize, d: usize, v: Vec3| {
                    for (o, c) in out[part].iter_mut().zip(weight(d)) {
                        *o += v * c;
                    }
                };
                for (d, r) in rails.iter().enumerate() {
                    match *r {
                        Rail::Line { a, b } => {
                            add(0, d, a.to_vec());
                            add(3, d, b - a);
                        }
                        Rail::Arc { circle, .. } => {
                            let (f, rad) = (&circle.frame, circle.radius);
                            add(0, d, f.origin.to_vec());
                            add(1, d, f.x().get() * rad);
                            add(2, d, f.y().get() * rad);
                        }
                        _ => unreachable!("lines and arcs only"),
                    }
                }
                for (j, &v) in slopes.iter().enumerate() {
                    add(0, rails.len() + j, v);
                }
                out
            })
            .collect();
        Some(Poly {
            start,
            sweep,
            arcs: angles.is_some(),
            spans,
        })
    }

    fn point(&self, spline: &Spline, uv: Point2) -> SurfDeriv {
        let i = spline.span(uv.y);
        let (k0, h) = (spline.knots[i], spline.knots[i + 1] - spline.knots[i]);
        let b = (uv.y - k0) / h;
        // each part's value and first two derivatives in s
        let cubic = |c: &[Vec3; 4]| {
            [
                ((c[3] * b + c[2]) * b + c[1]) * b + c[0],
                ((c[3] * (3.0 * b) + c[2] * 2.0) * b + c[1]) * (1.0 / h),
                (c[3] * (6.0 * b) + c[2] * 2.0) * (1.0 / (h * h)),
            ]
        };
        let sp = &self.spans[i];
        let (a, e) = (cubic(&sp[0]), cubic(&sp[3]));
        let t = uv.x;
        let mut d = SurfDeriv {
            p: Point3::ORIGIN + a[0] + e[0] * t,
            du: e[0],
            dv: a[1] + e[1] * t,
            duu: Vec3::ZERO,
            duv: e[1],
            dvv: a[2] + e[2] * t,
        };
        if self.arcs {
            let (sn, cs) = forge_math::dmath::sin_cos(self.start + self.sweep * t);
            let (bb, dd) = (cubic(&sp[1]), cubic(&sp[2]));
            let w = self.sweep;
            let r0 = bb[0] * cs + dd[0] * sn;
            d.p += r0;
            d.du += (dd[0] * cs - bb[0] * sn) * w;
            d.duu = r0 * (-w * w);
            d.dv += bb[1] * cs + dd[1] * sn;
            d.duv += (dd[1] * cs - bb[1] * sn) * w;
            d.dvv += bb[2] * cs + dd[2] * sn;
        }
        d
    }
}

impl Poly {
    /// A box holding the points over `uv`, span by span.
    fn bbox(&self, spline: &Spline, uv: Box2) -> Box3 {
        let k = &spline.knots;
        let mut out = Box3::EMPTY;
        let mut lo = uv.v.lo;
        loop {
            let i = spline.span(lo);
            let hi = if i + 2 < k.len() {
                uv.v.hi.min(k[i + 1])
            } else {
                uv.v.hi
            };
            let h = k[i + 1] - k[i];
            let b = (Interval::new(lo, hi.max(lo)) + (-k[i])) * (1.0 / h);
            let cubic = |c: &[Vec3; 4]| {
                let f = |x: fn(&Vec3) -> f64| ((b * x(&c[3]) + x(&c[2])) * b + x(&c[1])) * b + x(&c[0]);
                [f(|v| v.x), f(|v| v.y), f(|v| v.z)]
            };
            let sp = &self.spans[i];
            let (a, e) = (cubic(&sp[0]), cubic(&sp[3]));
            let mut p = [0, 1, 2].map(|j| a[j] + e[j] * uv.u);
            if self.arcs {
                let ang = uv.u * self.sweep + self.start;
                let (cs, sn) = (ang.cos(), ang.sin());
                let (bb, dd) = (cubic(&sp[1]), cubic(&sp[2]));
                for j in 0..3 {
                    p[j] = p[j] + bb[j] * cs + dd[j] * sn;
                }
            }
            out = out.hull(&Box3 {
                x: p[0],
                y: p[1],
                z: p[2],
            });
            if hi >= uv.v.hi {
                return out.grow(tol::LINEAR);
            }
            lo = hi;
        }
    }

    /// The control points of span i's piece over `uv` (v by the span's knots k0, k0 + h); None for a
    /// span mixing lines and arcs, or a piece turning half a turn or more.
    fn hull(&self, i: usize, k0: f64, h: f64, uv: Box2) -> Option<Hull> {
        let sp = &self.spans[i];
        let (b0, b1) = ((uv.v.lo - k0) / h, (uv.v.hi - k0) / h);
        let third = (b1 - b0) / 3.0;
        // a part's Bernstein points over [b0, b1], from its values and slopes at the ends
        let bern = |c: &[Vec3; 4]| {
            let val = |b: f64| ((c[3] * b + c[2]) * b + c[1]) * b + c[0];
            let der = |b: f64| (c[3] * (3.0 * b) + c[2] * 2.0) * b + c[1];
            [
                val(b0),
                val(b0) + der(b0) * third,
                val(b1) - der(b1) * third,
                val(b1),
            ]
        };
        let a = bern(&sp[0]);
        let mut out = Hull {
            pts: [Point3::ORIGIN; 12],
            w: [1.0; 3],
            n: 2,
        };
        if !self.arcs {
            let e = bern(&sp[3]);
            for r in 0..4 {
                out.pts[2 * r] = Point3::ORIGIN + a[r] + e[r] * uv.u.lo;
                out.pts[2 * r + 1] = Point3::ORIGIN + a[r] + e[r] * uv.u.hi;
            }
            return Some(out);
        }
        if sp[3].iter().any(|v| *v != Vec3::ZERO) {
            return None;
        }
        let (x0, x1) = (
            self.start + self.sweep * uv.u.lo,
            self.start + self.sweep * uv.u.hi,
        );
        let half = 0.5 * (x1 - x0);
        if half.abs() >= 0.49 * forge_math::dmath::PI {
            return None;
        }
        let ch = forge_math::dmath::cos(half);
        let (s0, c0) = forge_math::dmath::sin_cos(x0);
        let (sm, cm) = forge_math::dmath::sin_cos(x0 + half);
        let (s1, c1) = forge_math::dmath::sin_cos(x1);
        let (bb, dd) = (bern(&sp[1]), bern(&sp[2]));
        for (j, (cx, sy)) in [(c0, s0), (cm / ch, sm / ch), (c1, s1)].into_iter().enumerate() {
            for r in 0..4 {
                out.pts[3 * r + j] = Point3::ORIGIN + a[r] + bb[r] * cx + dd[r] * sy;
            }
        }
        out.w = [1.0, ch, 1.0];
        out.n = 3;
        Some(out)
    }
}

impl Data {
    fn new(rails: Vec<Rail>, slopes: Vec<Vec3>, spline: Arc<Spline>, shifts: Vec<Shift>) -> Data {
        let poly = Poly::new(&rails, &slopes, &spline, &shifts);
        Data {
            rails,
            slopes,
            spline,
            shifts,
            poly,
        }
    }

    fn point(&self, uv: Point2) -> SurfDeriv {
        if let Some(p) = &self.poly {
            return p.point(&self.spline, uv);
        }
        let w = self.spline.weights(uv.y);
        let mut d = SurfDeriv {
            p: Point3::ORIGIN,
            du: Vec3::ZERO,
            dv: Vec3::ZERO,
            duu: Vec3::ZERO,
            duv: Vec3::ZERO,
            dvv: Vec3::ZERO,
        };
        let mut p = Vec3::ZERO;
        // matched arcs usually turn through the same angles: one sin_cos serves them all
        let mut trig: Option<(f64, (f64, f64))> = None;
        for (k, r) in self.rails.iter().enumerate() {
            let (c, c1, c2) = match *r {
                Rail::Arc { circle, start, sweep } => {
                    let a = start + sweep * uv.x;
                    let (sn, cs) = match trig {
                        Some((b, sc)) if b == a => sc,
                        _ => {
                            let sc = forge_math::dmath::sin_cos(a);
                            trig = Some((a, sc));
                            sc
                        }
                    };
                    let (f, rad) = (&circle.frame, circle.radius);
                    let (x, y) = (f.x().get(), f.y().get());
                    let k1 = rad * sweep;
                    (
                        f.origin + (x * cs + y * sn) * rad,
                        (y * cs - x * sn) * k1,
                        (x * cs + y * sn) * (-k1 * sweep),
                    )
                }
                _ => r.at(uv.x),
            };
            let c = c.to_vec();
            let [w0, w1, w2] = w[k];
            p += c * w0;
            d.du += c1 * w0;
            d.duu += c2 * w0;
            d.dv += c * w1;
            d.duv += c1 * w1;
            d.dvv += c * w2;
        }
        let n = self.rails.len();
        for (j, &s) in self.slopes.iter().enumerate() {
            let [w0, w1, w2] = w[n + j];
            p += s * w0;
            d.dv += s * w1;
            d.dvv += s * w2;
        }
        let m = self.shifts.first().map(|sh| sh.map.weights(uv.y));
        for sh in &self.shifts {
            let [e0, e1, e2] = sh.at(&w, m.as_ref().expect("a guide's map"), &self.slopes);
            let (b0, b1) = sh.beta(uv.x);
            p += e0 * b0;
            d.du += e0 * b1;
            d.dv += e1 * b0;
            d.duv += e1 * b1;
            d.dvv += e2 * b0;
        }
        d.p = Point3::ORIGIN + p;
        d
    }

    /// A box holding the points over `uv`, from boxes of 4 x 4 pieces of it (the weights'
    /// boxes are loose over wide ranges).
    fn bbox(&self, uv: Box2) -> Box3 {
        let mut b = Box3::EMPTY;
        let piece = |x: Interval, k: usize| {
            let w = x.width() / 4.0;
            Interval::new(
                x.lo + w * k as f64,
                if k == 3 { x.hi } else { x.lo + w * (k + 1) as f64 },
            )
        };
        for i in 0..4 {
            for j in 0..4 {
                b = b.hull(&self.piece_box(Box2::new(piece(uv.u, i), piece(uv.v, j))));
            }
        }
        b
    }

    fn piece_box(&self, uv: Box2) -> Box3 {
        if let Some(p) = &self.poly {
            return p.bbox(&self.spline, uv);
        }
        let w = self.spline.weight_box(uv.v, 0);
        // the values' weights sum to one: measured from a point of the piece, the sum stays
        // as small as the piece
        let o = self.point(Point2::new(uv.u.mid(), uv.v.mid())).p;
        let mut b = Box3 {
            x: Interval::point(o.x),
            y: Interval::point(o.y),
            z: Interval::point(o.z),
        };
        let mut add = |w: Interval, c: Box3| {
            b.x = b.x + w * c.x;
            b.y = b.y + w * c.y;
            b.z = b.z + w * c.z;
        };
        let rel = |c: Box3| Box3 {
            x: c.x - Interval::point(o.x),
            y: c.y - Interval::point(o.y),
            z: c.z - Interval::point(o.z),
        };
        for (k, r) in self.rails.iter().enumerate() {
            add(w[k], rel(r.bbox(uv.u)));
        }
        let n = self.rails.len();
        for (j, &s) in self.slopes.iter().enumerate() {
            add(w[n + j], Box3::from_point(Point3::ORIGIN + s));
        }
        for sh in &self.shifts {
            let beta = match sh.along {
                Along::Start => Interval::point(1.0) - uv.u,
                Along::End => uv.u,
                Along::Whole => Interval::point(1.0),
            };
            add(beta, sh.bbox(uv.v, &w, &self.slopes));
        }
        b.grow(tol::LINEAR)
    }

    fn transformed(&self, f: &Frame) -> Data {
        Data::new(
            self.rails.iter().map(|r| r.transformed(f)).collect(),
            self.slopes.iter().map(|&s| f.vec_to_world(s)).collect(),
            self.spline.clone(),
            self.shifts.iter().map(|sh| sh.transformed(f)).collect(),
        )
    }
}

/// A piece of a span's control points: four rows (Bernstein in v) of two points (lines, in t) or of
/// three (arcs: a rational quadratic, weights 1, cos h, 1, h half the piece's turn). The piece lies
/// in their hull.
pub(crate) struct Hull {
    pts: [Point3; 12],
    w: [f64; 3],
    n: usize,
}

impl Hull {
    fn rows(&self) -> &[Point3] {
        &self.pts[..4 * self.n]
    }

    pub(crate) fn bbox(&self) -> Box3 {
        Box3::from_points(self.rows().iter().copied())
    }

    pub(crate) fn centre(&self) -> Vec3 {
        self.rows().iter().fold(Vec3::ZERO, |s, p| s + p.to_vec()) / (4 * self.n) as f64
    }

    pub(crate) fn along_u(&self) -> f64 {
        let n = self.n;
        self.pts[n].distance(self.pts[2 * n - 1]) + self.pts[2 * n].distance(self.pts[3 * n - 1])
    }

    pub(crate) fn along_v(&self) -> f64 {
        let c = self.n / 2;
        self.pts[c].distance(self.pts[3 * self.n + c])
    }

    pub(crate) fn normal(&self) -> Vec3 {
        let (n, c) = (self.n, self.n / 2);
        (self.pts[3 * n + c] - self.pts[c]).cross(self.pts[2 * n - 1] - self.pts[n])
    }

    pub(crate) fn extent(&self, u: Vec3) -> Interval {
        self.rows().iter().fold(Interval::EMPTY, |mut i, p| {
            i.include(p.to_vec().dot(u));
            i
        })
    }

    /// The quadric q.A.q + 2 b.q + c has one sign over the piece: Q(S) w^2 is a polynomial of
    /// degree 6 in v and 2(n - 1) in t, every Bernstein coefficient of it the same sign.
    pub(crate) fn misses(&self, q: &crate::meet::Alg) -> bool {
        let crate::meet::Alg::Quadric { o, a, b, c } = *q else {
            return false;
        };
        const C3: [f64; 4] = [1.0, 3.0, 3.0, 1.0];
        const C6: [f64; 7] = [1.0, 6.0, 15.0, 20.0, 15.0, 6.0, 1.0];
        let n = self.n;
        let (cn, c2n): (&[f64], &[f64]) = if n == 3 {
            (&[1.0, 2.0, 1.0], &[1.0, 4.0, 6.0, 4.0, 1.0])
        } else {
            (&[1.0, 1.0], &[1.0, 2.0, 1.0])
        };
        let h = |i: usize, j: usize| (self.pts[i * n + j] - o) * self.w[j];
        let mut coef = [[0.0; 5]; 7];
        for i in 0..4 {
            for j in 0..n {
                let (hij, ah) = (h(i, j), crate::meet::mul(&a, h(i, j)));
                for k in 0..4 {
                    for l in 0..n {
                        let v = h(k, l).dot(ah) + 2.0 * b.dot(hij) * self.w[l] + c * self.w[j] * self.w[l];
                        coef[i + k][j + l] += v * C3[i] * C3[k] * cn[j] * cn[l] / (C6[i + k] * c2n[j + l]);
                    }
                }
            }
        }
        let all = coef.iter().flat_map(|r| &r[..2 * n - 1]);
        let eps = 1e-12 * all.clone().fold(0.0f64, |m, x| m.max(x.abs()));
        all.clone().all(|&x| x > eps) || all.clone().all(|&x| x < -eps)
    }
}

impl Skinned {
    /// The control points of the face's piece over `uv` (inside its domain); see `Hull`.
    pub(crate) fn hull(&self, uv: Box2) -> Option<Hull> {
        let poly = self.data.poly.as_ref()?;
        let sp = &self.data.spline;
        let i = sp.span(self.span.mid());
        poly.hull(i, sp.knots[i], sp.knots[i + 1] - sp.knots[i], uv)
    }

    /// The face's domain carried on as far as the surface stays regular.
    pub(crate) fn reach(&self) -> Box2 {
        self.ext
    }

    /// The angle the arcs turn through across the face (none for lines).
    pub(crate) fn turn(&self) -> f64 {
        self.data
            .poly
            .as_ref()
            .map_or(0.0, |p| if p.arcs { p.sweep } else { 0.0 })
    }
}

/// Steps of the seed grid over the carried-on surface, in t and in s.
const GRID: (usize, usize) = (16, 8);
const ROWS: usize = (GRID.0 + 1) * (GRID.1 + 1);

/// One span (between two neighbouring sections) of a smooth loft's wall.
#[derive(Clone, Debug)]
pub struct Skinned {
    data: Data,
    /// The face's s range, and the carried-on box.
    span: Interval,
    ext: Box2,
    /// Points on a grid over `ext`, seeding nearest-point searches.
    rows: Vec<(Point2, SurfDeriv)>,
    /// The rows' points alone (the seed scan reads only these).
    pts: Vec<Point3>,
    scale: (f64, f64),
    radius: f64,
    /// The surface bends tighter than its seed grid is fine: its nearest points are sought from
    /// two seeds in different places.
    sharp: bool,
    /// Which surface this is, for remembering its last query.
    id: u64,
    patches: std::sync::OnceLock<Vec<Box3>>,
}

impl Skinned {
    /// The span of the surface through `rails` (matched section curves, one per knot) between
    /// knots `k` and `k + 1`, with end slopes `slopes` (as the spline takes them). None where
    /// it folds over or pinches.
    pub fn new(
        rails: Vec<Rail>,
        slopes: Vec<Vec3>,
        spline: Arc<Spline>,
        k: usize,
        shifts: Vec<Shift>,
    ) -> Option<Skinned> {
        let knots = spline.knots().to_vec();
        let span = Interval::new(knots[k], knots[k + 1]);
        let te = rails.iter().map(Rail::reach).fold(0.25, f64::min);
        let mut s = Skinned {
            data: Data::new(rails, slopes, spline, shifts),
            span,
            ext: Box2::new(Interval::new(0.0, 1.0), span),
            rows: Vec::new(),
            pts: Vec::new(),
            scale: (0.0, 0.0),
            radius: f64::INFINITY,
            sharp: false,
            id: fresh_id(),
            patches: std::sync::OnceLock::new(),
        };
        let regular = |s: &Skinned, b: Box2| {
            let (nu, nv) = (32, 32);
            let grid: Vec<Vec<SurfDeriv>> = (0..=nu)
                .map(|i| {
                    (0..=nv)
                        .map(|j| {
                            s.data.point(Point2::new(
                                b.u.lo + b.u.width() * i as f64 / nu as f64,
                                b.v.lo + b.v.width() * j as f64 / nv as f64,
                            ))
                        })
                        .collect()
                })
                .collect();
            let n = |d: &SurfDeriv| d.du.cross(d.dv);
            let scale = grid.iter().flatten().map(|d| n(d).length()).fold(0.0, f64::max);
            // the normal neither vanishes nor turns over, and neither direction doubles back,
            // between neighbouring grid points
            let agree = |a: &SurfDeriv, c: &SurfDeriv| {
                n(a).dot(n(c)) > 0.0 && a.du.dot(c.du) > 0.0 && a.dv.dot(c.dv) > 0.0
            };
            grid.iter().flatten().all(|d| n(d).length() > 1e-9 * scale)
                && (0..=nu).all(|i| {
                    (0..=nv).all(|j| {
                        (i == nu || agree(&grid[i][j], &grid[i + 1][j]))
                            && (j == nv || agree(&grid[i][j], &grid[i][j + 1]))
                    })
                })
        };
        if !regular(&s, s.ext) {
            return None;
        }
        // carried on as far as it stays regular
        let h = span.width();
        for (eu, ev) in [(te, 0.5 * h), (0.5 * te, 0.25 * h), (0.0, 0.0)] {
            let b = Box2::new(
                Interval::new(-eu, 1.0 + eu),
                Interval::new(span.lo - ev, span.hi + ev),
            );
            if regular(&s, b) {
                s.ext = b;
                break;
            }
        }
        let (nu, nv) = (GRID.0, GRID.1);
        for i in 0..=nu {
            for j in 0..=nv {
                let uv = Point2::new(
                    s.ext.u.lo + s.ext.u.width() * i as f64 / nu as f64,
                    s.ext.v.lo + s.ext.v.width() * j as f64 / nv as f64,
                );
                s.rows.push((uv, s.data.point(uv)));
            }
        }
        s.pts = s.rows.iter().map(|r| r.1.p).collect();
        let (mut kmax, mut su, mut sv) = (0.0f64, 0.0f64, 0.0f64);
        for i in 0..=8 {
            for j in 0..=4 {
                let d = s
                    .data
                    .point(Point2::new(i as f64 / 8.0, span.lo + h * j as f64 / 4.0));
                kmax = kmax.max(principal_max(&d));
                su = su.max(d.du.length());
                sv = sv.max(d.dv.length());
            }
        }
        s.scale = (su, sv);
        s.radius = if kmax > 0.0 { 1.0 / kmax } else { f64::INFINITY };
        let cell = (0..nu * (nv + 1))
            .map(|k| s.rows[k].1.p.distance(s.rows[k + nv + 1].1.p))
            // along s within a column (not from one column's end to the next one's start)
            .chain(
                (0..s.rows.len() - 1)
                    .filter(|k| (k + 1) % (nv + 1) != 0)
                    .map(|k| s.rows[k].1.p.distance(s.rows[k + 1].1.p)),
            )
            .fold(0.0, f64::max);
        s.sharp = s.radius < 0.1 * cell;
        Some(s)
    }

    pub fn surface(self) -> Surface {
        Surface::Procedural(Proc(Arc::new(self)))
    }

    fn point(&self, uv: Point2) -> SurfDeriv {
        self.data.point(uv)
    }

    /// The nearest point of the carried-on surface, from the nearest grid point, and on a sharp
    /// surface also from the nearest grid point away from that one's neighbours.
    fn solve(&self, p: Point3) -> (Point2, SurfDeriv) {
        let mut dist = [0.0; ROWS];
        for (d, q) in dist.iter_mut().zip(&self.pts) {
            *d = q.distance_sq(p);
        }
        let nearest = |keep: &dyn Fn(usize) -> bool, none: usize| {
            (0..ROWS)
                .filter(|&m| keep(m))
                .map(|m| (m, dist[m]))
                .fold((none, f64::INFINITY), |b, x| if x.1 < b.1 { x } else { b })
        };
        let (k, _) = nearest(&|_| true, 0);
        let from = |k: usize| {
            polish(
                &|uv| self.point(uv),
                p,
                self.rows[k].0,
                self.ext,
                Some(self.rows[k].1),
            )
        };
        let best = from(k);
        if !self.sharp {
            return best;
        }
        let w = GRID.1 + 1;
        let (i0, j0) = ((k / w) as i64, (k % w) as i64);
        let other = nearest(
            &|m| ((m / w) as i64 - i0).abs() > 1 || ((m % w) as i64 - j0).abs() > 1,
            k,
        );
        if other.1.is_infinite() {
            return best;
        }
        let alt = from(other.0);
        if alt.1.p.distance_sq(p) < best.1.p.distance_sq(p) {
            alt
        } else {
            best
        }
    }

    fn nearest(&self, p: Point3) -> Point2 {
        self.solve(p).0
    }

    fn value(&self, p: Point3) -> (f64, Vec3) {
        remembered(self.id, p, || {
            let (_, d) = self.solve(p);
            let n = d.du.cross(d.dv).normalize().map_or(Vec3::Z, |u| u.get());
            ((p - d.p).dot(n), n)
        })
    }

    /// The rail through the sections' points at fraction t of each, over the face's span, and a
    /// fence through it across the faces it bounds: lines `across[k]` (unit, in section k's
    /// plane) `half` long either way of each point, skinned the same way.
    pub fn rail(&self, across: &[Vec3], half: f64) -> crate::Curve {
        let points: Vec<Point3> = self.data.rails.iter().map(|r| r.at(0.0).0).collect();
        // along the wall's start it takes the whole of the guides' pulls there
        let shifts: Vec<Shift> = self
            .data
            .shifts
            .iter()
            .filter(|sh| sh.along != Along::End)
            .map(|sh| sh.with(Along::Whole))
            .collect();
        let fence = Skinned::new(
            points
                .iter()
                .zip(across)
                .map(|(&p, &a)| Rail::Line {
                    a: p - a * half,
                    b: p + a * half,
                })
                .collect(),
            self.data.slopes.clone(),
            self.data.spline.clone(),
            self.data.spline.span(self.span.mid()),
            shifts.clone(),
        )
        .map(Skinned::surface);
        crate::Curve::Procedural(ProcCurve(Arc::new(SplineRail {
            data: Data::new(
                points.iter().map(|&p| Rail::Line { a: p, b: p }).collect(),
                self.data.slopes.clone(),
                self.data.spline.clone(),
                shifts,
            ),
            span: self.span,
            fence,
        })))
    }
}

impl Procedural for Skinned {
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        self.point(uv)
    }
    fn domain(&self) -> Domain2 {
        Domain2 {
            u: Domain1 {
                lo: 0.0,
                hi: 1.0,
                period: None,
            },
            v: Domain1 {
                lo: self.span.lo,
                hi: self.span.hi,
                period: None,
            },
        }
    }
    fn project(&self, p: Point3) -> Point2 {
        self.nearest(p)
    }
    fn value_near(&self, p: Point3, uv: Option<Point2>) -> Option<(f64, Vec3, Point2)> {
        let near = uv
            .map(|uv| polish(&|uv| self.point(uv), p, uv, self.ext, None))
            .filter(|(_, d)| d.p.distance(p) <= tol::LINEAR);
        let (q, d) = near.unwrap_or_else(|| self.solve(p));
        let n = d.du.cross(d.dv).normalize().map_or(Vec3::Z, |u| u.get());
        Some(((p - d.p).dot(n), n, q))
    }
    fn project_near(&self, p: Point3, uv: Point2) -> Point2 {
        // a point lying on the surface is its own nearest point: Newton from nearby finds it
        let (q, d) = polish(&|uv| self.point(uv), p, uv, self.ext, None);
        if d.p.distance(p) <= tol::LINEAR {
            q
        } else {
            self.nearest(p)
        }
    }
    fn bbox(&self, uv: Box2) -> Box3 {
        self.data.bbox(uv)
    }
    fn implicit(&self, p: Point3) -> f64 {
        self.value(p).0
    }
    fn gradient(&self, p: Point3) -> Vec3 {
        self.value(p).1
    }
    fn transformed(&self, f: &Frame) -> Proc {
        let mut s = self.clone();
        s.id = fresh_id();
        s.patches = std::sync::OnceLock::new();
        s.data = self.data.transformed(f);
        s.rows = self
            .rows
            .iter()
            .map(|&(uv, d)| {
                let v = |x: Vec3| f.vec_to_world(x);
                let moved = SurfDeriv {
                    p: f.point_to_world(d.p),
                    du: v(d.du),
                    dv: v(d.dv),
                    duu: v(d.duu),
                    duv: v(d.duv),
                    dvv: v(d.dvv),
                };
                (uv, moved)
            })
            .collect();
        s.pts = s.rows.iter().map(|r| r.1.p).collect();
        Proc(Arc::new(s))
    }
    fn uv_scale(&self) -> (f64, f64) {
        (self.scale.0.max(tol::LINEAR), self.scale.1.max(tol::LINEAR))
    }
    fn shape_bounds(&self, uv: Box2) -> Option<ShapeBounds> {
        // S = sum w_k(v) C_k(u) + sum w_j(v) D_j over the sections C_k and end slopes D_j, so
        // S_(u^a v^b) = sum w_k^(b) C_k^(a) (the slopes' only for a = 0); the values' weights sum
        // to one, so for b > 0 the sum is of w_k^(b) (C_k^(a) - C_0^(a)), each difference within
        // its middle's plus the next derivatives' bounds over the half width (a guide's pull: none)
        let d = &self.data;
        if !d.shifts.is_empty() {
            return None;
        }
        let (u, hu) = (uv.u.mid(), 0.5 * uv.u.width());
        let bound: Vec<[f64; 3]> = d
            .rails
            .iter()
            .map(|r| r.derivative_bounds(uv.u))
            .collect::<Option<_>>()?;
        let at: Vec<(Point3, Vec3, Vec3)> = d.rails.iter().map(|r| r.at(u)).collect();
        let diff = |k: usize| {
            if k == 0 {
                return [0.0; 3];
            }
            let (p, d1, d2) = (at[k].0 - at[0].0, at[k].1 - at[0].1, at[k].2 - at[0].2);
            let (bk, b0) = (bound[k], bound[0]);
            [
                p.length() + (bk[0] + b0[0]) * hu,
                d1.length() + (bk[1] + b0[1]) * hu,
                d2.length() + (bk[2] + b0[2]) * hu,
            ]
        };
        let diffs: Vec<[f64; 3]> = (0..d.rails.len()).map(diff).collect();
        let mag = |i: Interval| i.lo.abs().max(i.hi.abs());
        let w: Vec<Vec<f64>> = (0..4)
            .map(|o| d.spline.weight_box(uv.v, o).into_iter().map(mag).collect())
            .collect();
        let n = d.rails.len();
        let sum = |o: usize, f: &dyn Fn(usize) -> f64| (0..n).map(|k| w[o][k] * f(k)).sum::<f64>();
        let slopes = |o: usize| {
            d.slopes
                .iter()
                .zip(&w[o][n..])
                .map(|(s, &x)| s.length() * x)
                .sum::<f64>()
        };
        Some(ShapeBounds::taylor(
            &self.point(Point2::new(u, uv.v.mid())),
            (hu, 0.5 * uv.v.width()),
            [
                sum(0, &|k| bound[k][1]),
                sum(1, &|k| diffs[k][1]),
                sum(2, &|k| diffs[k][0]) + slopes(2),
            ],
            [
                sum(0, &|k| bound[k][2]),
                sum(1, &|k| diffs[k][2]),
                sum(2, &|k| diffs[k][1]),
                sum(3, &|k| diffs[k][0]) + slopes(3),
            ],
        ))
    }
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
    fn min_radius(&self) -> f64 {
        let arcs = self
            .data
            .rails
            .iter()
            .filter_map(|r| match *r {
                Rail::Arc { circle, .. } => Some(circle.radius),
                _ => None,
            })
            .fold(f64::INFINITY, f64::min);
        0.8 * self.radius.min(arcs)
    }
    fn patches(&self) -> Option<&[Box3]> {
        // faces lie on the span itself, never on the carried-on part
        Some(self.patches.get_or_init(|| {
            let (nu, nv) = GRID;
            let cell = |x: Interval, k: usize, n: usize| {
                let w = x.width() / n as f64;
                Interval::new(
                    x.lo + w * k as f64,
                    if k + 1 == n {
                        x.hi
                    } else {
                        x.lo + w * (k + 1) as f64
                    },
                )
            };
            let whole = Interval::new(0.0, 1.0);
            (0..nu)
                .flat_map(|i| (0..nv).map(move |j| (i, j)))
                .map(|(i, j)| {
                    self.data
                        .bbox(Box2::new(cell(whole, i, nu), cell(self.span, j, nv)))
                })
                .collect()
        }))
    }
}

/// The spline through the sections' matched points: the rail between two smooth-loft faces.
#[derive(Debug)]
struct SplineRail {
    /// The points, as rails of no length.
    data: Data,
    span: Interval,
    fence: Option<Surface>,
}

impl SplineRail {
    fn at(&self, s: f64) -> SurfDeriv {
        self.data.point(Point2::new(0.0, s))
    }

    /// A bound on |P'| (`order` 1) or |P''| (2) over s: the derivatives' weights of the points
    /// sum to zero, so the points count from the first.
    fn bound(&self, s: Interval, order: usize) -> f64 {
        let w = self.data.spline.weight_box(s, order);
        let n = self.data.rails.len();
        let mag = |i: Interval| i.lo.abs().max(i.hi.abs());
        let o = self.data.rails[0].at(0.0).0;
        let pts: f64 = self
            .data
            .rails
            .iter()
            .zip(&w)
            .map(|(r, &wi)| (r.at(0.0).0 - o).length() * mag(wi))
            .sum();
        pts + self
            .data
            .slopes
            .iter()
            .zip(&w[n..])
            .map(|(s, &wi)| s.length() * mag(wi))
            .sum::<f64>()
    }
}

impl ProceduralCurve for SplineRail {
    fn deriv(&self, t: f64) -> CurveDeriv {
        let d = self.at(t);
        CurveDeriv {
            p: d.p,
            d1: d.dv,
            d2: d.dvv,
        }
    }
    fn domain(&self) -> Domain1 {
        Domain1 {
            lo: self.span.lo,
            hi: self.span.hi,
            period: None,
        }
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        let n = 32;
        let mut best = (f64::INFINITY, range.lo);
        for i in 0..=n {
            let s = range.lo + range.width() * i as f64 / n as f64;
            let d = self.at(s).p.distance_sq(p);
            if d < best.0 {
                best = (d, s);
            }
        }
        let (mut s, mut f) = (best.1, best.0);
        for _ in 0..12 {
            let d = self.at(s);
            let e = d.p - p;
            let (g, h) = (e.dot(d.dv), d.dv.length_sq() + e.dot(d.dvv));
            let h = if h > 0.0 { h } else { d.dv.length_sq() };
            let next = (s - g / h).clamp(range.lo, range.hi);
            if next == s {
                break;
            }
            let fnext = self.at(next).p.distance_sq(p);
            if fnext > f {
                break;
            }
            (s, f) = (next, fnext);
        }
        s
    }
    fn bbox(&self, range: Interval) -> Box3 {
        self.data.bbox(Box2::new(Interval::point(0.0), range))
    }
    fn transformed(&self, f: &Frame) -> ProcCurve {
        ProcCurve(Arc::new(SplineRail {
            data: self.data.transformed(f),
            span: self.span,
            fence: self.fence.as_ref().map(|s| crate::SurfaceGeom::transformed(s, f)),
        }))
    }
    fn speed_bound(&self, range: Interval) -> f64 {
        self.bound(range, 1) * (1.0 + 1e-9) + tol::LINEAR
    }
    fn segments(&self, range: Interval, chord: f64) -> usize {
        let acc = self.bound(range, 2);
        let n = range.width() * (acc / (8.0 * chord.max(tol::LINEAR))).sqrt();
        (n.ceil() as usize).max(1)
    }
    fn across(&self) -> Option<Surface> {
        self.fence.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Circle, SurfaceGeom};
    use forge_math::rng::Rng;

    fn random(r: &mut Rng) -> (Vec<Rail>, Arc<Spline>, Vec<Vec3>) {
        let n = 3 + r.below(3);
        let mut z = 0.0;
        let mut knots = Vec::new();
        let rails = (0..n)
            .map(|_| {
                z += r.range(3.0, 8.0);
                knots.push(z);
                let o = Point3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), z);
                if r.bool() {
                    Rail::Line {
                        a: o + Vec3::new(-r.range(2.0, 4.0), 0.0, 0.0),
                        b: o + Vec3::new(r.range(2.0, 4.0), r.range(-0.5, 0.5), 0.0),
                    }
                } else {
                    Rail::Arc {
                        circle: Circle {
                            frame: Frame::WORLD.with_origin(o),
                            radius: r.range(2.0, 5.0),
                        },
                        start: 0.0,
                        sweep: forge_math::dmath::PI,
                    }
                }
            })
            .collect::<Vec<_>>();
        // arcs and lines don't mix in one skin; keep one kind
        let rails = if rails.iter().all(|x| matches!(x, Rail::Arc { .. }))
            || rails.iter().all(|x| matches!(x, Rail::Line { .. }))
        {
            rails
        } else {
            rails
                .iter()
                .map(|x| match *x {
                    Rail::Arc { circle, .. } => Rail::Line {
                        a: circle.frame.origin - Vec3::X * circle.radius,
                        b: circle.frame.origin + Vec3::X * circle.radius,
                    },
                    l => l,
                })
                .collect()
        };
        let (s0, s1) = (r.bool(), r.bool());
        let slopes: Vec<Vec3> = [s0, s1]
            .iter()
            .filter(|&&b| b)
            .map(|_| Vec3::new(0.0, 0.0, r.range(0.5, 1.5)))
            .collect();
        (rails, Arc::new(Spline::new(knots, s0, s1).unwrap()), slopes)
    }

    #[test]
    fn per_span_form_matches_the_sum_and_boxes_it() {
        let mut r = Rng::new(11);
        for _ in 0..40 {
            let (rails, spline, slopes) = random(&mut r);
            let mut d = Data::new(rails, slopes, spline, Vec::new());
            assert!(d.poly.is_some());
            let k = d.spline.knots().to_vec();
            for _ in 0..50 {
                let uv = Point2::new(r.range(-0.25, 1.25), r.range(k[0] - 2.0, k[k.len() - 1] + 2.0));
                let fast = d.point(uv);
                let poly = d.poly.take();
                let sum = d.point(uv);
                d.poly = poly;
                let scale = 1.0 + sum.p.to_vec().abs_max();
                for (a, b) in [
                    (fast.p.to_vec(), sum.p.to_vec()),
                    (fast.du, sum.du),
                    (fast.dv, sum.dv),
                    (fast.duu, sum.duu),
                    (fast.duv, sum.duv),
                    (fast.dvv, sum.dvv),
                ] {
                    assert!((a - b).length() <= 1e-12 * scale, "{uv:?}: {a:?} vs {b:?}");
                }
            }
            for _ in 0..20 {
                let (u0, v0) = (r.range(-0.25, 1.0), r.range(k[0] - 2.0, k[k.len() - 1]));
                let uv = Box2::new(
                    Interval::new(u0, u0 + r.range(0.0, 0.5)),
                    Interval::new(v0, v0 + r.range(0.0, 8.0)),
                );
                let b = d.piece_box(uv);
                for i in 0..=6 {
                    for j in 0..=6 {
                        let q = Point2::new(
                            uv.u.lo + uv.u.width() * i as f64 / 6.0,
                            uv.v.lo + uv.v.width() * j as f64 / 6.0,
                        );
                        assert!(b.contains(d.point(q).p), "{uv:?} {q:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn spline_weights_interpolate_and_match_derivatives() {
        let sp = Spline::new(vec![0.0, 1.5, 4.0, 5.0], true, false).unwrap();
        for (k, &x) in sp.knots().iter().enumerate() {
            let w = sp.weights(x);
            for (d, wd) in w.iter().enumerate().take(4) {
                assert!((wd[0] - f64::from(d == k)).abs() < 1e-12);
            }
        }
        // the start slope datum sets the start derivative alone
        let w = sp.weights(0.0);
        assert!((w[4][1] - 1.0).abs() < 1e-12 && w[..4].iter().all(|x| x[1].abs() < 1e-12));
        // continuity of the second derivative at a middle knot
        let (a, b) = (sp.weights(1.5 - 1e-12), sp.weights(1.5 + 1e-12));
        for (x, y) in a.iter().zip(&b).take(5) {
            assert!((x[2] - y[2]).abs() < 1e-6, "{x:?} {y:?}");
        }
    }

    #[test]
    fn derivatives_project_bbox_and_rails() {
        let mut r = Rng::new(530);
        let mut made = 0;
        for _ in 0..80 {
            let (rails, sp, slopes) = random(&mut r);
            let k = r.below(sp.knots().len() - 1);
            let Some(s) = Skinned::new(rails, slopes, sp, k, vec![]) else {
                continue;
            };
            made += 1;
            let surf = s.clone().surface();
            let d = s.domain();
            for _ in 0..10 {
                let uv = Point2::new(r.range(0.0, 1.0), r.range(d.v.lo, d.v.hi));
                let q = s.point(uv);
                let h = 1e-6;
                let at = |du: f64, dv: f64| s.point(Point2::new(uv.x + du, uv.y + dv));
                let close =
                    |a: Vec3, b: Vec3| assert!((a - b).length() < 1e-4 * (1.0 + b.length()), "{a:?} {b:?}");
                close((at(h, 0.0).p - at(-h, 0.0).p) / (2.0 * h), q.du);
                close((at(0.0, h).p - at(0.0, -h).p) / (2.0 * h), q.dv);
                close((at(0.0, h).dv - at(0.0, -h).dv) / (2.0 * h), q.dvv);
                close((at(0.0, h).du - at(0.0, -h).du) / (2.0 * h), q.duv);
                assert!(surf.eval(surf.project(q.p)).distance(q.p) < 1e-9);
                let near = Point2::new(uv.x + r.range(-0.05, 0.05), uv.y + r.range(-0.5, 0.5));
                assert!(s.project_near(q.p, near).distance(surf.project(q.p)) < 1e-9);
                let n = q.du.cross(q.dv).normalize().unwrap();
                // within the local radius of curvature, the step along the normal reads its length
                let k = principal_max(&q);
                let t = r.range(-0.05, 0.05) * (0.3 / (k * 0.05)).min(1.0);
                assert!(
                    (s.value(q.p + n * t).0 - t).abs() < 1e-8,
                    "{} {t}",
                    s.value(q.p + n * t).0
                );
                let bx = Procedural::bbox(
                    &s,
                    Box2::new(Interval::new(0.0, 1.0), Interval::new(d.v.lo, d.v.hi)),
                );
                assert!(bx.contains(q.p));
            }
            let crate::Curve::Procedural(rail) = s.rail(&vec![Vec3::X; s.data.rails.len()], 1.0) else {
                unreachable!()
            };
            let range = Interval::new(d.v.lo, d.v.hi);
            let b = rail.bbox(range);
            for i in 0..=20 {
                let x = range.lo + range.width() * i as f64 / 20.0;
                let c = rail.deriv(x);
                let on = s.point(Point2::new(0.0, x)).p;
                assert!(on.distance(c.p) < 1e-9, "rail off its surface: {on:?} {:?}", c.p);
                assert!(surf.distance(c.p) < 1e-9);
                assert!(b.contains(c.p));
                assert!(c.d1.length() <= rail.speed_bound(range));
                assert!(rail.deriv(rail.closest(c.p, range)).p.distance(c.p) < 1e-9);
            }
        }
        assert!(made > 20, "{made}");
    }

    #[test]
    fn guided_derivatives_match_finite_differences() {
        let line = |z: f64, h: f64| Rail::Line {
            a: Point3::new(-h, -h, z),
            b: Point3::new(h, -h, z),
        };
        let knots = vec![0.0, 5.0, 9.0];
        let sp = Arc::new(Spline::new(knots.clone(), true, false).unwrap());
        // a guide up z, then bending towards +x
        let bend = Rail::Arc {
            circle: Circle {
                frame: Frame::from_zx(
                    Point3::new(20.0, 0.0, 3.0),
                    forge_math::UnitVec3::Y,
                    Vec3::new(-1.0, 0.0, 0.0),
                )
                .unwrap(),
                radius: 20.0,
            },
            start: 0.0,
            sweep: 0.7,
        };
        let g = Arc::new(GuidePath::new(&[
            Rail::Line {
                a: Point3::new(0.0, 0.0, -2.0),
                b: Point3::new(0.0, 0.0, 3.0),
            },
            bend,
        ]));
        let ps: Vec<f64> = [0.0, 5.0, 9.0]
            .iter()
            .map(|&z| g.crossings(Point3::new(0.0, 0.0, z), Vec3::Z)[0])
            .collect();
        let vals: Vec<Point3> = ps.iter().map(|&p| g.at(p).0).collect();
        for along in [Along::Start, Along::End, Along::Whole] {
            let shift = Shift::new(g.clone(), ps.clone(), vals.clone(), &knots, along).unwrap();
            let s = Skinned::new(
                vec![line(0.0, 4.0), line(5.0, 2.0), line(9.0, 3.0)],
                vec![Vec3::Z],
                sp.clone(),
                1,
                vec![shift],
            )
            .unwrap();
            for &uv in &[Point2::new(0.3, 6.0), Point2::new(0.8, 8.1)] {
                let q = s.point(uv);
                let h = 1e-6;
                let at = |du: f64, dv: f64| s.point(Point2::new(uv.x + du, uv.y + dv));
                let close =
                    |a: Vec3, b: Vec3| assert!((a - b).length() < 1e-4 * (1.0 + b.length()), "{a:?} {b:?}");
                close((at(h, 0.0).p - at(-h, 0.0).p) / (2.0 * h), q.du);
                close((at(0.0, h).p - at(0.0, -h).p) / (2.0 * h), q.dv);
                close((at(0.0, h).dv - at(0.0, -h).dv) / (2.0 * h), q.dvv);
                close((at(0.0, h).du - at(0.0, -h).du) / (2.0 * h), q.duv);
                let b = Procedural::bbox(&s, Box2::new(Interval::new(0.0, 1.0), Interval::new(5.0, 9.0)));
                assert!(b.contains(q.p));
            }
            // the guide's pull vanishes at the sections
            let sh = &s.data.shifts[0];
            let (w, m) = (s.data.spline.weights(5.0), sh.map.weights(5.0));
            assert!(sh.at(&w, &m, &s.data.slopes)[0].length() < 1e-12);
        }
    }
}
