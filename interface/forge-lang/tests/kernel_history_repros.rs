//! Programs the random history generators shrank from their failures (the .fc generator's,
//! forge_agent::history, ported to .fg feature for feature: each builds the very kernel features
//! the part file did, but for the names the language gives them; the helper sketches `h*` hold the
//! points and directions a placement needs): each builds valid, meshes closed and regenerates the
//! same n-ary, or is refused with a clear error; never a panic or a hang. Two forms the language
//! has no word for (a revolve kept or turned symmetrically, a revolve up to a plane) are written
//! as the nearest program and its kernel feature set as the part file had it.
use forge_lang::fuzz::{Built, Verdict, build, judge};
use forge_model::{Combine, Feature, Part, Target, Turn};

fn built(src: &str) -> Built {
    let b = build(src);
    assert!(b.run.errors.is_empty(), "{:?}", b.run.errors);
    b
}

fn refused(src: &str, why: &str) {
    match judge(src) {
        Verdict::Refused(w) if w.contains(why) => {}
        v => panic!("{v:?}, not refused with '{why}'"),
    }
}

/// Builds valid feature by feature.
fn seq_valid(src: &str) -> Built {
    let b = built(src);
    for s in b.solids_after(b.built()) {
        assert!(
            forge_topo::check(&s.body).is_empty(),
            "{:?}",
            forge_topo::check(&s.body)
        );
    }
    b
}

/// Builds valid feature by feature and n-ary (the mesh left to forge-mesh's own tests).
fn builds_valid(src: &str) {
    let b = seq_valid(src);
    nary_valid(&b.part(), &b.ids);
}

fn nary_valid(part: &Part, ids: &[u64]) {
    for s in part
        .regenerate_nary_solids_ids(1, Some(ids))
        .expect("regenerates in one pass")
    {
        assert!(
            forge_topo::check(&s.body).is_empty(),
            "{:?}",
            forge_topo::check(&s.body)
        );
    }
}

/// The last revolve of `part`, as the part file wrote it.
fn set_revolve(part: &mut Part, turn: Turn, combine: Combine) {
    let Some(Feature::Revolve { extent, op, .. }) = part
        .features
        .iter_mut()
        .rev()
        .find(|f| matches!(f, Feature::Revolve { .. }))
    else {
        panic!("a revolve")
    };
    *extent = turn;
    *op = combine;
}

/// A revolve keep whose tool misses the part leaves nothing: refused there, so a split of what
/// would be left no longer panics on an empty box. (The language revolves neither kept nor
/// symmetric: the kernel feature is set as the part file had it, `revolve keep sym 140.83`.)
#[test]
fn keep_that_leaves_nothing() {
    let src = r#"sketch s1(on: xy) {
  o = rect(corner: (0, 0), size: (117.88, 47.1))
}
f1 = extrude(s1, 14.83)
sketch h0(on: offset(xy, 26.03)) {
  p0 = pt(111.27, 40.53)
}
sketch s2(on: plane(at: h0.p0, normal: -Y, x: X)) {
  o = poly([(14.6, 5.61), (14.6, -5.61), (25.83, 0)])
  ax = line((0, 0), (0, -1), construction: true)
}
f2 = revolve(s2.o, axis: s2.ax, angle: 140.83deg)
"#;
    let split = format!(
        "{src}sketch w(on: xy) {{\n  p = pt(0, 0)\n}}\nhalf = split(by: plane(at: w.p, normal: X), keep: \"plus\")\n"
    );
    for src in [src, split.as_str()] {
        let b = built(src);
        let mut part = b.part();
        set_revolve(&mut part, Turn::Symmetric(140.83f64.to_radians()), Combine::Keep);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            part.regenerate_solids_ids(Some(&b.ids))
        }));
        match r {
            Ok(Err(e)) => assert!(format!("{e:?}").contains("Empty"), "{e:?}"),
            Ok(Ok(s)) => panic!("{} bodies, not refused", s.len()),
            Err(_) => panic!("panics"),
        }
    }
}

/// A swept tube patterned so its copies overlap, then a fillet of the sweep's arcs: some picked
/// arcs are seams between coplanar faces the copies left, removed when the faces merge (it
/// panicked looking them up). They are dropped; the rest round. Feature by feature only: the
/// n-ary regen names the arcs apart (the n-ary naming work has that).
#[test]
fn fillet_picks_a_flush_seam() {
    seq_valid(
        r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
  p1 = pt(0.8164156395068652, -0.5774647206268071)
}
sketch h1(on: offset(xy, 28.17)) {
  p0 = pt(68.63, -6.59)
}
sketch h2(on: offset(xy, 29.19)) {
  p0 = pt(55.92999999999999, 3.5500000000000007)
  p1 = pt(48.91, 31.45)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (97.81, 0))
  e2 = line((97.81, 0), (97.81, 38.36))
  e3 = line((97.81, 38.36), (0, 38.36))
  e4 = line((0, 38.36), (0, 0))
}
f1 = extrude(s1, 28.15)
sketch w2(on: plane(at: h2.p0, normal: Z, x: -X)) {
  q0 = line((7.019999999999996, -27.9), (7.019999999999996, 0))
  q1 = arc(center: (0, 0), from: (7.019999999999996, 0), to: (0, 7.02))
  q2 = line((0, 7.02), (-19.659999999999997, 7.020000000000001))
}
sketch s2(on: plane(at: h2.p1, normal: -Y, x: X)) {
  e1 = line((-2.09, -2.09), (2.09, -2.09))
  e2 = line((2.09, -2.09), (2.09, 2.09))
  e3 = line((2.09, 2.09), (-2.09, 2.09))
  e4 = line((-2.09, 2.09), (-2.09, -2.09))
}
f2 = sweep(s2, along: [w2.q0, w2.q1, w2.q2])
f3 = pattern(of: [f2], layout: lin(along: axis(h0.p0, h0.p1), n: 4, step: 10.8))
f4 = hole(at: h1.p0, on: plane(at: h1.p0, normal: X, x: -Z), d: 9.06, depth: thru)
f5 = fillet(edges(of: f2, shape: "circle"), vary(0.09, 0.09))
"#,
    );
}

/// A twisted ruled loft joined to a block: its sections are quadrilaterals whose area is
/// quadratic in height, so Simpson's rule is exact. The mesh once gave 103240 mm^3.
#[test]
fn twisted_loft_on_a_block_volume() {
    let b = built(
        r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h1(on: offset(xy, 10)) {
  p0 = pt(30, 30)
}
sketch h2(on: offset(xy, 40)) {
  p0 = pt(30, 30)
  p1 = pt(30.87758256189037, 30.479425538604204)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (100, 0))
  e2 = line((100, 0), (100, 60))
  e3 = line((100, 60), (0, 60))
  e4 = line((0, 60), (0, 0))
}
f1 = extrude(s1, 15)
sketch s2_0(on: plane(at: h1.p0, normal: Z, x: X)) {
  e1 = line((-15, -15), (15, -15))
  e2 = line((15, -15), (15, 15))
  e3 = line((15, 15), (-15, 15))
  e4 = line((-15, 15), (-15, -15))
}
sketch s2_1(on: plane(at: h2.p0, normal: Z, x: axis(h2.p0, h2.p1))) {
  e1 = line((-8, -8), (8, -8))
  e2 = line((8, -8), (8, 8))
  e3 = line((8, 8), (-8, 8))
  e4 = line((-8, 8), (-8, -8))
}
f2 = loft([s2_0, s2_1])
"#,
    );
    let (s, c) = forge_math::dmath::sin_cos(0.5);
    let area = |t: f64| {
        let p: Vec<(f64, f64)> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .iter()
            .map(|&(x, y)| {
                let (bx, by) = (8.0 * (c * x - s * y), 8.0 * (s * x + c * y));
                ((1.0 - t) * 15.0 * x + t * bx, (1.0 - t) * 15.0 * y + t * by)
            })
            .collect();
        0.5 * (0..4)
            .map(|i| p[i].0 * p[(i + 1) % 4].1 - p[(i + 1) % 4].0 * p[i].1)
            .sum::<f64>()
    };
    let t0 = 1.0 / 6.0;
    let want = 90000.0 + 30.0 * (1.0 - t0) / 6.0 * (area(t0) + 4.0 * area(0.5 * (t0 + 1.0)) + area(1.0));
    let q = forge_mesh::Quality {
        chord: 0.002,
        ..Default::default()
    };
    let nary = b
        .part()
        .regenerate_nary_solids_ids(1, Some(&b.ids))
        .expect("n-ary");
    for body in [b.body().expect("a body").compact(), nary[0].body.compact()] {
        let v = forge_mesh::tessellate(&body, &q).expect("meshes").volume();
        assert!((v - want).abs() < 1.0, "{v} vs {want}");
    }
}

