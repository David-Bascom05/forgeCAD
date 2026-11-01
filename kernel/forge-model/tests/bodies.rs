//! Multi-body parts: features on chosen bodies, combining, splitting and moving bodies, and
//! splitting faces. Every body valid, volumes worked out by hand, the n-ary regen the same.

use forge_math::{Frame, Point2, Point3, Vec3, dmath};
use forge_mesh::{Quality, tessellate};
use forge_model::{
    BodyError, BodyFeature, Combine, Cutter, Extent, Feature, Keep, Part, Pick, RegenError, Step,
};
use forge_ops::{Profile, Radius, Segment};
use forge_topo::{Body, Name, check};
use std::collections::HashSet;

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

fn block(corner: Point3, w: f64, d: f64, h: f64, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile {
            plane: Frame::WORLD.with_origin(corner),
            loops: vec![Profile::rectangle(Point2::ORIGIN, w, d, 0)],
        },
        extent: Extent::Distance(h),
        op,
    }
}

fn rod(base: Point3, r: f64, h: f64, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile {
            plane: Frame::WORLD.with_origin(base),
            loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
        },
        extent: Extent::Distance(h),
        op,
    }
}

fn body(b: BodyFeature) -> Feature {
    Feature::Body(Box::new(b))
}

fn on(ids: &[usize], f: Feature) -> Feature {
    body(BodyFeature::In {
        bodies: Pick::Some(ids.to_vec()),
        feature: f,
    })
}

fn p3(x: f64, y: f64, z: f64) -> Point3 {
    Point3::new(x, y, z)
}

/// Both regens: each body valid, the same bodies with the same volumes; the volumes.
fn bodies(features: Vec<Feature>) -> Vec<(usize, f64)> {
    let part = Part { features };
    let seq = part.regenerate_solids().expect("sequential");
    compare(&seq, &part)
}

/// The sequential regen alone, for histories naming faces the n-ary regen names otherwise
/// (a face a boolean only adds a loop to keeps its name feature by feature, not in n-ary).
fn sequential(features: Vec<Feature>) -> Vec<(usize, f64)> {
    let seq = Part { features }.regenerate_solids().expect("sequential");
    seq.iter()
        .map(|s| {
            assert!(check(&s.body).is_empty(), "body {}: {:?}", s.id, check(&s.body));
            (s.id, volume(&s.body))
        })
        .collect()
}

fn compare(seq: &[forge_model::Solid], part: &Part) -> Vec<(usize, f64)> {
    let nary = part.regenerate_nary_solids(4).expect("n-ary");
    let one = part.regenerate_nary_solids(1).expect("n-ary, one thread");
    assert_eq!(nary, one, "threads change nothing");
    assert_eq!(seq.len(), nary.len());
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (a, b) in seq.iter().zip(&nary) {
        assert_eq!(a.id, b.id);
        for s in [a, b] {
            assert!(check(&s.body).is_empty(), "body {}: {:?}", s.id, check(&s.body));
        }
        let (va, vb) = (volume(&a.body), volume(&b.body));
        assert!(
            (va - vb).abs() <= 1e-6 * va.abs().max(1.0),
            "body {}: {va} vs {vb}",
            a.id
        );
        // names stay unique across the part
        for f in a.body.face_ids() {
            assert!(seen.insert(a.body.face(f).name), "a face name in two bodies");
        }
        for e in a.body.edge_ids() {
            assert!(seen.insert(a.body.edge(e).name), "an edge name in two bodies");
        }
        out.push((a.id, va));
    }
    out
}

/// Equal to what the meshes' volumes can tell.
fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-4 * b.abs().max(1.0)
}

#[test]
fn a_new_solid_is_a_body_of_its_own() {
    // overlapping bodies stay apart
    let f = vec![
        block(p3(0.0, 0.0, 0.0), 10.0, 10.0, 10.0, Combine::New),
        block(p3(5.0, 5.0, 5.0), 10.0, 10.0, 10.0, Combine::New),
    ];
    let got = bodies(f.clone());
    assert_eq!(got.len(), 2);
    assert!(near(got[0].1, 1000.0) && near(got[1].1, 1000.0), "{got:?}");
    assert_eq!(
        Part { features: f.clone() }.regenerate(),
        Err(RegenError::Bodies { count: 2 })
    );
    assert_eq!(
        Part { features: f }.regenerate_nary(2),
        Err(RegenError::Bodies { count: 2 })
    );
}

