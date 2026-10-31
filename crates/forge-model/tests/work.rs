//! Work features worked out from references, following upstream edits; patterns along a path
//! and at points through both regens; holes sized from the standards.

use forge_geom::{Curve, CurveGeom, Surface};
use forge_math::{Frame, Point2, Point3, Vec3, dmath};
use forge_mesh::{Quality, tessellate};
use forge_model::{
    At, AxisBy, Combine, Convention, Datum, End, Extent, Feature, Fit, Head, Hole, Layout, Part, PlaneBy,
    PointBy, Ref, Source, Standard, Target, UcsBy, Work, standard, work,
};
use forge_ops::Profile;
use forge_topo::{Body, Name, check};

const PI: f64 = dmath::PI;

fn volume(b: &Body) -> f64 {
    let q = Quality {
        chord: 1e-4,
        ..Quality::default()
    };
    let m = tessellate(b, &q).expect("tessellates");
    assert!(m.open_edges().is_empty(), "mesh has open edges");
    m.volume()
}

fn extrude(plane: Frame, l: forge_ops::ProfileLoop, h: f64, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile {
            plane,
            loops: vec![l],
        },
        extent: Extent::Distance(h),
        op,
    }
}

/// A 40 x 20 x `t` block and a radius 4 boss 15 tall on it at (30, 10).
fn base(t: f64) -> Vec<Feature> {
    vec![
        extrude(
            Frame::WORLD,
            Profile::rectangle(Point2::ORIGIN, 40.0, 20.0, 0),
            t,
            Combine::New,
        ),
        extrude(
            Frame::WORLD.with_origin(Point3::new(30.0, 10.0, t)),
            Profile::circle(Point2::ORIGIN, 4.0, 0),
            15.0,
            Combine::Add,
        ),
    ]
}

fn r(feature: usize, name: Name) -> Ref {
    Ref {
        feature,
        name: Some(name),
    }
}

fn whole(feature: usize) -> Ref {
    Ref { feature, name: None }
}

#[allow(non_snake_case)]
fn TOP() -> Name {
    Name::feature(1).with_str("end")
}

#[allow(non_snake_case)]
fn CYL() -> Name {
    Name::feature(2).with_str("side").with(0)
}

/// The block's side face whose outward normal is `n`.
fn side(features: &[Feature], n: Vec3) -> Name {
    let b = Part {
        features: features.to_vec(),
    }
    .regenerate()
    .expect("regen");
    b.face_ids()
        .map(|f| b.face(f))
        .find(|f| matches!(&f.surface, Surface::Plane(p) if along(p.frame.z().get() * if f.same_sense { 1.0 } else { -1.0 }, n)))
        .expect("a face")
        .name
}

/// The name of the first edge of the part `features` make whose geometry passes.
fn edge(features: &[Feature], ok: impl Fn(&Curve, Point3, Point3) -> bool) -> Name {
    let b = Part {
        features: features.to_vec(),
    }
    .regenerate()
    .expect("regen");
    b.edge_ids()
        .map(|e| b.edge(e))
        .find(|e| ok(&e.curve, e.curve.eval(e.range.lo), e.curve.eval(e.range.hi)))
        .expect("an edge")
        .name
}

fn vertex(features: &[Feature], at: Point3) -> Name {
    let b = Part {
        features: features.to_vec(),
    }
    .regenerate()
    .expect("regen");
    b.vertex_ids()
        .map(|v| b.vertex(v))
        .find(|v| v.point.distance(at) < 1e-9)
        .expect("a vertex")
        .name
}

fn near(a: Point3, b: Point3) -> bool {
    a.distance(b) < 1e-9
}

fn along(a: Vec3, b: Vec3) -> bool {
    (a - b).length() < 1e-12
}

fn datum(features: &[Feature]) -> Datum {
    work::datum(forge_model::History::new(features, None), features.len() - 1).expect("work feature")
}

fn plane(features: &[Feature]) -> Frame {
    match datum(features) {
        Datum::Plane(f) => f,
        d => panic!("not a plane: {d:?}"),
    }
}

