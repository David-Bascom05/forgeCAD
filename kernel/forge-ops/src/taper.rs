//! A tapered extrude, its sides leaning in as the profile offsets. Plain cases are an extrude drafted.
//! When the offset wipes a segment out on the way (a corner's arc down to its point, a line between
//! two others that close on it), the faces change: slabs between those heights, each its own solid
//! with the wiped-out sides ending in a point, or a ridge where the profile closes up altogether.
//! Past a closed profile there is nothing, as in other CAD: a square tapered too far is a pyramid.
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

/// One slab: `p` at its base, offset `d` at height `h`, where some segments are gone. Walls leaning
/// `angle`; `top` vertices at one place are one vertex, top edges along one another one edge.
fn slab(p: &Profile, d: f64, h: f64, angle: f64, root: Name) -> Result<Body, TaperError> {
    let invalid = |e| TaperError::Draft(e);
    let mut b = Body::new();
    b.add_shell();
    let base = p.plane;
    let lid = base.with_origin(base.origin + base.z() * h);
    let (start, end) = (root.with_str("start"), root.with_str("end"));
    let bottom = b.add_face(ShellId(0), Surface::Plane(Plane::new(base)), false, start);
    let moved: Vec<Vec<Moved>> = p
        .loops
        .iter()
        .map(|l| offset(&l.segments, d).ok_or(TaperError::Draft(DraftError::BadAngle)))
        .collect::<Result<_, _>>()?;
    // a top is left where the loops still bound something
    let mut tops: Vec<(VertexId, Point2)> = Vec::new();
    let mut top_edges: Vec<(VertexId, VertexId, EdgeId, bool)> = Vec::new();
    let mut top_loops: Vec<Vec<(EdgeId, bool)>> = Vec::new();
    let mut bottom_loops: Vec<Vec<(EdgeId, bool)>> = Vec::new();
    let mut uprights: Vec<EdgeId> = Vec::new();
    let mut made = false;
    for (l, m) in p.loops.iter().zip(&moved) {
        let segs = &l.segments;
        let n = segs.len();
        let names: Vec<Name> = segs.iter().map(|s| root.with_str("side").with(s.1)).collect();
        let joint = |i: usize, n0: Name| if n <= 2 { n0.with_name(names[i]) } else { n0 };
        let mut verts: Vec<[VertexId; 2]> = Vec::with_capacity(n);
        for i in 0..n {
            let prev = names[(i + n - 1) % n];
            let name = joint(i, Name::NONE.with_set(&[start, prev, names[i]]));
            let vb = b.add_vertex(base.plane_point(segs[i].0.start()), name);
            let q = m[i].seg.start();
            let vt = match tops.iter().find(|t| t.1.distance(q) <= 2.0 * GONE) {
                Some(t) => t.0,
                None => {
                    let name = joint(i, Name::NONE.with_set(&[end, prev, names[i]]));
                    let v = b.add_vertex(lid.plane_point(q), name);
                    tops.push((v, q));
                    v
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

/// The solids of a tapered extrude, to be joined: `angle` leans the sides in going up (negative:
/// out), the profile as drawn at the sketch.
pub fn tapered(profile: &Profile, distance: f64, angle: f64, feature: u64) -> Result<Vec<Body>, TaperError> {
    crate::skin::curved(profile).map_err(|e| TaperError::Extrude(ExtrudeError::Profile(e)))?;
    let b = extrude(profile, distance, feature).map_err(TaperError::Extrude)?;
    let root = Name::feature(feature);
    let caps = [root.with_str("start"), root.with_str("end")];
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
    let plain = match draft(&b, &sides, &neutral, angle) {
        Ok(d) => return Ok(vec![d]),
        Err(e @ DraftError::BadAngle) => return Err(TaperError::Draft(e)),
        Err(e) => e,
    };
    let (sn, cs) = dmath::sin_cos(angle.abs());
    let lean = sn / cs;
    let sign = angle.signum();
    let mut p = prepared(profile, distance).map_err(TaperError::Extrude)?;
    let mut left = distance.abs();
    let mut out = Vec::new();
    let mut k = 0u64;
    while left > tol::LINEAR {
        // (each slab's names its own: they are joined)
        let id = feature.wrapping_add(k << 40);
        let Some(d) = next_event(&p, sign, left * lean) else {
            let e = extrude(&p, left, id).map_err(TaperError::Extrude)?;
            let r = Name::feature(id);
            let caps = [r.with_str("start"), r.with_str("end")];
            let s: Vec<Name> = e
                .face_ids()
                .map(|f| e.face(f).name)
                .filter(|n| !caps.contains(n))
                .collect();
            out.push(draft(&e, &s, &p.plane, angle).map_err(TaperError::Draft)?);
            break;
        };
        let h = d / lean;
        out.push(slab(&p, d * sign, h, angle, Name::feature(id))?);
        left -= h;
        k += 1;
        // what is left of each loop, up a slab
        let mut loops = Vec::new();
        for l in &p.loops {
            let Some(m) = offset(&l.segments, d * sign) else {
                return Err(TaperError::Draft(plain));
            };
            let kept: Vec<(Segment, u64)> = m
                .iter()
                .zip(&l.segments)
                .filter(|(x, _)| x.length > 2.0 * GONE)
                .map(|(x, s)| (x.seg.clone(), s.1))
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
                // the outline has closed up: nothing further
                return Ok(out);
            }
        }
        if loops.is_empty() {
            return Ok(out);
        }
        p = Profile {
            plane: p.plane.with_origin(p.plane.origin + p.plane.z() * h),
            loops,
        };
    }
    Ok(out)
}