/// A rod cut across a pocket wall just above a variable fillet round the pocket floor: the rod's
/// circle on the wall crosses the fillet's spring curve, which lies where the wall and the blend
/// touch. It was cut against a plane square to the wall through the curve's middle, off the
/// curved spring line by 0.008 there, so no cut was found ("inconsistent classification"); the
/// spring curve's own fence (the rays from the ball's centre through it) finds it.
#[test]
fn cut_across_a_variable_fillet_spring_line() {
    let src = r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h1(on: offset(xy, 40)) {
  p0 = pt(80, 55)
}
sketch h2(on: offset(xy, 40.5)) {
  p0 = pt(72, 75)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (160, 0))
  e2 = line((160, 0), (160, 110))
  e3 = line((160, 110), (0, 110))
  e4 = line((0, 110), (0, 0))
}
f1 = extrude(s1, 40)
sketch s2(on: plane(at: h1.p0, normal: -Z, x: X)) {
  e1 = line((-26, -7), (26, -7))
  e2 = arc(center: (26, -3), from: (26, -7), to: (30, -3))
  e3 = line((30, -3), (30, 3))
  e4 = arc(center: (26, 3), from: (30, 3), to: (26, 7))
  e5 = line((26, 7), (-26, 7))
  e6 = arc(center: (-26, 3), from: (-26, 7), to: (-30, 3.0000000000000004))
  e7 = line((-30, 3), (-30, -3))
  e8 = arc(center: (-26, -3), from: (-30, -2.9999999999999996), to: (-26, -7))
}
f2 = cut(s2, -2)
f3 = fillet(groups: [(edges(of: f2, crease: "concave", shape: "line"), vary(0.6, 0.9)), (edges(of: f2, crease: "concave", shape: "circle"), vary(0.9, 0.6))])
sketch s4(on: plane(at: h2.p0, normal: -Y, x: X)) {
  e1 = circle((0, 0), 2)
}
f4 = cut(s4, -20)
"#;
    builds_valid(src);
    assert_eq!(judge(src), Verdict::Ok);
}

/// Two mirrored swept arcs 12 apart: each tube's torus face had the whole torus as its box, the
/// two boxes overlapped, and the pair (tori whose centre circles cross away from the faces) has
/// no closed form. A partial torus face's box now follows the arc its loops span.
#[test]
fn disjoint_swept_arcs_never_meet() {
    built(
        r#"sketch h0(on: offset(xy, -100)) {
  p0 = pt(0, 10)
}
sketch h1(on: offset(xy, 0)) {
  p0 = pt(0, 10)
  p1 = pt(0, 0)
}
sketch w1(on: plane(at: h0.p0, normal: X, x: Y)) {
  q0 = arc(center: (0, 0), from: (0.000000000000005329070518200751, 100), to: (64.27876096865394, 76.60444431189781), cw: true)
}
sketch s1(on: plane(at: h1.p0, normal: Y, x: -Z)) {
  e1 = circle((0, 0), 4)
}
f1 = sweep(s1, along: [w1.q0])
f2 = pattern(of: [f1], layout: mirror(on: plane(at: h1.p1, normal: -Y, x: X)))
"#,
    );
}

/// A sweep cut boring a bent tube in a general plane: the closed-form meet of the bore's and
/// the tube's tori found thousands of events and never returned.
#[test]
fn bent_tube_bore_in_a_general_plane() {
    builds_valid(
        r#"sketch h0(on: offset(xy, 201.79933726017012)) {
  p0 = pt(-307.1020171053978, 160.96118911642225)
}
sketch h1(on: offset(xy, 202.0053237815034)) {
  p0 = pt(-305.7075538694895, 160.846988255089)
}
sketch h2(on: offset(xy, 202.598871100508)) {
  p0 = pt(-306.512352962495, 160.846988255089)
}
sketch h3(on: offset(xy, 440.256433711249)) {
  p0 = pt(-131.2375417024681, 10.10148065235589)
}
sketch h4(on: offset(xy, 440.4635307203644)) {
  p0 = pt(-129.84225947079852, 10)
}
sketch h5(on: offset(xy, 441.057078039369)) {
  p0 = pt(-130.647058563804, 10)
}
sketch w1(on: plane(at: h2.p0, normal: axis(h2.p0, h1.p0), x: axis(h2.p0, h0.p0))) {
  q0 = line((-311.5837200301258, -116.02291803024991), (-1.5353480814217768, -119.99017754078409))
  q1 = arc(center: (0, 0), from: (-1.5353480814216125, -119.99017754078403), to: (3.5811763335663516, -119.94655133044843))
}
sketch s1(on: plane(at: h5.p0, normal: axis(h5.p0, h3.p0), x: axis(h5.p0, h4.p0))) {
  e1 = circle((0, 0), 7)
}
f1 = sweep(s1, along: [w1.q0, w1.q1])
sketch w2(on: plane(at: h2.p0, normal: axis(h2.p0, h1.p0), x: axis(h2.p0, h0.p0))) {
  q0 = line((-311.5837200301258, -116.02291803024991), (-1.5353480814217768, -119.99017754078409))
  q1 = arc(center: (0, 0), from: (-1.5353480814216125, -119.99017754078403), to: (3.5811763335663516, -119.94655133044843))
}
sketch s2(on: plane(at: h5.p0, normal: axis(h5.p0, h3.p0), x: axis(h5.p0, h4.p0))) {
  e1 = circle((0, 0), 6)
}
f2 = sweep(s2, along: [w2.q0, w2.q1], cut: true)
"#,
    );
}

/// A runner swept and bored along a bend through a flange, then the flange's face thickened
/// (B5): it hung in the bore like the bent tube, then left open edges. Feature by feature only:
/// the n-ary regen names the flange face apart (the n-ary naming work has that).
#[test]
fn thicken_after_a_bent_bore() {
    seq_valid(
        r#"sketch h0(on: offset(xy, -60)) {
  p0 = pt(30, 0)
}
sketch h1(on: offset(xy, 0)) {
  p0 = pt(-12, 0)
}
sketch s1(on: plane(at: h1.p0, normal: X, x: Y)) {
  e1 = line((-52, -30), (52, -30))
  e2 = arc(center: (52, -22), from: (52, -30), to: (60, -22))
  e3 = line((60, -22), (60, 22))
  e4 = arc(center: (52, 22), from: (60, 22), to: (52, 30))
  e5 = line((52, 30), (-52, 30))
  e6 = arc(center: (-52, 22), from: (-52, 30), to: (-60, 22))
  e7 = line((-60, 22), (-60, -22))
  e8 = arc(center: (-52, -22), from: (-60, -22), to: (-52, -30))
}
f1 = extrude(s1, 12)
sketch w2(on: plane(at: h0.p0, normal: -Y, x: X)) {
  q0 = line((-42, 60), (0, 60))
  q1 = arc(center: (0, 0), from: (0.000000000000003552713678800501, 60), to: (60, 0), cw: true)
  q2 = line((60, 0), (60, -55))
}
sketch s2(on: plane(at: h1.p0, normal: X, x: Y)) {
  e1 = circle((0, 0), 19)
}
f2 = sweep(s2, along: [w2.q0, w2.q1, w2.q2])
sketch w3(on: plane(at: h0.p0, normal: -Y, x: X)) {
  q0 = line((-42, 60), (0, 60))
  q1 = arc(center: (0, 0), from: (0.000000000000003552713678800501, 60), to: (60, 0), cw: true)
  q2 = line((60, 0), (60, -70))
}
sketch s3(on: plane(at: h1.p0, normal: X, x: Y)) {
  e1 = circle((0, 0), 15.5)
}
f3 = sweep(s3, along: [w3.q0, w3.q1, w3.q2], cut: true)
f4 = thicken(faces(of: f1, normal: -X), 1.5)
"#,
    );
}

