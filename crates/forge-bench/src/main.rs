//! Kernel benchmarks: `cargo run --release -p forge-bench [filter]`.
//! Each case runs for about 0.3 s after a warm-up and reports time per call.

use forge_geom::intersect::{line_surface, plane_surface};
use forge_geom::testing::{rand_frame, rand_point, rand_surfaces, rand_unit, rand_uv};
use forge_geom::{Line, Plane, Surface, SurfaceGeom};
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, Vec2, dmath, poly};
use forge_mesh::{Quality, tessellate};
use forge_ops::{Axis2, Profile, extrude, revolve};
use forge_sketch::{Constraint, Sketch};
use std::hint::black_box;
use std::time::{Duration, Instant};

fn bench(filter: &str, name: &str, per_call: usize, mut f: impl FnMut()) {
    if !name.contains(filter) {
        return;
    }
    for _ in 0..3 {
        f();
    }
    let budget = Duration::from_millis(300);
    let start = Instant::now();
    let mut calls = 0u64;
    while start.elapsed() < budget {
        f();
        calls += 1;
    }
    let ns = start.elapsed().as_nanos() as f64 / (calls as f64 * per_call as f64);
    let (v, unit) = if ns >= 1e6 {
        (ns / 1e6, "ms")
    } else if ns >= 1e3 {
        (ns / 1e3, "us")
    } else {
        (ns, "ns")
    };
    println!("{name:<44} {v:>10.2} {unit}");
}

fn plate() -> Profile {
    Profile {
        plane: Frame::WORLD,
        loops: vec![
            Profile::rounded_rectangle(Point2::new(-40.0, -25.0), 80.0, 50.0, 8.0, 0),
            Profile::rounded_rectangle(Point2::new(-15.0, -4.0), 30.0, 8.0, 3.999, 100),
            Profile::circle(Point2::new(-30.0, 15.0), 3.25, 200),
            Profile::circle(Point2::new(30.0, 15.0), 3.25, 201),
        ],
    }
}

/// `n` rectangles, each fully constrained, then nudged off their solution.
fn rect_sketch(n: usize) -> Sketch {
    let mut s = Sketch::new(Frame::WORLD);
    for i in 0..n {
        let o = Point2::new(i as f64 * 30.0, 0.0);
        let p = [
            o,
            Point2::new(o.x + 20.0, o.y),
            Point2::new(o.x + 20.0, o.y + 10.0),
            Point2::new(o.x, o.y + 10.0),
        ];
        let pts: Vec<_> = p.iter().map(|&q| s.add_point(q)).collect();
        let lines: Vec<_> = (0..4)
            .map(|k| s.add_line(pts[k], pts[(k + 1) % 4]).unwrap())
            .collect();
        s.add_constraint(Constraint::Fixed(pts[0])).unwrap();
        s.add_constraint(Constraint::Horizontal(lines[0])).unwrap();
        s.add_constraint(Constraint::Horizontal(lines[2])).unwrap();
        s.add_constraint(Constraint::Vertical(lines[1])).unwrap();
        s.add_constraint(Constraint::Vertical(lines[3])).unwrap();
        s.add_constraint(Constraint::Length(lines[0], 21.0)).unwrap();
        s.add_constraint(Constraint::Length(lines[1], 11.0)).unwrap();
    }
    s
}

