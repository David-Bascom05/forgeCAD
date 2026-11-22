//! The quick tier of the properties (the deep one is `examples/meta.rs`): small fuzz histories
//! and a few suite parts, each property on each.
use forge_model::{Part, suite};
use forge_proof::props;

fn fuzz() -> Vec<(String, Part)> {
    (0..4)
        .map(|s| (format!("fuzz{s}"), suite::fuzz(s + 1000, 6)))
        .collect()
}

fn holds(prop: &str, f: props::Property, parts: &[(String, Part)]) {
    let seeded: Vec<(u64, &(String, Part))> =
        parts.iter().enumerate().map(|(i, p)| (i as u64 + 1, p)).collect();
    suite::each(&seeded, |&(seed, (name, part))| {
        if let Some(hit) = f(part, seed) {
            panic!(
                "{prop} fails on {name} (seed {seed}): {}: {}",
                hit.kind, hit.detail
            );
        }
    });
}

#[test]
fn scaled_by_powers_of_two() {
    let mut parts = fuzz();
    parts.push(("shaft".into(), suite::shaft()));
    holds("scale", props::scale, &parts);
}

/// Quarter turns and mirrors in coordinate planes map every coordinate exactly (even seeds pick
/// those; odd ones an inexact rotation or a tilted plane, which M1 still trips).
#[test]
fn exact_turns_and_mirrors() {
    for (name, part) in fuzz() {
        for seed in [2, 4, 6] {
            for (prop, f) in [
                ("rigid", props::rigid as props::Property),
                ("mirror", props::mirror),
            ] {
                if let Some(hit) = f(&part, seed) {
                    panic!(
                        "{prop} fails on {name} (seed {seed}): {}: {}",
                        hit.kind, hit.detail
                    );
                }
            }
        }
    }
}

#[test]
fn neighbouring_tools_commute() {
    holds("order", props::order, &fuzz());
}

#[test]
fn boolean_identities() {
    holds("boolean", props::identities, &fuzz());
}

#[test]
fn certified() {
    let mut parts = fuzz();
    parts.push(("housing".into(), suite::housing()));
    holds("certify", props::certificate, &parts);
}

#[test]
fn brep_agrees_with_csg_point_by_point() {
    holds("membership", props::membership, &fuzz());
}

/// Every stage moved by a few ulps (even seeds) or up to 1e-12 (odd), then its next feature:
/// valid, with the same topology as from the stage unmoved.
#[test]
fn indifferent_to_ulps() {
    let mut parts = fuzz();
    parts.push(("housing".into(), suite::housing()));
    holds("ulp", props::ulp, &parts);
}

/// The nudge is not vacuous: it moves every vertex off its bits and leaves the body valid.
#[test]
fn nudge_moves_every_vertex() {
    let b = suite::housing().regenerate().expect("builds");
    for seed in [2, 3] {
        let m = props::nudged(&b, seed);
        let moved = b
            .vertices
            .iter()
            .zip(&m.vertices)
            .filter(|(x, y)| x.point != y.point)
            .count();
        assert!(
            moved * 10 >= b.vertices.len() * 9,
            "seed {seed}: {moved} of {} moved",
            b.vertices.len()
        );
        assert!(
            forge_topo::check(&m).is_empty(),
            "seed {seed}: {:?}",
            forge_topo::check(&m)
        );
    }
}

/// U2: a void cut a tolerance short of the top leaves its rim as a hole of the top face, a
/// tolerance off its plane. The notch's split of that face placed the hole by rays whose 3D box
/// test the probe, a few ulps further off, passed beside.
#[test]
fn hole_a_tolerance_off_its_plane() {
    let part = fg(r#"sketch h0(on: offset(xy, -1)) {
  p0 = pt(50, 14)
}
sketch h1(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h2(on: offset(xy, 6.999995)) {
  p0 = pt(11.59574686652088, 22.93665566589452)
}
sketch h3(on: offset(xy, 7)) {
  p0 = pt(22, 29)
  p1 = pt(42, 12)
}
sketch s1(on: plane(at: h1.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (60, 0))
  e2 = line((60, 0), (60, 40))
  e3 = line((60, 40), (0, 40))
  e4 = line((0, 40), (0, 0))
}
f1 = extrude(s1, 10)
sketch s2(on: plane(at: h3.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 3)
}
f2 = cut(s2, -3)
sketch s3(on: plane(at: h2.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 3)
}
f3 = cut(s3, -2.999995)
sketch s4(on: plane(at: h3.p1, normal: Z, x: X)) {
  e1 = circle((0, 0), 3)
}
f4 = cut(s4, -3)
sketch s5(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (10, 0))
  e2 = line((10, 0), (10, 8))
  e3 = line((10, 8), (0, 8))
  e4 = line((0, 8), (0, 0))
}
f5 = cut(s5, -12)
"#);
    for seed in [404, 406, 408] {
        if let Some(hit) = props::ulp(&part, seed) {
            panic!("seed {seed}: {}: {}", hit.kind, hit.detail);
        }
    }
}

/// A planar face's first loop is its outer one: the booleans read the face's place from it and
/// keep it when only holes change. A face listing a hole first is flagged, and is what broke
/// later cuts after a local imprint appended a new outer loop last.
#[test]
fn outer_loop_first() {
    let mut b = fg(r#"sketch h0(on: offset(xy, -5)) {
  p0 = pt(20, 20)
}
sketch h1(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch s1(on: plane(at: h1.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (60, 0))
  e2 = line((60, 0), (60, 40))
  e3 = line((60, 40), (0, 40))
  e4 = line((0, 40), (0, 0))
}
f1 = extrude(s1, 10)
sketch s2(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 4)
}
f2 = cut(s2, -20)
"#)
    .regenerate()
    .expect("builds");
    assert!(forge_topo::check(&b).is_empty());
    let f = b
        .face_ids()
        .find(|&f| b.face(f).loops.len() == 2)
        .expect("a face with a hole");
    b.faces[f.index()].loops.swap(0, 1);
    assert!(
        forge_topo::check(&b).contains(&forge_topo::Problem::OuterNotFirst { face: f }),
        "{:?}",
        forge_topo::check(&b)
    );
}

