//! The wired features, one by one: holes (every head, end and size option), threads (on a
//! hole, a face, an axis), patterns (each layout), work features (every form), bodies (new,
//! combine, split, move, copy, a feature on chosen bodies), text, imports, projections and
//! direct edits; their role anchors; and their typed errors. Numbers are by hand where they can
//! be, else against the same part built a known way.

use forge_lang::error::Error;
use forge_lang::{Options, Run};
use std::f64::consts::PI;

const BASE: &str = "shared L = 60\nshared t = 10\nsketch b(on: xy) {\n  outline = rect(center: (0, 0), size: (L, L))\n  o = pt(0, 0)\n}\nbase = extrude(b, t)\n";

fn run_in(src: &str, dir: Option<std::path::PathBuf>) -> Run {
    let opts = Options {
        dir,
        ..Options::default()
    };
    forge_lang::run(src, opts, Default::default()).0
}

fn run(src: &str) -> Run {
    run_in(src, None)
}

/// Builds with no errors (lints allowed: these parts are small and literal).
fn ok(src: &str) -> Run {
    let r = run(src);
    assert!(r.errors.is_empty(), "{}", forge_lang::report::text(&r, src));
    r
}

fn vol(r: &Run) -> f64 {
    r.measure.as_ref().expect("measured").volume
}

fn base_vol() -> f64 {
    60.0 * 60.0 * 10.0
}

/// The kinds of the errors, and of the wired ones the kind inside.
fn errs(src: &str) -> Vec<&'static str> {
    run(src).errors.iter().map(Error::kind).collect()
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-6 * b.abs().max(1.0), "{a} vs {b}");
}

// ---- holes ------------------------------------------------------------------------------------

const TOP: &str = "sketch top(on: base.end) {\n  p = pt(0, 0)\n  q = pt(15, 15)\n  s = pt(-15, 15)\n}\n";

#[test]
fn a_plain_hole_through_and_blind() {
    let r = ok(&format!("{BASE}{TOP}h = hole(at: top.p, d: 6, depth: thru)\n"));
    close(vol(&r), base_vol() - PI * 9.0 * 10.0);
    let r = ok(&format!("{BASE}{TOP}h = hole(at: top.p, d: 6, depth: 4)\n"));
    close(vol(&r), base_vol() - PI * 9.0 * 4.0);
    // a drill point: a cone of the included angle below the depth
    let r = ok(&format!(
        "{BASE}{TOP}h = hole(at: top.p, d: 6, depth: 4, point: 90deg)\n"
    ));
    close(vol(&r), base_vol() - PI * 9.0 * 4.0 - PI * 9.0 * 3.0 / 3.0);
}

#[test]
fn counterbores_and_countersinks() {
    let r = ok(&format!(
        "{BASE}{TOP}h = hole(at: top.p, d: 6, cbore: (10, 3), depth: thru)\n"
    ));
    close(vol(&r), base_vol() - PI * 9.0 * 7.0 - PI * 25.0 * 3.0);
    // a 90 degree sink from 10 down to 6: a frustum 2 deep
    let r = ok(&format!(
        "{BASE}{TOP}h = hole(at: top.p, d: 6, csink: (10, 90deg), depth: thru)\n"
    ));
    let frustum = PI * 2.0 / 3.0 * (25.0 + 15.0 + 9.0);
    close(vol(&r), base_vol() - PI * 9.0 * 8.0 - frustum);
}

#[test]
fn screws_from_the_tables() {
    // M6 normal clearance 6.6, close 6.4 (ISO 273), tap 5 with the thread recorded
    for (fit, d) in [("normal", 6.6), ("close", 6.4), ("tap", 5.0)] {
        let r = ok(&format!(
            "{BASE}{TOP}h = hole(at: top.p, screw: \"M6\", fit: \"{fit}\", depth: thru)\n"
        ));
        close(vol(&r), base_vol() - PI * d * d / 4.0 * 10.0);
    }
    ok(&format!(
        "{BASE}{TOP}h = hole(at: top.p, screw: \"M6\", seat: \"cbore\", depth: thru)\n"
    ));
    ok(&format!(
        "{BASE}{TOP}h = hole(at: top.p, screw: \"M6\", seat: \"csink\", depth: thru)\n"
    ));
    ok(&format!(
        "{BASE}{TOP}h = hole(at: top.p, screw: \"1/4-20\", convention: \"ansi\", fit: \"tap\", depth: 8)\n"
    ));
    // the recorded thread is what a thread feature then models
    let r = ok(&format!(
        "{BASE}{TOP}h = hole(at: top.p, screw: \"M6\", fit: \"tap\", depth: thru)\nth = thread(hole: h, length: \"full\")\n"
    ));
    assert!(vol(&r) < base_vol() - PI * 6.25 * 10.0);
}

