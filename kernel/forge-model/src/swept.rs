//! Sweep, coil and loft features: their settings and ends, the tools they build, and the tools'
//! analytic solids (a sweep's stretches are prisms and solids of revolution; a coil is tested
//! turn by turn; a loft only by its box).

use crate::{Combine, ExtentError, Feature, History, RegenError, Target, Tool, cut_root, seg2, tool};
use forge_bool::{Op, Seg2, Shape};
use forge_geom::CurveGeom;
use forge_geom::screw::Section;
use forge_math::{Box3, Point2, Point3, UnitVec3, Vec3, dmath, tol};
use forge_ops::{
    Axis2, Handed, LoftGuides, LoftKind, PathSeg, Profile, Stretch, Winding, coil, coil_motion,
    loft_and_bounds, loft_bounds, sweep, sweep_stretches,
};
use forge_topo::Body;

/// Sweep a profile along a tangent-continuous path of lines and arcs, square to it.
#[derive(Clone, Debug, PartialEq)]
pub struct Sweep {
    pub profile: Profile,
    pub path: Vec<PathSeg>,
    pub ends: PathEnds,
    pub op: Combine,
}

/// Where a sweep stops.
#[derive(Clone, Debug, PartialEq)]
pub enum PathEnds {
    /// At the path's ends.
    Path,
    /// Carried straight on past them: `start` back before the first, `end` on after the last.
    Extended { start: f64, end: f64 },
    /// Cut off at a plane (a work plane or an earlier planar face), `offset` on past it; the side
    /// the path starts on is kept.
    To { target: Target, offset: f64 },
}

/// Wind a profile about an axis in its plane, widening by the angle `taper` as it rises.
#[derive(Clone, Debug, PartialEq)]
pub struct Coil {
    pub profile: Profile,
    pub axis: Axis2,
    pub size: CoilSize,
    pub taper: f64,
    pub handed: Handed,
    pub op: Combine,
}

/// How far a coil winds, by any two of pitch (rise per turn), turns and height (the profile's
/// rise in all), or up to a plane.
#[derive(Clone, Debug, PartialEq)]
pub enum CoilSize {
    PitchTurns {
        pitch: f64,
        turns: f64,
    },
    TurnsHeight {
        turns: f64,
        height: f64,
    },
    PitchHeight {
        pitch: f64,
        height: f64,
    },
    /// Wound on until it has passed the plane (a work plane or an earlier planar face), then
    /// cut off there, `offset` on past it.
    To {
        pitch: f64,
        target: Target,
        offset: f64,
    },
}

/// Walls through two or more profiles, matched loop by loop and segment by segment: ruled, or
/// a curvature-continuous blend.
#[derive(Clone, Debug, PartialEq)]
pub struct Loft {
    pub profiles: Vec<Profile>,
    pub kind: LoftKind,
    /// Rails or a centre line shaping a smooth loft.
    pub guides: LoftGuides,
    pub op: Combine,
}

/// How a sweep, coil or loft combines with the part; None for any other feature.
pub fn op(f: &Feature) -> Option<Combine> {
    match f {
        Feature::Sweep(s) => Some(s.op),
        Feature::Coil(c) => Some(c.op),
        Feature::Loft(l) => Some(l.op),
        _ => None,
    }
}

