//! A body's box, exact: the least box holding every point of it, along each world axis its
//! furthest points.
//!
//! An inner bound: every point it is the hull of is a point of the body, so it is never larger
//! than the body's box, and smaller where a searched kind's extreme is missed. What must hold
//! the body (a tool's reach through it, culling, a clash test's boxes) takes an outer bound,
//! `Body::faces_box`, not this.
//!
//! The furthest point of a face along a direction is on its boundary or inside it, and inside
//! only where the surface turns square to the direction (a stationary point of the height) or
//! at a point where the surface has no tangent plane (a cone's apex, a sphere's pole, a spindle
//! torus's pole). So the box is the hull of the edges' own extremes and, for each face, of
//! those interior points that lie in it. One rule for every face:
//!
//! - an edge: a line's ends; a circle's, ellipse's, parabola's or hyperbola's extremes in closed
//!   form; an intersection or constructed curve's stationary points, from the sign changes of
//!   each axis's height between samples of the curve, refined to rounding;
//! - a plane or a cylinder has no interior point further than its boundary (a line of the
//!   surface through any point meets the boundary both ways), a cone only its apex;
//! - a sphere: its centre out by the radius along each axis, and its poles; a torus: the four
//!   points of each tube circle square to the axis, and a spindle's poles;
//! - a constructed surface straight across in v lies between its boundary's points; any other:
//!   its interior stationary points, from a grid of samples inside the face (its loops drawn
//!   coarsely, four chords an edge: its points are each solved for), refined by Newton's steps
//!   to rounding (a climb stopped by the drawn boundary is a boundary point, left to the edges);
//!
//! each interior point counted only if it is in the face (in its drawn loops). The searched
//! kinds run last, and only where their cached proven bound (the edge's or face's box) reaches
//! past the box so far. Every point counted is a point of the body, so the box is never larger
//! than the body's; the closed forms make it the exact box to rounding, and the searched kinds
//! too wherever their samples bracket each extreme (not proven).

use crate::uv::{UvLoop, default_segments, face_uv_loops, face_uv_loops_with, point_in_loops};
use crate::{Body, FaceId};
use forge_geom::{Curve, CurveGeom, Surface, SurfaceGeom};
use forge_math::{Box3, Interval, Point2, Point3, Vec3, dmath};

/// The exact box of `b` (see the module): an inner bound, never larger than the body's.
pub fn exact_box(b: &Body) -> Box3 {
    // the closed forms first, then what is searched for, each only where its proven bound
    // (the edge's or face's cached box) reaches past the box so far
    let mut bb = Box3::EMPTY;
    let searched_curve = |c: &Curve| matches!(c, Curve::Intersection(_) | Curve::Procedural(_));
    for e in b.edge_ids() {
        let edge = b.edge(e);
        if !searched_curve(&edge.curve) {
            bb = bb.hull(&curve_box(&edge.curve, edge.range));
        }
    }
    for f in b.face_ids() {
        if !matches!(b.face(f).surface, Surface::Procedural(_)) {
            face_extremes(b, f, &mut bb);
        }
    }
    for e in b.edge_ids() {
        let edge = b.edge(e);
        if searched_curve(&edge.curve) && reaches(&b.edge_box(e), &bb) {
            curve_into(&edge.curve, edge.range, &b.edge_box(e), &mut bb);
        }
    }
    for f in b.face_ids() {
        if matches!(b.face(f).surface, Surface::Procedural(_)) {
            face_extremes(b, f, &mut bb);
        }
    }
    bb
}

const AXES: [Vec3; 3] = [Vec3::X, Vec3::Y, Vec3::Z];

