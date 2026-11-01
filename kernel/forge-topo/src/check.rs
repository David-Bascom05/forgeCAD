use crate::body::{Body, CoedgeId, EdgeId, FaceId, LoopId, VertexId};
use crate::name::Name;
use crate::uv::{UvLoop, default_segments, face_uv_loops_with, point_in_loops, unwrap_near};
use forge_geom::intersect::{Contact, CurveSurface, curve_surface};
use forge_geom::{Curve, CurveGeom, Line, SurfaceGeom};
use forge_math::{Box3, Interval, Point2, Point3, Vec3, tol};

#[derive(Clone, Debug, PartialEq)]
pub enum Problem {
    /// A solid edge must bound exactly two face sides.
    EdgeUses {
        edge: EdgeId,
        uses: usize,
    },
    /// The two uses of an edge must walk it in opposite directions.
    EdgeUsesSameWay {
        edge: EdgeId,
    },
    LoopEmpty {
        lp: LoopId,
    },
    /// Consecutive coedges do not share a vertex.
    LoopBroken {
        lp: LoopId,
        after: CoedgeId,
    },
    VertexOffEdge {
        edge: EdgeId,
        vertex: VertexId,
        distance: f64,
    },
    /// A tolerant vertex whose ball reaches halfway along an edge ending at it: the
    /// tolerance hides geometry instead of absorbing error.
    VertexTolerance {
        vertex: VertexId,
        tol: f64,
    },
    EdgeOffFace {
        edge: EdgeId,
        face: FaceId,
        distance: f64,
    },
    /// An open edge shorter than the tolerance, or a closed edge that is not a whole period.
    EdgeDegenerate {
        edge: EdgeId,
    },
    /// The face lies on the wrong side of this coedge: loop direction and face normal disagree.
    FaceWrongSide {
        face: FaceId,
        coedge: CoedgeId,
    },
    DuplicateName {
        name: Name,
    },
    /// V - E + 2F - L must be even and at most 2 per shell.
    Euler {
        value: i64,
    },
    FaceNoLoops {
        face: FaceId,
    },
    /// A planar face's first loop must be its outer boundary (the booleans rely on it), the
    /// rest its holes.
    OuterNotFirst {
        face: FaceId,
    },
    /// Shells overlap: just inside this face the body's material is counted twice (two outer
    /// shells overlapping, one inside the other), or just outside it there is a void outside
    /// any material.
    ShellsOverlap {
        face: FaceId,
    },
    /// One shell's V - E + 2F - L is odd or above 2.
    ShellEuler {
        shell: crate::body::ShellId,
        value: i64,
    },
    /// The faces round a vertex form more than one fan: the surface is pinched there.
    VertexFans {
        vertex: VertexId,
        fans: usize,
    },
}

/// Everything wrong with a body, in a deterministic order. Empty means it is a
/// valid closed solid.
pub fn check(body: &Body) -> Vec<Problem> {
    if body.dead > 0 {
        return check(&body.compact());
    }
    let mut out = Vec::new();
    check_edges(body, &mut out);
    check_loops(body, &mut out);
    check_geometry(body, &mut out);
    if out.is_empty() {
        check_sides(body, &mut out);
    }
    if out.is_empty() && body.shells.len() > 1 {
        check_shells(body, &mut out);
    }
    check_names(body, &mut out);
    check_euler(body, &mut out);
    // per shell and per vertex only once every edge has its two uses and every loop closes
    if !out
        .iter()
        .any(|p| matches!(p, Problem::EdgeUses { .. } | Problem::LoopBroken { .. }))
    {
        check_shell_euler(body, &mut out);
        check_fans(body, &mut out);
    }
    out
}

