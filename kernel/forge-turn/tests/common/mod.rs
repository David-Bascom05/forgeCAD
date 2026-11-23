//! Revolved test parts.
#![allow(dead_code)]
use forge_math::{Frame, Point2, Point3, Vec2, Vec3, dmath};
use forge_ops::profile::{Profile, ProfileLoop, Segment};
use forge_ops::revolve::{Axis2, revolve};

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
        loops: vec![ProfileLoop::new(segs.into_iter().enumerate().map(|(i, s)| (s, i as u64)).collect())],
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
