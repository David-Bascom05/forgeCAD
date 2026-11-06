//! A segment's tube: a circle swept along its centreline. Lines and arcs carry the section rigidly,
//! which on them is the rotation-minimising frame (derive.py section 4), so the tube never twists;
//! its faces are exact cylinders and tori.

use forge_math::{Frame, Point2};
use forge_ops::Profile;
use forge_ops::sweep::{PathSeg, sweep_unchecked};
use forge_topo::Body;

fn start(p: &PathSeg) -> Option<Frame> {
    match *p {
        PathSeg::Line { a, b } => Some(Frame::from_z(a, (b - a).normalize()?)),
        PathSeg::Arc { circle, start, sweep } => {
            let (s, c) = forge_math::dmath::sin_cos(start);
            let x = circle.frame.x().get();
            let y = circle.frame.y().get();
            let at = circle.frame.origin + (x * c + y * s) * circle.radius;
            let t = (y * c - x * s) * sweep.signum();
            Some(Frame::from_z(at, t.normalize()?))
        }
        PathSeg::Spline(_) => None,
    }
}

fn swept(pieces: &[PathSeg], r: f64, feature: u64) -> Option<Body> {
    let plane = start(pieces.first()?)?;
    let profile = Profile {
        plane,
        loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
    };
    sweep_unchecked(&profile, pieces, feature).ok()
}

/// The tube of diameter `d` along `pieces`: one body, or where the path bends tighter than the
/// tube (a corner built under its minimum), one body per piece the sweep takes.
pub fn tube(pieces: &[PathSeg], d: f64, feature: u64) -> Vec<Body> {
    if let Some(b) = swept(pieces, d / 2.0, feature) {
        return vec![b];
    }
    pieces
        .iter()
        .filter_map(|p| swept(std::slice::from_ref(p), d / 2.0, feature))
        .collect()
}