/// A tilted pad over a through hole leaves a 1.6 mm2 lens of its side plane between its edge and
/// two arcs of the hole meeting at the cylinder's seam (M9). The body was right; check's chords of
/// the arcs cut into the thin lens past its probe (FaceWrongSide). A miss is now looked at again
/// with the loops drawn finely.
#[test]
fn thin_lens_at_a_hole_seam() {
    builds_valid(
        r#"sketch h0(on: offset(xy, -1)) {
  p0 = pt(75, 19)
}
sketch h1(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h2(on: offset(xy, 3.107438691392427)) {
  p0 = pt(89.61130286796393, 14.859185197128173)
}
sketch h3(on: offset(xy, 3.4058649537841124)) {
  p0 = pt(89.69521759175012, 13.908448595830624)
}
sketch h4(on: offset(xy, 4.35969436405894)) {
  p0 = pt(89.63358433345313, 14.202405731424355)
}
sketch s1(on: plane(at: h1.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (120, 0))
  e2 = line((120, 0), (120, 60))
  e3 = line((120, 60), (0, 60))
  e4 = line((0, 60), (0, 0))
}
f1 = extrude(s1, 15)
sketch s2(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 3)
}
f2 = cut(s2, -17)
sketch s3(on: plane(at: h3.p0, normal: axis(h3.p0, h4.p0), x: axis(h3.p0, h2.p0))) {
  e1 = line((-5.77804831568736, -10.946070359275362), (5.77804831568736, -10.946070359275362))
  e2 = arc(center: (5.77804831568736, -3.0915511001916505), from: (5.778048315687361, -10.946070359275362), to: (13.632567574771072, -3.0915511001916505))
  e3 = line((13.632567574771072, -3.0915511001916505), (13.632567574771072, 3.0915511001916505))
  e4 = arc(center: (5.77804831568736, 3.0915511001916505), from: (13.632567574771072, 3.0915511001916505), to: (5.778048315687361, 10.946070359275362))
  e5 = line((5.77804831568736, 10.946070359275362), (-5.77804831568736, 10.946070359275362))
  e6 = arc(center: (-5.77804831568736, 3.0915511001916505), from: (-5.778048315687359, 10.946070359275362), to: (-13.632567574771072, 3.0915511001916514))
  e7 = line((-13.632567574771072, 3.0915511001916505), (-13.632567574771072, -3.0915511001916505))
  e8 = arc(center: (-5.77804831568736, -3.0915511001916505), from: (-13.632567574771072, -3.0915511001916496), to: (-5.778048315687362, -10.946070359275362))
}
f3 = extrude(s3, 8.945478516509796)
"#,
    );
}

/// A ball centred on a block's edge with its poles on the edge (M5): the cuts on the sphere run
/// through a pole, where u says nothing, and the ways out were ordered by garbage directions.
/// They are ordered in the tangent plane there; both frames build valid alike.
#[test]
fn ball_with_its_poles_on_an_edge() {
    builds_valid(
        r#"sketch h0(on: offset(xy, -20)) {
  p0 = pt(-20, -20)
}
sketch h1(on: offset(xy, 0)) {
  p0 = pt(20, 0)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (40, 0))
  e2 = line((40, 0), (40, 40))
  e3 = line((40, 40), (0, 40))
  e4 = line((0, 40), (0, 0))
}
f1 = extrude(s1, 20)
sketch s2(on: plane(at: h1.p0, normal: -Y, x: X)) {
  c = pt(0, 0)
}
f2 = ball(s2.c, 5)
"#,
    );
    builds_valid(
        r#"sketch h0(on: offset(xy, -20)) {
  p0 = pt(-20, -20)
}
sketch h1(on: offset(xy, 0)) {
  p0 = pt(20, 0)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (40, 0))
  e2 = line((40, 0), (40, 40))
  e3 = line((40, 40), (0, 40))
  e4 = line((0, 40), (0, 0))
}
f1 = extrude(s1, 20)
sketch s2(on: plane(at: h1.p0, normal: Y, x: X)) {
  c = pt(0, 0)
}
f2 = ball(s2.c, 5)
"#,
    );
}

/// A bent square sweep patterned so the copies overlap (r21): a new edge leaves an old vertex
/// along the tangent of the old arc beside it, and the face split took the wrong way there.
#[test]
fn new_edge_leaves_tangent_to_an_old_one() {
    builds_valid(
        r#"sketch h0(on: offset(xy, -1.59)) {
  p0 = pt(80.59, 80.34)
}
sketch h1(on: offset(xy, -6.72)) {
  p0 = pt(80.59, 67.73)
}
sketch h2(on: offset(xy, 0)) {
  p0 = pt(0, 0)
  p1 = pt(0.20952908873087348, -0.9778024140774096)
}
sketch s1(on: plane(at: h2.p0, normal: Z, x: X)) {
  e1 = line((13.8, 0), (85.52, 0))
  e2 = arc(center: (85.52, 13.8), from: (85.52, 0), to: (99.32, 13.8))
  e3 = line((99.32, 13.8), (99.32, 73.36))
  e4 = arc(center: (85.52, 73.36), from: (99.32, 73.36), to: (85.52, 87.16))
  e5 = line((85.52, 87.16), (13.8, 87.16))
  e6 = arc(center: (13.8, 73.36), from: (13.8, 87.16), to: (0, 73.36))
  e7 = line((0, 73.36), (0, 13.8))
  e8 = arc(center: (13.8, 13.8), from: (0, 13.800000000000002), to: (13.799999999999999, 0))
}
f1 = extrude(s1, 32.31)
sketch w2(on: plane(at: h1.p0, normal: X, x: Z)) {
  q0 = line((5.13, -12.61), (5.13, 0))
  q1 = arc(center: (0, 0), from: (5.13, 0), to: (3.627457787486989, 3.627457787486989))
  q2 = line((3.627457787486989, 3.627457787486989), (-7.870098474606272, 15.125014049580251))
}
sketch s2(on: plane(at: h0.p0, normal: -Y, x: -Z)) {
  e1 = line((-3.19, -3.19), (3.18, -3.19))
  e2 = line((3.18, -3.19), (3.18, 3.18))
  e3 = line((3.18, 3.18), (-3.19, 3.18))
  e4 = line((-3.19, 3.18), (-3.19, -3.19))
}
f2 = sweep(s2, along: [w2.q0, w2.q1, w2.q2], extend: (3.18, 3.81))
f3 = pattern(of: [f2], layout: lin(along: axis(h2.p0, h2.p1), n: 2, step: 8.64))
"#,
    );
}