/// The exact box of `c` over `range`.
pub fn curve_box(c: &Curve, range: Interval) -> Box3 {
    let mut b = Box3::from_points([c.eval(range.lo), c.eval(range.hi)]);
    let mut at = |t: f64| {
        if t > range.lo && t < range.hi {
            b.include(c.eval(t));
        }
    };
    match c {
        Curve::Line(_) => {}
        Curve::Circle(k) => return k.tight_box(range),
        Curve::Ellipse(k) => {
            // a·x cos t + b·y sin t along each axis: extremes half a turn apart
            let (x, y) = (k.frame.x().get() * k.major, k.frame.y().get() * k.minor);
            for a in AXES {
                let phi = dmath::atan2(y.dot(a), x.dot(a));
                for t0 in [phi, phi + dmath::PI] {
                    at(t0 + dmath::TAU * ((range.lo - t0) / dmath::TAU).ceil());
                }
            }
        }
        Curve::Parabola(k) => {
            // (t²/4f)·α + t·β: stationary at t = -2fβ/α
            for a in AXES {
                let (al, be) = (k.frame.x().get().dot(a), k.frame.y().get().dot(a));
                if al != 0.0 {
                    at(-2.0 * k.focal * be / al);
                }
            }
        }
        Curve::Hyperbola(k) => {
            // a√(1+t²)·α + b t·β: stationary where t/√(1+t²) = -bβ/(aα), inside (-1, 1)
            for a in AXES {
                let (al, be) = (k.a * k.frame.x().get().dot(a), k.b * k.frame.y().get().dot(a));
                if al != 0.0 {
                    let s = -be / al;
                    if s.abs() < 1.0 {
                        at(s / (1.0 - s * s).sqrt());
                    }
                }
            }
        }
        Curve::Intersection(_) | Curve::Procedural(_) => curve_into(c, range, &c.bbox(range), &mut b),
    }
    b
}

/// Whether `b` reaches past `bb` on some side.
fn reaches(b: &Box3, bb: &Box3) -> bool {
    bb.is_empty() || bb.hull(b) != *bb
}

/// Takes into `bb` a searched curve's extremes over `range`, along each direction its proven
/// bound `bound` reaches past `bb` on: the height sampled, a sample above both neighbours
/// bracketing a maximum, found by Newton's steps on the height's slope, kept in the bracket by
/// bisection.
fn curve_into(c: &Curve, range: Interval, bound: &Box3, bb: &mut Box3) {
    const N: usize = 32;
    let mut dirs = [(0usize, 0.0f64); 6];
    let mut nd = 0;
    for k in 0..3 {
        let (x, y) = (axis(bound, k), axis(bb, k));
        if bb.is_empty() || x.hi > y.hi {
            dirs[nd] = (k, 1.0);
            nd += 1;
        }
        if bb.is_empty() || x.lo < y.lo {
            dirs[nd] = (k, -1.0);
            nd += 1;
        }
    }
    if nd == 0 {
        return;
    }
    let t = |i: usize| (range.lo + range.width() * (i as f64 / N as f64)).min(range.hi);
    let mut ps = Vec::with_capacity(N + 1);
    for i in 0..=N {
        ps.push(c.eval(t(i)));
    }
    for p in &ps {
        bb.include(*p);
    }
    for &(k, s) in &dirs[..nd] {
        let a = AXES[k];
        let h = |p: Point3| s * p.to_vec().dot(a);
        for i in 1..N {
            if !(h(ps[i]) >= h(ps[i - 1]) && h(ps[i]) >= h(ps[i + 1])) {
                continue;
            }
            let x = peak(t(i - 1), t(i + 1), |u| {
                let d = c.deriv(u);
                (s * d.d1.dot(a), s * d.d2.dot(a))
            });
            bb.include(c.eval(x));
        }
    }
}

/// Where the slope `g` (with its own slope) of a height with one maximum in `[lo, hi]` is zero:
/// Newton's steps, a bisection wherever one leaves the bracket the slope's signs keep.
fn peak(mut lo: f64, mut hi: f64, g: impl Fn(f64) -> (f64, f64)) -> f64 {
    let (glo, ghi) = (g(lo).0, g(hi).0);
    if glo <= 0.0 {
        return lo;
    }
    if ghi >= 0.0 {
        return hi;
    }
    let mut x = 0.5 * (lo + hi);
    for _ in 0..100 {
        let (d1, d2) = g(x);
        if d1 == 0.0 {
            return x;
        }
        if d1 > 0.0 {
            lo = x;
        } else {
            hi = x;
        }
        let n = x - d1 / d2;
        let next = if d2 < 0.0 && n > lo && n < hi {
            n
        } else {
            0.5 * (lo + hi)
        };
        if next == x || lo >= hi || hi - lo <= 4.0 * f64::EPSILON * (lo.abs().max(hi.abs())) {
            return next;
        }
        x = next;
    }
    x
}

