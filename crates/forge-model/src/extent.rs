//! How far an extrude, revolve or hole goes: a distance or angle, both ways, through
//! everything, up to a plane or an earlier feature's face, or up to the next face. Each is
//! worked out from the feature's definition and the earlier features' tools, never from the
//! part as built, so the sequential and n-ary regens build the same tool. An end that is not a
//! plane square to the travel is cut off on its surface (upto.rs).

use crate::pattern::corners;
use crate::upto::{Plan, next_face, plan, samples};
use crate::{End, Feature, Head, Hole, RegenError, hole, shape, tools};
use forge_bool::{Op, Shape};
use forge_geom::{Plane, Surface};
use forge_math::{Frame, Point2, Point3, UnitVec3, dmath, tol};
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
    /// Up to a plane or a face's surface (extended past the face), then `offset` on past it
    /// (negative: short of it). A plane square to the travel gives a flat end; any other
    /// plane, a sphere, a cylinder or a cone an end on that surface.
    To { target: Target, offset: f64 },
    /// Up to the first face of the part the sketch's rays meet, along its normal (`reverse`:
    /// against it).
    ToNext { reverse: bool },
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

/// A plane to end at: an earlier work plane feature (its xy), or a face of an earlier feature's
/// tool (pattern copies included, by their instance names), its surface extended.
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
    /// The face is not planar (a revolve ends only at a plane).
    NotPlanar,
    /// The plane does not lie square to the travel (parallel to the sketch, or through a
    /// revolve's axis, or square to a hole), so the end would not be flat.
    NotSquare,
    /// The sketch already lies on the plane.
    AlreadyThere,
    /// The travel never meets the surface (it runs along it).
    NeverHits,
    /// The surface is not wholly ahead of the sketch: it crosses it, or lies behind.
    NotAhead,
    /// Some of the profile's rays miss the surface.
    NotCovered,
    /// No face lies ahead of the sketch.
    NoNextFace,
    /// The sketch's rays meet different faces first, or meet one at an edge.
    Ambiguous,
    Unsupported(&'static str),
}

/// What picking the next face found: whether the sketch starts in the part's material, and the
/// earlier tools' solids it looked at (each with its feature and how it combines), for proving
/// the pick (`upto::verify_next`).
#[derive(Clone, Debug)]
pub(crate) struct Next {
    pub material: bool,
    /// The surface picked.
    pub surface: Surface,
    pub tools: Vec<(usize, Shape, Op)>,
}

/// What an extrude or hole comes to: a plain one, or a sweep cut off at a surface.
pub(crate) enum Resolved<'a> {
    /// With what the next face's pick found, up to the next face.
    Plain(Cow<'a, Feature>, Option<Next>),
    /// The sweep (a plain extrude) and where it is cut off.
    UpTo(Feature, Box<UpTo>),
}

/// A sweep (its sketch faces `start`, its far end on the plane through `far` square to it), cut
/// off as `plan` says.
pub(crate) struct UpTo {
    pub plan: Plan,
    /// Up to the next face: what the pick found.
    pub next: Option<Next>,
    pub start: Vec<Name>,
    pub far: (Point3, UnitVec3),
}

fn bad(feature: usize) -> impl Fn(ExtentError) -> RegenError {
    move |error| RegenError::Extent { feature, error }
}

/// The surface a target names.
fn surface(features: &[Feature], i: usize, t: &Target) -> Result<Surface, RegenError> {
    let (k, name) = match *t {
        Target::Plane(k) => {
            return match features.get(k) {
                Some(Feature::Plane(f)) if k < i => Ok(Surface::Plane(Plane { frame: *f })),
                _ => Err(bad(i)(ExtentError::NoFace)),
            };
        }
        Target::Face { feature, name } => (feature, name),
    };
    if k >= i || crate::modifies(&features[k]) {
        return Err(bad(i)(ExtentError::NoFace));
    }
    // from the feature's definition where it can be, else from its built tool
    let plain = match &features[k] {
        Feature::Extrude { .. } | Feature::Revolve { .. } => match resolve(features, k)? {
            Resolved::Plain(f, _) => Some(f.into_owned()),
            Resolved::UpTo(..) => None,
        },
        Feature::Ball { .. } => Some(features[k].clone()),
        _ => None,
    };
    if let Some(s) = plain.and_then(|f| crate::upto::named_surface(&f, k, name)) {
        return Ok(s);
    }
    for (body, ..) in tools(features, k, true)? {
        if let Some(f) = body.face_by_name(name) {
            return Ok(body.face(f).surface.clone());
        }
    }
    Err(bad(i)(ExtentError::NoFace))
}

/// The plane a target names.
fn plane(features: &[Feature], i: usize, t: &Target) -> Result<Frame, RegenError> {
    match surface(features, i, t)? {
        Surface::Plane(p) => Ok(p.frame),
        _ => Err(bad(i)(ExtentError::NotPlanar)),
    }
}

