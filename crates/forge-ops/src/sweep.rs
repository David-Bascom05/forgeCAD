//! Sweeps and coils: a profile carried along a path, square to it, or wound about an axis.
//!
//! Along a path of lines and arcs the profile moves rigidly: a line translates it, an arc turns
//! it about the arc's axis, which lies in the profile's plane (that plane is square to the path,
//! the axis square to the path too). So each stretch of wall is an extrusion or a revolution of a
//! profile segment, and has a closed form: planes and cylinders along a line; planes, cylinders,
//! cones, spheres and tori round a bend. Only a bend tight enough that a segment's circle reaches
//! past the bend's axis (while the profile itself stays clear of it) has none: that wall is a
//! screw surface with no rise, defined by the turning.
//!
//! A coil winds the profile about an axis in its plane, rising `pitch` per turn and widening by
//! `taper` (an angle off the axis) as it rises: every wall is a screw surface and every rail a
//! helix, both defined by the motion.

use crate::profile::{ProfileError, Segment};
use crate::revolve::Axis2;
use crate::skin::{Skin, min_radius, prepare, rh_section, section_curve};
use forge_geom::screw::{Screw, Section};
use forge_geom::{Circle, Cone, Curve, CurveGeom, Cylinder, Line, Plane, Sphere, Surface, Torus};
use forge_math::{Frame, Interval, Point3, UnitVec3, Vec2, Vec3, dmath, tol};
use forge_topo::{Body, Name};