#[test]
fn hole_ends_next_upto_and_flip() {
    let wp = "wp = workplane(offset: base.end, d: -4)\n";
    let r = ok(&format!(
        "{BASE}{TOP}{wp}h = hole(at: top.p, d: 6, depth: upto(wp))\n"
    ));
    close(vol(&r), base_vol() - PI * 9.0 * 4.0);
    let r = ok(&format!("{BASE}{TOP}h = hole(at: top.p, d: 6, depth: next)\n"));
    close(vol(&r), base_vol() - PI * 9.0 * 10.0);
    // drilled from the underside up, with the bottom as its mouth
    let r = ok(&format!(
        "{BASE}sketch und(on: base.start) {{\n  p = pt(0, 0)\n}}\nh = hole(at: und.p, d: 6, depth: 4)\n"
    ));
    close(vol(&r), base_vol() - PI * 9.0 * 4.0);
    // flip: with the sketch normal, from a plane outside the part back into it
    let r = ok(&format!(
        "{BASE}sketch und(on: offset(base.end, -20)) {{\n  p = pt(0, 0)\n}}\nh = hole(at: und.p, d: 6, depth: 14, flip: true)\n"
    ));
    close(vol(&r), base_vol() - PI * 9.0 * 4.0);
}

#[test]
fn holes_at_several_places_and_on_a_plane() {
    let r = ok(&format!(
        "{BASE}{TOP}h = hole(at: [top.p, top.q, top.s], d: 4, depth: thru)\n"
    ));
    close(vol(&r), base_vol() - 3.0 * PI * 4.0 * 10.0);
    // points of another plane, and work points, dropped onto the plane given
    let r = ok(&format!(
        "{BASE}{TOP}wq = workpoint(at: top.q)\nh = hole(at: b.o, d: 4, depth: thru, on: base.end)\ng = hole(at: wq, d: 4, depth: thru, on: base.start)\n"
    ));
    close(vol(&r), base_vol() - 2.0 * PI * 4.0 * 10.0);
    assert_eq!(
        errs(&format!(
            "{BASE}{TOP}wq = workpoint(at: top.q)\ng = hole(at: wq, d: 4, depth: thru)\n"
        )),
        vec!["MissingArg"]
    );
}

/// A role anchor that resolves to faces: a chamfer on its edges may refuse the geometry, but its
/// selection is not empty.
fn picks(src: &str) -> bool {
    !errs(src).contains(&"EmptySelection")
}

#[test]
fn hole_roles_and_rim() {
    let r = ok(&format!(
        "{BASE}{TOP}h = hole(at: [top.q, top.s], d: 6, cbore: (10, 3), depth: 8)\nc = chamfer(h.rim, 0.5)\n"
    ));
    // both places' rims: two circles of the counterbore, each chamfered
    let plain = vol(&ok(&format!(
        "{BASE}{TOP}h = hole(at: [top.q, top.s], d: 6, cbore: (10, 3), depth: 8)\n"
    )));
    assert!(vol(&r) < plain);
    for role in ["wall", "floor", "bore", "seat"] {
        let s = format!(
            "{BASE}{TOP}h = hole(at: top.q, d: 6, cbore: (10, 3), depth: 8, point: 118deg)\nc = chamfer(edges(of: h.{role}), 0.2)\n"
        );
        assert!(picks(&s), "{role}");
    }
    let s = format!(
        "{BASE}{TOP}h = hole(at: top.q, d: 6, csink: (10, 90deg), depth: 8)\nc = chamfer(edges(of: h.sink), 0.2)\n"
    );
    assert!(picks(&s));
    // planar roles carry their plane: a sketch sits on the seat
    ok(&format!(
        "{BASE}{TOP}h = hole(at: top.q, d: 6, cbore: (10, 3), depth: 8)\nsketch ss(on: h.seat) {{\n  c = circle((0, 0), 1)\n}}\n"
    ));
    // a role the hole has no face for is a typed error
    let s = format!("{BASE}{TOP}h = hole(at: top.q, d: 6, depth: 8)\nc = chamfer(edges(of: h.sink), 0.2)\n");
    assert_eq!(errs(&s), vec!["NoFace"]);
}

