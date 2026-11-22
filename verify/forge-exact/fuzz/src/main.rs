//! `cargo run -p forge-exact-fuzz --release --bin exact-fuzz [-- SOURCE.. ] [--seeds N] [--shrink]`
//!
//! Every flat-face case of the chosen sources against the exact reference: totals per kernel
//! path (agree, kernel wrong, kernel refused, reference ambiguous, out of scope), then each
//! disagreement, and with --shrink one minimal repro per distinct kind.
//!
//! Sources: `families` (the reliability scoreboard's block families), `plane_plane` (the
//! degenerate plane–plane pairs), `notched` (the near-coincident box under a notched top,
//! 94fc9fb6, its stage moved by ulps), `suite` (forge_model::suite::fuzz histories), `lang`
//! (forge_lang::fuzz programs drawn from extrude, pattern, sketch, param and work planes).

use forge_exact_fuzz::{Case, Class, Judged, Reference, judge};
use forge_model::Part;
use std::collections::BTreeMap;

const FAMILIES: [&str; 6] = [
    "near_tolerance",
    "coincident_walls",
    "coplanar_stack",
    "through_vertex",
    "exact_patterns",
    "remove_all",
];

/// forge_lang::fuzz::MAKERS bits: extrude, pattern, workplane, param, sketch.
fn flat_kinds() -> u64 {
    let names = ["extrude", "pattern", "workplane", "param", "sketch"];
    forge_lang::fuzz::MAKERS
        .iter()
        .enumerate()
        .filter(|(_, m)| names.contains(&m.name))
        .fold(0u64, |k, (i, _)| k | 1 << i)
}

fn family(name: &str) -> forge_model::adversarial::Family {
    forge_model::adversarial::families()
        .into_iter()
        .find(|(f, _)| *f == name)
        .unwrap_or_else(|| panic!("no family {name}"))
        .1
}

fn adv_case(name: String, part: Part) -> Case {
    let part = forge_model::adversarial::canonical(&part);
    Case {
        source: forge_model::adversarial::literal(&part),
        name,
        part,
    }
}

/// The near-coincident part: adv near_tolerance 2, its first three features.
pub fn notched() -> Part {
    let mut part = forge_model::adversarial::canonical(&family("near_tolerance")(2));
    part.features.truncate(3);
    part
}

/// Statements that make curved geometry (by their text): dropped from a language program.
const CURVED: [&str; 12] = [
    "circle(", "arc(", "rounded", "revolve(", "ball(", "hole(", "fillet(", "chamfer(", "slot(", "spline(",
    "ellipse(", "coil(",
];

/// The flat-face sub-program: statements making curved geometry dropped.
fn flat_src(src: &str) -> String {
    let kept: Vec<String> = forge_lang::fuzz::statements(src)
        .into_iter()
        .filter(|s| !CURVED.iter().any(|c| s.contains(c)))
        .collect();
    kept.join("\n") + "\n"
}

fn lang_case(seed: u64, len: usize) -> Option<Case> {
    let run = forge_exact_fuzz::guarded(|| Ok(forge_lang::fuzz::run_kinds(seed, len, flat_kinds()))).ok()?;
    let src = flat_src(&run.src);
    let built = forge_exact_fuzz::guarded(|| Ok(forge_lang::fuzz::build(&src))).ok()?;
    Some(Case {
        name: format!("lang/{seed}"),
        part: built.part(),
        source: src,
    })
}

fn cases(sources: &[String], seeds: u64) -> Vec<(Case, Vec<u64>)> {
    let on = |s: &str| sources.is_empty() || sources.iter().any(|x| x == s);
    let mut out = Vec::new();
    if on("families") {
        for f in FAMILIES {
            let g = family(f);
            for s in 0..seeds {
                let part = forge_exact_fuzz::flat_part(&forge_model::adversarial::canonical(&g(s)));
                out.push((adv_case(format!("{f}/{s} blocks"), part), vec![]));
            }
        }
    }
    if on("plane_plane") {
        for (name, part) in forge_model::adversarial::degenerate_parts() {
            if name.starts_with("plane_plane/") {
                out.push((adv_case(name, part), vec![]));
            }
        }
    }
    if on("notched") {
        let ulps: Vec<u64> = (0..400).collect();
        out.push((
            adv_case("notched (near_tolerance/2, 3 features)".into(), notched()),
            ulps,
        ));
    }
    if on("suite") {
        for s in 0..seeds {
            let part = forge_exact_fuzz::flat_part(&forge_model::suite::fuzz(s, 8));
            out.push((adv_case(format!("suite/{s} flat"), part), vec![]));
        }
    }
    if on("lang") {
        for s in 1..=seeds * 8 {
            if let Some(c) = lang_case(s, 8) {
                out.push((c, vec![]));
            }
        }
    }
    out
}

fn main() {
    let mut sources = Vec::new();
    let mut seeds = 24;
    let mut shrink = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--seeds" => seeds = args.next().and_then(|s| s.parse().ok()).expect("--seeds N"),
            "--shrink" => shrink = true,
            s => sources.push(s.to_string()),
        }
    }
    let mut reference = Reference::start();
    let mut all: Vec<(Judged, usize)> = Vec::new();
    let cs = cases(&sources, seeds);
    for (i, (case, ulps)) in cs.iter().enumerate() {
        let t = std::time::Instant::now();
        for j in judge(&mut reference, case, ulps) {
            all.push((j, i));
        }
        eprintln!("{} ({:.1}s)", case.name, t.elapsed().as_secs_f64());
    }
    // totals per path kind
    let mut totals: BTreeMap<(String, &'static str), usize> = BTreeMap::new();
    for (j, _) in &all {
        let path = if j.path.starts_with("ulp") { "ulp" } else { j.path };
        *totals.entry((path.to_string(), j.class.key())).or_default() += 1;
    }
    println!("== totals (path, class: count)");
    for ((p, c), n) in &totals {
        println!("{p:12} {c:22} {n}");
    }
    println!("\n== disagreements");
    let mut kinds: BTreeMap<String, Vec<&(Judged, usize)>> = BTreeMap::new();
    for j in &all {
        if matches!(j.0.class, Class::Wrong(_) | Class::Refused(_)) {
            println!("{} [{}]: {}", j.0.name, j.0.path, j.0.class.detail());
            kinds.entry(j.0.class.kind()).or_default().push(j);
        }
    }
    println!("\n== out of scope");
    let mut scope: BTreeMap<String, usize> = BTreeMap::new();
    for (j, _) in &all {
        if let Class::Scope(s) = &j.class {
            *scope
                .entry(s.split(':').next_back().unwrap_or("").trim().to_string())
                .or_default() += 1;
        }
    }
    for (s, n) in scope {
        println!("{n:5} {s}");
    }
    println!("\n== kinds");
    for (k, js) in &kinds {
        println!("{k}: {} ({} first)", js.len(), js[0].0.name);
    }
    if shrink {
        println!("\n== shrunk repros");
        for (k, js) in &kinds {
            let (j, i) = js[0];
            let (case, _) = &cs[*i];
            let path = j.path;
            let small = forge_exact_fuzz::shrink::shrink(&mut reference, case, path, &j.class);
            println!(
                "--- {k}\n{} [{path}]: {}\n{}",
                case.name,
                small.1.detail(),
                small.0
            );
        }
    }
}
