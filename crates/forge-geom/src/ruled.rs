//! Ruled surfaces: straight lines between matched points of two exact curves, as a loft makes
//! between sections no plane, cylinder or cone fits. S(t, s) = A(t) + s (B(t) - A(t)), t the
//! fraction along both curves, s the fraction along the ruling. Points come straight from the
//! two curves.
//!
//! The surface is carried on past the face, a ruling's length past each end and up to a quarter
//! of the way past each side (as far as it stays regular). The implicit value is the signed
//! distance to its nearest point and projection searches all of it, so a point of the zero set
//! off the face projects to parameters off the face.

use crate::curve::{Circle, CurveGeom, Domain1};
use crate::screw::polish;
use crate::surface::{Domain2, Proc, Procedural, SurfDeriv, Surface};
use forge_math::{Box2, Box3, Frame, Interval, Point2, Point3, Vec3, tol};
use std::sync::Arc;

/// One of the two curves a ruled face spans, by the fraction t along it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rail {
    Line {
        a: Point3,
        b: Point3,
    },
    /// The arc of `circle` from angle `start` through `sweep`.
    Arc {
        circle: Circle,
        start: f64,
        sweep: f64,
    },
}

impl Rail {
    /// Point and first two derivatives at fraction t.
    pub fn at(&self, t: f64) -> (Point3, Vec3, Vec3) {
        match *self {
            Rail::Line { a, b } => (a.lerp(b, t), b - a, Vec3::ZERO),
            Rail::Arc { circle, start, sweep } => {
                let d = circle.deriv(start + sweep * t);
                (d.p, d.d1 * sweep, d.d2 * (sweep * sweep))
            }
        }
    }

    fn bbox(&self, t: Interval) -> Box3 {
        match *self {
            Rail::Line { .. } => Box3::from_points([self.at(t.lo).0, self.at(t.hi).0]),
            Rail::Arc { circle, start, sweep } => {
                circle.bbox(Interval::new(start + sweep * t.lo, start + sweep * t.hi))
            }
        }
    }

    fn transformed(&self, f: &Frame) -> Rail {
        match *self {
            Rail::Line { a, b } => Rail::Line {
                a: f.point_to_world(a),
                b: f.point_to_world(b),
            },
            Rail::Arc { circle, start, sweep } => Rail::Arc {
                circle: circle.transformed(f),
                start,
                sweep,
            },
        }
    }

    fn length(&self) -> f64 {
        match *self {
            Rail::Line { a, b } => a.distance(b),
            Rail::Arc { circle, sweep, .. } => circle.radius * sweep.abs(),
        }
    }

