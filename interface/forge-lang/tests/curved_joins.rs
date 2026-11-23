//! Curved joins the importer and the trace session found failing, each a judge fail where a valid
//! body is due (research/kernel-audit/exact-comparison-curved.md). The parts are the importer's.

use forge_lang::fuzz::{Verdict, judge_built};

/// A ball over a tapered pocket: the pocket's top edge is tangent to the ball at its pole, and two
/// split curves leave the pole along one tangent. At a pole the split's chords had no direction,
/// so the tie stood and the split crossed itself (SplitCrossed).
#[test]
fn a_ball_tangent_to_a_pocket_edge_at_its_pole_builds() {
    let (v, _) = judge_built(include_str!("kernel_parts/ball_over_tapered_pocket.fg"));
    assert!(matches!(v, Verdict::Ok), "{v:?}");
}

/// A slot lug joined to a solid cylinder: its arc of radius 3 is tangent to the cylinder's
/// radius at the lug's end face, so the curve it meets the wall in turns back exactly on the
/// wall's seam. Each edge's loop was drawn from its curve's ends, within tolerance of the vertex
/// but not at it, and the hairpin's two edges zigzagged across each other there (LoopsCross).
#[test]
fn a_lug_tangent_to_a_cylinder_at_its_end_joins_valid() {
    let (v, _) = judge_built(include_str!("kernel_parts/lug_join_loops_cross.fg"));
    assert!(matches!(v, Verdict::Ok), "{v:?}");
}

/// A drill down a lug's hole, its wall coincident with the hole's, after a counterbore cut again
/// through the lug: the counterbore's two faces on one cylinder meet in the lug's imprint, a curve
/// where two cylinders meet. Across such an edge the boolean took a plane through its middle,
/// which does not hold the curve, and fell back to a sampled search whose result was left open
/// (RetryInvalid). Across it now is the curve's other surface, which holds it exactly.
#[test]
fn a_drill_down_a_counterbored_lug_hole_builds() {
    let (v, _) = judge_built(include_str!("kernel_parts/drill_coincident_lug_hole.fg"));
    assert!(matches!(v, Verdict::Ok), "{v:?}");
}
