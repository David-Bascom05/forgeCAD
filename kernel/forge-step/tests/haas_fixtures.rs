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

/// HA2TS #92348: a sphere face bounded by four arcs whose loop draws no region in parameters
/// (mass fails).
#[test]
#[ignore = "open: a sphere face that draws no region"]
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

/// TRT210 #204568: a file's spline edge (a cylinder-cylinder intersection) whose gap from its faces
/// peaks between the reader's samples; a boolean that split it left a piece 1.3e-3 over its
/// tolerance (corpus K029, FLAT-SITES). Every tolerant edge's tolerance covers its gap measured
/// at 4096 points.
#[test]
fn a_tolerant_edges_gap_is_covered_everywhere() {
    use forge_geom::SurfaceGeom;
    let b = one("trt210_split_spline_edge.step");
    let mut n = 0;
    for e in b.edge_ids() {
        let x = b.edge(e);
        if x.tol <= forge_math::tol::LINEAR {
            continue;
        }
        n += 1;
        let pts = b.edge_points(e, 4096);
        for &c in &x.coedges {
            let s = &b.face(b.coedge_face(c)).surface;
            let mut near = None;
            for &p in &pts {
                let uv = near.map_or_else(|| s.project(p), |uv| s.project_near(p, uv));
                near = Some(uv);
                let gap = s.eval(uv).distance(p);
                assert!(gap <= x.tol, "edge {e:?}: gap {gap:e} over tolerance {:e}", x.tol);
            }
        }
    }
    assert!(n > 0, "the part has tolerant edges");
}

/// A boolean and a check run under one operation's work budget: past it they refuse, `TooLong`,
/// and return nothing of the work (an outer budget nests them, so a tiny one shows it at once on
/// UMC-400's NURBS faces, whose bore otherwise runs to the operation's whole budget).
#[test]
fn a_boolean_and_a_check_past_their_budget_refuse() {
    use forge_bool::{BoolError, Op, boolean};
    use forge_math::{Frame, Point2, UnitVec3, Vec3};
    let b = one("umc400_bore_runaway_73faces.step");
    let bx = b.faces_box();
    let small = bx.x.width().min(bx.y.width()).min(bx.z.width());
    let len = 3.0 * bx.diagonal();
    let p = forge_ops::Profile {
        plane: Frame::from_z(bx.center() - Vec3::from(UnitVec3::Z) * (0.5 * len), UnitVec3::Z),
        loops: vec![forge_ops::Profile::circle(Point2::ORIGIN, 0.13 * small, 7001)],
    };
    let tool = forge_ops::extrude(&p, len, 7000).unwrap();
    {
        let _tiny = forge_geom::budget::scoped(1);
        assert!(matches!(
            boolean(&b, &tool, Op::Subtract, 9000),
            Err(BoolError::TooLong)
        ));
    }
    {
        let _tiny = forge_geom::budget::scoped(1);
        assert_eq!(check(&b), vec![forge_topo::Problem::TooLong]);
    }
    // with the budget a check has, it finishes as before
    assert!(check(&b).is_empty());
}

/// UMC-400 #63060 (73 faces): a bore through it, which ran without end in the curve-surface search
/// on its NURBS faces (K051/K052 of the corpus), now stops at its work budget and refuses. Under
/// an outer budget of 2^22, which the boolean's own (`budget::OP`, 2^29: about 90 s at opt-level
/// 3) is held within: the search stops at whatever it is given.
#[test]
#[cfg_attr(target_os = "wasi", ignore = "90 s natively")]
fn a_bore_through_umc400s_part_finishes() {
    let _within = forge_geom::budget::scoped(1 << 22);
    use forge_bool::{BoolError, Op, boolean};
    use forge_math::{Frame, Point2, UnitVec3, Vec3};
    let b = one("umc400_bore_runaway_73faces.step");
    let bx = b.faces_box();
    let small = bx.x.width().min(bx.y.width()).min(bx.z.width());
    let len = 3.0 * bx.diagonal();
    let p = forge_ops::Profile {
        plane: Frame::from_z(bx.center() - Vec3::from(UnitVec3::Z) * (0.5 * len), UnitVec3::Z),
        loops: vec![forge_ops::Profile::circle(Point2::ORIGIN, 0.13 * small, 7001)],
    };
    let tool = forge_ops::extrude(&p, len, 7000).unwrap();
    match boolean(&b, &tool, Op::Subtract, 9000) {
        Err(BoolError::TooLong) => {}
        Ok(r) => assert!(check(&r).is_empty()),
        Err(e) => panic!("{e:?}"),
    }
}

/// `b` moved by `d`, as forge-stress moves its copies (names salted, so the two share none).
/// FALLBACK's, from fix/torus-tangent-branches' tests/haas_booleans.rs.
fn moved(b: &Body, d: forge_math::Vec3) -> Body {
    use forge_geom::{CurveGeom, SurfaceGeom};
    use forge_math::{Frame, Point3};
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

/// A guard, not a fix (FALLBACK's scan, the oracle corpus's): HRTA6 #56170 (167 faces, 18 tori:
/// O-ring grooves, drill-point cones) with a copy moved k of its width along x, k from 0.40 to
/// 0.43 (forge-stress's over2 is 0.4137). Main refuses all nine (InconsistentClassification),
/// which passes; where union and intersection both answer, they must make up twice the solid.
/// With the tori traced apart (fix/torus-tangent-branches, held) both check clean yet miss by
/// -972 mm3 at 0.4137 and +126 at 0.417: wrong answers only the volume sees. So a change that
/// makes main answer here must answer right.
#[test]
fn a_part_with_tori_with_copies_near_over_answers_right_or_refuses() {
    use forge_bool::{Op, boolean};
    let b = one("tori_167faces.step");
    let w = b.faces_box().x.width();
    let v = mass::mass_rel(&b, 1e-6).unwrap().volume;
    // each offset on a thread of its own: eighteen refusals of about 0.45 s each, one after
    // another, were 8 s
    let check = |k: f64| {
        let copy = moved(&b, forge_math::Vec3::new(k * w, 0.0, 0.0));
        let (Ok(u), Ok(i)) = (
            boolean(&b, &copy, Op::Union, 9000),
            boolean(&b, &copy, Op::Intersect, 9000),
        ) else {
            return;
        };
        let sum = mass::mass_rel(&u, 1e-6).unwrap().volume + mass::mass_rel(&i, 1e-6).unwrap().volume;
        let miss = sum - 2.0 * v;
        assert!(
            miss.abs() < 1e-6 * v,
            "offset {k}: union + intersection - 2V = {miss}"
        );
    };
    let offsets = [0.40, 0.405, 0.41, 0.412, 0.4137, 0.415, 0.417, 0.42, 0.43];
    // (wasm has no threads: one after another there)
    if cfg!(target_family = "wasm") {
        offsets.into_iter().for_each(check);
        return;
    }
    std::thread::scope(|s| {
        let all: Vec<_> = offsets.into_iter().map(|k| s.spawn(move || check(k))).collect();
        for t in all {
            t.join().unwrap();
        }
    });
}