/// A plane square to `at`'s z, or None.
fn square(s: &Surface, at: &Frame) -> Option<Frame> {
    match s {
        Surface::Plane(p) if p.frame.z().dot(at.z().get()).abs() >= 1.0 - tol::ANGULAR => Some(p.frame),
        _ => None,
    }
}

/// Which way along `z` from `p` the surface lies: +1 or -1.
fn toward(s: &Surface, p: Point3, z: UnitVec3) -> f64 {
    let at = match s {
        Surface::Plane(q) => {
            let n = q.frame.z();
            let t = n.dot(q.frame.origin - p) / n.dot(z.get());
            return if t < 0.0 { -1.0 } else { 1.0 };
        }
        Surface::Cylinder(c) => {
            let off = c.frame.origin - p;
            let a = c.frame.z();
            let across = off - a * a.dot(off);
            return if across.dot(z.get()) < 0.0 && across.length() > c.radius {
                -1.0
            } else {
                1.0
            };
        }
        Surface::Sphere(c) => c.frame.origin,
        Surface::Cone(c) => c.apex(),
        Surface::Torus(c) => c.frame.origin,
        Surface::Procedural(_) => return 1.0,
    };
    if z.dot(at - p) < 0.0 { -1.0 } else { 1.0 }
}

/// The solids of the tools before feature `i`, with how each combines (for finding the next
/// face): from the last new solid, with no part-changing feature since.
fn earlier(features: &[Feature], i: usize) -> Result<Vec<(usize, Shape, Op)>, RegenError> {
    let bad = bad(i);
    let start = features[..i]
        .iter()
        .rposition(crate::starts)
        .ok_or(bad(ExtentError::NoNextFace))?;
    if features[start..i].iter().any(crate::modifies) {
        return Err(bad(ExtentError::Unsupported(
            "the next face after a part-changing feature",
        )));
    }
    let mut out = Vec::new();
    for k in start..i {
        out.extend(
            tools(features, k, false)?
                .into_iter()
                .map(|t| (k, t.2, crate::op_of(t.1))),
        );
    }
    Ok(out)
}

/// The one surface every ray from `from` along `d` meets first, and whether they start in the
/// part's material (the pick; `upto::verify_next` proves it for the whole sweep).
fn next(features: &[Feature], i: usize, from: &[Point3], d: UnitVec3) -> Result<(Surface, Next), RegenError> {
    let all = earlier(features, i)?;
    let tools: Vec<(Shape, Op)> = all.iter().map(|t| (t.1.clone(), t.2)).collect();
    let mut found: Option<(Surface, bool)> = None;
    for &p in from {
        let s = next_face(&tools, p, d.get()).map_err(bad(i))?;
        if found.as_ref().is_some_and(|f| *f != s) {
            return Err(bad(i)(ExtentError::Ambiguous));
        }
        found = Some(s);
    }
    let (s, material) = found.ok_or(bad(i)(ExtentError::NoNextFace))?;
    Ok((
        s.clone(),
        Next {
            material,
            surface: s,
            tools: all,
        },
    ))
}

/// The four corners of a profile's box, on its plane.
fn foot(profile: &forge_ops::Profile) -> Vec<Point3> {
    let (mut lo, mut hi) = (
        Point2::new(f64::INFINITY, f64::INFINITY),
        Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for q in hole::profile_points(profile) {
        lo = Point2::new(lo.x.min(q.x), lo.y.min(q.y));
        hi = Point2::new(hi.x.max(q.x), hi.y.max(q.y));
    }
    [(lo.x, lo.y), (hi.x, lo.y), (lo.x, hi.y), (hi.x, hi.y)]
        .map(|(x, y)| profile.plane.plane_point(Point2::new(x, y)))
        .to_vec()
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
pub(crate) fn resolve(features: &[Feature], i: usize) -> Result<Resolved<'_>, RegenError> {
    let err = bad(i);
    match &features[i] {
        Feature::Extrude {
            extent: Extent::Distance(_),
            ..
        }
        | Feature::Revolve {
            extent: Turn::Angle(_),
            ..
        } => Ok(Resolved::Plain(Cow::Borrowed(&features[i]), None)),
        Feature::Extrude { profile, extent, op } => {
            let at = profile.plane;
            let mut material = None;
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
                    let s = surface(features, i, &target)?;
                    match square(&s, &at) {
                        Some(p) => {
                            let end = along(&at, &p, offset, i)?;
                            (end.min(0.0), end.max(0.0))
                        }
                        None => {
                            return up_to(i, profile, *op, &s, offset, None)
                                .map(|(f, u)| Resolved::UpTo(f, Box::new(u)));
                        }
                    }
                }
                Extent::ToNext { reverse } => {
                    let z = if reverse { -at.z() } else { at.z() };
                    let rays: Vec<Point3> = samples(profile).into_iter().map(|q| at.plane_point(q)).collect();
                    let (s, m) = next(features, i, &rays, z)?;
                    material = Some(m);
                    if let Some(p) = square(&s, &at) {
                        let end = along(&at, &p, 0.0, i)?;
                        if end * z.dot(at.z().get()) <= 0.0 {
                            return Err(err(ExtentError::NotAhead));
                        }
                        (end.min(0.0), end.max(0.0))
                    } else {
                        return up_to(i, profile, *op, &s, 0.0, Some(z))
                            .map(|(f, u)| Resolved::UpTo(f, Box::new(UpTo { next: material, ..u })));
                    }
                }
                Extent::Distance(_) => unreachable!(),
            };
            if !(positive(hi - lo) && lo.is_finite() && hi.is_finite()) {
                return Err(err(ExtentError::BadSize));
            }
            let mut profile = profile.clone();
            profile.plane = at.with_origin(at.origin + at.z() * lo);
            Ok(Resolved::Plain(
                Cow::Owned(Feature::Extrude {
                    profile,
                    extent: Extent::Distance(hi - lo),
                    op: *op,
                }),
                material,
            ))
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
            Ok(Resolved::Plain(
                Cow::Owned(Feature::Revolve {
                    profile,
                    axis: *axis,
                    extent: Turn::Angle(sweep),
                    op: *op,
                }),
                None,
            ))
        }
        _ => unreachable!("only extrudes and revolves have extents"),
    }
}

