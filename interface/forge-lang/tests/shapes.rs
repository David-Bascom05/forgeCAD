//! Shape features (sweep, coil, loft, shell, draft, face offsets and slabs, ribs, face fillets,
//! full rounds, balls): each built, measured against its exact volume where one exists, its role
//! anchors used by later features, and its options that cannot apply refused as typed errors.

use forge_lang::error::Error;
use forge_lang::{Options, Run};
use std::f64::consts::PI;

fn run(src: &str) -> Run {
    forge_lang::run(src, Options::default(), Default::default()).0
}

fn ok(src: &str) -> Run {
    let r = run(src);
    assert!(r.errors.is_empty(), "{}", forge_lang::report::text(&r, src));
    let m = r.measure.as_ref().expect("measured");
    assert_eq!(m.problems, 0, "{}", forge_lang::report::text(&r, src));
    r
}

fn volume(r: &Run) -> f64 {
    r.measure.as_ref().expect("measured").volume
}

fn close(a: f64, b: f64, rel: f64) {
    assert!((a - b).abs() <= rel * b.abs(), "{a} vs {b}");
}

fn errors(src: &str) -> Vec<&'static str> {
    run(src).errors.iter().map(Error::kind).collect()
}

const SWEEP: &str = "shared r = 3
shared h = 20
shared R = 15
shared w = 10
sketch sec(on: xy) {
  c = circle((0, 0), r)
  base = line((0, 0), (w, 0), construction: true)
}
sketch rail(on: rotate(sec.plane, about: sec.base, by: 90deg)) {
  path route(start: (0, 0)) {
    line(to: (0, h))
    tarc(to: (R, h + R))
    line(to: (R + h, h + R))
  }
}
";

#[test]
fn a_sweep_along_lines_and_an_arc_has_its_exact_volume_and_caps() {
    let src = format!(
        "{SWEEP}tube = sweep(sec.c, along: rail.route)\nsketch cap(on: tube.end) {{\n  k = circle((0, 0), r)\n}}\nplug = extrude(cap.k, r)\nedge = fillet(edges(between: (tube.side(sec.c), tube.start)), r / 3)\n"
    );
    let r = ok(&src);
    let len = 40.0 + PI / 2.0 * 15.0;
    let v = PI * 9.0 * (len + 3.0);
    // the fillet takes a little off the start's rim
    assert!(volume(&r) < v && volume(&r) > 0.98 * v, "{} vs {v}", volume(&r));
}

#[test]
fn a_sweep_extended_and_one_cut_off_at_a_face() {
    let r = ok(&format!(
        "{SWEEP}tube = sweep(sec.c, along: rail.route, extend: (w, w))\n"
    ));
    close(volume(&r), PI * 9.0 * (60.0 + PI / 2.0 * 15.0), 1e-6);
    // swept up to the face of a block standing across its last run
    let src = format!(
        "{SWEEP}shared s = 60
sketch wallsk(on: sec.plane) {{
  o = rect(corner: (h + R / 2, -s / 2), size: (w, s))
}}
blk = extrude(wallsk, s)
tube = sweep(sec.c, along: rail.route, upto: blk.side(wallsk.o[3]), offset: 0 - r)
"
    );
    let r = ok(&src);
    assert!(volume(&r) > 60.0 * 60.0 * 10.0);
    // both is refused
    let src = format!(
        "{SWEEP}sketch wallsk(on: sec.plane) {{\n  o = rect(corner: (h, 0), size: (w, w))\n}}\nblk = extrude(wallsk, w)\ntube = sweep(sec.c, along: rail.route, extend: (w, w), upto: blk.end)\n"
    );
    assert_eq!(errors(&src), ["NotWith"]);
}

#[test]
fn a_sweep_path_from_several_sketches_runs_in_3d() {
    // up from xy along z, then on along x in a second sketch on another plane
    let src = "shared r = 2
shared h = 30
shared w = 10
sketch sec(on: xy) {
  c = circle((0, 0), r)
  bx = line((0, 0), (w, 0), construction: true)
  by = line((0, 0), (0, w), construction: true)
}
sketch up(on: rotate(sec.plane, about: sec.bx, by: 90deg)) {
  l = line((0, 0), (0, h))
}
sketch over(on: offset(sec.plane, h)) {
  l = line((0, 0), (0, h))
}
tube = sweep(sec.c, along: [up.l, over.l])
";
    // the turn is a corner, not tangent: the kernel refuses it
    let r = run(src);
    assert_eq!(r.errors.iter().map(Error::kind).collect::<Vec<_>>(), ["Build"]);
    // pieces that do not meet are a typed gap
    let src = src.replace("offset(sec.plane, h)", "offset(sec.plane, h + w)");
    assert_eq!(errors(&src), ["PathGap"]);
}

