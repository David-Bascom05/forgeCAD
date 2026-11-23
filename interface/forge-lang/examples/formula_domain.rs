//! How much of the curved boolean domain the formula route (curved-design.md D2) would cover, as
//! asked by the master (2026-11-23): of the curved booleans in the speed-gate parts, the
//! reliability scoreboard's families and the repo's .fg parts, the fraction whose inputs both
//! come only from formula-producing features (extrude, revolve, ball, hole, pattern of those,
//! body combines and moves of those).
//!
//! A static walk of each part's feature history, one body's state at a time (the part's first
//! body; `New` bodies beside it are not followed):
//! - `formula`: every feature so far produces a sign formula;
//! - `curved`: the body has a curved face.
//!
//! A boolean is curved if the body or the tool is. Fillets, chamfers, shells, drafts, edits,
//! sweeps, coils, lofts, text, threads and imports end the formula. A profile with a spline
//! segment makes a non-quadric tool, which also ends it. This is an estimate of the domain, not
//! a run of the kernel.
//!
//! usage: cargo run --release -p forge-lang --example formula_domain   (WHY=1: print, to stderr,
//! the feature that ends each part's formula)

use forge_model::{BodyFeature, Combine, Feature, Part, adversarial, suite};
use forge_ops::Segment;

#[derive(Default, Clone, Copy)]
struct Tally {
    curved: usize,
    ok: usize,
    parts: usize,
    parts_all_ok: usize,
}

struct St {
    exists: bool,
    formula: bool,
    curved: bool,
}

fn profile_kind(p: &forge_ops::Profile) -> (bool, bool) {
    // (curved, quadric-only)
    let mut curved = false;
    let mut quad = true;
    for l in &p.loops {
        for (s, _) in &l.segments {
            match s {
                Segment::Line { .. } => {}
                Segment::Arc { .. } | Segment::Circle { .. } => curved = true,
                Segment::Spline(_) => {
                    curved = true;
                    quad = false;
                }
            }
        }
    }
    (curved, quad)
}

fn boolean(st: &mut St, t: &mut Tally, any_ok: &mut bool, tool_curved: bool, tool_formula: bool) {
    if st.curved || tool_curved {
        t.curved += 1;
        if st.formula && tool_formula {
            t.ok += 1;
        } else {
            *any_ok = false;
        }
    }
    st.curved |= tool_curved;
    st.formula &= tool_formula;
}

fn combine(
    st: &mut St,
    t: &mut Tally,
    any_ok: &mut bool,
    op: Combine,
    tool_curved: bool,
    tool_formula: bool,
) {
    if !st.exists {
        st.exists = true;
        st.formula = tool_formula;
        st.curved = tool_curved;
    } else if op != Combine::New {
        boolean(st, t, any_ok, tool_curved, tool_formula);
    }
}

fn walk(f: &Feature, st: &mut St, t: &mut Tally, any_ok: &mut bool) {
    match f {
        Feature::Extrude { profile, op, .. } => {
            let (c, q) = profile_kind(profile);
            combine(st, t, any_ok, *op, c, q);
        }
        Feature::Revolve { profile, op, .. } => {
            let (_, q) = profile_kind(profile);
            combine(st, t, any_ok, *op, true, q);
        }
        Feature::Ball { op, .. } => combine(st, t, any_ok, *op, true, true),
        Feature::Hole(_) => combine(st, t, any_ok, Combine::Cut, true, true),
        Feature::Pattern { .. } => {
            let c = st.curved;
            if st.exists {
                boolean(st, t, any_ok, c, true);
            }
        }
        Feature::Fillet { .. } | Feature::FaceFillet { .. } | Feature::FullRound { .. } => {
            st.formula = false;
            st.curved = true;
        }
        Feature::Chamfer { .. } | Feature::Shell { .. } | Feature::Draft { .. } => st.formula = false,
        Feature::Edit(_) => {
            // a rib or a direct edit: booleans with a tool that has no formula
            if st.exists && st.curved {
                t.curved += 1;
                *any_ok = false;
            }
            st.formula = false;
        }
        Feature::Sweep(s) => combine(st, t, any_ok, s.op, true, false),
        Feature::Coil(c) => combine(st, t, any_ok, c.op, true, false),
        Feature::Loft(l) => combine(st, t, any_ok, l.op, true, false),
        Feature::Text(x) => combine(st, t, any_ok, x.op, false, false),
        Feature::Import(i) => combine(st, t, any_ok, i.op, true, false),
        Feature::Thread(_) => combine(st, t, any_ok, Combine::Cut, true, false),
        Feature::Body(b) => match b.as_ref() {
            BodyFeature::In { feature, .. } => walk(feature, st, t, any_ok),
            BodyFeature::Combine { .. } | BodyFeature::Split { .. } => {
                if st.exists {
                    let c = st.curved;
                    boolean(st, t, any_ok, c, true);
                }
            }
            BodyFeature::SplitFace { .. } | BodyFeature::Move { .. } => {}
        },
        Feature::Plane(_) | Feature::Param(_) | Feature::Sketch(_) | Feature::Work(_) => {}
    }
}

