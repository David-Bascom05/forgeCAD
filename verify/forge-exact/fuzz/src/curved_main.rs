//! `cargo run -p forge-exact-fuzz --release --bin exact-curved [-- SOURCE..] [--seeds N]
//! [--lang N] [--rule strict|snap] [--threads N] [--only NAME] [--shard K/N] [--shrink] [--oracle]`
//!
//! The kernel's booleans of solids bounded by planes, cylinders, cones and spheres against the
//! exact curved reference: totals per source and path, every disagreement, and with --shrink one
//! minimal repro per distinct kind.
//!
//! Sources: `degen` (the degenerate pairs of the reliability scoreboard whose surfaces are
//! planes, cylinders, cones and spheres, with Add, Cut and Keep), `families` (every scoreboard
//! family, `--seeds` each, where its tools are quadric solids), `lang` (forge_lang::fuzz
//! programs drawn from extrude, revolve, ball, hole, pattern, workplane, param and sketch),
//! `band` (each degenerate pair with its tool moved by δ·LINEAR along x, y and z), `torus` (the
//! degenerate pairs with a torus, on the polynomial path) and `torusband` (those moved as `band`), and `fg:PATH` (a .fg file).
//!
//! With `--oracle`, each case also goes to the decide-once oracle (the curved ledger `qledgerB`,
//! proven exact on certified inputs by `curved_exact` in verify/forge-curved): its probe
//! memberships against the reference's, in an `== oracle` section. It takes minutes a case
//! (FORGE_EXACT_ORACLE_TIMEOUT, FORGE_EXACT_ORACLE_CARRIERS).

use forge_exact_fuzz::curved::{OClass, judge_curved, judge_curved_path, oracle_class, tool_moved};
use forge_exact_fuzz::{Class, Reference, short};
use forge_math::{Vec3, tol::LINEAR};
use forge_model::Part;
use std::collections::BTreeMap;

struct Case {
    name: String,
    source: String,
    part: Part,
    /// a forge_lang program's feature ids (its names refer to faces by them)
    ids: Option<Vec<u64>>,
}

fn quadric_pair(name: &str) -> bool {
    !name.contains("torus") && !name.starts_with("plane_plane")
}

fn lang_kinds() -> u64 {
    let names = [
        "extrude",
        "revolve",
        "ball",
        "hole",
        "pattern",
        "workplane",
        "param",
        "sketch",
    ];
    forge_lang::fuzz::MAKERS
        .iter()
        .enumerate()
        .filter(|(_, m)| names.contains(&m.name))
        .fold(0u64, |k, (i, _)| k | 1 << i)
}

fn lang_case(seed: u64, len: usize) -> Option<Case> {
    let run = forge_exact_fuzz::guarded(|| Ok(forge_lang::fuzz::run_kinds(seed, len, lang_kinds()))).ok()?;
    let built = forge_exact_fuzz::guarded(|| Ok(forge_lang::fuzz::build(&run.src))).ok()?;
    let (part, ids) = forge_exact_fuzz::curved::curved_part_ids(&built.part(), Some(&built.ids));
    Some(Case {
        name: format!("lang/{seed}"),
        part,
        ids,
        source: run.src,
    })
}

fn adv(name: String, part: Part) -> Case {
    let part = forge_model::adversarial::canonical(&part);
    Case {
        source: forge_model::adversarial::literal(&part),
        name,
        part,
        ids: None,
    }
}

/// The band: δ in LINEARs.
const BAND: [f64; 3] = [0.9, -0.9, 3.0];

