//! Exact mass vs mesh timing per body (copy to crates/forge-agent/examples/ to run; see README).
//! masstime [names..]: gate and bench parts by default, `fillets` for the fillet corpus.
//! MASSONLY=1 skips the mesh; COUNT=r regenerates, then makes r exact calls (MESH=1: meshes)
//! for instruction counting; REGEN=1 times the regeneration instead.
use forge_mesh::{Quality, tessellate};
use forge_model::{fillets, suite, torture};
use forge_topo::Body;

use std::time::Instant;

fn main() {
    let mut names: Vec<String> = std::env::args().skip(1).collect();
    if names.is_empty() {
        names = "torture plate shaft housing coplanar patterns mixed hole-grid tour upto upto-flat sweep coil loft thread text gearbox volute crank valve heatsink quadarm conrod jack frame manifold bracket insert clamp hub guard"
            .split(' ')
            .map(String::from)
            .collect();
    }
    if names == ["fillets"] {
        names = fillets::cases().into_iter().chain(fillets::variable_cases()).map(|c| c.0.to_string()).collect();
    }
    for name in &names {
        let n = name.as_str();
        let bodies: Vec<(String, Body)> = if forge_agent::bench::NAMES.contains(&n) {
            match forge_agent::bench::part(n).regenerate_solids() {
                Ok(s) => s.into_iter().enumerate().map(|(i, s)| (format!("{n}#{i}"), s.body)).collect(),
                Err(e) => {
                    println!("{n:24} regen failed {e:?}");
                    continue;
                }
            }
        } else {
            let part = match n {
                "torture" => torture::part(221),
                n => suite::part(n).unwrap_or_else(|| {
                    fillets::cases()
                        .into_iter()
                        .chain(fillets::variable_cases())
                        .find(|c| c.0 == n)
                        .expect("part")
                        .1
                }),
            };
            if std::env::var("REGEN").is_ok() {
                // the regeneration's time (booleans, blends), the least of three
                let mut best = f64::INFINITY;
                for _ in 0..3 {
                    let t = Instant::now();
                    std::hint::black_box(part.regenerate().ok());
                    best = best.min(t.elapsed().as_secs_f64() * 1e6);
                }
                println!("{n:24} regen {best:10.0} us");
                continue;
            }
            match part.regenerate() {
                Ok(b) => vec![(n.to_string(), b)],
                Err(e) => {
                    println!("{n:24} regen failed {e:?}");
                    continue;
                }
            }
        };
        for (n, b) in &bodies {
        let n = n.as_str();
        if let Ok(r) = std::env::var("COUNT") {
            // instruction counting: regenerate, then r exact mass calls (or meshes with MESH)
            for _ in 0..r.parse::<usize>().expect("reps") {
                if std::env::var("MESH").is_ok() {
                    std::hint::black_box(tessellate(b, &Quality::default()).ok());
                } else {
                    std::hint::black_box(forge_topo::mass::mass(b).ok());
                }
            }
            continue;
        }
        let t = Instant::now();
        let m = forge_topo::mass::mass(b);
        // the least of several single calls' on-CPU times: other loads only ever add
        let best = |f: &dyn Fn()| {
            let (mut best, mut spent, mut k) = (f64::INFINITY, 0.0, 0);
            while k < 3 || (spent < 0.4 && k < 200) {
                let t = Instant::now();
                f();
                let dt = t.elapsed().as_secs_f64();
                best = best.min(dt);
                spent += dt;
                k += 1;
            }
            best * 1e6
        };
        let _ = t;
        let ex = best(&|| {
            std::hint::black_box(forge_topo::mass::mass(b).ok());
        });
        let q = Quality::default();
        let me = if std::env::var("MASSONLY").is_ok() {
            1.0
        } else {
            best(&|| {
                std::hint::black_box(tessellate(b, &q).ok());
            })
        };
        match m {
            Ok(m) => println!(
                "{n:24} exact {ex:10.0} us  mesh {me:10.0} us  ratio {:5.2}  vol {:.12e} err {:.1e} meshvol {:.6e}",
                ex / me,
                m.volume,
                m.err[0] / m.volume.abs(),
                tessellate(b, &Quality { chord: 1e-3, ..Quality::default() }).map_or(0.0, |m| m.volume())
            ),
            Err(e) => println!("{n:24} exact ERR {e}  mesh {me:10.0} us"),
        }
    }
    }
}