#[test]
fn hole_options_that_cannot_apply_are_errors() {
    let e = |extra: &str| errs(&format!("{BASE}{TOP}h = hole(at: top.p, {extra})\n"));
    assert_eq!(
        e("d: 6, cbore: (10, 3), csink: (10, 90deg), depth: thru"),
        vec!["Conflict"]
    );
    assert_eq!(e("d: 6, screw: \"M6\", depth: thru"), vec!["Conflict"]);
    assert_eq!(e("d: 6, fit: \"tap\", depth: thru"), vec!["NotApplicable"]);
    assert_eq!(e("d: 6, depth: thru, point: 118deg"), vec!["NotApplicable"]);
    assert_eq!(e("screw: \"M7x9x\", depth: thru"), vec!["UnknownSize"]);
    assert_eq!(
        e("screw: \"M6\", fit: \"snug\", depth: thru"),
        vec!["UnknownWord"]
    );
    assert_eq!(e("screw: \"M6\", thread: \"M6\", depth: thru"), vec!["Conflict"]);
    assert_eq!(e("d: 6, depth: sym(4)"), vec!["WrongType"]);
}

// ---- threads ----------------------------------------------------------------------------------

#[test]
fn threads_on_a_hole_a_face_and_an_axis() {
    let hole = format!("{BASE}{TOP}h = hole(at: top.p, d: 8.5, depth: thru, thread: \"M10\")\n");
    let plain = vol(&ok(&hole));
    for opts in [
        "length: \"full\"",
        "length: 6",
        "length: 6, left: true",
        "length: 6, chamfer: true",
        "length: 6, runout: true",
    ] {
        let r = ok(&format!("{hole}th = thread(hole: h, {opts})\n"));
        assert!(vol(&r) < plain, "{opts}");
    }
    for role in ["flanks", "root", "chamfer"] {
        let s = format!(
            "{hole}th = thread(hole: h, length: 6, chamfer: true)\nf = chamfer(edges(of: th.{role}), 0.05)\n"
        );
        assert!(picks(&s), "{role}");
    }
    // round a boss: on its face, and on an axis from a plane
    let boss = format!(
        "{BASE}sketch bk(on: base.end) {{\n  c = circle((0, 0), 5)\n  p = pt(0, 0)\n}}\nboss = extrude(bk.c, 20)\n"
    );
    let plain = vol(&ok(&boss));
    let r = ok(&format!(
        "{boss}th = thread(face: boss.side(bk.c), spec: \"M10\", length: 12)\n"
    ));
    assert!(vol(&r) < plain);
    let r = ok(&format!(
        "{boss}th = thread(on: boss.end, at: bk.p, spec: (10, 1.5), length: 12)\n"
    ));
    assert!(vol(&r) < plain);
}

#[test]
fn thread_forms_are_typed() {
    let hole = format!("{BASE}{TOP}h = hole(at: top.p, d: 8.5, depth: thru, thread: \"M10\")\n");
    assert_eq!(errs(&format!("{hole}th = thread(length: 6)\n")), vec!["Forms"]);
    assert_eq!(
        errs(&format!("{hole}th = thread(hole: h, on: base.end, length: 6)\n")),
        vec!["Forms"]
    );
    assert_eq!(
        errs(&format!("{hole}th = thread(hole: h, spec: \"M10\", length: 6)\n")),
        vec!["NotApplicable"]
    );
    assert_eq!(
        errs(&format!(
            "{hole}th = thread(hole: h, internal: true, length: 6)\n"
        )),
        vec!["NotApplicable"]
    );
    // a hole with no recorded thread: the kernel's typed refusal
    let plain = format!("{BASE}{TOP}h = hole(at: top.p, d: 8.5, depth: thru)\n");
    assert_eq!(
        errs(&format!("{plain}th = thread(hole: h, length: 6)\n")),
        vec!["Build"]
    );
}

