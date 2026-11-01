use crate::body::{Body, CoedgeId, EdgeId, FaceId, LoopId, ShellId, VertexId};
use crate::name::Name;
use crate::uv::{
    UvLoop, default_segments, face_uv_loops_with, loops_cross, loops_cross_steeply, point_in_loops,
    unwrap_near,
};
use forge_geom::intersect::{Contact, CurveSurface, curve_surface};
use forge_geom::{Curve, CurveGeom, Line, SurfaceGeom};
use forge_math::{Box3, Interval, Point2, Point3, Vec3, tol};

#[derive(Clone, Debug, PartialEq)]
pub enum Problem {
    /// Two of a face's boundary chords cross, in its parameters: the face's loops cross themselves or each other.
    LoopsCross {
        face: FaceId,
    },
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

impl Problem {
    /// The rule broken, as a short word for a list or a log.
    pub fn rule(&self) -> &'static str {
        match self {
            Problem::LoopsCross { .. } => "loops-cross",
            Problem::EdgeUses { .. } => "edge-uses",
            Problem::EdgeUsesSameWay { .. } => "edge-same-way",
            Problem::LoopEmpty { .. } => "loop-empty",
            Problem::LoopBroken { .. } => "loop-broken",
            Problem::VertexOffEdge { .. } => "vertex-off-edge",
            Problem::VertexTolerance { .. } => "vertex-tolerance",
            Problem::EdgeOffFace { .. } => "edge-off-face",
            Problem::EdgeDegenerate { .. } => "edge-degenerate",
            Problem::FaceWrongSide { .. } => "face-wrong-side",
            Problem::DuplicateName { .. } => "duplicate-name",
            Problem::Euler { .. } => "euler",
            Problem::FaceNoLoops { .. } => "face-no-loops",
            Problem::OuterNotFirst { .. } => "outer-not-first",
            Problem::ShellsOverlap { .. } => "shells-overlap",
            Problem::ShellEuler { .. } => "shell-euler",
            Problem::VertexFans { .. } => "vertex-fans",
        }
    }

    /// In a sentence, for a person.
    pub fn reason(&self) -> String {
        match self {
            Problem::LoopsCross { .. } => "a face's boundary crosses itself".to_string(),
            Problem::EdgeUses { uses, .. } => format!("an edge is used by {uses} faces, not 2"),
            Problem::EdgeUsesSameWay { .. } => "the two faces at an edge turn the same way".to_string(),
            Problem::LoopEmpty { .. } => "a face has an empty boundary".to_string(),
            Problem::LoopBroken { .. } => "a face's boundary has a gap".to_string(),
            Problem::VertexOffEdge { distance, .. } => {
                format!("a corner lies {distance:.2e} mm off its edge")
            }
            Problem::VertexTolerance { tol, .. } => {
                format!("a corner's tolerance of {tol:.2e} mm is as large as its edge")
            }
            Problem::EdgeOffFace { distance, .. } => format!("an edge lies {distance:.2e} mm off its face"),
            Problem::EdgeDegenerate { .. } => "an edge has no length".to_string(),
            Problem::FaceWrongSide { .. } => "a face lies on the wrong side of its boundary".to_string(),
            Problem::DuplicateName { .. } => "two parts of the body have the same name".to_string(),
            Problem::Euler { .. } | Problem::ShellEuler { .. } => {
                "the faces, edges and corners do not count up to a closed solid".to_string()
            }
            Problem::FaceNoLoops { .. } => "a face has no boundary".to_string(),
            Problem::OuterNotFirst { .. } => "a face's outer boundary is not listed first".to_string(),
            Problem::ShellsOverlap { .. } => "separate skins of the body overlap".to_string(),
            Problem::VertexFans { fans, .. } => format!("{fans} separate fans of faces meet at a corner"),
        }
    }

