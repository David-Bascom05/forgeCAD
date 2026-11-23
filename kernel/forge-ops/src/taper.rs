//! A tapered extrude, its sides leaning in as the profile offsets. Plain cases are an extrude drafted.
//! The section at each height is the profile's mitred offset; where it changes on the way the
//! first event decides (research/sweep-spec DESIGN.md section 2.2), computed exactly from the
//! carriers where every joint moves straight (`first_event`):
//! - none within the depth: the sides drafted;
//! - a segment wiped out (a corner's arc down to its point, a line between two others that close
//!   on it): slabs between those heights, each its own solid with the wiped-out sides ending in a
//!   point, or a ridge where the profile closes up altogether. Past a closed profile there is
//!   nothing, as in other CAD: a square tapered too far is a pyramid. A hole that closes ends;
//! - a corner reaching a side it does not end, two loops touching, or neighbours whose carriers no
//!   longer meet: `WallsCross`, refused, as no solid is the taper past it.
//!
//! Loops with an arc meeting a neighbour at an angle still find wiped-out segments by search
//! (`next_event`).
use crate::draft::{DraftError, draft, drafted};
use crate::extrude::{ExtrudeError, extrude, prepared};
use crate::profile::{Profile, ProfileLoop, Segment};
use crate::rebuild::rebuild_some;
use forge_geom::{Circle, Curve, Cylinder, Line, Plane, Surface};
use forge_math::{Frame, Interval, Point2, Vec2, dmath, tol};
use forge_topo::{Body, EdgeId, Name, ShellId, VertexId, check};

#[derive(Clone, Debug, PartialEq)]
pub enum TaperError {
    Extrude(ExtrudeError),
    Draft(DraftError),
    /// The offset sections stop bounding a region `at` this height from the sketch: a corner
    /// reaches a side it does not end (a split), two loops touch, or neighbouring sides stop
    /// meeting. No solid is the taper past it.
    WallsCross {
        at: f64,
    },
}

/// What the offset does to a segment: where it starts by now, and how long it still is (<= 0: gone).
struct Moved {
    seg: Segment,
    length: f64,
}

/// A segment's direction at its start and end.
fn ends_dir(s: &Segment) -> (Vec2, Vec2) {
    match *s {
        Segment::Line { a, b } => {
            let d = b - a;
            (d, d)
        }
        Segment::Arc { start, sweep, .. } => {
            let t = |a: f64| {
                let (sn, cs) = dmath::sin_cos(a);
                let k = if sweep > 0.0 { 1.0 } else { -1.0 };
                Vec2::new(-sn * k, cs * k)
            };
            (t(start), t(start + sweep))
        }
        Segment::Circle { .. } | Segment::Spline(_) => {
            unreachable!("a circle is alone in its loop; curves are refused")
        }
    }
}

fn unit(v: Vec2) -> Vec2 {
    v * (1.0 / v.length_sq().sqrt().max(f64::MIN_POSITIVE))
}

/// Where the carriers of two neighbours, offset, meet nearest `near`.
fn meet(a: &Segment, b: &Segment, d: f64, near: Point2) -> Option<Point2> {
    // a carrier as a line (point, direction) or a circle (centre, radius)
    enum C {
        L(Point2, Vec2),
        O(Point2, f64),
    }
    let carrier = |s: &Segment| match *s {
        Segment::Line { a, b } => {
            let t = unit(b - a);
            C::L(a + t.perp() * d, t)
        }
        Segment::Arc {
            center,
            radius,
            sweep,
            ..
        } => C::O(center, if sweep > 0.0 { radius - d } else { radius + d }),
        Segment::Circle { .. } | Segment::Spline(_) => {
            unreachable!("a circle is alone in its loop; curves are refused")
        }
    };
    let on_line = |p: Point2, t: Vec2, c: Point2, r: f64| {
        let f = p - c;
        let b = f.dot(t);
        let disc = b * b - (f.dot(f) - r * r);
        (disc >= 0.0).then(|| {
            let q = disc.sqrt();
            [p + t * (-b + q), p + t * (-b - q)]
        })
    };
    let pick = |x: [Point2; 2]| {
        if x[0].distance(near) <= x[1].distance(near) {
            x[0]
        } else {
            x[1]
        }
    };
    match (carrier(a), carrier(b)) {
        (C::L(p, t), C::L(q, u)) => {
            let den = t.x * u.y - t.y * u.x;
            (den.abs() > 1e-12).then(|| {
                let w = q - p;
                p + t * ((w.x * u.y - w.y * u.x) / den)
            })
        }
        (C::L(p, t), C::O(c, r)) | (C::O(c, r), C::L(p, t)) => on_line(p, t, c, r).map(pick),
        (C::O(c, r), C::O(e, s)) => {
            let w = e - c;
            let dist = w.length_sq().sqrt();
            if dist <= f64::MIN_POSITIVE {
                return None;
            }
            let x = (dist * dist + r * r - s * s) / (2.0 * dist);
            let h2 = r * r - x * x;
            (h2 >= 0.0).then(|| {
                let t = w * (1.0 / dist);
                let m = c + t * x;
                let h = h2.sqrt();
                pick([m + t.perp() * h, m - t.perp() * h])
            })
        }
    }
}

