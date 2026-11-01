//! A curve carried along a spine, square to it: S(u, v) = P(v) + R(v) q(u), q(u) the section in the
//! start frame, R(v) a turn about a fixed axis. For a planar spine the axis is the plane's normal and
//! R(v) takes the start tangent to the tangent at v, with no trigonometry (cos = T0.T, sin =
//! (T0 x T).B), so nothing twists and the formula is exact. For a helix R(v) turns by v about the coil's
//! axis and P(v) = a v e_r + b v z. For a spatial spine R(v) is the rotation-minimizing frame (no twist
//! about the tangent): R' = [w] R with w = T x T', tabulated by Magnus steps at build and stepped from
//! the nearest entry at each v. Derivation: research/spline-loops/derive.py (7).

use super::Spline;
use super::extruded::{carried, foot as foot2};
use crate::curve::{CurveDeriv, Domain1, ProcCurve, ProceduralCurve};
use crate::ruled::principal_max;
use crate::screw::{fresh_id, polish, remembered};
use crate::surface::{Domain2, Proc, Procedural, SurfDeriv, Surface};
use forge_math::{Box2, Box3, Frame, Interval, Point2, Point3, UnitVec3, Vec3, dmath, tol};
use std::sync::Arc;

/// The motion a section is carried by.
#[derive(Clone, Debug)]
pub enum Spine {
    /// A curve in a plane of normal `b`, its parameter carried past its ends in `ext`.
    Planar {
        c: Arc<Spline>,
        b: UnitVec3,
        t0: Vec3,
        ext: Interval,
    },
    /// A curve in space, with R from its table: at each knot v, R(v) as a unit quaternion.
    Spatial {
        c: Arc<Spline>,
        ext: Interval,
        table: Arc<Vec<(f64, Quat)>>,
    },
    /// About `axis`'s z: the section's point (r, h) in the axis' xz at angle v lies at
    /// (r + a v) e_r(v) + (h + b v) z. v over `range`.
    Helix {
        axis: Frame,
        a: f64,
        b: f64,
        range: Interval,
    },
}

/// A rotation, as a unit quaternion (w, x, y, z).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quat(f64, Vec3);

impl Quat {
    const ONE: Quat = Quat(1.0, Vec3::ZERO);
    /// The rotation by the vector's length about its direction.
    fn of(v: Vec3) -> Quat {
        let a = v.length();
        if a <= 1e-300 {
            return Quat::ONE;
        }
        let (s, c) = dmath::sin_cos(0.5 * a);
        Quat(c, v * (s / a))
    }
    fn then(self, o: Quat) -> Quat {
        // o after self
        Quat(
            o.0 * self.0 - o.1.dot(self.1),
            self.1 * o.0 + o.1 * self.0 + o.1.cross(self.1),
        )
    }
    fn apply(self, v: Vec3) -> Vec3 {
        let t = self.1.cross(v) * 2.0;
        v + t * self.0 + self.1.cross(t)
    }
    fn undo(self, v: Vec3) -> Vec3 {
        Quat(self.0, -self.1).apply(v)
    }
}

/// The rotation rate of a curve's rotation-minimizing frame, and its derivative.
fn rates_at(c: &Spline, v: f64) -> (Vec3, Vec3) {
    let (d, d3) = c.deriv3(v);
    let l2 = d.d1.length_sq();
    let w = d.d1.cross(d.d2) / l2;
    (w, d.d1.cross(d3) / l2 - w * (2.0 * d.d1.dot(d.d2) / l2))
}

/// One step of the fourth-order Magnus rule from v over h: R(v + h) = this R(v), with no drift off a
/// rotation however many are taken.
fn magnus(c: &Spline, v: f64, h: f64) -> Quat {
    const G: f64 = 0.288_675_134_594_812_9; // sqrt(3) / 6
    let (w1, _) = rates_at(c, v + (0.5 - G) * h);
    let (w2, _) = rates_at(c, v + (0.5 + G) * h);
    Quat::of((w1 + w2) * (0.5 * h) - w1.cross(w2) * (h * h * G * 0.5))
}

