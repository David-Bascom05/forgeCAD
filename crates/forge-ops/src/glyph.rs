//! The solid of one region of laid-out text: its outline (lines and quadratic arcs) raised
//! between two levels, either flat on a plane (between two heights along its normal) or
//! wrapped round a cylinder (between two radii). Flat, a quadratic arc is an exact parabola and
//! its wall a parabolic cylinder; wrapped, every edge is its flat curve wrapped round the axis
//! and every wall the ruled surface between it at the two radii, exact by construction (plain
//! circles, lines and planes where a line runs round or along the cylinder).
//!
//! Names: the caps are "start" (the lower level) and "end"; segment k of loop l has the key
//! (l, k): its wall is "side" key, its edge on level j "section" j key, its first vertex there
//! "vertex" j key, and that vertex's rail between the levels "rail" key.

use forge_geom::ruled::{Rail, Ruled};
use forge_geom::wrap::{Flat, Wrap};
use forge_geom::{Circle, Curve, CurveGeom, Cylinder, Line, Parabola, Plane, Surface, SurfaceGeom};
use forge_math::{Frame, Interval, Point2, Point3, Vec2, Vec3, dmath};
use forge_topo::{Body, EdgeId, Name, VertexId};

/// A piece of an outline in the layout's (s, h): along the text and up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GlyphSeg {
    Line(Point2, Point2),
    /// From the first point to the last, pulled towards the control point between.
    Quad(Point2, Point2, Point2),
}

/// Where the layout lies: on a plane (s along x, h along y, raised along z), or round a cylinder
/// about `axis`'s z (s the length round it at `radius` from its x, h along z, raised outward).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Place {
    Flat(Frame),
    Wrapped { axis: Frame, radius: f64 },
}

#[derive(Clone, Debug, PartialEq)]
pub enum GlyphError {
    /// The levels are not increasing (wrapped, or not both positive).
    BadLevels,
    /// A loop of fewer than two pieces, or a wall that folds.
    Degenerate,
    /// Wrapped, the text runs a whole turn or more round the cylinder.
    TooLong,
}

/// A segment in (s, h) as a line or an exact parabola over its parameter range.
pub fn flat(s: &GlyphSeg) -> Flat {
    match *s {
        GlyphSeg::Line(a, b) => Flat::Line { a, b },
        GlyphSeg::Quad(p0, c, p2) => {
            // P(t) = p0 + u t + w t^2
            let u = (c - p0) * 2.0;
            let w = (p0 - c) + (p2 - c);
            let wl = w.length();
            if !wl.is_finite() || wl <= 1e-12 * u.length() || u.cross(w).abs() <= 1e-9 * u.length() * wl {
                return Flat::Line { a: p0, b: p2 };
            }
            // the axis along w; the vertex where the speed is square to it
            let x = w / wl;
            let y0 = Vec2::new(-x.y, x.x);
            let (ux, uy) = (u.dot(x), u.dot(y0));
            let (y, uy) = if uy < 0.0 { (-y0, -uy) } else { (y0, uy) };
            let tv = -ux / (2.0 * wl);
            let o = p0 + u * tv + w * (tv * tv);
            let z = x.cross(y).signum();
            let Some(frame) = Frame::from_axes(
                Point3::new(o.x, o.y, 0.0),
                Vec3::new(x.x, x.y, 0.0),
                Vec3::new(y.x, y.y, 0.0),
                Vec3::new(0.0, 0.0, z),
            ) else {
                return Flat::Line { a: p0, b: p2 };
            };
            Flat::Parabola {
                curve: Parabola {
                    frame,
                    focal: uy * uy / (4.0 * wl),
                },
                range: Interval::new(-uy * tv, uy * (1.0 - tv)),
            }
        }
    }
}

/// One level of the solid: `at` is the height (flat) or radius (wrapped).
struct Level {
    place: Place,
    at: f64,
}

impl Level {
    fn point(&self, q: Point2) -> Point3 {
        match self.place {
            Place::Flat(f) => f.point_to_world(Point3::new(q.x, q.y, self.at)),
            Place::Wrapped { axis, radius } => {
                let (sn, cs) = dmath::sin_cos(q.x / radius);
                axis.point_to_world(Point3::new(self.at * cs, self.at * sn, q.y))
            }
        }
    }

