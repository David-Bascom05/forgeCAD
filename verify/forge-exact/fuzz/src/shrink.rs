//! Cutting a disagreement down: a language program by its statements (the way
//! `forge_lang::fuzz::shrink` does, with the reference's verdict as the test), a part by its
//! features (the first kept).

use crate::{Case, Class, Reference, judge_kernel, nary, nudged_last, reference_input, sequential};
use forge_model::Part;

/// The class of one kernel path of `part` against the reference.
pub fn class_on(reference: &mut Reference, part: &Part, path: &str) -> Class {
    let input = match reference_input(part) {
        Ok(i) => i,
        Err(e) => return Class::Scope(e),
    };
    let ans = reference.ask(&input);
    let k = match path {
        "sequential" => sequential(part),
        "nary" => nary(part),
        p => nudged_last(part, p.trim_start_matches("ulp ").parse().unwrap_or(0)),
    };
    judge_kernel(reference, part, &ans, &k)
}

fn lang_part(src: &str) -> Option<Part> {
    crate::guarded(|| Ok(forge_lang::fuzz::build(src).part())).ok()
}

/// The smallest case found that still disagrees in the same way: its source and its class.
pub fn shrink(reference: &mut Reference, case: &Case, path: &str, want: &Class) -> (String, Class) {
    let same = |reference: &mut Reference, part: &Part| {
        let c = class_on(reference, part, path);
        (c.kind() == want.kind()).then_some(c)
    };
    if case.name.starts_with("lang/") {
        let mut stmts = forge_lang::fuzz::statements(&case.source);
        let mut best = want.clone();
        let mut chunk = stmts.len() / 2;
        while chunk >= 1 {
            let mut end = stmts.len();
            while end > 1 {
                let start = end.saturating_sub(chunk).max(1);
                let mut trial = stmts.clone();
                trial.drain(start..end);
                let src = trial.join("\n") + "\n";
                if let Some(p) = lang_part(&src)
                    && let Some(c) = same(reference, &p)
                {
                    stmts = trial;
                    best = c;
                }
                end = start;
            }
            chunk /= 2;
        }
        (stmts.join("\n") + "\n", best)
    } else {
        let mut part = case.part.clone();
        let mut best = want.clone();
        let mut i = part.features.len();
        while i > 1 {
            i -= 1;
            if i >= part.features.len() {
                continue;
            }
            let mut trial = part.clone();
            trial.features.remove(i);
            if let Some(c) = same(reference, &trial) {
                part = trial;
                best = c;
            }
        }
        (forge_model::adversarial::literal(&part), best)
    }
}
