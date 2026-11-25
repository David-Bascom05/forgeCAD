//! Rebase: an op made on the text at `base` brought to the head.
//!
//! Measured on 251 documents (research/hosted/MEASUREMENTS.md §c), every op pair on different
//! targets either commutes exactly or differs only by list position (server order decides) —
//! except a Rename committed before an op that still uses the old name. So the only transform is
//! Rename's **name substitution**; every other op is simply re-validated on the head
//! (`apply_checked`), which refuses it with its typed error if a collaborator's op made it
//! invalid (Delete vs Replace of one feature, two Adds of one name, ...).
//!
//! Same-target rules (DESIGN.md §4.2): SetParam vs SetParam: the later wins (it applies on the
//! head; `setLit_lww`). Rename vs Rename of one name: the later is refused (`RenameConflict`).
//! Ops addressed by byte span (LiftLiteral, Respell) are valid only on the text they were made
//! on: refused (`StaleSpan`) when another author's op landed after their base.

use crate::{Entry, Refusal};
use forge_lang::edit_ops::apply_rename;
use forge_lang::error::{Op, ParamKey, Scope};
use forge_lang::syntax::Seg;

/// `op`, made on the text at its base by `client`, brought through `since` (the entries committed
/// after its base, in seq order). The client's own entries are skipped: it had applied them
/// locally before it made `op`, so `op` already reads their names.
pub fn rebase<'a>(op: &Op, client: &str, since: impl IntoIterator<Item = &'a Entry>) -> Result<Op, Refusal> {
    let mut op = op.clone();
    for e in since {
        if e.client == client {
            continue;
        }
        if span_op(&op) {
            return Err(Refusal::StaleSpan);
        }
        for x in &e.ops {
            if let Op::Rename { from, to } = x {
                op = rename_in(&op, from, to)?;
            }
        }
    }
    Ok(op)
}

/// A group brought to the head: each of its ops through `since` like a single op. Never through
/// the group's own earlier ops: the client made each on the text after them (they are its own,
/// as its pending ops since base are). Any op refused refuses the group.
pub fn rebase_group<'a>(
    ops: &[Op],
    client: &str,
    since: impl IntoIterator<Item = &'a Entry> + Clone,
) -> Result<Vec<Op>, Refusal> {
    ops.iter().map(|op| rebase(op, client, since.clone())).collect()
}

/// An op that addresses bytes, not names: valid only on its base text.
pub fn span_op(op: &Op) -> bool {
    matches!(op, Op::LiftLiteral { .. } | Op::Respell { .. })
}

