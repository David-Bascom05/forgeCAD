//! Revolved test parts.
#![allow(dead_code)]
use forge_math::{Frame, Point2, Point3, Vec2, Vec3, dmath};
use forge_ops::profile::{Profile, ProfileLoop, Segment};
use forge_ops::revolve::{Axis2, revolve};
use forge_turn::insert::builtin;
use forge_turn::ops::{self, Chuck, Setup};
use forge_turn::profile::half_profile;

fn xz() -> Frame {
    Frame::from_axes(Point3::new(0.0, 0.0, 0.0), Vec3::X, Vec3::Z, -Vec3::Y).unwrap()
}

/// A revolved part from its (ρ, z) outline: lines, and arcs given by (centre, radius, start, sweep).
pub fn part(pts: &[(f64, f64)]) -> forge_topo::Body {
    let n = pts.len();
    let segs = (0..n)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            (
                Segment::Line {
                    a: Point2::new(a.0, a.1),
                    b: Point2::new(b.0, b.1),
                },
                i as u64,
            )
        })
        .collect();
    let pr = Profile {
        plane: xz(),
        loops: vec![ProfileLoop::new(segs)],
    };
    revolve(
        &pr,
        Axis2 {
            origin: Point2::ORIGIN,
            dir: Vec2::Y,
        },
        dmath::TAU,
        1,
    )
    .unwrap()
}

/// A revolved part from raw segments (lines and arcs) in (ρ, z).
pub fn part_segs(segs: Vec<Segment>) -> forge_topo::Body {
    let pr = Profile {
        plane: xz(),
        loops: vec![ProfileLoop::new(
            segs.into_iter().enumerate().map(|(i, s)| (s, i as u64)).collect(),
        )],
    };
    revolve(
        &pr,
        Axis2 {
            origin: Point2::ORIGIN,
            dir: Vec2::Y,
        },
        dmath::TAU,
        1,
    )
    .unwrap()
}

pub fn line(a: (f64, f64), b: (f64, f64)) -> Segment {
    Segment::Line {
        a: Point2::new(a.0, a.1),
        b: Point2::new(b.0, b.1),
    }
}

pub fn arc(c: (f64, f64), r: f64, start: f64, sweep: f64) -> Segment {
    Segment::Arc {
        center: Point2::new(c.0, c.1),
        radius: r,
        start,
        sweep,
    }
}

pub fn setup() -> Setup {
    // a shaft: radius 10 from z = -40 to -1, a 1 mm chamfer to the front face at z = 0, a
    // shoulder to radius 15 from z = -40 to -55, its back face at -55
    let b = part(&[
        (0.0, -55.0),
        (15.0, -55.0),
        (15.0, -40.0),
        (10.0, -40.0),
        (10.0, -1.0),
        (9.0, 0.0),
        (0.0, 0.0),
    ]);
    Setup {
        part: half_profile(&b).unwrap(),
        bar_r: 16.0,
        bar_front: 2.0,
        chuck: Chuck {
            z_face: -70.0,
            grip: 30.0,
            jaw_height: 15.0,
            body_r: 80.0,
            body_len: 60.0,
        },
        fixture_clearance: 1.0,
        holder_clearance: 0.25,
        rapid_clearance: 0.2,
        tol: 0.002,
        machine: None,
        place: None,
    }
}

pub fn job(s: &Setup) -> Vec<ops::OpPath> {
    let rough_t = builtin("od-rough-cnmg").unwrap();
    let fin_t = builtin("od-finish-dnmg").unwrap();
    let part_t = builtin("part-3").unwrap();
    let thr_t = builtin("thread-60").unwrap();
    vec![
        ops::face(s, &rough_t, 1, 0.0, 1.0).unwrap(),
        ops::rough(s, &rough_t, 1, 0.3, 2.0, -56.0, 0.0).unwrap(),
        ops::finish(s, &fin_t, 2, 0.0, -55.0).unwrap(),
        ops::thread(s, &thr_t, 4, 20.0, 1.5, 3.0, -30.0, 8).unwrap(),
        ops::part_off(s, &part_t, 5, -55.0).unwrap(),
    ]
}

/// A threaded bush (tests/id.rs, examples/machined/nut.fg): Ø50, an M30 x 1.5 internal thread
/// in a bore at its minor diameter, a Ø32 relief groove from z = -19 to -22, 30 long.
pub fn nut() -> Setup {
    let r1 = 15.0 - 0.625 * 1.5 * 0.75f64.sqrt();
    let b = part(&[
        (r1, -30.0),
        (25.0, -30.0),
        (25.0, 0.0),
        (r1, 0.0),
        (r1, -19.0),
        (16.0, -19.0),
        (16.0, -22.0),
        (r1, -22.0),
    ]);
    Setup {
        part: half_profile(&b).unwrap(),
        bar_r: 26.0,
        bar_front: 1.0,
        chuck: Chuck {
            z_face: -45.0,
            grip: 30.0,
            jaw_height: 15.0,
            body_r: 90.0,
            body_len: 60.0,
        },
        fixture_clearance: 1.0,
        holder_clearance: 0.25,
        rapid_clearance: 0.2,
        tol: 0.002,
        machine: None,
        place: None,
    }
}

pub fn nut_job(s: &Setup) -> Vec<ops::OpPath> {
    let bore_t = builtin("id-bore-ccmt").unwrap();
    vec![
        ops::face(s, &builtin("od-rough-cnmg").unwrap(), 1, 0.0, 1.0).unwrap(),
        ops::drill(s, &builtin("drill-26").unwrap(), 6, -40.0, 0.0).unwrap(),
        ops::bore_rough(s, &bore_t, 3, 0.2, 1.0, -31.0, 0.0, 13.0).unwrap(),
        ops::finish(s, &bore_t, 3, 0.0, -18.5).unwrap(),
        ops::id_groove(s, &builtin("id-groove-2").unwrap(), 7, -22.0, -19.0, 16.0, 14.1).unwrap(),
        ops::id_thread(s, &builtin("id-thread-60").unwrap(), 8, 30.0, 1.5, 3.0, -20.5, 8).unwrap(),
    ]
}
