//! Drilled holes: plain, counterbored or countersunk, through or to a depth, with a drill point,
//! each made as one revolved cut.

use crate::pattern::{Source, corners, instances};
use crate::{Combine, Extent, Feature, Turn};
use forge_math::{Box3, Frame, Point2, Vec2, dmath};
use forge_ops::{Axis2, Profile, ProfileLoop, Segment};

#[derive(Clone, Debug, PartialEq)]
pub struct Hole {
    /// The mouth: origin at its centre, z the drilling direction (into the material).
    pub at: Frame,
    pub diameter: f64,
    pub head: Head,
    pub end: End,
    /// A blind hole's drill point, as the cone's included angle (118 degrees is usual); None
    /// for a flat bottom. A through hole has none.
    pub point: Option<f64>,
    /// A cosmetic thread on the wall: recorded for drawings and lookups, not modelled.
    pub thread: Option<Thread>,
    /// The screw and fit the diameter (and a tapped hole's thread) came from, if a standard's.
    pub standard: Option<crate::Standard>,
    /// Drilled at each of a sketch's points (`At::Sketch`, the first `at`'s origin), as one
    /// feature: the rest are copies of the first, named as a pattern names them.
    pub places: Option<crate::At>,
}

impl Hole {
    /// The moves from the first place to each other one, keyed as a pattern's copies.
    pub(crate) fn copies(&self) -> Vec<([u64; 2], crate::pattern::Motion)> {
        let Some(crate::At::Sketch { at, .. }) = &self.places else {
            return vec![];
        };
        at.iter()
            .enumerate()
            .skip(1)
            .map(|(k, &p)| {
                let o = forge_math::Point3::ORIGIN + (p - at[0]);
                (
                    [k as u64, 0],
                    crate::pattern::Motion::Rigid(Frame::WORLD.with_origin(o)),
                )
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Head {
    Plain,
    Counterbore {
        diameter: f64,
        depth: f64,
    },
    /// A cone from `diameter` at the mouth down to the hole, `angle` its included angle.
    Countersink {
        diameter: f64,
        angle: f64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum End {
    /// Through everything the part can hold.
    Through,
    /// The wall's depth below the mouth (the drill point goes further).
    Depth(f64),
    /// The wall down to a plane or face's surface, then `offset` further: flat on a plane
    /// square to the hole, cut off on any other surface (no drill point then).
    To { target: crate::Target, offset: f64 },
    /// Down to the next face of the part.
    ToNext,
}

/// Nominal (major) diameter and pitch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Thread {
    pub size: f64,
    pub pitch: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HoleError {
    /// A size that is not positive, a head no wider than the hole or as deep as it, or a cone
    /// angle outside (0, pi).
    BadSize,
}

/// Profile segment ids, which name the hole's faces.
const MOUTH: u64 = 0;
const BORE: u64 = 1;
const SEAT: u64 = 2;
const SINK: u64 = 3;
const WALL: u64 = 4;
pub(crate) const BOTTOM: u64 = 5;
const AXIS: u64 = 6;

fn tan_half(angle: f64) -> f64 {
    let (s, c) = dmath::sin_cos(angle / 2.0);
    s / c
}

fn cone_ok(a: f64) -> bool {
    a > 0.0 && a < dmath::PI
}

impl Hole {
    /// The revolve that cuts the hole; `reach` holds all the part's material (for Through).
    pub(crate) fn revolve(&self, reach: &Box3) -> Result<Feature, HoleError> {
        let r = self.diameter / 2.0;
        let pos = |v: f64| v.is_finite() && v > 0.0;
        if !pos(r) || !self.at.origin.is_finite() {
            return Err(HoleError::BadSize);
        }
        let depth = match self.end {
            End::Depth(d) if pos(d) => d,
            End::Depth(_) => return Err(HoleError::BadSize),
            End::To { .. } | End::ToNext => unreachable!("resolved to a depth first"),
            End::Through => {
                // the part's whole width along the axis, from wherever the mouth is on it
                let z = self.at.z();
                let (mut lo, mut hi) = (0.0f64, 0.0f64);
                if !reach.is_empty() {
                    for c in corners(reach) {
                        let t = z.dot(c - self.at.origin);
                        (lo, hi) = (lo.min(t), hi.max(t));
                    }
                }
                hi - lo + 1.0
            }
        };
        // (radius, depth) going down the outside of the hole
        let mut rim: Vec<(Point2, u64)> = Vec::new();
        let mouth = match self.head {
            Head::Plain => r,
            Head::Counterbore { diameter, depth: d } => {
                if !(pos(d) && diameter / 2.0 > r && d < depth) {
                    return Err(HoleError::BadSize);
                }
                rim.push((Point2::new(diameter / 2.0, d), BORE));
                rim.push((Point2::new(r, d), SEAT));
                diameter / 2.0
            }
            Head::Countersink { diameter, angle } => {
                let d = (diameter / 2.0 - r) / tan_half(angle);
                if !(cone_ok(angle) && diameter / 2.0 > r && d < depth) {
                    return Err(HoleError::BadSize);
                }
                rim.push((Point2::new(r, d), SINK));
                diameter / 2.0
            }
        };
        rim.push((Point2::new(r, depth), WALL));
        let tip = match (self.end, self.point) {
            (End::Depth(_), Some(a)) if cone_ok(a) => depth + r / tan_half(a),
            (End::Depth(_), Some(_)) => return Err(HoleError::BadSize),
            _ => depth,
        };
        rim.push((Point2::new(0.0, tip), BOTTOM));
        if let Some(t) = self.thread
            && !(pos(t.size) && pos(t.pitch))
        {
            return Err(HoleError::BadSize);
        }
        let mut pts = vec![(Point2::new(mouth, 0.0), MOUTH)];
        pts.extend(rim);
        pts.push((Point2::ORIGIN, AXIS));
        let n = pts.len();
        // segment k runs from point k - 1 to point k, ids naming the faces
        let segs = (0..n)
            .map(|k| {
                let a = if k == 0 { Point2::ORIGIN } else { pts[k - 1].0 };
                (Segment::Line { a, b: pts[k].0 }, pts[k].1)
            })
            .collect();
        // sketch x radial, sketch y down the drill
        let plane = Frame::from_zx(self.at.origin, -self.at.y(), self.at.x().get()).expect("a frame's axes");
        Ok(Feature::Revolve {
            profile: Profile {
                plane,
                loops: vec![ProfileLoop::new(segs)],
            },
            axis: Axis2 {
                origin: Point2::ORIGIN,
                dir: Vec2::Y,
            },
            extent: Turn::Angle(dmath::TAU),
            op: Combine::Cut,
        })
    }
}

fn segment_points(s: &Segment) -> Vec<Point2> {
    match *s {
        Segment::Line { a, b } => vec![a, b],
        Segment::Arc { center, radius, .. } | Segment::Circle { center, radius, .. } => {
            [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)]
                .map(|(x, y)| center + Vec2::new(x * radius, y * radius))
                .to_vec()
        }
    }
}

pub(crate) fn profile_points(p: &Profile) -> impl Iterator<Item = Point2> + '_ {
    p.loops
        .iter()
        .flat_map(|l| l.segments.iter().flat_map(|(s, _)| segment_points(s)))
}

/// A box holding a solid feature's tool.
fn tool_box(f: &Feature) -> Box3 {
    match f {
        Feature::Extrude {
            profile,
            extent: Extent::Distance(distance),
            ..
        } => Box3::from_points(profile_points(profile).flat_map(|q| {
            let p = profile.plane.plane_point(q);
            [p, p + profile.plane.z() * *distance]
        })),
        Feature::Revolve { profile, axis, .. } => {
            let dir = axis.dir.normalize().unwrap_or(Vec2::Y);
            let (mut lo, mut hi, mut r) = (f64::INFINITY, f64::NEG_INFINITY, 0.0f64);
            for q in profile_points(profile) {
                let h = (q - axis.origin).dot(dir);
                (lo, hi) = (lo.min(h), hi.max(h));
                r = r.max((q - axis.origin).dot(dir.perp()).abs());
            }
            let at = |h: f64| profile.plane.plane_point(axis.origin + dir * h);
            Box3::from_points([at(lo), at(hi)]).grow(r)
        }
        Feature::Ball { frame, radius, .. } => Box3::from_point(frame.origin).grow(*radius),
        _ => Box3::EMPTY,
    }
}

/// Boxes of the material feature `i` stands for (for a pattern of features: its sources' and
/// its copies'); none for cuts and part-changing features.
fn adds(features: crate::History<'_>, i: usize) -> Vec<Box3> {
    match &features[i] {
        Feature::Extrude { op, .. } | Feature::Revolve { op, .. } | Feature::Ball { op, .. }
            if matches!(op, Combine::New | Combine::Add) =>
        {
            match &features[i] {
                Feature::Ball { .. } => vec![tool_box(&features[i])],
                // a part it cannot be built is no part of the reach
                _ => match crate::extent::resolve(features, i) {
                    Ok(crate::extent::Resolved::Plain(f, _)) => vec![tool_box(&f)],
                    Ok(crate::extent::Resolved::UpTo(f, _)) => vec![tool_box(&f)],
                    Err(_) => vec![],
                },
            }
        }
        Feature::Pattern {
            of: Source::Features(src),
            layout,
        } => {
            let base: Vec<Box3> = src
                .iter()
                .filter(|&&s| s < i)
                .flat_map(|&s| adds(features, s))
                .collect();
            let copies = instances(features, i, layout).unwrap_or_default();
            let moved: Vec<Box3> = copies
                .iter()
                .flat_map(|(_, m)| base.iter().map(|b| m.bbox(b)))
                .collect();
            base.into_iter().chain(moved).collect()
        }
        // what is added by a tool without a box of its own: a through cut that left these out went
        // a millimetre into a loft and said nothing
        Feature::Sweep(_) | Feature::Coil(_) | Feature::Loft(_) => {
            crate::swept::reach_of(features, i).into_iter().collect()
        }
        Feature::Thread(_) | Feature::Text(_) => crate::tools(features, i, true)
            .map(|ts| {
                ts.iter()
                    .filter(|t| !matches!(t.1, Combine::Cut | Combine::Keep))
                    .map(|t| t.0.face_ids().fold(Box3::EMPTY, |b, f| b.hull(&t.0.face_box(f))))
                    .collect()
            })
            .unwrap_or_default(),
        _ => vec![],
    }
}

/// A box holding all the material `features` can make, to size through holes by.
pub(crate) fn reach(features: crate::History<'_>) -> Box3 {
    let mut b = Box3::EMPTY;
    for (i, f) in features.iter().enumerate() {
        match f {
            Feature::Shell {
                thickness,
                outward: true,
                ..
            } if !b.is_empty() => b = b.grow(*thickness),
            Feature::Draft { angle, .. } if !b.is_empty() => {
                b = b.grow(b.diagonal() * tan_half(2.0 * angle.abs()))
            }
            Feature::Edit(crate::Edit::Offset { distance, .. }) if !b.is_empty() => {
                b = b.grow(distance.abs())
            }
            Feature::Edit(crate::Edit::Thicken { thickness, .. }) if !b.is_empty() => {
                b = b.grow(thickness.abs())
            }
            Feature::Pattern {
                of: Source::Body,
                layout,
            } => {
                for (_, m) in instances(features, i, layout).unwrap_or_default() {
                    b = b.hull(&m.bbox(&b));
                }
            }
            Feature::Body(bf) => {
                if let crate::BodyFeature::Move { steps, .. } = &**bf {
                    let m = crate::bodies::motion(steps);
                    b = b.hull(&m.bbox(&b));
                }
            }
            _ => {
                for a in adds(features, i) {
                    b = b.hull(&a);
                }
            }
        }
    }
    b
}
