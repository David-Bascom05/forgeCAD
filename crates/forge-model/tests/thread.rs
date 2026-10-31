//! Modeled threads: valid solids whose grooves remove the thread form's volume, turn by turn.
use forge_math::{Frame, Point2, Point3, UnitVec3, Vec3, dmath};
use forge_mesh::{Quality, tessellate};
use forge_model::thread::groove_moment;
use forge_model::{
    Combine, End, Extent, Feature, Head, Hole, Part, ScrewThread, Thread, ThreadLength, ThreadOn, ThreadSpec,
};
use forge_ops::{Handed, Profile};

/// The part's volume, the same feature by feature, in one pass, and in one pass placing points
/// without the tools' analytic shapes (a thread's several tools cut together).
fn volume(part: &Part) -> f64 {
    let vs: Vec<f64> = [
        part.regenerate(),
        part.regenerate_nary(1),
        part.regenerate_nary_with(1, false),
    ]
    .into_iter()
    .map(|b| {
        let b = b.expect("regenerates");
        assert!(forge_topo::check(&b).is_empty(), "{:?}", forge_topo::check(&b));
        let m = tessellate(
            &b,
            &Quality {
                chord: 0.001,
                ..Quality::default()
            },
        )
        .expect("meshes");
        assert!(m.open_edges().is_empty(), "closed mesh");
        m.volume()
    })
    .collect();
    assert!(vs.iter().all(|v| (v - vs[0]).abs() <= 1e-6 * vs[0]), "{vs:?}");
    vs[0]
}

fn rod(r: f64, h: f64) -> Feature {
    Feature::Extrude {
        profile: Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
        },
        extent: Extent::Distance(h),
        op: Combine::New,
    }
}

fn thread(on: ThreadOn, length: f64, handed: Handed, chamfer: bool) -> Feature {
    Feature::Thread(Box::new(ScrewThread {
        on,
        length: ThreadLength::Distance(length),
        handed,
        chamfer,
        runout: false,
    }))
}

/// Down the rod from its top.
fn top(h: f64) -> Frame {
    Frame::from_zx(Point3::new(0.0, 0.0, h), -UnitVec3::Z, Vec3::new(0.6, 0.8, 0.0)).expect("axes")
}

/// Two more turns of thread remove two more turns of the form's groove: 2 x 2 pi x the
/// integral of r over the groove's section (the ends are the same, turned).
#[test]
fn external_thread_removes_the_form_per_turn() {
    let m6 = ThreadSpec::parse("M6").expect("coarse M6");
    assert_eq!(
        m6,
        ThreadSpec::Iso {
            major: 6.0,
            pitch: 1.0
        }
    );
    for handed in [Handed::Right, Handed::Left] {
        let at = |len: f64| Part {
            features: vec![
                rod(3.0, 20.0),
                thread(
                    ThreadOn::Axis {
                        at: top(20.0),
                        spec: m6,
                        internal: false,
                    },
                    len,
                    handed,
                    false,
                ),
            ],
        };
        let (v1, v2) = (volume(&at(6.0)), volume(&at(8.0)));
        let want = 2.0 * dmath::TAU * groove_moment(6.0, 1.0, false);
        let got = v1 - v2;
        assert!((got - want).abs() < 0.01 * want, "{handed:?}: {got} vs {want}");
    }
}

