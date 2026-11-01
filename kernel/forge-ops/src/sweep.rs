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
use crate::skin::{Skin, prepare, rh_section, section_curve};
use forge_geom::screw::{Screw, Section};
use forge_geom::spline::swept::plane_of;
use forge_geom::spline::{Spine, Spline, Spun, Swept};
use forge_geom::{Circle, Cone, Curve, CurveGeom, Cylinder, Line, Plane, Sphere, Surface, Torus};
use forge_math::{Frame, Interval, Point3, UnitVec3, Vec2, Vec3, dmath, tol};
use forge_topo::{Body, Name};
use std::sync::Arc;

/// One piece of a sweep path.
#[derive(Clone, Debug, PartialEq)]
pub enum PathSeg {
    /// A free curve in space, lying in one plane.
    Spline(Arc<Spline>),
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
    /// Path piece `index` is a curve not lying in one plane.
    NotPlanar {
        index: usize,
    },
}

impl From<ProfileError> for SweepError {
    fn from(e: ProfileError) -> Self {
        SweepError::Profile(e)
    }
}

impl PathSeg {
    fn ends(&self) -> (Point3, Point3) {
        match *self {
            PathSeg::Spline(ref c) => c.ends(),
            PathSeg::Line { a, b } => (a, b),
            PathSeg::Arc { circle, start, sweep } => (circle.eval(start), circle.eval(start + sweep)),
        }
    }

