//! `forge_topo::check`'s combinatorial part, tied to its Lean model (`lean/`).
//!
//! - `encode` writes a body's topology and corner tables in the model's input format (`lean/Main.lean`).
//! - `combinatorial` keeps the problems the model decides, as the model prints them.
//! - `back_links` tests `WF` (`lean/TopoCheck/Spec.lean`): what the proof takes as given and
//!   `check` never tests.
//! - `mutants` breaks a body's topology in small ways, for the differential test.

use forge_math::rng::Rng;
use forge_topo::body::{Body, CoedgeId, EdgeId, FaceId, LoopId, ShellId, VertexId};
use forge_topo::check::Problem;
use std::fmt::Write;

/// The body `check` looks at: compacted if it has dead entities (check.rs:181–183).
pub fn as_checked(b: &Body) -> Body {
    if b.dead > 0 { b.compact() } else { b.clone() }
}

/// The model's input for `b` (`lean/Main.lean`): its topology, by index.
pub fn encode(b: &Body, out: &mut String) {
    let _ = writeln!(out, "body {}", b.vertices.len());
    for e in &b.edges {
        let _ = write!(out, "E {} {} {}", e.start.0, e.end.0, e.coedges.len());
        for c in e.coedges.iter() {
            let _ = write!(out, " {}", c.0);
        }
        out.push('\n');
    }
    for c in &b.coedges {
        let _ = writeln!(out, "C {} {} {}", c.edge.0, c.forward as u8, c.lp.0);
    }
    for (i, l) in b.loops.iter().enumerate() {
        let first = l.coedges().next().map_or(0, |c| c.0);
        let _ = writeln!(out, "L {} {} {}", first, l.coedges().len(), l.face.0);
        debug_assert!(l.coedges().len() == 0 || b.loops[i].coedge(0).0 == first);
    }
    for f in &b.faces {
        let _ = write!(out, "F {} {}", f.shell.0, f.loops.len());
        for l in f.loops.iter() {
            let _ = write!(out, " {}", l.0);
        }
        out.push('\n');
    }
    for s in &b.shells {
        let _ = write!(out, "S {}", s.faces.len());
        for f in &s.faces {
            let _ = write!(out, " {}", f.0);
        }
        out.push('\n');
    }
    encode_corners(b, out);
    out.push_str("end\n");
}

/// The problems of `check`'s answer that the model decides, as `lean/Main.lean` prints them, in
/// `check`'s order.
pub fn combinatorial(problems: &[Problem]) -> Vec<String> {
    problems
        .iter()
        .filter_map(|p| match p {
            Problem::EdgeUses { edge, uses } => Some(format!("edge-uses {} {uses}", edge.0)),
            Problem::EdgeUsesSameWay { edge } => Some(format!("edge-same-way {}", edge.0)),
            Problem::LoopEmpty { lp } => Some(format!("loop-empty {}", lp.0)),
            Problem::LoopBroken { lp, after } => Some(format!("loop-broken {} {}", lp.0, after.0)),
            Problem::FaceNoLoops { face } => Some(format!("face-no-loops {}", face.0)),
            Problem::ShellEuler { shell, value } => Some(format!("shell-euler {} {value}", shell.0)),
            Problem::VertexFans { vertex, fans } => Some(format!("vertex-fans {} {fans}", vertex.0)),
            Problem::BackLink { what, id } => Some(format!("back-link {what} {id}")),
            Problem::ShellFaces { shell } => Some(format!("shell-faces {}", shell.0)),
            Problem::ShellDisconnected { shell } => Some(format!("shell-disconnected {}", shell.0)),
            Problem::StrayVertex { vertex } => Some(format!("stray-vertex {}", vertex.0)),
            _ => None,
        })
        .collect()
}

