//! Work features: planes, axes, points and named frames (UCS) defined by references to earlier
//! features' faces, edges and vertices, or to other work features. Like extents, they are
//! worked out from the earlier features' tools, never from the part as built, so they follow
//! every upstream edit and both regens see the same. They make no solid.

use crate::{Feature, History, RegenError};
use forge_geom::intersect::{CurveSurface, LineSurface, curve_surface, line_surface};
use forge_geom::{Curve, CurveGeom, Line, Plane, Surface};
use forge_math::{Frame, Interval, Point3, Vec3, tol};
use forge_topo::{Body, Name};

/// An earlier feature (a work feature or plane) or, named, a face, edge or vertex of its tool.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ref {
    pub feature: usize,
    pub name: Option<Name>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Work {
    Plane(PlaneBy),
    Axis(AxisBy),
    Point(PointBy),
    /// A named frame for later features to sit on or end at (its xy).
    Ucs(UcsBy),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlaneBy {
    /// Parallel to a plane, this far along its normal (a face's points out of the material).
    Offset(Ref, f64),
    /// Through an axis lying along a plane, turned this far about the axis from it.
    Angle { plane: Ref, axis: Ref, angle: f64 },
    /// Tangent to a cylinder or sphere and parallel to a plane, on the side its normal points.
    Tangent { face: Ref, plane: Ref },
    /// Through three points: the first its origin, x toward the second.
    Points([Ref; 3]),
    /// Midway between two parallel planes, or bisecting two that meet.
    Mid(Ref, Ref),
    /// Through a point, square to a curve where the curve passes nearest it.
    Normal { curve: Ref, at: Ref },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AxisBy {
    /// From the first point through the second.
    Points(Ref, Ref),
    /// A line edge, a circle's axis, a cylinder's, cone's or torus's axis, a UCS's z.
    Along(Ref),
    /// Where two planes meet.
    Meet(Ref, Ref),
    /// Through a point, square to a plane.
    Normal { plane: Ref, at: Ref },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PointBy {
    /// A vertex, a circle's or sphere's centre, a UCS's origin.
    At(Ref),
    /// Halfway along an edge.
    Mid(Ref),
    /// Where an edge or axis first meets a face or plane.
    Meet(Ref, Ref),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UcsBy {
    Frame(Frame),
    /// Origin, a point along x, a point in the xy plane.
    Points([Ref; 3]),
}

/// What a work feature comes to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Datum {
    Point(Point3),
    Axis(Line),
    /// The frame's xy.
    Plane(Frame),
    Frame(Frame),
}

#[derive(Clone, Debug, PartialEq)]
pub enum WorkError {
    /// Not an earlier work feature, nor an entity of an earlier solid feature's tool.
    NoEntity(Ref),
    /// The reference is not what the definition needs (a point, an axis, a plane, a curve..).
    Not { needs: &'static str, at: Ref },
    /// Parallel planes that should meet, points in a line, an axis off its plane, no hit..
    Degenerate,
}

/// What a reference resolves to.
enum Geo {
    Datum(Datum),
    /// A plane's frame turned so z points out of the material.
    Face(Surface),
    Edge(Curve, Interval),
    Vertex(Point3),
}

fn err(i: usize) -> impl Fn(WorkError) -> RegenError {
    move |error| RegenError::Work { feature: i, error }
}

fn geo(features: History<'_>, i: usize, r: Ref) -> Result<Geo, RegenError> {
    let none = || err(i)(WorkError::NoEntity(r));
    let k = r.feature;
    if k >= i {
        return Err(none());
    }
    let Some(name) = r.name else {
        return match &features[k] {
            Feature::Plane(f) => Ok(Geo::Datum(Datum::Plane(*f))),
            Feature::Sketch(s) => Ok(Geo::Datum(Datum::Plane(s.plane))),
            Feature::Work(_) => datum(features, k).map(Geo::Datum),
            _ => Err(none()),
        };
    };
    if let Feature::Sketch(s) = &features[k] {
        return s.point_by(name).map(|p| Geo::Vertex(p.1)).ok_or_else(none);
    }
    if crate::modifies(&features[k]) {
        return Err(none());
    }
    for (b, ..) in crate::tools(features, k, true)? {
        if let Some(f) = b.face_by_name(name) {
            let f = b.face(f);
            return Ok(Geo::Face(match &f.surface {
                Surface::Plane(p) if !f.same_sense => Surface::Plane(Plane {
                    frame: p.frame.flipped(),
                }),
                s => s.clone(),
            }));
        }
        if let Some(e) = b.edge_by_name(name) {
            let e = b.edge(e);
            return Ok(Geo::Edge(e.curve.clone(), e.range));
        }
        if let Some(v) = b.vertex_ids().find(|&v| b.vertex(v).name == name) {
            return Ok(Geo::Vertex(b.vertex(v).point));
        }
    }
    Err(none())
}

fn point(features: History<'_>, i: usize, r: Ref) -> Result<Point3, RegenError> {
    match geo(features, i, r)? {
        Geo::Vertex(p) | Geo::Datum(Datum::Point(p)) => Ok(p),
        Geo::Datum(Datum::Frame(f)) => Ok(f.origin),
        Geo::Edge(Curve::Circle(c), _) => Ok(c.frame.origin),
        Geo::Face(Surface::Sphere(s)) => Ok(s.frame.origin),
        _ => Err(err(i)(WorkError::Not {
            needs: "a point",
            at: r,
        })),
    }
}

/// An axis: origin, direction.
fn axis(features: History<'_>, i: usize, r: Ref) -> Result<Line, RegenError> {
    let z = |f: &Frame| Line {
        origin: f.origin,
        dir: f.z(),
    };
    match geo(features, i, r)? {
        Geo::Datum(Datum::Axis(l)) => Ok(l),
        Geo::Datum(Datum::Frame(f)) => Ok(z(&f)),
        Geo::Edge(Curve::Line(l), range) => Ok(Line {
            origin: l.eval(range.lo),
            dir: l.dir,
        }),
        Geo::Edge(Curve::Circle(c), _) => Ok(z(&c.frame)),
        Geo::Face(Surface::Cylinder(c)) => Ok(z(&c.frame)),
        Geo::Face(Surface::Cone(c)) => Ok(z(&c.frame)),
        Geo::Face(Surface::Torus(c)) => Ok(z(&c.frame)),
        _ => Err(err(i)(WorkError::Not {
            needs: "an axis",
            at: r,
        })),
    }
}

/// A plane, as a frame whose xy it is.
fn plane(features: History<'_>, i: usize, r: Ref) -> Result<Frame, RegenError> {
    match geo(features, i, r)? {
        Geo::Datum(Datum::Plane(f) | Datum::Frame(f)) => Ok(f),
        Geo::Face(Surface::Plane(p)) => Ok(p.frame),
        Geo::Edge(Curve::Circle(c), _) => Ok(c.frame),
        _ => Err(err(i)(WorkError::Not {
            needs: "a plane",
            at: r,
        })),
    }
}

/// A curve over a range: an edge, or an axis (unbounded).
fn curve(features: History<'_>, i: usize, r: Ref) -> Result<(Curve, Option<Interval>), RegenError> {
    match geo(features, i, r)? {
        Geo::Edge(c, range) => Ok((c, Some(range))),
        Geo::Datum(Datum::Axis(l)) => Ok((Curve::Line(l), None)),
        _ => Err(err(i)(WorkError::Not {
            needs: "an edge or axis",
            at: r,
        })),
    }
}

/// A surface: a face, or a plane.
fn surface(features: History<'_>, i: usize, r: Ref) -> Result<Surface, RegenError> {
    match geo(features, i, r)? {
        Geo::Face(s) => Ok(s),
        Geo::Datum(Datum::Plane(f) | Datum::Frame(f)) => Ok(Surface::Plane(Plane { frame: f })),
        _ => Err(err(i)(WorkError::Not {
            needs: "a face or plane",
            at: r,
        })),
    }
}

/// The frame through `o` with z along `z`, x along `x` (made square to z).
fn frame(o: Point3, z: Vec3, x: Vec3) -> Option<Frame> {
    Frame::from_zx(o, z.normalize()?, x)
}

/// The plane through three points: origin the first, x toward the second.
fn three(p: [Point3; 3]) -> Option<Frame> {
    let (a, b) = (p[1] - p[0], p[2] - p[0]);
    if a.cross(b).length() <= tol::LINEAR * (a.length() + b.length()) {
        return None;
    }
    frame(p[0], a.cross(b), a)
}

/// Where two planes meet.
fn meet(p: &Frame, q: &Frame) -> Option<Line> {
    let (n, m) = (p.z().get(), q.z().get());
    if n.cross(m).length() <= tol::ANGULAR {
        return None;
    }
    let dir = n.cross(m).normalize()?;
    // the point of the line nearest p's origin: in both planes, square to the line
    let (dn, dm) = (n.dot(p.origin.to_vec()), m.dot(q.origin.to_vec()));
    let o = p.origin.to_vec();
    let du = dir.dot(o);
    // solve [n; m; dir] x = [dn; dm; du]
    let (a, b, c) = (n, m, dir.get());
    let det = a.dot(b.cross(c));
    let x = (b.cross(c) * dn + c.cross(a) * dm + a.cross(b) * du) * (1.0 / det);
    Some(Line {
        origin: Point3::ORIGIN + x,
        dir,
    })
}

/// What work feature `i` comes to.
pub fn datum(features: History<'_>, i: usize) -> Result<Datum, RegenError> {
    let Feature::Work(w) = &features[i] else {
        return match features[i] {
            Feature::Plane(f) => Ok(Datum::Plane(f)),
            _ => Err(err(i)(WorkError::NoEntity(Ref {
                feature: i,
                name: None,
            }))),
        };
    };
    let degenerate = || err(i)(WorkError::Degenerate);
    let pts = |rs: &[Ref; 3]| -> Result<[Point3; 3], RegenError> {
        Ok([
            point(features, i, rs[0])?,
            point(features, i, rs[1])?,
            point(features, i, rs[2])?,
        ])
    };
    Ok(match *w {
        Work::Plane(by) => Datum::Plane(match by {
            PlaneBy::Offset(r, d) => {
                let f = plane(features, i, r)?;
                f.with_origin(f.origin + f.z() * d)
            }
            PlaneBy::Angle {
                plane: p,
                axis: a,
                angle,
            } => {
                let (f, l) = (plane(features, i, p)?, axis(features, i, a)?);
                if f.z().dot(l.dir.get()).abs() > tol::ANGULAR {
                    return Err(degenerate());
                }
                // turn the plane's normal about the axis
                let n = Frame::from_zx(l.origin, l.dir, f.z().get())
                    .ok_or_else(degenerate)?
                    .rotated_about_z(angle)
                    .x();
                Frame::from_zx(l.origin, n, l.dir.get()).ok_or_else(degenerate)?
            }
            PlaneBy::Tangent { face, plane: p } => {
                let n = plane(features, i, p)?.z();
                let (c, r) = match surface(features, i, face)? {
                    Surface::Cylinder(c) if c.frame.z().dot(n.get()).abs() <= tol::ANGULAR => {
                        (c.frame.origin, c.radius)
                    }
                    Surface::Cylinder(_) => return Err(degenerate()),
                    Surface::Sphere(s) => (s.frame.origin, s.radius),
                    _ => {
                        return Err(err(i)(WorkError::Not {
                            needs: "a cylinder or sphere",
                            at: face,
                        }));
                    }
                };
                Frame::from_z(c + n * r, n)
            }
            PlaneBy::Points(rs) => three(pts(&rs)?).ok_or_else(degenerate)?,
            PlaneBy::Mid(a, b) => {
                let (p, q) = (plane(features, i, a)?, plane(features, i, b)?);
                let (n, m) = (p.z().get(), q.z().get());
                if n.cross(m).length() <= tol::ANGULAR {
                    p.with_origin(p.origin + n * (n.dot(q.origin - p.origin) / 2.0))
                } else {
                    let l = meet(&p, &q).ok_or_else(degenerate)?;
                    frame(l.origin, n - m, l.dir.get()).ok_or_else(degenerate)?
                }
            }
            PlaneBy::Normal { curve: c, at } => {
                let ((c, range), p) = (curve(features, i, c)?, point(features, i, at)?);
                let range = range.unwrap_or(Interval::new(-1e9, 1e9));
                let t = c.closest(p, range);
                let z = c.tangent(t).ok_or_else(degenerate)?;
                Frame::from_z(p, z)
            }
        }),
        Work::Axis(by) => Datum::Axis(match by {
            AxisBy::Points(a, b) => {
                let (p, q) = (point(features, i, a)?, point(features, i, b)?);
                if p.distance(q) <= tol::LINEAR {
                    return Err(degenerate());
                }
                Line::through(p, q).ok_or_else(degenerate)?
            }
            AxisBy::Along(r) => axis(features, i, r)?,
            AxisBy::Meet(a, b) => {
                meet(&plane(features, i, a)?, &plane(features, i, b)?).ok_or_else(degenerate)?
            }
            AxisBy::Normal { plane: p, at } => Line {
                origin: point(features, i, at)?,
                dir: plane(features, i, p)?.z(),
            },
        }),
        Work::Point(by) => Datum::Point(match by {
            PointBy::At(r) => point(features, i, r)?,
            PointBy::Mid(r) => match curve(features, i, r)? {
                (c, Some(range)) => c.eval(at_length(&c, range, length(&c, range) / 2.0)),
                _ => {
                    return Err(err(i)(WorkError::Not {
                        needs: "an edge",
                        at: r,
                    }));
                }
            },
            PointBy::Meet(a, b) => {
                let ((c, range), s) = (curve(features, i, a)?, surface(features, i, b)?);
                let first = match (&c, range) {
                    (Curve::Line(l), None) => match line_surface(l, &s) {
                        LineSurface::Hits(h) => h.first().map(|h| h.point),
                        LineSurface::OnSurface => None,
                    },
                    (_, range) => match curve_surface(&c, range.expect("edges have ranges"), &s) {
                        CurveSurface::Hits(h) => h.first().map(|h| h.point),
                        _ => None,
                    },
                };
                first.ok_or_else(degenerate)?
            }
        }),
        Work::Ucs(by) => Datum::Frame(match by {
            UcsBy::Frame(f) => f,
            UcsBy::Points(rs) => three(pts(&rs)?).ok_or_else(degenerate)?,
        }),
    })
}

/// The name feature `k`'s own solid gives the entity `name` of `part`, found by its geometry:
/// a later boolean renames the faces and edges it splits or trims. None if it is none of `k`'s.
pub fn own_name(features: History<'_>, k: usize, part: &Body, name: Name) -> Option<Name> {
    if crate::modifies(features.get(k)?) {
        return None;
    }
    let tools = crate::tools(features, k, true).ok()?;
    let has = |b: &Body| {
        b.face_by_name(name).is_some()
            || b.edge_by_name(name).is_some()
            || b.vertex_ids().any(|v| b.vertex(v).name == name)
    };
    let mut found = None;
    for (b, ..) in &tools {
        if has(b) {
            return Some(name);
        }
        found = found.or_else(|| {
            if let Some(f) = part.face_by_name(name) {
                let s = &part.face(f).surface;
                b.face_ids()
                    .map(|g| b.face(g))
                    .find(|g| g.surface == *s)
                    .map(|g| g.name)
            } else if let Some(e) = part.edge_by_name(name) {
                let e = part.edge(e);
                b.edge_ids()
                    .map(|x| b.edge(x))
                    .find(|x| x.curve == e.curve && x.range.lo < e.range.hi && e.range.lo < x.range.hi)
                    .map(|x| x.name)
            } else {
                let p = part
                    .vertex_ids()
                    .map(|v| part.vertex(v))
                    .find(|v| v.name == name)?
                    .point;
                b.vertex_ids()
                    .map(|v| b.vertex(v))
                    .find(|v| v.point.distance(p) <= 10.0 * tol::LINEAR)
                    .map(|v| v.name)
            }
        });
    }
    found
}

/// Where a reference to a point (a work point, a vertex, a sketch's point, a UCS's origin..)
/// is, as the line after `features` sees it.
pub fn locate(features: &[Feature], r: Ref) -> Result<Point3, RegenError> {
    point(features, features.len(), r)
}

/// The plane work feature or plane `k` stands for (a UCS: its xy), as seen from feature `i`.
pub(crate) fn plane_of(features: History<'_>, i: usize, k: usize) -> Result<Frame, RegenError> {
    plane(
        features,
        i,
        Ref {
            feature: k,
            name: None,
        },
    )
}

/// The points a reference stands for, as feature `i` sees them.
pub(crate) fn points(features: History<'_>, i: usize, r: Ref) -> Result<Vec<Point3>, RegenError> {
    Ok(vec![point(features, i, r)?])
}

/// The chain of edges and axes `refs` name, as feature `i` sees them.
pub(crate) fn curves(
    features: History<'_>,
    i: usize,
    refs: &[Ref],
) -> Result<Vec<(Curve, Option<Interval>)>, RegenError> {
    refs.iter().map(|&r| curve(features, i, r)).collect()
}

/// 5-point Gauss-Legendre nodes and weights on [-1, 1].
const GAUSS: [(f64, f64); 5] = [
    (0.0, 0.568_888_888_888_888_9),
    (-0.538_469_310_105_683_1, 0.478_628_670_499_366_5),
    (0.538_469_310_105_683_1, 0.478_628_670_499_366_5),
    (-0.906_179_845_938_664, 0.236_926_885_056_189_1),
    (0.906_179_845_938_664, 0.236_926_885_056_189_1),
];

/// Pieces a curve with no closed-form length is measured in.
const PIECES: usize = 64;

fn piece_length(c: &Curve, a: f64, b: f64) -> f64 {
    let (m, h) = ((a + b) / 2.0, (b - a) / 2.0);
    GAUSS
        .iter()
        .map(|&(x, w)| w * c.deriv(m + h * x).d1.length())
        .sum::<f64>()
        * h
}

/// The length of a curve over a range.
pub(crate) fn length(c: &Curve, range: Interval) -> f64 {
    match c {
        Curve::Line(_) => range.hi - range.lo,
        Curve::Circle(k) => k.radius * (range.hi - range.lo),
        _ => {
            let w = (range.hi - range.lo) / PIECES as f64;
            (0..PIECES)
                .map(|k| piece_length(c, range.lo + w * k as f64, range.lo + w * (k + 1) as f64))
                .sum()
        }
    }
}

/// The parameter `s` along a curve from the start of its range.
pub(crate) fn at_length(c: &Curve, range: Interval, s: f64) -> f64 {
    match c {
        Curve::Line(_) => range.lo + s,
        Curve::Circle(k) => range.lo + s / k.radius,
        _ => {
            let w = (range.hi - range.lo) / PIECES as f64;
            let mut gone = 0.0;
            for k in 0..PIECES {
                let a = range.lo + w * k as f64;
                let l = piece_length(c, a, a + w);
                if gone + l >= s || k + 1 == PIECES {
                    return a + w * ((s - gone) / l).clamp(0.0, 1.0);
                }
                gone += l;
            }
            range.hi
        }
    }
}
