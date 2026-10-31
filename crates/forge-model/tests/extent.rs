//! Extents through the part history: both ways, two distances, through everything, up to a
//! plane or an earlier feature's face, for extrudes, revolves and holes. Volumes by hand, and the
//! same solid from the sequential and n-ary regens.

use forge_math::{Frame, Point2, Point3, Vec2, Vec3, dmath};
use forge_mesh::{Quality, tessellate};
use forge_model::{
    Combine, End, Extent, ExtentError, Feature, Head, Hole, Part, RegenError, Target, Toward, Turn,
};
use forge_ops::{Axis2, Profile};
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

fn at(z: f64) -> Frame {
    Frame::WORLD.with_origin(Point3::new(0.0, 0.0, z))
}

fn rect(plane: Frame, x: f64, y: f64, w: f64, d: f64, extent: Extent, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile {
            plane,
            loops: vec![Profile::rectangle(Point2::new(x, y), w, d, 0)],
        },
        extent,
        op,
    }
}

fn disc(plane: Frame, x: f64, y: f64, r: f64, extent: Extent, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile {
            plane,
            loops: vec![Profile::circle(Point2::new(x, y), r, 0)],
        },
        extent,
        op,
    }
}

/// A 40 x 30 x 10 plate, then `rest`.
fn on_plate(rest: Vec<Feature>) -> Part {
    let mut features = vec![rect(
        at(0.0),
        0.0,
        0.0,
        40.0,
        30.0,
        Extent::Distance(10.0),
        Combine::New,
    )];
    features.extend(rest);
    Part { features }
}

/// A ring section r 4..7, h 0..3 in the sketch standing on the xz plane, turned about world z.
fn ring(extent: Turn) -> Feature {
    let plane = Frame::from_zx(Point3::ORIGIN, Vec3::Y.normalize().expect("y"), Vec3::X).expect("frame");
    Feature::Revolve {
        profile: Profile {
            plane,
            loops: vec![Profile::rectangle(Point2::new(4.0, -3.0), 3.0, 3.0, 0)],
        },
        axis: Axis2 {
            origin: Point2::ORIGIN,
            dir: Vec2::Y,
        },
        extent,
        op: Combine::New,
    }
}

fn parts() -> Vec<(&'static str, Part, f64)> {
    let slab = 12000.0;
    let pin = PI * 9.0 * 10.0;
    let ring_area = 0.5 * (49.0 - 16.0) * 3.0;
    let floor = Name::feature(1).with_str("start");
    vec![
        (
            "symmetric",
            Part {
                features: vec![rect(
                    at(5.0),
                    0.0,
                    0.0,
                    40.0,
                    30.0,
                    Extent::Symmetric(10.0),
                    Combine::New,
                )],
            },
            slab,
        ),
        (
            "two-sided",
            Part {
                features: vec![rect(
                    at(3.0),
                    0.0,
                    0.0,
                    40.0,
                    30.0,
                    Extent::TwoSided {
                        forward: 7.0,
                        back: 3.0,
                    },
                    Combine::New,
                )],
            },
            slab,
        ),
        (
            "through-back",
            on_plate(vec![disc(
                at(10.0),
                20.0,
                15.0,
                3.0,
                Extent::Through(Toward::Back),
                Combine::Cut,
            )]),
            slab - pin,
        ),
        (
            "through-both",
            on_plate(vec![disc(
                at(5.0),
                20.0,
                15.0,
                3.0,
                Extent::Through(Toward::Both),
                Combine::Cut,
            )]),
            slab - pin,
        ),
        (
            "through-forward",
            on_plate(vec![disc(
                at(-3.0),
                20.0,
                15.0,
                3.0,
                Extent::Through(Toward::Forward),
                Combine::Cut,
            )]),
            slab - pin,
        ),
        (
            "to-plane",
            on_plate(vec![
                Feature::Plane(at(18.0)),
                disc(
                    at(10.0),
                    20.0,
                    15.0,
                    4.0,
                    Extent::To {
                        target: Target::Plane(1),
                        offset: 0.0,
                    },
                    Combine::Add,
                ),
            ]),
            slab + PI * 16.0 * 8.0,
        ),
        (
            // a work plane given upside down, and an offset short of it
            "to-plane-offset",
            on_plate(vec![
                Feature::Plane(at(18.0).flipped()),
                disc(
                    at(10.0),
                    20.0,
                    15.0,
                    4.0,
                    Extent::To {
                        target: Target::Plane(1),
                        offset: -2.0,
                    },
                    Combine::Add,
                ),
            ]),
            slab + PI * 16.0 * 6.0,
        ),
        (
            // a pocket from the top toward the plate's own floor, stopping 4 short of it
            "to-face",
            on_plate(vec![rect(
                at(10.0),
                5.0,
                5.0,
                10.0,
                8.0,
                Extent::To {
                    target: Target::Face {
                        feature: 0,
                        name: floor,
                    },
                    offset: -4.0,
                },
                Combine::Cut,
            )]),
            slab - 80.0 * 6.0,
        ),
        (
            // a boss up to the top of an earlier boss's own end face
            "to-face-of-boss",
            on_plate(vec![
                disc(at(10.0), 8.0, 8.0, 3.0, Extent::Distance(5.0), Combine::Add),
                disc(
                    at(10.0),
                    25.0,
                    15.0,
                    4.0,
                    Extent::To {
                        target: Target::Face {
                            feature: 1,
                            name: Name::feature(2).with_str("end"),
                        },
                        offset: 1.0,
                    },
                    Combine::Add,
                ),
            ]),
            slab + PI * 9.0 * 5.0 + PI * 16.0 * 6.0,
        ),
        (
            "hole-to-plane",
            on_plate(vec![
                Feature::Plane(at(4.0)),
                Feature::Hole(Box::new(Hole {
                    at: Frame::from_zx(Point3::new(20.0, 15.0, 10.0), -forge_math::UnitVec3::Z, Vec3::X)
                        .expect("frame"),
                    diameter: 6.0,
                    head: Head::Plain,
                    end: End::To {
                        target: Target::Plane(1),
                        offset: 0.0,
                    },
                    point: None,
                    thread: None,
                    standard: None,
                    places: None,
                })),
            ]),
            slab - PI * 9.0 * 6.0,
        ),
        (
            "turn-symmetric",
            Part {
                features: vec![ring(Turn::Symmetric(PI))],
            },
            PI * ring_area,
        ),
        (
            "turn-two-sided",
            Part {
                features: vec![ring(Turn::TwoSided {
                    forward: 1.0,
                    back: 0.5,
                })],
            },
            1.5 * ring_area,
        ),
        (
            // to the yz plane: a quarter turn, then a bit more
            "turn-to-plane",
            Part {
                features: vec![
                    Feature::Plane(
                        Frame::from_zx(Point3::ORIGIN, forge_math::UnitVec3::X, Vec3::Y).expect("yz"),
                    ),
                    ring(Turn::To {
                        target: Target::Plane(0),
                        offset: 0.25,
                    }),
                ],
            },
            (PI / 2.0 + 0.25) * ring_area,
        ),
    ]
}