fn tally(parts: &[(String, Part)]) -> Tally {
    let mut t = Tally::default();
    for (_n, p) in parts {
        let mut st = St {
            exists: false,
            formula: true,
            curved: false,
        };
        let before = t.curved;
        let mut any_ok = true;
        for f in &p.features {
            let was = st.formula && st.exists;
            walk(f, &mut st, &mut t, &mut any_ok);
            if was && !st.formula && std::env::var("WHY").is_ok() {
                let k = format!("{f:?}");
                let k: String = k.chars().take_while(|c| c.is_alphanumeric()).collect();
                eprintln!("LOST {k} {}", _n);
            }
        }
        if t.curved > before {
            t.parts += 1;
            if any_ok {
                t.parts_all_ok += 1;
            }
        }
    }
    t
}

fn fg_parts(root: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(root) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if p.is_dir() {
            if name != "target" && !name.starts_with('.') {
                fg_parts(&p, out);
            }
        } else if name.ends_with(".fg") {
            out.push(p);
        }
    }
}

fn build(src: &str, dir: &std::path::Path) -> Option<Part> {
    let opts = forge_lang::Options {
        dir: Some(dir.to_path_buf()),
        checks: false,
        ..forge_lang::Options::default()
    };
    let (_run, cache) = forge_lang::run(src, opts, Default::default());
    let (f, _) = cache.history();
    if f.is_empty() {
        None
    } else {
        Some(Part { features: f.to_vec() })
    }
}

fn report(name: &str, t: Tally) {
    let pct = |a: usize, b: usize| if b == 0 { 0.0 } else { 100.0 * a as f64 / b as f64 };
    println!(
        "{name:<24} curved booleans {:>6}, formula-covered {:>6} ({:>5.1}%); parts with a curved boolean {:>4}, all covered {:>4} ({:>5.1}%)",
        t.curved,
        t.ok,
        pct(t.ok, t.curved),
        t.parts,
        t.parts_all_ok,
        pct(t.parts_all_ok, t.parts)
    );
}

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // the speed gate's synthetic parts
    let gate: Vec<(String, Part)> = suite::parts()
        .into_iter()
        .map(|(n, p)| (n.to_string(), p))
        .collect();
    // the reliability scoreboard: every family, seeds 0..24
    let mut rel: Vec<(String, Part)> = Vec::new();
    for (n, f) in adversarial::families() {
        for s in 0..24u64 {
            rel.push((format!("{n}/{s}"), f(s)));
        }
    }
    // the .fg parts: the gate's bench parts, and everything else in the repo
    let mut files = Vec::new();
    fg_parts(&root, &mut files);
    files.sort();
    let dir = std::env::temp_dir().join("forge-formula-domain");
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(dir.join(forge_lang::bench::FONT), forge_lang::bench::font_bytes()).expect("font");
    let (mut bench, mut other) = (Vec::new(), Vec::new());
    let mut unbuilt = 0;
    for p in &files {
        let Ok(src) = std::fs::read_to_string(p) else {
            continue;
        };
        match std::panic::catch_unwind(|| build(&src, &dir)) {
            Ok(Some(part)) => {
                let name = p.display().to_string();
                if name.contains("parts/bench/") {
                    bench.push((name, part));
                } else {
                    other.push((name, part));
                }
            }
            _ => unbuilt += 1,
        }
    }
    report("gate: synthetic parts", tally(&gate));
    report("gate: bench .fg parts", tally(&bench));
    report("reliability families", tally(&rel));
    report("other .fg parts", tally(&other));
    let mut all = gate.clone();
    all.extend(bench.iter().cloned());
    all.extend(rel.iter().cloned());
    all.extend(other.iter().cloned());
    report("all", tally(&all));
    println!(
        "(.fg files: {}, built {}, not built {unbuilt})",
        files.len(),
        bench.len() + other.len()
    );
}