/// Two histories that took minutes: seed 2's closed-form meet recounted every interval for each
/// sign flicker it dropped, and seed 24's trim subdivided a traced curve running a hair off a
/// spring curve's fence down to a millionth of its length. Both finish in seconds.
#[test]
fn slow_histories_finish() {
    builds_valid(
        r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
  p1 = pt(58.85, 27.94)
  p2 = pt(58.82001349089433, 28.939550303522367)
  p3 = pt(59.84955030352237, 27.969986509105674)
  p4 = pt(73.75, 10.87)
  p5 = pt(0.4512199429985135, 0.8924127761526156)
  p6 = pt(-0.8924127761526156, 0.4512199429985135)
}
sketch h1(on: offset(xy, 1)) {
  p0 = pt(73.75, 10.87)
}
sketch h2(on: offset(xy, 11.59)) {
  p0 = pt(16.66, 34.04)
}
sketch h3(on: offset(xy, 35.19)) {
  p0 = pt(58.94, 24.91)
}
sketch h4(on: offset(xy, 7.89)) {
  p0 = pt(20.22, 36.18)
  p1 = pt(31.54, 30.49)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (117.88, 0))
  e2 = line((117.88, 0), (117.88, 47.1))
  e3 = line((117.88, 47.1), (0, 47.1))
  e4 = line((0, 47.1), (0, 0))
}
f1 = extrude(s1, 14.83)
sketch w2(on: plane(at: h4.p0, normal: -Z, x: -Y)) {
  q0 = line((5.690000000000001, -11.32), (5.690000000000001, 0))
  q1 = arc(center: (0, 0), from: (5.690000000000001, 0), to: (2.844999999999999, 4.927684547533456))
  q2 = line((2.844999999999999, 4.927684547533456), (-12.856040570611874, 13.992684547533457))
}
sketch s2(on: plane(at: h4.p1, normal: -X, x: Y)) {
  e1 = line((-1.78, -2.54), (1.78, -2.54))
  e2 = arc(center: (1.78, -1.78), from: (1.78, -2.54), to: (2.54, -1.78))
  e3 = line((2.54, -1.78), (2.54, 1.78))
  e4 = arc(center: (1.78, 1.78), from: (2.54, 1.78), to: (1.78, 2.54))
  e5 = line((1.78, 2.54), (-1.78, 2.54))
  e6 = arc(center: (-1.78, 1.78), from: (-1.78, 2.54), to: (-2.54, 1.78))
  e7 = line((-2.54, 1.78), (-2.54, -1.78))
  e8 = arc(center: (-1.78, -1.78), from: (-2.54, -1.78), to: (-1.7800000000000002, -2.54))
}
f2 = sweep(s2, along: [w2.q0, w2.q1, w2.q2], cut: true)
sketch s3(on: plane(at: h2.p0, normal: Z, x: X)) {
  c = pt(0, 0)
}
f3 = ball(s3.c, 15.78, cut: true)
f4 = workplane(on: plane(at: h3.p0, normal: Z, x: Y))
f5 = pattern(of: [f2], layout: mirror(on: plane(at: h0.p1, normal: axis(h0.p1, h0.p2), x: axis(h0.p1, h0.p3))))
f6 = pattern(of: [f3], layout: circ(about: axis(h0.p4, h1.p0), n: 4, each: 32.1deg))
f7 = pattern(of: [f3, f5], layout: lin(along: axis(h0.p0, h0.p5), n: 4, step: 17.35, along2: axis(h0.p0, h0.p6), n2: 3, step2: 17.35))
"#,
    );
    // (its last feature, a pattern of the whole body, still leaves open edges)
    let b = build(
        r#"sketch h0(on: offset(xy, -1.65)) {
  p0 = pt(13.41, 9.48)
}
sketch h1(on: offset(xy, -10.39)) {
  p0 = pt(13.41, 30.54)
}
sketch h2(on: offset(xy, 0)) {
  p0 = pt(0, 0)
  p1 = pt(0.9817195150458079, 0.190333375366024)
}
sketch h3(on: offset(xy, 0.46)) {
  p0 = pt(10.11, 32.44)
}
sketch h4(on: offset(xy, 14.89)) {
  p0 = pt(31.51, 39.61)
}
sketch h5(on: offset(xy, 27.2)) {
  p0 = pt(109.65, 21.02)
}
sketch h6(on: offset(xy, 27.30804906541608)) {
  p0 = pt(110.64245067493287, 21.07802634994567)
}
sketch h7(on: offset(xy, 28.194125725253556)) {
  p0 = pt(109.5417685702345, 21.02)
}
sketch h8(on: offset(xy, 29.78)) {
  p0 = pt(36.42, 6.19)
}
sketch s1(on: plane(at: h2.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (109.97, 0))
  e2 = line((109.97, 0), (109.97, 42.04))
  e3 = line((109.97, 42.04), (0, 42.04))
  e4 = line((0, 42.04), (0, 0))
}
f1 = extrude(s1, 29.78)
f2 = hole(at: h8.p0, on: plane(at: h8.p0, normal: Z, x: -Y), d: 7.43, depth: thru)
sketch s3(on: plane(at: h4.p0, normal: Z, x: X)) {
  c = pt(0, 0)
}
f3 = ball(s3.c, 3.89)
sketch w5(on: plane(at: h1.p0, normal: -X, x: Z)) {
  q0 = line((8.74, -21.06), (8.74, 0))
  q1 = arc(center: (0, 0), from: (8.74, 0), to: (6.180113267570426, 6.180113267570427))
  q2 = line((6.180113267570426, 6.180113267570427), (-14.76438959117511, 27.124616126315964))
}
sketch s5(on: plane(at: h0.p0, normal: Y, x: -Z)) {
  e1 = line((-2.3099999999999996, -3.3), (2.3099999999999996, -3.3))
  e2 = arc(center: (2.3099999999999996, -2.3099999999999996), from: (2.3099999999999996, -3.3), to: (3.3, -2.3099999999999996))
  e3 = line((3.3, -2.3099999999999996), (3.3, 2.3099999999999996))
  e4 = arc(center: (2.3099999999999996, 2.3099999999999996), from: (3.3, 2.3099999999999996), to: (2.3099999999999996, 3.3))
  e5 = line((2.3099999999999996, 3.3), (-2.3099999999999996, 3.3))
  e6 = arc(center: (-2.3099999999999996, 2.3099999999999996), from: (-2.3099999999999996, 3.3), to: (-3.3, 2.3099999999999996))
  e7 = line((-3.3, 2.3099999999999996), (-3.3, -2.3099999999999996))
  e8 = arc(center: (-2.3099999999999996, -2.3099999999999996), from: (-3.3, -2.3099999999999996), to: (-2.3099999999999996, -3.3))
}
f5 = sweep(s5, along: [w5.q0, w5.q1, w5.q2], cut: true)
sketch s7(on: plane(at: h5.p0, normal: axis(h5.p0, h6.p0), x: axis(h5.p0, h7.p0))) {
  e1 = line((8.82, 1.04), (2.69, 8.81))
  e2 = line((2.69, 8.81), (-5.73, 8.85))
  e3 = line((-5.73, 8.85), (-9.56, -0.34))
  e4 = line((-9.56, -0.34), (-4.67, -6.03))
  e5 = line((-4.67, -6.03), (1.72, -7.68))
  e6 = line((1.72, -7.68), (8.82, 1.04))
}
f7 = extrude(s7, 12.72)
f8 = fillet(edges(of: f1.start, along: X, extreme: Y), vary(4.93, 2.1))
sketch s10(on: plane(at: h3.p0, normal: X, x: Z)) {
  e1 = line((-10.794399999653304, -6.24), (10.794400000346625, -6.24))
  e2 = line((10.794400000346625, -6.24), (10.794400000346625, 6.119999999999999))
  e3 = line((10.794400000346625, 6.119999999999999), (-10.794399999653304, 6.119999999999999))
  e4 = line((-10.794399999653304, 6.119999999999999), (-10.794399999653304, -6.24))
  e5 = circle((0, 0), 2.2550000000000034)
}
f10 = extrude(s10, 22.1)
f11 = pattern(whole: true, layout: lin(along: axis(h2.p0, h2.p1), n: 2, step: 13.21))
"#,
    );
    assert!(b.body().is_some());
}

