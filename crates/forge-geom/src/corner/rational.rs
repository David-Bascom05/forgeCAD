//! Rational four-sided hole patches, for sides that are lines and conics on planes, cylinders
//! and spheres (and the spokes of a split hole). The domain is the unit square; side i has a
//! position s and a distance d (side 0: s = u, d = v, then counter-clockwise). Its ribbon is
//! R_i = (A + d B + d^2 C) / E: A / E the side's exact rational curve, B / E a cross tangent
//! in the neighbour's tangent plane (a polynomial combination of C' and a polynomial normal
//! field crossed with C'), C / E a bend matching the neighbouring sides to second order at the
//! corners. The blend is homogeneous, P = sum B_i A_i.. / sum B_i E_i with
//! B_i = prod_(j != i) d_j^2, so N and W are tensor polynomials of degree (7, 7) whatever the
//! hole. The fold proof bounds the directions of P_u and P_v by the Bernstein coefficients of
//! N and W (a cone test), each corner's quarter in Duffy coordinates where the weights' common
//! factor is divided out exactly. See docs/fold-proof.md.

// polynomial coefficient loops read best indexed
#![allow(clippy::needless_range_loop)]

use super::{Scalar, Side, SurfaceRail};
use crate::curve::{Curve, CurveGeom};
use crate::surface::Surface;
use forge_math::{Box3, Frame, Interval, Point2, Point3, Vec3, dmath};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const U: f64 = f64::EPSILON * 0.5;
/// The blend's degree in u and in v.
const DEG: usize = SDEG + 4;
/// The ribbons' degree in s.
const SDEG: usize = 3;
/// Within this of a corner (both distances) the nearest ribbon is the patch.
const CUT: f64 = 1e-6;
/// A power of two at least CUT: the corner zones the ribbons alone are proven over.
const ZONE: f64 = 1.0 / 524_288.0;

const BINOM: [[f64; 33]; 33] = {
    let mut t = [[0.0; 33]; 33];
    let mut n = 0;
    while n < 33 {
        t[n][0] = 1.0;
        let mut k = 1;
        while k <= n {
            t[n][k] = t[n - 1][k - 1] + t[n - 1][k];
            k += 1;
        }
        n += 1;
    }
    t
};

fn binom(n: usize, k: usize) -> f64 {
    BINOM[n][k]
}

/// A polynomial on [0, 1] in Bernstein form.
#[derive(Clone, Debug, PartialEq)]
struct B1(Vec<f64>);

impl B1 {
    fn konst(v: f64) -> B1 {
        B1(vec![v])
    }
    fn deg(&self) -> usize {
        self.0.len() - 1
    }
    fn mul(&self, o: &B1) -> B1 {
        let (m, n) = (self.deg(), o.deg());
        let mut c = vec![0.0; m + n + 1];
        for i in 0..=m {
            for j in 0..=n {
                c[i + j] += binom(m, i) * binom(n, j) / binom(m + n, i + j) * self.0[i] * o.0[j];
            }
        }
        B1(c)
    }
    fn elevate(&self, q: usize) -> B1 {
        let mut c = self.0.clone();
        while c.len() - 1 < q {
            let m = c.len() - 1;
            let mut d = vec![0.0; m + 2];
            for (i, di) in d.iter_mut().enumerate() {
                let t = i as f64 / (m + 1) as f64;
                let a = if i > 0 { c[i - 1] } else { 0.0 };
                let b = if i <= m { c[i] } else { 0.0 };
                *di = t * a + (1.0 - t) * b;
            }
            c = d;
        }
        B1(c)
    }
    fn add(&self, o: &B1) -> B1 {
        let q = self.deg().max(o.deg());
        let (a, b) = (self.elevate(q), o.elevate(q));
        B1(a.0.iter().zip(&b.0).map(|(x, y)| x + y).collect())
    }
    fn scale(&self, s: f64) -> B1 {
        B1(self.0.iter().map(|x| x * s).collect())
    }
    fn deriv(&self) -> B1 {
        let m = self.deg();
        if m == 0 {
            return B1::konst(0.0);
        }
        B1((0..m).map(|i| m as f64 * (self.0[i + 1] - self.0[i])).collect())
    }
    fn eval(&self, s: f64) -> f64 {
        let mut c = [0.0; 16];
        let m = self.0.len();
        c[..m].copy_from_slice(&self.0);
        for r in 1..m {
            for i in 0..m - r {
                c[i] = c[i] * (1.0 - s) + c[i + 1] * s;
            }
        }
        c[0]
    }
}

type V1 = [B1; 3];

fn vk1(v: Vec3) -> V1 {
    [B1::konst(v.x), B1::konst(v.y), B1::konst(v.z)]
}
fn vmul1(a: &V1, s: &B1) -> V1 {
    std::array::from_fn(|i| a[i].mul(s))
}
fn vadd1(a: &V1, b: &V1) -> V1 {
    std::array::from_fn(|i| a[i].add(&b[i]))
}
fn cross1(a: &V1, b: &V1) -> V1 {
    let m = |i: usize, j: usize| a[i].mul(&b[j]).add(&a[j].mul(&b[i]).scale(-1.0));
    [m(1, 2), m(2, 0), m(0, 1)]
}
fn veval(a: &V1, s: f64) -> Vec3 {
    Vec3::new(a[0].eval(s), a[1].eval(s), a[2].eval(s))
}
fn vderiv(a: &V1) -> V1 {
    std::array::from_fn(|k| a[k].deriv())
}
fn velevate(a: &V1, q: usize) -> V1 {
    std::array::from_fn(|k| a[k].elevate(q))
}
/// The quadratic through these values at s = 0, 1/2, 1.
fn quad_through(v: [Vec3; 3]) -> V1 {
    let mid = v[1] * 2.0 - (v[0] + v[2]) * 0.5;
    std::array::from_fn(|k| {
        let g = |x: Vec3| [x.x, x.y, x.z][k];
        B1(vec![g(v[0]), g(mid), g(v[2])])
    })
}

/// A side as a rational curve in s in [0, 1]: Cw / D, and how the rail's parameter maps to s.
#[derive(Clone, Debug)]
struct Arc3 {
    cw: V1,
    d: B1,
    /// t = t0 + s (t1 - t0), or for a circle tan((t - tm) / 2) = k (2 s - 1).
    arc: Option<(f64, f64)>,
    t0: f64,
    t1: f64,
}

