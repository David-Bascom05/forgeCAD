//! Solids taken out of the Haas machine models (tools/kernel-stress/extract.py; the sweep is
//! research/kernel-stress/haas-report.md), one per failure class. The tests that pass guard the
//! fixes. The ignored ones state what a failure still open should do, and the kernel's rewrite
//! must pass them.
use forge_mesh::{Quality, tessellate_tolerant};
use forge_step::{Limits, import};
use forge_topo::{Body, check, mass};

fn read(name: &str) -> forge_step::Imported {
    let p = format!("{}/tests/data/haas/{name}", env!("CARGO_MANIFEST_DIR"));
    import(&std::fs::read(&p).unwrap(), &Limits::default()).unwrap()
}

/// Valid all the way: checks, weighs, meshes closed at the default chord, and its mesh agrees
/// (to 1%: 0.01 mm is coarse on a pin of 3 mm).
fn sound(b: &Body) {
    let p = check(b);
    assert!(p.is_empty(), "{p:?}");
    let m = mass::mass_rel(b, 1e-6).unwrap();
    let t = tessellate_tolerant(b, &Quality::default()).unwrap();
    assert!(
        t.open_edges().is_empty(),
        "{} open mesh edges",
        t.open_edges().len()
    );
    assert!(
        (t.volume() - m.volume).abs() < 1e-2 * m.volume,
        "{} vs {}",
        t.volume(),
        m.volume
    );
    assert!(b.unverified.is_none(), "{:?}", b.unverified);
}

fn one(name: &str) -> Body {
    let imp = read(name);
    assert!(imp.refusals.is_empty(), "{:?}", imp.refusals);
    assert_eq!(imp.solids.len(), 1);
    imp.solids.into_iter().next().unwrap().body
}

/// 08-1949 #440162: a turned pin of 9 faces, every rim a whole circle with no seam. The cone's
/// and cylinder's seams fall at other angles on a shared rim; the loop broke (LoopBroken, Euler).
#[test]
fn turned_pin_reads_valid() {
    sound(&one("pin_09faces_seams.step"));
}

/// 08-1949 #49364: the same on a part of 86 faces.
#[test]
fn turned_part_reads_valid() {
    sound(&one("part_86faces_seams.step"));
}

/// GM2-5AX #772465 Edge-Flange3: union with a copy of itself moved half its width gave an Ok that
/// failed check (EdgeOffFace on a spline edge, 2e-9 over its tolerance).
#[test]
fn flange_union_with_moved_copy_checks() {
    use forge_bool::{Op, boolean};
    use forge_geom::{CurveGeom, SurfaceGeom};
    use forge_math::{Frame, Point3, Vec3};
    let b = one("edge_flange_48faces.step");
    sound(&b);
    let w = b.faces_box().x.width();
    let f = Frame::WORLD.with_origin(Point3::ORIGIN + Vec3::new(0.5 * w, 0.0, 0.0));
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
    let u = boolean(&b, &c, Op::Union, 9000).unwrap();
    let p = check(&u);
    assert!(p.is_empty(), "{p:?}");
}

// ---- open: the rewrite must pass these ----

/// GM2-5AX #998581: a sphere face of zero area (two arcs of one great circle), which checks as
/// FaceWrongSide and leaves the mesh open. To heal: drop the sliver and join its edges.
#[test]
#[ignore = "open: a zero-area sliver face"]
fn sliver_sphere_face_is_healed() {
    sound(&one("sliver_sphere_309faces.step"));
}

/// 08-1949 #96088: a three-sided NURBS corner patch, its fourth side collapsed to a pole; the
/// loop crosses itself in the surface's parameters (LoopsCross).
#[test]
#[ignore = "open: a corner patch's collapsed side"]
fn corner_patch_with_a_pole_reads_valid() {
    sound(&one("corner_patch_82faces.step"));
}

/// HA2TS #92348: a sphere face bounded by four arcs whose loop drew no region in parameters
/// (mass failed): a loop through a pole whose whole turn went uncounted (mass-cap).
#[test]
fn sphere_face_of_four_arcs_weighs() {
    sound(&one("sphere_no_region_156faces.step"));
}

/// 08-1949 #521387: valid, but its mesh at the default chord (0.01) has a crack on a NURBS face
/// near a near-singular point; at 0.05 and 0.002 it is closed. forge-mesh.
#[test]
#[ignore = "open: a mesh crack at one chord"]
fn nurbs_face_meshes_closed_at_the_default_chord() {
    sound(&one("nurbs_mesh_crack_61faces.step"));
}

/// 08-1949 #525424: an O-ring as one closed rational B-spline face with no edges, refused ("a whole
/// spline surface with no edges"); only spheres are read whole.
#[test]
#[ignore = "open: a whole closed spline face"]
fn oring_as_one_closed_nurbs_face_reads() {
    sound(&one("oring_closed_nurbs.step"));
}

/// ST20LY-25LY #140177 (40 faces): a bore along z through the middle, subtracted. The Ok's new
/// cylinder-cylinder intersection edge (tolerance 1e-5) has a vertex 2.1e-5 off it
/// (VertexOffEdge); intersect failed alike (6 such Oks in the first 43 zips). Fixed by main's
/// train 6 (drift tolerances).
#[test]
fn bore_through_a_part_checks() {
    use forge_bool::{Op, boolean};
    use forge_math::{Frame, Point2, UnitVec3, Vec3};
    let b = one("bore_vertex_drift_40faces.step");
    sound(&b);
    let bx = b.faces_box();
    let small = bx.x.width().min(bx.y.width()).min(bx.z.width());
    let len = 3.0 * bx.diagonal();
    let p = forge_ops::Profile {
        plane: Frame::from_z(bx.center() - Vec3::from(UnitVec3::Z) * (0.5 * len), UnitVec3::Z),
        loops: vec![forge_ops::Profile::circle(Point2::ORIGIN, 0.13 * small, 7001)],
    };
    let tool = forge_ops::extrude(&p, len, 7000).unwrap();
    for op in [Op::Subtract, Op::Intersect] {
        let r = boolean(&b, &tool, op, 9000).unwrap();
        let p = check(&r);
        assert!(p.is_empty(), "{op:?}: {p:?}");
    }
}
