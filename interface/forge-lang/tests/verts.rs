//! Vertex references (`verts(..)`), a work point at a circle's centre, and sweeps and coils ended
//! on a work plane. Each part is held to its recorded numbers (volume, area, faces, edges).
use forge_model::{Feature, History};

fn build(src: &str) -> forge_lang::fuzz::Built {
    let b = forge_lang::fuzz::build(src);
    if let Some(e) = b.run.errors.first() {
        panic!(
            "{}\n{}",
            forge_lang::fuzz::message(e),
            forge_lang::report::text(&b.run, src)
        );
    }
    b
}

/// The part's volume, area, faces and edges, held to the recorded ones; the part valid.
fn holds(src: &str, want: (f64, f64, usize, usize)) {
    let b = build(src);
    let body = b.body_after(b.built()).expect("a part");
    assert!(forge_topo::check::check(&body).is_empty(), "invalid");
    let m = forge_topo::mass::mass(&body).expect("mass");
    let got = (m.volume, m.area, body.face_ids().count(), body.edge_ids().count());
    let near = |x: f64, y: f64| (x - y).abs() <= 1e-9 * x.abs().max(1.0);
    assert!(
        near(got.0, want.0) && near(got.1, want.1) && got.2 == want.2 && got.3 == want.3,
        "{got:?} != {want:?}"
    );
}

/// Each work feature's datum.
fn datums(b: &forge_lang::fuzz::Built) -> Vec<forge_model::Datum> {
    let h = History::new(&b.features, Some(&b.ids));
    (0..b.features.len())
        .filter(|&i| matches!(b.features[i], Feature::Work(_)))
        .map(|i| forge_model::work::datum(h, i).expect("a datum"))
        .collect()
}

/// The error a part stops at, as its report writes it.
fn refused(src: &str) -> String {
    let b = forge_lang::fuzz::build(src);
    assert!(!b.run.errors.is_empty(), "built");
    forge_lang::report::text(&b.run, src)
}

const BLOCK: &str =
    "sketch plan(on: xy) {\n  r = rect(corner: (0, 0), size: (40, 30))\n}\nbase = extrude(plan, 20)\n";
/// Corners (40, 0, 20), (0, 30, 20), (40, 30, 0), by the faces round them.
const V1: &str = "verts(of: base.side(plan.r[0]), between: (base.end, base.side(plan.r[1])))";
const V2: &str = "verts(of: base.side(plan.r[2]), between: (base.end, base.side(plan.r[3])))";
const V3: &str = "verts(of: base.side(plan.r[1]), between: (base.start, base.side(plan.r[2])))";

#[test]
fn work_features_at_vertices() {
    let b = build(&format!(
        "{BLOCK}p = workpoint(at: {V1})\nax = workaxis(through: ({V1}, {V2}))\npl = workplane(through: ({V1}, {V2}, {V3}))\n"
    ));
    let d = datums(&b);
    let forge_model::Datum::Point(p) = d[0] else {
        panic!("{d:?}")
    };
    assert_eq!(p, forge_math::Point3::new(40.0, 0.0, 20.0));
    let forge_model::Datum::Axis(l) = d[1] else {
        panic!("{d:?}")
    };
    assert_eq!(l.dir.get(), forge_math::Vec3::new(-0.8, 0.6, 0.0));
    // the kernel holds the vertex by name, never a point: the reference is the block's own
    let Feature::Work(forge_model::Work::Point(forge_model::PointBy::At(r))) = &b.features[2] else {
        panic!("{:?}", b.features[2])
    };
    assert_eq!(r.feature, 1);
    assert!(r.name.is_some());
}

/// The kernel follows the vertex: the block made deeper, the point rises with its corner.
#[test]
fn a_vertex_reference_follows_an_edit() {
    let mut b = build(&format!("{BLOCK}p = workpoint(at: {V1})\n"));
    let Feature::Extrude { extent, .. } = &mut b.features[1] else {
        panic!("{:?}", b.features[1])
    };
    *extent = forge_model::Extent::Distance(35.0);
    let forge_model::Datum::Point(p) = datums(&b)[0] else {
        panic!()
    };
    assert_eq!(p, forge_math::Point3::new(40.0, 0.0, 35.0));
}

#[test]
fn split_by_a_plane_through_vertices() {
    holds(
        &format!(
            "{BLOCK}pl = workplane(through: ({V1}, {V2}, {V3}))\nhalf = split(by: pl, keep: \"plus\")\n"
        ),
        (20000.0, 4681.024967590665, 7, 12),
    );
}

const BOSS: &str = "sketch top(on: base.end) {\n  c = circle((20, 15), 6)\n}\nboss = extrude(top.c, 8)\n";
const PIN: &str =
    "sketch ps(on: offset(xy, 20)) {\n  c = circle((5, 5), 2)\n  p = pt(5, 5)\n}\npin = extrude(ps.c, 4)\n";

/// A work point on a circular edge is its centre.
#[test]
fn a_work_point_at_a_circles_centre() {
    let b = build(&format!(
        "{BLOCK}{BOSS}cc = workpoint(at: edges(of: boss, shape: \"circle\", crease: \"convex\"))\n"
    ));
    assert_eq!(
        datums(&b),
        [forge_model::Datum::Point(forge_math::Point3::new(
            20.0, 15.0, 28.0
        ))]
    );
}

/// Copies placed at a circle's centre and at a vertex.
#[test]
fn a_pattern_placed_at_a_centre_and_a_vertex() {
    holds(
        &format!(
            "{BLOCK}{BOSS}{PIN}cc = workpoint(at: edges(of: boss, shape: \"circle\", crease: \"convex\"))\npins = pattern(of: pin, layout: places(at: [ps.p, cc, {V2}]))\n"
        ),
        (25055.575131606172, 5671.2388980384685, 15, 27),
    );
}

