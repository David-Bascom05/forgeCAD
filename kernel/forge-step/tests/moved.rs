use forge_geom::{CurveGeom, SurfaceGeom};
use forge_math::{Frame, Point3, Vec3};
use forge_step::{Limits, import};
use forge_topo::{Body, check};

/// `b` moved rigidly by `d`, every tolerance kept (as a copy placed beside the part).
fn moved(b: &Body, d: Vec3) -> Body {
    let f = Frame::WORLD.with_origin(Point3::ORIGIN + d);
    let mut c = b.clone();
    for v in &mut c.vertices {
        v.point = f.point_to_world(v.point);
    }
    for e in &mut c.edges {
        e.curve = e.curve.transformed(&f);
        e.bbox = Default::default();
    }
    for x in &mut c.faces {
        x.surface = x.surface.transformed(&f);
        x.bbox = Default::default();
        x.query = Default::default();
    }
    c.hulls = Default::default();
    c
}

/// Two cylinders crossing, written out (their meeting a B-spline) and read back: the read measures
/// each edge's distance from its faces where it passes LINEAR and records it as the edge's tolerance.
/// Recorded exactly, the part moved rigidly read a few 1e-15 over it on some edge, and `check` found
/// that edge off its face (EdgeOffFace on the copy and on every boolean with it: KERNEL-STRESS's
/// Haas sweep). `check` now allows the rounding of a distance measured at the edge's coordinates,
/// so the part reads true wherever it is moved.
#[test]
fn a_read_part_moved_rigidly_stays_valid() {
    use forge_math::Point2;
    use forge_ops::{Profile, extrude};
    use forge_step::{Options, export};
    let rod = |f: Frame, r: f64, len: f64, id: u64| {
        extrude(
            &Profile {
                plane: f,
                loops: vec![Profile::circle(Point2::ORIGIN, r, id)],
            },
            len,
            id,
        )
        .unwrap()
    };
    let a = rod(
        Frame::WORLD.with_origin(Point3::new(0.0, 0.0, -20.0)),
        6.0,
        40.0,
        1,
    );
    let side = Frame::from_zx(Point3::new(-20.0, 0.0, 0.0), forge_math::UnitVec3::X, Vec3::Y).unwrap();
    let b = rod(side, 4.5, 40.0, 2);
    let u = forge_bool::boolean(&a, &b, forge_bool::Op::Union, 3).unwrap();
    let text = export(&u, &Options::default()).expect("writes").text;
    let imp = import(text.as_bytes(), &Limits::default()).expect("reads");
    let part = &imp.solids[0].body;
    assert!(check(part).is_empty(), "{:?}", check(part));
    assert!(
        part.edge_ids()
            .any(|e| part.edge(e).tol > forge_math::tol::LINEAR),
        "no edge read off its faces: the test asks nothing"
    );
    for k in 1..200 {
        let k = k as f64;
        let d = Vec3::new(17.3 * k, -9.1 * k, 3.7 * k);
        let m = moved(part, d);
        assert!(check(&m).is_empty(), "moved by {d:?}: {:?}", check(&m));
    }
}