/// Extrude `i` of `profile` cut off at `s`: along `z` if given, else toward the surface.
fn up_to(
    i: usize,
    profile: &forge_ops::Profile,
    op: crate::Combine,
    s: &Surface,
    offset: f64,
    z: Option<UnitVec3>,
) -> Result<(Feature, UpTo), RegenError> {
    let at = profile.plane;
    let refp = samples(profile).first().map_or(at.origin, |&q| at.plane_point(q));
    let z = z.unwrap_or_else(|| {
        if toward(s, refp, at.z()) < 0.0 {
            -at.z()
        } else {
            at.z()
        }
    });
    let (t, plan) = plan(z, &foot(profile), refp, s, offset).map_err(bad(i))?;
    let sign = z.dot(at.z().get()).signum();
    let base = Feature::Extrude {
        profile: profile.clone(),
        extent: Extent::Distance(sign * t),
        op,
    };
    Ok((
        base,
        UpTo {
            plan,
            next: None,
            start: vec![Name::feature(i as u64 + 1).with_str("start")],
            far: (at.origin + z * t, z),
        },
    ))
}

/// Hole `i` with an end at a plane square to it turned into one at a depth; with any other end
/// surface, or up to the next face, a long hole (flat bottom) to cut off there.
pub(crate) fn hole_end(
    features: &[Feature],
    i: usize,
    h: &Hole,
) -> Result<(Hole, Option<UpTo>, Option<Next>), RegenError> {
    let mut material = None;
    let s = match h.end {
        End::To { target, .. } => surface(features, i, &target)?,
        End::ToNext => {
            let r = h.diameter / 4.0;
            let rays: Vec<Point3> = [(0.0, 0.0), (r, 0.0), (-r, 0.0), (0.0, r), (0.0, -r)]
                .map(|(x, y)| h.at.plane_point(Point2::new(x, y)))
                .to_vec();
            let (s, m) = next(features, i, &rays, h.at.z())?;
            material = Some(m);
            s
        }
        _ => return Ok((*h, None, None)),
    };
    let offset = if let End::To { offset, .. } = h.end {
        offset
    } else {
        0.0
    };
    if let Some(p) = square(&s, &h.at) {
        let depth = along(&h.at, &p, offset, i)?;
        if depth < 0.0 {
            return Err(bad(i)(ExtentError::BadSize));
        }
        return Ok((
            Hole {
                end: End::Depth(depth),
                ..*h
            },
            None,
            material,
        ));
    }
    let mouth = match h.head {
        Head::Plain => h.diameter,
        Head::Counterbore { diameter, .. } | Head::Countersink { diameter, .. } => diameter,
    } / 2.0;
    let foot: Vec<Point3> = [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)]
        .map(|(x, y)| h.at.plane_point(Point2::new(x * mouth, y * mouth)))
        .to_vec();
    let (t, plan) = plan(h.at.z(), &foot, h.at.origin, &s, offset).map_err(bad(i))?;
    let long = Hole {
        end: End::Depth(t),
        point: None,
        ..*h
    };
    let side = Name::feature(i as u64 + 1).with_str("side").with(0);
    let up = UpTo {
        plan,
        next: None,
        start: vec![side.with(0), side.with(1)],
        far: (h.at.origin + h.at.z() * t, h.at.z()),
    };
    Ok((long, Some(up), material))
}
