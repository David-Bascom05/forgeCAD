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

/// A ring, as written here: two faces on one torus, joined by two meridian circles.
fn ring() -> String {
    use forge_math::{Vec2, dmath};
    use forge_ops::{Axis2, revolve};
    let p = Profile {
        plane: Frame::WORLD,
        loops: vec![Profile::circle(Point2::new(10.0, 0.0), 2.0, 0)],
    };
    let axis = Axis2 {
        origin: Point2::ORIGIN,
        dir: Vec2::Y,
    };
    export(&revolve(&p, axis, dmath::TAU, 1).unwrap(), &Options::default())
        .unwrap()
        .text
}

/// The most an edge strays from its faces, on many more points than any reader takes.
fn dense_gap(b: &Body, e: forge_topo::EdgeId) -> f64 {
    use forge_geom::SurfaceGeom;
    let pts = b.edge_points(e, 20_000);
    b.edge(e)
        .coedges
        .iter()
        .map(|&c| {
            let s = &b.face(b.coedge_face(c)).surface;
            pts.iter()
                .map(|&p| s.eval(s.project(p)).distance(p))
                .fold(0.0, f64::max)
        })
        .fold(0.0, f64::max)
}

/// A tolerant edge's tolerance covers its whole gap, not just the gap at the points the reader
/// measured: `check` measures a trimmed edge at other points, and a gap that peaks between the
/// reader's samples came out over the tolerance (union_over on GM2-5AX's Edge-Flange3: a spline
/// edge 1.2577e-4 mm off, tolerance 1.2574e-4). Here a ring's meridian, its centre moved 0.1 um
/// towards angles between the reader's samples, so the gap peaks there.
#[test]
fn a_tolerance_covers_the_gap_between_the_samples() {
    let text = ring();
    let at = text.find("CIRCLE('',").expect("a meridian");
    let place = &text[at + 10..];
    let id = &place[..place.find(',').unwrap()];
    let pl = text.find(&format!("{id}=AXIS2_PLACEMENT_3D('',")).unwrap();
    let pt = &text[pl + id.len() + 23..];
    let pt = &pt[..pt.find(',').unwrap()];
    let mut worst = 0.0_f64;
    for k in 0..16 {
        let phi = forge_math::dmath::TAU / 255.0 * f64::from(k) / 16.0;
        let bad = edge_off_at(&text, pt, Vec3::new(1e-4 * phi.cos(), 1e-4 * phi.sin(), 0.0));
        let imp = import(bad.as_bytes(), &Limits::default()).unwrap();
        let b = &imp.solids[0].body;
        for e in b.edge_ids() {
            let x = b.edge(e);
            if x.tol > forge_math::tol::LINEAR {
                let gap = dense_gap(b, e);
                worst = worst.max(gap / x.tol);
                assert!(gap <= x.tol, "phi {phi}: gap {gap:e} over tolerance {:e}", x.tol);
            }
        }
    }
    assert!(worst > 0.5, "the meridian was read as tolerant: {worst}");
}

/// The file's text with point `pt` (its `#` id) moved by `d`.
fn edge_off_at(text: &str, pt: &str, d: Vec3) -> String {
    let p0 = text.find(&format!("{pt}=CARTESIAN_POINT(")).expect("the point");
    let open = p0 + text[p0..].find("',(").unwrap() + 3;
    let close = open + text[open..].find(')').unwrap();
    let xyz: Vec<f64> = text[open..close]
        .split(',')
        .map(|v| v.trim().parse().unwrap())
        .collect();
    let moved = format!("{:e},{:e},{:e}", xyz[0] + d.x, xyz[1] + d.y, xyz[2] + d.z);
    format!("{}{}{}", &text[..open], moved, &text[close..])
}

/// The point a surface's placement stands on, as its `#` id: the first `kind` in the file.
fn anchor_of<'a>(text: &'a str, kind: &str) -> &'a str {
    let at = text.find(&format!("{kind}('',")).expect("the surface");
    let rest = &text[at + kind.len() + 4..];
    let place = &rest[..rest.find([',', ')']).unwrap()];
    let pl = text.find(&format!("{place}=AXIS2_PLACEMENT_3D('',")).unwrap();
    let pt = &text[pl + place.len() + 23..];
    &pt[..pt.find(',').unwrap()]
}

