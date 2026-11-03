//! The folded and the flat solid of a sheet, built directly.
//!
//! Each wall and each bend zone is a strip with its own 2D coordinates. A wall's region is its
//! outline minus its cuts minus a band from each of its tangent lines to just past the mold line;
//! a bend zone's is a rectangle, BA by the joint's length, minus the cuts developed into it. The
//! regions meet on the tangent lines, whose stretches become the edges between a wall's skin and a
//! bend's; vertices there are made once, from the wall. The same topology is then laid out by the
//! fold map or the flat map. Folded, walls are placed from mold lines and angles only, so K, a bend
//! table or BA moves nothing folded but cuts that cross a bend.
//!
//! Side faces merge where they continue each other in one plane across a tangent line; skins stay
//! split there (a plane and a cylinder). Names come from the feature, wall, joint and cut ids and
//! the outline's segment ids (docs/naming.md), never from storage order.
use crate::error::{
    End, FeatureId, JointKindName, SheetError, SheetWarning, Side, Strip, TrimCurveKind, TrimRef, WallId,
};
use crate::graph::{CornerKind, Gauge, JointKind, Sheet, straight};
use crate::pattern::{BendLine, Curve2, Pattern};
use crate::region::{Bound, Piece, Region, RegionError, subtract};
use crate::relief::{CornerShape, DepthRef, Placement, Relief, ReliefKind, ReliefMade};
use crate::roles::{Roles, Skin};
use crate::shop::setback;
use forge_geom::ruled::{Rail, Ruled};
use forge_geom::spline::{Extruded, PlaneSpline, Spline, SplineCurve};
use forge_geom::wrap::{Flat, Wrap};
use forge_geom::{Circle, Curve, CurveGeom, Cylinder, Line, Plane, Surface, SurfaceGeom};
use forge_math::{Frame, Interval, Point2, Point3, Vec2, Vec3, dmath, tol};
use forge_ops::{ProfileLoop, Segment};
use forge_topo::{Body, EdgeId, Name, VertexId};
use std::collections::HashMap;

const TOL: f64 = tol::LINEAR;

/// Which solid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Map {
    Folded,
    Flat,
}

/// The setback on the reference skin: its radius is the inside one when the bend turns towards the
/// reference side, the outside one otherwise.
pub fn setback_of(g: &Gauge, radius: f64, angle: f64) -> f64 {
    setback(reference_radius(g, radius, angle), angle.abs())
}

fn reference_radius(g: &Gauge, radius: f64, angle: f64) -> f64 {
    if angle > 0.0 { radius } else { radius + g.thickness }
}

/// A rigid motion of the plane: p -> o + x p.x + perp(x) p.y.
#[derive(Clone, Copy, Debug)]
struct Rigid {
    o: Point2,
    x: Vec2,
}

impl Rigid {
    const ID: Rigid = Rigid {
        o: Point2::ORIGIN,
        x: Vec2 { x: 1.0, y: 0.0 },
    };
    fn y(&self) -> Vec2 {
        Vec2::new(-self.x.y, self.x.x)
    }
    fn apply(&self, p: Point2) -> Point2 {
        self.o + self.x * p.x + self.y() * p.y
    }
    fn vec(&self, v: Vec2) -> Vec2 {
        self.x * v.x + self.y() * v.y
    }
    fn local(&self, p: Point2) -> Point2 {
        let d = p - self.o;
        Point2::new(d.dot(self.x), d.dot(self.y()))
    }
    fn then(&self, inner: &Rigid) -> Rigid {
        Rigid {
            o: self.apply(inner.o),
            x: self.vec(inner.x),
        }
    }
    fn inverse(&self) -> Rigid {
        let o = self.local(Point2::ORIGIN);
        Rigid {
            o,
            x: Vec2::new(self.x.x, -self.x.y),
        }
    }
    fn angle(&self) -> f64 {
        dmath::atan2(self.x.y, self.x.x)
    }
    fn frame(&self) -> Frame {
        let y = self.y();
        Frame::from_axes(
            Point3::new(self.o.x, self.o.y, 0.0),
            Vec3::new(self.x.x, self.x.y, 0.0),
            Vec3::new(y.x, y.y, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        )
        .expect("a unit direction")
    }
}

fn shift(x: f64) -> Rigid {
    Rigid {
        o: Point2::new(x, 0.0),
        x: Vec2::new(1.0, 0.0),
    }
}

/// A bend's numbers and placements.
struct Bend {
    joint: usize,
    a: usize,
    b: usize,
    /// The joint's frame in a's and b's coordinates: origin at a's mold line start, x across into
    /// b, y along.
    ja: Rigid,
    jb: Rigid,
    len: f64,
    sigma: f64,
    theta: f64,
    rho0: f64,
    s: f64,
    ba: f64,
    /// The joint frame in the world, folded [0] and flat [1].
    frame: [Frame; 2],
}

/// How a strip's 2D coordinates and depth below the reference skin go to the world.
#[derive(Clone, Copy, Debug)]
enum Geo {
    /// Local (x, y, -w).
    Plane(Frame),
    /// Round the axis frame's z: angle u / rn, radius rho0 + sigma w, height -sigma y.
    Round {
        axis: Frame,
        rho0: f64,
        sigma: f64,
        rn: f64,
        theta: f64,
        ba: f64,
    },
}

impl Geo {
    fn point(&self, p: Point2, w: f64) -> Point3 {
        match *self {
            Geo::Plane(f) => f.point_to_world(Point3::new(p.x, p.y, -w)),
            Geo::Round {
                axis,
                rho0,
                sigma,
                rn,
                theta,
                ba,
            } => {
                let (s, c) = dmath::sin_cos(phi(p.x, rn, theta, ba));
                let r = rho0 + sigma * w;
                axis.point_to_world(Point3::new(r * c, r * s, -sigma * p.y))
            }
        }
    }
}

/// The angle at u across a bend; its far tangent line is the bend's angle exactly.
fn phi(u: f64, rn: f64, theta: f64, ba: f64) -> f64 {
    if u == ba { theta } else { u / rn }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Side,
    /// A stretch of tangent line: bend index and side.
    Tangent(usize, Side),
}

struct LoopData {
    strip: usize,
    comp: usize,
    bounds: Vec<Bound>,
    kinds: Vec<Kind>,
}

struct StripData {
    strip: Strip,
    /// Under the map built.
    geo: Geo,
    /// Folded, whatever the map: the clash check reads it.
    folded: Geo,
    region: Region,
}

/// Everything the build reads off the graph, in one place.
struct Ctx<'a> {
    sheet: &'a Sheet,
    feature: FeatureId,
    root: Name,
    sources: Vec<TrimRef>,
}

impl Ctx<'_> {
    fn source(&mut self, t: TrimRef) -> u64 {
        if let Some(i) = self.sources.iter().position(|s| *s == t) {
            return i as u64;
        }
        self.sources.push(t);
        (self.sources.len() - 1) as u64
    }
    fn name_of(&self, t: TrimRef) -> Name {
        let r = self.root;
        match t {
            TrimRef::Outline { wall, segment } => r.with_str("outline").with(wall.0 as u64).with(segment),
            TrimRef::Cut { cut, segment } => r.with_str("cut").with(cut.0 as u64).with(segment),
            TrimRef::TangentLine { joint, side } => {
                r.with_str("tangent").with(joint.0 as u64).with(side as u64)
            }
            TrimRef::BendEnd { joint, end } => r.with_str("bend end").with(joint.0 as u64).with(end as u64),
            TrimRef::MoldLine { joint, side } => r.with_str("mold").with(joint.0 as u64).with(side as u64),
            TrimRef::Corner { a, b, segment } => r
                .with_str("corner")
                .with(a.0 as u64)
                .with(b.0 as u64)
                .with(segment),
            TrimRef::Relief { joint, end, segment } => r
                .with_str("relief")
                .with(joint.0 as u64)
                .with(end as u64)
                .with(segment),
        }
    }
    fn strip_name(&self, s: Strip) -> Name {
        match s {
            Strip::Wall(w) => self.root.with_str("wall").with(w.0 as u64),
            Strip::Bend(j) => self.root.with_str("bend").with(j.0 as u64),
        }
    }
}

/// A loop with its segments' ids replaced by source ids, carried by a rigid motion.
fn carried(l: &ProfileLoop, by: Option<&Rigid>, id: impl Fn(u64) -> u64) -> Option<ProfileLoop> {
    let mut out = Vec::with_capacity(l.segments.len());
    for (s, sid) in &l.segments {
        let s = match by {
            None => s.clone(),
            Some(m) => match s {
                Segment::Line { a, b } => Segment::Line {
                    a: m.apply(*a),
                    b: m.apply(*b),
                },
                Segment::Arc {
                    center,
                    radius,
                    start,
                    sweep,
                } => Segment::Arc {
                    center: m.apply(*center),
                    radius: *radius,
                    start: start + m.angle(),
                    sweep: *sweep,
                },
                Segment::Circle { center, radius, ccw } => Segment::Circle {
                    center: m.apply(*center),
                    radius: *radius,
                    ccw: *ccw,
                },
                Segment::Spline(ps) => {
                    let c = ps.c.mapped(|q| {
                        let r = m.apply(Point2::new(q.x, q.y));
                        Point3::new(r.x, r.y, 0.0)
                    });
                    let (degree, knots, pts, weights) = c.control_form();
                    let pts2 = pts.iter().map(|p| Point2::new(p.x, p.y)).collect();
                    Segment::Spline(std::sync::Arc::new(PlaneSpline::control(
                        degree,
                        pts2,
                        Some(knots),
                        weights,
                    )?))
                }
            },
        };
        out.push((s, id(*sid)));
    }
    Some(ProfileLoop::new(out))
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64, ids: [u64; 4], by: &Rigid) -> ProfileLoop {
    let c = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].map(|(x, y)| by.apply(Point2::new(x, y)));
    ProfileLoop::new(
        (0..4)
            .map(|i| {
                (
                    Segment::Line {
                        a: c[i],
                        b: c[(i + 1) % 4],
                    },
                    ids[i],
                )
            })
            .collect(),
    )
}

fn bbox(l: &ProfileLoop) -> (Point2, Point2) {
    let mut lo = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for (s, _) in &l.segments {
        for p in s.samples(16) {
            lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
        }
    }
    (lo, hi)
}

/// Where a strip's own 2D coordinates lie in the developed plane (the first wall's coordinates):
/// p -> origin + x p.x + perp(x) p.y.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place2 {
    pub origin: Point2,
    pub x: Vec2,
}

impl Place2 {
    pub fn apply(&self, p: Point2) -> Point2 {
        Rigid {
            o: self.origin,
            x: self.x,
        }
        .apply(p)
    }
}

impl From<Rigid> for Place2 {
    fn from(r: Rigid) -> Place2 {
        Place2 { origin: r.o, x: r.x }
    }
}

/// A wall's place: its coordinates (reference skin z = 0, material on -z) in the world under the
/// map, and in the developed plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WallPlace {
    pub wall: WallId,
    pub frame: Frame,
    pub developed: Place2,
}

/// A bend's numbers, and its zone's coordinates (u across from wall a's tangent line, 0 to
/// `allowance`; y along the mold line, 0 to `length`) in the developed plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BendPlace {
    pub joint: crate::error::JointId,
    pub a: WallId,
    pub b: WallId,
    /// Signed as `JointKind::Bend`.
    pub angle: f64,
    pub radius: f64,
    pub allowance: f64,
    /// Mold line to tangent line on the reference skin.
    pub setback: f64,
    pub length: f64,
    pub developed: Place2,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layout {
    pub walls: Vec<WallPlace>,
    pub bends: Vec<BendPlace>,
}

