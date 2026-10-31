//! The hole patch in rational form, for holes whose sides are lines and conics on planes,
//! cylinders and spheres. The same side-based blend as `HolePatch`, but every piece is a
//! polynomial: each side is its exact rational quadratic, the cross tangent a C' + b (n x C')
//! with n the neighbour's (rational) normal field, the blend homogeneous
//! (P = sum B_i Num_i / sum B_i Den_i with B_i = prod_(j != i) d_j^2), and the domain
//! distances d_i exact affine functions vanishing exactly at the polygon's corners. So over any
//! region of the domain, n . (P_u x P_v) W^3 is a polynomial, and its Bernstein coefficients
//! prove it positive: the patch cannot fold. See docs/fold-proof.md.

// polynomial coefficient loops read best indexed
#![allow(clippy::needless_range_loop)]

use super::{HolePatch, MAX_SIDES, Scalar, Side, SurfaceRail};
use crate::curve::{Curve, CurveGeom};
use crate::surface::Surface;
use forge_math::{Box3, Frame, Interval, Point2, Point3, Vec2, Vec3, dmath};
use std::sync::OnceLock;

const U: f64 = f64::EPSILON * 0.5;
/// The ribbons' damping h = d / (1 + d / H) uses H = DAMP times the side's height: with the
/// homogeneous blend, a little more damping than the ribbon form keeps the shape the same.
const DAMP: f64 = 0.7;

fn binom(n: usize, k: usize) -> f64 {
    static T: OnceLock<Vec<Vec<f64>>> = OnceLock::new();
    let t = T.get_or_init(|| {
        let mut t = vec![vec![1.0]];
        for n in 1..=128 {
            let p: &Vec<f64> = &t[n - 1];
            let mut r = vec![1.0; n + 1];
            for k in 1..n {
                r[k] = p[k - 1] + p[k];
            }
            t.push(r);
        }
        t
    });
    t[n][k]
}

/// A polynomial on [0, 1] in Bernstein form.
#[derive(Clone, Debug, PartialEq)]
pub struct B1(pub Vec<f64>);

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
    pub fn eval(&self, s: f64) -> f64 {
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
    /// The range over [lo, hi] (any interval) from the Bernstein coefficients there.
    fn range(&self, lo: f64, hi: f64) -> Interval {
        let m = self.deg();
        // monomial coefficients, then s = lo + (hi - lo) t, then back to Bernstein in t
        let mut mono = vec![0.0; m + 1];
        for (k, mk) in mono.iter_mut().enumerate() {
            for i in 0..=k {
                let sign = if (k - i) % 2 == 0 { 1.0 } else { -1.0 };
                *mk += self.0[i] * binom(m, k) * binom(k, i) * sign;
            }
        }
        let w = hi - lo;
        let mut sub = vec![0.0; m + 1];
        for (k, &mk) in mono.iter().enumerate() {
            // (lo + w t)^k
            for j in 0..=k {
                sub[j] += mk * binom(k, j) * lo.powi((k - j) as i32) * w.powi(j as i32);
            }
        }
        let (mut a, mut b) = (f64::INFINITY, f64::NEG_INFINITY);
        let mut big: f64 = 0.0;
        for i in 0..=m {
            let mut c = 0.0;
            for j in 0..=i {
                c += sub[j] * binom(i, j) / binom(m, j);
            }
            a = a.min(c);
            b = b.max(c);
            big = big.max(c.abs());
        }
        let scale: f64 = mono
            .iter()
            .enumerate()
            .map(|(k, c)| c.abs() * lo.abs().max(hi.abs()).max(1.0).powi(k as i32))
            .sum();
        let e = 1e-12 * (scale + big) + f64::MIN_POSITIVE;
        Interval::new(a - e, b + e)
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
    fn of(side: &Side) -> Option<(Arc3, &SurfaceRail)> {
        let sr = side.rail.surface_rail()?;
        let (t0, t1) = (side.t0, side.t1);
        let p = |t: f64| sr.curve.deriv(t);
        let pt = |q: Point3| q - Point3::ORIGIN;
        let quad = |c: [Vec3; 3], w: [f64; 3]| -> V1 {
            std::array::from_fn(|k| {
                let g = |v: Vec3| [v.x, v.y, v.z][k];
                B1(vec![g(c[0]) * w[0], g(c[1]) * w[1], g(c[2]) * w[2]])
            })
        };
        let (cw, d, arc) = match &sr.curve {
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
                (
                    quad([pt(p(t0).p), mid, pt(p(t1).p)], [1.0, w1, 1.0]),
                    B1(vec![1.0, w1, 1.0]),
                    Some((tm, {
                        let (sn, cs) = dmath::sin_cos(0.25 * delta);
                        sn / cs
                    })),
                )
            }
            _ => return None,
        };
        Some((Arc3 { cw, d, arc, t0, t1 }, sr))
    }
    fn s_of_t(&self, t: f64) -> f64 {
        match self.arc {
            None => (t - self.t0) / (self.t1 - self.t0),
            Some((tm, k)) => {
                0.5 * (1.0 + {
                    let (sn, cs) = dmath::sin_cos(0.5 * (t - tm));
                    sn / cs
                } / k)
            }
        }
    }
    /// D^2 C' = Cw' D - Cw D', of degree 2 for a rational quadratic (1 for a parabola).
    fn q(&self) -> V1 {
        if self.d.deg() == 0 {
            return std::array::from_fn(|i| self.cw[i].deriv());
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
        let (c, c1, c2) = (
            veval(&self.cw, s),
            Vec3::new(
                self.cw[0].deriv().eval(s),
                self.cw[1].deriv().eval(s),
                self.cw[2].deriv().eval(s),
            ),
            Vec3::new(
                self.cw[0].deriv().deriv().eval(s),
                self.cw[1].deriv().deriv().eval(s),
                self.cw[2].deriv().deriv().eval(s),
            ),
        );
        let (d, d1, d2) = (
            self.d.eval(s),
            self.d.deriv().eval(s),
            self.d.deriv().deriv().eval(s),
        );
        let p = c / d;
        let p1 = (c1 - p * d1) / d;
        let p2 = (c2 - p1 * (2.0 * d1) - p * d2) / d;
        (p, p1, p2)
    }
}

/// A ribbon: R(s, h) = (A + h B + h^2 C) / E, all of degree q in s.
#[derive(Clone, Debug)]
struct RSide {
    a: V1,
    b: V1,
    c: V1,
    e: B1,
    /// First and second s derivatives of a, b, c, e, for evaluation.
    da: [V1; 2],
    db: [V1; 2],
    dc: [V1; 2],
    de: [B1; 2],
    q: usize,
    arc: Arc3,
}

/// The rational patch data (the domain is the `HolePatch`'s).
#[derive(Clone, Debug)]
pub struct Rational {
    sides: Vec<RSide>,
    corners: Vec<Point2>,
    lam: Vec<f64>,
    heights: Vec<f64>,
    /// Where both distances of a corner are under this, the nearest ribbon is the patch.
    cut: f64,
}

/// The neighbour's outward normal along a side, times D for a sphere or cylinder (a polynomial
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
            // v - z (z . v)
            let along = v[0].scale(z.x).add(&v[1].scale(z.y)).add(&v[2].scale(z.z));
            let f = vadd1(&v, &vmul1(&vk1(-z), &along));
            Some(std::array::from_fn(|i| f[i].scale(sg / cy.radius)))
        }
        _ => None,
    }
}