/// What is open in `body`, if it is not a closed solid: an edge not used by exactly two faces, or a
/// loop that does not close. The part of `check` far cheaper than its geometric tests, and what a
/// boolean that loses a face breaks; regeneration refuses a result that fails it.
pub fn open_at(body: &Body) -> Option<String> {
    let at = |v: crate::body::VertexId| {
        let p = body.vertex(v).point;
        format!("({:.3}, {:.3}, {:.3})", p.x, p.y, p.z)
    };
    for e in body.edge_ids() {
        let edge = body.edge(e);
        let uses = edge.coedges.len();
        if uses != 2 {
            let ends = format!("the edge from {} to {}", at(edge.start), at(edge.end));
            return Some(format!("{ends} is on {uses} face(s), not 2"));
        }
        if body.coedge(edge.coedges[0]).forward == body.coedge(edge.coedges[1]).forward {
            let ends = format!("the edge from {} to {}", at(edge.start), at(edge.end));
            return Some(format!("the faces either side of {ends} turn the same way"));
        }
    }
    for f in body.face_ids() {
        if body.face(f).loops.is_empty() {
            return Some("a face has no boundary".to_string());
        }
    }
    for l in body.loop_ids() {
        // each use's end against the next's start, the last's against the first's
        let mut uses = body.lp(l).coedges();
        let Some(first) = uses.next() else { continue };
        let (start, mut end) = (body.coedge_start(first), body.coedge_end(first));
        for c in uses {
            let e = body.edge(body.coedge(c).edge);
            let (from, to) = if body.coedge(c).forward {
                (e.start, e.end)
            } else {
                (e.end, e.start)
            };
            if from != end {
                return Some(format!("a face's boundary breaks at {}", at(end)));
            }
            end = to;
        }
        if end != start {
            return Some(format!("a face's boundary breaks at {}", at(end)));
        }
    }
    None
}

/// `check`, then every edge on its faces at 256 samples rather than 16.
pub fn certify(body: &Body) -> Vec<Problem> {
    if body.dead > 0 {
        return certify(&body.compact());
    }
    let mut out = check(body);
    if out
        .iter()
        .any(|p| matches!(p, Problem::EdgeUses { .. } | Problem::LoopBroken { .. }))
    {
        return out;
    }
    for e in body.edge_ids() {
        let mut faces: Vec<FaceId> = body
            .edge(e)
            .coedges
            .iter()
            .map(|&c| body.coedge_face(c))
            .collect();
        faces.dedup();
        let pts = body.edge_points(e, 256);
        for f in faces {
            let s = &body.face(f).surface;
            let worst = pts.iter().map(|&p| s.distance(p)).fold(0.0, f64::max);
            let known = |p: &Problem| matches!(p, Problem::EdgeOffFace { edge, face, .. } if *edge == e && *face == f);
            if worst > tol::LINEAR && !out.iter().any(known) {
                out.push(Problem::EdgeOffFace {
                    edge: e,
                    face: f,
                    distance: worst,
                });
            }
        }
    }
    out
}

fn check_shell_euler(body: &Body, out: &mut Vec<Problem>) {
    for (si, shell) in body.shells.iter().enumerate() {
        let (mut f, mut l) = (0i64, 0i64);
        let (mut edges, mut verts) = (Vec::new(), Vec::new());
        for &face in &shell.faces {
            f += 1;
            for &lp in &body.face(face).loops {
                l += 1;
                for c in body.lp(lp).coedges() {
                    let e = body.coedge(c).edge;
                    edges.push(e);
                    verts.extend([body.edge(e).start, body.edge(e).end]);
                }
            }
        }
        edges.sort();
        edges.dedup();
        verts.sort();
        verts.dedup();
        let value = verts.len() as i64 - edges.len() as i64 + 2 * f - l;
        if value % 2 != 0 || value > 2 {
            out.push(Problem::ShellEuler {
                shell: crate::body::ShellId(si as u32),
                value,
            });
        }
    }
}

/// Round each vertex, coedge to coedge: from one leaving it, back along its loop to the one
/// arriving, then across to that one's partner, which leaves it again. One cycle is one fan.
fn check_fans(body: &Body, out: &mut Vec<Problem>) {
    let mut prev = Vec::new();
    for (v, cs) in body.leaving().iter().enumerate() {
        let fans = body.fans(cs, &mut prev);
        if fans.len() > 1 {
            out.push(Problem::VertexFans {
                vertex: VertexId(v as u32),
                fans: fans.len(),
            });
        }
    }
}