impl Arc3 {
    fn of(curve: &Curve, t0: f64, t1: f64) -> Option<Arc3> {
        let p = |t: f64| curve.deriv(t);
        let pt = |q: Point3| q - Point3::ORIGIN;
        let quad = |c: [Vec3; 3], w: [f64; 3]| -> V1 {
            std::array::from_fn(|k| {
                let g = |v: Vec3| [v.x, v.y, v.z][k];
                B1(vec![g(c[0]) * w[0], g(c[1]) * w[1], g(c[2]) * w[2]])
            })
        };
        let (cw, d, arc) = match curve {
            Curve::Line(_) => {
                let (a, b) = (pt(p(t0).p), pt(p(t1).p));
                let cw = std::array::from_fn(|k| {
                    let g = |v: Vec3| [v.x, v.y, v.z][k];
                    B1(vec![g(a), g(b)])
                });
                (cw, B1::konst(1.0), None)
            }
            Curve::Parabola(_) => {
                let (a, b) = (p(t0), p(t1));
                let mid = pt(a.p) + a.d1 * (0.5 * (t1 - t0));
                (quad([pt(a.p), mid, pt(b.p)], [1.0; 3]), B1::konst(1.0), None)
            }
            Curve::Circle(c) => {
                let delta = t1 - t0;
                if delta.abs() >= 0.9 * dmath::PI {
                    return None;
                }
                let tm = 0.5 * (t0 + t1);
                let w1 = dmath::cos(0.5 * delta);
                let f = &c.frame;
                let (s, co) = dmath::sin_cos(tm);
                let mid = pt(f.origin) + (f.x().get() * co + f.y().get() * s) * (c.radius / w1);
                let (sn, cs) = dmath::sin_cos(0.25 * delta);
                (
                    quad([pt(p(t0).p), mid, pt(p(t1).p)], [1.0, w1, 1.0]),
                    B1(vec![1.0, w1, 1.0]),
                    Some((tm, sn / cs)),
                )
            }
            _ => return None,
        };
        Some(Arc3 { cw, d, arc, t0, t1 })
    }
    fn s_of_t(&self, t: f64) -> f64 {
        match self.arc {
            None => (t - self.t0) / (self.t1 - self.t0),
            Some((tm, k)) => {
                let (sn, cs) = dmath::sin_cos(0.5 * (t - tm));
                0.5 * (1.0 + sn / cs / k)
            }
        }
    }
    /// D^2 C' = Cw' D - Cw D', of degree 2 for a rational quadratic (1 for a parabola).
    fn q(&self) -> V1 {
        if self.d.deg() == 0 {
            return vderiv(&self.cw);
        }
        let w = &self.d.0;
        let p: Vec<Vec3> = (0..3)
            .map(|k| Vec3::new(self.cw[0].0[k], self.cw[1].0[k], self.cw[2].0[k]) / w[k])
            .collect();
        let c = [
            (p[1] - p[0]) * (2.0 * w[0] * w[1]),
            (p[2] - p[0]) * (w[0] * w[2]),
            (p[2] - p[1]) * (2.0 * w[1] * w[2]),
        ];
        std::array::from_fn(|k| B1(c.iter().map(|v| [v.x, v.y, v.z][k]).collect()))
    }
    /// Point, first and second s derivatives.
    fn jet(&self, s: f64) -> (Vec3, Vec3, Vec3) {
        let d1 = vderiv(&self.cw);
        let (c, c1, c2) = (veval(&self.cw, s), veval(&d1, s), veval(&vderiv(&d1), s));
        let (d, dd1, dd2) = (
            self.d.eval(s),
            self.d.deriv().eval(s),
            self.d.deriv().deriv().eval(s),
        );
        let p = c / d;
        let p1 = (c1 - p * dd1) / d;
        let p2 = (c2 - p1 * (2.0 * dd1) - p * dd2) / d;
        (p, p1, p2)
    }
}

/// The neighbour's outward normal along a side times D for a sphere or cylinder (a polynomial
/// vector), or None when it is not rational.
fn normal_d(sr: &SurfaceRail, arc: &Arc3) -> Option<V1> {
    let sg = sr.sign;
    match &sr.surface {
        Surface::Plane(pl) => Some(vk1(pl.frame.z().get() * sg)),
        Surface::Sphere(s) => {
            let o = s.frame.origin - Point3::ORIGIN;
            let v = vadd1(&arc.cw, &vmul1(&vk1(-o), &arc.d));
            Some(std::array::from_fn(|i| v[i].scale(sg / s.radius)))
        }
        Surface::Cylinder(cy) => {
            let o = cy.frame.origin - Point3::ORIGIN;
            let z = cy.frame.z().get();
            let v = vadd1(&arc.cw, &vmul1(&vk1(-o), &arc.d));
            let along = v[0].scale(z.x).add(&v[1].scale(z.y)).add(&v[2].scale(z.z));
            let f = vadd1(&v, &vmul1(&vk1(-z), &along));
            Some(std::array::from_fn(|i| f[i].scale(sg / cy.radius)))
        }
        _ => None,
    }
}

/// A side's rational curve and a polynomial field F across it in the neighbour's tangent
/// plane, pointing the way n x C' does (into the hole), of degree at most 2.
fn side_data(side: &Side) -> Option<(Arc3, V1)> {
    let (t0, t1) = (side.t0, side.t1);
    if let Some(sp) = side.rail.spoke() {
        // a cubic spoke's normal is w / |w|, w = p' x f: the plane is spanned by C' and f, which
        // points into the hole on either side of the spoke (so turns with the direction of travel)
        let (p, f0, f1) = sp.cubic?;
        if (t0.min(t1), t0.max(t1)) != (0.0, 1.0) {
            return None;
        }
        let (p, f0, f1) = if t0 < t1 {
            (p, f0, f1)
        } else {
            ([p[3], p[2], p[1], p[0]], -f1, -f0)
        };
        let g = |v: Vec3, k: usize| [v.x, v.y, v.z][k];
        let arc = Arc3 {
            cw: std::array::from_fn(|k| B1(p.iter().map(|x| g(*x - Point3::ORIGIN, k)).collect())),
            d: B1::konst(1.0),
            arc: None,
            t0,
            t1,
        };
        return Some((arc, std::array::from_fn(|k| B1(vec![g(f0, k), g(f1, k)]))));
    }
    let sr = side.rail.surface_rail()?;
    let arc = Arc3::of(&sr.curve, t0, t1)?;
    let q = arc.q();
    let f = cross1(&normal_d(sr, &arc)?, &q);
    if matches!(sr.surface, Surface::Plane(_)) || arc.d.deg() == 0 {
        return Some((arc, f));
    }
    // (n D) x Q is D times a quadratic for a circle on a sphere or cylinder
    let at = |s: f64| veval(&f, s) / arc.d.eval(s);
    let g = quad_through([at(0.0), at(0.5), at(1.0)]);
    Some((arc, g))
}

