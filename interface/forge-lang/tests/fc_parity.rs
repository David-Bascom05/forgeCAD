//! Extents, selections, suppression and fixes, each a Forge part held to a recorded solid (its
//! volume, area, faces and edges, below) and valid.
mod common;

/// The recorded solids: each one's key text, then its volume, area, faces and edges.
const RECORDED: &[(&str, f64, f64, usize, usize)] = &[
    (
        "base: extrude 20 rect 0 0 40 30\nother: extrude new 10 rect 60 0 20 20\nextrude cut 5 -xy at 0 0 20 circle 20 -15 4 in base\nextrude 3 at 0 0 20 circle 8 8 3 in base\nrevolve cut 90 axis x circle 20 10 4 in base\nball cut 3 at 32 22 20 in base\n",
        26987.378569507975,
        7304.739918919216,
        18,
        35,
    ),
    (
        "base: extrude 20 rect 0 0 40 30\nchamfer 2 corner point (edges of base top)\n",
        23730.666666666664,
        5029.3523804664965,
        10,
        20,
    ),
    (
        "base: extrude 20 rect 0 0 40 30\nchamfer 3 ang 30 face (faces normal +z) (edges near 20 0 20)\n",
        23896.076951545867,
        5144.085879880048,
        7,
        15,
    ),
    (
        "base: extrude 20 rect 0 0 40 30\nchamfer 3 5 face (faces normal +z) (edges near 20 0 20)\n",
        23700.0,
        5098.238075793812,
        7,
        15,
    ),
    (
        "base: extrude 20 rect 0 0 40 30\nextrude cut two 2 4 at 0 0 20 circle 20 15 4\n",
        23798.938070170254,
        5300.530964914873,
        8,
        15,
    ),
    (
        "extrude 20 along 1 0 1 rect 0 0 40 30\nextrude sym 10 along 1 0 1 at 0 0 20 circle 20 15 5\nextrude cut two 2 4 along 1 0 1 at 0 0 20 circle 8 8 3\nextrude cut thru along 1 0 1 circle 25 15 3\n",
        24560.77428866578,
        6014.321433980047,
        13,
        30,
    ),
    (
        "base: extrude 20 rect 0 0 40 30\nextrude cut thru circle 20 15 4\n",
        22994.69035085127,
        5602.123859659494,
        7,
        15,
    ),
    (
        "base: extrude 20 rect 0 0 40 30\nextrude next back at 0 0 30 circle 20 15 4\n",
        24502.654824574365,
        5451.3274122871835,
        8,
        15,
    ),
    (
        "revolve 120 xz axis 0 0 0 1 circle 20 0 5\n",
        3289.868133696452,
        1473.0268861580705,
        3,
        3,
    ),
    (
        "base: extrude 20 rect 0 0 40 30\nrevolve keep xz axis 0 0 1 0 circle 20 10 8\n",
        3158.273408348594,
        1191.692211746642,
        3,
        3,
    ),
    (
        "revolve sym 90 xz axis 0 0 0 1 circle 20 0 5\n",
        2467.401100272338,
        1144.0400727884253,
        3,
        3,
    ),
    (
        "revolve two 60 30 xz axis 0 0 0 1 circle 20 0 5\n",
        2467.4011002723387,
        1144.040072788425,
        3,
        3,
    ),
    (
        "base: extrude 20 rect 0 0 40 30\nholes: extrude cut 5 -xy at 0 0 20 circle 10 -10 2\nholes2: extrude cut 5 -xy at 0 0 20 circle 30 -15 5\nchamfer 0.5 (edges of holes holes2 circle r<3)\n",
        23544.20726584168,
        5412.550276275201,
        12,
        22,
    ),
    (
        "base: extrude 20 rect 0 0 40 30\nholes: extrude cut 5 -xy at 0 0 20 circle 10 -10 2\nholes2: extrude cut 5 -xy at 0 0 20 circle 30 -15 5\nchamfer 0.5 (edges of holes holes2 circle r>3)\n",
        23544.207265841676,
        5401.508462061073,
        12,
        22,
    ),
    (
        "base: extrude 20 rect 0 0 40 30\nchamfer 1 (edges along +x top)\n",
        23960.0,
        5151.137084989847,
        8,
        18,
    ),
    (
        "base: extrude 20 rect 0 0 40 30\nother: extrude new 10 rect 60 0 20 20\nchamfer 1 (edges along +z body other)\n",
        27979.999999999996,
        6772.568542494922,
        16,
        36,
    ),
    (
        "base: extrude 10 rect 0 0 40 30 circle 15 10 4\n",
        11497.345175425635,
        3950.79644737231,
        7,
        15,
    ),
];

