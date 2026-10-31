//! Parts that end in finishing features (chamfer, shell, draft), each with its volume worked
//! out by hand where the shape allows.

use crate::{Combine, Extent, Feature, Part};
use forge_math::{Frame, Point2, Point3, dmath};
use forge_ops::{ChamferSize, Profile, ProfileLoop};
use forge_topo::Name;

fn extrude(loops: Vec<ProfileLoop>, plane: Frame, d: f64, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile { plane, loops },
        extent: Extent::Distance(d),
        op,
    }
}

/// Face `role` of feature `i` (its position in the history).
fn face(i: usize, role: &str) -> Name {
    Name::feature(i as u64 + 1).with_str(role)
}

fn side(i: usize, k: u64) -> Name {
    face(i, "side").with(k)
}

/// The edge between two faces, as extrude names it.
fn edge(a: Name, b: Name) -> Name {
    Name::NONE.with_set(vec![a, b])
}

fn tan(a: f64) -> f64 {
    let (s, c) = dmath::sin_cos(a);
    s / c
}

/// A random plate (rounded or square corners, maybe holes) finished by a random mix of draft,
/// chamfer and shell, sizes from small to too large.
pub fn fuzz(seed: u64) -> Part {
    let mut r = forge_math::rng::Rng::new(seed);
    let (w, d) = (r.range(10.0, 60.0), r.range(10.0, 40.0));
    let h = r.range(3.0, 20.0);
    let rounded = r.bool();
    let mut loops = vec![if rounded {
        Profile::rounded_rectangle(Point2::ORIGIN, w, d, r.range(0.5, w.min(d) * 0.45), 0)
    } else {
        Profile::rectangle(Point2::ORIGIN, w, d, 0)
    }];
    let sides: u64 = if rounded { 8 } else { 4 };
    let holes = r.below(3) as u64;
    for k in 0..holes {
        let x = w * (k as f64 + 1.0) / (holes as f64 + 1.0);
        loops.push(Profile::circle(
            Point2::new(x, d / 2.0),
            r.range(0.5, (d / 2.0).min(w / (2.0 * holes as f64 + 2.0)) * 0.8),
            50 + k,
        ));
    }
    let walls: Vec<u64> = (0..sides).chain((0..holes).map(|k| 50 + k)).collect();
    let mut features = vec![extrude(loops, Frame::WORLD, h, Combine::New)];
    let (top, bottom) = (face(0, "end"), face(0, "start"));
    let size = |r: &mut forge_math::rng::Rng| r.range(0.1, 0.4) * h.min(d);
    for _ in 0..1 + r.below(2) {
        match r.below(3) {
            0 => {
                let pick: Vec<u64> = walls.iter().copied().filter(|_| r.bool()).collect();
                let cap = if r.bool() { top } else { bottom };
                features.push(Feature::Chamfer {
                    edges: pick.iter().map(|&k| edge(cap, side(0, k))).collect(),
                    size: ChamferSize::Equal(size(&mut r)),
                });
            }
            1 => features.push(Feature::Draft {
                faces: walls.iter().map(|&k| side(0, k)).collect(),
                neutral: Frame::WORLD.with_origin(Point3::new(0.0, 0.0, r.range(0.0, h))),
                angle: r.range(-0.15, 0.15),
            }),
            _ => features.push(Feature::Shell {
                thickness: size(&mut r) * 0.5,
                outward: r.below(4) == 0,
                open: if r.bool() { vec![top] } else { vec![] },
            }),
        }
    }
    Part { features }
}