/// A ribbon (A + d B + d^2 C) / E, all of degree SDEG in s.
#[derive(Clone, Debug)]
struct RSide {
    abc: [V1; 3],
    e: B1,
    /// The coefficients by Bernstein index: A, B, C and E (with E as the fourth of A's).
    flat: [[Vec3; 3]; SDEG + 1],
    ew: [f64; SDEG + 1],
    arc: Arc3,
}

impl RSide {
    fn new(abc: [V1; 3], e: B1, arc: Arc3) -> RSide {
        let flat = std::array::from_fn(|a| {
            std::array::from_fn(|v| Vec3::new(abc[v][0].0[a], abc[v][1].0[a], abc[v][2].0[a]))
        });
        let ew = std::array::from_fn(|a| e.0[a]);
        RSide {
            abc,
            e,
            flat,
            ew,
            arc,
        }
    }
    /// A, B, C and E at s with their first and second s derivatives (when `jet`).
    fn at(&self, s: f64, jet: bool) -> ([[Vec3; 3]; 3], [f64; 3]) {
        // the Bernstein basis of degree SDEG and its derivatives, from those of lower degree:
        // B_k^n' = n (B_(k-1)^(n-1) - B_k^(n-1)), B_k^n'' = n (n - 1) (B_(k-2) - 2 B_(k-1) + B_k)
        let n = SDEG;
        let basis = |deg: usize| {
            let mut c = [0.0; SDEG + 3];
            c[2] = 1.0;
            for d in 1..=deg {
                for k in (0..=d).rev() {
                    c[k + 2] = c[k + 1] * s + c[k + 2] * (1.0 - s);
                }
            }
            // c[k + 2] is B_k; c[0], c[1] and past deg stay zero
            c
        };
        let mut b = [[0.0; SDEG + 1]; 3];
        let b0 = basis(n);
        b[0].copy_from_slice(&b0[2..]);
        if jet {
            let (b1, b2) = (basis(n - 1), basis(n - 2));
            let nf = n as f64;
            for k in 0..=n {
                b[1][k] = nf * (b1[k + 1] - b1[k + 2]);
                b[2][k] = nf * (nf - 1.0) * (b2[k] - 2.0 * b2[k + 1] + b2[k + 2]);
            }
        }
        let mut v = [[Vec3::ZERO; 3]; 3];
        let mut e = [0.0; 3];
        for o in 0..if jet { 3 } else { 1 } {
            for a in 0..=n {
                let w = b[o][a];
                for q in 0..3 {
                    v[q][o] += self.flat[a][q] * w;
                }
                e[o] += self.ew[a] * w;
            }
        }
        (v, e)
    }
}

/// The rational quad.
#[derive(Clone, Debug)]
pub struct Quad {
    sides: Vec<RSide>,
    /// N (x, y, z) and W over the unit square.
    num: [Bp; 3],
    den: Bp,
}

impl Quad {
    /// The quad over four sides in order round it, None when a side is of another kind or
    /// its cross tangent would not turn into the hole.
    pub fn build(sides: &[Side]) -> Option<Quad> {
        if sides.len() != 4 {
            return None;
        }
        let mut data: Vec<(Arc3, V1)> = sides.iter().map(side_data).collect::<Option<_>>()?;
        // the corners exactly shared: each side starts where the one before ends
        for i in 0..4 {
            let prev = (i + 3) % 4;
            for k in 0..3 {
                let e = *data[prev].0.cw[k].0.last().unwrap();
                data[i].0.cw[k].0[0] = e;
            }
        }
        let mut out = Vec::with_capacity(4);
        for i in 0..4 {
            let (arc, f) = &data[i];
            let (prev, next) = (&data[(i + 3) % 4].0, &data[(i + 1) % 4].0);
            let q = arc.q();
            // T = (a Q + b F) / D: at the ends, the neighbouring sides' tangents
            let solve = |s: f64, target: Vec3| -> Option<(f64, f64)> {
                let (u, v) = (veval(&q, s), veval(f, s));
                forge_math::linalg::solve2(
                    u.dot(u),
                    u.dot(v),
                    u.dot(v),
                    v.dot(v),
                    target.dot(u),
                    target.dot(v),
                )
            };
            let (pj, nj) = (prev.jet(1.0), next.jet(0.0));
            let (a0, b0) = solve(0.0, -pj.1 * arc.d.eval(0.0))?;
            let (a1, b1) = solve(1.0, nj.1 * arc.d.eval(1.0))?;
            if !(b0 > 0.0 && b1 > 0.0) {
                return None;
            }
            let b: V1 =
                std::array::from_fn(|k| B1(vec![a0, a1]).mul(&q[k]).add(&B1(vec![b0, b1]).mul(&f[k])));
            // the bend: half the neighbours' second derivatives, linear in s, over D
            let (k0, k1) = (pj.2 * 0.5, nj.2 * 0.5);
            let kd: V1 = std::array::from_fn(|k| {
                let g = |v: Vec3| [v.x, v.y, v.z][k];
                B1(vec![g(k0), g(k1)]).mul(&arc.d)
            });
            out.push(RSide::new(
                [velevate(&arc.cw, SDEG), velevate(&b, SDEG), velevate(&kd, SDEG)],
                arc.d.elevate(SDEG),
                arc.clone(),
            ));
        }
        let (num, den) = tensor(&out);
        Some(Quad { sides: out, num, den })
    }

    /// The mean lengths per unit of u and of v, from the control net.
    pub(super) fn net(&self) -> (f64, f64) {
        let n1 = DEG + 1;
        let pt = |k: usize| {
            let w = self.den.c[k];
            (w > 0.0).then(|| Vec3::new(self.num[0].c[k], self.num[1].c[k], self.num[2].c[k]) / w)
        };
        let (mut su, mut sv, mut nu, mut nv) = (0.0, 0.0, 0, 0);
        for i in 0..n1 {
            for j in 0..n1 {
                let Some(p) = pt(i * n1 + j) else { continue };
                if let Some(q) = (i < DEG).then(|| pt((i + 1) * n1 + j)).flatten() {
                    su += (q - p).length() * DEG as f64;
                    nu += 1;
                }
                if let Some(q) = (j < DEG).then(|| pt(i * n1 + j + 1)).flatten() {
                    sv += (q - p).length() * DEG as f64;
                    nv += 1;
                }
            }
        }
        (su / nu.max(1) as f64, sv / nv.max(1) as f64)
    }

    pub fn side_s(&self, i: usize, t: f64) -> f64 {
        self.sides[i].arc.s_of_t(t)
    }

