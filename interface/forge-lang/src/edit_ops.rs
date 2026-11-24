//! Two edit ops beyond the first set, each a pure function of (document, op):
//! - `SetArg {id, arg, text}`: the keyword argument `arg` of the call that makes feature `id`
//!   becomes `text` (put in if the call has none). One span changes.
//! - `Rename {from, to}`: a top-level name and every use of it renamed (bindings, reads, the
//!   owners of its params); members, labels, keyword-argument names and types are not names
//!   and stay. For a name that hides a built-in.
use crate::edit::{bound, mentions_in};
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
    if let Some(n) = mentions_in(&probe, calls_closed(prog))
        .iter()
        .find(|n| later.contains(*n))
    {
        return Err(Error::StillRead {
            name: n.clone(),
            reader: id.into(),
            span: prog[k].span,
        });
    }
    Ok(())
}

/// `#K`: the call's K-th positional argument, which has no keyword to name it by.
fn positional(arg: &str) -> Option<usize> {
    arg.strip_prefix('#')?.parse().ok()
}

fn find_arg<'a>(args: &'a [Arg], arg: &str) -> Option<&'a Arg> {
    match positional(arg) {
        Some(k) => args.get(k).filter(|a| a.name.is_none()),
        None => args.iter().find(|a| a.name.as_ref().is_some_and(|n| n.0 == arg)),
    }
}