#[test]
fn internal_thread_removes_the_form_per_turn() {
    let spec = ThreadSpec::parse("1/4-20").expect("UTS");
    let (d, p) = (spec.major(), spec.pitch());
    let minor = d - 5.0 / 8.0 * 3f64.sqrt() * p;
    let block = Feature::Extrude {
        profile: Profile {
            plane: Frame::WORLD,
            loops: vec![
                Profile::rectangle(Point2::new(-8.0, -8.0), 16.0, 16.0, 0),
                Profile::circle(Point2::ORIGIN, minor / 2.0 + 0.01, 10),
            ],
        },
        extent: Extent::Distance(15.0),
        op: Combine::New,
    };
    let at = |turns: f64| Part {
        features: vec![
            block.clone(),
            thread(
                ThreadOn::Axis {
                    at: top(15.0),
                    spec,
                    internal: true,
                },
                turns * p,
                Handed::Right,
                false,
            ),
        ],
    };
    let (v1, v2) = (volume(&at(4.0)), volume(&at(6.0)));
    let r = minor / 2.0 + 0.01;
    // the groove's part outside the bore: the form's less the sliver the bore already took
    let sliver = {
        let (a, b) = (minor / 2.0, r);
        let w = |x: f64| p * 3.0 / 4.0 - 2.0 / 3f64.sqrt() * (x - a);
        // integral of x w(x) over [a, b], w linear: Simpson's rule is exact
        (b - a) / 6.0 * (a * w(a) + 4.0 * (a + b) / 2.0 * w((a + b) / 2.0) + b * w(b))
    };
    let want = 2.0 * dmath::TAU * (groove_moment(d, p, true) - sliver);
    let got = v1 - v2;
    assert!((got - want).abs() < 0.01 * want, "{got} vs {want}");
}

/// A hole's cosmetic thread made real: plain, with a chamfered start, a run-out, or both.
#[test]
fn a_cosmetic_thread_is_modeled() {
    let hole = Feature::Hole(Box::new(Hole {
        at: top(12.0),
        diameter: 6.8,
        head: Head::Plain,
        end: End::Depth(9.0),
        point: Some(118f64.to_radians()),
        thread: Some(Thread {
            size: 8.0,
            pitch: 1.25,
        }),
        standard: None,
        places: None,
    }));
    let block = Feature::Extrude {
        profile: Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::rectangle(Point2::new(-8.0, -8.0), 16.0, 16.0, 0)],
        },
        extent: Extent::Distance(12.0),
        op: Combine::New,
    };
    let plain = Part {
        features: vec![block.clone(), hole.clone()],
    };
    let v0 = volume(&plain);
    let mut vs = Vec::new();
    for (chamfer, runout) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut part = plain.clone();
        part.features.push(Feature::Thread(Box::new(ScrewThread {
            on: ThreadOn::Hole(1),
            length: ThreadLength::Full,
            handed: Handed::Right,
            chamfer,
            runout,
        })));
        let v = volume(&part);
        assert!(v < v0, "{chamfer} {runout}: {v} {v0}");
        vs.push(v);
    }
    // the chamfer and the run-out each take more
    assert!(
        vs[1] < vs[0] && vs[2] < vs[0] && vs[3] < vs[1] && vs[3] < vs[2],
        "{vs:?}"
    );
}

/// A run-out on a bolt ends its groove inside the rod, drawn out of it, left and right handed.
#[test]
fn a_run_out_draws_the_groove_out() {
    for handed in [Handed::Right, Handed::Left] {
        let part = |runout: bool| Part {
            features: vec![
                rod(3.0, 20.0),
                Feature::Thread(Box::new(ScrewThread {
                    on: ThreadOn::Axis {
                        at: top(20.0),
                        spec: ThreadSpec::parse("M6").expect("M6"),
                        internal: false,
                    },
                    length: ThreadLength::Distance(8.0),
                    handed,
                    chamfer: false,
                    runout,
                })),
            ],
        };
        let (plain, out) = (volume(&part(false)), volume(&part(true)));
        // at most a turn of the form more
        let turn = dmath::TAU * groove_moment(6.0, 1.0, false);
        assert!(
            out < plain && plain - out < turn,
            "{handed:?}: {plain} {out} {turn}"
        );
    }
}

