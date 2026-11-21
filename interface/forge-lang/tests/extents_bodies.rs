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
