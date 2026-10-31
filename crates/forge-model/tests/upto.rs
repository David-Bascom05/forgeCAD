//! Ends on a surface: extrudes and holes cut off at a tilted plane, a sphere, a cylinder or a
//! cone (from outside it or inside), and up to the next face. Volumes by hand (or by quadrature
//! where the end is a cone met from the side), and the same solid from both regens.

use forge_math::{Frame, Point2, Point3, Vec2, Vec3, dmath};
use forge_mesh::{Quality, tessellate};
use forge_model::{Combine, End, Extent, ExtentError, Feature, Head, Hole, Part, RegenError, Target, Turn};
use forge_ops::{Axis2, Profile, ProfileLoop, Segment};
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

fn frame(o: Point3, z: Vec3, x: Vec3) -> Frame {
    Frame::from_zx(o, z.normalize().expect("axis"), x).expect("frame")
}

fn at(x: f64, y: f64, z: f64) -> Frame {
    Frame::WORLD.with_origin(Point3::new(x, y, z))
}

fn extrude(plane: Frame, l: ProfileLoop, extent: Extent, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile {
            plane,
            loops: vec![l],
        },
        extent,
        op,
    }
}

fn to(feature: usize, name: Name, offset: f64) -> Extent {
    Extent::To {
        target: Target::Face { feature, name },
        offset,
    }
}

/// Feature `i`'s wall from profile segment `k` (its first half, for a revolve).
fn wall(i: usize, k: u64) -> Name {
    Name::feature(i as u64 + 1).with_str("side").with(k)
}

fn ball(c: Point3, r: f64, op: Combine) -> Feature {
    Feature::Ball {
        frame: Frame::WORLD.with_origin(c),
        radius: r,
        op,
    }
}

/// A solid cone standing on z = 10, radius 20 there, apex at z = 30.
fn cone() -> Feature {
    let plane = frame(Point3::ORIGIN, Vec3::Y, Vec3::X);
    let pts = [(0.0, -30.0), (20.0, -10.0), (0.0, -10.0)];
    let segs = (0..3)
        .map(|k| {
            let (a, b) = (pts[k], pts[(k + 1) % 3]);
            (
                Segment::Line {
                    a: Point2::new(a.0, a.1),
                    b: Point2::new(b.0, b.1),
                },
                k as u64,
            )
        })
        .collect();
    Feature::Revolve {
        profile: Profile {
            plane,
            loops: vec![ProfileLoop::new(segs)],
        },
        axis: Axis2 {
            origin: Point2::ORIGIN,
            dir: Vec2::Y,
        },
        extent: Turn::Angle(dmath::TAU),
        op: Combine::New,
    }
}

/// `f(y, z)` integrated over the disc of radius `r` about (0, `zc`), by Gauss-Legendre in both
/// polar directions.
fn over_disc(r: f64, zc: f64, f: impl Fn(f64, f64) -> f64) -> f64 {
    // 20-point Gauss-Legendre on [-1, 1]
    let nodes = [
        (0.076_526_521_133_497_33, 0.152_753_387_130_725_85),
        (0.227_785_851_141_645_08, 0.149_172_986_472_603_75),
        (0.373_706_088_715_419_56, 0.142_096_109_318_382_05),
        (0.510_867_001_950_827_1, 0.131_688_638_449_176_63),
        (0.636_053_680_726_515_1, 0.118_194_531_961_518_42),
        (0.746_331_906_460_150_8, 0.101_930_119_817_240_44),
        (0.839_116_971_822_218_8, 0.083_276_741_576_704_75),
        (0.912_234_428_251_326, 0.062_672_048_334_109_06),
        (0.963_971_927_277_913_8, 0.040_601_429_800_386_94),
        (0.993_128_599_185_094_9, 0.017_614_007_139_152_12),
    ];
    let gl: Vec<(f64, f64)> = nodes.iter().flat_map(|&(x, w)| [(x, w), (-x, w)]).collect();
    let mut sum = 0.0;
    for &(u, wu) in &gl {
        let rho = r * (u + 1.0) / 2.0;
        for &(v, wv) in &gl {
            let th = PI * (v + 1.0);
            let (s, c) = dmath::sin_cos(th);
            sum += wu * wv * rho * f(rho * c, zc + rho * s);
        }
    }
    sum * r / 2.0 * PI
}

