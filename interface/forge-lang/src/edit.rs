//! Typed edits: each a pure function of (document, op) to a document or a typed error. Every
//! op changes one defined span, or inserts at one point, and leaves all other bytes as they
//! were. `apply_ast` is the same op on the syntax tree, for checking the two agree.

use crate::error::{Error, Literal, Op, ParamKey, Scope, path_text};
use crate::syntax::{self, Arg, Expr, ExprKind, Seg, Span, Stmt, StmtKind, Unit};
use crate::values::Dim;
use std::collections::BTreeSet;

type R<T> = Result<T, Error>;

fn stmt_name(s: &Stmt) -> Option<&str> {
    match &s.kind {
        StmtKind::Assign { name, .. } | StmtKind::Block { name, .. } => Some(name),
        _ => None,
    }
}

fn param_stmt<'a>(prog: &'a [Stmt], key: &ParamKey) -> Option<&'a Stmt> {
    prog.iter().find(|s| match &s.kind {
        StmtKind::Shared { name, .. } => key.owner.is_empty() && *name == key.name,
        StmtKind::Param { owner, name, .. } => *owner == key.owner && *name == key.name,
        _ => false,
    })
}

fn rhs(s: &Stmt) -> &Expr {
    match &s.kind {
        StmtKind::Shared { value, .. } | StmtKind::Param { value, .. } => value,
        _ => unreachable!("a param statement"),
    }
}

/// The top-level statement making feature `id` (its first path segment).
fn feature_stmt<'a>(prog: &'a [Stmt], id: &str) -> R<&'a Stmt> {
    if let Some(s) = prog.iter().find(|s| stmt_name(s) == Some(id)) {
        return Ok(s);
    }
    // made in a loop or a call: an instance, not a statement of its own
    let base = id.split(['.', '[']).next().unwrap_or(id);
    if bound(prog).contains(base) {
        return Err(Error::NotAStatement { id: id.into() });
    }
    Err(Error::UnknownFeature { id: id.into() })
}

/// The statements a feature is: its own params written directly above it, and its statement.
/// Returns (first, statement) indices into `prog`.
pub(crate) fn block(prog: &[Stmt], id: &str) -> R<(usize, usize)> {
    let s = feature_stmt(prog, id)?;
    let k = prog.iter().position(|x| std::ptr::eq(x, s)).expect("found");
    let mut j = k;
    while j > 0
        && matches!(&prog[j - 1].kind, StmtKind::Param { owner, .. } if owner.first().is_some_and(|o| o.name == id))
    {
        j -= 1;
    }
    Ok((j, k))
}

/// Every name an expression or statement mentions.
fn mentions_expr(e: &Expr, out: &mut BTreeSet<String>) {
    match &e.kind {
        ExprKind::Name(n) => {
            out.insert(n.clone());
        }
        ExprKind::List(xs) | ExprKind::Tuple(xs) => xs.iter().for_each(|x| mentions_expr(x, out)),
        ExprKind::Map(xs) => xs.iter().for_each(|x| mentions_expr(&x.1, out)),
        ExprKind::Comp { item, iter, cond, .. } => {
            mentions_expr(item, out);
            mentions_expr(iter, out);
            if let Some(c) = cond {
                mentions_expr(c, out);
            }
        }
        ExprKind::Range { lo, hi, .. } | ExprKind::Bin(_, lo, hi) | ExprKind::Index(lo, hi) => {
            mentions_expr(lo, out);
            mentions_expr(hi, out);
        }
        ExprKind::Neg(a) | ExprKind::Not(a) | ExprKind::Attr(a, _) => mentions_expr(a, out),
        ExprKind::Call(f, args) => {
            mentions_expr(f, out);
            args.iter().for_each(|a| mentions_expr(&a.value, out));
        }
        ExprKind::If { cond, then, other } => {
            mentions_expr(cond, out);
            mentions_expr(then, out);
            mentions_expr(other, out);
        }
        _ => {}
    }
}

pub fn mentions(s: &Stmt) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    syntax::walk(std::slice::from_ref(s), &mut |st| match &st.kind {
        StmtKind::Shared { value, .. }
        | StmtKind::Param { value, .. }
        | StmtKind::Assign { value, .. }
        | StmtKind::Expr(value)
        | StmtKind::Return(value) => mentions_expr(value, &mut out),
        StmtKind::For { iter, .. } => mentions_expr(iter, &mut out),
        StmtKind::If { cond, .. } => mentions_expr(cond, &mut out),
        StmtKind::Block { args, .. } => args.iter().for_each(|a| mentions_expr(&a.value, &mut out)),
        StmtKind::Expect { args, cmp, at, .. } => {
            args.iter().for_each(|a| mentions_expr(a, &mut out));
            if let Some((_, v, _)) = cmp {
                mentions_expr(v, &mut out);
            }
            at.iter().for_each(|(_, v)| mentions_expr(v, &mut out));
        }
        StmtKind::Fn { .. } => {}
    });
    out
}

/// The top-level names statements bind (a loop's body binds its names at top level too).
pub fn bound(stmts: &[Stmt]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for s in stmts {
        match &s.kind {
            StmtKind::Assign { name, .. } | StmtKind::Block { name, .. } | StmtKind::Shared { name, .. } => {
                out.insert(name.clone());
            }
            StmtKind::Fn { name, .. } => {
                out.insert(name.clone());
            }
            StmtKind::For { body, .. } => out.extend(bound(body)),
            StmtKind::If { then, other, .. } => {
                out.extend(bound(then));
                out.extend(bound(other.as_deref().unwrap_or(&[])));
            }
            _ => {}
        }
    }
    out
}