/// A loop offset `d` to its left (material's side), or None where its segments no longer meet.
fn offset(segs: &[(Segment, u64)], d: f64) -> Option<Vec<Moved>> {
    if let [(Segment::Circle { center, radius, ccw }, _)] = segs[..] {
        let r = if ccw { radius - d } else { radius + d };
        return Some(vec![Moved {
            seg: Segment::Circle {
                center,
                radius: r.max(0.0),
                ccw,
            },
            length: r,
        }]);
    }
    let n = segs.len();
    let mut joints = Vec::with_capacity(n);
    for i in 0..n {
        let (a, b) = (&segs[(i + n - 1) % n].0, &segs[i].0);
        let (ta, tb) = (unit(ends_dir(a).1), unit(ends_dir(b).0));
        let p = b.start();
        let left = unit(ta + tb).perp();
        let c = ta.x * tb.y - ta.y * tb.x;
        // tangent neighbours stay tangent, their joint moves square to both
        joints.push(if c.abs() < 1e-7 && ta.dot(tb) > 0.0 {
            p + left * d
        } else {
            meet(a, b, d, p + left * d)?
        });
    }
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let (p, q) = (joints[i], joints[(i + 1) % n]);
        out.push(match segs[i].0 {
            Segment::Spline(_) => unreachable!("curves are refused"),
            Segment::Line { a, b } => {
                let t = unit(b - a);
                Moved {
                    seg: Segment::Line { a: p, b: q },
                    length: (q - p).dot(t),
                }
            }
            Segment::Arc {
                center,
                radius,
                sweep,
                ..
            } => {
                let r = if sweep > 0.0 { radius - d } else { radius + d };
                let at = |x: Point2| dmath::atan2(x.y - center.y, x.x - center.x);
                let a0 = at(p);
                // the sweep nearest what it was: it shrinks through nothing rather than jump a turn
                let mut sw = at(q) - a0;
                while sw - sweep > dmath::PI {
                    sw -= dmath::TAU;
                }
                while sw - sweep < -dmath::PI {
                    sw += dmath::TAU;
                }
                Moved {
                    seg: Segment::Arc {
                        center,
                        radius: r.max(0.0),
                        start: a0,
                        sweep: sw,
                    },
                    length: if r <= 0.0 { r } else { r * sw * sweep.signum() },
                }
            }
            Segment::Circle { .. } => unreachable!("a circle is alone in its loop"),
        });
    }
    Some(out)
}

const GONE: f64 = 1e-7;

/// Which segments of each loop of `p`, offset `d` (`moved`) to where a segment is wiped out,
/// are gone at this event: under 2 GONE long, or wiped out at the event's own offset. Segments
/// shrink at their own rates (a line between corners of other angles shrinks faster), so where
/// the first is GONE long, others closing with it are still a few GONE long: each segment's rate
/// is its length's change since half the offset (the loop keeps its segments in between), and a
/// segment whose length runs out within twice the soonest time, over all loops, goes with it.
fn gone_at(p: &Profile, d: f64, moved: &[Vec<Moved>]) -> Vec<Vec<bool>> {
    let short = |m: &[Moved]| m.iter().map(|x| x.length <= 2.0 * GONE).collect::<Vec<bool>>();
    let half: Option<Vec<Vec<Moved>>> = p.loops.iter().map(|l| offset(&l.segments, 0.5 * d)).collect();
    let Some(half) = half else {
        return moved.iter().map(|m| short(m)).collect();
    };
    // the offset each segment has left: its length over the rate it shrinks at
    let left: Vec<Vec<f64>> = moved
        .iter()
        .zip(&half)
        .map(|(m, h)| {
            m.iter()
                .zip(h)
                .map(|(x, y)| {
                    let rate = (y.length - x.length) / (0.5 * d.abs());
                    if rate > 0.0 {
                        x.length.max(0.0) / rate
                    } else {
                        f64::INFINITY
                    }
                })
                .collect()
        })
        .collect();
    let soonest = left.iter().flatten().fold(f64::INFINITY, |a, &b| a.min(b));
    moved
        .iter()
        .zip(&left)
        .map(|(m, l)| {
            m.iter()
                .zip(l)
                .map(|(x, &t)| x.length <= 2.0 * GONE || t <= 2.0 * soonest)
                .collect()
        })
        .collect()
}

/// The kind of a taper's first event (research/sweep-spec DESIGN.md section 2.1).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Event {
    /// A side's length or an arc's radius reaches 0: slabs go on without it.
    Edge,
    /// A corner reaches a side it does not end, or two loops touch: the walls cross.
    Cross,
}

/// A loop whose every joint moves straight as the profile offsets: between lines, or where
/// neighbours are tangent (the joint moves square to both). Joint `i`, where segment `i` starts,
/// is `at[i] + vel[i] δ` at offset `δ` the way the taper goes; an arc keeps its angles, its radius
/// `r + dr δ`.
struct Lin<'a> {
    segs: &'a [(Segment, u64)],
    at: Vec<Point2>,
    vel: Vec<Vec2>,
}