// ---- patterns ---------------------------------------------------------------------------------

const PEG: &str = "sketch pk(on: base.end) {\n  c = circle((-20, -20), 3)\n}\npeg = extrude(pk, 5)\n";

fn peg() -> f64 {
    PI * 9.0 * 5.0
}

#[test]
fn every_layout() {
    let p = |layout: &str| {
        vol(&ok(&format!(
            "{BASE}{PEG}pp = pattern(of: peg, layout: {layout})\n"
        )))
    };
    close(
        p("lin(along: b.outline[0].dir, n: count(3), step: 10)"),
        base_vol() + 3.0 * peg(),
    );
    close(
        p(
            "lin(along: b.outline[0].dir, n: count(3), step: 10, along2: b.outline[1].dir, n2: count(2), step2: 10)",
        ),
        base_vol() + 6.0 * peg(),
    );
    let ax = "ax = workaxis(meet: (base.side(b.outline[0]), base.side(b.outline[1])))\n";
    let r = ok(&format!(
        "{BASE}{PEG}{ax}pp = pattern(of: peg, layout: circ(about: ax, n: count(4)))\n"
    ));
    close(vol(&r), base_vol() + 4.0 * peg());
    let mid = "m = workplane(mid: (base.side(b.outline[1]), base.side(b.outline[3])))\n";
    let r = ok(&format!(
        "{BASE}{PEG}{mid}pp = pattern(of: peg, layout: mirror(on: m))\n"
    ));
    close(vol(&r), base_vol() + 2.0 * peg());
    let r = ok(&format!(
        "{BASE}{TOP}{PEG}pp = pattern(of: peg, layout: places(at: [pk.c.center, top.q]))\n"
    ));
    close(vol(&r), base_vol() + 2.0 * peg());
}

#[test]
fn circular_patterns_over_and_each() {
    let ax = "ax = workaxis(through: (b.o, top.p))\n";
    let src = |l: &str| format!("{BASE}{TOP}{ax}{PEG}pp = pattern(of: peg, layout: {l})\n");
    close(
        vol(&ok(&src("circ(about: ax, n: count(4))"))),
        base_vol() + 4.0 * peg(),
    );
    close(
        vol(&ok(&src("circ(about: ax, n: count(2), over: 90deg)"))),
        base_vol() + 2.0 * peg(),
    );
    close(
        vol(&ok(&src("circ(about: ax, n: count(3), each: 90deg)"))),
        base_vol() + 3.0 * peg(),
    );
    assert_eq!(
        errs(&src("circ(about: ax, n: count(3), over: 90deg, each: 90deg)")),
        vec!["Conflict"]
    );
}

#[test]
fn along_a_curve_and_an_axis() {
    let src = format!(
        "{BASE}{PEG}pp = pattern(of: peg, layout: follow(curve: edges(between: (base.end, base.side(b.outline[0]))), n: count(3), step: 10, fixed: true))\n"
    );
    close(vol(&ok(&src)), base_vol() + 3.0 * peg());
    let src = format!(
        "{BASE}{PEG}e = workaxis(meet: (base.end, base.side(b.outline[3])))\npp = pattern(of: peg, layout: follow(curve: e, n: count(2), step: 20))\n"
    );
    close(vol(&ok(&src)), base_vol() + 2.0 * peg());
}

#[test]
fn a_pattern_of_the_whole_part_and_of_several_features() {
    let src = format!(
        "{BASE}{PEG}{TOP}h = hole(at: top.q, d: 4, depth: thru)\npp = pattern(of: [peg, h], layout: lin(along: b.outline[1].dir, n: count(2), step: 10))\n"
    );
    close(vol(&ok(&src)), base_vol() + 2.0 * peg() - 2.0 * PI * 4.0 * 10.0);
    let src = format!(
        "{BASE}pp = pattern(whole: true, layout: lin(along: b.outline[0].dir, n: count(2), step: 60))\n"
    );
    close(vol(&ok(&src)), 2.0 * base_vol());
    assert_eq!(
        errs(&format!("{BASE}{PEG}pp = pattern(layout: mirror(on: b))\n")),
        vec!["Forms"]
    );
}

