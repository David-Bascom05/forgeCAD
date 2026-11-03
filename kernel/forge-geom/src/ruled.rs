//! Ruled surfaces: straight lines between matched points of two exact curves, as a loft makes
//! between sections no plane, cylinder or cone fits. S(t, s) = A(t) + s (B(t) - A(t)), t the
//! fraction along both curves, s the fraction along the ruling. Points come straight from the
//! two curves.
//!
//! The surface is carried on past the face, a ruling's length past each end and up to a quarter
//! of the way past each side (as far as it stays regular). The implicit value is the signed
//! distance to its nearest point and projection searches all of it, so a point of the zero set
//! off the face projects to parameters off the face.

use crate::curve::{Circle, CurveGeom, Domain1, Parabola};
use crate::screw::{KeptSeeds, fresh_id, normal_turn, polish, remembered};
use crate::surface::{Domain2, Proc, Procedural, ShapeBounds, SurfDeriv, Surface};
use crate::wrap::Wrap;
use forge_math::{Box2, Box3, Frame, Interval, Point2, Point3, Vec3, tol};
use std::sync::Arc;

mod meet;
pub use meet::{line_wall, meet_ruled, parabola_wall, wrap_wall};

/// One of the two curves a ruled face spans, by the fraction t along it.
#[derive(Clone, Debug, PartialEq)]
pub enum Rail {
    /// A free curve over its whole range.
    Curve(Arc<crate::spline::Spline>),
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
    /// The parabola over its parameter range.
    Parabola {
        curve: Parabola,
        range: Interval,
    },
    /// A flat curve wrapped round a cylinder.
    Wrap(Wrap),
}

impl Rail {
    /// Point and first two derivatives at fraction t.
    pub fn at(&self, t: f64) -> (Point3, Vec3, Vec3) {
        match *self {
            Rail::Curve(ref c) => {
                let r = c.domain();
                let w = r.width();
                let d = c.deriv(r.lo + w * t);
                (d.p, d.d1 * w, d.d2 * (w * w))
            }
            Rail::Line { a, b } => (a.lerp(b, t), b - a, Vec3::ZERO),
            Rail::Arc { circle, start, sweep } => {
                let d = circle.deriv(start + sweep * t);
                (d.p, d.d1 * sweep, d.d2 * (sweep * sweep))
            }
            Rail::Parabola { curve, range } => {
                let w = range.width();
                let d = curve.deriv(range.lo + w * t);
                (d.p, d.d1 * w, d.d2 * (w * w))
            }
            Rail::Wrap(w) => w.at(t),
        }
    }

    /// Bounds on |C'|, |C''| and |C'''| over the fractions `t` (None for a wrapped curve).
    pub(crate) fn derivative_bounds(&self, t: Interval) -> Option<[f64; 3]> {
        match *self {
            Rail::Curve(ref c) => {
                let r = c.domain();
                let w = r.width();
                let [a, b, d] = c.derivative_bounds(Interval::new(r.lo + w * t.lo, r.lo + w * t.hi));
                Some([a * w, b * w * w, d * w * w * w])
            }
            Rail::Line { a, b } => Some([a.distance(b), 0.0, 0.0]),
            Rail::Arc { circle, sweep, .. } => {
                let k = circle.radius * sweep.abs();
                Some([k, k * sweep.abs(), k * sweep * sweep])
            }
            Rail::Parabola { curve, range } => {
                // C' = w (2 k x, 1) in the parabola's frame, C'' = w^2 (2 k, 0), C''' = 0
                let (w, k2) = (range.width(), 0.5 / curve.focal.abs());
                let x = (range.lo + w * t.lo).abs().max((range.lo + w * t.hi).abs());
                Some([w * (1.0 + (k2 * x) * (k2 * x)).sqrt(), w * w * k2, 0.0])
            }
            Rail::Wrap(_) => None,
        }
    }