/// `b` along axis `k`.
fn axis(b: &Box3, k: usize) -> Interval {
    [b.x, b.y, b.z][k]
}

/// Whether `p` is past `bb` along some axis.
fn past(p: Point3, bb: &Box3) -> bool {
    bb.is_empty() || !bb.contains(p)
}

/// A point of a face's surface where its height along an axis may be furthest inside it.
enum Candidate {
    /// a stationary point
    At(Point3),
    /// a point with no tangent plane, at `v` (all u): a pole or an apex
    Pole(Point3, f64),
}

/// Takes into `bb` the interior points of face `f` that are further than its boundary.
fn face_extremes(b: &Body, f: FaceId, bb: &mut Box3) {
    let face = b.face(f);
    let s = &face.surface;
    let mut cand: Vec<Candidate> = Vec::new();
    match s {
        Surface::Plane(_) | Surface::Cylinder(_) => return,
        Surface::Cone(c) => cand.push(Candidate::Pole(c.apex(), c.apex_v())),
        Surface::Sphere(sp) => {
            let o = sp.frame.origin;
            for a in AXES {
                cand.push(Candidate::At(o + a * sp.radius));
                cand.push(Candidate::At(o - a * sp.radius));
            }
            let z = sp.frame.z().get();
            let half = 0.5 * dmath::PI;
            cand.push(Candidate::Pole(o + z * sp.radius, half));
            cand.push(Candidate::Pole(o - z * sp.radius, -half));
        }
        Surface::Torus(t) => {
            let (o, z) = (t.frame.origin, t.frame.z().get());
            for a in AXES {
                let dz = a.dot(z);
                let flat = a - z * dz;
                let l = flat.length();
                if l > 1e-12 {
                    for w in [flat * (1.0 / l), flat * (-1.0 / l)] {
                        let c = o + w * t.major;
                        let n = w * a.dot(w) + z * dz;
                        let n = n * (1.0 / n.length());
                        cand.push(Candidate::At(c + n * t.minor));
                        cand.push(Candidate::At(c - n * t.minor));
                    }
                } else {
                    // square to the axis all round a circle: one point of it (a face holding part
                    // of the circle and not this point has an edge across it, at this height)
                    let x = t.frame.x().get();
                    cand.push(Candidate::At(o + x * t.major + z * t.minor));
                    cand.push(Candidate::At(o + x * t.major - z * t.minor));
                }
            }
            if t.is_spindle() {
                let v = t.pole_v();
                for v in [v, -v] {
                    cand.push(Candidate::Pole(s.eval(Point2::new(0.0, v)), v));
                }
            }
        }
        Surface::Procedural(pr) => {
            if pr.v_degree() == Some(1) && !face.loops.is_empty() {
                return;
            }
            // the face's bound (`face_box`, out by LINEAR): no side of it past the box, nothing
            // inside it is
            let fb = b.face_box(f).grow(-forge_math::tol::LINEAR);
            let mut dirs = Vec::new();
            for (k, a) in AXES.iter().enumerate() {
                let (side, now) = (axis(&fb, k), axis(bb, k));
                if bb.is_empty() || side.hi > now.hi {
                    dirs.push((*a, 1.0));
                }
                if bb.is_empty() || side.lo < now.lo {
                    dirs.push((*a, -1.0));
                }
            }
            if dirs.is_empty() {
                return;
            }
            // (drawn coarsely: a constructed surface's points are each solved for)
            let loops = face_uv_loops_with(b, f, |e| default_segments(e).min(4));
            return searched(s, &loops, &dirs, bb);
        }
    }
    cand.retain(|c| match c {
        Candidate::At(p) | Candidate::Pole(p, _) => past(*p, bb),
    });
    if cand.is_empty() {
        return;
    }
    let loops = face_uv_loops(b, f);
    for c in cand {
        let (p, inside) = match c {
            Candidate::At(p) => (p, in_face(s, &loops, s.project(p))),
            Candidate::Pole(p, v) => (p, holds_pole(&loops, v)),
        };
        if inside {
            bb.include(p);
        }
    }
}

