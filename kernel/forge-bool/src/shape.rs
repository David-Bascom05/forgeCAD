//! Tools as analytic solids: whether a point is inside one answered from the profile it was
//! made from (a prism, a solid of revolution, a ball), with no B-rep in the loop.
//!
//! The answer is In or Out only when the point is clear of the boundary by a margin; nearer,
//! it is Near and the caller asks the B-rep.

use crate::Op;
use forge_math::{Box3, Frame, Point2, Point3, Vec2, Vec3, dmath};

/// A piece of a closed 2D profile.
#[derive(Clone, Debug, PartialEq)]
pub enum Seg2 {
    /// A free curve in the xy plane.
    Curve(std::sync::Arc<forge_geom::spline::Spline>),
    Line {
        a: Point2,
        b: Point2,
    },
    /// From angle `start` through `sweep` (negative: clockwise); a whole circle has |sweep| 2pi.
    Arc {
        c: Point2,
        r: f64,
        start: f64,
        sweep: f64,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    /// The profile (in `frame`'s xy) swept along its z from `lo` to `hi`.
    Prism {
        frame: Frame,
        segs: std::sync::Arc<[Seg2]>,
        lo: f64,
        hi: f64,
    },
    /// The profile (in its own plane's coordinates) turned about `frame`'s z, from its x through
    /// `angle` (None: a whole turn). A point at height h and radius r lies over the profile point
    /// `origin + dir * h + perp * r`.
    Revolved {
        frame: Frame,
        origin: Point2,
        dir: Vec2,
        perp: Vec2,
        segs: std::sync::Arc<[Seg2]>,
        angle: Option<f64>,
    },
    Ball {
        center: Point3,
        radius: f64,
    },
    /// `first`, then each later solid combined with it in turn (a tool cut off at a surface:
    /// its sweep, kept on the near side of a plane, less the solid beyond the surface). Parts
    /// are plain solids, never combinations.
    Csg {
        first: Box<Shape>,
        rest: Vec<(Op, Shape)>,
    },
    /// A profile wound about `frame`'s z: its point (r, h) at angle u in [0, `angle`] lies at
    /// ((r + a u) cos u, (r + a u) sin u, h + b u). `r` and `h` are the profile's extents.
    Coil {
        frame: Frame,
        segs: std::sync::Arc<[Seg2]>,
        a: f64,
        b: f64,
        angle: f64,
        r: (f64, f64),
        h: (f64, f64),
    },
    /// A solid with no analytic test here (a loft) inside this box: its B-rep decides every
    /// point in it.
    Brep(Box3),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Member {
    In,
    Out,
    Near,
}

/// Whether a point is inside a closed profile: by the parity of a ray along +x, with the
/// distance to the boundary; None where the ray grazes a vertex or an arc's top or bottom.
fn in_profile(segs: &[Seg2], q: Point2) -> Option<(bool, f64)> {
    let mut inside = false;
    let mut dist = f64::INFINITY;
    let graze = 1e-9;
    for s in segs {
        match *s {
            Seg2::Curve(ref c) => {
                let at = Point3::new(q.x, q.y, 0.0);
                let d = c.domain();
                dist = dist.min(c.point(c.closest(at, d)).distance(at));
                let (e0, e1) = c.ends();
                if (e0.y - q.y).abs() <= graze || (e1.y - q.y).abs() <= graze {
                    return None;
                }
                for (t, touch) in c.plane_roots(Vec3::Y, -q.y, d, graze) {
                    if touch {
                        return None;
                    }
                    if c.point(t).x > q.x {
                        inside = !inside;
                    }
                }
            }
            Seg2::Line { a, b } => {
                dist = dist.min(segment_distance(q, a, b));
                if (a.y > q.y) != (b.y > q.y) && q.x < a.x + (q.y - a.y) / (b.y - a.y) * (b.x - a.x) {
                    inside = !inside;
                }
            }
            Seg2::Arc { c, r, start, sweep } => {
                let d = q - c;
                let len = d.length();
                let at = dmath::atan2(d.y, d.x);
                dist = dist.min(if on_sweep(at, start, sweep) {
                    (len - r).abs()
                } else {
                    let p0 = c + Vec2::new(dmath::cos(start), dmath::sin(start)) * r;
                    let p1 = c + Vec2::new(dmath::cos(start + sweep), dmath::sin(start + sweep)) * r;
                    q.distance(p0).min(q.distance(p1))
                });
                let dy = q.y - c.y;
                if dy.abs() >= r - graze {
                    if dy.abs() <= r + graze {
                        return None;
                    }
                    continue;
                }
                if sweep.abs() < dmath::TAU - 1e-12 {
                    for e in [start, start + sweep] {
                        if (c.y + r * dmath::sin(e) - q.y).abs() <= graze {
                            return None;
                        }
                    }
                }
                let dx = (r * r - dy * dy).sqrt();
                for x in [c.x - dx, c.x + dx] {
                    if x > q.x && on_sweep(dmath::atan2(dy, x - c.x), start, sweep) {
                        inside = !inside;
                    }
                }
            }
        }
    }
    Some((inside, dist))
}

/// Where `p` is against a coil (see `Shape::Coil`). In each turn's half plane through p the
/// profile test gives a distance d; no point of that turn's walls lies nearer p than
/// d / sqrt(1 + pi^2 (a^2 + b^2) / (4 rho r)), r the coil's least radius (the walls slant by
/// the motion, and |p - s|^2 >= the in-plane offset plus 2 rho r (1 - cos w)). The ends lie on
/// the half planes at angles 0 and `angle`.
#[allow(clippy::too_many_arguments)]
fn coil_member(
    frame: &Frame,
    segs: &[Seg2],
    a: f64,
    b: f64,
    angle: f64,
    r: (f64, f64),
    h: (f64, f64),
    p: Point3,
    m: f64,
) -> Member {
    let l = frame.point_to_local(p);
    let rho = (l.x * l.x + l.y * l.y).sqrt();
    let (da, db) = (a * angle, b * angle);
    let least = r.0 + da.min(0.0);
    if rho < least - m
        || rho > r.1 + da.max(0.0) + m
        || l.z < h.0 + db.min(0.0) - m
        || l.z > h.1 + db.max(0.0) + m
    {
        return Member::Out;
    }
    if rho <= 0.0 || least <= 0.0 {
        return Member::Near;
    }
    let slant = (1.0 + dmath::PI * dmath::PI * (a * a + b * b) / (4.0 * rho * least)).sqrt();
    let phi = dmath::atan2(l.y, l.x);
    let k0 = ((-dmath::PI - phi) / dmath::TAU).ceil() as i64;
    let k1 = ((angle + dmath::PI - phi) / dmath::TAU).floor() as i64;
    let (mut inside_one, mut near) = (false, false);
    for k in k0..=k1 {
        let u = phi + dmath::TAU * k as f64;
        let q = Point2::new(rho - a * u, l.z - b * u);
        let gap = (h.0 - q.y).max(q.y - h.1).max(r.0 - q.x).max(q.x - r.1);
        if gap > slant * m {
            continue;
        }
        let Some((inside, d)) = in_profile(segs, q) else {
            near = true;
            continue;
        };
        let clear = d > slant * m;
        // how far round from the nearer end, and how far that end's half plane is
        let (into, off) = if u < 0.0 {
            (0.0, -u)
        } else if u > angle {
            (0.0, u - angle)
        } else {
            (u.min(angle - u), 0.0)
        };
        let to_end = |w: f64| {
            if w >= dmath::FRAC_PI_2 {
                rho
            } else {
                rho * dmath::sin(w)
            }
        };
        if off == 0.0 {
            if inside && clear && to_end(into) > m {
                inside_one = true;
            } else if inside || !clear {
                near = true;
            }
        } else if (inside || !clear) && to_end(off) <= m {
            near = true;
        }
    }
    if near {
        Member::Near
    } else if inside_one {
        Member::In
    } else {
        Member::Out
    }
}

/// Whether angle `a` lies on the arc from `start` through `sweep`.
fn on_sweep(a: f64, start: f64, sweep: f64) -> bool {
    if sweep.abs() >= dmath::TAU - 1e-12 {
        return true;
    }
    let (lo, w) = if sweep >= 0.0 {
        (start, sweep)
    } else {
        (start + sweep, -sweep)
    };
    (a - lo).rem_euclid(dmath::TAU) <= w
}

fn segment_distance(q: Point2, a: Point2, b: Point2) -> f64 {
    let ab = b - a;
    let t = ((q - a).dot(ab) / ab.dot(ab)).clamp(0.0, 1.0);
    q.distance(a + ab * t)
}

/// Whether segment `a`-`b` lies in a revolved solid's closed profile clear of its boundary by
/// more than `m` (true only when that is sure: an arc is taken as its whole circle). Lines on
/// the axis (through `axis.0`, square to `axis.1`) bound nothing.
fn segment_in(segs: &[Seg2], axis: (Point2, Vec2), a: Point2, b: Point2, m: f64) -> bool {
    let cross = |o: Point2, p: Point2, q: Point2| (p - o).x * (q - o).y - (p - o).y * (q - o).x;
    let on_axis = |p: Point2| (p - axis.0).dot(axis.1).abs() <= 1e-12;
    matches!(in_profile(segs, a), Some((true, _)))
        && segs.iter().all(|s| match *s {
            // the box says it only when clear
            Seg2::Curve(ref c) => {
                let bx = c.bbox(c.domain());
                let gap = |lo: f64, hi: f64, x0: f64, x1: f64| lo - x0.max(x1) > m || x0.min(x1) - hi > m;
                gap(bx.x.lo, bx.x.hi, a.x, b.x) || gap(bx.y.lo, bx.y.hi, a.y, b.y)
            }
            Seg2::Line { a: p, b: q } if on_axis(p) && on_axis(q) => true,
            Seg2::Line { a: p, b: q } => {
                let crosses =
                    cross(a, b, p) * cross(a, b, q) <= 0.0 && cross(p, q, a) * cross(p, q, b) <= 0.0;
                !crosses
                    && segment_distance(p, a, b)
                        .min(segment_distance(q, a, b))
                        .min(segment_distance(a, p, q))
                        .min(segment_distance(b, p, q))
                        > m
            }
            Seg2::Arc { c, r, .. } => {
                let far = c.distance(a).max(c.distance(b));
                far < r - m || segment_distance(c, a, b) > r + m
            }
        })
}

/// In, Out, or Near from a profile test and how far along an extent (`z` against `lo..hi`) the
/// point lies.
fn combine(prof: Option<(bool, f64)>, z: f64, lo: f64, hi: f64, m: f64) -> Member {
    let Some((in2, d2)) = prof else {
        return Member::Near;
    };
    if z < lo - m || z > hi + m || (!in2 && d2 > m) {
        return Member::Out;
    }
    let dz = (z - lo).min(hi - z);
    if in2 && d2 > m && dz > m {
        Member::In
    } else {
        Member::Near
    }
}

#[cold]
#[inline(never)]
fn curve_box(c: &forge_geom::spline::Spline) -> Box3 {
    c.bbox(c.domain())
}

/// The box around a profile (u lo, u hi, v lo, v hi).
#[inline]
fn profile_box(segs: &[Seg2]) -> (f64, f64, f64, f64) {
    let mut b = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    let mut take = |x: f64, y: f64, r: f64| {
        b = (b.0.min(x - r), b.1.max(x + r), b.2.min(y - r), b.3.max(y + r));
    };
    for s in segs {
        match *s {
            Seg2::Curve(ref c) => {
                let bx = curve_box(c);
                take(bx.x.lo, bx.y.lo, 0.0);
                take(bx.x.hi, bx.y.hi, 0.0);
            }
            Seg2::Line { a, b } => {
                take(a.x, a.y, 0.0);
                take(b.x, b.y, 0.0);
            }
            Seg2::Arc { c, r, .. } => take(c.x, c.y, r),
        }
    }
    b
}

/// The box `b` in `frame`'s coordinates, as a box there around its corners.
fn local_box(frame: &Frame, b: &Box3) -> Box3 {
    let mut l = Box3::EMPTY;
    for x in [b.x.lo, b.x.hi] {
        for y in [b.y.lo, b.y.hi] {
            for z in [b.z.lo, b.z.hi] {
                l.include(frame.point_to_local(Point3::new(x, y, z)));
            }
        }
    }
    l
}

impl Shape {
    /// For a prism or solid of revolution with a free curve in its profile, a box round it.
    pub fn curved_box(&self) -> Option<Box3> {
        let curved = |ss: &[Seg2]| ss.iter().any(|s| matches!(s, Seg2::Curve(_)));
        let world = |frame: &Frame, x: (f64, f64), y: (f64, f64), z: (f64, f64)| {
            let mut w = Box3::EMPTY;
            for a in [x.0, x.1] {
                for b in [y.0, y.1] {
                    for c in [z.0, z.1] {
                        w.include(frame.point_to_world(Point3::new(a, b, c)));
                    }
                }
            }
            w
        };
        match self {
            Shape::Prism { frame, segs, lo, hi } if curved(segs) => {
                let b = profile_box(segs);
                Some(world(frame, (b.0, b.1), (b.2, b.3), (*lo, *hi)))
            }
            Shape::Revolved {
                frame,
                origin,
                dir,
                perp,
                segs,
                ..
            } if curved(segs) => {
                let b = profile_box(segs);
                let (mut rm, mut h) = (0.0f64, (f64::INFINITY, f64::NEG_INFINITY));
                for x in [b.0, b.1] {
                    for y in [b.2, b.3] {
                        let q = Point2::new(x, y) - *origin;
                        rm = rm.max(q.dot(*perp).abs());
                        h = (h.0.min(q.dot(*dir)), h.1.max(q.dot(*dir)));
                    }
                }
                Some(world(frame, (-rm, rm), (-rm, rm), h))
            }
            Shape::Coil {
                frame,
                segs,
                a,
                b,
                angle,
                r,
                h,
            } if curved(segs) => {
                let rm = r.1 + (a * angle).max(0.0);
                let hs = (h.0 + (b * angle).min(0.0), h.1 + (b * angle).max(0.0));
                Some(world(frame, (-rm, rm), (-rm, rm), hs))
            }
            _ => None,
        }
    }
    /// Whether no point of `b` lies in the solid or within `m` of it, by bounds: true only when
    /// that is sure (false says nothing).
    pub fn box_out(&self, b: &Box3, m: f64) -> bool {
        // rounding in the bounds below is far under this
        let m = m + 1e-9 * (1.0 + b.diagonal());
        let apart = |lo: f64, hi: f64, plo: f64, phi: f64| hi < plo - m || lo > phi + m;
        match self {
            Shape::Prism { frame, segs, lo, hi } => {
                let l = local_box(frame, b);
                let p = profile_box(segs);
                apart(l.z.lo, l.z.hi, *lo, *hi)
                    || apart(l.x.lo, l.x.hi, p.0, p.1)
                    || apart(l.y.lo, l.y.hi, p.2, p.3)
            }
            Shape::Revolved {
                frame,
                origin,
                dir,
                perp,
                segs,
                ..
            } => {
                // the box's heights and radii about the axis, then the profile points they map to
                let l = local_box(frame, b);
                let gap = |lo: f64, hi: f64| {
                    if lo > 0.0 {
                        lo
                    } else if hi < 0.0 {
                        -hi
                    } else {
                        0.0
                    }
                };
                let (gx, gy) = (gap(l.x.lo, l.x.hi), gap(l.y.lo, l.y.hi));
                let r_lo = (gx * gx + gy * gy).sqrt();
                let (ax, ay) = (l.x.lo.abs().max(l.x.hi.abs()), l.y.lo.abs().max(l.y.hi.abs()));
                let r_hi = (ax * ax + ay * ay).sqrt();
                let span = |o: f64, d: f64, e: f64| {
                    let (h0, h1) = (d * l.z.lo, d * l.z.hi);
                    let (r0, r1) = (e * r_lo, e * r_hi);
                    (o + h0.min(h1) + r0.min(r1), o + h0.max(h1) + r0.max(r1))
                };
                let (u, v) = (span(origin.x, dir.x, perp.x), span(origin.y, dir.y, perp.y));
                let p = profile_box(segs);
                apart(u.0, u.1, p.0, p.1) || apart(v.0, v.1, p.2, p.3)
            }
            Shape::Ball { center, radius } => {
                let gap = |c: f64, lo: f64, hi: f64| (lo - c).max(c - hi).max(0.0);
                let d = Vec3::new(
                    gap(center.x, b.x.lo, b.x.hi),
                    gap(center.y, b.y.lo, b.y.hi),
                    gap(center.z, b.z.lo, b.z.hi),
                );
                d.length() > radius + m
            }
            // within the first solid or a solid added to it
            Shape::Csg { first, rest } => {
                first.box_out(b, m) && rest.iter().all(|(op, s)| *op != Op::Union || s.box_out(b, m))
            }
            Shape::Brep(within) => !within.grow(m).overlaps(b),
            Shape::Coil { .. } => false,
        }
    }

    /// Whether the solid is a revolved one, or a combination starting from one.
    pub fn revolved(&self) -> bool {
        match self {
            Shape::Revolved { .. } => true,
            Shape::Csg { first, .. } => first.revolved(),
            _ => false,
        }
    }

    /// Whether no point of a face on surface `s` within box `b` lies in the solid or within `m`
    /// of it: `box_out`, and for a cylinder about a revolved solid's axis, its one radius over
    /// the box's heights against the profile itself (true only when that is sure).
    pub fn face_out(&self, s: &forge_geom::Surface, b: &Box3, m: f64) -> bool {
        if self.box_out(b, m) {
            return true;
        }
        match (self, s) {
            (
                Shape::Revolved {
                    frame,
                    origin,
                    dir,
                    perp,
                    segs,
                    ..
                },
                forge_geom::Surface::Cylinder(c),
            ) => {
                // its axis within `off` of the solid's, along it to an angle's rounding
                let o = frame.point_to_local(c.frame.origin);
                let off = (o.x * o.x + o.y * o.y).sqrt();
                if c.frame.z().cross(frame.z()).length() > 1e-12 || off > 1e-6 {
                    return false;
                }
                let l = local_box(frame, b);
                let m = m + 1e-9 * (1.0 + b.diagonal());
                // the profile points the face can reach lie within `off` of a segment of heights
                // at its radius: walked from the lowest, each step as long as the clear disc at
                // the last point allows
                let mut h = l.z.lo;
                for _ in 0..32 {
                    match in_profile(segs, *origin + *dir * h + *perp * c.radius) {
                        Some((false, d)) if d > off + m => h += d - off - m,
                        _ => return false,
                    }
                    if h > l.z.hi {
                        return true;
                    }
                }
                false
            }
            (Shape::Csg { first, rest }, _) => {
                first.face_out(s, b, m) && rest.iter().all(|(op, x)| *op != Op::Union || x.face_out(s, b, m))
            }
            _ => false,
        }
    }

    /// Whether every point of `b` lies in the solid clear of its boundary by more than `m`, by
    /// bounds: true only when that is sure (false says nothing).
    pub fn box_in(&self, b: &Box3, m: f64) -> bool {
        let m = m + 1e-9 * (1.0 + b.diagonal());
        // the profile holds a disc of radius `r` about `c` clear by m
        let holds =
            |segs: &[Seg2], c: Point2, r: f64| matches!(in_profile(segs, c), Some((true, d)) if d > r + m);
        match self {
            Shape::Prism { frame, segs, lo, hi } => {
                let l = local_box(frame, b);
                let (w, h) = (l.x.hi - l.x.lo, l.y.hi - l.y.lo);
                l.z.lo > lo + m
                    && l.z.hi < hi - m
                    && holds(
                        segs,
                        Point2::new(0.5 * (l.x.lo + l.x.hi), 0.5 * (l.y.lo + l.y.hi)),
                        0.5 * (w * w + h * h).sqrt(),
                    )
            }
            Shape::Revolved {
                frame,
                origin,
                dir,
                perp,
                segs,
                angle: None,
            } => {
                // the box's heights and radii about the axis map onto a parallelogram of the profile
                let l = local_box(frame, b);
                let gap = |lo: f64, hi: f64| {
                    if lo > 0.0 {
                        lo
                    } else if hi < 0.0 {
                        -hi
                    } else {
                        0.0
                    }
                };
                let (gx, gy) = (gap(l.x.lo, l.x.hi), gap(l.y.lo, l.y.hi));
                let r_lo = (gx * gx + gy * gy).sqrt();
                let (ax, ay) = (l.x.lo.abs().max(l.x.hi.abs()), l.y.lo.abs().max(l.y.hi.abs()));
                let r_hi = (ax * ax + ay * ay).sqrt();
                let at = |h: f64, r: f64| *origin + *dir * h + *perp * r;
                let c = at(0.5 * (l.z.lo + l.z.hi), 0.5 * (r_lo + r_hi));
                let reach = [(l.z.lo, r_lo), (l.z.lo, r_hi), (l.z.hi, r_lo), (l.z.hi, r_hi)]
                    .iter()
                    .map(|&(h, r)| at(h, r).distance(c))
                    .fold(0.0, f64::max);
                holds(segs, c, reach)
            }
            Shape::Ball { center, radius } => {
                let far = |c: f64, lo: f64, hi: f64| (c - lo).abs().max((hi - c).abs());
                let d = Vec3::new(
                    far(center.x, b.x.lo, b.x.hi),
                    far(center.y, b.y.lo, b.y.hi),
                    far(center.z, b.z.lo, b.z.hi),
                );
                d.length() < radius - m
            }
            Shape::Csg { first, rest } => {
                first.box_in(b, m)
                    && rest.iter().all(|(op, s)| match op {
                        Op::Union => true,
                        Op::Subtract => s.box_out(b, m),
                        Op::Intersect => s.box_in(b, m),
                    })
            }
            _ => false,
        }
    }

    /// Whether the points of the plane through `at` square to `normal` lying within
    /// `reach(axis)` of a whole-turn revolved solid's axis all lie in it clear of its boundary by
    /// more than `m` (true only when that is sure; false for other solids, or the plane not
    /// square to the axis).
    pub fn plane_in(&self, at: Point3, normal: Vec3, reach: impl Fn(&Frame) -> f64, m: f64) -> bool {
        let Shape::Revolved {
            frame,
            origin,
            dir,
            perp,
            segs,
            angle: None,
        } = self
        else {
            return false;
        };
        if normal.cross(frame.z().get()).length() > 1e-12 {
            return false;
        }
        let h = frame.point_to_local(at).z;
        let r = reach(frame);
        let at = |r: f64| *origin + *dir * h + *perp * r;
        r.is_finite() && segment_in(segs, (*origin, *perp), at(0.0), at(r), m + 1e-9 * (1.0 + r))
    }

    /// Where `p` is, clear of the boundary by more than `m`, or Near.
    pub fn member(&self, p: Point3, m: f64) -> Member {
        match self {
            Shape::Prism { frame, segs, lo, hi } => {
                let l = frame.point_to_local(p);
                if l.z < lo - m || l.z > hi + m {
                    return Member::Out;
                }
                combine(in_profile(segs, Point2::new(l.x, l.y)), l.z, *lo, *hi, m)
            }
            Shape::Revolved {
                frame,
                origin,
                dir,
                perp,
                segs,
                angle,
            } => {
                let l = frame.point_to_local(p);
                let r = (l.x * l.x + l.y * l.y).sqrt();
                let q = *origin + *dir * l.z + *perp * r;
                let prof = in_profile(segs, q);
                let Some(angle) = angle else {
                    return combine(prof, 0.0, -1.0, 1.0, m.min(0.5));
                };
                // a part turn: the sector between the two profile planes
                let a = dmath::atan2(l.y, l.x).rem_euclid(dmath::TAU);
                let sector = if a <= *angle {
                    // distance to the nearer bounding half plane
                    let to = |b: f64| {
                        let d = (a - b).abs();
                        if d >= dmath::FRAC_PI_2 {
                            r
                        } else {
                            r * dmath::sin(d)
                        }
                    };
                    (true, to(0.0).min(to(*angle)))
                } else {
                    let to = |b: f64| {
                        let d = (a - b).abs().min(dmath::TAU - (a - b).abs());
                        if d >= dmath::FRAC_PI_2 {
                            r
                        } else {
                            r * dmath::sin(d)
                        }
                    };
                    (false, to(0.0).min(to(*angle)))
                };
                match combine(prof, 0.0, -1.0, 1.0, m.min(0.5)) {
                    Member::Out => Member::Out,
                    _ if !sector.0 && sector.1 > m => Member::Out,
                    Member::In if sector.0 && sector.1 > m => Member::In,
                    _ => Member::Near,
                }
            }
            Shape::Csg { first, rest } => {
                // a combination's boundary lies on its parts': clear of all of theirs is clear of it
                let mut v = first.member(p, m);
                for (op, sh) in rest {
                    if v == Member::Out && *op != Op::Union {
                        continue;
                    }
                    let w = sh.member(p, m);
                    v = match op {
                        Op::Union if v == Member::In || w == Member::In => Member::In,
                        Op::Union if v == Member::Out && w == Member::Out => Member::Out,
                        Op::Subtract if v == Member::Out || w == Member::In => Member::Out,
                        Op::Subtract if v == Member::In && w == Member::Out => Member::In,
                        Op::Intersect if v == Member::Out || w == Member::Out => Member::Out,
                        Op::Intersect if v == Member::In && w == Member::In => Member::In,
                        _ => Member::Near,
                    };
                }
                v
            }
            Shape::Brep(b) => {
                if b.grow(m).contains(p) {
                    Member::Near
                } else {
                    Member::Out
                }
            }
            Shape::Coil {
                frame,
                segs,
                a,
                b,
                angle,
                r,
                h,
            } => coil_member(frame, segs, *a, *b, *angle, *r, *h, p, m),
            Shape::Ball { center, radius } => {
                let d = p.distance(*center);
                if d < radius - m {
                    Member::In
                } else if d > radius + m {
                    Member::Out
                } else {
                    Member::Near
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geom::{Cylinder, Surface};

    #[test]
    fn a_disc_on_a_bores_bottom_is_in_it_on_the_bores_side_within_its_radius() {
        // a bore of radius 4.6 from height 0 to 24 about the world z, its profile closed on the axis
        let p = |h: f64, r: f64| Point2::new(h, r);
        let corners = [p(0.0, 0.0), p(0.0, 4.6), p(24.0, 4.6), p(24.0, 0.0)];
        let bore = Shape::Revolved {
            frame: Frame::WORLD,
            origin: Point2::ORIGIN,
            dir: Vec2::new(1.0, 0.0),
            perp: Vec2::new(0.0, 1.0),
            segs: (0..4)
                .map(|i| Seg2::Line {
                    a: corners[i],
                    b: corners[(i + 1) % 4],
                })
                .collect(),
            angle: None,
        };
        let z = Vec3::new(0.0, 0.0, 1.0);
        let at = |h: f64| Point3::new(1.0, 2.0, h);
        assert!(bore.plane_in(at(24.0 - 1e-4), z, |_| 4.0, 1e-5));
        assert!(!bore.plane_in(at(24.0 - 1e-4), z, |_| 4.6, 1e-5));
        assert!(!bore.plane_in(at(24.0 + 1e-4), z, |_| 4.0, 1e-5));
        assert!(!bore.plane_in(at(24.0 - 1e-6), z, |_| 4.0, 1e-5));
        assert!(!bore.plane_in(at(12.0), Vec3::new(0.0, 0.1, 1.0), |_| 1.0, 1e-5));
    }

    #[test]
    fn a_cylinder_about_a_revolved_axis_is_out_where_the_profile_is_narrower() {
        // radius 10 for heights 0..50, 20 for 50..100, about the world z
        let p = |h: f64, r: f64| Point2::new(h, r);
        let corners = [
            p(0.0, 0.0),
            p(100.0, 0.0),
            p(100.0, 20.0),
            p(50.0, 20.0),
            p(50.0, 10.0),
            p(0.0, 10.0),
        ];
        let shaft = Shape::Revolved {
            frame: Frame::WORLD,
            origin: Point2::ORIGIN,
            dir: Vec2::new(1.0, 0.0),
            perp: Vec2::new(0.0, 1.0),
            segs: (0..6)
                .map(|i| Seg2::Line {
                    a: corners[i],
                    b: corners[(i + 1) % 6],
                })
                .collect(),
            angle: None,
        };
        let face = |at: Point3, radius: f64, lo: f64, hi: f64| {
            let s = Surface::Cylinder(Cylinder {
                frame: Frame::WORLD.with_origin(at),
                radius,
            });
            let b = Box3::from_points([
                Point3::new(at.x - radius, at.y - radius, lo),
                Point3::new(at.x + radius, at.y + radius, hi),
            ]);
            shaft.face_out(&s, &b, 1e-6)
        };
        assert!(face(Point3::ORIGIN, 13.0, 20.0, 30.0));
        assert!(!face(Point3::ORIGIN, 9.0, 20.0, 30.0));
        // past the step, or reaching it
        assert!(!face(Point3::ORIGIN, 13.0, 60.0, 70.0));
        assert!(!face(Point3::ORIGIN, 13.0, 40.0, 55.0));
        // off the axis
        assert!(!face(Point3::new(0.5, 0.0, 0.0), 13.0, 20.0, 30.0));
    }
}
