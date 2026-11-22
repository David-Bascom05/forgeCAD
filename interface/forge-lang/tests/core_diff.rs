//! The evaluator and edit ops against the Lean core calculus (research/forge-lang/diff): random
//! programs and every proof counterexample that maps to a .fg, Lean's results in the fixture,
//! this crate's `fg` on the same programs. A difference not filed in impl-proof-findings.md (or
//! the documented mapping) fails, with an example.

use std::path::Path;
use std::process::Command;

#[test]
fn fg_agrees_with_the_core_calculus() {
    // the fixture's programs are unified documents (components, occurrences)
    if !forge_lang::syntax::KEYWORDS.contains(&"component") {
        eprintln!("core_diff: skipped, this grammar has no components");
        return;
    }
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/forge-lang/diff/core_diff.py");
    let out = match Command::new("python3")
        .arg(&script)
        .args(["test", "--fg", env!("CARGO_BIN_EXE_fg")])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("core_diff: skipped, no python3 ({e})");
            return;
        }
    };
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Make / Extract / Inline Component and file components against the Lean equations
/// (research/forge-lang/diff/refactor_diff.py); skipped where fg has no component refactors.
#[test]
fn refactors_agree_with_the_core_calculus() {
    let script =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/forge-lang/diff/refactor_diff.py");
    let out = match Command::new("python3")
        .arg(&script)
        .args(["--fg", env!("CARGO_BIN_EXE_fg"), "--count", "20"])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("refactor_diff: skipped, no python3 ({e})");
            return;
        }
    };
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// `build::history_id` against the Lean equation (research/forge-lang/lean/ForgeLang/HistoryId.lean:
/// `historyId`, FNV-1a then `(h >> 1) | 1`). `historyId_collides` proves the value below for both
/// names: two legal feature names with one kernel name, which a run refuses (`HistoryIdClash`,
/// `checked_distinct`; fc_parity.rs `a_history_id_clash_is_refused`).
#[test]
fn history_id_is_the_lean_equation() {
    use forge_lang::build::history_id;
    // the Lean function, word for word
    let lean = |s: &str| {
        let h = s.bytes().fold(0xcbf2_9ce4_8422_2325u128, |h, b| {
            ((h ^ u128::from(b)) * 0x0100_0000_01b3) % (1u128 << 64)
        });
        ((h >> 1) | 1) as u64
    };
    for s in [
        "",
        "base",
        "bolts[1]",
        "box1.lid1.plate",
        "e[lf]",
        "hole[2,3].bore",
    ] {
        assert_eq!(history_id(s), lean(s), "{s}");
    }
    // HistoryId.historyId_collides, by `decide`
    assert_eq!(history_id("qolagjaapenjgkgca"), 3_315_298_033_150_976_027);
    assert_eq!(history_id("qpibohgembfgikgid"), 3_315_298_033_150_976_027);
}