impl Rational {
    pub fn build(p: &HolePatch) -> Option<Rational> {
        let n = p.sides.len();
        let mut arcs = Vec::with_capacity(n);
        let mut rails = Vec::with_capacity(n);
        for s in &p.sides {
            let (a, r) = Arc3::of(s)?;
            arcs.push(a);
            rails.push(r);
        }
        // the hole's corners exactly shared: each side starts where the one before ends
        for i in 0..n {
            let prev = (i + n - 1) % n;
            let (w_end, w_start) = (*arcs[prev].d.0.last().unwrap(), arcs[i].d.0[0]);
            for k in 0..3 {
                let e = *arcs[prev].cw[k].0.last().unwrap() / w_end;
                arcs[i].cw[k].0[0] = e * w_start;
            }
        }
        let corners = p.corners.clone();
        let lam: Vec<f64> = (0..n)
            .map(|i| 1.0 / (corners[(i + 1) % n] - corners[i]).length())
            .collect();
        let edge = |i: usize| corners[(i + 1) % n] - corners[i];
        let cr = |a: Vec2, b: Vec2| a.x * b.y - a.y * b.x;
        let mut sides = Vec::with_capacity(n);
        for i in 0..n {
            let arc = &arcs[i];
            let q = arc.q();
            // T = (a Q + b E) / D^2 = a C' + b D^k (n x C'), E = (n D^k) x Q, inward for b > 0
            let e = cross1(&normal_d(rails[i], arc)?, &q);
            let dd = arc.d.mul(&arc.d);
            let tq = |s: f64| (veval(&q, s) / dd.eval(s), veval(&e, s) / dd.eval(s));
            let solve = |s: f64, target: Vec3| -> Option<(f64, f64)> {
                let (u, v) = tq(s);
                forge_math::linalg::solve2(
                    u.dot(u),
                    u.dot(v),
                    u.dot(v),
                    v.dot(v),
                    target.dot(u),
                    target.dot(v),
                )
            };
            let (prev, next) = (&arcs[(i + n - 1) % n], &arcs[(i + 1) % n]);
            let (lp, li, ln) = (edge((i + n - 1) % n), edge(i), edge((i + 1) % n));
            let (c0, c1) = (arc.jet(0.0), arc.jet(1.0));
            // s along side i is affine (x - c_i) . L_i / |L_i|^2; the ribbons of neighbouring sides
            // agree to first order at the corner, and to second along the neighbour's curve
            let (sg0, dl0) = (-lp.dot(li) / li.length_sq(), lam[i] * cr(lp, li));
            let (sg1, dl1) = (ln.dot(li) / li.length_sq(), lam[i] * cr(li, ln));
            let t0 = (-prev.jet(1.0).1 - c0.1 * sg0) / dl0;
            let t1 = (next.jet(0.0).1 - c1.1 * sg1) / dl1;
            let (a0, b0) = solve(0.0, t0)?;
            let (a1, b1) = solve(1.0, t1)?;
            if !(b0 > 0.0 && b1 > 0.0) {
                return None;
            }
            // a C' and b D^k (n x C'): through the middle, the frame components |C'| a and
            // D^k |C'| b interpolate linearly between the ends (as the ribbon form's do)
            let k = if matches!(rails[i].surface, Surface::Plane(_)) {
                0
            } else {
                1
            };
            let speed = |s: f64| arc.jet(s).1.length();
            let dk = |s: f64| arc.d.eval(s).powi(k);
            let (al0, al1) = (a0 * speed(0.0), a1 * speed(1.0));
            let (be0, be1) = (b0 * dk(0.0) * speed(0.0), b1 * dk(1.0) * speed(1.0));
            let am = 0.5 * (al0 + al1) / speed(0.5);
            let bm = 0.5 * (be0 + be1) / (dk(0.5) * speed(0.5));
            let quad = |e0: f64, m: f64, e1: f64| B1(vec![e0, 2.0 * m - 0.5 * (e0 + e1), e1]);
            let (la, lb) = (quad(a0, am, a1), quad(b0, bm, b1));
            if lb.0.iter().any(|&x| x <= 0.0) {
                return None;
            }
            let bv: V1 = std::array::from_fn(|k| la.mul(&q[k]).add(&lb.mul(&e[k])));
            // T' at the ends, from T = B / D^2
            let tdot = |s: f64| {
                let bd: Vec3 = Vec3::new(
                    bv[0].deriv().eval(s),
                    bv[1].deriv().eval(s),
                    bv[2].deriv().eval(s),
                );
                let (b0, d2, d2d) = (veval(&bv, s), dd.eval(s), dd.deriv().eval(s));
                (bd - b0 * (d2d / d2)) / d2
            };
            let hh = p.heights[i];
            // R = C + h T + h^2 K: along the neighbour, d = tau dl, s = end + tau sg, h = d - d^2 / H ..
            let kend = |target2: Vec3, c2: Vec3, sg: f64, dl: f64, t: Vec3, td: Vec3| {
                (target2 - c2 * (sg * sg) - td * (2.0 * dl * sg) + t * (2.0 * dl * dl / hh)) / (2.0 * dl * dl)
            };
            let k0 = kend(prev.jet(1.0).2, c0.2, sg0, dl0, t0, tdot(0.0));
            let k1 = kend(next.jet(0.0).2, c1.2, sg1, dl1, t1, tdot(1.0));
            // K / D linear in s (times D: of degree 1 + deg D)
            let kp: V1 = std::array::from_fn(|k| {
                B1(vec![
                    [k0.x, k0.y, k0.z][k] * arc.d.eval(0.0),
                    [k1.x, k1.y, k1.z][k] * arc.d.eval(1.0),
                ])
            });
            let av: V1 = std::array::from_fn(|k| arc.cw[k].mul(&arc.d));
            let cv: V1 = std::array::from_fn(|k| kp[k].mul(&arc.d));
            let q = [&av, &bv, &cv]
                .iter()
                .flat_map(|v| v.iter().map(|b| b.deg()))
                .max()
                .unwrap()
                .max(dd.deg());
            let el = |v: &V1| -> V1 { std::array::from_fn(|k| v[k].elevate(q)) };
            let dv = |v: &V1| -> [V1; 2] {
                let d1: V1 = std::array::from_fn(|k| v[k].deriv());
                let d2: V1 = std::array::from_fn(|k| d1[k].deriv());
                [d1, d2]
            };
            let (a, b, c, e) = (el(&av), el(&bv), el(&cv), dd.elevate(q));
            sides.push(RSide {
                da: dv(&a),
                db: dv(&b),
                dc: dv(&c),
                de: [e.deriv(), e.deriv().deriv()],
                a,
                b,
                c,
                e,
                q,
                arc: arc.clone(),
            });
        }
        Some(Rational {
            sides,
            corners,
            lam,
            heights: p.heights.iter().map(|h| h * DAMP).collect(),
            cut: 1e-6 * p.side,
        })
    }

