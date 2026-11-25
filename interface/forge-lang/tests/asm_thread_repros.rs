//! A modelled M6 thread grown to 6.3 inside its nut: the overlap check's exact intersection of
//! the helical faces took 5.7 s (hinge_pin) and 9.7 s (post_kit) on main 50af9c42 and then
//! refused (Unsupported: UntouchedOnOther), so the interferences were reported unconfirmed with no
//! volume. With anti-parallel screws in closed form and a thread's band on its bore Coincident,
//! each is exact. Here: the same pairs, each confirmed with its exact volume, one exact boolean
//! per new pair beyond the unedited document's, and well under the old times.
//! Volumes cross-checked by mesh point sampling (CURVED-SITES): 10.51 ± 0.21 mm³ for hinge_pin's
//! nut ∩ pin, 14.612 ± 0.053 for post_kit's nut_a[0] ∩ stud_a[0] (mesh tolerance 0.00025 mm).
use forge_lang::Run;
use forge_lang::asm::Issue;
use forge_lang::error::Warning;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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

/// An interference: its pair, whether confirmed, its exact volume, why the boolean failed.
type Hit = (String, String, bool, Option<f64>, Option<String>);

fn interferences(r: &Run) -> Vec<Hit> {
    let mut out: Vec<_> = r
        .warnings
        .iter()
        .filter_map(|w| match w {
            Warning::Asm(n) => match &n.issue {
                Issue::Interference {
                    a,
                    b,
                    confirmed,
                    volume,
                    boolean_failed,
                } => Some((a.clone(), b.clone(), *confirmed, *volume, boolean_failed.clone())),
                _ => None,
            },
            _ => None,
        })
        .collect();
    out.sort_by(|x, y| (&x.0, &x.1).cmp(&(&y.0, &y.1)));
    out
}

/// `file` with the line `shared NAME = 6` set to 6.3.
fn grown(file: &Path, name: &str) -> String {
    let text = std::fs::read_to_string(file).expect("example");
    let from = format!("shared {name} = 6\n");
    assert!(text.contains(&from), "{} has `{from}`", file.display());
    text.replace(&from, &format!("shared {name} = 6.3\n"))
}

fn repro(rel: &str, name: &str, want: &[(&str, &str)], volume: f64) {
    let file = repo().join(rel);
    let base = build(&file, &std::fs::read_to_string(&file).expect("example"));
    let b = base
        .asm
        .as_ref()
        .and_then(|a| a.overlap.clone())
        .expect("the overlap check ran");
    let t = Instant::now();
    let r = build(&file, &grown(&file, name));
    let took = t.elapsed();
    let got = interferences(&r);
    let pairs: Vec<(&str, &str)> = got.iter().map(|g| (g.0.as_str(), g.1.as_str())).collect();
    assert_eq!(pairs, want, "{rel}: the interfering pairs");
    for (a, b, confirmed, v, failed) in &got {
        assert!(
            *confirmed && failed.is_none(),
            "{rel}: {a}/{b} not exact: {failed:?}"
        );
        let v = v.expect("an exact volume");
        assert!(
            (v - volume).abs() <= 1e-3 * volume,
            "{rel}: {a}/{b} shares {v} mm³, not {volume}"
        );
    }
    // the work: one exact boolean more per new interference than the document as it was
    let o = r
        .asm
        .as_ref()
        .and_then(|a| a.overlap.clone())
        .expect("the overlap check ran");
    assert_eq!(
        o.exact_runs,
        b.exact_runs + want.len(),
        "{rel}: {o:?}, before {b:?}"
    );
    // and the time, with room: 5.7 s and 9.7 s before, under 0.2 s now (release)
    assert!(took < Duration::from_secs(3), "{rel}: {took:?}");
}

#[test]
fn a_thread_grown_into_its_nut_is_one_exact_interference() {
    repro(
        "examples/rack18u/hw/tests/hinge_pin.fg",
        "size_m6",
        &[("nut", "pin")],
        10.688,
    );
}

#[test]
fn post_kit_threads_grown_into_their_nuts_are_four_exact_interferences() {
    repro(
        "examples/rack18u/asm/post_kit.fg",
        "m6",
        &[
            ("nut_a[0]", "stud_a[0]"),
            ("nut_a[1]", "stud_a[1]"),
            ("stud_b[0]", "nut_b[0]"),
            ("stud_b[1]", "nut_b[1]"),
        ],
        14.641,
    );
}
