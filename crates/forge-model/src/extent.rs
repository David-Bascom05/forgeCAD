//! How far an extrude, revolve or hole goes: a distance or angle, both ways, through
//! everything, or up to a plane or an earlier feature's planar face. Each is worked out from
//! the feature's definition and the earlier features' tools, never from the part as built, so
//! the sequential and n-ary regens build the same tool.

use crate::pattern::corners;
use crate::{End, Feature, Hole, RegenError, hole, shape, tools};
use forge_bool::Shape;
use forge_geom::Surface;
use forge_math::{Frame, dmath, tol};
use forge_topo::Name;
use std::borrow::Cow;

/// How far an extrude goes along its sketch's normal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Extent {
    /// This far along the normal (negative: the other way).
    Distance(f64),
    /// This far in all, half each side of the sketch.
    Symmetric(f64),
    /// `forward` along the normal and `back` against it.
    TwoSided { forward: f64, back: f64 },
    /// Through everything the part can hold (cuts and intersections only).
    Through(Toward),
    /// Up to a plane parallel to the sketch, then `offset` on past it (negative: short of it).
    To { target: Target, offset: f64 },
}

/// How far a revolve turns about its axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Turn {
    /// This angle, counter-clockwise about the axis (a whole turn: 2pi).
    Angle(f64),
    /// This angle in all, half each side of the sketch.
    Symmetric(f64),
    TwoSided {
        forward: f64,
        back: f64,
    },
    /// Up to a plane through the axis, then `offset` radians on past it.
    To {
        target: Target,
        offset: f64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toward {
    Forward,
    Back,
    Both,
}

/// A plane to end at: an earlier work plane feature (its xy), or a planar face of an earlier
/// feature's tool (pattern copies included, by their instance names).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Plane(usize),
    Face { feature: usize, name: Name },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExtentError {
    /// A distance or angle that is not positive in all, or a whole turn exceeded.
    BadSize,
    /// Through everything, joined to the part: there is no end to it.
    ThroughJoins,
    /// The target is not an earlier work plane, or the face not among an earlier solid
    /// feature's.
    NoFace,
    /// The face is not planar.
    NotPlanar,
    /// The plane does not lie square to the travel (parallel to the sketch, or through a
    /// revolve's axis, or square to a hole), so the end would not be flat.
    NotSquare,
    /// The sketch already lies on the plane.
    AlreadyThere,
}

fn bad(feature: usize) -> impl Fn(ExtentError) -> RegenError {
    move |error| RegenError::Extent { feature, error }
}

/// The plane a target names.
fn plane(features: &[Feature], i: usize, t: &Target) -> Result<Frame, RegenError> {
    let (k, name) = match *t {
        Target::Plane(k) => {
            return match features.get(k) {
                Some(Feature::Plane(f)) if k < i => Ok(*f),
                _ => Err(bad(i)(ExtentError::NoFace)),
            };
        }
        Target::Face { feature, name } => (feature, name),
    };
    if k >= i || crate::modifies(&features[k]) {
        return Err(bad(i)(ExtentError::NoFace));
    }
    for (body, ..) in tools(features, k, true)? {
        if let Some(f) = body.face_by_name(name) {
            return match &body.face(f).surface {
                Surface::Plane(p) => Ok(p.frame),
                _ => Err(bad(i)(ExtentError::NotPlanar)),
            };
        }
    }
    Err(bad(i)(ExtentError::NoFace))
}

/// Where a plane parallel to `at`'s xy lies along its z, `offset` further on in the direction
/// of travel.
fn along(at: &Frame, p: &Frame, offset: f64, i: usize) -> Result<f64, RegenError> {
    if p.z().dot(at.z().get()).abs() < 1.0 - tol::ANGULAR {
        return Err(bad(i)(ExtentError::NotSquare));
    }
    let t = at.z().dot(p.origin - at.origin);
    if t.abs() <= tol::LINEAR {
        return Err(bad(i)(ExtentError::AlreadyThere));
    }
    let end = t + offset * t.signum();
    if end * t <= 0.0 {
        return Err(bad(i)(ExtentError::BadSize));
    }
    Ok(end)
}

fn positive(v: f64) -> bool {
    v.is_finite() && v > 0.0
}

/// Feature `i` (an extrude or revolve with any extent) as the plain extrude or revolve it
/// comes to: a distance from a moved sketch, or an angle from a turned one.
pub(crate) fn resolve(features: &[Feature], i: usize) -> Result<Cow<'_, Feature>, RegenError> {
    let err = bad(i);
    match &features[i] {
        Feature::Extrude {
            extent: Extent::Distance(_),
            ..
        }
        | Feature::Revolve {
            extent: Turn::Angle(_),
            ..
        } => Ok(Cow::Borrowed(&features[i])),
        Feature::Extrude { profile, extent, op } => {
            let at = profile.plane;
            let (lo, hi) = match *extent {
                Extent::Symmetric(d) => (-d / 2.0, d / 2.0),
                Extent::TwoSided { forward, back } => (-back, forward),
                Extent::Through(toward) => {
                    if matches!(op, crate::Combine::New | crate::Combine::Add) {
                        return Err(err(ExtentError::ThroughJoins));
                    }
                    let reach = hole::reach(&features[..i]);
                    let (mut lo, mut hi) = (0.0f64, 0.0f64);
                    if !reach.is_empty() {
                        for c in corners(&reach) {
                            let t = at.z().dot(c - at.origin);
                            (lo, hi) = (lo.min(t), hi.max(t));
                        }
                    }
                    match toward {
                        Toward::Forward => (0.0, hi + 1.0),
                        Toward::Back => (lo - 1.0, 0.0),
                        Toward::Both => (lo - 1.0, hi + 1.0),
                    }
                }
                Extent::To { target, offset } => {
                    let end = along(&at, &plane(features, i, &target)?, offset, i)?;
                    (end.min(0.0), end.max(0.0))
                }
                Extent::Distance(_) => unreachable!(),
            };
            if !(positive(hi - lo) && lo.is_finite() && hi.is_finite()) {
                return Err(err(ExtentError::BadSize));
            }
            let mut profile = profile.clone();
            profile.plane = at.with_origin(at.origin + at.z() * lo);
            Ok(Cow::Owned(Feature::Extrude {
                profile,
                extent: Extent::Distance(hi - lo),
                op: *op,
            }))
        }
        Feature::Revolve {
            profile,
            axis,
            extent,
            op,
        } => {
            // the axis frame the revolve turns in: z up the axis, x toward the profile
            let plain = Feature::Revolve {
                profile: profile.clone(),
                axis: *axis,
                extent: Turn::Angle(dmath::TAU),
                op: *op,
            };
            let Shape::Revolved { frame: rf, .. } = shape(&plain) else {
                unreachable!("a revolve's shape")
            };
            let (from, sweep) = match *extent {
                Turn::Symmetric(a) => (-a / 2.0, a),
                Turn::TwoSided { forward, back } => (-back, forward + back),
                Turn::To { target, offset } => {
                    let p = plane(features, i, &target)?;
                    let on = p.z().dot(rf.origin - p.origin).abs() <= tol::LINEAR;
                    if !on || p.z().dot(rf.z().get()).abs() > tol::ANGULAR {
                        return Err(err(ExtentError::NotSquare));
                    }
                    // the plane meets the turn at a and a + pi: the first one reached
                    let u = rf.z().cross(p.z());
                    let a = dmath::atan2(rf.y().dot(u), rf.x().dot(u)).rem_euclid(dmath::PI);
                    if a <= tol::ANGULAR || a >= dmath::PI - tol::ANGULAR {
                        return Err(err(ExtentError::AlreadyThere));
                    }
                    (0.0, a + offset)
                }
                Turn::Angle(_) => unreachable!(),
            };
            if !(positive(sweep) && sweep <= dmath::TAU + tol::ANGULAR && from.is_finite()) {
                return Err(err(ExtentError::BadSize));
            }
            let turn = rf.rotated_about_z(from).compose(&rf.inverse());
            let mut profile = profile.clone();
            profile.plane = turn.compose(&profile.plane);
            Ok(Cow::Owned(Feature::Revolve {
                profile,
                axis: *axis,
                extent: Turn::Angle(sweep),
                op: *op,
            }))
        }
        _ => unreachable!("only extrudes and revolves have extents"),
    }
}

/// Hole `i` with an end at a plane turned into one at a depth.
pub(crate) fn hole_end(features: &[Feature], i: usize, h: &Hole) -> Result<Hole, RegenError> {
    let End::To { target, offset } = h.end else {
        return Ok(*h);
    };
    let depth = along(&h.at, &plane(features, i, &target)?, offset, i)?;
    if depth < 0.0 {
        return Err(bad(i)(ExtentError::BadSize));
    }
    Ok(Hole {
        end: End::Depth(depth),
        ..*h
    })
}