/// The proven pre-check for adding, deleting or moving statements: none of the names they bind
/// may be read after the point where they would stop (or start) being bound, nor in any
/// function body. Returns the first reader as a typed error.
fn unread(prog: &[Stmt], names: &BTreeSet<String>, from: usize, to: usize) -> R<()> {
    for (k, s) in prog.iter().enumerate() {
        let in_fn = matches!(s.kind, StmtKind::Fn { .. });
        if !(in_fn || (from..to).contains(&k)) {
            continue;
        }
        let m = match &s.kind {
            StmtKind::Fn { body, .. } => body.iter().flat_map(mentions).collect(),
            _ => mentions(s),
        };
        if let Some(n) = names.iter().find(|n| m.contains(*n)) {
            return Err(Error::StillRead {
                name: n.clone(),
                reader: stmt_name(s)
                    .map_or_else(|| format!("statement at byte {}", s.span.lo), str::to_string),
                span: s.span,
            });
        }
    }
    Ok(())
}

/// The names a sketch block's rows bind (in its loops too).
fn sketch_names(s: &Stmt) -> Option<BTreeSet<String>> {
    match &s.kind {
        StmtKind::Block {
            kind: syntax::BlockKind::Sketch,
            body,
            ..
        } => Some(bound(body)),
        _ => None,
    }
}

/// Every `id.NAME` an expression reads.
fn members_read(e: &Expr, id: &str, out: &mut BTreeSet<String>) {
    if let ExprKind::Attr(base, m) = &e.kind
        && matches!(&base.kind, ExprKind::Name(n) if n == id)
    {
        out.insert(m.clone());
    }
    let mut sub = |x: &Expr| members_read(x, id, out);
    match &e.kind {
        ExprKind::List(xs) | ExprKind::Tuple(xs) => xs.iter().for_each(sub),
        ExprKind::Map(xs) => xs.iter().for_each(|x| sub(&x.1)),
        ExprKind::Comp { item, iter, cond, .. } => {
            sub(item);
            sub(iter);
            if let Some(c) = cond {
                sub(c);
            }
        }
        ExprKind::Range { lo, hi, .. } | ExprKind::Bin(_, lo, hi) | ExprKind::Index(lo, hi) => {
            sub(lo);
            sub(hi);
        }
        ExprKind::Neg(a) | ExprKind::Not(a) | ExprKind::Attr(a, _) => sub(a),
        ExprKind::Call(f, args) => {
            sub(f);
            args.iter().for_each(|a| sub(&a.value));
        }
        ExprKind::If { cond, then, other } => {
            sub(cond);
            sub(then);
            sub(other);
        }
        _ => {}
    }
}

/// A sketch's replacement keeps every entity a later statement reads by name (`sk.p`,
/// `sk.r[2]`): one it takes out is `EntityStillRead`, naming the reader.
fn sketch_entities_kept(prog: &[Stmt], k: usize, id: &str, new: &[Stmt]) -> R<()> {
    let Some(old) = sketch_names(&prog[k]) else {
        return Ok(());
    };
    let kept: BTreeSet<String> = new.iter().filter_map(sketch_names).flatten().collect();
    for s in &prog[k + 1..] {
        let mut read = BTreeSet::new();
        syntax::walk(std::slice::from_ref(s), &mut |st| match &st.kind {
            StmtKind::Shared { value, .. }
            | StmtKind::Param { value, .. }
            | StmtKind::Assign { value, .. }
            | StmtKind::Expr(value)
            | StmtKind::Return(value) => members_read(value, id, &mut read),
            StmtKind::For { iter, .. } => members_read(iter, id, &mut read),
            StmtKind::If { cond, .. } => members_read(cond, id, &mut read),
            StmtKind::Block { args, .. } => args.iter().for_each(|a| members_read(&a.value, id, &mut read)),
            _ => {}
        });
        if let Some(e) = read.iter().find(|e| old.contains(*e) && !kept.contains(*e)) {
            return Err(Error::EntityStillRead {
                sketch: id.into(),
                entity: e.clone(),
                reader: stmt_name(s)
                    .map_or_else(|| format!("statement at byte {}", s.span.lo), str::to_string),
                span: s.span,
            });
        }
    }
    Ok(())
}