    /// The patch at (u, v) in the unit square, with derivatives in the domain's (x, y) =
    /// (u, v) times `k`.
    pub(super) fn eval<S: Scalar>(&self, x: Point2, k: f64) -> S::V {
        let (uv, vv) = ((x.x / k).clamp(0.0, 1.0), (x.y / k).clamp(0.0, 1.0));
        let g = 1.0 / k;
        let u = S::affine(uv, g, 0.0);
        let v = S::affine(vv, 0.0, g);
        let one = S::konst(1.0);
        let s = [u, v, one - u, one - v];
        let d = [v, one - u, one - v, u];
        let (mut kk, mut second) = (0, f64::INFINITY);
        for i in 1..4 {
            if d[i].value() < d[kk].value() {
                second = d[kk].value();
                kk = i;
            } else {
                second = second.min(d[i].value());
            }
        }
        let mut r = [S::zero_v(); 4];
        let mut om = [S::konst(0.0); 4];
        for i in 0..4 {
            let sv = s[i].value();
            let (v, e) = self.sides[i].at(sv, S::JET);
            let ds = s[i] - S::konst(sv);
            let e = S::konst(e[0]) + ds * S::konst(e[1]) + ds * ds * S::konst(0.5 * e[2]);
            let num = s[i].along(v[0][0], v[0][1], v[0][2])
                + s[i].along(v[1][0], v[1][1], v[1][2]) * d[i]
                + s[i].along(v[2][0], v[2][1], v[2][2]) * (d[i] * d[i]);
            r[i] = num * e.inv();
            om[i] = e;
        }
        if second <= CUT {
            return r[kk];
        }
        // B_i E_i relative to the nearest side's: (d_k / d_i)^2 E_i
        let mut w = [S::konst(0.0); 4];
        let mut sum = S::konst(0.0);
        for i in 0..4 {
            let q = d[kk] * d[i].inv();
            w[i] = if i == kk { om[i] } else { q * q * om[i] };
            sum = sum + w[i];
        }
        let inv = sum.inv();
        let mut out = r[kk];
        for i in 0..4 {
            if i != kk {
                out = out + (r[i] - r[kk]) * (w[i] * inv);
            }
        }
        out
    }

    /// A box holding the patch over [u0, u1] x [v0, v1] of the unit square: the hull of the
    /// control points there (the weights are non-negative).
    pub(super) fn bbox(&self, u: Interval, v: Interval) -> Box3 {
        let cl = |i: Interval| {
            let w = 4.0 * f64::EPSILON;
            Interval::new((i.lo - w).max(0.0), (i.hi + w).min(1.0))
        };
        let (u, v) = (cl(u), cl(v));
        let part = |p: &Bp| p.restrict(true, u.lo, u.hi).restrict(false, v.lo, v.hi);
        let (n, w) = (self.num.map(|p| part(&p)), part(&self.den));
        let mut hull = Box3::EMPTY;
        let mut big: f64 = 0.0;
        for k in 0..w.c.len() {
            if w.c[k] > 0.0 {
                let q = Point3::new(n[0].c[k], n[1].c[k], n[2].c[k]) - Point3::ORIGIN;
                let q = Point3::ORIGIN + q / w.c[k];
                hull.include(q);
                big = big.max(q.x.abs()).max(q.y.abs()).max(q.z.abs());
            }
        }
        // the coefficients' rounding, and the corner zones, where the nearest ribbon stands in
        // for the blend (within about CUT^2 of it)
        hull.grow(1e-11 * big)
    }

    pub(super) fn transformed(&self, f: &Frame) -> Quad {
        let mut out = self.clone();
        let o = f.point_to_world(Point3::ORIGIN) - Point3::ORIGIN;
        let tv = |v: &V1, w: &B1, affine: bool| -> V1 {
            // a vector polynomial v (a point times the weight w when affine)
            let m = v[0].0.len();
            let mut out: V1 = std::array::from_fn(|_| B1(vec![0.0; m]));
            let w = w.elevate(m - 1);
            for j in 0..m {
                let x = f.vec_to_world(Vec3::new(v[0].0[j], v[1].0[j], v[2].0[j]))
                    + if affine { o * w.0[j] } else { Vec3::ZERO };
                out[0].0[j] = x.x;
                out[1].0[j] = x.y;
                out[2].0[j] = x.z;
            }
            out
        };
        for s in &mut out.sides {
            let abc = [
                tv(&s.abc[0], &s.e, true),
                tv(&s.abc[1], &s.e, false),
                tv(&s.abc[2], &s.e, false),
            ];
            let mut arc = s.arc.clone();
            arc.cw = tv(&s.arc.cw, &s.arc.d, true);
            *s = RSide::new(abc, s.e.clone(), arc);
        }
        (out.num, out.den) = tensor(&out.sides);
        out
    }
}

/// N and W over the unit square: each side's weighted ribbon in its (s, d), turned into (u, v).
fn tensor(sides: &[RSide]) -> ([Bp; 3], Bp) {
    let n1 = DEG + 1;
    let mut out = [Bp::zero(DEG, DEG); 4];
    for (i, sd) in sides.iter().enumerate() {
        // the ribbon in Bernstein form of degree (SDEG, 2) in (s, d), the weight's factor
        // s^2 (1 - s)^2 (1 - d)^2 = B_2^4(s) B_0^2(d) / 6 multiplied in, d elevated to DEG
        let mut g = [[[0.0; 4]; DEG + 1]; DEG + 1];
        for a in 0..=SDEG {
            let fs = binom(SDEG, a) * binom(4, 2) / binom(DEG, a + 2);
            for b in 0..3 {
                let x: [f64; 4] = std::array::from_fn(|k| match (k, b) {
                    (3, _) => sd.e.0[a],
                    (_, 0) => sd.abc[0][k].0[a],
                    (_, 1) => sd.abc[0][k].0[a] + 0.5 * sd.abc[1][k].0[a],
                    _ => sd.abc[0][k].0[a] + sd.abc[1][k].0[a] + sd.abc[2][k].0[a],
                });
                let fd = binom(2, b) / binom(4, b);
                // B_b^4 elevated to degree DEG
                for l in b..=b + DEG - 4 {
                    let el = binom(4, b) * binom(DEG - 4, l - b) / binom(DEG, l);
                    for k in 0..4 {
                        g[a + 2][l][k] += x[k] * fs * fd * el;
                    }
                }
            }
        }
        for al in 0..=DEG {
            for be in 0..=DEG {
                let (iu, iv) = match i {
                    0 => (al, be),
                    1 => (DEG - be, al),
                    2 => (DEG - al, DEG - be),
                    _ => (be, DEG - al),
                };
                for k in 0..4 {
                    out[k].c[iu * n1 + iv] += g[al][be][k];
                }
            }
        }
    }
    // a few roundings per term, a handful of terms per coefficient
    for p in &mut out {
        p.e = 64.0 * U * p.big();
    }
    ([out[0], out[1], out[2]], out[3])
}

/// Room for the coefficients of a Duffy triangle's polynomials.
const CAP: usize = (2 * DEG + 1) * (DEG + 3);

