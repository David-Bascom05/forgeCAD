//! The part's box, tight: what `Measure.bbox` reports.
//!
//! A face's own box (`Body::face_box`) bounds it, loosely where its edges are arcs in a tilted
//! plane or the surface curves past them. The box here is the edges' own (a line's ends, a
//! circle's arc's extremes, other curves at 65 points), widened only by a face that can reach
//! past its boundary (a sphere, torus, curved-across procedural surface, or a cone holding its
//! apex; a plane, cylinder, other cone or ruled surface is furthest along any axis on its
//! boundary) and whose bound does reach past the box so far: that face alone is meshed and its
//! points taken. The same box the retired `.fc` stats gave
//! (face triangles and edges), at the cost of meshing only the faces that stick out.

use forge_geom::{Curve, Surface};
use forge_math::Box3;
use forge_mesh::Quality;
use forge_topo::Body;

/// `b`'s box, tight to within the meshing's chord (0.05) where a face bulges past its edges.
pub fn tight_box(b: &Body) -> Box3 {
    let mut bb = Box3::EMPTY;
    for e in b.edge_ids() {
        bb = match &b.edge(e).curve {
            Curve::Line(_) => bb.hull(&b.edge_box(e)),
            Curve::Circle(c) => bb.hull(&c.tight_box(b.edge(e).range)),
            _ => b
                .edge_points(e, 64)
                .into_iter()
                .fold(bb, |x, p| x.hull(&Box3::from_point(p))),
        };
    }
    // past what is reached so far (a face within it adds nothing)
    let past = |f: &Box3, now: &Box3| {
        let eps = forge_math::tol::LINEAR;
        f.x.lo < now.x.lo - eps
            || f.y.lo < now.y.lo - eps
            || f.z.lo < now.z.lo - eps
            || f.x.hi > now.x.hi + eps
            || f.y.hi > now.y.hi + eps
            || f.z.hi > now.z.hi + eps
    };
    let q = Quality {
        chord: 0.05,
        ..Quality::default()
    };
    let rough = Quality {
        chord: 0.5,
        max_angle: forge_math::dmath::PI / 6.0,
    };
    for f in b.face_ids() {
        let face = b.face(f);
        let fb = b.face_box(f);
        let lies = match &face.surface {
            Surface::Plane(_) | Surface::Cylinder(_) => false,
            // straight across in v: each point between two boundary points (as `face_box` has it)
            Surface::Procedural(pr) => !(pr.v_degree() == Some(1) && !face.loops.is_empty()),
            // a cone's apex may sit inside its loops, on no edge
            Surface::Cone(c) => fb.grow(forge_math::tol::LINEAR).contains(c.apex()),
            _ => true,
        };
        if !lies || !past(&fb, &bb) {
            continue;
        }
        let alone = forge_agent::facemesh::alone(b, f);
        // coarsely first: a face whose coarse points, out by the coarse chord, stay within adds
        // nothing
        let coarse = forge_mesh::tessellate::tessellate_tolerant(&alone, &rough);
        if let Ok(m) = &coarse
            && !past(&m.bbox().grow(rough.chord), &bb)
        {
            continue;
        }
        bb = match forge_mesh::tessellate::tessellate_tolerant(&alone, &q) {
            Ok(m) => bb.hull(&m.bbox()),
            // what would not mesh stands in by its bound
            Err(_) => bb.hull(&fb),
        };
    }
    bb
}
