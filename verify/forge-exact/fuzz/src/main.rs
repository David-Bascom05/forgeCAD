//! `cargo run -p forge-exact-fuzz --release --bin exact-fuzz [-- SOURCE.. ] [--seeds N] [--shrink]`
//!
//! Every flat-face case of the chosen sources against the exact reference: totals per kernel
//! path (agree, kernel wrong, kernel refused, reference ambiguous, out of scope), then each
//! disagreement, and with --shrink one minimal repro per distinct kind.
//!
//! Sources: `families` (the reliability scoreboard's block families), `plane_plane` (the
//! degenerate plane–plane pairs), `notched` (the near-coincident box under a notched top,
//! 94fc9fb6, its stage moved by ulps), `suite` (forge_model::suite::fuzz histories), `lang`
//! (forge_lang::fuzz programs drawn from extrude, pattern, sketch, param and work planes),
//! `more` (every other scoreboard family's flat sub-history, where it combines two solids),
//! `boxes` (the box-box family: 13³ Allen patterns × 3 ops, one boolean each), `band` (the
//! box-box family's tolerance band: every pattern with coincident ends, B's coincident ends on
//! some axes moved by δ·LINEAR; `--band AXES:δ,δ..` picks sweeps, default the bands the coverage
//! worker found and controls either side). `--lang N`: language programs (default 8 × seeds),
//! `--lang-from N` the first.
//! `--only NAME` (repeatable): only the part cases of that name.
//! FORGE_EXACT_RULE=snap|strict picks the reference's rule.

use forge_exact_fuzz::{Case, Class, Judged, Reference, judge, pairs};

/// A case's family: its name up to the seed or pattern.
fn family_of(name: &str) -> String {
    if let Some(r) = name.strip_prefix("band ") {
        return format!("band {}", r.split('/').next().unwrap_or(""));
    }
    if name.starts_with("plane_plane/") {
        return "plane_plane".into();
    }
    if name.starts_with("notched") {
        return "notched".into();
    }
    name.split('/').next().unwrap_or(name).to_string()
}
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

/// The default sources: everything but the box family and its band (`boxes`, `band`).
const DEFAULT: [&str; 5] = ["families", "plane_plane", "notched", "suite", "lang"];

fn on(sources: &[String], s: &str) -> bool {
    if sources.is_empty() {
        DEFAULT.contains(&s)
    } else {
        sources.iter().any(|x| x == s)
    }
}

/// The band sweeps (axes, δ in LINEARs): (LINEAR/√2, LINEAR] on three and on two axes,
/// (LINEAR, 4.88 LINEAR) on one, each side; and 0.5 and 5 as controls.
fn default_bands() -> Vec<(Vec<usize>, f64)> {
    let mut out = Vec::new();
    for d in [0.5, 0.75, 0.9, 1.0, -0.75, -0.9, -1.0] {
        out.push((vec![0, 1, 2], d));
    }
    for d in [0.75, 0.9, 1.0, -0.9] {
        out.push((vec![0, 1], d));
    }
    for d in [1.5, 2.5, 4.0, 4.8, 5.0, -1.5, -2.5, -4.0] {
        out.push((vec![0], d));
    }
    for d in [1.5, 2.5, 4.0, 4.8] {
        out.push((vec![2], d));
    }
    out
}

