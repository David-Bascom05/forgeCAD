//! Refactors between "inline in this file" and "its own file", each all or nothing, each a pure
//! function of the document (and, to bring a file in, that file's text):
//! - `MakeComponent {statements, name}`: the statements that make some bodies become a
//!   `component NAME() { .. }` placed once (Create Components from Bodies).
//! - `ExtractComponent {component, file}`: a component becomes its own file; each call becomes
//!   `part(FILE, with: {..})`, the shared params it reads passed at every call, never copied
//!   (proof-findings 5), the functions it calls copied into the file (proof-findings 7).
//! - `InlineComponent {file, name}`: the inverse.
//!
//! Refusals are typed (`asm::CompProblem`): a moved statement reading a name left behind
//! (ReadsOutside), a statement left behind reading a moved name (StillReadBy, every reader),
//! a component read other than placed (ReadOtherwise), a file that exists (FileExists) or is a
//! link (TargetIsLink), a new name not plain (BadFileName), a file outside the project
//! (Outside), a document with no file (UntitledDocument), a file that is no plain part
//! (NotInlinable).

use crate::asm::{CompProblem, comp_error};
use crate::edit::{bound, mentions};
use crate::error::Error;
use crate::syntax::{self, Arg, BlockKind, Expr, ExprKind, Span, Stmt, StmtKind};
use forge_agent::json::Json;
use std::collections::BTreeSet;

type R<T> = Result<T, Error>;

#[derive(Clone, Debug, PartialEq)]
pub enum Refactor {
    MakeComponent { statements: Vec<String>, name: String },
    ExtractComponent { component: String, file: String },
    InlineComponent { file: String, name: String },
}

/// What a refactor writes: the document, and any file it makes (by its name, beside it).
#[derive(Clone, Debug, PartialEq)]
pub struct Written {
    pub doc: String,
    pub files: Vec<(String, String)>,
}

impl Refactor {
    pub fn json(&self) -> Json {
        match self {
            Refactor::MakeComponent { statements, name } => Json::obj(vec![
                ("op", "MakeComponent".into()),
                (
                    "statements",
                    Json::Arr(statements.iter().map(|s| Json::Str(s.clone())).collect()),
                ),
                ("name", name.as_str().into()),
            ]),
            Refactor::ExtractComponent { component, file } => Json::obj(vec![
                ("op", "ExtractComponent".into()),
                ("component", component.as_str().into()),
                ("file", file.as_str().into()),
            ]),
            Refactor::InlineComponent { file, name } => Json::obj(vec![
                ("op", "InlineComponent".into()),
                ("file", file.as_str().into()),
                ("name", name.as_str().into()),
            ]),
        }
    }

    pub fn from_json(j: &Json) -> Option<Refactor> {
        let s = |k: &str| j.get(k).and_then(Json::str).map(str::to_string);
        Some(match s("op")?.as_str() {
            "MakeComponent" => Refactor::MakeComponent {
                statements: match j.get("statements")? {
                    Json::Arr(xs) => xs.iter().filter_map(|x| x.str().map(str::to_string)).collect(),
                    _ => return None,
                },
                name: s("name")?,
            },
            "ExtractComponent" => Refactor::ExtractComponent {
                component: s("component")?,
                file: s("file")?,
            },
            "InlineComponent" => Refactor::InlineComponent {
                file: s("file")?,
                name: s("name")?,
            },
            _ => return None,
        })
    }

    /// The refactor, `read` saying what is at a file's name (beside the document).
    pub fn apply(&self, src: &str, read: &dyn Fn(&str) -> Found) -> R<Written> {
        match self {
            Refactor::MakeComponent { statements, name } => make(src, statements, name),
            Refactor::ExtractComponent { component, file } => extract(src, component, file, read),
            Refactor::InlineComponent { file, name } => inline(src, file, name, read),
        }
    }
}

fn text(src: &str, sp: Span) -> &str {
    &src[sp.lo as usize..sp.hi as usize]
}

