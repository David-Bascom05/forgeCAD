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
//! How the construction and the proof's parameters were chosen: research/corner-patch.

// polynomial coefficient loops read best indexed
#![allow(clippy::needless_range_loop)]

use super::{Scalar, Side, SurfaceRail};
use crate::curve::{Curve, CurveGeom};
use crate::surface::Surface;
use forge_math::{Box3, Frame, Interval, Point3, Vec2, Vec3, dmath};
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
    nw: Bp<4>,
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
        let nw = tensor(&out);
        Some(Quad { sides: out, nw })
    }

    /// The mean lengths per unit of u and of v, from the control net.
    pub(super) fn net(&self) -> (f64, f64) {
        let n1 = DEG + 1;
        let pt = |k: usize| {
            let [x, y, z, w] = self.nw.c[k];
            (w > 0.0).then(|| Vec3::new(x, y, z) / w)
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

    /// The patch at (u, v) in the unit square, with derivatives in the domain's (x, y), where
    /// u and v have the gradients `gu` and `gv`.
    pub(super) fn eval<S: Scalar>(&self, u: f64, v: f64, gu: Vec2, gv: Vec2) -> S::V {
        let u = S::affine(u.clamp(0.0, 1.0), gu.x, gu.y);
        let v = S::affine(v.clamp(0.0, 1.0), gv.x, gv.y);
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
        let p = self.nw.restrict(true, u.lo, u.hi).restrict(false, v.lo, v.hi);
        let mut hull = Box3::EMPTY;
        let mut big: f64 = 0.0;
        for &[x, y, z, w] in &p.c[..p.len()] {
            if w > 0.0 {
                let q = Point3::ORIGIN + Vec3::new(x, y, z) / w;
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
        out.nw = tensor(&out.sides);
        out
    }
}

/// N and W over the unit square: each side's weighted ribbon in its (s, d), turned into (u, v).
fn tensor(sides: &[RSide]) -> Bp<4> {
    let n1 = DEG + 1;
    let mut out = Bp::zero(DEG, DEG);
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
                    out.c[iu * n1 + iv][k] += g[al][be][k];
                }
            }
        }
    }
    // a few roundings per term, a handful of terms per coefficient
    out.b = out.big();
    out.e = out.b.map(|b| 64.0 * U * b);
    out
}

/// Room for the coefficients of a Duffy triangle's polynomials.
const CAP: usize = (2 * DEG) * (DEG + 2);

/// K polynomials over [0, 1]^2 in tensor Bernstein form, one per lane of each coefficient (N's
/// x, y, z then W), lane k's coefficients each within `e[k]` of the true ones (the rounding of
/// their construction). Every operation runs all lanes in one loop.
#[derive(Clone, Copy, Debug)]
struct Bp<const K: usize> {
    m: usize,
    n: usize,
    e: [f64; K],
    /// Each lane's largest coefficient (in size), or more.
    b: [f64; K],
    c: [[f64; K]; CAP],
}