/// The frame table of a spatial curve over `ext`, from its start: each step halved until two half steps
/// agree with the whole to a part in 1e13.
fn frame_table(c: &Spline, ext: Interval) -> Vec<(f64, Quat)> {
    let dom = c.domain();
    let walk = |from: f64, to: f64, first: Quat| -> Vec<(f64, Quat)> {
        let mut out = vec![(from, first)];
        let (mut v, mut q) = (from, first);
        // joints first: the rate is only continuous across them
        let mut stops: Vec<f64> = c.knots_in(Interval::new(from.min(to), from.max(to)));
        if to < from {
            stops.reverse();
        }
        stops.push(to);
        for stop in stops {
            while (stop - v).abs() > 1e-13 * (1.0 + stop.abs()) {
                let mut h = stop - v;
                loop {
                    let whole = magnus(c, v, h);
                    let half = magnus(c, v, 0.5 * h).then(magnus(c, v + 0.5 * h, 0.5 * h));
                    let e = (whole.1 - half.1).length() + (whole.0 - half.0).abs();
                    if e <= 1e-13 || h.abs() <= 1e-9 * (1.0 + stop.abs()) {
                        q = q.then(half);
                        v += h;
                        break;
                    }
                    h *= 0.5;
                }
                out.push((v, q));
            }
        }
        out
    };
    let back = walk(dom.lo, ext.lo, Quat::ONE);
    let mut all: Vec<(f64, Quat)> = back.into_iter().rev().collect();
    all.pop();
    all.extend(walk(dom.lo, ext.hi, Quat::ONE));
    all
}

/// How the section is turned at v.
#[derive(Clone, Copy)]
enum Turn {
    /// About a fixed axis, by cosine and sine.
    Axis(Vec3, f64, f64),
    Quat(Quat),
}

/// The spine at v: P and its derivatives, the turn, its rate and the rate's derivative as vectors.
struct At {
    p: Point3,
    d1: Vec3,
    d2: Vec3,
    turn: Turn,
    w: Vec3,
    dw: Vec3,
}

impl At {
    /// R q.
    fn turn(&self, q: Vec3) -> Vec3 {
        match self.turn {
            Turn::Axis(axis, cos, sin) => {
                let along = axis * axis.dot(q);
                along + (q - along) * cos + axis.cross(q) * sin
            }
            Turn::Quat(r) => r.apply(q),
        }
    }
    /// q from R q.
    fn back(&self, v: Vec3) -> Vec3 {
        match self.turn {
            Turn::Axis(axis, cos, sin) => {
                let along = axis * axis.dot(v);
                along + (v - along) * cos - axis.cross(v) * sin
            }
            Turn::Quat(r) => r.undo(v),
        }
    }
}

impl Spine {
    pub fn planar(c: Arc<Spline>, b: UnitVec3) -> Option<Spine> {
        let ext = carried(&c)?;
        let t0 = c.deriv(c.domain().lo).d1.normalize()?.get();
        Some(Spine::Planar { c, b, t0, ext })
    }

    /// The rotation-minimizing spine of a curve not in one plane.
    pub fn spatial(c: Arc<Spline>) -> Option<Spine> {
        let ext = carried(&c)?;
        let table = Arc::new(frame_table(&c, ext));
        Some(Spine::Spatial { c, ext, table })
    }

    /// A box round the spine.
    pub fn bbox(&self) -> Box3 {
        match self {
            Spine::Planar { c, .. } | Spine::Spatial { c, .. } => c.bbox(c.domain()),
            Spine::Helix { axis, a, b, range } => {
                let t = range.lo.abs().max(range.hi.abs());
                let r = (a * t).abs();
                let h = (b * t).abs();
                Box3 {
                    x: Interval::new(axis.origin.x - r - h, axis.origin.x + r + h),
                    y: Interval::new(axis.origin.y - r - h, axis.origin.y + r + h),
                    z: Interval::new(axis.origin.z - r - h, axis.origin.z + r + h),
                }
            }
        }
    }

    /// R at the spine's end against its start: where a frame at the start is carried.
    pub fn end_turn(&self, v: Vec3) -> Vec3 {
        self.at(self.domain().hi).turn(v)
    }

    pub fn domain(&self) -> Interval {
        match self {
            Spine::Planar { c, .. } | Spine::Spatial { c, .. } => c.domain(),
            Spine::Helix { range, .. } => *range,
        }
    }