fn with(mut fs: Vec<Feature>, w: Work) -> Vec<Feature> {
    fs.push(Feature::Work(w));
    fs
}

fn plane_z(f: &Frame) -> f64 {
    f.origin.z
}

#[test]
fn planes() {
    let fs = base(10.0);
    let (top, bottom, cyl) = (TOP(), Name::feature(1).with_str("start"), CYL());
    let front = side(&fs, -Vec3::Y);
    let x_edge = edge(&fs, |c, a, b| {
        matches!(c, Curve::Line(_)) && a.y == 0.0 && b.y == 0.0 && a.z == 10.0 && b.z == 10.0
    });

    let p = plane(&with(fs.clone(), Work::Plane(PlaneBy::Offset(r(0, top), 5.0))));
    assert!(plane_z(&p) == 15.0 && along(p.z().get(), Vec3::Z), "{p:?}");
    // a face's normal points out of the material
    let p = plane(&with(fs.clone(), Work::Plane(PlaneBy::Offset(r(0, bottom), 2.0))));
    assert!(plane_z(&p) == -2.0 && along(p.z().get(), -Vec3::Z), "{p:?}");
    let p = plane(&with(
        fs.clone(),
        Work::Plane(PlaneBy::Mid(r(0, top), r(0, bottom))),
    ));
    assert!((plane_z(&p) - 5.0).abs() < 1e-12, "{p:?}");

    // turned 90 degrees about the front top edge: the front face's plane
    let p = plane(&with(
        fs.clone(),
        Work::Plane(PlaneBy::Angle {
            plane: r(0, top),
            axis: r(0, x_edge),
            angle: PI / 2.0,
        }),
    ));
    assert!(
        p.z().get().y.abs() > 1.0 - 1e-12 && p.origin.y.abs() < 1e-12,
        "{p:?}"
    );

    // tangent to the boss, square to y: at y = 10 - 4 (the front face's normal is -y)
    let p = plane(&with(
        fs.clone(),
        Work::Plane(PlaneBy::Tangent {
            face: r(1, cyl),
            plane: r(0, front),
        }),
    ));
    assert!((p.z().dot(p.origin.to_vec()) - 4.0 + 10.0).abs() < 1e-12, "{p:?}");

    let c = [(0.0, 0.0), (40.0, 0.0), (0.0, 20.0)].map(|(x, y)| r(0, vertex(&fs, Point3::new(x, y, 10.0))));
    let p = plane(&with(fs.clone(), Work::Plane(PlaneBy::Points(c))));
    assert!(
        near(p.origin, Point3::new(0.0, 0.0, 10.0)) && along(p.z().get(), Vec3::Z),
        "{p:?}"
    );

    let p = plane(&with(
        fs.clone(),
        Work::Plane(PlaneBy::Normal {
            curve: r(0, x_edge),
            at: c[1],
        }),
    ));
    assert!(near(p.origin, Point3::new(40.0, 0.0, 10.0)) && p.z().get().x.abs() > 1.0 - 1e-12);

    // a plane parallel to the top that cannot meet it
    let mut bad = with(fs.clone(), Work::Plane(PlaneBy::Offset(r(0, top), 1.0)));
    bad.push(Feature::Work(Work::Axis(AxisBy::Meet(whole(2), r(0, top)))));
    assert!(work::datum(forge_model::History::new(&bad, None), 3).is_err());
}

