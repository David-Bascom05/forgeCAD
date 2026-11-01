//! Parts that end in ribs, offset or thickened faces and direct edits, each with its volume
//! worked out by hand where the shape allows, and a random mix of them.

use crate::{Combine, Edit, Extent, Feature, Motion, Part, Rib, Side, Target};
use forge_math::{Frame, Point2, Point3, Vec3, dmath};
use forge_ops::{ChamferSize, Profile, ProfileLoop, Radius, Segment};
use forge_topo::Name;

fn extrude(loops: Vec<ProfileLoop>, plane: Frame, d: f64, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile { plane, loops },
        extent: Extent::Distance(d),
        op,
    }
}

fn face(i: usize, role: &str) -> Name {
    Name::feature(i as u64 + 1).with_str(role)
}

fn side(i: usize, k: u64) -> Name {
    face(i, "side").with(k)
}

fn edge(a: Name, b: Name) -> Name {
    Name::NONE.with_set(&[a, b])
}

fn plane_at(o: Point3, n: Vec3) -> Frame {
    Frame::from_z(o, n.normalize().expect("non-zero normal"))
}

fn edit(e: Edit) -> Feature {
    Feature::Edit(e)
}

fn line(a: (f64, f64), b: (f64, f64)) -> Segment {
    Segment::Line {
        a: Point2::new(a.0, a.1),
        b: Point2::new(b.0, b.1),
    }
}

/// An L bracket: a floor 40 deep and a wall 40 high, both 5 thick and 30 wide.
fn bracket() -> Vec<Feature> {
    let xz = Frame::from_axes(Point3::ORIGIN, Vec3::X, Vec3::Z, Vec3::new(0.0, -1.0, 0.0)).expect("frame");
    vec![
        extrude(
            vec![Profile::rectangle(Point2::ORIGIN, 30.0, 40.0, 0)],
            Frame::WORLD,
            5.0,
            Combine::New,
        ),
        extrude(
            vec![Profile::rectangle(Point2::ORIGIN, 30.0, 40.0, 0)],
            xz,
            -5.0,
            Combine::Add,
        ),
    ]
}

/// The yz plane at x.
fn yz(x: f64) -> Frame {
    Frame::from_axes(Point3::new(x, 0.0, 0.0), Vec3::Y, Vec3::Z, Vec3::X).expect("frame")
}

fn gusset(draft: f64) -> Feature {
    edit(Edit::Rib(Box::new(Rib {
        plane: yz(15.0),
        path: vec![line((5.0, 25.0), (25.0, 5.0))],
        thickness: 2.0,
        side: Side::Both,
        web: false,
        flip: true,
        draft,
    })))
}