/// A draft of a face a variable fillet rounds into (r03): tilting it tore the tangent join and
/// the draft caught its own invalid body. It is refused: draft before rounding.
#[test]
fn draft_of_a_filleted_face() {
    refused(
        r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h1(on: offset(xy, 28.63)) {
  p0 = pt(70.26, 108.69)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (146.94, 0))
  e2 = line((146.94, 0), (146.94, 108.69))
  e3 = line((146.94, 108.69), (0, 108.69))
  e4 = line((0, 108.69), (0, 0))
}
f1 = extrude(s1, 30.98)
f2 = fillet(edges(of: f1.start, along: X, extreme: -Y) + edges(of: f1.end, along: Y, extreme: X), vary(8.41, 7.56))
f3 = hole(at: h1.p0, on: plane(at: h1.p0, normal: Y, x: Z), d: 8.1, cbore: (13.51, 6.07), depth: thru)
f4 = fillet(edges(of: f3, shape: "line", longer: 50, extreme: -X) + edges(of: f3, shape: "line", longer: 50, extreme: X) + edges(of: f3, shape: "circle", extreme: Y), 1.24)
f5 = draft(faces(of: f1, normal: X), angle: 1.57deg, neutral: offset(xy, -9.27))
"#,
        "Tangent",
    );
}

/// A pattern of a pattern and its source (r23): the source's ball came in twice, once as itself
/// and once through the inner pattern, and two equal spheres cut in one pass left open edges.
#[test]
fn pattern_of_a_pattern_and_its_source() {
    builds_valid(
        r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
  p1 = pt(0.029986509105671005, -0.9995503035223668)
  p2 = pt(0.9995503035223668, 0.029986509105671005)
  p3 = pt(0.11043152607484655, 0.9938837346736189)
}
sketch h1(on: offset(xy, 29.62)) {
  p0 = pt(57.27, 50.61)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (74.83, 0))
  e2 = line((74.83, 0), (74.83, 101.22))
  e3 = line((74.83, 101.22), (0, 101.22))
  e4 = line((0, 101.22), (0, 0))
}
f1 = extrude(s1, 29.9)
sketch s2(on: plane(at: h1.p0, normal: Z, x: X)) {
  c = pt(0, 0)
}
f2 = ball(s2.c, 3.11, cut: true)
f3 = pattern(of: [f2], layout: lin(along: axis(h0.p0, h0.p1), n: 4, step: 7.72, along2: axis(h0.p0, h0.p2), n2: 3, step2: 7.72))
f4 = pattern(of: [f3, f2], layout: lin(along: axis(h0.p0, h0.p3), n: 2, step: 9.35))
"#,
    );
    builds_valid(
        r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
  p1 = pt(0.11043152607484655, 0.9938837346736189)
}
sketch h1(on: offset(xy, 29.62)) {
  p0 = pt(57.27, 50.61)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (74.83, 0))
  e2 = line((74.83, 0), (74.83, 101.22))
  e3 = line((74.83, 101.22), (0, 101.22))
  e4 = line((0, 101.22), (0, 0))
}
f1 = extrude(s1, 29.9)
sketch s2(on: plane(at: h1.p0, normal: Z, x: X)) {
  c = pt(0, 0)
}
f2 = ball(s2.c, 3.11, cut: true)
f3 = pattern(of: [f2, f2], layout: lin(along: axis(h0.p0, h0.p1), n: 2, step: 9.35))
"#,
    );
}

/// A round tube swept on past a bend in a square one it crosses (r28): the round tube's bend
/// (a torus) against the square tube's bend (a cylinder square to the torus's axis) came out
/// Empty, the pieces of the cylinder face were not split, and the classification flooded
/// across. The closed form now finds where two pairs of roots meet at once.
#[test]
fn tube_bend_across_a_square_bend() {
    builds_valid(
        r#"sketch h0(on: offset(xy, -0.61)) {
  p0 = pt(61.39, 24.53)
}
sketch h1(on: offset(xy, -2.289999999999999)) {
  p0 = pt(60.44, 19.68)
}
sketch h2(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h3(on: offset(xy, 17.29)) {
  p0 = pt(60.44, 47.57)
}
sketch h4(on: offset(xy, 26.6)) {
  p0 = pt(85.87, 16.46)
  p1 = pt(60.44, -13.66)
}
sketch s1(on: plane(at: h2.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (127.51, 0))
  e2 = line((127.51, 0), (127.51, 107.86))
  e3 = line((127.51, 107.86), (0, 107.86))
  e4 = line((0, 107.86), (0, 0))
}
f1 = extrude(s1, 34.57)
sketch s2(on: plane(at: h0.p0, normal: -Z, x: Y)) {
  e1 = line((15.26, -2.4), (5.49, -12.01))
  e2 = line((5.49, -12.01), (-6.61, -11.1))
  e3 = line((-6.61, -11.1), (-18.54, -2.18))
  e4 = line((-18.54, -2.18), (-6.26, 16.88))
  e5 = line((-6.26, 16.88), (9.22, 9.43))
  e6 = line((9.22, 9.43), (15.26, -2.4))
}
f2 = cut(s2, thru)
sketch w3(on: plane(at: h1.p0, normal: X, x: Z)) {
  q0 = line((19.58, -27.89), (19.58, 0))
  q1 = arc(center: (0, 0), from: (19.58, 0), to: (16.95677740609931, 9.789999999999997))
  q2 = line((16.95677740609931, 9.789999999999997), (6.421777406099311, 28.03715525773812))
}
sketch s3(on: plane(at: h3.p0, normal: -Y, x: -Z)) {
  e1 = line((-4.64, -6.63), (4.64, -6.63))
  e2 = arc(center: (4.64, -4.64), from: (4.64, -6.63), to: (6.63, -4.64))
  e3 = line((6.63, -4.64), (6.63, 4.64))
  e4 = arc(center: (4.64, 4.64), from: (6.63, 4.64), to: (4.64, 6.63))
  e5 = line((4.64, 6.63), (-4.64, 6.63))
  e6 = arc(center: (-4.64, 4.64), from: (-4.64, 6.63), to: (-6.63, 4.64))
  e7 = line((-6.63, 4.64), (-6.63, -4.64))
  e8 = arc(center: (-4.64, -4.64), from: (-6.63, -4.64), to: (-4.64, -6.63))
}
f3 = sweep(s3, along: [w3.q0, w3.q1, w3.q2])
sketch w4(on: plane(at: h4.p0, normal: -Z, x: -X)) {
  q0 = line((25.430000000000007, -30.12), (25.430000000000007, 0))
  q1 = arc(center: (0, 0), from: (25.43, 0), to: (17.9817254455739, 17.9817254455739))
  q2 = line((17.9817254455739, 17.9817254455739), (1.1101576464628806, 34.85329324468493))
}
sketch s4(on: plane(at: h4.p1, normal: Y, x: X)) {
  e1 = circle((0, 0), 5.35)
}
f4 = sweep(s4, along: [w4.q0, w4.q1, w4.q2], extend: (1.84, 3.22))
"#,
    );
}

/// A countersunk hole on a face its edge's variable fillet rolls off (r10): the hole's top plane,
/// on that face, was traced against the blend, which only touches it along the spring curve; the
/// traced touch came out 1e-4 off the edge, a sliver piece split the face, and the flood failed.
/// A plane a blend rolls off along an edge only touches it there: the pair is skipped. (Each
/// edge's law runs the way the language's edge runs, not always the part file's: the volume
/// differs from it by 0.1 mm3 of 73458.)
#[test]
fn countersink_beside_a_variable_fillet() {
    builds_valid(
        r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h1(on: offset(xy, 3.5)) {
  p0 = pt(34.85, 92.38)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (69.44, 0))
  e2 = line((69.44, 0), (69.44, 92.38))
  e3 = line((69.44, 92.38), (0, 92.38))
  e4 = line((0, 92.38), (0, 0))
}
f1 = extrude(s1, 12.43)
f2 = fillet(edges(of: f1, shape: "line"), vary(1.99, 3.51))
f3 = hole(at: h1.p0, on: plane(at: h1.p0, normal: Y, x: Z), d: 8.39, csink: (15.68, 82deg), depth: thru, thread: (10.07, 1.46))
"#,
    );
}