/// A built sheet: the body, what each face and edge is, where the walls and bends lie, and (flat
/// only) the pattern for cutting.
#[derive(Clone, Debug)]
pub struct Built {
    pub body: Body,
    pub roles: Roles,
    pub layout: Layout,
    pub pattern: Option<Pattern>,
    /// The reliefs cut, asked for or added.
    pub reliefs: Vec<ReliefMade>,
    /// The corner reliefs cut.
    pub corners: Vec<CornerMade>,
    /// The other map's refusal, as a warning on this one: a flat overlap on a folded build, a
    /// folded clash on a flat one.
    pub warnings: Vec<SheetWarning>,
}

/// A corner relief the builder cut where two bends off one wall meet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CornerMade {
    pub joints: [crate::error::JointId; 2],
    pub wall: WallId,
    pub kind: CornerKind,
    /// Its radius, or half its side.
    pub size: f64,
    /// Sized to clear the bends (not asked for).
    pub automatic: bool,
}

/// Where a cut comes from, for its trims' names.
#[derive(Clone, Copy, Debug)]
enum CutSrc {
    Cut(crate::error::CutId),
    Corner(crate::error::JointId, crate::error::JointId),
}

impl CutSrc {
    fn trim(self, segment: u64) -> TrimRef {
        match self {
            CutSrc::Cut(cut) => TrimRef::Cut { cut, segment },
            CutSrc::Corner(a, b) => TrimRef::Corner { a, b, segment },
        }
    }
}

struct CutRef<'a> {
    wall: WallId,
    lp: &'a ProfileLoop,
    src: CutSrc,
}

/// Corner reliefs: wherever two bends leave one wall from outline segments meeting at a vertex.
fn corners(
    sheet: &Sheet,
    bends: &[Bend],
    t: f64,
) -> Result<Vec<(CutSrc, WallId, ProfileLoop, CornerMade)>, SheetError> {
    let mut out = Vec::new();
    for (i, b1) in bends.iter().enumerate() {
        for b2 in &bends[i + 1..] {
            if b1.a != b2.a {
                continue;
            }
            let w = &sheet.walls[b1.a];
            let (j1, j2) = (&sheet.joints[b1.joint], &sheet.joints[b2.joint]);
            let (Ok((p0, p1)), Ok((q0, q1))) = (straight(w, j1.on_a), straight(w, j2.on_a)) else {
                continue;
            };
            let v = if p1.distance(q0) <= TOL {
                p1
            } else if q1.distance(p0) <= TOL {
                q1
            } else {
                continue;
            };
            let joints = [j1.id, j2.id];
            let rmax = [b1, b2]
                .iter()
                .map(|b| joint_radius(sheet, b))
                .fold(0.0, f64::max);
            let rule = sheet.rule.corner;
            let (kind, from_rule) = match sheet.corner(j1.id, j2.id) {
                Some(k) => (k, false),
                None => {
                    let size = rule.size.map(|x| x.at(t, rmax));
                    let placement = rule.placement;
                    let k = match rule.shape {
                        CornerShape::Round => CornerKind::Circular { size, placement },
                        CornerShape::Square => CornerKind::Square { size, placement },
                        CornerShape::Tear => CornerKind::Tear,
                        CornerShape::TrimToBend => CornerKind::TrimToBend,
                        CornerShape::Closed => CornerKind::Closed,
                    };
                    (k, true)
                }
            };
            // the edges' outward normals, and each bend's reach either side of its mold line
            let out_of = |a: Point2, b: Point2| {
                let d = (b - a) * (1.0 / a.distance(b));
                Vec2::new(d.y, -d.x)
            };
            let (n1, n2) = (out_of(p0, p1), out_of(q0, q1));
            let at = |x: f64, y: f64| v + n1 * x + n2 * y;
            // the square of both zones' ends near the corner, a thickness to spare
            let lo = |b: &Bend| -b.s - t;
            let hi = |b: &Bend| (b.ba - b.s).max(0.0) + t;
            let q = [
                at(lo(b1), lo(b2)),
                at(hi(b1), lo(b2)),
                at(hi(b1), hi(b2)),
                at(lo(b1), hi(b2)),
            ];
            let centre = |p: Placement| match p {
                Placement::Intersection => at(0.5 * b1.ba - b1.s, 0.5 * b2.ba - b2.s),
                Placement::Tangent => at(-b1.s, -b2.s),
            };
            let reach = |c: Point2, square: bool| {
                q.iter()
                    .map(|p| {
                        let d = *p - c;
                        if square {
                            d.dot(n1).abs().max(d.dot(n2).abs())
                        } else {
                            d.length()
                        }
                    })
                    .fold(0.0, f64::max)
            };
            let polygon = |pts: &[Point2]| {
                let n = pts.len();
                // counter-clockwise
                let area: f64 = (0..n)
                    .map(|k| pts[k].to_vec().cross(pts[(k + 1) % n].to_vec()))
                    .sum();
                let pts: Vec<Point2> = if area < 0.0 {
                    pts.iter().rev().copied().collect()
                } else {
                    pts.to_vec()
                };
                ProfileLoop::new(
                    (0..n)
                        .map(|k| {
                            (
                                Segment::Line {
                                    a: pts[k],
                                    b: pts[(k + 1) % n],
                                },
                                k as u64,
                            )
                        })
                        .collect(),
                )
            };
            let unbuilt = |size: f64| SheetError::CornerUnbuilt {
                feature: sheet.feature,
                joints,
                size,
            };
            let square_at = |c: Point2, h: f64| {
                polygon(&[
                    c + (n1 + n2) * -h,
                    c + (n1 - n2) * h,
                    c + (n1 + n2) * h,
                    c + (n2 - n1) * h,
                ])
            };
            let (lp, size, automatic) = match kind {
                CornerKind::Circular { size, placement } | CornerKind::Square { size, placement } => {
                    let square = matches!(kind, CornerKind::Square { .. });
                    let c = centre(placement);
                    let h = size.unwrap_or_else(|| reach(c, square));
                    if h.is_nan() || h <= RELIEF_MIN {
                        return Err(unbuilt(h));
                    }
                    let lp = if square {
                        square_at(c, h)
                    } else {
                        ProfileLoop::new(vec![(
                            Segment::Circle {
                                center: c,
                                radius: h,
                                ccw: true,
                            },
                            0,
                        )])
                    };
                    (lp, h, size.is_none())
                }
                CornerKind::Open { gap } => {
                    let h = reach(v, true) + gap;
                    if h.is_nan() || h <= RELIEF_MIN {
                        return Err(unbuilt(h));
                    }
                    (square_at(v, h), h, false)
                }
                CornerKind::TrimToBend => {
                    let (a, b) = (-b1.s, -b2.s);
                    let (c, d) = (hi(b1), hi(b2));
                    (polygon(&[at(a, b), at(c, b), at(c, d), at(a, d)]), 0.0, true)
                }
                CornerKind::Tear => {
                    // along the diagonal, from past the edge to past both tangent lines' crossing
                    let m = (n1 + n2) * (1.0 / (n1 + n2).length());
                    let across = Vec2::new(-m.y, m.x) * (0.5 * sheet.rule.kerf);
                    let inner = at(-b1.s, -b2.s) - v;
                    let (from, to) = (v - m * (inner.length() + t), v + m * (t + b1.ba.max(b2.ba)));
                    (
                        polygon(&[from - across, to - across, to + across, from + across]),
                        sheet.rule.kerf,
                        true,
                    )
                }
                CornerKind::Closed | CornerKind::Overlap { .. } => return Err(unbuilt(f64::NAN)),
            };
            // sized by the builder: from the rule, or a relief left unsized
            let automatic = automatic
                && (from_rule
                    || matches!(
                        kind,
                        CornerKind::Circular { size: None, .. } | CornerKind::Square { size: None, .. }
                    ));
            out.push((
                CutSrc::Corner(j1.id, j2.id),
                w.id,
                lp,
                CornerMade {
                    joints,
                    wall: w.id,
                    kind,
                    size,
                    automatic,
                },
            ));
        }
    }
    Ok(out)
}

fn map_index(map: Map) -> usize {
    match map {
        Map::Folded => 0,
        Map::Flat => 1,
    }
}

/// A relief narrower than this is no cut.
const RELIEF_MIN: f64 = 1e-3;

fn joint_radius(sheet: &Sheet, b: &Bend) -> f64 {
    match sheet.joints[b.joint].kind {
        JointKind::Bend { radius, .. } => radius,
        _ => sheet.gauge.radius,
    }
}

/// The wall's material beyond a relief, along the mold line: how far rays from the relief's far
/// side, across the relief's depth, run through the wall before leaving it (outline lines
/// exactly, curves by their sampled chords).
#[allow(clippy::too_many_arguments)]
fn remnant(w: &crate::graph::Wall, b: &Bend, side: Side, end: End, width: f64, depth: f64, j: &Rigid) -> f64 {
    let into = if side == Side::A { -1.0 } else { 1.0 };
    let (y, dy) = match end {
        End::Start => (-width, -1.0),
        End::End => (b.len + width, 1.0),
    };
    let dir = j.vec(Vec2::new(0.0, dy));
    let chords: Vec<(Point2, Point2)> = w
        .outline
        .iter()
        .flat_map(|l| &l.segments)
        .flat_map(|(g, _)| {
            let pts = match g {
                Segment::Line { a, b } => vec![*a, *b],
                _ => g.samples(64),
            };
            pts.windows(2).map(|p| (p[0], p[1])).collect::<Vec<_>>()
        })
        .collect();
    let mut best = f64::INFINITY;
    for f in [0.1, 0.3, 0.5, 0.7, 0.9] {
        let o = j.apply(Point2::new(into * f * depth, y));
        for &(a, c) in &chords {
            let e = c - a;
            let den = dir.cross(e);
            if den.abs() <= 1e-15 {
                continue;
            }
            let u = (a - o).cross(e) / den;
            let v = (a - o).cross(dir) / den;
            if u > TOL && (0.0..=1.0).contains(&v) {
                best = best.min(u);
            }
        }
    }
    best
}

/// A relief at one end of a bend, in the wall's coordinates: from past the edge (beyond the mold
/// line) into the wall to `depth`, beside the zone's end, counter-clockwise.
#[allow(clippy::too_many_arguments)]
fn relief_loop(
    kind: ReliefKind,
    width: f64,
    depth: f64,
    b: &Bend,
    side: Side,
    end: End,
    t: f64,
    j: &Rigid,
    ids: &[u64],
) -> ProfileLoop {
    // into the wall: -x for a's side, +x for b's (the joint frame's x runs from a into b)
    let into = if side == Side::A { -1.0 } else { 1.0 };
    let out = -into * (t + width + b.s);
    let (y0, y1) = match end {
        End::Start => (-width, 0.0),
        End::End => (b.len, b.len + width),
    };
    let p = Point2::new;
    let local = match kind {
        ReliefKind::Rect | ReliefKind::Tear => {
            let (x0, x1) = (out.min(into * depth), out.max(into * depth));
            rect(x0, y0, x1, y1, [ids[0], ids[1], ids[2], ids[3]], &Rigid::ID)
        }
        ReliefKind::Obround => {
            let r = 0.5 * width;
            let xc = into * (depth - r);
            let ym = 0.5 * (y0 + y1);
            let segs = if side == Side::A {
                vec![
                    (
                        Segment::Line {
                            a: p(xc, y0),
                            b: p(out, y0),
                        },
                        ids[0],
                    ),
                    (
                        Segment::Line {
                            a: p(out, y0),
                            b: p(out, y1),
                        },
                        ids[1],
                    ),
                    (
                        Segment::Line {
                            a: p(out, y1),
                            b: p(xc, y1),
                        },
                        ids[2],
                    ),
                    (
                        Segment::Arc {
                            center: p(xc, ym),
                            radius: r,
                            start: 0.5 * dmath::PI,
                            sweep: dmath::PI,
                        },
                        ids[3],
                    ),
                ]
            } else {
                vec![
                    (
                        Segment::Line {
                            a: p(out, y0),
                            b: p(xc, y0),
                        },
                        ids[0],
                    ),
                    (
                        Segment::Arc {
                            center: p(xc, ym),
                            radius: r,
                            start: -0.5 * dmath::PI,
                            sweep: dmath::PI,
                        },
                        ids[3],
                    ),
                    (
                        Segment::Line {
                            a: p(xc, y1),
                            b: p(out, y1),
                        },
                        ids[2],
                    ),
                    (
                        Segment::Line {
                            a: p(out, y1),
                            b: p(out, y0),
                        },
                        ids[1],
                    ),
                ]
            };
            ProfileLoop::new(segs)
        }
    };
    carried(&local, Some(j), |s| s).expect("lines and arcs carry")
}