    pub fn side_s(&self, i: usize, t: f64) -> f64 {
        self.sides[i].arc.s_of_t(t)
    }

    /// s_i = (x - c_i) . L_i / |L_i|^2 and its gradient.
    pub(super) fn param(&self, i: usize, x: Point2) -> (f64, Vec2) {
        let n = self.corners.len();
        let (a, b) = (self.corners[i], self.corners[(i + 1) % n]);
        let e = b - a;
        let g = e / e.length_sq();
        ((x - a).dot(g), g)
    }

    /// The domain point of side i at s.
    pub(super) fn side_uv(&self, i: usize, s: f64) -> Point2 {
        let n = self.corners.len();
        self.corners[i].lerp(self.corners[(i + 1) % n], s)
    }

    pub(super) fn dist(&self, i: usize, x: Point2) -> (f64, Vec2) {
        let n = self.corners.len();
        let (a, b) = (self.corners[i], self.corners[(i + 1) % n]);
        let e = b - a;
        let l = self.lam[i];
        (
            l * (e.x * (x - a).y - e.y * (x - a).x),
            Vec2::new(-e.y * l, e.x * l),
        )
    }

    pub(super) fn eval<S: Scalar>(&self, x: Point2) -> S::V {
        let n = self.corners.len();
        let mut d = [S::konst(0.0); MAX_SIDES];
        let (mut k, mut second) = (0, f64::INFINITY);
        for i in 0..n {
            let (v, g) = self.dist(i, x);
            d[i] = S::affine(v.max(0.0), g.x, g.y);
            if i > 0 && v < d[k].value() {
                second = d[k].value();
                k = i;
            } else if i > 0 {
                second = second.min(v);
            }
        }
        let mut r = [S::zero_v(); MAX_SIDES];
        let mut om = [S::konst(0.0); MAX_SIDES];
        for i in 0..n {
            let (sv0, sg) = self.param(i, x);
            let s = S::affine(sv0, sg.x, sg.y);
            let sd = &self.sides[i];
            let hh = self.heights[i];
            let hp = d[i] + S::konst(hh);
            let h = d[i] * S::konst(hh) * hp.inv();
            let sv = sv0;
            let ds = s - S::konst(sv);
            let along = |v: &V1, d: &[V1; 2]| -> S::V {
                if S::JET {
                    s.along(veval(v, sv), veval(&d[0], sv), veval(&d[1], sv))
                } else {
                    s.along(veval(v, sv), Vec3::ZERO, Vec3::ZERO)
                }
            };
            let e = if S::JET {
                S::konst(sd.e.eval(sv))
                    + ds * S::konst(sd.de[0].eval(sv))
                    + ds * ds * S::konst(0.5 * sd.de[1].eval(sv))
            } else {
                S::konst(sd.e.eval(sv))
            };
            let num = along(&sd.a, &sd.da) + along(&sd.b, &sd.db) * h + along(&sd.c, &sd.dc) * (h * h);
            r[i] = num * e.inv();
            // Den = E(s) (H + d)^2, the ribbon's share of the homogeneous blend
            // Den = E(s) (H + d)^2 (2H - d)^2: the last factor cancels most of (H + d)^2's growth
            // across the domain, so the blend's weights stay close to d^-2
            let m = S::konst(2.0 * hh) - d[i];
            om[i] = e * hp * hp * m * m;
        }
        if second <= self.cut {
            return r[k];
        }
        // B_i Den_i relative to the nearest side's
        let mut w = [S::konst(0.0); MAX_SIDES];
        let mut sum = S::konst(0.0);
        for i in 0..n {
            let q = d[k] * d[i].inv();
            w[i] = if i == k { om[i] } else { q * q * om[i] };
            sum = sum + w[i];
        }
        let inv = sum.inv();
        let mut out = r[k];
        for i in 0..n {
            if i != k {
                out = out + (r[i] - r[k]) * (w[i] * inv);
            }
        }
        out
    }