/// The same ball cut twice: in one n-ary pass the two equal spheres met everywhere and left open
/// edges. A tool equal to an earlier one combined the same way is left out of the pass.
#[test]
fn the_same_cut_twice() {
    builds_valid(
        r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h1(on: offset(xy, 29.62)) {
  p0 = pt(57.27, 50.61)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (74.83, 0))
  e2 = line((74.83, 0), (74.83, 101.22))
  e3 = line((74.83, 101.22), (0, 101.22))
  e4 = line((0, 101.22), (0, 0))
}
f1 = extrude(s1, 29.9)
sketch s2(on: plane(at: h1.p0, normal: Z, x: X)) {
  c = pt(0, 0)
}
f2 = ball(s2.c, 3.11, cut: true)
sketch s3(on: plane(at: h1.p0, normal: Z, x: X)) {
  c = pt(0, 0)
}
f3 = ball(s3.c, 3.11, cut: true)
"#,
    );
    builds_valid(
        r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h1(on: offset(xy, 20)) {
  p0 = pt(10, 10)
  p1 = pt(30, 20)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (40, 0))
  e2 = line((40, 0), (40, 30))
  e3 = line((40, 30), (0, 30))
  e4 = line((0, 30), (0, 0))
}
f1 = extrude(s1, 20)
sketch s2(on: plane(at: h1.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 3)
}
f2 = cut(s2, -5)
sketch s3(on: plane(at: h1.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 3)
}
f3 = cut(s3, -5)
sketch s4(on: plane(at: h1.p1, normal: Z, x: X)) {
  e1 = line((0, 0), (5, 0))
  e2 = line((5, 0), (5, 5))
  e3 = line((5, 5), (0, 5))
  e4 = line((0, 5), (0, 0))
}
f4 = extrude(s4, 4)
sketch s5(on: plane(at: h1.p1, normal: Z, x: X)) {
  e1 = line((0, 0), (5, 0))
  e2 = line((5, 0), (5, 5))
  e3 = line((5, 5), (0, 5))
  e4 = line((0, 5), (0, 0))
}
f5 = extrude(s5, 4)
"#,
    );
}

/// The same ball cut, added back and cut again (or added, cut and added): the third is no
/// repeat of the first, as the second undid it between them. The n-ary pass left it out and
/// built the block with the ball's other half; equal tools are left out only with nothing
/// combined otherwise between them. Also tools equal but for a hair, and the same loft or sweep
/// twice (known by their B-rep), regenerate as feature by feature.
#[test]
fn the_same_tool_after_its_undoing() {
    let same = |src: &str| {
        let b = built(src);
        let part = b.part();
        let volume = |s: Vec<forge_model::Solid>| -> f64 {
            s.iter()
                .map(|x| forge_topo::mass::mass(&x.body).expect("mass").volume)
                .sum()
        };
        let (seq, nary) = (
            volume(part.regenerate_solids_ids(Some(&b.ids)).expect("seq")),
            volume(part.regenerate_nary_solids_ids(1, Some(&b.ids)).expect("nary")),
        );
        assert!((seq - nary).abs() <= 1e-9 * seq, "{src}: seq {seq} nary {nary}");
    };
    same(
        r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h1(on: offset(xy, 20)) {
  p0 = pt(20, 15)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (40, 0))
  e2 = line((40, 0), (40, 30))
  e3 = line((40, 30), (0, 30))
  e4 = line((0, 30), (0, 0))
}
f1 = extrude(s1, 20)
sketch s2(on: plane(at: h1.p0, normal: Z, x: X)) {
  c = pt(0, 0)
}
f2 = ball(s2.c, 6, cut: true)
sketch s3(on: plane(at: h1.p0, normal: Z, x: X)) {
  c = pt(0, 0)
}
f3 = ball(s3.c, 6)
sketch s4(on: plane(at: h1.p0, normal: Z, x: X)) {
  c = pt(0, 0)
}
f4 = ball(s4.c, 6, cut: true)
"#,
    );
    same(
        r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h1(on: offset(xy, 20)) {
  p0 = pt(20, 15)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (40, 0))
  e2 = line((40, 0), (40, 30))
  e3 = line((40, 30), (0, 30))
  e4 = line((0, 30), (0, 0))
}
f1 = extrude(s1, 20)
sketch s2(on: plane(at: h1.p0, normal: Z, x: X)) {
  c = pt(0, 0)
}
f2 = ball(s2.c, 6)
sketch s3(on: plane(at: h1.p0, normal: Z, x: X)) {
  c = pt(0, 0)
}
f3 = ball(s3.c, 6, cut: true)
sketch s4(on: plane(at: h1.p0, normal: Z, x: X)) {
  c = pt(0, 0)
}
f4 = ball(s4.c, 6)
"#,
    );
    same(
        r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h1(on: offset(xy, 20)) {
  p0 = pt(20, 15)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (40, 0))
  e2 = line((40, 0), (40, 30))
  e3 = line((40, 30), (0, 30))
  e4 = line((0, 30), (0, 0))
}
f1 = extrude(s1, 20)
sketch s2(on: plane(at: h1.p0, normal: Z, x: X)) {
  c = pt(0, 0)
}
f2 = ball(s2.c, 6, cut: true)
sketch s3(on: plane(at: h1.p0, normal: Z, x: X)) {
  c = pt(0, 0)
}
f3 = ball(s3.c, 6.0000001, cut: true)
"#,
    );
    same(
        r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h1(on: offset(xy, 20)) {
  p0 = pt(20, 15)
  p1 = pt(20.0000001, 15)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (40, 0))
  e2 = line((40, 0), (40, 30))
  e3 = line((40, 30), (0, 30))
  e4 = line((0, 30), (0, 0))
}
f1 = extrude(s1, 20)
sketch s2(on: plane(at: h1.p0, normal: Z, x: X)) {
  c = pt(0, 0)
}
f2 = ball(s2.c, 6, cut: true)
sketch s3(on: plane(at: h1.p1, normal: Z, x: X)) {
  c = pt(0, 0)
}
f3 = ball(s3.c, 6, cut: true)
"#,
    );
    same(
        r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h1(on: offset(xy, 14)) {
  p0 = pt(20, 15)
}
sketch h2(on: offset(xy, 22)) {
  p0 = pt(20, 15)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (40, 0))
  e2 = line((40, 0), (40, 30))
  e3 = line((40, 30), (0, 30))
  e4 = line((0, 30), (0, 0))
}
f1 = extrude(s1, 20)
sketch s2_0(on: plane(at: h1.p0, normal: Z, x: X)) {
  e1 = line((-3, -3), (3, -3))
  e2 = line((3, -3), (3, 3))
  e3 = line((3, 3), (-3, 3))
  e4 = line((-3, 3), (-3, -3))
}
sketch s2_1(on: plane(at: h2.p0, normal: Z, x: X)) {
  e1 = line((-5, -5), (5, -5))
  e2 = line((5, -5), (5, 5))
  e3 = line((5, 5), (-5, 5))
  e4 = line((-5, 5), (-5, -5))
}
f2 = loft([s2_0, s2_1], cut: true)
sketch s3_0(on: plane(at: h1.p0, normal: Z, x: X)) {
  e1 = line((-3, -3), (3, -3))
  e2 = line((3, -3), (3, 3))
  e3 = line((3, 3), (-3, 3))
  e4 = line((-3, 3), (-3, -3))
}
sketch s3_1(on: plane(at: h2.p0, normal: Z, x: X)) {
  e1 = line((-5, -5), (5, -5))
  e2 = line((5, -5), (5, 5))
  e3 = line((5, 5), (-5, 5))
  e4 = line((-5, 5), (-5, -5))
}
f3 = loft([s3_0, s3_1], cut: true)
"#,
    );
    same(
        r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h1(on: offset(xy, 18)) {
  p0 = pt(5, 15)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (40, 0))
  e2 = line((40, 0), (40, 30))
  e3 = line((40, 30), (0, 30))
  e4 = line((0, 30), (0, 0))
}
f1 = extrude(s1, 20)
sketch w2(on: plane(at: h1.p0, normal: Y, x: X)) {
  q0 = line((0, 0), (30, 0))
}
sketch s2(on: plane(at: h1.p0, normal: X, x: -Z)) {
  e1 = circle((0, 0), 3)
}
f2 = sweep(s2, along: [w2.q0], cut: true)
sketch w3(on: plane(at: h1.p0, normal: Y, x: X)) {
  q0 = line((0, 0), (30, 0))
}
sketch s3(on: plane(at: h1.p0, normal: X, x: -Z)) {
  e1 = circle((0, 0), 3)
}
f3 = sweep(s3, along: [w3.q0], cut: true)
"#,
    );
}

