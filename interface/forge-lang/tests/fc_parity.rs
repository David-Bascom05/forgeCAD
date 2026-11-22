//! Forms `.fc` had that Forge parts lacked, each built both ways: the `.fc` line on the retired
//! engine (`forge_agent::bench::model`) and the same geometry written as a Forge part, holding
//! the two to one solid (volume, area, faces, edges) and the Forge part valid.
mod common;

/// The `.fc` part's finished body (all its bodies as one).
fn fc(src: &str) -> forge_topo::Body {
    let m = forge_agent::bench::model(src);
    if let Some(e) = &m.error {
        panic!("fc line {}: {}", m.lines[e.at].no, e.msg);
    }
    let b = m.body().expect("a part").clone();
    assert!(forge_topo::check::check(&b).is_empty(), "fc invalid");
    b
}

/// The Forge part and the `.fc` part are one solid.
fn same(fg: &str, fc_src: &str) -> forge_lang::Measure {
    let m = common::holds(fg);
    let b = fc(fc_src);
    let f = forge_topo::mass::mass(&b).expect("fc mass");
    let near = |x: f64, y: f64| (x - y).abs() <= 1e-7 * x.abs().max(y.abs()).max(1.0);
    assert!(near(m.volume, f.volume), "volume fg {} fc {}", m.volume, f.volume);
    assert!(near(m.area, f.area), "area fg {} fc {}", m.area, f.area);
    assert_eq!(m.faces, b.face_ids().count(), "faces");
    assert_eq!(m.edges, b.edge_ids().count(), "edges");
    m
}

/// A block 40 x 30 x 20 on xy, its corner at the origin.
const BLOCK_FC: &str = "base: extrude 20 rect 0 0 40 30\n";
const BLOCK: &str = r#"
sketch plan(on: xy) {
  r = rect(corner: (0, 0), size: (40, 30))
}
base = extrude(plan, 20)
"#;

#[test]
fn chamfer_two_distances() {
    same(
        &format!(
            "{BLOCK}edge = chamfer(edges(between: (base.end, base.side(plan.r[0]))), 3, d2: 5, face: base.end)\n"
        ),
        &format!("{BLOCK_FC}chamfer 3 5 face (faces normal +z) (edges near 20 0 20)\n"),
    );
}

#[test]
fn chamfer_distance_angle() {
    same(
        &format!(
            "{BLOCK}edge = chamfer(edges(between: (base.end, base.side(plan.r[0]))), 3, angle: 30deg, face: base.end)\n"
        ),
        &format!("{BLOCK_FC}chamfer 3 ang 30 face (faces normal +z) (edges near 20 0 20)\n"),
    );
}

#[test]
fn chamfer_corner_point() {
    same(
        &format!("{BLOCK}edge = chamfer(edges(of: base.end), 2, corner: \"point\")\n"),
        &format!("{BLOCK_FC}chamfer 2 corner point (edges of base top)\n"),
    );
}

#[test]
fn chamfer_forms_refused() {
    let r = common::refused(&format!(
        "{BLOCK}edge = chamfer(edges(of: base.end), 2, d2: 1, angle: 30deg, face: base.end)\n"
    ));
    assert!(r.contains("Conflict"), "{r}");
    let r = common::refused(&format!("{BLOCK}edge = chamfer(edges(of: base.end), 2, d2: 1)\n"));
    assert!(r.contains("MissingArg"), "{r}");
    let r = common::refused(&format!(
        "{BLOCK}edge = chamfer(edges(of: base.end), 2, face: base.end)\n"
    ));
    assert!(r.contains("NotApplicable"), "{r}");
    let r = common::refused(&format!(
        "{BLOCK}edge = chamfer(edges(of: base.end), 2, corner: \"round\")\n"
    ));
    assert!(r.contains("UnknownWord"), "{r}");
}

/// A revolve's profile: a disc off the z axis on xz, turned about z.
const REV: &str = r#"
sketch s(on: xz) {
  c = circle((20, 0), 5)
  ax = line((0, 0), (0, 10), construction: true)
}
"#;

#[test]
fn revolve_symmetric() {
    same(
        &format!("{REV}ring = revolve(s.c, axis: s.ax, angle: sym(90deg))\n"),
        "revolve sym 90 xz axis 0 0 0 1 circle 20 0 5\n",
    );
}

/// The axis a plain line of the sketch, not construction: no side of the profile.
#[test]
fn revolve_about_a_plain_line() {
    same(
        "sketch s(on: xz) {\n  c = circle((20, 0), 5)\n  ax = line((0, 0), (0, 10))\n}\nring = revolve(s.c, axis: s.ax, angle: 120deg)\n",
        "revolve 120 xz axis 0 0 0 1 circle 20 0 5\n",
    );
    same(
        "sketch s(on: xz) {\n  c = circle((20, 0), 5)\n  ax = line((0, 0), (0, 10))\n}\nring = revolve(s, axis: s.ax, angle: 120deg)\n",
        "revolve 120 xz axis 0 0 0 1 circle 20 0 5\n",
    );
}

