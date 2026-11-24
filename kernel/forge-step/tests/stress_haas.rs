//! Reproducers shrunk from the Haas machine-model sweep (tools/kernel-stress, research/kernel-stress/
//! haas-report.md). Each stands for a class of failure the sweep found on real files, made from
//! nothing but the kernel's own parts so no model's geometry is in the repository.
use forge_math::{Frame, Point2, Point3, Vec3};
use forge_ops::{Profile, extrude};
use forge_step::{Limits, Options, export, import};
use forge_topo::{Body, check};

/// A round bar: its one LINE is the seam, an edge with the one face on both sides.
fn bar(r: f64, h: f64) -> Body {
    let p = Profile {
        plane: Frame::WORLD,
        loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
    };
    extrude(&p, h, 1).unwrap()
}

/// The file's text with the first LINE's point moved by `d`: an edge that misses its faces, as
/// SolidWorks writes some (HA5C2: 20 edges up to 2.06e-4 mm off).
fn edge_off(text: &str, d: Vec3) -> String {
    let at = text.find("=LINE(").expect("a line");
    let refs: Vec<&str> = text[at..]
        .split(['#', ',', ')'])
        .filter(|s| s.chars().all(|c| c.is_ascii_digit()) && !s.is_empty())
        .collect();
    let pt = format!("#{}=", refs[0]);
    let p0 = text.find(&pt).expect("the line's point");
    let open = p0 + text[p0..].find("',(").unwrap() + 3;
    let close = open + text[open..].find(')').unwrap();
    let xyz: Vec<f64> = text[open..close]
        .split(',')
        .map(|v| v.trim().parse().unwrap())
        .collect();
    let moved = format!("{:e},{:e},{:e}", xyz[0] + d.x, xyz[1] + d.y, xyz[2] + d.z);
    format!("{}{}{}", &text[..open], moved, &text[close..])
}