/// The pre-checks every op makes, before it touches text or tree.
fn precheck(prog: &[Stmt], op: &Op, dim_of: &dyn Fn(&ParamKey) -> Option<Dim>) -> R<()> {
    let taken = |key: ParamKey| -> R<()> {
        if param_stmt(prog, &key).is_some() {
            return Err(Error::NameTaken {
                name: key.to_string(),
            });
        }
        Ok(())
    };
    let known = |p: &ParamKey| {
        param_stmt(prog, p)
            .map(|_| ())
            .ok_or(Error::UnknownParam { param: p.clone() })
    };
    match op {
        Op::SetParam { param, value } => {
            // a used file's param is edited in that file, never in this one
            if param_stmt(prog, param).is_none()
                && let Some(e) = crate::uses::edit_target(prog, param)
            {
                return Err(e);
            }
            known(param)?;
            if !value.value.is_finite() {
                return Err(Error::NotFinite {
                    param: param.clone(),
                    value: value.value,
                });
            }
            match dim_of(param) {
                Some(d) if d != value.dim && d != Dim::Any => Err(Error::EditDim {
                    param: param.clone(),
                    want: d,
                    found: value.dim,
                }),
                _ => Ok(()),
            }
        }
        Op::AddLocalParam { feature, name, value } => {
            if !value.value.is_finite() {
                return Err(Error::NotFinite {
                    param: ParamKey {
                        owner: feature.clone(),
                        name: name.clone(),
                    },
                    value: value.value,
                });
            }
            block(prog, &feature[0].name)?;
            taken(ParamKey {
                owner: feature.clone(),
                name: name.clone(),
            })
        }
        Op::MakeShared { param } => {
            known(param)?;
            if param.is_shared() {
                return Err(Error::NameTaken {
                    name: param.name.clone(),
                });
            }
            taken(ParamKey::shared(&param.name))
        }
        Op::SplitInstance { id, param } => {
            known(param)?;
            taken(ParamKey {
                owner: id.clone(),
                name: param.name.clone(),
            })
        }
        Op::LiftLiteral { span, name, scope } => {
            prog.iter()
                .find(|s| s.span.lo <= span.lo && span.hi <= s.span.hi)
                .ok_or(Error::EmptySelection { span: *span })?;
            taken(match scope {
                Scope::Shared => ParamKey::shared(name),
                Scope::Local(o) => ParamKey {
                    owner: o.clone(),
                    name: name.clone(),
                },
            })
        }
        Op::DeleteFeature { id } => {
            let (j, k) = block(prog, id)?;
            unread(prog, &bound(&prog[j..=k]), k + 1, prog.len())
        }
        Op::ReplaceFeature { id, text } => {
            let (_, k) = block(prog, id)?;
            let new = syntax::parse(&format!("{}\n", text.trim_end()))?;
            sketch_entities_kept(prog, k, id, &new)
        }
        Op::MoveFeature { id, before } => {
            let (j, k) = block(prog, id)?;
            let to = match before {
                Some(b) => block(prog, b)?.0,
                None => prog.len(),
            };
            if to > k {
                // down: nothing between may read what it binds
                unread(prog, &bound(&prog[j..=k]), k + 1, to)
            } else {
                // up: it may read nothing that what it passes binds
                let passed = bound(&prog[to..j]);
                let reads: BTreeSet<String> = prog[j..=k].iter().flat_map(mentions).collect();
                if let Some(n) = passed.iter().find(|n| reads.contains(*n)) {
                    return Err(Error::StillRead {
                        name: n.clone(),
                        reader: id.clone(),
                        span: prog[k].span,
                    });
                }
                unread(prog, &bound(&prog[j..=k]), prog.len(), prog.len())
            }
        }
        Op::SetArg { id, text, .. } => crate::edit_ops::precheck_set_arg(prog, id, text),
        Op::Rename { from, to } => crate::edit_ops::precheck_rename(prog, from, to),
        Op::SetProperty { .. } => Ok(()),
        Op::AddFeature { before, text } => {
            let new = syntax::parse(text)?;
            let names = bound(&new);
            let all = bound(prog);
            if let Some(n) = names.iter().find(|n| all.contains(*n)) {
                return Err(Error::NameTaken { name: n.clone() });
            }
            let at = match before {
                Some(b) => block(prog, b)?.0,
                None => prog.len(),
            };
            // what it reads must be bound above it
            let reads: BTreeSet<String> = new.iter().flat_map(mentions).collect();
            let later = bound(&prog[at..]);
            if let Some(n) = reads.iter().find(|n| later.contains(*n) && !names.contains(*n)) {
                return Err(Error::StillRead {
                    name: n.clone(),
                    reader: names.iter().next().cloned().unwrap_or_default(),
                    span: Span::default(),
                });
            }
            unread(prog, &names, at, prog.len())
        }
    }
}

fn splice(src: &str, lo: u32, hi: u32, text: &str) -> String {
    let (lo, hi) = (lo as usize, hi as usize);
    let mut out = String::with_capacity(src.len() + text.len());
    out.push_str(&src[..lo]);
    out.push_str(text);
    out.push_str(&src[hi..]);
    out
}

/// The dimension a param's definition gives, from a check of the document.
pub(crate) fn param_dim(src: &str, key: &ParamKey) -> Option<Dim> {
    let opts = crate::Options {
        build: false,
        ..crate::Options::default()
    };
    let (run, _) = crate::run(src, opts, Default::default());
    run.shared
        .iter()
        .chain(&run.locals)
        .find(|p| &p.key == key)
        .and_then(|p| match &p.value {
            crate::eval::Value::Num(n) => Some(n.dim),
            _ => None,
        })
}

/// `Rename` of a reserved word: on the tokens, so it mends a text that no longer parses.
fn rename(src: &str, from: &str, to: &str) -> R<String> {
    let toks = syntax::lex(src)?;
    if !syntax::name_uses(&toks, to).is_empty() || syntax::KEYWORDS.contains(&to) {
        return Err(Error::NameTaken { name: to.into() });
    }
    let uses = syntax::name_uses(&toks, from);
    if uses.is_empty() {
        return Err(Error::UnknownName {
            span: crate::syntax::Span { lo: 0, hi: 0 },
            name: from.into(),
        });
    }
    let mut out = String::with_capacity(src.len() + uses.len() * to.len());
    let mut at = 0usize;
    for sp in uses {
        out.push_str(&src[at..sp.lo as usize]);
        out.push_str(to);
        at = sp.hi as usize;
    }
    out.push_str(&src[at..]);
    Ok(out)
}

pub fn apply(src: &str, op: &Op) -> R<String> {
    apply_with(src, op, &|k| param_dim(src, k))
}

/// `apply`, a param's dimension in `src` asked of `dim_of` (one who already evaluated `src`).
pub(crate) fn apply_with(src: &str, op: &Op, dim_of: &dyn Fn(&ParamKey) -> Option<Dim>) -> R<String> {
    // a reserved word used as a name: the text does not parse, so the rename is on its tokens
    if let Op::Rename { from, to } = op
        && syntax::RESERVED.contains(&from.as_str())
    {
        return rename(src, from, to);
    }
    let prog = syntax::parse(src)?;

    // a feature of a component its placements read from outside (`l1.plate` where l1 places
    // lid): taking it away is refused, every reader named (R2)
    if let Op::DeleteFeature { id } = op
        && let Some((comp, feat)) = component_feature(&prog, id)
    {
        let readers = crate::refactor::occurrence_readers(&prog, &comp, &feat);
        if !readers.is_empty() {
            return Err(crate::asm::comp_error(
                Span::default(),
                crate::asm::CompProblem::StillReadBy {
                    name: id.clone(),
                    readers,
                },
            ));
        }
    }
    apply_scoped(src, &prog, op, None, dim_of)
}