/// Whether `uv` (or it moved by whole periods) is in the face drawn as `loops` (all of the
/// surface when it has none).
fn in_face(s: &Surface, loops: &[UvLoop], uv: Point2) -> bool {
    if loops.is_empty() {
        return true;
    }
    let d = s.domain();
    let shifts = |p: Option<f64>| match p {
        Some(t) => [0.0, t, -t, 2.0 * t, -2.0 * t],
        None => [0.0; 5],
    };
    let (su, sv) = (shifts(d.u.period), shifts(d.v.period));
    for (i, du) in su.iter().enumerate() {
        for (j, dv) in sv.iter().enumerate() {
            if (i > 0 && d.u.period.is_none()) || (j > 0 && d.v.period.is_none()) {
                continue;
            }
            if point_in_loops(Point2::new(uv.x + du, uv.y + dv), loops) {
                return true;
            }
        }
    }
    false
}

/// Whether the face drawn as `loops` holds the pole or apex at `v`: its loops reach the line
/// `v` stands for, or the face lies along it.
fn holds_pole(loops: &[UvLoop], v: f64) -> bool {
    if loops.is_empty() {
        return true;
    }
    let (mut su, mut sv) = (Interval::EMPTY, Interval::EMPTY);
    for l in loops {
        for p in &l.points {
            su.include(p.x);
            sv.include(p.y);
        }
    }
    let near = 1e-9 * (1.0 + v.abs() + sv.width());
    if loops
        .iter()
        .any(|l| l.points.iter().any(|p| (p.y - v).abs() <= near))
    {
        return true;
    }
    let d = 1e-6 * sv.width();
    point_in_loops(Point2::new(su.mid(), v + d), loops) || point_in_loops(Point2::new(su.mid(), v - d), loops)
}

