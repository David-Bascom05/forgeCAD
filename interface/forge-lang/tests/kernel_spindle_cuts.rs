//! Cuts through a spindle torus (a revolved arc ending on the axis: a lemon, or an apple), judged by the
//! definition: the body holds a point exactly when the spindle does and the tool does not, at points
//! spread through the box, and volumes where a slice integral has a closed form. Own numbers: arc
//! radius 13, centre 5 off the axis on either side, axis z (research/spindle-priorart).
mod common;
mod kernel_util;

use forge_bool::{Index, Where};
use forge_math::{Point3, dmath};
use kernel_util::built;

/// The arc through (0, -12) and (0, 12), centre (-5, 0) for the lemon and (5, 0) for the apple,
/// turned about the line joining its ends.
fn spindle(lemon: bool) -> String {
    let c = if lemon { -5 } else { 5 };
    format!(
        "sketch sec(on: xz) {{
  a = arc(center: ({c}, 0), from: (0, -12), to: (0, 12))
  l = line((0, 12), (0, -12))
}}
spindle = revolve(sec, axis: sec.l)
"
    )
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

/// A xorshift stream in [0, 1).
fn stream() -> impl FnMut() -> f64 {
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Builds valid; at `n` points `at` gives, the body agrees with `want` at all but those within a
/// hair of a boundary, judged from both sides; the body's volume.
fn agrees_at(src: &str, mut at: impl FnMut() -> Point3, want: impl Fn(Point3) -> bool) -> f64 {
    let b = built(src);
    let p = forge_topo::check(&b.body);
    assert!(p.is_empty(), "{p:?}");
    let ix = Index::new(&b.body);
    let (mut bad, mut n) = (Vec::new(), 0);
    for _ in 0..20000 {
        let p = at();
        let w = want(p);
        // the definition near its own boundary is the rounding's: a hair off it either way
        let h = 1e-6;
        let firm = [-h, h].iter().all(|&e| {
            [
                Point3::new(p.x + e, p.y, p.z),
                Point3::new(p.x, p.y + e, p.z),
                Point3::new(p.x, p.y, p.z + e),
            ]
            .iter()
            .all(|&q| want(q) == w)
        });
        let got = ix.classify(p);
        n += 1;
        if firm && got != Where::On && (got == Where::Inside) != w {
            bad.push(p);
        }
    }
    assert!(
        bad.is_empty(),
        "{} of {n} disagree, first {:?}",
        bad.len(),
        bad.first()
    );
    b.volume()
}

/// Through the spindle's box.
fn agrees(src: &str, lemon: bool, kept: impl Fn(Point3) -> bool) -> f64 {
    let mut next = stream();
    agrees_at(
        src,
        move || Point3::new(36.0 * next() - 18.0, 36.0 * next() - 18.0, 26.0 * next() - 13.0),
        |p| in_spindle(lemon, p) && kept(p),
    )
}

/// The lemon below a plane square to its axis: pi times the integral of (sqrt(169 - z^2) - 5)^2.
fn lemon_below(z: f64) -> f64 {
    let f = |z: f64| {
        194.0 * z - z * z * z / 3.0 - 10.0 * (z / 2.0 * (169.0 - z * z).sqrt() + 84.5 * dmath::asin(z / 13.0))
    };
    std::f64::consts::PI * (f(z) - f(-12.0))
}

/// A block 100 across cut from the part: above the plane z, square to the axis.
fn above(z: f64) -> String {
    format!(
        "sketch tool(on: offset(xy, {z})) {{
  r = rect(corner: (-50, -50), size: (100, 100))
}}
cutter = cut(tool, -50)
"
    )
}

/// Past the plane y = c, along the axis.
fn past_y(c: f64) -> String {
    format!(
        "sketch tool(on: offset(xz, -50)) {{
  r = rect(corner: (-50, -50), size: (100, 100))
}}
cutter = cut(tool, -{})
",
        50.0 - c
    )
}

