//! Ribs and webs: a thin wall from an open sketch path, grown until it meets the part.
//!
//! The wall is an extrusion reaching far past the part (the path's ends run on along their
//! tangents); the part is cut from it and the piece holding the path is what the part bounds.
//! Every face is exact: the extrusion's planes and cylinders (cones once drafted), cut by the
//! part's own faces.

use crate::Side;
use forge_bool::{BoolError, Index, Op, Where, boolean};
use forge_math::{Box3, Frame, Point2, Point3, Vec2, dmath, tol};
use forge_ops::{DraftError, ExtrudeError, Profile, ProfileLoop, Segment, draft, extrude};
use forge_topo::{Body, EdgeId, Name, ShellId, VertexId};

/// A rib: a wall `thickness` thick along an open `path` (lines and arcs end to end, in the
/// plane's xy). Across the path in the sketch plane by default, its thickness along the plane's
/// normal; a `web` instead reaches along the normal, its thickness across the path. It fills
/// from the path to the part on the path's left (or along the plane's -z for a web), or the
/// other way when `flip`. `draft` (0 or more) tapers its sides, thicker towards the part.
#[derive(Clone, Debug, PartialEq)]
pub struct Rib {
    pub plane: Frame,
    pub path: Vec<Segment>,
    pub thickness: f64,
    pub side: Side,
    pub web: bool,
    pub flip: bool,
    pub draft: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RibError {
    /// No path, a gap between its pieces, a whole circle, or a thickness of nothing.
    BadPath,
    /// The path turns a sharp corner at an arc (only lines meet at corners).
    Corner,
    /// An arc tighter than the web's thickness.
    TooTight,
    /// A draft that is negative (a rib thins towards the part) or reaches a right angle.
    BadDraft,
    /// A drafted rib across the sketch plane needs a straight path.
    DraftNeedsLine,
    /// The path lies inside the part: there is nothing to fill.
    NoRoom,
    /// The wall runs off the part without meeting it.
    RunsOff,
    Extrude(ExtrudeError),
    Draft(DraftError),
    Boolean(BoolError),
}

fn tangent(s: &Segment, end: bool) -> Option<Vec2> {
    match *s {
        Segment::Line { a, b } => (b - a).normalize(),
        Segment::Arc { start, sweep, .. } => {
            let (sn, cs) = dmath::sin_cos(if end { start + sweep } else { start });
            Some(Vec2::new(-sn, cs) * sweep.signum())
        }
        Segment::Circle { .. } => None,
    }
}

fn left(t: Vec2) -> Vec2 {
    Vec2::new(-t.y, t.x)
}

/// The path moved `o` to its left; sharp corners between lines are mitred.
fn offset_path(path: &[Segment], o: f64) -> Result<Vec<Segment>, RibError> {
    let mut out: Vec<Segment> = Vec::new();
    for s in path {
        let n = match *s {
            Segment::Line { a, b } => {
                let m = left(tangent(s, false).ok_or(RibError::BadPath)?) * o;
                Segment::Line { a: a + m, b: b + m }
            }
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            } => {
                let radius = radius - o * sweep.signum();
                if radius <= tol::MIN_LENGTH {
                    return Err(RibError::TooTight);
                }
                Segment::Arc {
                    center,
                    radius,
                    start,
                    sweep,
                }
            }
            Segment::Circle { .. } => return Err(RibError::BadPath),
        };
        if let Some(prev) = out.last_mut()
            && prev.end().distance(n.start()) > tol::LINEAR
        {
            let (Segment::Line { a: pa, b: pb }, Segment::Line { a: na, b: nb }) = (*prev, n) else {
                return Err(RibError::Corner);
            };
            let (d1, d2) = (pb - pa, nb - na);
            let den = d1.cross(d2);
            if den.abs() <= 1e-12 * d1.length() * d2.length() {
                return Err(RibError::Corner);
            }
            let p = pa + d1 * ((na - pa).cross(d2) / den);
            *prev = Segment::Line { a: pa, b: p };
            out.push(Segment::Line { a: p, b: nb });
            continue;
        }
        out.push(n);
    }
    Ok(out)
}