#[test]
fn axes_points_and_frames() {
    let fs = base(10.0);
    let (top, cyl) = (TOP(), CYL());
    let rim = edge(&fs, |c, a, _| matches!(c, Curve::Circle(_)) && a.z == 25.0);
    let x_edge = edge(&fs, |c, a, b| {
        matches!(c, Curve::Line(_)) && a.y == 0.0 && b.y == 0.0 && a.z == 10.0 && b.z == 10.0
    });
    let corner = r(0, vertex(&fs, Point3::new(40.0, 20.0, 10.0)));

    let Datum::Axis(a) = datum(&with(fs.clone(), Work::Axis(AxisBy::Along(r(1, cyl))))) else {
        panic!("an axis")
    };
    assert!(near(a.origin, Point3::new(30.0, 10.0, 10.0)) && along(a.dir.get(), Vec3::Z));
    let Datum::Axis(a) = datum(&with(fs.clone(), Work::Axis(AxisBy::Along(r(1, rim))))) else {
        panic!("an axis")
    };
    assert!(near(a.origin, Point3::new(30.0, 10.0, 25.0)));
    let Datum::Axis(a) = datum(&with(fs.clone(), Work::Axis(AxisBy::Along(r(0, x_edge))))) else {
        panic!("an axis")
    };
    assert!(a.dir.get().x.abs() > 1.0 - 1e-12);

    // the two side faces at the far corner meet in its upright edge
    let (right, back) = (side(&fs, Vec3::X), side(&fs, Vec3::Y));
    let Datum::Axis(a) = datum(&with(
        fs.clone(),
        Work::Axis(AxisBy::Meet(r(0, right), r(0, back))),
    )) else {
        panic!("an axis")
    };
    assert!(a.dir.get().z.abs() > 1.0 - 1e-12);
    assert!(
        (a.origin.x - 40.0).abs() < 1e-12 && (a.origin.y - 20.0).abs() < 1e-12,
        "{a:?}"
    );

    let Datum::Axis(a) = datum(&with(
        fs.clone(),
        Work::Axis(AxisBy::Normal {
            plane: r(0, top),
            at: corner,
        }),
    )) else {
        panic!("an axis")
    };
    assert!(near(a.origin, Point3::new(40.0, 20.0, 10.0)) && along(a.dir.get(), Vec3::Z));

    let pt = |w: PointBy| match datum(&with(fs.clone(), Work::Point(w))) {
        Datum::Point(p) => p,
        d => panic!("not a point: {d:?}"),
    };
    assert!(near(pt(PointBy::At(r(1, rim))), Point3::new(30.0, 10.0, 25.0)));
    assert!(near(pt(PointBy::Mid(r(0, x_edge))), Point3::new(20.0, 0.0, 10.0)));
    assert!(near(pt(PointBy::At(corner)), Point3::new(40.0, 20.0, 10.0)));
    // the boss's axis meets the block's top; an edge meets a plane
    let mut fa = with(fs.clone(), Work::Axis(AxisBy::Along(r(1, cyl))));
    fa.push(Feature::Work(Work::Point(PointBy::Meet(whole(2), r(0, top)))));
    assert!(matches!(datum(&fa), Datum::Point(p) if near(p, Point3::new(30.0, 10.0, 10.0))));
    let mut fe = with(
        fs.clone(),
        Work::Plane(PlaneBy::Offset(r(0, side(&fs, -Vec3::X)), -12.5)),
    );
    fe.push(Feature::Work(Work::Point(PointBy::Meet(r(0, x_edge), whole(2)))));
    assert!(matches!(datum(&fe), Datum::Point(p) if near(p, Point3::new(12.5, 0.0, 10.0))));

    let o = r(0, vertex(&fs, Point3::new(0.0, 0.0, 10.0)));
    let x = r(0, vertex(&fs, Point3::new(0.0, 20.0, 10.0)));
    let y = r(0, vertex(&fs, Point3::new(40.0, 20.0, 10.0)));
    let Datum::Frame(u) = datum(&with(fs.clone(), Work::Ucs(UcsBy::Points([o, x, y])))) else {
        panic!("a frame")
    };
    assert!(
        along(u.x().get(), Vec3::Y) && along(u.z().get(), -Vec3::Z),
        "{u:?}"
    );
}