/// Past the plane x = c, along the axis.
fn past_x(c: f64) -> String {
    format!(
        "sketch tool(on: offset(yz, {c})) {{
  r = rect(corner: (-50, -50), size: (100, 100))
}}
cutter = cut(tool, -50)
"
    )
}

#[test]
fn a_lemon_cut_square_to_its_axis() {
    for z in [3.0, -6.0] {
        let src = spindle(true) + &above(z);
        let v = agrees(&src, true, |p| p.z < z);
        assert!((v - lemon_below(z)).abs() < 1e-9 * lemon_below(z), "{z}: {v}");
    }
}

#[test]
fn spindles_cut_by_planes_along_the_axis() {
    for lemon in [true, false] {
        for c in [3.0, -6.0] {
            agrees(&(spindle(lemon) + &past_y(c)), lemon, |p| p.y < c);
        }
        for c in [2.0, -4.0] {
            agrees(&(spindle(lemon) + &past_x(c)), lemon, |p| p.x < c);
        }
    }
}

#[test]
fn spindles_cut_by_a_cylinder_across_the_axis() {
    for lemon in [true, false] {
        for (y, z) in [(0.0, 0.0), (3.0, 1.0)] {
            // through along +x from x = -40: the sketch faces -x, its y down
            let src = spindle(lemon)
                + &format!(
                    "sketch o(on: xy) {{
  p = pt(-40, 0)
}}
sketch tool(on: plane(at: o.p, normal: -X, x: Y)) {{
  c = circle(({y}, {}), 2)
}}
cutter = cut(tool, thru)
",
                    -z
                );
            agrees(&src, lemon, |p| {
                ((p.y - y) * (p.y - y) + (p.z - z) * (p.z - z)).sqrt() > 2.0
            });
        }
    }
}

#[test]
fn an_apple_cut_square_to_its_axis() {
    for z in [3.0, -6.0] {
        agrees(&(spindle(false) + &above(z)), false, |p| p.z < z);
    }
}

/// A plane through the axis meets both apices, where the sheets meet: refused, not built wrong.
#[test]
fn a_plane_through_a_spindles_axis_is_refused() {
    for lemon in [true, false] {
        let (r, _) = common::run(&(spindle(lemon) + &past_x(0.0)));
        assert!(!r.errors.is_empty(), "lemon {lemon}");
    }
}

/// A whole circle tangent to the axis turned part way round: an apex where it touches, like a cone's (was
/// refused as a pinch, which only the whole turn is). Pappus: the disc's area times its centre's path.
#[test]
fn a_circle_on_the_axis_turned_part_way() {
    let turned = |angle: &str| {
        format!(
            "sketch sec(on: xz) {{
  c = circle((3, 0), 3)
  ax = line((0, 0), (0, 1), construction: true)
}}
turned = revolve(sec.c, axis: sec.ax{angle})
"
        )
    };
    for a in [90.0f64, 299.0] {
        let b = built(&turned(&format!(", angle: {a}deg")));
        let p = forge_topo::check(&b.body);
        assert!(p.is_empty(), "{p:?}");
        let v = b.volume();
        let pappus = std::f64::consts::PI * 9.0 * 3.0 * a * std::f64::consts::PI / 180.0;
        assert!((v - pappus).abs() < 1e-9 * pappus, "{a}: {v}");
    }
    assert!(!common::run(&turned("")).0.errors.is_empty());
}

#[test]
fn spindles_cut_by_a_cylinder_along_the_axis() {
    for lemon in [true, false] {
        // through along +z from z = -30: the sketch faces -z
        let src = spindle(lemon)
            + "sketch o(on: offset(xy, -30)) {
  p = pt(0, 0)
}
sketch tool(on: plane(at: o.p, normal: -Z, x: X)) {
  c = circle((3, 0), 2)
}
cutter = cut(tool, thru)
";
        agrees(&src, lemon, |p| {
            ((p.x - 3.0) * (p.x - 3.0) + p.y * p.y).sqrt() > 2.0
        });
    }
}