#[test]
fn a_coil_by_any_two_sizes() {
    let base = "shared R = 10
shared r = 1
shared p = 5
shared n = count(4)
shared H = 20
shared a = 5deg
sketch s(on: xz) {
  c = circle((R, 0), r)
  ax = line((0, 0), (0, H), construction: true)
}
";
    // a profile in the axial plane: Pappus holds for the screw motion too
    let exact = PI * 1.0 * 2.0 * PI * 10.0 * 4.0;
    for size in ["pitch: p, turns: n", "turns: n, height: H", "pitch: p, height: H"] {
        let r = ok(&format!("{base}spring = coil(s.c, axis: s.ax, {size})\n"));
        close(volume(&r), exact, 1e-3);
    }
    let r = ok(&format!(
        "{base}spring = coil(s.c, axis: s.ax, pitch: p, turns: n, left: true, taper: a)\n"
    ));
    assert!(volume(&r) > exact);
    assert_eq!(
        errors(&format!(
            "{base}spring = coil(s.c, axis: s.ax, pitch: p, turns: n, height: H)\n"
        )),
        ["NotWith"]
    );
    assert_eq!(
        errors(&format!("{base}spring = coil(s.c, axis: s.ax, pitch: p)\n")),
        ["MissingArg"]
    );
}

const LOFT: &str = "shared a = 20
shared r = 8
shared h = 30
sketch bot(on: xy) {
  sq = rect(center: (0, 0), size: (a, a))
}
sketch top(on: offset(bot.plane, h)) {
  c = circle((0, 0), r)
}
";

#[test]
fn a_loft_matches_a_circle_to_a_square_itself() {
    let src = format!(
        "{LOFT}blend = loft([bot.sq, top.c])\nsketch cap(on: blend.end) {{\n  k = circle((0, 0), r / 2)\n}}\nstub = extrude(cap.k, r)\nedge = fillet(edges(between: (blend.side(bot.sq), blend.start)), r / 8)\n"
    );
    let r = ok(&src);
    let m = r.measure.as_ref().expect("measured");
    assert_eq!(m.bodies, 1);
    // and the other way round: the circle first
    ok(&format!("{LOFT}blend = loft([top.c, bot.sq], align: true)\n"));
}

#[test]
fn a_loft_through_several_profiles_smooth_and_to_a_point() {
    let src = format!(
        "{LOFT}sketch hi(on: offset(bot.plane, 2 * h)) {{\n  s = rect(center: (0, 0), size: (r, r))\n}}\nshared k = ratio(1.5)\nblend = loft([bot.sq, top.c, hi.s], smooth: true, end: k)\ncone = loft([hi.s, offset(hi.plane, h).origin])\n"
    );
    let r = ok(&src);
    assert!(volume(&r) > 8.0 * 8.0 * 30.0 / 3.0);
    // end weights belong to a smooth loft
    assert_eq!(
        errors(&format!(
            "{LOFT}shared k = ratio(1.5)\nblend = loft([bot.sq, top.c], end: k)\n"
        )),
        ["NotWith"]
    );
}

#[test]
fn a_loft_with_holes_matches_loop_by_loop_or_says_which_profile_differs() {
    // a square with a hole to a circle with one: each loop matched to its partner
    let src = "shared a = 20
shared r = 8
shared h = 30
shared q = 2
sketch bot(on: xy) {
  sq = rect(center: (0, 0), size: (a, a))
  c0 = circle((0, 0), q)
}
sketch top(on: offset(bot.plane, h)) {
  c = circle((0, 0), r)
  c1 = circle((0, 0), q)
}
blend = loft([bot, top])
";
    let r = ok(src);
    assert!(volume(&r) > 0.0);
    let src = src.replace("  c1 = circle((0, 0), q)\n", "");
    assert_eq!(errors(&src), ["LoftLoops"]);
}

const BOX: &str = "shared L = 60
shared W = 40
shared H = 30
shared t = 2
shared a = 3deg
shared g = 1.5
sketch base(on: xy) {
  r = rect(center: (0, 0), size: (L, W))
  x = line((0, 0), (L / 2, 0), construction: true)
}
box = extrude(base, H)
";