/// The stationary points of a constructed surface's height inside the face drawn as `loops`:
/// a grid of samples over the loops' span, each sample in the face and above its neighbours in
/// the face refined by compass search, in the face, to rounding.
fn searched(s: &Surface, loops: &[UvLoop], dirs: &[(Vec3, f64)], bb: &mut Box3) {
    const G: usize = 12;
    let span = if loops.is_empty() {
        let d = s.domain();
        forge_math::Box2::new(d.u.interval(), d.v.interval())
    } else {
        let mut b = forge_math::Box2::new(Interval::EMPTY, Interval::EMPTY);
        for l in loops {
            for p in &l.points {
                b.u.include(p.x);
                b.v.include(p.y);
            }
        }
        b
    };
    if span.u.is_empty() || span.v.is_empty() {
        return;
    }
    let inside = |uv: Point2| loops.is_empty() || point_in_loops(uv, loops);
    let at = |i: usize, j: usize| {
        Point2::new(
            span.u.lo + span.u.width() * (i as f64 / G as f64),
            span.v.lo + span.v.width() * (j as f64 / G as f64),
        )
    };
    let mut grid: Vec<Option<Point3>> = Vec::with_capacity((G + 1) * (G + 1));
    for j in 0..=G {
        for i in 0..=G {
            let uv = at(i, j);
            grid.push(inside(uv).then(|| s.eval(uv)));
        }
    }
    let step0 = (span.u.width() / G as f64, span.v.width() / G as f64);
    for &(a, sg) in dirs {
        {
            let h = |p: Point3| sg * p.to_vec().dot(a);
            for j in 0..=G {
                for i in 0..=G {
                    let Some(p) = grid[j * (G + 1) + i] else { continue };
                    let mut top = true;
                    for (di, dj) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)] {
                        let (ni, nj) = (i as i64 + di, j as i64 + dj);
                        if ni < 0 || nj < 0 || ni > G as i64 || nj > G as i64 {
                            continue;
                        }
                        if let Some(q) = grid[nj as usize * (G + 1) + ni as usize]
                            && h(q) > h(p)
                        {
                            top = false;
                        }
                    }
                    if !top {
                        continue;
                    }
                    // Newton's steps on the height's gradient from the sample (a step up the
                    // gradient where the height does not curve down), each halved until it
                    // climbs and stays in the face; until no step climbs
                    let (mut uv, mut best, mut bestp) = (at(i, j), h(p), p);
                    for _ in 0..60 {
                        let d = s.deriv(uv);
                        let (gu, gv) = (sg * d.du.dot(a), sg * d.dv.dot(a));
                        let (huu, huv, hvv) = (sg * d.duu.dot(a), sg * d.duv.dot(a), sg * d.dvv.dot(a));
                        let det = huu * hvv - huv * huv;
                        let (mut su, mut sv) = if huu < 0.0 && det > 0.0 {
                            (-(hvv * gu - huv * gv) / det, -(huu * gv - huv * gu) / det)
                        } else {
                            // up the gradient, a sample's spacing at most
                            let g = (gu * gu / (step0.0 * step0.0) + gv * gv / (step0.1 * step0.1)).sqrt();
                            if g == 0.0 {
                                break;
                            }
                            (gu / g, gv / g)
                        };
                        let mut moved = false;
                        for _ in 0..40 {
                            let q = Point2::new(uv.x + su, uv.y + sv);
                            if (q.x == uv.x && q.y == uv.y) || !span.contains(q) && !inside(q) {
                                break;
                            }
                            if inside(q) {
                                let pq = s.eval(q);
                                if h(pq) > best {
                                    (uv, best, bestp, moved) = (q, h(pq), pq, true);
                                    break;
                                }
                            }
                            su *= 0.5;
                            sv *= 0.5;
                        }
                        if !moved {
                            break;
                        }
                    }
                    // only a stationary point counts: a climb stopped by the drawn boundary is
                    // at a boundary point (the edges' extremes have it), or past it
                    let d = s.deriv(uv);
                    let rise = (sg * d.du.dot(a) * step0.0).abs() + (sg * d.dv.dot(a) * step0.1).abs();
                    if rise <= 1e-9 * (1.0 + best.abs()) {
                        bb.include(bestp);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::Frame;

    /// The closed forms against a fine sampling: never short of it by more than its spacing
    /// lets a curve hide, never past it by more than rounding.
    #[test]
    fn conic_boxes_are_their_extremes() {
        let fr = Frame::WORLD
            .rotated_about_z(0.7)
            .with_origin(Point3::new(1.0, -2.0, 3.0));
        let tilt = Frame::from_zx(
            Point3::new(0.5, 0.0, -1.0),
            Vec3::new(0.3, -0.5, 0.8).normalize().unwrap(),
            Vec3::new(1.0, 0.2, 0.0),
        )
        .unwrap();
        let curves = [
            (
                Curve::Ellipse(forge_geom::Ellipse {
                    frame: tilt,
                    major: 3.0,
                    minor: 1.25,
                }),
                Interval::new(-2.0, 2.9),
            ),
            (
                Curve::Parabola(forge_geom::Parabola {
                    frame: tilt,
                    focal: 0.8,
                }),
                Interval::new(-3.0, 1.5),
            ),
            (
                Curve::Hyperbola(forge_geom::Hyperbola {
                    frame: fr,
                    a: 1.5,
                    b: 0.7,
                }),
                Interval::new(-2.0, 2.5),
            ),
            (
                Curve::Hyperbola(forge_geom::Hyperbola {
                    frame: tilt,
                    a: 1.0,
                    b: 2.0,
                }),
                Interval::new(-1.0, 3.0),
            ),
        ];
        for (c, r) in &curves {
            let b = curve_box(c, *r);
            let n = 200_000;
            let mut s = Box3::EMPTY;
            for i in 0..=n {
                s.include(c.eval(r.lo + r.width() * i as f64 / n as f64));
            }
            for (x, y) in [(b.x, s.x), (b.y, s.y), (b.z, s.z)] {
                assert!(
                    x.lo <= y.lo + 1e-12 && x.hi >= y.hi - 1e-12,
                    "{c:?}: {b:?} short of {s:?}"
                );
                assert!(
                    x.lo >= y.lo - 1e-9 && x.hi <= y.hi + 1e-9,
                    "{c:?}: {b:?} past {s:?}"
                );
            }
        }
    }
}