    /// A box holding the patch over the domain points with distances in `d` (the patch is a
    /// convex combination of its ribbons there).
    pub(super) fn bbox(&self, d: &[Interval], sr: &[Interval]) -> Box3 {
        let n = self.corners.len();
        let mut hull = Box3::EMPTY;
        for i in 0..n {
            let s = sr[i];
            let hh = self.heights[i];
            let damp = |x: f64| x / (1.0 + x / hh);
            let h = Interval::new(
                damp(d[i].lo.max(0.0)).next_down().max(0.0),
                damp(d[i].hi.max(0.0)).next_up().next_up(),
            );
            let sd = &self.sides[i];
            let e = sd.e.range(s.lo, s.hi);
            let rib: [Interval; 3] = std::array::from_fn(|k| {
                let num = sd.a[k].range(s.lo, s.hi)
                    + h * sd.b[k].range(s.lo, s.hi)
                    + h.sqr() * sd.c[k].range(s.lo, s.hi);
                num.div(&e)
            });
            hull = hull.hull(&Box3 {
                x: rib[0],
                y: rib[1],
                z: rib[2],
            });
        }
        let m = hull
            .x
            .lo
            .abs()
            .max(hull.x.hi.abs())
            .max(hull.y.lo.abs().max(hull.y.hi.abs()))
            .max(hull.z.lo.abs().max(hull.z.hi.abs()));
        hull.grow(64.0 * f64::EPSILON * m)
    }

    pub(super) fn transformed(&self, f: &Frame) -> Rational {
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
            s.a = tv(&s.a, &s.e, true);
            s.b = tv(&s.b, &s.e, false);
            s.c = tv(&s.c, &s.e, false);
            s.arc.cw = tv(&s.arc.cw, &s.arc.d, true);
            let dv = |v: &V1| -> [V1; 2] {
                let d1: V1 = std::array::from_fn(|k| v[k].deriv());
                let d2: V1 = std::array::from_fn(|k| d1[k].deriv());
                [d1, d2]
            };
            (s.da, s.db, s.dc) = (dv(&s.a), dv(&s.b), dv(&s.c));
        }
        out
    }
}

/// A polynomial over [0, 1]^2 in tensor Bernstein form, each coefficient within `e` of the true
/// one (rounding of its construction).
#[derive(Clone, Debug)]
struct Bp {
    m: usize,
    n: usize,
    c: Vec<f64>,
    e: f64,
}