#[test]
fn features_go_to_the_current_or_chosen_bodies() {
    let base = || {
        vec![
            block(p3(0.0, 0.0, 0.0), 10.0, 10.0, 10.0, Combine::New),
            block(p3(20.0, 0.0, 0.0), 10.0, 10.0, 10.0, Combine::New),
        ]
    };
    let hole = |x: f64| rod(p3(x, 5.0, -1.0), 2.0, 12.0, Combine::Cut);
    let cyl = PI * 4.0 * 10.0;
    // the current body: the last made
    let mut f = base();
    f.push(hole(25.0));
    let got = bodies(f);
    assert!(near(got[0].1, 1000.0) && near(got[1].1, 1000.0 - cyl), "{got:?}");
    // a chosen one
    let mut f = base();
    f.push(on(&[0], hole(5.0)));
    let got = bodies(f);
    assert!(near(got[0].1, 1000.0 - cyl) && near(got[1].1, 1000.0), "{got:?}");
    // slabs cut from both
    let mut f = base();
    f.push(on(
        &[0, 1],
        block(p3(-5.0, -5.0, 8.0), 50.0, 20.0, 5.0, Combine::Cut),
    ));
    f.push(body(BodyFeature::In {
        bodies: Pick::All,
        feature: block(p3(-5.0, -5.0, 6.0), 50.0, 20.0, 1.0, Combine::Cut),
    }));
    let got = bodies(f);
    assert!(near(got[0].1, 700.0) && near(got[1].1, 700.0), "{got:?}");
    // a body nothing made
    let mut f = base();
    f.push(on(&[5], hole(5.0)));
    assert_eq!(
        Part { features: f }.regenerate_solids(),
        Err(RegenError::Body {
            feature: 2,
            error: BodyError::NoBody { id: 5 }
        })
    );
}

#[test]
fn a_fillet_finds_its_body_by_its_edges() {
    let mut f = vec![
        block(p3(0.0, 0.0, 0.0), 10.0, 10.0, 10.0, Combine::New),
        block(p3(20.0, 0.0, 0.0), 10.0, 10.0, 10.0, Combine::New),
    ];
    let first = Part { features: f.clone() }.regenerate_solids().expect("regen");
    let b = &first[0].body;
    let edge = b
        .edge_ids()
        .find(|&e| {
            let m = b.edge_points(e, 2)[1];
            m.x == 10.0 && m.y == 10.0
        })
        .expect("a vertical edge");
    f.push(Feature::Fillet {
        only: false,
        edges: vec![(b.edge(edge).name, Radius::Constant(2.0))],
        starts: vec![],
        setbacks: vec![],
    });
    let got = bodies(f);
    assert!(near(got[0].1, 1000.0 - (4.0 - PI) * 10.0), "{got:?}");
    assert!(near(got[1].1, 1000.0), "{got:?}");
}

#[test]
fn combining_bodies() {
    let base = || {
        vec![
            block(p3(0.0, 0.0, 0.0), 10.0, 10.0, 10.0, Combine::New),
            block(p3(5.0, 0.0, 0.0), 10.0, 10.0, 10.0, Combine::New),
            block(p3(0.0, 0.0, 8.0), 4.0, 10.0, 4.0, Combine::New),
        ]
    };
    let combine = |op, tools: Vec<usize>, keep| {
        body(BodyFeature::Combine {
            op,
            target: 0,
            tools,
            keep,
        })
    };
    let cases = [
        (Combine::Add, vec![1], false, vec![1500.0, 160.0]),
        (Combine::Cut, vec![1], false, vec![500.0, 160.0]),
        (Combine::Keep, vec![1], false, vec![500.0, 160.0]),
        (Combine::Add, vec![1, 2], false, vec![1500.0 + 80.0]),
    ];
    for (op, tools, keep, want) in cases {
        let mut f = base();
        f.push(combine(op, tools, keep));
        let got: Vec<f64> = bodies(f).into_iter().map(|b| b.1).collect();
        assert_eq!(got.len(), want.len(), "{op:?} {got:?}");
        assert!(got.iter().zip(&want).all(|(a, b)| near(*a, *b)), "{op:?} {got:?}");
    }
    // the tool kept
    let mut f = base();
    f.push(combine(Combine::Cut, vec![1], true));
    let got = bodies(f);
    assert_eq!(got.iter().map(|b| b.0).collect::<Vec<_>>(), [0, 1, 2]);
    assert!(near(got[0].1, 500.0) && near(got[1].1, 1000.0), "{got:?}");
    // later features see the result
    let mut f = base();
    f.push(combine(Combine::Add, vec![1], false));
    f.push(rod(p3(12.0, 5.0, -1.0), 1.0, 12.0, Combine::Cut));
    let got = bodies(f);
    assert_eq!(got.iter().map(|b| b.0).collect::<Vec<_>>(), [0, 2]);
    assert!(near(got[0].1, 1500.0) && near(got[1].1, 160.0), "{got:?}");
}