/// Two pads on a block meeting along a vertical line, then a tilted rod added across that line
/// (forge-proof boolean on coplanar_stack 3): the rod cut the line's edge where the pads' boolean
/// had already named its end vertex from the same mark, and the new vertex took the same name
/// (DuplicateName). A name taken by an end of an edge being cut is passed over.
#[test]
fn a_cut_edge_end_keeps_its_name() {
    builds_valid(
        r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h1(on: offset(xy, 10)) {
  p0 = pt(25, 9)
  p1 = pt(19, 22)
}
sketch h2(on: offset(xy, 7.688098481494464)) {
  p0 = pt(20.318637845239692, 35.6698553783422)
}
sketch h3(on: offset(xy, 7.74183659105474)) {
  p0 = pt(20.88144878342117, 34.84501838629821)
}
sketch h4(on: offset(xy, 8.605430915799232)) {
  p0 = pt(21.269955244726273, 35.166371108074905)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (80, 0))
  e2 = line((80, 0), (80, 60))
  e3 = line((80, 60), (0, 60))
  e4 = line((0, 60), (0, 0))
}
f1 = extrude(s1, 10)
sketch s2(on: plane(at: h1.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (8, 0))
  e2 = line((8, 0), (8, 14))
  e3 = line((8, 14), (0, 14))
  e4 = line((0, 14), (0, 0))
}
f2 = extrude(s2, 3)
sketch s3(on: plane(at: h1.p1, normal: Z, x: X)) {
  e1 = line((0, 0), (15, 0))
  e2 = line((15, 0), (15, 5))
  e3 = line((15, 5), (0, 5))
  e4 = line((0, 5), (0, 0))
}
f3 = extrude(s3, 5)
sketch s4(on: plane(at: h3.p0, normal: axis(h3.p0, h4.p0), x: axis(h3.p0, h2.p0))) {
  e1 = circle((0, 0), 12.659848035169743)
}
f4 = extrude(s4, 6.017499452448846)
"#,
    );
}

/// A smooth loft added over a revolve and a countersunk hole on a slanted plane (history r12): the
/// loft's face split crossed itself, a traced curve ending inside a face, not on an edge. It
/// builds valid on 982b866 (and n-ary); kept so it stays that way. (The language revolves up to
/// no plane: the kernel feature is set as the part file had it, `to L9 off 5`.)
#[test]
fn loft_over_slanted_revolve_and_countersink() {
    let b = built(
        r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h1(on: offset(xy, 0.96)) {
  p0 = pt(-16.75, -1.13)
  p1 = pt(-17.627895572914383, -0.6511478693194266)
  p2 = pt(-16.271147869319428, -0.2521044270856154)
}
sketch h2(on: offset(xy, 16.12)) {
  p0 = pt(-10.84, 1.08)
}
sketch h3(on: offset(xy, 28.29)) {
  p0 = pt(-10.84, 1.08)
}
sketch h4(on: offset(xy, 5.070991656662093)) {
  p0 = pt(-14.521025678039965, 0.4719997839912516)
}
sketch h5(on: offset(xy, 5.38)) {
  p0 = pt(-13.57, 0.48)
}
sketch h6(on: offset(xy, 6.03)) {
  p0 = pt(-10.84, 1.08)
}
sketch h7(on: offset(xy, 6.3310561139661345)) {
  p0 = pt(-13.879018232613603, 0.48)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 17.88)
}
f1 = extrude(s1, 21.4)
f2 = workplane(on: plane(at: h1.p0, normal: axis(h1.p0, h1.p1), x: axis(h1.p0, h1.p2)))
sketch hr(on: offset(xy, -0.02)) {
  p0 = pt(-16.75, -1.13)
}
sketch s3(on: plane(at: hr.p0, normal: -Y, x: X)) {
  e1 = circle((7.24, -1.22), 1.37)
  ax = line((0, 0), (0, 1), construction: true)
}
f3 = revolve(s3, axis: s3.ax)
f4 = hole(at: h5.p0, on: plane(at: h5.p0, normal: axis(h5.p0, h4.p0), x: axis(h5.p0, h7.p0)), d: 1.51, csink: (3.26, 120deg), depth: 2.88)
sketch s5_0(on: plane(at: h6.p0, normal: Z, x: X)) {
  e1 = line((-1.16, -1.16), (1.16, -1.16))
  e2 = line((1.16, -1.16), (1.16, 1.16))
  e3 = line((1.16, 1.16), (-1.16, 1.16))
  e4 = line((-1.16, 1.16), (-1.16, -1.16))
}
sketch s5_1(on: plane(at: h2.p0, normal: Z, x: X)) {
  e1 = line((-5.02, -5.02), (5.02, -5.02))
  e2 = line((5.02, -5.02), (5.02, 5.02))
  e3 = line((5.02, 5.02), (-5.02, 5.02))
  e4 = line((-5.02, 5.02), (-5.02, -5.02))
}
sketch s5_2(on: plane(at: h3.p0, normal: Z, x: X)) {
  e1 = line((-3.98, -3.98), (3.98, -3.98))
  e2 = line((3.98, -3.98), (3.98, 3.98))
  e3 = line((3.98, 3.98), (-3.98, 3.98))
  e4 = line((-3.98, 3.98), (-3.98, -3.98))
}
f5 = loft([s5_0, s5_1, s5_2], smooth: true)
"#,
    );
    let mut part = b.part();
    let plane = part
        .features
        .iter()
        .position(|f| matches!(f, Feature::Plane(_)))
        .expect("the work plane");
    set_revolve(
        &mut part,
        Turn::To {
            target: Target::Plane(plane),
            offset: 5f64.to_radians(),
        },
        Combine::Add,
    );
    for s in part.regenerate_solids_ids(Some(&b.ids)).expect("regenerates") {
        assert!(
            forge_topo::check(&s.body).is_empty(),
            "{:?}",
            forge_topo::check(&s.body)
        );
    }
    nary_valid(&part, &b.ids);
}

/// A character without a glyph among ones that have one fails the text: it would be short.
#[test]
#[cfg_attr(target_family = "wasm", ignore = "the font is a file in a temp folder")]
fn a_character_the_font_lacks_is_not_left_out() {
    let text = |s: &str| {
        format!(
            "sketch s(on: xy) {{\n  o = rect(corner: (0, 0), size: (80, 30))\n}}\nb = extrude(s, 10)\n\
             sketch t(on: b.end) {{\n  p = pt(5, 5)\n}}\n\
             m = text(\"{s}\", font: \"{}\", size: 8, depth: 1, on: b.end, at: t.p)\n",
            forge_lang::fuzz::FONT
        )
    };
    assert_eq!(judge(&text("D")), Verdict::Ok);
    match judge(&text("D\u{263a}")) {
        Verdict::Refused(w) => assert!(w.contains("NoGlyph"), "{w}"),
        v => panic!("{v:?}"),
    }
}