#[test]
fn cut_two_sided() {
    same(
        &format!(
            "{BLOCK}sketch s(on: base.end) {{\n  c = circle((20, 15), 4)\n}}\npit = cut(s.c, two(4, 2))\n"
        ),
        &format!("{BLOCK_FC}extrude cut two 2 4 at 0 0 20 circle 20 15 4\n"),
    );
}

#[test]
fn revolve_two_sided() {
    same(
        &format!("{REV}ring = revolve(s.c, axis: s.ax, angle: two(60deg, 30deg))\n"),
        "revolve two 60 30 xz axis 0 0 0 1 circle 20 0 5\n",
    );
}

#[test]
fn revolve_keep() {
    same(
        &format!(
            "{BLOCK}sketch s(on: base.side(plan.r[0])) {{\n  c = circle((20, 10), 8)\n  ax = line((0, 0), (40, 0), construction: true)\n}}\nring = revolve(s.c, axis: s.ax, keep: true)\n"
        ),
        &format!("{BLOCK_FC}revolve keep xz axis 0 0 1 0 circle 20 10 8\n"),
    );
}

#[test]
fn extrude_along() {
    same(
        r#"
sketch slant(on: xz) {
  l = line((0, 0), (1, 1), construction: true)
}
sketch plan(on: xy) {
  r = rect(corner: (0, 0), size: (40, 30))
}
base = extrude(plan.r, 20, along: slant.l.dir)
sketch top(on: base.end) {
  c = circle((20, 15), 5)
  d = circle((8, 8), 3)
}
boss = extrude(top.c, sym(10), along: slant.l.dir)
pit = cut(top.d, two(4, 2), along: slant.l.dir)
sketch under(on: plan) {
  c = circle((25, 15), 3)
}
bore = cut(under.c, thru, flip: true, along: slant.l.dir)
"#,
        "extrude 20 along 1 0 1 rect 0 0 40 30\n\
         extrude sym 10 along 1 0 1 at 0 0 20 circle 20 15 5\n\
         extrude cut two 2 4 along 1 0 1 at 0 0 20 circle 8 8 3\n\
         extrude cut thru along 1 0 1 circle 25 15 3\n",
    );
}

#[test]
fn bodies_on_cut_and_extrude() {
    // two blocks; the second cut and extruded on the first body, not the last made
    same(
        &format!(
            "{BLOCK}sketch p2(on: plan) {{\n  r = rect(corner: (60, 0), size: (20, 20))\n}}\nother = extrude(p2, 10, new: true)\nsketch s(on: base.end) {{\n  c = circle((20, 15), 4)\n  d = circle((8, 8), 3)\n}}\npit = cut(s.c, 5, bodies: base)\nboss = extrude(s.d, 3, bodies: base)\nsketch q(on: plan) {{\n  c = circle((20, 10), 4)\n  ax = line((0, 0), (40, 0), construction: true)\n}}\nring = revolve(q.c, axis: q.ax, angle: 90deg, cut: true, bodies: base)\nsketch bp(on: base.end) {{\n  p = pt(32, 22)\n}}\ndent = ball(bp.p, 3, cut: true, bodies: base)\n"
        ),
        &format!(
            "{BLOCK_FC}other: extrude new 10 rect 60 0 20 20\nextrude cut 5 -xy at 0 0 20 circle 20 -15 4 in base\nextrude 3 at 0 0 20 circle 8 8 3 in base\nrevolve cut 90 axis x circle 20 10 4 in base\nball cut 3 at 32 22 20 in base\n"
        ),
    );
}

#[test]
fn extrude_flip_thru_and_next() {
    // a cut through the part along the sketch's normal (a plain cut runs against it)
    same(
        &format!(
            "{BLOCK}sketch s(on: plan) {{\n  c = circle((20, 15), 4)\n}}\nbore = cut(s.c, thru, flip: true)\n"
        ),
        &format!("{BLOCK_FC}extrude cut thru circle 20 15 4\n"),
    );
    // a post above the part, down to the face it meets first
    same(
        &format!(
            "{BLOCK}sketch s(on: offset(base.end, 10)) {{\n  c = circle((20, 15), 4)\n}}\npost = extrude(s.c, next, flip: true)\n"
        ),
        &format!("{BLOCK_FC}extrude next back at 0 0 30 circle 20 15 4\n"),
    );
}

/// Sketch rows `hdist` and `vdist`: a point placed by its x and y offsets from another.
#[test]
fn sketch_horizontal_and_vertical_distances() {
    same(
        "sketch s(on: xy) {\n  r = rect(corner: (0, 0), size: (40, 30))\n  o = pt(0, 0)\n  q = near(12, 9)\n  c = circle(q, 4)\n  fix(o)\n  hdistance(o, q, 15)\n  vdistance(o, q, 10)\n}\nbase = extrude(s, 10)\n",
        "base: extrude 10 rect 0 0 40 30 circle 15 10 4\n",
    );
}