impl Lin<'_> {
    /// Err(false) where a joint does not move straight (an arc meeting a neighbour at an angle),
    /// Err(true) where two neighbouring lines run back along each other: their carriers no longer
    /// meet (event J).
    fn new(segs: &[(Segment, u64)], sign: f64) -> Result<Lin<'_>, bool> {
        let n = segs.len();
        if let [(Segment::Circle { .. }, _)] = segs[..] {
            return Ok(Lin {
                segs,
                at: vec![segs[0].0.start()],
                vel: vec![Vec2::new(0.0, 0.0)],
            });
        }
        let mut at = Vec::with_capacity(n);
        let mut vel = Vec::with_capacity(n);
        for i in 0..n {
            let (a, b) = (&segs[(i + n - 1) % n].0, &segs[i].0);
            let (ta, tb) = (unit(ends_dir(a).1), unit(ends_dir(b).0));
            let c = ta.x * tb.y - ta.y * tb.x;
            let w = if c.abs() < 1e-7 && ta.dot(tb) > 0.0 {
                unit(ta + tb).perp()
            } else if matches!((a, b), (Segment::Line { .. }, Segment::Line { .. })) {
                // the meet of the two carriers, each moved 1 along its left normal
                let (na, nb) = (ta.perp(), tb.perp());
                let den = 1.0 + na.dot(nb);
                if den <= 1e-12 {
                    return Err(true);
                }
                (na + nb) * (1.0 / den)
            } else {
                return Err(false);
            };
            at.push(b.start());
            vel.push(w * sign);
        }
        Ok(Lin { segs, at, vel })
    }

    fn joint(&self, i: usize, d: f64) -> Point2 {
        self.at[i] + self.vel[i] * d
    }

    /// An arc's (or a circle's) centre and radius at offset `d`, and the radius's rate.
    fn round(&self, i: usize, sign: f64) -> Option<(Point2, f64, f64)> {
        match self.segs[i].0 {
            Segment::Arc {
                center,
                radius,
                sweep,
                ..
            } => Some((center, radius, if sweep > 0.0 { -sign } else { sign })),
            Segment::Circle { center, radius, ccw } => Some((center, radius, if ccw { -sign } else { sign })),
            _ => None,
        }
    }

    /// Whether direction `v` from an arc's centre lies within its sweep (a circle: always).
    fn within(&self, i: usize, v: Vec2) -> bool {
        match self.segs[i].0 {
            Segment::Arc { start, sweep, .. } => {
                let a = dmath::atan2(v.y, v.x);
                let (lo, hi) = (start.min(start + sweep), start.max(start + sweep));
                let mut t = a;
                while t < lo - 1e-12 {
                    t += dmath::TAU;
                }
                while t > hi + 1e-12 {
                    t -= dmath::TAU;
                }
                t >= lo - 1e-12 && t <= hi + 1e-12
            }
            _ => true,
        }
    }
}

/// The least root in (0, reach] of `a δ² + b δ + c` (c the value at 0), or None.
fn least_root(a: f64, b: f64, c: f64, reach: f64) -> Option<f64> {
    let roots: Vec<f64> = if a.abs() <= 1e-15 * (b.abs() + c.abs()).max(f64::MIN_POSITIVE) {
        if b == 0.0 { vec![] } else { vec![-c / b] }
    } else {
        let disc = b * b - 4.0 * a * c;
        if disc < 0.0 {
            vec![]
        } else {
            let q = -0.5 * (b + b.signum() * disc.sqrt());
            if q == 0.0 { vec![0.0] } else { vec![q / a, c / q] }
        }
    };
    roots
        .into_iter()
        .filter(|&r| r > 0.0 && r <= reach)
        .fold(None, |m: Option<f64>, r| Some(m.map_or(r, |m| m.min(r))))
}