/// A named part and its exact volume, when known.
pub fn parts() -> Vec<(&'static str, Part, Option<f64>)> {
    let pi = dmath::PI;
    let (a, b, h) = (40.0, 30.0, 20.0);
    let block = || {
        extrude(
            vec![Profile::rectangle(Point2::ORIGIN, a, b, 0)],
            Frame::WORLD,
            h,
            Combine::New,
        )
    };
    let rod = |r: f64| {
        extrude(
            vec![Profile::circle(Point2::ORIGIN, r, 0)],
            Frame::WORLD,
            h,
            Combine::New,
        )
    };
    let sides4: Vec<Name> = (0..4).map(|k| side(0, k)).collect();
    let k = tan(0.05);
    let mut out = vec![
        (
            "drafted-box",
            Part {
                features: vec![
                    block(),
                    Feature::Draft {
                        faces: sides4.clone(),
                        neutral: Frame::WORLD,
                        angle: 0.05,
                    },
                ],
            },
            Some(a * b * h - k * h * h * (a + b) + 4.0 / 3.0 * k * k * h * h * h),
        ),
        (
            "drafted-boss",
            Part {
                features: vec![
                    rod(10.0),
                    Feature::Draft {
                        faces: vec![side(0, 0)],
                        neutral: Frame::WORLD,
                        angle: 0.08,
                    },
                ],
            },
            {
                let (r0, r1) = (10.0, 10.0 - h * tan(0.08));
                Some(pi * h / 3.0 * (r0 * r0 + r0 * r1 + r1 * r1))
            },
        ),
        (
            "shelled-box",
            Part {
                features: vec![
                    block(),
                    Feature::Shell {
                        thickness: 2.0,
                        outward: false,
                        open: vec![face(0, "end")],
                    },
                ],
            },
            Some(a * b * h - (a - 4.0) * (b - 4.0) * (h - 2.0)),
        ),
        (
            "shelled-box-out",
            Part {
                features: vec![
                    block(),
                    Feature::Shell {
                        thickness: 1.5,
                        outward: true,
                        open: vec![face(0, "end")],
                    },
                ],
            },
            Some((a + 3.0) * (b + 3.0) * (h + 1.5) - a * b * h),
        ),
        (
            "shelled-rod",
            Part {
                features: vec![
                    rod(10.0),
                    Feature::Shell {
                        thickness: 1.0,
                        outward: false,
                        open: vec![face(0, "end")],
                    },
                ],
            },
            Some(pi * (100.0 * h - 81.0 * (h - 1.0))),
        ),
        (
            "hollow-ball",
            Part {
                features: vec![
                    Feature::Ball {
                        frame: Frame::WORLD,
                        radius: 10.0,
                        op: Combine::New,
                    },
                    Feature::Shell {
                        thickness: 1.0,
                        outward: false,
                        open: vec![],
                    },
                ],
            },
            Some(4.0 / 3.0 * pi * (1000.0 - 729.0)),
        ),
    ];
    // a drafted, shelled tray with a hole cut after: the n-ary regen runs on past the shell
    let tray = vec![
        extrude(
            vec![Profile::rounded_rectangle(Point2::ORIGIN, 60.0, 40.0, 6.0, 0)],
            Frame::WORLD,
            15.0,
            Combine::New,
        ),
        Feature::Draft {
            faces: (0..8).map(|k| side(0, k)).collect(),
            neutral: Frame::WORLD,
            angle: 0.04,
        },
        Feature::Shell {
            thickness: 1.5,
            outward: false,
            open: vec![face(0, "end")],
        },
        extrude(
            vec![Profile::circle(Point2::new(30.0, 20.0), 4.0, 0)],
            Frame::WORLD.with_origin(Point3::new(0.0, 0.0, -1.0)),
            4.0,
            Combine::Cut,
        ),
    ];
    out.push(("tray", Part { features: tray }, None));

    // corners rounder than the wall is thick vanish inside: the inner walls meet sharp
    let rr = 2.0;
    out.push((
        "shelled-rounded",
        Part {
            features: vec![
                extrude(
                    vec![Profile::rounded_rectangle(Point2::ORIGIN, a, b, rr, 0)],
                    Frame::WORLD,
                    h,
                    Combine::New,
                ),
                Feature::Shell {
                    thickness: 3.0,
                    outward: false,
                    open: vec![face(0, "end")],
                },
            ],
        },
        Some((a * b - (4.0 - pi) * rr * rr) * h - (a - 6.0) * (b - 6.0) * (h - 3.0)),
    ));
    // a hole narrower than the wall closes in the outer skin
    let t = 2.0;
    out.push((
        "shelled-hole-out",
        Part {
            features: vec![
                extrude(
                    vec![
                        Profile::rectangle(Point2::ORIGIN, a, b, 0),
                        Profile::circle(Point2::new(20.0, 15.0), 1.5, 50),
                    ],
                    Frame::WORLD,
                    h,
                    Combine::New,
                ),
                Feature::Shell {
                    thickness: t,
                    outward: true,
                    open: vec![],
                },
            ],
        },
        Some((a + 2.0 * t) * (b + 2.0 * t) * (h + 2.0 * t) - (a * b - pi * 2.25) * h),
    ));

    let top = face(0, "end");
    let tops: Vec<Name> = (0..4).map(|k| edge(top, side(0, k))).collect();
    let bottoms: Vec<Name> = (0..4).map(|k| edge(face(0, "start"), side(0, k))).collect();
    let uprights: Vec<Name> = (0..4).map(|k| edge(side(0, (k + 3) % 4), side(0, k))).collect();
    let chamfered = |edges: Vec<Name>, size: ChamferSize| Part {
        features: vec![block(), Feature::Chamfer { edges, size }],
    };
    let d = 2.0;
    out.push((
        "chamfer-edge",
        chamfered(tops[..1].to_vec(), ChamferSize::Equal(d)),
        Some(a * b * h - d * d / 2.0 * a),
    ));
    out.push((
        "chamfer-top",
        chamfered(tops.clone(), ChamferSize::Equal(d)),
        Some(a * b * h - d * d * (a + b) + 4.0 / 3.0 * d * d * d),
    ));
    let all: Vec<Name> = tops.iter().chain(&bottoms).chain(&uprights).copied().collect();
    out.push((
        "chamfer-all",
        chamfered(all, ChamferSize::Equal(d)),
        Some(a * b * h - 2.0 * d * d * (a + b + h) + 16.0 / 3.0 * d * d * d),
    ));
    out.push((
        "chamfer-two",
        chamfered(
            tops[..1].to_vec(),
            ChamferSize::Distances {
                face: top,
                d1: 1.0,
                d2: 3.0,
            },
        ),
        Some(a * b * h - 1.5 * a),
    ));
    let angle = dmath::PI / 6.0;
    let other = 2.0 * dmath::sin(angle) / dmath::sin(dmath::FRAC_PI_2 + angle);
    out.push((
        "chamfer-angle",
        chamfered(
            tops[..1].to_vec(),
            ChamferSize::DistanceAngle {
                face: top,
                d: 2.0,
                angle,
            },
        ),
        Some(a * b * h - other * a),
    ));
    out.push((
        "chamfer-mitre",
        chamfered(
            tops[..2].to_vec(),
            ChamferSize::Distances {
                face: top,
                d1: 1.0,
                d2: 2.5,
            },
        ),
        None,
    ));
    let (r, dc) = (10.0, 1.5);
    out.push((
        "chamfer-rod",
        Part {
            features: vec![
                rod(r),
                Feature::Chamfer {
                    edges: vec![edge(top, side(0, 0)), edge(face(0, "start"), side(0, 0))],
                    size: ChamferSize::Equal(dc),
                },
            ],
        },
        Some(pi * r * r * h - 2.0 * pi * (r - dc / 3.0) * dc * dc),
    ));
    // a rounded plate with a hole: the top's tangent chain and the hole's rim, then shelled
    let plate = || {
        extrude(
            vec![
                Profile::rounded_rectangle(Point2::ORIGIN, 50.0, 30.0, 5.0, 0),
                Profile::circle(Point2::new(25.0, 15.0), 4.0, 50),
            ],
            Frame::WORLD,
            10.0,
            Combine::New,
        )
    };
    let rims: Vec<Name> = (0..8).chain([50]).map(|k| edge(top, side(0, k))).collect();
    out.push((
        "chamfer-plate",
        Part {
            features: vec![
                plate(),
                Feature::Chamfer {
                    edges: rims.clone(),
                    size: ChamferSize::Equal(1.0),
                },
                Feature::Shell {
                    thickness: 1.0,
                    outward: false,
                    open: vec![face(0, "start")],
                },
            ],
        },
        None,
    ));
    // chamfered uprights, shelled open at the top, a boss added after
    out.push((
        "chamfer-box-shell",
        Part {
            features: vec![
                block(),
                Feature::Chamfer {
                    edges: uprights.clone(),
                    size: ChamferSize::Equal(4.0),
                },
                Feature::Shell {
                    thickness: 1.5,
                    outward: false,
                    open: vec![top],
                },
                extrude(
                    vec![Profile::circle(Point2::new(20.0, 15.0), 3.0, 0)],
                    Frame::WORLD,
                    12.0,
                    Combine::Add,
                ),
            ],
        },
        None,
    ));
    out
}
