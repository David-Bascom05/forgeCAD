// Measurements of the rational corner quads against today's patch (see README.md)
use forge_geom::corner::{HolePatch, audit, clear_proof_cache, fit_sphere, rational_pieces};
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
    let Some(n) = d.du.cross(d.dv).normalize() else {
        return f64::INFINITY;
    };
    let n = n.get();
    let (e, f, g) = (d.du.dot(d.du), d.du.dot(d.dv), d.dv.dot(d.dv));
    let (l, m, nn) = (d.duu.dot(n), d.duv.dot(n), d.dvv.dot(n));
    let det = e * g - f * f;
    let h = (e * nn - 2.0 * f * m + g * l) / (2.0 * det);
    let k = (l * nn - m * m) / det;
    h.abs() + (h * h - k).max(0.0).sqrt()
}

fn grid(p: &HolePatch, m: usize) -> Vec<Point2> {
    let d = p.domain();
    let mut out = Vec::new();
    for i in 0..=m {
        for j in 0..=m {
            let x = Point2::new(
                d.u.lo + (d.u.hi - d.u.lo) * i as f64 / m as f64,
                d.v.lo + (d.v.hi - d.v.lo) * j as f64 / m as f64,
            );
            let q = p.project(p.point(x));
            if q.distance(x) < 1e-9 * (d.u.hi - d.u.lo) {
                out.push(x);
            }
        }
    }
    out
}