/// A box round what sweep, coil or loft `i` adds, none if it cuts or cannot be built: without
/// building it (a through cut sizes itself by this, on every regen).
pub(crate) fn reach_of(features: History<'_>, i: usize) -> Option<Box3> {
    // (asked for each through hole of a regen: a loft's bounds cost something)
    thread_local! {
        static SEEN: std::cell::RefCell<Vec<(Feature, Option<Box3>)>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    let f = &features[i];
    let free = !matches!(f, Feature::Coil(c) if matches!(c.size, CoilSize::To { .. }));
    if free && let Some(b) = SEEN.with(|s| s.borrow().iter().find(|x| x.0 == *f).map(|x| x.1)) {
        return b;
    }
    let b = reach_built(features, i);
    if free {
        SEEN.with(|s| {
            let mut s = s.borrow_mut();
            if s.len() == 16 {
                s.remove(0);
            }
            s.push((f.clone(), b));
        });
    }
    b
}

fn reach_built(features: History<'_>, i: usize) -> Option<Box3> {
    match &features[i] {
        Feature::Sweep(s) if adds(s.op) => {
            let (profile, path) = match s.ends {
                PathEnds::Extended { start, end } => extended(&s.profile, &s.path, start, end, i).ok()?,
                _ => (s.profile.clone(), s.path.clone()),
            };
            Some(path_box(&path).grow(profile_reach(&profile, path.first()?.start())))
        }
        Feature::Coil(c) if adds(c.op) => {
            let winding = |pitch: f64, turns: f64| Winding {
                pitch,
                turns,
                taper: c.taper,
                handed: c.handed,
            };
            let w = match &c.size {
                CoilSize::PitchTurns { pitch, turns } => winding(*pitch, *turns),
                CoilSize::TurnsHeight { turns, height } => winding(height / turns, *turns),
                CoilSize::PitchHeight { pitch, height } => winding(*pitch, height / pitch),
                CoilSize::To {
                    pitch,
                    target,
                    offset,
                    ..
                } => {
                    let w = winding(*pitch, 1.0);
                    winding(*pitch, turns_to(features, i, c, &w, target, *offset).ok()?.0)
                }
            };
            Some(coil_box(&coil_motion(&c.profile, c.axis, &w).ok()?))
        }
        Feature::Loft(l) if adds(l.op) => loft_bounds(&l.profiles, l.kind, &l.guides).ok(),
        _ => None,
    }
}

fn adds(op: Combine) -> bool {
    matches!(op, Combine::New | Combine::Add)
}

/// The tool of sweep, coil or loft `i` (its solid empty unless `build`).
pub(crate) fn tool_of(features: History<'_>, i: usize, build: bool) -> Result<Tool, RegenError> {
    let id = features.id(i);
    match &features[i] {
        Feature::Sweep(s) => {
            let err = |error| RegenError::Sweep { feature: i, error };
            let (profile, path) = match s.ends {
                PathEnds::Extended { start, end } => extended(&s.profile, &s.path, start, end, i)?,
                _ => (s.profile.clone(), s.path.clone()),
            };
            let (prepared, stretches) = sweep_stretches(&profile, &path).map_err(err)?;
            let shape = sweep_shape(&prepared, &stretches);
            let body = if build {
                sweep(&profile, &path, id).map_err(err)?
            } else {
                Body::new()
            };
            let base = (body, s.op, shape);
            match &s.ends {
                PathEnds::To { target, offset } => {
                    let start = path[0].start();
                    let reach = path_box(&path).grow(profile_reach(&profile, start));
                    trim(features, i, base, target, *offset, start, reach, build)
                }
                _ => Ok(base),
            }
        }
        Feature::Coil(c) => {
            let err = |error| RegenError::Coil { feature: i, error };
            let winding = |pitch: f64, turns: f64| Winding {
                pitch,
                turns,
                taper: c.taper,
                handed: c.handed,
            };
            let w = match c.size {
                CoilSize::PitchTurns { pitch, turns } => winding(pitch, turns),
                CoilSize::TurnsHeight { turns, height } => winding(height / turns, turns),
                CoilSize::PitchHeight { pitch, height } => winding(pitch, height / pitch),
                CoilSize::To { pitch, .. } => winding(pitch, 1.0),
            };
            let (w, cut) = match &c.size {
                CoilSize::To { target, offset, .. } => {
                    let (turns, plane) = turns_to(features, i, c, &w, target, *offset)?;
                    (winding(w.pitch, turns), Some(plane))
                }
                _ => (w, None),
            };
            let m = coil_motion(&c.profile, c.axis, &w).map_err(err)?;
            let shape = coil_shape(&m);
            let body = if build {
                coil(&c.profile, c.axis, &w, id).map_err(err)?
            } else {
                Body::new()
            };
            let base = (body, c.op, shape);
            match cut {
                Some((at, z)) => {
                    let reach = coil_box(&m);
                    let start = m.frame.origin;
                    cut_at((i, features.id(i)), base, at, z, start, reach, build)
                }
                None => Ok(base),
            }
        }
        Feature::Loft(l) => {
            let err = |error| RegenError::Loft { feature: i, error };
            let (body, bounds) = if build {
                loft_and_bounds(&l.profiles, l.kind, &l.guides, id).map_err(err)?
            } else {
                (
                    Body::new(),
                    loft_bounds(&l.profiles, l.kind, &l.guides).map_err(err)?,
                )
            };
            Ok((body, l.op, Shape::Brep(bounds)))
        }
        _ => unreachable!("only sweeps, coils and lofts"),
    }
}

/// The path carried straight on `start` before it and `end` after it, the profile moved back
/// with it.
fn extended(
    profile: &Profile,
    path: &[PathSeg],
    start: f64,
    end: f64,
    i: usize,
) -> Result<(Profile, Vec<PathSeg>), RegenError> {
    if !(start >= 0.0 && end >= 0.0 && start.is_finite() && end.is_finite()) {
        return Err(RegenError::Extent {
            feature: i,
            error: ExtentError::BadSize,
        });
    }
    let (Some(first), Some(last)) = (path.first(), path.last()) else {
        return Err(RegenError::Sweep {
            feature: i,
            error: forge_ops::SweepError::EmptyPath,
        });
    };
    let mut out = Vec::with_capacity(path.len() + 2);
    let mut profile = profile.clone();
    if start > tol::LINEAR {
        let (p, t) = (first.start(), first.direction(0.0));
        profile.plane = profile.plane.with_origin(profile.plane.origin - t * start);
        out.push(PathSeg::Line {
            a: p - t * start,
            b: p,
        });
    }
    out.extend_from_slice(path);
    if end > tol::LINEAR {
        let (p, t) = (last.end(), last.direction(1.0));
        out.push(PathSeg::Line { a: p, b: p + t * end });
    }
    Ok((profile, out))
}

/// Ends, directions and boxes of path pieces.
trait PathPoints {
    fn start(&self) -> Point3;
    fn end(&self) -> Point3;
    /// Unit direction of travel at fraction t.
    fn direction(&self, t: f64) -> Vec3;
}

impl PathPoints for PathSeg {
    fn start(&self) -> Point3 {
        match *self {
            PathSeg::Line { a, .. } => a,
            PathSeg::Arc { circle, start, .. } => circle.eval(start),
        }
    }
    fn end(&self) -> Point3 {
        match *self {
            PathSeg::Line { b, .. } => b,
            PathSeg::Arc { circle, start, sweep } => circle.eval(start + sweep),
        }
    }
    fn direction(&self, t: f64) -> Vec3 {
        let d = match *self {
            PathSeg::Line { a, b } => b - a,
            PathSeg::Arc { circle, start, sweep } => circle.deriv(start + sweep * t).d1 * sweep,
        };
        d.normalize().map_or(Vec3::Z, |u| u.get())
    }
}

fn path_box(path: &[PathSeg]) -> Box3 {
    path.iter().fold(Box3::EMPTY, |b, s| {
        b.hull(&match *s {
            PathSeg::Line { a, b } => Box3::from_points([a, b]),
            PathSeg::Arc { circle, start, sweep } => circle.bbox(forge_math::Interval::new(
                start.min(start + sweep),
                start.max(start + sweep),
            )),
        })
    })
}

/// How far the profile reaches from the point `at` of its plane.
fn profile_reach(profile: &Profile, at: Point3) -> f64 {
    profile
        .loops
        .iter()
        .flat_map(|l| l.segments.iter().flat_map(|(s, _)| s.samples(16)))
        .map(|q| profile.plane.plane_point(q).distance(at))
        .fold(0.0, f64::max)
        // a chord of 16 samples sags at most r (1 - cos(pi/16)) < r / 50
        * 1.02
}

/// A sweep's solid: its stretches' prisms and solids of revolution, together.
fn sweep_shape(profile: &Profile, stretches: &[Stretch]) -> Shape {
    let segs: std::sync::Arc<[Seg2]> = profile
        .loops
        .iter()
        .flat_map(|l| l.segments.iter().map(|(s, _)| seg2(s)))
        .collect();
    let mut parts = stretches.iter().map(|s| match *s {
        Stretch::Straight { plane, length } => Shape::Prism {
            frame: plane,
            segs: segs.clone(),
            lo: 0.0,
            hi: length,
        },
        Stretch::Bend { plane, axis, angle } => {
            let in_plane = |v: Vec3| forge_math::Vec2::new(v.dot(plane.x().get()), v.dot(plane.y().get()));
            let o = in_plane(axis.origin - plane.origin);
            Shape::Revolved {
                frame: axis,
                origin: Point2::new(o.x, o.y),
                dir: in_plane(axis.z().get()),
                perp: in_plane(axis.x().get()),
                segs: segs.clone(),
                angle: Some(angle),
            }
        }
    });
    let first = parts.next().expect("a path has a stretch");
    let rest: Vec<(Op, Shape)> = parts.map(|p| (Op::Union, p)).collect();
    if rest.is_empty() {
        first
    } else {
        Shape::Csg {
            first: Box::new(first),
            rest,
        }
    }
}

pub(crate) fn coil_shape(m: &forge_ops::Wound) -> Shape {
    let segs = m
        .sections
        .iter()
        .map(|s| match *s {
            Section::Line { a, b } => Seg2::Line { a, b },
            Section::Arc { c, r, start, sweep } => Seg2::Arc { c, r, start, sweep },
        })
        .collect();
    let (rr, hh) = extents(&m.sections);
    Shape::Coil {
        frame: m.frame,
        segs,
        a: m.a,
        b: m.b,
        angle: m.angle,
        r: rr,
        h: hh,
    }
}

/// The (r, h) extents of a profile's sections.
fn extents(sections: &[Section]) -> ((f64, f64), (f64, f64)) {
    sections.iter().fold(
        (
            (f64::INFINITY, f64::NEG_INFINITY),
            (f64::INFINITY, f64::NEG_INFINITY),
        ),
        |(r, h), s| {
            let (sr, sh) = s.extent();
            ((r.0.min(sr.0), r.1.max(sr.1)), (h.0.min(sh.0), h.1.max(sh.1)))
        },
    )
}

/// A box holding the whole coil.
fn coil_box(m: &forge_ops::Wound) -> Box3 {
    let ((_, r1), (h0, h1)) = extents(&m.sections);
    let r = r1 + (m.a * m.angle).max(0.0);
    let rise = m.b * m.angle;
    let (lo, hi) = (h0 + rise.min(0.0), h1 + rise.max(0.0));
    let f = m.frame;
    Box3::from_points(
        [-r, r]
            .into_iter()
            .flat_map(|x| [-r, r].into_iter().flat_map(move |y| [lo, hi].map(|z| (x, y, z))))
            .map(|(x, y, z)| f.point_to_world(Point3::new(x, y, z))),
    )
}

/// The plane a target names, its normal turned away from `from`, moved `offset` on along it.
fn target_plane(
    features: History<'_>,
    i: usize,
    target: &Target,
    offset: f64,
    from: Point3,
) -> Result<(Point3, UnitVec3), RegenError> {
    let f = crate::extent::plane(features, i, target)?;
    let z = if f.z().dot(f.origin - from) > 0.0 {
        f.z()
    } else {
        -f.z()
    };
    let at = f.origin + z * offset;
    if z.dot(at - from) <= tol::LINEAR {
        return Err(RegenError::Extent {
            feature: i,
            error: ExtentError::NotAhead,
        });
    }
    Ok((at, z))
}

#[allow(clippy::too_many_arguments)]
fn trim(
    features: History<'_>,
    i: usize,
    base: Tool,
    target: &Target,
    offset: f64,
    start: Point3,
    reach: Box3,
    build: bool,
) -> Result<Tool, RegenError> {
    let (at, z) = target_plane(features, i, target, offset, start)?;
    cut_at((i, features.id(i)), base, at, z, start, reach, build)
}

/// The tool `base` (lying in `reach`) kept on the near side of the plane through `at` square to
/// `z`.
/// `(i, id)`: the feature's place and its id.
fn cut_at(
    (i, id): (usize, u64),
    base: Tool,
    at: Point3,
    z: UnitVec3,
    start: Point3,
    reach: Box3,
    build: bool,
) -> Result<Tool, RegenError> {
    let corners: Vec<Point3> = [reach.x.lo, reach.x.hi]
        .into_iter()
        .flat_map(|x| {
            [reach.y.lo, reach.y.hi]
                .into_iter()
                .flat_map(move |y| [reach.z.lo, reach.z.hi].map(|w| Point3::new(x, y, w)))
        })
        .collect();
    if corners.iter().all(|&p| z.dot(p - at) < 0.0) {
        return Err(RegenError::Extent {
            feature: i,
            error: ExtentError::NeverHits,
        });
    }
    let mut pts = corners;
    pts.push(start);
    let keep = crate::upto::near_box(at, z, &pts);
    let (kb, _, ks) = tool(i, id + 2_000_000, &keep, build)?;
    let shape = match base.2.clone() {
        Shape::Csg { first, mut rest } => {
            rest.push((Op::Intersect, ks.clone()));
            Shape::Csg { first, rest }
        }
        plain => Shape::Csg {
            first: Box::new(plain),
            rest: vec![(Op::Intersect, ks.clone())],
        },
    };
    if !build {
        return Ok((Body::new(), base.1, shape));
    }
    let body = forge_bool::nary_with(
        &[&base.0, &kb],
        &[Op::Intersect],
        Some(&[base.2, ks]),
        None,
        1,
        cut_root(i + 2_000_000),
    )
    .map_err(|error| RegenError::Boolean { feature: i, error })?;
    Ok((body, base.1, shape))
}

/// Turns enough for a coil to pass wholly beyond the target plane, and that plane.
fn turns_to(
    features: History<'_>,
    i: usize,
    c: &Coil,
    w: &Winding,
    target: &Target,
    offset: f64,
) -> Result<(f64, (Point3, UnitVec3)), RegenError> {
    let m = coil_motion(&c.profile, c.axis, w).map_err(|error| RegenError::Coil { feature: i, error })?;
    let (at, z) = target_plane(features, i, target, offset, m.frame.origin)?;
    let ((r0, r1), (h0, h1)) = extents(&m.sections);
    // the side of the plane: at least d0 + rate u at angle u, for each point of the profile
    let ax = m.frame.z();
    let along = z.dot(ax.get());
    let across = (1.0 - along * along).max(0.0).sqrt();
    let base = z.dot(m.frame.origin - at);
    let worst = |h: f64, r: f64| base + along * h - across * r;
    // the profile starts short of the plane, and the coil must get past it
    if base + along.max(0.0) * h1 + along.min(0.0) * h0 + across * r1 >= -tol::LINEAR {
        return Err(RegenError::Extent {
            feature: i,
            error: ExtentError::NotAhead,
        });
    }
    let rate = along * m.b - across * m.a.abs();
    if rate <= 0.0 {
        return Err(RegenError::Extent {
            feature: i,
            error: ExtentError::NeverHits,
        });
    }
    let start = worst(if along >= 0.0 { h0 } else { h1 }, r1.max(r0));
    // past the plane by a margin, with the end section wholly beyond it
    let angle = -start / rate + 0.25;
    Ok((angle / dmath::TAU, (at, z)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::suite;
    use forge_bool::{Index, Member, Where};
    use forge_math::rng::Rng;

    /// The analytic coil and sweep agree with their B-reps wherever they answer, and answer
    /// most points.
    #[test]
    fn coil_and_sweep_shapes_agree_with_their_tools() {
        let mut r = Rng::new(91);
        for name in ["sweep", "coil"] {
            let part = suite::part(name).expect("a suite part");
            let (mut answered, mut near) = (0, 0);
            for i in 0..part.features.len() {
                if op(&part.features[i]).is_none() {
                    continue;
                }
                let (body, _, sh) =
                    tool_of(crate::History::new(&part.features, None), i, true).expect("builds");
                let index = Index::new(&body);
                let b = index.bbox.grow(1.0);
                for _ in 0..3000 {
                    let p = Point3::new(
                        r.range(b.x.lo, b.x.hi),
                        r.range(b.y.lo, b.y.hi),
                        r.range(b.z.lo, b.z.hi),
                    );
                    let want = match sh.member(p, 1e-4) {
                        Member::Near => {
                            near += 1;
                            continue;
                        }
                        Member::In => Where::Inside,
                        Member::Out => Where::Outside,
                    };
                    answered += 1;
                    assert_eq!(index.classify(p), want, "{name} feature {i} at {p:?}");
                }
            }
            assert!(answered > 20 * near, "{name}: {answered} answered, {near} near");
        }
    }
}