    /// The name of the face, edge or corner at fault in `body`, the body it was found in.
    pub fn entity(&self, body: &Body) -> Option<Entity> {
        let e = |e: EdgeId| Entity::Edge(body.edge(e).name);
        let f = |f: FaceId| Entity::Face(body.face(f).name);
        let v = |v: VertexId| Entity::Vertex(body.vertex(v).name);
        match *self {
            Problem::EdgeUses { edge, .. }
            | Problem::EdgeUsesSameWay { edge }
            | Problem::EdgeDegenerate { edge }
            | Problem::EdgeOffFace { edge, .. }
            | Problem::VertexOffEdge { edge, .. } => Some(e(edge)),
            Problem::LoopsCross { face }
            | Problem::FaceWrongSide { face, .. }
            | Problem::FaceNoLoops { face }
            | Problem::OuterNotFirst { face }
            | Problem::ShellsOverlap { face } => Some(f(face)),
            Problem::LoopEmpty { lp } | Problem::LoopBroken { lp, .. } => Some(f(body.lp(lp).face)),
            Problem::VertexTolerance { vertex, .. } | Problem::VertexFans { vertex, .. } => Some(v(vertex)),
            Problem::DuplicateName { name } => Some(Entity::Named(name)),
            Problem::Euler { .. } | Problem::ShellEuler { .. } => None,
        }
    }
}

/// What a problem is about, by its stable name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entity {
    Face(Name),
    Edge(Name),
    Vertex(Name),
    /// Whichever entities hold this name.
    Named(Name),
}

/// Everything wrong with a body, in a deterministic order. Empty means it is a
/// valid closed solid.
pub fn check(body: &Body) -> Vec<Problem> {
    if body.dead > 0 {
        return check(&body.compact());
    }
    check_known(body, &Known::default())
}

/// `check`, given that `old` passed it: an edge or face whose every input to a geometric test is
/// the same as an edge's or face's of `old` passes that test here too, so it is not run again.
pub fn check_against(body: &Body, old: &Body) -> Vec<Problem> {
    match (body.dead > 0, old.dead > 0) {
        (false, false) => {
            let local = check_known(body, &Known::new(body, old));
            if BOTH.load(std::sync::atomic::Ordering::Relaxed) {
                let (before, full) = (check(old), check(body));
                assert!(before.is_empty(), "the old body fails `check`: {before:?}");
                assert_eq!(local, full, "`check_against` differs from `check`");
            }
            local
        }
        (true, false) => check_against(&body.compact(), old),
        (_, true) => check_against(body, &old.compact()),
    }
}