fn cases(sources: &[String], seeds: u64, lang: (u64, u64)) -> Vec<(Case, Vec<u64>)> {
    let on = |s: &str| on(sources, s);
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
    if on("more") {
        // every other scoreboard family, its flat sub-history, where that combines two solids
        for (f, g) in forge_model::adversarial::families() {
            if FAMILIES.contains(&f) {
                continue;
            }
            for s in 0..seeds {
                let Ok(raw) = forge_exact_fuzz::guarded(|| Ok(forge_model::adversarial::canonical(&g(s))))
                else {
                    continue;
                };
                let part = forge_exact_fuzz::flat_part(&raw);
                let solids = part
                    .features
                    .iter()
                    .filter(|x| {
                        matches!(
                            x,
                            forge_model::Feature::Extrude { .. } | forge_model::Feature::Pattern { .. }
                        )
                    })
                    .count();
                if solids >= 2 {
                    out.push((adv_case(format!("{f}/{s} flat"), part), vec![]));
                }
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
            adv_case(
                "notched (near_tolerance/2, 3 features)".into(),
                forge_exact_fuzz::notched(),
            ),
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
        for s in lang.0..=lang.1 {
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
    let mut lang = None;
    let mut lang_from = 1;
    let mut only: Vec<String> = Vec::new();
    let mut bands: Vec<(Vec<usize>, f64)> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--seeds" => seeds = args.next().and_then(|s| s.parse().ok()).expect("--seeds N"),
            "--lang-from" => lang_from = args.next().and_then(|s| s.parse().ok()).expect("--lang-from N"),
            "--only" => only.push(args.next().expect("--only NAME")),
            "--lang" => lang = Some(args.next().and_then(|s| s.parse().ok()).expect("--lang N")),
            "--band" => {
                let spec = args.next().expect("--band AXES:δ,δ..");
                let (ax, ds) = spec.split_once(':').expect("--band AXES:δ,δ..");
                let axes: Vec<usize> = ax
                    .chars()
                    .filter_map(|c| c.to_digit(10))
                    .map(|d| d as usize)
                    .collect();
                for d in ds.split(',') {
                    bands.push((axes.clone(), d.parse().expect("δ")));
                }
            }
            "--shrink" => shrink = true,
            s => sources.push(s.to_string()),
        }
    }
    let mut reference = Reference::start();
    let mut all: Vec<(Judged, usize)> = Vec::new();
    let mut cs = cases(&sources, seeds, (lang_from, lang.unwrap_or(seeds * 8)));
    if !only.is_empty() {
        cs.retain(|(c, _)| only.contains(&c.name));
    }
    for (i, (case, ulps)) in cs.iter().enumerate() {
        let t = std::time::Instant::now();
        for j in judge(&mut reference, case, ulps) {
            all.push((j, i));
        }
        eprintln!("{} ({:.1}s)", case.name, t.elapsed().as_secs_f64());
    }
    // two-body cases: index past the part cases, never shrunk (two boxes are the repro)
    let mut pairs = Vec::new();
    if on(&sources, "boxes") {
        pairs.extend(pairs::box_family());
    }
    if on(&sources, "band") {
        if bands.is_empty() {
            bands = default_bands();
        }
        for (axes, d) in &bands {
            pairs.extend(pairs::band(axes, *d));
        }
    }
    let t = std::time::Instant::now();
    for (i, p) in pairs.iter().enumerate() {
        all.push((pairs::judge_pair(&mut reference, p), usize::MAX));
        if i % 500 == 499 {
            eprintln!("{} pairs ({:.1}s)", i + 1, t.elapsed().as_secs_f64());
        }
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
    // totals per family (the name before its seed or pattern) and path
    let mut fam: BTreeMap<(String, String), BTreeMap<&'static str, usize>> = BTreeMap::new();
    for (j, _) in &all {
        let f = family_of(&j.name);
        let path = if j.path.starts_with("ulp") { "ulp" } else { j.path };
        *fam.entry((f, path.to_string()))
            .or_default()
            .entry(j.class.key())
            .or_default() += 1;
    }
    println!("\n== per family (family, path: agree / unmerged / wrong / refused / ambiguous / out of scope)");
    for ((f, p), m) in &fam {
        let g = |k: &str| m.get(k).copied().unwrap_or(0);
        println!(
            "{f:32} {p:10} {:5} {:5} {:5} {:5} {:5} {:5}",
            g("agree"),
            g("kernel unmerged"),
            g("kernel wrong"),
            g("kernel refused"),
            g("reference ambiguous"),
            g("out of scope")
        );
    }
    println!("\n== disagreements");
    let mut kinds: BTreeMap<String, Vec<&(Judged, usize)>> = BTreeMap::new();
    for j in &all {
        if matches!(
            j.0.class,
            Class::Wrong(_) | Class::Refused(_) | Class::Unmerged(_)
        ) {
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
            if *i == usize::MAX {
                println!(
                    "--- {k}\n{} [{}]: {} (two boxes: already minimal)",
                    j.name,
                    j.path,
                    j.class.detail()
                );
                continue;
            }
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
