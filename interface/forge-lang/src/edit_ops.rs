//! Two edit ops beyond the first set, each a pure function of (document, op):
//! - `SetArg {id, arg, text}`: the keyword argument `arg` of the call that makes feature `id`
//!   becomes `text` (put in if the call has none). One span changes.
//! - `Rename {from, to}`: a top-level name and every use of it renamed (bindings, reads, the
//!   owners of its params); members, labels, keyword-argument names and types are not names
//!   and stay. For a name that hides a built-in.
use crate::edit::{bound, mentions};
use crate::error::Error;
use crate::syntax::{self, Arg, Expr, ExprKind, Span, Stmt, StmtKind, TokKind};
use std::collections::BTreeSet;

type R<T> = Result<T, Error>;

/// The statement index and call of the assignment making top-level feature `id`.
fn call_of(prog: &[Stmt], id: &str) -> R<usize> {
    prog.iter()
        .position(|s| matches!(&s.kind, StmtKind::Assign { name, value } if name == id && matches!(value.kind, ExprKind::Call(..))))
        .ok_or(Error::UnknownFeature { id: id.into() })
}

/// `text` as an expression.
fn expr_of(text: &str) -> R<Expr> {
    let prog = syntax::parse(&format!("x_ = {text}"))?;
    match prog.into_iter().next().map(|s| s.kind) {
        Some(StmtKind::Assign { value, .. }) => Ok(value),
        _ => Err(Error::Syntax(crate::error::SyntaxError::Expected {
            span: Span::default(),
            what: "an expression",
            expected: crate::syntax::TermSet::default(),
        })),
    }
}

pub fn precheck_set_arg(prog: &[Stmt], id: &str, text: &str) -> R<()> {
    let k = call_of(prog, id)?;
    let e = expr_of(text)?;
    // what it reads must be bound above the statement (or be built-in)
    let probe = Stmt {
        kind: StmtKind::Expr(e),
        span: Span::default(),
        full: Span::default(),
    };
    let later = bound(&prog[k..]);
    if let Some(n) = mentions(&probe).iter().find(|n| later.contains(*n)) {
        return Err(Error::StillRead {
            name: n.clone(),
            reader: id.into(),
            span: prog[k].span,
        });
    }
    Ok(())
}

pub fn apply_set_arg(src: &str, prog: &[Stmt], id: &str, arg: &str, text: &str) -> R<String> {
    let k = call_of(prog, id)?;
    let StmtKind::Assign { value, .. } = &prog[k].kind else {
        unreachable!("an assignment")
    };
    let ExprKind::Call(_, args) = &value.kind else {
        unreachable!("a call")
    };
    let (lo, hi, new) = match args.iter().find(|a| a.name.as_ref().is_some_and(|n| n.0 == arg)) {
        Some(a) => (a.value.span.lo, a.value.span.hi, text.to_string()),
        None => {
            // before the closing parenthesis
            let close = value.span.hi - 1;
            let sep = if args.is_empty() { "" } else { ", " };
            (close, close, format!("{sep}{arg}: {text}"))
        }
    };
    let (lo, hi) = (lo as usize, hi as usize);
    Ok(format!("{}{}{}", &src[..lo], new, &src[hi..]))
}

pub fn ast_set_arg(prog: &mut [Stmt], id: &str, arg: &str, text: &str) -> R<()> {
    let k = call_of(prog, id)?;
    let e = expr_of(text)?;
    let StmtKind::Assign { value, .. } = &mut prog[k].kind else {
        unreachable!("an assignment")
    };
    let ExprKind::Call(_, args) = &mut value.kind else {
        unreachable!("a call")
    };
    match args
        .iter_mut()
        .find(|a| a.name.as_ref().is_some_and(|n| n.0 == arg))
    {
        Some(a) => a.value = e,
        None => args.push(Arg {
            name: Some((arg.into(), Span::default())),
            value: e,
        }),
    }
    Ok(())
}

fn ident(s: &str) -> bool {
    let mut c = s.chars();
    c.next().is_some_and(|f| f.is_ascii_alphabetic() || f == '_')
        && c.all(|x| x.is_ascii_alphanumeric() || x == '_')
}

pub fn precheck_rename(prog: &[Stmt], from: &str, to: &str) -> R<()> {
    let all = bound(prog);
    if !all.contains(from) {
        return Err(Error::UnknownFeature { id: from.into() });
    }
    let builtin = crate::builtins::BUILTINS.contains(&to) || crate::builtins::constant(to).is_some();
    if all.contains(to) || builtin || syntax::KEYWORDS.contains(&to) || !ident(to) {
        return Err(Error::NameTaken { name: to.into() });
    }
    Ok(())
}

