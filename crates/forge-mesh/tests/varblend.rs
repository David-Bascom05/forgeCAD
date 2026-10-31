//! A variable-radius blend face, bounded by its spring curves and end circles, meshes onto
//! the exact surface.

use forge_geom::varblend::{Guide, RadiusLaw, VarBlend};
use forge_geom::{Curve, CurveGeom, Cylinder, Line, Plane, Surface, SurfaceGeom};
use forge_math::{Frame, Interval, Point2, Point3, UnitVec3};
use forge_mesh::{Quality, tessellate};
use forge_topo::{Body, Name};
use std::sync::Arc;

fn sheet(blend: &Arc<VarBlend>) -> Body {
    let mut b = Body::new();
    let r = blend.range();
    let (s0, s1) = (blend.spring(0), blend.spring(1));
    let (c_lo, a_lo) = blend.section(r.lo);
    let (c_hi, a_hi) = blend.section(r.hi);
    let v = [
        b.add_vertex(s0.eval(r.lo), Name(1)),
        b.add_vertex(s0.eval(r.hi), Name(2)),
        b.add_vertex(s1.eval(r.hi), Name(3)),
        b.add_vertex(s1.eval(r.lo), Name(4)),
    ];
    let e = [
        b.add_edge(s0, r, v[0], v[1], Name(5)),
        b.add_edge(Curve::Circle(c_hi), Interval::new(0.0, a_hi), v[1], v[2], Name(6)),
        b.add_edge(s1, r, v[3], v[2], Name(7)),
        b.add_edge(Curve::Circle(c_lo), Interval::new(0.0, a_lo), v[0], v[3], Name(8)),
    ];
    let sh = b.add_shell();
    let f = b.add_face(sh, blend.surface(), true, Name(9));
    let mut uses = vec![(e[0], true), (e[1], true), (e[2], false), (e[3], false)];
    // counter-clockwise in (u, v) only when the contact with a is at v = 0
    if blend.v_on(0) != 0.0 {
        uses = uses.into_iter().rev().map(|(e, f)| (e, !f)).collect();
    }
    b.add_loop(f, &uses);
    b
}

fn check(blend: &Arc<VarBlend>) {
    let body = sheet(blend);
    let surface = blend.surface();
    let mesh = tessellate(&body, &Quality::default()).unwrap();
    assert!(mesh.triangles.len() > 20, "{}", mesh.triangles.len());
    for p in &mesh.positions {
        assert!(surface.distance(*p) < 1e-9, "{p:?}");
    }
    assert_eq!(mesh.degenerate_triangles(1e-12), 0);
    // area against a fine quadrature of |du x dv|
    let r = blend.range();
    let (n, m) = (400, 100);
    let mut area = 0.0;
    for i in 0..n {
        for j in 0..m {
            let uv = Point2::new(
                r.lo + r.width() * (i as f64 + 0.5) / n as f64,
                (j as f64 + 0.5) / m as f64,
            );
            let d = surface.deriv(uv);
            area += d.du.cross(d.dv).length() * r.width() / (n * m) as f64;
        }
    }
    let rel = (mesh.area() - area).abs() / area;
    assert!(rel < 5e-3, "area {} vs {area}", mesh.area());
    // facing the natural normal
    for (k, t) in mesh.triangles.iter().enumerate().step_by(7) {
        let c = Point3::ORIGIN
            + (mesh.positions[t[0] as usize].to_vec()
                + mesh.positions[t[1] as usize].to_vec()
                + mesh.positions[t[2] as usize].to_vec())
                / 3.0;
        let n = surface.normal(surface.project(c)).unwrap();
        assert!(mesh.triangle_normal(*t).dot(n.get()) > 0.0, "triangle {k}");
    }
}

#[test]
fn variable_blends_mesh() {
    let floor = Surface::Plane(Plane::new(Frame::WORLD));
    let wall = Surface::Plane(Plane::new(Frame::from_z(Point3::ORIGIN, UnitVec3::X)));
    let g = Guide::Line(Line::new(Point3::ORIGIN, UnitVec3::Y));
    let law = Arc::new(RadiusLaw::linear(-5.0, 1.0, 5.0, 3.0));
    check(&VarBlend::new(&floor, 1.0, &wall, 1.0, g, law, Interval::new(-5.0, 5.0)).unwrap());
    let cyl = Surface::Cylinder(Cylinder {
        frame: Frame::WORLD,
        radius: 10.0,
    });
    let circle = forge_geom::Circle {
        frame: Frame::WORLD,
        radius: 10.0,
    };
    let law = Arc::new(RadiusLaw::smooth(&[(0.0, 1.0), (1.5, 3.0), (3.0, 1.5)], None).unwrap());
    let range = Interval::new(0.0, 3.0);
    check(&VarBlend::new(&floor, 1.0, &cyl, 1.0, Guide::Circle(circle), law, range).unwrap());
}