/// `WF` of `lean/TopoCheck/Spec.lean`: an edge's uses are coedges of that edge, each in the loop it
/// names; a loop's coedges name that loop, and each is among its edge's uses. The first failure.
pub fn back_links(b: &Body) -> Result<(), String> {
    let in_loop = |c: CoedgeId, l: LoopId| {
        b.loops
            .get(l.index())
            .is_some_and(|lp| lp.coedges().any(|x| x == c))
    };
    for (e, edge) in b.edges.iter().enumerate() {
        for &c in edge.coedges.iter() {
            let Some(co) = b.coedges.get(c.index()) else {
                return Err(format!("edge {e}: use {} is no coedge", c.0));
            };
            if co.edge != EdgeId(e as u32) {
                return Err(format!("edge {e}: use {} is of edge {}", c.0, co.edge.0));
            }
            if !in_loop(c, co.lp) {
                return Err(format!("coedge {}: not in its loop {}", c.0, co.lp.0));
            }
        }
    }
    for (l, lp) in b.loops.iter().enumerate() {
        for c in lp.coedges() {
            let Some(co) = b.coedges.get(c.index()) else {
                return Err(format!("loop {l}: coedge {} is none", c.0));
            };
            if co.lp != LoopId(l as u32) {
                return Err(format!("loop {l}: coedge {} names loop {}", c.0, co.lp.0));
            }
            if !b
                .edges
                .get(co.edge.index())
                .is_some_and(|e| e.coedges.contains(&c))
            {
                return Err(format!(
                    "loop {l}: coedge {} is not a use of its edge {}",
                    c.0, co.edge.0
                ));
            }
        }
    }
    Ok(())
}

/// Small breaks of `b`'s topology (compact, every entity alive), each a whole body.
pub fn mutants(b: &Body, r: &mut Rng) -> Vec<(String, Body)> {
    let mut out = Vec::new();
    let pick = |r: &mut Rng, n: usize| (r.next_u64() % n.max(1) as u64) as usize;
    let (nv, ne, nc, ns) = (b.vertices.len(), b.edges.len(), b.coedges.len(), b.shells.len());
    if nc > 0 {
        let c = pick(r, nc);
        let mut m = b.clone();
        m.coedges[c].forward = !m.coedges[c].forward;
        out.push((format!("flip coedge {c}"), m));
    }
    if ne > 0 {
        let e = pick(r, ne);
        let mut m = b.clone();
        let x = &mut m.edges[e];
        std::mem::swap(&mut x.start, &mut x.end);
        out.push((format!("swap ends of edge {e}"), m));
        let e = pick(r, ne);
        let mut m = b.clone();
        let first = m.edges[e].coedges.iter().next().copied();
        m.edges[e].coedges.retain(|&c| Some(c) != first);
        out.push((format!("drop a use of edge {e}"), m));
        let e = pick(r, ne);
        let mut m = b.clone();
        if let Some(&c) = m.edges[e].coedges.iter().next() {
            m.edges[e].coedges.push(c);
        }
        out.push((format!("repeat a use of edge {e}"), m));
    }
    if ne > 0 && nv > 1 {
        let (e, v) = (pick(r, ne), pick(r, nv));
        let mut m = b.clone();
        m.edges[e].end = forge_topo::body::VertexId(v as u32);
        out.push((format!("end edge {e} at vertex {v}"), m));
    }
    if nv > 1 {
        // two vertices made one: loops still close and edges keep their uses, but the vertex is
        // pinched, its faces in two fans
        let (v, w) = (pick(r, nv) as u32, pick(r, nv) as u32);
        let mut m = b.clone();
        for e in m.edges.iter_mut() {
            for x in [&mut e.start, &mut e.end] {
                if x.0 == w {
                    x.0 = v;
                }
            }
        }
        out.push((format!("merge vertex {w} into {v}"), m));
    }
    if ne > 1 {
        // two edges trade uses: each loop still walks the same vertices, the fans do not
        let (e, f) = (pick(r, ne), pick(r, ne));
        let mut m = b.clone();
        let (x, y) = (
            m.edges[e].coedges.iter().next().copied(),
            m.edges[f].coedges.iter().next().copied(),
        );
        if let (Some(x), Some(y)) = (x, y)
            && e != f
        {
            m.edges[e].coedges.retain(|&c| c != x);
            m.edges[e].coedges.push(y);
            m.edges[f].coedges.retain(|&c| c != y);
            m.edges[f].coedges.push(x);
            out.push((format!("trade uses of edges {e} and {f}"), m));
        }
    }
    if ns > 1 {
        let mut m = b.clone();
        let moved = std::mem::take(&mut m.shells[ns - 1].faces);
        for &f in &moved {
            m.faces[f.index()].shell = ShellId(0);
        }
        m.shells[0].faces.extend(moved);
        m.shells.pop();
        out.push(("merge the last shell into the first".to_string(), m));
    }
    {
        let mut m = b.clone();
        m.add_vertex(forge_math::Point3::new(1e3, 1e3, 1e3), forge_topo::Name::NONE);
        out.push(("a vertex on no edge".to_string(), m));
    }
    if ns > 0 && !b.shells[0].faces.is_empty() {
        let mut m = b.clone();
        let k = pick(r, m.shells[0].faces.len());
        m.shells[0].faces.remove(k);
        out.push((format!("drop face {k} from shell 0's list"), m));
    }
    if nc > 1 {
        let (c, d) = (pick(r, nc), pick(r, nc));
        let mut m = b.clone();
        m.coedges[c].lp = b.coedges[d].lp;
        out.push((format!("coedge {c} names coedge {d}'s loop"), m));
    }
    if !b.faces.is_empty() {
        let f = pick(r, b.faces.len());
        let mut m = b.clone();
        let last = m.faces[f].loops.iter().last().copied();
        m.faces[f].loops.retain(|&l| Some(l) != last);
        out.push((format!("drop a loop of face {f} from its list"), m));
    }
    out
}