#[test]
fn splitting_bodies() {
    let cube = || block(p3(0.0, 0.0, 0.0), 10.0, 10.0, 10.0, Combine::New);
    let level = Frame::WORLD.with_origin(p3(0.0, 0.0, 4.0));
    let split = |by, keep| body(BodyFeature::Split { by, keep });
    let got = bodies(vec![cube(), split(Cutter::Plane(level), Keep::Both)]);
    assert_eq!(got.iter().map(|b| b.0).collect::<Vec<_>>(), [0, 1]);
    assert!(near(got[0].1, 400.0) && near(got[1].1, 600.0), "{got:?}");
    let got = bodies(vec![cube(), split(Cutter::Plane(level), Keep::Plus)]);
    assert!(got.len() == 1 && near(got[0].1, 600.0), "{got:?}");
    let got = bodies(vec![cube(), split(Cutter::Plane(level), Keep::Minus)]);
    assert!(got.len() == 1 && near(got[0].1, 400.0), "{got:?}");
    // by a slanted plane through opposite edges: two wedges
    let diag = Frame::from_zx(
        p3(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, -1.0).normalize().expect("n"),
        Vec3::Y,
    )
    .expect("frame");
    let got = bodies(vec![cube(), split(Cutter::Plane(diag), Keep::Both)]);
    assert!(near(got[0].1, 500.0) && near(got[1].1, 500.0), "{got:?}");
    // by a sketch circle's cylinder: the core and the rest
    let got = bodies(vec![
        cube(),
        split(
            Cutter::Curve(
                Frame::WORLD,
                Segment::Circle {
                    center: Point2::new(5.0, 5.0),
                    radius: 3.0,
                    ccw: true,
                },
            ),
            Keep::Both,
        ),
    ]);
    assert!(
        near(got[0].1, PI * 90.0) && near(got[1].1, 1000.0 - PI * 90.0),
        "{got:?}"
    );
    // missing the body
    let away = Frame::WORLD.with_origin(p3(0.0, 0.0, 20.0));
    assert_eq!(
        Part {
            features: vec![cube(), split(Cutter::Plane(away), Keep::Both)]
        }
        .regenerate_solids(),
        Err(RegenError::Body {
            feature: 1,
            error: BodyError::Misses
        })
    );
    // a work plane, then each piece worked on
    let got = bodies(vec![
        Feature::Plane(Frame::WORLD.with_origin(p3(0.0, 0.0, 5.0))),
        cube(),
        split(Cutter::Target(forge_model::Target::Plane(0)), Keep::Both),
        on(&[1], rod(p3(5.0, 5.0, -1.0), 1.0, 20.0, Combine::Cut)),
    ]);
    assert!(
        near(got[0].1, 500.0 - PI * 5.0) && near(got[1].1, 500.0),
        "{got:?}"
    );
}

#[test]
fn moving_and_copying_bodies() {
    let cube = || block(p3(0.0, 0.0, 0.0), 10.0, 10.0, 10.0, Combine::New);
    let mv = |steps, copy| body(BodyFeature::Move { steps, copy });
    let steps = vec![
        Step::Translate(Vec3::new(20.0, 0.0, 0.0)),
        Step::Rotate {
            origin: p3(0.0, 0.0, 0.0),
            axis: Vec3::Z,
            angle: PI / 2.0,
        },
    ];
    let part = Part {
        features: vec![cube(), mv(steps.clone(), false)],
    };
    let b = part.regenerate().expect("one body");
    let bb = b
        .face_ids()
        .fold(forge_math::Box3::EMPTY, |a, f| a.hull(&b.face_box(f)));
    assert!((bb.x.lo + 10.0).abs() < 1e-4 && bb.x.hi.abs() < 1e-4, "{bb:?}");
    assert!(
        (bb.y.lo - 20.0).abs() < 1e-4 && (bb.y.hi - 30.0).abs() < 1e-4,
        "{bb:?}"
    );
    let got = bodies(vec![cube(), mv(steps, true)]);
    assert_eq!(got.iter().map(|b| b.0).collect::<Vec<_>>(), [0, 1]);
    assert!(near(got[1].1, 1000.0));
    // by a frame, then joined back on
    let at = Frame::from_zx(p3(5.0, 5.0, 5.0), Vec3::X.normalize().expect("x"), Vec3::Y).expect("frame");
    let got = bodies(vec![
        cube(),
        mv(vec![Step::Frame(at)], true),
        body(BodyFeature::Combine {
            op: Combine::Add,
            target: 0,
            tools: vec![1],
            keep: false,
        }),
    ]);
    assert!(near(got[0].1, 2000.0 - 125.0), "{got:?}");
}