/// A polynomial over [0, 1]^2 in tensor Bernstein form, each coefficient within `e` of the true
/// one (the rounding of its construction).
#[derive(Clone, Copy, Debug)]
struct Bp {
    m: usize,
    n: usize,
    e: f64,
    c: [f64; CAP],
}

impl Bp {
    fn zero(m: usize, n: usize) -> Bp {
        Bp {
            m,
            n,
            e: 0.0,
            c: [0.0; CAP],
        }
    }
    fn len(&self) -> usize {
        (self.m + 1) * (self.n + 1)
    }
    fn at(&self, i: usize, j: usize) -> f64 {
        self.c[i * (self.n + 1) + j]
    }
    fn big(&self) -> f64 {
        self.c[..self.len()].iter().fold(0.0f64, |a, &x| a.max(x.abs()))
    }
    /// The two parts either side of tau along u (or v): de Casteljau.
    fn split(&self, along_u: bool, tau: f64) -> (Bp, Bp) {
        let (m, n) = (self.m, self.n);
        let (mut l, mut r) = (*self, *self);
        let (deg, lines, step, stride) = if along_u {
            (m, n + 1, n + 1, 1)
        } else {
            (n, m + 1, 1, n + 1)
        };
        let mut col = [0.0; 2 * DEG + 2];
        let om = 1.0 - tau;
        for line in 0..lines {
            let base = line * stride;
            for k in 0..=deg {
                col[k] = self.c[base + k * step];
            }
            l.c[base] = col[0];
            r.c[base + deg * step] = col[deg];
            for lev in 1..=deg {
                for i in 0..=deg - lev {
                    col[i] = om * col[i] + tau * col[i + 1];
                }
                l.c[base + lev * step] = col[0];
                r.c[base + (deg - lev) * step] = col[deg - lev];
            }
        }
        // convex combinations: a few roundings per level
        let e = self.e + (deg as f64 + 1.0) * 4.0 * U * self.big();
        (l.e, r.e) = (e, e);
        (l, r)
    }
    /// One half along u (or v): the first (0) or the second (1).
    fn half(&self, along_u: bool, which: usize) -> Bp {
        let (m, n) = (self.m, self.n);
        let mut out = *self;
        let (deg, lines, step, stride) = if along_u {
            (m, n + 1, n + 1, 1)
        } else {
            (n, m + 1, 1, n + 1)
        };
        let mut col = [0.0; 2 * DEG + 2];
        for line in 0..lines {
            let base = line * stride;
            for k in 0..=deg {
                col[k] = self.c[base + k * step];
            }
            if which == 0 {
                for lev in 1..=deg {
                    for i in 0..=deg - lev {
                        col[i] = 0.5 * (col[i] + col[i + 1]);
                    }
                    out.c[base + lev * step] = col[0];
                }
            } else {
                for lev in 1..=deg {
                    for i in 0..=deg - lev {
                        col[i] = 0.5 * (col[i] + col[i + 1]);
                    }
                    out.c[base + (deg - lev) * step] = col[deg - lev];
                }
            }
        }
        out.e = self.e + (deg as f64 + 1.0) * 2.0 * U * self.big();
        out
    }
    /// The polynomial over [lo, hi] along u (or v), rescaled to [0, 1]; the part covered is
    /// at least [lo, hi].
    fn restrict(&self, along_u: bool, lo: f64, hi: f64) -> Bp {
        let p = if hi < 1.0 {
            self.split(along_u, hi).0
        } else {
            *self
        };
        if lo > 0.0 {
            // the split point lo / hi, rounded down so the part only grows
            p.split(along_u, (lo / hi).next_down()).1
        } else {
            p
        }
    }
    /// Reversed along u (or v), exactly.
    fn flip(&self, along_u: bool) -> Bp {
        let mut out = *self;
        for i in 0..=self.m {
            for j in 0..=self.n {
                let (a, b) = if along_u { (self.m - i, j) } else { (i, self.n - j) };
                out.c[i * (self.n + 1) + j] = self.at(a, b);
            }
        }
        out
    }
    /// u and v swapped, exactly.
    fn transpose(&self) -> Bp {
        let mut out = Bp::zero(self.n, self.m);
        out.e = self.e;
        for i in 0..=self.m {
            for j in 0..=self.n {
                out.c[j * (self.m + 1) + i] = self.at(i, j);
            }
        }
        out
    }
    /// self - k o (same degrees).
    fn sub_scaled(&self, o: &Bp, k: f64) -> Bp {
        let mut out = *self;
        for i in 0..self.len() {
            out.c[i] = self.c[i] - k * o.c[i];
        }
        out.e = self.e + k.abs() * o.e + 3.0 * U * (self.big() + k.abs() * o.big());
        out
    }
    /// g(sigma, t) = f(sigma, sigma t), of degree (m + n, n): with
    /// B_j^n(sigma t) = sum_(k >= j) B_k^n(sigma) B_j^k(t), a map with non-negative weights
    /// summing to one for every output coefficient.
    fn duffy(&self) -> Bp {
        let (m, n) = (self.m, self.n);
        debug_assert!(m == DEG && n == DEG);
        static T: OnceLock<Vec<(u16, u16, f64)>> = OnceLock::new();
        let table = T.get_or_init(|| {
            let mut t = Vec::new();
            for i in 0..=DEG {
                for j in 0..=DEG {
                    for k in j..=DEG {
                        let a = binom(DEG, i) * binom(DEG, k) / binom(2 * DEG, i + k) * binom(k, j);
                        for l in j..=j + DEG - k {
                            let w = a * binom(DEG - k, l - j) / binom(DEG, l);
                            t.push(((i * (DEG + 1) + j) as u16, ((i + k) * (DEG + 1) + l) as u16, w));
                        }
                    }
                }
            }
            t
        });
        let mut out = Bp::zero(m + n, n);
        for &(src, dst, w) in table {
            out.c[dst as usize] += self.c[src as usize] * w;
        }
        out.e = self.e + ((m + 1) * (n + 1) + 8) as f64 * 4.0 * U * self.big();
        out
    }
    /// self / sigma^k (sigma along u), where the first k rows vanish in exact arithmetic.
    fn div_rows(&self, k: usize) -> Bp {
        let (mm, n) = (self.m, self.n);
        let m = mm - k;
        let mut out = Bp::zero(m, n);
        let mut amp: f64 = 0.0;
        for j in 0..=m {
            let f = binom(mm, j + k) / binom(m, j);
            amp = amp.max(f);
            for l in 0..=n {
                out.c[j * (n + 1) + l] = self.at(j + k, l) * f;
            }
        }
        out.e = amp * (self.e + 2.0 * U * self.big());
        out
    }
    /// (a + b u) times self: of one degree more in u.
    fn mul_affine(&self, a: f64, b: f64) -> Bp {
        let (m, n) = (self.m, self.n);
        let mut out = Bp::zero(m + 1, n);
        let (s0, s1) = (a, a + b);
        let k = 1.0 / (m + 1) as f64;
        for i in 0..=m + 1 {
            let (f0, f1) = ((m + 1 - i) as f64 * s0 * k, i as f64 * s1 * k);
            for l in 0..=n {
                let g0 = if i <= m { self.at(i, l) } else { 0.0 };
                let g1 = if i > 0 { self.at(i - 1, l) } else { 0.0 };
                out.c[i * (n + 1) + l] = f0 * g0 + f1 * g1;
            }
        }
        out.e = (self.e + 4.0 * U * self.big()) * (s0.abs().max(s1.abs()));
        out
    }
    /// Raised by one degree along u.
    fn elevate_u(&self) -> Bp {
        let (m, n) = (self.m, self.n);
        let mut out = Bp::zero(m + 1, n);
        for i in 0..=m + 1 {
            let t = i as f64 / (m + 1) as f64;
            for j in 0..=n {
                let a = if i > 0 { self.at(i - 1, j) } else { 0.0 };
                let b = if i <= m { self.at(i, j) } else { 0.0 };
                out.c[i * (n + 1) + j] = t * a + (1.0 - t) * b;
            }
        }
        out.e = self.e + 3.0 * U * self.big();
        out
    }
    /// self times a polynomial of degree (a, b) given by its coefficients, row by row: in the
    /// scaled basis (coefficients times binomials) a product is a plain convolution.
    fn mul_small(&self, a: usize, b: usize, f: &[f64]) -> Bp {
        let (m, n) = (self.m, self.n);
        let (mo, no) = (m + a, n + b);
        let mut out = Bp::zero(mo, no);
        let w = no + 1;
        let mut fs = [0.0; 9];
        for p in 0..=a {
            for q in 0..=b {
                fs[p * (b + 1) + q] = f[p * (b + 1) + q] * BINOM[a][p] * BINOM[b][q];
            }
        }
        for i in 0..=m {
            let ri = BINOM[m][i];
            for j in 0..=n {
                let c = self.c[i * (n + 1) + j] * ri * BINOM[n][j];
                for p in 0..=a {
                    let row = (i + p) * w + j;
                    let fr = &fs[p * (b + 1)..(p + 1) * (b + 1)];
                    for (q, &fq) in fr.iter().enumerate() {
                        out.c[row + q] += c * fq;
                    }
                }
            }
        }
        for i in 0..=mo {
            let ri = 1.0 / BINOM[mo][i];
            for j in 0..=no {
                out.c[i * w + j] *= ri / BINOM[no][j];
            }
        }
        let fm = f.iter().fold(0.0f64, |acc, &x| acc.max(x.abs()));
        out.e = (self.e + ((a + 1) * (b + 1) + 8) as f64 * 4.0 * U * self.big()) * fm;
        out
    }
    fn lower(&self) -> f64 {
        self.c[..self.len()].iter().fold(f64::INFINITY, |a, &x| a.min(x)) - self.e
    }
}

