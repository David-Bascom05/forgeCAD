//! Inverse ops, for hosted undo (DESIGN.md §4.6): undo = the inverse of the user's own last op,
//! rebased onto the head like any op, refused with its typed error if a collaborator's later op
//! made it invalid. Undo is never a rewind: other authors' edits stay.
//!
//! The inverse is computed when the op commits, from the text before and after, and kept only if
//! it is exact: `apply(after, inverse) == before`, byte for byte. An op with no exact inverse has
//! none (undo says so) rather than an approximate one.

use forge_lang::edit::{apply, bound};
use forge_lang::error::{Op, ParamKey};
use forge_lang::syntax;

/// The exact inverse of `op`, which took `before` to `after`, or None.
pub fn inverse(before: &str, op: &Op, after: &str) -> Option<Op> {
    let cand = candidate(before, op, after)?;
    (apply(after, &cand).ok()? == before).then_some(cand)
}

fn candidate(before: &str, op: &Op, after: &str) -> Option<Op> {
    Some(match op {
        Op::SetParam { param, .. } => Op::SetParam {
            param: param.clone(),
            value: literal(before, param)?,
        },
        Op::Rename { from, to } => Op::Rename {
            from: to.clone(),
            to: from.clone(),
        },
        Op::Suppress { id } => Op::Unsuppress { id: id.clone() },
        Op::Unsuppress { id } => Op::Suppress { id: id.clone() },
        Op::AddFeature { text, .. } => {
            // the one name the added statement binds
            let names = bound(&syntax::parse(text).ok()?);
            let mut it = names.into_iter();
            let id = it.next()?;
            if it.next().is_some() {
                return None;
            }
            Op::DeleteFeature { id }
        }
        Op::DeleteFeature { .. } => {
            let (lo, removed, _) = changed(before, after)?;
            Op::AddFeature {
                before: next_id(after, lo),
                text: removed.trim_end_matches('\n').to_string(),
            }
        }
        Op::ReplaceFeature { id, .. } => {
            let (_, old, _) = changed(before, after)?;
            Op::ReplaceFeature {
                id: id.clone(),
                text: old.trim_end_matches('\n').to_string(),
            }
        }
        Op::MoveFeature { id, .. } => {
            // where it was: above the statement that followed it before
            let prog = syntax::parse(before).ok()?;
            let k = prog
                .iter()
                .position(|s| bound(std::slice::from_ref(s)).contains(id))?;
            let next = prog[k + 1..]
                .iter()
                .find_map(|s| bound(std::slice::from_ref(s)).into_iter().next());
            Op::MoveFeature {
                id: id.clone(),
                before: next,
            }
        }
        Op::SetProperty { key, .. } => {
            let (_, old, _) = changed(before, after)?;
            // the old argument's text (`key: TEXT`), or none if it was not there
            let t = old.trim().trim_start_matches(',').trim();
            let text = t
                .strip_prefix(key.as_str())
                .and_then(|r| r.trim_start().strip_prefix(':'))
                .map(|v| v.trim().trim_end_matches(',').trim().to_string());
            Op::SetProperty {
                key: key.clone(),
                text,
            }
        }
        _ => return None,
    })
}

/// The literal a param is set to in `src`.
fn literal(src: &str, key: &ParamKey) -> Option<forge_lang::error::Literal> {
    forge_lang::edit::handles(src)
        .into_iter()
        .find(|(k, _)| k == key)
        .map(|(_, l)| l)
}

/// The one changed region: (offset, the bytes `before` had there, the bytes `after` has).
fn changed<'a>(before: &'a str, after: &'a str) -> Option<(usize, &'a str, &'a str)> {
    let (a, b) = (before.as_bytes(), after.as_bytes());
    let mut p = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    // whole lines: an op takes out or puts in statements with their line breaks
    while p > 0 && a[p - 1] != b'\n' {
        p -= 1;
    }
    let max_s = a.len().min(b.len()) - p;
    let s = a
        .iter()
        .rev()
        .zip(b.iter().rev())
        .take(max_s)
        .take_while(|(x, y)| x == y)
        .count();
    let (mut ea, mut eb) = (a.len() - s, b.len() - s);
    // the common tail is the same bytes in both: widen both ends to the end of the line
    while ea > p && ea < a.len() && a[ea - 1] != b'\n' {
        ea += 1;
        eb += 1;
    }
    Some((p, before.get(p..ea)?, after.get(p..eb)?))
}

/// The id of the first top-level statement at or after byte `at` of `src` that binds a name.
fn next_id(src: &str, at: usize) -> Option<String> {
    let prog = syntax::parse(src).ok()?;
    prog.iter()
        .filter(|s| s.span.lo as usize >= at)
        .find_map(|s| bound(std::slice::from_ref(s)).into_iter().next())
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_lang::edit::apply_checked;

    const DOC: &str = "shared w = 40
shared h = 12
sketch plan(on: xy) {
  o = rect(center: (0, 0), size: (w, 30))
}
base = extrude(plan, h)
sketch ring(on: base.end) {
  c = circle((0, 0), 6)
}
param boss.d = 8
boss = extrude(ring, d)
sketch tab(on: xy) {
  o = rect(center: (40, 0), size: (8, 8))
}
param lid.t = 2
lid = extrude(tab, t, new: true)
post = extrude(tab, 9, new: true)
";

    #[test]
    fn inverses_are_exact() {
        let ops = [
            Op::SetParam {
                param: ParamKey::parse("w").unwrap(),
                value: forge_lang::error::Literal {
                    value: 12.5,
                    dim: forge_lang::values::Dim::Length,
                },
            },
            Op::Rename {
                from: "lid".into(),
                to: "cap".into(),
            },
            Op::DeleteFeature { id: "lid".into() },
            Op::DeleteFeature { id: "post".into() },
            Op::MoveFeature {
                id: "post".into(),
                before: Some("lid".into()),
            },
            Op::MoveFeature {
                id: "lid".into(),
                before: None,
            },
            Op::ReplaceFeature {
                id: "post".into(),
                text: "post = extrude(tab, 5, new: true)".into(),
            },
            Op::AddFeature {
                before: None,
                text: "peg = extrude(plan, 1, new: true)".into(),
            },
        ];
        for op in ops {
            assert!(
                forge_lang::run(
                    DOC,
                    forge_lang::Options {
                        build: false,
                        ..Default::default()
                    },
                    Default::default()
                )
                .0
                .errors
                .is_empty()
            );
            let (after, _) = apply_checked(DOC, &op).unwrap_or_else(|e| panic!("{op:?}: {e:?}"));
            let inv = inverse(DOC, &op, &after).unwrap_or_else(|| {
                let c = candidate(DOC, &op, &after);
                let back = c.as_ref().map(|c| apply(&after, c));
                panic!("no inverse for {op:?}: candidate {c:?} gives {back:?}")
            });
            assert_eq!(apply(&after, &inv).unwrap(), DOC, "{op:?} / {inv:?}");
        }
    }
}
