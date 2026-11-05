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
