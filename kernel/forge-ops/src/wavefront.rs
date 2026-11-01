//! A body's faces moved to new surfaces can no longer meet as they did: an edge shrinks away and its
//! neighbours meet instead, or a vertex of four faces, no longer on one point, parts in two. These are
//! the local events of a moving wavefront (straight skeletons; Forsyth 1995 on shelling): each changes
//! the topology to what the moved faces want and the rebuild goes on. research/shell-topology
use crate::rebuild::{RebuildError, rebuild, solve_point};
use forge_geom::{Curve, Line, Surface};
use forge_math::{Interval, Point3};
use forge_topo::{Body, FaceId, Name, VertexId};

type NetEdge = (Curve, Interval, usize, usize, Name);
type NetFace = (FaceId, Vec<Vec<(usize, bool)>>);

/// Edges and loops as plain lists, to change round a vertex and build again.
#[derive(Clone)]
struct Net {
    verts: Vec<(Point3, Name)>,
    edges: Vec<NetEdge>,
    faces: Vec<NetFace>,
}

fn net(b: &Body) -> Net {
    Net {
        verts: b.vertices.iter().map(|v| (v.point, v.name)).collect(),
        edges: b
            .edges
            .iter()
            .map(|e| (e.curve.clone(), e.range, e.start.index(), e.end.index(), e.name))
            .collect(),
        faces: b
            .face_ids()
            .map(|f| {
                let loops = b
                    .face(f)
                    .loops
                    .iter()
                    .map(|&l| {
                        b.lp(l)
                            .coedges()
                            .map(|c| (b.coedge(c).edge.index(), b.coedge(c).forward))
                            .collect()
                    })
                    .collect();
                (f, loops)
            })
            .collect(),
    }
}

fn built(b: &Body, n: &Net) -> Body {
    let mut out = Body::new();
    out.joins = b.joins.clone();
    for _ in &b.shells {
        out.add_shell();
    }
    let vs: Vec<VertexId> = n.verts.iter().map(|v| out.add_vertex(v.0, v.1)).collect();
    let mut used = vec![false; n.edges.len()];
    for (_, loops) in &n.faces {
        for &(e, _) in loops.iter().flatten() {
            used[e] = true;
        }
    }
    let es: Vec<_> = n
        .edges
        .iter()
        .enumerate()
        .map(|(i, e)| used[i].then(|| out.add_edge(e.0.clone(), e.1, vs[e.2], vs[e.3], e.4)))
        .collect();
    for (f, loops) in &n.faces {
        let face = b.face(*f);
        let nf = out.add_face(face.shell, face.surface.clone(), face.same_sense, face.name);
        for l in loops {
            let uses: Vec<_> = l.iter().map(|&(e, fw)| (es[e].expect("used"), fw)).collect();
            out.add_loop(nf, &uses);
        }
    }
    out.into_compact()
}

fn head(n: &Net, u: (usize, bool)) -> usize {
    if u.1 { n.edges[u.0].3 } else { n.edges[u.0].2 }
}

/// The corners at vertex `v`: (face in the net, loop, index of the use arriving at `v`).
fn corners(n: &Net, v: usize) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    for (i, (_, loops)) in n.faces.iter().enumerate() {
        for (j, l) in loops.iter().enumerate() {
            for (k, &u) in l.iter().enumerate() {
                if head(n, u) == v {
                    out.push((i, j, k));
                }
            }
        }
    }
    out
}

fn surface_of<'a>(b: &'a Body, n: &Net, face: usize) -> &'a Surface {
    &b.face(n.faces[face].0).surface
}