/// The spans of the identifier tokens that are uses or bindings of `from`.
fn rename_spans(src: &str, from: &str) -> R<Vec<Span>> {
    let toks = syntax::lex(src)?;
    let op = |i: usize, o: &str| toks.get(i).is_some_and(|t| t.kind == TokKind::Op(leak(o)));
    let mut out = vec![];
    // inside a param's owner brackets, or a fn's header: labels and types, not names
    let (mut in_param, mut depth, mut in_fn) = (false, 0, false);
    for (i, t) in toks.iter().enumerate() {
        match &t.kind {
            TokKind::Kw("param") => in_param = true,
            TokKind::Kw("fn") => in_fn = true,
            TokKind::Op("=") if in_param => in_param = false,
            TokKind::Op("[") if in_param => depth += 1,
            TokKind::Op("]") if in_param => depth -= 1,
            TokKind::Op("{") if in_fn => in_fn = false,
            TokKind::Id(n) if n == from => {
                let prev = |o: &str| i > 0 && op(i - 1, o);
                let member = prev(".");
                let label = op(i + 1, ":") && (prev("(") || prev(",") || prev("{"));
                let metric = i > 0 && toks[i - 1].kind == TokKind::Kw("expect");
                let key = in_param && depth > 0;
                let ty = in_fn && prev(":");
                if !(member || label || metric || key || ty) {
                    out.push(t.span);
                }
            }
            _ => {}
        }
    }
    Ok(out)
}

/// The lexer's operators are static strings; compare against one of them.
fn leak(o: &str) -> &'static str {
    match o {
        "." => ".",
        ":" => ":",
        "(" => "(",
        "," => ",",
        "{" => "{",
        _ => "",
    }
}

pub fn apply_rename(src: &str, from: &str, to: &str) -> R<String> {
    let mut out = src.to_string();
    for sp in rename_spans(src, from)?.iter().rev() {
        out.replace_range(sp.lo as usize..sp.hi as usize, to);
    }
    Ok(out)
}

pub fn ast_rename(prog: &mut [Stmt], from: &str, to: &str) {
    fn name(n: &mut String, from: &str, to: &str) {
        if n == from {
            *n = to.into();
        }
    }
    fn ex(e: &mut Expr, from: &str, to: &str) {
        match &mut e.kind {
            ExprKind::Name(n) => name(n, from, to),
            ExprKind::List(xs) | ExprKind::Tuple(xs) => xs.iter_mut().for_each(|x| ex(x, from, to)),
            ExprKind::Map(xs) => xs.iter_mut().for_each(|x| ex(&mut x.1, from, to)),
            ExprKind::Comp {
                item,
                vars,
                iter,
                cond,
            } => {
                vars.iter_mut().for_each(|v| name(v, from, to));
                ex(item, from, to);
                ex(iter, from, to);
                if let Some(c) = cond {
                    ex(c, from, to);
                }
            }
            ExprKind::Range { lo, hi, .. } | ExprKind::Bin(_, lo, hi) | ExprKind::Index(lo, hi) => {
                ex(lo, from, to);
                ex(hi, from, to);
            }
            ExprKind::Neg(a) | ExprKind::Not(a) | ExprKind::Attr(a, _) => ex(a, from, to),
            ExprKind::Call(f, args) => {
                ex(f, from, to);
                args.iter_mut().for_each(|a| ex(&mut a.value, from, to));
            }
            ExprKind::If { cond, then, other } => {
                ex(cond, from, to);
                ex(then, from, to);
                ex(other, from, to);
            }
            _ => {}
        }
    }
    fn st(s: &mut Stmt, from: &str, to: &str) {
        match &mut s.kind {
            StmtKind::Shared { name: n, value } => {
                name(n, from, to);
                ex(value, from, to);
            }
            StmtKind::Param { owner, value, .. } => {
                if let Some(o) = owner.first_mut() {
                    name(&mut o.name, from, to);
                }
                ex(value, from, to);
            }
            StmtKind::Assign { name: n, value } => {
                name(n, from, to);
                ex(value, from, to);
            }
            StmtKind::Expr(e) | StmtKind::Return(e) => ex(e, from, to),
            StmtKind::Fn {
                name: n,
                params,
                body,
            } => {
                name(n, from, to);
                for p in params.iter_mut() {
                    name(&mut p.name, from, to);
                    if let Some(d) = &mut p.default {
                        ex(d, from, to);
                    }
                }
                body.iter_mut().for_each(|b| st(b, from, to));
            }
            StmtKind::For { vars, iter, body } => {
                vars.iter_mut().for_each(|v| name(v, from, to));
                ex(iter, from, to);
                body.iter_mut().for_each(|b| st(b, from, to));
            }
            StmtKind::If { cond, then, other } => {
                ex(cond, from, to);
                then.iter_mut()
                    .chain(other.iter_mut().flatten())
                    .for_each(|b| st(b, from, to));
            }
            StmtKind::Block {
                name: n, args, body, ..
            } => {
                name(n, from, to);
                args.iter_mut().for_each(|a| ex(&mut a.value, from, to));
                body.iter_mut().for_each(|b| st(b, from, to));
            }
            StmtKind::Expect { args, cmp, .. } => {
                args.iter_mut().for_each(|a| ex(a, from, to));
                if let Some((_, v, tol)) = cmp {
                    ex(v, from, to);
                    if let Some((t, _)) = tol {
                        ex(t, from, to);
                    }
                }
            }
        }
    }
    prog.iter_mut().for_each(|s| st(s, from, to));
}

/// The names a document binds at top level, for callers that suggest a new one.
pub fn free_name(prog: &[Stmt], base: &str) -> String {
    let all: BTreeSet<String> = bound(prog);
    (1..)
        .map(|k| format!("{base}_{k}"))
        .find(|n| {
            !all.contains(n)
                && crate::builtins::constant(n).is_none()
                && !crate::builtins::BUILTINS.contains(&n.as_str())
        })
        .expect("a free name")
}