/// Builds the sheet's folded or flat solid.
pub fn build(sheet: &Sheet, map: Map) -> Result<Body, SheetError> {
    build_with(sheet, map).map(|b| b.body)
}

/// `build`, with the roles, the layout and (flat) the pattern. Where a wall needs a relief at a
/// joint end left `Auto`, the sheet's default is cut there and the build tried again.
pub fn build_with(sheet: &Sheet, map: Map) -> Result<Built, SheetError> {
    let mut auto: Vec<(crate::error::JointId, End, WallId)> = Vec::new();
    loop {
        match build_once(sheet, map, &auto) {
            Err(SheetError::NeedsRelief { joint, wall, end, .. })
                if sheet.relief(joint, end) == Relief::Auto && !auto.contains(&(joint, end, wall)) =>
            {
                auto.push((joint, end, wall));
            }
            r => return r,
        }
    }
}

fn build_once(
    sheet: &Sheet,
    map: Map,
    auto: &[(crate::error::JointId, End, WallId)],
) -> Result<Built, SheetError> {
    let feature = sheet.feature;
    let mut made: Vec<ReliefMade> = Vec::new();
    let mut corners_made: Vec<CornerMade> = Vec::new();
    let t = sheet.gauge.thickness;
    let mut cx = Ctx {
        sheet,
        feature,
        root: Name::feature(feature.0),
        sources: Vec::new(),
    };
    let nw = sheet.walls.len();
    let wall_index = |id: WallId| sheet.walls.iter().position(|w| w.id == id);
    // place the walls along the bends from the first, in 3D under the map and developed in 2D
    // each wall's frame folded [0] and flat [1]: both are always laid out, for the checks
    let mut frames: Vec<Option<[Frame; 2]>> = vec![None; nw];
    let mut dev: Vec<Rigid> = vec![Rigid::ID; nw];
    frames[0] = Some([sheet.frame; 2]);
    let mi = map_index(map);
    let mut bends: Vec<Bend> = Vec::new();
    let mut queue = vec![0usize];
    while let Some(wi) = queue.pop() {
        for (ji, j) in sheet.joints.iter().enumerate() {
            if j.a != sheet.walls[wi].id {
                continue;
            }
            let (angle, radius) = match j.kind {
                JointKind::Bend { angle, radius } => (angle, radius),
                JointKind::Rip { .. } => {
                    return Err(SheetError::JointKindUnbuilt {
                        feature,
                        joint: j.id,
                        kind: JointKindName::Rip,
                    });
                }
                JointKind::Tangent => {
                    return Err(SheetError::JointKindUnbuilt {
                        feature,
                        joint: j.id,
                        kind: JointKindName::Tangent,
                    });
                }
            };
            let bi = wall_index(j.b).ok_or(SheetError::JointWalls { feature, joint: j.id })?;
            if frames[bi].is_some() {
                return Err(SheetError::JointWalls { feature, joint: j.id });
            }
            let segment_error = |wall: WallId, segment: u64, e: Option<TrimCurveKind>| match e {
                None => SheetError::NoSuchSegment {
                    feature,
                    wall,
                    segment,
                },
                Some(curve) => SheetError::NotStraight {
                    feature,
                    trim: TrimRef::Outline { wall, segment },
                    curve,
                },
            };
            let (p0, p1) = straight(&sheet.walls[wi], j.on_a).map_err(|e| segment_error(j.a, j.on_a, e))?;
            let (q0, q1) = straight(&sheet.walls[bi], j.on_b).map_err(|e| segment_error(j.b, j.on_b, e))?;
            let (la, lb) = (p0.distance(p1), q0.distance(q1));
            if (la - lb).abs() > TOL || la <= TOL {
                return Err(SheetError::JointLengths {
                    feature,
                    joint: j.id,
                    a: la,
                    b: lb,
                    limit: TOL,
                });
            }
            let theta = angle.abs();
            if !(theta > tol::ANGULAR && theta < dmath::PI - tol::ANGULAR) {
                return Err(SheetError::BendAngle {
                    feature,
                    joint: j.id,
                    angle,
                    limit: dmath::PI - tol::ANGULAR,
                });
            }
            let ba = sheet
                .gauge
                .rule
                .allowance(t, radius, theta)
                .map_err(|why| SheetError::BendRule {
                    feature,
                    joint: j.id,
                    why,
                })?;
            let right = |d: Vec2| Vec2::new(d.y, -d.x);
            let ja = Rigid {
                o: p0,
                x: right((p1 - p0) * (1.0 / la)),
            };
            let jb = Rigid {
                o: q1,
                x: right((q0 - q1) * (1.0 / lb)),
            };
            let sigma = angle.signum();
            let rho0 = reference_radius(&sheet.gauge, radius, angle);
            let s = setback(rho0, theta);
            let fa = frames[wi].expect("placed before its joints");
            let jframe = fa.map(|f| f.compose(&ja.frame()));
            let (sn, cs) = dmath::sin_cos(theta);
            let turned = [
                Frame::from_axes(
                    Point3::ORIGIN,
                    Vec3::new(cs, 0.0, sigma * sn),
                    Vec3::new(0.0, 1.0, 0.0),
                    Vec3::new(-sigma * sn, 0.0, cs),
                )
                .expect("square axes"),
                Frame::WORLD.with_origin(Point3::new(ba - 2.0 * s, 0.0, 0.0)),
            ];
            frames[bi] = Some([0, 1].map(|m| jframe[m].compose(&turned[m]).compose(&jb.frame().inverse())));
            dev[bi] = dev[wi].then(&ja).then(&shift(ba - 2.0 * s)).then(&jb.inverse());
            bends.push(Bend {
                joint: ji,
                a: wi,
                b: bi,
                ja,
                jb,
                len: la,
                sigma,
                theta,
                rho0,
                s,
                ba,
                frame: jframe,
            });
            queue.push(bi);
        }
    }
    if let Some(w) = frames.iter().position(Option::is_none) {
        return Err(SheetError::WallUnplaced {
            feature,
            wall: sheet.walls[w].id,
        });
    }
    let jid = |b: &Bend| sheet.joints[b.joint].id;
    // the strips: walls, then bend zones
    let mut strips: Vec<StripData> = Vec::with_capacity(nw + bends.len());
    let mut strip_dev: Vec<Rigid> = dev.clone();
    strip_dev.extend(bends.iter().map(|b| dev[b.a].then(&b.ja).then(&shift(-b.s))));
    // which strips each cut reaches: its wall, and on across bends while it overlaps the zone
    let mut cut_strips: Vec<Vec<ProfileLoop>> = vec![Vec::new(); nw + bends.len()];
    // the cuts asked for, and corner reliefs where two bends off one wall meet
    let mut all: Vec<(CutSrc, WallId, ProfileLoop)> = sheet
        .cuts
        .iter()
        .map(|c| (CutSrc::Cut(c.id), c.wall, c.lp.clone()))
        .collect();
    for (src, wall, lp, done) in corners(sheet, &bends, t)? {
        corners_made.push(done);
        all.push((src, wall, lp));
    }
    for (src, cwall, clp) in &all {
        let c = CutRef {
            wall: *cwall,
            lp: clp,
            src: *src,
        };
        let anchor = wall_index(c.wall).ok_or(SheetError::NoSuchWall {
            feature,
            wall: c.wall,
        })?;
        let ids: Vec<u64> =
            c.lp.segments
                .iter()
                .map(|(_, s)| cx.source(c.src.trim(*s)))
                .collect();
        let seg_ids: Vec<u64> = c.lp.segments.iter().map(|s| s.1).collect();
        let id_of = |s: u64| ids[seg_ids.iter().position(|q| *q == s).expect("its own segment")];
        let mut seen = vec![false; nw];
        let mut todo = vec![anchor];
        seen[anchor] = true;
        while let Some(w) = todo.pop() {
            let by = (w != anchor).then(|| strip_dev[w].inverse().then(&strip_dev[anchor]));
            let lp = carried(c.lp, by.as_ref(), id_of).ok_or(SheetError::TrimCurveUnsupported {
                feature,
                strip: Strip::Wall(sheet.walls[w].id),
                trim: c.src.trim(c.lp.segments[0].1),
                curve: TrimCurveKind::Spline,
            })?;
            cut_strips[w].push(lp);
            for (bi, b) in bends.iter().enumerate() {
                let other = if b.a == w {
                    b.b
                } else if b.b == w {
                    b.a
                } else {
                    continue;
                };
                let si = nw + bi;
                let by = strip_dev[si].inverse().then(&strip_dev[anchor]);
                let lp = carried(c.lp, Some(&by), id_of).ok_or(SheetError::TrimCurveUnsupported {
                    feature,
                    strip: Strip::Bend(jid(b)),
                    trim: c.src.trim(c.lp.segments[0].1),
                    curve: TrimCurveKind::Spline,
                })?;
                let (lo, hi) = bbox(&lp);
                if hi.x < -TOL || lo.x > b.ba + TOL || hi.y < -TOL || lo.y > b.len + TOL {
                    continue;
                }
                if !cut_strips[si].contains(&lp) {
                    cut_strips[si].push(lp);
                }
                if !seen[other] {
                    seen[other] = true;
                    todo.push(other);
                }
            }
        }
    }
    for (wi, w) in sheet.walls.iter().enumerate() {
        let mut loops = Vec::with_capacity(w.outline.len());
        for l in &w.outline {
            let ids: Vec<u64> = l
                .segments
                .iter()
                .map(|(_, s)| {
                    cx.source(TrimRef::Outline {
                        wall: w.id,
                        segment: *s,
                    })
                })
                .collect();
            let seg_ids: Vec<u64> = l.segments.iter().map(|s| s.1).collect();
            loops.push(
                carried(l, None, |s| {
                    ids[seg_ids.iter().position(|q| *q == s).expect("its own")]
                })
                .expect("no motion"),
            );
        }
        let mut cuts = std::mem::take(&mut cut_strips[wi]);
        for b in &bends {
            let (side, j) = if b.a == wi {
                (Side::A, &b.ja)
            } else if b.b == wi {
                (Side::B, &b.jb)
            } else {
                continue;
            };
            let joint = jid(b);
            let (start, end) = (
                cx.source(TrimRef::BendEnd {
                    joint,
                    end: End::Start,
                }),
                cx.source(TrimRef::BendEnd { joint, end: End::End }),
            );
            let (mold, tangent) = (
                cx.source(TrimRef::MoldLine { joint, side }),
                cx.source(TrimRef::TangentLine { joint, side }),
            );
            // the joint frame's x runs from a into b: a's band reaches back to -s, b's on to +s
            let past = t.max(b.s);
            cuts.push(match side {
                Side::A => rect(-b.s, 0.0, past, b.len, [start, mold, end, tangent], j),
                Side::B => rect(-past, 0.0, b.s, b.len, [start, tangent, end, mold], j),
            });
            // reliefs: asked for (on the joint's first wall), or added where this wall needs one
            for e in [End::Start, End::End] {
                let asked = sheet.relief(joint, e);
                let rule = sheet.rule.bend;
                let r = joint_radius(sheet, b);
                let (kind, width, given, automatic) = match asked {
                    Relief::Cut { kind, width, depth } if side == Side::A => (kind, width, depth, false),
                    Relief::Auto if auto.contains(&(joint, e, w.id)) => {
                        (rule.kind, rule.width.at(t, r), rule.depth.at(t, r), true)
                    }
                    _ => continue,
                };
                if rule.extend {
                    return Err(SheetError::ReliefExtendUnbuilt {
                        feature,
                        joint,
                        end: e,
                    });
                }
                let mut width = if kind == ReliefKind::Tear {
                    sheet.rule.kerf
                } else {
                    width
                };
                if width.is_nan() || width < RELIEF_MIN {
                    return Err(SheetError::ReliefWidth {
                        feature,
                        joint,
                        end: e,
                        width,
                        limit: RELIEF_MIN,
                    });
                }
                // to the tangent line at least: a round bottom's straight sides too
                let round = if kind == ReliefKind::Obround {
                    0.5 * width
                } else {
                    0.0
                };
                let need = b.s + round;
                // from the mold line, by the rule's reference
                let depth = match rule.reference {
                    // the bend line in folded terms: the mold line (so K moves nothing folded)
                    DepthRef::BendLine => given,
                    DepthRef::UpToBend => need,
                    DepthRef::TangentToBend => b.s,
                };
                let depth = if automatic { depth.max(need) } else { depth };
                let touching = rule.reference == DepthRef::TangentToBend;
                if depth.is_nan() || (!touching && depth < need - 1e-12) {
                    return Err(SheetError::ReliefTooShallow {
                        feature,
                        joint,
                        end: e,
                        depth,
                        limit: need + RELIEF_MIN,
                    });
                }
                let ids: Vec<u64> = (0..5)
                    .map(|k| {
                        cx.source(TrimRef::Relief {
                            joint,
                            end: e,
                            segment: k,
                        })
                    })
                    .collect();
                // material left between the relief and the wall's edge under the rule's minimum
                // goes too: the relief widened past the edge
                let min_remnant = rule.min_remnant.at(t, r);
                let mut remnant_taken = false;
                if min_remnant > 0.0 {
                    let left = remnant(w, b, side, e, width, depth, j);
                    if left > TOL && left < min_remnant {
                        width += left + t;
                        remnant_taken = true;
                    }
                }
                cuts.push(relief_loop(kind, width, depth, b, side, e, t, j, &ids));
                made.push(ReliefMade {
                    joint,
                    end: e,
                    wall: w.id,
                    kind,
                    width,
                    depth,
                    automatic,
                    remnant_taken,
                });
            }
        }
        let region = subtract(&loops, &cuts).map_err(|e| {
            SheetError::from_region(feature, Strip::Wall(w.id), e, |i| cx.sources[i as usize])
        })?;
        let f = frames[wi].expect("placed");
        strips.push(StripData {
            strip: Strip::Wall(w.id),
            geo: Geo::Plane(f[mi]),
            folded: Geo::Plane(f[0]),
            region,
        });
    }
    for (bi, b) in bends.iter().enumerate() {
        let joint = jid(b);
        let ids = [
            cx.source(TrimRef::BendEnd {
                joint,
                end: End::Start,
            }),
            cx.source(TrimRef::TangentLine { joint, side: Side::B }),
            cx.source(TrimRef::BendEnd { joint, end: End::End }),
            cx.source(TrimRef::TangentLine { joint, side: Side::A }),
        ];
        let wall = [rect(0.0, 0.0, b.ba, b.len, ids, &Rigid::ID)];
        let cuts = std::mem::take(&mut cut_strips[nw + bi]);
        let region = subtract(&wall, &cuts).map_err(|e| {
            SheetError::from_region(feature, Strip::Bend(joint), e, |i| cx.sources[i as usize])
        })?;
        let axis = Frame::from_axes(
            Point3::new(-b.s, 0.0, b.sigma * b.rho0),
            Vec3::new(0.0, 0.0, -b.sigma),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, -b.sigma, 0.0),
        )
        .expect("square axes");
        let folded = Geo::Round {
            axis: b.frame[0].compose(&axis),
            rho0: b.rho0,
            sigma: b.sigma,
            rn: b.ba / b.theta,
            theta: b.theta,
            ba: b.ba,
        };
        let geo = match map {
            Map::Folded => folded,
            Map::Flat => Geo::Plane(b.frame[1].compose(&shift(-b.s).frame())),
        };
        strips.push(StripData {
            strip: Strip::Bend(joint),
            geo,
            folded,
            region,
        });
    }
    let layout = Layout {
        walls: (0..nw)
            .map(|wi| WallPlace {
                wall: sheet.walls[wi].id,
                frame: frames[wi].expect("placed")[mi],
                developed: dev[wi].into(),
            })
            .collect(),
        bends: bends
            .iter()
            .enumerate()
            .map(|(bi, b)| BendPlace {
                joint: jid(b),
                a: sheet.walls[b.a].id,
                b: sheet.walls[b.b].id,
                angle: b.sigma * b.theta,
                radius: match sheet.joints[b.joint].kind {
                    JointKind::Bend { radius, .. } => radius,
                    _ => unreachable!("only bends are placed"),
                },
                allowance: b.ba,
                setback: b.s,
                length: b.len,
                developed: strip_dev[nw + bi].into(),
            })
            .collect(),
    };
    let built = Strips::new(cx, strips, &bends, nw, t)?;
    // both checks, whatever the map: each refuses its own body and warns on the other
    let clash = built.clash().err();
    let overlap = built.flat_overlap(&strip_dev);
    let mut warnings = Vec::new();
    match map {
        Map::Folded => {
            if let Some(e) = clash {
                return Err(e);
            }
            warnings.extend(overlap.and_then(|e| e.as_warning()));
        }
        Map::Flat => {
            if let Some(e) = overlap {
                return Err(e);
            }
            warnings.extend(clash.and_then(|e| e.as_warning()));
        }
    }
    let pattern = (map == Map::Flat).then(|| built.pattern(&strip_dev, &layout));
    let (body, roles) = built.body()?;
    Ok(Built {
        body,
        roles,
        layout,
        pattern,
        reliefs: made,
        corners: corners_made,
        warnings,
    })
}