/// The Forge part builds the solid recorded under `fc_src`.
fn same(fg: &str, fc_src: &str) -> forge_lang::Measure {
    let m = common::holds(fg);
    let &(_, volume, area, faces, edges) = RECORDED
        .iter()
        .find(|r| r.0 == fc_src)
        .unwrap_or_else(|| panic!("no recorded solid for {fc_src:?}"));
    let near = |x: f64, y: f64| (x - y).abs() <= 1e-7 * x.abs().max(y.abs()).max(1.0);
    assert!(near(m.volume, volume), "volume fg {} want {}", m.volume, volume);
    assert!(near(m.area, area), "area fg {} want {}", m.area, area);
    assert_eq!(m.faces, faces, "faces");
    assert_eq!(m.edges, edges, "edges");
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

/// Selections: radius under or over a value, touching an entity, in a body, spline faces.
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

/// Suppress and unsuppress: the feature and what reads it out of the build and back.
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

/// Suppress takes out exactly the statements it names, a line shared with ones kept split
/// (Suppress.lean: `suppress_live`; it took the whole line, `same_line_drops`), and Unsuppress
/// gives the text back byte for byte, whichever of two features sharing a line goes back first
/// (`unsuppress_suppress`).
#[test]
fn suppress_takes_one_statement_of_a_line() {
    use forge_lang::edit::apply;
    use forge_lang::error::Op;
    let sup = |id: &str| Op::Suppress { id: id.into() };
    let unsup = |id: &str| Op::Unsuppress { id: id.into() };
    let ids = |src: &str| -> Vec<String> { common::run(src).0.feats.iter().map(|f| f.id.clone()).collect() };
    let src = format!(
        "{BLOCK}a = extrude(plan, 2, new: true); b = extrude(plan, 3, new: true); c = extrude(plan, 4, new: true)  # three\nd = extrude(plan, 5, new: true);e = extrude(plan, 6, new: true)\n"
    );
    let all = ids(&src);
    for one in ["a", "b", "c", "d", "e"] {
        let out = apply(&src, &sup(one)).expect("suppressed");
        let want: Vec<_> = all.iter().filter(|x| *x != one).cloned().collect();
        assert_eq!(ids(&out), want, "{out}");
        assert_eq!(apply(&out, &unsup(one)).expect("back"), src, "{out}");
    }
    // two of one line, put back in either order
    for (x, y) in [
        ("a", "b"),
        ("b", "a"),
        ("a", "c"),
        ("c", "a"),
        ("b", "c"),
        ("d", "e"),
        ("e", "d"),
    ] {
        let two = apply(&apply(&src, &sup(x)).expect("x"), &sup(y)).expect("y");
        let want: Vec<_> = all.iter().filter(|i| *i != x && *i != y).cloned().collect();
        assert_eq!(ids(&two), want, "{two}");
        for (p, q) in [(x, y), (y, x)] {
            let one = apply(&two, &unsup(p)).expect("p back");
            let want: Vec<_> = all.iter().filter(|i| *i != q).cloned().collect();
            assert_eq!(ids(&one), want, "{x} {y}, {p} first:\n{two}---\n{one}");
            assert_eq!(
                apply(&one, &unsup(q)).expect("q back"),
                src,
                "{x} {y}, {p} first:\n{one}"
            );
        }
    }
}

/// Respell takes a read of a name, the whole token: a binder (Rename's) and a part of a token
/// are refused (Respell.lean: `respell_refuses_binder`, `respell_refuses_inside`).
#[test]
fn respell_refuses_a_binder_and_a_part_of_a_token() {
    use forge_lang::edit::apply;
    use forge_lang::error::Op;
    use forge_lang::syntax::Span;
    let src = format!("{BLOCK}edge = chamfer(edges(of: base.end), 1)\n");
    let at = |w: &str| src.find(w).expect("in the text") as u32;
    let respell = |lo: u32, hi: u32, to: &str| {
        apply(
            &src,
            &Op::Respell {
                span: Span { lo, hi },
                to: to.into(),
            },
        )
    };
    // `base` the binder, and `as` inside it
    let b = at("base =");
    assert_eq!(
        respell(b, b + 4, "plate").expect_err("a binder").kind(),
        "NotARead"
    );
    assert_eq!(
        respell(b + 1, b + 3, "x").expect_err("inside a token").kind(),
        "NotARead"
    );
    // a member and a keyword argument's name are not reads either
    let r = at("base.end");
    assert_eq!(
        respell(r + 5, r + 8, "start").expect_err("a member").kind(),
        "NotARead"
    );
    let o = at("of:");
    assert_eq!(
        respell(o, o + 2, "on").expect_err("an argument name").kind(),
        "NotARead"
    );
    assert_eq!(
        respell(r, r + 3, "bas").expect_err("part of a read").kind(),
        "NotARead"
    );
    // the read itself
    let out = respell(r, r + 4, "plan").expect("a read");
    assert!(
        out.contains("edges(of: plan.end)") && out.contains("base = extrude"),
        "{out}"
    );
}

/// Two features whose ids hash to one kernel name (HistoryId.lean: `historyId_collides`) are a
/// typed error naming both, not a fillet that rounds the other's edges too
/// (`checked_distinct`).
#[test]
fn a_history_id_clash_is_refused() {
    let (a, b) = ("qolagjaapenjgkgca", "qpibohgembfgikgid");
    assert_eq!(forge_lang::build::history_id(a), forge_lang::build::history_id(b));
    let src = format!(
        "{BLOCK}sketch s(on: base.end) {{\n  c = circle((20, 15), 4)\n}}\n{a} = extrude(s.c, 5)\n{b} = extrude(s.c, 3, new: true)\nround = fillet(edges(of: {a}, crease: \"convex\"), 1)\n"
    );
    let (r, _) = common::run(&src);
    let e = r
        .errors
        .iter()
        .find(|e| e.kind() == "HistoryIdClash")
        .expect("refused");
    let text = e.text();
    assert!(text.contains(a) && text.contains(b), "{text}");
    assert_eq!(
        e.span().map(|s| &src[s.lo as usize..s.hi as usize]),
        Some("extrude(s.c, 3, new: true)")
    );
    // other names: it builds
    common::holds(&src.replace(a, "boss").replace(b, "post"));
}

/// The typo fix for a name: at equal distance a name of the part before a built-in (`tt` with a
/// param `t` is `t`, not the built-in `pt`), then the alphabetically first.
#[test]
fn a_typo_respells_to_the_parts_name_before_a_builtin() {
    let src = "shared t = 6\nsketch plan(on: xy) {\n  r = rect(corner: (0, 0), size: (40, 30))\n}\nbase = extrude(plan, tt)\n";
    let (r, _) = common::run(src);
    let e = r.errors.first().expect("refused");
    let [fix @ forge_lang::error::Op::Respell { to, .. }] = &e.fixes()[..] else {
        panic!("{e:?}")
    };
    assert_eq!(to, "t");
    common::holds(&forge_lang::edit::apply(src, fix).expect("applies"));
    // between two built-ins, or two names of the part, the alphabetically first
    assert_eq!(
        forge_lang::edit_ops::nearest("tt", ["t", "pt"].into_iter()).as_deref(),
        Some("t")
    );
    assert_eq!(
        forge_lang::edit_ops::nearest("tt", ["pt", "t"].into_iter()).as_deref(),
        Some("t")
    );
    assert_eq!(
        forge_lang::edit_ops::nearest("tt", ["tu", "ta"].into_iter()).as_deref(),
        Some("ta")
    );
}

/// A misspelt name: the nearest name in reach, as a Respell fix.
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

/// A round too big for where it goes: the largest radius that builds, as a
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