/// The op where its id is: inside a component (at any depth), the same op on its statements.
fn apply_scoped(
    src: &str,
    prog: &[Stmt],
    op: &Op,
    end: Option<u32>,
    dim_of: &dyn Fn(&ParamKey) -> Option<Dim>,
) -> R<String> {
    // a component's default (`SetArg {id: "cup", arg: "d"}`): its header, put in if missing
    if let Op::SetArg { id, arg, text } = op
        && let Some(k) = component(prog, id)
    {
        return crate::edit_ops::apply_component_arg(src, &prog[k], arg, text);
    }
    // a literal default's handle (`cup.d`): the same edit
    if let Op::SetParam { param, value } = op
        && let [c] = &param.owner[..]
        && c.index.is_empty()
        && let Some(k) = component(prog, &c.name)
    {
        return crate::edit_ops::apply_component_arg(src, &prog[k], &param.name, &value.text());
    }
    if let Some((k, inner)) = scoped(prog, op)? {
        let StmtKind::Block { body, .. } = &prog[k].kind else {
            unreachable!("a component")
        };
        // a component's own param dimensions are its evaluation's: not checked against the document's
        return apply_scoped(src, body, &inner, Some(prog[k].span.hi - 1), &|_| None);
    }
    precheck(prog, op, dim_of)?;
    apply_in(src, prog, op, end, dim_of)
}

/// Whether an op edits a component's header (its defaults), at any depth.
fn on_header(prog: &[Stmt], op: &Op) -> bool {
    let here = match op {
        Op::SetArg { id, .. } => component(prog, id).is_some(),
        Op::SetParam { param, .. } => {
            matches!(&param.owner[..], [c] if c.index.is_empty() && component(prog, &c.name).is_some())
        }
        _ => false,
    };
    here || matches!(scoped(prog, op), Ok(Some((k, inner))) if match &prog[k].kind {
        StmtKind::Block { body, .. } => on_header(body, &inner),
        _ => false,
    })
}

/// A component's feature an id names, at any depth (`box.lid.plate`: `lid`, `plate`).
fn component_feature(prog: &[Stmt], id: &str) -> Option<(String, String)> {
    let (head, rest) = id.split_once('.')?;
    let k = component(prog, head)?;
    let StmtKind::Block { body, .. } = &prog[k].kind else {
        return None;
    };
    if let Some(inner) = component_feature(body, rest) {
        return Some(inner);
    }
    let feat = rest.split(['.', '[']).next()?.to_string();
    Some((head.to_string(), feat))
}

/// The top-level `component NAME(..) { .. }` named `name`.
fn component(prog: &[Stmt], name: &str) -> Option<usize> {
    prog.iter().position(|s| {
        matches!(&s.kind, StmtKind::Block { kind: syntax::BlockKind::Component | syntax::BlockKind::Harness, name: n, .. } if n == name)
    })
}

/// An id inside a component (`cup.body`): the component and the id within (`body`; `cup.` is
/// its end).
fn inner_id(prog: &[Stmt], id: &str) -> Option<(usize, Option<String>)> {
    let (head, rest) = id.split_once('.')?;
    let k = component(prog, head)?;
    Some((k, (!rest.is_empty()).then(|| rest.to_string())))
}

/// An op on a component's statements, as the same op on its body: the component and the op
/// with its ids taken inside. Ops naming two places must name one component.
fn scoped(prog: &[Stmt], op: &Op) -> R<Option<(usize, Op)>> {
    let inner = |id: &str| -> R<Option<(usize, String)>> {
        match inner_id(prog, id) {
            Some((k, Some(x))) => Ok(Some((k, x))),
            Some((_, None)) => Err(Error::UnknownFeature { id: id.into() }),
            None => Ok(None),
        }
    };
    let owner = |o: &[Seg]| {
        o.first()
            .and_then(|f| component(prog, &f.name).filter(|_| o.len() > 1 || f.index.is_empty()))
    };
    let strip = |key: &ParamKey| ParamKey {
        owner: key.owner[1..].to_vec(),
        name: key.name.clone(),
    };
    let other = |k: usize, before: &Option<String>| -> R<Option<String>> {
        match before {
            None => Ok(None),
            Some(b) => match inner_id(prog, b) {
                Some((j, x)) if j == k => Ok(x),
                _ => Err(Error::UnknownFeature { id: b.clone() }),
            },
        }
    };
    Ok(match op {
        Op::DeleteFeature { id } => inner(id)?.map(|(k, id)| (k, Op::DeleteFeature { id })),
        Op::ReplaceFeature { id, text } => inner(id)?.map(|(k, id)| {
            (
                k,
                Op::ReplaceFeature {
                    id,
                    text: text.clone(),
                },
            )
        }),
        Op::SetArg { id, arg, text } => inner(id)?.map(|(k, id)| {
            (
                k,
                Op::SetArg {
                    id,
                    arg: arg.clone(),
                    text: text.clone(),
                },
            )
        }),
        Op::MoveFeature { id, before } => match inner(id)? {
            Some((k, id)) => Some((
                k,
                Op::MoveFeature {
                    id,
                    before: other(k, before)?,
                },
            )),
            None => None,
        },
        Op::AddFeature {
            before: Some(b),
            text,
        } => inner_id(prog, b).map(|(k, x)| {
            (
                k,
                Op::AddFeature {
                    before: x,
                    text: text.clone(),
                },
            )
        }),
        Op::SetParam { param, value } if param.owner.len() > 1 => owner(&param.owner).map(|k| {
            (
                k,
                Op::SetParam {
                    param: strip(param),
                    value: *value,
                },
            )
        }),
        Op::AddLocalParam { feature, name, value } if feature.len() > 1 => owner(feature).map(|k| {
            (
                k,
                Op::AddLocalParam {
                    feature: feature[1..].to_vec(),
                    name: name.clone(),
                    value: *value,
                },
            )
        }),
        Op::LiftLiteral {
            span,
            name,
            scope: Scope::Local(o),
        } if o.len() > 1 => owner(o).map(|k| {
            (
                k,
                Op::LiftLiteral {
                    span: *span,
                    name: name.clone(),
                    scope: Scope::Local(o[1..].to_vec()),
                },
            )
        }),
        _ => None,
    })
}