impl Bp {
    fn konst(v: f64) -> Bp {
        Bp {
            m: 0,
            n: 0,
            c: vec![v],
            e: 0.0,
        }
    }
    /// The bilinear polynomial with these corner values: (u, v) = (0,0), (0,1), (1,0), (1,1).
    fn bilinear(v: [Interval; 4]) -> Bp {
        let mut e: f64 = 0.0;
        let c: Vec<f64> = v
            .iter()
            .map(|i| {
                let m = i.mid();
                e = e.max((i.hi - m).max(m - i.lo));
                m
            })
            .collect();
        Bp { m: 1, n: 1, c, e }
    }
    fn at(&self, i: usize, j: usize) -> f64 {
        self.c[i * (self.n + 1) + j]
    }
    fn big(&self) -> f64 {
        self.c.iter().fold(0.0f64, |a, &x| a.max(x.abs()))
    }
    fn mul(&self, o: &Bp) -> Bp {
        let (m, n) = (self.m + o.m, self.n + o.n);
        // in the scaled basis (coefficients times binomials) a product is a plain convolution
        let scaled = |p: &Bp| -> Vec<f64> {
            let mut c = p.c.clone();
            for i in 0..=p.m {
                for j in 0..=p.n {
                    c[i * (p.n + 1) + j] *= binom(p.m, i) * binom(p.n, j);
                }
            }
            c
        };
        let (a, b) = (scaled(self), scaled(o));
        let mut c = vec![0.0; (m + 1) * (n + 1)];
        for i in 0..=self.m {
            for j in 0..=self.n {
                let x = a[i * (self.n + 1) + j];
                if x == 0.0 {
                    continue;
                }
                for k in 0..=o.m {
                    let row = (i + k) * (n + 1) + j;
                    let orow = &b[k * (o.n + 1)..(k + 1) * (o.n + 1)];
                    let out = &mut c[row..row + o.n + 1];
                    for l in 0..=o.n {
                        out[l] += x * orow[l];
                    }
                }
            }
        }
        for i in 0..=m {
            for j in 0..=n {
                c[i * (n + 1) + j] /= binom(m, i) * binom(n, j);
            }
        }
        // each scaled term carries a few roundings; the unscaled sum of weights is 1
        let terms = ((self.m.min(o.m) + 1) * (self.n.min(o.n) + 1)) as f64;
        let (ba, bb) = (self.big(), o.big());
        let e = self.e * bb + o.e * ba + self.e * o.e + (terms + 16.0) * U * ba * bb;
        Bp {
            m,
            n,
            c,
            e: e * (1.0 + 1e-12),
        }
    }
    fn elevate(&self, m: usize, n: usize) -> Bp {
        let mut p = self.clone();
        while p.m < m {
            let (pm, pn) = (p.m, p.n);
            let mut c = vec![0.0; (pm + 2) * (pn + 1)];
            for i in 0..=pm + 1 {
                let t = i as f64 / (pm + 1) as f64;
                for j in 0..=pn {
                    let a = if i > 0 { p.at(i - 1, j) } else { 0.0 };
                    let b = if i <= pm { p.at(i, j) } else { 0.0 };
                    c[i * (pn + 1) + j] = t * a + (1.0 - t) * b;
                }
            }
            p = Bp {
                m: pm + 1,
                n: pn,
                e: p.e + 2.0 * U * p.big(),
                c,
            };
        }
        while p.n < n {
            let (pm, pn) = (p.m, p.n);
            let mut c = vec![0.0; (pm + 1) * (pn + 2)];
            for i in 0..=pm {
                for j in 0..=pn + 1 {
                    let t = j as f64 / (pn + 1) as f64;
                    let a = if j > 0 { p.at(i, j - 1) } else { 0.0 };
                    let b = if j <= pn { p.at(i, j) } else { 0.0 };
                    c[i * (pn + 2) + j] = t * a + (1.0 - t) * b;
                }
            }
            p = Bp {
                m: pm,
                n: pn + 1,
                e: p.e + 2.0 * U * p.big(),
                c,
            };
        }
        p
    }
    fn lin(&self, s: f64, o: &Bp, t: f64) -> Bp {
        let (m, n) = (self.m.max(o.m), self.n.max(o.n));
        let (a, b) = (self.elevate(m, n), o.elevate(m, n));
        let c: Vec<f64> = a.c.iter().zip(&b.c).map(|(x, y)| s * x + t * y).collect();
        let big = c.iter().fold(0.0f64, |a, &x| a.max(x.abs()));
        Bp {
            m,
            n,
            e: (a.e * s.abs() + b.e * t.abs() + 2.0 * U * (big + s.abs() * a.big() + t.abs() * b.big()))
                * (1.0 + 1e-12),
            c,
        }
    }
    fn add(&self, o: &Bp) -> Bp {
        self.lin(1.0, o, 1.0)
    }
    fn sub(&self, o: &Bp) -> Bp {
        self.lin(1.0, o, -1.0)
    }
    fn scale(&self, s: f64) -> Bp {
        let c: Vec<f64> = self.c.iter().map(|x| x * s).collect();
        Bp {
            m: self.m,
            n: self.n,
            e: (self.e * s.abs() + U * self.big() * s.abs()) * (1.0 + 1e-12),
            c,
        }
    }
    fn du(&self) -> Bp {
        let (m, n) = (self.m, self.n);
        if m == 0 {
            return Bp::konst(0.0);
        }
        let mut c = vec![0.0; m * (n + 1)];
        for i in 0..m {
            for j in 0..=n {
                c[i * (n + 1) + j] = m as f64 * (self.at(i + 1, j) - self.at(i, j));
            }
        }
        Bp {
            m: m - 1,
            n,
            e: (2.0 * m as f64 * self.e + 4.0 * m as f64 * U * self.big()) * (1.0 + 1e-12),
            c,
        }
    }
    fn dv(&self) -> Bp {
        let (m, n) = (self.m, self.n);
        if n == 0 {
            return Bp::konst(0.0);
        }
        let mut c = vec![0.0; (m + 1) * n];
        for i in 0..=m {
            for j in 0..n {
                c[i * n + j] = n as f64 * (self.at(i, j + 1) - self.at(i, j));
            }
        }
        Bp {
            m,
            n: n - 1,
            e: (2.0 * n as f64 * self.e + 4.0 * n as f64 * U * self.big()) * (1.0 + 1e-12),
            c,
        }
    }
    /// The two halves along u (at 1/2) or v.
    fn halves(&self, along_u: bool) -> (Bp, Bp) {
        let (m, n) = (self.m, self.n);
        let mut l = self.clone();
        let mut r = self.clone();
        if along_u {
            for j in 0..=n {
                let mut col: Vec<f64> = (0..=m).map(|i| self.at(i, j)).collect();
                l.c[j] = col[0];
                r.c[m * (n + 1) + j] = col[m];
                for k in 1..=m {
                    for i in 0..=m - k {
                        col[i] = 0.5 * (col[i] + col[i + 1]);
                    }
                    l.c[k * (n + 1) + j] = col[0];
                    r.c[(m - k) * (n + 1) + j] = col[m - k];
                }
            }
        } else {
            for i in 0..=m {
                let mut row: Vec<f64> = (0..=n).map(|j| self.at(i, j)).collect();
                l.c[i * (n + 1)] = row[0];
                r.c[i * (n + 1) + n] = row[n];
                for k in 1..=n {
                    for j in 0..=n - k {
                        row[j] = 0.5 * (row[j] + row[j + 1]);
                    }
                    l.c[i * (n + 1) + k] = row[0];
                    r.c[i * (n + 1) + n - k] = row[n - k];
                }
            }
        }
        // averaging is exact up to one rounding per level
        let e = self.e + ((m.max(n) + 1) as f64) * 2.0 * U * self.big();
        l.e = e;
        r.e = e;
        (l, r)
    }
    fn lower(&self) -> f64 {
        self.c.iter().fold(f64::INFINITY, |a, &x| a.min(x)) - self.e
    }
    fn upper(&self) -> f64 {
        self.c.iter().fold(f64::NEG_INFINITY, |a, &x| a.max(x)) + self.e
    }
}

/// An exact real point c + t e, as intervals.
fn ipt(c: Point2, t: f64, e: [Interval; 2]) -> [Interval; 2] {
    [Interval::point(c.x) + e[0] * t, Interval::point(c.y) + e[1] * t]
}

/// The outcome of a proof.
#[derive(Clone, Copy, Debug, Default)]
pub struct Proof {
    pub proved: bool,
    pub regions: usize,
    pub pieces: usize,
    pub max_degree: usize,
}

/// One region: d_j as polynomials over [0, 1]^2, and where the sides a, b of a corner were
/// factored out (the weights over sigma^2).
struct Reg {
    d: Vec<Bp>,
    /// s_i as polynomials.
    s: Vec<Bp>,
    corner: Option<(usize, Bp)>,
    /// The map's Jacobian sign (from a sample), and the normal to measure against.
    sign: f64,
    n0: Vec3,
}

impl Rational {
    /// d_j at an exact real point.
    fn idist(&self, j: usize, x: [Interval; 2]) -> Interval {
        let n = self.corners.len();
        let (a, b) = (self.corners[j], self.corners[(j + 1) % n]);
        let ex = Interval::point(b.x) + Interval::point(-a.x);
        let ey = Interval::point(b.y) + Interval::point(-a.y);
        let rx = x[0] + Interval::point(-a.x);
        let ry = x[1] + Interval::point(-a.y);
        (ex * ry + -(ey * rx)) * self.lam[j]
    }

