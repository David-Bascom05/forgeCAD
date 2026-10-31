// Stage 1 measurement (ran on refs/experiments/corner-rational-stage1, commit 4b81f12): today's patch against the n-gon rational patch
use forge_geom::corner::{HolePatch, audit, fit_sphere};
use forge_geom::surface::Procedural;
use forge_math::rng::Rng;
use forge_math::{Point2, Point3, Vec3, dmath};
use forge_ops::corner::synthetic::*;
use forge_ops::corner::{face_curves, setbacks};
use forge_ops::fillet::CornerInput;
use std::time::Instant;

fn cone(az: &[f64], h: &[f64], radii: &[f64]) -> CornerInput {
    let n = az.len();
    let dirs: Vec<Vec3> = (0..n)
        .map(|i| {
            let (s, c) = dmath::sin_cos(az[i]);
            Vec3::new(c, s, h[i])
        })
        .collect();
    let normals: Vec<Vec3> = (0..n)
        .map(|i| {
            let m = dirs[(i + n - 1) % n].cross(dirs[i]).normalize().unwrap().get();
            if m.z < 0.0 { -m } else { m }
        })
        .collect();
    let fillets: Vec<Option<(f64, bool)>> = (0..n)
        .map(|i| {
            let (d, e) = (dirs[i].normalize().unwrap().get(), dirs[(i + n - 1) % n]);
            let w = e - d * d.dot(e);
            Some((radii[i], normals[(i + 1) % n].dot(w) < 0.0))
        })
        .collect();
    corner(Point3::new(0.5, -1.0, 2.0), &dirs, &normals, &fillets)
}

fn flipped(az: &[f64], h: &[f64], radii: &[f64]) -> CornerInput {
    let rev = |v: &[f64]| v.iter().rev().copied().collect::<Vec<_>>();
    let az: Vec<f64> = rev(az).iter().map(|&a| -a).collect();
    let h: Vec<f64> = rev(h).iter().map(|&x| -x).collect();
    cone(&az, &h, &rev(radii))
}

fn max_curvature(p: &HolePatch, x: Point2) -> f64 {
    let d = p.deriv(x);
    let Some(n) = d.du.cross(d.dv).normalize() else { return f64::INFINITY };
    let n = n.get();
    let (e, f, g) = (d.du.dot(d.du), d.du.dot(d.dv), d.dv.dot(d.dv));
    let (l, m, nn) = (d.duu.dot(n), d.duv.dot(n), d.dvv.dot(n));
    let det = e * g - f * f;
    let h = (e * nn - 2.0 * f * m + g * l) / (2.0 * det);
    let k = (l * nn - m * m) / det;
    h.abs() + (h * h - k).max(0.0).sqrt()
}

