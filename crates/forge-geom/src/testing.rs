//! Random geometry for property tests (here and in downstream crates).

use crate::surface::{Cone, Cylinder, Plane, Sphere, Surface, SurfaceGeom, Torus};
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, Point3, UnitVec3, Vec3, dmath};

pub fn rand_point(r: &mut Rng, size: f64) -> Point3 {
    Point3::new(r.range(-size, size), r.range(-size, size), r.range(-size, size))
}

pub fn rand_unit(r: &mut Rng) -> UnitVec3 {
    loop {
        let v = Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0));
        let l = v.length();
        if l > 0.1 && l <= 1.0 {
            return v.normalize().unwrap();
        }
    }
}

pub fn rand_frame(r: &mut Rng) -> Frame {
    let o = rand_point(r, 50.0);
    let z = rand_unit(r);
    Frame::from_zx(o, z, rand_unit(r).get()).unwrap_or_else(|| Frame::from_z(o, z))
}

pub fn rand_surfaces(r: &mut Rng) -> Vec<Surface> {
    let f = rand_frame(r);
    let major = r.range(5.0, 30.0);
    vec![
        Surface::Plane(Plane::new(f)),
        Surface::Cylinder(Cylinder {
            frame: f,
            radius: r.range(0.5, 30.0),
        }),
        Surface::Cone(Cone::new(f, r.range(0.5, 20.0), r.range(0.05, 1.4)).unwrap()),
        Surface::Sphere(Sphere {
            frame: f,
            radius: r.range(0.5, 30.0),
        }),
        Surface::Torus(Torus::new(f, major, r.range(0.5, major * 0.9)).unwrap()),
    ]
}

/// Parameters away from singular points (cone apex, sphere poles).
pub fn rand_uv(r: &mut Rng, s: &Surface) -> Point2 {
    let u = r.range(0.0, dmath::TAU);
    match s {
        Surface::Plane(_) => Point2::new(r.range(-50.0, 50.0), r.range(-50.0, 50.0)),
        Surface::Cylinder(_) => Point2::new(u, r.range(-50.0, 50.0)),
        Surface::Cone(c) => {
            let lo = c.apex_v() + 0.5 / c.tan_a();
            Point2::new(u, r.range(lo, lo + 40.0))
        }
        Surface::Sphere(_) => Point2::new(u, r.range(-1.4, 1.4)),
        Surface::Torus(_) => Point2::new(u, r.range(0.0, dmath::TAU)),
    }
}

/// A length well under every radius of curvature of the surface near rand_uv points.
pub fn curvature_safe(s: &Surface) -> f64 {
    match s {
        Surface::Plane(_) => 1.0,
        Surface::Cylinder(c) => c.radius,
        Surface::Cone(_) => 0.5,
        Surface::Sphere(sp) => sp.radius,
        Surface::Torus(t) => t.minor.min(t.major - t.minor),
    }
}

pub fn on_surface(s: &Surface, p: Point3, tol: f64) -> bool {
    s.distance(p) <= tol
}