    fn lifted(f: &Frame, at: f64) -> Frame {
        f.with_origin(f.point_to_world(Point3::new(0.0, 0.0, at)))
    }

    fn cap(&self) -> Surface {
        match self.place {
            Place::Flat(f) => Surface::Plane(Plane::new(Level::lifted(&f, self.at))),
            Place::Wrapped { axis, .. } => Surface::Cylinder(Cylinder {
                frame: axis,
                radius: self.at,
            }),
        }
    }

    /// The segment's curve here, its range, and whether the outline runs with its parameter.
    /// None for a piece too short to draw.
    fn curve(&self, f: &Flat) -> Option<(Curve, Interval, bool)> {
        let (a, b) = (f.at(0.0).0, f.at(1.0).0);
        let line = || {
            let (pa, pb) = (self.point(a), self.point(b));
            Some((
                Curve::Line(Line::through(pa, pb)?),
                Interval::new(0.0, pa.distance(pb)),
                true,
            ))
        };
        Some(match (self.place, *f) {
            (Place::Flat(_), Flat::Line { .. }) => line()?,
            (Place::Flat(fr), Flat::Parabola { curve, range }) => (
                Curve::Parabola(Parabola {
                    frame: Level::lifted(&fr, self.at).compose(&curve.frame),
                    focal: curve.focal,
                }),
                range,
                true,
            ),
            // along the axis: a line; round it: a circle
            (Place::Wrapped { .. }, Flat::Line { .. }) if a.x == b.x => line()?,
            (Place::Wrapped { axis, radius }, Flat::Line { .. }) if a.y == b.y => {
                let (t0, t1) = (a.x / radius, b.x / radius);
                (
                    Curve::Circle(Circle {
                        frame: axis.with_origin(axis.point_to_world(Point3::new(0.0, 0.0, a.y))),
                        radius: self.at,
                    }),
                    Interval::new(t0.min(t1), t0.max(t1)),
                    t1 > t0,
                )
            }
            (Place::Wrapped { axis, radius }, _) => {
                (self.wrap(axis, radius, f).curve(), Interval::new(0.0, 1.0), true)
            }
        })
    }

    fn wrap(&self, axis: Frame, unroll: f64, f: &Flat) -> Wrap {
        Wrap {
            axis,
            radius: self.at,
            unroll,
            flat: *f,
        }
    }
}

/// The wall of segment f between the two levels; None where it folds.
fn wall(lo: &Level, hi: &Level, f: &Flat) -> Option<Surface> {
    let (a, b) = (f.at(0.0).0, f.at(1.0).0);
    match (lo.place, *f) {
        (Place::Flat(fr), Flat::Line { .. }) => {
            let along = fr.vec_to_world(Vec3::new(b.x - a.x, b.y - a.y, 0.0));
            let out = along.cross(fr.z().get());
            Some(Surface::Plane(Plane::new(Frame::from_zx(
                lo.point(a),
                out.normalize()?,
                along,
            )?)))
        }
        (Place::Flat(_), Flat::Parabola { range, .. }) => {
            let rail = |l: &Level| match l.curve(f)?.0 {
                Curve::Parabola(p) => Some(Rail::Parabola { curve: p, range }),
                _ => None,
            };
            Ruled::new(rail(lo)?, rail(hi)?).map(Ruled::surface)
        }
        (Place::Wrapped { axis, radius }, Flat::Line { .. }) if a.x == b.x => {
            // along the axis: the plane through it
            let (sn, cs) = dmath::sin_cos(a.x / radius);
            let round = axis.vec_to_world(Vec3::new(-sn, cs, 0.0));
            Some(Surface::Plane(Plane::new(Frame::from_zx(
                lo.point(a),
                round.normalize()?,
                axis.z().get(),
            )?)))
        }
        (Place::Wrapped { axis, .. }, Flat::Line { .. }) if a.y == b.y => {
            let o = axis.point_to_world(Point3::new(0.0, 0.0, a.y));
            Some(Surface::Plane(Plane::new(axis.with_origin(o))))
        }
        (Place::Wrapped { axis, radius }, _) => Ruled::new(
            Rail::Wrap(lo.wrap(axis, radius, f)),
            Rail::Wrap(hi.wrap(axis, radius, f)),
        )
        .map(Ruled::surface),
    }
}

