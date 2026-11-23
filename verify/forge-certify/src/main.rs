//! `cargo run -p forge-certify --release --bin certify [-- SOURCE..] [--seeds N] [--suite N]
//! [--lang N] [--frames x|xyz] [--time]`
//!
//! Every boolean the kernel makes while regenerating the cases of the chosen sources (the
//! forge-exact fuzz corpus: `families`, `more`, `plane_plane`, `notched` with its 400 ulp stages,
//! `suite` (forge_model::suite::fuzz, `--suite N` seeds, default FUZZ_SEEDS or 150), `lang`)
//! certified and handed to the proven checker: totals accepted / rejected / out of scope, each
//! rejection with its case and reason, the out-of-scope reasons, and with `--time` the checker's
//! time against the kernel's per boolean.

use forge_certify::{Checker, Verdict, dump, kernel_time};
use forge_exact_fuzz::{Case, guarded};
use forge_model::{History, Part};
use std::collections::BTreeMap;

const FAMILIES: [&str; 6] = [
    "near_tolerance",
    "coincident_walls",
    "coplanar_stack",
    "through_vertex",
    "exact_patterns",
    "remove_all",
];

fn flat_kinds() -> u64 {
    let names = ["extrude", "pattern", "workplane", "param", "sketch"];
    forge_lang::fuzz::MAKERS
        .iter()
        .enumerate()
        .filter(|(_, m)| names.contains(&m.name))
        .fold(0u64, |k, (i, _)| k | 1 << i)
}

const CURVED: [&str; 12] = [
    "circle(", "arc(", "rounded", "revolve(", "ball(", "hole(", "fillet(", "chamfer(", "slot(", "spline(",
    "ellipse(", "coil(",
];

fn flat_src(src: &str) -> String {
    let kept: Vec<String> = forge_lang::fuzz::statements(src)
        .into_iter()
        .filter(|s| !CURVED.iter().any(|c| s.contains(c)))
        .collect();
    kept.join("\n") + "\n"
}

fn case(name: String, part: Part) -> Case {
    let part = forge_model::adversarial::canonical(&part);
    Case {
        source: String::new(),
        name,
        part,
    }
}

fn cases(sources: &[String], seeds: u64, suite: u64, lang: u64) -> Vec<(Case, Vec<u64>)> {
    let on = |s: &str| {
        if sources.is_empty() {
            ["families", "plane_plane", "notched", "suite", "lang"].contains(&s)
        } else {
            sources.iter().any(|x| x == s)
        }
    };
    let mut out = Vec::new();
    let fams = forge_model::adversarial::families();
    if on("families") || on("more") {
        for (f, g) in &fams {
            let core = FAMILIES.contains(f);
            if (core && !on("families")) || (!core && !on("more")) {
                continue;
            }
            for s in 0..seeds {
                let Ok(raw) = guarded(|| Ok(forge_model::adversarial::canonical(&g(s)))) else {
                    continue;
                };
                out.push((
                    case(format!("{f}/{s}"), forge_exact_fuzz::flat_part(&raw)),
                    vec![],
                ));
            }
        }
    }
    if on("plane_plane") {
        for (name, part) in forge_model::adversarial::degenerate_parts() {
            if name.starts_with("plane_plane/") {
                out.push((case(name, part), vec![]));
            }
        }
    }
    if on("notched") {
        out.push((
            case("notched".into(), forge_exact_fuzz::notched()),
            (0..400).collect(),
        ));
    }
    if on("suite") {
        for s in 0..suite {
            out.push((
                case(
                    format!("suite/{s}"),
                    forge_exact_fuzz::flat_part(&forge_model::suite::fuzz(s, 8)),
                ),
                vec![],
            ));
        }
    }
    if on("lang") {
        for s in 1..=lang {
            let Ok(run) = guarded(|| Ok(forge_lang::fuzz::run_kinds(s, 8, flat_kinds()))) else {
                continue;
            };
            let src = flat_src(&run.src);
            if let Ok(b) = guarded(|| Ok(forge_lang::fuzz::build(&src))) {
                out.push((case(format!("lang/{s}"), b.part()), vec![]));
            }
        }
    }
    out
}

/// Every boolean of the case's regen (and of the last stage moved by each ulp seed).
fn records(c: &Case, ulps: &[u64]) -> Vec<(String, forge_bool::certify::Record)> {
    let mut out = Vec::new();
    forge_bool::certify::record(true);
    let _ = guarded(|| c.part.regenerate().map_err(|e| format!("{e:?}")));
    out.extend(
        forge_bool::certify::take()
            .into_iter()
            .map(|r| ("seq".to_string(), r)),
    );
    let h = History::new(&c.part.features, None);
    let k = c.part.features.len().saturating_sub(1);
    for &s in ulps {
        forge_bool::certify::record(false);
        let stage = guarded(|| {
            let mut body = None;
            for i in 0..k {
                body = forge_model::step(h, i, body).map_err(|e| format!("{e:?}"))?;
            }
            body.ok_or_else(|| "no stage".to_string())
        });
        let Ok(stage) = stage else { continue };
        let moved = forge_proof::props::nudged(&stage, s);
        forge_bool::certify::record(true);
        let _ = guarded(|| {
            forge_model::step(h, k, Some(moved))
                .map_err(|e| format!("{e:?}"))
                .map(|_| ())
        });
        out.extend(
            forge_bool::certify::take()
                .into_iter()
                .map(|r| (format!("ulp {s}"), r)),
        );
    }
    forge_bool::certify::record(false);
    out
}