#[test]
fn world_directions_in_a_layout_are_the_world_lint() {
    let r = run(&format!(
        "{BASE}{PEG}pp = pattern(of: peg, layout: lin(along: X, n: count(2), step: 10))\n"
    ));
    assert!(r.errors.is_empty());
    assert!(
        r.warnings
            .iter()
            .any(|w| format!("{w:?}").contains("WorldAfterFirst"))
    );
}

#[test]
fn pattern_copies_keep_their_names_when_the_count_changes() {
    // the copies are named by the pattern's id and the copy's index: a fillet on the second
    // copy's edges picks the same named edges at count 3 and 4
    let names = |n: usize| -> Vec<u64> {
        let src = format!(
            "{BASE}{PEG}pp = pattern(of: peg, layout: lin(along: b.outline[0].dir, n: count({n}), step: 10))\n"
        );
        let r = ok(&src);
        let b = r.body.expect("a body");
        let mut v: Vec<u64> = b.face_ids().map(|f| b.face(f).name.0).collect();
        v.sort_unstable();
        v
    };
    let (three, four) = (names(3), names(4));
    assert!(
        three.iter().all(|n| four.contains(n)),
        "every face at 3 copies is at 4"
    );
}

// ---- work features ----------------------------------------------------------------------------

#[test]
fn every_work_feature_form() {
    let forms = [
        "w = workplane(on: offset(base.end, 2))",
        "w = workplane(offset: base.end, d: 2)",
        "a = workaxis(meet: (base.end, base.side(b.outline[0])))\nw = workplane(turn: base.end, about: a, by: 30deg)",
        "w = workplane(through: (b.o, top.q, top.s))",
        "w = workplane(mid: (base.start, base.end))",
        "w = workplane(normal_to: edges(between: (base.end, base.side(b.outline[0]))), at: b.o)",
        "w = workaxis(along: edges(between: (base.end, base.side(b.outline[0]))))",
        "w = workaxis(through: (b.o, top.q))",
        "w = workaxis(meet: (base.end, base.side(b.outline[0])))",
        "w = workaxis(normal_to: base.end, at: top.q)",
        "w = workpoint(at: top.q)",
        "w = workpoint(mid: edges(between: (base.end, base.side(b.outline[0]))))",
        "a = workaxis(through: (b.o, top.q))\nw = workpoint(meet: (a, base.end))",
        "w = ucs(on: base.end)",
        "w = ucs(through: (b.o, top.q, top.s))",
    ];
    for f in forms {
        ok(&format!("{BASE}{TOP}{f}\n"));
    }
    // tangent to a cylinder, parallel to a plane
    let boss =
        format!("{BASE}sketch bk(on: base.end) {{\n  c = circle((0, 0), 5)\n}}\nboss = extrude(bk.c, 20)\n");
    ok(&format!(
        "{boss}w = workplane(tangent: boss.side(bk.c), parallel: base.side(b.outline[0]))\n"
    ));
    // a work plane is a sketch plane and an end
    let r = ok(&format!(
        "{BASE}w = workplane(offset: base.end, d: 5)\nsketch s(on: w) {{\n  c = circle((0, 0), 4)\n}}\ne = extrude(s, 3)\n"
    ));
    close(vol(&r), base_vol() + PI * 16.0 * 3.0);
}

#[test]
fn work_feature_forms_are_typed() {
    let e = |f: &str| errs(&format!("{BASE}{TOP}{f}\n"));
    assert_eq!(e("w = workplane(offset: base.end)"), vec!["MissingArg"]);
    assert_eq!(
        e("w = workplane(offset: base.end, d: 2, by: 30deg)"),
        vec!["NotApplicable"]
    );
    assert_eq!(
        e("w = workplane(on: base.end, offset: base.end, d: 2)"),
        vec!["Forms"]
    );
    assert_eq!(e("w = workpoint(at: mid(b.outline[0]))"), vec!["NotARef"]);
    assert_eq!(e("w = workaxis(along: faces(of: base))"), vec!["NotOne"]);
}

// ---- bodies -----------------------------------------------------------------------------------

const TWO: &str = "sketch k2(on: base.end) {\n  r = rect(center: (0, 0), size: (20, 20))\n}\nblock = extrude(k2, sym(8), new: true)\n";