static BOTH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// For tests: `check_against` also runs `check` on both bodies and panics unless the old body passes
/// and the two answers are the same.
pub fn check_against_both_ways(on: bool) {
    BOTH.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// The faces of `body` with an input to a test of `check` not the same as in `old`, found by comparing
/// the two: the only faces that can fail a test of one face, or of a pair, where `old` passed.
pub fn changed_faces(body: &Body, old: &Body) -> Vec<bool> {
    Known::new(body, old).faces.iter().map(|&k| !k).collect()
}

/// For tests: how many faces `check_against` compares loop by loop.
#[doc(hidden)]
pub fn faces_by_loop(body: &Body, old: &Body) -> usize {
    Known::new(&body.compact(), &old.compact())
        .quiet
        .iter()
        .flatten()
        .count()
}

/// Which of a body's edges and faces need no geometric test. Found by comparing with the old body,
/// never told by an operation; a name only says which old entity to compare with.
#[derive(Default)]
struct Known {
    edges: Vec<bool>,
    faces: Vec<bool>,
    /// A plane face whose loops changed only in places: per face (none if it is not so), per loop of it,
    /// whether the loop is as in `old`; and the box of the loops that are new or gone.
    quiet: Vec<Option<(Vec<bool>, Box3)>>,
    /// Where the material of a shell can differ from `old`'s: the box of every face changed or gone;
    /// a closed shell's change in material lies within the changes to its boundary. None: anywhere.
    changed: Option<Box3>,
}

impl Known {
    fn new(body: &Body, old: &Body) -> Known {
        let by = |names: &mut Vec<(Name, u32)>| names.sort_unstable();
        let mut old_edges: Vec<(Name, u32)> = old.edge_ids().map(|e| (old.edge(e).name, e.0)).collect();
        let mut old_faces: Vec<(Name, u32)> = old.face_ids().map(|f| (old.face(f).name, f.0)).collect();
        by(&mut old_edges);
        by(&mut old_faces);
        // the one entity of that name, none if it has no name or shares it
        let find = |list: &[(Name, u32)], n: Name| {
            let at = list.partition_point(|x| x.0 < n);
            (n != Name::NONE
                && list.get(at).is_some_and(|x| x.0 == n)
                && list.get(at + 1).is_none_or(|x| x.0 != n))
            .then(|| list[at].1)
        };
        let mut kept_old = vec![None; old.faces.len()];
        let faces: Vec<bool> = body
            .face_ids()
            .map(|f| {
                let o = find(&old_faces, body.face(f).name).filter(|&o| same_face(body, f, old, FaceId(o)));
                if let Some(o) = o {
                    kept_old[o as usize] = Some(f);
                }
                o.is_some()
            })
            .collect();
        let quiet = body
            .face_ids()
            .map(|f| {
                let o = find(&old_faces, body.face(f).name).filter(|_| !faces[f.index()])?;
                quiet_loops(body, f, old, FaceId(o))
            })
            .collect();
        // the unchanged faces of a shell must have shared a shell, and only with each other
        let mut to_new: Vec<Option<ShellId>> = vec![None; old.shells.len()];
        let mut to_old: Vec<Option<ShellId>> = vec![None; body.shells.len()];
        let mut same_shells = true;
        for (o, n) in kept_old.iter().enumerate() {
            let Some(n) = n else { continue };
            let (so, sn) = (old.faces[o].shell, body.face(*n).shell);
            same_shells &=
                *to_new[so.index()].get_or_insert(sn) == sn && *to_old[sn.index()].get_or_insert(so) == so;
        }
        let changed = same_shells.then(|| {
            let new = body
                .face_ids()
                .filter(|f| !faces[f.index()])
                .map(|f| body.face_box(f));
            let gone = old
                .face_ids()
                .filter(|f| kept_old[f.index()].is_none())
                .map(|f| old.face_box(f));
            new.chain(gone)
                .fold(Box3::EMPTY, |a, b| a.hull(&b))
                .grow(tol::LINEAR)
        });
        Known {
            edges: body
                .edge_ids()
                .map(|e| {
                    find(&old_edges, body.edge(e).name).is_some_and(|o| same_edge(body, e, old, EdgeId(o)))
                })
                .collect(),
            faces,
            quiet,
            changed,
        }
    }
    fn edge(&self, e: EdgeId) -> bool {
        self.edges.get(e.index()).is_some_and(|&k| k)
    }
    fn face(&self, f: FaceId) -> bool {
        self.faces.get(f.index()).is_some_and(|&k| k)
    }
}

/// An edge's curve, its stretch of it and its end points: all a face's side test reads of an edge.
fn same_span(a: &Body, e: EdgeId, b: &Body, o: EdgeId) -> bool {
    let (x, y) = (a.edge(e), b.edge(o));
    x.range == y.range
        && a.vertex(x.start).point == b.vertex(y.start).point
        && a.vertex(x.end).point == b.vertex(y.end).point
        && x.curve == y.curve
}

/// All `check_geometry` reads of an edge: its span, its ends' tolerances and its faces' surfaces.
fn same_edge(a: &Body, e: EdgeId, b: &Body, o: EdgeId) -> bool {
    let (x, y) = (a.edge(e), b.edge(o));
    same_span(a, e, b, o)
        && a.vertex(x.start).tol == b.vertex(y.start).tol
        && a.vertex(x.end).tol == b.vertex(y.end).tol
        && x.coedges.len() == y.coedges.len()
        && x.coedges.iter().zip(y.coedges.iter()).all(|(&c, &d)| {
            a.face(a.coedge_face(c)).surface == b.face(b.coedge_face(d)).surface
                && (a.coedge_face(c) == a.coedge_face(x.coedges[0]))
                    == (b.coedge_face(d) == b.coedge_face(y.coedges[0]))
        })
}

/// All `check_sides` reads of a face: its surface and sense and its loops, coedge by coedge in order.
fn same_face(a: &Body, f: FaceId, b: &Body, o: FaceId) -> bool {
    let (x, y) = (a.face(f), b.face(o));
    x.same_sense == y.same_sense
        && x.surface == y.surface
        && x.loops.len() == y.loops.len()
        && x.loops.iter().zip(y.loops.iter()).all(|(&l, &m)| {
            let (l, m) = (a.lp(l), b.lp(m));
            l.coedges().len() == m.coedges().len()
                && l.coedges().zip(m.coedges()).all(|(c, d)| {
                    let (c, d) = (a.coedge(c), b.coedge(d));
                    c.forward == d.forward && same_span(a, c.edge, b, d.edge)
                })
        })
}

/// Face `f` as `quiet` says, if it is. A side test of a kept loop's coedge is the same as in `old` when
/// the new and gone loops are far enough off (`check_sides`) and their edges at least 2 long: the step is
/// at most 0.1 and only edges under 2 long set it below that.
fn quiet_loops(a: &Body, f: FaceId, b: &Body, o: FaceId) -> Option<(Vec<bool>, Box3)> {
    let (x, y) = (a.face(f), b.face(o));
    if !matches!(x.surface, forge_geom::Surface::Plane(_))
        || x.same_sense != y.same_sense
        || x.surface != y.surface
    {
        return None;
    }
    let first = |body: &Body, l: LoopId| {
        body.lp(l)
            .coedges()
            .next()
            .map(|c| body.edge(body.coedge(c).edge).name)
    };
    let same = |l: LoopId, m: LoopId| {
        let (l, m) = (a.lp(l), b.lp(m));
        l.coedges().len() == m.coedges().len()
            && l.coedges().zip(m.coedges()).all(|(c, d)| {
                let (c, d) = (a.coedge(c), b.coedge(d));
                c.forward == d.forward && same_span(a, c.edge, b, d.edge)
            })
    };
    let mut olds: Vec<(Option<Name>, LoopId, bool)> =
        y.loops.iter().map(|&m| (first(b, m), m, false)).collect();
    olds.sort_unstable_by_key(|o| (o.0, o.1));
    let kept: Vec<bool> = x
        .loops
        .iter()
        .map(|&l| {
            let n = first(a, l);
            let at = olds.partition_point(|o| o.0 < n);
            let hit = olds[at..]
                .iter()
                .take_while(|o| o.0 == n)
                .position(|o| !o.2 && same(l, o.1));
            if let Some(k) = hit {
                olds[at + k].2 = true;
            }
            hit.is_some()
        })
        .collect();
    let mut fresh = Box3::EMPTY;
    let mut take = |body: &Body, l: LoopId| {
        for c in body.lp(l).coedges() {
            let e = body.coedge(c).edge;
            if edge_length(body, e) < 2.0 {
                return false;
            }
            fresh = fresh.hull(&body.edge_box(e));
        }
        true
    };
    let new = x.loops.iter().zip(&kept).filter(|k| !k.1).all(|k| take(a, *k.0));
    let gone = olds.iter().filter(|o| !o.2).all(|o| take(b, o.1));
    (new && gone).then_some((kept, fresh))
}

fn check_known(body: &Body, known: &Known) -> Vec<Problem> {
    let mut out = Vec::new();
    check_edges(body, &mut out);
    check_loops(body, &mut out);
    check_geometry(body, known, &mut out);
    if out.is_empty() {
        check_sides(body, known, &mut out);
    }
    if out.is_empty() && body.shells.len() > 1 {
        check_shells(body, known, &mut out);
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
            if worst > body.edge(e).tol.max(tol::LINEAR) && !out.iter().any(known) {
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

fn check_geometry(body: &Body, known: &Known, out: &mut Vec<Problem>) {
    for e in body.edge_ids().filter(|&e| !known.edge(e)) {
        let edge = body.edge(e);
        // what the edge was read with, if it came from a file (at least the tolerance)
        let band = edge.tol.max(tol::LINEAR);
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
fn check_sides(body: &Body, known: &Known, out: &mut Vec<Problem>) {
    let faces: Vec<FaceId> = body.face_ids().filter(|&f| !known.face(f)).collect();
    check_sides_of(body, &faces, Some(known), out);
}

/// Whether each of `faces` lies on the left of its loops: what `check` asks of them.
pub fn sides_of(body: &Body, faces: &[FaceId]) -> Vec<Problem> {
    let mut out = Vec::new();
    check_sides_of(body, faces, None, &mut out);
    out
}

fn check_sides_of(body: &Body, faces: &[FaceId], known: Option<&Known>, out: &mut Vec<Problem>) {
    for &f in faces {
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
        // drawn finely before it is believed: two chords of curved loops close by may cross where the loops do not
        if loops_cross(&loops)
            && loops_cross_steeply(
                fine.get_or_init(|| face_uv_loops_with(body, f, |e| 8 * default_segments(e))),
                &|uv| surface.eval(uv),
            )
        {
            out.push(Problem::LoopsCross { face: f });
        }
        let step = (0.05 * shortest).min(0.1);
        let boxes = loop_boxes(&loops);
        let in_loops = |p: Point2| in_boxed_loops(p, &loops, &boxes);
        let quiet = known
            .and_then(|k| k.quiet.get(f.index()))
            .and_then(|q| q.as_ref());
        for (li, ul) in loops.iter().enumerate() {
            for (k, &c) in ul.coedges.iter().enumerate() {
                let co = body.coedge(c);
                let e = body.edge(co.edge);
                // as it was in `old`: every probe at most 0.1 off the edge, its step set by nothing new
                if let Some((kept, fresh)) = quiet
                    && kept[li]
                    && box_gap(body.edge_box(co.edge), *fresh) >= 0.4
                {
                    continue;
                }
                let t = e.range.mid();
                let d = e.curve.deriv(t);
                let d0 = d.p;
                let along = if co.forward { d.d1 } else { -d.d1 };
                let mid_uv = ul.point_of_sample(ul.coedge_first[k] + default_segments(e) / 2);
                let Some(n) = body.face_normal(f, mid_uv) else {
                    continue;
                };
                let Some(inward) = n.cross(along).normalize() else {
                    continue;
                };
                // and well short of any other edge of the face (a hole may pass within a hair): one
                // whose box is 4 steps off leaves the step as it is
                let near = |b: Box3| box_distance(b, d0) < 4.0 * step;
                let clearance = loops
                    .iter()
                    .flat_map(|l| l.coedges.iter())
                    .map(|&o| body.coedge(o).edge)
                    .filter(|&oe| oe != co.edge && near(body.edge_box(oe)))
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
                    in_loops(probe)
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

/// Each loop's box in parameters.
fn loop_boxes(loops: &[UvLoop]) -> Vec<(Point2, Point2)> {
    loops
        .iter()
        .map(|l| {
            l.points.iter().fold(
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
        .collect()
}

/// `point_in_loops`: a loop whose box the point is outside of adds nothing to the parity.
fn in_boxed_loops(p: Point2, loops: &[UvLoop], boxes: &[(Point2, Point2)]) -> bool {
    let mut inside = false;
    for (l, (lo, hi)) in loops.iter().zip(boxes) {
        if p.x >= lo.x && p.x <= hi.x && p.y >= lo.y && p.y <= hi.y {
            inside ^= forge_math::predicates::inside_polygon(p, &l.points);
        }
    }
    inside
}

/// How far apart two boxes are.
fn box_gap(a: Box3, b: Box3) -> f64 {
    let d = |x: Interval, y: Interval| (y.lo - x.hi).max(x.lo - y.hi).max(0.0);
    Vec3::new(d(a.x, b.x), d(a.y, b.y), d(a.z, b.z)).length()
}

/// How far `p` lies from box `b`.
fn box_distance(b: Box3, p: Point3) -> f64 {
    let d = |x: f64, i: Interval| (i.lo - x).max(x - i.hi).max(0.0);
    Vec3::new(d(p.x, b.x), d(p.y, b.y), d(p.z, b.z)).length()
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

fn check_shells(body: &Body, known: &Known, out: &mut Vec<Problem>) {
    // drawn finely, on first asking: a ray's hit near an edge must not fall the wrong side of its
    // chords; with the loops' parameter box: a point is moved by whole periods into it before the test
    type Drawn = (Vec<UvLoop>, Vec<(Point2, Point2)>, (Point2, Point2));
    let drawn: Vec<std::cell::OnceCell<Drawn>> = vec![std::cell::OnceCell::new(); body.faces.len()];
    let loops_of = |f: FaceId| {
        drawn[f.index()].get_or_init(|| {
            let ls = face_uv_loops_with(body, f, |e| 8 * default_segments(e));
            let boxes = loop_boxes(&ls);
            let all = boxes.iter().fold(
                (
                    Point2::new(f64::INFINITY, f64::INFINITY),
                    Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
                ),
                |(lo, hi), b| {
                    (
                        Point2::new(lo.x.min(b.0.x), lo.y.min(b.0.y)),
                        Point2::new(hi.x.max(b.1.x), hi.y.max(b.1.y)),
                    )
                },
            );
            (ls, boxes, all)
        })
    };
    let in_face = |f: FaceId, p: Point3| {
        let s = &body.face(f).surface;
        let uv = s.project(p);
        let d = s.domain();
        let (ls, each, (lo, hi)) = loops_of(f);
        let (lo, hi) = (*lo, *hi);
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
                .any(|&v| in_boxed_loops(Point2::new(u, v), ls, each))
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
    let coedges_of = |f: FaceId| body.face(f).loops.iter().flat_map(|&l| body.lp(l).coedges());
    // a point on face f a little in from coedge c at fraction `at` along it, and its normal there
    let beside = |f: FaceId, c: CoedgeId, at: f64| -> Option<(Point3, Vec3)> {
        let co = body.coedge(c);
        let e = body.edge(co.edge);
        let d = e.curve.deriv(e.range.lo + at * e.range.width());
        let along = if co.forward { d.d1 } else { -d.d1 };
        let surface = &body.face(f).surface;
        let n = body.face_normal(f, surface.project(d.p))?;
        let inward = n.cross(along).normalize()?;
        let step = (0.2 * edge_length(body, co.edge)).min(1.0);
        // an edge whose box is 4 steps off leaves the step as it is
        let near = |b: Box3| box_distance(b, d.p) < 4.0 * step;
        let clearance = coedges_of(f)
            .map(|o| body.coedge(o).edge)
            .filter(|&oe| oe != co.edge && near(body.edge_box(oe)))
            .map(|oe| {
                let other = body.edge(oe);
                other
                    .curve
                    .eval(other.curve.closest(d.p, other.range))
                    .distance(d.p)
            })
            .fold(f64::INFINITY, f64::min);
        let step = step.min(0.25 * clearance);
        let q = surface.eval(surface.project(d.p + inward * step));
        let n = body.face_normal(f, surface.project(q))?;
        Some((q, n.get()))
    };
    let probes = |f: FaceId| coedges_of(f).flat_map(move |c| [0.25, 0.5, 0.75].map(|at| (c, at)));
    // +1 for an outer skin, -1 for a void's, on first asking
    let signs: Vec<std::cell::OnceCell<Option<i64>>> = vec![std::cell::OnceCell::new(); body.shells.len()];
    let sign = |k: usize| {
        *signs[k].get_or_init(|| {
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
    };
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
    // a point on an unchanged face, outside where any material changed, is as it was
    let asked = |f: FaceId| {
        !known.face(f)
            || match known.changed {
                Some(r) => r.overlaps(&body.face_box(f)),
                None => true,
            }
    };
    for (k, shell) in body.shells.iter().enumerate() {
        for &f in shell.faces.iter().filter(|&&f| body.face(f).alive && asked(f)) {
            let Some(own) = sign(k) else { break };
            let want = i64::from(own < 0);
            // a probe is the nearest point to one at most 1 off an edge, so at most 2 from it: an outer
            // skin's face clear of every other shell's box by that is out of them all
            let reach = body.face_box(f).grow(2.0 + tol::LINEAR);
            let clear = !matches!(body.face(f).surface, forge_geom::Surface::Procedural(_))
                && (0..body.shells.len())
                    .filter(|&j| j != k)
                    .all(|j| shell_box[j].is_none_or(|b| !b.overlaps(&reach)));
            if want == 0 && clear {
                continue;
            }
            for (c, at) in probes(f) {
                let Some((q, _)) = beside(f, c, at) else { continue };
                let mut w = Some(0);
                for j in (0..body.shells.len()).filter(|&j| j != k) {
                    if shell_box[j].is_none_or(|b| !b.contains(q)) {
                        continue;
                    }
                    w = match (w, agreed(&|r: Vec3| parity(j, q, None, r))) {
                        (Some(w), Some(true)) => sign(j).map(|s| w + s),
                        (Some(w), Some(false)) => Some(w),
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