/// The oracle is not vacuous: a part's tools against the body without its last cut disagree.
#[test]
fn membership_catches_a_missing_cut() {
    let part = fg(r#"sketch h0(on: offset(xy, -5)) {
  p0 = pt(50, 50)
}
sketch h1(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch s1(on: plane(at: h1.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (100, 0))
  e2 = line((100, 0), (100, 100))
  e3 = line((100, 100), (0, 100))
  e4 = line((0, 100), (0, 0))
}
f1 = extrude(s1, 20)
sketch s2(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 20)
}
f2 = cut(s2, -30)
"#);
    let body = fg(r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (100, 0))
  e2 = line((100, 0), (100, 100))
  e3 = line((100, 100), (0, 100))
  e4 = line((0, 100), (0, 0))
}
f1 = extrude(s1, 20)
"#)
    .regenerate()
    .expect("builds");
    assert!(props::membership_of(&part, &body, 1).is_some());
}

fn block(x: f64, y: f64, z: f64) -> forge_topo::Body {
    let part = Part {
        features: vec![forge_model::Feature::Extrude {
            profile: forge_ops::Profile {
                plane: forge_math::Frame::WORLD.with_origin(forge_math::Point3::new(x, y, z)),
                loops: vec![forge_ops::Profile::rectangle(
                    forge_math::Point2::ORIGIN,
                    10.0,
                    10.0,
                    0,
                )],
            },
            extent: forge_model::Extent::Distance(10.0),
            op: forge_model::Combine::New,
        }],
    };
    let mut b = part.regenerate().expect("a block");
    for f in &mut b.faces {
        f.name = forge_topo::Name::NONE;
    }
    for e in &mut b.edges {
        e.name = forge_topo::Name::NONE;
    }
    for v in &mut b.vertices {
        v.name = forge_topo::Name::NONE;
    }
    b
}

fn kinds(b: &forge_topo::Body) -> Vec<String> {
    forge_proof::cert::certify(b, 0.05)
        .iter()
        .map(|f| f.kind())
        .collect()
}

/// A block turned inside out (every face's normal and loop reversed) passes `check`, whose
/// probes are local, but not ray parity.
#[test]
fn certify_catches_an_inside_out_solid() {
    let a = block(0.0, 0.0, 0.0);
    assert!(kinds(&a).is_empty(), "{:?}", kinds(&a));
    // rebuilt with each face's sense and each loop's way round reversed
    let mut b = forge_topo::Body::new();
    for v in &a.vertices {
        b.add_tolerant_vertex(v.point, v.tol, v.name);
    }
    for e in &a.edges {
        b.add_edge(e.curve.clone(), e.range, e.start, e.end, e.name);
    }
    let shell = b.add_shell();
    for f in a.face_ids() {
        let face = a.face(f);
        let g = b.add_face(shell, face.surface.clone(), !face.same_sense, face.name);
        for &l in &face.loops {
            let uses: Vec<_> = a
                .lp(l)
                .coedges()
                .rev()
                .map(|c| (a.coedge(c).edge, !a.coedge(c).forward))
                .collect();
            b.add_loop(g, &uses);
        }
    }
    assert!(forge_topo::check(&b).is_empty(), "{:?}", forge_topo::check(&b));
    assert!(
        kinds(&b).iter().all(|k| k == "Inverted") && kinds(&b).len() == 6,
        "{:?}",
        kinds(&b)
    );
}

/// Two overlapping blocks in one body: each shell is closed, but their faces cross (check's
/// ShellsOverlap says so too).
#[test]
fn certify_catches_crossing_shells() {
    let mut b = block(0.0, 0.0, 0.0);
    b.append(&block(4.3, 3.7, 5.9));
    assert!(kinds(&b).iter().any(|k| k == "ShellsOverlap"), "{:?}", kinds(&b));
    let k: Vec<String> = forge_proof::cert::geometric(&b, 0.05)
        .iter()
        .map(|f| f.kind())
        .collect();
    assert!(
        k.iter().any(|k| k == "Crossing") && k.iter().any(|k| k == "NotBounding"),
        "{k:?}"
    );
}

/// Two blocks sharing one corner vertex: every edge has two faces, but the vertex has two fans.
#[test]
fn certify_catches_a_pinched_vertex() {
    let mut b = block(0.0, 0.0, 0.0);
    let n = b.vertices.len() as u32;
    b.append(&block(10.0, 10.0, 10.0));
    let corner = |b: &forge_topo::Body, p: forge_math::Point3| {
        (0..b.vertices.len() as u32).find(|&v| b.vertices[v as usize].point.distance(p) < 1e-9)
    };
    let p = forge_math::Point3::new(10.0, 10.0, 10.0);
    let keep = corner(&b, p).expect("the first block's corner");
    assert!(keep < n);
    let gone = (n..b.vertices.len() as u32)
        .find(|&v| b.vertices[v as usize].point.distance(p) < 1e-9)
        .expect("the second block's corner");
    for e in &mut b.edges {
        for end in [&mut e.start, &mut e.end] {
            if end.0 == gone {
                *end = forge_topo::VertexId(keep);
            }
        }
    }
    let probs = forge_topo::certify(&b);
    assert!(
        probs
            .iter()
            .any(|p| matches!(p, forge_topo::Problem::VertexFans { fans: 2, .. })),
        "{probs:?}"
    );
}