fn cases(sources: &[String], seeds: u64, lang: u64) -> Vec<Case> {
    let only_fg = !sources.is_empty() && sources.iter().all(|s| s.starts_with("fg:"));
    let on = |s: &str| {
        !only_fg && (sources.is_empty() && s != "band" && s != "torusband" || sources.iter().any(|x| x == s))
    };
    let mut out = Vec::new();
    if on("degen") || on("band") || on("torus") || on("torusband") {
        for (name, part) in forge_model::adversarial::degenerate_parts() {
            if name.starts_with("plane_plane") {
                continue;
            }
            let (whole, band) = if quadric_pair(&name) {
                ("degen", "band")
            } else {
                ("torus", "torusband")
            };
            if on(whole) {
                out.push(adv(name.clone(), part.clone()));
            }
            if on(band) {
                for (ax, axis) in [(0, Vec3::X), (1, Vec3::Y), (2, Vec3::Z)] {
                    for d in BAND {
                        if let Some(p) = tool_moved(&part, axis * (d * LINEAR)) {
                            out.push(adv(format!("band {name} {}{d}", ["x", "y", "z"][ax]), p));
                        }
                    }
                }
            }
        }
    }
    if on("families") {
        for (f, g) in forge_model::adversarial::families() {
            for s in 0..seeds {
                if let Ok(p) = forge_exact_fuzz::guarded(|| Ok(g(s))) {
                    let p = forge_exact_fuzz::curved::curved_part(&forge_model::adversarial::canonical(&p));
                    if p.features.len() >= 2 {
                        out.push(adv(format!("{f}/{s}"), p));
                    }
                }
            }
        }
    }
    // `fg:PATH`: a .fg file, built by forge_lang, its convex sub-history judged
    for src in sources.iter().filter_map(|s| s.strip_prefix("fg:")) {
        let text = std::fs::read_to_string(src).unwrap_or_else(|e| panic!("{src}: {e}"));
        match forge_exact_fuzz::guarded(|| Ok(forge_lang::fuzz::build(&text))) {
            Ok(built) => {
                let (part, ids) = forge_exact_fuzz::curved::curved_part_ids(&built.part(), Some(&built.ids));
                out.push(Case {
                    name: format!("fg/{src}"),
                    part,
                    ids,
                    source: text,
                })
            }
            Err(e) => eprintln!("{src}: {e}"),
        }
    }
    if on("lang") {
        for s in 1..=lang {
            if let Some(c) = lang_case(s, 8) {
                out.push(c);
            }
        }
    }
    out
}

fn family_of(name: &str) -> String {
    if let Some(r) = name.strip_prefix("band ") {
        return format!("band {}", r.split('/').next().unwrap_or(""));
    }
    name.split('/').next().unwrap_or(name).to_string()
}

struct Judged {
    name: String,
    path: &'static str,
    class: Class,
    case: usize,
    note: String,
    oracle: Option<OClass>,
}

