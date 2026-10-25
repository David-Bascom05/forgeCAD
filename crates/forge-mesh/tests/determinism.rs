//! One hash over a broad sweep of kernel results. The constant is the same on
//! every target; CI runs this natively and as wasm32, so a mismatch means some
//! operation rounds differently somewhere.

use forge_geom::intersect::{LineSurface, SurfaceSurface, line_surface, plane_surface};
use forge_geom::testing::{rand_frame, rand_point, rand_surfaces, rand_unit, rand_uv};
use forge_geom::{Curve, CurveGeom, Line, Plane, SurfaceGeom};
use forge_math::hash::BitHasher;
use forge_math::rng::Rng;
use forge_math::{Point2, dmath, poly};
use forge_mesh::{Quality, tessellate};
use forge_ops::{Profile, extrude};

const EXPECTED: u64 = 0xd5d418f889fa981a;

fn sweep() -> u64 {
    let mut h = BitHasher::new();
    let mut r = Rng::new(0xF0_76E);
    for _ in 0..2000 {
        let x = r.range(-1e3, 1e3);
        let (s, c) = dmath::sin_cos(x);
        h.f64(s);
        h.f64(c);
        h.f64(dmath::atan2(x, r.range(-10.0, 10.0)));
        h.f64(dmath::acos(r.range(-1.0, 1.0)));
        h.f64(dmath::cbrt(x));
    }
    for _ in 0..200 {
        let c: Vec<f64> = (0..5).map(|_| r.range(-10.0, 10.0)).collect();
        for root in poly::all_real_roots(&c) {
            h.f64(root.x);
        }
    }
    for _ in 0..200 {
        for s in rand_surfaces(&mut r) {
            let uv = rand_uv(&mut r, &s);
            let d = s.deriv(uv);
            h.p3(d.p);
            h.v3(d.du);
            h.v3(d.dvv);
            h.p2(s.project(rand_point(&mut r, 40.0)));
            let line = Line::new(rand_point(&mut r, 40.0), rand_unit(&mut r));
            if let LineSurface::Hits(hits) = line_surface(&line, &s) {
                for hit in hits {
                    h.f64(hit.t);
                }
            }
            let plane = Plane::new(rand_frame(&mut r));
            if let SurfaceSurface::Curves(cs) = plane_surface(&plane, &s) {
                for (c, _) in cs {
                    h.p3(c.eval(0.3));
                    if let Curve::Ellipse(e) = c {
                        h.f64(e.major);
                    }
                }
            }
        }
    }
    let p = Profile {
        plane: rand_frame(&mut r),
        loops: vec![
            Profile::rounded_rectangle(Point2::ORIGIN, 40.0, 25.0, 6.0, 0),
            Profile::circle(Point2::new(20.0, 12.5), 5.0, 20),
        ],
    };
    let body = extrude(&p, 12.0, 1).unwrap();
    let mesh = tessellate(&body, &Quality::default()).unwrap();
    for q in &mesh.positions {
        h.p3(*q);
    }
    for t in &mesh.triangles {
        for &i in t {
            h.u64(i as u64);
        }
    }
    h.finish()
}

#[test]
fn same_bits_on_every_target() {
    let got = sweep();
    assert_eq!(got, EXPECTED, "kernel results changed: {got:#018x}");
}