// A barrel: the lemon's arc between heights -h and h, capped flat, its apices off the part.
const BARREL: (f64, f64, f64) = (40.0, 70.0, 30.0); // R (across the axis), r, half height
fn barrel() -> String {
    let (cr, r, h) = BARREL;
    let w = (r * r - h * h).sqrt() - cr;
    format!(
        "sketch sec(on: xz) {{
  a = arc(center: ({}, 0), from: ({w}, {}), to: ({w}, {h}))
  top = line(({w}, {h}), (0, {h}))
  spine = line((0, {h}), (0, {}))
  foot = line((0, {}), ({w}, {}))
}}
barrel = revolve(sec, axis: sec.spine)
",
        -cr, -h, -h, -h, -h
    )
}
fn in_barrel(p: Point3) -> bool {
    let (cr, r, h) = BARREL;
    p.z.abs() < h && (p.x * p.x + p.y * p.y).sqrt() < (r * r - p.z * p.z).sqrt() - cr
}
fn barrel_agrees(src: &str, kept: impl Fn(Point3) -> bool) -> f64 {
    let mut next = stream();
    agrees_at(
        src,
        move || Point3::new(70.0 * next() - 35.0, 70.0 * next() - 35.0, 70.0 * next() - 35.0),
        |p| in_barrel(p) && kept(p),
    )
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
        let v = barrel_agrees(&(barrel() + &past_y(c)), |p| p.y < c);
        let e = barrel_below(c);
        assert!((v - e).abs() < 1e-5 * e, "{c}: {v} {e}");
    }
}
#[test]
fn a_barrel_with_a_slot_and_a_cross_hole() {
    let tool = |profile: &str| {
        format!(
            "sketch tool(on: offset(xz, -50)) {{
  {profile}
}}
cutter = cut(tool, -100)
"
        )
    };
    let src = barrel() + &tool("r = rect(corner: (-5, -40), size: (10, 80))");
    barrel_agrees(&src, |p| p.x.abs() > 5.0);
    let src = barrel() + &tool("c = circle((0, 8), 6)");
    barrel_agrees(&src, |p| (p.x * p.x + (p.z - 8.0) * (p.z - 8.0)).sqrt() > 6.0);
    let src = barrel() + &tool("c = circle((11, 8), 6)");
    barrel_agrees(&src, |p| {
        ((p.x - 11.0) * (p.x - 11.0) + (p.z - 8.0) * (p.z - 8.0)).sqrt() > 6.0
    });
}

#[test]
fn spindles_cut_by_planes_just_off_an_apex() {
    for lemon in [true, false] {
        for c in [1.0, 0.1, 0.01, 1e-3, 1e-4, -1e-3] {
            agrees(&(spindle(lemon) + &past_y(c)), lemon, |p| p.y < c);
        }
    }
}

/// A barrel against a smooth loft running in through its side, the pair only the tracer can do:
/// what the two make together and what they share add up to the two alone.
#[test]
fn a_barrel_and_a_loft_add_up() {
    let arm = |op: &str| {
        format!(
            "sketch a0(on: offset(yz, -5)) {{
  c = circle((0, 3), 7)
}}
sketch a1(on: offset(yz, 25)) {{
  c = circle((0, 6), 6)
}}
sketch a2(on: offset(yz, 70)) {{
  c = circle((0, 9), 5)
}}
arm = loft([a0, a1, a2], smooth: true{op})
"
        )
    };
    let mass = |src: String| {
        let b = built(&src);
        assert!(forge_topo::check(&b.body).is_empty());
        b.volume()
    };
    let a = mass(barrel());
    let b = mass(arm(""));
    let or = mass(barrel() + &arm(""));
    let and = mass(barrel() + &arm(", keep: true"));
    let less = mass(barrel() + &arm(", cut: true"));
    assert!((or + and - a - b).abs() < 1e-6 * (a + b), "{or} {and} {a} {b}");
    assert!((less + and - a).abs() < 1e-6 * a, "{less} {and} {a}");
    assert!(and > 0.0 && and < b && less < a);
}