/// The op on `prog` (the document, or a component's statements ending at `end`, its `}`).
fn apply_in(
    src: &str,
    prog: &[Stmt],
    op: &Op,
    end: Option<u32>,
    dim_of: &dyn Fn(&ParamKey) -> Option<Dim>,
) -> R<String> {
    let prog = prog.to_vec();
    match op {
        Op::SetArg { id, arg, text } => crate::edit_ops::apply_set_arg(src, &prog, id, arg, text),
        Op::Rename { from, to } => crate::edit_ops::apply_rename(src, from, to),
        Op::SetProperty { key, text } => crate::props::apply_set(src, &prog, key, text.as_deref()),
        Op::SetParam { param, value } => {
            let s = param_stmt(&prog, param).ok_or(Error::UnknownParam { param: param.clone() })?;
            if let Some(d) = dim_of(param)
                && d != value.dim
                && d != Dim::Any
            {
                return Err(Error::EditDim {
                    param: param.clone(),
                    want: d,
                    found: value.dim,
                });
            }
            let r = rhs(s).span;
            Ok(splice(src, r.lo, r.hi, &value.text()))
        }
        Op::AddLocalParam { feature, name, value } => {
            let key = ParamKey {
                owner: feature.clone(),
                name: name.clone(),
            };
            if param_stmt(&prog, &key).is_some() {
                return Err(Error::NameTaken {
                    name: key.to_string(),
                });
            }
            let at = feature_stmt(&prog, &feature[0].name)?;
            let line = format!("param {} = {}\n", key, value.text());
            Ok(splice(src, at.span.lo, at.span.lo, &line))
        }
        Op::MakeShared { param } => {
            let s = param_stmt(&prog, param).ok_or(Error::UnknownParam { param: param.clone() })?;
            if param.is_shared() {
                return Err(Error::NameTaken {
                    name: param.name.clone(),
                });
            }
            if param_stmt(&prog, &ParamKey::shared(&param.name)).is_some() {
                return Err(Error::NameTaken {
                    name: param.name.clone(),
                });
            }
            let r = rhs(s).span;
            let text = format!("shared {} = {}", param.name, &src[r.lo as usize..r.hi as usize]);
            Ok(splice(src, s.span.lo, s.span.hi, &text))
        }
        Op::SplitInstance { id, param } => {
            let s = param_stmt(&prog, param).ok_or(Error::UnknownParam { param: param.clone() })?;
            let key = ParamKey {
                owner: id.clone(),
                name: param.name.clone(),
            };
            if param_stmt(&prog, &key).is_some() {
                return Err(Error::NameTaken {
                    name: key.to_string(),
                });
            }
            let r = rhs(s).span;
            let line = format!("param {} = {}\n", key, &src[r.lo as usize..r.hi as usize]);
            Ok(splice(src, s.full.hi, s.full.hi, &line))
        }
        Op::AddFeature { before, text } => {
            let body = text.trim_end().to_string() + "\n";
            syntax::parse(&body)?;
            match before {
                Some(b) => {
                    let at = &prog[block(&prog, b)?.0];
                    Ok(splice(src, at.span.lo, at.span.lo, &body))
                }
                None => match end {
                    Some(e) => Ok(splice(src, e, e, &indented(&body))),
                    None => {
                        let sep = if src.is_empty() || src.ends_with('\n') {
                            ""
                        } else {
                            "\n"
                        };
                        Ok(format!("{src}{sep}{body}"))
                    }
                },
            }
        }
        Op::DeleteFeature { id } => {
            let (j, k) = block(&prog, id)?;
            Ok(splice(src, prog[j].full.lo, prog[k].full.hi, ""))
        }
        Op::ReplaceFeature { id, text } => {
            let (j, k) = block(&prog, id)?;
            // inside a component: its later lines at the statement's own indent
            let lo = prog[j].span.lo as usize;
            let indent: String = src[src[..lo].rfind('\n').map_or(0, |n| n + 1)..lo]
                .chars()
                .take_while(|c| c.is_whitespace())
                .collect();
            let text = match end {
                Some(_) if !indent.is_empty() => text.trim_end().replace('\n', &format!("\n{indent}")),
                _ => text.trim_end().to_string(),
            };
            Ok(splice(src, prog[j].span.lo, prog[k].span.hi, &text))
        }
        Op::MoveFeature { id, before } => {
            let (j, k) = block(&prog, id)?;
            let (lo0, hi0) = (prog[j].full.lo, prog[k].full.hi);
            let mut text = src[lo0 as usize..hi0 as usize].to_string();
            if !text.ends_with('\n') {
                text.push('\n');
            }
            let at = match before {
                Some(b) => Some(prog[block(&prog, b)?.0].span.lo),
                None => end,
            };
            let removed = splice(src, lo0, hi0, "");
            let len = hi0 - lo0;
            match at {
                Some(lo) => {
                    let lo = if lo > lo0 { lo - len } else { lo };
                    Ok(splice(&removed, lo, lo, &text))
                }
                None => {
                    let sep = if removed.is_empty() || removed.ends_with('\n') {
                        ""
                    } else {
                        "\n"
                    };
                    Ok(format!("{removed}{sep}{text}"))
                }
            }
        }
        Op::LiftLiteral { span, name, scope } => {
            let lit = &src[span.lo as usize..span.hi as usize];
            let top = prog
                .iter()
                .find(|s| s.span.lo <= span.lo && span.hi <= s.span.hi)
                .ok_or(Error::EmptySelection { span: *span })?;
            let def = match scope {
                Scope::Shared => format!("shared {name} = {lit}\n"),
                Scope::Local(o) => format!("param {}.{name} = {lit}\n", path_text(o)),
            };
            let key = match scope {
                Scope::Shared => ParamKey::shared(name),
                Scope::Local(o) => ParamKey {
                    owner: o.clone(),
                    name: name.clone(),
                },
            };
            if param_stmt(&prog, &key).is_some() {
                return Err(Error::NameTaken {
                    name: key.to_string(),
                });
            }
            let replaced = splice(src, span.lo, span.hi, name);
            Ok(splice(&replaced, top.span.lo, top.span.lo, &def))
        }
    }
}