    /// s_i at an exact real point.
    fn iparam(&self, j: usize, x: [Interval; 2]) -> Interval {
        let n = self.corners.len();
        let (a, b) = (self.corners[j], self.corners[(j + 1) % n]);
        let ex = Interval::point(b.x) + Interval::point(-a.x);
        let ey = Interval::point(b.y) + Interval::point(-a.y);
        let rx = x[0] + Interval::point(-a.x);
        let ry = x[1] + Interval::point(-a.y);
        (ex * rx + ey * ry).div(&(ex.sqr() + ey.sqr()))
    }

    /// The homogeneous ribbon over a region: (Num, Den), with s_i and d_i as polynomials there.
    fn ribbon_poly(&self, i: usize, sp: &Bp, di: &Bp) -> ([Bp; 3], Bp) {
        let sd = &self.sides[i];
        let q = sd.q;
        let one_minus = Bp::konst(1.0).sub(sp);
        let mut ap = vec![Bp::konst(1.0)];
        let mut bp = vec![Bp::konst(1.0)];
        for k in 1..=q {
            ap.push(ap[k - 1].mul(sp));
            bp.push(bp[k - 1].mul(&one_minus));
        }
        let mono: Vec<Bp> = (0..=q)
            .map(|k| ap[k].mul(&bp[q - k]).scale(binom(q, k)))
            .collect();
        let hom = |f: &B1| -> Bp {
            let mut acc = mono[0].scale(f.0[0]);
            for k in 1..=q {
                acc = acc.add(&mono[k].scale(f.0[k]));
            }
            acc
        };
        let hh = self.heights[i];
        let hp = di.add(&Bp::konst(hh));
        let hp2 = hp.mul(&hp);
        let mid = di.mul(&hp).scale(hh);
        let end = di.mul(di).scale(hh * hh);
        let m = Bp::konst(2.0 * hh).sub(di);
        let m2 = m.mul(&m);
        let num = std::array::from_fn(|k| {
            hom(&sd.a[k])
                .mul(&hp2)
                .add(&hom(&sd.b[k]).mul(&mid))
                .add(&hom(&sd.c[k]).mul(&end))
                .mul(&m2)
        });
        (num, hom(&sd.e).mul(&hp2).mul(&m2))
    }

    /// N and W over a region.
    fn blend_poly(&self, reg: &Reg) -> ([Bp; 3], Bp) {
        let n = self.corners.len();
        let sq: Vec<Bp> = reg.d.iter().map(|x| x.mul(x)).collect();
        let weights: Vec<Bp> = match &reg.corner {
            None => products_but_one(&sq),
            Some((c, sigma)) => {
                let (a, b) = ((c + n - 1) % n, *c);
                // d_a = sigma alpha, d_b = sigma beta: divide every weight by sigma^2
                let far: Vec<usize> = (0..n).filter(|&j| j != a && j != b).collect();
                let fsq: Vec<Bp> = far.iter().map(|&j| sq[j].clone()).collect();
                let fprod = products_but_one(&fsq);
                let mut all = Bp::konst(1.0);
                for s in &fsq {
                    all = all.mul(s);
                }
                let (al, be) = (reg.d[a].clone(), reg.d[b].clone());
                // alpha = d_a / sigma etc. are given by the caller through `sigma`: d = sigma x
                let al2 = al.mul(&al);
                let be2 = be.mul(&be);
                let ss = sigma.mul(sigma);
                // with d_a = sigma alpha: B_a / sigma^2 = beta^2 prod, B_far / sigma^2 = sigma^2 alpha^2 beta^2 prod'
                let mut w: Vec<Bp> = vec![Bp::konst(0.0); n];
                // here reg.d[a], reg.d[b] hold alpha and beta (not d_a, d_b)
                w[a] = be2.mul(&all);
                w[b] = al2.mul(&all);
                let ab2 = al2.mul(&be2).mul(&ss);
                for (q, &j) in far.iter().enumerate() {
                    w[j] = ab2.mul(&fprod[q]);
                }
                w
            }
        };
        // the ribbons see the true distances
        let dtrue: Vec<Bp> = match &reg.corner {
            None => reg.d.clone(),
            Some((c, sigma)) => {
                let (a, b) = ((c + n - 1) % n, *c);
                let mut d = reg.d.clone();
                d[a] = reg.d[a].mul(sigma);
                d[b] = reg.d[b].mul(sigma);
                d
            }
        };
        let mut num: [Bp; 3] = std::array::from_fn(|_| Bp::konst(0.0));
        let mut den = Bp::konst(0.0);
        for i in 0..n {
            let (rn, rd) = self.ribbon_poly(i, &reg.s[i], &dtrue[i]);
            for k in 0..3 {
                num[k] = num[k].add(&weights[i].mul(&rn[k]));
            }
            den = den.add(&weights[i].mul(&rd));
        }
        (num, den)
    }

    /// Whether n0 . (P_u x P_v) has the region's sign everywhere: Some(true) proven,
    /// Some(false) a sub-box surely of the wrong sign (folds), None undecided.
    fn prove_poly(num: &[Bp; 3], den: &Bp, reg: &Reg, stats: &mut Proof) -> bool {
        Self::prove_poly_in(num, den, reg, stats) == Some(true)
    }

