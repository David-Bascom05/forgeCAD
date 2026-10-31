// The rational corner patch on corners with sharp edges: odd holes (a spoke from the middle of
// a side) and kinked vertices (no spoke may leave them). See README.md.
use forge_geom::corner::{Audit, audit, fit_sphere, rational_pieces};
use forge_math::rng::Rng;
use forge_math::{Point3, Vec3, dmath};
use forge_ops::corner::synthetic::*;
use forge_ops::corner::{face_curves, setbacks};
use forge_ops::fillet::CornerInput;
use std::collections::BTreeMap;

// as rq.rs, with a radius of 0 or less for a sharp edge
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
            (radii[i] > 0.0).then_some((radii[i], normals[(i + 1) % n].dot(w) < 0.0))
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

/// CASES SHARP: 3-, 4- and 5-edge corners, SHARP neighbouring edges of each sharp.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cases: u64 = args.get(1).map_or(60, |s| s.parse().unwrap());
    let sharp: usize = args.get(2).map_or(1, |s| s.parse().unwrap());
    let mut rng = Rng::new(4243);
    let mut per: BTreeMap<usize, (usize, usize)> = BTreeMap::new();
    let mut worst = Audit::default();
    for case in 0..cases {
        let n = 3 + (case % 3) as usize;
        let az: Vec<f64> = (0..n)
            .map(|i| (i as f64 + rng.range(-0.15, 0.15)) * dmath::TAU / n as f64)
            .collect();
        let saddle = n >= 4 && case % 2 == 1;
        let h: Vec<f64> = (0..n)
            .map(|i| {
                if saddle && i % 2 == 1 {
                    rng.range(0.2, 0.8)
                } else {
                    -rng.range(0.4, 1.5)
                }
            })
            .collect();
        let mut radii: Vec<f64> = (0..n).map(|_| rng.range(0.5, 2.0)).collect();
        let first = rng.range(0.0, n as f64) as usize;
        for j in 0..sharp.min(n) {
            radii[(first + j) % n] = 0.0;
        }
        for c in [cone(&az, &h, &radii), flipped(&az, &h, &radii)] {
            let Some(s) = setbacks(&c) else { continue };
            let Ok(trims) = face_curves(&c, &s) else { continue };
            let sides = sides(&hole(&c, &s, &trims));
            if fit_sphere(&sides).is_some() {
                continue;
            }
            let e = per.entry(sides.len()).or_default();
            e.0 += 1;
            let Some((ps, pr)) = rational_pieces(&sides) else {
                continue;
            };
            if pr.proved {
                e.1 += 1;
            }
            let a = audit(&ps[0], 7);
            worst.boundary = worst.boundary.max(a.boundary);
            worst.normal = worst.normal.max(a.normal);
            worst.project = worst.project.max(a.project);
            worst.derivs = worst.derivs.max(a.derivs);
            worst.bbox_misses += a.bbox_misses;
        }
    }
    println!("{sharp} sharp: per sides (holes, proven) {per:?}");
    println!(
        "worst: boundary {:.1e} normal {:.1e} project {:.1e} derivs {:.1e} bbox misses {}",
        worst.boundary, worst.normal, worst.project, worst.derivs, worst.bbox_misses
    );
}