/// Edits by span, applied from the last.
fn splice_all(src: &str, mut edits: Vec<(u32, u32, String)>) -> String {
    edits.sort_by_key(|e| std::cmp::Reverse((e.0, e.1)));
    let mut out = src.to_string();
    for (lo, hi, t) in edits {
        out.replace_range(lo as usize..hi as usize, &t);
    }
    out
}

fn shared_names(prog: &[Stmt]) -> BTreeSet<String> {
    prog.iter()
        .filter_map(|s| match &s.kind {
            StmtKind::Shared { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

fn fn_names(prog: &[Stmt]) -> BTreeSet<String> {
    prog.iter()
        .filter_map(|s| match &s.kind {
            StmtKind::Fn { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

fn stmt_label(s: &Stmt) -> String {
    match &s.kind {
        StmtKind::Assign { name, .. } | StmtKind::Block { name, .. } | StmtKind::Shared { name, .. } => {
            name.clone()
        }
        StmtKind::Param { owner, name, .. } => format!("{}.{name}", crate::error::path_text(owner)),
        _ => format!("statement at byte {}", s.span.lo),
    }
}

fn indent(t: &str) -> String {
    t.lines()
        .map(|l| {
            if l.is_empty() {
                "\n".to_string()
            } else {
                format!("  {l}\n")
            }
        })
        .collect()
}

fn dedent(t: &str) -> String {
    t.lines()
        .map(|l| format!("{}\n", l.strip_prefix("  ").unwrap_or(l)))
        .collect()
}

/// Create Components from Bodies: `statements` (and the params written just above each) move
/// into `component NAME() { .. }`, placed once, where the first of them was.
fn make(src: &str, statements: &[String], name: &str) -> R<Written> {
    let prog = syntax::parse(src)?;
    if bound(&prog).contains(name) {
        return Err(Error::NameTaken { name: name.into() });
    }
    let mut ranges: Vec<(usize, usize)> = vec![];
    for id in statements {
        ranges.push(crate::edit::block(&prog, id)?);
    }
    ranges.sort();
    ranges.dedup();
    let moved: Vec<usize> = ranges.iter().flat_map(|&(j, k)| j..=k).collect();
    let moved_names: BTreeSet<String> = moved
        .iter()
        .flat_map(|&k| bound(std::slice::from_ref(&prog[k])))
        .collect();
    let top = bound(&prog);
    let (shared, fns) = (shared_names(&prog), fn_names(&prog));
    // what moves may read only what moves with it, shared params and functions (proof-findings 4)
    for &k in &moved {
        for n in mentions(&prog[k]) {
            if top.contains(&n) && !moved_names.contains(&n) && !shared.contains(&n) && !fns.contains(&n) {
                return Err(comp_error(
                    prog[k].span,
                    CompProblem::ReadsOutside {
                        name: n,
                        reader: stmt_label(&prog[k]),
                    },
                ));
            }
        }
    }
    // what stays may not read what moves: every name and every reader said at once (R2)
    let (mut read, mut readers): (Vec<String>, Vec<String>) = (vec![], vec![]);
    let mut at = Span::default();
    for n in &moved_names {
        let by: Vec<String> = (0..prog.len())
            .filter(|k| !moved.contains(k) && mentions(&prog[*k]).contains(n))
            .map(|k| stmt_label(&prog[k]))
            .collect();
        if !by.is_empty() {
            if read.is_empty() {
                at = moved
                    .iter()
                    .find(|&&k| bound(std::slice::from_ref(&prog[k])).contains(n))
                    .map_or(Span::default(), |&k| prog[k].span);
            }
            read.push(n.clone());
            readers.extend(
                by.into_iter()
                    .filter(|b| !readers.contains(b))
                    .collect::<Vec<_>>(),
            );
        }
    }
    if !read.is_empty() {
        return Err(comp_error(
            at,
            CompProblem::StillReadBy {
                name: read.join(", "),
                readers,
            },
        ));
    }
    let occ = (1..)
        .map(|k| format!("{name}_{k}"))
        .find(|o| !top.contains(o))
        .expect("a free name");
    let mut body = String::new();
    for &(j, k) in &ranges {
        body.push_str(text(
            src,
            Span::new(prog[j].span.lo as usize, prog[k].span.hi as usize),
        ));
        body.push('\n');
    }
    let block = format!(
        "component {name}() {{\n{}}}\n{occ} = part({name})\n",
        indent(&body)
    );
    let mut edits = vec![];
    for (n, &(j, k)) in ranges.iter().enumerate() {
        let t = if n == 0 { block.clone() } else { String::new() };
        edits.push((prog[j].span.lo, prog[k].full.hi, t));
    }
    Ok(Written {
        doc: splice_all(src, edits),
        files: vec![],
    })
}

/// `part(NAME)` / `part(NAME(..), ..)`: the call's file argument and the component's own args.
fn placing<'a>(e: &'a Expr, name: &str) -> Option<(&'a Arg, Vec<&'a Arg>, &'a [Arg])> {
    let ExprKind::Call(f, args) = &e.kind else {
        return None;
    };
    if !matches!(&f.kind, ExprKind::Name(n) if n == "part") {
        return None;
    }
    let first = args.iter().find(|a| a.name.is_none())?;
    match &first.value.kind {
        ExprKind::Name(n) if n == name => Some((first, vec![], args)),
        ExprKind::Call(g, cargs) if matches!(&g.kind, ExprKind::Name(n) if n == name) => {
            Some((first, cargs.iter().collect(), args))
        }
        _ => None,
    }
}

/// Every expression of a statement, nested statements too.
fn exprs<'a>(s: &'a Stmt, out: &mut Vec<&'a Expr>) {
    syntax::walk(std::slice::from_ref(s), &mut |st| match &st.kind {
        StmtKind::Shared { value, .. } | StmtKind::Param { value, .. } | StmtKind::Assign { value, .. } => {
            out.push(value)
        }
        StmtKind::Expr(e) | StmtKind::Return(e) => out.push(e),
        StmtKind::For { iter, .. } => out.push(iter),
        StmtKind::If { cond, .. } => out.push(cond),
        StmtKind::Block { args, .. } => out.extend(args.iter().map(|a| &a.value)),
        StmtKind::Expect { args, .. } => out.extend(args.iter()),
        StmtKind::Fn { .. } => {}
    });
}

/// Whether an expression reads `name` anywhere but as a placed component (`part(name..)`).
fn reads_otherwise(e: &Expr, name: &str) -> bool {
    if placing(e, name).is_some() {
        let ExprKind::Call(_, args) = &e.kind else {
            unreachable!("a call")
        };
        // its other arguments may still read it (`at: name..` is no placement)
        return args.iter().skip(1).any(|a| reads_otherwise(&a.value, name))
            || match &args[0].value.kind {
                ExprKind::Call(_, cargs) => cargs.iter().any(|a| reads_otherwise(&a.value, name)),
                _ => false,
            };
    }
    let mut hit = false;
    let mut go = |x: &Expr| hit = hit || reads_otherwise(x, name);
    match &e.kind {
        ExprKind::Name(n) => return n == name,
        ExprKind::List(xs) | ExprKind::Tuple(xs) => xs.iter().for_each(go),
        ExprKind::Map(xs) => xs.iter().for_each(|x| go(&x.1)),
        ExprKind::Comp { item, iter, cond, .. } => {
            go(item);
            go(iter);
            if let Some(c) = cond {
                go(c)
            }
        }
        ExprKind::Range { lo, hi, .. } | ExprKind::Bin(_, lo, hi) | ExprKind::Index(lo, hi) => {
            go(lo);
            go(hi)
        }
        ExprKind::Neg(a) | ExprKind::Not(a) | ExprKind::Attr(a, _) => go(a),
        ExprKind::Call(f, args) => {
            go(f);
            args.iter().for_each(|a| go(&a.value))
        }
        ExprKind::If { cond, then, other } => {
            go(cond);
            go(then);
            go(other)
        }
        ExprKind::Num(..) | ExprKind::Str(_) | ExprKind::Bool(_) => {}
    }
    hit
}

/// Every placement of `name` in a statement, for rewriting.
fn placements<'a>(e: &'a Expr, name: &str, out: &mut Vec<&'a Expr>) {
    if placing(e, name).is_some() {
        out.push(e);
        return;
    }
    let mut go = |x: &'a Expr| placements(x, name, out);
    match &e.kind {
        ExprKind::List(xs) | ExprKind::Tuple(xs) => xs.iter().for_each(&mut go),
        ExprKind::Map(xs) => xs.iter().for_each(|x| go(&x.1)),
        ExprKind::Call(f, args) => {
            go(f);
            args.iter().for_each(|a| go(&a.value))
        }
        ExprKind::If { cond, then, other } => {
            go(cond);
            go(then);
            go(other)
        }
        _ => {}
    }
}

/// The functions `names` reach (themselves, and what they call), in the document's order.
fn fns_reached(prog: &[Stmt], mut names: BTreeSet<String>) -> Vec<&Stmt> {
    let fns = fn_names(prog);
    loop {
        let more: BTreeSet<String> = prog
            .iter()
            .filter(|s| matches!(&s.kind, StmtKind::Fn { name, .. } if names.contains(name)))
            .flat_map(|s| match &s.kind {
                StmtKind::Fn { body, .. } => body.iter().flat_map(mentions).collect::<Vec<_>>(),
                _ => vec![],
            })
            .filter(|n| fns.contains(n) && !names.contains(n))
            .collect();
        if more.is_empty() {
            break;
        }
        names.extend(more);
    }
    prog.iter()
        .filter(|s| matches!(&s.kind, StmtKind::Fn { name, .. } if names.contains(name)))
        .collect()
}

/// Move to Own File: the component's block becomes FILE's top level.
fn extract(src: &str, name: &str, file: &str, read: &dyn Fn(&str) -> Found) -> R<Written> {
    let prog = syntax::parse(src)?;
    let Some(k) = prog.iter().position(
        |s| matches!(&s.kind, StmtKind::Block { kind: BlockKind::Component, name: n, .. } if n == name),
    ) else {
        return Err(Error::UnknownFeature { id: name.into() });
    };
    if !plain_name(file) {
        return Err(comp_error(
            prog[k].span,
            CompProblem::BadFileName { file: file.into() },
        ));
    }
    let taken = match read(file) {
        Found::Missing => None,
        Found::Link => Some(CompProblem::TargetIsLink { file: file.into() }),
        Found::Text(_) | Found::Taken | Found::Outside => Some(CompProblem::FileExists { file: file.into() }),
    };
    if let Some(p) = taken {
        return Err(comp_error(prog[k].span, p));
    }
    let StmtKind::Block { args, body, .. } = &prog[k].kind else {
        unreachable!("a block")
    };
    // every reader of the name places it
    for (j, s) in prog.iter().enumerate().filter(|(j, _)| *j != k) {
        let mut es = vec![];
        exprs(s, &mut es);
        if es.iter().any(|e| reads_otherwise(e, name)) {
            return Err(comp_error(
                s.span,
                CompProblem::ReadOtherwise {
                    component: name.into(),
                    reader: stmt_label(&prog[j]),
                },
            ));
        }
    }
    // what the block reads of the document: shared params passed at each call, functions copied
    let shared = shared_names(&prog);
    let reads: BTreeSet<String> = mentions(&prog[k]);
    let fns_called = fns_reached(&prog, reads.intersection(&fn_names(&prog)).cloned().collect());
    let mut reads_all = reads.clone();
    for f in &fns_called {
        reads_all.extend(mentions(f));
    }
    let passed: Vec<&Stmt> = prog
        .iter()
        .filter(|s| matches!(&s.kind, StmtKind::Shared { name, .. } if reads_all.contains(name)))
        .collect();
    let passed_names: Vec<String> = passed
        .iter()
        .filter_map(|s| match &s.kind {
            StmtKind::Shared { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();
    let _ = shared;
    let mut out = format!("# {name}, moved here from its own block\n");
    for s in &passed {
        out.push_str(text(src, s.span));
        out.push('\n');
    }
    for a in args {
        let (n, _) = a.name.as_ref().expect("a component's params are named");
        out.push_str(&format!("shared {n} = {}\n", text(src, a.value.span)));
    }
    for f in &fns_called {
        out.push_str(text(src, f.span));
        out.push('\n');
    }
    if let (Some(first), Some(last)) = (body.first(), body.last()) {
        let lo = src[..first.span.lo as usize].rfind('\n').map_or(0, |n| n + 1);
        out.push_str(&dedent(&src[lo..last.span.hi as usize]));
    }
    // each placement: the file, with: its own args and every shared param the block reads
    // a line where it was, so bringing it back puts it there again (the text's round trip)
    let mut edits = vec![(
        prog[k].span.lo,
        prog[k].full.hi,
        format!("{}\n", marker(name, file)),
    )];
    for s in prog.iter().filter(|s| !std::ptr::eq(*s, &prog[k])) {
        let mut es = vec![];
        exprs(s, &mut es);
        for e in es {
            let mut ps = vec![];
            placements(e, name, &mut ps);
            for p in ps {
                let (first, cargs, _) = placing(p, name).expect("a placement");
                let mut with: Vec<String> = cargs
                    .iter()
                    .map(|a| {
                        format!(
                            "{}: {}",
                            a.name.as_ref().map_or("", |n| n.0.as_str()),
                            text(src, a.value.span)
                        )
                    })
                    .collect();
                with.extend(passed_names.iter().map(|n| format!("{n}: {n}")));
                let t = if with.is_empty() {
                    format!("{file:?}")
                } else {
                    format!("{file:?}, with: {{{}}}", with.join(", "))
                };
                edits.push((first.value.span.lo, first.value.span.hi, t));
            }
        }
    }
    Ok(Written {
        doc: splice_all(src, edits),
        files: vec![(file.into(), out)],
    })
}

/// The line Extract leaves where a component was, and Inline puts it back at.
fn marker(name: &str, file: &str) -> String {
    format!("# {name} is in {file:?}")
}

/// Bring Into This File: FILE's top level becomes `component NAME(p: DEFAULT, ..) { .. }`,
/// its shared params the params; each `part(FILE, with: {..})` a call.
fn inline(src: &str, file: &str, name: &str, read: &dyn Fn(&str) -> Found) -> R<Written> {
    let prog = syntax::parse(src)?;
    let no = |why: &str| {
        comp_error(
            Span::default(),
            CompProblem::NotInlinable {
                file: file.into(),
                why: why.into(),
            },
        )
    };
    let theirs = match read(file) {
        Found::Text(t) => t,
        Found::Outside => {
            return Err(comp_error(
                Span::default(),
                CompProblem::Outside { file: file.into() },
            ));
        }
        Found::Taken => return Err(no("it is not a Forge file this reads")),
        Found::Missing | Found::Link => return Err(no("no such file")),
    };
    let fp = syntax::parse(&theirs).map_err(|_| no("it doesn't parse"))?;
    if bound(&prog).contains(name) {
        return Err(Error::NameTaken { name: name.into() });
    }
    if crate::asm::has_components(&fp) {
        return Err(no("it places components or joins them (a subassembly)"));
    }
    let fshared = shared_names(&fp);
    // its params the document shares under the same name, passed through at every placement
    // (`with: {W: W}`, what Extract writes): read from the document's own, no param of the block
    let doc_shared = shared_names(&prog);
    let mut all: Vec<FilePlacement> = vec![];
    for s in &prog {
        let mut es = vec![];
        exprs(s, &mut es);
        for e in es {
            file_placements(e, file, &mut all);
        }
    }
    let given_text = |w: Option<&Arg>, k: &str| -> Option<String> {
        match &w?.value.kind {
            ExprKind::Map(xs) => xs
                .iter()
                .find(|x| x.0 == k)
                .map(|x| text(src, x.1.span).to_string()),
            _ => None,
        }
    };
    let mut through: BTreeSet<String> = BTreeSet::new();
    for n in fshared.intersection(&doc_shared) {
        if all
            .iter()
            .all(|p| given_text(p.1, n).as_deref() == Some(n.as_str()))
        {
            through.insert(n.clone());
        } else {
            return Err(no(&format!(
                "its param {n} is a shared param here, and a placement gives it otherwise"
            )));
        }
    }
    let mut params = vec![];
    let mut body = String::new();
    let mut fns = vec![];
    let ours = bound(&prog);
    for s in &fp {
        match &s.kind {
            StmtKind::Shared { name: n, .. } if through.contains(n) => {}
            StmtKind::Shared { name: n, value } => {
                // a default sees only the document's shared params (E10): none of the file's own
                let mut probe = BTreeSet::new();
                probe.extend(mentions(s));
                probe.remove(n);
                if probe.iter().any(|m| fshared.contains(m)) {
                    return Err(no(&format!("its param {n} is made of its other params")));
                }
                params.push(format!("{n}: {}", text(&theirs, value.span)));
            }
            StmtKind::Fn { name: n, .. } => {
                // the same function written the same here (Extract copied it): one is enough
                let same = prog.iter().any(|o| {
                    matches!(&o.kind, StmtKind::Fn { name: m, .. } if m == n)
                        && text(src, o.span) == text(&theirs, s.span)
                });
                if same {
                    continue;
                }
                if ours.contains(n) {
                    return Err(no(&format!("its function {n} has a name this file uses")));
                }
                fns.push(text(&theirs, s.span).to_string());
            }
            StmtKind::Assign { value, .. } if matches!(&value.kind, ExprKind::Call(f, _) if matches!(&f.kind, ExprKind::Name(n) if n == "use")) =>
            {
                return Err(no("it uses another file"));
            }
            StmtKind::Expr(e) if matches!(&e.kind, ExprKind::Name(n) if n == "assembly") => {}
            _ => {
                body.push_str(text(&theirs, s.span));
                body.push('\n');
            }
        }
    }
    let block = format!(
        "{}component {name}({}) {{\n{}}}\n",
        fns.iter().map(|f| format!("{f}\n")).collect::<String>(),
        params.join(", "),
        indent(&body)
    );
    // its placements become calls; the block goes in above the first of them
    let mut edits = vec![];
    let mut first_at: Option<u32> = None;
    for s in &prog {
        let mut es = vec![];
        exprs(s, &mut es);
        let mut hit = false;
        for e in es {
            let mut ps = vec![];
            file_placements(e, file, &mut ps);
            for (lit, with, args) in ps {
                hit = true;
                let given: Vec<String> = match with {
                    Some(w) => match &w.value.kind {
                        ExprKind::Map(xs) => xs
                            .iter()
                            .filter(|(k, _)| !through.contains(k))
                            .map(|(k, v)| format!("{k}: {}", text(src, v.span)))
                            .collect(),
                        _ => return Err(no("a with: that is no map")),
                    },
                    None => vec![],
                };
                for g in &given {
                    let k = g.split(':').next().unwrap_or("");
                    if !fshared.contains(k) {
                        return Err(no(&format!("with: names {k}, which it has no param for")));
                    }
                }
                let call = if given.is_empty() {
                    name.to_string()
                } else {
                    format!("{name}({})", given.join(", "))
                };
                edits.push((lit.value.span.lo, lit.value.span.hi, call));
                if let Some(w) = with {
                    // the with: argument and the comma before it
                    let k = args
                        .iter()
                        .position(|a| std::ptr::eq(a, w))
                        .expect("its argument");
                    let lo = args[k - 1].value.span.hi;
                    edits.push((lo, w.value.span.hi, String::new()));
                }
            }
        }
        if hit && first_at.is_none() {
            first_at = Some(s.span.lo);
        }
    }
    // where it was (Extract's line), else above its first use, else at the end
    match src.find(&format!("{}\n", marker(name, file))) {
        Some(lo) => {
            let hi = lo + marker(name, file).len() + 1;
            edits.push((lo as u32, hi as u32, block));
        }
        None => {
            let at = first_at.unwrap_or(src.len() as u32);
            let block = if first_at.is_none() && !src.is_empty() && !src.ends_with('\n') {
                format!("\n{block}")
            } else {
                block
            };
            edits.push((at, at, block));
        }
    }
    Ok(Written {
        doc: splice_all(src, edits),
        files: vec![],
    })
}

/// `part("FILE", ..)`: its file argument, its `with:` argument, and all its arguments.
type FilePlacement<'a> = (&'a Arg, Option<&'a Arg>, &'a [Arg]);

fn file_placements<'a>(e: &'a Expr, file: &str, out: &mut Vec<FilePlacement<'a>>) {
    if let ExprKind::Call(f, args) = &e.kind
        && matches!(&f.kind, ExprKind::Name(n) if n == "part")
        && let Some(first) = args.iter().find(|a| a.name.is_none())
        && matches!(&first.value.kind, ExprKind::Str(s) if s == file)
    {
        let with = args
            .iter()
            .find(|a| a.name.as_ref().is_some_and(|n| n.0 == "with"));
        out.push((first, with, args));
        return;
    }
    let mut go = |x: &'a Expr| file_placements(x, file, out);
    match &e.kind {
        ExprKind::List(xs) | ExprKind::Tuple(xs) => xs.iter().for_each(&mut go),
        ExprKind::Call(f, args) => {
            go(f);
            args.iter().for_each(|a| go(&a.value))
        }
        _ => {}
    }
}

/// The refactor, then the document as it leaves it checked (no error it did not have), and each
/// new file parsed: all or nothing.
pub fn apply_checked(src: &str, r: &Refactor, read: &dyn Fn(&str) -> Found) -> R<Written> {
    let w = r.apply(src, read)?;
    crate::edit::checked(src, &w.doc)?;
    for (_, t) in &w.files {
        syntax::parse(t)?;
    }
    Ok(w)
}

/// Each statement (at any depth, the document's or a block's) reading feature `feat` of an
/// occurrence of component `comp` through its path (`l1.plate`, `b1.l1.plate`): what a delete of
/// `comp.feat` would leave unbound (R2).
pub fn occurrence_readers(prog: &[Stmt], comp: &str, feat: &str) -> Vec<String> {
    // the occurrences' names, wherever they are placed
    let mut occ: BTreeSet<String> = BTreeSet::new();
    syntax::walk(prog, &mut |s| {
        if let StmtKind::Assign { name, value } = &s.kind
            && placing(value, comp).is_some()
        {
            occ.insert(name.clone());
        }
    });
    let mut out = vec![];
    syntax::walk(prog, &mut |s| {
        if matches!(
            s.kind,
            StmtKind::Block {
                kind: BlockKind::Component,
                ..
            }
        ) {
            return;
        }
        let mut es = vec![];
        exprs(s, &mut es);
        let mut chains = vec![];
        for e in es {
            attr_chains(e, &mut chains);
        }
        if chains
            .iter()
            .any(|c| c.windows(2).any(|w| occ.contains(&w[0]) && w[1] == feat))
        {
            let l = stmt_label(s);
            if !out.contains(&l) {
                out.push(l);
            }
        }
    });
    out
}

/// Every chain of names an expression reads (`b1.l1.plate.end` is [b1, l1, plate, end]).
fn attr_chains(e: &Expr, out: &mut Vec<Vec<String>>) {
    fn chain(e: &Expr) -> Option<Vec<String>> {
        match &e.kind {
            ExprKind::Name(n) => Some(vec![n.clone()]),
            ExprKind::Attr(o, n) => {
                let mut c = chain(o)?;
                c.push(n.clone());
                Some(c)
            }
            ExprKind::Index(o, _) => chain(o),
            _ => None,
        }
    }
    if let Some(c) = chain(e)
        && c.len() > 1
    {
        out.push(c);
    }
    let mut go = |x: &Expr| attr_chains(x, out);
    match &e.kind {
        ExprKind::List(xs) | ExprKind::Tuple(xs) => xs.iter().for_each(&mut go),
        ExprKind::Map(xs) => xs.iter().for_each(|x| go(&x.1)),
        ExprKind::Comp { item, iter, cond, .. } => {
            go(item);
            go(iter);
            if let Some(c) = cond {
                go(c)
            }
        }
        ExprKind::Range { lo, hi, .. } | ExprKind::Bin(_, lo, hi) | ExprKind::Index(lo, hi) => {
            go(lo);
            go(hi)
        }
        ExprKind::Neg(a) | ExprKind::Not(a) | ExprKind::Attr(a, _) => go(a),
        ExprKind::Call(f, args) => {
            go(f);
            args.iter().for_each(|a| go(&a.value))
        }
        ExprKind::If { cond, then, other } => {
            go(cond);
            go(then);
            go(other)
        }
        _ => {}
    }
}

/// What is at a name a refactor reads or would write.
#[derive(Clone, Debug, PartialEq)]
pub enum Found {
    /// Nothing at all.
    Missing,
    /// A file inside the fence, read.
    Text(String),
    /// A link (dangling or not), where a file would be written.
    Link,
    /// Something not readable as a part (a folder, too big, not text).
    Taken,
    /// There, but outside the fence once links are resolved: never read.
    Outside,
}

/// What is at `file` beside `dir`, read only inside `fence` (links resolved first) and no
/// further than a part file's limit.
pub fn found(dir: &std::path::Path, fence: Option<&std::path::Path>, file: &str) -> Found {
    let at = dir.join(file);
    let Ok(meta) = std::fs::symlink_metadata(&at) else {
        return Found::Missing;
    };
    let Ok(real) = std::fs::canonicalize(&at) else {
        // a dangling link
        return if meta.file_type().is_symlink() {
            Found::Link
        } else {
            Found::Taken
        };
    };
    if fence.is_some_and(|f| !real.starts_with(f)) {
        return Found::Outside;
    }
    match crate::asm::read_capped(&real) {
        Some(t) if real.is_file() => Found::Text(t),
        _ if meta.file_type().is_symlink() => Found::Link,
        _ => Found::Taken,
    }
}

/// A refactor of the document at `doc` (None: untitled, refused), reading beside it inside
/// `root` (else the project it is in), the files it makes written new beside it: the new text.
pub fn run_on_disk(
    doc: Option<&std::path::Path>,
    root: Option<&std::path::Path>,
    src: &str,
    r: &Refactor,
) -> R<Written> {
    let untitled = || comp_error(Span::default(), CompProblem::UntitledDocument);
    let doc = doc
        .and_then(|f| std::fs::canonicalize(f).ok())
        .ok_or_else(untitled)?;
    let dir = doc.parent().ok_or_else(untitled)?.to_path_buf();
    let fence = crate::fence_for(Some(&dir), root);
    let read = |f: &str| found(&dir, fence.as_deref(), f);
    let w = apply_checked(src, r, &read)?;
    for (f, t) in &w.files {
        write_new(&dir, f, t).map_err(|e| {
            let file = f.clone();
            comp_error(
                Span::default(),
                match e.kind() {
                    std::io::ErrorKind::InvalidInput => CompProblem::BadFileName { file },
                    std::io::ErrorKind::AlreadyExists => CompProblem::FileExists { file },
                    _ => CompProblem::NotInlinable {
                        file,
                        why: e.to_string(),
                    },
                },
            )
        })?;
    }
    Ok(w)
}

/// A file a refactor may make: a plain name beside the document (no folder, no `..`, not
/// absolute), ending `.fg`.
pub fn plain_name(file: &str) -> bool {
    let p = std::path::Path::new(file);
    file.ends_with(".fg")
        && file.len() > 3
        && !file.contains(['/', '\\'])
        && !file.starts_with('.')
        && p.components().count() == 1
        && matches!(p.components().next(), Some(std::path::Component::Normal(_)))
}

/// Writes a file a refactor made, in `dir`: refused unless its name is plain and nothing at all
/// is there (a file, a directory, a link, dangling or not), and created new, never through a link.
pub fn write_new(dir: &std::path::Path, file: &str, text: &str) -> std::io::Result<()> {
    use std::io::Write;
    if !plain_name(file) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{file}: not a plain .fg name"),
        ));
    }
    let path = dir.join(file);
    if std::fs::symlink_metadata(&path).is_ok() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("{file} exists already"),
        ));
    }
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    f.write_all(text.as_bytes())
}
