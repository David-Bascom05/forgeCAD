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
use crate::screw::polish;
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

/// The data a skinned surface or rail sums: section curves (or points) and end slopes.
#[derive(Clone, Debug)]
struct Data {
    rails: Vec<Rail>,
    slopes: Vec<Vec3>,
    spline: Arc<Spline>,
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
        for (k, r) in self.rails.iter().enumerate() {
            let (c, c1, c2) = r.at(uv.x);
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
        let mut b = Box3 {
            x: Interval::point(0.0),
            y: Interval::point(0.0),
            z: Interval::point(0.0),
        };
        let mut add = |w: Interval, c: Box3| {
            b.x = b.x + w * c.x;
            b.y = b.y + w * c.y;
            b.z = b.z + w * c.z;
        };
        for (k, r) in self.rails.iter().enumerate() {
            add(w[k], r.bbox(uv.u));
        }
        let n = self.rails.len();
        for (j, &s) in self.slopes.iter().enumerate() {
            add(w[n + j], Box3::from_point(Point3::ORIGIN + s));
        }
        b.grow(tol::LINEAR)
    }

    fn transformed(&self, f: &Frame) -> Data {
        Data {
            rails: self.rails.iter().map(|r| r.transformed(f)).collect(),
            slopes: self.slopes.iter().map(|&s| f.vec_to_world(s)).collect(),
            spline: self.spline.clone(),
        }
    }
}

/// One span (between two neighbouring sections) of a smooth loft's wall.
#[derive(Clone, Debug)]
pub struct Skinned {
    data: Data,
    /// The face's s range, and the carried-on box.
    span: Interval,
    ext: Box2,
    /// Points on a grid over `ext`, seeding nearest-point searches.
    rows: Vec<(Point2, Point3)>,
    scale: (f64, f64),
    radius: f64,
}

impl Skinned {
    /// The span of the surface through `rails` (matched section curves, one per knot) between
    /// knots `k` and `k + 1`, with end slopes `slopes` (as the spline takes them). None where
    /// it folds over or pinches.
    pub fn new(rails: Vec<Rail>, slopes: Vec<Vec3>, spline: Arc<Spline>, k: usize) -> Option<Skinned> {
        let knots = spline.knots().to_vec();
        let span = Interval::new(knots[k], knots[k + 1]);
        let te = rails.iter().map(Rail::reach).fold(0.25, f64::min);
        let mut s = Skinned {
            data: Data {
                rails,
                slopes,
                spline,
            },
            span,
            ext: Box2::new(Interval::new(0.0, 1.0), span),
            rows: Vec::new(),
            scale: (0.0, 0.0),
            radius: f64::INFINITY,
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
            // nor stalls along the loft (a near cusp)
            let fast = grid.iter().flatten().map(|d| d.dv.length()).fold(0.0, f64::max);
            grid.iter()
                .flatten()
                .all(|d| n(d).length() > 1e-9 * scale && d.dv.length() > 0.02 * fast)
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
        let (nu, nv) = (16, 8);
        for i in 0..=nu {
            for j in 0..=nv {
                let uv = Point2::new(
                    s.ext.u.lo + s.ext.u.width() * i as f64 / nu as f64,
                    s.ext.v.lo + s.ext.v.width() * j as f64 / nv as f64,
                );
                s.rows.push((uv, s.data.point(uv).p));
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
            .map(|(k, r)| (k, r.1.distance_sq(p)))
            .fold((0, f64::INFINITY), |b, x| if x.1 < b.1 { x } else { b })
    }

    fn nearest(&self, p: Point3) -> Point2 {
        let (k, _) = self.seed(p);
        polish(&|uv| self.point(uv), p, self.rows[k].0, self.ext)
    }

    fn value(&self, p: Point3) -> (f64, Vec3) {
        let (k, _) = self.seed(p);
        let d = self.point(polish(&|uv| self.point(uv), p, self.rows[k].0, self.ext));
        let n = d.du.cross(d.dv).normalize().map_or(Vec3::Z, |u| u.get());
        ((p - d.p).dot(n), n)
    }

    /// The rail through the sections' points at fraction t of each, over the face's span, and a
    /// fence through it across the faces it bounds: lines `across[k]` (unit, in section k's
    /// plane) `half` long either way of each point, skinned the same way.
    pub fn rail(&self, t: f64, across: &[Vec3], half: f64) -> crate::Curve {
        let points: Vec<Point3> = self.data.rails.iter().map(|r| r.at(t).0).collect();
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
        };
        let fence = Skinned::new(
            fence.rails,
            fence.slopes,
            fence.spline,
            self.data.spline.span(self.span.mid()),
        )
        .map(Skinned::surface);
        crate::Curve::Procedural(ProcCurve(Arc::new(SplineRail {
            data: Data {
                rails: points.iter().map(|&p| Rail::Line { a: p, b: p }).collect(),
                slopes: self.data.slopes.clone(),
                spline: self.data.spline.clone(),
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
        s.data = self.data.transformed(f);
        s.rows = self
            .rows
            .iter()
            .map(|&(uv, q)| (uv, f.point_to_world(q)))
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
                Rail::Line { .. } => None,
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
            let Some(s) = Skinned::new(rails, slopes, sp, k) else {
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
            let crate::Curve::Procedural(rail) = s.rail(0.0, &vec![Vec3::X; s.data.rails.len()], 1.0) else {
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
}