/// Statements as a component's last lines: two spaces in, on a line of their own.
fn indented(body: &str) -> String {
    body.lines().map(|l| format!("  {l}\n")).collect()
}

fn lit_expr(l: &Literal) -> Expr {
    let num = |v: f64, u: Unit| Expr {
        kind: ExprKind::Num(v, u),
        span: Span::default(),
    };
    let call = |f: &str, v: f64| Expr {
        kind: ExprKind::Call(
            Box::new(Expr {
                kind: ExprKind::Name(f.into()),
                span: Span::default(),
            }),
            vec![Arg {
                name: None,
                value: num(v, Unit::None),
            }],
        ),
        span: Span::default(),
    };
    match l.dim {
        Dim::Angle => num(l.value, Unit::Deg),
        Dim::Count => call("count", l.value),
        Dim::Ratio => call("ratio", l.value),
        _ => num(l.value, Unit::None),
    }
}

/// The same op on the syntax tree.
pub fn apply_ast(src: &str, op: &Op) -> R<Vec<Stmt>> {
    let reserved = matches!(op, Op::Rename { from, .. } if syntax::RESERVED.contains(&from.as_str()));
    if reserved || matches!(op, Op::SetProperty { .. }) {
        return syntax::parse(&apply(src, op)?);
    }
    let mut prog = syntax::parse(src)?;
    // an edit of a component's header (its defaults), at any depth: the text's edit, read back
    if on_header(&prog, op) {
        return syntax::parse(&apply(src, op)?);
    }
    let dim_of = |k: &ParamKey| param_dim(src, k);
    ast_scoped(&mut prog, op, &dim_of)?;
    Ok(prog)
}

fn ast_scoped(prog: &mut Vec<Stmt>, op: &Op, dim_of: &dyn Fn(&ParamKey) -> Option<Dim>) -> R<()> {
    if let Some((k, inner)) = scoped(prog, op)? {
        let StmtKind::Block { body, .. } = &mut prog[k].kind else {
            unreachable!("a component")
        };
        return ast_scoped(body, &inner, &|_| None);
    }
    precheck(prog, op, dim_of)?;
    ast_in(prog, op)
}

fn ast_in(prog: &mut Vec<Stmt>, op: &Op) -> R<()> {
    let find_param = |prog: &[Stmt], key: &ParamKey| {
        prog.iter().position(|s| match &s.kind {
            StmtKind::Shared { name, .. } => key.owner.is_empty() && *name == key.name,
            StmtKind::Param { owner, name, .. } => *owner == key.owner && *name == key.name,
            _ => false,
        })
    };
    let find_feat = |prog: &[Stmt], id: &str| -> R<usize> { Ok(block(prog, id)?.0) };
    let blank = |kind: StmtKind| Stmt {
        kind,
        span: Span::default(),
        full: Span::default(),
    };
    match op {
        Op::SetArg { id, arg, text } => crate::edit_ops::ast_set_arg(prog, id, arg, text)?,
        Op::Rename { from, to } => crate::edit_ops::ast_rename(prog, from, to),
        Op::SetProperty { .. } => unreachable!("parsed from its text above"),
        Op::SetParam { param, value } => {
            let k = find_param(prog, param).ok_or(Error::UnknownParam { param: param.clone() })?;
            match &mut prog[k].kind {
                StmtKind::Shared { value: v, .. } | StmtKind::Param { value: v, .. } => *v = lit_expr(value),
                _ => unreachable!("a param"),
            }
        }
        Op::AddLocalParam { feature, name, value } => {
            let k = block(prog, &feature[0].name)?.1;
            prog.insert(
                k,
                blank(StmtKind::Param {
                    owner: feature.clone(),
                    name: name.clone(),
                    value: lit_expr(value),
                }),
            );
        }
        Op::MakeShared { param } => {
            let k = find_param(prog, param).ok_or(Error::UnknownParam { param: param.clone() })?;
            let v = rhs(&prog[k]).clone();
            prog[k].kind = StmtKind::Shared {
                name: param.name.clone(),
                value: v,
            };
        }
        Op::SplitInstance { id, param } => {
            let k = find_param(prog, param).ok_or(Error::UnknownParam { param: param.clone() })?;
            let v = rhs(&prog[k]).clone();
            prog.insert(
                k + 1,
                blank(StmtKind::Param {
                    owner: id.clone(),
                    name: param.name.clone(),
                    value: v,
                }),
            );
        }
        Op::AddFeature { before, text } => {
            let new = syntax::parse(text)?;
            let k = match before {
                Some(b) => find_feat(prog, b)?,
                None => prog.len(),
            };
            for (j, s) in new.into_iter().enumerate() {
                prog.insert(k + j, s);
            }
        }
        Op::DeleteFeature { id } => {
            let (j, k) = block(prog, id)?;
            prog.drain(j..=k);
        }
        Op::ReplaceFeature { id, text } => {
            let (j, k) = block(prog, id)?;
            let new = syntax::parse(&format!("{}\n", text.trim_end()))?;
            prog.splice(j..=k, new);
        }
        Op::MoveFeature { id, before } => {
            let (j, k) = block(prog, id)?;
            let at = match before {
                Some(b) => Some(block(prog, b)?.0),
                None => None,
            };
            let moved: Vec<Stmt> = prog.drain(j..=k).collect();
            let n = moved.len();
            let at = match at {
                Some(a) if a > k => a - n,
                Some(a) => a,
                None => prog.len(),
            };
            for (i, s) in moved.into_iter().enumerate() {
                prog.insert(at + i, s);
            }
        }
        Op::LiftLiteral { span, name, scope } => {
            let k = prog
                .iter()
                .position(|s| s.span.lo <= span.lo && span.hi <= s.span.hi)
                .ok_or(Error::EmptySelection { span: *span })?;
            let mut lit = None;
            replace_num(&mut prog[k], *span, name, &mut lit);
            let (v, u) = lit.ok_or(Error::EmptySelection { span: *span })?;
            let value = Expr {
                kind: ExprKind::Num(v, u),
                span: Span::default(),
            };
            let def = match scope {
                Scope::Shared => StmtKind::Shared {
                    name: name.clone(),
                    value,
                },
                Scope::Local(o) => StmtKind::Param {
                    owner: o.clone(),
                    name: name.clone(),
                    value,
                },
            };
            prog.insert(k, blank(def));
        }
    }
    Ok(())
}