/// `op` as it reads after `Rename {from, to}`: every top-level name `from` it mentions written
/// `to`. A Rename of the same name is a conflict (DESIGN.md §4.2: the later one is refused).
pub fn rename_in(op: &Op, from: &str, to: &str) -> Result<Op, Refusal> {
    let n = |s: &String| if s == from { to.to_string() } else { s.clone() };
    // a feature path or id: its first segment is the top-level name (`bolt[2].hub`: `bolt`)
    let id = |s: &String| {
        let cut = s.find(['.', '[']).unwrap_or(s.len());
        if &s[..cut] == from {
            format!("{to}{}", &s[cut..])
        } else {
            s.clone()
        }
    };
    let path = |p: &Vec<Seg>| {
        let mut p = p.clone();
        if let Some(first) = p.first_mut() {
            first.name = n(&first.name);
        }
        p
    };
    // a param's owner is a feature (its first segment); a shared param's own name is top level
    let key = |k: &ParamKey| {
        if k.owner.is_empty() {
            ParamKey {
                owner: vec![],
                name: n(&k.name),
            }
        } else {
            ParamKey {
                owner: path(&k.owner),
                name: k.name.clone(),
            }
        }
    };
    // a statement or expression's text: every use and binding of the name (forge-lang's own
    // rename, the one the Rename op applies); text that does not lex is left for validation
    let text = |t: &String| apply_rename(t, from, to).unwrap_or_else(|_| t.clone());
    Ok(match op {
        Op::SetParam { param, value } => Op::SetParam {
            param: key(param),
            value: *value,
        },
        Op::AddLocalParam { feature, name, value } => Op::AddLocalParam {
            feature: path(feature),
            name: name.clone(),
            value: *value,
        },
        Op::MakeShared { param } => Op::MakeShared { param: key(param) },
        Op::SplitInstance { id: p, param } => Op::SplitInstance {
            id: path(p),
            param: key(param),
        },
        Op::AddFeature { before, text: t } => Op::AddFeature {
            before: before.as_ref().map(id),
            text: text(t),
        },
        Op::DeleteFeature { id: i } => Op::DeleteFeature { id: id(i) },
        Op::ReplaceFeature { id: i, text: t } => Op::ReplaceFeature {
            id: id(i),
            text: text(t),
        },
        Op::MoveFeature { id: i, before } => Op::MoveFeature {
            id: id(i),
            before: before.as_ref().map(id),
        },
        Op::SetArg { id: i, arg, text: t } => Op::SetArg {
            id: id(i),
            arg: arg.clone(),
            text: text(t),
        },
        Op::Rename { from: f, to: t } => {
            if f == from {
                return Err(Refusal::RenameConflict { name: f.clone() });
            }
            Op::Rename {
                from: f.clone(),
                to: t.clone(),
            }
        }
        Op::SetProperty { key: k, text: t } => Op::SetProperty {
            key: k.clone(),
            text: t.as_ref().map(text),
        },
        Op::Suppress { id: i } => Op::Suppress { id: id(i) },
        Op::Unsuppress { id: i } => Op::Unsuppress { id: id(i) },
        Op::LiftLiteral { span, name, scope } => Op::LiftLiteral {
            span: *span,
            name: name.clone(),
            scope: match scope {
                Scope::Shared => Scope::Shared,
                Scope::Local(p) => Scope::Local(path(p)),
            },
        },
        Op::Respell { .. } => op.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_lang::edit::apply;

    fn e(client: &str, op: Op) -> Entry {
        Entry {
            seq: 1,
            author: client.into(),
            client: client.into(),
            id: 1,
            base: 0,
            ops: vec![op],
            inv: None,
            th: 0,
        }
    }

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
    fn rename_then_late_ops_reach_the_renamed_feature() {
        let r = Op::Rename {
            from: "boss".into(),
            to: "hub".into(),
        };
        let head = apply(DOC, &r).unwrap();
        let late = [
            Op::SetParam {
                param: ParamKey::parse("boss.d").unwrap(),
                value: forge_lang::error::Literal {
                    value: 6.0,
                    dim: forge_lang::values::Dim::Length,
                },
            },
            Op::DeleteFeature { id: "boss".into() },
            Op::MoveFeature {
                id: "boss".into(),
                before: None,
            },
        ];
        for op in late {
            let on_base = apply(DOC, &op).unwrap();
            let moved = rebase(&op, "b", [&e("a", r.clone())]).unwrap();
            // rebased on the head = the op on the base, then the rename (the transform lemma)
            // (a deleted feature cannot be renamed after: then the rename's text is the delete's)
            let want = apply(&on_base, &r).unwrap_or(on_base);
            assert_eq!(apply(&head, &moved).unwrap(), want, "{op:?}");
        }
    }

    #[test]
    fn own_entries_are_not_transformed() {
        let r = Op::Rename {
            from: "boss".into(),
            to: "hub".into(),
        };
        // the client renamed, then (seeing `hub`) added a new `boss`: it must stay `boss`
        let add = Op::AddFeature {
            before: None,
            text: "boss = box(1, 1, 1)".into(),
        };
        assert_eq!(rebase(&add, "a", [&e("a", r.clone())]).unwrap(), add);
        // another client's identical add, made before the rename, does refer to the old one
        assert!(
            matches!(rebase(&add, "b", [&e("a", r)]).unwrap(), Op::AddFeature { text, .. } if text.starts_with("hub"))
        );
    }

    #[test]
    fn rename_race_is_a_conflict_and_span_ops_go_stale() {
        let r = Op::Rename {
            from: "boss".into(),
            to: "hub".into(),
        };
        let r2 = Op::Rename {
            from: "boss".into(),
            to: "post".into(),
        };
        assert!(matches!(
            rebase(&r2, "b", [&e("a", r.clone())]),
            Err(Refusal::RenameConflict { .. })
        ));
        let sp = Op::Respell {
            span: forge_lang::syntax::Span { lo: 0, hi: 1 },
            to: "x".into(),
        };
        assert_eq!(rebase(&sp, "b", [&e("a", r.clone())]), Err(Refusal::StaleSpan));
        assert_eq!(rebase(&sp, "a", [&e("a", r)]).unwrap(), sp);
    }
}