/// A placement's origin far out where it changes nothing of the shape: a cylinder's 600 m along
/// its axis, a plane's 600 m across it (TR210 and 15 more: refused as "outside the 1 km model
/// box"). It is moved to the place nearest the world's origin and the solid is read.
#[test]
fn a_far_anchor_on_an_axis_or_a_plane_is_read() {
    let text = export(&bar(5.0, 10.0), &Options::default()).unwrap().text;
    let cyl = anchor_of(&text, "CYLINDRICAL_SURFACE").to_string();
    let far = edge_off_at(&text, &cyl, Vec3::new(0.0, 0.0, 6e5));
    let pl = anchor_of(&far, "PLANE").to_string();
    let far = edge_off_at(&far, &pl, Vec3::new(6e5, -6e5, 0.0));
    let imp = import(far.as_bytes(), &Limits::default()).unwrap();
    assert!(imp.refusals.is_empty(), "{:?}", imp.refusals);
    let b = &imp.solids[0].body;
    assert!(check(b).is_empty(), "{:?}", check(b));
    let want = forge_math::dmath::PI * 25.0 * 10.0;
    let v = forge_topo::mass::mass(b).unwrap().volume;
    assert!((v - want).abs() < 1e-6 * want, "{v} vs {want}");
    // a point that is far out and anchors nothing is still refused
    let pt = text.find("=VERTEX_POINT('',").map(|i| &text[i + 17..]).unwrap();
    let pt = &pt[..pt.find(')').unwrap()];
    let bad = edge_off_at(&text, pt, Vec3::new(6e5, 0.0, 0.0));
    assert!(import(bad.as_bytes(), &Limits::default()).map_or(true, |i| !i.refusals.is_empty()));
}

/// A body written as a surface model, its shell an OPEN_SHELL that closes (TRT160: the whole
/// machine so, "the file holds no solid"): read as a solid. A shell that does not close is a
/// sheet, refused as one.
#[test]
fn a_surface_model_shell_that_closes_is_a_solid() {
    let text = export(&bar(5.0, 10.0), &Options::default()).unwrap().text;
    let sm = text
        .replace("CLOSED_SHELL(", "OPEN_SHELL(")
        .replace("MANIFOLD_SOLID_BREP(", "SHELL_BASED_SURFACE_MODEL(")
        .replace(
            "ADVANCED_BREP_SHAPE_REPRESENTATION(",
            "MANIFOLD_SURFACE_SHAPE_REPRESENTATION(",
        );
    // the model's shell given as a list, as the schema has it
    let at = sm.find("SHELL_BASED_SURFACE_MODEL('',").unwrap() + 29;
    let end = at + sm[at..].find(')').unwrap();
    let sm = format!("{}({}){}", &sm[..at], &sm[at..end], &sm[end..]);
    let imp = import(sm.as_bytes(), &Limits::default()).unwrap();
    assert!(imp.refusals.is_empty(), "{:?}", imp.refusals);
    let b = &imp.solids[0].body;
    assert!(check(b).is_empty(), "{:?}", check(b));
    let want = forge_math::dmath::PI * 25.0 * 10.0;
    let v = forge_topo::mass::mass(b).unwrap().volume;
    assert!((v - want).abs() < 1e-6 * want, "{v} vs {want}");
    // one face less: a sheet
    let sh = sm.find("OPEN_SHELL('',(").unwrap() + 15;
    let comma = sh + sm[sh..].find(',').unwrap();
    let sheet = format!("{}{}", &sm[..sh], &sm[comma + 1..]);
    let r = import(sheet.as_bytes(), &Limits::default());
    assert!(r.map_or(true, |i| i.solids.is_empty()
        && i.refusals.iter().any(|x| x.kind == "OPEN_SHELL")));
}

/// A reference past 32 bits (SolidWorks writes #18446744073709551615, -1, for one it lost: VF-2,
/// VF-10, ST-30, VF-6) refused the whole file as a parse error; now only what needs it is refused.
#[test]
fn a_lost_reference_refuses_its_solid_not_the_file() {
    let text = export(&bar(5.0, 10.0), &Options::default()).unwrap().text;
    let at = text.find("=EDGE_CURVE('',").unwrap() + 15;
    let end = at + text[at..].find(',').unwrap();
    let bad = format!("{}#18446744073709551615{}", &text[..at], &text[end..]);
    match import(bad.as_bytes(), &Limits::default()) {
        Ok(i) => assert!(i.solids.is_empty() && !i.refusals.is_empty()),
        Err(e) => assert!(!matches!(e, forge_step::ImportError::Parse(_)), "{e}"),
    }
    // a reference with no digits at all is still an error in the text
    let worse = format!("{}#{}", &text[..at], &text[end..]);
    assert!(matches!(
        import(worse.as_bytes(), &Limits::default()),
        Err(forge_step::ImportError::Parse(_))
    ));
}
