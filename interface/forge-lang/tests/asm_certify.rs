//! The interference check settles a pair from its meshes (forge-asm `check::certify`) only where
//! the exact boolean would give the same verdict. The two cases that made it (a modelled M6 thread
//! grown to 6.3 inside its nut: ~6 s and ~9 s of exact helical intersection before) and every
//! assembly in `examples/`, run with the certificate on and with it off (the exact path alone):
//! the same interfering pairs. Its own test binary: the switch is process-wide.
use forge_lang::Run;
use forge_lang::asm::Issue;
use forge_lang::error::Warning;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn build(file: &Path, text: &str) -> Run {
    let opts = forge_lang::Options {
        dir: file.parent().map(Path::to_path_buf),
        ..forge_lang::Options::default()
    };
    forge_lang::run(text, opts, Default::default()).0
}

/// The interfering pairs, sorted, and whether each was confirmed.
fn interferences(r: &Run) -> Vec<(String, String, bool)> {
    let mut out: Vec<(String, String, bool)> = r
        .warnings
        .iter()
        .filter_map(|w| match w {
            Warning::Asm(n) => match &n.issue {
                Issue::Interference { a, b, confirmed, .. } => Some((a.clone(), b.clone(), *confirmed)),
                _ => None,
            },
            _ => None,
        })
        .collect();
    out.sort();
    out
}