#[test]
fn a_fillet_law_from_a_vertex_and_a_setback_at_one() {
    holds(
        &format!(
            "{BLOCK}round = fillet(edges(between: (base.end, base.side(plan.r[1]))), vary(2, 5), from: {V1})\n"
        ),
        (23916.579974574553, 5148.878497239277, 7, 15),
    );
    holds(
        &format!(
            "{BLOCK}round = fillet(edges(between: (base.end, base.side(plan.r[1]))) + edges(between: (base.end, base.side(plan.r[0]))) + edges(between: (base.side(plan.r[0]), base.side(plan.r[1]))), 3, setbacks: [({V1}, 6)])\n"
        ),
        (23818.25569278973, 5071.229705477091, 10, 24),
    );
}

/// A sweep from inside the block out along +x, ended on a work plane at x = 60 (and 5 past it).
#[test]
fn a_sweep_ends_on_a_work_plane() {
    for (off, want) in [
        ("", (24565.486677646164, 5576.991118430775, 8, 15)),
        (", offset: 5", (24706.858347057703, 5671.2388980384685, 8, 15)),
    ] {
        holds(
            &format!(
                "{BLOCK}sketch H(on: offset(xy, 10)) {{\n  a = pt(30, 15)\n  b = pt(130, 15)\n  w = pt(60, 15)\n  o = pt(0, 0)\n  x = pt(0, 100)\n}}\nW = workplane(on: plane(at: H.w, normal: axis(H.a, H.b)))\nsketch P(on: plane(at: H.a, normal: axis(H.o, H.x), x: axis(H.a, H.b))) {{\n  path route(start: (0, 0)) {{\n    line(to: (60, 0))\n  }}\n}}\nsketch S(on: plane(at: H.a, normal: axis(H.a, H.b), x: axis(H.o, H.x))) {{\n  o = circle((0, 0), 3)\n}}\nrod = sweep(S.o, along: P.route, upto: W{off})\n"
            ),
            want,
        );
    }
}

/// A coil wound up to a work plane over the block (and 2 past it).
#[test]
fn a_coil_ends_on_a_work_plane() {
    for (off, want) in [
        ("", (25332.396594147067, 7075.248909073437, 10, 18)),
        (", offset: 2", (25510.04947336668, 7312.496093872512, 10, 18)),
    ] {
        holds(
            &format!(
                "{BLOCK}sketch H(on: offset(xy, 40)) {{\n  w = pt(20, 15)\n}}\nW = workplane(on: plane(at: H.w, normal: Z))\nsketch C(on: xz) {{\n  o = circle((10, 25), 1.5)\n  ax = line((0, 0), (0, 1), construction: true)\n}}\nspring = coil(C.o, axis: C.ax, pitch: 5, upto: W{off})\n"
            ),
            want,
        );
    }
}

/// A revolve turned up to a plane through its axis (and 5 degrees past it).
#[test]
fn a_revolve_ends_on_a_work_plane() {
    for (off, want) in [
        ("", (24640.644511744733, 5570.62421292653, 9, 18)),
        (", 5deg", (24720.326713953153, 5577.810115373606, 9, 18)),
    ] {
        holds(
            &format!(
                "{BLOCK}sketch H(on: offset(xy, 0)) {{\n  w = pt(20, 15)\n  o = pt(0, 0)\n  n = pt(-50, 86.6)\n}}\nW = workplane(on: plane(at: H.w, normal: axis(H.o, H.n)))\nsketch C(on: plane(at: H.w, normal: -Y, x: X)) {{\n  o = circle((25, 10), 4)\n  ax = line((0, 0), (0, 1), construction: true)\n}}\nring = revolve(C.o, axis: C.ax, angle: upto(W{off}))\n"
            ),
            want,
        );
    }
}

/// `next` the other way on an add reaches the kernel as its own reversed ToNext.
#[test]
fn next_back_is_the_kernels_reversed_to_next() {
    let b = build(&format!(
        "{BLOCK}sketch s(on: offset(base.end, 10)) {{\n  c = circle((20, 15), 4)\n}}\npost = extrude(s.c, next, flip: true)\n"
    ));
    assert!(matches!(
        b.features.last(),
        Some(Feature::Extrude {
            extent: forge_model::Extent::ToNext { reverse: true },
            op: forge_model::Combine::Add,
            ..
        })
    ));
}

#[test]
fn misuse_is_a_typed_error() {
    // several vertices where one is wanted
    assert!(refused(&format!("{BLOCK}p = workpoint(at: verts(of: base))\n")).contains("NotOne"));
    assert!(
        refused(&format!(
            "{BLOCK}round = fillet(edges(between: (base.end, base.side(plan.r[1]))), 2, from: verts(of: base.end))\n"
        ))
        .contains("NotOne")
    );
    // vertices are no edges, and verts has no crease
    assert!(refused(&format!("{BLOCK}round = fillet(verts(of: base), 2)\n")).contains("WrongType"));
    assert!(
        refused(&format!(
            "{BLOCK}p = workpoint(at: verts(of: base, crease: \"convex\"))\n"
        ))
        .contains("UnknownArg")
    );
    // a sweep ends on faces or a plane, not a number
    assert!(
        refused(&format!(
            "{BLOCK}sketch P(on: base.end) {{\n  path route(start: (5, 5)) {{\n    line(to: (5, 25))\n  }}\n}}\nsketch S(on: base.side(plan.r[0])) {{\n  o = circle((5, 10), 2)\n}}\nrod = sweep(S.o, along: P.route, upto: 5)\n"
        ))
        .contains("WrongType")
    );
}