/// Y = X - q W, q the ratio of the coefficient sums (near the box's middle), with how far the
/// control points Y_ij / W_ij reach from the origin and a bound on their rounding: P - q lies
/// in their hull. All W_ij must be positive.
struct About {
    y: [Bp; 3],
    reach: f64,
    blur: f64,
}

impl About {
    fn of(x: &[Bp; 3], w: &Bp) -> About {
        let len = w.len();
        let sw: f64 = w.c[..len].iter().sum();
        let mut y = *x;
        for k in 0..3 {
            let q = x[k].c[..len].iter().sum::<f64>() / sw;
            y[k] = x[k].sub_scaled(w, q);
        }
        let ey = y.iter().map(|p| p.e).fold(0.0, f64::max) * 1.8;
        let (mut r2, mut wmin) = (0.0f64, f64::INFINITY);
        for k in 0..len {
            let v = Vec3::new(y[0].c[k], y[1].c[k], y[2].c[k]) / w.c[k];
            r2 = r2.max(v.length_sq());
            wmin = wmin.min(w.c[k]);
        }
        let reach = r2.sqrt() * (1.0 + 4.0 * U);
        // v / w against the true ratio: v and w each off by their rounding
        let blur = (ey + reach * (w.e + 2.0 * U * wmin)) / (wmin - w.e);
        About { y, reach, blur }
    }

    /// The sum of the coefficients of d/du (or d/dv) of Y: the rows' (columns') ends.
    fn axis(&self, along_u: bool) -> Vec3 {
        let y = &self.y;
        let (m, n) = (y[0].m, y[0].n);
        let mut s = Vec3::ZERO;
        let at = |i: usize, j: usize| Vec3::new(y[0].at(i, j), y[1].at(i, j), y[2].at(i, j));
        if along_u {
            for j in 0..=n {
                s += at(m, j) - at(0, j);
            }
        } else {
            for i in 0..=m {
                s += at(i, n) - at(i, 0);
            }
        }
        s
    }

    /// W P_u (or W P_v) is a Bernstein combination of K_ij - w_ij (P - q), the coefficients of
    /// d/du of Y and of W (the same basis), with P - q in the control points' hull: the tangents
    /// of the angles from `axis` (unit, across n0) it can make, None when it may not be within a
    /// quarter turn of the axis.
    fn sector(&self, w: &Bp, along_u: bool, n0: Vec3, axis: Vec3) -> Option<(f64, f64)> {
        let (y, len) = (&self.y, w.len());
        let side = n0.cross(axis);
        let pt = |k: usize| Vec3::new(y[0].c[k], y[1].c[k], y[2].c[k]);
        // the control points along the axis and across it
        let (mut pl, mut ph, mut ql, mut qh) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
        for k in 0..len {
            let v = pt(k) / w.c[k];
            let (p, q) = (axis.dot(v), side.dot(v));
            (pl, ph, ql, qh) = (pl.min(p), ph.max(p), ql.min(q), qh.max(q));
        }
        let (m, n) = (w.m, w.n);
        let deg = if along_u { m } else { n } as f64;
        // each difference of two coefficients, times the degree; the weights' rounding times
        // the points' reach; the points' rounding times the weights
        let e = y.iter().map(|p| p.e + U * p.big()).fold(0.0, f64::max) * 2.0 * deg * 1.8
            + (w.e + U * w.big()) * 2.0 * deg * (self.reach + self.blur)
            + self.blur * w.big() * 2.0 * deg;
        let (ri, rj, step) = if along_u { (m, n + 1, n + 1) } else { (m + 1, n, 1) };
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for i in 0..ri {
            for j in 0..rj {
                let t = i * (n + 1) + j;
                let a = (pt(t + step) - pt(t)) * deg;
                let wd = (w.c[t + step] - w.c[t]) * deg;
                // a - wd y: the part along the axis stays positive, the ratio across it is bounded
                let (c, s) = (axis.dot(a), side.dot(a));
                let ((wl, wh), (vl, vh)) = if wd >= 0.0 {
                    ((wd * pl, wd * ph), (wd * ql, wd * qh))
                } else {
                    ((wd * ph, wd * pl), (wd * qh, wd * ql))
                };
                let (cl, ch) = (c - wh - e, c - wl + e);
                let (sl, sh) = (s - vh - e, s - vl + e);
                if cl <= 0.0 {
                    return None;
                }
                lo = lo.min(sl / if sl >= 0.0 { ch } else { cl });
                hi = hi.max(sh / if sh >= 0.0 { cl } else { ch });
            }
        }
        // the divisions' rounding
        Some((lo - 4.0 * U * lo.abs(), hi + 4.0 * U * hi.abs()))
    }
}

