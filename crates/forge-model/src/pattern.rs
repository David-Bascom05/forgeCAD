//! Copies of features or of the whole part: rectangular and circular patterns, mirrors, along a
//! path and at points.
//! A copy is the built solid moved by a rigid motion or reflected, never rebuilt.

use forge_bool::{Seg2, Shape};
use forge_geom::intcurve::IntCurve;
use forge_geom::{Curve, CurveGeom, Line, Surface, SurfaceGeom};
use forge_math::{Box3, Frame, Point2, Point3, Vec3, dmath, tol};
use forge_topo::{Body, EdgeId, Name, ShellId, VertexId};

use crate::work::Ref;
use crate::{History, RegenError};

/// What a pattern or mirror copies.
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    /// These earlier features (by position in the history), each copy applying them in order.
    Features(Vec<usize>),
    /// The whole part so far, every copy added to it.
    Body,
}

/// One direction of a rectangular pattern: `count` places (the original's included), `spacing` apart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Row {
    pub dir: Vec3,
    pub count: usize,
    pub spacing: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Layout {
    /// Along one or two directions.
    Linear { first: Row, second: Option<Row> },
    /// `count` places about the line through `origin` along `axis`. Fitted, they fill `angle`
    /// (a whole turn spaces them evenly round; less puts the first and last `angle` apart);
    /// `incremental`, `angle` is the step from one to the next.
    Circular {
        origin: Point3,
        axis: Vec3,
        count: usize,
        angle: f64,
        incremental: bool,
    },
    /// Reflected in the plane's xy.
    Mirror { plane: Frame },
    /// `count` places along a chain of edges or an axis, the original at its start: `spacing`
    /// apart, or (None) spread over its whole length. `turn`: each copy turned as the path's
    /// tangent turns, else only moved. `reverse`: the chain walked the other way.
    Path {
        along: Vec<Ref>,
        count: usize,
        spacing: Option<f64>,
        turn: bool,
        reverse: bool,
    },
    /// The original at the first point, a copy moved to each of the others.
    Points(Vec<At>),
}