fn check_edges(body: &Body, out: &mut Vec<Problem>) {
    for e in body.edge_ids() {
        let edge = body.edge(e);
        if edge.coedges.len() != 2 {
            out.push(Problem::EdgeUses {
                edge: e,
                uses: edge.coedges.len(),
            });
            continue;
        }
        let (a, b) = (body.coedge(edge.coedges[0]), body.coedge(edge.coedges[1]));
        if a.forward == b.forward {
            out.push(Problem::EdgeUsesSameWay { edge: e });
        }
        let closed = edge.start == edge.end;
        let degenerate = if closed {
            edge.curve
                .domain()
                .period
                .is_none_or(|p| (edge.range.width() - p).abs() > tol::ANGULAR * 10.0)
        } else {
            edge.curve
                .eval(edge.range.lo)
                .distance(edge.curve.eval(edge.range.hi))
                <= tol::LINEAR
        };
        if degenerate {
            out.push(Problem::EdgeDegenerate { edge: e });
        }
    }
}

fn check_loops(body: &Body, out: &mut Vec<Problem>) {
    for f in body.face_ids() {
        if body.face(f).loops.is_empty() {
            out.push(Problem::FaceNoLoops { face: f });
        }
    }
    for l in body.loop_ids() {
        let lp = body.lp(l);
        let n = lp.coedges().len();
        if n == 0 {
            out.push(Problem::LoopEmpty { lp: l });
            continue;
        }
        for (i, c) in lp.coedges().enumerate() {
            let next = lp.coedge((i + 1) % n);
            if body.coedge_end(c) != body.coedge_start(next) {
                out.push(Problem::LoopBroken { lp: l, after: c });
            }
        }
    }
}

fn check_geometry(body: &Body, out: &mut Vec<Problem>) {
    let band = tol::LINEAR;
    for e in body.edge_ids() {
        let edge = body.edge(e);
        for (v, t) in [(edge.start, edge.range.lo), (edge.end, edge.range.hi)] {
            let vtol = body.vertex(v).tol;
            let d = body.vertex(v).point.distance(edge.curve.eval(t));
            if d > band.max(vtol) {
                out.push(Problem::VertexOffEdge {
                    edge: e,
                    vertex: v,
                    distance: d,
                });
            }
            // (an edge this short is legal at the floor: rounding past it hides nothing more)
            if vtol > 1.1 * band && 2.0 * vtol >= edge_length(body, e) {
                out.push(Problem::VertexTolerance { vertex: v, tol: vtol });
            }
        }
        let mut faces: Vec<FaceId> = edge.coedges.iter().map(|&c| body.coedge_face(c)).collect();
        faces.dedup();
        for f in faces {
            let s = &body.face(f).surface;
            let points = body.edge_points(e, 16);
            let mut worst = 0.0f64;
            // each point from the one before, where it stays on the face that far; else the search
            let mut near = None;
            for &p in &points {
                let uv = near.map_or_else(|| s.project(p), |uv| s.project_near(p, uv));
                near = Some(uv);
                worst = worst.max(s.eval(uv).distance(p));
            }
            if worst > band {
                worst = points.iter().map(|&p| s.distance(p)).fold(0.0, f64::max);
            }
            if worst > band {
                out.push(Problem::EdgeOffFace {
                    edge: e,
                    face: f,
                    distance: worst,
                });
            }
        }
    }
}

