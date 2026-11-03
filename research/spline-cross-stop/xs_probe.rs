//! Probe: the leaf Newton of spline::meet::cross_sections far from the origin. Random closed blob
//! pairs, each moved out to several distances; for each crossing a polyline finds, run the leaf's
//! Newton from a nearby start and record the step measure and residual floor, and count crossings
//! the kernel's cross_sections drops against a fine polyline.
use forge_geom::spline::Spline;
use forge_geom::spline::meet::cross_sections;
use forge_math::rng::Rng;
use forge_math::{Point3, Vec3};

fn blob(rng: &mut Rng, c: (f64, f64), r: f64) -> Spline {
    let n = 5 + rng.below(6);
    let mut pts: Vec<Point3> = (0..n)
        .map(|i| {
            let a = std::f64::consts::TAU * (i as f64 + rng.range(-0.3, 0.3)) / n as f64;
            let rr = r * rng.range(0.6, 1.4);
            Point3::new(c.0 + rr * a.cos(), c.1 + rr * a.sin(), 0.0)
        })
        .collect();
    pts.push(pts[0]);
    Spline::through(&pts, None, None).unwrap()
}

fn poly(s: &Spline, n: usize) -> Vec<(f64, Point3)> {
    let d = s.domain();
    (0..=n)
        .map(|k| {
            let t = d.lo + d.width() * k as f64 / n as f64;
            (t, s.point(t))
        })
        .collect()
}

/// Crossings of two fine polylines: (t on a, t on b, sine of the angle).
fn poly_hits(a: &Spline, b: &Spline) -> Vec<(f64, f64, f64)> {
    let (pa, pb) = (poly(a, 3000), poly(b, 3000));
    let mut out = Vec::new();
    for i in 0..pa.len() - 1 {
        for j in 0..pb.len() - 1 {
            let (p, r) = (pa[i].1, pa[i + 1].1 - pa[i].1);
            let (q, s) = (pb[j].1, pb[j + 1].1 - pb[j].1);
            let d = r.x * s.y - r.y * s.x;
            if d == 0.0 {
                continue;
            }
            let w = q - p;
            let (t, u) = ((w.x * s.y - w.y * s.x) / d, (w.x * r.y - w.y * r.x) / d);
            if (0.0..1.0).contains(&t) && (0.0..1.0).contains(&u) {
                let sin = d.abs() / (r.length() * s.length());
                out.push((
                    pa[i].0 + (pa[i + 1].0 - pa[i].0) * t,
                    pb[j].0 + (pb[j + 1].0 - pb[j].0) * u,
                    sin,
                ));
            }
        }
    }
    out
}

/// The leaf's Newton from (s, t), 12 steps: per step the step measure and the residual before it.
fn newton(a: &Spline, b: &Spline, mut s: f64, mut t: f64) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for _ in 0..12 {
        let (p, q) = (a.deriv(s), b.deriv(t));
        let e = q.p - p.p;
        let det = p.d1.x * (-q.d1.y) + q.d1.x * p.d1.y;
        let ds = (e.x * (-q.d1.y) + q.d1.x * e.y) / det;
        let dt = (p.d1.x * e.y - p.d1.y * e.x) / det;
        s += ds;
        t += dt;
        out.push(((ds * p.d1.length()).abs() + (dt * q.d1.length()).abs(), e.length()));
    }
    out
}

const SHALLOW: bool = option_env!("SHALLOW").is_some();
const PARAM: bool = option_env!("PARAM").is_some();