/// The path run on `far` past both ends along its tangents.
fn extended(path: &[Segment], far: f64) -> Vec<Segment> {
    let mut out = path.to_vec();
    let t0 = tangent(&path[0], false).expect("checked");
    let t1 = tangent(&path[path.len() - 1], true).expect("checked");
    match out[0] {
        Segment::Line { a, b } => out[0] = Segment::Line { a: a - t0 * far, b },
        s => out.insert(
            0,
            Segment::Line {
                a: s.start() - t0 * far,
                b: s.start(),
            },
        ),
    }
    let k = out.len() - 1;
    match out[k] {
        Segment::Line { a, b } => out[k] = Segment::Line { a, b: b + t1 * far },
        s => out.push(Segment::Line {
            a: s.end(),
            b: s.end() + t1 * far,
        }),
    }
    out
}

fn closed(segs: Vec<Segment>) -> ProfileLoop {
    let mut segs = segs;
    let (a, b) = (segs[segs.len() - 1].end(), segs[0].start());
    if a.distance(b) > tol::LINEAR {
        segs.push(Segment::Line { a, b });
    }
    ProfileLoop::new(segs.into_iter().zip(0u64..).collect())
}

/// Shell `s` of `b` as a body of its own.
fn shell_body(b: &Body, s: ShellId) -> Body {
    let mut out = Body::new();
    let sh = out.add_shell();
    let mut vmap: Vec<Option<VertexId>> = vec![None; b.vertices.len()];
    let mut emap: Vec<Option<EdgeId>> = vec![None; b.edges.len()];
    for &f in &b.shell(s).faces {
        let face = b.face(f);
        let nf = out.add_face(sh, face.surface.clone(), face.same_sense, face.name);
        for &l in &face.loops {
            let mut uses = Vec::new();
            for &c in &b.lp(l).coedges {
                let co = b.coedge(c);
                let e = *emap[co.edge.index()].get_or_insert_with(|| {
                    let edge = b.edge(co.edge);
                    let mut v = |v: VertexId| {
                        *vmap[v.index()].get_or_insert_with(|| {
                            let x = b.vertex(v);
                            out.add_tolerant_vertex(x.point, x.tol, x.name)
                        })
                    };
                    let (s, e) = (v(edge.start), v(edge.end));
                    out.add_edge(edge.curve.clone(), edge.range, s, e, edge.name)
                });
                uses.push((e, co.forward));
            }
            out.add_loop(nf, &uses);
        }
    }
    out
}

fn inside(inner: &Box3, outer: &Box3) -> bool {
    outer.x.lo <= inner.x.lo
        && inner.x.hi <= outer.x.hi
        && outer.y.lo <= inner.y.lo
        && inner.y.hi <= outer.y.hi
        && outer.z.lo <= inner.z.lo
        && inner.z.hi <= outer.z.hi
}