/// A point to place a copy at.
#[derive(Clone, Debug, PartialEq)]
pub enum At {
    Xyz(Point3),
    /// A work point or vertex.
    Ref(Ref),
    /// A sketch's points: those named, or (none named) all its `pt` points, and with `centers`
    /// its circles' and arcs' centres too. Kept by name so the file says them again; `at` is
    /// where they were solved when the line was read, in the sketch's order.
    Sketch {
        feature: usize,
        points: Vec<String>,
        centers: bool,
        at: Vec<Point3>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum PatternError {
    /// A zero or non-finite direction, axis, spacing or angle, or a count of 0.
    BadLayout,
    /// The copied feature is not an earlier solid feature or feature pattern.
    BadSource { feature: usize },
    /// The copy holds geometry a mirror cannot reflect yet.
    Unsupported(&'static str),
}

/// The name an entity gets in copy `key` of pattern `pattern` (a feature position): stable
/// across regens, so later features can refer to one copy's faces and edges.
pub fn instance_name(id: u64, key: [u64; 2], name: Name) -> Name {
    Name::feature(id)
        .with_str("instance")
        .with(key[0])
        .with(key[1])
        .with_name(name)
}

/// A rigid motion (local coordinates of the frame to world), or a reflection in a frame's xy.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Motion {
    Rigid(Frame),
    Mirror(Frame),
}

impl Motion {
    pub(crate) fn point(&self, p: Point3) -> Point3 {
        match self {
            Motion::Rigid(f) => f.point_to_world(p),
            Motion::Mirror(m) => {
                let l = m.point_to_local(p);
                m.point_to_world(Point3::new(l.x, l.y, -l.z))
            }
        }
    }

    fn vec(&self, v: Vec3) -> Vec3 {
        match self {
            Motion::Rigid(f) => f.vec_to_world(v),
            Motion::Mirror(m) => {
                let l = m.vec_to_local(v);
                m.vec_to_world(Vec3::new(l.x, l.y, -l.z))
            }
        }
    }

    /// The image of a frame. A reflection's image is left-handed, so one axis is turned back:
    /// z (`keep_z` false, local x and y stay put: right for planar curves) or y (local y
    /// negated: right for surfaces of revolution, whose angle then runs the other way).
    pub(crate) fn frame(&self, f: &Frame, keep_z: bool) -> Frame {
        match self {
            Motion::Rigid(r) => r.compose(f),
            Motion::Mirror(_) => {
                let o = self.point(f.origin);
                let (x, z) = (self.vec(f.x().get()), self.vec(f.z().get()));
                let z = if keep_z { z } else { -z };
                z.normalize()
                    .and_then(|z| Frame::from_zx(o, z, x))
                    .expect("a reflected frame stays orthonormal")
            }
        }
    }

    fn mirrors(&self) -> bool {
        matches!(self, Motion::Mirror(_))
    }

    pub(crate) fn bbox(&self, b: &Box3) -> Box3 {
        if b.is_empty() {
            return *b;
        }
        Box3::from_points(corners(b).map(|p| self.point(p)))
    }
}

pub(crate) fn corners(b: &Box3) -> [Point3; 8] {
    std::array::from_fn(|k| {
        Point3::new(
            if k & 1 == 0 { b.x.lo } else { b.x.hi },
            if k & 2 == 0 { b.y.lo } else { b.y.hi },
            if k & 4 == 0 { b.z.lo } else { b.z.hi },
        )
    })
}

/// For each copy pattern `i`'s layout makes (the original left out), the point it carries
/// onto `p`.
pub fn copy_sources(
    features: History<'_>,
    i: usize,
    layout: &Layout,
    p: Point3,
) -> Result<Vec<Point3>, RegenError> {
    Ok(instances(features, i, layout)?
        .into_iter()
        .map(|(_, m)| match m {
            Motion::Rigid(f) => f.point_to_local(p),
            // a reflection undoes itself
            Motion::Mirror(_) => m.point(p),
        })
        .collect())
}

/// Each copy pattern `i`'s layout makes, as the affine map it is: the images of x, y and z and
/// of the origin (a reflection for a mirror), for checking against another kernel.
pub fn placements(features: History<'_>, i: usize, layout: &Layout) -> Result<Vec<[Point3; 4]>, RegenError> {
    Ok(instances(features, i, layout)?
        .iter()
        .map(|(_, m)| columns(m))
        .collect())
}

/// A body move's steps as the affine map they make (as `placements` gives it).
pub fn body_motion(steps: &[crate::Step]) -> [Point3; 4] {
    columns(&crate::bodies::motion(steps))
}

fn columns(m: &Motion) -> [Point3; 4] {
    let o = Point3::ORIGIN;
    let at = m.point(o);
    let col = |v: Vec3| Point3::ORIGIN + (m.point(o + v) - at);
    [col(Vec3::X), col(Vec3::Y), col(Vec3::Z), at]
}

/// Every copy pattern `i`'s layout makes (the original left out), keyed by its place in it.
pub(crate) fn instances(
    features: History<'_>,
    i: usize,
    layout: &Layout,
) -> Result<Vec<([u64; 2], Motion)>, RegenError> {
    let pe = |error| RegenError::Pattern { feature: i, error };
    match layout {
        Layout::Path {
            along,
            count,
            spacing,
            turn,
            reverse,
        } => crate::path::along(features, i, along, *count, *spacing, *turn, *reverse),
        Layout::Points(at) => crate::path::at_points(features, i, at),
        l => fixed(l).map_err(pe),
    }
}

fn fixed(layout: &Layout) -> Result<Vec<([u64; 2], Motion)>, PatternError> {
    match *layout {
        Layout::Linear { first, second } => {
            let row = |r: Row| -> Result<(Vec3, usize), PatternError> {
                let d = r.dir.normalize().ok_or(PatternError::BadLayout)?;
                if r.count == 0 || !r.spacing.is_finite() || !r.dir.is_finite() {
                    return Err(PatternError::BadLayout);
                }
                Ok((d * r.spacing, r.count))
            };
            let (a, na) = row(first)?;
            let (b, nb) = match second {
                Some(r) => row(r)?,
                None => (Vec3::ZERO, 1),
            };
            let mut out = Vec::with_capacity(na * nb);
            for j in 0..nb {
                for i in 0..na {
                    if i + j > 0 {
                        let o = Point3::ORIGIN + a * i as f64 + b * j as f64;
                        out.push(([i as u64, j as u64], Motion::Rigid(Frame::WORLD.with_origin(o))));
                    }
                }
            }
            Ok(out)
        }
        Layout::Circular {
            origin,
            axis,
            count,
            angle,
            incremental,
        } => {
            let z = axis.normalize().ok_or(PatternError::BadLayout)?;
            if count == 0 || !angle.is_finite() || angle.abs() <= tol::ANGULAR || !origin.is_finite() {
                return Err(PatternError::BadLayout);
            }
            let step = if incremental {
                angle
            } else if angle.abs() >= dmath::TAU - tol::ANGULAR {
                angle.signum() * dmath::TAU / count as f64
            } else if count > 1 {
                angle / (count - 1) as f64
            } else {
                0.0
            };
            let at = Frame::from_z(origin, z);
            let back = at.inverse();
            Ok((1..count)
                .map(|k| {
                    let turn = at.rotated_about_z(step * k as f64).compose(&back);
                    ([k as u64, 0], Motion::Rigid(turn))
                })
                .collect())
        }
        Layout::Mirror { plane } => Ok(vec![([1, 0], Motion::Mirror(plane))]),
        Layout::Path { .. } | Layout::Points(_) => unreachable!("placed from the features"),
    }
}

fn curve(m: &Motion, c: &Curve) -> Result<Curve, PatternError> {
    if let Motion::Rigid(f) = m {
        return Ok(c.transformed(f));
    }
    Ok(match c {
        Curve::Line(l) => Curve::Line(Line {
            origin: m.point(l.origin),
            dir: m.vec(l.dir.get()).normalize().expect("a reflected unit vector"),
        }),
        Curve::Circle(c) => Curve::Circle(forge_geom::Circle {
            frame: m.frame(&c.frame, false),
            ..*c
        }),
        Curve::Ellipse(c) => Curve::Ellipse(forge_geom::Ellipse {
            frame: m.frame(&c.frame, false),
            ..*c
        }),
        Curve::Parabola(c) => Curve::Parabola(forge_geom::Parabola {
            frame: m.frame(&c.frame, false),
            ..*c
        }),
        Curve::Hyperbola(c) => Curve::Hyperbola(forge_geom::Hyperbola {
            frame: m.frame(&c.frame, false),
            ..*c
        }),
        Curve::Intersection(c) => Curve::Intersection(std::sync::Arc::new(IntCurve::new(
            surface(m, &c.a)?,
            surface(m, &c.b)?,
            c.pts.iter().map(|&p| m.point(p)).collect(),
            c.tans.iter().map(|&v| m.vec(v)).collect(),
            c.closed,
            c.sag,
        ))),
        Curve::Procedural(_) => {
            return Err(PatternError::Unsupported(
                "mirroring a curve built by construction",
            ));
        }
    })
}

fn surface(m: &Motion, s: &Surface) -> Result<Surface, PatternError> {
    if let Motion::Rigid(f) = m {
        return Ok(s.transformed(f));
    }
    // every natural normal is the reflection of the old one, so faces keep their sense
    Ok(match s {
        Surface::Plane(p) => Surface::Plane(forge_geom::Plane {
            frame: m.frame(&p.frame, true),
        }),
        Surface::Cylinder(c) => {
            let mut c = *c;
            c.frame = m.frame(&c.frame, true);
            Surface::Cylinder(c)
        }
        Surface::Cone(c) => {
            let mut c = *c;
            c.frame = m.frame(&c.frame, true);
            Surface::Cone(c)
        }
        Surface::Sphere(c) => {
            let mut c = *c;
            c.frame = m.frame(&c.frame, true);
            Surface::Sphere(c)
        }
        Surface::Torus(c) => {
            let mut c = *c;
            c.frame = m.frame(&c.frame, true);
            Surface::Torus(c)
        }
        Surface::Procedural(_) => {
            return Err(PatternError::Unsupported(
                "mirroring a surface built by construction",
            ));
        }
    })
}

/// A copy of `b` moved by `m`, every name passed through `rename`. A reflection turns the
/// solid inside out, so its loops are walked the other way.
pub(crate) fn moved(b: &Body, m: &Motion, rename: impl Fn(Name) -> Name) -> Result<Body, PatternError> {
    let mut out = Body::new();
    let mut vmap: Vec<Option<VertexId>> = vec![None; b.vertices.len()];
    let mut emap: Vec<Option<EdgeId>> = vec![None; b.edges.len()];
    let mut smap: Vec<Option<ShellId>> = vec![None; b.shells.len()];
    for f in b.face_ids() {
        let face = b.face(f);
        let shell = *smap[face.shell.index()].get_or_insert_with(|| out.add_shell());
        let nf = out.add_face(
            shell,
            surface(m, &face.surface)?,
            face.same_sense,
            rename(face.name),
        );
        for &l in &face.loops {
            let mut uses = Vec::with_capacity(b.lp(l).coedges().len());
            for c in b.lp(l).coedges() {
                let co = b.coedge(c);
                let e = match emap[co.edge.index()] {
                    Some(e) => e,
                    None => {
                        let edge = b.edge(co.edge);
                        let mut vert = |v: VertexId| {
                            *vmap[v.index()].get_or_insert_with(|| {
                                let x = b.vertex(v);
                                out.add_tolerant_vertex(m.point(x.point), x.tol, rename(x.name))
                            })
                        };
                        let (s, t) = (vert(edge.start), vert(edge.end));
                        let e = out.add_edge(curve(m, &edge.curve)?, edge.range, s, t, rename(edge.name));
                        emap[co.edge.index()] = Some(e);
                        e
                    }
                };
                uses.push((e, co.forward));
            }
            if m.mirrors() {
                uses = uses.into_iter().rev().map(|(e, f)| (e, !f)).collect();
            }
            out.add_loop(nf, &uses);
        }
    }
    Ok(out)
}

fn mirror2(p: Point2) -> Point2 {
    Point2::new(p.x, -p.y)
}

/// A tool's analytic solid moved by `m`.
pub(crate) fn moved_shape(s: &Shape, m: &Motion) -> Shape {
    match (s, m) {
        (Shape::Csg { first, rest }, _) => Shape::Csg {
            first: Box::new(moved_shape(first, m)),
            rest: rest.iter().map(|(op, s)| (*op, moved_shape(s, m))).collect(),
        },
        (Shape::Brep(b), _) => Shape::Brep(m.bbox(b)),
        // a mirrored coil: the frame's z turned back, so the profile's h and the rise with it
        (
            Shape::Coil {
                frame,
                segs,
                a,
                b,
                angle,
                r,
                h,
            },
            _,
        ) => {
            let flip = m.mirrors();
            let fy = |p: Point2| if flip { Point2::new(p.x, -p.y) } else { p };
            Shape::Coil {
                frame: m.frame(frame, !flip),
                segs: segs
                    .iter()
                    .map(|s| match *s {
                        Seg2::Line { a, b } => Seg2::Line { a: fy(a), b: fy(b) },
                        Seg2::Arc { c, r, start, sweep } if flip => Seg2::Arc {
                            c: fy(c),
                            r,
                            start: -start,
                            sweep: -sweep,
                        },
                        arc => arc,
                    })
                    .collect(),
                a: *a,
                b: if flip { -b } else { *b },
                angle: *angle,
                r: *r,
                h: if flip { (-h.1, -h.0) } else { *h },
            }
        }
        (Shape::Ball { center, radius }, _) => Shape::Ball {
            center: m.point(*center),
            radius: *radius,
        },
        (Shape::Prism { frame, segs, lo, hi }, Motion::Rigid(f)) => Shape::Prism {
            frame: f.compose(frame),
            segs: segs.clone(),
            lo: *lo,
            hi: *hi,
        },
        (Shape::Revolved { frame, .. }, Motion::Rigid(f)) => {
            let mut s = s.clone();
            if let Shape::Revolved { frame: to, .. } = &mut s {
                *to = f.compose(frame);
            }
            s
        }
        // the reflected frame negates local y: the profile's y with it
        (Shape::Prism { frame, segs, lo, hi }, Motion::Mirror(_)) => Shape::Prism {
            frame: m.frame(frame, true),
            segs: segs
                .iter()
                .map(|s| match *s {
                    Seg2::Line { a, b } => Seg2::Line {
                        a: mirror2(a),
                        b: mirror2(b),
                    },
                    Seg2::Arc { c, r, start, sweep } => Seg2::Arc {
                        c: mirror2(c),
                        r,
                        start: -start,
                        sweep: -sweep,
                    },
                })
                .collect(),
            lo: *lo,
            hi: *hi,
        },
        // a revolved point keeps its height and radius; only its angle runs backwards
        (Shape::Revolved { frame, angle, .. }, Motion::Mirror(_)) => {
            let f = m.frame(frame, true);
            let f = angle.map_or(f, |a| f.rotated_about_z(-a));
            let mut s = s.clone();
            if let Shape::Revolved { frame: to, .. } = &mut s {
                *to = f;
            }
            s
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{suite, torture};
    use forge_mesh::{Quality, tessellate};

    fn volume(b: &Body) -> f64 {
        let m = tessellate(b, &Quality::default()).expect("tessellates");
        assert!(m.open_edges().is_empty(), "mesh has open edges");
        forge_topo::mass::mass(b).expect("mass properties").volume
    }

    /// A moved or reflected part (planes, cylinders, cones, spheres, tori, intersection curves)
    /// is a valid solid of the same volume.
    #[test]
    fn copies_keep_volume() {
        let tilted = Frame::from_zx(
            Point3::new(30.0, -20.0, 7.0),
            Vec3::new(0.3, -0.5, 1.0).normalize().expect("axis"),
            Vec3::X,
        )
        .expect("frame");
        for part in [
            suite::mixed(),
            suite::shaft(),
            suite::fuzz(3, 40),
            torture::part(60),
        ] {
            let b = part.regenerate_nary(4).expect("regen");
            let v = volume(&b);
            for m in [
                Motion::Rigid(tilted),
                Motion::Mirror(tilted),
                Motion::Mirror(Frame::WORLD),
            ] {
                let c = moved(&b, &m, |n| n.with(1)).expect("moved");
                assert!(
                    forge_topo::check(&c).is_empty(),
                    "{m:?}: {:?}",
                    forge_topo::check(&c)
                );
                let vc = volume(&c);
                assert!((vc - v).abs() <= 1e-9 * v, "{m:?}: volume {vc} vs {v}");
            }
        }
    }

    /// A reflected part turn's analytic solid agrees with its reflected B-rep.
    #[test]
    fn mirrored_shapes_agree() {
        use forge_bool::{Index, Member, Where};
        use forge_ops::{Axis2, Profile};
        let turn = crate::Feature::Revolve {
            profile: Profile {
                plane: Frame::from_zx(
                    Point3::new(3.0, 1.0, 2.0),
                    Vec3::Y.normalize().expect("y"),
                    Vec3::X,
                )
                .expect("frame"),
                loops: vec![Profile::rectangle(Point2::new(4.0, -2.0), 3.0, 5.0, 0)],
            },
            axis: Axis2 {
                origin: Point2::ORIGIN,
                dir: forge_math::Vec2::Y,
            },
            extent: crate::Turn::Angle(2.0),
            op: crate::Combine::Add,
        };
        let (body, _, shape) = crate::tool(0, 1, &turn, true).expect("tool");
        let plane = Frame::from_zx(
            Point3::new(1.0, 2.0, 0.0),
            Vec3::new(1.0, 0.5, 0.2).normalize().expect("n"),
            Vec3::Z,
        )
        .expect("plane");
        let m = Motion::Mirror(plane);
        let (body, shape) = (moved(&body, &m, |n| n).expect("moved"), moved_shape(&shape, &m));
        let index = Index::new(&body);
        let b = index.bbox.grow(1.0);
        let mut r = forge_math::rng::Rng::new(5);
        let mut answered = 0;
        for _ in 0..2000 {
            let p = Point3::new(
                r.range(b.x.lo, b.x.hi),
                r.range(b.y.lo, b.y.hi),
                r.range(b.z.lo, b.z.hi),
            );
            let want = match shape.member(p, 1e-4) {
                Member::Near => continue,
                Member::In => Where::Inside,
                Member::Out => Where::Outside,
            };
            answered += 1;
            assert_eq!(index.classify(p), want, "at {p:?}");
        }
        assert!(answered > 1900);
    }

    /// Fitted copies fill the angle; incremental ones step by it.
    #[test]
    fn circular_spacing() {
        let place = |angle: f64, incremental: bool| {
            let layout = Layout::Circular {
                origin: Point3::ORIGIN,
                axis: Vec3::Z,
                count: 4,
                angle,
                incremental,
            };
            let last = instances(History::new(&[], None), 0, &layout)
                .expect("layout")
                .pop()
                .expect("copies")
                .1;
            let p = last.point(Point3::new(1.0, 0.0, 0.0));
            dmath::atan2(p.y, p.x)
        };
        assert!((place(0.9, false) - 0.9).abs() < 1e-12);
        assert!((place(0.3, true) - 0.9).abs() < 1e-12);
        assert!((place(dmath::TAU, false) - 1.5 * dmath::PI + dmath::TAU).abs() < 1e-12);
        assert!((place(-0.3, true) + 0.9).abs() < 1e-12);
    }
}