/// A split face: the solid the same, one more face per piece, every body valid.
fn split_faces(features: Vec<Feature>, faces: Vec<Name>, by: Cutter, more: usize) {
    split_faces_with(features, faces, by, more, bodies)
}

fn split_faces_with(
    features: Vec<Feature>,
    faces: Vec<Name>,
    by: Cutter,
    more: usize,
    regen: fn(Vec<Feature>) -> Vec<(usize, f64)>,
) {
    let before = Part {
        features: features.clone(),
    }
    .regenerate()
    .expect("regen");
    let mut f = features;
    f.push(body(BodyFeature::SplitFace { faces, by }));
    let got = regen(f.clone());
    let after = Part { features: f }.regenerate().expect("regen");
    assert!(near(got[0].1, volume(&before)), "{got:?}");
    assert_eq!(
        after.face_ids().count(),
        before.face_ids().count() + more,
        "faces"
    );
}

#[test]
fn splitting_faces() {
    let cube = block(p3(0.0, 0.0, 0.0), 10.0, 10.0, 10.0, Combine::New);
    let top = Name::feature(1).with_str("end");
    let at =
        |x: f64| Frame::from_zx(p3(x, 0.0, 0.0), Vec3::X.normalize().expect("x"), Vec3::Y).expect("frame");
    // a plane across the top
    split_faces(vec![cube.clone()], vec![top], Cutter::Plane(at(4.0)), 1);
    // a sketch line slanting across it, and a circle inside it
    let line = Segment::Line {
        a: Point2::new(-1.0, 2.0),
        b: Point2::new(11.0, 7.0),
    };
    split_faces(
        vec![cube.clone()],
        vec![top],
        Cutter::Curve(Frame::WORLD, line),
        1,
    );
    let circle = Segment::Circle {
        center: Point2::new(5.0, 5.0),
        radius: 2.0,
        ccw: true,
    };
    split_faces(
        vec![cube.clone()],
        vec![top],
        Cutter::Curve(Frame::WORLD, circle),
        1,
    );
    // an arc's cylinder clipping a corner
    let arc = Segment::Arc {
        center: Point2::new(10.0, 10.0),
        radius: 3.0,
        start: PI,
        sweep: PI / 2.0,
    };
    split_faces(vec![cube.clone()], vec![top], Cutter::Curve(Frame::WORLD, arc), 1);
    // across a face with a hole in it, the hole cut too
    let holed = vec![cube.clone(), rod(p3(5.0, 5.0, -1.0), 2.0, 12.0, Combine::Cut)];
    split_faces_with(holed.clone(), vec![top], Cutter::Plane(at(5.0)), 1, sequential);
    // beside the hole, which goes with its piece
    split_faces_with(holed, vec![top], Cutter::Plane(at(8.0)), 1, sequential);
    // a rod's side, along its axis and across it
    let side = Name::feature(1).with_str("side").with(0);
    let r = vec![rod(p3(0.0, 0.0, 0.0), 5.0, 10.0, Combine::New)];
    split_faces(r.clone(), vec![side], Cutter::Plane(at(0.0)), 1);
    split_faces(
        r.clone(),
        vec![side],
        Cutter::Plane(Frame::WORLD.with_origin(p3(0.0, 0.0, 4.0))),
        1,
    );
    // two faces at once
    split_faces(
        r,
        vec![side, Name::feature(1).with_str("end")],
        Cutter::Plane(at(1.0)),
        2,
    );
    // a face the cutter misses
    let away = Frame::WORLD.with_origin(p3(0.0, 0.0, 20.0));
    assert!(matches!(
        Part {
            features: vec![
                cube,
                body(BodyFeature::SplitFace {
                    faces: vec![top],
                    by: Cutter::Plane(away)
                })
            ]
        }
        .regenerate_solids(),
        Err(RegenError::Body {
            error: BodyError::NotCrossed { .. },
            ..
        })
    ));
}

#[test]
fn a_single_body_part_is_untouched() {
    // `in` naming the only body regenerates as plain features do
    let plain = vec![
        block(p3(0.0, 0.0, 0.0), 10.0, 10.0, 10.0, Combine::New),
        rod(p3(5.0, 5.0, -1.0), 2.0, 12.0, Combine::Cut),
    ];
    let mut wrapped = plain.clone();
    wrapped[1] = on(&[0], wrapped[1].clone());
    let a = Part { features: plain }.regenerate().expect("plain");
    let b = Part { features: wrapped }.regenerate_solids().expect("wrapped");
    assert_eq!(b.len(), 1);
    assert!(near(volume(&a), volume(&b[0].body)));
}
