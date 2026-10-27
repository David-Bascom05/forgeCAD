//! Parts that end in finishing features (chamfer, shell, draft), each with its volume worked
//! out by hand where the shape allows.

use crate::{Combine, Feature, Part};
use forge_math::{Frame, Point2, Point3, dmath};
use forge_ops::{ChamferSize, Profile, ProfileLoop};
use forge_topo::Name;

fn extrude(loops: Vec<ProfileLoop>, plane: Frame, d: f64, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile { plane, loops },
        distance: d,
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
    let _ = (edge, ChamferSize::Equal(1.0));
    out
}