fn main() {
    let eps = f64::EPSILON;
    let dists = [0.0, 30.0, 300.0, 1000.0, 1e4, 1e5];
    let cases = if SHALLOW { 600 } else { 1500 };
    // per distance: crossings, kernel drops, worst (floor step / eps scale), worst (floor step sin / eps scale),
    // worst (floor residual / eps scale), worst 4th-step residual / eps scale
    let mut stats = vec![(0usize, 0usize, 0.0f64, 0.0f64, 0.0f64, 0usize, 0usize, 1.0f64, 0.0f64); dists.len()];
    let mut rng = Rng::new(7);
    for _ in 0..cases {
        let r = rng.range(5.0, 40.0);
        let a0 = blob(&mut rng, (0.0, 0.0), r);
        let c = (rng.range(-1.2, 1.2) * r, rng.range(-1.2, 1.2) * r);
        let rb = r * rng.range(0.4, 1.2);
        // shallow: a0 mirrored across a line through one of its points, phi/2 off its tangent
        let (t_o, phi) = (
            a0.domain().lo + a0.domain().width() * rng.unit(),
            10f64.powf(rng.range(-5.5, -2.0)),
        );
        let b0 = if SHALLOW {
            let d = a0.deriv(t_o);
            let o = d.p;
            let g = d.d1.y.atan2(d.d1.x) + 0.5 * phi;
            let u = Vec3::new(g.cos(), g.sin(), 0.0);
            let _ = (c, rb);
            a0.mapped(|p| {
                let v = p - o;
                o + u * (2.0 * v.dot(u)) - v
            })
        } else {
            blob(&mut rng, c, rb)
        };
        let dir = rng.range(0.0, std::f64::consts::TAU);
        for (k, &d) in dists.iter().enumerate() {
            let off = Vec3::new(d * dir.cos(), d * dir.sin(), 0.0);
            let (a, b) = (a0.mapped(|p| p + off), b0.mapped(|p| p + off));
            let hits = if SHALLOW { vec![(t_o, t_o, phi.sin())] } else { poly_hits(&a, &b) };
            let got = cross_sections(&a, a.domain(), &b, b.domain());
            let st = &mut stats[k];
            for &(s, t, sin) in &hits {
                st.0 += 1;
                if !got.iter().any(|g| (g.0 - s).abs() <= 1e-3) {
                    st.1 += 1;
                    if d == 0.0 {
                        let dom = (a.domain(), b.domain());
                        let pa = a.deriv(s);
                        let pb = b.deriv(t);
                        let cr = (pa.d1.x * pb.d1.y - pa.d1.y * pb.d1.x) / (pa.d1.length() * pb.d1.length());
                        eprintln!("drop s {s} t {t} sin {sin:.3e} dom {dom:?} gap {:.2e} cross {cr:.3e} got {got:?} hits {hits:?}", pa.p.distance(pb.p));
                    }
                }
                let steps = newton(&a, &b, s, t);
                let (p, q) = (a.deriv(s), b.deriv(t));
                let scale = 1.0 + p.p.to_vec().abs_max() + if PARAM { s.abs() * p.d1.length() + t.abs() * q.d1.length() } else { 0.0 };
                st.7 = st.7.min(sin);
                if sin < 1e-3 {
                    st.8 = st.8.max(steps[5..].iter().map(|x| x.1).fold(0.0, f64::max) / (eps * scale));
                    continue;
                }
                let floor = &steps[5..];
                let ms = floor.iter().map(|x| x.0).fold(0.0, f64::max);
                let mr = floor.iter().map(|x| x.1).fold(0.0, f64::max);
                st.2 = st.2.max(ms / (eps * scale));
                st.3 = st.3.max(ms * sin / (eps * scale));
                st.4 = st.4.max(mr / (eps * scale));
                // does the old stop ever pass on the floor, and does a scaled one?
                if floor.iter().all(|x| x.0 > 1e-13) {
                    st.5 += 1;
                }
                if floor.iter().all(|x| x.1 > 64.0 * eps * scale) {
                    st.6 += 1;
                }
            }
        }
    }
    println!("dist    crossings  kernel-drops  step/eps.s  step*sin/eps.s  resid/eps.s  never<1e-13  never-resid<64eps.s  least-sin  shallow-resid/eps.s");
    for (k, &d) in dists.iter().enumerate() {
        let s = stats[k];
        println!(
            "{d:<8} {:>9} {:>13} {:>11.1} {:>15.1} {:>12.1} {:>12} {:>20} {:>10.1e} {:>8.1}",
            s.0, s.1, s.2, s.3, s.4, s.5, s.6, s.7, s.8
        );
    }
}
