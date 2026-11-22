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
/// though the second was made last (1800 mm3, 12 faces, 24 edges, 2 bodies); and `in b`, a body
/// not holding the face, refused as .fc refused it (the kernel's MissingEdge).
#[test]
fn thicken_in_a_body() {
    let v = shape(&format!(
        "{TWO}th = thicken(a.end, 2, side: \"in\", cut: true, bodies: a)\n"
    ));
    assert!((v.0 - 1800.0).abs() < 1e-6, "{v:?}");
    assert_eq!((v.1, v.2, v.3), (12, 24, 2));
    let r = common::refused(&format!(
        "{TWO}th = thicken(a.end, 2, side: \"in\", cut: true, bodies: b)\n"
    ));
    assert!(r.contains("has no edge"), "{r}");
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

/// Face edits on chosen bodies, as .fc's `offset|move|draft .. in FEAT|all`: offset in a 2200
/// mm3, moved up 3 in a 2300, drafted 5deg in all 1956.255668 (12 faces, 24 edges, 2 bodies
/// each); moved in b, which does not hold the face, refused as .fc refused it.
#[test]
fn face_edits_in_bodies() {
    for (line, vol) in [
        ("o = offset_faces(a.end, 2, bodies: a)\n", 2200.0),
        ("o = move_faces(a.end, along: Z, d: 3, bodies: a)\n", 2300.0),
        (
            "o = draft(a.side(sa.r[1]), angle: 5deg, neutral: xy, bodies: \"all\")\n",
            1956.255668,
        ),
    ] {
        let v = shape(&format!("{TWO}{line}"));
        assert!((v.0 - vol).abs() < 1e-6, "{line}: {v:?}");
        assert_eq!((v.1, v.2, v.3), (12, 24, 2), "{line}");
    }
    let r = common::refused(&format!(
        "{TWO}o = move_faces(a.end, along: Z, d: 3, bodies: b)\n"
    ));
    assert!(r.contains("has no edge"), "{r}");
}

/// `.fc` `revolve to t ..`, t a sketch on a plane through the axis: a disc turned from xz up to
/// yz, a quarter turn (2467.4011 mm3, 3 faces, 3 edges), and `off 10` 10deg on (2741.556778);
/// t on xy, square to the axis, refused as .fc refused it (NotSquare).
#[test]
fn revolve_up_to_a_sketch_plane() {
    let turn = |on: &str, angle: &str| {
        format!(
            "sketch t(on: {on}) {{\n  p = pt(0, 0)\n}}\nsketch s(on: xz) {{\n  c = circle((20, 0), 5)\n  ax = line((0, 0), (0, 10), construction: true)\n}}\nring = revolve(s.c, axis: s.ax, angle: {angle})\n"
        )
    };
    let v = shape(&turn("yz", "upto(t)"));
    assert!((v.0 - 2467.4011).abs() < 1e-4, "{v:?}");
    assert_eq!((v.1, v.2, v.3), (3, 3, 1));
    let v = shape(&turn("yz", "upto(t, 10deg)"));
    assert!((v.0 - 2741.556778).abs() < 1e-4, "{v:?}");
    let (r, _) = common::run(&turn("xy", "upto(t)"));
    let e = format!("{:?}", r.errors);
    assert!(e.contains("NotSquare"), "{e}");
}

/// `.fc`'s `revolve .. axis` took an origin axis, a work axis or a straight edge of the part:
/// Forge's revolve takes each, where it lies in the profile's plane (a square 10 x 10, its near
/// side 10 off the axis, a quarter turn: pi / 4 * (20^2 - 10^2) * 10).
#[test]
fn revolve_about_an_axis_of_the_part_or_the_origin() {
    let quarter = std::f64::consts::PI / 4.0 * (400.0 - 100.0) * 10.0;
    let prof = "sketch s(on: xz) {
  r = rect(corner: (10, 0), size: (10, 10))
}
";
    for (pre, axis) in [("", "Z"), ("", "axis(s.r[3].a, s.r[3].b)")] {
        let src = format!("{prof}{pre}param r1.a = 90deg\nr1 = revolve(s.r, axis: {axis}, angle: a)\n");
        let (v, ..) = shape(&src);
        let want = if axis == "Z" {
            quarter
        } else {
            std::f64::consts::PI / 4.0 * 100.0 * 10.0
        };
        assert!((v - want).abs() < 1e-6, "{axis}: {v} {want}");
    }
    // out of the profile's plane: refused, typed
    let src = format!("{prof}r1 = revolve(s.r, axis: Y, angle: 90deg)\n");
    let r = forge_lang::run(&src, forge_lang::Options::default(), Default::default()).0;
    assert!(format!("{:?}", r.errors).contains("BadValue"), "{:?}", r.errors);
}