/// A thread on a rod's cylinder face, and the same thread on its two patterned copies' holes.
#[test]
fn face_and_patterned_hole_threads() {
    let base = Part {
        features: vec![rod(3.0, 20.0)],
    };
    let body = base.regenerate().expect("rod");
    let side = body
        .face_ids()
        .find(|&f| matches!(body.face(f).surface, forge_geom::Surface::Cylinder(_)))
        .map(|f| body.face(f).name)
        .expect("a cylinder face");
    let on_face = |length: ThreadLength| {
        let mut p = base.clone();
        p.features.push(Feature::Thread(Box::new(ScrewThread {
            on: ThreadOn::Face {
                feature: 0,
                name: side,
                spec: ThreadSpec::parse("M6").expect("M6"),
                flip: false,
            },
            length,
            handed: Handed::Right,
            chamfer: true,
            runout: false,
        })));
        p
    };
    let (full, part) = (
        volume(&on_face(ThreadLength::Full)),
        volume(&on_face(ThreadLength::Distance(8.0))),
    );
    assert!(full < part && part < volume(&base), "{full} {part}");
    // a spec that doesn't fit the face is refused
    let mut bad = on_face(ThreadLength::Full);
    if let Feature::Thread(t) = &mut bad.features[1]
        && let ThreadOn::Face { spec, .. } = &mut t.on
    {
        *spec = ThreadSpec::parse("M8").expect("M8");
    }
    assert!(matches!(
        bad.regenerate(),
        Err(forge_model::RegenError::Thread {
            error: forge_model::ThreadError::BadSize,
            ..
        })
    ));

    let block = Feature::Extrude {
        profile: Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::rectangle(Point2::new(-8.0, -8.0), 40.0, 16.0, 0)],
        },
        extent: Extent::Distance(10.0),
        op: Combine::New,
    };
    let hole = Feature::Hole(Box::new(Hole {
        at: top(10.0),
        diameter: 5.0,
        head: Head::Plain,
        end: End::Through,
        point: None,
        thread: Some(Thread {
            size: 6.0,
            pitch: 1.0,
        }),
        standard: None,
        places: None,
    }));
    let pattern = Feature::Pattern {
        of: forge_model::Source::Features(vec![1]),
        layout: forge_model::Layout::Linear {
            first: forge_model::Row {
                dir: Vec3::X,
                count: 2,
                spacing: 20.0,
            },
            second: None,
        },
    };
    let modeled = Feature::Thread(Box::new(ScrewThread {
        on: ThreadOn::Hole(1),
        length: ThreadLength::Full,
        handed: Handed::Right,
        chamfer: false,
        runout: false,
    }));
    let one = volume(&Part {
        features: vec![block.clone(), hole.clone(), modeled.clone()],
    });
    let plain = volume(&Part {
        features: vec![block.clone(), hole.clone()],
    });
    let two = volume(&Part {
        features: vec![block, hole, pattern, modeled],
    });
    let per = plain - one;
    // the copy's bore is cut too, and threaded alike
    let bore = dmath::PI * 2.5 * 2.5 * 10.0;
    assert!(
        ((plain - bore - 2.0 * per) - two).abs() < 0.01 * per,
        "{plain} {one} {two}"
    );
}

/// A groove and its run-out cut in one pass, wherever the thread starts: from the rod's top or
/// inside it, axis down or up, from any start direction, of any length. The run-out's start
/// lies inside the groove, whose end cap's plane passes through the axis; the screw faces'
/// section lines in that plane once ran on through the axis and split the cap where nothing met it.
#[test]
fn run_outs_cut_in_one_pass() {
    let down = |z: f64, x: Vec3| Frame::from_zx(Point3::new(0.0, 0.0, z), -UnitVec3::Z, x).expect("frame");
    let up = |z: f64, x: Vec3| Frame::from_zx(Point3::new(0.0, 0.0, z), UnitVec3::Z, x).expect("frame");
    let slant = Vec3::new(0.6, 0.8, 0.0);
    for (at, length) in [
        (down(20.0, slant), 8.0),
        (down(20.0, Vec3::X), 6.0),
        (down(20.0, Vec3::X), 8.3),
        (down(14.0, slant), 8.0),
        (up(10.0, Vec3::X), 8.0),
    ] {
        let part = Part {
            features: vec![
                rod(3.0, 20.0),
                Feature::Thread(Box::new(ScrewThread {
                    on: ThreadOn::Axis {
                        at,
                        spec: ThreadSpec::parse("M6").expect("M6"),
                        internal: false,
                    },
                    length: ThreadLength::Distance(length),
                    handed: Handed::Right,
                    chamfer: false,
                    runout: true,
                })),
            ],
        };
        assert!(
            volume(&part)
                < volume(&Part {
                    features: vec![rod(3.0, 20.0)]
                })
        );
    }
}

