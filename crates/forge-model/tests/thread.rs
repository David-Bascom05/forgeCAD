//! Modeled threads: valid solids whose grooves remove the thread form's volume, turn by turn.
use forge_math::{Frame, Point2, Point3, UnitVec3, Vec3, dmath};
use forge_mesh::{Quality, tessellate};
use forge_model::thread::groove_moment;
use forge_model::{
    Combine, End, Extent, Feature, Head, Hole, Part, ScrewThread, Thread, ThreadLength, ThreadOn, ThreadSpec,
};
use forge_ops::{Handed, Profile};

fn volume(part: &Part) -> f64 {
    let b = part.regenerate().expect("regenerates");
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