/// At each coedge's midpoint, step a little to its left on the face (the
/// side the loop rule says is the face) and check the point is inside it.
fn check_sides(body: &Body, out: &mut Vec<Problem>) {
    for f in body.face_ids() {
        let loops = face_uv_loops_with(body, f, default_segments);
        // drawn finely when a probe misses: a chord of a long arc may cut across a sliver of face
        let fine = std::cell::OnceCell::new();
        let surface = &body.face(f).surface;
        if let forge_geom::Surface::Plane(_) = surface
            && loops.len() > 1
            && (crate::uv::signed_area(&loops[0].points) > 0.0) != body.face(f).same_sense
        {
            out.push(Problem::OuterNotFirst { face: f });
        }
        // probe a small fraction of the face's shortest edge so narrow faces are not stepped across
        let shortest = loops
            .iter()
            .flat_map(|l| l.coedges.iter())
            .map(|&c| edge_length(body, body.coedge(c).edge))
            .fold(f64::INFINITY, f64::min);
        let step = (0.05 * shortest).min(0.1);
        for ul in &loops {
            for (k, &c) in ul.coedges.iter().enumerate() {
                let co = body.coedge(c);
                let e = body.edge(co.edge);
                let t = e.range.mid();
                let d = e.curve.deriv(t);
                let along = if co.forward { d.d1 } else { -d.d1 };
                let mid_uv = ul.point_of_sample(ul.coedge_first[k] + default_segments(e) / 2);
                let Some(n) = body.face_normal(f, mid_uv) else {
                    continue;
                };
                let Some(inward) = n.cross(along).normalize() else {
                    continue;
                };
                // and well short of any other edge of the face (a hole may pass within a hair)
                let clearance = loops
                    .iter()
                    .flat_map(|l| l.coedges.iter())
                    .map(|&o| body.coedge(o).edge)
                    .filter(|&oe| oe != co.edge)
                    .map(|oe| {
                        let other = body.edge(oe);
                        other
                            .curve
                            .eval(other.curve.closest(d.p, other.range))
                            .distance(d.p)
                    })
                    .fold(f64::INFINITY, f64::min);
                let step = step.min(0.25 * clearance);
                let inside = |step: f64| {
                    let probe = surface.project(d.p + inward * step);
                    let probe = unwrap_near(body, f, probe, mid_uv);
                    point_in_loops(probe, &loops)
                        || point_in_loops(
                            probe,
                            fine.get_or_init(|| face_uv_loops_with(body, f, |e| 8 * default_segments(e))),
                        )
                };
                if ![1.0, 0.1, 0.01].iter().any(|&k| inside(k * step)) {
                    out.push(Problem::FaceWrongSide { face: f, coedge: c });
                }
            }
        }
    }
}

/// Fixed, irregular ray directions: one rarely grazes a model edge.
const RAYS: [[f64; 3]; 3] = [
    [0.5773, 0.6121, 0.5404],
    [-0.6311, 0.2347, 0.7393],
    [0.1823, -0.8741, 0.4502],
];

/// Shells must neither overlap nor nest wrongly: a point on an outer skin lies in no other
/// shell's material, a point on a void's skin in exactly one (the material round the void).
/// Each shell's material side is told by a ray leaving one of its faces outward: it crosses the
/// shell again an even number of times from an outer skin, an odd number from a void's. Points
/// on faces (beside each edge) are tested against the other shells by ray parity, counted only
/// where every ray agrees (point membership by ray parity, as in Requicha and Tilove's set
/// membership classification; a ray grazing an edge or face is not counted, so needs agreement).
/// The stretch of the ray from `line`'s origin along it (from a tolerance behind) inside `b`.
fn ray_in(line: &Line, b: &Box3) -> Option<Interval> {
    let (mut lo, mut hi) = (-tol::LINEAR, f64::INFINITY);
    let (o, d) = (line.origin, line.dir.get());
    for (oi, di, iv) in [(o.x, d.x, b.x), (o.y, d.y, b.y), (o.z, d.z, b.z)] {
        if di == 0.0 {
            if oi < iv.lo || oi > iv.hi {
                return None;
            }
            continue;
        }
        let (a, c) = ((iv.lo - oi) / di, (iv.hi - oi) / di);
        lo = lo.max(a.min(c));
        hi = hi.min(a.max(c));
    }
    (lo <= hi).then(|| Interval::new(lo, hi))
}

