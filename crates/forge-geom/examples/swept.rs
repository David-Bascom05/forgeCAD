//! Instruction counts for screw (coil) and ruled (loft) surfaces:
//! `swept <screw|ruled|skinned> <build|deriv|project|implicit> [n]`, counted with
//! `perf stat -e instructions:u`. Subtract the `build` count (n = 0) for the per-call cost.

use forge_geom::ruled::{Rail, Ruled};
use forge_geom::screw::{Screw, Section};
use forge_geom::skinned::{Skinned, Spline};
use forge_geom::{Circle, Surface, SurfaceGeom};
use forge_math::rng::Rng;
use forge_math::{Frame, Interval, Point2, Point3, Vec3, dmath};
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let kind = args.get(1).map_or("screw", |s| s.as_str());
    let mode = args.get(2).map_or("deriv", |s| s.as_str());
    let n: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(100_000);
    // a coil of 6 turns: a square bar's top face and a round wire's outer half; a loft's twisted
    // side and an oblique cone's half
    let surfaces: Vec<Surface> = if kind == "screw" {
        let turns = Interval::new(0.0, 6.0 * dmath::TAU);
        let (b, a) = (5.0 / dmath::TAU, 0.02);
        let c = [
            Point2::new(10.0, -1.0),
            Point2::new(12.0, -1.0),
            Point2::new(12.0, 1.0),
            Point2::new(10.0, 1.0),
        ];
        let bar: Vec<Section> = (0..4)
            .map(|i| Section::Line {
                a: c[i],
                b: c[(i + 1) % 4],
            })
            .collect();
        let wire = [0.0, dmath::PI].map(|start| Section::Arc {
            c: Point2::new(20.0, 0.0),
            r: 1.5,
            start,
            sweep: dmath::PI,
        });
        let f = Frame::WORLD;
        vec![
            Screw::loop_faces(f, &bar, a, b, turns).unwrap()[2]
                .clone()
                .surface(),
            Screw::loop_faces(f, &wire, 0.0, b, turns).unwrap()[0]
                .clone()
                .surface(),
        ]
    } else if kind == "skinned" {
        // a smooth loft through three squares' sides and through three half circles
        let line = |z: f64, h: f64| Rail::Line {
            a: Point3::new(-h, -h, z),
            b: Point3::new(h, -h, z),
        };
        let half = |z: f64, r: f64| Rail::Arc {
            circle: Circle {
                frame: Frame::WORLD.with_origin(Point3::new(0.0, 0.0, z)),
                radius: r,
            },
            start: 0.0,
            sweep: dmath::PI,
        };
        let spline = Arc::new(Spline::new(vec![0.0, 15.0, 25.0], false, false).unwrap());
        vec![
            Skinned::new(
                vec![line(0.0, 4.0), line(15.0, 6.0), line(25.0, 2.5)],
                vec![],
                spline.clone(),
                0,
                vec![],
            )
            .unwrap()
            .surface(),
            Skinned::new(
                vec![half(0.0, 4.0), half(15.0, 6.0), half(25.0, 3.0)],
                vec![],
                spline,
                1,
                vec![],
            )
            .unwrap()
            .surface(),
        ]
    } else {
        let line = |a: [f64; 3], b: [f64; 3]| Rail::Line {
            a: Point3::new(a[0], a[1], a[2]),
            b: Point3::new(b[0], b[1], b[2]),
        };
        let arc = |o: Point3, r: f64| Rail::Arc {
            circle: Circle {
                frame: Frame::WORLD.with_origin(o),
                radius: r,
            },
            start: 0.0,
            sweep: dmath::PI,
        };
        vec![
            Ruled::new(
                line([-6.0, -6.0, 0.0], [6.0, -6.0, 0.0]),
                line([-3.0, -5.0, 10.0], [5.0, -1.0, 10.0]),
            )
            .unwrap()
            .surface(),
            Ruled::new(arc(Point3::ORIGIN, 8.0), arc(Point3::new(3.0, 0.0, 10.0), 4.0))
                .unwrap()
                .surface(),
        ]
    };
    let mut r = Rng::new(1);
    let mut acc = 0.0;
    for i in 0..n {
        let s = &surfaces[i % 2];
        let d = s.domain();
        let uv = Point2::new(r.range(d.u.lo, d.u.hi), r.range(d.v.lo, d.v.hi));
        let jitter = |r: &mut Rng, k: f64| Vec3::new(r.range(-k, k), r.range(-k, k), r.range(-k, k));
        match mode {
            "deriv" => acc += s.deriv(uv).du.x,
            "project" => {
                let p = s.eval(uv) + jitter(&mut r, 0.1);
                acc += s.project(p).x;
            }
            "implicit" => {
                let p = s.eval(uv) + jitter(&mut r, 1.0);
                acc += s.implicit(p);
            }
            _ => {}
        }
    }
    println!("{kind} {mode} {n} {acc}");
}
