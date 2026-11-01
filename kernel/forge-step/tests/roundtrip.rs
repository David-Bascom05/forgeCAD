//! Bodies out to a file and back; files damaged every way, for no panic.
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, Vec2, dmath};
use forge_ops::{Axis2, Profile, ProfileLoop, Segment, extrude, revolve};
use forge_step::{Limits, Options, export, import};
use forge_topo::{Body, check, mass::mass};

const Y: Axis2 = Axis2 {
    origin: Point2::ORIGIN,
    dir: Vec2::Y,
};

fn line(a: (f64, f64), b: (f64, f64), id: u64) -> (Segment, u64) {
    (
        Segment::Line {
            a: Point2::new(a.0, a.1),
            b: Point2::new(b.0, b.1),
        },
        id,
    )
}

fn parts() -> Vec<(&'static str, Body)> {
    let flat = |loops| Profile {
        plane: Frame::WORLD,
        loops,
    };
    let disc = |x: f64, r: f64| Profile::circle(Point2::new(x, 0.0), r, 0);
    let cone = ProfileLoop::new(vec![
        line((0.0, 0.0), (4.0, 0.0), 0),
        line((4.0, 0.0), (0.0, 5.0), 1),
        line((0.0, 5.0), (0.0, 0.0), 2),
    ]);
    let ball = ProfileLoop::new(vec![
        line((0.0, 5.0), (0.0, -5.0), 0),
        (
            Segment::Arc {
                center: Point2::ORIGIN,
                radius: 5.0,
                start: -dmath::FRAC_PI_2,
                sweep: dmath::PI,
            },
            1,
        ),
    ]);
    let holed = vec![
        Profile::rectangle(Point2::new(-5.0, -3.0), 10.0, 6.0, 0),
        Profile::circle(Point2::new(1.0, 0.0), 1.5, 9),
    ];
    vec![
        (
            "block",
            extrude(
                &flat(vec![Profile::rectangle(Point2::new(0.0, 0.0), 10.0, 20.0, 0)]),
                5.0,
                1,
            )
            .unwrap(),
        ),
        ("post", extrude(&flat(vec![disc(0.0, 5.0)]), 3.0, 1).unwrap()),
        ("plate with a hole", extrude(&flat(holed), 2.0, 1).unwrap()),
        ("cone", revolve(&flat(vec![cone]), Y, dmath::TAU, 1).unwrap()),
        ("ball", revolve(&flat(vec![ball]), Y, dmath::TAU, 1).unwrap()),
        (
            "torus",
            revolve(&flat(vec![disc(10.0, 2.0)]), Y, dmath::TAU, 1).unwrap(),
        ),
    ]
}

#[test]
fn analytic_bodies_come_back_the_same() {
    for (name, b) in parts() {
        let e = export(&b, &Options::default()).unwrap();
        assert_eq!(e.fit_faces.count, 0, "{name}");
        let i = import(e.text.as_bytes(), &Limits::default()).unwrap();
        assert!(i.refusals.is_empty(), "{name}: {:?}", i.refusals);
        assert_eq!(i.solids.len(), 1, "{name}");
        let r = &i.solids[0].body;
        assert_eq!(check(r), vec![], "{name}");
        assert_eq!(
            (r.faces.len(), r.edges.len(), r.vertices.len(), r.loops.len()),
            (b.faces.len(), b.edges.len(), b.vertices.len(), b.loops.len()),
            "{name}"
        );
        // reals read back to the same bits, so every point written is where it was
        let bits = |b: &Body| {
            let mut v: Vec<[u64; 3]> = b
                .vertices
                .iter()
                .map(|x| [x.point.x, x.point.y, x.point.z].map(f64::to_bits))
                .collect();
            v.sort();
            v
        };
        assert_eq!(bits(&b), bits(r), "{name}");
        let (m, n) = (mass(&b).unwrap(), mass(r).unwrap());
        assert!(
            (m.volume - n.volume).abs() <= 1e-9 * m.volume,
            "{name}: {} {}",
            m.volume,
            n.volume
        );
        assert!(
            (m.area - n.area).abs() <= 1e-9 * m.area,
            "{name}: {} {}",
            m.area,
            n.area
        );
        // nothing an analytic body holds needs a tolerance
        assert!(r.edges.iter().all(|e| e.tol <= forge_math::tol::LINEAR), "{name}");
    }
}

/// The numbers count in the file's own unit: a cube of side 1 in centimetres.
#[test]
fn units_come_from_the_context() {
    let b = extrude(
        &Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::rectangle(Point2::new(0.0, 0.0), 1.0, 1.0, 0)],
        },
        1.0,
        1,
    )
    .unwrap();
    let t = export(&b, &Options::default()).unwrap().text;
    let cm = t.replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT(.CENTI.,.METRE.)");
    let i = import(cm.as_bytes(), &Limits::default()).unwrap();
    let v = mass(&i.solids[0].body).unwrap().volume;
    assert!((v - 1000.0).abs() < 1e-6, "{v}");
}

/// What a file can do wrong, on a real one: a read ends in a value or an error, never a panic.
#[test]
fn damaged_files_end_in_an_error_or_a_solid() {
    let mut r = Rng::new(7);
    let tokens: [&[u8]; 12] = [
        b"#",
        b"(",
        b")",
        b",",
        b";",
        b"'",
        b"$",
        b"1E308",
        b"-1.",
        b"0.",
        b"#1",
        b"#99999999",
    ];
    for (_, b) in parts() {
        let t = export(&b, &Options::default()).unwrap().text.into_bytes();
        for _ in 0..60 {
            let mut m = t.clone();
            for _ in 0..1 + r.below(8) {
                let at = r.below(m.len().max(1));
                match r.below(4) {
                    0 => m[at] = r.below(256) as u8,
                    1 => {
                        m.splice(at..at, tokens[r.below(tokens.len())].iter().copied());
                    }
                    2 => {
                        let end = (at + 40).min(m.len());
                        m.drain(at..end);
                    }
                    _ => m.truncate(at.max(1)),
                }
            }
            if let Ok(i) = import(&m, &Limits::default()) {
                for s in &i.solids {
                    let _ = check(&s.body);
                }
            }
        }
    }
}

/// References in a circle.
#[test]
fn circles_of_references_are_refused() {
    let t = b"ISO-10303-21;DATA;\
        #1=MANIFOLD_SOLID_BREP('',#2);#2=CLOSED_SHELL('',(#3));\
        #3=ADVANCED_FACE('',(#4),#9,.T.);#4=FACE_BOUND('',#5,.T.);#5=EDGE_LOOP('',(#6));\
        #6=ORIENTED_EDGE('',*,*,#7,.T.);#7=EDGE_CURVE('',#10,#10,#8,.T.);\
        #8=SURFACE_CURVE('',#8,(),.PCURVE_S1.);#9=RECTANGULAR_TRIMMED_SURFACE('',#9,0.,1.,0.,1.,.T.,.T.);\
        #10=VERTEX_POINT('',#11);#11=CARTESIAN_POINT('',(0.,0.,0.));\
        #12=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#1),#13);\
        #13=(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNIT_ASSIGNED_CONTEXT((#14))REPRESENTATION_CONTEXT('',''));\
        #14=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));ENDSEC;";
    let i = import(t, &Limits::default()).unwrap();
    assert!(i.solids.is_empty());
    assert_eq!(i.refusals.len(), 1);
    assert!(
        i.refusals[0].why.contains("deep") || i.refusals[0].why.contains("circle"),
        "{:?}",
        i.refusals
    );
}
