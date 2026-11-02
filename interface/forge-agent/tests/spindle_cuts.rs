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

// A barrel: the lemon's arc between heights -h and h, capped flat, its apices off the part.
const BARREL: (f64, f64, f64) = (40.0, 70.0, 30.0); // R (across the axis), r, half height
fn barrel() -> String {
    let (cr, r, h) = BARREL;
    let a = dmath::asin(h / r) * 180.0 / std::f64::consts::PI;
    let w = (r * r - h * h).sqrt() - cr;
    format!(
        "revolve xz axis y loop arc {} 0 {r} {} {} line {w} {h} 0 {h} line 0 {h} 0 {} line 0 {} {w} {}\n",
        -cr,
        -a,
        2.0 * a,
        -h,
        -h,
        -h
    )
}
fn in_barrel(p: Point3) -> bool {
    let (cr, r, h) = BARREL;
    p.z.abs() < h && (p.x * p.x + p.y * p.y).sqrt() < (r * r - p.z * p.z).sqrt() - cr
}
fn barrel_agrees(src: &str, kept: impl Fn(Point3) -> bool) -> f64 {
    let mut m = Model::new(src);
    assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
    let b = m.body().expect("a body");
    assert!(forge_topo::check(b).is_empty(), "{:?}", forge_topo::check(b));
    let ix = Index::new(b);
    let (mut seed, mut bad, mut n) = (0x9E37_79B9_7F4A_7C15u64, Vec::new(), 0);
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    for _ in 0..20000 {
        let p = Point3::new(70.0 * next() - 35.0, 70.0 * next() - 35.0, 70.0 * next() - 35.0);
        let want = in_barrel(p) && kept(p);
        let h = 1e-6;
        let firm = [-h, h].iter().all(|&e| {
            [
                Point3::new(p.x + e, p.y, p.z),
                Point3::new(p.x, p.y + e, p.z),
                Point3::new(p.x, p.y, p.z + e),
            ]
            .iter()
            .all(|&q| (in_barrel(q) && kept(q)) == want)
        });
        let got = ix.classify(p);
        n += 1;
        if firm && got != Where::On && (got == Where::Inside) != want {
            bad.push(p);
        }
    }
    assert!(
        bad.is_empty(),
        "{} of {n} disagree, first {:?}",
        bad.len(),
        bad.first()
    );
    m.mass().expect("a mass").volume
}
/// Barrel on y < c: integral over z of the area of the disc of radius a(z) with y < c.
fn barrel_below(c: f64) -> f64 {
    let (cr, r, h) = BARREL;
    let n = 20000;
    let seg = |z: f64| {
        let a = (r * r - z * z).sqrt() - cr;
        if c >= a {
            return std::f64::consts::PI * a * a;
        }
        if c <= -a {
            return 0.0;
        }
        // the disc's part below y = c
        a * a * (std::f64::consts::PI - dmath::acos(c / a)) + c * (a * a - c * c).sqrt()
    };
    let d = 2.0 * h / n as f64;
    (0..=n)
        .map(|i| {
            seg(-h + d * i as f64)
                * if i == 0 || i == n {
                    1.0
                } else if i % 2 == 1 {
                    4.0
                } else {
                    2.0
                }
        })
        .sum::<f64>()
        * d
        / 3.0
}
#[test]
fn a_barrel_cut_by_planes_along_its_axis() {
    // through the axis, near it, across it, and tangent to the waist of the arc
    let (cr, r, _) = BARREL;
    for c in [0.0, 1e-3, 3.0, -6.0, 20.0, 29.5, r - cr - 1e-3] {
        let src = barrel() + &format!("extrude cut {} xz at 0 50 0 rect -50 -50 100 100\n", 50.0 - c);
        let v = barrel_agrees(&src, |p| p.y < c);
        let e = barrel_below(c);
        assert!((v - e).abs() < 1e-5 * e, "{c}: {v} {e}");
    }
}
#[test]
fn a_barrel_with_a_slot_and_a_cross_hole() {
    let src = barrel() + "extrude cut 100 xz at 0 50 0 rect -5 -40 10 80\n";
    barrel_agrees(&src, |p| p.x.abs() > 5.0);
    let src = barrel() + "extrude cut 100 xz at 0 50 8 circle 0 0 6\n";
    barrel_agrees(&src, |p| (p.x * p.x + (p.z - 8.0) * (p.z - 8.0)).sqrt() > 6.0);
    let src = barrel() + "extrude cut 100 xz at 11 50 8 circle 0 0 6\n";
    barrel_agrees(&src, |p| {
        ((p.x - 11.0) * (p.x - 11.0) + (p.z - 8.0) * (p.z - 8.0)).sqrt() > 6.0
    });
}

#[test]
fn spindles_cut_by_planes_just_off_an_apex() {
    for lemon in [true, false] {
        for c in [1.0, 0.1, 0.01, 1e-3, 1e-4, -1e-3] {
            let src =
                spindle(lemon) + &format!("extrude cut {} xz at 0 50 0 rect -50 -50 100 100\n", 50.0 - c);
            agrees(&src, lemon, |p| p.y < c);
        }
    }
}

/// A barrel against a smooth loft running in through its side, the pair only the tracer can do:
/// what the two make together and what they share add up to the two alone.
#[test]
fn a_barrel_and_a_loft_add_up() {
    let arm = "smooth profile yz at -5 0 3 circle 0 0 7 profile yz at 25 0 6 circle 0 0 6 profile yz at 70 0 9 circle 0 0 5\n";
    let mass = |src: String| {
        let mut m = Model::new(&src);
        assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
        assert!(forge_topo::check(m.body().expect("a body")).is_empty());
        m.mass().expect("a mass").volume
    };
    let a = mass(barrel());
    let b = mass(format!("loft {arm}"));
    let or = mass(barrel() + &format!("loft add {arm}"));
    let and = mass(barrel() + &format!("loft keep {arm}"));
    let less = mass(barrel() + &format!("loft cut {arm}"));
    assert!((or + and - a - b).abs() < 1e-6 * (a + b), "{or} {and} {a} {b}");
    assert!((less + and - a).abs() < 1e-6 * a, "{less} {and} {a}");
    assert!(and > 0.0 && and < b && less < a);
}