/// The outcome of a proof.
#[derive(Clone, Copy, Debug, Default)]
pub struct Proof {
    pub proved: bool,
    /// Regions tried, cone tests run, and the deepest split.
    pub regions: usize,
    pub tests: usize,
    pub depth: u32,
    /// Whether it came from the cache.
    pub cached: bool,
}

/// Deepest split of a region.
const MAX_DEPTH: u32 = 12;
/// The size of the box round a corner of the square proven in Duffy triangles.
const CORNER_BOX: f64 = 0.25;

impl Quad {
    /// Proves the patch is regular (P_u x P_v never zero) over the unit square: boxes of the
    /// blend, the boxes round its corners in Duffy triangles, and each corner's zone, where the
    /// nearest ribbon is the patch, by that ribbon.
    pub fn prove(&self) -> Proof {
        let key = self.key();
        if let Some(&ok) = cache().lock().unwrap().get(&key) {
            return Proof {
                proved: ok,
                cached: true,
                ..Proof::default()
            };
        }
        let mut st = Proof::default();
        st.proved = self.prove_all(&mut st);
        let mut c = cache().lock().unwrap();
        if c.len() > 20_000 {
            c.clear();
        }
        c.insert(key, st.proved);
        st
    }

    fn prove_all(&self, st: &mut Proof) -> bool {
        self.prove_zones(st) && self.blend_box(&self.num, &self.den, (0.0, 0.0, 1.0), st)
    }

    fn prove_zones(&self, st: &mut Proof) -> bool {
        for c in 0..4 {
            let pc = self.corner(c);
            // the zone: sides c (at its start) and c - 1 (at its end), alone
            for (i, at_start) in [(c, true), ((c + 3) % 4, false)] {
                let (rn, rw) = ribbon_tensor(&self.sides[i]);
                let part = |p: &Bp| {
                    let p = if at_start {
                        p.split(true, ZONE).0
                    } else {
                        p.split(true, 1.0 - ZONE).1
                    };
                    p.split(false, ZONE).0
                };
                let w = part(&rw);
                let n: [Bp; 3] = std::array::from_fn(|k| part(&rn[k].sub_scaled(&rw, [pc.x, pc.y, pc.z][k])));
                st.regions += 1;
                if !plain_region(&n, &w, 0, st) {
                    return false;
                }
            }
        }
        true
    }

    /// The corner point, exactly as the sides share it.
    fn corner(&self, c: usize) -> Vec3 {
        let a = &self.sides[c].abc[0];
        Vec3::new(a[0].0[0], a[1].0[0], a[2].0[0])
    }

    /// The blend over the box at (u0, v0) of this size: quarters while it holds a corner of
    /// the square, until the corner's box is small enough for two Duffy triangles.
    fn blend_box(&self, n: &[Bp; 3], w: &Bp, (u0, v0, size): (f64, f64, f64), st: &mut Proof) -> bool {
        let holds = |c: usize| {
            let (cu, cv) = ([0.0, 1.0, 1.0, 0.0][c], [0.0, 0.0, 1.0, 1.0][c]);
            (u0..=u0 + size).contains(&cu) && (v0..=v0 + size).contains(&cv)
        };
        let corner = (0..4).find(|&c| holds(c));
        if corner.is_none() {
            st.regions += 1;
            return plain_box(n, w, st);
        }
        if size > CORNER_BOX {
            let h = 0.5 * size;
            return (0..4).all(|q| {
                let part = |p: &Bp| p.half(true, q / 2).half(false, q % 2);
                let at = (u0 + h * (q / 2) as f64, v0 + h * (q % 2) as f64, h);
                self.blend_box(&[part(&n[0]), part(&n[1]), part(&n[2])], &part(w), at, st)
            });
        }
        let c = corner.unwrap();
        // the corner to the origin, N about it: N - p_c W = sigma^3 G and W = sigma^2 W~ in
        // either triangle (every weight has u^2 or v^2, and the ribbons meet at p_c)
        let pc = self.corner(c);
        let (fu, fv) = (c == 1 || c == 2, c >= 2);
        let turn = |p: &Bp| {
            let p = if fu { p.flip(true) } else { *p };
            if fv { p.flip(false) } else { p }
        };
        let w = turn(w);
        let n: [Bp; 3] = std::array::from_fn(|k| turn(&n[k]).sub_scaled(&w, [pc.x, pc.y, pc.z][k]));
        (0..2).all(|tri| {
            let t = |p: &Bp| if tri == 0 { *p } else { p.transpose() };
            let g: [Bp; 3] = std::array::from_fn(|k| t(&n[k]).duffy().div_rows(3));
            let wt = t(&w).duffy().div_rows(2);
            st.regions += 1;
            let Some(phi) = normal_phi(&wt, 1) else {
                return false;
            };
            duffy_region(&g.map(|p| times(&p, &phi)), &times(&wt, &phi), (0.0, 1.0), 0, st)
        })
    }

    /// The exact inputs of the proof: every coefficient of every ribbon.
    fn key(&self) -> Vec<u64> {
        let mut k = Vec::with_capacity(160);
        for s in &self.sides {
            for v in &s.abc {
                for b in v {
                    k.extend(b.0.iter().map(|x| x.to_bits()));
                }
            }
            k.extend(s.e.0.iter().map(|x| x.to_bits()));
        }
        k
    }
}

/// Forgets every proof (for timing).
#[doc(hidden)]
pub fn clear_proof_cache() {
    cache().lock().unwrap().clear();
}

fn cache() -> &'static Mutex<HashMap<Vec<u64>, bool>> {
    static C: OnceLock<Mutex<HashMap<Vec<u64>, bool>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