    /// Unit tangents at the start and the end.
    fn tangents(&self) -> Option<(UnitVec3, UnitVec3)> {
        match *self {
            PathSeg::Spline(ref c) => {
                let d = c.domain();
                Some((c.deriv(d.lo).d1.normalize()?, c.deriv(d.hi).d1.normalize()?))
            }
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
            PathSeg::Spline(ref c) => !c.is_regular() || c.bbox(c.domain()).diagonal() <= tol::MIN_LENGTH,
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
            PathSeg::Spline(ref c) => {
                let d = c.domain();
                let (t0, t1) = (c.deriv(d.lo).d1.normalize(), c.deriv(d.hi).d1.normalize());
                let (Some(t0), Some(t1)) = (t0, t1) else {
                    return f.with_origin(c.ends().1);
                };
                let turned = |v: Vec3| {
                    let axis = t0.get().cross(t1.get());
                    let (cs, sn) = (t0.dot(t1.get()), axis.length());
                    let Some(ax) = axis.normalize() else {
                        return if cs < 0.0 { -v } else { v };
                    };
                    let along = ax * ax.dot(v);
                    along + (v - along) * cs + ax.cross(v) * sn
                };
                Frame::from_zx(c.ends().1, t1, turned(f.x().get()))
                    .unwrap_or_else(|| f.with_origin(c.ends().1))
            }
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
    let (prep, stretches) = stretches(profile, path)?;
    let segs: Vec<Segment> = prep.segments().cloned().collect();
    let mut planes: Vec<Frame> = stretches.iter().map(|s| s.plane()).collect();
    planes.push(path[path.len() - 1].carry(&planes[planes.len() - 1]));
    let curves = planes
        .iter()
        .map(|f| segs.iter().map(|s| section_curve(s, f)).collect())
        .collect();
    let mut rails = Vec::with_capacity(path.len());
    let mut walls = Vec::with_capacity(path.len());
    let starts = |f: &Frame| -> Vec<Point3> { segs.iter().map(|s| f.plane_point(s.start())).collect() };
    for st in &stretches {
        match *st {
            Stretch::Along {
                plane: f,
                ref path,
                normal,
            } => {
                let spine = Spine::planar(path.clone(), normal).expect("checked regular");
                let w: Vec<Swept> = segs
                    .iter()
                    .map(|sg| {
                        Swept::new(spine.clone(), section_spline(sg), &f)
                            .expect("a profile's curves are regular")
                    })
                    .collect();
                let range = path.domain();
                rails.push(
                    w.iter()
                        .zip(&segs)
                        .map(|(x, sg)| {
                            let local = sg.start();
                            (x.rail(local), range)
                        })
                        .collect(),
                );
                walls.push(w.into_iter().map(Swept::surface).collect());
            }
            Stretch::Straight { plane: f, length } => {
                rails.push(
                    starts(&f)
                        .into_iter()
                        .map(|p| (Curve::Line(Line::new(p, f.z())), Interval::new(0.0, length)))
                        .collect(),
                );
                walls.push(segs.iter().map(|sg| extruded(sg, &f, length)).collect());
            }
            Stretch::Bend {
                plane: f,
                axis: rf,
                angle,
            } => {
                let w = rf.z();
                let foot = |p: Point3| rf.origin + w * w.dot(p - rf.origin);
                rails.push(
                    starts(&f)
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
                let ring: Vec<Section> = segs.iter().map(|sg| ring_section(&rf, &f, sg)).collect();
                let mut w = turned(&rf, &ring, &prep.loop_sizes(), angle);
                for (k, sg) in segs.iter().enumerate() {
                    if let Segment::Spline(c) = sg {
                        let (m, ..) = rh_curve(&rf, &f, &c.c);
                        w[k] = Spun::new(rf, Arc::new(m)).expect("clear of the axis").surface();
                    }
                }
                walls.push(w);
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
        apex: [false; 2],
    }
    .build(Name::feature(feature));
    if path.len() > 1 && crate::selfx::crossing(&body, &vec![true; body.faces.len()]).is_some() {
        return Err(SweepError::SelfIntersecting);
    }
    Ok(body)
}

/// One stretch of a sweep: the profile's plane where it starts (z along the path), and how it
/// moves from there.
#[derive(Clone, Debug, PartialEq)]
pub enum Stretch {
    /// Along a free curve in a plane of this normal, turning with its tangent.
    Along {
        plane: Frame,
        path: Arc<Spline>,
        normal: UnitVec3,
    },
    /// Along the plane's z.
    Straight { plane: Frame, length: f64 },
    /// Turning `angle` about `axis`'s z (its x points from the axis towards the profile).
    Bend { plane: Frame, axis: Frame, angle: f64 },
}

impl Stretch {
    pub fn plane(&self) -> Frame {
        match *self {
            Stretch::Straight { plane, .. } | Stretch::Bend { plane, .. } | Stretch::Along { plane, .. } => {
                plane
            }
        }
    }
}

/// The checked profile (square to the path, circles in halves) and the stretches a sweep
/// along `path` is made of.
pub fn sweep_stretches(
    profile: &crate::Profile,
    path: &[PathSeg],
) -> Result<(crate::Profile, Vec<Stretch>), SweepError> {
    stretches(profile, path).map(|(p, s)| (p.profile, s))
}

/// Each piece's start and end tangents, once the path is found tangent-continuous.
pub(crate) fn check_path(path: &[PathSeg]) -> Result<Vec<(UnitVec3, UnitVec3)>, SweepError> {
    if path.is_empty() {
        return Err(SweepError::EmptyPath);
    }
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
    Ok(tangents)
}

fn stretches(
    profile: &crate::Profile,
    path: &[PathSeg],
) -> Result<(crate::skin::Prepared, Vec<Stretch>), SweepError> {
    let first = path.first().ok_or(SweepError::EmptyPath)?;
    let tangents = check_path(path)?;
    let t0 = tangents[0].0;
    let start = first.ends().0;
    let z = profile.plane.z();
    if z.cross(t0).length() > tol::ANGULAR || z.dot(start - profile.plane.origin).abs() > tol::LINEAR {
        return Err(SweepError::NotSquare);
    }
    let prep = prepare(profile, t0.get())?;
    let segs: Vec<Segment> = prep.segments().cloned().collect();
    let mut plane = prep.profile.plane;
    let mut out = Vec::with_capacity(path.len());
    for (k, s) in path.iter().enumerate() {
        out.push(match *s {
            PathSeg::Spline(ref c) => {
                // straight, it lies in any plane: one through the profile's x
                let normal = match plane_of(c) {
                    Some(n) => n,
                    None if straight(c) => plane.x(),
                    None => return Err(SweepError::NotPlanar { index: k }),
                };
                if too_tight(c, normal, &plane, &segs) {
                    return Err(SweepError::TooTight { index: k });
                }
                Stretch::Along {
                    plane,
                    path: c.clone(),
                    normal,
                }
            }
            PathSeg::Line { a, b } => Stretch::Straight {
                plane,
                length: a.distance(b),
            },
            PathSeg::Arc { circle, sweep, .. } => {
                let (axis, angle) = PathSeg::turn(&circle, sweep);
                let w = axis.z();
                let at = s.ends().0;
                let foot = axis.origin + w * w.dot(at - axis.origin);
                let rf =
                    Frame::from_zx(axis.origin, w, at - foot).ok_or(SweepError::TooTight { index: k })?;
                if segs.iter().any(|sg| match sg {
                    Segment::Spline(c) => rh_curve(&rf, &plane, &c.c).1.0 <= tol::LINEAR,
                    _ => rh_section(&rf, &plane, sg).extent().0.0 <= tol::LINEAR,
                }) {
                    return Err(SweepError::TooTight { index: k });
                }
                Stretch::Bend {
                    plane,
                    axis: rf,
                    angle,
                }
            }
        });
        plane = s.carry(&plane);
    }
    Ok((prep, out))
}

/// A segment as a curve of its plane.
fn section_spline(s: &Segment) -> Arc<Spline> {
    match *s {
        Segment::Spline(ref c) => c.c.clone(),
        Segment::Line { a, b } => {
            let row = [[a.x, a.y, 0.0, 1.0], [b.x, b.y, 0.0, 1.0]];
            Arc::new(
                Spline::from_pieces(1, vec![0.0, a.distance(b)], row.to_vec())
                    .expect("profile rejects degenerate lines"),
            )
        }
        Segment::Arc {
            center,
            radius,
            start,
            sweep,
        } => Arc::new(
            Spline::ellipse(
                Point3::new(center.x, center.y, 0.0),
                Vec3::X * radius,
                Vec3::Y * radius,
                start,
                sweep,
            )
            .expect("profile rejects degenerate arcs"),
        ),
        Segment::Circle { .. } => unreachable!("prepare splits circles"),
    }
}

/// A curve drawn in `plane` as (r, h) about the axis frame, its radii and heights' spans.
pub(crate) fn rh_curve(rf: &Frame, plane: &Frame, c: &Spline) -> (Spline, (f64, f64), (f64, f64)) {
    let m = c.mapped(|q| {
        let p = crate::skin::radius_height(rf, plane, forge_math::Point2::new(q.x, q.y));
        Point3::new(p.x, p.y, 0.0)
    });
    let b = m.bbox(m.domain());
    (m, (b.x.lo, b.x.hi), (b.y.lo, b.y.hi))
}

/// A segment in (r, h), a free curve standing as its chord where only its ends are read.
fn ring_section(rf: &Frame, plane: &Frame, s: &Segment) -> Section {
    match s {
        Segment::Spline(c) => Section::Line {
            a: crate::skin::radius_height(rf, plane, c.start()),
            b: crate::skin::radius_height(rf, plane, c.end()),
        },
        _ => rh_section(rf, plane, s),
    }
}

fn straight(c: &Spline) -> bool {
    let (a, b) = c.ends();
    let Some(u) = (b - a).normalize() else {
        return false;
    };
    let (_, _, pts) = c.bezier_data();
    pts.iter()
        .all(|h| u.get().cross(Point3::new(h[0], h[1], h[2]) - a).length() <= tol::LINEAR)
}

/// Whether the profile reaches past the path's centre of curvature, where the wall would fold.
fn too_tight(c: &Spline, normal: UnitVec3, plane: &Frame, segs: &[Segment]) -> bool {
    let d = c.domain();
    let t0 = c.deriv(d.lo).d1;
    let Some(n0) = normal.get().cross(t0).normalize() else {
        return true;
    };
    let (mut lo, mut hi) = (0.0f64, 0.0f64);
    for sg in segs {
        for q in sg.samples(32) {
            let s = plane.vec_to_world(Vec3::new(q.x, q.y, 0.0)).dot(n0.get());
            lo = lo.min(s);
            hi = hi.max(s);
        }
    }
    let n = 32 * c.pieces();
    (0..=n).any(|i| {
        let q = c.deriv(d.lo + d.width() * i as f64 / n as f64);
        let k = q.d1.cross(q.d2).dot(normal.get()) / q.d1.length().powi(3);
        // a section point s along the normal moves at speed 1 - k s
        1.0 - k * hi <= 1e-6 || 1.0 - k * lo <= 1e-6
    })
}

/// The wall a segment drawn in `f` makes moving along `f`'s z.
fn extruded(s: &Segment, f: &Frame, length: f64) -> Surface {
    match *s {
        Segment::Spline(ref c) => {
            forge_geom::spline::Extruded::new(*f, c.c.clone(), Interval::new(0.0, length))
                .expect("a profile's curves are regular")
                .surface()
        }
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

/// Which way a coil turns as it rises along its axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handed {
    Right,
    Left,
}

/// How a coil winds: `turns` turns rising `pitch` each along the axis, widening by `taper`
/// (radians off the axis; negative narrows it) as it rises.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Winding {
    pub pitch: f64,
    pub turns: f64,
    pub taper: f64,
    pub handed: Handed,
}

/// A coil's motion and profile: section point (r, h) at angle u (0 to `angle`) lies at
/// ((r + a u) cos u, (r + a u) sin u, h + b u) in `frame` (z the axis, x through the profile).
#[derive(Clone, Debug, PartialEq)]
pub struct Wound {
    pub frame: Frame,
    /// The profile's loops in (r, h), outer first, each segment's section in turn (a free curve's
    /// chord, the curve itself in `curves`).
    pub sections: Vec<Section>,
    pub curves: Vec<Option<Arc<Spline>>>,
    pub loops: Vec<usize>,
    pub a: f64,
    pub b: f64,
    pub angle: f64,
}

/// Winds a profile about an axis in its plane. `feature` is the history id the names derive
/// from (see `skin`).
pub fn coil(profile: &crate::Profile, axis: Axis2, w: &Winding, feature: u64) -> Result<Body, CoilError> {
    let (prep, wound) = wind(profile, axis, w)?;
    let Wound {
        frame: rf,
        sections: ring,
        a,
        b,
        angle: total,
        ..
    } = wound;
    let f0 = prep.profile.plane;
    let segs: Vec<Segment> = prep.segments().cloned().collect();
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
    let range = Interval::new(0.0, total);
    let surfaces = walls
        .into_iter()
        .zip(&segs)
        .map(|(w, sg)| match sg {
            Segment::Spline(c) => Swept::new(
                Spine::Helix {
                    axis: rf,
                    a,
                    b,
                    range,
                },
                c.c.clone(),
                &f0,
            )
            .expect("a profile's curves are regular")
            .surface(),
            _ => w.surface(),
        })
        .collect();
    Ok(Skin {
        loops: prep.loop_sizes(),
        keys: prep.keys,
        planes,
        curves,
        rails,
        walls: vec![surfaces],
        apex: [false; 2],
    }
    .build(Name::feature(feature)))
}

/// The motion and sections of a coil, checked, without building it.
pub fn coil_motion(profile: &crate::Profile, axis: Axis2, w: &Winding) -> Result<Wound, CoilError> {
    wind(profile, axis, w).map(|(_, m)| m)
}

fn wind(
    profile: &crate::Profile,
    axis: Axis2,
    w: &Winding,
) -> Result<(crate::skin::Prepared, Wound), CoilError> {
    let Winding {
        pitch,
        turns,
        taper,
        handed,
    } = *w;
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
    // left-handed: turning the right way about the reversed axis while falling along it
    let (z, sign) = match handed {
        Handed::Right => (z, 1.0),
        Handed::Left => (-z, -1.0),
    };
    let rf = Frame::from_zx(plane.plane_point(axis.origin), z, vec3(perp2 * side.signum()))
        .ok_or(CoilError::BadAxis)?;
    let prep = prepare(profile, rf.y().get())?;
    let f0 = prep.profile.plane;
    let ring: Vec<Section> = prep.segments().map(|s| ring_section(&rf, &f0, s)).collect();
    let curves: Vec<Option<(Spline, (f64, f64), (f64, f64))>> = prep
        .segments()
        .map(|s| match s {
            Segment::Spline(c) => Some(rh_curve(&rf, &f0, &c.c)),
            _ => None,
        })
        .collect();
    // a chord's extent, or the curve's where there is one
    let extent = |k: usize| match &curves[k] {
        Some((_, r, h)) => (*r, *h),
        None => ring[k].extent(),
    };
    let rmin = (0..ring.len())
        .map(|k| extent(k).0.0)
        .fold(f64::INFINITY, f64::min);
    if rmin <= tol::LINEAR {
        return Err(CoilError::CrossesAxis);
    }
    let (lo, hi) = (0..ring.len())
        .map(|k| extent(k).1)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |a, h| {
            (a.0.min(h.0), a.1.max(h.1))
        });
    if pitch <= hi - lo + tol::LINEAR {
        return Err(CoilError::TurnsTouch);
    }
    let angle = dmath::TAU * turns;
    let b = sign * pitch / dmath::TAU;
    let a = pitch / dmath::TAU * dmath::sin(taper) / dmath::cos(taper);
    if rmin + (a * angle).min(0.0) <= tol::LINEAR {
        return Err(CoilError::BadTaper);
    }
    let loops = prep.loop_sizes();
    Ok((
        prep,
        Wound {
            frame: rf,
            sections: ring,
            curves: curves.into_iter().map(|c| c.map(|(m, ..)| Arc::new(m))).collect(),
            loops,
            a,
            b,
            angle,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Profile;
    use crate::profile::ProfileLoop;
    use forge_math::Point2;
    use forge_topo::check;

    fn wind(pitch: f64, turns: f64, taper: f64, handed: Handed) -> Winding {
        Winding {
            pitch,
            turns,
            taper,
            handed,
        }
    }

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
            for handed in [Handed::Right, Handed::Left] {
                let b = coil(prof, up, &wind(4.0, 2.5, taper, handed), 1).unwrap();
                valid(&b);
            }
        }
        assert_eq!(
            coil(&bar, up, &wind(1.5, 2.0, 0.0, Handed::Right), 1).unwrap_err(),
            CoilError::TurnsTouch
        );
        assert_eq!(
            coil(&bar, up, &wind(4.0, 0.0, 0.0, Handed::Right), 1).unwrap_err(),
            CoilError::BadTurns
        );
        assert_eq!(
            coil(&bar, up, &wind(4.0, 10.0, -0.5, Handed::Right), 1).unwrap_err(),
            CoilError::BadTaper
        );
    }
}