fn main() {
    let filter = std::env::args().nth(1).unwrap_or_default();
    let f = filter.as_str();
    let mut rng = Rng::new(1);
    let xs: Vec<f64> = (0..1024).map(|_| rng.range(-10.0, 10.0)).collect();

    bench(f, "math: dmath::sin_cos", xs.len(), || {
        for &x in &xs {
            black_box(dmath::sin_cos(black_box(x)));
        }
    });
    #[allow(clippy::disallowed_methods)]
    bench(f, "math: std sin_cos (platform, for reference)", xs.len(), || {
        for &x in &xs {
            black_box(black_box(x).sin_cos());
        }
    });
    bench(f, "math: dmath::atan2", xs.len(), || {
        for w in xs.windows(2) {
            black_box(dmath::atan2(black_box(w[0]), w[1]));
        }
    });
    let quartics: Vec<Vec<f64>> = (0..64)
        .map(|_| (0..5).map(|_| rng.range(-10.0, 10.0)).collect())
        .collect();
    bench(f, "math: quartic real roots", quartics.len(), || {
        for c in &quartics {
            black_box(poly::all_real_roots(black_box(c)));
        }
    });

    let surfaces: Vec<Surface> = (0..64).flat_map(|_| rand_surfaces(&mut rng)).collect();
    let uvs: Vec<Point2> = surfaces.iter().map(|s| rand_uv(&mut rng, s)).collect();
    let pts: Vec<_> = surfaces.iter().map(|_| rand_point(&mut rng, 40.0)).collect();
    bench(f, "geom: surface eval (mixed)", surfaces.len(), || {
        for (s, &uv) in surfaces.iter().zip(&uvs) {
            black_box(s.eval(black_box(uv)));
        }
    });
    bench(f, "geom: surface deriv (mixed)", surfaces.len(), || {
        for (s, &uv) in surfaces.iter().zip(&uvs) {
            black_box(s.deriv(black_box(uv)));
        }
    });
    bench(f, "geom: surface project (mixed)", surfaces.len(), || {
        for (s, &p) in surfaces.iter().zip(&pts) {
            black_box(s.project(black_box(p)));
        }
    });
    let lines: Vec<Line> = surfaces
        .iter()
        .map(|_| Line::new(rand_point(&mut rng, 40.0), rand_unit(&mut rng)))
        .collect();
    bench(f, "geom: line x surface (mixed)", surfaces.len(), || {
        for (s, l) in surfaces.iter().zip(&lines) {
            black_box(line_surface(black_box(l), s));
        }
    });
    let tori: Vec<(Surface, Line)> = surfaces
        .iter()
        .zip(&lines)
        .filter(|(s, _)| matches!(s, Surface::Torus(_)))
        .map(|(s, l)| (*s, *l))
        .collect();
    bench(f, "geom: line x torus", tori.len(), || {
        for (s, l) in &tori {
            black_box(line_surface(black_box(l), s));
        }
    });
    let planes: Vec<Plane> = surfaces
        .iter()
        .map(|_| Plane::new(rand_frame(&mut rng)))
        .collect();
    bench(f, "geom: plane x surface (mixed)", surfaces.len(), || {
        for (s, p) in surfaces.iter().zip(&planes) {
            black_box(plane_surface(black_box(p), s));
        }
    });

    let p = plate();
    bench(f, "ops: extrude plate (20 faces)", 1, || {
        black_box(extrude(black_box(&p), 6.0, 1).unwrap());
    });
    let body = extrude(&p, 6.0, 1).unwrap();
    bench(f, "topo: check plate", 1, || {
        black_box(forge_topo::check(black_box(&body)));
    });
    bench(f, "mesh: tessellate plate (chord 0.01)", 1, || {
        black_box(tessellate(black_box(&body), &Quality::default()).unwrap());
    });
    let fine = Quality {
        chord: 0.001,
        ..Quality::default()
    };
    bench(f, "mesh: tessellate plate (chord 0.001)", 1, || {
        black_box(tessellate(black_box(&body), &fine).unwrap());
    });
    let ring = Profile {
        plane: Frame::WORLD,
        loops: vec![Profile::rounded_rectangle(
            Point2::new(10.0, 0.0),
            8.0,
            12.0,
            2.0,
            0,
        )],
    };
    let y = Axis2 {
        origin: Point2::ORIGIN,
        dir: Vec2::Y,
    };
    bench(f, "ops: revolve rounded ring", 1, || {
        black_box(revolve(black_box(&ring), y, dmath::TAU, 1).unwrap());
    });
    let rbody = revolve(&ring, y, dmath::TAU, 1).unwrap();
    bench(f, "mesh: tessellate revolved ring (chord 0.01)", 1, || {
        black_box(tessellate(black_box(&rbody), &Quality::default()).unwrap());
    });

    for n in [1, 10, 50] {
        let base = rect_sketch(n);
        let mut jitter = Rng::new(9);
        let nudged: Vec<f64> = base
            .params()
            .iter()
            .map(|&v| v + jitter.range(-0.5, 0.5))
            .collect();
        bench(
            f,
            &format!("sketch: solve {n} rectangle(s) from nudged"),
            1,
            || {
                let mut s = base.clone();
                s.set_params(&nudged).unwrap();
                black_box(s.solve());
            },
        );
    }
}