/// A ribbon alone as a tensor polynomial of degree (SDEG, 2) in (s, d): numerator and weight.
fn ribbon_tensor(sd: &RSide) -> ([Bp; 3], Bp) {
    let mut out = [Bp::zero(SDEG, 2); 4];
    for a in 0..=SDEG {
        for b in 0..3 {
            for k in 0..3 {
                let (av, bv, cv) = (sd.abc[0][k].0[a], sd.abc[1][k].0[a], sd.abc[2][k].0[a]);
                out[k].c[a * 3 + b] = match b {
                    0 => av,
                    1 => av + 0.5 * bv,
                    _ => av + bv + cv,
                };
            }
            out[3].c[a * 3 + b] = sd.e.0[a];
        }
    }
    for p in &mut out {
        p.e = 4.0 * U * p.big();
    }
    ([out[0], out[1], out[2]], out[3])
}

/// Whether W P_u (from X1) and W P_v (from X2) lie in angular sectors of the plane across some
/// n0 that are ordered and less than a half turn apart, P = X1 / W + const = X2 / W + const':
/// then n0 . (P_u x P_v) > 0 over the box.
fn separated(x1: &[Bp; 3], x2: Option<&[Bp; 3]>, w: &Bp) -> bool {
    if w.lower() <= 0.0 {
        return false;
    }
    let a1 = About::of(x1, w);
    let a2 = x2.map(|x2| About::of(x2, w));
    let a2 = a2.as_ref().unwrap_or(&a1);
    let (sx, sy) = (a1.axis(true), a2.axis(false));
    let Some(n0) = sx.cross(sy).normalize() else {
        return false;
    };
    let n0 = n0.get();
    let flat = |v: Vec3| (v - n0 * n0.dot(v)).normalize().map(|u| u.get());
    let (Some(ua), Some(va)) = (flat(sx), flat(sy)) else {
        return false;
    };
    let (Some(a), Some(b)) = (a1.sector(w, true, n0, ua), a2.sector(w, false, n0, va)) else {
        return false;
    };
    let beta = dmath::atan2(n0.dot(ua.cross(va)), ua.dot(va));
    // room for the rounding of the angles and tangents
    let slack = 1e-9;
    beta + dmath::atan(b.0) - dmath::atan(a.1) > slack
        && beta + dmath::atan(b.1) - dmath::atan(a.0) < dmath::PI - slack
}

/// Phi, biquadratic with 1 / W at the box's corners, edges' middles and middle (or bilinear
/// through the corners when that is not positive): W phi is nearly constant over the box and its
/// parts, so the shift (P - q) (W phi)_u is small. P = X phi / W phi.
fn normal_phi(w: &Bp, order: usize) -> Option<(usize, Vec<f64>)> {
    let (m, n) = (w.m, w.n);
    // W at u = 0, 1/2, 1 (rows of coefficients in v), each at v = 0, 1/2, 1
    let mid = w.half(true, 0);
    let mut f = [[0.0; 3]; 3];
    for (i, row) in [
        &w.c[..=n],
        &mid.c[m * (n + 1)..=m * (n + 1) + n],
        &w.c[m * (n + 1)..=m * (n + 1) + n],
    ]
    .into_iter()
    .enumerate()
    {
        let mut c = [0.0; 2 * DEG + 2];
        c[..=n].copy_from_slice(row);
        for r in 1..=n {
            for k in 0..=n - r {
                c[k] = 0.5 * (c[k] + c[k + 1]);
            }
        }
        f[i] = [row[0], c[0], row[n]];
    }
    if f.iter().flatten().any(|&v| v <= 0.0) {
        return None;
    }
    let f = f.map(|r| r.map(|v| 1.0 / v));
    // the quadratic through f0, f1/2, f1 has the middle coefficient 2 f1/2 - (f0 + f1) / 2
    let mid = |a: f64, m: f64, b: f64| 2.0 * m - 0.5 * (a + b);
    let rows: Vec<[f64; 3]> = f.iter().map(|r| [r[0], mid(r[0], r[1], r[2]), r[2]]).collect();
    let c: Vec<f64> = (0..3)
        .flat_map(|i| (0..3).map(move |j| (i, j)))
        .map(|(i, j)| {
            if i == 1 {
                mid(rows[0][j], rows[1][j], rows[2][j])
            } else {
                rows[i][j]
            }
        })
        .collect();
    if order == 2 && c.iter().all(|&x| x > 0.0) {
        Some((2, c))
    } else {
        Some((1, vec![f[0][0], f[0][2], f[2][0], f[2][2]]))
    }
}

/// X times phi.
fn times(x: &Bp, phi: &(usize, Vec<f64>)) -> Bp {
    x.mul_small(phi.0, phi.0, &phi.1)
}

/// P = N / W regular over the box; boxes that fail are split in four.
fn plain_box(n: &[Bp; 3], w: &Bp, st: &mut Proof) -> bool {
    let Some(phi) = normal_phi(w, 2) else {
        return false;
    };
    plain_region(&n.map(|p| times(&p, &phi)), &times(w, &phi), 0, st)
}

fn plain_region(n: &[Bp; 3], w: &Bp, depth: u32, st: &mut Proof) -> bool {
    st.tests += 1;
    st.depth = st.depth.max(depth);
    if separated(n, None, w) {
        return true;
    }
    // halves, along u and v in turn
    depth < MAX_DEPTH
        && (0..2).all(|h| {
            let p = |x: &Bp| x.half(depth.is_multiple_of(2), h);
            plain_region(&n.map(|x| p(&x)), &p(w), depth + 1, st)
        })
}

/// A corner's Duffy triangle (x = sigma, y = sigma t, sigma in `sig`, start and width) where
/// N - p_c W = sigma^3 G and W = sigma^2 W~: P - p_c = sigma G / W~ and
/// (P - p_c) / sigma = G / W~, so the cones are of W~ P_sigma (from sigma G) and
/// W~ P_t / sigma (from G), both finite at the corner.
fn duffy_region(g: &[Bp; 3], w: &Bp, sig: (f64, f64), depth: u32, st: &mut Proof) -> bool {
    // the whole triangle rarely passes: start from its halves
    if depth > 0 {
        st.tests += 1;
        st.depth = st.depth.max(depth);
        let h = g.map(|p| p.mul_affine(sig.0, sig.1));
        if separated(&h, Some(&g.map(|p| p.elevate_u())), w) {
            return true;
        }
    }
    // halves, along sigma and t in turn
    let u = depth.is_multiple_of(2);
    depth < MAX_DEPTH
        && (0..2).all(|q| {
            let p = |x: &Bp| x.half(u, q);
            let s = if u {
                (sig.0 + 0.5 * sig.1 * q as f64, 0.5 * sig.1)
            } else {
                sig
            };
            duffy_region(&g.map(|x| p(&x)), &p(w), s, depth + 1, st)
        })
}
