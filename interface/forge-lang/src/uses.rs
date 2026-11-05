//! `steel = use("shop.fg")`: another file's top-level `shared` values and functions as a
//! read-only namespace. The file is found as part files are (`asm::locate`): beside the using
//! document, a `.fg` of bounded size, under the session's root. It is evaluated on its own; its
//! values come in driven by its params, keyed under the binding (`steel.t`), so provenance says
//! which file a number is from, and a SetParam on one is refused here as an edit of that file.
//! Its functions see its names, never the caller's. A file that uses itself, directly or not, is
//! the typed error `UseCycle`.

use crate::builtins::Args;
use crate::error::{Error, ParamKey};
use crate::eval::{Eval, FnDef, R, Value, driving};
use crate::syntax::{ExprKind, Key, Seg, Span, StmtKind};
use crate::values::{Num, Src, Ty};
use forge_agent::json::Json;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::rc::Rc;

/// A used file: its names as its functions see them.
#[derive(Debug)]
pub struct Lib {
    pub binding: String,
    /// The file as the using document wrote it.
    pub file: String,
    pub path: PathBuf,
    pub names: RefCell<BTreeMap<String, Value>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UseErr {
    /// The file is not there, not a `.fg`, or too big.
    Missing { span: Span, file: String },
    /// The file is outside the session's root: never read.
    Outside { span: Span, file: String },
    /// There, but no Forge file (not a plain `.fg` of text, too big, or it does not parse: the
    /// line it stops at): none of its text is said.
    NotForge { span: Span, file: String },
    /// The file uses, directly or through others, a file already being used. `chain`: the
    /// files being used, as each document named them.
    Cycle {
        span: Span,
        file: String,
        chain: Vec<String>,
    },
    /// `use(..)` not bound by an assignment (`name = use(..)`).
    NotBound { span: Span },
    /// `use` inside a function, loop, sketch or path.
    NotTopLevel { span: Span },
    /// Reading the used file failed: its own error, at its own place in that file.
    InFile {
        span: Span,
        file: String,
        error: Box<Error>,
        line: usize,
    },
    /// A SetParam names a used file's param: the edit is to that file, not this one.
    ParamInUsedFile {
        param: ParamKey,
        file: String,
        name: String,
    },
}

impl UseErr {
    pub fn span(&self) -> Option<Span> {
        match self {
            UseErr::Missing { span, .. }
            | UseErr::Outside { span, .. }
            | UseErr::NotForge { span, .. }
            | UseErr::Cycle { span, .. }
            | UseErr::NotBound { span }
            | UseErr::NotTopLevel { span }
            | UseErr::InFile { span, .. } => Some(*span),
            UseErr::ParamInUsedFile { .. } => None,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            UseErr::Missing { .. } => "UseMissing",
            UseErr::Outside { .. } => "UseOutside",
            UseErr::NotForge { .. } => "UseNotForge",
            UseErr::Cycle { .. } => "UseCycle",
            UseErr::NotBound { .. } => "UseNotBound",
            UseErr::NotTopLevel { .. } => "UseNotTopLevel",
            UseErr::InFile { .. } => "UseFailed",
            UseErr::ParamInUsedFile { .. } => "ParamInUsedFile",
        }
    }

    pub fn text(&self) -> String {
        match self {
            UseErr::Outside { file, .. } => format!("use: {file} is outside the session's root, not read"),
            UseErr::NotForge { file, .. } => {
                format!(
                    "use: {file} is not a Forge file this reads (a plain .fg of text that parses, at most 64 MB)"
                )
            }
            UseErr::Missing { file, .. } => {
                format!("use: no file {file} (a .fg beside this one, under the session's root)")
            }
            UseErr::Cycle { file, chain, .. } => {
                format!(
                    "use: {file} is already being used ({} -> {file})",
                    chain.join(" -> ")
                )
            }
            UseErr::NotBound { .. } => "use(..) is bound by a name: `shop = use(\"shop.fg\")`".into(),
            UseErr::NotTopLevel { .. } => "use(..) is a top-level statement".into(),
            UseErr::InFile {
                file, error, line, ..
            } => {
                format!("in {file}, line {line}: {} ({})", error.text(), error.kind())
            }
            UseErr::ParamInUsedFile { param, file, name } => {
                format!("{param} is the param {name} of {file}: edit it there")
            }
        }
    }

