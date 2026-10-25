use crate::body::{Body, CoedgeId, EdgeId, FaceId, LoopId, VertexId};
use crate::name::Name;
use crate::uv::{face_uv_loops, point_in_loops, unwrap_near};
use forge_geom::{CurveGeom, SurfaceGeom};
use forge_math::tol;

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
}

/// Everything wrong with a body, in a deterministic order. Empty means it is a
/// valid closed solid.
pub fn check(body: &Body) -> Vec<Problem> {
    let mut out = Vec::new();
    check_edges(body, &mut out);
    check_loops(body, &mut out);
    check_geometry(body, &mut out);
    if out.is_empty() {
        check_sides(body, &mut out);
    }
    check_names(body, &mut out);
    check_euler(body, &mut out);
    out
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
        let cs = &body.lp(l).coedges;
        if cs.is_empty() {
            out.push(Problem::LoopEmpty { lp: l });
            continue;
        }
        for (i, &c) in cs.iter().enumerate() {
            let next = cs[(i + 1) % cs.len()];
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
            let d = body.vertex(v).point.distance(edge.curve.eval(t));
            if d > band {
                out.push(Problem::VertexOffEdge {
                    edge: e,
                    vertex: v,
                    distance: d,
                });
            }
        }
        let mut faces: Vec<FaceId> = edge.coedges.iter().map(|&c| body.coedge_face(c)).collect();
        faces.dedup();
        for f in faces {
            let s = &body.face(f).surface;
            let worst = body
                .edge_points(e, 16)
                .into_iter()
                .map(|p| s.distance(p))
                .fold(0.0, f64::max);
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
        let loops = face_uv_loops(body, f);
        let surface = &body.face(f).surface;
        // probe a small fraction of the face's shortest edge so narrow faces are not stepped across
        let shortest = loops
            .iter()
            .flat_map(|l| l.coedges.iter())
            .map(|&c| edge_length(body, body.coedge(c).edge))
            .fold(f64::INFINITY, f64::min);
        let step = (0.05 * shortest).clamp(10.0 * tol::LINEAR, 0.1);
        for ul in &loops {
            for (k, &c) in ul.coedges.iter().enumerate() {
                let co = body.coedge(c);
                let e = body.edge(co.edge);
                let t = e.range.mid();
                let d = e.curve.deriv(t);
                let along = if co.forward { d.d1 } else { -d.d1 };
                let mid_uv = ul.point_of_sample(ul.coedge_first[k] + crate::uv::default_segments(e) / 2);
                let Some(n) = body.face_normal(f, mid_uv) else {
                    continue;
                };
                let Some(inward) = n.cross(along).normalize() else {
                    continue;
                };
                let probe = surface.project(d.p + inward * step);
                let probe = unwrap_near(body, f, probe, mid_uv);
                if !point_in_loops(probe, &loops) {
                    out.push(Problem::FaceWrongSide { face: f, coedge: c });
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
    let value = body.vertices.len() as i64 - body.edges.len() as i64 + 2 * body.faces.len() as i64
        - body.loops.len() as i64;
    body.shells.len() as i64 - value / 2
}