fn replace_num(s: &mut Stmt, at: Span, name: &str, found: &mut Option<(f64, Unit)>) {
    fn ex(e: &mut Expr, at: Span, name: &str, found: &mut Option<(f64, Unit)>) {
        if e.span == at
            && let ExprKind::Num(v, u) = e.kind
        {
            *found = Some((v, u));
            e.kind = ExprKind::Name(name.into());
            return;
        }
        match &mut e.kind {
            ExprKind::List(xs) | ExprKind::Tuple(xs) => xs.iter_mut().for_each(|x| ex(x, at, name, found)),
            ExprKind::Map(xs) => xs.iter_mut().for_each(|x| ex(&mut x.1, at, name, found)),
            ExprKind::Comp { item, iter, cond, .. } => {
                ex(item, at, name, found);
                ex(iter, at, name, found);
                if let Some(c) = cond {
                    ex(c, at, name, found);
                }
            }
            ExprKind::Range { lo, hi, .. } | ExprKind::Bin(_, lo, hi) | ExprKind::Index(lo, hi) => {
                ex(lo, at, name, found);
                ex(hi, at, name, found);
            }
            ExprKind::Neg(a) | ExprKind::Not(a) | ExprKind::Attr(a, _) => ex(a, at, name, found),
            ExprKind::Call(f, args) => {
                ex(f, at, name, found);
                args.iter_mut().for_each(|a| ex(&mut a.value, at, name, found));
            }
            ExprKind::If { cond, then, other } => {
                ex(cond, at, name, found);
                ex(then, at, name, found);
                ex(other, at, name, found);
            }
            _ => {}
        }
    }
    match &mut s.kind {
        StmtKind::Shared { value, .. }
        | StmtKind::Param { value, .. }
        | StmtKind::Assign { value, .. }
        | StmtKind::Expr(value)
        | StmtKind::Return(value) => ex(value, at, name, found),
        StmtKind::For { iter, body, .. } => {
            ex(iter, at, name, found);
            body.iter_mut().for_each(|b| replace_num(b, at, name, found));
        }
        StmtKind::If { cond, then, other } => {
            ex(cond, at, name, found);
            then.iter_mut()
                .chain(other.iter_mut().flatten())
                .for_each(|b| replace_num(b, at, name, found));
        }
        StmtKind::Fn { body, .. } => body.iter_mut().for_each(|b| replace_num(b, at, name, found)),
        StmtKind::Block { args, body, .. } => {
            args.iter_mut().for_each(|a| ex(&mut a.value, at, name, found));
            body.iter_mut().for_each(|b| replace_num(b, at, name, found));
        }
        StmtKind::Expect { .. } => {}
    }
}

/// The tree with every span cleared, for comparing trees.
pub fn strip(prog: &[Stmt]) -> Vec<Stmt> {
    fn ex(e: &Expr) -> Expr {
        let b = |x: &Expr| Box::new(ex(x));
        let kind = match &e.kind {
            ExprKind::List(xs) => ExprKind::List(xs.iter().map(ex).collect()),
            ExprKind::Tuple(xs) => ExprKind::Tuple(xs.iter().map(ex).collect()),
            ExprKind::Map(xs) => ExprKind::Map(xs.iter().map(|(k, x)| (k.clone(), ex(x))).collect()),
            ExprKind::Comp {
                item,
                vars,
                iter,
                cond,
            } => ExprKind::Comp {
                item: b(item),
                vars: vars.clone(),
                iter: b(iter),
                cond: cond.as_ref().map(|c| b(c)),
            },
            ExprKind::Range { lo, hi, inclusive } => ExprKind::Range {
                lo: b(lo),
                hi: b(hi),
                inclusive: *inclusive,
            },
            ExprKind::Bin(o, x, y) => ExprKind::Bin(*o, b(x), b(y)),
            ExprKind::Neg(a) => ExprKind::Neg(b(a)),
            ExprKind::Not(a) => ExprKind::Not(b(a)),
            ExprKind::Call(f, args) => ExprKind::Call(
                b(f),
                args.iter()
                    .map(|a| Arg {
                        name: a.name.as_ref().map(|(n, _)| (n.clone(), Span::default())),
                        value: ex(&a.value),
                    })
                    .collect(),
            ),
            ExprKind::Attr(a, n) => ExprKind::Attr(b(a), n.clone()),
            ExprKind::Index(a, i) => ExprKind::Index(b(a), b(i)),
            ExprKind::If { cond, then, other } => ExprKind::If {
                cond: b(cond),
                then: b(then),
                other: b(other),
            },
            k => k.clone(),
        };
        Expr {
            kind,
            span: Span::default(),
        }
    }
    fn st(s: &Stmt) -> Stmt {
        let kind = match &s.kind {
            StmtKind::Shared { name, value } => StmtKind::Shared {
                name: name.clone(),
                value: ex(value),
            },
            StmtKind::Param { owner, name, value } => StmtKind::Param {
                owner: owner.clone(),
                name: name.clone(),
                value: ex(value),
            },
            StmtKind::Fn { name, params, body } => StmtKind::Fn {
                name: name.clone(),
                params: params
                    .iter()
                    .map(|p| syntax::FnParam {
                        name: p.name.clone(),
                        ty: p.ty.clone(),
                        default: p.default.as_ref().map(ex),
                    })
                    .collect(),
                body: strip(body),
            },
            StmtKind::For { vars, iter, body } => StmtKind::For {
                vars: vars.clone(),
                iter: ex(iter),
                body: strip(body),
            },
            StmtKind::If { cond, then, other } => StmtKind::If {
                cond: ex(cond),
                then: strip(then),
                other: other.as_ref().map(|o| strip(o)),
            },
            StmtKind::Return(e) => StmtKind::Return(ex(e)),
            StmtKind::Expect {
                metric,
                args,
                cmp,
                at,
            } => StmtKind::Expect {
                metric: metric.clone(),
                args: args.iter().map(ex).collect(),
                cmp: cmp
                    .as_ref()
                    .map(|(c, v, t)| (*c, ex(v), t.as_ref().map(|(x, p)| (ex(x), *p)))),
                at: at.iter().map(|(p, v)| (p.clone(), ex(v))).collect(),
            },
            StmtKind::Block {
                kind,
                name,
                args,
                body,
            } => StmtKind::Block {
                kind: *kind,
                name: name.clone(),
                args: args
                    .iter()
                    .map(|a| Arg {
                        name: a.name.as_ref().map(|(n, _)| (n.clone(), Span::default())),
                        value: ex(&a.value),
                    })
                    .collect(),
                body: strip(body),
            },
            StmtKind::Assign { name, value } => StmtKind::Assign {
                name: name.clone(),
                value: ex(value),
            },
            StmtKind::Expr(e) => StmtKind::Expr(ex(e)),
        };
        Stmt {
            kind,
            span: Span::default(),
            full: Span::default(),
        }
    }
    prog.iter().map(st).collect()
}