#[test]
fn draft_offset_and_thicken_move_faces() {
    // the far side (y = W/2) leans in by a about the base: a wedge of H^2 tan(a) / 2 per unit length off
    let r = ok(&format!(
        "{BOX}tilt = draft(box.side(base.r[2]), angle: a, neutral: base.plane)\n"
    ));
    let (s, c) = forge_math::dmath::sin_cos(3.0f64.to_radians());
    let off = 60.0 * 30.0 * 30.0 * (s / c) / 2.0;
    close(volume(&r), 60.0 * 40.0 * 30.0 - off, 1e-9);
    let r = ok(&format!("{BOX}grow = offset_faces(box.side(base.r[1]), g)\n"));
    close(volume(&r), 61.5 * 40.0 * 30.0, 1e-9);
    let r = ok(&format!("{BOX}slab = thicken(box.end, t)\n"));
    close(volume(&r), 60.0 * 40.0 * 32.0, 1e-9);
    let r = ok(&format!(
        "{BOX}slab = thicken(box.end, t, side: \"in\", cut: true)\n"
    ));
    close(volume(&r), 60.0 * 40.0 * 28.0, 1e-9);
}

#[test]
fn a_shell_inside_or_out_names_its_inner_skin_and_rim() {
    let src = format!(
        "{BOX}cup = shell(t, open: box.end)\nlip = fillet(edges(between: (cup.inner, box.side(base.r[0]))), t / 4)\n"
    );
    let r = run(&src);
    // the inner skin is named; it meets no outer wall, so that selection is empty
    assert_eq!(
        r.errors.iter().map(Error::kind).collect::<Vec<_>>(),
        ["EmptySelection"]
    );
    let src = format!(
        "{BOX}cup = shell(t, open: box.end)\nlip = fillet(edges(between: (cup.inner, cup.rim)), t / 4)\n"
    );
    let r = ok(&src);
    assert!(volume(&r) < 60.0 * 40.0 * 30.0 - 56.0 * 36.0 * 28.0);
    let r = ok(&format!("{BOX}cup = shell(t, open: box.end, out: true)\n"));
    close(volume(&r), 64.0 * 44.0 * 32.0 - 60.0 * 40.0 * 30.0, 1e-9);
}

#[test]
fn a_rib_fills_from_its_line_to_the_part() {
    let src = format!(
        "{BOX}cup = shell(t, open: box.end)\nsketch web(on: rotate(base.plane, about: base.x, by: 90deg)) {{\n  l = line((0 - L / 4, H / 2), (L / 4, H / 2))\n}}\nwall = rib(web.l, t, flip: true)\n"
    );
    let before = volume(&ok(&format!("{BOX}cup = shell(t, open: box.end)\n")));
    let r = ok(&src);
    // a wall t thick across the cup (its ends run on to the walls), from half height to the floor
    close(volume(&r) - before, 2.0 * 56.0 * 13.0, 1e-9);
}

#[test]
fn face_fillets_full_rounds_and_balls() {
    let r = ok(&format!(
        "{BOX}shared rr = 5\nround = facefillet(box.side(base.r[0]), box.end, rr)\n"
    ));
    close(volume(&r), 72000.0 - 60.0 * 25.0 * (1.0 - PI / 4.0), 1e-9);
    let r = ok(&format!(
        "{BOX}full = fullround(side: box.side(base.r[0]), middle: box.end, other: box.side(base.r[2]))\n"
    ));
    // the top becomes a half round of radius W/2
    close(volume(&r), 60.0 * (40.0 * 10.0 + PI * 400.0 / 2.0), 1e-6);
    assert_eq!(
        errors(&format!(
            "{BOX}full = fullround(side: box.side(base.r), middle: box.end, other: box.side(base.r[2]))\n"
        )),
        ["OneFace"]
    );
    let r = ok("shared r = 6\nsketch s(on: xy) {\n  c = pt(0, 0)\n}\nknob = ball(s.c, r)\n");
    close(volume(&r), 4.0 / 3.0 * PI * 216.0, 1e-9);
    let r = ok(&format!(
        "{BOX}shared br = 6\nsketch top(on: box.end) {{\n  c = pt(0, 0)\n}}\npit = sphere(top.c, br, cut: true)\n"
    ));
    close(volume(&r), 72000.0 - 2.0 / 3.0 * PI * 216.0, 1e-9);
}

#[test]
fn a_taper_is_refused_where_it_cannot_apply() {
    let src = format!(
        "{BOX}sketch k(on: box.end) {{\n  c = circle((0, 0), g)\n}}\nh = cut(k.c, thru_all, taper: a)\n"
    );
    assert_eq!(errors(&src), ["NotWith"]);
    let src = "shared L = 40\nshared a = 5deg\nsketch s(on: xy) {\n  o = spline([(0, 0), (L, 0), (L, L), (0, L)])\n}\nb = extrude(s, L, taper: a)\n";
    assert_eq!(errors(src), ["TaperOnCurve"]);
    // a depth takes one
    ok(&format!(
        "{BOX}sketch k(on: box.end) {{\n  c = circle((0, 0), W / 4)\n}}\nh = cut(k.c, W / 4, taper: a)\n"
    ));
}