    fn prove_poly_in(num: &[Bp; 3], den: &Bp, reg: &Reg, stats: &mut Proof) -> Option<bool> {
        if den.lower() <= 0.0 {
            return None;
        }
        // J is the same about any origin: about a point of the region, N is small and the
        // rounding bounds (relative to the operands' size) with it
        let mid = den.c.len() / 2;
        let num: [Bp; 3] = std::array::from_fn(|k| num[k].sub(&den.scale(num[k].c[mid] / den.c[mid])));
        let n0 = reg.n0;
        let e1 = if n0.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
        let e1 = (e1 - n0 * n0.dot(e1)).normalize().map_or(Vec3::Z, |u| u.get());
        let e2 = n0.cross(e1);
        let proj = |v: Vec3| num[0].scale(v.x).add(&num[1].scale(v.y)).add(&num[2].scale(v.z));
        let (n1, n2) = (proj(e1), proj(e2));
        let (n1u, n1v, n2u, n2v) = (n1.du(), n1.dv(), n2.du(), n2.dv());
        let (wu, wv) = (den.du(), den.dv());
        let x = n1u.mul(&n2v).sub(&n2u.mul(&n1v));
        let y = n1.mul(&n2v).sub(&n2.mul(&n1v));
        let z = n1u.mul(&n2).sub(&n2u.mul(&n1));
        let q = den.mul(&x).sub(&wu.mul(&y)).sub(&wv.mul(&z)).scale(reg.sign);
        stats.max_degree = stats.max_degree.max(q.m.max(q.n));
        let mut todo = vec![(q, 0u32)];
        while let Some((p, depth)) = todo.pop() {
            stats.pieces += 1;
            if p.lower() > 0.0 {
                continue;
            }
            // n0 . J < 0 somewhere is not a fold (the normal may have turned past 90 degrees):
            // the caller splits the region and measures against nearer normals
            if p.upper() < 0.0
                || [p.at(0, 0), p.at(p.m, 0), p.at(0, p.n), p.at(p.m, p.n)]
                    .iter()
                    .any(|&v| v < -p.e)
            {
                return Some(false);
            }
            if depth >= 8 {
                return None;
            }
            let (a, b) = p.halves(depth % 2 == 0);
            todo.push((a, depth + 1));
            todo.push((b, depth + 1));
        }
        Some(true)
    }

    /// Proves the patch never folds over the domain, region by region.
    pub fn prove(&self, patch: &HolePatch) -> Proof {
        let n = self.corners.len();
        let mut st = Proof::default();
        let normal_at = |x: Point2| -> Option<Vec3> {
            let d = <HolePatch as crate::surface::Procedural>::deriv(patch, x);
            d.du.cross(d.dv).normalize().map(|u| u.get())
        };
        let ie = |a: Point2, b: Point2| {
            [
                Interval::point(b.x) + Interval::point(-a.x),
                Interval::point(b.y) + Interval::point(-a.y),
            ]
        };
        let reach: f64 = 0.4;
        let mut cut_pts: Vec<[[Interval; 2]; 2]> = Vec::with_capacity(n);
        for c in 0..n {
            let (a, b) = ((c + n - 1) % n, c);
            let cp = self.corners[c];
            // e_a runs along side b to the next corner, e_b along side a back
            let ea = ie(cp, self.corners[(c + 1) % n]);
            let eb = ie(cp, self.corners[(c + n - 1) % n]);
            let ka = self.idist(a, ipt(cp, 1.0, ea));
            let kb = self.idist(b, ipt(cp, 1.0, eb));
            let kmax = ka.hi.max(kb.hi);
            // the evaluation takes the nearest ribbon where d_a, d_b <= cut: sigma < cut / kmax
            let s0 = self.cut / kmax * (1.0 - 1e-9);
            // the zone d_a, d_b <= cut: p e_a + q e_b with p <= cut / ka, q <= cut / kb
            let (pa, pb) = (self.cut / ka.lo * (1.0 + 1e-9), self.cut / kb.lo * (1.0 + 1e-9));
            // (u, v) -> c + u pb e_b + v pa e_a
            let zone = [
                ipt(cp, 0.0, ea),
                ipt(cp, pa, ea),
                ipt(cp, pb, eb),
                [ipt(cp, pa, ea)[0] + eb[0] * pb, ipt(cp, pa, ea)[1] + eb[1] * pb],
            ];
            let dz: Vec<Bp> = (0..n)
                .map(|j| Bp::bilinear(zone.map(|x| self.idist(j, x))))
                .collect();
            let sz: Vec<Bp> = (0..n)
                .map(|j| Bp::bilinear(zone.map(|x| self.iparam(j, x))))
                .collect();
            let mid = Point2::new(
                cp.x + 1e-3 * (self.corners[(c + 1) % n].x - cp.x),
                cp.y + 1e-3 * (self.corners[(c + 1) % n].y - cp.y),
            );
            let Some(n0) = normal_at(mid) else {
                return st;
            };
            // the map (u, v) -> c + u pb e_b + v pa e_a: Jacobian sign of (e_b, e_a)
            let s_zone = (eb[0].mid() * ea[1].mid() - eb[1].mid() * ea[0].mid()).signum();
            for i in [a, b] {
                let (rn, rd) = self.ribbon_poly(i, &sz[i], &dz[i]);
                let reg = Reg {
                    d: Vec::new(),
                    s: Vec::new(),
                    corner: None,
                    sign: s_zone,
                    n0,
                };
                st.regions += 1;
                if !Self::prove_poly(&rn, &rd, &reg, &mut st) {
                    return st;
                }
            }
            // the corner: x = c + sigma (w e_a + (1 - w) e_b), in shells so the polynomials'
            // range (which sets their rounding bound) stays small in each
            let at = |sg: f64, w: f64| {
                [
                    ipt(cp, sg * w, ea)[0] + eb[0] * (sg * (1.0 - w)),
                    ipt(cp, sg * w, ea)[1] + eb[1] * (sg * (1.0 - w)),
                ]
            };
            let s1 = (1e-3 * reach).max(4.0 * s0);
            for (lo, hi) in [(s0, s1), (s1, reach)] {
                if !self.prove_corner(c, (lo, hi), (0.0, 1.0), (ea, eb, ka, kb), &normal_at, 0, &mut st) {
                    return st;
                }
            }
            cut_pts.push([at(reach, 0.0), at(reach, 1.0)]);
        }
        // the middle: the polygon with the corners' triangles cut off, as quads round a centre
        let mut ring: Vec<[Interval; 2]> = Vec::with_capacity(2 * n);
        for c in 0..n {
            ring.push(cut_pts[c][0]);
            ring.push(cut_pts[c][1]);
        }
        let mut cx = 0.0;
        let mut cy = 0.0;
        for p in &ring {
            cx += p[0].mid();
            cy += p[1].mid();
        }
        let o = [
            Interval::point(cx / ring.len() as f64),
            Interval::point(cy / ring.len() as f64),
        ];
        let m = ring.len();
        for k in (0..m).step_by(2) {
            // bilinear over (o, p0, p2, p1): u from o toward p0..p1, v across
            if !self.prove_quad(
                [o, ring[k], ring[(k + 2) % m], ring[(k + 1) % m]],
                &normal_at,
                0,
                &mut st,
            ) {
                return st;
            }
        }
        st.proved = true;
        st
    }

