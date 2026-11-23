//! Drilled holes: plain, counterbored or countersunk, through or to a depth, with a drill point,
//! each made as one revolved cut.

use crate::pattern::{Source, corners, instances};
use crate::{Combine, Extent, Feature, Turn};
use forge_math::{Box3, Frame, Interval, Point2, Vec2, dmath};
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
const BOTTOM: u64 = 5;
const AXIS: u64 = 6;

/// The names of a hole's faces by role, from its root (`Name::feature(id)`): each is the revolve
/// wall of one profile segment, a whole turn built as two halves (`forge_ops::roles::revolve_side`).
/// At a hole's other places, and in a pattern's copies, each is salted as `instance_name` salts.
pub mod roles {
    use forge_ops::roles::revolve_side;
    use forge_topo::Name;

    fn halves(root: Name, segment: u64) -> [Name; 2] {
        [0, 1].map(|s| revolve_side(root, segment, s))
    }

    /// A counterbore's wall.
    pub fn bore(root: Name) -> [Name; 2] {
        halves(root, super::BORE)
    }

    /// A counterbore's floor, the ring the head sits on.
    pub fn seat(root: Name) -> [Name; 2] {
        halves(root, super::SEAT)
    }

    /// A countersink's cone.
    pub fn sink(root: Name) -> [Name; 2] {
        halves(root, super::SINK)
    }

    /// The drilled wall.
    pub fn wall(root: Name) -> [Name; 2] {
        halves(root, super::WALL)
    }

    /// The flat bottom or the drill point's cone.
    pub fn bottom(root: Name) -> [Name; 2] {
        halves(root, super::BOTTOM)
    }
}

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

fn segment_points(s: &Segment) -> impl Iterator<Item = Point2> {
    let (pts, n) = match *s {
        Segment::Spline(ref c) => {
            let b = c.c.bbox(c.c.domain());
            (
                [
                    (b.x.lo, b.y.lo),
                    (b.x.lo, b.y.hi),
                    (b.x.hi, b.y.lo),
                    (b.x.hi, b.y.hi),
                ]
                .map(|(x, y)| Point2::new(x, y)),
                4,
            )
        }
        Segment::Line { a, b } => ([a, b, a, b], 2),
        Segment::Arc { center, radius, .. } | Segment::Circle { center, radius, .. } => (
            [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)]
                .map(|(x, y)| center + Vec2::new(x * radius, y * radius)),
            4,
        ),
    };
    pts.into_iter().take(n)
}

pub(crate) fn profile_points(p: &Profile) -> impl Iterator<Item = Point2> + '_ {
    p.loops
        .iter()
        .flat_map(|l| l.segments.iter().flat_map(|(s, _)| segment_points(s)))
}

/// The box of a profile's points on its plane.
fn plane_box(profile: &Profile) -> Box3 {
    Box3::from_points(profile_points(profile).map(|q| profile.plane.plane_point(q)))
}

/// `b` moved by `v`: the box of its points each moved by `v`, to the bit (rounding `+ v` is
/// monotone, so the least and greatest coordinates move to the least and greatest).
fn shifted(b: &Box3, v: forge_math::Vec3) -> Box3 {
    if b.is_empty() {
        return *b;
    }
    let m = |i: Interval, d: f64| Interval {
        lo: i.lo + d,
        hi: i.hi + d,
    };
    Box3 {
        x: m(b.x, v.x),
        y: m(b.y, v.y),
        z: m(b.z, v.z),
    }
}

/// A box holding a solid feature's tool.
fn tool_box(f: &Feature) -> Box3 {
    match f {
        Feature::Extrude {
            profile,
            extent: Extent::Distance(distance),
            ..
        } => {
            let b = plane_box(profile);
            b.hull(&shifted(&b, profile.plane.z() * *distance))
        }
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
        // a slanted extrude: a ruled loft between the profile moved to each end (lib.rs `tool`),
        // so in the hull of the two moved boxes
        Feature::Extrude {
            profile,
            extent: Extent::Along { forward, back, dir },
            ..
        } => {
            let Some(unit) = dir.normalize() else {
                return Box3::EMPTY;
            };
            let rise = unit.dot(profile.plane.z().get());
            if rise == 0.0 || !rise.is_finite() {
                return Box3::EMPTY;
            }
            let b = plane_box(profile);
            shifted(&b, unit.get() * (-*back / rise)).hull(&shifted(&b, unit.get() * (*forward / rise)))
        }
        // a tapered extrude: the profile's prism, its walls leaning by at most |d tan a| (the
        // loop offset by that much at the far end, `forge_ops::tapered`)
        Feature::Extrude {
            profile,
            extent: Extent::Tapered { distance, angle },
            ..
        } => {
            let lean = (*distance * dmath::sin(*angle) / dmath::cos(*angle)).abs();
            let b = plane_box(profile);
            b.hull(&shifted(&b, profile.plane.z() * *distance))
                .grow(if lean.is_finite() { lean } else { 0.0 })
        }
        _ => Box3::EMPTY,
    }
}