/// `Body::corners` (body.rs, private there), transcribed: the corners of face `f` at
/// vertex `v` where it meets `v` more than once, each coedge leaving `v` with the arriving one that
/// closes its corner, by the angle round the face normal a vertex tolerance out along each edge.
pub fn corners(b: &Body, f: FaceId, v: VertexId) -> Vec<(CoedgeId, CoedgeId)> {
    use forge_geom::{CurveGeom, SurfaceGeom};
    let at = b.vertex(v);
    let uses: Vec<CoedgeId> = b
        .face(f)
        .loops
        .iter()
        .flat_map(|&l| b.lp(l).coedges())
        .filter(|&c| b.coedge_start(c) == v || b.coedge_end(c) == v)
        .collect();
    if uses.iter().filter(|&&c| b.coedge_start(c) == v).count() < 2 {
        return Vec::new();
    }
    let surface = &b.face(f).surface;
    let Some(n) = b.face_normal(f, surface.project(at.point)) else {
        return Vec::new();
    };
    let e1 = n.any_perpendicular();
    let e2 = n.cross(e1);
    let reach = at.tol.max(forge_math::tol::LINEAR);
    let angle = |c: CoedgeId, leaving: bool| {
        let co = b.coedge(c);
        let edge = b.edge(co.edge);
        let at_start = leaving == co.forward;
        let (t, sign) = if at_start {
            (edge.range.lo, 1.0)
        } else {
            (edge.range.hi, -1.0)
        };
        let d = edge.curve.deriv(t);
        let dt = sign * reach / d.d1.length().max(f64::MIN_POSITIVE);
        let off = d.d1 * dt + d.d2 * (0.5 * dt * dt);
        forge_math::dmath::atan2(e2.dot(off), e1.dot(off))
    };
    let arriving: Vec<(CoedgeId, f64)> = uses
        .iter()
        .filter(|&&c| b.coedge_end(c) == v)
        .map(|&c| (c, angle(c, false)))
        .collect();
    let tau = forge_math::dmath::TAU;
    uses.iter()
        .filter(|&&c| b.coedge_start(c) == v)
        .filter_map(|&c| {
            let a = angle(c, true);
            let own = b.coedge(c).edge;
            arriving
                .iter()
                .map(|&(x, t)| {
                    (
                        x,
                        if b.coedge(x).edge == own {
                            tau
                        } else {
                            (t - a).rem_euclid(tau)
                        },
                    )
                })
                .filter(|x| x.0 != c)
                .min_by(|x, y| x.1.total_cmp(&y.1))
                .map(|x| (c, x.0))
        })
        .collect()
}

/// Every non-empty corner table of `b`, as the model's `K` records: per face and vertex it meets
/// with two coedges leaving or more.
pub fn encode_corners(b: &Body, out: &mut String) {
    for (fi, f) in b.faces.iter().enumerate() {
        let mut vs: Vec<VertexId> = f
            .loops
            .iter()
            .flat_map(|&l| b.lp(l).coedges())
            .map(|c| b.coedge_start(c))
            .collect();
        vs.sort();
        vs.dedup();
        for v in vs {
            let k = corners(b, FaceId(fi as u32), v);
            if k.is_empty() {
                continue;
            }
            let _ = write!(out, "K {fi} {} {}", v.0, k.len());
            for (a, c) in k {
                let _ = write!(out, " {} {}", a.0, c.0);
            }
            out.push('\n');
        }
    }
}