impl<const K: usize> Bp<K> {
    fn zero(m: usize, n: usize) -> Self {
        Bp {
            m,
            n,
            e: [0.0; K],
            b: [0.0; K],
            c: [[0.0; K]; CAP],
        }
    }
    fn len(&self) -> usize {
        (self.m + 1) * (self.n + 1)
    }
    fn at(&self, i: usize, j: usize) -> [f64; K] {
        self.c[i * (self.n + 1) + j]
    }
    /// Each lane's largest coefficient (in size), measured.
    fn big(&self) -> [f64; K] {
        let mut b = [0.0f64; K];
        for x in &self.c[..self.len()] {
            for k in 0..K {
                b[k] = b[k].max(x[k].abs());
            }
        }
        b
    }
    /// Lanes `from..from + J` alone.
    fn lanes<const J: usize>(&self, from: usize) -> Bp<J> {
        let mut out = Bp::zero(self.m, self.n);
        for i in 0..self.len() {
            out.c[i] = std::array::from_fn(|k| self.c[i][from + k]);
        }
        out.e = std::array::from_fn(|k| self.e[from + k]);
        out.b = std::array::from_fn(|k| self.b[from + k]);
        out
    }
    /// The (deg, lines, step, stride) of a pass of de Casteljau along u (or v).
    fn axes(&self, along_u: bool) -> (usize, usize, usize, usize) {
        let (m, n) = (self.m, self.n);
        if along_u {
            (m, n + 1, n + 1, 1)
        } else {
            (n, m + 1, 1, n + 1)
        }
    }
    /// Each lane's error grown by `per` times its largest coefficient.
    fn grown(&self, per: f64) -> [f64; K] {
        std::array::from_fn(|k| self.e[k] + per * self.b[k])
    }
    /// The two parts either side of tau along u (or v): de Casteljau.
    fn split(&self, along_u: bool, tau: f64) -> (Self, Self) {
        let (mut l, mut r) = (Self::zero(self.m, self.n), Self::zero(self.m, self.n));
        let (deg, lines, step, stride) = self.axes(along_u);
        let mut col = [[0.0; K]; 2 * DEG + 2];
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
                    for k in 0..K {
                        col[i][k] = om * col[i][k] + tau * col[i + 1][k];
                    }
                }
                l.c[base + lev * step] = col[0];
                r.c[base + (deg - lev) * step] = col[deg - lev];
            }
        }
        // convex combinations: a few roundings per level
        let e = self.grown((deg as f64 + 1.0) * 4.0 * U);
        (l.e, r.e, l.b, r.b) = (e, e, self.b, self.b);
        (l, r)
    }
    /// Both halves along u (or v) in one pass.
    fn halves(&self, along_u: bool) -> (Self, Self) {
        let (mut l, mut r) = (Self::zero(self.m, self.n), Self::zero(self.m, self.n));
        let (deg, lines, step, stride) = self.axes(along_u);
        let mut col = [[0.0; K]; 2 * DEG + 2];
        for line in 0..lines {
            let base = line * stride;
            for k in 0..=deg {
                col[k] = self.c[base + k * step];
            }
            l.c[base] = col[0];
            r.c[base + deg * step] = col[deg];
            for lev in 1..=deg {
                for i in 0..=deg - lev {
                    for k in 0..K {
                        col[i][k] = 0.5 * (col[i][k] + col[i + 1][k]);
                    }
                }
                l.c[base + lev * step] = col[0];
                r.c[base + (deg - lev) * step] = col[deg - lev];
            }
        }
        let e = self.grown((deg as f64 + 1.0) * 2.0 * U);
        (l.e, r.e, l.b, r.b) = (e, e, self.b, self.b);
        (l, r)
    }
    /// The polynomial over [lo, hi] along u (or v), rescaled to [0, 1]; the part covered is
    /// at least [lo, hi].
    fn restrict(&self, along_u: bool, lo: f64, hi: f64) -> Self {
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
    fn flip(&self, along_u: bool) -> Self {
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
    fn transpose(&self) -> Self {
        let mut out = Self::zero(self.n, self.m);
        (out.e, out.b) = (self.e, self.b);
        for i in 0..=self.m {
            for j in 0..=self.n {
                out.c[j * (self.m + 1) + i] = self.at(i, j);
            }
        }
        out
    }
    /// g(sigma, t) = f(sigma, sigma t), of degree (m + n, n): with
    /// B_j^n(sigma t) = sum_(k >= j) B_k^n(sigma) B_j^k(t), a map with non-negative weights
    /// summing to one for every output coefficient.
    fn duffy(&self) -> Self {
        let (m, n) = (self.m, self.n);
        debug_assert!(m == DEG && n == DEG);
        // the weight of coefficient (i, j) in (i + k, l), by [i][k][j][l]
        type Table = [[[[f64; DEG + 1]; DEG + 1]; DEG + 1]; DEG + 1];
        static T: OnceLock<Box<Table>> = OnceLock::new();
        let table = T.get_or_init(|| {
            let mut t = Box::new([[[[0.0; DEG + 1]; DEG + 1]; DEG + 1]; DEG + 1]);
            for i in 0..=DEG {
                for j in 0..=DEG {
                    for k in j..=DEG {
                        let a = binom(DEG, i) * binom(DEG, k) / binom(2 * DEG, i + k) * binom(k, j);
                        for l in j..=j + DEG - k {
                            t[i][k][j][l] = a * binom(DEG - k, l - j) / binom(DEG, l);
                        }
                    }
                }
            }
            t
        });
        let mut out = Self::zero(m + n, n);
        // for each output coefficient the terms add by rising i, then j
        for i in 0..=DEG {
            for k in 0..=DEG {
                let dst = &mut out.c[(i + k) * (DEG + 1)..(i + k + 1) * (DEG + 1)];
                for j in 0..=k {
                    let c = self.c[i * (DEG + 1) + j];
                    let w = &table[i][k][j];
                    for l in j..=j + DEG - k {
                        for q in 0..K {
                            dst[l][q] += c[q] * w[l];
                        }
                    }
                }
            }
        }
        out.e = self.grown(((m + 1) * (n + 1) + 8) as f64 * 4.0 * U);
        out.b = self.b;
        out
    }
    /// self / sigma^k (sigma along u), where the first k rows vanish in exact arithmetic.
    fn div_rows(&self, k: usize) -> Self {
        let (mm, n) = (self.m, self.n);
        let m = mm - k;
        let mut out = Self::zero(m, n);
        let mut amp: f64 = 0.0;
        for j in 0..=m {
            let f = binom(mm, j + k) / binom(m, j);
            amp = amp.max(f);
            for l in 0..=n {
                out.c[j * (n + 1) + l] = self.at(j + k, l).map(|x| x * f);
            }
        }
        out.e = std::array::from_fn(|q| amp * (self.e[q] + 2.0 * U * self.b[q]));
        out.b = out.big();
        out
    }
    /// (a + b u) times self: of one degree more in u.
    fn mul_affine(&self, a: f64, b: f64) -> Self {
        let (m, n) = (self.m, self.n);
        let mut out = Self::zero(m + 1, n);
        let (s0, s1) = (a, a + b);
        let k = 1.0 / (m + 1) as f64;
        for i in 0..=m + 1 {
            let (f0, f1) = ((m + 1 - i) as f64 * s0 * k, i as f64 * s1 * k);
            for l in 0..=n {
                let g0 = if i <= m { self.at(i, l) } else { [0.0; K] };
                let g1 = if i > 0 { self.at(i - 1, l) } else { [0.0; K] };
                out.c[i * (n + 1) + l] = std::array::from_fn(|q| f0 * g0[q] + f1 * g1[q]);
            }
        }
        let e = self.grown(4.0 * U);
        out.e = e.map(|x| x * s0.abs().max(s1.abs()));
        out.b = out.big();
        out
    }
    /// Raised by one degree along u.
    fn elevate_u(&self) -> Self {
        let (m, n) = (self.m, self.n);
        let mut out = Self::zero(m + 1, n);
        for i in 0..=m + 1 {
            let t = i as f64 / (m + 1) as f64;
            for j in 0..=n {
                let a = if i > 0 { self.at(i - 1, j) } else { [0.0; K] };
                let b = if i <= m { self.at(i, j) } else { [0.0; K] };
                out.c[i * (n + 1) + j] = std::array::from_fn(|q| t * a[q] + (1.0 - t) * b[q]);
            }
        }
        out.e = self.grown(3.0 * U);
        out.b = self.b;
        out
    }
    /// self times a polynomial of degree (a, a) given by its coefficients, row by row: in the
    /// scaled basis (coefficients times binomials) a product is a plain convolution.
    fn times(&self, phi: &Phi) -> Self {
        let (a, f) = (phi.0, &phi.1[..(phi.0 + 1) * (phi.0 + 1)]);
        let (m, n) = (self.m, self.n);
        let (mo, no) = (m + a, n + a);
        let mut out = Self::zero(mo, no);
        let w = no + 1;
        let mut fs = [[0.0; 3]; 3];
        for p in 0..=a {
            for q in 0..=a {
                fs[p][q] = f[p * (a + 1) + q] * BINOM[a][p] * BINOM[a][q];
            }
        }
        let mut row = [[0.0; K]; 2 * DEG + 2];
        for i in 0..=m {
            let ri = BINOM[m][i];
            for j in 0..=n {
                let x = self.c[i * (n + 1) + j];
                row[j] = std::array::from_fn(|k| x[k] * ri * BINOM[n][j]);
            }
            for p in 0..=a {
                let dst = &mut out.c[(i + p) * w..(i + p + 1) * w];
                // q falling, so each coefficient sums its terms by rising j
                for q in (0..=a).rev() {
                    let fq = fs[p][q];
                    for (d, x) in dst[q..=q + n].iter_mut().zip(&row[..=n]) {
                        for k in 0..K {
                            d[k] += x[k] * fq;
                        }
                    }
                }
            }
        }
        for i in 0..=mo {
            let ri = 1.0 / BINOM[mo][i];
            for j in 0..=no {
                let s = ri / BINOM[no][j];
                for x in &mut out.c[i * w + j] {
                    *x *= s;
                }
            }
        }
        let fm = f.iter().fold(0.0f64, |acc, &x| acc.max(x.abs()));
        let e = self.grown(((a + 1) * (a + 1) + 8) as f64 * 4.0 * U);
        out.e = e.map(|x| x * fm);
        out.b = out.big();
        out
    }
    /// Lane k's lowest coefficient, less its error.
    fn lower(&self, k: usize) -> f64 {
        self.c[..self.len()]
            .iter()
            .fold(f64::INFINITY, |a, x| a.min(x[k]))
            - self.e[k]
    }
}

impl Bp<4> {
    /// N - p W for a point p (lanes x, y, z less p times lane w).
    fn about_point(&self, p: Vec3) -> Bp<4> {
        let mut out = *self;
        let p = [p.x, p.y, p.z];
        for x in &mut out.c[..self.len()] {
            for k in 0..3 {
                x[k] -= p[k] * x[3];
            }
        }
        let big = self.b;
        for k in 0..3 {
            out.e[k] = self.e[k] + p[k].abs() * self.e[3] + 3.0 * U * (big[k] + p[k].abs() * big[3]);
        }
        out.b = out.big();
        out
    }
}

/// Y = X - q W, q the ratio of the coefficient sums (near the box's middle), with how far the
/// control points Y_ij / W_ij reach from the origin and a bound on their rounding: P - q lies
/// in their hull. All W_ij must be positive. Coordinates are kept apart (x, y, z rows), so the
/// loops over coefficients run several at a time.
struct About {
    /// Y's coefficients.
    ys: [[f64; CAP]; 3],
    /// Each coefficient of Y within this of the true one, plus a rounding of the largest.
    ye: f64,
    /// The control points.
    pts: [[f64; CAP]; 3],
    /// W's coefficients.
    ws: [f64; CAP],
    reach: f64,
    blur: f64,
}

impl About {
    /// X from lanes `x..x + 3` of p, W from lane `w`; `big` bounds each lane's coefficients.
    fn of<const K: usize>(p: &Bp<K>, x: usize, w: usize, big: &[f64; K]) -> About {
        let len = p.len();
        let (mut sw, mut sx) = (0.0, [0.0; 3]);
        for c in &p.c[..len] {
            sw += c[w];
            for k in 0..3 {
                sx[k] += c[x + k];
            }
        }
        let q = sx.map(|v| v / sw);
        let (mut ys, mut pts, mut ws) = ([[0.0; CAP]; 3], [[0.0; CAP]; 3], [0.0; CAP]);
        let (mut r2, mut wmin) = (0.0f64, f64::INFINITY);
        let mut ybig = [0.0f64; 3];
        for i in 0..len {
            let c = &p.c[i];
            let y: [f64; 3] = std::array::from_fn(|k| c[x + k] - q[k] * c[w]);
            let v = y.map(|a| a / c[w]);
            for k in 0..3 {
                ybig[k] = ybig[k].max(y[k].abs());
                ys[k][i] = y[k];
                pts[k][i] = v[k];
            }
            ws[i] = c[w];
            r2 = r2.max(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
            wmin = wmin.min(c[w]);
        }
        // as Bp::about_point bounds Y's rounding
        let (we, wb) = (p.e[w], big[w]);
        let e: [f64; 3] =
            std::array::from_fn(|k| p.e[x + k] + q[k].abs() * we + 3.0 * U * (big[x + k] + q[k].abs() * wb));
        let ey = e.iter().fold(0.0f64, |a, &b| a.max(b)) * 1.8;
        let ye = (0..3).map(|k| e[k] + U * ybig[k]).fold(0.0, f64::max);
        let reach = r2.sqrt() * (1.0 + 4.0 * U);
        // v / w against the true ratio: v and w each off by their rounding
        let blur = (ey + reach * (we + 2.0 * U * wmin)) / (wmin - we);
        About {
            ys,
            ye,
            pts,
            ws,
            reach,
            blur,
        }
    }

    /// The sum of the coefficients of d/du (or d/dv) of Y: the rows' (columns') ends.
    fn axis(&self, m: usize, n: usize, along_u: bool) -> Vec3 {
        let mut s = Vec3::ZERO;
        let at = |t: usize| Vec3::new(self.ys[0][t], self.ys[1][t], self.ys[2][t]);
        if along_u {
            for j in 0..=n {
                s += at(m * (n + 1) + j) - at(j);
            }
        } else {
            for i in 0..=m {
                s += at(i * (n + 1) + n) - at(i * (n + 1));
            }
        }
        s
    }

    /// W P_u (or W P_v) is a Bernstein combination of K_ij - w_ij (P - q), the coefficients of
    /// d/du of Y and of W (the same basis), with P - q in the control points' hull: the tangents
    /// of the angles from `axis` (unit, across n0) it can make, None when it may not be within a
    /// quarter turn of the axis. W's error is `we`, its largest coefficient `wbig`.
    #[allow(clippy::too_many_arguments)]
    fn sector(
        &self,
        (m, n): (usize, usize),
        we: f64,
        wbig: f64,
        along_u: bool,
        n0: Vec3,
        axis: Vec3,
    ) -> Option<(f64, f64)> {
        let len = (m + 1) * (n + 1);
        let side = n0.cross(axis);
        let [px, py, pz] = &self.pts;
        let [yx, yy, yz] = &self.ys;
        // the control points along the axis and across it
        let (mut pl, mut ph, mut ql, mut qh) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
        for i in 0..len {
            let p = axis.x * px[i] + axis.y * py[i] + axis.z * pz[i];
            let q = side.x * px[i] + side.y * py[i] + side.z * pz[i];
            (pl, ph, ql, qh) = (pl.min(p), ph.max(p), ql.min(q), qh.max(q));
        }
        let deg = if along_u { m } else { n } as f64;
        // each difference of two coefficients, times the degree; the weights' rounding times
        // the points' reach; the points' rounding times the weights
        let e = self.ye * 2.0 * deg * 1.8
            + (we + U * wbig) * 2.0 * deg * (self.reach + self.blur)
            + self.blur * wbig * 2.0 * deg;
        let (ri, rj, step) = if along_u { (m, n + 1, n + 1) } else { (m + 1, n, 1) };
        let (mut lo, mut hi, mut least) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY);
        for i in 0..ri {
            let r = i * (n + 1);
            for t in r..r + rj {
                let u = t + step;
                let (ax, ay, az) = (
                    (yx[u] - yx[t]) * deg,
                    (yy[u] - yy[t]) * deg,
                    (yz[u] - yz[t]) * deg,
                );
                let wd = (self.ws[u] - self.ws[t]) * deg;
                // a - wd y: the part along the axis stays positive, the ratio across it is bounded
                let c = axis.x * ax + axis.y * ay + axis.z * az;
                let s = side.x * ax + side.y * ay + side.z * az;
                let up = wd >= 0.0;
                let (wl, wh) = if up {
                    (wd * pl, wd * ph)
                } else {
                    (wd * ph, wd * pl)
                };
                let (vl, vh) = if up {
                    (wd * ql, wd * qh)
                } else {
                    (wd * qh, wd * ql)
                };
                let (cl, ch) = (c - wh - e, c - wl + e);
                let (sl, sh) = (s - vh - e, s - vl + e);
                least = least.min(cl);
                lo = lo.min(sl / if sl >= 0.0 { ch } else { cl });
                hi = hi.max(sh / if sh >= 0.0 { cl } else { ch });
            }
        }
        if least <= 0.0 {
            return None;
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
/// The size of the box round a corner of the square proven in Duffy triangles (measured:
/// research/corner-patch).
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
        self.prove_zones(st) && self.blend_regions(st)
    }

    fn prove_zones(&self, st: &mut Proof) -> bool {
        for c in 0..4 {
            let pc = self.corner(c);
            // the zone: sides c (at its start) and c - 1 (at its end), alone
            for (i, at_start) in [(c, true), ((c + 3) % 4, false)] {
                let p = ribbon_tensor(&self.sides[i]).about_point(pc);
                let p = if at_start {
                    p.split(true, ZONE).0
                } else {
                    p.split(true, 1.0 - ZONE).1
                };
                st.regions += 1;
                if !plain_region(&p.split(false, ZONE).0, true, 0, st) {
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

    /// The blend over the square cut at 1/4 and 3/4 both ways: the box round each corner, the
    /// strips between them along the sides, and the middle (measured: research/corner-patch).
    fn blend_regions(&self, st: &mut Proof) -> bool {
        let cut = |p: &Bp<4>, along_u: bool, k: usize| match k {
            0 => p.split(along_u, CORNER_BOX).0,
            1 => p.restrict(along_u, CORNER_BOX, 1.0 - CORNER_BOX),
            _ => p.split(along_u, (1.0 - CORNER_BOX).next_down()).1,
        };
        (0..3).all(|a| {
            let pu = cut(&self.nw, true, a);
            (0..3).all(|b| {
                let p = cut(&pu, false, b);
                if a != 1 && b != 1 {
                    let c = [[0, 3], [1, 2]][a / 2][b / 2];
                    return self.corner_box(c, &p, st);
                }
                if a == 1 && b == 1 {
                    st.regions += 1;
                    return plain_box(&p, st);
                }
                // a strip between two corners' boxes: its halves (they rarely pass whole)
                let (h0, h1) = p.halves(a == 1);
                st.regions += 2;
                plain_box(&h0, st) && plain_box(&h1, st)
            })
        })
    }

    /// The box round corner c (N and W over it): its quarter at the corner in two Duffy
    /// triangles (whole, they nearly always pass at that size), the other three quarters plain.
    fn corner_box(&self, c: usize, p: &Bp<4>, st: &mut Proof) -> bool {
        // the corner to the origin, N about it: N - p_c W = sigma^3 G and W = sigma^2 W~ in
        // either triangle (every weight has u^2 or v^2, and the ribbons meet at p_c)
        let (fu, fv) = (c == 1 || c == 2, c >= 2);
        let p = if fu { p.flip(true) } else { *p };
        let p = if fv { p.flip(false) } else { p };
        let (pu0, pu1) = p.about_point(self.corner(c)).halves(true);
        let (p00, p01) = pu0.halves(false);
        let (p10, p11) = pu1.halves(false);
        st.regions += 3;
        if !(plain_box(&p01, st) && plain_box(&p10, st) && plain_box(&p11, st)) {
            return false;
        }
        (0..2).all(|tri| {
            let d = if tri == 0 { p00 } else { p00.transpose() }.duffy();
            let (g, wt) = (d.lanes::<3>(0).div_rows(3), d.lanes::<1>(3).div_rows(2));
            st.regions += 1;
            let Some(phi) = normal_phi(&wt, 0, 1) else {
                return false;
            };
            // sigma G and G raised a degree (the cones of W~ P_sigma and W~ P_t / sigma), W~
            let (g, wt) = (g.times(&phi), wt.times(&phi));
            let (h, ge) = (g.mul_affine(0.0, 1.0), g.elevate_u());
            let mut x = Bp::<7>::zero(wt.m, wt.n);
            for i in 0..x.len() {
                let (a, b) = (h.c[i], ge.c[i]);
                x.c[i] = [a[0], a[1], a[2], b[0], b[1], b[2], wt.c[i][0]];
            }
            x.e = [h.e[0], h.e[1], h.e[2], ge.e[0], ge.e[1], ge.e[2], wt.e[0]];
            x.b = [h.b[0], h.b[1], h.b[2], ge.b[0], ge.b[1], ge.b[2], wt.b[0]];
            duffy_region(&x, 0, st)
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
fn ribbon_tensor(sd: &RSide) -> Bp<4> {
    let mut out = Bp::zero(SDEG, 2);
    for a in 0..=SDEG {
        for b in 0..3 {
            for k in 0..3 {
                let (av, bv, cv) = (sd.abc[0][k].0[a], sd.abc[1][k].0[a], sd.abc[2][k].0[a]);
                out.c[a * 3 + b][k] = match b {
                    0 => av,
                    1 => av + 0.5 * bv,
                    _ => av + bv + cv,
                };
            }
            out.c[a * 3 + b][3] = sd.e.0[a];
        }
    }
    out.b = out.big();
    out.e = out.b.map(|b| 4.0 * U * b);
    out
}

/// Whether W P_u (from X1, lanes x1..x1 + 3 of p) and W P_v (from X2, lanes x2..x2 + 3) lie in
/// angular sectors of the plane across some n0 that are ordered and less than a half turn
/// apart, P = X1 / W + const = X2 / W + const', W lane w: then n0 . (P_u x P_v) > 0 over the box.
fn separated<const K: usize>(p: &Bp<K>, x1: usize, x2: usize, w: usize) -> bool {
    if p.lower(w) <= 0.0 {
        return false;
    }
    let big = p.b;
    let a1 = About::of(p, x1, w, &big);
    let a2 = (x2 != x1).then(|| About::of(p, x2, w, &big));
    let a2 = a2.as_ref().unwrap_or(&a1);
    let (sx, sy) = (a1.axis(p.m, p.n, true), a2.axis(p.m, p.n, false));
    let Some(n0) = sx.cross(sy).normalize() else {
        return false;
    };
    let n0 = n0.get();
    let flat = |v: Vec3| (v - n0 * n0.dot(v)).normalize().map(|u| u.get());
    let (Some(ua), Some(va)) = (flat(sx), flat(sy)) else {
        return false;
    };
    let (Some(a), Some(b)) = (
        a1.sector((p.m, p.n), p.e[w], big[w], true, n0, ua),
        a2.sector((p.m, p.n), p.e[w], big[w], false, n0, va),
    ) else {
        return false;
    };
    let beta = dmath::atan2(n0.dot(ua.cross(va)), ua.dot(va));
    // room for the rounding of the angles and tangents
    let slack = 1e-9;
    beta + dmath::atan(b.0) - dmath::atan(a.1) > slack
        && beta + dmath::atan(b.1) - dmath::atan(a.0) < dmath::PI - slack
}

/// A polynomial of degree (a, a) by its coefficients (the first (a + 1)^2).
type Phi = (usize, [f64; 9]);

/// Phi, biquadratic with 1 / W at the box's corners, edges' middles and middle (or bilinear
/// through the corners when that is not positive): W phi is nearly constant over the box and its
/// parts, so the shift (P - q) (W phi)_u is small. P = X phi / W phi. W is lane `w` of p.
fn normal_phi<const K: usize>(p: &Bp<K>, w: usize, order: usize) -> Option<Phi> {
    let (m, n) = (p.m, p.n);
    // W along v at u = 0, 1/2 (de Casteljau down each column), 1; each at v = 0, 1/2, 1
    let mut mid = [0.0; 2 * DEG + 2];
    let mut col = [0.0; 2 * DEG + 2];
    for j in 0..=n {
        for i in 0..=m {
            col[i] = p.c[i * (n + 1) + j][w];
        }
        for r in 1..=m {
            for i in 0..=m - r {
                col[i] = 0.5 * (col[i] + col[i + 1]);
            }
        }
        mid[j] = col[0];
    }
    let row = |i: usize| -> [f64; 2 * DEG + 2] {
        std::array::from_fn(|j| if j <= n { p.c[i * (n + 1) + j][w] } else { 0.0 })
    };
    let mut f = [[0.0; 3]; 3];
    for (i, r) in [row(0), mid, row(m)].into_iter().enumerate() {
        let mut c = r;
        for l in 1..=n {
            for k in 0..=n - l {
                c[k] = 0.5 * (c[k] + c[k + 1]);
            }
        }
        f[i] = [r[0], c[0], r[n]];
    }
    if f.iter().flatten().any(|&v| v <= 0.0) {
        return None;
    }
    let f = f.map(|r| r.map(|v| 1.0 / v));
    // the quadratic through f0, f1/2, f1 has the middle coefficient 2 f1/2 - (f0 + f1) / 2
    let mid = |a: f64, m: f64, b: f64| 2.0 * m - 0.5 * (a + b);
    let rows = f.map(|r| [r[0], mid(r[0], r[1], r[2]), r[2]]);
    let c: [f64; 9] = std::array::from_fn(|k| {
        let (i, j) = (k / 3, k % 3);
        if i == 1 {
            mid(rows[0][j], rows[1][j], rows[2][j])
        } else {
            rows[i][j]
        }
    });
    if order == 2 && c.iter().all(|&x| x > 0.0) {
        Some((2, c))
    } else {
        Some((1, [f[0][0], f[0][2], f[2][0], f[2][2], 0.0, 0.0, 0.0, 0.0, 0.0]))
    }
}

/// P = N / W regular over the box (N and W the lanes of p).
fn plain_box(p: &Bp<4>, st: &mut Proof) -> bool {
    let Some(phi) = normal_phi(p, 3, 2) else {
        return false;
    };
    plain_region(&p.times(&phi), true, 0, st)
}

/// P = N / W regular over the box; boxes that fail are split in halves.
fn plain_region(p: &Bp<4>, first_u: bool, depth: u32, st: &mut Proof) -> bool {
    st.tests += 1;
    st.depth = st.depth.max(depth);
    if separated(p, 0, 0, 3) {
        return true;
    }
    // halves, along u and v in turn
    if depth >= MAX_DEPTH {
        return false;
    }
    let u = depth.is_multiple_of(2) == first_u;
    let (h0, h1) = p.halves(u);
    plain_region(&h0, first_u, depth + 1, st) && plain_region(&h1, first_u, depth + 1, st)
}

/// A corner's Duffy triangle (x = sigma, y = sigma t) where N - p_c W = sigma^3 G and
/// W = sigma^2 W~: P - p_c = sigma G / W~ and (P - p_c) / sigma = G / W~, so the cones are of
/// W~ P_sigma (from sigma G, lanes 0..3 of p) and W~ P_t / sigma (from G, raised a degree in
/// sigma to match, lanes 3..6), both finite at the corner; W~ is lane 6. Parts of the triangle
/// carry the same polynomials over the part.
fn duffy_region(p: &Bp<7>, depth: u32, st: &mut Proof) -> bool {
    st.tests += 1;
    st.depth = st.depth.max(depth);
    if separated(p, 0, 3, 6) {
        return true;
    }
    // halves, along sigma and t in turn
    if depth >= MAX_DEPTH {
        return false;
    }
    let (h0, h1) = p.halves(depth.is_multiple_of(2));
    duffy_region(&h0, depth + 1, st) && duffy_region(&h1, depth + 1, st)
}