fn parts() -> Vec<(&'static str, Part, f64)> {
    let tilt = Vec3::new(0.3, 0.0, 1.0);
    let rect = || Profile::rectangle(Point2::ORIGIN, 10.0, 6.0, 0);
    let disc = |r: f64| Profile::circle(Point2::ORIGIN, r, 0);
    let cap = |h: f64, r: f64| PI / 3.0 * h * h * (3.0 * r - h);
    let k = 1.09f64.sqrt();
    // the sphere's surface seen from a rod of radius 3 below it (centre 20 up, radius 10): the
    // rod's top is the sphere's bottom cap over the rod's disc
    let (rad, shadow) = (10.0, (100.0f64 - 9.0).sqrt());
    let rod_to_sphere = PI * 9.0 * 20.0 - (PI * 9.0 * shadow + cap(rad - shadow, rad));
    let half_chord = 3.0 * 91.0f64.sqrt() + 100.0 * dmath::asin(0.3);
    let side_cone = over_disc(2.0, 20.0, |y, z| 40.0 - ((30.0 - z) * (30.0 - z) - y * y).sqrt());
    vec![
        (
            "tilted-plane",
            Part {
                features: vec![
                    Feature::Plane(frame(Point3::new(0.0, 0.0, 20.0), tilt, Vec3::Y)),
                    extrude(
                        at(0.0, 0.0, 0.0),
                        rect(),
                        Extent::To {
                            target: Target::Plane(0),
                            offset: 0.0,
                        },
                        Combine::New,
                    ),
                ],
            },
            1200.0 - 0.3 * 6.0 * 50.0,
        ),
        (
            "tilted-plane-offset",
            Part {
                features: vec![
                    Feature::Plane(frame(Point3::new(0.0, 0.0, 20.0), tilt, Vec3::Y)),
                    extrude(
                        at(0.0, 0.0, 0.0),
                        rect(),
                        Extent::To {
                            target: Target::Plane(0),
                            offset: 2.0,
                        },
                        Combine::New,
                    ),
                ],
            },
            1110.0 + 60.0 * 2.0 * k,
        ),
        (
            "sphere-outside",
            Part {
                features: vec![
                    ball(Point3::new(0.0, 0.0, 20.0), 10.0, Combine::New),
                    extrude(
                        at(0.0, 0.0, 0.0),
                        disc(3.0),
                        to(0, wall(0, 0).with(0), 0.0),
                        Combine::New,
                    ),
                ],
            },
            rod_to_sphere,
        ),
        (
            "sphere-inside",
            Part {
                features: vec![
                    ball(Point3::new(0.0, 0.0, 20.0), 10.0, Combine::New),
                    extrude(
                        at(0.0, 0.0, 15.0),
                        disc(3.0),
                        to(0, wall(0, 0).with(1), 0.0),
                        Combine::New,
                    ),
                ],
            },
            PI * 9.0 * 5.0 + PI * 9.0 * shadow + cap(rad - shadow, rad),
        ),
        (
            // a rod lying along y at height 20, radius 10: a block below it rises to its underside
            "cylinder-outside",
            Part {
                features: vec![
                    extrude(
                        frame(Point3::new(0.0, -20.0, 20.0), Vec3::Y, Vec3::X),
                        disc(10.0),
                        Extent::Distance(40.0),
                        Combine::New,
                    ),
                    extrude(
                        at(-3.0, -2.0, 0.0),
                        Profile::rectangle(Point2::ORIGIN, 6.0, 4.0, 0),
                        to(0, wall(0, 0), 0.0),
                        Combine::New,
                    ),
                ],
            },
            4.0 * (120.0 - half_chord),
        ),
        (
            // a rod inside the cone's extension below it rises to the cone
            "cone-inside",
            Part {
                features: vec![
                    cone(),
                    extrude(
                        at(0.0, 0.0, 0.0),
                        disc(3.0),
                        to(0, wall(0, 0).with(0), 0.0),
                        Combine::New,
                    ),
                ],
            },
            PI * 9.0 * 30.0 - 2.0 * PI * 27.0 / 3.0,
        ),
        (
            // a rod along x at height 20 from x = -40 runs into the cone's side
            "cone-outside",
            Part {
                features: vec![
                    cone(),
                    extrude(
                        frame(Point3::new(-40.0, 0.0, 20.0), Vec3::X, Vec3::Y),
                        disc(2.0),
                        to(0, wall(0, 0).with(1), 0.0),
                        Combine::New,
                    ),
                ],
            },
            side_cone,
        ),
        (
            // a hole from the top of a slab down to a tilted plane under it
            "hole-tilted",
            Part {
                features: vec![
                    extrude(
                        at(0.0, 0.0, 0.0),
                        Profile::rectangle(Point2::ORIGIN, 40.0, 30.0, 0),
                        Extent::Distance(20.0),
                        Combine::New,
                    ),
                    Feature::Plane(frame(Point3::new(20.0, 15.0, 8.0), tilt, Vec3::Y)),
                    Feature::Hole(Box::new(Hole {
                        at: frame(Point3::new(20.0, 15.0, 20.0), -Vec3::Z, Vec3::X),
                        diameter: 6.0,
                        head: Head::Plain,
                        end: End::To {
                            target: Target::Plane(1),
                            offset: 0.0,
                        },
                        point: None,
                        thread: None,
                    })),
                ],
            },
            // the plane's height over the hole averages 8 (the hole is centred on it)
            40.0 * 30.0 * 20.0 - PI * 9.0 * 12.0,
        ),
        (
            // a post from the slab's top up to the next face: a floating ball's underside
            "to-next",
            Part {
                features: vec![
                    extrude(
                        at(0.0, 0.0, 0.0),
                        Profile::rectangle(Point2::new(-20.0, -15.0), 40.0, 30.0, 0),
                        Extent::Distance(10.0),
                        Combine::New,
                    ),
                    ball(Point3::new(0.0, 0.0, 30.0), 10.0, Combine::Add),
                    extrude(
                        at(0.0, 0.0, 10.0),
                        disc(3.0),
                        Extent::ToNext { reverse: false },
                        Combine::Add,
                    ),
                ],
            },
            12000.0 + 4.0 / 3.0 * PI * 1000.0 + rod_to_sphere,
        ),
        (
            // a pocket from the top down to the next face: the slab's floor, flat
            "to-next-floor",
            Part {
                features: vec![
                    extrude(
                        at(0.0, 0.0, 0.0),
                        Profile::rectangle(Point2::ORIGIN, 40.0, 30.0, 0),
                        Extent::Distance(10.0),
                        Combine::New,
                    ),
                    extrude(
                        at(0.0, 0.0, 10.0),
                        Profile::rectangle(Point2::new(5.0, 5.0), 10.0, 8.0, 0),
                        Extent::ToNext { reverse: true },
                        Combine::Cut,
                    ),
                ],
            },
            12000.0 - 800.0,
        ),
    ]
}