#[test]
fn combining_bodies() {
    let bodies = |r: &Run| r.measure.as_ref().expect("measured").bodies;
    let r = ok(&format!("{BASE}{TWO}"));
    assert_eq!(bodies(&r), 2);
    let r = ok(&format!("{BASE}{TWO}u = union(base, tools: block)\n"));
    assert_eq!(bodies(&r), 1);
    close(vol(&r), base_vol() + 400.0 * 4.0);
    let r = ok(&format!("{BASE}{TWO}u = subtract(base, tools: block)\n"));
    close(vol(&r), base_vol() - 400.0 * 4.0);
    let r = ok(&format!(
        "{BASE}{TWO}u = subtract(base, tools: block, keep: true)\n"
    ));
    assert_eq!(bodies(&r), 2);
    let r = ok(&format!("{BASE}{TWO}u = intersect(base, tools: [block])\n"));
    close(vol(&r), 400.0 * 4.0);
    let r = ok(&format!(
        "{BASE}{TWO}u = combine(base, tools: block, op: \"cut\")\n"
    ));
    close(vol(&r), base_vol() - 400.0 * 4.0);
    assert_eq!(
        errs(&format!(
            "{BASE}{TWO}u = union(base, tools: block, op: \"cut\")\n"
        )),
        vec!["NotApplicable"]
    );
    assert_eq!(
        errs(&format!(
            "{BASE}{TWO}u = combine(base, tools: block, op: \"glue\")\n"
        )),
        vec!["UnknownWord"]
    );
}

#[test]
fn splitting_and_moving_bodies() {
    let bodies = |r: &Run| r.measure.as_ref().expect("measured").bodies;
    let cut = "c = workplane(offset: base.end, d: -5)\n";
    let r = ok(&format!("{BASE}{cut}s = split(by: c)\n"));
    assert_eq!(bodies(&r), 2);
    close(vol(&r), base_vol());
    let r = ok(&format!("{BASE}{cut}s = split(by: c, keep: \"minus\")\n"));
    assert_eq!(bodies(&r), 1);
    close(vol(&r), base_vol() / 2.0);
    // a sketch line swept along its plane's normal
    let r = ok(&format!(
        "{BASE}sketch ms(on: b.plane) {{\n  l = line((0, -40), (0, 40))\n}}\ns = split(by: ms.l)\n"
    ));
    assert_eq!(bodies(&r), 2);
    let r = ok(&format!(
        "{BASE}{cut}s = splitface(faces: base.side(b.outline[0]), by: c)\n"
    ));
    assert_eq!(bodies(&r), 1);
    let r = ok(&format!(
        "{BASE}c = copybody(of: base, moves: [slide(along: b.outline[0].dir, by: 100), spin(about: b.outline[0], by: 90deg)])\n"
    ));
    assert_eq!(bodies(&r), 2);
    close(vol(&r), 2.0 * base_vol());
    let r = ok(&format!(
        "{BASE}m = movebody(of: base, moves: [slide(along: b.outline[1].dir, by: 7)])\n"
    ));
    assert!((r.measure.as_ref().expect("measured").bbox[1] + 23.0).abs() < 1e-4);
    let r = ok(&format!(
        "{BASE}m = movebody(of: base, moves: place(on: base.end))\n"
    ));
    assert!((r.measure.as_ref().expect("measured").bbox[2] - 10.0).abs() < 1e-4);
}

#[test]
fn a_feature_on_chosen_bodies() {
    // the hole goes through the block only, not the base it overlaps
    let src = format!(
        "{BASE}{TWO}sketch hk(on: base.end) {{\n  p = pt(0, 0)\n}}\nh = hole(at: hk.p, d: 4, depth: thru, bodies: block)\n"
    );
    let r = ok(&src);
    // drilled down from the base's top: through the block's lower half only
    close(vol(&r), base_vol() + 400.0 * 8.0 - PI * 4.0 * 4.0);
    let src = format!(
        "{BASE}{TWO}sketch hk(on: base.end) {{\n  p = pt(0, 0)\n}}\nh = hole(at: hk.p, d: 4, depth: thru, bodies: \"all\")\n"
    );
    close(vol(&ok(&src)), base_vol() + 400.0 * 8.0 - PI * 4.0 * 14.0);
}

// ---- direct edits (fg-shapes' replace_face takes a work plane too) ---------------------------

