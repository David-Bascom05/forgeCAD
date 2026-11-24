//! Booleans on solids taken out of the Haas machine models (tools/kernel-stress/extract.py on the
//! kernel-stress branch; corpus ids are research/kernel-stress/corpus.tsv's). Each test does what
//! the sweep (verify/forge-stress) did to the solid, and asks for a valid answer of the right size.
use forge_bool::{Op, boolean};
use forge_math::{Frame, Point2, Point3, UnitVec3};
use forge_step::{Limits, import};
use forge_topo::{Body, check, mass};

fn one(name: &str) -> Body {
    let p = format!("{}/tests/data/haas/{name}", env!("CARGO_MANIFEST_DIR"));
    let imp = import(&std::fs::read(&p).unwrap(), &Limits::default()).unwrap();
    assert!(imp.refusals.is_empty(), "{:?}", imp.refusals);
    assert_eq!(imp.solids.len(), 1);
    imp.solids.into_iter().next().unwrap().body
}

fn volume(b: &Body) -> f64 {
    mass::mass_rel(b, 1e-6).unwrap().volume
}

/// forge-stress's box_sub and box_int: a box from the solid's centre to past its +x and +z sides,
/// through it in y. Each answer checks, and the two make up the solid.
fn quarter_box(b: &Body) {
    let bx = b.faces_box();
    let (c, m) = (bx.center(), 0.1 * bx.diagonal() + 1.0);
    let (lo, hi) = (
        Point3::new(c.x, bx.y.lo - m, c.z),
        Point3::new(bx.x.hi + m, bx.y.hi + m, bx.z.hi + m),
    );
    let p = forge_ops::Profile {
        plane: Frame::from_z(lo, UnitVec3::Z),
        loops: vec![forge_ops::Profile::rectangle(
            Point2::ORIGIN,
            hi.x - lo.x,
            hi.y - lo.y,
            8000,
        )],
    };
    let tool = forge_ops::extrude(&p, hi.z - lo.z, 8010).unwrap();
    let mut sum = 0.0;
    for op in [Op::Subtract, Op::Intersect] {
        let r = boolean(b, &tool, op, 9000).unwrap_or_else(|e| panic!("{op:?}: {e}"));
        let p = check(&r);
        assert!(p.is_empty(), "{op:?}: {p:?}");
        sum += volume(&r);
    }
    let v = volume(b);
    assert!((sum - v).abs() < 1e-6 * v, "{sum} vs {v}");
}

/// K012/K013, HA5C-T #51342: a ring of 12 torus patches (an O-ring's groove), its seams on the
/// planes through its axis and its equator. The quarter box's faces lie on those seams; both ops
/// were refused, Internal(SplitCrossed) on the box's face in the equator plane.
#[test]
fn a_box_on_a_torus_rings_seams() {
    quarter_box(&one("torus_ring_12faces.step"));
}

/// `b` moved by `d`, as forge-stress moves its copies (names salted, so the two do not share any).
fn moved(b: &Body, d: forge_math::Vec3) -> Body {
    use forge_geom::{CurveGeom, SurfaceGeom};
    let f = Frame::WORLD.with_origin(Point3::ORIGIN + d);
    let salt = forge_topo::Name::feature(0x5eed);
    let mut c = b.clone();
    for v in &mut c.vertices {
        v.point = f.point_to_world(v.point);
        v.name = v.name.salted(salt);
    }
    for e in &mut c.edges {
        e.curve = e.curve.transformed(&f);
        e.name = e.name.salted(salt);
        e.bbox = Default::default();
    }
    for x in &mut c.faces {
        x.surface = x.surface.transformed(&f);
        x.name = x.name.salted(salt);
        x.bbox = Default::default();
        x.query = Default::default();
    }
    c.hulls = Default::default();
    c
}

/// forge-stress's union_over2 and int_over2: with a copy moved 0.4137 of its width along x. Each
/// answer checks, and union + intersection = twice the solid.
fn over_copy(b: &Body) {
    let w = b.faces_box().x.width();
    let copy = moved(b, forge_math::Vec3::new(0.4137 * w, 0.0, 0.0));
    let mut sum = 0.0;
    for op in [Op::Union, Op::Intersect] {
        let r = boolean(b, &copy, op, 9000).unwrap_or_else(|e| panic!("{op:?}: {e}"));
        let p = check(&r);
        assert!(p.is_empty(), "{op:?}: {p:?}");
        sum += volume(&r);
    }
    let v = 2.0 * volume(b);
    assert!((sum - v).abs() < 1e-6 * v, "{sum} vs {v}");
}

/// K004/K005, HRTA6 #14022: an O-ring of 4 torus patches with a copy beside it, overlapping. Two
/// equal tori side by side are tangent where the plane between them crosses the top and bottom of
/// the tube, and there the intersection's branches cross. Both ops were refused,
/// Internal(InconsistentClassification).
#[test]
fn an_oring_with_a_copy_over_it() {
    over_copy(&one("oring_4faces.step"));
}

/// K006, TRT210 #66296: the same on another O-ring.
#[test]
fn another_oring_with_a_copy_over_it() {
    over_copy(&one("oring_k006.step"));
}

/// HRTA6 #56170 (167 faces, 18 tori): with a copy over it, main refuses
/// (InconsistentClassification); once its tori are traced apart (above) the answer fails check
/// (FaceWrongSide on a cone face). Open: the O-ring fix waits on it, a wrong answer being worse
/// than a refusal.
#[test]
#[ignore = "open: a wrong answer once the tori trace apart"]
fn a_part_with_tori_with_a_copy_over_it() {
    over_copy(&one("tori_167faces.step"));
}