/// Two overlapping coils united: an M6 groove section, and one 15/16 its size starting an eighth
/// of a turn before the first's end, inside it, and tapering out over a turn and an eighth. The
/// first's outer face is a cylinder's wall, whose section carried past its ends once reached its
/// next turn, so the second's helices were found twice.
#[test]
fn overlapping_coils_unite() {
    use forge_model::{Coil, CoilSize};
    use forge_ops::{Axis2, ProfileLoop, Segment};
    let coil = |plane: Frame, pts: &[Point2], turns: f64, taper: f64, op: Combine| {
        let n = pts.len();
        let lp = ProfileLoop::new(
            (0..n)
                .map(|k| {
                    (
                        Segment::Line {
                            a: pts[k],
                            b: pts[(k + 1) % n],
                        },
                        k as u64,
                    )
                })
                .collect(),
        );
        Feature::Coil(Box::new(Coil {
            profile: Profile {
                plane,
                loops: vec![lp],
            },
            axis: Axis2 {
                origin: Point2::ORIGIN,
                dir: forge_math::Vec2::Y,
            },
            size: CoilSize::PitchTurns { pitch: 1.0, turns },
            taper,
            handed: Handed::Right,
            op,
        }))
    };
    let rn = 3.0 - 5.0 / 16.0 * 3f64.sqrt();
    let r = 3.0 + 1.0 / 16.0;
    let w = 1.0 / 8.0 + (r - rn) / 3f64.sqrt();
    let g = [
        Point2::new(rn, 0.125),
        Point2::new(rn, -0.125),
        Point2::new(r, -w),
        Point2::new(r + 1.0, -w),
        Point2::new(r + 1.0, w),
        Point2::new(r, w),
    ];
    let c = Point2::new(g.iter().map(|q| q.x).sum::<f64>() / 6.0, 0.0);
    let small: Vec<Point2> = g.iter().map(|q| c + (*q - c) * (15.0 / 16.0)).collect();
    let plane = Frame::from_zx(Point3::new(0.0, 0.0, 10.0), -UnitVec3::Y, Vec3::X).expect("frame");
    let ang = 3.5 * dmath::TAU - dmath::TAU / 8.0;
    let (sn, cs) = dmath::sin_cos(ang);
    let x = Vec3::new(cs, sn, 0.0);
    let tail = Frame::from_zx(
        Point3::new(0.0, 0.0, 10.0 + ang / dmath::TAU),
        x.cross(Vec3::Z).normalize().expect("unit"),
        x,
    )
    .expect("frame");
    let taper = dmath::atan(0.9 / 1.125);
    let first = coil(plane, &g, 3.5, 0.0, Combine::New);
    let both = |op: Combine| Part {
        features: vec![first.clone(), coil(tail, &small, 1.125, taper, op)],
    };
    let (one, two) = (
        volume(&Part {
            features: vec![first.clone()],
        }),
        volume(&Part {
            features: vec![coil(tail, &small, 1.125, taper, Combine::New)],
        }),
    );
    let (union, common) = (volume(&both(Combine::Add)), volume(&both(Combine::Keep)));
    assert!(
        common > 0.0 && (union - (one + two - common)).abs() < 1e-3 * union,
        "{one} {two} {union} {common}"
    );
}