pub fn apply_set_arg(src: &str, prog: &[Stmt], id: &str, arg: &str, text: &str) -> R<String> {
    let k = call_of(prog, id)?;
    let StmtKind::Assign { value, .. } = &prog[k].kind else {
        unreachable!("an assignment")
    };
    let ExprKind::Call(_, args) = &value.kind else {
        unreachable!("a call")
    };
    let (lo, hi, new) = match find_arg(args, arg) {
        Some(a) => (a.value.span.lo, a.value.span.hi, text.to_string()),
        None if positional(arg).is_some() => {
            return Err(Error::UnknownArg {
                span: value.span,
                func: "SetArg",
                arg: arg.into(),
            });
        }
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

/// A component's param default (`component cup(d: 80)`): `arg`'s default becomes `text`, or
/// `arg: text` is put in before the header's `)`.
pub fn apply_component_arg(src: &str, s: &Stmt, arg: &str, text: &str) -> R<String> {
    let StmtKind::Block { args, body, .. } = &s.kind else {
        unreachable!("a component")
    };
    expr_of(text)?;
    if let Some(a) = args.iter().find(|a| a.name.as_ref().is_some_and(|n| n.0 == arg)) {
        let (lo, hi) = (a.value.span.lo as usize, a.value.span.hi as usize);
        return Ok(format!("{}{}{}", &src[..lo], text, &src[hi..]));
    }
    // the header's `)`: the last before the body's `{`
    let open = body.first().map_or(s.span.hi as usize, |b| b.span.lo as usize);
    let head = &src[s.span.lo as usize..open];
    let brace = head.rfind('{').unwrap_or(head.len());
    let close = s.span.lo as usize
        + head[..brace].rfind(')').ok_or(Error::UnknownArg {
            span: s.span,
            func: "SetArg",
            arg: arg.into(),
        })?;
    let sep = if args.is_empty() { "" } else { ", " };
    Ok(format!("{}{sep}{arg}: {text}{}", &src[..close], &src[close..]))
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
    if let Some(k) = positional(arg) {
        return match args.get_mut(k).filter(|a| a.name.is_none()) {
            Some(a) => {
                a.value = e;
                Ok(())
            }
            None => Err(Error::UnknownArg {
                span: Span::default(),
                func: "SetArg",
                arg: arg.into(),
            }),
        };
    }
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

/// Whether a document's calls name built-ins only, never a binding: a machining document (its
/// first statement the bare name `machining`; forge-cam-lang and forge-turn-lang look a call's
/// head up among their built-ins alone), where `drill = drill(..)` binds a name a call also
/// spells. A call's head there is not a use of the name.
pub fn calls_closed(prog: &[Stmt]) -> bool {
    matches!(prog.first().map(|s| &s.kind), Some(StmtKind::Expr(Expr { kind: ExprKind::Name(n), .. })) if n == "machining")
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

/// The spans of the identifier tokens that are uses or bindings of `from` (`closed`: a call's head
/// is a built-in, not a use, as in a machining document).
fn rename_spans(src: &str, from: &str, closed: bool) -> R<Vec<Span>> {
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
                let head = closed && op(i + 1, "(");
                if !(member || label || metric || key || ty || head) {
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
    let closed = syntax::parse(src).is_ok_and(|p| calls_closed(&p));
    let mut out = src.to_string();
    for sp in rename_spans(src, from, closed)?.iter().rev() {
        out.replace_range(sp.lo as usize..sp.hi as usize, to);
    }
    Ok(out)
}

pub fn ast_rename(prog: &mut [Stmt], from: &str, to: &str) {
    let closed = calls_closed(prog);
    fn name(n: &mut String, from: &str, to: &str) {
        if n == from {
            *n = to.into();
        }
    }
    fn ex(e: &mut Expr, from: &str, to: &str, closed: bool) {
        match &mut e.kind {
            ExprKind::Name(n) => name(n, from, to),
            ExprKind::List(xs) | ExprKind::Tuple(xs) => xs.iter_mut().for_each(|x| ex(x, from, to, closed)),
            ExprKind::Map(xs) => xs.iter_mut().for_each(|x| ex(&mut x.1, from, to, closed)),
            ExprKind::Comp {
                item,
                vars,
                iter,
                cond,
            } => {
                vars.iter_mut().for_each(|v| name(v, from, to));
                ex(item, from, to, closed);
                ex(iter, from, to, closed);
                if let Some(c) = cond {
                    ex(c, from, to, closed);
                }
            }
            ExprKind::Range { lo, hi, .. } | ExprKind::Bin(_, lo, hi) | ExprKind::Index(lo, hi) => {
                ex(lo, from, to, closed);
                ex(hi, from, to, closed);
            }
            ExprKind::Neg(a) | ExprKind::Not(a) | ExprKind::Attr(a, _) => ex(a, from, to, closed),
            ExprKind::Call(f, args) => {
                // a machining document's call heads are built-ins, never a binding
                if !(closed && matches!(f.kind, ExprKind::Name(_))) {
                    ex(f, from, to, closed);
                }
                args.iter_mut().for_each(|a| ex(&mut a.value, from, to, closed));
            }
            ExprKind::If { cond, then, other } => {
                ex(cond, from, to, closed);
                ex(then, from, to, closed);
                ex(other, from, to, closed);
            }
            _ => {}
        }
    }
    fn st(s: &mut Stmt, from: &str, to: &str, closed: bool) {
        match &mut s.kind {
            StmtKind::Shared { name: n, value } => {
                name(n, from, to);
                ex(value, from, to, closed);
            }
            StmtKind::Param { owner, value, .. } => {
                if let Some(o) = owner.first_mut() {
                    name(&mut o.name, from, to);
                }
                ex(value, from, to, closed);
            }
            StmtKind::Assign { name: n, value } => {
                name(n, from, to);
                ex(value, from, to, closed);
            }
            StmtKind::Expr(e) | StmtKind::Return(e) => ex(e, from, to, closed),
            StmtKind::Fn {
                name: n,
                params,
                body,
            } => {
                name(n, from, to);
                for p in params.iter_mut() {
                    name(&mut p.name, from, to);
                    if let Some(d) = &mut p.default {
                        ex(d, from, to, closed);
                    }
                }
                std::rc::Rc::make_mut(body)
                    .iter_mut()
                    .for_each(|b| st(b, from, to, closed));
            }
            StmtKind::For { vars, iter, body } => {
                vars.iter_mut().for_each(|v| name(v, from, to));
                ex(iter, from, to, closed);
                body.iter_mut().for_each(|b| st(b, from, to, closed));
            }
            StmtKind::If { cond, then, other } => {
                ex(cond, from, to, closed);
                then.iter_mut()
                    .chain(other.iter_mut().flatten())
                    .for_each(|b| st(b, from, to, closed));
            }
            StmtKind::Block {
                name: n, args, body, ..
            } => {
                name(n, from, to);
                args.iter_mut().for_each(|a| ex(&mut a.value, from, to, closed));
                body.iter_mut().for_each(|b| st(b, from, to, closed));
            }
            StmtKind::Expect { args, cmp, .. } => {
                args.iter_mut().for_each(|a| ex(a, from, to, closed));
                if let Some((_, v, tol)) = cmp {
                    ex(v, from, to, closed);
                    if let Some((t, _)) = tol {
                        ex(t, from, to, closed);
                    }
                }
            }
        }
    }
    prog.iter_mut().for_each(|s| st(s, from, to, closed));
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

/// `Respell {span, to}`: the name at `span` written `to` (an UnknownName's fix). The span is a
/// read of a name, the whole token: a binder (`base` in `base = ..`) is refused, as Rename's,
/// and so is a span inside a token (Respell.lean: `respell_refuses_binder`,
/// `respell_refuses_inside`).
pub fn apply_respell(src: &str, span: Span, to: &str) -> R<String> {
    let (lo, hi) = (span.lo as usize, span.hi as usize);
    if !ident(to) || syntax::KEYWORDS.contains(&to) {
        return Err(Error::NameTaken { name: to.into() });
    }
    let prog = syntax::parse(src)?;
    if !crate::edit::reads_at(&prog, span) {
        return Err(Error::NotARead {
            span,
            text: src.get(lo..hi).unwrap_or_default().into(),
        });
    }
    Ok(format!("{}{to}{}", &src[..lo], &src[hi..]))
}

/// The nearest of `names` to `name`, two edits or fewer away. At
/// equal distance a name the part binds (a param, a feature, a local, a function) comes before
/// a built-in, then the alphabetically first: `tt` in a part with a param `t` is `t`, not `pt`.
pub fn nearest<'a>(name: &str, names: impl Iterator<Item = &'a str>) -> Option<String> {
    let builtin = |n: &str| crate::builtins::BUILTINS.contains(&n) || crate::shapes::NAMES.contains(&n);
    let dist = |a: &str, b: &str| {
        let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
        let mut row: Vec<usize> = (0..=b.len()).collect();
        for i in 1..=a.len() {
            let mut prev = row[0];
            row[0] = i;
            for j in 1..=b.len() {
                let cur = row[j];
                row[j] = (row[j] + 1)
                    .min(row[j - 1] + 1)
                    .min(prev + usize::from(a[i - 1] != b[j - 1]));
                prev = cur;
            }
        }
        row[b.len()]
    };
    names
        .filter(|n| *n != name)
        .map(|n| (dist(name, n), builtin(n), n))
        .filter(|(d, _, _)| *d <= 2 && *d < name.chars().count())
        .min()
        .map(|(_, _, n)| n.to_string())
}

/// The marker a suppressed line starts with: `#~ID`, then `<` when Suppress put in the line
/// break before it (the statement shared its line with one kept before it), `>` when it put in
/// the one after it (with one kept after it), then a space (ID the feature `Suppress` named).
fn suppress_tag(id: &str, before: bool, after: bool) -> String {
    let b = if before { "<" } else { "" };
    let a = if after { ">" } else { "" };
    format!("#~{id}{b}{a} ")
}

/// A line `Suppress` tagged: its id, whether it put in the break before and after it, and the
/// line as it was.
fn tagged(line: &str) -> Option<(&str, bool, bool, &str)> {
    let rest = line.strip_prefix("#~")?;
    let n = rest.find(|c: char| !(c.is_alphanumeric() || c == '_'))?;
    let (id, rest) = rest.split_at(n);
    let (before, rest) = rest.strip_prefix('<').map_or((false, rest), |r| (true, r));
    let (after, rest) = rest.strip_prefix('>').map_or((false, rest), |r| (true, r));
    let rest = rest.strip_prefix(' ')?;
    (!id.is_empty()).then_some((id, before, after, rest))
}

/// `Suppress {id}`: the statement making top-level feature `id`, its params, and every later
/// statement reading what they bind (and their params), out of the build: each kept as a comment
/// `#~ID LINE`, so `Unsuppress {id}` puts them back as they were. Exactly those statements: one
/// sharing a line with a statement kept goes on a line of its own, the break put in marked in
/// its tag (`<` before it, `>` after it), and Unsuppress takes the break out again
/// (Suppress.lean: `suppress_live`, `unsuppress_suppress`).
pub fn apply_suppress(src: &str, prog: &[Stmt], id: &str) -> R<String> {
    if src.lines().any(|l| tagged(l).is_some_and(|t| t.0 == id)) {
        return Err(Error::NameTaken {
            name: format!("#~{id}"),
        });
    }
    let (j, k) = crate::edit::block(prog, id)?;
    let mut out: BTreeSet<usize> = (j..=k).collect();
    let mut names = bound(&prog[j..=k]);
    names.insert(id.to_string());
    let closed = calls_closed(prog);
    for m in k + 1..prog.len() {
        if mentions_in(&prog[m], closed).iter().any(|n| names.contains(n)) {
            out.insert(m);
            names.extend(bound(&prog[m..=m]));
        }
    }
    // the params of every feature taken out, wherever they are written
    for (m, s) in prog.iter().enumerate() {
        if let StmtKind::Param { owner, .. } = &s.kind
            && owner.first().is_some_and(|o| names.contains(&o.name))
        {
            out.insert(m);
        }
    }
    let blank = |t: &str| t.bytes().all(|c| c == b' ' || c == b'\t');
    let mut text = String::with_capacity(src.len() + out.len() * (id.len() + 8));
    let mut at = 0usize;
    for &m in &out {
        let s = &prog[m];
        // the statement and its separator (a `;`, or the line break)
        let (slo, shi) = (s.full.lo as usize, s.full.hi as usize);
        if slo < at {
            continue;
        }
        // from its line's start (or the break put in after the last one taken out), unless a
        // statement kept is before it on the line
        let start = src[..slo].rfind('\n').map_or(0, |n| n + 1).max(at);
        let before = !blank(&src[start..slo]);
        let lo = if before { slo } else { start };
        // to its line's end, unless a statement kept is after it on the line: then up to it
        let hi = if src[..shi].ends_with('\n') {
            shi
        } else {
            let eol = src[shi..].find('\n').map_or(src.len(), |n| shi + n + 1);
            let semis = src[shi..eol].len() - src[shi..eol].trim_start_matches(';').len();
            let rest = src[shi + semis..eol].trim();
            if rest.is_empty() || rest.starts_with('#') {
                eol
            } else {
                shi + semis
            }
        };
        let after = !src[..hi].ends_with('\n') && hi < src.len();
        text.push_str(&src[at..lo]);
        if before {
            text.push('\n');
        }
        let lines: Vec<&str> = src[lo..hi].split_inclusive('\n').collect();
        for (i, l) in lines.iter().enumerate() {
            text.push_str(&suppress_tag(id, before && i == 0, after && i + 1 == lines.len()));
            text.push_str(l);
        }
        if after {
            text.push('\n');
        }
        at = hi;
    }
    text.push_str(&src[at..]);
    Ok(text)
}

/// `Unsuppress {id}`: the lines `Suppress {id}` kept as comments put back as they were, and the
/// line breaks it put in taken out. A break next to another feature's suppressed line stays, its
/// mark moved onto that line, for that feature's Unsuppress to take out.
pub fn apply_unsuppress(src: &str, id: &str) -> R<String> {
    let lines: Vec<&str> = src.split_inclusive('\n').collect();
    if !lines.iter().any(|l| tagged(l).is_some_and(|t| t.0 == id)) {
        return Err(Error::UnknownFeature { id: id.into() });
    }
    // another feature's suppressed line, its tag written again with a mark more
    let other = |l: &str, before: bool, after: bool| {
        tagged(l)
            .filter(|t| t.0 != id)
            .map(|(o, b, a, r)| format!("{}{r}", suppress_tag(o, b || before, a || after)))
    };
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    // the line before ended in a break put in: this one joins it
    let mut glue = false;
    // the break before this line stays: its mark moves onto it
    let mut mark = false;
    for (i, &l) in lines.iter().enumerate() {
        let l = match mark {
            true => other(l, true, false).expect("another feature's suppressed line"),
            false => l.to_string(),
        };
        mark = false;
        let Some((_, before, after, rest)) = tagged(&l).filter(|t| t.0 == id) else {
            match out.last_mut() {
                Some(prev) if glue => prev.push_str(&l),
                _ => out.push(l),
            }
            glue = false;
            continue;
        };
        match out.last_mut() {
            Some(prev) if before => match other(prev, false, true) {
                Some(marked) => {
                    *prev = marked;
                    out.push(rest.to_string());
                }
                None => {
                    if prev.ends_with('\n') {
                        prev.pop();
                    }
                    prev.push_str(rest);
                }
            },
            Some(prev) if glue => prev.push_str(rest),
            _ => out.push(rest.to_string()),
        }
        glue = false;
        if after {
            if lines.get(i + 1).is_some_and(|n| other(n, false, false).is_some()) {
                mark = true;
            } else if let Some(last) = out.last_mut()
                && last.ends_with('\n')
            {
                last.pop();
                glue = true;
            }
        }
    }
    Ok(out.concat())
}
