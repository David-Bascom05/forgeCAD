//! Cuts through a spindle torus (a revolved arc ending on the axis: a lemon, or an apple), judged by the
//! definition: the body holds a point exactly when the spindle does and the tool does not, at points
//! spread through the box, and volumes where a slice integral has a closed form. Own numbers: arc
//! radius 13, centre 5 off the axis on either side, axis z (research/spindle-priorart).
use forge_agent::model::Model;
use forge_bool::{Index, Where};
use forge_math::{Point3, dmath};

fn spindle(lemon: bool) -> String {
    // the arc through (0, -12) and (0, 12), centre (-5, 0) for the lemon and (5, 0) for the apple
    let d = dmath::acos(5.0 / 13.0) * 180.0 / std::f64::consts::PI;
    if lemon {
        format!(
            "revolve xz axis y loop arc -5 0 13 {} {} line 0 12 0 -12\n",
            -d,
            2.0 * d
        )
    } else {
        format!(
            "revolve xz axis y loop arc 5 0 13 {} {} line 0 12 0 -12\n",
            d - 180.0,
            2.0 * (180.0 - d)
        )
    }
}

/// In the spindle: the profile turned about z, by its distance from the axis at each height.
fn in_spindle(lemon: bool, p: Point3) -> bool {
    let rho = (p.x * p.x + p.y * p.y).sqrt();
    let s = 169.0 - p.z * p.z;
    if s <= 0.0 {
        return false;
    }
    let s = s.sqrt();
    if lemon {
        rho < s - 5.0
    } else if p.z.abs() <= 12.0 {
        rho < 5.0 + s
    } else {
        rho > 5.0 - s && rho < 5.0 + s
    }
}

/// Builds; at points through the spindle's box the body agrees with `kept` (on the definition) at all
/// but those within a hair of a boundary, judged from both sides.
fn agrees(src: &str, lemon: bool, kept: impl Fn(Point3) -> bool) -> f64 {
    let mut m = Model::new(src);
    assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
    let b = m.body().expect("a body");
    assert!(forge_topo::check(b).is_empty(), "{:?}", forge_topo::check(b));
    let ix = Index::new(b);
    let (mut seed, mut bad) = (0x9E37_79B9_7F4A_7C15u64, Vec::new());
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    for _ in 0..20000 {
        let p = Point3::new(36.0 * next() - 18.0, 36.0 * next() - 18.0, 26.0 * next() - 13.0);
        let want = in_spindle(lemon, p) && kept(p);
        // the definition near its own boundary is the rounding's: a hair off it either way
        let h = 1e-6;
        let firm = [-h, h].iter().all(|&e| {
            [
                Point3::new(p.x + e, p.y, p.z),
                Point3::new(p.x, p.y + e, p.z),
                Point3::new(p.x, p.y, p.z + e),
            ]
            .iter()
            .all(|&q| (in_spindle(lemon, q) && kept(q)) == want)
        });
        let got = ix.classify(p);
        if firm && got != Where::On && (got == Where::Inside) != want {
            bad.push(p);
        }
    }
    assert!(
        bad.is_empty(),
        "{} points disagree, first {:?}",
        bad.len(),
        bad.first()
    );
    m.mass().expect("a mass").volume
}

/// The lemon below a plane square to its axis: pi times the integral of (sqrt(169 - z^2) - 5)^2.
fn lemon_below(z: f64) -> f64 {
    let f = |z: f64| {
        194.0 * z - z * z * z / 3.0 - 10.0 * (z / 2.0 * (169.0 - z * z).sqrt() + 84.5 * dmath::asin(z / 13.0))
    };
    std::f64::consts::PI * (f(z) - f(-12.0))
}

#[test]
fn a_lemon_cut_square_to_its_axis() {
    for z in [3.0, -6.0] {
        let src = spindle(true) + &format!("extrude cut 50 at 0 0 {z} rect -50 -50 100 100\n");
        let v = agrees(&src, true, |p| p.z < z);
        assert!((v - lemon_below(z)).abs() < 1e-9 * lemon_below(z), "{z}: {v}");
    }
}

#[test]
fn spindles_cut_by_planes_along_the_axis() {
    for lemon in [true, false] {
        for c in [3.0, -6.0] {
            let src =
                spindle(lemon) + &format!("extrude cut {} xz at 0 50 0 rect -50 -50 100 100\n", 50.0 - c);
            agrees(&src, lemon, |p| p.y < c);
        }
        for c in [2.0, -4.0] {
            let src = spindle(lemon) + &format!("extrude cut 50 yz at {c} 0 0 rect -50 -50 100 100\n");
            agrees(&src, lemon, |p| p.x < c);
        }
    }
}

#[test]
fn spindles_cut_by_a_cylinder_across_the_axis() {
    for lemon in [true, false] {
        for (y, z) in [(0.0, 0.0), (3.0, 1.0)] {
            let src = spindle(lemon) + &format!("extrude cut thru yz at -40 0 0 circle {y} {z} 2\n");
            agrees(&src, lemon, |p| {
                ((p.y - y) * (p.y - y) + (p.z - z) * (p.z - z)).sqrt() > 2.0
            });
        }
    }
}

#[test]
fn an_apple_cut_square_to_its_axis() {
    for z in [3.0, -6.0] {
        let src = spindle(false) + &format!("extrude cut 50 at 0 0 {z} rect -50 -50 100 100\n");
        agrees(&src, false, |p| p.z < z);
    }
}

/// A plane through the axis meets both apices, where the sheets meet: refused, not built wrong.
#[test]
fn a_plane_through_a_spindles_axis_is_refused() {
    for lemon in [true, false] {
        let m = Model::new(&(spindle(lemon) + "extrude cut 50 yz at 0 0 0 rect -50 -50 100 100\n"));
        assert!(m.error.is_some(), "lemon {lemon}");
    }
}

/// A whole circle tangent to the axis turned part way round: an apex where it touches, like a cone's (was
/// refused as a pinch, which only the whole turn is). Pappus: the disc's area times its centre's path.
#[test]
fn a_circle_on_the_axis_turned_part_way() {
    for a in [90.0f64, 299.0] {
        let mut m = Model::new(&format!("revolve {a} xz axis y circle 3 0 3\n"));
        assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
        let b = m.body().expect("a body");
        assert!(forge_topo::check(b).is_empty(), "{:?}", forge_topo::check(b));
        let v = m.mass().expect("a mass").volume;
        let pappus = std::f64::consts::PI * 9.0 * 3.0 * a * std::f64::consts::PI / 180.0;
        assert!((v - pappus).abs() < 1e-9 * pappus, "{a}: {v}");
    }
    assert!(Model::new("revolve xz axis y circle 3 0 3\n").error.is_some());
}

#[test]
fn spindles_cut_by_a_cylinder_along_the_axis() {
    for lemon in [true, false] {
        let src = spindle(lemon) + "extrude cut thru xy at 0 0 -30 circle 3 0 2\n";
        agrees(&src, lemon, |p| {
            ((p.x - 3.0) * (p.x - 3.0) + p.y * p.y).sqrt() > 2.0
        });
    }
}