    pub(crate) fn bbox(&self, t: Interval) -> Box3 {
        match *self {
            Rail::Curve(ref c) => {
                let r = c.domain();
                let w = r.width();
                c.bbox(Interval::new(r.lo + w * t.lo, r.lo + w * t.hi))
            }
            Rail::Line { .. } => Box3::from_points([self.at(t.lo).0, self.at(t.hi).0]),
            Rail::Arc { circle, start, sweep } => {
                circle.bbox(Interval::new(start + sweep * t.lo, start + sweep * t.hi))
            }
            Rail::Parabola { curve, range } => {
                let w = range.width();
                curve.bbox(Interval::new(range.lo + w * t.lo, range.lo + w * t.hi))
            }
            Rail::Wrap(w) => w.bbox(t),
        }
    }

    pub(crate) fn transformed(&self, f: &Frame) -> Rail {
        match *self {
            Rail::Curve(ref c) => Rail::Curve(Arc::new(c.mapped(|p| f.point_to_world(p)))),
            Rail::Line { a, b } => Rail::Line {
                a: f.point_to_world(a),
                b: f.point_to_world(b),
            },
            Rail::Arc { circle, start, sweep } => Rail::Arc {
                circle: circle.transformed(f),
                start,
                sweep,
            },
            Rail::Parabola { curve, range } => Rail::Parabola {
                curve: curve.transformed(f),
                range,
            },
            Rail::Wrap(w) => Rail::Wrap(w.transformed(f)),
        }
    }

    pub(crate) fn length(&self) -> f64 {
        match *self {
            Rail::Line { a, b } => a.distance(b),
            Rail::Arc { circle, sweep, .. } => circle.radius * sweep.abs(),
            // chords: a little short, as a scale
            _ => (0..16)
                .map(|i| {
                    self.at(i as f64 / 16.0)
                        .0
                        .distance(self.at((i + 1) as f64 / 16.0).0)
                })
                .sum(),
        }
    }