/// A .fg part's kernel features. The parts here name no face or edge, so they regenerate the same
/// without the ids the language names features by (the properties regenerate them without).
fn fg(src: &str) -> Part {
    let b = forge_lang::fuzz::build(src);
    assert!(b.run.errors.is_empty(), "{src}: {:?}", b.run.errors);
    b.part()
}

/// M1: the same part rotated by an inexact rotation gets one more vertex between the same two
/// faces (forgecad-gate/repros/M1-redundant-vertex-under-rotation).
#[test]
#[ignore = "M1: booleans leave vertices between the same two faces, as rounding falls"]
fn m1_rotation_keeps_topology() {
    let part = fg(r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h1(on: offset(xy, 10.782129369606862)) {
  p0 = pt(77.1111125075544, 114.15395243903338)
}
sketch h2(on: offset(xy, 9.650791877482094)) {
  p0 = pt(77.53666804727273, 114.88810362341427)
}
sketch h3(on: offset(xy, 9.80044547195848)) {
  p0 = pt(76.92553480349156, 114.11085208111551)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (82, 0))
  e2 = line((82, 0), (82, 138))
  e3 = line((82, 138), (0, 138))
  e4 = line((0, 138), (0, 0))
}
f1 = extrude(s1, 17)
sketch s2(on: plane(at: h3.p0, normal: axis(h3.p0, h1.p0), x: axis(h3.p0, h2.p0))) {
  e1 = line((-5.087646897958789, -3.8327248781946053), (5.087646897958789, -3.8327248781946053))
  e2 = arc(center: (5.087646897958789, -1.2135283248381938), from: (5.087646897958789, -3.8327248781946053), to: (7.706843451315201, -1.2135283248381938))
  e3 = line((7.706843451315201, -1.2135283248381938), (7.706843451315201, 1.2135283248381938))
  e4 = arc(center: (5.087646897958789, 1.2135283248381938), from: (7.706843451315201, 1.2135283248381938), to: (5.087646897958789, 3.8327248781946053))
  e5 = line((5.087646897958789, 3.8327248781946053), (-5.087646897958789, 3.8327248781946053))
  e6 = arc(center: (-5.087646897958789, 1.2135283248381938), from: (-5.087646897958789, 3.8327248781946053), to: (-7.706843451315201, 1.213528324838194))
  e7 = line((-7.706843451315201, 1.2135283248381938), (-7.706843451315201, -1.2135283248381938))
  e8 = arc(center: (-5.087646897958789, -1.2135283248381938), from: (-7.706843451315201, -1.2135283248381936), to: (-5.08764689795879, -3.8327248781946053))
}
f2 = extrude(s2, 15.625457658136117)
"#);
    assert_eq!(props::rigid(&part, 1), None);
}

/// M3: two blind cuts sharing an end plane inside a block: seq and n-ary disagree on the faces.
#[test]
#[ignore = "M3: feature by feature leaves the coplanar ends of a void unmerged"]
fn m3_void_ends_seq_matches_nary() {
    let part = fg(r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h1(on: offset(xy, 45)) {
  p0 = pt(6, 28)
  p1 = pt(6, 26)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (200, 0))
  e2 = line((200, 0), (200, 150))
  e3 = line((200, 150), (0, 150))
  e4 = line((0, 150), (0, 0))
}
f1 = extrude(s1, 80)
sketch s2(on: plane(at: h1.p0, normal: X, x: Y)) {
  e1 = circle((0, 0), 2)
}
f2 = cut(s2, -13)
sketch s3(on: plane(at: h1.p1, normal: X, x: Y)) {
  e1 = circle((0, 0), 1)
}
f3 = cut(s3, -9)
"#);
    assert_eq!(props::backends(&part, 1), None);
}

/// M4: a part split by a cylinder into its difference and intersection, combined again.
#[test]
fn m4_split_halves_combine_valid() {
    let part = fg(r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h1(on: offset(xy, 16)) {
  p0 = pt(46, 60)
}
sketch h2(on: offset(xy, 20)) {
  p0 = pt(41, 71)
}
sketch h3(on: offset(xy, 22)) {
  p0 = pt(36, 58)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (100, 0))
  e2 = line((100, 0), (100, 100))
  e3 = line((100, 100), (0, 100))
  e4 = line((0, 100), (0, 0))
}
f1 = extrude(s1, 20)
sketch s2(on: plane(at: h3.p0, normal: Y, x: X)) {
  c = pt(0, 0)
}
f2 = ball(s2.c, 6, cut: true)
sketch s3(on: plane(at: h2.p0, normal: Y, x: X)) {
  e1 = circle((7, -1), 2)
  ax = line((0, 0), (0, 1), construction: true)
}
f3 = revolve(s3, axis: s3.ax)
sketch s4(on: plane(at: h1.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 8)
}
f4 = cut(s4, -17)
sketch s5(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (100, 0))
  e2 = line((100, 0), (100, 100))
  e3 = line((100, 100), (0, 100))
  e4 = line((0, 100), (0, 0))
}
f5 = extrude(s5, 20, new: true)
sketch s6(on: plane(at: h3.p0, normal: Y, x: X)) {
  c = pt(0, 0)
}
f6 = ball(s6.c, 6, cut: true)
sketch s7(on: plane(at: h2.p0, normal: Y, x: X)) {
  e1 = circle((7, -1), 2)
  ax = line((0, 0), (0, 1), construction: true)
}
f7 = revolve(s7, axis: s7.ax)
sketch s8(on: plane(at: h1.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 8)
}
f8 = intersect(s8, 17)
f9 = combine(f1, tools: [f5], op: "add")
"#);
    let body = props::build(&part).expect("builds");
    assert!(
        forge_topo::check(&body).is_empty(),
        "{:?}",
        forge_topo::check(&body)
    );
}

