//! Whether each feature left a valid part, one feature against the part before it. Regeneration keeps
//! its cheap checks, so a drag is not held up: this runs after a commit, on a worker, over the stages the
//! regen kept. A stage already looked at need not be again, so a caller goes on from the first new one.
use crate::bodies::Solid;
use forge_topo::{Entity, Problem, check, check_against};
use std::sync::atomic::{AtomicBool, Ordering};

/// One thing wrong: the rule broken, in a sentence, and the face, edge or corner at fault.
#[derive(Clone, Debug, PartialEq)]
pub struct Issue {
    pub rule: &'static str,
    pub reason: String,
    pub entity: Option<Entity>,
}

/// A feature that left a part `check` faults, with what it found.
#[derive(Clone, Debug, PartialEq)]
pub struct Fault {
    pub feature: usize,
    pub issues: Vec<Issue>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
    /// Each feature with an issue, in order; none if the stages passed.
    Done(Vec<Fault>),
    /// Asked to stop: the part changed.
    Stopped,
}

fn issues(found: &[Problem], body: &forge_topo::Body) -> Vec<Issue> {
    let body = body.compact();
    found
        .iter()
        .map(|p| Issue {
            rule: p.rule(),
            reason: p.reason(),
            entity: p.entity(&body),
        })
        .collect()
}

/// What is wrong with stage `new`, against the stage before it. Only what differs from `old` is tested, so with
/// `old` at fault its unchanged faces and edges are not told of again.
pub fn stage(old: &[Solid], new: &[Solid]) -> Vec<Issue> {
    let mut out = Vec::new();
    for s in new {
        let found = match old.iter().find(|o| o.id == s.id) {
            Some(o) if *o == *s => Vec::new(),
            Some(o) => check_against(&s.body, &o.body),
            None => check(&s.body),
        };
        // pieces a boolean split can share a name until they are joined: only a finished part must not
        if !found.is_empty() && found.iter().all(|p| matches!(p, Problem::DuplicateName { .. })) {
            let done = crate::finish(s.body.clone());
            out.extend(issues(&check(&done), &done));
        } else {
            out.extend(issues(&found, &s.body));
        }
    }
    out
}

/// `stage` over `stages[from..]` in order. Stages before `from` are not looked at: what was found on them stands.
pub fn faults<'a>(stages: impl IntoIterator<Item = &'a [Solid]>, from: usize, stop: &AtomicBool) -> Verdict {
    let mut out: Vec<Fault> = Vec::new();
    let mut old: &[Solid] = &[];
    let mut before: Vec<Issue> = Vec::new();
    for (i, new) in stages.into_iter().enumerate() {
        if stop.load(Ordering::Relaxed) {
            return Verdict::Stopped;
        }
        if i >= from {
            let found = stage(old, new);
            // a rule over the whole body says the same again until the fault is gone: the feature that made it
            let fresh: Vec<Issue> = found.iter().filter(|x| !before.contains(x)).cloned().collect();
            if !fresh.is_empty() {
                out.push(Fault {
                    feature: i,
                    issues: fresh,
                });
            }
            before = found;
        }
        old = new;
    }
    Verdict::Done(out)
}

/// The whole check of a finished part, for a save: nothing taken from earlier stages.
pub fn full(solids: &[Solid]) -> Vec<Issue> {
    solids
        .iter()
        .flat_map(|s| {
            let done = crate::finish(s.body.clone());
            issues(&check(&done), &done)
        })
        .collect()
}