fn q(v: &mut Vec<f64>, f: f64) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v.get(((v.len() as f64 - 1.0) * f) as usize)
        .copied()
        .unwrap_or(f64::NAN)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cases: u64 = args.get(1).map_or(60, |s| s.parse().unwrap());
    let edges: usize = args.get(2).map_or(3, |s| s.parse().unwrap());
    let parity: usize = args.get(3).map_or(0, |s| s.parse().unwrap());
    let verbose = std::env::var("PER").is_ok();
    let noshape = std::env::var("NOSHAPE").is_ok();
    let mut rng = Rng::new(4242);
    let mut holes = Vec::new();
    // the survey's 3-, 4- and 5-edge corners (as in the stage 1 measurements); more edges on
    // their own seeds
    let span = if edges <= 5 { 3 } else { 1 };
    for case in 0..cases * span {
        let n = if edges <= 5 {
            3 + (case % 3) as usize
        } else {
            edges
        };
        let az: Vec<f64> = (0..n)
            .map(|i| (i as f64 + rng.range(-0.15, 0.15)) * dmath::TAU / n as f64)
            .collect();
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
    let (mut built, mut proved, mut none) = (0, 0, 0);
    let (mut tb0, mut tb1, mut tbw) = (Vec::new(), Vec::new(), Vec::new());
    let mut tests = Vec::new();
    let mut rows = Vec::new();
    let mut depth = 0;
    if let Ok(mode) = std::env::var("COUNT") {
        // one kind of work only, 5 times over, for instruction counts
        let quads: Vec<Vec<HolePatch>> = if mode == "proof" {
            holes
                .iter()
                .filter_map(|(_, s)| forge_geom::corner::rational_quads(s, parity))
                .collect()
        } else {
            Vec::new()
        };
        for _ in 0..5 {
            clear_proof_cache();
            for (_, s) in &holes {
                match mode.as_str() {
                    "today" => {
                        let _ = HolePatch::new(s.clone());
                    }
                    "build" => {
                        let _ = forge_geom::corner::rational_quads(s, parity);
                    }
                    _ => {}
                }
            }
            for q in &quads {
                for p in q {
                    let _ = p.prove_rational();
                }
            }
        }
        println!("{} holes", holes.len());
        return;
    }
    if std::env::var("PROOFONLY").is_ok() {
        let quads: Vec<Vec<HolePatch>> = holes
            .iter()
            .filter_map(|(_, s)| forge_geom::corner::rational_quads(s, parity))
            .collect();
        let t = Instant::now();
        let mut n = 0;
        for _ in 0..5 {
            clear_proof_cache();
            for q in &quads {
                for p in q {
                    n += p.prove_rational().unwrap().tests;
                }
            }
        }
        let dt = t.elapsed().as_secs_f64();
        println!(
            "proof only: {:.3} ms per hole, {:.2} us per test",
            dt * 1e3 / (5.0 * quads.len() as f64),
            dt * 1e6 / n as f64
        );
        let t = Instant::now();
        for _ in 0..5 {
            for (_, s) in &holes {
                let _ = forge_geom::corner::rational_quads(s, parity);
            }
        }
        println!(
            "build only: {:.3} ms per hole",
            t.elapsed().as_secs_f64() * 1e3 / (5.0 * holes.len() as f64)
        );
        return;
    }
    for (case, sides) in &holes {
        let t = Instant::now();
        let old = HolePatch::new(sides.clone()).ok();
        tb0.push(t.elapsed().as_secs_f64() * 1e3);
        clear_proof_cache();
        let t = Instant::now();
        if std::env::var("FOLDS").is_ok() {
            // today's split pieces with the rational sides... the rational quads, audited unproven
            if let Some(ps) = forge_geom::corner::rational_quads(sides, parity) {
                for (i, p) in ps.iter().enumerate() {
                    let a = audit(p, 3);
                    let ss = p.sides();
                    let mut angs = Vec::new();
                    for c in 0..4 {
                        let (s0, s1) = (&ss[(c + 3) % 4], &ss[c]);
                        let j0 = s0.rail.jet(s0.t1);
                        let j1 = s1.rail.jet(s1.t0);
                        let back = (j0.dp * (s0.t0 - s0.t1)).normalize().unwrap().get();
                        let ahead = (j1.dp * (s1.t1 - s1.t0)).normalize().unwrap().get();
                        let n = j1.n.normalize().unwrap().get();
                        angs.push(dmath::atan2(back.dot(n.cross(ahead)), back.dot(ahead)).to_degrees());
                    }
                    println!(
                        "case {case} quad {i}: proved {:?} fold {:.3} corner angles {:.0?}",
                        p.prove_rational().map(|x| x.proved),
                        a.fold,
                        angs
                    );
                    if a.fold < 0.5 {
                        // dense Jacobian scan against the normal at the middle, and the worst pair
                        let d = p.domain();
                        let k = d.u.hi - d.u.lo;
                        let m = 400;
                        let (mut jmin, mut jmax) = (f64::INFINITY, f64::NEG_INFINITY);
                        let mut worst = (1.0, Point2::new(0.0, 0.0));
                        let mut prev: Vec<Option<Vec3>> = vec![None; m + 1];
                        for a in 0..=m {
                            for b in 0..=m {
                                let x = Point2::new(k * a as f64 / m as f64, k * b as f64 / m as f64);
                                let dd = p.deriv(x);
                                let j = dd.du.cross(dd.dv);
                                let l = j.length();
                                jmin = jmin.min(l);
                                jmax = jmax.max(l);
                                let nn = j.normalize().map(|u| u.get());
                                if let (Some(n1), Some(n2)) = (nn, prev[b]) {
                                    if n1.dot(n2) < worst.0 {
                                        worst = (n1.dot(n2), x);
                                    }
                                }
                                prev[b] = nn;
                            }
                        }
                        println!(
                            "   |J| min {jmin:.3e} max {jmax:.3e}; worst neighbour cosine {:.3} at ({:.4}, {:.4})",
                            worst.0,
                            worst.1.x / k,
                            worst.1.y / k
                        );
                    }
                }
            }
        }
        let Some((pieces, pr)) = rational_pieces(sides) else {
            none += 1;
            if verbose {
                println!("case {case}: no rational form");
            }
            continue;
        };
        tb1.push(t.elapsed().as_secs_f64() * 1e3);
        // warm: the proof from the cache
        let t = Instant::now();
        let _ = rational_pieces(sides);
        tbw.push(t.elapsed().as_secs_f64() * 1e3);
        built += 1;
        tests.push(pr.tests as f64);
        depth = depth.max(pr.depth);
        if pr.proved {
            proved += 1;
        }
        if std::env::var("PROJ").is_ok() {
            for (i, p) in pieces.iter().enumerate() {
                let a = audit(p, 7);
                if a.project > 1e-6 {
                    let d = p.domain();
                    let k = d.u.hi - d.u.lo;
                    let mut worst = (0.0, Point2::new(0.0, 0.0), Point2::new(0.0, 0.0));
                    for a in 0..=50 {
                        for b in 0..=50 {
                            let x = Point2::new(k * a as f64 / 50.0, k * b as f64 / 50.0);
                            let y = p.project(p.point(x));
                            let e = p.point(y).distance(p.point(x));
                            if e > worst.0 {
                                worst = (e, x, y);
                            }
                        }
                    }
                    println!(
                        "case {case} piece {i}: project {:.2e}; grid worst {:.2e} at ({:.3},{:.3}) -> ({:.3},{:.3})",
                        a.project,
                        worst.0,
                        worst.1.x / k,
                        worst.1.y / k,
                        worst.2.x / k,
                        worst.2.y / k
                    );
                }
            }
        }
        let mut aud = audit(&pieces[0], 7);
        for p in &pieces[1..] {
            let a = audit(p, 7);
            aud.boundary = aud.boundary.max(a.boundary);
            aud.normal = aud.normal.max(a.normal);
            aud.derivs = aud.derivs.max(a.derivs);
            aud.project = aud.project.max(a.project);
            aud.bbox_misses += a.bbox_misses;
            aud.fold = aud.fold.min(a.fold);
        }
        let Some(old) = old else {
            rows.push((aud.clone(), aud, 1.0, 1.0, 1.0, 1.0, 0.0));
            continue;
        };
        let a0 = audit(&old, 7);
        if noshape {
            rows.push((a0, aud, 1.0, 1.0, 1.0, 1.0, 0.0));
            continue;
        }
        let size = sides
            .iter()
            .map(|s| s.rail.point(s.t0).0.distance(s.rail.point(s.t1).0))
            .fold(0.0, f64::max);
        let mut bpts = Vec::new();
        for s in sides {
            for k in 0..16 {
                let t = s.t0 + (s.t1 - s.t0) * (k as f64 + 0.5) / 16.0;
                bpts.push(s.rail.point(t));
            }
        }
        let c0 = bpts.iter().fold(Vec3::ZERO, |a, p| a + (p.0 - Point3::ORIGIN)) / bpts.len() as f64;
        let nc = bpts
            .iter()
            .fold(Vec3::ZERO, |a, p| a + p.1)
            .normalize()
            .unwrap()
            .get();
        let shape = |ps: &[&HolePatch]| {
            let (mut kmax, mut bulge): (f64, f64) = (0.0, f64::NEG_INFINITY);
            let mut pts = Vec::new();
            for p in ps {
                for x in grid(p, 24) {
                    kmax = kmax.max(max_curvature(p, x));
                    let q = p.point(x);
                    bulge = bulge.max((q - Point3::ORIGIN - c0).dot(nc).abs());
                    pts.push(q);
                }
            }
            (kmax, bulge, pts)
        };
        if std::env::var("KLOC").is_ok() {
            for (pi, p) in pieces.iter().enumerate() {
                let d = p.domain();
                let (mut best, mut at) = (0.0, Point2::new(0.0, 0.0));
                for x in grid(p, 40) {
                    let k = max_curvature(p, x);
                    if k > best {
                        best = k;
                        at = x;
                    }
                }
                let w = d.u.hi - d.u.lo;
                println!(
                    "case {case} piece {pi}: kmax {best:.2} at ({:.3}, {:.3})",
                    (at.x - d.u.lo) / w,
                    (at.y - d.v.lo) / w
                );
            }
            let mut best: f64 = 0.0;
            for x in grid(&old, 40) {
                best = best.max(max_curvature(&old, x));
            }
            println!("case {case} today kmax {best:.2}");
        }
        let (k0, b0, _) = shape(&[&old]);
        let (k1, b1, pts) = shape(&pieces.iter().collect::<Vec<_>>());
        let mut apart: f64 = 0.0;
        for q in pts.iter().step_by(3) {
            let back = old.point(old.project(*q));
            apart = apart.max(back.distance(*q));
        }
        if verbose {
            println!(
                "case {case}: {} pieces proof {:?} | boundary {:.1e} normal {:.1e} fold {:.2} misses {} | kmax {:.2}/{:.2} bulge {:.3}/{:.3} apart {:.3} of {:.2}",
                pieces.len(),
                pr,
                aud.boundary,
                aud.normal,
                aud.fold,
                aud.bbox_misses,
                k0,
                k1,
                b0,
                b1,
                apart,
                size
            );
        }
        rows.push((a0, aud, k0, k1, b0, b1, apart / size));
    }
    println!(
        "{} holes of {} sides; rational built {built} (none {none}); proved {proved}; deepest split {depth}",
        holes.len(),
        2 * edges
    );
    println!(
        "build ms: today median {:.3} p90 {:.3} | rational cold (with proof) median {:.3} p90 {:.3} max {:.3} | warm (cached proof) median {:.3}",
        q(&mut tb0, 0.5),
        q(&mut tb0, 0.9),
        q(&mut tb1, 0.5),
        q(&mut tb1, 0.9),
        q(&mut tb1, 1.0),
        q(&mut tbw, 0.5)
    );
    println!(
        "cone tests per hole: median {} max {}",
        q(&mut tests, 0.5),
        q(&mut tests, 1.0)
    );
    let worst = |f: &dyn Fn(
        &(
            forge_geom::corner::Audit,
            forge_geom::corner::Audit,
            f64,
            f64,
            f64,
            f64,
            f64,
        ),
    ) -> f64| { rows.iter().map(f).fold(0.0f64, f64::max) };
    println!(
        "worst boundary today {:.1e} rational {:.1e}",
        worst(&|r| r.0.boundary),
        worst(&|r| r.1.boundary)
    );
    println!(
        "worst normal (rad) today {:.1e} rational {:.1e}",
        worst(&|r| r.0.normal),
        worst(&|r| r.1.normal)
    );
    println!(
        "worst derivs today {:.1e} rational {:.1e}; project today {:.1e} rational {:.1e}",
        worst(&|r| r.0.derivs),
        worst(&|r| r.1.derivs),
        worst(&|r| r.0.project),
        worst(&|r| r.1.project)
    );
    println!(
        "bbox misses today {} rational {}; min fold cosine today {:.3} rational {:.3}",
        rows.iter().map(|r| r.0.bbox_misses).sum::<usize>(),
        rows.iter().map(|r| r.1.bbox_misses).sum::<usize>(),
        rows.iter().map(|r| r.0.fold).fold(1.0f64, f64::min),
        rows.iter().map(|r| r.1.fold).fold(1.0f64, f64::min)
    );
    if !noshape {
        let mut kr: Vec<f64> = rows.iter().map(|r| r.3 / r.2).collect();
        let mut br: Vec<f64> = rows.iter().map(|r| r.5 / r.4).collect();
        let mut ap: Vec<f64> = rows.iter().map(|r| r.6).collect();
        println!(
            "max curvature rational/today: min {:.2} median {:.2} max {:.2}",
            q(&mut kr, 0.0),
            q(&mut kr, 0.5),
            q(&mut kr, 1.0)
        );
        println!(
            "bulge rational/today: min {:.2} median {:.2} max {:.2}",
            q(&mut br, 0.0),
            q(&mut br, 0.5),
            q(&mut br, 1.0)
        );
        println!(
            "largest gap / hole size: median {:.3} p90 {:.3} max {:.3}",
            q(&mut ap, 0.5),
            q(&mut ap, 0.9),
            q(&mut ap, 1.0)
        );
    }
}