/// Every editable handle: the params, with value and dimension (driving dimensions are
/// exactly these; a driven value has no handle).
pub fn handles(src: &str) -> Vec<(ParamKey, Literal)> {
    let opts = crate::Options {
        build: false,
        ..crate::Options::default()
    };
    let (run, _) = crate::run(src, opts, Default::default());
    run.shared
        .iter()
        .chain(&run.locals)
        .filter_map(|p| match &p.value {
            crate::eval::Value::Num(n) => Some((
                p.key.clone(),
                Literal {
                    value: if n.dim == Dim::Angle {
                        n.v.to_degrees()
                    } else {
                        n.v
                    },
                    dim: n.dim,
                },
            )),
            _ => None,
        })
        .collect()
}

pub fn seg(name: &str) -> Seg {
    Seg {
        name: name.into(),
        index: vec![],
    }
}

/// What a SetParam changes: how many feature dimensions the param drives or feeds, and whether
/// evaluation observed it (then the edit re-runs; otherwise it re-reads the stored formulas).
#[derive(Clone, Debug, PartialEq)]
pub struct Reach {
    pub values: usize,
    pub features: Vec<String>,
    pub structural: bool,
}

/// `apply`, then the edited document checked: an op whose result does not evaluate is refused
/// with that run's first new typed error. With the edited text, what a SetParam reaches.
pub fn apply_checked(src: &str, op: &Op) -> R<(String, Option<Reach>)> {
    apply_checked_in(src, op, None)
}

/// `apply_checked` for a document in `dir`: the files it places are read for the reach (a
/// `part(FILE, with: ..)` is a call whose values its params drive).
pub fn apply_checked_in(src: &str, op: &Op, dir: Option<std::path::PathBuf>) -> R<(String, Option<Reach>)> {
    let text = apply(src, op)?;
    let after = checked_in(src, &text, dir)?;
    let reach = match op {
        Op::SetParam { param, .. } => {
            // the param, and every param linked to it (transitively)
            let mut reach: BTreeSet<ParamKey> = [param.clone()].into();
            loop {
                let more: Vec<ParamKey> = after
                    .shared
                    .iter()
                    .chain(&after.locals)
                    .filter(|p| !reach.contains(&p.key) && p.links.iter().any(|l| reach.contains(l)))
                    .map(|p| p.key.clone())
                    .collect();
                if more.is_empty() {
                    break;
                }
                reach.extend(more);
            }
            let mut features = vec![];
            let mut values = 0;
            // a component's features too, in every evaluation, named once by their definition
            // ids (proof E2)
            let comps = after.components.iter().flat_map(|c| &c.feats);
            for f in after.feats.iter().chain(comps) {
                let n = f
                    .dims
                    .iter()
                    .filter(|d| d.num.src.params().iter().any(|p| reach.contains(p)))
                    .count();
                values += n;
                if (n > 0 || f.reads.iter().any(|p| reach.contains(p))) && !features.contains(&f.id) {
                    features.push(f.id.clone());
                }
            }
            Some(Reach {
                values,
                features,
                structural: after.observed.contains(param),
            })
        }
        _ => None,
    };
    Ok((text, reach))
}

/// The edited text's run (evaluated, not built), or its first typed error the source hadn't.
pub(crate) fn checked(src: &str, text: &str) -> R<crate::Run> {
    checked_in(src, text, None)
}

pub(crate) fn checked_in(src: &str, text: &str, dir: Option<std::path::PathBuf>) -> R<crate::Run> {
    let opts = crate::Options {
        build: false,
        dir,
        ..crate::Options::default()
    };
    let (before, _) = crate::run(src, opts.clone(), Default::default());
    let (after, _) = crate::run(text, opts, Default::default());
    // refused when it adds an error, of a kind already there too (proof: an IndexOut more is a
    // new error, not the old one): counted by kind, spans move with the edit
    let count = |r: &crate::Run, k: &str| r.errors.iter().filter(|e| e.kind() == k).count();
    if let Some(e) = after
        .errors
        .iter()
        .find(|e| count(&after, e.kind()) > count(&before, e.kind()))
    {
        return Err(e.clone());
    }
    Ok(after)
}

/// Several ops as one edit (an app's one undo step): each applied to the text the last one made,
/// the result checked once, as `apply_checked` checks one.
pub fn apply_all_checked(src: &str, ops: &[Op]) -> R<String> {
    let mut text = src.to_string();
    for op in ops {
        text = apply(&text, op)?;
    }
    checked(src, &text)?;
    Ok(text)
}