/// A work feature follows an upstream edit, and a later extrude ends on it in both regens.
#[test]
fn follows_edits() {
    let up_to = |t: f64| {
        let mut fs = base(t);
        let top = Name::feature(1).with_str("end");
        fs.push(Feature::Work(Work::Plane(PlaneBy::Offset(r(0, top), 6.0))));
        fs.push(Feature::Extrude {
            profile: Profile {
                plane: Frame::WORLD.with_origin(Point3::new(5.0, 5.0, t)),
                loops: vec![Profile::circle(Point2::ORIGIN, 3.0, 0)],
            },
            extent: Extent::To {
                target: Target::Plane(2),
                offset: 0.0,
            },
            op: Combine::Add,
        });
        Part { features: fs }
    };
    for t in [10.0, 12.5] {
        let part = up_to(t);
        assert!(matches!(datum(&part.features[..3]), Datum::Plane(p) if p.origin.z == t + 6.0));
        let (a, b) = (
            part.regenerate().expect("regen"),
            part.regenerate_nary(2).expect("nary"),
        );
        assert!(check(&a).is_empty() && check(&b).is_empty());
        let want = 800.0 * t + PI * 16.0 * 15.0 + PI * 9.0 * 6.0;
        for v in [volume(&a), volume(&b)] {
            assert!((v - want).abs() < 2e-5 * want, "{v} vs {want}");
        }
    }
}

fn pattern(of: usize, layout: Layout) -> Feature {
    Feature::Pattern {
        of: Source::Features(vec![of]),
        layout,
    }
}

/// Copies along a straight edge, an edge chain of lines and arcs, an axis, and at points:
/// valid, the same in both regens, the volumes worked out by hand.
#[test]
fn along_paths_and_at_points() {
    let (w, d, t) = (60.0, 40.0, 5.0);
    let plate = extrude(
        Frame::WORLD,
        Profile::rounded_rectangle(Point2::ORIGIN, w, d, 8.0, 0),
        t,
        Combine::New,
    );
    let slab = w * d * t - (4.0 - PI) * 64.0 * t;
    // a 2 x 1 x 3 block on the top: asymmetric, so turns show
    let lug = |at: Point3| {
        extrude(
            Frame::WORLD.with_origin(at),
            Profile::rectangle(Point2::new(-1.0, -0.5), 2.0, 1.0, 0),
            3.0,
            Combine::Add,
        )
    };
    let fs = vec![plate.clone()];
    let rim: Vec<Name> = {
        let b = Part { features: fs.clone() }.regenerate().expect("plate");
        b.edge_ids()
            .map(|e| b.edge(e))
            .filter(|e| e.curve.eval(e.range.lo).z == t && e.curve.eval(e.range.hi).z == t)
            .map(|e| e.name)
            .collect()
    };
    assert_eq!(rim.len(), 8);
    let front = edge(&fs, |c, a, b| {
        matches!(c, Curve::Line(_)) && a.y == 0.0 && b.y == 0.0 && a.z == t
    });
    // the rim's path starts where its first edge does
    let start = {
        let b = Part { features: fs.clone() }.regenerate().expect("plate");
        let e = b.edge(b.edge_by_name(rim[0]).expect("rim"));
        e.curve.eval(e.range.lo)
    };
    let lug_v = 6.0;
    let cases: Vec<(&str, Layout, Point3, f64)> = vec![
        (
            "straight, fitted",
            Layout::Path {
                along: vec![r(0, front)],
                count: 5,
                spacing: None,
                turn: true,
                reverse: false,
            },
            Point3::new(8.0, 2.0, t),
            5.0,
        ),
        (
            "round the rim, turned",
            Layout::Path {
                along: rim.iter().map(|&n| r(0, n)).collect(),
                count: 12,
                spacing: None,
                turn: true,
                reverse: false,
            },
            start,
            12.0,
        ),
        (
            "round the rim, fixed, backwards, spaced",
            Layout::Path {
                along: rim.iter().map(|&n| r(0, n)).collect(),
                count: 7,
                spacing: Some(20.0),
                turn: false,
                reverse: true,
            },
            start,
            7.0,
        ),
        (
            "at points",
            Layout::Points(vec![
                At::Xyz(Point3::new(10.0, 10.0, t)),
                At::Xyz(Point3::new(50.0, 10.0, t)),
                At::Ref(r(0, vertex(&fs, Point3::new(52.0, 0.0, t)))),
                At::Xyz(Point3::new(30.0, 30.0, t)),
            ]),
            Point3::new(10.0, 10.0, t),
            4.0,
        ),
    ];
    for (what, layout, at, n) in cases {
        let part = Part {
            features: vec![plate.clone(), lug(at), pattern(1, layout)],
        };
        let a = part.regenerate().unwrap_or_else(|e| panic!("{what}: {e:?}"));
        let b = part
            .regenerate_nary(2)
            .unwrap_or_else(|e| panic!("{what}: {e:?}"));
        assert!(check(&a).is_empty(), "{what}: {:?}", check(&a));
        assert!(check(&b).is_empty(), "{what}: {:?}", check(&b));
        // the lugs sit on the plate's top, on it or hanging off its rim
        let want = slab + n * lug_v;
        for v in [volume(&a), volume(&b)] {
            assert!((v - want).abs() < 2e-5 * want, "{what}: {v} vs {want}");
        }
    }

    // along a work axis, square to the plate: copies stacked up
    let mut fs = vec![plate.clone(), lug(Point3::new(30.0, 20.0, t))];
    let top = Name::feature(1).with_str("end");
    fs.push(Feature::Work(Work::Axis(AxisBy::Normal {
        plane: r(0, top),
        at: r(0, vertex(std::slice::from_ref(&plate), Point3::new(52.0, 0.0, t))),
    })));
    fs.push(pattern(
        1,
        Layout::Path {
            along: vec![whole(2)],
            count: 3,
            spacing: Some(3.0),
            turn: true,
            reverse: false,
        },
    ));
    let b = Part { features: fs }.regenerate_nary(1).expect("stacked");
    assert!((volume(&b) - slab - 3.0 * lug_v).abs() < 0.1, "{}", volume(&b));
}

