//! Instruction counts for variable blends: `varblend <build|deriv|project|implicit> [n]`,
//! counted with `perf stat -e instructions:u`. Each mode builds the same two blends, so
//! subtract the `build` count (n = 0) to get the per-call cost.

use forge_geom::varblend::{Guide, RadiusLaw, VarBlend};
use forge_geom::{Circle, Cylinder, Line, Plane, Surface, SurfaceGeom};
use forge_math::rng::Rng;
use forge_math::{Frame, Interval, Point2, Point3, UnitVec3, Vec3};
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map_or("deriv", |s| s.as_str());
    let n: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(100_000);
    let floor = Surface::Plane(Plane::new(Frame::WORLD));
    let wall = Surface::Plane(Plane::new(Frame::from_z(Point3::ORIGIN, UnitVec3::X)));
    let cyl = Surface::Cylinder(Cylinder {
        frame: Frame::WORLD,
        radius: 10.0,
    });
    let line = Guide::Line(Line::new(Point3::ORIGIN, UnitVec3::Y));
    let circle = Guide::Circle(Circle {
        frame: Frame::WORLD,
        radius: 10.0,
    });
    let law = Arc::new(RadiusLaw::smooth(&[(-5.0, 1.0), (0.0, 2.5), (5.0, 1.5)], None).unwrap());
    let blends = [
        VarBlend::new(&floor, 1.0, &wall, 1.0, line, law, Interval::new(-5.0, 5.0)).unwrap(),
        VarBlend::new(
            &floor,
            1.0,
            &cyl,
            1.0,
            circle,
            Arc::new(RadiusLaw::linear(0.0, 1.0, 3.0, 3.0)),
            Interval::new(0.0, 3.0),
        )
        .unwrap(),
    ];
    let surfaces: Vec<Surface> = blends.iter().map(|b| b.surface()).collect();
    let mut r = Rng::new(1);
    let mut acc = 0.0;
    for i in 0..n {
        let (b, s) = (&blends[i % 2], &surfaces[i % 2]);
        let range = b.range();
        let uv = Point2::new(r.range(range.lo, range.hi), r.unit());
        match mode {
            "deriv" => acc += s.deriv(uv).du.x,
            "project" => {
                let p = s.eval(uv) + Vec3::new(r.range(-0.1, 0.1), r.range(-0.1, 0.1), r.range(-0.1, 0.1));
                acc += s.project(p).x;
            }
            "implicit" => {
                let p = s.eval(uv) + Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0));
                acc += s.implicit(p);
            }
            _ => {}
        }
    }
    println!("{mode} {n} {acc}");
}