    pub fn data(&self) -> Vec<(&'static str, Json)> {
        let s = |x: &str| Json::Str(x.into());
        match self {
            UseErr::Missing { file, .. } | UseErr::Outside { file, .. } => vec![("file", s(file))],
            UseErr::NotForge { file, .. } => vec![("file", s(file))],
            UseErr::Cycle { file, chain, .. } => vec![
                ("file", s(file)),
                ("chain", Json::Arr(chain.iter().map(|c| s(c)).collect())),
            ],
            UseErr::NotBound { .. } | UseErr::NotTopLevel { .. } => vec![],
            UseErr::InFile {
                file, error, line, ..
            } => vec![
                ("file", s(file)),
                ("line", (*line).into()),
                ("error", s(error.kind())),
            ],
            UseErr::ParamInUsedFile { param, file, name } => vec![
                ("param", s(&param.to_string())),
                ("file", s(file)),
                ("name", s(name)),
            ],
        }
    }
}

fn err(e: UseErr) -> Error {
    Error::Use(Box::new(e))
}

/// An error raised in a used file, shown at `span` in the using document.
pub fn in_file(lib: &Lib, span: Span, error: Error) -> Error {
    // already shown in the file it happened in: here at this document's `use` or call
    if let Error::Use(u) = &error
        && let UseErr::InFile {
            file, error, line, ..
        } = &**u
    {
        return err(UseErr::InFile {
            span,
            file: file.clone(),
            error: error.clone(),
            line: *line,
        });
    }
    let line = error
        .span()
        .and_then(|sp| crate::asm::read_capped(&lib.path).map(|t| sp.lines(&t).0))
        .unwrap_or(0);
    err(UseErr::InFile {
        span,
        file: lib.file.clone(),
        error: Box::new(error),
        line,
    })
}

/// `name = use("file.fg")`.
pub fn call(ev: &mut Eval, args: &[crate::syntax::Arg], span: Span) -> R<Value> {
    let binding = ev.binding(span).ok_or(err(UseErr::NotBound { span }))?;
    if !ev.at_top() {
        return Err(err(UseErr::NotTopLevel { span }));
    }
    let a = Args::bind(ev, "use", &["file"], args, span)?;
    let file = match a.req("file")? {
        Value::Str(s) => s.to_string(),
        o => {
            return Err(Error::WrongType {
                span: a.span("file"),
                what: "use's file",
                want: Ty::Str,
                found: o.ty(),
            });
        }
    };
    let fence = ev.root.clone();
    let Some(path) = crate::asm::locate(&ev.dir, &fence, &file) else {
        if crate::outside(ev.dir.as_deref(), fence.as_deref(), &file) {
            return Err(err(UseErr::Outside { span, file }));
        }
        if ev
            .dir
            .as_ref()
            .is_some_and(|d| std::fs::symlink_metadata(d.join(&file)).is_ok())
        {
            return Err(err(UseErr::NotForge { span, file }));
        }
        return Err(err(UseErr::Missing { span, file }));
    };
    if ev.use_chain.contains(&path) {
        // the files as the documents wrote them: no path of this machine leaves in an error
        let chain = ev.use_files.clone();
        return Err(err(UseErr::Cycle { span, file, chain }));
    }
    let Some(text) = crate::asm::read_capped(&path) else {
        return Err(err(UseErr::NotForge { span, file }));
    };
    let lib = Rc::new(Lib {
        binding: binding.clone(),
        file: file.clone(),
        path: path.clone(),
        names: RefCell::new(BTreeMap::new()),
    });
    let fail = |e: Error| in_file(&lib, span, e);
    let prog = crate::syntax::parse(&text).map_err(fail)?;
    let mut sub = Eval::new(&text, ev.rules, false, Default::default());
    sub.dir = path.parent().map(|p| p.to_path_buf());
    sub.root = ev.root.clone();
    sub.use_chain = ev.use_chain.clone();
    sub.use_chain.push(path.clone());
    sub.use_files = ev.use_files.clone();
    sub.use_files.push(file.clone());
    // only what a used file gives: its shared values, its functions and the files it uses
    for s in &prog {
        let keep = match &s.kind {
            StmtKind::Shared { .. } | StmtKind::Fn { .. } => true,
            StmtKind::Assign { value, .. } => {
                matches!(&value.kind, ExprKind::Call(f, _) if matches!(&f.kind, ExprKind::Name(n) if n == "use"))
            }
            _ => false,
        };
        if keep && let Err(e) = sub.run_one(s) {
            return Err(fail(e));
        }
    }
    if let Some(e) = sub.errors.first() {
        return Err(fail(e.clone()));
    }
    let key = |name: &str| ParamKey {
        owner: vec![Seg {
            name: binding.clone(),
            index: vec![],
        }],
        name: name.into(),
    };
    // what the namespace holds: shared values driven by `binding.name`, the functions
    let mut out: BTreeMap<String, Value> = BTreeMap::new();
    for p in &sub.shared {
        out.insert(p.key.name.clone(), driving(&p.value, &key(&p.key.name), span));
    }
    // its components are not placed from here yet: named, so placing one says so (typed)
    for s in &prog {
        if let StmtKind::Block {
            kind: crate::syntax::BlockKind::Component,
            name,
            ..
        } = &s.kind
        {
            out.insert(
                name.clone(),
                Value::Asm(Rc::new(crate::asm::AsmVal::Foreign {
                    name: name.clone(),
                    file: file.clone(),
                })),
            );
        }
    }
    for (name, f) in &sub.fns {
        out.insert(
            name.clone(),
            Value::Func(Rc::new(FnDef {
                lib: Some(Rc::downgrade(&lib)),
                ..(**f).clone()
            })),
        );
    }
    // the functions also see the files this one uses, under this binding
    let mut names = out.clone();
    for (name, v) in sub.globals() {
        if let Value::Map(_) = v
            && sub.libs.iter().any(|l| l.binding == name)
        {
            names.insert(name.clone(), rekey(&v, &binding));
        }
    }
    *lib.names.borrow_mut() = names;
    ev.libs.push(lib);
    ev.libs.append(&mut sub.libs);
    Ok(Value::Map(Rc::new(
        out.into_iter().map(|(k, v)| (Key::Label(k), v)).collect(),
    )))
}

/// A nested namespace's values, their params put under `binding` too (`steel.mat.k`).
fn rekey(v: &Value, binding: &str) -> Value {
    let under = |k: &ParamKey| {
        let mut owner = vec![Seg {
            name: binding.into(),
            index: vec![],
        }];
        owner.extend(k.owner.iter().cloned());
        ParamKey {
            owner,
            name: k.name.clone(),
        }
    };
    match v {
        Value::Num(n) => Value::Num(Num {
            src: match &n.src {
                Src::Const => Src::Const,
                Src::Driving(k) => Src::Driving(under(k)),
                Src::Driven(ks) => Src::Driven(ks.iter().map(under).collect::<BTreeSet<_>>()),
            },
            ..n.clone()
        }),
        Value::List(xs) => Value::List(Rc::new(xs.iter().map(|x| rekey(x, binding)).collect())),
        Value::Tuple(xs) => Value::Tuple(Rc::new(xs.iter().map(|x| rekey(x, binding)).collect())),
        Value::Map(m) => Value::Map(Rc::new(
            m.iter().map(|(k, x)| (k.clone(), rekey(x, binding))).collect(),
        )),
        o => o.clone(),
    }
}

/// A SetParam on `param` when `param` is a used file's: the typed refusal naming that file. The
/// document's own statements say which bindings are used files (`name = use("file")`).
pub fn edit_target(prog: &[crate::syntax::Stmt], param: &ParamKey) -> Option<Error> {
    let [first, rest @ ..] = &param.owner[..] else {
        return None;
    };
    prog.iter().find_map(|s| match &s.kind {
        StmtKind::Assign { name, value } if *name == first.name => match &value.kind {
            ExprKind::Call(f, args) if matches!(&f.kind, ExprKind::Name(n) if n == "use") => {
                let file = match args.first().map(|a| &a.value.kind) {
                    Some(ExprKind::Str(s)) => s.clone(),
                    _ => "?".into(),
                };
                let mut name = rest.iter().map(|s| s.name.clone()).collect::<Vec<_>>();
                name.push(param.name.clone());
                Some(err(UseErr::ParamInUsedFile {
                    param: param.clone(),
                    file,
                    name: name.join("."),
                }))
            }
            _ => None,
        },
        _ => None,
    })
}
