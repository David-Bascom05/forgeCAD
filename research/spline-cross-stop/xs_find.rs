//! Finder: round-number blob pairs near (300, 150) whose crossings the old fixed stop drops.
use forge_geom::spline::Spline;
use forge_math::{Interval, Point3, Vec3};

fn old(a: &Spline, ra: Interval, b: &Spline, rb: Interval) -> usize {
    let mut stack = vec![(ra, rb)];
    let mut out: Vec<f64> = Vec::new();
    while let Some((x, y)) = stack.pop() {
        let (bx, by) = (a.bbox(x), b.bbox(y));
        if !bx.grow(1e-5).overlaps(&by.grow(1e-5)) {
            continue;
        }
        let size = |b: &forge_math::Box3| b.x.width().max(b.y.width());
        if size(&bx) <= 1e-4 && size(&by) <= 1e-4 {
            let (mut s, mut t) = (x.mid(), y.mid());
            let mut good = false;
            for _ in 0..12 {
                let (p, q) = (a.deriv(s), b.deriv(t));
                let e = q.p - p.p;
                let det = p.d1.x * (-q.d1.y) + q.d1.x * p.d1.y;
                let ds = (e.x * (-q.d1.y) + q.d1.x * e.y) / det;
                let dt = (p.d1.x * e.y - p.d1.y * e.x) / det;
                s += ds;
                t += dt;
                if (ds * p.d1.length()).abs() + (dt * q.d1.length()).abs() <= 1e-13 {
                    good = true;
                    break;
                }
            }
            if good && ra.contains(s) && rb.contains(t) && out.iter().all(|h| (h - s).abs() > 1e-7) {
                out.push(s);
            }
            continue;
        }
        if size(&bx) >= size(&by) {
            let m = x.mid();
            stack.push((Interval::new(x.lo, m), y));
            stack.push((Interval::new(m, x.hi), y));
        } else {
            let m = y.mid();
            stack.push((x, Interval::new(y.lo, m)));
            stack.push((x, Interval::new(m, y.hi)));
        }
    }
    out.len()
}

fn closed(c: (f64, f64), v: &[(f64, f64)]) -> Spline {
    let mut p: Vec<Point3> = v.iter().map(|&(x, y)| Point3::new(c.0 + x, c.1 + y, 0.0)).collect();
    p.push(p[0]);
    Spline::through(&p, None, None).unwrap()
}

fn main() {
    let a = [(20.0, 0.0), (12.0, 15.0), (-5.0, 18.0), (-19.0, 6.0), (-16.0, -11.0), (0.0, -17.0), (14.0, -13.0)];
    let b = [(15.0, 0.0), (8.0, 11.0), (-6.0, 13.0), (-14.0, 2.0), (-9.0, -12.0), (6.0, -10.0)];
    let mut found = 0;
    for i in 0..40 {
        for j in 0..40 {
            let off = (i as f64 - 20.0, j as f64 - 20.0);
            let a0 = closed((0.0, 0.0), &a);
            let b0 = closed(off, &b);
            let n0 = old(&a0, a0.domain(), &b0, b0.domain());
            if n0 == 0 {
                continue;
            }
            let m = Vec3::new(300.0, 150.0, 0.0);
            let (a1, b1) = (a0.mapped(|p| p + m), b0.mapped(|p| p + m));
            let n1 = old(&a1, a1.domain(), &b1, b1.domain());
            if n1 != n0 {
                println!("offset {off:?}: origin {n0}, at 300 {n1}");
                found += 1;
                if found > 10 {
                    return;
                }
            }
        }
    }
}