fn valid(src: &str) {
    let body = props::build(&fg(src)).expect("builds");
    assert!(
        forge_topo::check(&body).is_empty(),
        "{:?}",
        forge_topo::check(&body)
    );
}

/// M5: a ball whose poles lie on an edge; with its frame turned over (-xz) it builds.
#[test]
#[ignore = "M5: face split crossed itself"]
fn m5_ball_on_edge_either_frame() {
    valid(
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
}

/// M6: n-ary leaves open edges where a pocket's mouth is 0.00001 above the top.
#[test]
#[ignore = "M6: n-ary open edges near tolerance"]
fn m6_nary_near_tolerance_pocket() {
    let part = fg(r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h1(on: offset(xy, 7.00001)) {
  p0 = pt(20.829744830901628, 31.98774163479715)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (60, 0))
  e2 = line((60, 0), (60, 40))
  e3 = line((60, 40), (0, 40))
  e4 = line((0, 40), (0, 0))
}
f1 = extrude(s1, 10)
sketch s2(on: plane(at: h1.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 3)
}
f2 = cut(s2, -3.00001)
"#);
    assert_eq!(props::backends(&part, 1), None);
}

/// M9: a tilted pad added across a hole, its side plane cut at the hole's seam.
#[test]
#[ignore = "M9: FaceWrongSide on a lens at the hole's seam"]
fn m9_tilted_pad_over_hole() {
    valid(
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

/// A cone framed at its apex with a torus on its rim (degen_cone_torus 3, scaled 4 times): the
/// mesh laid the cone out with a floor of 1 mm on its radius, so the part and its scaled copy
/// meshed differently and their areas parted by 0.2%.
#[test]
fn cone_framed_at_its_apex_scales() {
    let (_, g) = forge_model::adversarial::families()
        .into_iter()
        .find(|(f, _)| *f == "degen_cone_torus")
        .expect("the family");
    let part = forge_model::adversarial::canonical(&g(3));
    if let Some(hit) = props::scale(&part, 662) {
        panic!("{}: {}", hit.kind, hit.detail);
    }
}

/// A seam stopped part way up a hole's wall by a torus that eats it: the wall's loop winds past
/// one whole turn, and a point of the wall drawn on the copy outside the loop was called Outside,
/// so a point in the open hole counted one crossing short and came out Inside (fuzz68 at count 20).
#[test]
fn point_in_a_hole_whose_seam_a_torus_cuts_short() {
    let part = fg(r#"sketch h0(on: offset(xy, -4)) {
  p0 = pt(166, 88)
  p1 = pt(166.98871550422476, 88.14980537942799)
}
sketch h1(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h2(on: offset(xy, 11.1)) {
  p0 = pt(167.8, 97.4)
}
sketch h3(on: offset(xy, 7)) {
  p0 = pt(162, 93)
  p1 = pt(161.63900589744227, 93.93256809827409)
}
sketch s1(on: plane(at: h1.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (199, 0))
  e2 = line((199, 0), (199, 103))
  e3 = line((199, 103), (0, 103))
  e4 = line((0, 103), (0, 0))
}
f1 = extrude(s1, 39)
sketch s2(on: plane(at: h0.p0, normal: Z, x: axis(h0.p0, h0.p1))) {
  e1 = circle((0, 0), 4)
}
f2 = cut(s2, -35)
sketch s3(on: plane(at: h2.p0, normal: Y, x: X)) {
  e1 = circle((8.7, 0), 2.7)
  ax = line((0, 0), (0, 1), construction: true)
}
f3 = revolve(s3, axis: s3.ax)
sketch s4(on: plane(at: h3.p0, normal: Z, x: axis(h3.p0, h3.p1))) {
  e1 = circle((0, 0), 7)
}
f4 = extrude(s4, 32)
"#);
    let body = part.regenerate().expect("builds").compact();
    let p = forge_math::Point3::new(167.46767621091539, 88.39294512605494, 2.2139809034473172);
    for body in [body, part.regenerate_nary(1).expect("n-ary").compact()] {
        assert_eq!(
            forge_bool::Index::new(&body).classify(p),
            forge_bool::Where::Outside
        );
        assert_eq!(props::membership_of(&part, &body, 69), None);
    }
}

/// A point of a sphere near its pole: a line in (u, v) from an edge's end is far from the edge
/// there, and the drawn loop said the wrong side, so a ray miscounted.
#[test]
fn point_on_a_ball_near_its_pole() {
    for (seed, count, at) in [(91, 6, 92), (18, 16, 19)] {
        let part = suite::fuzz(seed + 1000, count);
        assert_eq!(props::membership(&part, at), None, "fuzz{seed} count {count}");
    }
}

/// A torus crossing a top with enough holes for a local split: its two circles make a ring and a
/// disc inside it, and the older hole in the disc went to the ring (the first region holding it),
/// which is dropped: the hole's rim was left open and the volume wrong (fuzz19 at count 60).
#[test]
fn a_hole_in_a_disc_inside_a_ring_goes_to_the_disc() {
    let part = fg(r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h1(on: offset(xy, 10.045180038623714)) {
  p0 = pt(26.36181772890427, 48.31999358963703)
}
sketch h2(on: offset(xy, 10.12006847653015)) {
  p0 = pt(58.404464415959175, 98.776984432684)
}
sketch h3(on: offset(xy, 10.210897973462817)) {
  p0 = pt(26.85612632367834, 47.46664923991346)
}
sketch h4(on: offset(xy, 10.224669555097256)) {
  p0 = pt(30.777739606003802, 25.146464930412808)
  p1 = pt(30.285048461934732, 26.01666918031964)
}
sketch h5(on: offset(xy, 10.410305724956826)) {
  p0 = pt(54.636499807943885, 52.16417873140876)
}
sketch h6(on: offset(xy, 10.700248852150338)) {
  p0 = pt(31.964188863899732, 36.04360694203633)
}
sketch h7(on: offset(xy, 11.15338732730056)) {
  p0 = pt(26.52205464328622, 47.45616447656381)
}
sketch h8(on: offset(xy, 5.115359149262295)) {
  p0 = pt(30.847639674790706, 6.694448623656604)
  p1 = pt(31.846538678893488, 6.6475361562041915)
}
sketch h9(on: offset(xy, 5.372937186750471)) {
  p0 = pt(21.252169984522265, 29.55297220289709)
}
sketch h10(on: offset(xy, 9.99707989128916)) {
  p0 = pt(31.574209806131584, 35.7199672865349)
  p1 = pt(31.11173179774462, 36.606598039305425)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (70.72881866274645, 0))
  e2 = line((70.72881866274645, 0), (70.72881866274645, 103.24012644860403))
  e3 = line((70.72881866274645, 103.24012644860403), (0, 103.24012644860403))
  e4 = line((0, 103.24012644860403), (0, 0))
}
f1 = extrude(s1, 10.566712767523594)
sketch s2(on: plane(at: h5.p0, normal: Y, x: X)) {
  e1 = circle((8.351160778816524, 0), 2.7599878880234967)
  ax = line((0, 0), (0, 1), construction: true)
}
f2 = revolve(s2, axis: s2.ax, cut: true)
sketch s3(on: plane(at: h8.p0, normal: Z, x: axis(h8.p0, h8.p1))) {
  e1 = circle((0, 0), 2.9515512238302954)
}
f3 = extrude(s3, 7.466192008905245)
sketch s4(on: plane(at: h4.p0, normal: Z, x: axis(h4.p0, h4.p1))) {
  e1 = circle((0, 0), 6.201177187633486)
}
f4 = cut(s4, -6.096829815264994)
sketch s5(on: plane(at: h3.p0, normal: axis(h3.p0, h7.p0), x: axis(h3.p0, h1.p0))) {
  e1 = line((-3.105823834349979, -2.0705492228999858), (3.105823834349979, -2.0705492228999858))
  e2 = line((3.105823834349979, -2.0705492228999858), (3.105823834349979, 2.0291382384419863))
  e3 = line((3.105823834349979, 2.0291382384419863), (-3.105823834349979, 2.0291382384419863))
  e4 = line((-3.105823834349979, 2.0291382384419863), (-3.105823834349979, -2.0705492228999858))
}
f5 = cut(s5, -6.878676428117549)
sketch s6(on: plane(at: h2.p0, normal: Y, x: X)) {
  c = pt(0, 0)
}
f6 = ball(s6.c, 4.1911579125771)
sketch s7(on: plane(at: h9.p0, normal: Y, x: X)) {
  c = pt(0, 0)
}
f7 = ball(s7.c, 5.307822009781472)
sketch s8(on: plane(at: h10.p0, normal: Z, x: axis(h10.p0, h10.p1))) {
  e1 = circle((0, 0), 3.6656106845435033)
}
f8 = cut(s8, -4.281061686283083)
sketch s9(on: plane(at: h6.p0, normal: Y, x: X)) {
  e1 = circle((5.2713150056640465, 0), 1.0039475779366047)
  ax = line((0, 0), (0, 1), construction: true)
}
f9 = revolve(s9, axis: s9.ax)
"#);
    for b in [part.regenerate(), part.regenerate_nary(1)] {
        let b = b.expect("builds");
        assert!(forge_topo::check(&b).is_empty(), "{:?}", forge_topo::check(&b));
    }
    assert_eq!(props::membership(&part, 20), None);
}

#[test]
fn every_feature_takes_effect_or_is_refused() {
    let mut parts = fuzz();
    parts.extend(
        suite::parts()
            .into_iter()
            .filter(|p| p.1.features.len() <= 40)
            .map(|(n, p)| (n.to_string(), p)),
    );
    holds("effect", props::effect, &parts);
}

/// The check itself: a body missing one of a pattern's copies fails it, for a cut and for an add.
#[test]
fn effect_catches_a_skipped_copy() {
    for op in ["cut", "add"] {
        // a cut runs against its sketch's normal: -5 is 5 up, as the add
        let tool = if op == "cut" {
            "cut(s, -5)"
        } else {
            "extrude(s, 5)"
        };
        let part = |n: usize| {
            fg(&format!(
                "sketch b(on: xy) {{
  o = rect(corner: (0, 0), size: (100, 40))
}}
base = extrude(b, 10)
sketch s(on: offset(xy, 8)) {{
  c = circle((10, 20), 4)
}}
h = {tool}
copies = pattern(of: h, layout: lin(along: X, n: {n}, step: 20))
"
            ))
        };
        let (whole, short) = (part(4), part(3));
        // the pattern
        let last = whole.features.len() - 1;
        let body = short.regenerate().expect("builds").compact();
        let mut r = forge_math::rng::Rng::new(5);
        assert!(
            props::tools_held(&whole, last, &body, false, &mut r).is_some(),
            "{op}"
        );
        let all = whole.regenerate().expect("builds").compact();
        assert_eq!(props::tools_held(&whole, last, &all, false, &mut r), None, "{op}");
    }
}

/// A cut may take a whole body of several and say so; nothing else may lose one.
#[test]
fn effect_lets_a_cut_take_a_whole_body() {
    use forge_model::{BodyFeature, Combine, Feature, Pick};
    let cut = fg(r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h1(on: offset(xy, 8)) {
  p0 = pt(10, 20)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (100, 0))
  e2 = line((100, 0), (100, 40))
  e3 = line((100, 40), (0, 40))
  e4 = line((0, 40), (0, 0))
}
f1 = extrude(s1, 10)
sketch s2(on: plane(at: h1.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 4)
}
f2 = cut(s2, -5)
"#)
    .features
    .last()
    .expect("a tool")
    .clone();
    let add = fg(r#"sketch h0(on: offset(xy, 0)) {
  p0 = pt(0, 0)
}
sketch h1(on: offset(xy, 8)) {
  p0 = pt(10, 20)
}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = line((0, 0), (100, 0))
  e2 = line((100, 0), (100, 40))
  e3 = line((100, 40), (0, 40))
  e4 = line((0, 40), (0, 0))
}
f1 = extrude(s1, 10)
sketch s2(on: plane(at: h1.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 4)
}
f2 = extrude(s2, 5)
"#)
    .features
    .last()
    .expect("a tool")
    .clone();
    let on = |feature| {
        Feature::Body(Box::new(BodyFeature::In {
            bodies: Pick::All,
            feature,
        }))
    };
    let bodies = |n: usize| vec![forge_topo::Body::new(); n];
    assert_eq!(props::bodies_held(&on(cut.clone()), &bodies(2), &bodies(1)), None);
    assert!(props::bodies_held(&on(add), &bodies(2), &bodies(1)).is_some());
    let combine = |op| {
        Feature::Body(Box::new(BodyFeature::Combine {
            op,
            target: 0,
            tools: vec![1],
            keep: true,
        }))
    };
    assert_eq!(
        props::bodies_held(&combine(Combine::Cut), &bodies(2), &bodies(1)),
        None
    );
    assert!(props::bodies_held(&combine(Combine::Add), &bodies(2), &bodies(1)).is_some());
}

// Ruled lofts cut by a cylinder: the curve on the wall comes to a tip, so a plane across it cuts two
// arms (Newton from a far seed landed on the other, as exact as the right one), and one chain step
// can hold the whole turn at the tip.
#[test]
fn mass_agrees_with_the_mesh_on_a_curve_with_two_arms() {
    let parts = [
        (
            "two arms",
            r#"sketch h0(on: offset(xy, -5)) {
  p0 = pt(17.0662172828, 19.6484941013)
}
sketch h1(on: offset(xy, 0)) {
  p0 = pt(1.92387, 2.39903)
}
sketch h2(on: offset(xy, 11.8710917543)) {
  p0 = pt(-105.516428921, 17.6006689353)
  p1 = pt(-104.53005713435162, 17.43613717999394)
}
sketch h3(on: offset(xy, 18.826858799168352)) {
  p0 = pt(63.19909787219998, -86.31291108709927)
}
sketch h4(on: offset(xy, 19.811013581356804)) {
  p0 = pt(62.6083101842512, -85.48683041763428)
}
sketch h5(on: offset(xy, 19.826670494)) {
  p0 = pt(63.1990978722, -86.2935055498)
}
sketch h6(on: offset(xy, 66.9086)) {
  p0 = pt(2.77072, 2.03649)
}
sketch s1_0(on: plane(at: h1.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 20.2127)
}
sketch s1_1(on: plane(at: h6.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 23.9307)
}
f1 = loft([s1_0, s1_1])
sketch s2(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 1.69944)
}
f2 = cut(s2, -76.9086)
sketch s3(on: plane(at: h2.p0, normal: axis(h2.p0, h2.p1), x: Z)) {
  e1 = circle((0, 0), 5.42322)
}
f3 = cut(s3, -213.949)
sketch s4(on: plane(at: h5.p0, normal: axis(h5.p0, h4.p0), x: axis(h5.p0, h3.p0))) {
  e1 = circle((0, 0), 2.62392)
}
f4 = cut(s4, -213.949)
"#,
        ),
        (
            "no region",
            r#"sketch h0(on: offset(xy, -5)) {
  p0 = pt(-11.4885874662, -1.52046134588)
  p1 = pt(7.88027351596, 8.6001202425)
}
sketch h1(on: offset(xy, 0)) {
  p0 = pt(-2.76075, -0.743971)
}
sketch h2(on: offset(xy, 112.723)) {
  p0 = pt(-1.45221, -2.28102)
}
sketch h3(on: offset(xy, 56.3615)) {
  p0 = pt(1.86503, 0.567644)
}
sketch s1_0(on: plane(at: h1.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 5.58991)
}
sketch s1_1(on: plane(at: h3.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 10.7266)
}
sketch s1_2(on: plane(at: h2.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 12.0474)
}
f1 = loft([s1_0, s1_1, s1_2])
sketch s2(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 2.05112)
}
f2 = cut(s2, -122.723)
sketch s3(on: plane(at: h0.p1, normal: Z, x: X)) {
  e1 = circle((0, 0), 1.84516)
}
f3 = cut(s3, -122.723)
"#,
        ),
        (
            "turn in a chain step",
            r#"sketch h0(on: offset(xy, -5)) {
  p0 = pt(17.0008755596, 5.12124836525)
}
sketch h1(on: offset(xy, 0)) {
  p0 = pt(1.71584, 2.90262)
}
sketch h2(on: offset(xy, 22.4474540782)) {
  p0 = pt(-18.3281298161, 108.113191765)
  p1 = pt(-18.160987367789954, 107.12725900767173)
}
sketch h3(on: offset(xy, 47.8258)) {
  p0 = pt(-0.877333, -0.276794)
}
sketch h4(on: offset(xy, 95.6516)) {
  p0 = pt(2.41955, -0.408761)
}
sketch s1_0(on: plane(at: h1.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 22.4298)
}
sketch s1_1(on: plane(at: h3.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 10.8143)
}
sketch s1_2(on: plane(at: h4.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 17.746)
}
f1 = loft([s1_0, s1_1, s1_2])
sketch s2(on: plane(at: h2.p0, normal: axis(h2.p0, h2.p1), x: Z)) {
  e1 = circle((0, 0), 4.05556)
}
f2 = cut(s2, -219.311)
sketch s3(on: plane(at: h0.p0, normal: Z, x: X)) {
  e1 = circle((0, 0), 1.14856)
}
f3 = extrude(s3, 105.652)
"#,
        ),
    ];
    for (name, src) in parts {
        let part = fg(src);
        assert!(part.regenerate().is_ok(), "{name} builds");
        assert!(
            props::mass_mesh(&part, 1).is_none(),
            "{name}: {:?}",
            props::mass_mesh(&part, 1)
        );
        // and through the n-ary road
        let body = part.regenerate_nary(1).expect("n-ary builds");
        let m = forge_topo::mass::mass(&body.compact()).expect("a mass");
        let seq = forge_topo::mass::mass(&part.regenerate().unwrap().compact()).expect("a mass");
        assert!(
            (m.volume - seq.volume).abs() < 1e-6 * seq.volume,
            "{name}: {} vs {}",
            m.volume,
            seq.volume
        );
    }
}