/// `adds`, hulled into `b`: the common features (an extrude, revolve or ball) without a list.
fn add_into(features: crate::History<'_>, i: usize, b: &mut Box3) {
    match &features[i] {
        Feature::Ball {
            op: Combine::New | Combine::Add,
            ..
        } => *b = b.hull(&tool_box(&features[i])),
        Feature::Extrude {
            op: Combine::New | Combine::Add,
            ..
        }
        | Feature::Revolve {
            op: Combine::New | Combine::Add,
            ..
        } => {
            // a part it cannot be built is no part of the reach (the build of feature i, from
            // the same `resolve`, fails the same way, so no later feature is built)
            match crate::extent::resolve(features, i) {
                Ok(crate::extent::Resolved::Plain(f, _)) => *b = b.hull(&tool_box(&f)),
                Ok(crate::extent::Resolved::UpTo(f, _)) => *b = b.hull(&tool_box(&f)),
                Err(_) => {}
            }
        }
        Feature::Extrude { .. } | Feature::Revolve { .. } | Feature::Ball { .. } => {}
        _ => {
            for a in adds(features, i) {
                *b = b.hull(&a);
            }
        }
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
        Feature::Import(t) => t
            .body
            .iter()
            .filter(|_| !matches!(t.op, Combine::Cut | Combine::Keep))
            .map(|b| b.faces_box())
            .collect(),
        Feature::Text(t) if matches!(t.op, Combine::Cut | Combine::Keep) => vec![],
        Feature::Text(_) => crate::tools(features, i, true)
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

/// A feature whose material the history gives no sound box for: faces moved, replaced,
/// deleted, offset or drafted, or an outward shell (their faces' neighbours are
/// re-intersected, so an edge can slide far past any box grown by the distance: by d / sin θ
/// for faces at θ), or a feature on other bodies. Each is a part-changing feature, so the regen
/// has the part as built after it (`Given::base`).
pub(crate) fn unbounded(f: &Feature) -> bool {
    match f {
        // a rib is bounded (`rib_reach`), and so is a thickening: a slab of the faces themselves,
        // offset by its thickness with ruled sides, no neighbour re-intersected (`reach` grows by it)
        Feature::Edit(e) => !matches!(e, crate::Edit::Rib(_) | crate::Edit::Thicken { .. }),
        Feature::Draft { .. } => true,
        Feature::Shell { outward, .. } => *outward,
        Feature::Body(b) => matches!(**b, crate::BodyFeature::In { .. }),
        _ => false,
    }
}

/// A feature whose tool is sized by `reach`: a through hole, a through extrude. (A thread on a
/// through hole takes its full length from the hole's own step, `thread::hole_depth`.)
pub(crate) fn thru_consumer(f: &Feature) -> bool {
    match f {
        Feature::Hole(h) => matches!(h.end, End::Through),
        Feature::Extrude { extent, .. } => matches!(extent, Extent::Through(_) | Extent::ThroughAlong { .. }),
        _ => false,
    }
}

/// Whether feature `i`'s tool needs the part's box as built (`Given::base`).
#[inline]
pub(crate) fn needs_base(features: crate::History<'_>, i: usize) -> bool {
    // most features are none of these: a test of the kind, inline, and no more
    match &features[i] {
        Feature::Hole(h) if matches!(h.end, End::Through) => {}
        Feature::Extrude {
            extent: Extent::Through(_) | Extent::ThroughAlong { .. },
            ..
        } => {}
        Feature::Pattern {
            of: Source::Features(_),
            ..
        } => {}
        _ => return false,
    }
    needs_base_cold(features, i)
}

#[cold]
#[inline(never)]
fn needs_base_cold(features: crate::History<'_>, i: usize) -> bool {
    sized_through(features, i) && features[..i].iter().any(unbounded)
}

/// Feature `i` makes a tool sized by `reach`: a through tool, or a pattern copying one.
pub(crate) fn sized_through(features: crate::History<'_>, i: usize) -> bool {
    match &features[i] {
        Feature::Pattern {
            of: Source::Features(src),
            ..
        } => src.iter().any(|&s| s < i && sized_through(features, s)),
        f => thru_consumer(f),
    }
}

/// A box holding the body: what the regen hands a through tool as the part's extent. Any box that
/// holds the part sizes a through tool to the same cut (research/ops-spec, H1), so this one is
/// sound and cheap rather than tight: the hull of the vertices and the curved edges' boxes (kept
/// ones, else a circle's or ellipse's whole box, else `Curve::bbox` over the range), which holds every plane, cylinder, cone and ruled face (a straight line through any of
/// their points meets the boundary on both sides), with a sphere's or torus's whole box for the
/// faces that can bulge past their edges, and the face's own box for any other procedural face.
pub(crate) fn body_box(b: &forge_topo::Body) -> Box3 {
    use forge_geom::{Curve, Surface};
    // every edge's ends: so a line edge adds nothing more
    let mut bx = Box3::from_points(b.vertex_ids().map(|v| b.vertex(v).point));
    for e in b.edge_ids() {
        let edge = b.edge(e);
        // a curved edge's box if it is kept already, else one as cheap as sound: a circle's or
        // ellipse's whole box (no arc extremes worked out)
        let eb = match (&edge.curve, edge.bbox.get()) {
            (Curve::Line(_), _) => continue,
            (_, Some(k)) => *k,
            (Curve::Circle(c), None) => Box3::from_point(c.frame.origin).grow(c.radius.abs()),
            (Curve::Ellipse(el), None) => {
                Box3::from_point(el.frame.origin).grow(el.major.abs().max(el.minor.abs()))
            }
            (_, None) => b.edge_box(e),
        };
        bx = bx.hull(&eb);
    }
    for f in b.face_ids() {
        match &b.face(f).surface {
            Surface::Sphere(s) => bx = bx.hull(&Box3::from_point(s.frame.origin).grow(s.radius)),
            Surface::Torus(t) => bx = bx.hull(&Box3::from_point(t.frame.origin).grow(t.major + t.minor)),
            Surface::Procedural(pr) if pr.v_degree() == Some(1) => {}
            Surface::Procedural(_) => bx = bx.hull(&b.face_box(f)),
            _ => {}
        }
    }
    bx.grow(forge_math::tol::LINEAR)
}

/// A box holding the part after rib `r` is added to a part inside `b`.
///
/// Every point of the rib's lump lies on a segment from a point within `t` of the path's plane
/// region (inside the path's box grown by `t`) to the part or to the path: the wall is the region
/// between the path and the part, thickened across the plane (or, for a web, the thickened path
/// run along the normal to the part). So it lies in the hull of `b` and the path's box, grown by
/// the thickness, the taper and the millimetre `rib` allows (rib.rs, `margin`); boxes are convex.
/// The path's box is taken from whole circles for arcs, not from samples, so it holds the path.
fn rib_reach(b: &Box3, r: &crate::rib::Rib) -> Box3 {
    let pl = &r.plane;
    let mut path = Box3::EMPTY;
    for s in &r.path {
        let pts: Vec<Point2> = match s {
            Segment::Line { a, b } => vec![*a, *b],
            Segment::Arc { center, radius, .. } | Segment::Circle { center, radius, .. } => {
                let (c, q) = (*center, radius.abs());
                vec![
                    Point2::new(c.x - q, c.y - q),
                    Point2::new(c.x + q, c.y - q),
                    Point2::new(c.x - q, c.y + q),
                    Point2::new(c.x + q, c.y + q),
                ]
            }
            // a rib refuses these (`BadPath`): it adds nothing
            Segment::Spline(_) => vec![],
        };
        for p in pts {
            path = path.hull(&Box3::from_point(pl.plane_point(p)));
        }
    }
    let bound = b.hull(&path);
    if bound.is_empty() {
        return bound;
    }
    let t = r.thickness.abs();
    let taper = if (0.0..dmath::FRAC_PI_2).contains(&r.draft) {
        bound.diagonal() * dmath::sin(r.draft) / dmath::cos(r.draft)
    } else {
        0.0
    };
    bound.grow(t + taper + 1.0)
}

/// A box holding all the material `features` can make, to size through holes by.
pub(crate) fn reach(features: crate::History<'_>, given: Option<&crate::Given>) -> Box3 {
    if let Some(b) = given.and_then(|g| g.reach) {
        return b;
    }
    // past a feature the history does not bound, the part as the regen built it does
    let (from, mut b) = match given.and_then(|g| g.base) {
        Some((at, bx)) if at < features.len() && features[..=at].iter().any(unbounded) => (at + 1, bx),
        _ => (0, Box3::EMPTY),
    };
    for (i, f) in features.iter().enumerate().skip(from) {
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
            Feature::Edit(crate::Edit::Rib(r)) => b = rib_reach(&b, r),
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
            _ => add_into(features, i, &mut b),
        }
    }
    b
}