    fn reach(&self) -> Interval {
        match self {
            Spine::Planar { ext, .. } | Spine::Spatial { ext, .. } => *ext,
            Spine::Helix { range, .. } => {
                let e = 0.25 * range.width().min(dmath::PI);
                Interval::new(range.lo - e, range.hi + e)
            }
        }
    }

    fn at(&self, v: f64) -> At {
        match self {
            Spine::Planar { c, b, t0, .. } => {
                let (d, d3) = c.deriv3(v);
                let bv = b.get();
                let l2 = d.d1.length_sq();
                let l = l2.sqrt();
                let t = d.d1 / l;
                // the tangent's angle rate, and its own: w = (C' x C'').b / |C'|^2
                let n = d.d1.cross(d.d2).dot(bv);
                let w = n / l2;
                let dn = d.d1.cross(d3).dot(bv);
                let dw = (dn - 2.0 * w * d.d1.dot(d.d2)) / l2;
                At {
                    p: d.p,
                    d1: d.d1,
                    d2: d.d2,
                    turn: Turn::Axis(bv, t0.dot(t), t0.cross(t).dot(bv)),
                    w: bv * w,
                    dw: bv * dw,
                }
            }
            Spine::Spatial { c, table, .. } => {
                let d = c.deriv(v);
                let (w, dw) = rates_at(c, v);
                // from the table's entry nearest below, one step on
                let k = table.partition_point(|e| e.0 <= v).max(1) - 1;
                let (v0, q0) = table[k];
                At {
                    p: d.p,
                    d1: d.d1,
                    d2: d.d2,
                    turn: Turn::Quat(if v == v0 {
                        q0
                    } else {
                        q0.then(magnus(c, v0, v - v0))
                    }),
                    w,
                    dw,
                }
            }
            Spine::Helix { axis, a, b, .. } => {
                let (s, c) = dmath::sin_cos(v);
                let (ex, ey, ez) = (axis.x().get(), axis.y().get(), axis.z().get());
                let radial = ex * c + ey * s;
                let round = ey * c - ex * s;
                At {
                    p: axis.origin + radial * (*a * v) + ez * (*b * v),
                    d1: radial * *a + round * (*a * v) + ez * *b,
                    d2: round * (2.0 * *a) - radial * (*a * v),
                    turn: Turn::Axis(ez, c, s),
                    w: ez,
                    dw: Vec3::ZERO,
                }
            }
        }
    }

    /// Parameters of the spine worth starting from for a point near the wall.
    fn starts(&self, p: Point3, out: &mut Vec<f64>) {
        match self {
            Spine::Planar { c, b, ext, .. } => {
                let bv = b.get();
                let o = c.point(c.domain().lo);
                let q = p - bv * bv.dot(p - o);
                out.push(c.closest(q, *ext));
            }
            Spine::Spatial { c, ext, .. } => out.push(c.closest(p, *ext)),
            Spine::Helix { axis, range, .. } => {
                let l = axis.point_to_local(p);
                let th = dmath::atan2(l.y, l.x);
                let t = dmath::TAU;
                let r = self.reach();
                let k0 = ((r.lo - th) / t).floor() as i64;
                let k1 = ((r.hi - th) / t).ceil() as i64;
                for k in k0..=k1 {
                    out.push((th + k as f64 * t).clamp(r.lo, r.hi));
                }
                out.push(range.mid());
            }
        }
    }

    /// Bounds on |P'| and on the turn's rate over `v`.
    fn rates(&self, v: Interval) -> (f64, f64) {
        match self {
            Spine::Planar { c, .. } | Spine::Spatial { c, .. } => {
                let [m1, m2, _] = c.derivative_bounds(v);
                let mut low = f64::INFINITY;
                for i in 0..=8 {
                    low = low.min(c.deriv(v.lo + v.width() * i as f64 / 8.0).d1.length());
                }
                (m1, m2 / (0.5 * low).max(1e-12))
            }
            Spine::Helix { a, b, .. } => {
                let t = v.lo.abs().max(v.hi.abs());
                ((a * a * (1.0 + t * t) + b * b).sqrt(), 1.0)
            }
        }
    }