/// `body` with rib `r` (feature `id`) added.
pub(crate) fn rib(body: &Body, r: &Rib, id: u64) -> Result<Body, RibError> {
    let t = r.thickness;
    let joined = r
        .path
        .windows(2)
        .all(|w| w[0].end().distance(w[1].start()) <= tol::LINEAR);
    let ok = |s: &Segment| tangent(s, false).is_some() && s.start().distance(s.end()) > tol::MIN_LENGTH;
    if r.path.is_empty() || !joined || !r.path.iter().all(ok) || !t.is_finite() || t <= tol::MIN_LENGTH {
        return Err(RibError::BadPath);
    }
    if !(0.0..dmath::FRAC_PI_2).contains(&r.draft) {
        return Err(RibError::BadDraft);
    }
    let pl = &r.plane;
    let to3 = |p: Point2| pl.plane_point(p);
    let path_pts: Vec<Point3> = r.path.iter().flat_map(|s| s.samples(8)).map(to3).collect();
    let pts = body
        .vertices
        .iter()
        .map(|v| v.point)
        .chain(path_pts.iter().copied());
    let bound = Box3::from_points(pts);
    let diag = Point3::new(bound.x.lo, bound.y.lo, bound.z.lo)
        .distance(Point3::new(bound.x.hi, bound.y.hi, bound.z.hi));
    // the wall may stand out of the part's box by its thickness and taper; past that it ran off
    let margin = t + diag * dmath::sin(r.draft) / dmath::cos(r.draft) + 1.0;
    let far = 2.0 * (diag + margin) + 10.0;
    let (lo, hi) = match r.side {
        Side::Both => (-t / 2.0, t / 2.0),
        Side::Pos => (0.0, t),
        Side::Neg => (-t, 0.0),
    };
    let sign = if r.flip { -1.0 } else { 1.0 };
    // where the middle of the path is, and which way the wall fills from it
    let mid_seg = &r.path[r.path.len() / 2];
    let mid = mid_seg.samples(2)[1];
    let mid_left = left(tangent(mid_seg, false).expect("checked"));
    let root = Name::feature(id);
    let (tool, probe) = if r.web {
        let (a, b) = (offset_path(&r.path, hi)?, offset_path(&r.path, lo)?);
        let mut segs = extended(&a, far);
        let back = extended(&b, far);
        let cap = segs[segs.len() - 1].end();
        segs.push(Segment::Line {
            a: cap,
            b: back[back.len() - 1].end(),
        });
        segs.extend(back.iter().rev().map(|s| s.reversed()));
        let walls: Vec<Name> = (0..segs.len() as u64)
            .filter(|&k| k as usize != segs.len() - 1 - back.len())
            .map(|k| root.with_str("side").with(k))
            .collect();
        let profile = Profile {
            plane: *pl,
            loops: vec![closed(segs)],
        };
        let mut tool = extrude(&profile, -sign * far, id).map_err(RibError::Extrude)?;
        if r.draft != 0.0 {
            let neutral = if r.flip { pl.flipped() } else { *pl };
            let names: Vec<Name> = walls
                .into_iter()
                .filter(|n| tool.face_by_name(*n).is_some())
                .collect();
            tool = draft(&tool, &names, &neutral, r.draft).map_err(RibError::Draft)?;
        }
        let p = to3(mid + mid_left * ((lo + hi) / 2.0)) - pl.z() * (sign * 1e-3);
        (tool, p)
    } else {
        let m = |s: &Segment, end| left(tangent(s, end).expect("checked")) * sign * far;
        let mut segs = extended(&r.path, far);
        let (s0, e1) = (segs[0].start(), segs[segs.len() - 1].end());
        let (m0, m1) = (m(&r.path[0], false), m(&r.path[r.path.len() - 1], true));
        segs.push(Segment::Line { a: e1, b: e1 + m1 });
        segs.push(Segment::Line {
            a: e1 + m1,
            b: s0 + m0,
        });
        let plane = pl.with_origin(pl.origin + pl.z() * lo);
        let profile = Profile {
            plane,
            loops: vec![closed(segs)],
        };
        let mut tool = extrude(&profile, hi - lo, id).map_err(RibError::Extrude)?;
        let fill = pl.vec_to_world(forge_math::Vec3::new(mid_left.x, mid_left.y, 0.0)) * sign;
        if r.draft != 0.0 {
            let [Segment::Line { a, b }] = r.path[..] else {
                return Err(RibError::DraftNeedsLine);
            };
            let neutral =
                Frame::from_zx(to3(a), -fill.normalize().expect("unit"), to3(b) - to3(a)).expect("across");
            let caps = [root.with_str("start"), root.with_str("end")];
            tool = draft(&tool, &caps, &neutral, r.draft).map_err(RibError::Draft)?;
        }
        let p = to3(mid) + fill * 1e-3 + pl.z() * ((lo + hi) / 2.0);
        (tool, p)
    };
    let mut rest = boolean(&tool, body, Op::Subtract, id + 2_000_000).map_err(RibError::Boolean)?;
    rest.rebuild_shells();
    let lump = (0..rest.shells.len() as u32)
        .map(|s| shell_body(&rest, ShellId(s)))
        .find(|b| Index::new(b).classify(probe) == Where::Inside)
        .ok_or(RibError::NoRoom)?;
    let lb = Box3::from_points(lump.vertices.iter().map(|v| v.point));
    if !inside(&lb, &bound.grow(margin)) {
        return Err(RibError::RunsOff);
    }
    boolean(body, &lump, Op::Union, id + 3_000_000).map_err(RibError::Boolean)
}