/// The net with vertex `v`, of four corners, parted in two joined by a new edge between the faces of
/// corners `k` and `k + 2`: the other two faces each keep their corner, on one vertex or the other.
fn split(b: &Body, n: &Net, v: usize, k: usize) -> Option<Net> {
    let cs = corners(n, v);
    if cs.len() != 4 {
        return None;
    }
    // round the vertex: a corner's leaving edge is the next corner's arriving edge
    let next = |c: (usize, usize, usize)| {
        let l = &n.faces[c.0].1[c.1];
        let out = l[(c.2 + 1) % l.len()].0;
        cs.iter()
            .copied()
            .find(|&d| n.faces[d.0].1[d.1][d.2].0 == out && d != c)
    };
    let mut ring = vec![cs[0]];
    while ring.len() < 4 {
        let c = next(*ring.last()?)?;
        if ring.contains(&c) {
            return None;
        }
        ring.push(c);
    }
    let at = |i: usize| ring[(k + i) % 4];
    let (a, x, c, y) = (at(0), at(1), at(2), at(3));
    // x's corner goes with one new vertex, y's with the other
    let solve = |faces: [usize; 3]| {
        let cons: Vec<Surface> = faces.iter().map(|&f| surface_of(b, n, f).clone()).collect();
        solve_point(&cons, n.verts[v].0)
    };
    let (px, py) = (solve([a.0, x.0, c.0])?, solve([c.0, y.0, a.0])?);
    let line = Line::through(px, py)?;
    let mut m = n.clone();
    let name = n.verts[v].1;
    let (vx, vy) = (m.verts.len(), m.verts.len() + 1);
    m.verts.push((px, name.with_str("part")));
    m.verts.push((py, name.with_str("part").with(1)));
    let f = m.edges.len();
    m.edges.push((
        Curve::Line(line),
        Interval::new(0.0, px.distance(py)),
        vx,
        vy,
        name.with_str("parted"),
    ));
    // the edges between corners go to the vertex on their side
    let edge_after = |c: (usize, usize, usize)| {
        let l = &n.faces[c.0].1[c.1];
        l[(c.2 + 1) % l.len()].0
    };
    let move_end = |m: &mut Net, e: usize, to: usize| {
        if m.edges[e].2 == v {
            m.edges[e].2 = to;
        }
        if m.edges[e].3 == v {
            m.edges[e].3 = to;
        }
    };
    // a -> x -> c share the edges either side of x; c -> y -> a those of y
    move_end(&mut m, edge_after(a), vx);
    move_end(&mut m, edge_after(x), vx);
    move_end(&mut m, edge_after(c), vy);
    move_end(&mut m, edge_after(y), vy);
    // corner a's face runs in from y's side, out to x's: from vy to vx; corner c's the other way
    let la = &mut m.faces[a.0].1[a.1];
    la.insert(a.2 + 1, (f, false));
    let shift = usize::from(c.0 == a.0 && c.1 == a.1 && c.2 > a.2);
    let lc = &mut m.faces[c.0].1[c.1];
    lc.insert(c.2 + shift + 1, (f, true));
    Some(m)
}

