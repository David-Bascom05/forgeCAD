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
//! Where an arc meets a neighbour at an angle (a line or another arc) its joint moves on a circle:
//! `curved_event`, exact as well.
use crate::draft::{DraftError, draft, drafted};
use crate::extrude::{ExtrudeError, extrude, prepared};
use crate::profile::{Profile, ProfileLoop, Segment};
use crate::rebuild::rebuild_some;
use crate::taper_events::{Event, adjacent_seg, cyc, first_event, least_root};
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
    /// An outline with a spline whose taper is not proved free of events within the depth (its
    /// joints all smooth, no part curling, no two parts meeting): refused, not searched for
    /// (research/kernel-audit/RESUME-taper-splines.md).
    SplineEvent,
}

/// What the offset does to a segment: where it starts by now, and how long it still is (<= 0: gone).
struct Moved {
    seg: Segment,
    length: f64,
}

/// A segment's direction at its start and end.
pub(crate) fn ends_dir(s: &Segment) -> (Vec2, Vec2) {
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

pub(crate) fn unit(v: Vec2) -> Vec2 {
    v * (1.0 / v.length_sq().sqrt().max(f64::MIN_POSITIVE))
}

/// Where the carriers of two neighbours, offset, meet: of a line's or a circle's two meets with a
/// circle, the one on the side of the centre's foot on the line (of the centres' line) where their
/// joint `from` was, so a joint is followed however far it goes; where it was on that axis, the
/// one nearest `near`.
fn meet(a: &Segment, b: &Segment, d: f64, near: Point2, from: Point2) -> Option<Point2> {
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
        // (a line all but tangent, by rounding only: it touches)
        let disc = if disc < 0.0 && disc >= -1e-12 * (b * b + r * r).max(1.0) {
            0.0
        } else {
            disc
        };
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
        (C::L(p, t), C::O(c, r)) | (C::O(c, r), C::L(p, t)) => on_line(p, t, c, r).map(|x| {
            // the foot of the centre on the line moves square to it, so the side is along t
            let foot = p + t * (c - p).dot(t);
            let was = (from - (foot - t.perp() * d)).dot(t);
            let side = |y: Point2| (y - foot).dot(t);
            if was.abs() <= 1e-9 * (1.0 + r) || (side(x[0]) > 0.0) == (side(x[1]) > 0.0) {
                pick(x)
            } else if (side(x[0]) > 0.0) == (was > 0.0) {
                x[0]
            } else {
                x[1]
            }
        }),
        (C::O(c, r), C::O(e, s)) => {
            let w = e - c;
            let dist = w.length_sq().sqrt();
            if dist <= f64::MIN_POSITIVE {
                return None;
            }
            let x = (dist * dist + r * r - s * s) / (2.0 * dist);
            let h2 = r * r - x * x;
            let h2 = if h2 < 0.0 && h2 >= -1e-12 * (r * r).max(1.0) {
                0.0
            } else {
                h2
            };
            (h2 >= 0.0).then(|| {
                let t = w * (1.0 / dist);
                let m = c + t * x;
                let h = h2.sqrt();
                let was = (from - c).dot(t.perp());
                if was.abs() <= 1e-9 * (1.0 + r) || h == 0.0 {
                    pick([m + t.perp() * h, m - t.perp() * h])
                } else if was > 0.0 {
                    m + t.perp() * h
                } else {
                    m - t.perp() * h
                }
            })
        }
    }
}

/// Joint `i` of a loop offset `d`: where segment `i` starts.
fn joint(segs: &[(Segment, u64)], i: usize, d: f64) -> Option<Point2> {
    let n = segs.len();
    let (a, b) = (&cyc(segs, i + n - 1).0, &cyc(segs, i).0);
    let (ta, tb) = (unit(ends_dir(a).1), unit(ends_dir(b).0));
    let p = b.start();
    let left = unit(ta + tb).perp();
    let c = ta.x * tb.y - ta.y * tb.x;
    // tangent neighbours stay tangent, their joint moves square to both
    if c.abs() < 1e-7 && ta.dot(tb) > 0.0 {
        Some(p + left * d)
    } else {
        meet(a, b, d, p + left * d, p)
    }
}