fn check_shells(body: &Body, out: &mut Vec<Problem>) {
    // drawn finely: a ray's hit near an edge must not fall the wrong side of its chords
    let loops: Vec<Vec<UvLoop>> = body
        .face_ids()
        .map(|f| face_uv_loops_with(body, f, |e| 8 * default_segments(e)))
        .collect();
    // the loops' parameter box: a point is moved by whole periods into it before the test
    let boxes: Vec<(Point2, Point2)> = loops
        .iter()
        .map(|ls| {
            ls.iter().flat_map(|l| l.points.iter()).fold(
                (
                    Point2::new(f64::INFINITY, f64::INFINITY),
                    Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
                ),
                |(lo, hi), q| {
                    (
                        Point2::new(lo.x.min(q.x), lo.y.min(q.y)),
                        Point2::new(hi.x.max(q.x), hi.y.max(q.y)),
                    )
                },
            )
        })
        .collect();
    let in_face = |f: FaceId, p: Point3| {
        let s = &body.face(f).surface;
        let uv = s.project(p);
        let d = s.domain();
        let (lo, hi) = boxes[f.index()];
        let shifts = |x: f64, lo: f64, hi: f64, per: Option<f64>| -> Vec<f64> {
            match per {
                Some(p) => {
                    let k0 = ((lo - x) / p).floor() as i64;
                    let k1 = ((hi - x) / p).ceil() as i64;
                    (k0..=k1).map(|k| x + k as f64 * p).collect()
                }
                None => vec![x],
            }
        };
        shifts(uv.x, lo.x, hi.x, d.u.period).iter().any(|&u| {
            shifts(uv.y, lo.y, hi.y, d.v.period)
                .iter()
                .any(|&v| point_in_loops(Point2::new(u, v), &loops[f.index()]))
        })
    };
    // a shell's crossings parity along a ray from p (on face `from`, or on none), or None where
    // it grazes a face or passes within the tolerance of p on another
    let parity = |shell: usize, p: Point3, from: Option<FaceId>, dir: Vec3| -> Option<bool> {
        let line = Line::new(p, dir.normalize()?);
        let mut odd = false;
        for &f in &body.shells[shell].faces {
            // a hit counts only in the face, so inside its box: only that stretch of the ray
            // is searched (a procedural surface's own box may be far bigger)
            if !body.face(f).alive {
                continue;
            }
            let Some(range) = ray_in(&line, &body.face_box(f).grow(tol::LINEAR)) else {
                continue;
            };
            let CurveSurface::Hits(hits) = curve_surface(&Curve::Line(line), range, &body.face(f).surface)
            else {
                return None;
            };
            for h in &hits {
                if h.t.abs() <= tol::LINEAR {
                    if Some(f) != from && in_face(f, h.point) {
                        return None;
                    }
                    continue;
                }
                if h.t < 0.0 {
                    continue;
                }
                if h.contact != Contact::Crossing {
                    return None;
                }
                if in_face(f, h.point) {
                    odd = !odd;
                }
            }
        }
        Some(odd)
    };
    let agreed = |each: &dyn Fn(Vec3) -> Option<bool>| -> Option<bool> {
        let got: Vec<bool> = RAYS
            .iter()
            .filter_map(|r| each(Vec3::new(r[0], r[1], r[2])))
            .collect();
        (!got.is_empty() && got.iter().all(|&g| g == got[0])).then(|| got[0])
    };
    // a point on face f a little in from coedge c at fraction `at` along it, and its normal there
    let beside = |f: FaceId, c: CoedgeId, at: f64| -> Option<(Point3, Vec3)> {
        let co = body.coedge(c);
        let e = body.edge(co.edge);
        let d = e.curve.deriv(e.range.lo + at * e.range.width());
        let along = if co.forward { d.d1 } else { -d.d1 };
        let surface = &body.face(f).surface;
        let n = body.face_normal(f, surface.project(d.p))?;
        let inward = n.cross(along).normalize()?;
        let clearance = loops[f.index()]
            .iter()
            .flat_map(|l| l.coedges.iter())
            .map(|&o| body.coedge(o).edge)
            .filter(|&oe| oe != co.edge)
            .map(|oe| {
                let other = body.edge(oe);
                other
                    .curve
                    .eval(other.curve.closest(d.p, other.range))
                    .distance(d.p)
            })
            .fold(f64::INFINITY, f64::min);
        let step = (0.2 * edge_length(body, co.edge)).min(1.0).min(0.25 * clearance);
        let q = surface.eval(surface.project(d.p + inward * step));
        let n = body.face_normal(f, surface.project(q))?;
        Some((q, n.get()))
    };
    let probes = |f: FaceId| {
        loops[f.index()]
            .iter()
            .flat_map(|l| l.coedges.iter())
            .flat_map(move |&c| [0.25, 0.5, 0.75].map(|at| (c, at)))
    };
    // +1 for an outer skin, -1 for a void's
    let sign: Vec<Option<i64>> = (0..body.shells.len())
        .map(|k| {
            body.shells[k]
                .faces
                .iter()
                .filter(|&&f| body.face(f).alive)
                .flat_map(|&f| probes(f).map(move |(c, at)| (f, c, at)))
                .find_map(|(f, c, at)| {
                    let (q, n) = beside(f, c, at)?;
                    let again =
                        agreed(&|r: Vec3| parity(k, q, Some(f), if r.dot(n) < 0.0 { -r } else { r }))?;
                    Some(if again { -1 } else { 1 })
                })
        })
        .collect();
    // a ray from outside a shell's box crosses it an even number of times
    let shell_box: Vec<Option<Box3>> = body
        .shells
        .iter()
        .map(|s| {
            s.faces
                .iter()
                .filter(|&&f| body.face(f).alive)
                .map(|&f| body.face_box(f))
                .reduce(|a, b| a.hull(&b))
                .map(|b| b.grow(tol::LINEAR))
        })
        .collect();
    for (k, shell) in body.shells.iter().enumerate() {
        let Some(own) = sign[k] else { continue };
        let want = i64::from(own < 0);
        for &f in shell.faces.iter().filter(|&&f| body.face(f).alive) {
            for (c, at) in probes(f) {
                let Some((q, _)) = beside(f, c, at) else { continue };
                let mut w = Some(0);
                for j in (0..body.shells.len()).filter(|&j| j != k) {
                    if shell_box[j].is_none_or(|b| !b.contains(q)) {
                        continue;
                    }
                    w = match (w, agreed(&|r: Vec3| parity(j, q, None, r)), sign[j]) {
                        (Some(w), Some(true), Some(s)) => Some(w + s),
                        (Some(w), Some(false), _) => Some(w),
                        _ => None,
                    };
                }
                if w.is_some_and(|w| w != want) {
                    out.push(Problem::ShellsOverlap { face: f });
                    return;
                }
            }
        }
    }
}

