//! The certified tracer on random quadric and torus pairs: the march's chains and meet.rs's closed
//! forms, each certified, with the cost.
//! cargo run --release -p forge-geom --features certified_trace --example certify_sweep -- [n] [seed]
use forge_geom::certify::{Why, certify};
use forge_geom::intersect::SurfaceSurface;
use forge_geom::surface::{Cone, Cylinder, Plane, Sphere, Surface, Torus};
use forge_geom::{Curve, intcurve::IntCurve};
use forge_math::{Box3, Frame, Point3, Vec3, rng::Rng};
use std::collections::BTreeMap;
use std::time::Instant;

fn frame(r: &mut Rng) -> Frame {
    let o = Point3::new(r.range(-3.0, 3.0), r.range(-3.0, 3.0), r.range(-3.0, 3.0));
    // axis-aligned half the time: the configurations parts are made of
    let z = if r.bool() {
        [Vec3::X, Vec3::Y, Vec3::Z][r.below(3)]
    } else {
        Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0))
    };
    Frame::from_z(o, z.normalize().unwrap_or(forge_math::UnitVec3::Z))
}

fn surface(kind: usize, r: &mut Rng) -> Surface {
    let f = frame(r);
    match kind {
        0 => Surface::Plane(Plane { frame: f }),
        1 => Surface::Sphere(Sphere {
            frame: f,
            radius: r.range(1.0, 6.0),
        }),
        2 => Surface::Cylinder(Cylinder {
            frame: f,
            radius: r.range(1.0, 5.0),
        }),
        3 => Surface::Cone(Cone::new(f, r.range(1.0, 4.0), r.range(0.2, 1.0)).expect("cone")),
        _ => {
            let minor = r.range(0.8, 2.5);
            Surface::Torus(Torus {
                frame: f,
                major: minor + r.range(0.5, 4.0),
                minor,
            })
        }
    }
}

const NAMES: [&str; 5] = ["plane", "sphere", "cylinder", "cone", "torus"];

fn chains(r: &SurfaceSurface) -> Option<Vec<IntCurve>> {
    match r {
        SurfaceSurface::Empty => Some(Vec::new()),
        SurfaceSurface::Curves(cs) => cs
            .iter()
            .map(|(c, _)| match c {
                Curve::Intersection(c) => Some((**c).clone()),
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(400);
    let seed: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    let mut rng = Rng::new(seed);
    let region = Box3::from_points([Point3::new(-8.0, -8.0, -8.0), Point3::new(8.0, 8.0, 8.0)]);
    // per pair: [march certified, march refused, closed form certified, closed form refused, closed form n/a]
    let mut tally: BTreeMap<String, [usize; 5]> = BTreeMap::new();
    let mut whys: BTreeMap<String, usize> = BTreeMap::new();
    let (mut t_march, mut t_cert) = (0.0, 0.0);
    for _ in 0..n {
        let (i, j) = (rng.below(5), 1 + rng.below(4));
        let (i, j) = (i.min(j), i.max(j));
        let (a, b) = (surface(i, &mut rng), surface(j, &mut rng));
        let key = format!("{}_{}", NAMES[i], NAMES[j]);
        let e = tally.entry(key.clone()).or_default();
        let t0 = Instant::now();
        let marched = forge_geom::intcurve::trace_marched(&a, &b, region, 1.0);
        t_march += t0.elapsed().as_secs_f64();
        if let Some(cs) = &marched {
            let t1 = Instant::now();
            let c = certify(&a, &b, region, cs);
            t_cert += t1.elapsed().as_secs_f64();
            match c {
                Ok(()) => e[0] += 1,
                Err(r) => {
                    e[1] += 1;
                    *whys.entry(format!("march {key} {:?}", r.why)).or_default() += 1;
                }
            }
        } else {
            e[1] += 1;
            *whys.entry(format!("march {key} none")).or_default() += 1;
        }
        match forge_geom::meet::intersect(&a, &b, region)
            .as_ref()
            .and_then(chains)
        {
            Some(cs) => match certify(&a, &b, region, &cs) {
                Ok(()) => e[2] += 1,
                Err(r) => {
                    e[3] += 1;
                    if r.why == Why::Twice && std::env::var("SHOW").is_ok() {
                        println!(
                            "  {key}: march {:?} curves, closed form {} curves; a {a:?}\n  b {b:?}",
                            marched.as_ref().map(Vec::len),
                            cs.len()
                        );
                        for c in &cs {
                            let hs: Vec<f64> = c.pts.windows(2).map(|w| w[0].distance(w[1])).collect();
                            let (lo, hi) = hs
                                .iter()
                                .fold((f64::MAX, 0.0f64), |m, &h| (m.0.min(h), m.1.max(h)));
                            println!(
                                "    {} pts closed {} h {lo:.2e}..{hi:.2e} close {:.2e} refusal at {:?}",
                                c.pts.len(),
                                c.closed,
                                c.pts[0].distance(c.pts[c.pts.len() - 1]),
                                r.region.center()
                            );
                        }
                    }
                    if r.why != Why::OutOfScope {
                        *whys.entry(format!("closed {key} {:?}", r.why)).or_default() += 1;
                    }
                }
            },
            None => e[4] += 1,
        }
    }
    println!("pair                march ok/refused   closed form ok/refused/none");
    for (k, v) in &tally {
        println!(
            "{k:<20} {:>5} {:>5}        {:>5} {:>5} {:>5}",
            v[0], v[1], v[2], v[3], v[4]
        );
    }
    for (k, v) in &whys {
        println!("  {k}: {v}");
    }
    println!("march {t_march:.2} s, certificate {t_cert:.2} s");
}
