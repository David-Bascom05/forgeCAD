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

/// A groove 0.35 to 1 mm wide cut 0.07 into a 0.17 plate, its outer corners a little skewed (the
/// scale-programs session's zerothick-groove.fg, from 11 DeepCAD parts). The plate's top face splits
/// into the plate, the groove's ring and the island; the island's loop was filed under the plate,
/// not the ring, so its edge had three faces kept (ZeroThickness). The hole's probe stepped a
/// thousandth of its longest piece (0.44 mm) off it, past the ring's far side where the ring is
/// 0.35 wide; it now stays under half the way to the outer cycles' pieces that near.
#[test]
fn a_thin_skewed_groove_cuts_a_plate() {
    let (v, built) = judge_built(include_str!("kernel_parts/zerothick-groove.fg"));
    assert!(matches!(v, Verdict::Ok), "{v:?}");
    let area = |p: &[(f64, f64)]| {
        (0..p.len())
            .map(|i| {
                let (a, b) = (p[i], p[(i + 1) % p.len()]);
                a.0 * b.1 - b.0 * a.1
            })
            .sum::<f64>()
            .abs()
            / 2.0
    };
    let ring = area(&[(220.0, -133.0), (220.0, 133.0), (-220.0, 133.0), (-220.0, -132.7)])
        - area(&[(219.0, 132.0), (219.5, -132.5), (-219.5, -132.5), (-219.0, 132.0)]);
    let want = 460.0 * 300.0 * 0.17 - ring * 0.07;
    let b = built.expect("built").part();
    let got = b
        .regenerate()
        .map(|b| forge_topo::mass::mass(&b).expect("measures").volume);
    assert!(
        got.as_ref().is_ok_and(|v| (v - want).abs() < 1e-6),
        "{got:?} where {want}"
    );
}

/// A cylinder r 40 whose end edge is rounded r 40, so the end closes to a ball's pole, cut by a bar
/// a quadrant of whose corner runs along the axis through the pole (the scale-programs session's
/// classify-fillet-cut.fg, from 3 CADFS parts). The bar's top face meets the ball in a great circle
/// half of which is the ball's seam meridian: the trim dropped the whole circle as along the seam,
/// so the notch's other side had no edge (InconsistentClassification); a curve now goes as along a
/// seam only where it runs along it all its length. The notch's loop then passes the pole twice, and
/// the check drew it with a period shift that walks the pole's line back over itself (each shift had
/// the same area to rounding): FaceWrongSide, refused as a touch. The shortest walk along the
/// singular lines now breaks the tie. Exact volume.
#[test]
fn a_bar_cut_through_a_rounded_ends_pole_builds() {
    let (v, built) = judge_built(include_str!("kernel_parts/classify-fillet-cut.fg"));
    assert!(matches!(v, Verdict::Ok), "{v:?}");
    // the bar takes y in [-24, 0], z in [-17, 0] from x = 40 - sqrt(1600 - y² - z²) to 80
    let n = 400;
    let (hy, hz) = (24.0 / n as f64, 17.0 / n as f64);
    let w = |i: usize| {
        if i == 0 || i == n {
            1.0
        } else if i % 2 == 1 {
            4.0
        } else {
            2.0
        }
    };
    let mut taken = 0.0;
    for i in 0..=n {
        for j in 0..=n {
            let (y, z) = (-24.0 + i as f64 * hy, -17.0 + j as f64 * hz);
            taken += w(i) * w(j) * (40.0 + (1600.0 - y * y - z * z).sqrt());
        }
    }
    taken *= hy * hz / 9.0;
    let pi = std::f64::consts::PI;
    let want = pi * 1600.0 * 40.0 + 2.0 / 3.0 * pi * 64000.0 - taken;
    let body = built.expect("built").body().expect("a body");
    let got = forge_topo::mass::mass(&body).expect("measures").volume;
    assert!((got - want).abs() < 1e-6 * want, "{got} where {want}");
}