    /// A quad (vertices at (u, v) = (0,0), (0,1), (1,0), (1,1)), split in four where its
    /// normal turns too far for one direction to measure against.
    fn prove_quad(
        &self,
        q: [[Interval; 2]; 4],
        normal_at: &dyn Fn(Point2) -> Option<Vec3>,
        depth: u32,
        st: &mut Proof,
    ) -> bool {
        let n = self.corners.len();
        let mx = |p: [Interval; 2]| Vec2::new(p[0].mid(), p[1].mid());
        let ctr = Point2::ORIGIN + (mx(q[0]) + mx(q[1]) + mx(q[2]) + mx(q[3])) * 0.25;
        let xu = (mx(q[2]) - mx(q[0]) + mx(q[3]) - mx(q[1])) * 0.5;
        let xv = (mx(q[1]) - mx(q[0]) + mx(q[3]) - mx(q[2])) * 0.5;
        let Some(n0) = normal_at(ctr) else {
            return false;
        };
        let reg = Reg {
            d: (0..n)
                .map(|j| Bp::bilinear(q.map(|x| self.idist(j, x))))
                .collect(),
            s: (0..n)
                .map(|j| Bp::bilinear(q.map(|x| self.iparam(j, x))))
                .collect(),
            corner: None,
            sign: (xu.x * xv.y - xu.y * xv.x).signum(),
            n0,
        };
        let (num, den) = self.blend_poly(&reg);
        st.regions += 1;
        if Self::prove_poly(&num, &den, &reg, st) {
            return true;
        }
        if depth >= 3 {
            return false;
        }
        let at = |u: f64, v: f64| -> [Interval; 2] {
            std::array::from_fn(|k| {
                q[0][k] * ((1.0 - u) * (1.0 - v))
                    + q[1][k] * ((1.0 - u) * v)
                    + q[2][k] * (u * (1.0 - v))
                    + q[3][k] * (u * v)
            })
        };
        [(0.0, 0.0), (0.0, 0.5), (0.5, 0.0), (0.5, 0.5)]
            .into_iter()
            .all(|(u, v)| {
                let sub = [at(u, v), at(u, v + 0.5), at(u + 0.5, v), at(u + 0.5, v + 0.5)];
                self.prove_quad(sub, normal_at, depth + 1, st)
            })
    }

    /// The corner region sigma in `sg`, w in `w`, split likewise.
    #[allow(clippy::too_many_arguments)]
    fn prove_corner(
        &self,
        c: usize,
        sg: (f64, f64),
        w: (f64, f64),
        frame: ([Interval; 2], [Interval; 2], Interval, Interval),
        normal_at: &dyn Fn(Point2) -> Option<Vec3>,
        depth: u32,
        st: &mut Proof,
    ) -> bool {
        let n = self.corners.len();
        let (a, b) = ((c + n - 1) % n, c);
        let (ea, eb, ka, kb) = frame;
        let cp = self.corners[c];
        let at = |s: f64, w: f64| {
            [
                ipt(cp, s * w, ea)[0] + eb[0] * (s * (1.0 - w)),
                ipt(cp, s * w, ea)[1] + eb[1] * (s * (1.0 - w)),
            ]
        };
        let quad = [at(sg.0, w.0), at(sg.0, w.1), at(sg.1, w.0), at(sg.1, w.1)];
        let mut d: Vec<Bp> = (0..n)
            .map(|j| Bp::bilinear(quad.map(|x| self.idist(j, x))))
            .collect();
        let rad = |k: Interval| (k.hi - k.mid()).max(k.mid() - k.lo);
        // alpha = d_a / sigma = w ka, beta = (1 - w) kb, exactly
        d[a] = Bp {
            m: 0,
            n: 1,
            c: vec![w.0 * ka.mid(), w.1 * ka.mid()],
            e: rad(ka) * 1.01,
        };
        d[b] = Bp {
            m: 0,
            n: 1,
            c: vec![(1.0 - w.0) * kb.mid(), (1.0 - w.1) * kb.mid()],
            e: rad(kb) * 1.01,
        };
        let sigma = Bp {
            m: 1,
            n: 0,
            c: vec![sg.0, sg.1],
            e: 0.0,
        };
        let (sm, wm) = (0.5 * (sg.0 + sg.1), 0.5 * (w.0 + w.1));
        let p = at(sm, wm);
        let Some(n0) = normal_at(Point2::new(p[0].mid(), p[1].mid())) else {
            return false;
        };
        // dx/du = (s1 - s0) q, dx/dw = sigma (e_a - e_b): det = -(..) sigma det[e_a, e_b]
        let reg = Reg {
            d,
            s: (0..n)
                .map(|j| Bp::bilinear(quad.map(|x| self.iparam(j, x))))
                .collect(),
            corner: Some((c, sigma)),
            sign: -(ea[0].mid() * eb[1].mid() - ea[1].mid() * eb[0].mid()).signum(),
            n0,
        };
        let (num, den) = self.blend_poly(&reg);
        st.regions += 1;
        if Self::prove_poly(&num, &den, &reg, st) {
            return true;
        }
        if depth >= 3 {
            return false;
        }
        [(sg.0, sm), (sm, sg.1)].into_iter().all(|s2| {
            [(w.0, wm), (wm, w.1)]
                .into_iter()
                .all(|w2| self.prove_corner(c, s2, w2, frame, normal_at, depth + 1, st))
        })
    }
}

fn products_but_one(x: &[Bp]) -> Vec<Bp> {
    let n = x.len();
    let mut pre = vec![Bp::konst(1.0); n + 1];
    for i in 0..n {
        pre[i + 1] = pre[i].mul(&x[i]);
    }
    let mut out = vec![Bp::konst(1.0); n];
    let mut suf = Bp::konst(1.0);
    for i in (0..n).rev() {
        out[i] = pre[i].mul(&suf);
        suf = suf.mul(&x[i]);
    }
    out
}