/// Selections .fc had: radius under or over a value, touching an entity, in a body, spline faces.
#[test]
fn selections_by_radius_touching_and_body() {
    let src = format!(
        "{BLOCK}sketch h(on: base.end) {{\n  a = circle((10, 10), 2)\n  b = circle((30, 15), 5)\n}}\nholes = cut(h.a, 5)\nholes2 = cut(h.b, 5)\nsmall = chamfer(edges(of: [holes, holes2], shape: \"circle\", r_below: 3), 0.5)\n"
    );
    same(
        &src,
        &format!(
            "{BLOCK_FC}holes: extrude cut 5 -xy at 0 0 20 circle 10 -10 2\nholes2: extrude cut 5 -xy at 0 0 20 circle 30 -15 5\nchamfer 0.5 (edges of holes holes2 circle r<3)\n"
        ),
    );
    let src = format!(
        "{BLOCK}sketch h(on: base.end) {{\n  a = circle((10, 10), 2)\n  b = circle((30, 15), 5)\n}}\nholes = cut(h.a, 5)\nholes2 = cut(h.b, 5)\nbig = chamfer(edges(of: [holes, holes2], shape: \"circle\", r_above: 3), 0.5)\n"
    );
    same(
        &src,
        &format!(
            "{BLOCK_FC}holes: extrude cut 5 -xy at 0 0 20 circle 10 -10 2\nholes2: extrude cut 5 -xy at 0 0 20 circle 30 -15 5\nchamfer 0.5 (edges of holes holes2 circle r>3)\n"
        ),
    );
    // the top's edges: those touching the top face
    same(
        &format!("{BLOCK}rim = chamfer(edges(touching: base.end, along: X), 1)\n"),
        &format!("{BLOCK_FC}chamfer 1 (edges along +x top)\n"),
    );
    // two bodies: the second's top edges only
    same(
        &format!(
            "{BLOCK}sketch p2(on: plan) {{\n  r = rect(corner: (60, 0), size: (20, 20))\n}}\nother = extrude(p2, 10, new: true)\nrim = chamfer(edges(body: other, along: Z), 1)\n"
        ),
        &format!("{BLOCK_FC}other: extrude new 10 rect 60 0 20 20\nchamfer 1 (edges along +z body other)\n"),
    );
}

/// `.fc`'s `suppress` and `unsuppress`: the feature and what reads it out of the build and back.
#[test]
fn suppress_and_unsuppress() {
    use forge_lang::edit::apply;
    use forge_lang::error::Op;
    let src = format!(
        "{BLOCK}param boss.h = 5\nsketch s(on: base.end) {{\n  c = circle((20, 15), 4)\n}}\nboss = extrude(s.c, h)\nround = fillet(edges(of: boss, crease: \"concave\"), 1)\nexpect volume > 0\n"
    );
    let out = apply(&src, &Op::Suppress { id: "boss".into() }).expect("suppressed");
    assert!(out.contains("#~boss boss = extrude"), "{out}");
    assert!(out.contains("#~boss round = fillet"), "{out}");
    assert!(out.contains("#~boss param boss.h"), "{out}");
    let m = common::holds(&out);
    assert!((m.volume - 40.0 * 30.0 * 20.0).abs() < 1e-6, "{}", m.volume);
    let back = apply(&out, &Op::Unsuppress { id: "boss".into() }).expect("back");
    assert_eq!(back, src);
    assert!(apply(&back, &Op::Unsuppress { id: "boss".into() }).is_err());
}

/// `.fc`'s `try:` for a misspelt name: the nearest name in reach, as a Respell fix.
#[test]
fn a_misspelt_name_is_respelt() {
    let src = format!("{BLOCK}sketch s(on: bsae.end) {{\n  c = circle((20, 15), 4)\n}}\n");
    let (r, _) = common::run(&src);
    let e = r.errors.first().expect("refused");
    let fixes = e.fixes();
    let [fix] = &fixes[..] else {
        panic!("{e:?}: {fixes:?}")
    };
    let fixed = forge_lang::edit::apply(&src, fix).expect("applies");
    assert!(fixed.contains("on: base.end"), "{fixed}");
    common::holds(&fixed);
    // a misspelt function too
    let src = format!("{BLOCK}edge = chamfr(edges(of: base.end), 1)\n");
    let (r, _) = common::run(&src);
    let fixed = forge_lang::edit::apply(&src, &r.errors[0].fixes()[0]).expect("applies");
    common::holds(&fixed);
}

/// `.fc`'s `try:` for a round too big for where it goes: the largest radius that builds, as a
/// SetParam fix on the param driving it (`Options::fit_sizes`).
#[test]
fn a_round_too_big_gets_the_largest_that_builds() {
    let src = format!("{BLOCK}param round.r = 25\nround = fillet(edges(of: base.end), r)\n");
    let dir = std::env::temp_dir();
    let opts = forge_lang::Options {
        dir: Some(dir),
        fit_sizes: true,
        ..forge_lang::Options::default()
    };
    let (r, _) = forge_lang::run(&src, opts, Default::default());
    let e = r.errors.first().expect("refused");
    let [fix @ forge_lang::error::Op::SetParam { value, .. }] = &e.fixes()[..] else {
        panic!("{e:?}")
    };
    assert!(value.value > 1.0 && value.value < 25.0, "{}", value.value);
    common::holds(&forge_lang::edit::apply(&src, fix).expect("applies"));
}