/// Arc `i`'s sweep, offset `d`, from the turn `raw` between its joints: the one nearest what it
/// was (`sweep`), so it shrinks through nothing rather than jump a turn. Where it has moved at all
/// (only an arc meeting a neighbour at an angle turns) it may have turned more than half a turn on
/// the way (a major arc shrinking to its point, which the nearest takes for a whole circle), and
/// the sweep followed there in steps that each turn less than half a turn decides.
fn arc_sweep(segs: &[(Segment, u64)], i: usize, d: f64, sweep: f64, raw: f64) -> f64 {
    let near = |to: f64, mut sw: f64| {
        while sw - to > dmath::PI {
            sw -= dmath::TAU;
        }
        while sw - to < -dmath::PI {
            sw += dmath::TAU;
        }
        sw
    };
    let sw = near(sweep, raw);
    let Segment::Arc { center, .. } = segs[i].0 else {
        return sw;
    };
    if (sw - sweep).abs() <= 0.25 {
        return sw;
    }
    let n = segs.len();
    let at = |x: Point2| dmath::atan2(x.y - center.y, x.x - center.x);
    let swept = |t: f64| Some(at(joint(segs, (i + 1) % n, t)?) - at(joint(segs, i, t)?));
    const STEPS: u32 = 32;
    let mut last = sweep;
    let mut from = 0.0;
    for k in 1..=STEPS {
        // each step halved where the sweep turns fast (an arc's sweep running out goes as a root
        // of what is left, most of it in the last step)
        let mut ends = vec![d * f64::from(k) / f64::from(STEPS)];
        while let Some(&t) = ends.last() {
            let Some(s) = swept(t) else {
                if k == STEPS {
                    return near(last, raw);
                }
                return sw;
            };
            let s = near(last, s);
            if (s - last).abs() > 0.25 && ends.len() < 64 && t - from > 1e-15 * d {
                ends.push(0.5 * (from + t));
                continue;
            }
            last = s;
            from = t;
            ends.pop();
        }
    }
    near(last, raw)
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
        joints.push(joint(segs, i, d)?);
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
                let sw = arc_sweep(segs, i, d, sweep, at(q) - a0);
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
    let mut top_edges: Vec<(VertexId, VertexId, EdgeId, bool, Point2)> = Vec::new();
    let mut top_loops: Vec<Vec<(EdgeId, bool)>> = Vec::new();
    let mut bottom_loops: Vec<Vec<(EdgeId, bool)>> = Vec::new();
    let mut uprights: Vec<EdgeId> = Vec::new();
    // top edges two walls share: where a loop has closed up to a ridge or a slit
    let mut shared: Vec<EdgeId> = Vec::new();
    for (li, (l, m)) in p.loops.iter().zip(moved).enumerate() {
        let segs = &l.segments;
        let n = segs.len();
        let names: Vec<Name> = segs.iter().map(|s| crate::roles::side(root, s.1)).collect();
        let joint = |i: usize, n0: Name| if n <= 2 { n0.with_name(names[i]) } else { n0 };
        // a segment the offset has wiped out (`gone_at`, as `tapered` counts one gone) ends where it
        // starts: its two joints are one top vertex, decided by the segment, not by how far apart
        // the joints happen to be (a hexagon closing on its centre has joints 2 GONE apart)
        let mut first: Vec<usize> = (0..n).collect();
        for i in (0..n).filter(|&i| *cyc(cyc(wiped, li), i)) {
            let (a, c) = (*cyc(&first, i), *cyc(&first, i + 1));
            let (lo, hi) = (a.min(c), a.max(c));
            first.iter_mut().filter(|f| **f == hi).for_each(|f| *f = lo);
        }
        let mut verts: Vec<[VertexId; 2]> = Vec::with_capacity(n);
        for i in 0..n {
            let prev = *cyc(&names, i + n - 1);
            let name = joint(i, Name::NONE.with_set(&[start, prev, *cyc(&names, i)]));
            let vb = b.add_vertex(base.plane_point(cyc(segs, i).0.start()), name);
            let q = cyc(m, i).seg.start();
            let vt = if *cyc(&first, i) < i {
                verts[*cyc(&first, i)][1]
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
                // run along another wall's top edge: that edge, used the other way (the same ends
                // and the same way between them: a lens's two arcs share their ends, not their edge)
                let mid = m[i].seg.samples(2)[1];
                let used = match top_edges
                    .iter()
                    .find(|x| (x.0, x.1) == (tc, ta) && x.4.distance(mid) <= 2.0 * GONE)
                {
                    Some(&(_, _, e, fwd, _)) => {
                        shared.push(e);
                        (e, !fwd)
                    }
                    None => {
                        let (_, _, c, r, f) = wall(&m[i].seg, &lid);
                        let (s, en) = if f { (ta, tc) } else { (tc, ta) };
                        let e = b.add_edge(c, r, s, en, Name::NONE.with_set(&[end, names[i]]));
                        top_edges.push((ta, tc, e, !f, mid));
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
    // top edges no two walls share bound a lid: the loops that still bound something (a hole
    // closed to a slit ends under it, an outline closed to a ridge has none)
    let open: Vec<&Vec<(EdgeId, bool)>> = top_loops
        .iter()
        .filter(|l| !l.is_empty() && !l.iter().any(|u| shared.contains(&u.0)))
        .collect();
    if !open.is_empty() {
        let top = b.add_face(ShellId(0), Surface::Plane(Plane::new(lid)), true, end);
        for uses in open {
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

/// A loop closed up: no wider than what counts as gone.
fn closed(lp: &ProfileLoop) -> bool {
    let around: f64 = lp.segments.iter().map(|(x, _)| x.start().distance(x.end())).sum();
    lp.signed_area().abs() <= 8.0 * GONE * around
}

/// Whether a loop, offset `d`, has closed up there (segments that no longer meet have not: that
/// is walls crossing).
pub(crate) fn closes(segs: &[(Segment, u64)], d: f64) -> bool {
    offset(segs, d).is_some_and(|m| {
        let kept: Vec<(Segment, u64)> = m
            .iter()
            .zip(segs)
            .filter(|(x, _)| x.length > 2.0 * GONE)
            .map(|(x, s)| (x.seg.clone(), s.1))
            .collect();
        kept.is_empty() || closed(&ProfileLoop::new(kept))
    })
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
        let lp = ProfileLoop::new(kept);
        if !closed(&lp) {
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
    let spline = crate::skin::curved(profile).is_err();
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
    if spline {
        // drafted where nothing can happen on the way, else refused
        if !crate::taper_spline::clear(&p, sign, distance.abs() * lean) {
            return Err(TaperError::SplineEvent);
        }
        return draft(&b, &sides, &neutral, angle)
            .map(|d| vec![d])
            .map_err(TaperError::Draft);
    }
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
        // an event at the start: a side with no length, no slab to make; where it is the outline
        // closing up (within LINEAR of the last slab's top), closed there
        if let Some(d) = event.filter(|d| d / lean <= tol::LINEAR) {
            if !out.is_empty() && p.loops.iter().all(|l| closes(&l.segments, d * sign)) {
                break;
            }
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

// ---- the first event where an arc meets a neighbour at an angle (taper_events)

/// A polynomial in the offset δ, coefficients ascending.
type Poly = Vec<f64>;

fn padd(a: &[f64], b: &[f64]) -> Poly {
    (0..a.len().max(b.len()))
        .map(|i| a.get(i).copied().unwrap_or(0.0) + b.get(i).copied().unwrap_or(0.0))
        .collect()
}

fn pscale(a: &[f64], k: f64) -> Poly {
    a.iter().map(|x| x * k).collect()
}

fn pmul(a: &[f64], b: &[f64]) -> Poly {
    if a.is_empty() || b.is_empty() {
        return vec![];
    }
    let mut out = vec![0.0; a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (o, y) in out.iter_mut().skip(i).zip(b) {
            *o += x * y;
        }
    }
    out
}

/// A joint at offset δ: `p(δ) + v σ sqrt(r(δ))`, p a pair of polynomials; a joint between lines,
/// or tangent neighbours, has v = 0.
struct Joint {
    p: [Poly; 2],
    v: Vec2,
    r: Poly,
    sg: f64,
}

impl Joint {
    fn root(&self, d: f64) -> f64 {
        forge_math::poly::eval(&self.r, d).max(0.0).sqrt() * self.sg
    }
    fn at(&self, d: f64) -> Point2 {
        let w = self.root(d);
        Point2::new(
            forge_math::poly::eval(&self.p[0], d) + self.v.x * w,
            forge_math::poly::eval(&self.p[1], d) + self.v.y * w,
        )
    }
    /// `(J - o) . n` as `a(δ) + b σ sqrt(r(δ))`, with `o` and `n` fixed.
    fn along(&self, o: Point2, n: Vec2) -> (Poly, f64) {
        let a = padd(&pscale(&self.p[0], n.x), &pscale(&self.p[1], n.y));
        (padd(&a, &[-(o.x * n.x + o.y * n.y)]), self.v.dot(n) * self.sg)
    }
}

/// A loop's segments and its joints as they move.
type Moving<'a> = (&'a [(Segment, u64)], Vec<Joint>);

/// A line's carrier at δ: the points x with `(x - a) . n = sign δ`, `u` along it.
struct Carrier {
    a: Point2,
    u: Vec2,
    n: Vec2,
}

/// The roots in (0, reach] of `a + b1 sqrt(r1) + b2 sqrt(r2)` (b1, b2 already signed), found
/// on the polynomial its squares give and kept where the expression itself vanishes.
fn radical_roots(a: &[f64], (b1, r1): (f64, &[f64]), (b2, r2): (f64, &[f64]), reach: f64) -> Vec<f64> {
    let mut poly = if b1 == 0.0 && b2 == 0.0 {
        a.to_vec()
    } else if b2 == 0.0 || b1 == 0.0 {
        let (b, r) = if b1 == 0.0 { (b2, r2) } else { (b1, r1) };
        padd(&pmul(a, a), &pscale(&pmul(&[b * b], r), -1.0))
    } else {
        // (a + b1 w1)^2 = b2^2 r2, then the w1 term squared away
        let k = padd(&padd(&pmul(a, a), &pscale(r1, b1 * b1)), &pscale(r2, -b2 * b2));
        padd(&pmul(&k, &k), &pscale(&pmul(&pmul(a, a), r1), -4.0 * b1 * b1))
    };
    let lead = poly.iter().rposition(|x| x.abs() > 0.0).map_or(0, |i| i + 1);
    if lead <= 1 {
        return vec![];
    }
    let f = |d: f64| {
        let w = |r: &[f64]| forge_math::poly::eval(r, d).max(0.0).sqrt();
        forge_math::poly::eval(a, d) + b1 * w(r1) + b2 * w(r2)
    };
    let scale = a.iter().fold(0.0f64, |m, x| m.max(x.abs())) + b1.abs() + b2.abs() + 1.0;
    poly.truncate(lead);
    forge_math::poly::real_roots(&poly, 0.0, reach)
        .into_iter()
        .map(|r| r.x)
        .filter(|&d| d > 0.0 && f(d).abs() <= 1e-9 * scale * (1.0 + d))
        .collect()
}

/// The offsets in (0, reach] where joint `jk` is on the circle about `cc` of radius r0 + dr δ:
/// `|J - c|² - ρ² = A + B w` (w² = r), whose roots are those of `A² - B² r` kept where the radical
/// form vanishes.
fn on_circle(jk: &Joint, cc: Point2, r0: f64, dr: f64, reach: f64) -> Vec<f64> {
    let px = padd(&jk.p[0], &[-cc.x]);
    let py = padd(&jk.p[1], &[-cc.y]);
    let rho = [r0, dr];
    let a = padd(
        &padd(
            &padd(&pmul(&px, &px), &pmul(&py, &py)),
            &pscale(&jk.r, jk.v.dot(jk.v)),
        ),
        &pscale(&pmul(&rho, &rho), -1.0),
    );
    let b = padd(
        &pscale(&px, 2.0 * jk.v.x * jk.sg),
        &pscale(&py, 2.0 * jk.v.y * jk.sg),
    );
    if forge_math::poly::eval(&a, 0.0).abs() <= 0.0 {
        return vec![];
    }
    let mut poly = padd(&pmul(&a, &a), &pscale(&pmul(&pmul(&b, &b), &jk.r), -1.0));
    let lead = poly.iter().rposition(|x| x.abs() > 0.0).map_or(0, |i| i + 1);
    if lead <= 1 {
        return vec![];
    }
    poly.truncate(lead);
    forge_math::poly::real_roots(&poly, 0.0, reach)
        .into_iter()
        .map(|r| r.x)
        .filter(|&d| {
            let rho_d = r0 + dr * d;
            d > 0.0 && ((jk.at(d) - cc).length() - rho_d).abs() <= 1e-9 * (1.0 + rho_d)
        })
        .collect()
}

/// `first_event` where some joint is an arc meeting its neighbour at an angle: the joint moves
/// on the circle where the other's carrier meets it, so a side's length, a corner's distance from
/// a line and its distance from a circle are radicals of polynomials in the offset, whose roots
/// are those of the polynomial squaring gives (kept where the radical form vanishes). The joint is
/// lost (event J) where its radicand reaches 0. Two arcs meeting at an angle: the joint is where
/// their circles meet, `c1 + e a(δ) ± e⊥ sqrt(ρ1² - a²)` with `a = (ρ1² - ρ2² + D²) / 2D`, the
/// same form (p quadratic, r quartic in δ).
pub(crate) fn curved_event(p: &Profile, sign: f64, reach: f64) -> Option<Option<(f64, Event)>> {
    let mut loops: Vec<Moving> = Vec::new();
    let line = |s: &Segment| match *s {
        Segment::Line { a, b } => {
            let u = unit(b - a);
            Some(Carrier { a, u, n: u.perp() })
        }
        _ => None,
    };
    // a circle's centre, radius at 0 and its rate in δ
    let round = |s: &Segment| match *s {
        Segment::Arc {
            center,
            radius,
            sweep,
            ..
        } => Some((center, radius, if sweep > 0.0 { -sign } else { sign })),
        Segment::Circle { center, radius, ccw } => Some((center, radius, if ccw { -sign } else { sign })),
        _ => None,
    };
    for l in &p.loops {
        let segs = &l.segments[..];
        let n = segs.len();
        if n == 1 {
            loops.push((
                segs,
                vec![Joint {
                    p: [vec![segs[0].0.start().x], vec![segs[0].0.start().y]],
                    v: Vec2::new(0.0, 0.0),
                    r: vec![0.0],
                    sg: 0.0,
                }],
            ));
            continue;
        }
        let mut js = Vec::with_capacity(n);
        for i in 0..n {
            let (a, b) = (&cyc(segs, i + n - 1).0, &cyc(segs, i).0);
            let (ta, tb) = (unit(ends_dir(a).1), unit(ends_dir(b).0));
            let at = b.start();
            let c = ta.x * tb.y - ta.y * tb.x;
            let still = |w: Vec2| Joint {
                p: [vec![at.x, w.x * sign], vec![at.y, w.y * sign]],
                v: Vec2::new(0.0, 0.0),
                r: vec![0.0],
                sg: 0.0,
            };
            js.push(if c.abs() < 1e-7 && ta.dot(tb) > 0.0 {
                still(unit(ta + tb).perp())
            } else if let (Some(ca), Some(cb)) = (line(a), line(b)) {
                let den = 1.0 + ca.n.dot(cb.n);
                if den <= 1e-12 {
                    return Some(Some((0.0, Event::Cross)));
                }
                still((ca.n + cb.n) * (1.0 / den))
            } else if let (Some((c1, r1, k1)), Some((c2, r2, k2)), None, None) =
                (round(a), round(b), line(a), line(b))
            {
                // two circles: along the centres' line a(δ) from c1, across it sqrt(ρ1² - a²)
                let dd = c2 - c1;
                let dist = dd.length();
                if dist <= 1e-9 * (1.0 + r1 + r2) {
                    return None;
                }
                let e = dd * (1.0 / dist);
                let (rho1, rho2) = ([r1, k1], [r2, k2]);
                let along = pscale(
                    &padd(
                        &padd(&pmul(&rho1, &rho1), &pscale(&pmul(&rho2, &rho2), -1.0)),
                        &[dist * dist],
                    ),
                    0.5 / dist,
                );
                let r = padd(&pmul(&rho1, &rho1), &pscale(&pmul(&along, &along), -1.0));
                let v = e.perp();
                let mid = c1 + e * along[0];
                Joint {
                    p: [
                        padd(&[c1.x], &pscale(&along, e.x)),
                        padd(&[c1.y], &pscale(&along, e.y)),
                    ],
                    v,
                    r,
                    sg: if (at - mid).dot(v) < 0.0 { -1.0 } else { 1.0 },
                }
            } else {
                let l = line(a).or_else(|| line(b))?;
                let (cc, r0, dr) = round(a).or_else(|| round(b))?;
                // the foot of the centre on the carrier, d(δ) from it, and along it sqrt(ρ² - d²)
                let d = [(cc - l.a).dot(l.n), -sign];
                let foot = [
                    vec![cc.x - l.n.x * d[0], -l.n.x * d[1]],
                    vec![cc.y - l.n.y * d[0], -l.n.y * d[1]],
                ];
                let rho = [r0, dr];
                let r = padd(&pmul(&rho, &rho), &pscale(&pmul(&d, &d), -1.0));
                let side = (at - Point2::new(foot[0][0], foot[1][0])).dot(l.u);
                Joint {
                    p: foot,
                    v: l.u,
                    r,
                    sg: if side < 0.0 { -1.0 } else { 1.0 },
                }
            });
        }
        loops.push((segs, js));
    }
    let mut edge = f64::INFINITY;
    let mut lost = f64::INFINITY;
    let least = |m: &mut f64, ds: Vec<f64>| {
        for d in ds {
            *m = m.min(d);
        }
    };
    // E and J
    for (segs, js) in &loops {
        let n = segs.len();
        for j in js {
            if j.v != Vec2::new(0.0, 0.0) {
                // where the joint's radicand runs out, if it goes through 0
                let lostd: Vec<f64> = forge_math::poly::real_roots(&j.r, 0.0, reach)
                    .into_iter()
                    .map(|r| r.x)
                    .filter(|&d| d > 0.0)
                    .collect();
                least(&mut lost, lostd);
            }
        }
        for i in 0..n {
            let (a, b) = (&cyc(js, i), &cyc(js, i + 1));
            match cyc(segs, i).0 {
                Segment::Line { .. } => {
                    let c = line(&cyc(segs, i).0)?;
                    // (b - a) . u
                    let (pa, wa) = a.along(Point2::new(0.0, 0.0), c.u);
                    let (pb, wb) = b.along(Point2::new(0.0, 0.0), c.u);
                    let len = padd(&pb, &pscale(&pa, -1.0));
                    least(&mut edge, radical_roots(&len, (-wa, &a.r), (wb, &b.r), reach));
                }
                _ => {
                    let (_, r0, dr) = round(&cyc(segs, i).0)?;
                    least(&mut edge, least_root(0.0, dr, r0, reach).into_iter().collect());
                    if n > 1 {
                        // its sweep running out: its start reaching the next side's carrier
                        if let Some(cb) = line(&cyc(segs, i + 1).0) {
                            let (g, w) = a.along(cb.a, cb.n);
                            let g = padd(&g, &[0.0, -sign]);
                            for d in radical_roots(&g, (w, &a.r), (0.0, &[]), reach) {
                                if a.at(d).distance(b.at(d)) <= 1e-7 * (1.0 + r0) {
                                    edge = edge.min(d);
                                }
                            }
                        } else if let Some((cn, rn, dn)) = round(&cyc(segs, i + 1).0) {
                            // ... or the next arc's circle
                            for d in on_circle(a, cn, rn, dn, reach) {
                                if a.at(d).distance(b.at(d)) <= 1e-7 * (1.0 + r0) {
                                    edge = edge.min(d);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let same = |d: f64| edge.is_finite() && (d - edge).abs() <= 1e-9 * edge.max(1.0);
    let mut cross = lost;
    // the arc's sweep from its start joint to `q`, against its whole sweep, at δ
    let on_arc = |segs: &[(Segment, u64)], js: &[Joint], j: usize, q: Point2, d: f64| -> bool {
        let Segment::Arc { center, sweep, .. } = cyc(segs, j).0 else {
            return true;
        };
        let ang = |x: Point2| dmath::atan2(x.y - center.y, x.x - center.x);
        let k = if sweep > 0.0 { 1.0 } else { -1.0 };
        let turn = |from: f64, to: f64| (k * (to - from)).rem_euclid(dmath::TAU);
        let a0 = ang(cyc(js, j).at(d));
        let whole = turn(a0, ang(cyc(js, j + 1).at(d)));
        let t = turn(a0, ang(q));
        t <= whole + 1e-9 || t >= dmath::TAU - 1e-9
    };
    for (la, (sa, ja)) in loops.iter().enumerate() {
        let na = sa.len();
        for (lb, (sb, jb)) in loops.iter().enumerate() {
            let nb = sb.len();
            for j in 0..nb {
                let near = |k: usize| la == lb && (k == j || k == (j + 1) % nb);
                if let Some(c) = line(&cyc(sb, j).0) {
                    let len = |d: f64| (cyc(jb, j + 1).at(d) - cyc(jb, j).at(d)).dot(c.u);
                    for k in (0..na).filter(|&k| !near(k) && na > 1) {
                        let (g, w) = cyc(ja, k).along(c.a, c.n);
                        let g = padd(&g, &[0.0, -sign]);
                        if forge_math::poly::eval(&g, 0.0)
                            + w * forge_math::poly::eval(&cyc(ja, k).r, 0.0).max(0.0).sqrt()
                            == 0.0
                        {
                            continue;
                        }
                        for d in radical_roots(&g, (w, &cyc(ja, k).r), (0.0, &[]), reach) {
                            let f = (cyc(ja, k).at(d) - cyc(jb, j).at(d)).dot(c.u);
                            let l = len(d);
                            let e = 1e-9 * l.abs().max(1.0);
                            if f >= -e && f <= l + e && !(same(d) && (f <= e || f >= l - e)) {
                                cross = cross.min(d);
                            }
                        }
                    }
                    for i in (0..na).filter(|&i| !adjacent_seg(la, lb, i, j, na)) {
                        let Some((cc, r0, dr)) = round(&cyc(sa, i).0) else {
                            continue;
                        };
                        let e0 = (cc - c.a).dot(c.n);
                        for side in [1.0, -1.0] {
                            if let Some(d) = least_root(0.0, -sign - side * dr, e0 - side * r0, reach) {
                                let e = e0 - sign * d;
                                let foot = cc - c.n * e;
                                let f = (foot - cyc(jb, j).at(d)).dot(c.u);
                                if f >= 0.0 && f <= len(d) && on_arc(sa, ja, i, foot, d) && !same(d) {
                                    cross = cross.min(d);
                                }
                            }
                        }
                    }
                } else if let Some((cc, r0, dr)) = round(&cyc(sb, j).0) {
                    for k in (0..na).filter(|&k| !near(k) && na > 1) {
                        let jk = cyc(ja, k);
                        for d in on_circle(jk, cc, r0, dr, reach) {
                            if on_arc(sb, jb, j, jk.at(d), d) && !same(d) {
                                cross = cross.min(d);
                            }
                        }
                    }
                    for i in 0..na {
                        if (la, i) >= (lb, j) || adjacent_seg(la, lb, i, j, na) {
                            continue;
                        }
                        let Some((c2, r2, dr2)) = round(&cyc(sa, i).0) else {
                            continue;
                        };
                        let dist = (c2 - cc).length();
                        for (s1, s2) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0)] {
                            if let Some(d) =
                                least_root(0.0, s1 * dr + s2 * dr2, s1 * r0 + s2 * r2 - dist, reach)
                            {
                                let dir = if dist > 0.0 {
                                    (c2 - cc) * (1.0 / dist)
                                } else {
                                    Vec2::new(1.0, 0.0)
                                };
                                let t = cc + dir * (s1 * (r0 + dr * d));
                                if on_arc(sb, jb, j, t, d) && on_arc(sa, ja, i, t, d) && !same(d) {
                                    cross = cross.min(d);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Some(if cross <= edge && cross.is_finite() {
        // (a joint lost where its arc and line turn tangent: the loop closes to that point, its
        // area going as the joint goes, so it is closed just before)
        let shut = |d: f64| p.loops.iter().any(|l| closes(&l.segments, d * sign));
        if shut(cross) || (cross == lost && shut(cross * (1.0 - 1e-9))) {
            Some((cross, Event::Edge))
        } else {
            Some((cross, Event::Cross))
        }
    } else if edge.is_finite() {
        Some((edge, Event::Edge))
    } else {
        None
    })
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

    /// A slot of a hole tapered out closes to a slit (its ends wiped out, its sides meeting along
    /// one edge at offset 0.5): it ends there, and the plate goes on over it with its lid.
    #[test]
    fn a_hole_that_closes_to_a_slit_ends_under_the_lid() {
        let plate = profile(vec![
            lp(&[(0.0, 0.0), (10.0, 0.0), (10.0, 6.0), (0.0, 6.0)], 1),
            lp(&[(3.0, 2.5), (3.0, 3.5), (7.0, 3.5), (7.0, 2.5)], 11),
        ]);
        let a = 30f64.to_radians();
        let (t, d) = (tan(a), 2.0);
        // ∫ (10 + 2th)(6 + 2th) dh over the depth, less ∫ (4 - 2th)(1 - 2th) dh to where it closes
        let q = |c0: f64, c1: f64, c2: f64, h: f64| c0 * h + c1 * h * h / 2.0 + c2 * h * h * h / 3.0;
        let shut = 0.5 / t;
        let exact = q(60.0, 32.0 * t, 4.0 * t * t, d) - q(4.0, -10.0 * t, 4.0 * t * t, shut);
        close(volume(tapered(&plate, d, -a, 7)), exact);
    }

    /// A round hole growing into the plate's side (an arc reaching a line: C, at offset 0.5)
    /// is refused there, with no edge event before it.
    #[test]
    fn a_round_hole_meets_the_side() {
        let plate = profile(vec![
            lp(&[(0.0, 0.0), (10.0, 0.0), (10.0, 6.0), (0.0, 6.0)], 1),
            ProfileLoop::new(vec![(
                Segment::Circle {
                    center: Point2::new(8.0, 3.0),
                    radius: 1.0,
                    ccw: false,
                },
                11,
            )]),
        ]);
        let a = 45f64.to_radians();
        match tapered(&plate, 0.6, a, 7) {
            Err(TaperError::WallsCross { at }) => close(at, 0.5),
            other => panic!("{other:?}"),
        }
        assert!(tapered(&plate, 0.4, a, 7).is_ok());
    }

    /// A half disc: its arc meets its chord at a right angle (joints on a circle-line meet), and
    /// tapered in it closes to a point at offset r / 2: exact against its sections, the segment of
    /// the circle of radius r - t above the chord moved t.
    #[test]
    fn a_half_disc_closes_to_its_point() {
        let r = 2.0;
        let half = profile(vec![ProfileLoop::new(vec![
            (
                Segment::Arc {
                    center: Point2::new(0.0, 0.0),
                    radius: r,
                    start: 0.0,
                    sweep: PI,
                },
                1,
            ),
            (
                Segment::Line {
                    a: Point2::new(-r, 0.0),
                    b: Point2::new(r, 0.0),
                },
                2,
            ),
        ])]);
        let a = 45f64.to_radians();
        let area = |t: f64| {
            let q = r - t;
            if t >= q {
                0.0
            } else {
                q * q * dmath::acos(t / q) - t * (q * q - t * t).sqrt()
            }
        };
        for d in [0.5, 1.5] {
            // Simpson's rule, fine enough for 1e-9 here (the section is smooth up to its point)
            let (n, h) = (20000, d / 20000.0);
            let exact = (0..=n)
                .map(|k| {
                    let w = if k == 0 || k == n {
                        1.0
                    } else if k % 2 == 1 {
                        4.0
                    } else {
                        2.0
                    };
                    w * area(k as f64 * h * tan(a))
                })
                .sum::<f64>()
                * h
                / 3.0;
            assert!((volume(tapered(&half, d, a, 7)) - exact).abs() <= 1e-6 * exact);
        }
    }

    /// A major segment of a disc (its arc more than half a turn, closed by its chord at an angle):
    /// tapered in, the chord crosses the centre and the region closes to a point where the chord
    /// turns tangent (its joints lost, J, the loop's collapse). Its arc turns more than half a turn
    /// on the way, which `offset` follows. Exact against its sections: the disc of radius r - t
    /// cut by the chord moved t, the centre e0 - t inside it.
    #[test]
    fn a_major_segment_shrinks_past_its_centre_to_a_point() {
        let (r, a0, sw) = (2.5, 1.0, 4.4);
        let (s, c) = (dmath::sin_cos(a0), dmath::sin_cos(a0 + sw));
        let (p, q) = (Point2::new(r * s.1, r * s.0), Point2::new(r * c.1, r * c.0));
        let seg = profile(vec![ProfileLoop::new(vec![
            (
                Segment::Arc {
                    center: Point2::new(0.0, 0.0),
                    radius: r,
                    start: a0,
                    sweep: sw,
                },
                1,
            ),
            (Segment::Line { a: q, b: p }, 2),
        ])]);
        let n = (p - q) * (1.0 / (p - q).length());
        let e0 = (Point2::new(0.0, 0.0) - q).dot(n.perp());
        let area = |t: f64| {
            let (rho, e) = (r - t, e0 - t);
            if rho <= 0.0 || e <= -rho {
                0.0
            } else {
                rho * rho * dmath::acos(-e / rho) + e * (rho * rho - e * e).sqrt()
            }
        };
        let a = 50f64.to_radians();
        // past the point it closes to: (r + e0) / 2 from the start
        let d = 0.6 * (r + e0) / tan(a);
        let (k, h) = (20000, d / 20000.0);
        let exact = (0..=k)
            .map(|i| {
                let w = if i == 0 || i == k {
                    1.0
                } else if i % 2 == 1 {
                    4.0
                } else {
                    2.0
                };
                w * area(i as f64 * h * tan(a))
            })
            .sum::<f64>()
            * h
            / 3.0;
        assert!((volume(tapered(&seg, d, a, 7)) - exact).abs() <= 1e-6 * exact);
    }

    /// A rectangle with a round notch in its top, meeting it square: the notch grows as the bottom
    /// rises, and its lowest point reaches the bottom at offset 1.25 (S, an arc reaching a line,
    /// on the curved path): refused there, built below.
    #[test]
    fn a_round_notch_splits_where_it_reaches_the_bottom() {
        let p2 = Point2::new;
        let notch = profile(vec![ProfileLoop::new(vec![
            (
                Segment::Line {
                    a: p2(0.0, 0.0),
                    b: p2(10.0, 0.0),
                },
                1,
            ),
            (
                Segment::Line {
                    a: p2(10.0, 0.0),
                    b: p2(10.0, 4.0),
                },
                2,
            ),
            (
                Segment::Line {
                    a: p2(10.0, 4.0),
                    b: p2(6.5, 4.0),
                },
                3,
            ),
            (
                Segment::Arc {
                    center: p2(5.0, 4.0),
                    radius: 1.5,
                    start: 0.0,
                    sweep: -PI,
                },
                4,
            ),
            (
                Segment::Line {
                    a: p2(3.5, 4.0),
                    b: p2(0.0, 4.0),
                },
                5,
            ),
            (
                Segment::Line {
                    a: p2(0.0, 4.0),
                    b: p2(0.0, 0.0),
                },
                6,
            ),
        ])]);
        let a = 60f64.to_radians();
        match tapered(&notch, 0.75, a, 7) {
            Err(TaperError::WallsCross { at }) => close(at, 1.25 / tan(a)),
            other => panic!("{other:?}"),
        }
        assert!(tapered(&notch, 0.7, a, 7).is_ok());
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

    /// Simpson's rule over [0, d] of `area` at the offset `h tan a`.
    fn swept(area: impl Fn(f64) -> f64, d: f64, a: f64) -> f64 {
        let (n, h) = (20000, d / 20000.0);
        (0..=n)
            .map(|k| {
                let w = if k == 0 || k == n {
                    1.0
                } else if k % 2 == 1 {
                    4.0
                } else {
                    2.0
                };
                w * area(k as f64 * h * tan(a))
            })
            .sum::<f64>()
            * h
            / 3.0
    }

    fn arc(cx: f64, cy: f64, radius: f64, start: f64, sweep: f64, id: u64) -> (Segment, u64) {
        (
            Segment::Arc {
                center: Point2::new(cx, cy),
                radius,
                start,
                sweep,
            },
            id,
        )
    }

    /// The area two discs share, radii r1 and r2, centres d apart.
    fn lens(r1: f64, r2: f64, d: f64) -> f64 {
        if d >= r1 + r2 {
            return 0.0;
        }
        if d <= (r1 - r2).abs() {
            return PI * r1.min(r2) * r1.min(r2);
        }
        let a1 = dmath::acos((d * d + r1 * r1 - r2 * r2) / (2.0 * d * r1));
        let a2 = dmath::acos((d * d + r2 * r2 - r1 * r1) / (2.0 * d * r2));
        let (s1, c1) = dmath::sin_cos(a1);
        let (s2, c2) = dmath::sin_cos(a2);
        r1 * r1 * (a1 - s1 * c1) + r2 * r2 * (a2 - s2 * c2)
    }

    /// Two arcs meeting at an angle (the joint where their circles meet, moving on neither): a
    /// lens of two radius 2 discs 2 apart closes to a point where the circles turn tangent, 1 in.
    /// Exact against its sections, short of the point and past it.
    #[test]
    fn a_lens_closes_to_its_point() {
        let w = FRAC_PI_2 / 1.5; // 60 deg
        let lens2 = profile(vec![ProfileLoop::new(vec![
            arc(-1.0, 0.0, 2.0, -w, 2.0 * w, 1),
            arc(1.0, 0.0, 2.0, PI - w, 2.0 * w, 2),
        ])]);
        let a = 45f64.to_radians();
        let area = |t: f64| lens(2.0 - t, 2.0 - t, 2.0);
        for d in [0.5, 1.5] {
            let exact = swept(area, d, a);
            assert!((volume(tapered(&lens2, d, a, 7)) - exact).abs() <= 1e-6 * exact);
        }
    }

    /// A disc of radius 3 with a bite of radius 2 out of it, 3 from its centre. In, the bite grows
    /// as the disc shrinks and the crescent closes at a point where the bite takes it all (2 in):
    /// exact. Out, the bite shrinks inside the growing disc and the crescent's tips meet where the
    /// circles turn tangent inside, a hole pinched off at (d - r1 + r2) / 2 = 1: refused there.
    #[test]
    fn a_crescent_closes_in_and_pinches_out() {
        let (r1, r2, d) = (3.0, 2.0, 3.0);
        let w1 = dmath::acos((r1 * r1 + d * d - r2 * r2) / (2.0 * r1 * d));
        let w2 = dmath::acos((r2 * r2 + d * d - r1 * r1) / (2.0 * r2 * d));
        let crescent = profile(vec![ProfileLoop::new(vec![
            arc(0.0, 0.0, r1, w1, 2.0 * PI - 2.0 * w1, 1),
            arc(d, 0.0, r2, PI + w2, -2.0 * w2, 2),
        ])]);
        let a = 45f64.to_radians();
        let area = |t: f64| {
            let (q1, q2) = (r1 - t, r2 + t);
            if q2 - q1 >= d {
                0.0
            } else {
                PI * q1 * q1 - lens(q1, q2, d)
            }
        };
        for depth in [1.0, 3.0] {
            let exact = swept(area, depth, a);
            assert!((volume(tapered(&crescent, depth, a, 7)) - exact).abs() <= 1e-6 * exact);
        }
        let a = 30f64.to_radians();
        match tapered(&crescent, 3.0, -a, 7) {
            Err(TaperError::WallsCross { at }) => close(at, 1.0 / tan(a)),
            other => panic!("{:?}", other.map(|v| v.len())),
        }
    }

    /// A Reuleaux triangle (three discs of radius √3 about the corners of a triangle of circumradius
    /// 1): in, its three arcs run out together at the centre, √3 - 1 in. Each section is the
    /// triangle of its corners, q = sqrt(ρ² - 3/4) - 1/2 from the centre, and three segments.
    #[test]
    fn a_reuleaux_triangle_closes_on_its_centre() {
        let s = 3f64.sqrt();
        let v = |k: f64| dmath::sin_cos(FRAC_PI_2 + k * 2.0 * PI / 3.0);
        let (v0, v1, v2) = (v(0.0), v(1.0), v(2.0));
        // the arc about v2 from v0 to v1, about v0 from v1 to v2, about v1 from v2 to v0
        let start = |c: (f64, f64), p: (f64, f64)| dmath::atan2(p.0 - c.0, p.1 - c.1);
        let reuleaux = profile(vec![ProfileLoop::new(vec![
            arc(v2.1, v2.0, s, start(v2, v0), PI / 3.0, 1),
            arc(v0.1, v0.0, s, start(v0, v1), PI / 3.0, 2),
            arc(v1.1, v1.0, s, start(v1, v2), PI / 3.0, 3),
        ])]);
        let a = 60f64.to_radians();
        let area = |t: f64| {
            let rho = s - t;
            let q = (rho * rho - 0.75).max(0.0).sqrt() - 0.5;
            if q <= 0.0 {
                return 0.0;
            }
            let l = q * s;
            let phi = 2.0 * dmath::asin(l / (2.0 * rho));
            s / 4.0 * l * l + 1.5 * rho * rho * (phi - dmath::sin_cos(phi).0)
        };
        for depth in [0.2, 1.0] {
            let exact = swept(area, depth, a);
            assert!((volume(tapered(&reuleaux, depth, a, 7)) - exact).abs() <= 1e-6 * exact);
        }
    }

    /// A closed smooth spline through five points.
    fn blob() -> Profile {
        let pts = [
            (0.0, 0.0),
            (40.0, -5.0),
            (60.0, 20.0),
            (30.0, 40.0),
            (-5.0, 25.0),
            (0.0, 0.0),
        ];
        let sp = forge_geom::spline::PlaneSpline::through(
            pts.iter().map(|&(x, y)| Point2::new(x, y)).collect(),
            Some(Vec2::new(1.0, -0.6)),
            Some(Vec2::new(1.0, -0.6)),
        )
        .expect("a spline");
        profile(vec![ProfileLoop::new(vec![(
            Segment::Spline(std::sync::Arc::new(sp)),
            1,
        )])])
    }

    /// A spline outline with nothing on the way: each section the parallel curve, its area
    /// A0 - L0 δ + π δ² (Steiner, a smooth closed curve), in and out. A0 and L0 by Gauss-Legendre
    /// (A0's integrand a polynomial, exactly).
    #[test]
    fn a_smooth_spline_drafts_to_its_parallel_curves() {
        let p = blob();
        let Segment::Spline(ref sp) = p.loops[0].segments[0].0 else {
            unreachable!()
        };
        let c = &sp.c;
        let g = [
            (-0.906_179_845_938_664, 0.236_926_885_056_189_1),
            (-0.538_469_310_105_683_1, 0.478_628_670_499_366_5),
            (0.0, 0.568_888_888_888_888_9),
            (0.538_469_310_105_683_1, 0.478_628_670_499_366_5),
            (0.906_179_845_938_664, 0.236_926_885_056_189_1),
        ];
        let (mut a0, mut l0) = (0.0, 0.0);
        for w in c.breaks().windows(2) {
            for k in 0..64 {
                let (lo, hi) = (
                    w[0] + (w[1] - w[0]) * k as f64 / 64.0,
                    w[0] + (w[1] - w[0]) * (k + 1) as f64 / 64.0,
                );
                for &(x, wt) in &g {
                    let d = c.deriv(0.5 * (lo + hi) + 0.5 * (hi - lo) * x);
                    a0 += 0.5 * (hi - lo) * wt * 0.5 * (d.p.x * d.d1.y - d.p.y * d.d1.x);
                    l0 += 0.5 * (hi - lo) * wt * (d.d1.x * d.d1.x + d.d1.y * d.d1.y).sqrt();
                }
            }
        }
        let a0 = a0.abs();
        let d = 10.0;
        for deg in [5.0f64, 20.0, -5.0] {
            let a = deg.to_radians();
            let t = tan(a);
            let exact = a0 * d - l0 * t * d * d / 2.0 + PI * t * t * d * d * d / 3.0;
            let got = volume(tapered(&p, d, a, 7));
            assert!(
                (got - exact).abs() <= 1e-9 * exact,
                "{deg}: {got} against {exact}"
            );
        }
    }

    /// What the proof does not cover is refused, not searched: an outline that would curl (a
    /// taper deep enough that δ κ reaches 1), and a spline meeting a line at a corner.
    #[test]
    fn a_spline_outline_with_an_event_is_refused() {
        assert!(matches!(
            tapered(&blob(), 10.0, 40f64.to_radians(), 7),
            Err(TaperError::SplineEvent)
        ));
        let sp = forge_geom::spline::PlaneSpline::through(
            vec![
                Point2::new(0.0, 0.0),
                Point2::new(10.0, 8.0),
                Point2::new(20.0, 0.0),
            ],
            None,
            None,
        )
        .expect("a spline");
        let d = profile(vec![ProfileLoop::new(vec![
            (Segment::Spline(std::sync::Arc::new(sp)), 1),
            (
                Segment::Line {
                    a: Point2::new(20.0, 0.0),
                    b: Point2::new(0.0, 0.0),
                },
                2,
            ),
        ])]);
        assert!(matches!(
            tapered(&d, 1.0, 5f64.to_radians(), 7),
            Err(TaperError::SplineEvent)
        ));
    }
}
