//! Holes, patterns and mirrors through the part history: valid solids, the volumes worked out
//! by hand, the same result from the sequential and n-ary regens, and names that survive.

use forge_math::{Frame, Point2, Point3, Vec3, dmath};
use forge_mesh::{Quality, tessellate};
use forge_model::{
    Combine, End, Extent, Feature, Head, Hole, HoleError, Layout, Part, PatternError, RegenError, Row,
    Source, Thread, instance_name, suite,
};
use forge_ops::{ChamferSize, Profile};
use forge_topo::{Body, Name, check};

const PI: f64 = dmath::PI;

/// The exact volume, from the surfaces; the mesh only shows the solid is closed.
fn volume(b: &Body) -> f64 {
    let m = tessellate(b, &Quality::default()).expect("tessellates");
    assert!(m.open_edges().is_empty(), "mesh has open edges");
    let m = forge_topo::mass::mass(b).expect("mass properties");
    assert!(m.err[0] <= 1e-9 * m.volume.abs(), "error bound {}", m.err[0]);
    m.volume
}

fn frame(o: Point3, z: Vec3, x: Vec3) -> Frame {
    Frame::from_zx(o, z.normalize().expect("axis"), x).expect("frame")
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

/// A hole drilled straight down from `at`.
fn drill(at: Point3, diameter: f64, head: Head, end: End) -> Hole {
    Hole {
        at: frame(at, -Vec3::Z, Vec3::X),
        diameter,
        head,
        end,
        point: None,
        thread: None,
        standard: None,
        places: None,
    }
}

fn tan(a: f64) -> f64 {
    let (s, c) = dmath::sin_cos(a);
    s / c
}

fn linear(of: Source, first: Row, second: Option<Row>) -> Feature {
    Feature::Pattern {
        of,
        layout: Layout::Linear { first, second },
    }
}

fn row(dir: Vec3, count: usize, spacing: f64) -> Row {
    Row { dir, count, spacing }
}

fn mirror(of: Source, plane: Frame) -> Feature {
    Feature::Pattern {
        of,
        layout: Layout::Mirror { plane },
    }
}

/// Named parts and their exact volumes.
fn parts() -> Vec<(&'static str, Part, f64)> {
    let (a, b, t) = (40.0, 30.0, 10.0);
    let plate = || block(Point3::ORIGIN, a, b, t, Combine::New);
    let top = Point3::new(20.0, 15.0, t);
    let one = |h: Hole| Part {
        features: vec![plate(), Feature::Hole(Box::new(h))],
    };
    let slab = a * b * t;
    let point = 118.0 / 180.0 * PI;
    let tip = 3.0 / tan(point / 2.0);
    let boss = PI * 16.0 * 5.0;
    let mut out = vec![
        (
            "through",
            one(drill(top, 6.0, Head::Plain, End::Through)),
            slab - PI * 9.0 * t,
        ),
        (
            "counterbore",
            one(drill(
                top,
                6.0,
                Head::Counterbore {
                    diameter: 10.0,
                    depth: 4.0,
                },
                End::Through,
            )),
            slab - PI * (25.0 * 4.0 + 9.0 * 6.0),
        ),
        (
            // a 90 degree sink 3 deep, then the hole
            "countersink",
            one(drill(
                top,
                6.0,
                Head::Countersink {
                    diameter: 12.0,
                    angle: PI / 2.0,
                },
                End::Through,
            )),
            slab - PI * (36.0 + 18.0 + 9.0) - PI * 9.0 * 7.0,
        ),
        (
            "blind-flat",
            one(drill(top, 6.0, Head::Plain, End::Depth(5.0))),
            slab - PI * 9.0 * 5.0,
        ),
        (
            "blind-point",
            one(Hole {
                point: Some(point),
                thread: Some(Thread {
                    size: 6.0,
                    pitch: 1.0,
                }),
                ..drill(top, 6.0, Head::Plain, End::Depth(5.0))
            }),
            slab - PI * 9.0 * (5.0 + tip / 3.0),
        ),
        (
            // a sideways hole into the plate's end face
            "side-hole",
            one(Hole {
                at: frame(Point3::new(a, 15.0, 5.0), -Vec3::X, Vec3::Y),
                ..drill(top, 4.0, Head::Plain, End::Depth(12.0))
            }),
            slab - PI * 4.0 * 12.0,
        ),
        (
            "grid",
            Part {
                features: vec![
                    plate(),
                    Feature::Hole(Box::new(drill(
                        Point3::new(8.0, 8.0, t),
                        4.0,
                        Head::Plain,
                        End::Through,
                    ))),
                    linear(
                        Source::Features(vec![1]),
                        row(Vec3::X, 4, 8.0),
                        Some(row(Vec3::Y, 2, 14.0)),
                    ),
                ],
            },
            slab - 8.0 * PI * 4.0 * t,
        ),
        (
            "ring",
            Part {
                features: vec![
                    plate(),
                    rod(Point3::new(30.0, 15.0, t), 2.0, 5.0, Combine::Add),
                    Feature::Pattern {
                        of: Source::Features(vec![1]),
                        layout: Layout::Circular {
                            origin: Point3::new(20.0, 15.0, 0.0),
                            axis: Vec3::Z,
                            count: 6,
                            angle: dmath::TAU,
                            incremental: false,
                        },
                    },
                ],
            },
            slab + 6.0 * PI * 4.0 * 5.0,
        ),
        (
            // three slots fanned over 90 degrees about a tilted axis through the plate
            "fan",
            Part {
                features: vec![
                    plate(),
                    rod(Point3::new(28.0, 15.0, -1.0), 1.5, t + 2.0, Combine::Cut),
                    Feature::Pattern {
                        of: Source::Features(vec![1]),
                        layout: Layout::Circular {
                            origin: Point3::new(20.0, 15.0, 0.0),
                            axis: Vec3::Z,
                            count: 3,
                            angle: PI / 2.0,
                            incremental: false,
                        },
                    },
                ],
            },
            slab - 3.0 * PI * 2.25 * t,
        ),
        (
            // a boss with a hole in it, both copied, then mirrored
            "boss-mirror",
            Part {
                features: vec![
                    plate(),
                    rod(Point3::new(8.0, 8.0, t), 4.0, 5.0, Combine::Add),
                    Feature::Hole(Box::new(drill(
                        Point3::new(8.0, 8.0, t + 5.0),
                        3.0,
                        Head::Plain,
                        End::Depth(8.0),
                    ))),
                    linear(Source::Features(vec![1, 2]), row(Vec3::X, 2, 12.0), None),
                    mirror(
                        Source::Features(vec![3]),
                        frame(Point3::new(0.0, 15.0, 0.0), Vec3::Y, Vec3::X),
                    ),
                ],
            },
            slab + 4.0 * (boss - PI * 2.25 * 8.0),
        ),
        (
            // a plate with an off-centre hole, mirrored about its end face: one solid twice as long
            "body-mirror",
            Part {
                features: vec![
                    plate(),
                    Feature::Hole(Box::new(drill(
                        Point3::new(10.0, 10.0, t),
                        6.0,
                        Head::Plain,
                        End::Through,
                    ))),
                    mirror(Source::Body, frame(Point3::new(a, 0.0, 0.0), Vec3::X, Vec3::Y)),
                ],
            },
            2.0 * (slab - PI * 9.0 * t),
        ),
        (
            // blocks side by side sharing faces, then a hole through all three
            "body-row",
            Part {
                features: vec![
                    block(Point3::ORIGIN, 10.0, 10.0, 10.0, Combine::New),
                    linear(Source::Body, row(Vec3::X, 3, 10.0), None),
                    Feature::Hole(Box::new(drill(
                        Point3::new(15.0, 5.0, 10.0),
                        4.0,
                        Head::Plain,
                        End::Through,
                    ))),
                ],
            },
            3000.0 - PI * 4.0 * 10.0,
        ),
        (
            // a tilted boss mirrored by a slanted plane, the copy kept clear of the original
            "slanted-mirror",
            Part {
                features: vec![
                    plate(),
                    Feature::Extrude {
                        profile: Profile {
                            plane: frame(
                                Point3::new(10.0, 10.0, t - 1.0),
                                Vec3::new(0.2, 0.1, 1.0),
                                Vec3::X,
                            ),
                            loops: vec![Profile::rectangle(Point2::ORIGIN, 4.0, 3.0, 0)],
                        },
                        extent: Extent::Distance(6.0),
                        op: Combine::Add,
                    },
                    mirror(
                        Source::Features(vec![1]),
                        frame(Point3::new(20.0, 15.0, 0.0), Vec3::new(1.0, 1.0, 0.0), Vec3::Z),
                    ),
                ],
            },
            0.0,
        ),
    ];
    // the slanted boss: a right prism 4 x 3 x 6, the part below the plate's top inside it
    let f = frame(
        Point3::new(10.0, 10.0, t - 1.0),
        Vec3::new(0.2, 0.1, 1.0),
        Vec3::X,
    );
    let (xz, yz, nz) = (f.x().get().z, f.y().get().z, f.z().get().z);
    let sunk = 12.0 * (1.0 - 2.0 * xz - 1.5 * yz) / nz;
    out.last_mut().expect("parts").2 = slab + 2.0 * (72.0 - sunk);
    out
}

#[test]
fn volumes_by_hand() {
    for (name, part, want) in parts() {
        let seq = part.regenerate().unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert!(check(&seq).is_empty(), "{name}: {:?}", check(&seq));
        let v = volume(&seq);
        assert!((v - want).abs() <= 1e-9 * want, "{name}: volume {v} want {want}");
        for threads in [1, 4] {
            let nary = part
                .regenerate_nary(threads)
                .unwrap_or_else(|e| panic!("{name} nary: {e:?}"));
            assert!(check(&nary).is_empty(), "{name} nary: {:?}", check(&nary));
            assert_eq!(nary.face_ids().count(), seq.faces.len(), "{name}");
            let vn = volume(&nary);
            assert!((vn - v).abs() <= 1e-9 * v, "{name}: nary volume {vn} vs {v}");
        }
    }
}

/// The suite's pattern parts regenerate the same both ways.
#[test]
fn suite_pattern_parts() {
    for name in ["tour", "hole-grid"] {
        let part = suite::parts()
            .into_iter()
            .find(|p| p.0 == name)
            .expect("suite part")
            .1;
        let seq = part.regenerate().expect("seq");
        let nary = part.regenerate_nary(4).expect("nary");
        assert!(check(&seq).is_empty() && check(&nary).is_empty(), "{name}");
        assert_eq!(seq.faces.len(), nary.face_ids().count(), "{name}");
    }
    let grid = suite::hole_grid().regenerate_nary(4).expect("grid");
    let (w, t) = (400.0, 12.0);
    let hole = PI * (5.5 * 5.5 * 6.5 + 3.3 * 3.3 * 5.5);
    let want = w * w * t - 400.0 * hole;
    let v = volume(&grid);
    assert!((v - want).abs() <= 1e-9 * want, "grid volume {v} want {want}");
}

/// Copies keep names made from the pattern, their place in it and the original's name: the same
/// whatever the spacing, and a later chamfer can pick out one copy's edge.
#[test]
fn copies_have_stable_names() {
    let (a, b, t) = (60.0, 30.0, 10.0);
    let part = |spacing: f64, chamfer: bool| {
        let mut features = vec![
            block(Point3::ORIGIN, a, b, t, Combine::New),
            rod(Point3::new(8.0, 15.0, t - 1.0), 3.0, 6.0, Combine::Add),
            linear(Source::Features(vec![1]), row(Vec3::X, 4, spacing), None),
        ];
        let boss = Name::feature(2);
        let rim = Name::NONE.with_set(&[boss.with_str("end"), boss.with_str("side").with(0)]);
        if chamfer {
            features.push(Feature::Chamfer {
                corner: Default::default(),
                edges: vec![instance_name(3, [2, 0], rim)],
                size: ChamferSize::Equal(1.0),
            });
        }
        Part { features }
    };
    let cap = Name::feature(2).with_str("end");
    let names = |b: &Body| -> Vec<Name> { b.face_ids().map(|f| b.face(f).name).collect() };
    let (n1, n2) = (
        names(&part(12.0, false).regenerate().expect("12")),
        names(&part(14.0, false).regenerate_nary(1).expect("14")),
    );
    for k in 1..4 {
        let copy = instance_name(3, [k, 0], cap);
        assert!(n1.contains(&copy) && n2.contains(&copy), "copy {k}");
    }
    let plain = volume(&part(12.0, false).regenerate().expect("plain"));
    for b in [
        part(12.0, true).regenerate().expect("seq chamfer"),
        part(12.0, true).regenerate_nary(4).expect("nary chamfer"),
    ] {
        assert!(check(&b).is_empty());
        let cut = PI * (3.0 - 1.0 / 3.0) * 1.0;
        let v = volume(&b);
        assert!(
            (plain - cut - v).abs() <= 1e-9 * plain,
            "chamfered copy: {v} vs {}",
            plain - cut
        );
    }
}

/// Threads are listed for every copy of a tapped hole, each found on its own wall faces.
#[test]
fn threads_follow_copies() {
    let mut tour = suite::tour();
    let body = tour.regenerate_nary(4).expect("tour");
    let found = tour.thread_faces(&body);
    // one tapped hole, three in its row, mirrored, then the whole part mirrored
    assert_eq!(found.len(), 12);
    let mut all: Vec<_> = found.iter().flat_map(|f| f.1.clone()).collect();
    for (t, faces) in &found {
        assert_eq!((t.size, t.pitch), (5.0, 0.8));
        assert!(!faces.is_empty());
        assert!(faces.iter().all(|&f| body.face(f).name != Name::NONE));
    }
    all.sort();
    all.dedup();
    assert_eq!(
        all.len(),
        found.iter().map(|f| f.1.len()).sum::<usize>(),
        "a face on two threads"
    );
    tour.features.truncate(3);
    assert_eq!(tour.threads().len(), 1);
}

/// Bad sizes and layouts are refused with the reason.
#[test]
fn refusals() {
    let plate = block(Point3::ORIGIN, 40.0, 30.0, 10.0, Combine::New);
    let top = Point3::new(20.0, 15.0, 10.0);
    let refused = |f: Feature| {
        Part {
            features: vec![plate.clone(), f],
        }
        .regenerate()
        .expect_err("refused")
    };
    let narrow = Head::Counterbore {
        diameter: 5.0,
        depth: 2.0,
    };
    for h in [
        drill(top, 0.0, Head::Plain, End::Through),
        drill(top, 6.0, narrow, End::Through),
        drill(top, 6.0, Head::Plain, End::Depth(-1.0)),
        drill(
            top,
            6.0,
            Head::Countersink {
                diameter: 12.0,
                angle: PI,
            },
            End::Through,
        ),
        Hole {
            point: Some(0.0),
            ..drill(top, 6.0, Head::Plain, End::Depth(3.0))
        },
    ] {
        assert_eq!(
            refused(Feature::Hole(Box::new(h))),
            RegenError::Hole {
                feature: 1,
                error: HoleError::BadSize
            }
        );
    }
    let bad = |error| RegenError::Pattern { feature: 1, error };
    assert_eq!(
        refused(linear(Source::Body, row(Vec3::ZERO, 2, 5.0), None)),
        bad(PatternError::BadLayout)
    );
    assert_eq!(
        refused(linear(Source::Body, row(Vec3::X, 0, 5.0), None)),
        bad(PatternError::BadLayout)
    );
    assert_eq!(
        refused(linear(Source::Features(vec![1]), row(Vec3::X, 2, 5.0), None)),
        bad(PatternError::BadSource { feature: 1 })
    );
}