fn main() {
    let mut sources = Vec::new();
    let mut seeds = 24;
    let mut suite: u64 = std::env::var("FUZZ_SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(150);
    let mut lang = 0;
    let mut frames = "xyz".to_string();
    let mut time = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--seeds" => seeds = args.next().and_then(|s| s.parse().ok()).expect("--seeds N"),
            "--suite" => suite = args.next().and_then(|s| s.parse().ok()).expect("--suite N"),
            "--lang" => lang = args.next().and_then(|s| s.parse().ok()).expect("--lang N"),
            "--frames" => frames = args.next().expect("--frames x|xyz"),
            "--time" => time = true,
            s => sources.push(s.to_string()),
        }
    }
    if lang == 0 {
        lang = seeds * 8;
    }
    let mut checker = Checker::start();
    let mut totals: BTreeMap<&str, usize> = BTreeMap::new();
    let mut rejected: Vec<(String, String)> = Vec::new();
    let mut scope: BTreeMap<String, usize> = BTreeMap::new();
    let mut ratios: Vec<(f64, f64, f64)> = Vec::new();
    for (c, ulps) in cases(&sources, seeds, suite, lang) {
        let t = std::time::Instant::now();
        let recs = records(&c, &ulps);
        for (i, (path, r)) in recs.iter().enumerate() {
            let what = format!("{} [{path}] boolean {i} {}", c.name, forge_certify::op_name(r.op));
            match dump(r, &frames) {
                Err(e) => {
                    *totals.entry("out of scope").or_default() += 1;
                    *scope.entry(e).or_default() += 1;
                }
                Ok(line) => {
                    if let Ok(dir) = std::env::var("FORGE_CERTIFY_DUMP_ALL") {
                        let name: String = what
                            .chars()
                            .map(|c| if c.is_alphanumeric() { c } else { '_' })
                            .collect();
                        let _ = std::fs::write(format!("{dir}/{name}.json"), &line);
                    }
                    let t0 = std::time::Instant::now();
                    let v = checker.ask(&line);
                    let tc = t0.elapsed().as_secs_f64();
                    if time {
                        let tk = kernel_time(r, 3).as_secs_f64();
                        ratios.push((tc, tk, line.len() as f64));
                    }
                    match v {
                        Verdict::Accepted => *totals.entry("accepted").or_default() += 1,
                        Verdict::Rejected(why) => {
                            if let Ok(dir) = std::env::var("FORGE_CERTIFY_DUMP") {
                                let name: String = what
                                    .chars()
                                    .map(|c| if c.is_alphanumeric() { c } else { '_' })
                                    .collect();
                                let _ = std::fs::write(format!("{dir}/{name}.json"), &line);
                            }
                            *totals.entry("rejected").or_default() += 1;
                            rejected.push((what, why));
                        }
                        Verdict::Scope(why) => {
                            *totals.entry("out of scope").or_default() += 1;
                            *scope.entry(why).or_default() += 1;
                        }
                    }
                }
            }
        }
        eprintln!(
            "{} ({} booleans, {:.1}s)",
            c.name,
            recs.len(),
            t.elapsed().as_secs_f64()
        );
    }
    println!("== totals");
    for (k, n) in &totals {
        println!("{k:14} {n}");
    }
    println!("\n== rejected");
    for (w, why) in &rejected {
        println!("{w}: {why}");
    }
    println!("\n== out of scope");
    for (s, n) in &scope {
        println!("{n:6} {s}");
    }
    if time && !ratios.is_empty() {
        let mut r: Vec<f64> = ratios.iter().map(|(c, k, _)| c / k.max(1e-9)).collect();
        r.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let tc: f64 = ratios.iter().map(|x| x.0).sum();
        let tk: f64 = ratios.iter().map(|x| x.1).sum();
        let q = |p: f64| r[((r.len() - 1) as f64 * p) as usize];
        println!("\n== cost ({} booleans checked)", ratios.len());
        println!(
            "checker {:.3}s, kernel {:.3}s, total ratio {:.0}x",
            tc,
            tk,
            tc / tk.max(1e-9)
        );
        println!(
            "per boolean checker/kernel: median {:.0}x, p90 {:.0}x, max {:.0}x",
            q(0.5),
            q(0.9),
            q(1.0)
        );
        let mc: f64 = ratios.iter().map(|x| x.0).fold(0.0, f64::max);
        println!("slowest check {:.3}s", mc);
    }
}