#[test]
fn every_fillet_holds_against_the_part_before_it() {
    let corpus: Vec<(String, Part)> = forge_model::fillets::cases()
        .into_iter()
        .map(|(n, p, _)| (n.to_string(), p))
        .collect();
    holds("blend", props::blend, &corpus);
}

/// A check never seen to fail proves nothing: each of the blend checks against a body made wrong
/// in the way it is there to see.
#[test]
fn the_ridge_check_tells_a_ball_on_a_ridge_from_one_on_a_true_rim() {
    use forge_proof::blends::ridges;
    let body = |src: &str| {
        let b = forge_lang::fuzz::build(src);
        assert!(b.run.errors.is_empty(), "{src}: {:?}", b.run.errors);
        b.body().expect("a body")
    };
    // a ring of a shallow dome: wall, then short flats rising inward. An arc of radius 2 from the
    // wall ends on the first ridge, with the ball's centre under the top: the next flat runs into it
    let (wx, wy) = (31.7f64, 11.5f64);
    let ridge = (31.26f64, 11.53f64);
    let cy = ridge.1 - (4.0f64 - (ridge.0 - (wx - 2.0)) * (ridge.0 - (wx - 2.0))).sqrt();
    let cx = wx - 2.0;
    // the outline turned round the sketch's y; its second segment the wall's top or the arc (the
    // same entity name e2, so the faces after it keep their names), the rest from the ridge
    let ring = |second: String, wall_top: f64| {
        format!(
            "sketch h0(on: offset(xy, 0)) {{
  p0 = pt(0, 0)
}}
sketch s1(on: plane(at: h0.p0, normal: Z, x: X)) {{
  e1 = line(({wx}, 0), ({wx}, {wall_top}))
  e2 = {second}
  e3 = line(({r0}, {r1}), (30, 11.6))
  e4 = line((30, 11.6), (25, 11.6))
  e5 = line((25, 11.6), (25, 0))
  e6 = line((25, 0), ({wx}, 0))
  ax = line((0, 0), (0, 1), construction: true)
}}
f1 = revolve(s1, axis: s1.ax)
",
            r0 = ridge.0,
            r1 = ridge.1
        )
    };
    let sharp = ring(format!("line(({wx}, {wy}), ({}, {}))", ridge.0, ridge.1), wy);
    let rounded = ring(
        format!(
            "arc(center: ({cx}, {cy}), from: ({wx}, {cy}), to: ({}, {}))",
            ridge.0, ridge.1
        ),
        cy,
    );
    let (before, after) = (body(&sharp), body(&rounded));
    assert!(
        !ridges(&before, &after).is_empty(),
        "a ridge the next flat covers"
    );
    // past the wall's far side: a plate 1 thick, ball 2, its far face falling away
    let plate_src = "sketch s(on: xy) {\n  o = rect(corner: (0, 0), size: (1, 30))\n}\nb = extrude(s, 20)\n";
    let plate = body(plate_src);
    // the plate's top edge at x = 0
    let rim = body(&format!(
        "{plate_src}r = fillet(edges(of: b.end, along: Y, extreme: -X), 2)\n"
    ));
    assert!(ridges(&plate, &rim).is_empty(), "a true rim");
}

