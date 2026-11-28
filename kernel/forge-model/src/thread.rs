//! Modeled screw threads: the 60 degree basic form ISO metric and Unified threads share (crest
//! flat P/8 at the major diameter, root flat P/4 at the minor, flanks 5H/8 deep, H = sqrt3/2 P),
//! cut as a coil round a cylinder: outside for a bolt, inside for a nut. Every flank and flat is
//! an exact screw surface. A chamfered start is a cone cut at the mouth; a run-out carries the
//! groove a turn on, a tapered coil that draws it out of the material.

use crate::pattern::{instance_name, instances, moved, moved_shape};
use crate::{Combine, Feature, History, Hole, RegenError, Source, Target, Tool, Turn};
use forge_geom::{CurveGeom, Surface};
use forge_math::{Frame, Point2, Vec2, dmath};
use forge_ops::{Axis2, CoilError, Handed, Profile, ProfileLoop, Segment, Winding, coil, coil_motion};
use forge_topo::{Body, Name};

/// A thread's size: both standards' sizes come down to a major diameter and a pitch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Spec {
    /// ISO metric: major diameter and pitch, mm.
    Iso { major: f64, pitch: f64 },
    /// Unified (UTS): major diameter in inches and threads per inch.
    Uts { major: f64, tpi: f64 },
}

/// ISO metric coarse pitches by nominal diameter.
const COARSE: [(f64, f64); 22] = [
    (1.0, 0.25),
    (1.2, 0.25),
    (1.6, 0.35),
    (2.0, 0.4),
    (2.5, 0.45),
    (3.0, 0.5),
    (4.0, 0.7),
    (5.0, 0.8),
    (6.0, 1.0),
    (8.0, 1.25),
    (10.0, 1.5),
    (12.0, 1.75),
    (14.0, 2.0),
    (16.0, 2.0),
    (20.0, 2.5),
    (24.0, 3.0),
    (30.0, 3.5),
    (36.0, 4.0),
    (42.0, 4.5),
    (48.0, 5.0),
    (56.0, 5.5),
    (64.0, 6.0),
];

impl Spec {
    /// Major diameter, mm.
    pub fn major(&self) -> f64 {
        match *self {
            Spec::Iso { major, .. } => major,
            Spec::Uts { major, .. } => major * 25.4,
        }
    }

    /// Pitch, mm.
    pub fn pitch(&self) -> f64 {
        match *self {
            Spec::Iso { pitch, .. } => pitch,
            Spec::Uts { tpi, .. } => 25.4 / tpi,
        }
    }

    /// `M6` (coarse), `M6x0.75`, `1/4-20`, `#10-24` or `0.25-20`.
    pub fn parse(s: &str) -> Option<Spec> {
        let num = |t: &str| t.parse::<f64>().ok().filter(|x| x.is_finite() && *x > 0.0);
        if let Some(m) = s.strip_prefix('M') {
            let (major, pitch) = match m.split_once('x') {
                Some((d, p)) => (num(d)?, num(p)?),
                None => {
                    let d = num(m)?;
                    (d, COARSE.iter().find(|c| c.0 == d)?.1)
                }
            };
            return Some(Spec::Iso { major, pitch });
        }
        let (d, n) = s.split_once('-')?;
        let major = if let Some(g) = d.strip_prefix('#') {
            let g: u32 = g.parse().ok().filter(|g| *g <= 12)?;
            0.06 + 0.013 * g as f64
        } else if let Some((a, b)) = d.split_once('/') {
            num(a)? / num(b)?
        } else {
            num(d)?
        };
        Some(Spec::Uts { major, tpi: num(n)? })
    }

    /// The designation `parse` reads back to this spec exactly.
    pub fn designation(&self) -> String {
        match *self {
            Spec::Iso { major, pitch } => format!("M{major}x{pitch}"),
            Spec::Uts { major, tpi } => format!("{major}-{tpi}"),
        }
    }
}

