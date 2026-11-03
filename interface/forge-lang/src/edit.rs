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
fn block(prog: &[Stmt], id: &str) -> R<(usize, usize)> {
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

/// The pre-checks every op makes, before it touches text or tree.
fn precheck(src: &str, prog: &[Stmt], op: &Op) -> R<()> {
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
            known(param)?;
            if !value.value.is_finite() {
                return Err(Error::NotFinite {
                    param: param.clone(),
                    value: value.value,
                });
            }
            match param_dim(src, param) {
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
fn param_dim(src: &str, key: &ParamKey) -> Option<Dim> {
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

pub fn apply(src: &str, op: &Op) -> R<String> {
    let prog = syntax::parse(src)?;
    precheck(src, &prog, op)?;
    match op {
        Op::SetArg { id, arg, text } => crate::edit_ops::apply_set_arg(src, &prog, id, arg, text),
        Op::Rename { from, to } => crate::edit_ops::apply_rename(src, from, to),
        Op::SetParam { param, value } => {
            let s = param_stmt(&prog, param).ok_or(Error::UnknownParam { param: param.clone() })?;
            if let Some(d) = param_dim(src, param)
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
                None => {
                    let sep = if src.is_empty() || src.ends_with('\n') {
                        ""
                    } else {
                        "\n"
                    };
                    Ok(format!("{src}{sep}{body}"))
                }
            }
        }
        Op::DeleteFeature { id } => {
            let (j, k) = block(&prog, id)?;
            Ok(splice(src, prog[j].full.lo, prog[k].full.hi, ""))
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
                None => None,
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
    let mut prog = syntax::parse(src)?;
    precheck(src, &prog, op)?;
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
        Op::SetArg { id, arg, text } => crate::edit_ops::ast_set_arg(&mut prog, id, arg, text)?,
        Op::Rename { from, to } => crate::edit_ops::ast_rename(&mut prog, from, to),
        Op::SetParam { param, value } => {
            let k = find_param(&prog, param).ok_or(Error::UnknownParam { param: param.clone() })?;
            match &mut prog[k].kind {
                StmtKind::Shared { value: v, .. } | StmtKind::Param { value: v, .. } => *v = lit_expr(value),
                _ => unreachable!("a param"),
            }
        }
        Op::AddLocalParam { feature, name, value } => {
            let k = block(&prog, &feature[0].name)?.1;
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
            let k = find_param(&prog, param).ok_or(Error::UnknownParam { param: param.clone() })?;
            let v = rhs(&prog[k]).clone();
            prog[k].kind = StmtKind::Shared {
                name: param.name.clone(),
                value: v,
            };
        }
        Op::SplitInstance { id, param } => {
            let k = find_param(&prog, param).ok_or(Error::UnknownParam { param: param.clone() })?;
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
                Some(b) => find_feat(&prog, b)?,
                None => prog.len(),
            };
            for (j, s) in new.into_iter().enumerate() {
                prog.insert(k + j, s);
            }
        }
        Op::DeleteFeature { id } => {
            let (j, k) = block(&prog, id)?;
            prog.drain(j..=k);
        }
        Op::MoveFeature { id, before } => {
            let (j, k) = block(&prog, id)?;
            let at = match before {
                Some(b) => Some(block(&prog, b)?.0),
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
    Ok(prog)
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
    let text = apply(src, op)?;
    let opts = crate::Options {
        build: false,
        ..crate::Options::default()
    };
    let (before, _) = crate::run(src, opts.clone(), Default::default());
    let (after, _) = crate::run(&text, opts, Default::default());
    let old: Vec<&str> = before.errors.iter().map(Error::kind).collect();
    if let Some(e) = after.errors.iter().find(|e| !old.contains(&e.kind())) {
        return Err(e.clone());
    }
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
            for f in &after.feats {
                let n = f
                    .dims
                    .iter()
                    .filter(|d| d.num.src.params().iter().any(|p| reach.contains(p)))
                    .count();
                values += n;
                if n > 0 || f.reads.iter().any(|p| reach.contains(p)) {
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