    /// The same spine moved rigidly.
    fn transformed(&self, f: &Frame) -> Spine {
        match self {
            Spine::Planar { c, b, t0, ext } => Spine::Planar {
                c: Arc::new(c.mapped(|p| f.point_to_world(p))),
                b: f.unit_to_world(*b),
                t0: f.vec_to_world(*t0),
                ext: *ext,
            },
            Spine::Spatial { c, .. } => Spine::spatial(Arc::new(c.mapped(|p| f.point_to_world(p))))
                .expect("a rigid motion keeps it regular"),
            Spine::Helix { axis, a, b, range } => Spine::Helix {
                axis: f.compose(axis),
                a: *a,
                b: *b,
                range: *range,
            },
        }
    }
}

/// A section carried along a spine. The section lies in `frame` at the spine's start: for a planar
/// spine its xy plane is square to the start tangent; for a helix its xz plane holds the axis.
#[derive(Clone, Debug)]
pub struct Swept {
    spine: Spine,
    c: Arc<Spline>,
    /// The section's x and y in the start frame, world directions.
    x: Vec3,
    y: Vec3,
    /// Where section coordinates are measured from, relative to the spine's start point P(lo).
    off: Vec3,
    ext: Interval,
    id: u64,
    radius: f64,
    scale: (f64, f64),
}

impl Swept {
    /// `plane`: the section's frame at the spine's start, the section in its xy. None where the
    /// section has a cusp.
    pub fn new(spine: Spine, c: Arc<Spline>, plane: &Frame) -> Option<Swept> {
        let ext = carried(&c)?;
        let start = spine.at(spine.domain().lo);
        // the section's frame without the start's turn: R(lo) (x0, y0) = plane's axes
        let (x, y) = (start.back(plane.x().get()), start.back(plane.y().get()));
        let off = start.back(plane.origin - start.p);
        let mut s = Swept {
            spine,
            c,
            x,
            y,
            off,
            ext,
            id: fresh_id(),
            radius: f64::INFINITY,
            scale: (1.0, 1.0),
        };
        let dom = s.domain();
        let (mut su, mut sv, mut k) = (0.0f64, 0.0f64, 0.0f64);
        for i in 0..=8 {
            for j in 0..=8 {
                let d = s.at(Point2::new(
                    dom.u.lo + (dom.u.hi - dom.u.lo) * i as f64 / 8.0,
                    dom.v.lo + (dom.v.hi - dom.v.lo) * j as f64 / 8.0,
                ));
                su = su.max(d.du.length());
                sv = sv.max(d.dv.length());
                k = k.max(principal_max(&d));
            }
        }
        s.scale = (su, sv);
        s.radius = if k > 0.0 { 0.5 / k } else { f64::INFINITY };
        Some(s)
    }

    pub fn surface(self) -> Surface {
        Surface::Procedural(Proc(Arc::new(self)))
    }

    /// The path a point of the section (`q` in its frame) follows, as a curve over the spine's
    /// parameter.
    pub fn rail(&self, q: Point2) -> crate::Curve {
        crate::Curve::Procedural(ProcCurve(Arc::new(Rail {
            spine: self.spine.clone(),
            q: self.q_at(q),
        })))
    }

    fn q_at(&self, q: Point2) -> Vec3 {
        self.off + self.x * q.x + self.y * q.y
    }

    fn at(&self, uv: Point2) -> SurfDeriv {
        let c = self.c.deriv(uv.x);
        let m = self.spine.at(uv.y);
        let q = self.off + self.x * c.p.x + self.y * c.p.y;
        let (q1, q2) = (
            self.x * c.d1.x + self.y * c.d1.y,
            self.x * c.d2.x + self.y * c.d2.y,
        );
        let (rq, r1, r2) = (m.turn(q), m.turn(q1), m.turn(q2));
        let wrq = m.w.cross(rq);
        SurfDeriv {
            p: m.p + rq,
            du: r1,
            dv: m.d1 + wrq,
            duu: r2,
            duv: m.w.cross(r1),
            dvv: m.d2 + m.dw.cross(rq) + m.w.cross(wrq),
        }
    }

