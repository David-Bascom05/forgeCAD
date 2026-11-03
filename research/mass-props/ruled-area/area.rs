//! The area and its reported error of four bodies with ruled walls, against the references of
//! reference.py: copy to kernel/forge-model/examples/ and run (see README.md).
use forge_geom::ruled::{Rail, Ruled};
use forge_geom::wrap::{Flat, Wrap};
use forge_geom::{Curve, Cylinder, Line, Surface, SurfaceGeom};
use forge_math::{Frame, Interval, Point2, Point3, Vec3, dmath};
use forge_ops::{GlyphSeg, LoftGuides, LoftKind, Place, Profile, ProfileLoop, Segment, glyph, loft};
use forge_topo::mass::mass;
use forge_topo::{Body, Name};

/// The disc of radius 3 about (2, 1) in (s, h), unrolled at 10.8, between radii 10 and 12.
fn plug() -> Body {
    let (axis, unroll, levels) = (Frame::WORLD, 10.8, [10.0, 12.0]);
    let flats = [0.0, dmath::PI].map(|start| Flat::Arc {
        center: Point2::new(2.0, 1.0),
        radius: 3.0,
        start,
        sweep: dmath::PI,
    });
    let wrap = |radius: f64, flat: Flat| Wrap {
        axis,
        radius,
        unroll,
        flat,
    };
    let name = Name::feature(1);
    let mut body = Body::new();
    let shell = body.add_shell();
    let verts = [0, 1].map(|j| {
        [0, 1].map(|k| body.add_vertex(wrap(levels[j], flats[k]).at(0.0).0, name.with(j as u64).with(k as u64)))
    });
    let edges = [0, 1].map(|j| {
        [0, 1].map(|k| {
            let c = wrap(levels[j], flats[k]).curve();
            body.add_edge(c, Interval::new(0.0, 1.0), verts[j][k], verts[j][1 - k], name.with(10 + j as u64).with(k as u64))
        })
    });
    let rails = [0, 1].map(|k| {
        let (p0, p1) = (body.vertex(verts[0][k]).point, body.vertex(verts[1][k]).point);
        let line = Curve::Line(Line::through(p0, p1).expect("a rail"));
        body.add_edge(line, Interval::new(0.0, p0.distance(p1)), verts[0][k], verts[1][k], name.with(20 + k as u64))
    });
    for k in 0..2 {
        let s = Ruled::new(Rail::Wrap(wrap(levels[0], flats[k])), Rail::Wrap(wrap(levels[1], flats[k])))
            .expect("a wall")
            .surface();
        let (p, d1, _) = wrap(levels[0], flats[k]).at(0.5);
        let up = body.vertex(verts[1][k]).point - body.vertex(verts[0][k]).point;
        let same = s.normal(s.project(p)).is_none_or(|n| n.dot(d1.cross(up)) > 0.0);
        let face = body.add_face(shell, s, same, name.with(30 + k as u64));
        body.add_loop(face, &[(edges[0][k], true), (rails[1 - k], true), (edges[1][k], false), (rails[k], false)]);
    }
    for (j, same) in [(0, false), (1, true)] {
        let cap = Surface::Cylinder(Cylinder { frame: axis, radius: levels[j] });
        let face = body.add_face(shell, cap, same, name.with(40 + j as u64));
        let uses = if same {
            [(edges[j][0], true), (edges[j][1], true)]
        } else {
            [(edges[j][1], false), (edges[j][0], false)]
        };
        body.add_loop(face, &uses);
    }
    body
}

fn turned(z: f64, turn: f64, l: ProfileLoop) -> Profile {
    let (sn, cs) = dmath::sin_cos(turn);
    Profile {
        plane: Frame::from_axes(Point3::new(0.0, 0.0, z), Vec3::new(cs, sn, 0.0), Vec3::new(-sn, cs, 0.0), Vec3::Z)
            .unwrap(),
        loops: vec![l],
    }
}

/// kernel/forge-model/tests/ruled_area.rs's hyperboloid: rings of radius 30 and 24, 40 apart, turned 0.5.
fn hyperboloid() -> Body {
    let ring = |r: f64| {
        let half = |start: f64, id: u64| (Segment::Arc { center: Point2::ORIGIN, radius: r, start, sweep: dmath::PI }, id);
        ProfileLoop::new(vec![half(0.0, 0), half(dmath::PI, 1)])
    };
    loft(&[turned(0.0, 0.0, ring(30.0)), turned(40.0, 0.5, ring(24.0))], LoftKind::Ruled, &LoftGuides::None, 1).unwrap()
}

/// A 20 square to a 12 square 15 above, turned 0.6: four hyperbolic paraboloids.
fn twisted_square() -> Body {
    let sq = |w: f64| Profile::rectangle(Point2::new(-w / 2.0, -w / 2.0), w, w, 0);
    loft(&[turned(0.0, 0.0, sq(20.0)), turned(15.0, 0.6, sq(12.0))], LoftKind::Ruled, &LoftGuides::None, 1).unwrap()
}

/// forge-ops glyph's ring (a rounded square of four quadratic arcs round a square hole) wrapped at
/// radius 10, between radii 9 and 11.
fn glyph_ring() -> Body {
    let p = Point2::new;
    let q = |a: Point2, c: Point2, b: Point2| GlyphSeg::Quad(a, c, b);
    let outer = vec![
        q(p(3.0, 0.0), p(6.0, 0.0), p(6.0, 3.5)),
        q(p(6.0, 3.5), p(6.0, 7.0), p(3.0, 7.0)),
        q(p(3.0, 7.0), p(0.0, 7.0), p(0.0, 3.5)),
        q(p(0.0, 3.5), p(0.0, 0.0), p(3.0, 0.0)),
    ];
    let c = [p(2.0, 2.0), p(2.0, 5.0), p(4.0, 5.0), p(4.0, 2.0)];
    let hole = (0..4).map(|k| GlyphSeg::Line(c[k], c[(k + 1) % 4])).collect();
    glyph(&[outer, hole], Place::Wrapped { axis: Frame::WORLD, radius: 10.0 }, 9.0, 11.0, Name::feature(1)).unwrap()
}

fn main() {
    for (name, b) in [("plug", plug()), ("hyperboloid", hyperboloid()), ("twisted-square", twisted_square()), ("glyph-ring", glyph_ring())] {
        let m = mass(&b).expect("mass");
        println!("{name} {:.17e} {:.3e}", m.area, m.err[1]);
    }
}
