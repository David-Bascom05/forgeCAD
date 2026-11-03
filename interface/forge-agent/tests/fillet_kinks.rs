//! A fillet on one edge where the rim carries on past a slight kink: within half a degree the next
//! edges round with it, as a tangent chain; past that the round ends on the next wall.
use forge_agent::model::Model;
use forge_math::dmath;

/// A 30 x 20 x 10 block whose back edge is a line meeting a quarter arc of radius 10, turned
/// `kink` degrees outward where they meet; fillet 2 on the line's top edge only.
fn block(kink: f64) -> (f64, usize) {
    let t = kink.to_radians();
    let y0 = 20.0 + 20.0 * dmath::sin(t) / dmath::cos(t);
    let src = format!(
        "b: extrude 10 loop line 0 {y0} 20 20 arc 20 10 10 90 -90 line 30 10 30 0 line 30 0 0 0 line 0 0 0 {y0}\n\
         fillet 2 (edges top line near 10 {} 10)\n",
        (y0 + 20.0) / 2.0
    );
    let m = Model::new(&src);
    if let Some(f) = &m.error {
        panic!("{}\n{src}", f.msg);
    }
    let b = m.body().expect("built");
    assert!(forge_topo::check(b).is_empty());
    (
        forge_topo::mass::mass(b).expect("mass").volume,
        b.face_ids().count(),
    )
}

#[test]
fn a_kink_under_half_a_degree_rounds_on() {
    // OpenCascade's fillet of the same block, which chains these edges too (research/blend-ends)
    let (v, faces) = block(0.03);
    assert_eq!(faces, 10, "the arc and the line after it round too");
    assert!((v - 5747.809067323634).abs() < 1e-4 * v, "{v}");
}

#[test]
fn a_kink_over_half_a_degree_ends_the_round() {
    let (_, faces) = block(1.0);
    assert_eq!(faces, 8, "only the picked edge rounds");
}