/// The solid of `loops` (the outline first, counter-clockwise in (s, h), then its holes,
/// clockwise) between the levels `lo` < `hi`, named from `root`.
pub fn glyph(
    loops: &[Vec<GlyphSeg>],
    place: Place,
    lo: f64,
    hi: f64,
    root: Name,
) -> Result<Body, GlyphError> {
    if !(lo < hi && lo.is_finite() && hi.is_finite()) {
        return Err(GlyphError::BadLevels);
    }
    let finite = |q: &Point2| q.x.is_finite() && q.y.is_finite();
    let ok = |s: &GlyphSeg| match s {
        GlyphSeg::Line(a, b) => finite(a) && finite(b) && a != b,
        GlyphSeg::Quad(a, c, b) => finite(a) && finite(c) && finite(b) && a != b,
    };
    if loops.iter().any(|l| l.len() < 2 || !l.iter().all(ok)) {
        return Err(GlyphError::Degenerate);
    }
    if let Place::Wrapped { radius, .. } = place {
        if !(lo > 0.0 && radius > 0.0) {
            return Err(GlyphError::BadLevels);
        }
        let (mut s0, mut s1) = (f64::INFINITY, f64::NEG_INFINITY);
        for s in loops.iter().flatten() {
            let (sx, _) = flat(s).bbox(Interval::new(0.0, 1.0));
            (s0, s1) = (s0.min(sx.lo), s1.max(sx.hi));
        }
        if s1 - s0 >= dmath::TAU * radius {
            return Err(GlyphError::TooLong);
        }
    }
    let levels = [Level { place, at: lo }, Level { place, at: hi }];
    let flats: Vec<Vec<Flat>> = loops.iter().map(|l| l.iter().map(flat).collect()).collect();
    let key = |l: usize, k: usize| Name::NONE.with(l as u64).with(k as u64);
    let mut body = Body::new();
    let shell = body.add_shell();
    // vertices[j][l][k]: where segment k of loop l starts, on level j
    let verts: Vec<Vec<Vec<VertexId>>> = (0..2)
        .map(|j| {
            flats
                .iter()
                .enumerate()
                .map(|(l, fl)| {
                    fl.iter()
                        .enumerate()
                        .map(|(k, f)| {
                            let name = root.with_str("vertex").with(j as u64).with_name(key(l, k));
                            body.add_vertex(levels[j].point(f.at(0.0).0), name)
                        })
                        .collect()
                })
                .collect()
        })
        .collect();
    let mut edges: Vec<Vec<Vec<(EdgeId, bool)>>> = Vec::new();
    for (j, level) in levels.iter().enumerate() {
        let mut per = Vec::new();
        for (l, fl) in flats.iter().enumerate() {
            let n = fl.len();
            let mut row = Vec::new();
            for (k, f) in fl.iter().enumerate() {
                let (curve, range, fw) = level.curve(f).ok_or(GlyphError::Degenerate)?;
                let (a, b) = (verts[j][l][k], verts[j][l][(k + 1) % n]);
                let (start, end) = if fw { (a, b) } else { (b, a) };
                let name = root.with_str("section").with(j as u64).with_name(key(l, k));
                row.push((body.add_edge(curve, range, start, end, name), fw));
            }
            per.push(row);
        }
        edges.push(per);
    }
    for (l, fl) in flats.iter().enumerate() {
        let n = fl.len();
        let rails: Vec<EdgeId> = (0..n)
            .map(|k| -> Result<EdgeId, GlyphError> {
                let (p0, p1) = (
                    body.vertex(verts[0][l][k]).point,
                    body.vertex(verts[1][l][k]).point,
                );
                let line = Line::through(p0, p1).ok_or(GlyphError::Degenerate)?;
                let name = root.with_str("rail").with_name(key(l, k));
                Ok(body.add_edge(
                    Curve::Line(line),
                    Interval::new(0.0, p0.distance(p1)),
                    verts[0][l][k],
                    verts[1][l][k],
                    name,
                ))
            })
            .collect::<Result<_, _>>()?;
        for (k, f) in fl.iter().enumerate() {
            let surface = wall(&levels[0], &levels[1], f).ok_or(GlyphError::Degenerate)?;
            // outward: along the outline, crossed with the way up
            let (e, fw) = edges[0][l][k];
            let ed = body.edge(e);
            let mid = ed.curve.deriv(ed.range.mid());
            let along = if fw { mid.d1 } else { -mid.d1 };
            let up = body.vertex(verts[1][l][k]).point - body.vertex(verts[0][l][k]).point;
            let same = surface
                .normal(surface.project(mid.p))
                .is_none_or(|nat| nat.dot(along.cross(up)) > 0.0);
            let face = body.add_face(shell, surface, same, root.with_str("side").with_name(key(l, k)));
            body.add_loop(
                face,
                &[
                    (e, fw),
                    (rails[(k + 1) % n], true),
                    (edges[1][l][k].0, !edges[1][l][k].1),
                    (rails[k], false),
                ],
            );
        }
    }
    for (j, same, name) in [(0, false, "start"), (1, true, "end")] {
        let face = body.add_face(shell, levels[j].cap(), same, root.with_str(name));
        for row in &edges[j] {
            let uses: Vec<(EdgeId, bool)> = if same {
                row.clone()
            } else {
                row.iter().rev().map(|&(e, fw)| (e, !fw)).collect()
            };
            body.add_loop(face, &uses);
        }
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_topo::check;

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    /// A rounded square of four quadratic arcs, counter-clockwise, round a square hole.
    fn ring() -> Vec<Vec<GlyphSeg>> {
        let q = |a: Point2, c: Point2, b: Point2| GlyphSeg::Quad(a, c, b);
        let outer = vec![
            q(p(3.0, 0.0), p(6.0, 0.0), p(6.0, 3.5)),
            q(p(6.0, 3.5), p(6.0, 7.0), p(3.0, 7.0)),
            q(p(3.0, 7.0), p(0.0, 7.0), p(0.0, 3.5)),
            q(p(0.0, 3.5), p(0.0, 0.0), p(3.0, 0.0)),
        ];
        let c = [p(2.0, 2.0), p(2.0, 5.0), p(4.0, 5.0), p(4.0, 2.0)];
        let hole = (0..4).map(|k| GlyphSeg::Line(c[k], c[(k + 1) % 4])).collect();
        vec![outer, hole]
    }

    fn valid(b: &Body) {
        let problems = check(b);
        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn a_quad_is_its_parabola() {
        let s = GlyphSeg::Quad(p(1.0, 2.0), p(4.0, 7.0), p(9.0, 3.0));
        let f = flat(&s);
        assert!(matches!(f, Flat::Parabola { .. }));
        for i in 0..=8 {
            let t = i as f64 / 8.0;
            let v = |x: f64, y: f64| Vec2::new(x, y);
            let want = Point2::ORIGIN
                + v(1.0, 2.0) * ((1.0 - t) * (1.0 - t))
                + v(4.0, 7.0) * (2.0 * t * (1.0 - t))
                + v(9.0, 3.0) * (t * t);
            assert!(f.at(t).0.distance(want) < 1e-12, "{t}");
        }
        assert!(matches!(
            flat(&GlyphSeg::Quad(p(0.0, 0.0), p(1.0, 1.0), p(2.0, 2.0))),
            Flat::Line { .. }
        ));
    }

    #[test]
    fn flat_and_wrapped_glyphs_are_valid() {
        let b = glyph(&ring(), Place::Flat(Frame::WORLD), 0.0, 1.5, Name::feature(1)).expect("flat");
        valid(&b);
        let axis = Frame::WORLD;
        let b = glyph(
            &ring(),
            Place::Wrapped { axis, radius: 10.0 },
            9.0,
            11.0,
            Name::feature(1),
        )
        .expect("wrapped");
        valid(&b);
        assert_eq!(
            glyph(
                &ring(),
                Place::Wrapped { axis, radius: 0.9 },
                0.5,
                2.0,
                Name::feature(1)
            )
            .unwrap_err(),
            GlyphError::TooLong
        );
    }
}