#[test]
fn the_blend_checks_fail_what_they_should() {
    use forge_math::Point3;
    use forge_ops::{FilletEdge, FilletOptions, Radius, fillet, fillet_with};
    use forge_proof::blends::Radii;
    let before = forge_lang::fuzz::build(
        "sketch s(on: xy) {\n  o = rect(corner: (0, 0), size: (100, 80))\n}\nb = extrude(s, 40)\n",
    )
    .body()
    .expect("a block");
    let edge = |p: Point3| {
        before
            .edge_ids()
            .find(|&e| {
                use forge_geom::CurveGeom;
                let x = before.edge(e);
                x.curve.eval(x.range.mid()).distance(p) < 1e-9
            })
            .expect("the edge")
    };
    let (a, b) = (
        edge(Point3::new(50.0, 80.0, 40.0)),
        edge(Point3::new(100.0, 40.0, 40.0)),
    );
    let pick = |r: f64| {
        [a, b].map(|edge| FilletEdge {
            edge,
            radius: Radius::Constant(r),
        })
    };
    let good = fillet(&before, &pick(5.0), 1).expect("a mitre");
    let held = forge_proof::blends::audit(&before, &good, &Radii::List(vec![5.0]), 1);
    assert_eq!(held.unsupported, 0);
    assert_eq!(forge_proof::blends::points(&before, &good, 400, 1).wrong, 0);
    assert!(forge_proof::blends::needless_patches(&before, &good).is_empty());
    // held to the wrong radius, the balls stand off their supports
    assert!(forge_proof::blends::audit(&before, &good, &Radii::List(vec![5.01]), 1).unsupported > 0);
    // a patched corner where two blends meet beside a sharp edge: seen as a patch, and by points
    let v = before.edge(a).start;
    let at = if before.vertex(v).point.distance(Point3::new(100.0, 80.0, 40.0)) < 1e-9 {
        v
    } else {
        before.edge(a).end
    };
    let opts = FilletOptions {
        setbacks: vec![(at, None, 6.0)],
        ..FilletOptions::default()
    };
    let patched = fillet_with(&before, &pick(5.0), &opts, 1).expect("a patch");
    assert!(!forge_proof::blends::needless_patches(&before, &patched).is_empty());
    assert!(forge_proof::blends::points(&before, &patched, 2000, 1).wrong > 0);
    // a blend from another part entirely: points the round did not change are claimed by nothing
    let other = fillet(&before, &pick(9.0), 1).expect("another mitre");
    assert!(forge_proof::blends::audit(&before, &other, &Radii::List(vec![5.0]), 1).unsupported > 0);
}