    /// How far past each end the curve can be carried and stay one turn at most.
    pub(crate) fn reach(&self) -> f64 {
        match *self {
            Rail::Line { .. } | Rail::Parabola { .. } | Rail::Curve(_) => 0.25,
            Rail::Wrap(w) => w.flat.reach(),
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
    /// The same at the two ends of `ext.u`, read off the rails.
    ends: [(f64, Point3, Vec3); 2],
    /// How far a point of the carried-on surface can lie from the nearest of `rows`.
    reach: f64,
    scale: (f64, f64),
    radius: f64,
    /// Which surface this is, for remembering its last query.
    id: u64,
    /// Set when the rails are one parabola and its translate square to its plane (a parabolic
    /// cylinder): nearest points are then the parabola's own, solved in closed form.
    extruded: bool,
}

/// Largest principal curvature magnitude from first and second derivatives.
pub(crate) fn principal_max(d: &SurfDeriv) -> f64 {
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
            a: a.clone(),
            b: b.clone(),
            ext: Box2::new(Interval::new(-te, 1.0 + te), Interval::new(-1.0, 2.0)),
            rows: Vec::new(),
            ends: [(0.0, Point3::ORIGIN, Vec3::ZERO); 2],
            reach: 0.0,
            scale: (a.length().max(b.length()), 0.0),
            radius: f64::INFINITY,
            id: fresh_id(),
            extruded: false,
        };
        if let (&Rail::Parabola { curve: ca, range: ra }, &Rail::Parabola { curve: cb, range: rb }) = (&a, &b)
        {
            let (fa, fb) = (&ca.frame, &cb.frame);
            let d = fb.origin - fa.origin;
            r.extruded = ra == rb
                && ca.focal == cb.focal
                && fa.x() == fb.x()
                && fa.y() == fb.y()
                && d.cross(fa.z().get()).length() <= 1e-12 * d.length();
        }
        // the normal is linear in s along each ruling: it vanishes (the surface folds) between
        // s0 and s1 where the segment between its values there passes the origin
        // (a wall extruded square to its rails' plane has the same normal all along a ruling, and a
        // parabola's tangent is never zero: nothing to look for)
        let extruded = r.extruded;
        let regular = |t: Interval, s0: f64, s1: f64| {
            extruded
                || (0..=64).all(|i| {
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
        // a rail of no length is an apex: the wall stops there, and the normal vanishes only there
        let (a_apex, b_apex) = (a.length() == 0.0, b.length() == 0.0);
        let (te, lo, hi) = if a_apex || b_apex {
            let (s0, s1) = (
                if a_apex { 1e-3 } else { 0.0 },
                if b_apex { 1.0 - 1e-3 } else { 1.0 },
            );
            if !regular(Interval::new(0.0, 1.0), s0, s1) {
                return None;
            }
            let te = [te, 0.5 * te, 0.25 * te]
                .into_iter()
                .find(|&e| regular(Interval::new(-e, 1.0 + e), s0, s1))
                .unwrap_or(0.0);
            let t = Interval::new(-te, 1.0 + te);
            let lo = if a_apex {
                0.0
            } else {
                [-1.0, -0.5, -0.25]
                    .into_iter()
                    .find(|&s| regular(t, s, s1))
                    .unwrap_or(0.0)
            };
            let hi = if b_apex {
                1.0
            } else {
                [2.0, 1.5, 1.25]
                    .into_iter()
                    .find(|&s| regular(t, s0, s))
                    .unwrap_or(1.0)
            };
            (te, lo, hi)
        } else if let Some((ra, rb)) = r.radii() {
            // a wall of rays from the axis folds only where it reaches the axis
            let keeps = |s: f64| (ra + (rb - ra) * s) * ra > 0.0;
            if !(keeps(0.0) && keeps(1.0)) {
                return None;
            }
            let lo = [-1.0, -0.5, -0.25].into_iter().find(|&s| keeps(s)).unwrap_or(0.0);
            let hi = [2.0, 1.5, 1.25].into_iter().find(|&s| keeps(s)).unwrap_or(1.0);
            (te, lo, hi)
        } else {
            if !regular(Interval::new(0.0, 1.0), 0.0, 1.0) {
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
            (te, lo, hi)
        };
        let t = Interval::new(-te, 1.0 + te);
        r.ext = Box2::new(t, Interval::new(lo, hi));
        let sweep = |rail: &Rail| match *rail {
            Rail::Line { .. } => 0.0,
            Rail::Arc { sweep, .. } => sweep.abs(),
            Rail::Parabola { .. } | Rail::Wrap(_) => 1.0,
            Rail::Curve(ref c) => c.pieces() as f64,
        };
        let k = (16.0 + 8.0 * sweep(&a).max(sweep(&b))).ceil() as usize;
        r.rows = (0..=k)
            .map(|i| {
                let t = r.ext.u.lo + r.ext.u.width() * i as f64 / k as f64;
                let (pa, _, _) = a.at(t);
                (t, pa, b.at(t).0 - pa)
            })
            .collect();
        r.ends = r.rail_ends();
        // between two rows a point at s moves (1 - s) of one rail's step and s of the other's,
        // at most half of it to the nearer row (0.75: a step's arc, not its chord)
        let carried =
            (1.0 - r.ext.v.lo).abs().max(r.ext.v.hi.abs()) + r.ext.v.lo.abs().max((1.0 - r.ext.v.hi).abs());
        r.reach = 0.75
            * carried
            * r.rows
                .windows(2)
                .map(|w| {
                    (w[1].1 - w[0].1)
                        .length()
                        .max(((w[1].1 + w[1].2) - (w[0].1 + w[0].2)).length())
                })
                .fold(0.0, f64::max);
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

    /// The two radii of a wall between one wrapped curve at two radii about one axis.
    fn radii(&self) -> Option<(f64, f64)> {
        let (&Rail::Wrap(wa), &Rail::Wrap(wb)) = (&self.a, &self.b) else {
            return None;
        };
        (wa.flat == wb.flat && wa.axis == wb.axis && wa.unroll == wb.unroll).then_some((wa.radius, wb.radius))
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

    fn rail_ends(&self) -> [(f64, Point3, Vec3); 2] {
        [self.ext.u.lo, self.ext.u.hi].map(|t| {
            let (pa, _, _) = self.a.at(t);
            (t, pa, self.b.at(t).0 - pa)
        })
    }

    /// Parameters of the point nearest p on the carried-on surface.
    fn nearest(&self, p: Point3) -> Point2 {
        let dom = self.ext;
        if self.extruded
            && let (&Rail::Parabola { curve, range }, &Rail::Parabola { curve: cb, .. }) = (&self.a, &self.b)
        {
            // square to the rulings onto the parabola's plane, then its nearest point there
            let d = cb.frame.origin - curve.frame.origin;
            let w = range.width();
            let along = Interval::new(range.lo + w * dom.u.lo, range.lo + w * dom.u.hi);
            let flat = p - d * (d.dot(p - curve.frame.origin) / d.length_sq());
            let t = (parabola_nearest(&curve, flat, along) - range.lo) / w;
            let s = (d.dot(p - self.a.at(t).0) / d.length_sq()).clamp(dom.v.lo, dom.v.hi);
            return Point2::new(t, s);
        }
        let start = match self.unwrapped(p, dom) {
            Some((uv, true)) => return uv,
            Some((uv, false)) => self.scan(p, (self.point(uv).p.distance_sq(p), uv)).0,
            None => self.seed(p),
        };
        polish(&|uv| self.point(uv), p, start, dom, None).0
    }

    /// Where on its ruling p is nearest and how near, for one of `rows` or `ends`.
    fn on_ruling(&self, p: Point3, &(t, pa, d): &(f64, Point3, Vec3)) -> (f64, Point2) {
        let w = p - pa;
        let s = (w.dot(d) / d.length_sq()).clamp(self.ext.v.lo, self.ext.v.hi);
        ((w - d * s).length_sq(), Point2::new(t, s))
    }

    /// The nearest of the rulings (the first, in a tie), better than `best` so far, and for
    /// `seed` which it is, its squared distance, the next nearest's and the size of the numbers.
    fn scan(&self, p: Point3, mut best: (f64, Point2)) -> (Point2, usize, f64, f64, f64) {
        let (mut at, mut next, mut size) = (usize::MAX, f64::INFINITY, 0.0f64);
        let rows = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, r)| self.ext.u.contains(r.0));
        let ends = self
            .ends
            .iter()
            .enumerate()
            .map(|(k, r)| (self.rows.len() + k, r));
        for (k, r) in rows.chain(ends) {
            let (e, uv) = self.on_ruling(p, r);
            if e < best.0 {
                next = next.min(best.0);
                (best, at) = ((e, uv), k);
            } else {
                next = next.min(e);
            }
            size = size.max(2.0 * (p - r.1).length_sq() + 8.0 * r.2.length_sq());
        }
        (best.1, at, best.0, next, size)
    }

    /// `scan`'s answer, without looking at every ruling while p stays near where it last looked.
    fn seed(&self, p: Point3) -> Point2 {
        if let Some([k, _]) = KeptSeeds::get(self.id, p) {
            let n = self.rows.len();
            return self
                .on_ruling(p, if k < n { &self.rows[k] } else { &self.ends[k - n] })
                .1;
        }
        let far = (f64::INFINITY, Point2::new(self.ext.u.mid(), self.ext.v.mid()));
        let (uv, k, best, next, size) = self.scan(p, far);
        if k != usize::MAX {
            KeptSeeds::put(self.id, p, [k, 0], &[(best, next)], size);
        }
        uv
    }

    /// A wrapped wall's parameters of a point, read straight off its turn, height and radius
    /// (for a point on it, exact), kept in `dom`; None for any other wall.
    fn unwrapped(&self, p: Point3, dom: Box2) -> Option<(Point2, bool)> {
        let (ra, rb) = self.radii()?;
        let Rail::Wrap(wa) = &self.a else {
            return None;
        };
        let l = wa.axis.point_to_local(p);
        let (sr, _) = wa.flat.bbox(dom.u);
        // the turn nearest the flat curve's
        let turn = forge_math::dmath::TAU * wa.unroll;
        let mut sv = forge_math::dmath::atan2(l.y, l.x) * wa.unroll;
        sv += turn * ((sr.mid() - sv) / turn).round();
        let q = Point2::new(sv, l.z);
        let t = wa.flat.param(q);
        let rho = (l.x * l.x + l.y * l.y).sqrt();
        let s = (rho - ra) / (rb - ra);
        // off the flat curve by this much, as far round as the point is out
        let off = wa.flat.at(t).0.distance(q) * (rho / wa.unroll).max(1.0);
        let on = off <= 1e-12 * (1.0 + rho + l.z.abs()) && dom.u.contains(t) && dom.v.contains(s);
        Some((
            Point2::new(t.clamp(dom.u.lo, dom.u.hi), s.clamp(dom.v.lo, dom.v.hi)),
            on,
        ))
    }

    /// Signed distance to the carried-on surface and its gradient.
    fn value(&self, p: Point3) -> (f64, Vec3) {
        remembered(self.id, p, || {
            let uv = self.nearest(p);
            let d = self.point(uv);
            // at an apex, the normal just off it
            let n = d.du.cross(d.dv).normalize().or_else(|| {
                let q = self.point(Point2::new(uv.x, (uv.y).clamp(1e-3, 1.0 - 1e-3)));
                q.du.cross(q.dv).normalize()
            });
            let n = n.map_or(Vec3::Z, |u| u.get());
            ((p - d.p).dot(n), n)
        })
    }
}

/// The parameter in `range` of the parabola's point nearest q: where (P - q).P' = 0, the
/// depressed cubic 2k^2 t^3 + (1 - 2k x) t - y = 0 solved by Cardano, its roots polished.
fn parabola_nearest(par: &Parabola, q: Point3, range: Interval) -> f64 {
    use forge_math::dmath;
    let l = par.frame.point_to_local(q);
    let k = 1.0 / (4.0 * par.focal);
    // derivation: research/closed-forms/
    let (a, b, c) = (2.0 * k * k, 1.0 - 2.0 * k * l.x, -l.y);
    let (pp, qq) = (b / a, c / a);
    let disc = qq * qq / 4.0 + pp * pp * pp / 27.0;
    let mut roots = [f64::NAN; 3];
    if disc >= 0.0 {
        let r = disc.sqrt();
        roots[0] = dmath::cbrt(-qq / 2.0 + r) + dmath::cbrt(-qq / 2.0 - r);
    } else {
        let m = 2.0 * (-pp / 3.0).sqrt();
        let th = dmath::acos_clamped(3.0 * qq / (pp * m)) / 3.0;
        for (i, r) in roots.iter_mut().enumerate() {
            *r = m * dmath::cos(th - dmath::TAU * i as f64 / 3.0);
        }
    }
    let dist = |t: f64| par.eval(t).distance_sq(q);
    let mut best = (dist(range.lo), range.lo);
    for mut t in roots.into_iter().filter(|t| t.is_finite()).chain([range.hi]) {
        for _ in 0..2 {
            let d = 3.0 * a * t * t + b;
            if d != 0.0 {
                t -= (a * t * t * t + b * t + c) / d;
            }
        }
        let t = t.clamp(range.lo, range.hi);
        let e = dist(t);
        if e < best.0 {
            best = (e, t);
        }
    }
    best.1
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
    fn kind(&self) -> &'static str {
        "ruled"
    }
    fn exact_d1(&self) -> bool {
        true
    }
    fn v_degree(&self) -> Option<usize> {
        Some(1)
    }
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        self.point(uv)
    }
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
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
        self.nearest(p)
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
        // each ruling runs between its points at the box's two s, which lie in these boxes
        let (ba, bb) = (self.a.bbox(uv.u), self.b.bbox(uv.u));
        lerp_box(&ba, &bb, uv.v.lo)
            .hull(&lerp_box(&ba, &bb, uv.v.hi))
            .grow(tol::LINEAR)
    }
    /// A ruling is a line and `other` a quadric: where each of a fan of rulings a leaf apart
    /// meets it is a quadratic. A branch of the curve crosses the rulings it spans; one narrower
    /// than the fan lies between two rulings, so a ruling's point nearest the quadric, where
    /// that is within the fan's spacing, is a seed too.
    fn seeds(&self, other: &Surface, region: Box3, leaf: f64) -> Option<Vec<Point3>> {
        let crate::meet::Alg::Quadric { o, a, b, c } = crate::meet::Alg::of(other)? else {
            return None;
        };
        // the region's own sphere clear of the quadric: nothing to look for
        if other.implicit(region.center()).abs() > 0.5 * region.diagonal() {
            return Some(Vec::new());
        }
        let along = |f: fn(&Ruled, usize) -> Point3| {
            (1..self.rows.len())
                .map(|i| f(self, i).distance(f(self, i - 1)))
                .sum::<f64>()
        };
        let long = along(|r, i| r.rows[i].1).max(along(|r, i| r.rows[i].1 + r.rows[i].2));
        let fan = ((long / leaf).ceil() as usize).clamp(16, 4096);
        let gap = long / fan as f64;
        let mut out = Vec::new();
        let mut roots = Vec::with_capacity(2);
        // rows whose rulings' box misses the region have nothing in the fan between them
        let rows = self.rows.len() - 1;
        let near: Vec<bool> = (0..rows)
            .map(|k| {
                let (r0, r1) = (&self.rows[k], &self.rows[k + 1]);
                let mut b = Box3::from_points([
                    r0.1 + r0.2 * self.ext.v.lo,
                    r0.1 + r0.2 * self.ext.v.hi,
                    r1.1 + r1.2 * self.ext.v.lo,
                    r1.1 + r1.2 * self.ext.v.hi,
                ]);
                b = b.grow(self.reach);
                b.overlaps(&region)
            })
            .collect();
        for i in 0..=fan {
            let k = (i * rows / fan).min(rows - 1);
            if !near[k] {
                continue;
            }
            let u = self.ext.u.lo + self.ext.u.width() * i as f64 / fan as f64;
            let pa = self.a.at(u).0;
            let d = self.b.at(u).0 - pa;
            // the stretch of the ruling inside the region
            let (mut lo, mut hi) = (self.ext.v.lo, self.ext.v.hi);
            for (x, w, s) in [
                (pa.x, d.x, region.x),
                (pa.y, d.y, region.y),
                (pa.z, d.z, region.z),
            ] {
                if w.abs() < 1e-300 {
                    if x < s.lo || x > s.hi {
                        hi = lo - 1.0;
                    }
                    continue;
                }
                let (t0, t1) = ((s.lo - x) / w, (s.hi - x) / w);
                lo = lo.max(t0.min(t1));
                hi = hi.min(t0.max(t1));
            }
            if lo > hi {
                continue;
            }
            let h = pa - o;
            let ad = crate::meet::mul(&a, d);
            let (q2, q1, q0) = (
                d.dot(ad),
                2.0 * (h.dot(ad) + b.dot(d)),
                h.dot(crate::meet::mul(&a, h)) + 2.0 * b.dot(h) + c,
            );
            roots.clear();
            roots.extend(
                forge_math::poly::quadratic(q2, q1, q0)
                    .into_iter()
                    .map(|r| r.x)
                    .filter(|&v| v >= lo && v <= hi),
            );
            if roots.is_empty() {
                let v = if q2 != 0.0 { -q1 / (2.0 * q2) } else { lo };
                let near = [v.clamp(lo, hi), lo, hi];
                roots.extend(
                    near.into_iter()
                        .filter(|&v| other.implicit(pa + d * v).abs() <= gap),
                );
            }
            let apart = match roots[..] {
                [x, y] => (y - x) * d.length(),
                _ => f64::INFINITY,
            };
            out.extend(roots.iter().map(|&v| (apart, pa + d * v)));
        }
        // where two roots are far apart first: the curve crosses the ruling squarely there,
        // and a chain started at a narrow tip can run past its own start
        out.sort_by(|a, b| b.0.total_cmp(&a.0));
        Some(out.into_iter().map(|s| s.1).collect())
    }
    /// The nearest ruling among `rows` puts the surface's distance within the rows' spacing; only
    /// where r falls inside that does it take the search.
    fn within(&self, p: Point3, r: f64) -> bool {
        if !self.extruded {
            let mut near = f64::INFINITY;
            for &(_, pa, d) in &self.rows {
                let w = p - pa;
                let s = (w.dot(d) / d.length_sq()).clamp(self.ext.v.lo, self.ext.v.hi);
                near = near.min((w - d * s).length_sq());
            }
            let hi = near.sqrt();
            if hi - self.reach > r {
                return false;
            }
            if hi <= r {
                return true;
            }
        }
        self.implicit(p).abs() <= r
    }
    fn implicit(&self, p: Point3) -> f64 {
        self.value(p).0
    }
    fn gradient(&self, p: Point3) -> Vec3 {
        self.value(p).1
    }
    fn has_foot_near(&self) -> bool {
        !self.extruded && self.radii().is_none()
    }
    fn foot_near(&self, p: Point3, uv: Point2) -> Option<Point2> {
        self.has_foot_near()
            .then(|| polish(&|uv| self.point(uv), p, uv, self.ext, None).0)
    }
    fn gradient_turn(&self, p: Point3, v: Vec3) -> Option<Vec3> {
        let uv = self.nearest(p);
        let dom = self.ext;
        let free = uv.x > dom.u.lo && uv.x < dom.u.hi && uv.y > dom.v.lo && uv.y < dom.v.hi;
        free.then(|| normal_turn(&self.point(uv), p, v)).flatten()
    }
    fn transformed(&self, f: &Frame) -> Proc {
        let mut r = self.clone();
        r.id = fresh_id();
        r.a = self.a.transformed(f);
        r.b = self.b.transformed(f);
        r.rows = self
            .rows
            .iter()
            .map(|&(t, p, d)| (t, f.point_to_world(p), f.vec_to_world(d)))
            .collect();
        r.ends = r.rail_ends();
        Proc(Arc::new(r))
    }
    fn uv_scale(&self) -> (f64, f64) {
        (self.scale.0.max(tol::LINEAR), self.scale.1.max(tol::LINEAR))
    }
    fn shape_bounds(&self, uv: Box2) -> Option<ShapeBounds> {
        // S = (1 - v) A(u) + v B(u): S_uu = (1 - v) A'' + v B'', S_uv = B' - A', S_vv = 0, and of
        // the third derivatives only S_uuu = (1 - v) A''' + v B''' and S_uuv = B'' - A''; B' - A'
        // and B'' - A'' stray from the middle's by at most their next derivatives' bounds (they
        // vanish between matched rails)
        let [a1, a2, a3] = self.a.derivative_bounds(uv.u)?;
        let [b1, b2, b3] = self.b.derivative_bounds(uv.u)?;
        let (wa, wb) = (
            (1.0 - uv.v.lo).abs().max((1.0 - uv.v.hi).abs()),
            uv.v.lo.abs().max(uv.v.hi.abs()),
        );
        let (u, hu) = (uv.u.mid(), 0.5 * uv.u.width());
        let ((_, da, dda), (_, db, ddb)) = (self.a.at(u), self.b.at(u));
        let twist = (a1 + b1).min((db - da).length() + (a2 + b2) * hu);
        let bend = (ddb - dda).length() + (a3 + b3) * hu;
        Some(ShapeBounds::taylor(
            &self.point(Point2::new(u, uv.v.mid())),
            (hu, 0.5 * uv.v.width()),
            [wa * a2 + wb * b2, twist, 0.0],
            [wa * a3 + wb * b3, bend, 0.0, 0.0],
        ))
    }
    fn mesh_radius(&self) -> f64 {
        let arc = |r: &Rail| match *r {
            Rail::Line { .. } => f64::INFINITY,
            Rail::Arc { circle, .. } => circle.radius,
            // curvature |C' x C''| / |C'|^3 at samples, the tightest halved
            Rail::Parabola { .. } | Rail::Wrap(_) | Rail::Curve(_) => {
                let k = (0..=16)
                    .map(|i| {
                        let (_, d1, d2) = r.at(i as f64 / 16.0);
                        {
                            let l = d1.length();
                            d1.cross(d2).length() / (l * l * l).max(1e-300)
                        }
                    })
                    .fold(0.0, f64::max);
                0.5 / k.max(1e-300)
            }
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
    fn a_seed_kept_from_a_nearby_point_is_the_ruling_a_scan_picks() {
        let mut r = Rng::new(733);
        let (mut steps, mut crossed) = (0, 0);
        for _ in 0..60 {
            let s = random_ruled(&mut r);
            let far = (f64::INFINITY, Point2::new(s.ext.u.mid(), s.ext.v.mid()));
            let d = s.point(rand_uv(&mut r));
            let mut p = d.p + d.du.cross(d.dv).normalize().unwrap() * r.range(-1.0, 1.0);
            let mut last = usize::MAX;
            // small steps stay on one ruling, then cross to the next; the odd large one jumps
            for k in 0..400 {
                let h = if k % 50 == 49 { 2.0 } else { 0.02 };
                p += Vec3::new(r.range(-h, h), r.range(-h, h), r.range(-h, h));
                let (want, at, ..) = s.scan(p, far);
                assert_eq!(s.seed(p), want, "{s:?} {p:?}");
                steps += 1;
                crossed += usize::from(at != last);
                last = at;
            }
        }
        // the walks did cross from ruling to ruling
        assert!(crossed > 200 && crossed < steps / 2, "{crossed} of {steps}");
    }

    #[test]
    fn the_normals_turn_is_the_difference_of_nearby_normals() {
        let mut r = Rng::new(811);
        for _ in 0..200 {
            let s = random_ruled(&mut r);
            let d = s.point(rand_uv(&mut r));
            let Some(n) = d.du.cross(d.dv).normalize() else {
                continue;
            };
            let p = d.p + n * r.range(-0.2, 0.2);
            let v = Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0));
            let Some(t) = s.gradient_turn(p, v) else {
                continue;
            };
            // wide enough that the searches' stopping tolerance does not show
            let h = 1e-3;
            let fd = (s.value(p + v * h).1 - s.value(p - v * h).1) / (2.0 * h);
            assert!((t - fd).length() < 1e-4 * (1.0 + fd.length()), "{t:?} {fd:?}");
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
                let near = Point2::new(uv.x + r.range(-0.05, 0.05), uv.y + r.range(-0.05, 0.05));
                assert!(
                    s.point(s.project_near(d.p, near)).p.distance(d.p) < 1e-10,
                    "{s:?} {uv:?}"
                );
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
    fn a_cell_holding_the_surface_is_never_ruled_out() {
        let mut r = Rng::new(524);
        for _ in 0..200 {
            let s = random_ruled(&mut r);
            for _ in 0..40 {
                let (u, v) = (r.range(s.ext.u.lo, s.ext.u.hi), r.range(s.ext.v.lo, s.ext.v.hi));
                let q = s.point(Point2::new(u, v)).p;
                let cell = r.range(0.001, 3.0);
                let off = Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0));
                let p = q + off.normalize().map_or(Vec3::ZERO, |n| n.get()) * (cell * r.range(0.0, 0.999));
                assert!(s.within(p, cell), "{s:?} {q:?} {p:?} {cell}");
            }
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