/// A random plate (maybe holes, maybe a chamfer or shell first) changed by a random mix of
/// offsets, moves, replacements, deletions, thickenings and ribs, sizes from small to too large.
pub fn fuzz(seed: u64) -> Part {
    let mut r = forge_math::rng::Rng::new(seed);
    let (w, d) = (r.range(10.0, 60.0), r.range(10.0, 40.0));
    let h = r.range(3.0, 20.0);
    let mut loops = vec![Profile::rectangle(Point2::ORIGIN, w, d, 0)];
    let holes = r.below(3) as u64;
    for k in 0..holes {
        let x = w * (k as f64 + 1.0) / (holes as f64 + 1.0);
        loops.push(Profile::circle(
            Point2::new(x, d / 2.0),
            r.range(0.5, (d / 2.0).min(w / (2.0 * holes as f64 + 2.0)) * 0.8),
            50 + k,
        ));
    }
    let mut features = vec![extrude(loops, Frame::WORLD, h, Combine::New)];
    let (top, bottom) = (face(0, "end"), face(0, "start"));
    let walls: Vec<Name> = (0..4).map(|k| side(0, k)).collect();
    let bores: Vec<Name> = (0..holes).map(|k| side(0, 50 + k)).collect();
    let mut chamfered = None;
    match r.below(4) {
        0 => {
            let k = r.below(4) as u64;
            features.push(Feature::Chamfer {
                edges: vec![edge(top, side(0, k))],
                size: ChamferSize::Equal(r.range(0.2, 0.4) * h.min(d)),
            });
            chamfered = Some(
                Name::feature(2)
                    .with_str("chamfer")
                    .with_name(edge(top, side(0, k))),
            );
        }
        1 => features.push(Feature::Shell {
            thickness: r.range(0.05, 0.25) * h.min(d),
            outward: false,
            open: vec![top],
        }),
        _ => {}
    }
    let shelled = matches!(features.last(), Some(Feature::Shell { .. }));
    let pick = |r: &mut forge_math::rng::Rng, from: &[Name]| -> Vec<Name> {
        let mut out: Vec<Name> = from.iter().copied().filter(|_| r.bool()).collect();
        if out.is_empty() && !from.is_empty() {
            out.push(from[r.below(from.len())]);
        }
        out
    };
    for _ in 0..1 + r.below(2) {
        let mut faces: Vec<Name> = if shelled { vec![bottom] } else { vec![top, bottom] };
        faces.extend(&walls);
        faces.extend(&bores);
        let size = r.range(-0.6, 0.6) * h;
        match r.below(if shelled { 6 } else { 5 }) {
            0 => features.push(edit(Edit::Offset {
                faces: pick(&mut r, &faces),
                distance: size,
            })),
            1 => {
                let motion = if r.bool() {
                    Motion::Translate(Vec3::new(r.range(-3.0, 3.0), r.range(-3.0, 3.0), size))
                } else {
                    Motion::Rotate {
                        origin: Point3::new(r.range(0.0, w), r.range(0.0, d), r.range(0.0, h)),
                        axis: Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0)),
                        angle: r.range(-0.3, 0.3),
                    }
                };
                let one = faces[r.below(faces.len())];
                features.push(edit(Edit::Move {
                    faces: vec![one],
                    motion,
                }));
            }
            2 => {
                let k = features.len();
                let n = Vec3::new(r.range(-0.3, 0.3), r.range(-0.3, 0.3), 1.0);
                features.push(Feature::Plane(plane_at(
                    Point3::new(w / 2.0, d / 2.0, h + size),
                    n,
                )));
                features.push(edit(Edit::Replace {
                    face: top,
                    with: Target::Plane(k),
                }));
            }
            3 => {
                let mut gone: Vec<Name> = pick(&mut r, &bores);
                if let Some(c) = chamfered.filter(|_| r.bool()) {
                    gone = vec![c];
                } else if gone.is_empty() || r.below(4) == 0 {
                    gone.push(faces[r.below(faces.len())]);
                }
                features.push(edit(Edit::Delete { faces: gone }));
            }
            4 => {
                let ops = [Combine::Add, Combine::Cut, Combine::Keep, Combine::New];
                let sides = [Side::Pos, Side::Neg, Side::Both];
                let on: Vec<Name> = if r.bool() {
                    vec![faces[0]]
                } else {
                    pick(&mut r, &bores)
                };
                features.push(edit(Edit::Thicken {
                    faces: if on.is_empty() { vec![bottom] } else { on },
                    thickness: size.abs().max(0.1),
                    side: sides[r.below(3)],
                    op: ops[r.below(4)],
                }));
            }
            _ => {
                let (y0, y1) = (r.range(0.1, 0.9) * d, r.range(0.1, 0.9) * d);
                let (x0, x1) = (r.range(0.1, 0.4) * w, r.range(0.6, 0.9) * w);
                let path = if r.bool() {
                    vec![line((x0, y0), (x1, y1))]
                } else {
                    let xm = (x0 + x1) / 2.0;
                    vec![line((x0, y0), (xm, y1)), line((xm, y1), (x1, y0))]
                };
                features.push(edit(Edit::Rib(Box::new(Rib {
                    plane: Frame::WORLD.with_origin(Point3::new(0.0, 0.0, h)),
                    path,
                    thickness: r.range(0.3, 3.0),
                    side: [Side::Pos, Side::Neg, Side::Both][r.below(3)],
                    web: true,
                    flip: false,
                    draft: if r.bool() { r.range(0.0, 0.1) } else { 0.0 },
                }))));
            }
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
    let holed = |rad: f64| {
        extrude(
            vec![
                Profile::rectangle(Point2::ORIGIN, a, b, 0),
                Profile::circle(Point2::new(20.0, 15.0), rad, 50),
            ],
            Frame::WORLD,
            h,
            Combine::New,
        )
    };
    let top = face(0, "end");
    let hole = side(0, 50);
    let part = |fs: Vec<Feature>| Part { features: fs };
    let with = |e: Edit| part(vec![block(), edit(e)]);
    let tan = |x: f64| dmath::sin(x) / dmath::cos(x);
    let th = 0.1;
    // the bracket and a gusset: a triangle 20 on the legs, 2 thick, wider by 2 s tan(draft) at s
    // from the path
    let l = 20.0;
    let gusseted = 30.0 * 40.0 * 5.0 * 2.0 - 30.0 * 25.0 + 2.0 * l * l / 2.0;
    let mut out = vec![
        (
            "offset-top",
            with(Edit::Offset {
                faces: vec![top],
                distance: 2.0,
            }),
            Some(a * b * (h + 2.0)),
        ),
        (
            "offset-sides",
            with(Edit::Offset {
                faces: (0..4).map(|k| side(0, k)).collect(),
                distance: -2.0,
            }),
            Some((a - 4.0) * (b - 4.0) * h),
        ),
        (
            "offset-hole",
            part(vec![
                holed(4.0),
                edit(Edit::Offset {
                    faces: vec![hole],
                    distance: 1.0,
                }),
            ]),
            Some(a * b * h - pi * 9.0 * h),
        ),
        (
            "move-top",
            with(Edit::Move {
                faces: vec![top],
                motion: Motion::Translate(Vec3::new(0.0, 0.0, -5.0)),
            }),
            Some(a * b * (h - 5.0)),
        ),
        (
            "move-side-tilt",
            with(Edit::Move {
                faces: vec![side(0, 1)],
                motion: Motion::Rotate {
                    origin: Point3::new(a, 0.0, 0.0),
                    axis: Vec3::Y,
                    angle: th,
                },
            }),
            Some(a * b * h + b * h * h * tan(th) / 2.0),
        ),
        (
            "move-hole",
            part(vec![
                holed(4.0),
                edit(Edit::Move {
                    faces: vec![hole],
                    motion: Motion::Translate(Vec3::new(6.0, -3.0, 0.0)),
                }),
            ]),
            Some(a * b * h - pi * 16.0 * h),
        ),
        (
            "replace-top",
            part(vec![
                block(),
                Feature::Plane(Frame::WORLD.with_origin(Point3::new(0.0, 0.0, 25.0))),
                edit(Edit::Replace {
                    face: top,
                    with: Target::Plane(1),
                }),
            ]),
            Some(a * b * 25.0),
        ),
        (
            "replace-top-slant",
            part(vec![
                block(),
                Feature::Plane(plane_at(Point3::new(0.0, 0.0, h), Vec3::new(-0.1, 0.0, 1.0))),
                edit(Edit::Replace {
                    face: top,
                    with: Target::Plane(1),
                }),
            ]),
            Some(b * (h * a + 0.05 * a * a)),
        ),
        (
            "replace-side-by-face",
            part(vec![
                block(),
                extrude(
                    vec![Profile::rectangle(Point2::new(50.0, 0.0), 10.0, 10.0, 0)],
                    Frame::WORLD,
                    5.0,
                    Combine::Cut,
                ),
                edit(Edit::Replace {
                    face: side(0, 1),
                    with: Target::Face {
                        feature: 1,
                        name: side(1, 3),
                    },
                }),
            ]),
            Some(50.0 * b * h),
        ),
        (
            "delete-hole",
            part(vec![holed(4.0), edit(Edit::Delete { faces: vec![hole] })]),
            Some(a * b * h),
        ),
        (
            "delete-boss",
            part(vec![
                block(),
                extrude(
                    vec![Profile::circle(Point2::new(20.0, 15.0), 5.0, 0)],
                    Frame::WORLD.with_origin(Point3::new(0.0, 0.0, h)),
                    6.0,
                    Combine::Add,
                ),
                edit(Edit::Delete {
                    faces: vec![side(1, 0), face(1, "end")],
                }),
            ]),
            Some(a * b * h),
        ),
        (
            "delete-pocket",
            part(vec![
                block(),
                extrude(
                    vec![Profile::rectangle(Point2::new(10.0, 10.0), 15.0, 8.0, 0)],
                    Frame::WORLD.with_origin(Point3::new(0.0, 0.0, h - 5.0)),
                    10.0,
                    Combine::Cut,
                ),
                edit(Edit::Delete {
                    // the walls were cut down to the top: each keeps its name, one piece kept
                    faces: (0..4).map(|k| side(1, k)).chain([face(1, "start")]).collect(),
                }),
            ]),
            Some(a * b * h),
        ),
        (
            "delete-chamfer",
            part(vec![
                block(),
                Feature::Chamfer {
                    edges: vec![edge(top, side(0, 0))],
                    size: ChamferSize::Equal(3.0),
                },
                edit(Edit::Delete {
                    faces: vec![
                        Name::feature(2)
                            .with_str("chamfer")
                            .with_name(edge(top, side(0, 0))),
                    ],
                }),
            ]),
            Some(a * b * h),
        ),
        (
            "delete-fillet",
            part(vec![
                block(),
                Feature::Fillet {
                    only: false,
                    edges: vec![(edge(top, side(0, 1)), Radius::Constant(4.0))],
                    starts: vec![],
                    setbacks: vec![],
                },
                edit(Edit::Delete {
                    faces: vec![
                        Name::feature(2)
                            .with_str("fillet")
                            .with_name(edge(top, side(0, 1))),
                    ],
                }),
            ]),
            Some(a * b * h),
        ),
        (
            "thicken-top",
            with(Edit::Thicken {
                faces: vec![top],
                thickness: 3.0,
                side: Side::Pos,
                op: Combine::Add,
            }),
            Some(a * b * (h + 3.0)),
        ),
        (
            "thicken-new",
            with(Edit::Thicken {
                faces: vec![top],
                thickness: 2.0,
                side: Side::Both,
                op: Combine::New,
            }),
            Some(a * b * 2.0),
        ),
        (
            "thicken-hole-cut",
            part(vec![
                holed(4.0),
                edit(Edit::Thicken {
                    faces: vec![hole],
                    thickness: 1.0,
                    side: Side::Neg,
                    op: Combine::Cut,
                }),
            ]),
            Some(a * b * h - pi * 25.0 * h),
        ),
        (
            "gusset",
            part([bracket(), vec![gusset(0.0)]].concat()),
            Some(gusseted),
        ),
        (
            "gusset-drafted",
            part([bracket(), vec![gusset(0.03)]].concat()),
            Some(gusseted + tan(0.03) * l * l * l * 2.0f64.sqrt() / 6.0),
        ),
    ];

    // a web across an open box, from the rim to the floor and wall to wall
    let tray = |path: Vec<Segment>, draft: f64| {
        part(vec![
            block(),
            Feature::Shell {
                thickness: 2.0,
                outward: false,
                open: vec![top],
            },
            edit(Edit::Rib(Box::new(Rib {
                plane: Frame::WORLD.with_origin(Point3::new(0.0, 0.0, h)),
                path,
                thickness: 2.0,
                side: Side::Both,
                web: true,
                flip: false,
                draft,
            }))),
        ])
    };
    let shelled = a * b * h - (a - 4.0) * (b - 4.0) * (h - 2.0);
    let straight = || vec![line((10.0, 15.0), (30.0, 15.0))];
    out.push(("web", tray(straight(), 0.0), Some(shelled + 36.0 * 2.0 * 18.0)));
    out.push((
        "web-drafted",
        tray(straight(), 0.05),
        Some(shelled + 36.0 * (2.0 * 18.0 + tan(0.05) * 18.0 * 18.0)),
    ));
    // a half ring of radius 8 about the middle, run on along its tangents to the long walls
    let arc = Segment::Arc {
        center: Point2::new(20.0, 15.0),
        radius: 8.0,
        start: 0.0,
        sweep: pi,
    };
    out.push((
        "web-arc",
        tray(vec![arc], 0.0),
        Some(shelled + 18.0 * (2.0 * 2.0 * 13.0 + pi / 2.0 * (81.0 - 49.0))),
    ));
    out
}
