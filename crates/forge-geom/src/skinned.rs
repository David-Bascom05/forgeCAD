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
use crate::surface::{Domain2, Proc, Procedural, SurfDeriv, Surface};
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

    /// Boxes of each datum's weight (`order` 0) or its derivative (1, 2) over s.
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
                    _ => a * mi + b * mj,
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
}

impl Data {
    fn point(&self, uv: Point2) -> SurfDeriv {
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
        Data {
            rails: self.rails.iter().map(|r| r.transformed(f)).collect(),
            slopes: self.slopes.iter().map(|&s| f.vec_to_world(s)).collect(),
            spline: self.spline.clone(),
            shifts: self.shifts.iter().map(|sh| sh.transformed(f)).collect(),
        }
    }
}

/// Steps of the seed grid over the carried-on surface, in t and in s.
const GRID: (usize, usize) = (16, 8);

/// One span (between two neighbouring sections) of a smooth loft's wall.
#[derive(Clone, Debug)]
pub struct Skinned {
    data: Data,
    /// The face's s range, and the carried-on box.
    span: Interval,
    ext: Box2,
    /// Points on a grid over `ext`, seeding nearest-point searches.
    rows: Vec<(Point2, SurfDeriv)>,
    scale: (f64, f64),
    radius: f64,
    /// The surface bends tighter than its seed grid is fine: its nearest points are sought from
    /// two seeds in different places.
    sharp: bool,
    /// Which surface this is, for remembering its last query.
    id: u64,
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
            data: Data {
                rails,
                slopes,
                spline,
                shifts,
            },
            span,
            ext: Box2::new(Interval::new(0.0, 1.0), span),
            rows: Vec::new(),
            scale: (0.0, 0.0),
            radius: f64::INFINITY,
            sharp: false,
            id: fresh_id(),
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
            .chain((0..s.rows.len() - 1).map(|k| s.rows[k].1.p.distance(s.rows[k + 1].1.p)))
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

    /// The row point nearest p: its index and squared distance.
    fn seed(&self, p: Point3) -> (usize, f64) {
        self.rows
            .iter()
            .enumerate()
            .map(|(k, r)| (k, r.1.p.distance_sq(p)))
            .fold((0, f64::INFINITY), |b, x| if x.1 < b.1 { x } else { b })
    }

    /// The nearest point of the carried-on surface, from the nearest grid point, and on a sharp
    /// surface also from the nearest grid point away from that one's neighbours.
    fn solve(&self, p: Point3) -> (Point2, SurfDeriv) {
        let (k, _) = self.seed(p);
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
        let other = (0..self.rows.len())
            .filter(|&m| ((m / w) as i64 - i0).abs() > 1 || ((m % w) as i64 - j0).abs() > 1)
            .map(|m| (m, self.rows[m].1.p.distance_sq(p)))
            .fold((k, f64::INFINITY), |b, x| if x.1 < b.1 { x } else { b });
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
        let fence = Data {
            rails: points
                .iter()
                .zip(across)
                .map(|(&p, &a)| Rail::Line {
                    a: p - a * half,
                    b: p + a * half,
                })
                .collect(),
            slopes: self.data.slopes.clone(),
            spline: self.data.spline.clone(),
            shifts: shifts.clone(),
        };
        let fence = Skinned::new(
            fence.rails,
            fence.slopes,
            fence.spline,
            self.data.spline.span(self.span.mid()),
            fence.shifts,
        )
        .map(Skinned::surface);
        crate::Curve::Procedural(ProcCurve(Arc::new(SplineRail {
            data: Data {
                rails: points.iter().map(|&p| Rail::Line { a: p, b: p }).collect(),
                slopes: self.data.slopes.clone(),
                spline: self.data.spline.clone(),
                shifts,
            },
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
        Proc(Arc::new(s))
    }
    fn uv_scale(&self) -> (f64, f64) {
        (self.scale.0.max(tol::LINEAR), self.scale.1.max(tol::LINEAR))
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