    fn near(&self, p: Point3, seed: Option<Point2>) -> Point2 {
        let reach = Box2::new(self.ext, self.spine.reach());
        let s = |uv: Point2| self.at(uv);
        if let Some(g) = seed {
            let (uv, d) = polish(&s, p, g, reach, None);
            if d.p.distance(p) <= tol::LINEAR {
                return uv;
            }
        }
        let mut starts = Vec::with_capacity(4);
        self.spine.starts(p, &mut starts);
        let mut best = (
            f64::INFINITY,
            Point2::new(self.ext.mid(), self.spine.domain().mid()),
        );
        for v in starts {
            // the point in the section's plane at v: its foot on the section starts the search
            let m = self.spine.at(v);
            let local = m.back(p - m.p) - self.off;
            let q = Point3::new(local.dot(self.x), local.dot(self.y), 0.0);
            let (u, ..) = foot2(&self.c, self.ext, q, None);
            let (uv, d) = polish(&s, p, Point2::new(u, v), reach, None);
            let e = d.p.distance_sq(p);
            if e < best.0 {
                best = (e, uv);
            }
        }
        best.1
    }

    fn value(&self, p: Point3) -> (f64, Vec3) {
        remembered(self.id, p, || self.value_from(p, None).0)
    }

    fn value_from(&self, p: Point3, seed: Option<Point2>) -> ((f64, Vec3), Point2) {
        let uv = self.near(p, seed);
        let d = self.at(uv);
        let n = d.du.cross(d.dv).normalize().map_or(Vec3::Z, |u| u.get());
        (((p - d.p).dot(n), n), uv)
    }
}

impl Procedural for Swept {
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        self.at(uv)
    }
    fn exact_d1(&self) -> bool {
        true
    }
    fn regular(&self) -> bool {
        true
    }
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
    fn domain(&self) -> Domain2 {
        let (a, b) = (self.c.domain(), self.spine.domain());
        Domain2 {
            u: Domain1 {
                lo: a.lo,
                hi: a.hi,
                period: None,
            },
            v: Domain1 {
                lo: b.lo,
                hi: b.hi,
                period: None,
            },
        }
    }
    fn project(&self, p: Point3) -> Point2 {
        self.near(p, None)
    }
    fn project_near(&self, p: Point3, near: Point2) -> Point2 {
        self.near(p, Some(near))
    }
    fn value_near(&self, p: Point3, uv: Option<Point2>) -> Option<(f64, Vec3, Point2)> {
        let ((f, g), q) = self.value_from(p, uv);
        Some((f, g, q))
    }
    fn bbox(&self, uv: Box2) -> Box3 {
        // the middle's point, with how far each parameter can move it
        let (hu, hv) = (0.5 * uv.u.width(), 0.5 * uv.v.width());
        let d = self.at(uv.center());
        let [m1, ..] = self.c.derivative_bounds(uv.u);
        let (pv, w) = self.spine.rates(uv.v);
        let b = self.c.bbox(uv.u);
        let far = [b.x.lo, b.x.hi]
            .iter()
            .flat_map(|&x| [b.y.lo, b.y.hi].map(|y| self.q_at(Point2::new(x, y)).length()))
            .fold(0.0, f64::max);
        let r = m1 * self.x.length().max(self.y.length()) * hu + (pv + w * far) * hv + tol::LINEAR;
        Box3 {
            x: Interval::new(d.p.x - r, d.p.x + r),
            y: Interval::new(d.p.y - r, d.p.y + r),
            z: Interval::new(d.p.z - r, d.p.z + r),
        }
    }
    fn implicit(&self, p: Point3) -> f64 {
        self.value(p).0
    }
    fn gradient(&self, p: Point3) -> Vec3 {
        self.value(p).1
    }
    fn implicit_gradient(&self, p: Point3) -> (f64, Vec3) {
        self.value(p)
    }
    fn transformed(&self, f: &Frame) -> Proc {
        let mut s = self.clone();
        s.id = fresh_id();
        s.spine = self.spine.transformed(f);
        s.x = f.vec_to_world(self.x);
        s.y = f.vec_to_world(self.y);
        s.off = f.vec_to_world(self.off);
        Proc(Arc::new(s))
    }
    fn uv_scale(&self) -> (f64, f64) {
        (self.scale.0.max(tol::LINEAR), self.scale.1.max(tol::LINEAR))
    }
    fn mesh_radius(&self) -> f64 {
        self.radius
    }
    fn u_kinks(&self, u: Interval) -> Vec<f64> {
        self.c.knots_in(u)
    }
}