/// A revolve kept across a block's corner: what is left is a closed solid, or the step is refused
/// as leaving none (`NotClosed`, naming the feature); never an open body. (The language keeps no
/// revolve: the kernel feature is set as the part file had it, `revolve keep`.)
#[test]
fn a_kept_revolve_leaves_a_closed_solid_or_is_refused() {
    // (built as an add, which the kernel may refuse: only its features are wanted)
    let b = build(
        r#"sketch h(on: offset(xy, -20)) {
  p = pt(-20, -20)
}
sketch s(on: plane(at: h.p, normal: Z, x: X)) {
  o = rect(corner: (0, 0), size: (40, 40))
}
f1 = extrude(s, 20)
sketch g(on: offset(xy, -14)) {
  p = pt(0, 0)
}
sketch r(on: plane(at: g.p, normal: -Z, x: -X)) {
  c = circle((10, 0), 4)
  ax = line((0, 0), (0, 1), construction: true)
}
f2 = revolve(r.c, axis: r.ax)
"#,
    );
    let mut part = b.part();
    set_revolve(&mut part, Turn::Angle(std::f64::consts::TAU), Combine::Keep);
    match part.regenerate_solids_ids(Some(&b.ids)) {
        Ok(s) => {
            for s in s {
                assert!(
                    forge_topo::check(&s.body).is_empty(),
                    "{:?}",
                    forge_topo::check(&s.body)
                );
            }
        }
        Err(e) => assert!(format!("{e:?}").contains("NotClosed"), "{e:?}"),
    }
}

/// Seed 0 of the .fg program generator at 12 groups: its last statement, an outside M6 thread on a
/// block moved and turned, its axis all but square to the block's side. The groove's flat past the
/// cylinder spans the whole pitch, and its carried-on surface's track reached the next turn's end
/// to end: the side's plane met it in loops touching tip to tip, and the trace ran on from turn to
/// turn (a chain of 114404 points, four minutes; cut down, a stretch run twice and the boolean's
/// classification inconsistent). The tracks now stay clear of each other: valid, in seconds.
#[test]
fn slow_thread_after_a_moved_body() {
    let t = std::time::Instant::now();
    let v = judge(
        r#"sketch S0(on: xy) {
  o = rect(corner: (0, 0), size: (72.78, 100.81))
}
L0 = extrude(S0, 15.69)
sketch H0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch H1(on: offset(xy, 21.099999999999998)) {
  p0 = pt(97.8, 0)
  p1 = pt(-97.8, 0)
}
L1 = fillet(edges(extreme: axis(H0.p0, H1.p0)) + edges(extreme: axis(H0.p0, H1.p1)), 3.55)
sketch H2(on: offset(xy, 1)) {
  p0 = pt(0, 50.41)
}
sketch S2(on: plane(at: H2.p0, normal: -Y, x: X)) {
  o = circle((6.67, -0.84), 3.27)
  ax = line((0, 0), (0, -1), construction: true)
}
L2 = revolve(S2.o, axis: S2.ax, cut: true)
sketch H3(on: offset(xy, 0)) {
  p0 = pt(56.07, 50.41)
}
L3 = hole(at: H3.p0, on: plane(at: H3.p0, normal: -Z), screw: "M4", convention: "ansi", fit: "tap", depth: 11.62)
sketch H4(on: offset(xy, 11.96)) {
  p0 = pt(34.66, 97.02)
}
L4 = ball(H4.p0, 14.32, cut: true)
sketch H5(on: offset(xy, -0.32)) {
  p0 = pt(0.73, 50.41)
}
sketch H6(on: offset(xy, -3)) {
  p0 = pt(0.73, 50.41)
}
sketch H7(on: offset(xy, 25.62)) {
  p0 = pt(0.73, 50.41)
}
sketch H8(on: offset(xy, 8.7)) {
  p0 = pt(0.73, 50.41)
}
sketch S5_0(on: plane(at: H5.p0, normal: Z)) {
  o = circle((0, 0), 8.91)
}
sketch S5_1(on: plane(at: H8.p0, normal: Z)) {
  o = circle((0, 0), 2.7)
}
sketch S5_2(on: plane(at: H7.p0, normal: Z)) {
  o = circle((0, 0), 7)
}
sketch C5(on: plane(at: H6.p0, normal: X, x: Z)) {
  l = line((0, 0), (31.62, 0))
}
L5 = loft([S5_0.o, S5_1.o, S5_2.o], smooth: true, centre: C5.l, cut: true)
sketch H9(on: offset(xy, 6.64)) {
  p0 = pt(89.7, 101.68)
}
sketch P6(on: plane(at: H9.p0, normal: Y, x: -X)) {
  path route(start: (0, 0)) {
    line(to: (43.5, 0))
    tarc(to: (48.01134126397017, 1.8686587360298264))
    line(to: (73.58032247167573, 27.43763994373538))
  }
}
sketch S6(on: plane(at: H9.p0, normal: -X, x: Z)) {
  o = rect(corner: (-1.74, -1.74), size: (3.48, 3.48))
}
L6 = sweep(S6.o, along: P6.route, cut: true)
sketch S7(on: faces(shape: "plane", normal: Y, extreme: -Y)) {
  o = poly([(-4.18, -2.79), near(4.18, -2.79), near(4.18, 2.79), near(-4.18, 2.79)])
  horizontal(o[0])
  vertical(o[1])
  horizontal(o[2])
  vertical(o[3])
  distance(o[1], 5.58)
  distance(o[0], 8.36)
  c = circle(near(0, 0), 0.51)
}
L7 = cut(S7, 4.9)
L8 = thread(hole: L3, length: 21.88)
sketch H10(on: offset(xy, -21.4)) {
  p0 = pt(-15.8, -96.39999999999999)
}
sketch H11(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch H12(on: offset(xy, 11.99)) {
  p0 = pt(32.25, 55.67)
}
sketch H13(on: offset(xy, 21.99)) {
  p0 = pt(32.25, 55.67)
}
L9 = movebody(of: L0, moves: [slide(along: axis(H11.p0, H10.p0), by: 40.37587893780147), spin(about: axis(H12.p0, H13.p0), by: 56.88deg)])
sketch H14(on: offset(xy, 0)) {
  p0 = pt(0, 0)
  p1 = pt(54.6, 83.8)
}
sketch H15(on: offset(xy, 1.32)) {
  p0 = pt(87.92, 60.13)
}
L10 = thread(on: plane(at: H15.p0, normal: axis(H14.p0, H14.p1)), at: H15.p0, spec: "M6", length: 18.43)
"#,
    );
    assert!(matches!(v, Verdict::Ok), "{v:?}");
    assert!(t.elapsed().as_secs() < 60, "{:.0} s", t.elapsed().as_secs_f64());
}

/// `slow_thread_after_a_moved_body` cut down to the block, the move and the thread: refused (an
/// inconsistent classification) while the trace ran on from turn to turn.
#[test]
fn thread_all_but_square_to_a_moved_side() {
    builds_valid(
        r#"sketch S0(on: xy) {
  o = rect(corner: (0, 0), size: (72.78, 100.81))
}
L0 = extrude(S0, 15.69)
sketch H10(on: offset(xy, -21.4)) {
  p0 = pt(-15.8, -96.39999999999999)
}
sketch H11(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch H12(on: offset(xy, 11.99)) {
  p0 = pt(32.25, 55.67)
}
sketch H13(on: offset(xy, 21.99)) {
  p0 = pt(32.25, 55.67)
}
L9 = movebody(of: L0, moves: [slide(along: axis(H11.p0, H10.p0), by: 40.37587893780147), spin(about: axis(H12.p0, H13.p0), by: 56.88deg)])
sketch H14(on: offset(xy, 0)) {
  p0 = pt(0, 0)
  p1 = pt(54.6, 83.8)
}
sketch H15(on: offset(xy, 1.32)) {
  p0 = pt(87.92, 60.13)
}
L10 = thread(on: plane(at: H15.p0, normal: axis(H14.p0, H14.p1)), at: H15.p0, spec: "M6", length: 18.43)
"#,
    );
}