    /// How far past each end the curve can be carried and stay one turn at most.
    fn reach(&self) -> f64 {
        match *self {
            Rail::Line { .. } => 0.25,
            Rail::Arc { sweep, .. } => (0.5 * (forge_math::dmath::TAU / sweep.abs() - 1.0)).clamp(0.0, 0.25),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Ruled {
    a: Rail,
    b: Rail,
    /// The carried-on parameter box the implicit value reads.
    ext: Box2,
    /// Rulings (t, A(t), B(t) - A(t)) at even steps over `ext.u`, seeding nearest-point searches.
    rows: Vec<(f64, Point3, Vec3)>,
    scale: (f64, f64),
    radius: f64,
}

/// Largest principal curvature magnitude from first and second derivatives.
fn principal_max(d: &SurfDeriv) -> f64 {
    let Some(n) = d.du.cross(d.dv).normalize().map(|u| u.get()) else {
        return 0.0;
    };
    let (e, f, g) = (d.du.length_sq(), d.du.dot(d.dv), d.dv.length_sq());
    let (l, m, nn) = (d.duu.dot(n), d.duv.dot(n), d.dvv.dot(n));
    let det = e * g - f * f;
    if det <= 0.0 {
        return 0.0;
    }
    let k = (l * nn - m * m) / det;
    let h = (e * nn - 2.0 * f * m + g * l) / (2.0 * det);
    h.abs() + (h * h - k).max(0.0).sqrt()
}

impl Ruled {
    /// The surface ruled between `a` and `b`; None where it folds (its normal turns over or
    /// vanishes) on the face.
    pub fn new(a: Rail, b: Rail) -> Option<Ruled> {
        let te = a.reach().min(b.reach());
        let mut r = Ruled {
            a,
            b,
            ext: Box2::new(Interval::new(-te, 1.0 + te), Interval::new(-1.0, 2.0)),
            rows: Vec::new(),
            scale: (a.length().max(b.length()), 0.0),
            radius: f64::INFINITY,
        };
        // the normal is linear in s along each ruling: it vanishes (the surface folds) between
        // s0 and s1 where the segment between its values there passes the origin
        let regular = |t: Interval, s0: f64, s1: f64| {
            (0..=64).all(|i| {
                let t = t.lo + t.width() * i as f64 / 64.0;
                let n = |s: f64| {
                    let d = r.point(Point2::new(t, s));
                    d.du.cross(d.dv)
                };
                let (m0, m1) = (n(s0), n(s1));
                let d = m1 - m0;
                let l = (-m0.dot(d) / d.length_sq().max(1e-300)).clamp(0.0, 1.0);
                (m0 + d * l).length() > 1e-6 * m0.length().max(m1.length()).max(1e-6)
            })
        };
        let face = Interval::new(0.0, 1.0);
        if !regular(face, 0.0, 1.0) {
            return None;
        }
        // carried on only as far as it stays regular
        let te = [te, 0.5 * te, 0.25 * te]
            .into_iter()
            .find(|&e| regular(Interval::new(-e, 1.0 + e), 0.0, 1.0))
            .unwrap_or(0.0);
        let t = Interval::new(-te, 1.0 + te);
        let lo = [-1.0, -0.5, -0.25]
            .into_iter()
            .find(|&s| regular(t, s, 1.0))
            .unwrap_or(0.0);
        let hi = [2.0, 1.5, 1.25]
            .into_iter()
            .find(|&s| regular(t, 0.0, s))
            .unwrap_or(1.0);
        r.ext = Box2::new(t, Interval::new(lo, hi));
        let sweep = |rail: &Rail| match *rail {
            Rail::Line { .. } => 0.0,
            Rail::Arc { sweep, .. } => sweep.abs(),
        };
        let k = (16.0 + 8.0 * sweep(&a).max(sweep(&b))).ceil() as usize;
        r.rows = (0..=k)
            .map(|i| {
                let t = r.ext.u.lo + r.ext.u.width() * i as f64 / k as f64;
                let (pa, _, _) = a.at(t);
                (t, pa, b.at(t).0 - pa)
            })
            .collect();
        let mut kmax: f64 = 0.0;
        let mut ruling: f64 = 0.0;
        for i in 0..=8 {
            for j in 0..=4 {
                let d = r.point(Point2::new(i as f64 / 8.0, j as f64 / 4.0));
                kmax = kmax.max(principal_max(&d));
                ruling = ruling.max(d.dv.length());
            }
        }
        r.scale.1 = ruling;
        r.radius = if kmax > 0.0 { 1.0 / kmax } else { f64::INFINITY };
        Some(r)
    }

    pub fn surface(self) -> Surface {
        Surface::Procedural(Proc(Arc::new(self)))
    }

    fn point(&self, uv: Point2) -> SurfDeriv {
        let (pa, da, dda) = self.a.at(uv.x);
        let (pb, db, ddb) = self.b.at(uv.x);
        let s = uv.y;
        SurfDeriv {
            p: pa + (pb - pa) * s,
            du: da + (db - da) * s,
            dv: pb - pa,
            duu: dda + (ddb - dda) * s,
            duv: db - da,
            dvv: Vec3::ZERO,
        }
    }

    /// Parameters of the point nearest p within `dom`.
    fn nearest(&self, p: Point3, dom: Box2) -> Point2 {
        let mut best = (f64::INFINITY, Point2::new(dom.u.mid(), dom.v.mid()));
        let mut take = |t: f64, pa: Point3, d: Vec3| {
            let w = p - pa;
            let s = (w.dot(d) / d.length_sq()).clamp(dom.v.lo, dom.v.hi);
            let e = (w - d * s).length_sq();
            if e < best.0 {
                best = (e, Point2::new(t, s));
            }
        };
        for &(t, pa, d) in &self.rows {
            if dom.u.contains(t) {
                take(t, pa, d);
            }
        }
        for t in [dom.u.lo, dom.u.hi] {
            let (pa, _, _) = self.a.at(t);
            take(t, pa, self.b.at(t).0 - pa);
        }
        polish(&|uv| self.point(uv), p, best.1, dom)
    }

    /// Signed distance to the carried-on surface and its gradient.
    fn value(&self, p: Point3) -> (f64, Vec3) {
        let d = self.point(self.nearest(p, self.ext));
        let n = d.du.cross(d.dv).normalize().map_or(Vec3::Z, |u| u.get());
        ((p - d.p).dot(n), n)
    }
}

fn lerp_box(a: &Box3, b: &Box3, s: f64) -> Box3 {
    let mix = |x: Interval, y: Interval| x * (1.0 - s) + y * s;
    Box3 {
        x: mix(a.x, b.x),
        y: mix(a.y, b.y),
        z: mix(a.z, b.z),
    }
}

impl Procedural for Ruled {
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        self.point(uv)
    }
    fn domain(&self) -> Domain2 {
        let unit = Domain1 {
            lo: 0.0,
            hi: 1.0,
            period: None,
        };
        Domain2 { u: unit, v: unit }
    }
    fn project(&self, p: Point3) -> Point2 {
        self.nearest(p, self.ext)
    }
    fn bbox(&self, uv: Box2) -> Box3 {
        // each ruling runs between its points at the box's two s, which lie in these boxes
        let (ba, bb) = (self.a.bbox(uv.u), self.b.bbox(uv.u));
        lerp_box(&ba, &bb, uv.v.lo)
            .hull(&lerp_box(&ba, &bb, uv.v.hi))
            .grow(tol::LINEAR)
    }
    fn implicit(&self, p: Point3) -> f64 {
        self.value(p).0
    }
    fn gradient(&self, p: Point3) -> Vec3 {
        self.value(p).1
    }
    fn transformed(&self, f: &Frame) -> Proc {
        let mut r = self.clone();
        r.a = self.a.transformed(f);
        r.b = self.b.transformed(f);
        r.rows = self
            .rows
            .iter()
            .map(|&(t, p, d)| (t, f.point_to_world(p), f.vec_to_world(d)))
            .collect();
        Proc(Arc::new(r))
    }
    fn uv_scale(&self) -> (f64, f64) {
        (self.scale.0.max(tol::LINEAR), self.scale.1.max(tol::LINEAR))
    }
    fn min_radius(&self) -> f64 {
        let arc = |r: &Rail| match *r {
            Rail::Line { .. } => f64::INFINITY,
            Rail::Arc { circle, .. } => circle.radius,
        };
        0.8 * self.radius.min(arc(&self.a)).min(arc(&self.b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SurfaceGeom;
    use crate::testing::rand_frame;
    use forge_math::rng::Rng;

    fn random_ruled(r: &mut Rng) -> Ruled {
        loop {
            let f = rand_frame(r);
            let g = f.with_origin(f.point_to_world(Point3::new(
                r.range(-2.0, 2.0),
                r.range(-2.0, 2.0),
                r.range(4.0, 12.0),
            )));
            let rail = |r: &mut Rng, f: &Frame| {
                if r.bool() {
                    Rail::Line {
                        a: f.point_to_world(Point3::new(r.range(-6.0, -2.0), r.range(-3.0, 3.0), 0.0)),
                        b: f.point_to_world(Point3::new(r.range(2.0, 6.0), r.range(-3.0, 3.0), 0.0)),
                    }
                } else {
                    Rail::Arc {
                        circle: Circle {
                            frame: f.with_origin(f.point_to_world(Point3::new(0.0, r.range(-1.0, 1.0), 0.0))),
                            radius: r.range(3.0, 8.0),
                        },
                        start: r.range(-0.5, 0.5),
                        sweep: r.range(0.5, 2.5),
                    }
                }
            };
            let (a, b) = (rail(r, &f), rail(r, &g));
            if let Some(s) = Ruled::new(a, b) {
                return s;
            }
        }
    }

    fn rand_uv(r: &mut Rng) -> Point2 {
        Point2::new(r.range(0.0, 1.0), r.range(0.0, 1.0))
    }

    #[test]
    fn derivatives_match_finite_differences() {
        let mut r = Rng::new(520);
        for _ in 0..300 {
            let s = random_ruled(&mut r);
            let uv = rand_uv(&mut r);
            let h = 1e-6;
            let d = s.point(uv);
            let at = |du: f64, dv: f64| s.point(Point2::new(uv.x + du, uv.y + dv));
            let close =
                |a: Vec3, b: Vec3| assert!((a - b).length() < 1e-5 * (1.0 + b.length()), "{a:?} {b:?}");
            close((at(h, 0.0).p - at(-h, 0.0).p) / (2.0 * h), d.du);
            close((at(0.0, h).p - at(0.0, -h).p) / (2.0 * h), d.dv);
            close((at(h, 0.0).du - at(-h, 0.0).du) / (2.0 * h), d.duu);
            close((at(0.0, h).du - at(0.0, -h).du) / (2.0 * h), d.duv);
        }
    }

    #[test]
    fn project_inverts_eval_and_is_nearest() {
        let mut r = Rng::new(521);
        for _ in 0..100 {
            let s = random_ruled(&mut r);
            let surf = s.clone().surface();
            for _ in 0..5 {
                let uv = rand_uv(&mut r);
                let d = s.point(uv);
                assert!(surf.eval(surf.project(d.p)).distance(d.p) < 1e-10, "{s:?} {uv:?}");
                let n = d.du.cross(d.dv).normalize().unwrap();
                let off = d.p + n * r.range(-0.5, 0.5);
                let q = surf.eval(surf.project(off));
                let best = (0..=100)
                    .flat_map(|i| (0..=40).map(move |j| (i, j)))
                    .map(|(i, j)| {
                        s.point(Point2::new(i as f64 / 100.0, j as f64 / 40.0))
                            .p
                            .distance(off)
                    })
                    .fold(f64::INFINITY, f64::min);
                assert!(q.distance(off) <= best + 1e-9, "{s:?} {uv:?}");
            }
        }
    }

    #[test]
    fn implicit_is_signed_distance_near_the_face() {
        let mut r = Rng::new(522);
        for _ in 0..200 {
            let s = random_ruled(&mut r);
            let uv = rand_uv(&mut r);
            let d = s.point(uv);
            let n = d.du.cross(d.dv).normalize().unwrap();
            assert!(s.value(d.p).0.abs() < 1e-10);
            let t = r.range(-0.05, 0.05);
            let (f, g) = s.value(d.p + n * t);
            assert!((f - t).abs() < 1e-9, "{f} {t}");
            assert!(g.dot(n.get()) > 1.0 - 1e-6);
        }
    }

    #[test]
    fn bbox_contains_samples() {
        let mut r = Rng::new(523);
        for _ in 0..200 {
            let s = random_ruled(&mut r);
            let (a, b) = (rand_uv(&mut r), rand_uv(&mut r));
            let bx = Box2::new(Interval::new(a.x, b.x), Interval::new(a.y, b.y));
            let bb = Procedural::bbox(&s, bx);
            for i in 0..=10 {
                for j in 0..=10 {
                    let q = Point2::new(
                        (bx.u.lo + bx.u.width() * i as f64 / 10.0).min(bx.u.hi),
                        (bx.v.lo + bx.v.width() * j as f64 / 10.0).min(bx.v.hi),
                    );
                    assert!(bb.contains(s.point(q).p));
                }
            }
        }
    }

    #[test]
    fn a_folded_ruling_is_refused() {
        let line = |a: [f64; 3], b: [f64; 3]| Rail::Line {
            a: Point3::new(a[0], a[1], a[2]),
            b: Point3::new(b[0], b[1], b[2]),
        };
        // the far edge turned half way round: every ruling passes through the axis
        assert!(
            Ruled::new(
                line([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
                line([1.0, 0.0, 5.0], [-1.0, 0.0, 5.0])
            )
            .is_none()
        );
        // a quarter turn twists but stays regular
        let twisted = Ruled::new(
            line([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
            line([0.0, -1.0, 5.0], [0.0, 1.0, 5.0]),
        );
        assert!(twisted.is_some());
    }
}