/// A thin blade with round corners running out over a disc's rim. Rounds at its foot wider than the
/// flat at its narrow end mitre there, some resting on an edge of the blade carried on past its end by
/// less than the radius. The audit says so as such, and still refuses another radius.
#[test]
fn audit_tells_a_mitre_on_a_rim_carried_on() {
    use forge_bool::{Op, boolean};
    use forge_math::{Frame, Point2, Point3, UnitVec3, Vec2, Vec3};
    use forge_ops::{
        Axis2, FilletEdge, Profile, ProfileLoop, Radius, Segment, extrude, fillet, fillet_kind, revolve,
    };
    use forge_proof::blends::Radii;
    use std::f64::consts::{PI, TAU};
    let line = |a: (f64, f64), b: (f64, f64), id: u64| {
        (
            Segment::Line {
                a: Point2::new(a.0, a.1),
                b: Point2::new(b.0, b.1),
            },
            id,
        )
    };
    let hub = revolve(
        &Profile {
            plane: Frame::from_zx(Point3::ORIGIN, -UnitVec3::Y, Vec3::X).expect("frame"),
            loops: vec![ProfileLoop::new(vec![
                line((0.0, -5.0), (84.0, -5.0), 0),
                line((84.0, -5.0), (84.0, 2.0), 1),
                line((84.0, 2.0), (30.0, 20.0), 2),
                line((30.0, 20.0), (0.0, 20.0), 3),
                line((0.0, 20.0), (0.0, -5.0), 4),
            ])],
        },
        Axis2 {
            origin: Point2::ORIGIN,
            dir: Vec2::Y,
        },
        TAU,
        1,
    )
    .expect("hub");
    let (a, b, c) = (28.4, 0.4, 1.6);
    let arc = |x: f64, y: f64, start: f64, id| {
        (
            Segment::Arc {
                center: Point2::new(x, y),
                radius: c,
                start,
                sweep: PI / 2.0,
            },
            id,
        )
    };
    let outline = ProfileLoop::new(vec![
        line((-a, -2.0), (a, -2.0), 0),
        arc(a, -b, -PI / 2.0, 1),
        line((30.0, -b), (30.0, b), 2),
        arc(a, b, 0.0, 3),
        line((a, 2.0), (-a, 2.0), 4),
        arc(-a, b, PI / 2.0, 5),
        line((-30.0, b), (-30.0, -b), 6),
        arc(-a, -b, PI, 7),
    ]);
    let x = Vec3::new(1.0, 1.0, 0.0);
    let centre = Point3::ORIGIN + x.normalize().expect("unit").get() * 55.0;
    let blade = extrude(
        &Profile {
            plane: Frame::from_zx(centre, UnitVec3::Z, x).expect("frame"),
            loops: vec![outline],
        },
        60.0,
        2,
    )
    .expect("blade");
    let before = boolean(&hub, &blade, Op::Union, 3).expect("union");
    let root: Vec<FilletEdge> = before
        .edge_ids()
        .filter(|&e| fillet_kind(&before, e) == Some(false))
        .map(|edge| FilletEdge {
            edge,
            radius: Radius::Constant(1.5),
        })
        .collect();
    let done = fillet(&before, &root, 9).expect("rounds");
    let held = forge_proof::blends::audit(&before, &done, &Radii::List(vec![1.5]), 1);
    assert_eq!(held.unsupported, 0);
    assert!(held.carried > 0);
    assert!(held.worst_carried < 0.5);
    assert!(forge_proof::blends::audit(&before, &done, &Radii::List(vec![1.51]), 1).unsupported > 0);
}