#[test]
fn a_face_replaced_onto_a_work_plane() {
    let r = ok(&format!(
        "{BASE}w = workplane(offset: base.end, d: 3)\nm = replace_face(base.end, with: w)\n"
    ));
    close(vol(&r), 60.0 * 60.0 * 13.0);
}

// ---- text and imports -------------------------------------------------------------------------

fn font_dir() -> std::path::PathBuf {
    // made once for the whole run: tests on other threads read the font while one would be
    // writing it again (a file is read whole each time, never kept by its size and time)
    static DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        // wasi has no temp_dir or pid; its runner opens the whole filesystem
        let d = if cfg!(target_os = "wasi") {
            let t = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock");
            std::path::PathBuf::from("/tmp").join(format!("forge-lang-wired-{}", t.as_nanos()))
        } else {
            std::env::temp_dir().join(format!("forge-lang-wired-{}", std::process::id()))
        };
        std::fs::create_dir_all(&d).expect("temp dir");
        std::fs::write(d.join("block.ttf"), forge_agent::bench::font_bytes()).expect("font");
        d
    })
    .clone()
}

#[test]
fn text_raised_and_cut_with_roles() {
    let d = Some(font_dir());
    let t = |extra: &str| {
        let src = format!(
            "{BASE}m = text(\"170\", font: \"block.ttf\", size: 8, depth: 0.5, on: base.end, at: b.o{extra})\n"
        );
        let r = run_in(&src, d.clone());
        assert!(r.errors.is_empty(), "{}", forge_lang::report::text(&r, &src));
        vol(&r)
    };
    let (up, down) = (t(""), t(", cut: true"));
    assert!(up > base_vol() && down < base_vol());
    close(up - base_vol(), base_vol() - down);
    let src = format!(
        "{BASE}m = text(\"170\", font: \"block.ttf\", size: 8, depth: 0.5, on: base.end, at: b.o)\nc = chamfer(edges(of: m.top), 0.1)\n"
    );
    let r = run_in(&src, d.clone());
    assert!(r.errors.is_empty(), "{}", forge_lang::report::text(&r, &src));
    let src = format!("{BASE}m = text(\"1\", font: \"none.ttf\", size: 8, depth: 0.5, on: base.end)\n");
    let r = run_in(&src, d);
    assert_eq!(r.errors.iter().map(Error::kind).collect::<Vec<_>>(), vec!["File"]);
}

#[test]
fn an_imported_solid_with_its_entities_by_name() {
    let dir = font_dir();
    // a box written as STEP by the kernel's own writer
    let boxed = forge_ops::extrude(
        &forge_ops::Profile {
            plane: forge_math::Frame::WORLD,
            loops: vec![forge_ops::Profile::rectangle(
                forge_math::Point2::new(0.0, 0.0),
                10.0,
                20.0,
                0,
            )],
        },
        5.0,
        1,
    )
    .expect("a box");
    let step = forge_step::export(&boxed, &forge_step::Options::default()).expect("exports");
    std::fs::write(dir.join("box.step"), &step.text).expect("write");
    let r = run_in("i = import(\"box.step\")\n", Some(dir.clone()));
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    close(vol(&r), 1000.0);
    // the file's first face entity, by its number
    let face = step
        .text
        .lines()
        .find(|l| l.contains("ADVANCED_FACE"))
        .and_then(|l| l.trim_start_matches('#').split('=').next())
        .and_then(|n| n.trim().parse::<u64>().ok())
        .expect("a face entity");
    let solid = step
        .text
        .lines()
        .find(|l| l.contains("MANIFOLD_SOLID_BREP"))
        .and_then(|l| l.trim_start_matches('#').split('=').next())
        .and_then(|n| n.trim().parse::<u64>().ok())
        .expect("a solid entity");
    let src = format!(
        "i = import(\"box.step\")\nc = chamfer(edges(of: entity(of: i, solid: count({solid}), id: count({face}))), 0.5)\n"
    );
    let r = run_in(&src, Some(dir.clone()));
    assert!(r.errors.is_empty(), "{}", forge_lang::report::text(&r, &src));
    assert!(vol(&r) < 1000.0);
    // cut from a part
    let src = format!("{BASE}i = import(\"box.step\", op: \"cut\")\n");
    let r = run_in(&src, Some(dir.clone()));
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    close(vol(&r), base_vol() - 1000.0);
    let r = run_in("i = import(\"box.txt\")\n", Some(dir));
    assert_eq!(r.errors.iter().map(Error::kind).collect::<Vec<_>>(), vec!["File"]);
}