/// The first event of tapering `p` by offsets up to `reach` (`sign` the way they go), each
/// computed exactly from the carriers (lines move straight, so lengths and contacts with lines
/// are linear in the offset, contacts with arcs quadratic), or None when the profile's loops are
/// not all `Lin` (`next_event` decides those). `Some(None)`: no event within `reach`; a crossing
/// at 0: neighbouring sides that no longer meet.
fn first_event(p: &Profile, sign: f64, reach: f64) -> Option<Option<(f64, Event)>> {
    let lins: Vec<Lin> = match p.loops.iter().map(|l| Lin::new(&l.segments, sign)).collect() {
        Ok(l) => l,
        // neighbours that no longer meet: the walls cross here
        Err(true) => return Some(Some((0.0, Event::Cross))),
        Err(false) => return None,
    };
    let mut edge = f64::INFINITY;
    // E: a side's length, or a radius, running out
    let mut roots: Vec<Vec<Option<f64>>> = Vec::with_capacity(lins.len());
    for l in &lins {
        let n = l.segs.len();
        let mut rs = Vec::with_capacity(n);
        for i in 0..n {
            let r = match l.segs[i].0 {
                Segment::Line { a, b } => {
                    let u = unit(b - a);
                    let len0 = (b - a).dot(u);
                    let rate = (l.vel[(i + 1) % n] - l.vel[i]).dot(u);
                    least_root(0.0, rate, len0, reach)
                }
                _ => {
                    let (_, r, dr) = l.round(i, sign)?;
                    least_root(0.0, dr, r, reach)
                }
            };
            if let Some(r) = r {
                edge = edge.min(r);
            }
            rs.push(r);
        }
        roots.push(rs);
    }
    let mut cross = f64::INFINITY;
    let same = |d: f64| (d - edge).abs() <= 1e-9 * edge.max(1.0);
    // the sides that run out with the first: their ends come together, which is the edge event
    let gone: Vec<Vec<bool>> = roots
        .iter()
        .map(|rs| rs.iter().map(|r| r.is_some_and(same)).collect())
        .collect();
    // whether joints k and m of loop l come together at the edge event: every side between them,
    // one way round, runs out with it
    let merged = |l: usize, k: usize, m: usize| {
        let n = gone[l].len();
        let run = |a: usize, b: usize| {
            (0..n)
                .map(|t| (a + t) % n)
                .take_while(|&t| t != b)
                .all(|t| gone[l][t])
        };
        run(k, m) || run(m, k)
    };
    // a contact at δ is the edge event itself when it comes with it, on the end of a side whose
    // joint there comes together with the corner by sides that run out
    let ends = |f: f64, len: f64| {
        let e = 1e-9 * len.max(1.0);
        (f <= e, f >= len - e)
    };
    for (la, a) in lins.iter().enumerate() {
        let na = a.segs.len();
        for (lb, b) in lins.iter().enumerate() {
            let nb = b.segs.len();
            for j in 0..nb {
                let adjacent = |k: usize| la == lb && (k == j || k == (j + 1) % nb);
                match b.segs[j].0 {
                    Segment::Line { a: s0, b: s1 } => {
                        let u = unit(s1 - s0);
                        let nrm = u.perp();
                        let len = |d: f64| (b.joint((j + 1) % nb, d) - b.joint(j, d)).dot(u);
                        // S, C: a corner reaching this side
                        for k in (0..na).filter(|&k| !adjacent(k)) {
                            if a.segs.len() == 1 && matches!(a.segs[0].0, Segment::Circle { .. }) {
                                break;
                            }
                            let g0 = (a.at[k] - b.at[j]).dot(nrm);
                            let g1 = (a.vel[k] - b.vel[j]).dot(nrm);
                            if g0 == 0.0 {
                                continue;
                            }
                            if let Some(d) = least_root(0.0, g1, g0, reach) {
                                let f = (a.joint(k, d) - b.joint(j, d)).dot(u);
                                let l = len(d);
                                let (at0, at1) = ends(f, l);
                                let with_edge = la == lb
                                    && same(d)
                                    && ((at0 && merged(la, k, j)) || (at1 && merged(la, k, (j + 1) % nb)));
                                if f >= -1e-9 * l.max(1.0) && f <= l + 1e-9 * l.max(1.0) && !with_edge {
                                    cross = cross.min(d);
                                }
                            }
                        }
                        // an arc or circle of loop a touching this side along its length
                        for i in (0..na).filter(|&i| !adjacent_seg(la, lb, i, j, na)) {
                            let Some((c, r, dr)) = a.round(i, sign) else {
                                continue;
                            };
                            // the centre's distance from the side's carrier, against the radius
                            let e0 = (c - b.at[j]).dot(nrm);
                            let e1 = -b.vel[j].dot(nrm);
                            for side in [1.0, -1.0] {
                                // e(δ) = side r(δ)
                                if let Some(d) = least_root(0.0, e1 - side * dr, e0 - side * r, reach) {
                                    let e = e0 + e1 * d;
                                    let foot = c - nrm * e;
                                    let f = (foot - b.joint(j, d)).dot(u);
                                    let l = len(d);
                                    if f >= 0.0 && f <= l && a.within(i, foot - c) && !same(d) {
                                        cross = cross.min(d);
                                    }
                                }
                            }
                        }
                    }
                    Segment::Arc { .. } | Segment::Circle { .. } => {
                        let (c, r, dr) = b.round(j, sign)?;
                        // S, C: a corner reaching this arc
                        if !(a.segs.len() == 1 && matches!(a.segs[0].0, Segment::Circle { .. })) {
                            for k in (0..na).filter(|&k| !adjacent(k)) {
                                let (p0, v) = (a.at[k] - c, a.vel[k]);
                                let qa = v.dot(v) - dr * dr;
                                let qb = 2.0 * (p0.dot(v) - r * dr);
                                let qc = p0.dot(p0) - r * r;
                                if qc == 0.0 {
                                    continue;
                                }
                                if let Some(d) = least_root(qa, qb, qc, reach)
                                    && b.within(j, a.joint(k, d) - c)
                                    && !same(d)
                                {
                                    cross = cross.min(d);
                                }
                            }
                        }
                        // another arc or circle touching this one
                        for i in 0..na {
                            if (la, i) >= (lb, j) || adjacent_seg(la, lb, i, j, na) {
                                continue;
                            }
                            let Some((c2, r2, dr2)) = a.round(i, sign) else {
                                continue;
                            };
                            let dist = (c2 - c).length();
                            for (s1, s2) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0)] {
                                // s1 r + s2 r2 = dist
                                if let Some(d) =
                                    least_root(0.0, s1 * dr + s2 * dr2, s1 * r + s2 * r2 - dist, reach)
                                {
                                    let rj = r + dr * d;
                                    let dir = if dist > 0.0 {
                                        (c2 - c) * (1.0 / dist)
                                    } else {
                                        Vec2::new(1.0, 0.0)
                                    };
                                    // the touch point, seen from each centre
                                    let t = c + dir * (s1 * rj);
                                    if b.within(j, t - c) && a.within(i, t - c2) && !same(d) {
                                        cross = cross.min(d);
                                    }
                                }
                            }
                        }
                    }
                    Segment::Spline(_) => return None,
                }
            }
        }
    }
    Some(if cross <= edge && cross.is_finite() {
        Some((cross, Event::Cross))
    } else if edge.is_finite() {
        Some((edge, Event::Edge))
    } else {
        None
    })
}

/// Whether segments `i` (of loop `la`) and `j` (of loop `lb`) are one segment or neighbours.
fn adjacent_seg(la: usize, lb: usize, i: usize, j: usize, n: usize) -> bool {
    la == lb && (i == j || (i + 1) % n == j || (j + 1) % n == i)
}