/// The net with edge `e`, whose ends have three corners each, gone: the two faces at its ends meet
/// in a new edge, and each of the faces along it keeps one vertex.
fn flip(b: &Body, n: &Net, e: usize) -> Option<Net> {
    let (v1, v2) = (n.edges[e].2, n.edges[e].3);
    let (c1, c2) = (corners(n, v1), corners(n, v2));
    if v1 == v2 || c1.len() != 3 || c2.len() != 3 {
        return None;
    }
    // the faces along e, and the one across each end
    let uses: Vec<(usize, usize, usize)> = n
        .faces
        .iter()
        .enumerate()
        .flat_map(|(i, f)| {
            f.1.iter().enumerate().flat_map(move |(j, l)| {
                l.iter()
                    .enumerate()
                    .filter(move |(_, u)| u.0 == e)
                    .map(move |(k, _)| (i, j, k))
            })
        })
        .collect();
    let [ua, ub] = uses[..] else { return None };
    if ua.0 == ub.0 {
        return None;
    }
    let other = |cs: &[(usize, usize, usize)]| cs.iter().map(|c| c.0).find(|&f| f != ua.0 && f != ub.0);
    let (fc, fd) = (other(&c1)?, other(&c2)?);
    if fc == fd {
        return None;
    }
    let mid = {
        let (p, q) = (n.verts[v1].0, n.verts[v2].0);
        p + (q - p) * 0.5
    };
    let solve = |faces: [usize; 3]| {
        let cons: Vec<Surface> = faces.iter().map(|&f| surface_of(b, n, f).clone()).collect();
        solve_point(&cons, mid)
    };
    let (pa, pb) = (solve([ua.0, fc, fd])?, solve([ub.0, fc, fd])?);
    let line = Line::through(pa, pb)?;
    let mut m = n.clone();
    // v1 becomes face a's vertex, v2 face b's; of each old vertex's other edges, the one on the far
    // face's side changes vertex
    m.verts[v1].0 = pa;
    m.verts[v2].0 = pb;
    let on_face = |f: usize, ed: usize| n.faces[f].1.iter().flatten().any(|u| u.0 == ed);
    for ed in 0..m.edges.len() {
        if ed == e {
            continue;
        }
        let ends = (m.edges[ed].2, m.edges[ed].3);
        if on_face(ub.0, ed) && ends.0 == v1 {
            m.edges[ed].2 = v2;
        }
        if on_face(ub.0, ed) && ends.1 == v1 {
            m.edges[ed].3 = v2;
        }
        if on_face(ua.0, ed) && ends.0 == v2 {
            m.edges[ed].2 = v1;
        }
        if on_face(ua.0, ed) && ends.1 == v2 {
            m.edges[ed].3 = v1;
        }
    }
    let f = m.edges.len();
    m.edges.push((
        Curve::Line(line),
        Interval::new(0.0, pa.distance(pb)),
        v1,
        v2,
        n.edges[e].4.with_str("flipped"),
    ));
    // out of the two faces along it; into the two across its ends where the old corner was
    let at_end = |cs: &[(usize, usize, usize)], f: usize| cs.iter().copied().find(|c| c.0 == f);
    let (cc, cd) = (at_end(&c1, fc)?, at_end(&c2, fd)?);
    // face c's loop reached v1 by its edge on a's or b's side
    let c_in_a = on_face(ua.0, n.faces[cc.0].1[cc.1][cc.2].0);
    let d_in_b = on_face(ub.0, n.faces[cd.0].1[cd.1][cd.2].0);
    let ins_c = (f, c_in_a);
    let ins_d = (f, d_in_b);
    let lc = &mut m.faces[cc.0].1[cc.1];
    lc.insert(cc.2 + 1, ins_c);
    let shift = usize::from(cd.0 == cc.0 && cd.1 == cc.1 && cd.2 > cc.2);
    let ld = &mut m.faces[cd.0].1[cd.1];
    ld.insert(cd.2 + shift + 1, ins_d);
    for (i, j, _) in [ua, ub] {
        m.faces[i].1[j].retain(|u| u.0 != e);
    }
    Some(m)
}

/// `rebuild`, changing the topology at the edges and vertices that stop it. Ok with the body as it
/// came out, Err with the first refusal no event answers.
pub(crate) fn rebuild_moving(
    body: &mut Body,
    changed: &dyn Fn(&Body, FaceId) -> bool,
) -> Result<(), RebuildError> {
    // the entities every event at which was tried and failed further on: a branch that comes to one
    // again fails the same way, so the search stays linear in them (both splits of a vertex once
    // led to the same refusal 2^8 times)
    fn go(
        body: &Body,
        changed: &dyn Fn(&Body, FaceId) -> bool,
        left: usize,
        dead: &mut Vec<RebuildError>,
    ) -> Result<Body, RebuildError> {
        let mut b = body.clone();
        let flags: Vec<bool> = (0..b.faces.len() as u32)
            .map(|f| changed(&b, FaceId(f)))
            .collect();
        let e = match rebuild(&mut b, &flags) {
            Ok(()) => return Ok(b),
            Err(e) => e,
        };
        if left == 0 || dead.contains(&e) {
            return Err(e);
        }
        let n = net(body);
        let tries: Vec<Net> = match &e {
            RebuildError::Collapsed { edge } => body
                .edge_ids()
                .find(|&x| body.edge(x).name == *edge)
                .and_then(|x| flip(body, &n, x.index()))
                .into_iter()
                .collect(),
            RebuildError::Vertex { vertex } => body
                .vertex_ids()
                .find(|&v| body.vertex(v).name == *vertex)
                .map(|v| (0..2).filter_map(|k| split(body, &n, v.index(), k)).collect())
                .unwrap_or_default(),
            RebuildError::Edge { .. } => Vec::new(),
        };
        for m in tries {
            if let Ok(r) = go(&built(body, &m), changed, left - 1, dead) {
                return Ok(r);
            }
        }
        dead.push(e.clone());
        Err(e)
    }
    *body = go(body, changed, 12, &mut Vec::new())?;
    Ok(())
}