/// The path of one section point along a spine.
#[derive(Clone, Debug)]
struct Rail {
    spine: Spine,
    q: Vec3,
}

impl ProceduralCurve for Rail {
    fn deriv(&self, t: f64) -> CurveDeriv {
        let m = self.spine.at(t);
        let rq = m.turn(self.q);
        let wrq = m.w.cross(rq);
        CurveDeriv {
            p: m.p + rq,
            d1: m.d1 + wrq,
            d2: m.d2 + m.dw.cross(rq) + m.w.cross(wrq),
        }
    }
    fn domain(&self) -> Domain1 {
        let d = self.spine.domain();
        Domain1 {
            lo: d.lo,
            hi: d.hi,
            period: None,
        }
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        let mut best = (f64::INFINITY, range.lo);
        let n = 8 * (1 + (self.segments(range, 1e-2)).min(64));
        for i in 0..=n {
            let t = range.lo + range.width() * i as f64 / n as f64;
            let e = self.point(t).distance_sq(p);
            if e < best.0 {
                best = (e, t);
            }
        }
        let mut t = best.1;
        for _ in 0..16 {
            let d = self.deriv(t);
            let e = d.p - p;
            let h = d.d1.length_sq() + e.dot(d.d2);
            if h <= 0.0 {
                break;
            }
            let next = (t - e.dot(d.d1) / h).clamp(range.lo, range.hi);
            if (next - t).abs() * d.d1.length() <= 1e-13 {
                t = next;
                break;
            }
            t = next;
        }
        if self.point(t).distance_sq(p) < best.0 {
            t
        } else {
            best.1
        }
    }
    fn bbox(&self, range: Interval) -> Box3 {
        let (pv, w) = self.spine.rates(range);
        let r = (pv + w * self.q.length()) * 0.5 * range.width() + tol::LINEAR;
        let c = self.point(range.mid());
        Box3 {
            x: Interval::new(c.x - r, c.x + r),
            y: Interval::new(c.y - r, c.y + r),
            z: Interval::new(c.z - r, c.z + r),
        }
    }
    fn transformed(&self, f: &Frame) -> ProcCurve {
        ProcCurve(Arc::new(Rail {
            spine: self.spine.transformed(f),
            q: f.vec_to_world(self.q),
        }))
    }
    fn speed_bound(&self, range: Interval) -> f64 {
        let (pv, w) = self.spine.rates(range);
        pv + w * self.q.length()
    }
    fn segments(&self, range: Interval, chord: f64) -> usize {
        let m = (0..=16)
            .map(|i| self.deriv(range.lo + range.width() * i as f64 / 16.0).d2.length())
            .fold(0.0, f64::max);
        (((1.5 * m * range.width() * range.width() / (8.0 * chord.max(1e-12))).sqrt()).ceil() as usize).max(1)
    }
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
    fn exact_d1(&self) -> bool {
        true
    }
    fn kinks(&self, range: Interval) -> Vec<f64> {
        match &self.spine {
            Spine::Planar { c, .. } | Spine::Spatial { c, .. } => c.knots_in(range),
            Spine::Helix { .. } => Vec::new(),
        }
    }
}

/// Whether a curve lies in one plane, and its unit normal (None: not in one, or straight).
pub fn plane_of(c: &Spline) -> Option<UnitVec3> {
    let (_, _, pts) = c.bezier_data();
    let p: Vec<Point3> = pts.iter().map(|h| Point3::new(h[0], h[1], h[2])).collect();
    let o = p[0];
    let far = p
        .iter()
        .copied()
        .fold(o, |m, q| if q.distance(o) > m.distance(o) { q } else { m });
    let u = (far - o).normalize()?;
    let mut n: Option<UnitVec3> = None;
    let mut best = 1e-9;
    for q in &p {
        let c = u.get().cross(*q - o);
        if c.length() > best {
            best = c.length();
            n = c.normalize();
        }
    }
    let n = n?;
    p.iter().all(|q| n.dot(*q - o).abs() <= tol::LINEAR).then_some(n)
}