/// `b` moved rigidly by `d`, everything else as it was.
fn moved(b: &Body, d: Vec3) -> Body {
    use forge_geom::{CurveGeom, SurfaceGeom};
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

/// An edge a file leaves off its faces is read with a tolerance: it must hold with room, so the
/// body still checks once moved. It was set to the gap measured to the last bit, and the copy
/// beside it, or any boolean of it, failed `check` with EdgeOffFace at exactly that gap
/// (union_beside, union_over, int_over on HA5C2 and others).
#[test]
fn a_tolerant_edge_still_checks_once_moved() {
    let text = export(&bar(5.0, 10.0), &Options::default()).unwrap().text;
    assert_eq!(text.matches("=LINE(").count(), 1);
    // the seam, out of the cylinder by 0.2 um: nothing heals a seam, so it is read as tolerant
    let bad = edge_off(&text, Vec3::new(1.4e-4, 1.4e-4, 0.0));
    let imp = import(bad.as_bytes(), &Limits::default()).unwrap();
    let b = &imp.solids[0].body;
    assert!(imp.stats.tolerant_edges > 0, "the edge is read as tolerant");
    assert!(check(b).is_empty(), "{:?}", check(b));
    for d in [
        Vec3::new(10.0, 0.0, 0.0),
        Vec3::new(5.0, 0.0, 0.0),
        Vec3::new(123.456, -78.9, 0.001),
        Vec3::new(-1e3, 1e3, 1e3),
    ] {
        let p = check(&moved(b, d));
        assert!(p.is_empty(), "moved by {d:?}: {p:?}");
    }
}

/// A file's solid of revolution as hand-written STEP: a disk, a cylinder r 5 from z 0 to 10, a
/// cone widening to r 7 at z 15 and a disk on top, each rim a whole circle with one vertex and no
/// seam edges (as SolidWorks writes turned parts). `a`: the angles of the three rims' vertices,
/// bottom to top. A seam starts at the first rim's vertex and runs to the other rim, so the
/// cylinder's seam lies at `a[0]` and the cone's at `a[2]`, both ending on the middle rim.
fn turned(a: [f64; 3]) -> String {
    let p = |r: f64, t: f64| (r * t.cos(), r * t.sin());
    let (p0, p1, p2) = (p(5.0, a[0]), p(5.0, a[1]), p(7.0, a[2]));
    let semi = (2.0_f64 / 5.0).atan();
    format!(
        "ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('turned','',(''),(''),'','','');
FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));
ENDSEC;
DATA;
#1=DIRECTION('',(0.,0.,1.));
#2=DIRECTION('',(1.,0.,0.));
#10=CARTESIAN_POINT('',(0.,0.,0.));
#11=CARTESIAN_POINT('',(0.,0.,10.));
#12=CARTESIAN_POINT('',(0.,0.,15.));
#20=AXIS2_PLACEMENT_3D('',#10,#1,#2);
#21=AXIS2_PLACEMENT_3D('',#11,#1,#2);
#22=AXIS2_PLACEMENT_3D('',#12,#1,#2);
#23=AXIS2_PLACEMENT_3D('',#10,#1,#2);
#24=AXIS2_PLACEMENT_3D('',#11,#1,#2);
#30=CIRCLE('',#20,5.);
#31=CIRCLE('',#21,5.);
#32=CIRCLE('',#22,7.);
#40=CARTESIAN_POINT('',({:?},{:?},0.));
#41=CARTESIAN_POINT('',({:?},{:?},10.));
#42=CARTESIAN_POINT('',({:?},{:?},15.));
#43=VERTEX_POINT('',#40);
#44=VERTEX_POINT('',#41);
#45=VERTEX_POINT('',#42);
#50=EDGE_CURVE('',#43,#43,#30,.T.);
#51=EDGE_CURVE('',#44,#44,#31,.T.);
#52=EDGE_CURVE('',#45,#45,#32,.T.);
#60=PLANE('',#20);
#61=CYLINDRICAL_SURFACE('',#23,5.);
#62=CONICAL_SURFACE('',#24,5.,{semi});
#63=PLANE('',#22);
#70=ORIENTED_EDGE('',*,*,#50,.T.);
#71=ORIENTED_EDGE('',*,*,#51,.F.);
#72=ORIENTED_EDGE('',*,*,#51,.T.);
#73=ORIENTED_EDGE('',*,*,#52,.F.);
#74=ORIENTED_EDGE('',*,*,#50,.F.);
#75=ORIENTED_EDGE('',*,*,#52,.T.);
#80=EDGE_LOOP('',(#70));
#81=EDGE_LOOP('',(#71));
#82=EDGE_LOOP('',(#72));
#83=EDGE_LOOP('',(#73));
#84=EDGE_LOOP('',(#74));
#85=EDGE_LOOP('',(#75));
#90=FACE_OUTER_BOUND('',#80,.T.);
#91=FACE_BOUND('',#81,.T.);
#92=FACE_OUTER_BOUND('',#82,.T.);
#93=FACE_BOUND('',#83,.T.);
#94=FACE_OUTER_BOUND('',#84,.T.);
#95=FACE_OUTER_BOUND('',#85,.T.);
#100=ADVANCED_FACE('',(#90,#91),#61,.T.);
#101=ADVANCED_FACE('',(#93,#92),#62,.T.);
#102=ADVANCED_FACE('',(#94),#60,.F.);
#103=ADVANCED_FACE('',(#95),#63,.T.);
#110=CLOSED_SHELL('',(#100,#101,#102,#103));
#111=MANIFOLD_SOLID_BREP('turned',#110);
#120=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));
#121=(NAMED_UNIT(*)PLANE_ANGLE_UNIT()SI_UNIT($,.RADIAN.));
#122=(NAMED_UNIT(*)SI_UNIT($,.STERADIAN.)SOLID_ANGLE_UNIT());
#123=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.E-06),#120,'','');
#124=(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#123))GLOBAL_UNIT_ASSIGNED_CONTEXT((#120,#121,#122))REPRESENTATION_CONTEXT('',''));
#125=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#111),#124);
ENDSEC;
END-ISO-10303-21;
",
        p0.0, p0.1, p1.0, p1.1, p2.0, p2.1
    )
}

/// A whole circle between two faces of revolution whose seams lie at other angles: the first
/// face's seam ended at the circle's vertex, and the second face's then moved that vertex round
/// to its own seam, leaving the first face's loop broken (LoopBroken and Euler on 24 solids of
/// 08-1949 and others; the smallest a 9-face turned pin). The circle is split there instead.
#[test]
fn a_rim_between_two_seams_at_other_angles_is_split_not_moved() {
    use forge_math::dmath::FRAC_PI_2;
    for a in [
        [0.0, 0.0, FRAC_PI_2],
        [0.0, 1.0, 2.0],
        [1.0, 0.0, -2.5],
        [0.3, 0.3, 0.3],
        [0.0, FRAC_PI_2, 0.0],
    ] {
        let text = turned(a);
        let imp = import(text.as_bytes(), &Limits::default()).unwrap();
        assert!(imp.refusals.is_empty(), "{:?}", imp.refusals);
        let b = &imp.solids[0].body;
        let p = check(b);
        assert!(p.is_empty(), "{a:?}: {p:?}");
        // pi r^2 h, and the frustum's pi h (R^2 + R r + r^2) / 3
        let pi = forge_math::dmath::PI;
        let want = pi * 25.0 * 10.0 + pi * 5.0 * (49.0 + 35.0 + 25.0) / 3.0;
        let m = forge_topo::mass::mass(b).unwrap();
        assert!((m.volume - want).abs() < 1e-6 * want, "{} vs {want}", m.volume);
        assert!(b.unverified.is_none(), "{:?}", b.unverified);
    }
}