fn edge_length(body: &Body, e: EdgeId) -> f64 {
    let pts = body.edge_points(e, 16);
    pts.windows(2).map(|w| w[0].distance(w[1])).sum()
}

fn check_names(body: &Body, out: &mut Vec<Problem>) {
    let mut names: Vec<Name> = body
        .faces
        .iter()
        .map(|f| f.name)
        .chain(body.edges.iter().map(|e| e.name))
        .chain(body.vertices.iter().map(|v| v.name))
        .filter(|&n| n != Name::NONE)
        .collect();
    names.sort();
    for w in names.windows(2) {
        if w[0] == w[1] && out.last() != Some(&Problem::DuplicateName { name: w[0] }) {
            out.push(Problem::DuplicateName { name: w[0] });
        }
    }
}

fn check_euler(body: &Body, out: &mut Vec<Problem>) {
    let v = body.vertices.len() as i64;
    let e = body.edges.len() as i64;
    let f = body.faces.len() as i64;
    let l = body.loops.len() as i64;
    let value = v - e + 2 * f - l;
    if value % 2 != 0 || value > 2 * body.shells.len() as i64 {
        out.push(Problem::Euler { value });
    }
}

/// Genus (number of through-holes) of a valid single-shell body.
pub fn genus(body: &Body) -> i64 {
    if body.dead > 0 {
        return genus(&body.compact());
    }
    let value = body.vertices.len() as i64 - body.edges.len() as i64 + 2 * body.faces.len() as i64
        - body.loops.len() as i64;
    body.shells.len() as i64 - value / 2
}