fn main() {
    let mut sources = Vec::new();
    let mut seeds = 16;
    let mut lang = 400;
    let mut rule = std::env::var("FORGE_EXACT_RULE").unwrap_or_else(|_| "snap".into());
    let mut threads = 4;
    let mut only: Vec<String> = Vec::new();
    let mut shrink = false;
    let mut shard: Option<(usize, usize)> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--seeds" => seeds = args.next().and_then(|s| s.parse().ok()).expect("--seeds N"),
            "--lang" => lang = args.next().and_then(|s| s.parse().ok()).expect("--lang N"),
            "--rule" => rule = args.next().expect("--rule strict|snap"),
            "--threads" => threads = args.next().and_then(|s| s.parse().ok()).expect("--threads N"),
            "--only" => only.push(args.next().expect("--only NAME")),
            "--shrink" => shrink = true,
            "--oracle" => forge_exact_fuzz::curved::ORACLE.store(true, std::sync::atomic::Ordering::Relaxed),
            "--shard" => {
                let v = args.next().expect("--shard K/N");
                let (k, n) = v.split_once('/').expect("--shard K/N");
                shard = Some((k.parse().expect("K"), n.parse().expect("N")));
            }
            s => sources.push(s.to_string()),
        }
    }
    let mut cs = cases(&sources, seeds, lang);
    if !only.is_empty() {
        cs.retain(|c| only.contains(&c.name));
    }
    if let Some((k, n)) = shard {
        let mut i = 0;
        cs.retain(|_| {
            i += 1;
            (i - 1) % n == k
        });
    }
    eprintln!("{} cases, rule {rule}", cs.len());
    let cs = std::sync::Arc::new(cs);
    let next = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (tx, rx) = std::sync::mpsc::channel();
    let mut handles = Vec::new();
    for _ in 0..threads {
        let cs = cs.clone();
        let next = next.clone();
        let tx = tx.clone();
        let rule = rule.clone();
        handles.push(std::thread::spawn(move || {
            let mut reference = Reference::start();
            loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if i >= cs.len() {
                    break;
                }
                let c = &cs[i];
                let t = std::time::Instant::now();
                let judged = forge_exact_fuzz::curved::with_ids(c.ids.clone(), || {
                    judge_curved(&mut reference, &c.part, &rule)
                });
                for (path, class, ans) in judged {
                    let oracle = ans
                        .as_ref()
                        .filter(|_| path == "sequential")
                        .and_then(oracle_class);
                    let note = ans.and_then(|a| a.notes.first().cloned()).unwrap_or_default();
                    tx.send(Judged {
                        name: c.name.clone(),
                        path,
                        class,
                        case: i,
                        note,
                        oracle,
                    })
                    .expect("send");
                }
                eprintln!("{} ({:.1}s)", c.name, t.elapsed().as_secs_f64());
            }
        }));
    }
    drop(tx);
    let mut all: Vec<Judged> = rx.iter().collect();
    for h in handles {
        let _ = h.join();
    }
    all.sort_by(|a, b| (a.case, a.path).cmp(&(b.case, b.path)));
    let mut totals: BTreeMap<(String, &'static str), usize> = BTreeMap::new();
    for j in &all {
        *totals.entry((j.path.to_string(), j.class.key())).or_default() += 1;
    }
    println!("== totals (path, class: count), rule {rule}");
    for ((p, c), n) in &totals {
        println!("{p:12} {c:22} {n}");
    }
    let mut fam: BTreeMap<(String, String), BTreeMap<&'static str, usize>> = BTreeMap::new();
    for j in &all {
        *fam.entry((family_of(&j.name), j.path.to_string()))
            .or_default()
            .entry(j.class.key())
            .or_default() += 1;
    }
    println!("\n== per family (family, path: agree / wrong / refused / ambiguous / out of scope)");
    for ((f, p), m) in &fam {
        let g = |k: &str| m.get(k).copied().unwrap_or(0);
        println!(
            "{f:40} {p:10} {:5} {:5} {:5} {:5} {:5}",
            g("agree"),
            g("kernel wrong"),
            g("kernel refused"),
            g("reference ambiguous"),
            g("out of scope")
        );
    }
    println!("\n== disagreements");
    let mut kinds: BTreeMap<String, Vec<&Judged>> = BTreeMap::new();
    for j in &all {
        if matches!(j.class, Class::Wrong(_) | Class::Refused(_)) {
            println!("{} [{}]: {}", j.name, j.path, j.class.detail());
            kinds.entry(j.class.kind()).or_default().push(j);
        }
    }
    println!("\n== ambiguous");
    for j in &all {
        if matches!(j.class, Class::Ambiguous) && j.path == "sequential" {
            println!("{}: {}", j.name, j.note);
        }
    }
    println!("\n== out of scope");
    let mut scope: BTreeMap<String, usize> = BTreeMap::new();
    for j in &all {
        if let Class::Scope(s) = &j.class {
            *scope
                .entry(s.split(':').next_back().unwrap_or("").trim().to_string())
                .or_default() += 1;
        }
    }
    for (s, n) in scope {
        println!("{n:5} {s}");
    }
    if forge_exact_fuzz::curved::oracle_on() {
        println!("\n== oracle (the decide-once ledger against the reference, at the probes)");
        let mut tally: BTreeMap<String, usize> = BTreeMap::new();
        let mut alone = 0;
        for j in &all {
            match &j.oracle {
                Some(OClass::Agree(n)) => {
                    *tally.entry("agree".into()).or_default() += 1;
                    alone += n;
                }
                Some(OClass::Differ(d)) => {
                    *tally.entry("differ".into()).or_default() += 1;
                    println!("{}: {d}", j.name);
                }
                Some(OClass::Skip(s)) => *tally.entry(format!("skip: {}", short(s))).or_default() += 1,
                None => {}
            }
        }
        for (k, n) in &tally {
            println!("{n:5} {k}");
        }
        println!("{alone:5} probes decided by the oracle alone");
    }
    println!("\n== kinds");
    for (k, js) in &kinds {
        println!("{k}: {} ({} first)", js.len(), js[0].name);
    }
    if shrink {
        let mut reference = Reference::start();
        println!("\n== shrunk repros");
        for (k, js) in &kinds {
            let j = js[0];
            let c = &cs[j.case];
            let want = j.class.kind();
            let mut part = c.part.clone();
            let mut best = j.class.clone();
            let mut kept: Vec<usize> = (0..part.features.len()).collect();
            let mut i = part.features.len();
            while i > 1 {
                i -= 1;
                if i >= part.features.len() {
                    continue;
                }
                let mut trial = part.clone();
                trial.features.remove(i);
                let (cl, _) = judge_curved_path(&mut reference, &trial, &rule, j.path);
                if cl.kind() == want {
                    part = trial;
                    kept.remove(i);
                    best = cl;
                }
            }
            println!(
                "--- {k}\n{} [{}]: {}\nfeatures kept: {kept:?}\n{}",
                c.name,
                j.path,
                best.detail(),
                forge_model::adversarial::literal(&part)
            );
            if c.name.starts_with("lang/") {
                println!("source:\n{}", c.source);
            }
        }
    }
}