/// One piece of a sweep path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathSeg {
    Line {
        a: Point3,
        b: Point3,
    },
    /// The arc of `circle` from angle `start` through `sweep` (negative: clockwise about the
    /// circle's z).
    Arc {
        circle: Circle,
        start: f64,
        sweep: f64,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum SweepError {
    Profile(ProfileError),
    EmptyPath,
    /// Path piece `index` is too short, or an arc of a whole turn or more.
    Degenerate {
        index: usize,
    },
    /// Path piece `index` does not start where the one before ends, or turns a corner there:
    /// the path must be tangent-continuous.
    Kink {
        index: usize,
    },
    /// The path does not start on the profile's plane, square to it.
    NotSquare,
    /// The bend of path piece `index` is tighter than the profile: it would fold through the
    /// bend's axis.
    TooTight {
        index: usize,
    },
    /// The path brings the swept solid back into itself.
    SelfIntersecting,
}

impl From<ProfileError> for SweepError {
    fn from(e: ProfileError) -> Self {
        SweepError::Profile(e)
    }
}

impl PathSeg {
    fn ends(&self) -> (Point3, Point3) {
        match *self {
            PathSeg::Line { a, b } => (a, b),
            PathSeg::Arc { circle, start, sweep } => (circle.eval(start), circle.eval(start + sweep)),
        }
    }

    /// Unit tangents at the start and the end.
    fn tangents(&self) -> Option<(UnitVec3, UnitVec3)> {
        match *self {
            PathSeg::Line { a, b } => (b - a).normalize().map(|d| (d, d)),
            PathSeg::Arc { circle, start, sweep } => {
                let s = sweep.signum();
                let t = |a: f64| (circle.deriv(a).d1 * s).normalize();
                Some((t(start)?, t(start + sweep)?))
            }
        }
    }

    fn degenerate(&self) -> bool {
        match *self {
            PathSeg::Line { a, b } => a.distance(b) <= tol::MIN_LENGTH,
            PathSeg::Arc { circle, sweep, .. } => {
                circle.radius <= tol::MIN_LENGTH
                    || (circle.radius * sweep).abs() <= tol::MIN_LENGTH
                    || sweep.abs() >= dmath::TAU - tol::ANGULAR
            }
        }
    }

    /// The axis frame an arc turns about (z so the turn is positive), and the angle.
    fn turn(circle: &Circle, sweep: f64) -> (Frame, f64) {
        let z = if sweep > 0.0 {
            circle.frame.z()
        } else {
            -circle.frame.z()
        };
        (Frame::from_z(circle.frame.origin, z), sweep.abs())
    }

    /// The frame `f` carried from the piece's start to its end.
    fn carry(&self, f: &Frame) -> Frame {
        match *self {
            PathSeg::Line { a, b } => f.with_origin(f.origin + (b - a)),
            PathSeg::Arc { circle, sweep, .. } => {
                let (r, angle) = PathSeg::turn(&circle, sweep);
                r.rotated_about_z(angle).compose(&r.inverse().compose(f))
            }
        }
    }
}

/// Sweeps a profile along a tangent-continuous path of lines and arcs that starts on the profile's
/// plane, square to it, keeping the profile square to the path. `feature` is the history id the
/// names derive from (see `skin`).
pub fn sweep(profile: &crate::Profile, path: &[PathSeg], feature: u64) -> Result<Body, SweepError> {
    let first = path.first().ok_or(SweepError::EmptyPath)?;
    let mut tangents = Vec::with_capacity(path.len());
    for (i, s) in path.iter().enumerate() {
        if s.degenerate() {
            return Err(SweepError::Degenerate { index: i });
        }
        tangents.push(s.tangents().ok_or(SweepError::Degenerate { index: i })?);
        if i > 0 {
            let gap = path[i - 1].ends().1.distance(s.ends().0);
            let (t0, t1) = (tangents[i - 1].1, tangents[i].0);
            if gap > tol::LINEAR || t0.cross(t1).length() > tol::ANGULAR || t0.dot(t1) <= 0.0 {
                return Err(SweepError::Kink { index: i });
            }
        }
    }
    let t0 = tangents[0].0;
    let start = first.ends().0;
    let z = profile.plane.z();
    if z.cross(t0).length() > tol::ANGULAR || z.dot(start - profile.plane.origin).abs() > tol::LINEAR {
        return Err(SweepError::NotSquare);
    }
    let prep = prepare(profile, t0.get())?;
    let segs: Vec<Segment> = prep.segments().copied().collect();
    let mut planes = vec![prep.profile.plane];
    for s in path {
        let next = s.carry(planes.last().expect("starts with the profile's plane"));
        planes.push(next);
    }
    let curves = planes
        .iter()
        .map(|f| segs.iter().map(|s| section_curve(s, f)).collect())
        .collect();
    let mut rails = Vec::with_capacity(path.len());
    let mut walls = Vec::with_capacity(path.len());
    let starts = |f: &Frame| -> Vec<Point3> { segs.iter().map(|s| f.plane_point(s.start())).collect() };
    for (k, s) in path.iter().enumerate() {
        let f = &planes[k];
        match *s {
            PathSeg::Line { a, b } => {
                let len = a.distance(b);
                let dir = (b - a).normalize().expect("not degenerate");
                rails.push(
                    starts(f)
                        .into_iter()
                        .map(|p| (Curve::Line(Line::new(p, dir)), Interval::new(0.0, len)))
                        .collect(),
                );
                walls.push(segs.iter().map(|sg| extruded(sg, f)).collect());
            }
            PathSeg::Arc { circle, sweep, .. } => {
                let (axis, angle) = PathSeg::turn(&circle, sweep);
                let w = axis.z();
                let foot = |p: Point3| axis.origin + w * w.dot(p - axis.origin);
                let at = path[k].ends().0;
                let rf =
                    Frame::from_zx(axis.origin, w, at - foot(at)).ok_or(SweepError::TooTight { index: k })?;
                let ring: Vec<Section> = segs.iter().map(|sg| rh_section(&rf, f, sg)).collect();
                if ring.iter().any(|sc| min_radius(sc) <= tol::LINEAR) {
                    return Err(SweepError::TooTight { index: k });
                }
                rails.push(
                    starts(f)
                        .into_iter()
                        .map(|p| {
                            let c = foot(p);
                            let frame = Frame::from_zx(c, w, p - c).expect("clear of the axis");
                            let circle = Circle {
                                frame,
                                radius: p.distance(c),
                            };
                            (Curve::Circle(circle), Interval::new(0.0, angle))
                        })
                        .collect(),
                );
                walls.push(turned(&rf, &ring, &prep.loop_sizes(), angle));
            }
        }
    }
    let body = Skin {
        loops: prep.loop_sizes(),
        keys: prep.keys,
        planes,
        curves,
        rails,
        walls,
    }
    .build(Name::feature(feature));
    if path.len() > 1 && crate::selfx::crossing(&body, &vec![true; body.faces.len()]).is_some() {
        return Err(SweepError::SelfIntersecting);
    }
    Ok(body)
}

/// The wall a segment drawn in `f` makes moving along `f`'s z.
fn extruded(s: &Segment, f: &Frame) -> Surface {
    match *s {
        Segment::Line { a, b } => {
            let (pa, pb) = (f.plane_point(a), f.plane_point(b));
            let along = pb - pa;
            let n = along
                .cross(f.z().get())
                .normalize()
                .expect("profile rejects degenerate lines");
            Surface::Plane(Plane::new(
                Frame::from_zx(pa, n, along).expect("perpendicular axes"),
            ))
        }
        Segment::Arc { center, radius, .. } => Surface::Cylinder(Cylinder {
            frame: f.with_origin(f.plane_point(center)),
            radius,
        }),
        Segment::Circle { .. } => unreachable!("prepare splits circles"),
    }
}

/// The walls of a profile (in (r, h) about `rf`, loop by loop) turned through `angle` about rf's
/// z: a plane, cylinder, cone, sphere or torus where one fits, else a screw surface with no rise.
fn turned(rf: &Frame, ring: &[Section], loops: &[usize], angle: f64) -> Vec<Surface> {
    let at = |h: f64| rf.with_origin(rf.origin + rf.z() * h);
    let mut out = Vec::with_capacity(ring.len());
    let mut first = 0;
    for &len in loops {
        let lp = &ring[first..first + len];
        let mut screws: Option<Vec<Screw>> = None;
        for (k, sc) in lp.iter().enumerate() {
            let closed = match *sc {
                Section::Line { a, b } => {
                    if (b.y - a.y).abs() <= tol::LINEAR {
                        Some(Surface::Plane(Plane::new(at(a.y))))
                    } else if (b.x - a.x).abs() <= tol::LINEAR {
                        Some(Surface::Cylinder(Cylinder {
                            frame: *rf,
                            radius: 0.5 * (a.x + b.x),
                        }))
                    } else {
                        // grows along z from the narrow end: its frame's z points the way it widens
                        let k = (b.x - a.x) / (b.y - a.y);
                        let (n, w) = if a.x <= b.x { (a, b) } else { (b, a) };
                        let frame = if w.y > n.y { at(n.y) } else { at(n.y).flipped() };
                        Cone::new(frame, n.x, dmath::atan(k.abs())).map(Surface::Cone)
                    }
                }
                Section::Arc { c, r, .. } => {
                    if c.x <= tol::LINEAR {
                        Some(Surface::Sphere(Sphere {
                            frame: at(c.y),
                            radius: r,
                        }))
                    } else {
                        Torus::new(at(c.y), c.x, r).map(Surface::Torus)
                    }
                }
            };
            out.push(closed.unwrap_or_else(|| {
                let s = screws.get_or_insert_with(|| {
                    Screw::loop_faces(*rf, lp, 0.0, 0.0, Interval::new(0.0, angle))
                        .expect("clear of the axis")
                });
                s[k].clone().surface()
            }));
        }
        first += len;
    }
    out
}

#[derive(Clone, Debug, PartialEq)]
pub enum CoilError {
    Profile(ProfileError),
    BadAxis,
    /// Part of the profile lies on the axis or across it.
    CrossesAxis,
    /// Turns must be positive (and fewer than a thousand), the pitch positive.
    BadTurns,
    /// The pitch must exceed the profile's height along the axis, or one turn runs into the next.
    TurnsTouch,
    /// The taper must be less than a right angle, and must not draw the coil onto its axis.
    BadTaper,
}

impl From<ProfileError> for CoilError {
    fn from(e: ProfileError) -> Self {
        CoilError::Profile(e)
    }
}

/// Winds a profile about an axis in its plane through `turns` turns (right-handed about the
/// axis's direction), rising `pitch` per turn along it, the coil widening by `taper` (radians
/// off the axis; negative narrows it) as it rises.
pub fn coil(
    profile: &crate::Profile,
    axis: Axis2,
    pitch: f64,
    turns: f64,
    taper: f64,
    feature: u64,
) -> Result<Body, CoilError> {
    if !(turns > tol::ANGULAR && turns < 1000.0 && pitch > tol::MIN_LENGTH) {
        return Err(CoilError::BadTurns);
    }
    if taper.is_nan() || taper.abs() >= dmath::FRAC_PI_2 - 1e-6 {
        return Err(CoilError::BadTaper);
    }
    let dir2 = axis.dir.normalize().ok_or(CoilError::BadAxis)?;
    let plane = profile.plane;
    let vec3 = |v: Vec2| plane.x() * v.x + plane.y() * v.y;
    // the side of the axis the profile is on
    let perp2 = dir2.perp();
    let mut side = 0.0f64;
    for l in &profile.loops {
        for (s, _) in &l.segments {
            for p in s.samples(32) {
                let r = (p - axis.origin).dot(perp2);
                side = if r.abs() > side.abs() { r } else { side };
            }
        }
    }
    if side.abs() <= tol::LINEAR {
        return Err(CoilError::CrossesAxis);
    }
    let z = vec3(dir2).normalize().ok_or(CoilError::BadAxis)?;
    let rf = Frame::from_zx(plane.plane_point(axis.origin), z, vec3(perp2 * side.signum()))
        .ok_or(CoilError::BadAxis)?;
    let prep = prepare(profile, rf.y().get())?;
    let f0 = prep.profile.plane;
    let segs: Vec<Segment> = prep.segments().copied().collect();
    let ring: Vec<Section> = segs.iter().map(|s| rh_section(&rf, &f0, s)).collect();
    if ring.iter().any(|s| min_radius(s) <= tol::LINEAR) {
        return Err(CoilError::CrossesAxis);
    }
    let (lo, hi) = ring
        .iter()
        .map(crate::skin::heights)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |a, h| {
            (a.0.min(h.0), a.1.max(h.1))
        });
    if pitch <= hi - lo + tol::LINEAR {
        return Err(CoilError::TurnsTouch);
    }
    let total = dmath::TAU * turns;
    let b = pitch / dmath::TAU;
    let a = b * dmath::sin(taper) / dmath::cos(taper);
    let mut walls = Vec::with_capacity(ring.len());
    let mut first = 0;
    for len in prep.loop_sizes() {
        let faces = Screw::loop_faces(rf, &ring[first..first + len], a, b, Interval::new(0.0, total))
            .ok_or(CoilError::BadTaper)?;
        walls.extend(faces);
        first += len;
    }
    // the end section: the start's turned, raised and drawn out
    let local = rf.inverse().compose(&f0);
    let drawn = local.with_origin(local.origin + Vec3::X * (a * total));
    let f1 = rf
        .rotated_about_z(total)
        .with_origin(rf.origin + rf.z() * (b * total))
        .compose(&drawn);
    let planes = vec![f0, f1];
    let curves = planes
        .iter()
        .map(|f| segs.iter().map(|s| section_curve(s, f)).collect())
        .collect();
    let rails = vec![
        walls
            .iter()
            .map(|w| (w.helix(0.0), Interval::new(0.0, total)))
            .collect(),
    ];
    Ok(Skin {
        loops: prep.loop_sizes(),
        keys: prep.keys,
        planes,
        curves,
        rails,
        walls: vec![walls.into_iter().map(Screw::surface).collect()],
    }
    .build(Name::feature(feature)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Profile;
    use crate::profile::ProfileLoop;
    use forge_math::Point2;
    use forge_topo::check;

    fn valid(b: &Body) {
        let problems = check(b);
        assert!(problems.is_empty(), "{problems:?}");
    }

    fn square(plane: Frame, s: f64) -> Profile {
        Profile {
            plane,
            loops: vec![Profile::rectangle(Point2::new(-s, -s), 2.0 * s, 2.0 * s, 0)],
        }
    }

    fn round(plane: Frame, r: f64) -> Profile {
        Profile {
            plane,
            loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
        }
    }

    /// A line up z, a quarter bend towards x, a line along x.
    fn elbow(bend: f64) -> Vec<PathSeg> {
        let c = Point3::new(bend, 0.0, 10.0);
        let circle = Circle {
            frame: Frame::from_zx(c, UnitVec3::Y, Vec3::new(-1.0, 0.0, 0.0)).unwrap(),
            radius: bend,
        };
        vec![
            PathSeg::Line {
                a: Point3::ORIGIN,
                b: Point3::new(0.0, 0.0, 10.0),
            },
            PathSeg::Arc {
                circle,
                start: 0.0,
                sweep: dmath::FRAC_PI_2,
            },
            PathSeg::Line {
                a: Point3::new(bend, 0.0, 10.0 + bend),
                b: Point3::new(bend + 10.0, 0.0, 10.0 + bend),
            },
        ]
    }

    #[test]
    fn elbows_are_closed_form_and_valid() {
        for prof in [square(Frame::WORLD, 1.0), round(Frame::WORLD, 1.5)] {
            let b = sweep(&prof, &elbow(5.0), 1).unwrap();
            valid(&b);
            assert!(
                b.faces
                    .iter()
                    .all(|f| !matches!(f.surface, Surface::Procedural(_)))
            );
        }
    }

    #[test]
    fn a_bend_tighter_than_a_circle_uses_a_screw_wall() {
        // a lens of two arcs: the left one's circle reaches past the bend's axis, the lens does not
        let (d, rad) = (2.0, 5f64.sqrt());
        let half = dmath::atan(0.5);
        let prof = Profile {
            plane: Frame::WORLD,
            loops: vec![ProfileLoop::new(vec![
                (
                    Segment::Arc {
                        center: Point2::new(-d, 0.0),
                        radius: rad,
                        start: -half,
                        sweep: 2.0 * half,
                    },
                    0,
                ),
                (
                    Segment::Arc {
                        center: Point2::new(d, 0.0),
                        radius: rad,
                        start: dmath::PI - half,
                        sweep: 2.0 * half,
                    },
                    1,
                ),
            ])],
        };
        let b = sweep(&prof, &elbow(2.5), 1);
        let Ok(b) = b else { panic!("{b:?}") };
        valid(&b);
        assert!(
            b.faces
                .iter()
                .any(|f| matches!(f.surface, Surface::Procedural(_)))
        );
    }

    #[test]
    fn bad_paths_are_refused() {
        let prof = square(Frame::WORLD, 1.0);
        assert_eq!(sweep(&prof, &[], 1), Err(SweepError::EmptyPath));
        let mut p = elbow(5.0);
        p[2] = PathSeg::Line {
            a: Point3::new(5.0, 0.0, 15.0),
            b: Point3::new(5.0, 10.0, 15.0),
        };
        assert_eq!(sweep(&prof, &p, 1), Err(SweepError::Kink { index: 2 }));
        assert_eq!(
            sweep(&prof, &elbow(0.5), 1),
            Err(SweepError::TooTight { index: 1 })
        );
        let tilted = Frame::from_z(Point3::ORIGIN, Vec3::new(0.1, 0.0, 1.0).normalize().unwrap());
        assert_eq!(
            sweep(&square(tilted, 1.0), &elbow(5.0), 1),
            Err(SweepError::NotSquare)
        );
    }

    #[test]
    fn coils_are_valid() {
        let standing = Frame::from_zx(
            Point3::ORIGIN,
            UnitVec3::new_unchecked(Vec3::new(0.0, -1.0, 0.0)),
            Vec3::X,
        )
        .unwrap();
        let up = Axis2 {
            origin: Point2::ORIGIN,
            dir: Vec2::new(0.0, -1.0),
        };
        let wire = Profile {
            plane: standing,
            loops: vec![Profile::circle(Point2::new(10.0, 0.0), 1.0, 0)],
        };
        let bar = Profile {
            plane: standing,
            loops: vec![Profile::rectangle(Point2::new(8.0, -1.0), 3.0, 2.0, 0)],
        };
        for (prof, taper) in [(&wire, 0.0), (&bar, 0.0), (&wire, 0.1), (&bar, -0.1)] {
            let b = coil(prof, up, 4.0, 2.5, taper, 1).unwrap();
            valid(&b);
        }
        assert_eq!(
            coil(&bar, up, 1.5, 2.0, 0.0, 1).unwrap_err(),
            CoilError::TurnsTouch
        );
        assert_eq!(coil(&bar, up, 4.0, 0.0, 0.0, 1).unwrap_err(), CoilError::BadTurns);
        assert_eq!(
            coil(&bar, up, 4.0, 10.0, -0.5, 1).unwrap_err(),
            CoilError::BadTaper
        );
    }
}
