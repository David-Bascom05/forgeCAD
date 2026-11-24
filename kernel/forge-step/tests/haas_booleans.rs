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