fn lower_bounds(r: &Run) -> Vec<f64> {
    r.warnings
        .iter()
        .filter_map(|w| match w {
            Warning::Asm(n) => match &n.issue {
                Issue::Interference { volume_at_least, .. } => *volume_at_least,
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// `file` with the line `shared NAME = OLD` set to NEW.
fn edited(file: &Path, name: &str, old: &str, new: &str) -> String {
    let text = std::fs::read_to_string(file).expect("example");
    let from = format!("shared {name} = {old}\n");
    assert!(text.contains(&from), "{} has `{from}`", file.display());
    text.replace(&from, &format!("shared {name} = {new}\n"))
}

/// One repro: the verdict the exact path gives (measured on main 50af9c42), every interfering pair
/// proven with a positive lower bound and none sent to the exact boolean.
fn repro(rel: &str, name: &str, want: &[(&str, &str)]) {
    forge_asm::check::CERTIFY_FROM_MESHES.store(true, Ordering::Relaxed);
    let file = repo().join(rel);
    let r = build(&file, &edited(&file, name, "6", "6.3"));
    let got = interferences(&r);
    let want: Vec<(String, String, bool)> = want.iter().map(|(a, b)| (a.to_string(), b.to_string(), true)).collect();
    assert_eq!(got, want, "{rel}: the interfering pairs");
    let o = r.asm.as_ref().and_then(|a| a.overlap.clone()).expect("the overlap check ran");
    // the work bound: every interference proven from the meshes, and no exact intersection run
    // beyond those the document as it was runs (its touching pairs, each a fast boolean)
    let base = build(&file, &std::fs::read_to_string(&file).expect("example"));
    let b = base.asm.as_ref().and_then(|a| a.overlap.clone()).expect("the overlap check ran");
    assert_eq!((o.certified, o.exact_runs), (want.len(), b.exact_runs), "{rel}: {o:?}, before {b:?}");
    let lo = lower_bounds(&r);
    assert!(lo.len() == want.len() && lo.iter().all(|v| *v > 0.0), "{rel}: {lo:?}");
}

#[test]
fn a_thread_grown_into_its_nut_is_proven_from_the_meshes() {
    repro("examples/rack18u/hw/tests/hinge_pin.fg", "size_m6", &[("nut", "pin")]);
}

#[test]
fn post_kit_threads_grown_into_their_nuts_are_proven_from_the_meshes() {
    // the pairs the exact path finds (main 50af9c42: each unconfirmed there, its boolean
    // refusing the screw faces as Unsupported after ~4.6 s each)
    repro(
        "examples/rack18u/asm/post_kit.fg",
        "m6",
        &[("nut_a[0]", "stud_a[0]"), ("nut_a[1]", "stud_a[1]"), ("stud_b[0]", "nut_b[0]"), ("stud_b[1]", "nut_b[1]")],
    );
}

/// Every assembly in examples/: the same interfering pairs with the certificate as with the exact
/// path alone. A pair the certificate settled is confirmed in both (the exact path confirms it
/// with its volume), so the comparison is on (a, b) and confirmed.
#[test]
#[ignore = "builds every example assembly twice (minutes); run with --ignored"]
fn every_example_assembly_has_the_exact_paths_verdicts() {
    let mut files = vec![];
    let mut dirs = vec![repo().join("examples")];
    while let Some(d) = dirs.pop() {
        for e in std::fs::read_dir(&d).expect("dir").flatten() {
            let p = e.path();
            if p.is_dir() {
                dirs.push(p);
            } else if p.extension().is_some_and(|x| x == "fg") {
                files.push(p);
            }
        }
    }
    files.sort();
    let (mut asms, mut certified) = (0, 0);
    let mut bad = vec![];
    for f in &files {
        let text = std::fs::read_to_string(f).expect("read");
        forge_asm::check::CERTIFY_FROM_MESHES.store(true, Ordering::Relaxed);
        let with = build(f, &text);
        let Some(o) = with.asm.as_ref().and_then(|a| a.overlap.clone()) else {
            continue;
        };
        asms += 1;
        certified += o.certified;
        forge_asm::check::CERTIFY_FROM_MESHES.store(false, Ordering::Relaxed);
        let without = build(f, &text);
        forge_asm::check::CERTIFY_FROM_MESHES.store(true, Ordering::Relaxed);
        let (x, y) = (interferences(&with), interferences(&without));
        let unconfirmed_exact = y.iter().any(|p| !p.2);
        // a pair the exact boolean failed on is unconfirmed there; proven here it is confirmed:
        // the same pair, a better answer
        let pairs = |v: &[(String, String, bool)]| v.iter().map(|p| (p.0.clone(), p.1.clone())).collect::<Vec<_>>();
        if pairs(&x) != pairs(&y) || (!unconfirmed_exact && x != y) {
            bad.push(format!("{}: with {x:?}, exact {y:?}", f.display()));
        }
    }
    println!("{asms} assemblies, {certified} pairs certified");
    assert!(bad.is_empty(), "{bad:#?}");
}

/// The same comparison on edited assemblies, where interference is the point: each example
/// assembly with each of its first four `shared` numbers grown by 5% (thread sizes, offsets and
/// clearances move parts into each other).
#[test]
#[ignore = "builds every example assembly's variants twice (many minutes); run with --ignored"]
fn grown_example_assemblies_have_the_exact_paths_verdicts() {
    let mut files = vec![];
    let mut dirs = vec![repo().join("examples")];
    while let Some(d) = dirs.pop() {
        for e in std::fs::read_dir(&d).expect("dir").flatten() {
            let p = e.path();
            if p.is_dir() {
                dirs.push(p);
            } else if p.extension().is_some_and(|x| x == "fg") {
                files.push(p);
            }
        }
    }
    files.sort();
    let (mut runs, mut certified, mut hits) = (0, 0, 0);
    let mut bad = vec![];
    for f in &files {
        let text = std::fs::read_to_string(f).expect("read");
        if !text.lines().any(|l| l.trim() == "assembly") {
            continue;
        }
        let shared: Vec<(String, f64)> = text
            .lines()
            .filter_map(|l| {
                let rest = l.strip_prefix("shared ")?;
                let (name, v) = rest.split_once(" = ")?;
                let v: f64 = v.split_whitespace().next()?.parse().ok()?;
                (v != 0.0).then(|| (name.trim().to_string(), v))
            })
            .take(4)
            .collect();
        for (name, v) in shared {
            let Some(grown) = grow(&text, &name, v) else { continue };
            forge_asm::check::CERTIFY_FROM_MESHES.store(true, Ordering::Relaxed);
            let with = build(f, &grown);
            let Some(o) = with.asm.as_ref().and_then(|a| a.overlap.clone()) else {
                continue;
            };
            runs += 1;
            certified += o.certified;
            forge_asm::check::CERTIFY_FROM_MESHES.store(false, Ordering::Relaxed);
            let without = build(f, &grown);
            forge_asm::check::CERTIFY_FROM_MESHES.store(true, Ordering::Relaxed);
            let (x, y) = (interferences(&with), interferences(&without));
            hits += y.len();
            let pairs = |v: &[(String, String, bool)]| v.iter().map(|p| (p.0.clone(), p.1.clone())).collect::<Vec<_>>();
            // confirmed may differ only where the exact boolean failed (unconfirmed) and the
            // certificate proved the pair
            let ok = pairs(&x) == pairs(&y) && x.iter().zip(&y).all(|(p, q)| p.2 == q.2 || (p.2 && !q.2));
            if !ok {
                bad.push(format!("{} {name}={}: with {x:?}, exact {y:?}", f.display(), v * 1.05));
            }
        }
    }
    println!("{runs} edited assemblies, {hits} interferences, {certified} certified");
    assert!(bad.is_empty(), "{bad:#?}");
}

/// `text` with the first line `shared NAME = V..` set to V × 1.05 (the rest of the line kept).
fn grow(text: &str, name: &str, v: f64) -> Option<String> {
    let prefix = format!("shared {name} = ");
    let line = text.lines().find(|l| l.starts_with(&prefix))?;
    let rest = &line[prefix.len()..];
    let num = rest.split_whitespace().next()?;
    let new_line = format!("{prefix}{}{}", v * 1.05, &rest[num.len()..]);
    Some(text.replacen(line, &new_line, 1))
}