/// Interior grid points of the patch's domain.
fn grid(p: &HolePatch, m: usize) -> Vec<Point2> {
    let d = p.domain();
    let mut out = Vec::new();
    for i in 0..=m {
        for j in 0..=m {
            let x = Point2::new(d.u.lo + (d.u.hi - d.u.lo) * i as f64 / m as f64, d.v.lo + (d.v.hi - d.v.lo) * j as f64 / m as f64);
            // inside: the patch's clamp leaves it where it is
            let q = p.project(p.point(x));
            if q.distance(x) < 1e-9 * (d.u.hi - d.u.lo) {
                out.push(x);
            }
        }
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cases: u64 = args.get(1).map_or(60, |s| s.parse().unwrap());
    let edges: usize = args.get(2).map_or(3, |s| s.parse().unwrap());
    let verbose = std::env::var("PER").is_ok();
    let mut rng = Rng::new(4242);
    let mut holes = Vec::new();
    for case in 0..cases * 3 {
        let n = 3 + (case % 3) as usize;
        let az: Vec<f64> = (0..n).map(|i| (i as f64 + rng.range(-0.15, 0.15)) * dmath::TAU / n as f64).collect();
        let saddle = n >= 4 && case % 2 == 1;
        let flat = case % 5 == 2;
        let h: Vec<f64> = (0..n)
            .map(|i| {
                if flat && i == 0 {
                    -rng.range(0.02, 0.1)
                } else if saddle && i % 2 == 1 {
                    rng.range(0.2, 0.8)
                } else {
                    -rng.range(0.4, 1.5)
                }
            })
            .collect();
        let radii: Vec<f64> = (0..n).map(|_| rng.range(0.5, 2.0)).collect();
        if n != edges {
            continue;
        }
        for c in [cone(&az, &h, &radii), flipped(&az, &h, &radii)] {
            let Some(s) = setbacks(&c) else { continue };
            let Ok(trims) = face_curves(&c, &s) else { continue };
            let sides = sides(&hole(&c, &s, &trims));
            if fit_sphere(&sides).is_some() {
                continue;
            }
            holes.push((case, sides));
        }
    }
    if std::env::var("KLOC").is_ok() {
        for (case, sides) in holes.iter().take(6) {
            let old = HolePatch::new(sides.clone()).unwrap();
            let new = HolePatch::new_rational(sides.clone()).unwrap();
            let g = grid(&new, 40);
            let d = new.domain();
            let (mut best, mut at) = (0.0, Point2::new(0.0, 0.0));
            for &x in &g {
                let k = max_curvature(&new, x);
                if k > best {
                    best = k;
                    at = x;
                }
            }
            // where: relative position and distance to the nearest domain corner
            println!("case {case}: kmax {best:.2} (today {:.2} there) at ({:.2}, {:.2}) of [{:.2},{:.2}]x[{:.2},{:.2}]", max_curvature(&old, at), at.x, at.y, d.u.lo, d.u.hi, d.v.lo, d.v.hi);
            for i in 0..sides.len() {
                print!(" c{i} {:.2},{:.2}", new.side_uv(i, 0.0).x, new.side_uv(i, 0.0).y);
            }
            println!();
        }
        return;
    }
    if std::env::var("DBG").is_ok() {
        let (_, sides) = &holes[0];
        let old = HolePatch::new(sides.clone()).unwrap();
        let new = HolePatch::new_rational(sides.clone()).unwrap();
        let d = new.domain();
        let c = Point2::new(0.5 * (d.u.lo + d.u.hi), 0.5 * (d.v.lo + d.v.hi));
        let e = new.side_param_uv(0, 0.5 * (sides[0].t0 + sides[0].t1));
        for k in 0..=20 {
            let x = c.lerp(e, k as f64 / 20.0);
            let (a, b) = (old.deriv(x), new.deriv(x));
            let h = 1e-6 * (d.u.hi - d.u.lo);
            let fd = (new.point(Point2::new(x.x + h, x.y)) - new.point(Point2::new(x.x - h, x.y))) / (2.0 * h);
            println!("{k:2} old {:.4?} new {:.4?} |J| {:.3} {:.3} du {:.3?} fd {:.3?}", a.p, b.p, a.du.cross(a.dv).length(), b.du.cross(b.dv).length(), b.du, fd);
        }
        return;
    }
    let (mut built, mut proved, mut folds, mut refused, mut no_rat) = (0, 0, 0, 0, 0);
    let mut rows = Vec::new();
    let (mut tb0, mut tb1, mut tp, mut te0, mut te1) = (0.0, 0.0, 0.0, 0.0, 0.0);
    let mut prove_times = Vec::new();
    for (case, sides) in &holes {
        let t = Instant::now();
        let Ok(old) = HolePatch::new(sides.clone()) else { continue };
        tb0 += t.elapsed().as_secs_f64();
        let t = Instant::now();
        let Some(new) = HolePatch::new_rational(sides.clone()) else {
            no_rat += 1;
            if verbose {
                println!("case {case}: no rational form");
            }
            continue;
        };
        tb1 += t.elapsed().as_secs_f64();
        built += 1;
        let t = Instant::now();
        let pr = new.prove_rational().unwrap();
        let dt = t.elapsed().as_secs_f64();
        tp += dt;
        prove_times.push(dt * 1e3);
        if pr.proved {
            proved += 1;
        } else if false {
            folds += 1;
        } else {
            refused += 1;
        }
        let (a0, a1) = (audit(&old, 7), audit(&new, 7));
        // shape: curvature range and bulge off the boundary's mean plane, and how far apart
        let size = sides.iter().map(|s| s.rail.point(s.t0).0.distance(s.rail.point(s.t1).0)).fold(0.0, f64::max);
        let mut bpts = Vec::new();
        for s in sides {
            for k in 0..16 {
                let t = s.t0 + (s.t1 - s.t0) * (k as f64 + 0.5) / 16.0;
                bpts.push(s.rail.point(t));
            }
        }
        let c0 = bpts.iter().fold(Vec3::ZERO, |a, p| a + (p.0 - Point3::ORIGIN)) / bpts.len() as f64;
        let nc = bpts.iter().fold(Vec3::ZERO, |a, p| a + p.1).normalize().unwrap().get();
        let shape = |p: &HolePatch, te: &mut f64| {
            let g = grid(p, 24);
            let (mut kmax, mut bulge): (f64, f64) = (0.0, f64::NEG_INFINITY);
            let t = Instant::now();
            for &x in &g {
                kmax = kmax.max(max_curvature(p, x));
                bulge = bulge.max((p.point(x) - Point3::ORIGIN - c0).dot(nc).abs());
            }
            *te += t.elapsed().as_secs_f64() / g.len() as f64;
            (kmax, bulge, g)
        };
        let (k0, b0, _) = shape(&old, &mut te0);
        let (k1, b1, g1) = shape(&new, &mut te1);
        let mut apart: f64 = 0.0;
        for &x in g1.iter().step_by(3) {
            let q = new.point(x);
            let back = old.point(old.project(q));
            apart = apart.max(back.distance(q));
        }
        if verbose {
            println!(
                "case {case}: proof {:?} {:.3} ms | boundary {:.1e}/{:.1e} normal {:.1e}/{:.1e} fold {:.2}/{:.2} bbox misses {}/{} | kmax {:.2}/{:.2} bulge {:.3}/{:.3} apart {:.3} of {:.2}",
                pr, dt * 1e3, a0.boundary, a1.boundary, a0.normal, a1.normal, a0.fold, a1.fold, a0.bbox_misses, a1.bbox_misses, k0, k1, b0, b1, apart, size
            );
        }
        rows.push((a0, a1, k0, k1, b0, b1, apart / size));
    }
    let n = rows.len().max(1) as f64;
    prove_times.sort_by(|a, b| a.total_cmp(b));
    let q = |f: f64| prove_times.get(((prove_times.len() as f64 - 1.0) * f) as usize).copied().unwrap_or(0.0);
    println!("{} holes of {edges} edges; rational built {built} (none: {no_rat}); proved {proved}, folds {folds}, refused {refused}", holes.len());
    println!("proof split: blend {:.1} ms, jacobian + bernstein {:.1} ms total", forge_geom::corner::T_BLEND.load(std::sync::atomic::Ordering::Relaxed) as f64 * 1e-6, forge_geom::corner::T_PROVE.load(std::sync::atomic::Ordering::Relaxed) as f64 * 1e-6);
    println!("proof ms: median {:.2} p90 {:.2} max {:.2}", q(0.5), q(0.9), q(1.0));
    println!("build ms: today {:.3}, rational {:.3} (without proof)", tb0 / n * 1e3, tb1 / n * 1e3);
    println!("point + deriv + curvature per sample us: today {:.2}, rational {:.2}", te0 / n * 1e6, te1 / n * 1e6);
    let worst = |f: &dyn Fn(&(forge_geom::corner::Audit, forge_geom::corner::Audit, f64, f64, f64, f64, f64)) -> f64| rows.iter().map(f).fold(0.0f64, f64::max);
    println!("worst boundary today {:.1e} rational {:.1e}", worst(&|r| r.0.boundary), worst(&|r| r.1.boundary));
    println!("worst normal (rad) today {:.1e} rational {:.1e}", worst(&|r| r.0.normal), worst(&|r| r.1.normal));
    println!("worst derivs today {:.1e} rational {:.1e}; project today {:.1e} rational {:.1e}", worst(&|r| r.0.derivs), worst(&|r| r.1.derivs), worst(&|r| r.0.project), worst(&|r| r.1.project));
    println!("bbox misses today {} rational {}", rows.iter().map(|r| r.0.bbox_misses).sum::<usize>(), rows.iter().map(|r| r.1.bbox_misses).sum::<usize>());
    println!("min fold cosine today {:.3} rational {:.3}", rows.iter().map(|r| r.0.fold).fold(1.0f64, f64::min), rows.iter().map(|r| r.1.fold).fold(1.0f64, f64::min));
    let mut kr: Vec<f64> = rows.iter().map(|r| r.3 / r.2).collect();
    kr.sort_by(|a, b| a.total_cmp(b));
    let mut br: Vec<f64> = rows.iter().map(|r| r.5 / r.4).collect();
    br.sort_by(|a, b| a.total_cmp(b));
    let mut ap: Vec<f64> = rows.iter().map(|r| r.6).collect();
    ap.sort_by(|a, b| a.total_cmp(b));
    let m = |v: &Vec<f64>, f: f64| v.get(((v.len() as f64 - 1.0) * f) as usize).copied().unwrap_or(0.0);
    println!("max curvature rational/today: min {:.2} median {:.2} max {:.2}", m(&kr, 0.0), m(&kr, 0.5), m(&kr, 1.0));
    println!("bulge rational/today: min {:.2} median {:.2} max {:.2}", m(&br, 0.0), m(&br, 0.5), m(&br, 1.0));
    println!("largest distance between the two / hole size: median {:.3} max {:.3}", m(&ap, 0.5), m(&ap, 1.0));
}