/// A wall's stretch of tangent line: its ends' y in the joint frame, its loop and index.
type Stretch = (f64, f64, usize, usize);

/// The strips' loops, classified and tied together, ready to lay out.
struct Strips<'a> {
    cx: Ctx<'a>,
    strips: Vec<StripData>,
    loops: Vec<LoopData>,
    t: f64,
    /// Vertex slot of each bound's start, per loop.
    slot: Vec<Vec<usize>>,
    /// Each slot's strip and 2D point there.
    slots: Vec<(usize, Point2)>,
    /// A tangent-line bound's twin in the other strip.
    twin: HashMap<(usize, usize), (usize, usize)>,
}

impl<'a> Strips<'a> {
    fn new(
        cx: Ctx<'a>,
        strips: Vec<StripData>,
        bends: &[Bend],
        nw: usize,
        t: f64,
    ) -> Result<Strips<'a>, SheetError> {
        let feature = cx.feature;
        let mut loops = Vec::new();
        for (si, s) in strips.iter().enumerate() {
            for (ci, c) in s.region.parts.iter().enumerate() {
                for l in std::iter::once(&c.outer).chain(&c.holes) {
                    let mut kinds = Vec::with_capacity(l.len());
                    let mut bounds = Vec::with_capacity(l.len());
                    for b in l {
                        let src = cx.sources[b.id as usize];
                        let k = match (src, s.strip) {
                            (TrimRef::TangentLine { joint, side }, _) => {
                                let bi = bends
                                    .iter()
                                    .position(|q| cx.sheet.joints[q.joint].id == joint)
                                    .expect("a bend's own line");
                                Kind::Tangent(bi, side)
                            }
                            (TrimRef::BendEnd { joint, end }, Strip::Wall(wall)) => {
                                return Err(SheetError::NeedsRelief {
                                    feature,
                                    joint,
                                    wall,
                                    end,
                                    at: b.start,
                                });
                            }
                            (TrimRef::MoldLine { joint, side }, _) => {
                                return Err(SheetError::TangentMismatch {
                                    feature,
                                    joint,
                                    side,
                                    at: b.start,
                                    gap: 0.0,
                                    limit: TOL,
                                });
                            }
                            _ => Kind::Side,
                        };
                        let round = matches!(s.geo, Geo::Round { .. });
                        // a spline in a bend is wrapped piece by piece (Bezier, cubic at most)
                        if let Piece::Spline { s: ref c, .. } = b.piece
                            && k == Kind::Side
                            && round
                            && c.degree() > 3
                        {
                            return Err(SheetError::TrimCurveUnsupported {
                                feature,
                                strip: s.strip,
                                trim: src,
                                curve: TrimCurveKind::Spline,
                            });
                        }
                        // cut points along the piece: an arc in a bend turning past half way goes
                        // in pieces, so the ruled side wall along each is regular to its ends; a
                        // spline in a bend goes at its knots
                        let mut cuts: Vec<f64> = match b.piece {
                            Piece::Arc { t0, t1, .. } if round => {
                                let turn = (t1 - t0).abs();
                                let parts = (turn / (0.99 * dmath::PI)).ceil().max(1.0) as usize;
                                // sweeps are checked when cuts are made: at most a whole turn
                                debug_assert!(parts <= 3, "an arc of {turn} rad in a bend");
                                (1..parts).map(|q| q as f64 / parts as f64).collect()
                            }
                            Piece::Spline { s: ref c, t0, t1 } if round => c
                                .knots_in(Interval::new(t0.min(t1), t0.max(t1)))
                                .into_iter()
                                .map(|k| (k - t0) / (t1 - t0))
                                .filter(|u| *u > 1e-12 && *u < 1.0 - 1e-12)
                                .collect(),
                            _ => Vec::new(),
                        };
                        cuts.sort_by(f64::total_cmp);
                        let parts = cuts.len() + 1;
                        for q in 0..parts {
                            let u0 = if q == 0 { 0.0 } else { cuts[q - 1] };
                            let u1 = if q == parts - 1 { 1.0 } else { cuts[q] };
                            bounds.push(Bound {
                                piece: if parts == 1 {
                                    b.piece.clone()
                                } else {
                                    b.piece.sub(u0, u1)
                                },
                                id: b.id,
                                start: if q == 0 { b.start } else { b.piece.at(u0) },
                            });
                            kinds.push(k);
                        }
                    }
                    loops.push(LoopData {
                        strip: si,
                        comp: ci,
                        bounds,
                        kinds,
                    });
                }
            }
        }
        // a side piece lying along a tangent line within the joint's span cannot be told from the
        // shared edge it runs on: refused
        for l in &loops {
            if l.strip >= nw {
                continue;
            }
            for (i, b) in l.bounds.iter().enumerate() {
                if l.kinds[i] != Kind::Side {
                    continue;
                }
                let Piece::Line { a, b: e } = b.piece else {
                    continue;
                };
                for q in bends {
                    let (j, x) = if q.a == l.strip {
                        (&q.ja, -q.s)
                    } else if q.b == l.strip {
                        (&q.jb, q.s)
                    } else {
                        continue;
                    };
                    let (la, le) = (j.local(a), j.local(e));
                    // beside the zone, on the line's extension, it is a free edge of the wall
                    // (a relief to the tangent line): only pieces along the zone itself are refused
                    let (y0, y1) = (la.y.min(le.y), la.y.max(le.y));
                    let beside = y1 <= TOL || y0 >= q.len - TOL;
                    if !beside && (la.x - x).abs() <= TOL && (le.x - x).abs() <= TOL {
                        return Err(SheetError::TrimAlongBendLine {
                            feature,
                            joint: cx.sheet.joints[q.joint].id,
                            trim: cx.sources[b.id as usize],
                            from: a,
                            to: e,
                        });
                    }
                }
            }
        }
        // vertex slots: walls' first, then bends' (sharing the walls' on tangent lines)
        let mut slot: Vec<Vec<usize>> = loops.iter().map(|l| vec![usize::MAX; l.bounds.len()]).collect();
        let mut slots: Vec<(usize, Point2)> = Vec::new();
        // per (bend, side): the walls' tangent stretches, (y0, y1, loop, index), y in the joint frame
        let mut stretches: HashMap<(usize, Side), Vec<Stretch>> = HashMap::new();
        for (li, l) in loops.iter().enumerate() {
            if l.strip >= nw {
                continue;
            }
            for (i, b) in l.bounds.iter().enumerate() {
                slot[li][i] = slots.len();
                slots.push((l.strip, b.start));
                if let Kind::Tangent(bi, side) = l.kinds[i] {
                    let j = if side == Side::A {
                        &bends[bi].ja
                    } else {
                        &bends[bi].jb
                    };
                    let end = l.bounds[(i + 1) % l.bounds.len()].start;
                    stretches.entry((bi, side)).or_default().push((
                        j.local(b.start).y,
                        j.local(end).y,
                        li,
                        i,
                    ));
                }
            }
        }
        let mut twin = HashMap::new();
        for (li, l) in loops.iter().enumerate() {
            if l.strip < nw {
                continue;
            }
            let n = l.bounds.len();
            for i in 0..n {
                let Kind::Tangent(bi, side) = l.kinds[i] else {
                    continue;
                };
                let (y0, y1) = (l.bounds[i].start.y, l.bounds[(i + 1) % n].start.y);
                let list = stretches.get(&(bi, side)).map(Vec::as_slice).unwrap_or(&[]);
                // the wall runs the stretch the other way
                let best = list
                    .iter()
                    .map(|&(a, b, wl, wi)| ((a - y1).abs().max((b - y0).abs()), wl, wi))
                    .min_by(|a, b| a.0.total_cmp(&b.0));
                let joint = cx.sheet.joints[bends[bi].joint].id;
                let Some((_, wl, wi)) = best.filter(|b| b.0 <= TOL) else {
                    let at = l.bounds[i].start;
                    return Err(SheetError::TangentMismatch {
                        feature,
                        joint,
                        side,
                        at,
                        gap: best.map_or(f64::INFINITY, |b| b.0),
                        limit: TOL,
                    });
                };
                let wn = loops[wl].bounds.len();
                slot[li][i] = slot[wl][(wi + 1) % wn];
                slot[li][(i + 1) % n] = slot[wl][wi];
                twin.insert((li, i), (wl, wi));
                twin.insert((wl, wi), (li, i));
            }
        }
        // every wall stretch must have found its bend's
        for (&(bi, side), list) in &stretches {
            for &(_, _, wl, wi) in list {
                if !twin.contains_key(&(wl, wi)) {
                    return Err(SheetError::TangentMismatch {
                        feature,
                        joint: cx.sheet.joints[bends[bi].joint].id,
                        side,
                        at: loops[wl].bounds[wi].start,
                        gap: f64::INFINITY,
                        limit: TOL,
                    });
                }
            }
        }
        for (li, l) in loops.iter().enumerate() {
            for (i, b) in l.bounds.iter().enumerate() {
                if slot[li][i] == usize::MAX {
                    slot[li][i] = slots.len();
                    slots.push((l.strip, b.start));
                }
            }
        }
        // one piece of sheet: every strip component reached from the first across tangent lines
        let pieces: Vec<(usize, usize)> = {
            let mut v: Vec<(usize, usize)> = loops.iter().map(|l| (l.strip, l.comp)).collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        if pieces.len() > 1 {
            let at = |p: (usize, usize)| pieces.iter().position(|x| *x == p).expect("listed");
            let mut reached = vec![false; pieces.len()];
            reached[0] = true;
            let mut todo = vec![0usize];
            while let Some(k) = todo.pop() {
                for (&(l, _), &(m, _)) in &twin {
                    let (a, b) = (
                        at((loops[l].strip, loops[l].comp)),
                        at((loops[m].strip, loops[m].comp)),
                    );
                    for (x, y) in [(a, b), (b, a)] {
                        if x == k && !reached[y] {
                            reached[y] = true;
                            todo.push(y);
                        }
                    }
                }
            }
            if let Some(k) = reached.iter().position(|r| !r) {
                return Err(SheetError::SheetSplit {
                    feature,
                    a: strips[pieces[0].0].strip,
                    b: strips[pieces[k].0].strip,
                });
            }
        }
        Ok(Strips {
            cx,
            strips,
            loops,
            t,
            slot,
            slots,
            twin,
        })
    }

