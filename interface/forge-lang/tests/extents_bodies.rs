//! Extents and body targets the retired `.fc` format had, written in Forge: each part held to
//! what the `.fc` part built (volume, faces, edges, bodies).
mod common;

/// Volume, faces, edges and bodies of a part that builds valid.
fn shape(src: &str) -> (f64, usize, usize, usize) {
    let m = common::holds(src);
    (m.volume, m.faces, m.edges, m.bodies)
}

const TWO: &str = "sketch sa(on: xy) {
  r = rect(corner: (0, 0), size: (10, 10))
}
a = extrude(sa, 10)
sketch sb(on: xy) {
  r = rect(corner: (40, 0), size: (10, 10))
}
b = extrude(sb, 10, new: true)
";

/// `.fc` `thicken cut 2 neg (faces plane normal +z of a) in a`: the slab cut from the first body
/// though the second was made last (1800 mm3, 12 faces, 24 edges, 2 bodies). Without `bodies:`
/// it cuts the last body, which the slab misses.
#[test]
fn thicken_in_a_body() {
    let v = shape(&format!(
        "{TWO}th = thicken(a.end, 2, side: \"in\", cut: true, bodies: a)\n"
    ));
    assert!((v.0 - 1800.0).abs() < 1e-6, "{v:?}");
    assert_eq!((v.1, v.2, v.3), (12, 24, 2));
}

/// `.fc` `thicken 2 (faces plane normal +z of a) in all`: 2200 mm3, 12 faces, 24 edges, 2 bodies.
#[test]
fn thicken_in_every_body() {
    let v = shape(&format!("{TWO}th = thicken(a.end, 2, bodies: \"all\")\n"));
    assert!((v.0 - 2200.0).abs() < 1e-6, "{v:?}");
    assert_eq!((v.1, v.2, v.3), (12, 24, 2));
}

const UPTO: &str = "sketch s0(on: xy) {
  r = rect(corner: (-20, -20), size: (40, 40))
}
b0 = extrude(s0, 5)
sketch t(on: offset(xy, 30)) {
  p = pt(0, 0)
}
sketch s(on: xy) {
  r = rect(corner: (0, 0), size: (10, 10))
}
";

/// `.fc` `extrude to s ..`, s a sketch: up to the sketch's plane (10500 mm3, 11 faces, 24
/// edges), and `off -4` short of it (10100 mm3).
#[test]
fn up_to_a_sketch_plane() {
    let v = shape(&format!("{UPTO}a = extrude(s, upto(t))\n"));
    assert!((v.0 - 10500.0).abs() < 1e-6, "{v:?}");
    assert_eq!((v.1, v.2, v.3), (11, 24, 1));
    let v = shape(&format!("{UPTO}a = extrude(s, upto(t, -4))\n"));
    assert!((v.0 - 10100.0).abs() < 1e-6, "{v:?}");
    assert_eq!((v.1, v.2, v.3), (11, 24, 1));
}
