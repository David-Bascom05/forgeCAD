//! Weldment members: a section carried along a line (an extrusion) or an arc (a revolve), run on
//! past its path's ends as far as its end cuts need and cut there by planes (half-spaces taken
//! away), each its own body. And the end trim of a member by other solids or other members'
//! envelopes, at one end only. Exact: lines, arcs and planes throughout; the corner rules that
//! place the cuts are forge-weld's.

use crate::bodies::{BodyError, bool_id, half};
use crate::{Combine, RegenError, Tool, part_shape};
use forge_bool::{Op, boolean};
use forge_geom::{Plane, Surface};
use forge_math::{Box3, Frame, Vec3};
use forge_ops::{Axis2, Profile};
use forge_topo::Body;

/// A member: `profile` on the plane square to the path at its start (z along the path).
#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub profile: Profile,
    pub path: MemberPath,
    /// How far the solid runs on before the path's start and past its end, for the cuts (a
    /// length on a line, an angle on an arc).
    pub reach: [f64; 2],
    /// The planes it is cut on: each frame's z points out of what is kept.
    pub cuts: Vec<Frame>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MemberPath {
    Line {
        length: f64,
    },
    /// About `axis` (in the profile's plane coordinates), turning `sweep` (> 0) the way the
    /// path runs.
    Arc {
        axis: Axis2,
        sweep: f64,
    },
}

/// An end trim: the member (the body feature `member` made) less, beyond `region`'s plane
/// (z towards the end trimmed), each tool.
#[derive(Clone, Debug, PartialEq)]
pub struct Trim {
    pub member: usize,
    pub region: Frame,
    pub tools: Vec<TrimTool>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TrimTool {
    /// A body, by the feature that made it.
    Body(usize),
    /// A member's envelope: its outline alone, carried along its path uncut.
    Envelope(Box<Member>),
}

/// The id a member's k-th cut names its face by.
pub fn cut_id(id: u64, k: usize) -> u64 {
    id ^ ((k as u64 + 1) << 52) ^ 0x5745_4c44
}

fn turned(f: &Frame, origin: forge_math::Point3, axis: Vec3, angle: f64) -> Option<Frame> {
    let rot = |v: Vec3| {
        let (s, c) = forge_math::dmath::sin_cos(angle);
        let k = axis;
        v * c + k.cross(v) * s + k * (k.dot(v) * (1.0 - c))
    };
    let o = origin + rot(f.origin - origin);
    Frame::from_axes(o, rot(f.x().into()), rot(f.y().into()), rot(f.z().into()))
}

/// The member's solid, uncut: its section along the path, run on by `reach`.
pub fn uncut(m: &Member, i: usize, id: u64) -> Result<Body, RegenError> {
    let pl = m.profile.plane;
    match m.path {
        MemberPath::Line { length } => {
            let z: Vec3 = pl.z().into();
            let p = Profile {
                plane: pl.with_origin(pl.origin - z * m.reach[0]),
                loops: m.profile.loops.clone(),
            };
            forge_ops::extrude(&p, length + m.reach[0] + m.reach[1], id)
                .map_err(|error| RegenError::Extrude { feature: i, error })
        }
        MemberPath::Arc { axis, sweep } => {
            let o = pl.plane_point(axis.origin);
            let x: Vec3 = pl.x().into();
            let y: Vec3 = pl.y().into();
            let dir = (x * axis.dir.x + y * axis.dir.y)
                .normalize()
                .ok_or(RegenError::Body {
                    feature: i,
                    error: BodyError::Unsupported("a member arc with no axis"),
                })?;
            // the revolve turns the profile about `axis` the right-hand way; the path's way is
            // where the plane's z points
            let probe = turned(&pl, o, dir.into(), 1e-3).expect("a frame");
            let sense = if (probe.origin - pl.origin).dot(pl.z().into()) >= 0.0 {
                1.0
            } else {
                -1.0
            };
            let (axis, dirv) = if sense > 0.0 {
                (axis, Vec3::from(dir))
            } else {
                (
                    Axis2 {
                        origin: axis.origin,
                        dir: -axis.dir,
                    },
                    -Vec3::from(dir),
                )
            };
            let back = turned(&pl, o, dirv, -m.reach[0]).expect("a frame");
            let p = Profile {
                plane: back,
                loops: m.profile.loops.clone(),
            };
            forge_ops::revolve(&p, axis, sweep + m.reach[0] + m.reach[1], id)
                .map_err(|error| RegenError::Revolve { feature: i, error })
        }
    }
}

fn bbox(b: &Body) -> Box3 {
    b.face_ids().fold(Box3::EMPTY, |acc, f| acc.hull(&b.face_box(f)))
}

/// Feature `i`'s member body: uncut, then each cut's half-space taken away.
pub fn body(m: &Member, i: usize, id: u64) -> Result<Body, RegenError> {
    let mut b = uncut(m, i, id)?;
    for (k, c) in m.cuts.iter().enumerate() {
        let (h, _) = half(
            &Surface::Plane(Plane { frame: *c }),
            &bbox(&b),
            (i, cut_id(id, k)),
        )?;
        b = boolean(&b, &h, Op::Subtract, bool_id(id, k as u64))
            .map_err(|error| RegenError::Boolean { feature: i, error })?;
        b = crate::not_empty(b, i)?;
    }
    Ok(b)
}

/// The member as a tool: a new body.
pub(crate) fn tool(m: &Member, i: usize, id: u64, build: bool) -> Result<Tool, RegenError> {
    let b = body(m, i, id)?;
    let sh = part_shape(&b);
    Ok((if build { b } else { Body::new() }, Combine::New, sh))
}

/// How many pieces a body is in.
pub fn pieces(b: &Body) -> usize {
    let mut c = b.compact();
    c.rebuild_shells();
    c.shells.iter().filter(|s| !s.faces.is_empty()).count()
}

/// The trim's result on `body`: each tool's part beyond the region plane taken away; refused
/// where that leaves the member in more than one piece.
pub fn trimmed(t: &Trim, body: &Body, tools: &[&Body], i: usize, id: u64) -> Result<Body, RegenError> {
    let mut b = body.clone();
    for (k, tool) in tools.iter().enumerate() {
        let (h, _) = half(
            &Surface::Plane(Plane { frame: t.region }),
            &bbox(tool).hull(&bbox(&b)),
            (i, cut_id(id, 100 + k)),
        )?;
        let piece = boolean(tool, &h, Op::Intersect, bool_id(id, 200 + k as u64))
            .map_err(|error| RegenError::Boolean { feature: i, error })?;
        if piece.face_ids().next().is_none() {
            continue;
        }
        b = boolean(&b, &piece, Op::Subtract, bool_id(id, k as u64))
            .map_err(|error| RegenError::Boolean { feature: i, error })?;
        b = crate::not_empty(b, i)?;
    }
    let n = pieces(&b);
    if n > 1 {
        return Err(RegenError::Body {
            feature: i,
            error: BodyError::Pieces { pieces: n },
        });
    }
    Ok(b)
}