#[test]
fn extents_by_hand() {
    for (name, part, want) in parts() {
        let seq = part.regenerate().unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert!(check(&seq).is_empty(), "{name}: {:?}", check(&seq));
        let v = volume(&seq);
        assert!((v - want).abs() <= 1e-9 * want, "{name}: volume {v} want {want}");
        let nary = part
            .regenerate_nary(4)
            .unwrap_or_else(|e| panic!("{name} nary: {e:?}"));
        assert!(check(&nary).is_empty(), "{name} nary");
        assert_eq!(nary.faces.len(), seq.faces.len(), "{name}");
        assert!((volume(&nary) - v).abs() <= 1e-9 * v, "{name} nary volume");
    }
}

#[test]
fn extent_refusals() {
    // each case after a work plane, at 15 unless it says otherwise
    let refused = |plane: Frame, f: Feature| {
        on_plate(vec![Feature::Plane(plane), f])
            .regenerate()
            .expect_err("refused")
    };
    let bad = |error| RegenError::Extent { feature: 2, error };
    let tilted = Frame::from_zx(
        Point3::new(0.0, 0.0, 15.0),
        Vec3::new(0.0, 0.3, 1.0).normalize().expect("n"),
        Vec3::X,
    )
    .expect("frame");
    let to = |target, offset| Extent::To { target, offset };
    let cases = [
        // not a work plane
        (
            at(15.0),
            disc(at(10.0), 20.0, 15.0, 3.0, to(Target::Plane(0), 0.0), Combine::Add),
            ExtentError::NoFace,
        ),
        (
            at(15.0),
            disc(
                at(10.0),
                20.0,
                15.0,
                3.0,
                Extent::Through(Toward::Forward),
                Combine::Add,
            ),
            ExtentError::ThroughJoins,
        ),
        (
            tilted,
            // tilted, it ends on the plane, which here cuts the sketch
            disc(at(10.0), 20.0, 15.0, 3.0, to(Target::Plane(1), 0.0), Combine::Add),
            ExtentError::NotAhead,
        ),
        (
            at(10.0),
            disc(at(10.0), 20.0, 15.0, 3.0, to(Target::Plane(1), 0.0), Combine::Add),
            ExtentError::AlreadyThere,
        ),
        (
            at(14.0),
            disc(
                at(10.0),
                20.0,
                15.0,
                3.0,
                to(Target::Plane(1), -5.0),
                Combine::Add,
            ),
            ExtentError::BadSize,
        ),
        (
            at(15.0),
            disc(
                at(10.0),
                20.0,
                15.0,
                3.0,
                to(
                    Target::Face {
                        feature: 0,
                        name: Name::feature(1).with_str("nothing"),
                    },
                    0.0,
                ),
                Combine::Add,
            ),
            ExtentError::NoFace,
        ),
        (
            at(15.0),
            disc(at(10.0), 20.0, 15.0, 3.0, Extent::Symmetric(-1.0), Combine::Add),
            ExtentError::BadSize,
        ),
    ];
    for (plane, f, want) in cases {
        assert_eq!(refused(plane, f), bad(want));
    }
    // a rod's side, travelled along
    let part = on_plate(vec![
        disc(at(10.0), 8.0, 8.0, 3.0, Extent::Distance(5.0), Combine::Add),
        disc(
            at(10.0),
            25.0,
            15.0,
            3.0,
            Extent::To {
                target: Target::Face {
                    feature: 1,
                    name: Name::feature(2).with_str("side").with(0),
                },
                offset: 0.0,
            },
            Combine::Add,
        ),
    ]);
    assert_eq!(
        part.regenerate().expect_err("refused"),
        RegenError::Extent {
            feature: 2,
            error: ExtentError::NeverHits
        }
    );
}