#[test]
fn a_size_over_a_bare_number_is_a_size() {
    // in params too: W / 2 is a Length, W * 0.25 a Length (not an Area)
    let src = "shared W = 40\nshared half = W / 2\nshared q = W * 0.25\nsketch s(on: xy) {\n  o = rect(center: (0, 0), size: (half, q))\n}\nb = extrude(s, half)\n";
    close(volume(&ok(src)), 4000.0, 1e-12);
    // a count over a size is nothing
    assert_eq!(
        errors("shared W = 40\nshared n = count(4)\nx = n / W\n"),
        ["DimMismatch"]
    );
    // a bare number over a count is a ratio
    assert!(errors("shared n = count(4)\nshared W = 40\nx = W * (1 / n)\n").is_empty());
}

#[test]
fn a_loft_follows_rails_from_corners_and_a_centre_line() {
    let base = "shared a = 20
shared h = 40
shared c = 40
sketch bot(on: xy) {
  sq = rect(center: (0, 0), size: (a, a))
  x = line((0, 0), (a, 0), construction: true)
}
sketch top(on: offset(bot.plane, h)) {
  sq = rect(center: (0, 0), size: (a, a))
}
sketch front(on: rotate(bot.plane, about: bot.sq[0], by: 90deg)) {
  r = arc(center: (a - c, h / 2), from: (a, 0), to: (a, h))
}
";
    // a rail through one corner, bowed out: more than the straight prism
    let r = ok(&format!(
        "{base}bulge = loft([bot.sq, top.sq], smooth: true, rails: [(bot.sq[1].a, front.r)])\n"
    ));
    assert!(volume(&r) > 16000.0);
    // a rail's point must be a corner of the first profile
    assert_eq!(
        errors(&format!(
            "{base}bulge = loft([bot.sq, top.sq], smooth: true, rails: [(top.sq[1].a, front.r)])\n"
        )),
        ["NotAVertex"]
    );
    // a centre line: up, then bent over
    let r = ok(&format!(
        "{base}sketch spine(on: rotate(bot.plane, about: bot.x, by: 90deg)) {{\n  path cl(start: (0, 0)) {{\n    line(to: (0, h / 2))\n    tarc(to: (a / 4, h))\n  }}\n}}\nbent = loft([bot.sq, top.sq], smooth: true, centre: spine.cl)\n"
    ));
    assert!(volume(&r) > 0.0);
    assert_eq!(
        errors(&format!(
            "{base}bulge = loft([bot.sq, top.sq], smooth: true, rails: [(bot.sq[1].a, front.r)], centre: front.r)\n"
        )),
        ["NotWith"]
    );
}

const BOSS: &str = "shared L = 60
shared t = 8
shared R = 10
shared bh = 12
shared fr = 4
shared d = 2
shared c = 0.5
sketch plate(on: xy) {
  o = rect(center: (0, 0), size: (L, L))
}
base = extrude(plate, t)
sketch bk(on: base.end) {
  b = circle((0, 0), R)
}
boss = extrude(bk.b, bh)
blend = fillet(edges(between: (boss, base)), fr)
";

#[test]
fn a_hole_rim_is_its_mouth_on_a_blend_too() {
    // drilled into the boss's blend: the mouth is no circle, and still the rim
    let hole =
        "sketch hk(on: base.end) {\n  p = pt(R + fr / 2, 0)\n}\nbore = hole(at: hk.p, d: d, depth: t)\n";
    let plain = volume(&ok(&format!("{BOSS}{hole}")));
    let r = ok(&format!("{BOSS}{hole}brk = chamfer(bore.rim, c)\n"));
    let taken = plain - volume(&r);
    // a ring of chamfer round a 2 mm mouth: more than nothing, less than a 3 mm disc 0.5 deep
    assert!(taken > 0.0 && taken < PI * 1.5 * 1.5 * 0.5, "{taken}");
    // through the boss's top: its rim there
    let r = ok(&format!(
        "{BOSS}sketch hk(on: boss.end) {{\n  p = pt(0, 0)\n}}\nbore = hole(at: hk.p, d: d, depth: thru)\nbrk = chamfer(bore.rim, c)\n"
    ));
    assert!(volume(&r) > 0.0);
}

#[test]
fn a_rim_a_later_fillet_rounded_is_a_typed_error() {
    let src = format!(
        "{BOSS}sketch hk(on: boss.end) {{\n  p = pt(0, 0)\n}}\nbore = hole(at: hk.p, d: d, depth: bh)\nround = fillet(edges(of: boss, crease: \"convex\"), c)\nbrk = chamfer(bore.rim, c)\n"
    );
    assert_eq!(errors(&src), ["MouthRounded"]);
}