/// A path pattern places its copies exactly: down a straight edge, a lin pattern's places.
#[test]
fn straight_path_is_linear() {
    let fs = base(10.0);
    let x_edge = edge(&fs, |c, a, b| {
        matches!(c, Curve::Line(_)) && a.y == 0.0 && b.y == 0.0 && a.z == 10.0 && b.z == 10.0
    });
    let runs_back = {
        let b = Part { features: fs.clone() }.regenerate().expect("regen");
        let e = b.edge(b.edge_by_name(x_edge).expect("edge"));
        e.curve.eval(e.range.lo).x > e.curve.eval(e.range.hi).x
    };
    let hole = Feature::Hole(Box::new(Hole {
        at: Frame::from_zx(Point3::new(4.0, 4.0, 10.0), -forge_math::UnitVec3::Z, Vec3::X).expect("frame"),
        diameter: 3.0,
        head: Head::Plain,
        end: End::Through,
        point: None,
        thread: None,
        standard: None,
    }));
    let path = Layout::Path {
        along: vec![r(0, x_edge)],
        count: 4,
        spacing: Some(10.0),
        turn: true,
        reverse: runs_back,
    };
    let lin = Layout::Linear {
        first: forge_model::Row {
            dir: Vec3::X,
            count: 4,
            spacing: 10.0,
        },
        second: None,
    };
    let a = Part {
        features: vec![fs[0].clone(), hole.clone(), pattern(1, path)],
    }
    .regenerate_nary(1)
    .expect("path");
    let b = Part {
        features: vec![fs[0].clone(), hole, pattern(1, lin)],
    }
    .regenerate_nary(1)
    .expect("lin");
    assert_eq!(a, b);
    // past the end of the edge
    let long = Layout::Path {
        along: vec![r(0, x_edge)],
        count: 6,
        spacing: Some(10.0),
        turn: true,
        reverse: runs_back,
    };
    let e = Part {
        features: vec![fs[0].clone(), pattern(0, long)],
    }
    .regenerate();
    assert!(e.is_err());
}

