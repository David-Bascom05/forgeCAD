//! The part's box, tight: what `Measure.bbox` reports, the body's exact box
//! (`forge_topo::extent::exact_box`): its edges' extremes and the points inside its faces that
//! reach further (a sphere's or torus's furthest points, a cone's apex, a constructed surface's
//! stationary points), each in its face.

use forge_math::Box3;
use forge_topo::Body;

/// `b`'s box, exact (`forge_topo::extent::exact_box`).
pub fn tight_box(b: &Body) -> Box3 {
    forge_topo::extent::exact_box(b)
}