// ---- projections ------------------------------------------------------------------------------

#[test]
fn projecting_edges_and_faces_into_a_sketch() {
    // the base's top outline laid on a plane above, extruded back down onto it
    let src = format!(
        "{BASE}sketch s(on: offset(base.end, 5)) {{\n  o = project(base.end)\n}}\ne = extrude(s, 3)\n"
    );
    let r = ok(&src);
    close(vol(&r), base_vol() + 60.0 * 60.0 * 3.0);
    let src = format!(
        "{BASE}{PEG}pe = edges(of: peg.end)\nsketch s(on: offset(base.end, 10)) {{\n  c = project(pe)\n}}\ne = extrude(s, 2)\n"
    );
    close(vol(&ok(&src)), base_vol() + peg() + PI * 9.0 * 2.0);
    let src = format!("{BASE}w = workpoint(at: b.o)\nsketch s(on: b.plane) {{\n  p = project(w)\n}}\n");
    ok(&src);
}

// ---- fillets with every option ----------------------------------------------------------------

#[test]
fn fillets_varying_and_their_options() {
    let rim = "rim = edges(of: base.end)\nsketch tc(on: base.end) {\n  c = pt(30, -30)\n}\n";
    let fill = |f: &str| {
        let src = format!("{BASE}{rim}{f}\n");
        let r = run(&src);
        assert!(r.errors.is_empty(), "{f}: {}", forge_lang::report::text(&r, &src));
        vol(&r)
    };
    let constant = fill("f = fillet(rim, 2)");
    let linear = fill("f = fillet(rim, vary(1, 3))");
    let points = fill("f = fillet(rim, vary([(0, 1), (ratio(0.5), 3), (1, 1)]))");
    assert!(constant < base_vol() && linear < base_vol() && points < base_vol());
    assert!((linear - constant).abs() > 1e-3 && (points - constant).abs() > 1e-3);
    // the law from the other end of each edge: a different solid
    let flat = "rim0 = edges(between: (base.end, base.side(b.outline[0])))\n";
    let one = fill(&format!("{flat}f = fillet(rim0, vary(1, 3), only: true)"));
    let back = fill(&format!(
        "{flat}f = fillet(rim0, vary(1, 3), from: tc.c, only: true)"
    ));
    assert!(one < base_vol() && back < base_vol());
    fill(
        "post = edges(between: (base.side(b.outline[0]), base.side(b.outline[3])))\nf = fillet(groups: [(rim, 2), (post, vary(1, 2), b.outline[0].a)])",
    );
    fill("f = fillet(rim + edges(of: base, along: Z), 2, setbacks: [(tc.c, 3)])");
    fill("f = fillet(rim, 2, only: true)");
}

#[test]
fn fillet_errors_are_typed() {
    let e = |f: &str| errs(&format!("{BASE}rim = edges(of: base.end)\n{f}\n"));
    assert_eq!(e("f = fillet(rim, vary([(0, 1), (0, 3)]))"), vec!["BadValue"]);
    assert_eq!(e("f = fillet(rim, vary([(0, 1)]))"), vec!["NotOne"]);
    assert_eq!(e("f = fillet(rim, vary(1, -3))"), vec!["BadValue"]);
    assert_eq!(
        e("f = fillet(groups: [(rim, 2)], from: b.o)"),
        vec!["NotApplicable"]
    );
    assert_eq!(e("f = fillet(rim, 2, from: b.o)"), vec!["NotOne"]);
}

#[test]
fn of_means_the_features_own_not_its_copies() {
    let src = |of: &str| {
        format!(
            "{BASE}{PEG}pegs = pattern(of: peg, layout: lin(along: b.outline[0].dir, n: count(2), step: 20))\nr = fillet(edges(of: {of}, crease: \"concave\"), 1)\n"
        )
    };
    let own = vol(&ok(&src("peg")));
    let both = vol(&ok(&src("[peg, pegs]")));
    // the fillet on the copy's root too adds as much again
    close(
        both - base_vol() - 2.0 * peg(),
        2.0 * (own - base_vol() - 2.0 * peg()),
    );
}