/// `M6 tap` is a 5 mm hole that records its M6x1 thread; standard heads fit their screws.
#[test]
fn standard_holes() {
    let m6 = standard::screw("M6").expect("M6");
    let std = |convention, fit| Standard {
        screw: m6,
        convention,
        fit,
    };
    let (tap, clear, ansi) = (
        std(Convention::Iso, Fit::Tap),
        std(Convention::Iso, Fit::Normal),
        std(Convention::Ansi, Fit::Normal),
    );
    let at = Frame::from_zx(Point3::new(20.0, 10.0, 10.0), -forge_math::UnitVec3::Z, Vec3::X).expect("frame");
    let hole = |s: Standard, head: Head| Hole {
        at,
        diameter: s.diameter(),
        head,
        end: End::Through,
        point: None,
        thread: s.thread(),
        standard: Some(s),
    };
    let tapped = hole(tap, Head::Plain);
    assert_eq!(tapped.diameter, 5.0);
    assert_eq!(tapped.thread.map(|t| (t.size, t.pitch)), Some((6.0, 1.0)));
    for (h, cut) in [
        (tapped, PI * 6.25 * 10.0),
        // DIN 974-1: 11 across; ASME B18.3.1M: 11.25
        (
            hole(clear, clear.counterbore()),
            PI * (5.5 * 5.5 * 6.0 + 3.3 * 3.3 * 4.0),
        ),
        (
            hole(ansi, ansi.counterbore()),
            PI * (5.625 * 5.625 * 6.0 + 3.3 * 3.3 * 4.0),
        ),
    ] {
        let mut fs = base(10.0);
        fs.truncate(1);
        fs.push(Feature::Hole(Box::new(h)));
        let part = Part { features: fs };
        let b = part.regenerate_nary(1).expect("standard hole");
        assert!((volume(&b) - (8000.0 - cut)).abs() < 0.1, "{}", volume(&b));
        assert_eq!(part.threads().len(), h.thread.iter().count());
    }
}
/// Round a disc's rim, turned copies turn with the tangent; fixed ones keep facing one way.
#[test]
fn turned_or_fixed() {
    let t = 4.0;
    let disc = extrude(
        Frame::WORLD,
        Profile::circle(Point2::ORIGIN, 20.0, 0),
        t,
        Combine::New,
    );
    let rim = edge(std::slice::from_ref(&disc), |c, a, _| {
        matches!(c, Curve::Circle(_)) && a.z == t
    });
    let b = Part {
        features: vec![disc.clone()],
    }
    .regenerate()
    .expect("disc");
    let e = b.edge(b.edge_by_name(rim).expect("rim"));
    let start = e.curve.eval(e.range.lo);
    let lug = extrude(
        Frame::WORLD.with_origin(start),
        Profile::rectangle(Point2::new(-1.0, -1.0), 2.0, 2.0, 0),
        3.0,
        Combine::Add,
    );
    let out = Vec3::new(start.x, start.y, 0.0)
        .normalize()
        .expect("off centre")
        .get();
    // the lug's top keeps its sketch's frame: x points out from the centre
    let cap = Name::feature(2).with_str("end");
    for turn in [false, true] {
        let part = Part {
            features: vec![
                disc.clone(),
                lug.clone(),
                pattern(
                    1,
                    Layout::Path {
                        along: vec![r(0, rim)],
                        count: 4,
                        spacing: None,
                        turn,
                        reverse: false,
                    },
                ),
            ],
        };
        let b = part.regenerate_nary(1).expect("pattern");
        assert!(check(&b).is_empty());
        for k in 1..4u64 {
            let f = b
                .face_by_name(forge_model::instance_name(3, [k, 0], cap))
                .expect("copy");
            let Surface::Plane(p) = &b.face(f).surface else {
                panic!("a plane")
            };
            let o = p.frame.origin;
            assert!(
                (Vec3::new(o.x, o.y, 0.0).length() - 20.0).abs() < 1e-9,
                "copy {k} at {o:?}"
            );
            let x = p.frame.x().get();
            if turn {
                let radial = Vec3::new(o.x, o.y, 0.0).normalize().expect("r").get();
                assert!(along(x, radial), "copy {k}: {x:?} at {o:?}");
            } else {
                assert!(along(x, out), "copy {k}: {x:?}");
            }
        }
    }
}
