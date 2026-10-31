//! The one-pass n-ary regen against the feature-by-feature one: time, validity, and whether
//! the two solids agree (face count, meshed volume).
use forge_mesh::{Quality, tessellate};
use forge_model::{Part, suite, torture};
use std::time::Instant;

fn best_of(
    k: usize,
    f: impl Fn() -> Result<forge_topo::Body, forge_model::RegenError>,
) -> (f64, Result<forge_topo::Body, String>) {
    let mut best = f64::INFINITY;
    let mut out = None;
    for _ in 0..k {
        let t = Instant::now();
        let r = f();
        best = best.min(t.elapsed().as_secs_f64() * 1e3);
        out = Some(r);
    }
    (best, out.expect("ran").map_err(|e| format!("{e:?}")))
}

fn volume(b: &forge_topo::Body) -> Option<f64> {
    let chord = std::env::var("CHORD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.05);
    let q = Quality {
        chord,
        max_angle: 0.5,
    };
    tessellate(b, &q).ok().map(|m| m.volume())
}

fn verdict(r: &Result<forge_topo::Body, String>) -> String {
    match r {
        Ok(b) => {
            let p = forge_topo::check(b);
            if p.is_empty() {
                format!("ok {}f", b.face_ids().count())
            } else {
                format!("INVALID {}p {:?}", p.len(), p.first())
            }
        }
        Err(e) => format!("FAIL {}", e.chars().take(90).collect::<String>()),
    }
}

fn main() {
    let threads: usize = std::env::var("THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, |n| n.get()));
    let seeds: u64 = std::env::var("FUZZ_SEEDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);
    let only = std::env::var("ONLY").ok();
    let mut parts: Vec<(String, Part)> = suite::parts()
        .into_iter()
        .map(|(n, p)| (n.to_string(), p))
        .collect();
    parts.push(("torture".into(), torture::part(221)));
    for s in 1..=seeds {
        parts.push((format!("fuzz{s}"), suite::fuzz(s, 80)));
    }
    println!(
        "{:<10} {:>9} {:>9} {:>9}  {:<24} {:<24} volume",
        "part", "seq ms", "nary1 ms", "naryN ms", "seq", "nary"
    );
    let (mut agree, mut total) = (0, 0);
    for (name, part) in parts {
        if only.as_deref().is_some_and(|o| o != name) {
            continue;
        }
        total += 1;
        let (ts, seq) = best_of(3, || part.regenerate());
        let (t1, _) = best_of(1, || part.regenerate_nary(1));
        let (tn, nary) = best_of(3, || part.regenerate_nary(threads));
        let vol = match (&seq, &nary) {
            (Ok(a), Ok(b)) => match (volume(a), volume(b)) {
                (Some(x), Some(y)) => {
                    let rel = (x - y).abs() / x.abs().max(1e-9);
                    if rel < 1e-3 && a.face_ids().count() == b.face_ids().count() {
                        agree += 1;
                    }
                    format!("{x:.1} vs {y:.1} ({rel:.1e})")
                }
                _ => "mesh failed".into(),
            },
            _ => "-".into(),
        };
        println!(
            "{name:<10} {ts:>9.2} {t1:>9.2} {tn:>9.2}  {:<24} {:<24} {vol}",
            verdict(&seq),
            verdict(&nary)
        );
        if std::env::var_os("NARY_PHASES").is_some() {
            let _ = part.regenerate_nary(threads);
            let p = forge_bool::nary_phase_times();
            println!(
                "    phases ms: intersect {:.2} triple {:.2} pieces {:.2} split {:.2} classify {:.2} build {:.2}",
                p[0] as f64 / 1e6,
                p[1] as f64 / 1e6,
                p[2] as f64 / 1e6,
                p[3] as f64 / 1e6,
                p[4] as f64 / 1e6,
                p[5] as f64 / 1e6
            );
        }
    }
    println!("agree (same faces, volume within 0.1%): {agree}/{total}  threads {threads}");
}