/// The least offset in (0, reach], `sign` the way it goes, at which a segment of the profile is
/// wiped out.
fn next_event(p: &Profile, sign: f64, reach: f64) -> Option<f64> {
    let wiped = |d: f64| {
        p.loops
            .iter()
            .any(|l| offset(&l.segments, d * sign).is_none_or(|m| m.iter().any(|x| x.length <= GONE)))
    };
    let n = 256;
    let mut lo = 0.0;
    for i in 1..=n {
        let hi = reach * (i as f64 / n as f64);
        if wiped(hi) {
            let mut hi = hi;
            for _ in 0..60 {
                let mid = 0.5 * (lo + hi);
                if wiped(mid) {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            // (the last offset whose segments still meet: what is about to go is under `2 GONE` long)
            return Some(lo);
        }
        lo = hi;
    }
    None
}

fn wall(s: &Segment, frame: &Frame) -> (Surface, bool, Curve, Interval, bool) {
    match *s {
        Segment::Spline(_) => unreachable!("curves are refused"),
        Segment::Line { a, b } => {
            let (pa, pb) = (frame.plane_point(a), frame.plane_point(b));
            let line = Line::through(pa, pb).expect("a line of some length");
            let out = line.dir.cross(frame.z()).normalize().expect("in the plane");
            let w = Frame::from_zx(pa, out, line.dir.get()).expect("perpendicular");
            (
                Surface::Plane(Plane::new(w)),
                true,
                Curve::Line(line),
                Interval::new(0.0, pa.distance(pb)),
                true,
            )
        }
        Segment::Arc {
            center,
            radius,
            start,
            sweep,
        } => {
            let axis = frame.with_origin(frame.plane_point(center));
            (
                Surface::Cylinder(Cylinder { frame: axis, radius }),
                sweep > 0.0,
                Curve::Circle(Circle { frame: axis, radius }),
                Interval::new(start.min(start + sweep), start.max(start + sweep)),
                sweep > 0.0,
            )
        }
        Segment::Circle { center, radius, ccw } => {
            let axis = frame.with_origin(frame.plane_point(center));
            (
                Surface::Cylinder(Cylinder { frame: axis, radius }),
                ccw,
                Curve::Circle(Circle { frame: axis, radius }),
                Interval::new(0.0, dmath::TAU),
                ccw,
            )
        }
    }
}

/// One slab: `p` at its base, its loops offset at height `h` to `moved`, where the segments
/// `wiped` (`gone_at`) are gone. Walls leaning `angle`; `top` vertices at one place are one vertex,
/// top edges along one another one edge.
fn slab(
    p: &Profile,
    (moved, wiped): (&[Vec<Moved>], &[Vec<bool>]),
    h: f64,
    angle: f64,
    root: Name,
) -> Result<Body, TaperError> {
    let invalid = |e| TaperError::Draft(e);
    let mut b = Body::new();
    b.add_shell();
    let base = p.plane;
    let lid = base.with_origin(base.origin + base.z() * h);
    let (start, end) = (crate::roles::start(root), crate::roles::end(root));
    let bottom = b.add_face(ShellId(0), Surface::Plane(Plane::new(base)), false, start);
    // a top is left where the loops still bound something
    let mut tops: Vec<(VertexId, Point2)> = Vec::new();
    let mut top_edges: Vec<(VertexId, VertexId, EdgeId, bool)> = Vec::new();
    let mut top_loops: Vec<Vec<(EdgeId, bool)>> = Vec::new();
    let mut bottom_loops: Vec<Vec<(EdgeId, bool)>> = Vec::new();
    let mut uprights: Vec<EdgeId> = Vec::new();
    let mut made = false;
    for (li, (l, m)) in p.loops.iter().zip(moved).enumerate() {
        let segs = &l.segments;
        let n = segs.len();
        let names: Vec<Name> = segs.iter().map(|s| crate::roles::side(root, s.1)).collect();
        let joint = |i: usize, n0: Name| if n <= 2 { n0.with_name(names[i]) } else { n0 };
        // a segment the offset has wiped out (`gone_at`, as `tapered` counts one gone) ends where it
        // starts: its two joints are one top vertex, decided by the segment, not by how far apart
        // the joints happen to be (a hexagon closing on its centre has joints 2 GONE apart)
        let mut first: Vec<usize> = (0..n).collect();
        for i in (0..n).filter(|&i| wiped[li][i]) {
            let (a, c) = (first[i], first[(i + 1) % n]);
            let (lo, hi) = (a.min(c), a.max(c));
            first.iter_mut().filter(|f| **f == hi).for_each(|f| *f = lo);
        }
        let mut verts: Vec<[VertexId; 2]> = Vec::with_capacity(n);
        for i in 0..n {
            let prev = names[(i + n - 1) % n];
            let name = joint(i, Name::NONE.with_set(&[start, prev, names[i]]));
            let vb = b.add_vertex(base.plane_point(segs[i].0.start()), name);
            let q = m[i].seg.start();
            let vt = if first[i] < i {
                verts[first[i]][1]
            } else {
                match tops.iter().find(|t| t.1.distance(q) <= 2.0 * GONE) {
                    Some(t) => t.0,
                    None => {
                        let name = joint(i, Name::NONE.with_set(&[end, prev, names[i]]));
                        let v = b.add_vertex(lid.plane_point(q), name);
                        tops.push((v, q));
                        v
                    }
                }
            };
            verts.push([vb, vt]);
        }
        let mut rises = Vec::with_capacity(n);
        for i in 0..n {
            let prev = names[(i + n - 1) % n];
            let (pa, pb) = (b.vertex(verts[i][0]).point, b.vertex(verts[i][1]).point);
            let line = Line::through(pa, pb).expect("the walls rise");
            let e = b.add_edge(
                Curve::Line(line),
                Interval::new(0.0, pa.distance(pb)),
                verts[i][0],
                verts[i][1],
                joint(i, Name::NONE.with_set(&[prev, names[i]])),
            );
            uprights.push(e);
            rises.push(e);
        }
        let (mut up, mut down) = (Vec::with_capacity(n), Vec::with_capacity(n));
        for i in 0..n {
            let j = (i + 1) % n;
            let (surface, same, curve, range, forward) = wall(&segs[i].0, &base);
            let (a, c) = (verts[i][0], verts[j][0]);
            let (s, e) = if forward { (a, c) } else { (c, a) };
            let lo = b.add_edge(curve, range, s, e, Name::NONE.with_set(&[start, names[i]]));
            down.push((lo, !forward));
            let surface =
                drafted(&surface, same, &Plane::new(base), base.z(), angle, names[i]).map_err(invalid)?;
            let face = b.add_face(ShellId(0), surface, same, names[i]);
            let (ta, tc) = (verts[i][1], verts[j][1]);
            if n == 1 {
                // a whole circle: the seam twice, and a rim unless it closes on its point
                let rim = (m[i].length > 2.0 * GONE).then(|| {
                    let (_, _, c, r, _) = wall(&m[i].seg, &lid);
                    let e = b.add_edge(c, r, ta, ta, Name::NONE.with_set(&[end, names[i]]));
                    (e, !forward)
                });
                let mut uses = vec![(lo, forward), (rises[0], true)];
                uses.extend(rim);
                uses.push((rises[0], false));
                b.add_loop(face, &uses);
                up.extend(rim.map(|r| (r.0, forward)));
                continue;
            }
            let mut uses = vec![(lo, forward), (rises[j], true)];
            if ta != tc {
                // run along another wall's top edge: that edge, used the other way
                let used = match top_edges.iter().find(|x| (x.0, x.1) == (tc, ta)) {
                    Some(&(_, _, e, fwd)) => {
                        made = true;
                        (e, !fwd)
                    }
                    None => {
                        let (_, _, c, r, f) = wall(&m[i].seg, &lid);
                        let (s, en) = if f { (ta, tc) } else { (tc, ta) };
                        let e = b.add_edge(c, r, s, en, Name::NONE.with_set(&[end, names[i]]));
                        top_edges.push((ta, tc, e, !f));
                        up.push((e, f));
                        (e, !f)
                    }
                };
                uses.push(used);
            }
            uses.push((rises[i], false));
            b.add_loop(face, &uses);
        }
        down.reverse();
        bottom_loops.push(down);
        top_loops.push(up);
    }
    for uses in &bottom_loops {
        b.add_loop(bottom, uses);
    }
    // top edges no two walls share bound a lid
    if !made && top_loops.iter().any(|l| !l.is_empty()) {
        let top = b.add_face(ShellId(0), Surface::Plane(Plane::new(lid)), true, end);
        for uses in top_loops.iter().filter(|l| !l.is_empty()) {
            b.add_loop(top, uses);
        }
    }
    let verts = vec![false; b.vertices.len()];
    let mut edges = vec![false; b.edges.len()];
    for e in &uprights {
        edges[e.index()] = true;
    }
    rebuild_some(&mut b, &verts, &edges).map_err(|e| invalid(DraftError::Rebuild(e)))?;
    let problems = check(&b);
    if !problems.is_empty() {
        return Err(invalid(DraftError::Invalid(problems)));
    }
    Ok(b)
}

/// What is left of `p`'s loops up a slab `h` high, offset to `moved` where `wiped` are gone: the
/// profile the next slab starts from, or None where the outline has closed up (a loop no wider
/// than what counts as gone, or no loop left). A hole that closes is dropped.
fn left_up(p: &Profile, moved: &[Vec<Moved>], wiped: &[Vec<bool>], h: f64) -> Option<Profile> {
    let mut loops = Vec::new();
    for ((l, m), w) in p.loops.iter().zip(moved).zip(wiped) {
        let kept: Vec<(Segment, u64)> = m
            .iter()
            .zip(&l.segments)
            .zip(w)
            .filter(|(_, w)| !**w)
            .map(|((x, s), _)| (x.seg.clone(), s.1))
            .collect();
        if kept.is_empty() {
            continue;
        }
        // (closed up: a loop no wider than what counts as gone)
        let around: f64 = kept.iter().map(|(x, _)| x.start().distance(x.end())).sum();
        let lp = ProfileLoop::new(kept);
        if lp.signed_area().abs() > 8.0 * GONE * around {
            loops.push(lp);
        } else if loops.is_empty() {
            return None;
        }
    }
    (!loops.is_empty()).then(|| Profile {
        plane: p.plane.with_origin(p.plane.origin + p.plane.z() * h),
        loops,
    })
}

/// Where a tapered extrude's outline closes up short of its depth: the height from the sketch at
/// which `tapered` stops with the pyramid or ridge its walls make (sweep-spec 2.2, rule 3), or None
/// where it reaches the depth, is drafted whole, or is refused. Nothing is built.
pub fn closes_at(profile: &Profile, distance: f64, angle: f64) -> Option<f64> {
    if !(angle.abs() > tol::ANGULAR && angle.abs() < dmath::FRAC_PI_2 - 1e-6) {
        return None;
    }
    let (sn, cs) = dmath::sin_cos(angle.abs());
    let lean = sn / cs;
    let sign = angle.signum();
    let mut p = prepared(profile, distance).ok()?;
    let mut left = distance.abs();
    while left > tol::LINEAR {
        let d = match first_event(&p, sign, left * lean) {
            Some(Some((d, Event::Edge))) => d,
            Some(_) => return None,
            None => next_event(&p, sign, left * lean)?,
        };
        if d / lean <= tol::LINEAR {
            return None;
        }
        let h = d / lean;
        let moved: Vec<Vec<Moved>> = p
            .loops
            .iter()
            .map(|l| offset(&l.segments, d * sign))
            .collect::<Option<_>>()?;
        let wiped = gone_at(&p, d * sign, &moved);
        left -= h;
        match left_up(&p, &moved, &wiped, h) {
            Some(next) => p = next,
            None => return (left > tol::LINEAR).then_some(distance.abs() - left),
        }
    }
    None
}

/// The solids of a tapered extrude, to be joined: `angle` leans the sides in going up (negative:
/// out), the profile as drawn at the sketch.
pub fn tapered(profile: &Profile, distance: f64, angle: f64, feature: u64) -> Result<Vec<Body>, TaperError> {
    crate::skin::curved(profile).map_err(|e| TaperError::Extrude(ExtrudeError::Profile(e)))?;
    let b = extrude(profile, distance, feature).map_err(TaperError::Extrude)?;
    let root = Name::feature(feature);
    let caps = [crate::roles::start(root), crate::roles::end(root)];
    let sides: Vec<Name> = b
        .face_ids()
        .map(|k| b.face(k).name)
        .filter(|n| !caps.contains(n))
        .collect();
    let neutral = if distance < 0.0 {
        profile.plane.flipped()
    } else {
        profile.plane
    };
    if !(angle.abs() > tol::ANGULAR && angle.abs() < dmath::FRAC_PI_2 - 1e-6) {
        return Err(TaperError::Draft(DraftError::BadAngle));
    }
    let (sn, cs) = dmath::sin_cos(angle.abs());
    let lean = sn / cs;
    let sign = angle.signum();
    let mut p = prepared(profile, distance).map_err(TaperError::Extrude)?;
    // the first event decides, where the loops' joints all move straight: none within the depth,
    // the sides drafted; a side wiped out, slabs; walls crossing, refused
    let first = first_event(&p, sign, distance.abs() * lean);
    match first {
        Some(None) => {
            return draft(&b, &sides, &neutral, angle)
                .map(|d| vec![d])
                .map_err(TaperError::Draft);
        }
        Some(Some((d, Event::Cross))) => return Err(TaperError::WallsCross { at: d / lean }),
        Some(Some((_, Event::Edge))) => {}
        // (other loops: the draft, unless it fails)
        None => match draft(&b, &sides, &neutral, angle) {
            Ok(d) => return Ok(vec![d]),
            Err(e @ DraftError::BadAngle) => return Err(TaperError::Draft(e)),
            // the sides wiped out on the way: slabs
            Err(_) => {}
        },
    }
    let mut pending = Some(first);
    let mut left = distance.abs();
    let mut out = Vec::new();
    let mut k = 0u64;
    while left > tol::LINEAR {
        // (each slab's names its own: they are joined)
        let id = feature.wrapping_add(k << 40);
        let up = distance.abs() - left;
        let event = match pending
            .take()
            .unwrap_or_else(|| first_event(&p, sign, left * lean))
        {
            Some(Some((d, Event::Cross))) => return Err(TaperError::WallsCross { at: up + d / lean }),
            Some(Some((d, Event::Edge))) => Some(d),
            Some(None) => None,
            None => next_event(&p, sign, left * lean),
        };
        // an event at the start: a side with no length, no slab to make
        if event.is_some_and(|d| d / lean <= tol::LINEAR) {
            return Err(TaperError::WallsCross { at: up });
        }
        let Some(d) = event else {
            let e = extrude(&p, left, id).map_err(TaperError::Extrude)?;
            let r = Name::feature(id);
            let caps = [crate::roles::start(r), crate::roles::end(r)];
            let s: Vec<Name> = e
                .face_ids()
                .map(|f| e.face(f).name)
                .filter(|n| !caps.contains(n))
                .collect();
            out.push(draft(&e, &s, &p.plane, angle).map_err(TaperError::Draft)?);
            break;
        };
        let h = d / lean;
        // each loop offset to the slab's top, and what is gone there: the slab's top and what is
        // left of the loops up it, decided once
        let moved: Vec<Vec<Moved>> = p
            .loops
            .iter()
            .map(|l| offset(&l.segments, d * sign).ok_or(TaperError::Draft(DraftError::BadAngle)))
            .collect::<Result<_, _>>()?;
        let wiped = gone_at(&p, d * sign, &moved);
        out.push(slab(&p, (&moved, &wiped), h, angle, Name::feature(id))?);
        left -= h;
        k += 1;
        // what is left of each loop, up a slab
        let Some(next) = left_up(&p, &moved, &wiped, h) else {
            // the outline has closed up: nothing further
            return Ok(out);
        };
        p = next;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, PI};

    fn lp(pts: &[(f64, f64)], id: u64) -> ProfileLoop {
        let n = pts.len();
        let p = |i: usize| Point2::new(pts[i % n].0, pts[i % n].1);
        ProfileLoop::new(
            (0..n)
                .map(|i| (Segment::Line { a: p(i), b: p(i + 1) }, id + i as u64))
                .collect(),
        )
    }

    fn profile(loops: Vec<ProfileLoop>) -> Profile {
        Profile {
            plane: Frame::WORLD,
            loops,
        }
    }

    /// The slabs' volume, each valid.
    fn volume(r: Result<Vec<Body>, TaperError>) -> f64 {
        let v = r.expect("builds");
        for b in &v {
            assert!(check(b).is_empty(), "{:?}", check(b));
        }
        v.iter()
            .map(|b| forge_topo::mass::mass(b).expect("a mass").volume)
            .sum()
    }

    fn tan(a: f64) -> f64 {
        let (s, c) = dmath::sin_cos(a);
        s / c
    }

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() <= 1e-9 * b.abs().max(1.0), "{a} against {b}");
    }

    /// A hexagon whose 80 deg walls close on its centre before the depth: the pyramid they make
    /// (sqrt 3 / tan 80 deg high), a regular one and one with a side moved.
    #[test]
    fn walls_that_meet_make_their_pyramid() {
        let s = 3f64.sqrt();
        let hex = profile(vec![lp(
            &[
                (2.0, 0.0),
                (1.0, s),
                (-1.0, s),
                (-2.0, 0.0),
                (-1.0, -s),
                (1.0, -s),
            ],
            1,
        )]);
        let h = s / tan(80f64.to_radians());
        close(
            volume(tapered(&hex, 0.5, 80f64.to_radians(), 7)),
            6.0 * s * h / 3.0,
        );
        close(
            volume(tapered(&hex, -0.5, 80f64.to_radians(), 7)),
            6.0 * s * h / 3.0,
        );
        let odd = profile(vec![lp(
            &[
                (2.0, 0.0),
                (1.0, s),
                (-1.0, s),
                (-2.0, 0.0),
                (-1.0, -s),
                (1.3, -s),
            ],
            1,
        )]);
        let v = volume(tapered(&odd, 0.5, 80f64.to_radians(), 7));
        assert!(v > 0.0 && v < 6.0 * s * 0.5);
        // where it closes: what forge-lang's lint says
        close(closes_at(&hex, 0.5, 80f64.to_radians()).expect("closes"), h);
        assert_eq!(closes_at(&hex, 0.5, 70f64.to_radians()), None);
        assert_eq!(closes_at(&hex, 0.25, 80f64.to_radians()), None);
    }

    /// A rectangle with a notch in its top: its floor and the bottom meet at offset 1 (an S event),
    /// so the taper is refused past height 1 / tan angle, and below it is the drafted solid.
    #[test]
    fn a_split_is_refused_where_it_comes() {
        let notch = profile(vec![lp(
            &[
                (0.0, 0.0),
                (10.0, 0.0),
                (10.0, 4.0),
                (6.0, 4.0),
                (6.0, 2.0),
                (4.0, 2.0),
                (4.0, 4.0),
                (0.0, 4.0),
            ],
            1,
        )]);
        let a = 60f64.to_radians();
        let at = 1.0 / tan(a);
        match tapered(&notch, 0.6, a, 7) {
            Err(TaperError::WallsCross { at: got }) => close(got, at),
            other => panic!("{other:?}"),
        }
        // below: the section's area is 36 - 32 t + 4 t^2 at offset t (the notch widens)
        let (d, t) = (0.5, tan(a));
        let exact = 36.0 * d - 16.0 * t * d * d + 4.0 * t * t * d * d * d / 3.0;
        close(volume(tapered(&notch, d, a, 7)), exact);
        // outward the notch closes at offset 1: its walls meet (no longer meeting carriers, J)
        let b = 30f64.to_radians();
        match tapered(&notch, 2.0, -b, 7) {
            Err(TaperError::WallsCross { at: got }) => close(got, 1.0 / tan(b)),
            other => panic!("{other:?}"),
        }
    }

    /// A hole growing into the outer loop (a C event at offset 0.5) is refused there; outward the
    /// hole closes at offset 1 and is dropped, and the solid goes on (exact volumes).
    #[test]
    fn a_hole_meets_the_outside_or_closes() {
        let plate = profile(vec![
            lp(&[(0.0, 0.0), (10.0, 0.0), (10.0, 6.0), (0.0, 6.0)], 1),
            lp(&[(7.0, 2.0), (7.0, 4.0), (9.0, 4.0), (9.0, 2.0)], 11),
        ]);
        let a = 45f64.to_radians();
        match tapered(&plate, 0.6, a, 7) {
            Err(TaperError::WallsCross { at }) => close(at, 0.5),
            other => panic!("{other:?}"),
        }
        // (10 - 2z)(6 - 2z) - (2 + 2z)^2 = 56 - 40 z
        close(volume(tapered(&plate, 0.4, a, 7)), 56.0 * 0.4 - 20.0 * 0.16);
        // outward: 56 + 40 z to the hole's closing at 1, then (10 + 2z)(6 + 2z)
        close(
            volume(tapered(&plate, 1.5, -a, 7)),
            76.0 + 30.0 + 20.0 + (4.0 / 3.0) * (3.375 - 1.0),
        );
    }

    /// A rounded rectangle whose corners close to points on the way (E events, its arcs' radii):
    /// exact against its sections, (w - 2t)(h - 2t) - (4 - pi)(r - t)^2 while t < r.
    #[test]
    fn rounded_corners_run_out_exactly() {
        let (w, h, r) = (4.0, 3.0, 0.5);
        let (x0, y0, x1, y1) = (-w / 2.0, -h / 2.0, w / 2.0, h / 2.0);
        let p = |x: f64, y: f64| Point2::new(x, y);
        let segs = vec![
            (
                Segment::Line {
                    a: p(x0 + r, y0),
                    b: p(x1 - r, y0),
                },
                1,
            ),
            (
                Segment::Arc {
                    center: p(x1 - r, y0 + r),
                    radius: r,
                    start: -FRAC_PI_2,
                    sweep: FRAC_PI_2,
                },
                2,
            ),
            (
                Segment::Line {
                    a: p(x1, y0 + r),
                    b: p(x1, y1 - r),
                },
                3,
            ),
            (
                Segment::Arc {
                    center: p(x1 - r, y1 - r),
                    radius: r,
                    start: 0.0,
                    sweep: FRAC_PI_2,
                },
                4,
            ),
            (
                Segment::Line {
                    a: p(x1 - r, y1),
                    b: p(x0 + r, y1),
                },
                5,
            ),
            (
                Segment::Arc {
                    center: p(x0 + r, y1 - r),
                    radius: r,
                    start: FRAC_PI_2,
                    sweep: FRAC_PI_2,
                },
                6,
            ),
            (
                Segment::Line {
                    a: p(x0, y1 - r),
                    b: p(x0, y0 + r),
                },
                7,
            ),
            (
                Segment::Arc {
                    center: p(x0 + r, y0 + r),
                    radius: r,
                    start: PI,
                    sweep: FRAC_PI_2,
                },
                8,
            ),
        ];
        let rr = profile(vec![ProfileLoop::new(segs)]);
        let a = 45f64.to_radians();
        let d = 1.0;
        // ∫0^d (w - 2z)(h - 2z) dz - (4 - pi) ∫0^r (r - z)^2 dz
        let exact = w * h * d - (w + h) * d * d + 4.0 * d * d * d / 3.0 - (4.0 - PI) * r * r * r / 3.0;
        close(volume(tapered(&rr, d, a, 7)), exact);
    }
}
