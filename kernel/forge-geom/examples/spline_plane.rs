//! Plane against an extruded spline: the closed form against the general tracer, on random planes.
//! Provenance: research/spline-loops/README.md.
use forge_geom::intersect::SurfaceSurface;
use forge_geom::spline::{Extruded, Spline, meet::meet_spline};
use forge_geom::{Curve, CurveGeom, Plane, Surface, intcurve};
use forge_math::rng::Rng;
use forge_math::{Frame, Interval, Point3, Vec3};
use std::sync::Arc;

fn worst(c: &Curve, range: Interval, a: &Surface, b: &Surface) -> f64 {
    let mut w = 0.0f64;
    for i in 0..=64 {
        let p = c.eval(range.lo + range.width() * i as f64 / 64.0);
        w = w.max(a.implicit(p).abs()).max(b.implicit(p).abs());
    }
    w
}

fn main() {
    let mut r = Rng::new(1);
    let (mut t_closed, mut t_trace) = (0.0, 0.0);
    let (mut e_closed, mut e_trace) = (0.0f64, 0.0f64);
    let (mut n, mut mismatch, mut none) = (0, 0, 0);
    for k in 0..400 {
        let d = [2, 3, 5][k % 3];
        let m = d + 1 + r.below(8);
        let ctrl: Vec<Point3> = (0..m)
            .map(|i| Point3::new(i as f64 * 4.0, r.range(-6.0, 6.0), 0.0))
            .collect();
        let knots = forge_geom::spline::uniform_knots(d, m);
        let c = Spline::from_nurbs(d, &knots, &ctrl, None).unwrap();
        let Some(w) = Extruded::new(Frame::WORLD, Arc::new(c), Interval::new(0.0, 20.0)) else {
            continue;
        };
        let s = w.surface();
        let z = Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(0.2, 1.0));
        let o = Point3::new(
            r.range(0.0, 4.0 * m as f64),
            r.range(-3.0, 3.0),
            r.range(5.0, 15.0),
        );
        let pl = Surface::Plane(Plane::new(Frame::from_z(o, z.normalize().unwrap())));
        let region = forge_math::Box3 {
            x: Interval::new(-5.0, 4.0 * m as f64 + 5.0),
            y: Interval::new(-12.0, 12.0),
            z: Interval::new(-5.0, 25.0),
        };
        let t0 = std::time::Instant::now();
        let closed = meet_spline(&pl, &s);
        t_closed += t0.elapsed().as_secs_f64();
        let t1 = std::time::Instant::now();
        let traced = intcurve::trace(&pl, &s, region);
        t_trace += t1.elapsed().as_secs_f64();
        let Some(SurfaceSurface::Curves(cs)) = closed else {
            none += 1;
            continue;
        };
        for (c, _) in &cs {
            let g = match c {
                Curve::Procedural(p) => p.domain(),
                _ => continue,
            };
            e_closed = e_closed.max(worst(c, Interval::new(g.lo, g.hi), &pl, &s));
        }
        n += 1;
        match traced {
            Some(t) => {
                for ic in &t {
                    for i in 0..=64 {
                        let p = ic.eval(ic.domain_hi() * i as f64 / 64.0);
                        e_trace = e_trace.max(pl.implicit(p).abs()).max(s.implicit(p).abs());
                    }
                }
                if t.is_empty() {
                    mismatch += 1;
                }
            }
            None => mismatch += 1,
        }
    }
    println!(
        "{n} cuts ({none} none), closed {:.1} us each, worst off either surface {e_closed:.2e} mm; traced {:.1} us each, worst {e_trace:.2e} mm, {mismatch} traced nothing",
        1e6 * t_closed / 400.0,
        1e6 * t_trace / 400.0
    );
}