    /// Refuses a folded part whose strips pass through each other, deeper than the limit (ten
    /// tolerances) through another strip's thickness. b's depth is affine along each ruling of
    /// a's faces (planes, cylinders, ruled walls), so its greatest over a's boundary is on a's
    /// edges: each edge at both skins, and each rail. Each is cut in halves while a rigorous box of
    /// the half (the piece's own box, or the wrapped curve's) can still reach past the limit into
    /// b's thickness and meet b's region; a box under half the limit across is decided at its
    /// middle, stepped ten tolerances into a's own material (so walls ending flush along one line
    /// still find each other). Depth is 1-Lipschitz, so no point deeper than 1.5 limits is missed.
    fn clash(&self) -> Result<(), SheetError> {
        let t = self.t;
        let limit = 10.0 * TOL;
        let ns = self.strips.len();
        let mut joined = vec![false; ns * ns];
        for (&(l, _), &(m, _)) in &self.twin {
            let (a, b) = (self.loops[l].strip, self.loops[m].strip);
            joined[a * ns + b] = true;
            joined[b * ns + a] = true;
        }
        // every strip's box, and its region's box in its own coordinates
        let mut boxes = vec![forge_math::Box3::EMPTY; ns];
        let mut flat = vec![
            (
                Point2::new(f64::INFINITY, f64::INFINITY),
                Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY)
            );
            ns
        ];
        for ld in &self.loops {
            let (si, geo) = (ld.strip, self.strips[ld.strip].folded);
            for b in &ld.bounds {
                let bx = b.piece.bbox();
                let f = &mut flat[si];
                *f = (
                    Point2::new(f.0.x.min(bx.x0), f.0.y.min(bx.y0)),
                    Point2::new(f.1.x.max(bx.x1), f.1.y.max(bx.y1)),
                );
                for w in [0.0, t] {
                    boxes[si] = boxes[si].hull(&piece_box(geo, &b.piece, w, Interval::new(0.0, 1.0)));
                }
            }
        }
        for a in 0..ns {
            let geo = self.strips[a].folded;
            for b in 0..ns {
                if a == b || joined[a * ns + b] || !boxes[a].overlaps(&boxes[b]) {
                    continue;
                }
                let reach = |bx: &forge_math::Box3| self.may_reach(b, bx, flat[b], limit);
                for ld in self.loops.iter().filter(|l| l.strip == a) {
                    for bd in &ld.bounds {
                        let inset = |u: f64| {
                            let d = bd.piece.tangent(u);
                            bd.piece.at(u) + Vec2::new(-d.y, d.x) * (10.0 * TOL / d.length().max(1e-300))
                        };
                        let mut found = None;
                        for w in [0.0, t] {
                            found = found.or_else(|| {
                                self.hunt(
                                    b,
                                    limit,
                                    &|u| piece_box(geo, &bd.piece, w, u),
                                    &|u| geo.point(inset(u), w),
                                    &reach,
                                )
                            });
                        }
                        // the rail at the piece's start, straight through the sheet
                        let q = inset(0.0);
                        found = found.or_else(|| {
                            self.hunt(
                                b,
                                limit,
                                &|u| {
                                    forge_math::Box3::from_points([
                                        geo.point(q, t * u.lo),
                                        geo.point(q, t * u.hi),
                                    ])
                                    .grow(TOL)
                                },
                                &|u| geo.point(q, t * u),
                                &reach,
                            )
                        });
                        if let Some((at, depth)) = found {
                            return Err(SheetError::FoldedClash {
                                feature: self.cx.feature,
                                a: self.strips[a].strip,
                                b: self.strips[b].strip,
                                at,
                                depth,
                                limit,
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// The deepest point found in strip `b` along a curve over fractions 0..1, by halving while
    /// `reach` says a half's box may get deeper than `limit`: a point and its depth, if one is.
    fn hunt(
        &self,
        b: usize,
        limit: f64,
        box_of: &dyn Fn(Interval) -> forge_math::Box3,
        point: &dyn Fn(f64) -> Point3,
        reach: &dyn Fn(&forge_math::Box3) -> bool,
    ) -> Option<(Point3, f64)> {
        let mut stack = vec![(Interval::new(0.0, 1.0), 0u32)];
        while let Some((u, level)) = stack.pop() {
            let bx = box_of(u);
            if !reach(&bx) {
                continue;
            }
            let size = (bx.x.width().powi(2) + bx.y.width().powi(2) + bx.z.width().powi(2)).sqrt();
            if size <= 0.5 * limit || level >= 48 {
                let p = point(u.mid());
                if let Some(d) = self.inside(b, p)
                    && d > limit
                {
                    return Some((p, d));
                }
                continue;
            }
            let m = u.mid();
            stack.push((Interval::new(u.lo, m), level + 1));
            stack.push((Interval::new(m, u.hi), level + 1));
        }
        None
    }

    /// Whether a box may hold a point deeper than `limit` in strip `s`'s thickness, over its region's
    /// box `flat`: rigorous, from the box's corners in the strip's own terms.
    fn may_reach(&self, s: usize, bx: &forge_math::Box3, flat: (Point2, Point2), limit: f64) -> bool {
        let t = self.t;
        let corners = [0, 1, 2, 3, 4, 5, 6, 7].map(|k| {
            Point3::new(
                if k & 1 == 0 { bx.x.lo } else { bx.x.hi },
                if k & 2 == 0 { bx.y.lo } else { bx.y.hi },
                if k & 4 == 0 { bx.z.lo } else { bx.z.hi },
            )
        });
        let local = |f: &Frame| {
            let ps = corners.map(|c| f.point_to_local(c));
            forge_math::Box3::from_points(ps)
        };
        let (w, y, x) = match self.strips[s].folded {
            Geo::Plane(f) => {
                let l = local(&f);
                (Interval::new(-l.z.hi, -l.z.lo), l.y, Some(l.x))
            }
            Geo::Round {
                axis, rho0, sigma, ..
            } => {
                let l = local(&axis);
                // the radius over the box: its nearest and farthest points from the axis
                let near = |i: Interval| {
                    if i.lo > 0.0 {
                        i.lo
                    } else if i.hi < 0.0 {
                        -i.hi
                    } else {
                        0.0
                    }
                };
                let far = |i: Interval| i.lo.abs().max(i.hi.abs());
                let len = |a: f64, b: f64| (a * a + b * b).sqrt();
                let (rlo, rhi) = (len(near(l.x), near(l.y)), len(far(l.x), far(l.y)));
                let (w0, w1) = (sigma * (rlo - rho0), sigma * (rhi - rho0));
                (
                    Interval::new(w0.min(w1), w0.max(w1)),
                    Interval::new(-l.z.hi, -l.z.lo) * sigma,
                    None,
                )
            }
        };
        if w.hi <= limit || w.lo >= t - limit {
            return false;
        }
        let y = Interval::new(y.lo.min(y.hi), y.lo.max(y.hi));
        if y.hi < flat.0.y || y.lo > flat.1.y {
            return false;
        }
        x.is_none_or(|x| x.hi >= flat.0.x && x.lo <= flat.1.x)
    }

    /// The pair of strips (not joined by a tangent line) whose developed regions overlap most,
    /// with the total over all pairs, by area, exactly: area(A and B) = area(A) - area(A less B), B's holes given back. Region
    /// refusals within a pair (a near-coincident touch) count as no overlap.
    fn flat_overlap(&self, strip_dev: &[Rigid]) -> Option<SheetError> {
        const LIMIT: f64 = 1e-6;
        let ns = self.strips.len();
        let joined: std::collections::HashSet<(usize, usize)> = self
            .twin
            .iter()
            .map(|(&(l, _), &(m, _))| (self.loops[l].strip, self.loops[m].strip))
            .collect();
        // each strip's loops in the developed plane: per component, outer then holes
        let comps: Vec<Vec<Vec<ProfileLoop>>> = (0..ns)
            .map(|s| {
                let by = &strip_dev[s];
                self.strips[s]
                    .region
                    .parts
                    .iter()
                    .map(|c| {
                        std::iter::once(&c.outer)
                            .chain(&c.holes)
                            .map(|l| {
                                ProfileLoop::new(
                                    l.iter()
                                        .enumerate()
                                        .filter_map(|(k, b)| {
                                            moved(&b.piece, by).to_segment().map(|g| (g, k as u64))
                                        })
                                        .collect(),
                                )
                            })
                            .collect()
                    })
                    .collect()
            })
            .collect();
        let boxes: Vec<(Point2, Point2)> = comps
            .iter()
            .map(|cs| {
                cs.iter().flatten().map(bbox).fold(
                    (
                        Point2::new(f64::INFINITY, f64::INFINITY),
                        Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
                    ),
                    |(lo, hi), (a, b)| {
                        (
                            Point2::new(lo.x.min(a.x), lo.y.min(a.y)),
                            Point2::new(hi.x.max(b.x), hi.y.max(b.y)),
                        )
                    },
                )
            })
            .collect();
        // area of A inside loop x (counter-clockwise), and a point where it enters
        let inside = |a: &[ProfileLoop], x: &ProfileLoop| -> Result<(f64, Point2), RegionError> {
            let whole = subtract(a, &[])?.area();
            let off = 1u64 << 40;
            let cut = ProfileLoop::new(x.segments.iter().map(|(g, k)| (g.clone(), k + off)).collect());
            let rest = subtract(a, &[cut])?;
            let at = rest
                .parts
                .iter()
                .flat_map(|c| std::iter::once(&c.outer).chain(&c.holes))
                .flatten()
                .find(|b| b.id >= off)
                .map(|b| b.piece.at(0.5))
                .unwrap_or(Point2::ORIGIN);
            Ok((whole - rest.area(), at))
        };
        let mut worst: Option<(usize, usize, Point2, f64)> = None;
        let mut undecided: Option<(usize, usize, RegionError)> = None;
        let mut total = 0.0;
        for a in 0..ns {
            for b in a + 1..ns {
                let (la, ha) = boxes[a];
                let (lb, hb) = boxes[b];
                if joined.contains(&(a, b))
                    || joined.contains(&(b, a))
                    || ha.x <= lb.x + TOL
                    || hb.x <= la.x + TOL
                    || ha.y <= lb.y + TOL
                    || hb.y <= la.y + TOL
                {
                    continue;
                }
                for ca in &comps[a] {
                    for cb in &comps[b] {
                        let (mut area, at) = match inside(ca, &cb[0]) {
                            Ok(x) => x,
                            Err(e) => {
                                undecided.get_or_insert((a, b, e));
                                continue;
                            }
                        };
                        for h in &cb[1..] {
                            match inside(ca, &reversed_loop(h)) {
                                Ok((x, _)) => area -= x,
                                Err(e) => {
                                    undecided.get_or_insert((a, b, e));
                                }
                            }
                        }
                        if area > LIMIT {
                            total += area;
                            if worst.is_none_or(|w| area > w.3) {
                                worst = Some((a, b, at, area));
                            }
                        }
                    }
                }
            }
        }
        if worst.is_none()
            && let Some((a, b, e)) = undecided
        {
            // a pair the exact boolean cannot tell apart from touching: not taken as clear
            let (at, gap, limit) = match e {
                RegionError::NearCoincident { at, gap, limit, .. } => (at, gap, limit),
                RegionError::Pinched { pinches } => pinches
                    .first()
                    .map_or((Point2::ORIGIN, 0.0, 0.0), |p| (p.at, p.width, p.limit)),
                RegionError::Unchained { at, .. } | RegionError::HoleOutside { at, .. } => {
                    (at, f64::NAN, f64::NAN)
                }
            };
            return Some(SheetError::FlatOverlapUndecided {
                feature: self.cx.feature,
                a: self.strips[a].strip,
                b: self.strips[b].strip,
                at,
                gap,
                limit,
            });
        }
        worst.map(|(a, b, at, area)| SheetError::FlatOverlap {
            feature: self.cx.feature,
            a: self.strips[a].strip,
            b: self.strips[b].strip,
            at,
            area,
            total,
            limit: LIMIT,
        })
    }

    /// How deep `p` lies in strip `s`'s material, through its thickness; None outside.
    fn inside(&self, s: usize, p: Point3) -> Option<f64> {
        let t = self.t;
        let (q, w) = match self.strips[s].folded {
            Geo::Plane(f) => {
                let l = f.point_to_local(p);
                (Point2::new(l.x, l.y), -l.z)
            }
            Geo::Round {
                axis,
                rho0,
                sigma,
                rn,
                theta,
                ..
            } => {
                let l = axis.point_to_local(p);
                let phi = dmath::atan2(l.y, l.x);
                if !(phi > 0.0 && phi < theta) {
                    return None;
                }
                let rho = (l.x * l.x + l.y * l.y).sqrt();
                (Point2::new(phi * rn, -sigma * l.z), (rho - rho0) * sigma)
            }
        };
        let skin = w.min(t - w);
        if skin <= 0.0 {
            return None;
        }
        self.strips[s]
            .region
            .parts
            .iter()
            .any(|c| crate::region::holds(c, q))
            .then_some(skin)
    }

    /// The flat pattern: each boundary followed piece by piece across the strips, in the
    /// developed plane, runs of one line or one circle joined; and the bends' middle lines.
    fn pattern(&self, strip_dev: &[Rigid], layout: &Layout) -> Pattern {
        let mut seen: Vec<Vec<bool>> = self.loops.iter().map(|l| vec![false; l.bounds.len()]).collect();
        let mut contours = Vec::new();
        for l0 in 0..self.loops.len() {
            for i0 in 0..self.loops[l0].bounds.len() {
                if seen[l0][i0] || self.loops[l0].kinds[i0] != Kind::Side {
                    continue;
                }
                let mut out: Vec<Curve2> = Vec::new();
                let (mut l, mut i) = (l0, i0);
                while !seen[l][i] {
                    seen[l][i] = true;
                    let ld = &self.loops[l];
                    let by = &strip_dev[ld.strip];
                    let (a, b) = (ld.bounds[i].start, ld.bounds[(i + 1) % ld.bounds.len()].start);
                    if let Some(c) = developed(&ld.bounds[i].piece, a, b, by) {
                        push_joined(&mut out, c);
                    }
                    (l, i) = self.next(l, i).0;
                }
                if out.len() > 1
                    && let Some(c) = joined(&out[out.len() - 1], &out[0])
                {
                    out[0] = c;
                    out.pop();
                }
                contours.push(out);
            }
        }
        let bends = layout
            .bends
            .iter()
            .map(|b| BendLine {
                joint: b.joint,
                a: b.developed.apply(Point2::new(0.5 * b.allowance, 0.0)),
                b: b.developed.apply(Point2::new(0.5 * b.allowance, b.length)),
                angle: b.angle,
                radius: b.radius,
                allowance: b.allowance,
            })
            .collect();
        Pattern { contours, bends }
    }

    /// The side piece after (l, i) along the developed boundary, and whether it crossed a tangent line.
    fn next(&self, l: usize, i: usize) -> ((usize, usize), bool) {
        let (mut l, mut i) = (l, (i + 1) % self.loops[l].bounds.len());
        let mut crossed = false;
        while self.loops[l].kinds[i] != Kind::Side {
            let (l2, i2) = self.twin[&(l, i)];
            l = l2;
            i = (i2 + 1) % self.loops[l].bounds.len();
            crossed = true;
        }
        ((l, i), crossed)
    }

    fn body(self) -> Result<(Body, Roles), SheetError> {
        let depth = [self.t, 0.0];
        // levels: the back skin at depth t, the reference skin at 0
        let level = [Skin::Back, Skin::Reference];
        let mut roles = Roles::default();
        let mut body = Body::new();
        let shell = body.add_shell();
        // the side pieces, their successors, and which successors share a face
        let sides: Vec<(usize, usize)> = (0..self.loops.len())
            .flat_map(|l| (0..self.loops[l].bounds.len()).map(move |i| (l, i)))
            .filter(|&(l, i)| self.loops[l].kinds[i] == Kind::Side)
            .collect();
        let index: HashMap<(usize, usize), usize> = sides.iter().enumerate().map(|(k, s)| (*s, k)).collect();
        let next: Vec<(usize, bool)> = sides
            .iter()
            .map(|&(l, i)| {
                let (n, crossed) = self.next(l, i);
                (index[&n], crossed)
            })
            .collect();
        let mut prev = vec![0usize; sides.len()];
        for (k, n) in next.iter().enumerate() {
            prev[n.0] = k;
        }
        let surfaces: Vec<Surface> = sides
            .iter()
            .map(|&(l, i)| self.side_surface(l, i))
            .collect::<Result<_, _>>()?;
        let merged: Vec<bool> = (0..sides.len())
            .map(|k| next[k].1 && coplanar(&surfaces[k], &surfaces[next[k].0]))
            .collect();
        // names: vertices from the side pieces meeting there, ties by where they lie
        let src = |k: usize| {
            let (l, i) = sides[k];
            self.cx.sources[self.loops[l].bounds[i].id as usize]
        };
        let mut vname: Vec<Name> = vec![Name::NONE; self.slots.len()];
        let mut ranked: Vec<(Name, usize, [i64; 3])> = Vec::with_capacity(sides.len());
        for k in 0..sides.len() {
            let (l, i) = sides[k];
            let v = self.slot[l][i];
            let base = self
                .cx
                .root
                .with_str("vertex")
                .with_name(self.cx.name_of(src(prev[k])))
                .with_name(self.cx.name_of(src(k)));
            let (s, p) = self.slots[v];
            let key = [s as i64, (p.x * 1e6).round() as i64, (p.y * 1e6).round() as i64];
            ranked.push((base, v, key));
        }
        ranked.sort_by(|a, b| a.0.cmp(&b.0).then(a.2.cmp(&b.2)));
        for (r, w) in ranked.iter().enumerate() {
            let rank = ranked[..r].iter().rev().take_while(|q| q.0 == w.0).count();
            vname[w.1] = if rank == 0 { w.0 } else { w.0.with(rank as u64) };
        }
        // a source cut into several side pieces names each by its ends
        let mut count: HashMap<u64, usize> = HashMap::new();
        for &(l, i) in &sides {
            *count.entry(self.loops[l].bounds[i].id).or_default() += 1;
        }
        let piece_name = |k: usize| {
            let (l, i) = sides[k];
            let whole = self.cx.name_of(src(k));
            if count[&self.loops[l].bounds[i].id] == 1 {
                whole
            } else {
                let n = self.loops[l].bounds.len();
                let (a, b) = (self.slot[l][i], self.slot[l][(i + 1) % n]);
                whole.piece(vname[a].with_name(vname[b]))
            }
        };
        let names: Vec<Name> = (0..sides.len()).map(piece_name).collect();
        // vertices on both levels
        let verts: Vec<[VertexId; 2]> = (0..self.slots.len())
            .map(|v| {
                let (s, p) = self.slots[v];
                let geo = self.strips[s].geo;
                [0, 1].map(|lev| body.add_vertex(geo.point(p, depth[lev]), vname[v].with(lev as u64)))
            })
            .collect();
        // side edges
        let mut edges: Vec<[(EdgeId, bool); 2]> = Vec::with_capacity(sides.len());
        for (k, &(l, i)) in sides.iter().enumerate() {
            let n = self.loops[l].bounds.len();
            let (a, b) = (self.slot[l][i], self.slot[l][(i + 1) % n]);
            let mut pair = Vec::with_capacity(2);
            for (lev, &w) in depth.iter().enumerate() {
                let (curve, range, fw) = self.edge_curve(l, i, w).ok_or_else(|| self.too_short(l, i))?;
                let (s, e) = if fw {
                    (verts[a][lev], verts[b][lev])
                } else {
                    (verts[b][lev], verts[a][lev])
                };
                let name = names[k].with_str("edge").with(lev as u64);
                roles.edges.push((src(k), level[lev], name));
                pair.push((body.add_edge(curve, range, s, e, name), fw));
            }
            edges.push([pair[0], pair[1]]);
        }
        // tangent-line edges, made from the wall's side
        let mut tedges: HashMap<(usize, usize), [EdgeId; 2]> = HashMap::new();
        for (l, ld) in self.loops.iter().enumerate() {
            if !matches!(self.strips[ld.strip].strip, Strip::Wall(_)) {
                continue;
            }
            let n = ld.bounds.len();
            for i in 0..n {
                if ld.kinds[i] == Kind::Side {
                    continue;
                }
                let (a, b) = (self.slot[l][i], self.slot[l][(i + 1) % n]);
                let whole = self.cx.name_of(self.cx.sources[ld.bounds[i].id as usize]);
                let name = whole.piece(vname[a].with_name(vname[b]));
                let mut pair = Vec::with_capacity(2);
                for (lev, (va, vb)) in verts[a].iter().zip(&verts[b]).enumerate() {
                    let (pa, pb) = (body.vertex(*va).point, body.vertex(*vb).point);
                    let line = Line::through(pa, pb).ok_or_else(|| self.too_short(l, i))?;
                    pair.push(body.add_edge(
                        Curve::Line(line),
                        Interval::new(0.0, pa.distance(pb)),
                        *va,
                        *vb,
                        name.with(lev as u64),
                    ));
                }
                tedges.insert((l, i), [pair[0], pair[1]]);
            }
        }
        // rails where side faces part
        let mut rails: HashMap<usize, EdgeId> = HashMap::new();
        for k in 0..sides.len() {
            if merged[prev[k]] {
                continue;
            }
            let (l, i) = sides[k];
            let v = self.slot[l][i];
            let (p0, p1) = (body.vertex(verts[v][0]).point, body.vertex(verts[v][1]).point);
            let line = Line::through(p0, p1).ok_or_else(|| self.too_short(l, i))?;
            let e = body.add_edge(
                Curve::Line(line),
                Interval::new(0.0, p0.distance(p1)),
                verts[v][0],
                verts[v][1],
                vname[v].with_str("rail"),
            );
            rails.insert(v, e);
        }
        // side faces: runs of merged pieces
        let mut done = vec![false; sides.len()];
        for k0 in 0..sides.len() {
            if done[k0] || merged[prev[k0]] {
                continue;
            }
            let mut run = vec![k0];
            let mut k = k0;
            while merged[k] && !done[next[k].0] && next[k].0 != k0 {
                k = next[k].0;
                run.push(k);
            }
            for &q in &run {
                done[q] = true;
            }
            let (l0, i0) = sides[k0];
            let (lk, ik) = sides[*run.last().expect("one piece at least")];
            let v0 = self.slot[l0][i0];
            let vk = self.slot[lk][(ik + 1) % self.loops[lk].bounds.len()];
            let surface = surfaces[k0].clone();
            let same = self.side_sense(&body, &surface, edges[k0][0], l0, i0);
            let name = run
                .iter()
                .map(|&q| names[q])
                .min()
                .expect("one piece at least")
                .with_str("side");
            let face = body.add_face(shell, surface, same, name);
            for &q in &run {
                if !roles.sides.contains(&(src(q), name)) {
                    roles.sides.push((src(q), name));
                }
            }
            let mut uses: Vec<(EdgeId, bool)> = run.iter().map(|&q| edges[q][0]).collect();
            uses.push((rails[&vk], true));
            uses.extend(run.iter().rev().map(|&q| (edges[q][1].0, !edges[q][1].1)));
            uses.push((rails[&v0], false));
            body.add_loop(face, &uses);
        }
        // skins: each strip component's two
        for (si, s) in self.strips.iter().enumerate() {
            let comps = s.region.parts.len();
            for ci in 0..comps {
                let mine: Vec<usize> = (0..self.loops.len())
                    .filter(|&l| self.loops[l].strip == si && self.loops[l].comp == ci)
                    .collect();
                let mut name = self.cx.strip_name(s.strip).with_str("skin");
                if comps > 1 {
                    let least = sides
                        .iter()
                        .enumerate()
                        .filter(|(_, (l, _))| *l == mine[0])
                        .map(|(k, _)| names[k])
                        .min()
                        .unwrap_or(Name::NONE);
                    name = name.piece(least);
                }
                for (lev, &w) in depth.iter().enumerate() {
                    let top = lev == 1;
                    let (surface, same) = skin(&s.geo, w, top);
                    let face = body.add_face(shell, surface, same, name.with(lev as u64));
                    roles.skins.push((s.strip, level[lev], name.with(lev as u64)));
                    for &l in &mine {
                        let ld = &self.loops[l];
                        let mut uses: Vec<(EdgeId, bool)> = (0..ld.bounds.len())
                            .map(|i| match ld.kinds[i] {
                                Kind::Side => edges[index[&(l, i)]][lev],
                                Kind::Tangent(..) => match tedges.get(&(l, i)) {
                                    Some(e) => (e[lev], true),
                                    None => (tedges[&self.twin[&(l, i)]][lev], false),
                                },
                            })
                            .collect();
                        if !top {
                            uses.reverse();
                            for u in &mut uses {
                                u.1 = !u.1;
                            }
                        }
                        body.add_loop(face, &uses);
                    }
                }
            }
        }
        Ok((body, roles))
    }

    fn too_short(&self, l: usize, i: usize) -> SheetError {
        let ld = &self.loops[l];
        let b = &ld.bounds[i];
        SheetError::EdgeTooShort {
            feature: self.cx.feature,
            strip: self.strips[ld.strip].strip,
            trim: self.cx.sources[b.id as usize],
            length: b.start.distance(ld.bounds[(i + 1) % ld.bounds.len()].start),
            limit: TOL,
        }
    }

    /// The curve of side piece (l, i) at depth w, its range, and whether it runs with the piece.
    fn edge_curve(&self, l: usize, i: usize, w: f64) -> Option<(Curve, Interval, bool)> {
        let ld = &self.loops[l];
        let n = ld.bounds.len();
        let geo = self.strips[ld.strip].geo;
        let (a, b) = (ld.bounds[i].start, ld.bounds[(i + 1) % n].start);
        let line = || {
            let (pa, pb) = (geo.point(a, w), geo.point(b, w));
            Some((
                Curve::Line(Line::through(pa, pb)?),
                Interval::new(0.0, pa.distance(pb)),
                true,
            ))
        };
        match (geo, &ld.bounds[i].piece) {
            (Geo::Plane(_), Piece::Line { .. }) => line(),
            (Geo::Plane(f), &Piece::Arc { c, r, t0, t1 }) => Some((
                Curve::Circle(Circle {
                    frame: f.with_origin(f.point_to_world(Point3::new(c.x, c.y, -w))),
                    radius: r,
                }),
                Interval::new(t0.min(t1), t0.max(t1)),
                t1 > t0,
            )),
            (Geo::Round { .. }, Piece::Line { .. }) if a.x == b.x => line(),
            (
                Geo::Round {
                    axis,
                    rho0,
                    sigma,
                    rn,
                    theta,
                    ba,
                },
                Piece::Line { .. },
            ) => {
                if a.y == b.y {
                    let (p0, p1) = (phi(a.x, rn, theta, ba), phi(b.x, rn, theta, ba));
                    Some((
                        Curve::Circle(Circle {
                            frame: axis.with_origin(axis.point_to_world(Point3::new(0.0, 0.0, -sigma * a.y))),
                            radius: rho0 + sigma * w,
                        }),
                        Interval::new(p0.min(p1), p0.max(p1)),
                        p1 > p0,
                    ))
                } else {
                    wrapped(axis, rho0 + sigma * w, rn, &ld.bounds[i].piece, a, b, sigma)
                }
            }
            (
                Geo::Round {
                    axis,
                    rho0,
                    sigma,
                    rn,
                    ..
                },
                Piece::Arc { .. } | Piece::Spline { .. },
            ) => wrapped(axis, rho0 + sigma * w, rn, &ld.bounds[i].piece, a, b, sigma),
            (Geo::Plane(f), &Piece::Spline { ref s, t0, t1 }) => {
                let range = Interval::new(t0.min(t1), t0.max(t1));
                let world = own_stretch(s, range)?.mapped(|q| f.point_to_world(Point3::new(q.x, q.y, -w)));
                Some((
                    SplineCurve::curve(std::sync::Arc::new(world), range),
                    range,
                    t1 > t0,
                ))
            }
        }
    }

    /// The surface of the side wall along side piece (l, i), through the sheet.
    fn side_surface(&self, l: usize, i: usize) -> Result<Surface, SheetError> {
        let ld = &self.loops[l];
        let n = ld.bounds.len();
        let geo = self.strips[ld.strip].geo;
        let piece = &ld.bounds[i].piece;
        let (a, b) = (ld.bounds[i].start, ld.bounds[(i + 1) % n].start);
        let plane = |normal: Vec3, at: Point3, up: Vec3| -> Option<Surface> {
            Some(Surface::Plane(Plane::new(Frame::from_zx(
                at,
                normal.normalize()?,
                up,
            )?)))
        };
        let up_at = |p: Point2| geo.point(p, 0.0) - geo.point(p, self.t);
        let fail = || match self.strips[ld.strip].strip {
            Strip::Bend(joint) => SheetError::WallFolds {
                feature: self.cx.feature,
                joint,
                trim: self.cx.sources[ld.bounds[i].id as usize],
            },
            Strip::Wall(_) => self.too_short(l, i),
        };
        let surface = match (geo, piece) {
            (Geo::Plane(f), &Piece::Spline { s: ref c, t0, t1 }) => {
                own_stretch(c, Interval::new(t0.min(t1), t0.max(t1))).and_then(|c| {
                    Extruded::new(f, std::sync::Arc::new(c), Interval::new(-self.t, 0.0))
                        .map(Extruded::surface)
                })
            }
            (Geo::Plane(f), &Piece::Arc { c, r, .. }) => Some(Surface::Cylinder(Cylinder {
                frame: f.with_origin(f.point_to_world(Point3::new(c.x, c.y, 0.0))),
                radius: r,
            })),
            (Geo::Round { axis, .. }, Piece::Line { .. }) if a.y == b.y => {
                let at = geo.point(a, self.t);
                plane(axis.z().get(), at, up_at(a))
            }
            (
                Geo::Round {
                    axis,
                    rho0,
                    sigma,
                    rn,
                    ..
                },
                Piece::Line { .. } | Piece::Arc { .. } | Piece::Spline { .. },
            ) if a.x != b.x || !matches!(piece, Piece::Line { .. }) => {
                let flat = flat_in_bend(piece, a, b, sigma).ok_or_else(fail)?;
                let wrap = |w: f64| {
                    Rail::Wrap(Wrap {
                        axis,
                        radius: rho0 + sigma * w,
                        unroll: rn,
                        flat,
                    })
                };
                Ruled::new(wrap(self.t), wrap(0.0)).map(Ruled::surface)
            }
            (_, Piece::Line { .. }) => {
                let m = a.lerp(b, 0.5);
                let along = geo.point(b, 0.0) - geo.point(a, 0.0);
                let up = up_at(m);
                plane(along.cross(up), geo.point(a, self.t), up)
            }
            _ => None,
        };
        surface.ok_or_else(fail)
    }

    /// Whether the side surface's own normal looks out of the material, judged at the edge's middle.
    fn side_sense(&self, body: &Body, s: &Surface, e: (EdgeId, bool), l: usize, i: usize) -> bool {
        // an extruded spline's normal is C' x z, right of the curve: out of the material (on the
        // piece's left) when the piece runs with the curve
        if let (Geo::Plane(_), Piece::Spline { t0, t1, .. }) = (
            self.strips[self.loops[l].strip].geo,
            &self.loops[l].bounds[i].piece,
        ) {
            return t1 > t0;
        }
        let ed = body.edge(e.0);
        let mid = ed.curve.deriv(ed.range.mid());
        let along = if e.1 { mid.d1 } else { -mid.d1 };
        let ld = &self.loops[l];
        let geo = self.strips[ld.strip].geo;
        let m = ld.bounds[i].piece.at(0.5);
        let up = geo.point(m, 0.0) - geo.point(m, self.t);
        s.normal(s.project(mid.p))
            .is_none_or(|nat| nat.dot(along.cross(up)) > 0.0)
    }
}

/// A bend's oblique line or arc as a flat curve in its wrap's (s, h) = (u, -sigma y): a reflection
/// when sigma is 1, so the arc's angles turn the other way.
/// A piece's own stretch of its spline, in the spline's parameter: a wall built on the whole of a
/// closed spline is carried past its ends onto itself at the seam, where points project to either.
/// A piece carried by a rigid motion.
fn moved(p: &Piece, by: &Rigid) -> Piece {
    match p {
        Piece::Line { a, b } => Piece::Line {
            a: by.apply(*a),
            b: by.apply(*b),
        },
        Piece::Arc { c, r, t0, t1 } => Piece::Arc {
            c: by.apply(*c),
            r: *r,
            t0: t0 + by.angle(),
            t1: t1 + by.angle(),
        },
        Piece::Spline { s, t0, t1 } => Piece::Spline {
            s: std::sync::Arc::new(s.mapped(|q| {
                let m = by.apply(Point2::new(q.x, q.y));
                Point3::new(m.x, m.y, 0.0)
            })),
            t0: *t0,
            t1: *t1,
        },
    }
}

/// A loop run the other way.
fn reversed_loop(l: &ProfileLoop) -> ProfileLoop {
    ProfileLoop::new(
        l.segments
            .iter()
            .rev()
            .map(|(g, k)| {
                let p = &crate::region::pieces(&ProfileLoop::new(vec![(g.clone(), *k)]))[0].0;
                (p.reversed().to_segment().unwrap_or_else(|| g.clone()), *k)
            })
            .collect(),
    )
}

/// A boundary piece from a to b in a strip, carried into the developed plane.
fn developed(p: &Piece, a: Point2, b: Point2, by: &Rigid) -> Option<Curve2> {
    Some(match *p {
        Piece::Line { .. } => Curve2::Line {
            a: by.apply(a),
            b: by.apply(b),
        },
        Piece::Arc { c, r, t0, t1 } => Curve2::Arc {
            c: by.apply(c),
            r,
            start: t0 + by.angle(),
            sweep: t1 - t0,
        },
        Piece::Spline { ref s, t0, t1 } => {
            let st = own_stretch(s, Interval::new(t0.min(t1), t0.max(t1)))?;
            let (degree, mut knots, pts, mut weights) = st.control_form();
            let mut points: Vec<Point2> = pts.iter().map(|q| by.apply(Point2::new(q.x, q.y))).collect();
            if t1 < t0 {
                let (lo, hi) = (knots[0], knots[knots.len() - 1]);
                knots = knots.iter().rev().map(|k| lo + hi - k).collect();
                points.reverse();
                if let Some(w) = &mut weights {
                    w.reverse();
                }
            }
            Curve2::Spline {
                degree,
                knots,
                points,
                weights,
            }
        }
    })
}

/// Two pieces that continue one line or one circle, as one.
fn joined(x: &Curve2, y: &Curve2) -> Option<Curve2> {
    match (x, y) {
        (Curve2::Line { a, b }, Curve2::Line { a: b2, b: c }) => {
            let (u, v) = (*b - *a, *c - *b2);
            let same = u.cross(v).abs() <= 1e-12 * u.length() * v.length() && u.dot(v) > 0.0;
            (same && b.distance(*b2) <= TOL).then_some(Curve2::Line { a: *a, b: *c })
        }
        (
            Curve2::Arc { c, r, start, sweep },
            Curve2::Arc {
                c: c2,
                r: r2,
                start: s2,
                sweep: w2,
            },
        ) => {
            let gap = dmath::sin(0.5 * (start + sweep - s2)).abs();
            let same = c.distance(*c2) <= TOL && (r - r2).abs() <= TOL && sweep * w2 > 0.0 && gap <= 1e-12;
            (same && (sweep + w2).abs() <= dmath::TAU * (1.0 + 1e-12)).then_some(Curve2::Arc {
                c: *c,
                r: *r,
                start: *start,
                sweep: sweep + w2,
            })
        }
        _ => None,
    }
}

fn push_joined(out: &mut Vec<Curve2>, c: Curve2) {
    if let Some(last) = out.last()
        && let Some(j) = joined(last, &c)
    {
        *out.last_mut().expect("a last") = j;
        return;
    }
    out.push(c);
}

fn own_stretch(s: &Spline, range: Interval) -> Option<Spline> {
    s.stretch(range)
}

/// A spline piece (one knot span, cut so) is its Bezier piece, run with the piece. None past cubic.
fn flat_in_bend(p: &Piece, a: Point2, b: Point2, sigma: f64) -> Option<Flat> {
    Some(match *p {
        Piece::Arc { c, r, t0, t1 } => Flat::Arc {
            center: Point2::new(c.x, -sigma * c.y),
            radius: r,
            start: -sigma * t0,
            sweep: -sigma * (t1 - t0),
        },
        Piece::Spline { ref s, t0, t1 } => {
            // cut at a knot to rounding, so a sliver of the next span may come along: the span
            // holding the middle is the piece
            let range = Interval::new(t0.min(t1), t0.max(t1));
            let mid = range.mid();
            let k = s.knots_in(range).iter().filter(|&&k| k <= mid).count();
            let mut h = s.bezier_pieces(range).into_iter().nth(k)?;
            if t1 < t0 {
                h.reverse();
            }
            for q in &mut h {
                q[1] *= -sigma;
            }
            Flat::bezier(&h)?
        }
        Piece::Line { .. } => Flat::Line {
            a: Point2::new(a.x, -sigma * a.y),
            b: Point2::new(b.x, -sigma * b.y),
        },
    })
}

fn wrapped(
    axis: Frame,
    radius: f64,
    rn: f64,
    p: &Piece,
    a: Point2,
    b: Point2,
    sigma: f64,
) -> Option<(Curve, Interval, bool)> {
    let flat = flat_in_bend(p, a, b, sigma)?;
    Some((
        Wrap {
            axis,
            radius,
            unroll: rn,
            flat,
        }
        .curve(),
        Interval::new(0.0, 1.0),
        true,
    ))
}

/// A rigorous box of a strip piece's edge at depth w over fractions u: the piece's own 2D box laid
/// out on a plane strip, the wrapped curve's box on a bend.
fn piece_box(geo: Geo, p: &Piece, w: f64, u: Interval) -> forge_math::Box3 {
    let sub = p.sub(u.lo, u.hi);
    match geo {
        Geo::Plane(f) => {
            let b = sub.bbox();
            forge_math::Box3::from_points(
                [(b.x0, b.y0), (b.x1, b.y0), (b.x0, b.y1), (b.x1, b.y1)]
                    .map(|(x, y)| f.point_to_world(Point3::new(x, y, -w))),
            )
            .grow(TOL)
        }
        Geo::Round {
            axis,
            rho0,
            sigma,
            rn,
            ..
        } => match flat_in_bend(&sub, sub.at(0.0), sub.at(1.0), sigma) {
            Some(flat) => Wrap {
                axis,
                radius: rho0 + sigma * w,
                unroll: rn,
                flat,
            }
            .bbox(Interval::new(0.0, 1.0)),
            // past cubic is refused before this; a box that holds everything never prunes
            None => forge_math::Box3::from_points([
                Point3::new(-1e300, -1e300, -1e300),
                Point3::new(1e300, 1e300, 1e300),
            ]),
        },
    }
}

/// Two side surfaces in one plane, facing one way.
fn coplanar(a: &Surface, b: &Surface) -> bool {
    let (Surface::Plane(p), Surface::Plane(q)) = (a, b) else {
        return false;
    };
    let (n, m) = (p.frame.z().get(), q.frame.z().get());
    n.dot(m).abs() > 1.0 - 1e-12 && (q.frame.origin - p.frame.origin).dot(n).abs() <= TOL
}

fn skin(g: &Geo, w: f64, top: bool) -> (Surface, bool) {
    match *g {
        Geo::Plane(f) => (
            Surface::Plane(Plane::new(
                f.with_origin(f.point_to_world(Point3::new(0.0, 0.0, -w))),
            )),
            top,
        ),
        Geo::Round {
            axis, rho0, sigma, ..
        } => (
            Surface::Cylinder(Cylinder {
                frame: axis,
                radius: rho0 + sigma * w,
            }),
            top == (sigma < 0.0),
        ),
    }
}