#[test]
fn up_to_surfaces_by_hand() {
    for (name, part, want) in parts() {
        let seq = part.regenerate().unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert!(check(&seq).is_empty(), "{name}: {:?}", check(&seq));
        let v = volume(&seq);
        assert!((v - want).abs() <= 1e-4 * want, "{name}: volume {v} want {want}");
        let nary = part
            .regenerate_nary(4)
            .unwrap_or_else(|e| panic!("{name} nary: {e:?}"));
        assert!(check(&nary).is_empty(), "{name} nary");
        assert_eq!(nary.faces.len(), seq.faces.len(), "{name}");
        assert!((volume(&nary) - v).abs() <= 1e-6 * v, "{name} nary volume");
    }
}

/// The priced pair: ends on surfaces, and the same ends flat.
#[test]
fn suite_up_to_parts() {
    for flat in [false, true] {
        let part = forge_model::suite::upto(flat);
        let seq = part.regenerate().expect("seq");
        let nary = part.regenerate_nary(4).expect("nary");
        assert!(check(&seq).is_empty() && check(&nary).is_empty(), "flat {flat}");
        assert_eq!(seq.faces.len(), nary.faces.len(), "flat {flat}");
        assert!(
            (volume(&seq) - volume(&nary)).abs() <= 1e-6 * volume(&seq),
            "flat {flat}"
        );
    }
}

#[test]
fn up_to_refusals() {
    let refused = |features: Vec<Feature>| Part { features }.regenerate().expect_err("refused");
    let bad = |feature, error| RegenError::Extent { feature, error };
    let disc = |r: f64| Profile::circle(Point2::ORIGIN, r, 0);
    let sphere = || ball(Point3::new(0.0, 0.0, 20.0), 10.0, Combine::New);
    // wider than the sphere: some of it misses
    assert_eq!(
        refused(vec![
            sphere(),
            extrude(
                at(0.0, 0.0, 0.0),
                disc(12.0),
                to(0, wall(0, 0).with(0), 0.0),
                Combine::New
            )
        ]),
        bad(1, ExtentError::NotCovered)
    );
    // the sketch cuts the sphere: part of it starts inside
    assert_eq!(
        refused(vec![
            sphere(),
            extrude(
                at(9.0, 0.0, 20.0),
                disc(3.0),
                to(0, wall(0, 0).with(0), 0.0),
                Combine::New
            )
        ]),
        bad(1, ExtentError::NotAhead)
    );
    // travel along a cylinder's axis never meets it
    assert_eq!(
        refused(vec![
            extrude(
                at(0.0, 0.0, 0.0),
                disc(10.0),
                Extent::Distance(40.0),
                Combine::New
            ),
            extrude(
                at(30.0, 0.0, 0.0),
                disc(2.0),
                to(0, wall(0, 0), 0.0),
                Combine::New
            ),
        ]),
        bad(1, ExtentError::NeverHits)
    );
    // a torus
    let ring = Feature::Revolve {
        profile: Profile {
            plane: frame(Point3::new(0.0, 0.0, 20.0), Vec3::Y, Vec3::X),
            loops: vec![Profile::circle(Point2::new(10.0, 0.0), 3.0, 0)],
        },
        axis: Axis2 {
            origin: Point2::ORIGIN,
            dir: Vec2::Y,
        },
        extent: Turn::Angle(dmath::TAU),
        op: Combine::New,
    };
    assert_eq!(
        refused(vec![
            ring,
            extrude(
                at(10.0, 0.0, 0.0),
                disc(1.0),
                to(0, wall(0, 0).with(0), 0.0),
                Combine::New
            )
        ]),
        bad(1, ExtentError::Unsupported("ending on a torus"))
    );
}
