//! Role names of holes, threads and text: every face the role functions name is on the built
//! part (whole, or as pieces of it), so a language can refer to a hole's wall or a thread's
//! flanks by name, never by a hash read off a part.

use forge_math::{Frame, Point2, Point3, Vec3, dmath};
use forge_model::{
    Combine, End, Extent, Feature, Head, Hole, Part, ScrewThread, Thread, ThreadLength, ThreadOn, hole,
    thread,
};
use forge_ops::{Handed, Profile};
use forge_topo::{Body, Name};

fn block() -> Feature {
    Feature::Extrude {
        profile: Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::rectangle(Point2::new(-30.0, -20.0), 60.0, 40.0, 0)],
        },
        extent: Extent::Distance(20.0),
        op: Combine::New,
    }
}

fn down(x: f64) -> Frame {
    Frame::from_zx(Point3::new(x, 0.0, 20.0), -forge_math::UnitVec3::Z, Vec3::X).expect("axes")
}

fn has(b: &Body, n: Name) -> bool {
    b.face_ids().any(|f| {
        let m = b.face(f).name;
        m == n || m.is_piece_of(n)
    })
}

/// The part's faces after regenerating, ids each feature's place from 1.
fn built(features: Vec<Feature>) -> Body {
    Part { features }.regenerate().expect("builds")
}

#[test]
fn a_holes_faces_carry_its_role_names() {
    let blind = |x: f64, head: Head| {
        Feature::Hole(Box::new(Hole {
            at: down(x),
            diameter: 6.0,
            head,
            end: End::Depth(12.0),
            point: Some(118.0 / 180.0 * dmath::PI),
            thread: None,
            standard: None,
            places: None,
        }))
    };
    let b = built(vec![
        block(),
        blind(
            -12.0,
            Head::Counterbore {
                diameter: 11.0,
                depth: 4.0,
            },
        ),
        blind(
            12.0,
            Head::Countersink {
                diameter: 12.0,
                angle: dmath::PI / 2.0,
            },
        ),
    ]);
    let (cb, cs) = (Name::feature(2), Name::feature(3));
    for n in [
        hole::roles::bore(cb),
        hole::roles::seat(cb),
        hole::roles::wall(cb),
        hole::roles::bottom(cb),
        hole::roles::sink(cs),
        hole::roles::wall(cs),
        hole::roles::bottom(cs),
    ] {
        assert!(n.iter().any(|&h| has(&b, h)), "{n:?}");
    }
    // a plain hole has no seat
    assert!(!hole::roles::sink(cb).iter().any(|&h| has(&b, h)));
}

#[test]
fn a_threads_faces_carry_its_role_names() {
    let b = built(vec![
        block(),
        Feature::Hole(Box::new(Hole {
            at: down(0.0),
            diameter: 8.5,
            head: Head::Plain,
            end: End::Depth(16.0),
            point: None,
            thread: Some(Thread {
                size: 10.0,
                pitch: 1.5,
            }),
            standard: None,
            places: None,
        })),
        Feature::Thread(Box::new(ScrewThread {
            on: ThreadOn::Hole(1),
            length: ThreadLength::Distance(12.0),
            handed: Handed::Right,
            chamfer: true,
            runout: true,
        })),
    ]);
    assert!(has(&b, thread::roles::root_flat(3)));
    assert!(has(&b, thread::roles::runout_flat(3)));
    for n in thread::roles::flanks(3) {
        assert!(has(&b, n), "{n:?}");
    }
    assert!(thread::roles::chamfer(3).iter().any(|&n| has(&b, n)));
}

#[test]
fn texts_caps_carry_its_role_names() {
    let part = forge_model::suite::text();
    let b = part.regenerate().expect("builds");
    let mut seen = 0;
    for (i, f) in part.features.iter().enumerate() {
        let Feature::Text(t) = f else { continue };
        let id = i as u64 + 1;
        let n = match t.op {
            Combine::Add => forge_model::text::roles::end(id, 0),
            _ => forge_model::text::roles::start(id, 0),
        };
        assert!(has(&b, n), "text {i}");
        seen += 1;
    }
    assert!(seen > 1);
}
