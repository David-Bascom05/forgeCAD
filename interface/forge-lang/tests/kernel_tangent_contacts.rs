//! Tangent contacts the boolean decides once: a curve meeting a face's surface only at points is
//! not on it (closed form where there is one), however its samples land.

/// A drill started on a plane tangent to a round crown, cut up to a bore (from CADFS 00308391):
/// the drill's rim meets the crown at two points only, at a quarter and three quarters round,
/// where the along-test sampled it, so the whole rim was imprinted into the crown and the cut
/// was NotClosed. The rim meets it in two touching hits, so it is not on it: the cut builds.
#[test]
fn a_drill_from_a_tangent_plane_to_a_bore_builds() {
    use forge_lang::fuzz::{Verdict, judge_built};
    let (v, _) = judge_built(include_str!("kernel_parts/drill_from_tangent_plane.fg"));
    assert!(matches!(v, Verdict::Ok), "{v:?}");
}