/// Where the thread goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum On {
    /// About `at`'s z: starting at its origin, running along z into the material; inside a bore
    /// of the minor diameter (`internal`) or round a rod of the major.
    Axis { at: Frame, spec: Spec, internal: bool },
    /// On a cylinder face of an earlier feature (outside or inside, as the face looks), from the
    /// face's end furthest along the cylinder's axis, or its other end if `flip`.
    Face {
        feature: usize,
        name: Name,
        spec: Spec,
        flip: bool,
    },
    /// The cosmetic thread of hole feature `usize`, and of every copy of it a pattern made.
    Hole(usize),
}

/// How far the thread runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Length {
    /// The whole face, or the hole's depth (through the part for a through hole).
    Full,
    Distance(f64),
    /// To where the axis meets a plane, then `offset` on.
    To {
        target: Target,
        offset: f64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrewThread {
    pub on: On,
    pub length: Length,
    pub handed: Handed,
    /// A 45 degree chamfer at the start, just past the thread's root.
    pub chamfer: bool,
    /// The groove drawn out of the material over a turn past its end, not stopped square.
    pub runout: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ThreadError {
    /// A size or length that is not positive, or a face whose radius the spec's form doesn't fit.
    BadSize,
    /// The face is not a cylinder, or not among the feature's.
    NotCylinder,
    /// The feature is not a hole with a cosmetic thread.
    NotThreaded,
    /// A full-length thread on a hole that ends at a surface: give a length.
    NeedsLength,
    Coil(CoilError),
}

/// One thread to cut: origin on the axis where it starts, z along it into the material.
struct Placed {
    at: Frame,
    internal: bool,
    major: f64,
    pitch: f64,
    length: f64,
}

fn err(i: usize) -> impl Fn(ThreadError) -> RegenError {
    move |error| RegenError::Thread { feature: i, error }
}

/// The tools thread feature `i` cuts with: a coil and a chamfer cone per
/// thread, a pattern's copies of a hole's included.
pub(crate) fn tools(features: History<'_>, i: usize, build: bool) -> Result<Vec<Tool>, RegenError> {
    let Feature::Thread(t) = &features[i] else {
        unreachable!("only threads")
    };
    placed_tools(features, i, build, &|p| cut(i, features.id(i), p, t, build))
}

/// The rings thread feature `i`'s grooves lie in (`envelope`), one per thread it cuts, as solids
/// named from feature id `id`.
pub(crate) fn envelopes(features: History<'_>, i: usize, id: u64) -> Result<Vec<Body>, RegenError> {
    let Feature::Thread(t) = &features[i] else {
        unreachable!("only threads")
    };
    let rings = placed_tools(features, i, true, &|p| envelope(i, id, p, t))?;
    Ok(rings.into_iter().map(|r| r.0).collect())
}

/// `make`'s tools for each thread feature `i` cuts, a pattern's copies of a hole's included.
fn placed_tools(
    features: History<'_>,
    i: usize,
    build: bool,
    make: &dyn Fn(&Placed) -> Result<Vec<Tool>, RegenError>,
) -> Result<Vec<Tool>, RegenError> {
    let Feature::Thread(t) = &features[i] else {
        unreachable!("only threads")
    };
    let e = err(i);
    let place = |at: Frame, internal: bool, spec: Spec, full: f64| -> Result<Placed, RegenError> {
        let length = match t.length {
            Length::Full => full,
            Length::Distance(d) => d,
            Length::To { target, offset } => {
                let p = crate::extent::plane(features, i, &target)?;
                let across = p.z().dot(at.z().get());
                if across.abs() < 1e-9 {
                    return Err(e(ThreadError::BadSize));
                }
                p.z().dot(p.origin - at.origin) / across + offset
            }
        };
        let (major, pitch) = (spec.major(), spec.pitch());
        if !(length > 0.0 && length.is_finite() && major > 0.0 && pitch > 0.0 && pitch < major) {
            return Err(e(ThreadError::BadSize));
        }
        Ok(Placed {
            at,
            internal,
            major,
            pitch,
            length,
        })
    };
    match t.on {
        On::Axis { at, spec, internal } => make(&place(at, internal, spec, f64::NAN)?),
        On::Face {
            feature,
            name,
            spec,
            flip,
        } => {
            let (at, internal, radius, full) = face_axis(features, i, feature, name, flip)?;
            let p = place(at, internal, spec, full)?;
            let (rmin, rmax) = form(p.major, p.pitch, internal).span;
            if !(radius > rmin + 1e-6 && radius < rmax - 1e-6) {
                return Err(e(ThreadError::BadSize));
            }
            make(&p)
        }
        On::Hole(k) => {
            let Some(Feature::Hole(h)) = features.get(k).filter(|_| k < i) else {
                return Err(e(ThreadError::NotThreaded));
            };
            let th = h.thread.ok_or(e(ThreadError::NotThreaded))?;
            let full = hole_depth(features, k, h)?.ok_or(e(ThreadError::NeedsLength));
            let full = match t.length {
                Length::Full => full?,
                _ => f64::NAN,
            };
            let spec = Spec::Iso {
                major: th.size,
                pitch: th.pitch,
            };
            let p = place(h.at, true, spec, full)?;
            let (rmin, rmax) = form(p.major, p.pitch, true).span;
            if !(h.diameter / 2.0 > rmin + 1e-6 && h.diameter / 2.0 < rmax - 1e-6) {
                return Err(e(ThreadError::BadSize));
            }
            let mut out = make(&p)?;
            // at the hole's other places, then copies by the patterns after it that hold it, as
            // `Part::threads` finds them
            let mut placed = Vec::new();
            for (key, m) in h.copies() {
                for (b, op, sh) in &out {
                    let b = if build {
                        moved(b, &m, |n| instance_name(features.id(k), key, n))
                            .map_err(|error| RegenError::Pattern { feature: i, error })?
                    } else {
                        Body::new()
                    };
                    placed.push((b, *op, moved_shape(sh, &m)));
                }
            }
            out.extend(placed);
            let mut held = vec![k];
            for (j, f) in features.iter().enumerate().take(i).skip(k + 1) {
                let Feature::Pattern { of, layout } = f else {
                    continue;
                };
                let holds = match of {
                    Source::Body => true,
                    Source::Features(src) => src.iter().any(|s| held.contains(s)),
                };
                if !holds {
                    continue;
                }
                held.push(j);
                let copies = instances(features, j, layout)?;
                let mut added = Vec::new();
                for (key, m) in &copies {
                    for (b, op, sh) in &out {
                        let b = if build {
                            moved(b, m, |n| instance_name(features.id(j), *key, n))
                                .map_err(|error| RegenError::Pattern { feature: i, error })?
                        } else {
                            Body::new()
                        };
                        added.push((b, *op, moved_shape(sh, m)));
                    }
                }
                out.extend(added);
            }
            Ok(out)
        }
    }
}

/// The depth a hole's wall runs, through the part for a through hole; None if it ends at a
/// surface that is not a plane square to it.
fn hole_depth(features: History<'_>, k: usize, h: &Hole) -> Result<Option<f64>, RegenError> {
    let (h, up, _) = crate::extent::hole_end(features, k, h)?;
    Ok(match (h.end, up) {
        (crate::End::Depth(d), None) => Some(d),
        (crate::End::Through, _) => {
            let reach = crate::hole::reach(features.upto(k));
            let z = h.at.z();
            let (mut lo, mut hi) = (0.0f64, 0.0f64);
            for c in crate::pattern::corners(&reach) {
                let t = z.dot(c - h.at.origin);
                (lo, hi) = (lo.min(t), hi.max(t));
            }
            Some(hi - lo + 1.0)
        }
        _ => None,
    })
}

/// A cylinder face's thread axis (origin at the starting end, z along the face), whether it is
/// inside a bore, its radius and its length.
fn face_axis(
    features: History<'_>,
    i: usize,
    k: usize,
    name: Name,
    flip: bool,
) -> Result<(Frame, bool, f64, f64), RegenError> {
    let e = err(i);
    if k >= i || crate::modifies(&features[k]) {
        return Err(e(ThreadError::NotCylinder));
    }
    for (body, ..) in crate::tools(features, k, true)? {
        let Some(f) = body.face_by_name(name) else {
            continue;
        };
        let face = body.face(f);
        let Surface::Cylinder(c) = face.surface else {
            return Err(e(ThreadError::NotCylinder));
        };
        let z = c.frame.z();
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for &l in &face.loops {
            for ce in body.lp(l).coedges() {
                let ed = body.edge(body.coedge(ce).edge);
                for s in 0..=8 {
                    let p = ed.curve.eval(ed.range.lo + ed.range.width() * s as f64 / 8.0);
                    let t = z.dot(p - c.frame.origin);
                    (lo, hi) = (lo.min(t), hi.max(t));
                }
            }
        }
        let (start, dir) = if flip { (lo, z) } else { (hi, -z) };
        let at =
            Frame::from_zx(c.frame.origin + z * start, dir, c.frame.x().get()).expect("a cylinder's axes");
        return Ok((at, !face.same_sense, c.radius, hi - lo));
    }
    Err(e(ThreadError::NotCylinder))
}

/// The groove's section in (r, h) about h = 0, and the radii the cylinder may have: the form's
/// root and flanks, its flanks carried on past the cylinder by P/16.
struct Form {
    groove: [Point2; 6],
    span: (f64, f64),
    /// The thread's minor and major radii.
    minor: f64,
    major: f64,
}

fn form(major: f64, pitch: f64, internal: bool) -> Form {
    let p = pitch;
    let depth = 5.0 / 16.0 * 3f64.sqrt() * p;
    let (rj, rn) = (major / 2.0, major / 2.0 - depth);
    let slope = 1.0 / 3f64.sqrt();
    let past = p / 16.0;
    let groove = if internal {
        // the nut's root flat P/8 at the major radius, the bore at the minor; on into the bore
        // square, well clear of its wall
        let r = rn - past;
        let w = p / 16.0 + (rj - r) * slope;
        let far = r - p.min(r / 2.0);
        [
            Point2::new(rj, -p / 16.0),
            Point2::new(rj, p / 16.0),
            Point2::new(r, w),
            Point2::new(far, w),
            Point2::new(far, -w),
            Point2::new(r, -w),
        ]
    } else {
        // the bolt's root flat P/4 at the minor radius, the rod at the major; on out square
        let r = rj + past;
        let w = p / 8.0 + (r - rn) * slope;
        [
            Point2::new(rn, p / 8.0),
            Point2::new(rn, -p / 8.0),
            Point2::new(r, -w),
            Point2::new(r + p, -w),
            Point2::new(r + p, w),
            Point2::new(r, w),
        ]
    };
    let span = if internal {
        (rn - past, rj)
    } else {
        (rn, rj + past)
    };
    Form {
        groove,
        span,
        minor: rn,
        major: rj,
    }
}

fn polygon(pts: &[Point2], first_id: u64) -> ProfileLoop {
    let n = pts.len();
    ProfileLoop::new(
        (0..n)
            .map(|k| {
                (
                    Segment::Line {
                        a: pts[k],
                        b: pts[(k + 1) % n],
                    },
                    first_id + k as u64,
                )
            })
            .collect(),
    )
}

/// Profile ids: the groove's, the chamfer's and the run-out's segments.
const GROOVE: u64 = 0;
const CHAMFER: u64 = 10;
const RUNOUT: u64 = 20;
/// The run-out's coil is named from the thread's id moved on by this.
const RUNOUT_ID: u64 = 5_000_000;

/// The names of a thread's faces by role, from its history id. The groove (and the run-out) is
/// a coil: its walls are named as `forge_ops`'s skins name them, one per profile segment; the
/// chamfer is a revolve, a whole turn in two halves. Of the groove's section, segment 0 is the
/// thread's root flat (at the major diameter in a nut, the minor on a bolt), 1 and 5 its flanks.
/// A hole's thread at the hole's other places, or in a pattern's copies, is salted as
/// `instance_name` salts.
pub mod roles {
    use forge_topo::Name;

    fn coil_wall(id: u64, segment: u64) -> Name {
        Name::feature(id)
            .with_str("side")
            .with_name(Name::NONE.with(segment).with(0))
            .with(0)
    }

    /// The root flat between the flanks.
    pub fn root_flat(id: u64) -> Name {
        coil_wall(id, super::GROOVE)
    }

    /// The two flanks of the groove, then the two of its run-out (if it has one).
    pub fn flanks(id: u64) -> [Name; 4] {
        let r = id + super::RUNOUT_ID;
        [
            coil_wall(id, super::GROOVE + 1),
            coil_wall(id, super::GROOVE + 5),
            coil_wall(r, super::RUNOUT + 1),
            coil_wall(r, super::RUNOUT + 5),
        ]
    }

    /// The run-out's root flat.
    pub fn runout_flat(id: u64) -> Name {
        coil_wall(id + super::RUNOUT_ID, super::RUNOUT)
    }

    /// The 45 degree chamfer cone at the start.
    pub fn chamfer(id: u64) -> [Name; 2] {
        [0, 1].map(|s| forge_ops::roles::revolve_side(Name::feature(id), super::CHAMFER + 2, s))
    }
}

fn cut(i: usize, id: u64, p: &Placed, t: &ScrewThread, build: bool) -> Result<Vec<Tool>, RegenError> {
    let e = err(i);
    let f = form(p.major, p.pitch, p.internal);
    let pitch = p.pitch;
    // the sketch: x out from the axis, y along it
    let plane = Frame::from_zx(p.at.origin, -p.at.y(), p.at.x().get()).expect("a frame's axes");
    let axis = Axis2 {
        origin: Point2::ORIGIN,
        dir: Vec2::Y,
    };
    // the groove starts a pitch back, clear of the material, and its full depth ends at the length
    let start = -pitch;
    let turns = (p.length - pitch / 2.0 - start) / pitch;
    let shifted = |h0: f64| -> Vec<Point2> { f.groove.iter().map(|q| Point2::new(q.x, q.y + h0)).collect() };
    let profile = Profile {
        plane,
        loops: vec![polygon(&shifted(start), GROOVE)],
    };
    let winding = |turns: f64, taper: f64| Winding {
        pitch,
        turns,
        taper,
        handed: t.handed,
    };
    let coil_tool = |profile: &Profile, w: &Winding, id: u64| -> Result<Tool, RegenError> {
        let ce = |error| e(ThreadError::Coil(error));
        let m = coil_motion(profile, axis, w).map_err(ce)?;
        let shape = crate::swept::coil_shape(&m);
        let body = if build {
            coil(profile, axis, w, id).map_err(ce)?
        } else {
            Body::new()
        };
        Ok((body, Combine::Cut, shape))
    };
    let mut out = Vec::new();
    let main = coil_tool(&profile, &winding(turns, 0.0), id)?;
    if t.runout {
        // a second groove, a hair smaller, from an eighth of a turn before the groove's end
        // (inside it, so no faces lie on each other) on a turn past it, drifting out of the
        // material past its depth
        let back = dmath::TAU / 8.0;
        let angle = turns * dmath::TAU - back;
        let end = start + angle / dmath::TAU * pitch;
        let (sn, cs) = dmath::sin_cos(match t.handed {
            Handed::Right => angle,
            Handed::Left => -angle,
        });
        let x = p.at.x() * cs + p.at.y() * sn;
        let z = x.cross(p.at.z().get()).normalize().expect("square axes");
        let turned = Frame::from_zx(p.at.origin + p.at.z() * end, z, x).expect("a frame's axes");
        let out_by = (f.major - f.minor) + pitch / 4.0;
        let drift = if p.internal { -out_by } else { out_by };
        // its far side drawn halfway in and the whole a sixteenth smaller, so while it overlaps
        // the groove it lies wholly inside it and leaves through the groove's end alone
        let mut g = f.groove;
        let mid = |a: Point2, b: Point2| Point2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
        (g[3], g[4]) = (mid(g[2], g[3]), mid(g[5], g[4]));
        let c = g.iter().fold(Point2::ORIGIN, |m, q| {
            Point2::new(m.x + q.x / 6.0, m.y + q.y / 6.0)
        });
        let smaller: Vec<Point2> = g.iter().map(|q| c + (*q - c) * (15.0 / 16.0)).collect();
        let tail = Profile {
            plane: turned,
            loops: vec![polygon(&smaller, RUNOUT)],
        };
        let turns = 1.0 + back / dmath::TAU;
        let a = drift / (turns * pitch);
        out.push(main);
        out.push(coil_tool(&tail, &winding(turns, dmath::atan(a)), id + RUNOUT_ID)?);
    } else {
        out.push(main);
    }
    if t.chamfer {
        // 45 degrees from just past the root at the mouth out past the cylinder, starting
        // clear of it (not through the root's own edge)
        let (m, e) = (pitch.min(f.minor / 4.0), pitch / 16.0);
        let tri = if p.internal {
            let (c, r) = (f.major + e, f.minor - m);
            [Point2::new(c + m, -m), Point2::new(r, -m), Point2::new(r, c - r)]
        } else {
            let (c, r) = (f.minor - e, f.major + m);
            [Point2::new(c - m, -m), Point2::new(r, -m), Point2::new(r, r - c)]
        };
        let cone = Feature::Revolve {
            profile: Profile {
                plane,
                loops: vec![polygon(&tri, CHAMFER)],
            },
            axis,
            extent: Turn::Angle(dmath::TAU),
            op: Combine::Cut,
        };
        // first: the groove then cuts the cone, not the cone the groove
        out.insert(0, crate::tool(i, id, &cone, build)?);
    }
    Ok(out)
}

/// The ring about a thread's axis its grooves, chamfer and run-out lie in, P/8 past them all round:
/// the groove's section out to where it runs on square past the cylinder (inside a collar the
/// thread starts at, that part is cut too), the run-out's drift, from a pitch and a half before its
/// start to past its end. A shell hollows the part as if the thread were not there: less this ring,
/// the walls are measured from just past the thread's far radius (the major inside a bore, the
/// minor round a rod).
fn envelope(i: usize, id: u64, p: &Placed, t: &ScrewThread) -> Result<Vec<Tool>, RegenError> {
    let f = form(p.major, p.pitch, p.internal);
    let q = p.pitch;
    let drift = if t.runout {
        (f.major - f.minor) + q / 4.0
    } else {
        0.0
    };
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for g in &f.groove {
        (lo, hi) = (lo.min(g.x), hi.max(g.x));
    }
    let (lo, hi) = if p.internal {
        (lo - drift - q / 8.0, hi + q / 8.0)
    } else {
        (lo - q / 8.0, hi + drift + q / 8.0)
    };
    let lo = lo.max(0.0);
    let end = p.length + if t.runout { 1.25 * q } else { q / 4.0 };
    let plane = Frame::from_zx(p.at.origin, -p.at.y(), p.at.x().get()).expect("a frame's axes");
    let ring = Feature::Revolve {
        profile: Profile {
            plane,
            loops: vec![polygon(
                &[
                    Point2::new(lo, -1.5 * q),
                    Point2::new(hi, -1.5 * q),
                    Point2::new(hi, end),
                    Point2::new(lo, end),
                ],
                0,
            )],
        },
        axis: Axis2 {
            origin: Point2::ORIGIN,
            dir: Vec2::Y,
        },
        extent: Turn::Angle(dmath::TAU),
        op: Combine::Cut,
    };
    Ok(vec![crate::tool(i, id, &ring, true)?])
}

/// The groove's volume per radian of turn, inside a cylinder of the form's own radius: the
/// integral of r over the thread form's groove (Pappus, as the screw motion carries it).
pub fn groove_moment(major: f64, pitch: f64, internal: bool) -> f64 {
    let f = form(major, pitch, internal);
    let slope = 1.0 / 3f64.sqrt();
    let (a, b) = (f.minor, f.major);
    // width w(r) = w0 + k (r - a) over [a, b]
    let (w0, k) = if internal {
        (pitch * 3.0 / 4.0, -2.0 * slope)
    } else {
        (pitch / 4.0, 2.0 * slope)
    };
    let d = b - a;
    // integral of r (w0 + k (r - a)) dr from a to b
    w0 * (b * b - a * a) / 2.0 + k * (d * d * d / 3.0 + a * d * d / 2.0)
}
