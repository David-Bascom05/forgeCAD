//! The machining document as the workspace sees it: its text (the source of truth, as a part's
//! is), the statements read from it, and edits made as forge-lang's own edit ops on that text
//! (`SetArg`, `AddFeature`, `DeleteFeature`, `MoveFeature`, `Suppress`, `Rename`), so every
//! change the dialogs make is one the CLI and the agents could make, and the text stays the
//! user's (comments, spacing and order kept).

use forge_lang::error::Op;
use forge_lang::syntax::{self, ExprKind, StmtKind};

/// What a statement makes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Role {
    Part,
    Machine,
    Tool,
    Setup,
    /// A vise or soft jaws.
    Fixture,
    /// An operation of this kind (`facing`, `clear`, ...).
    Op(String),
    Program,
    /// `shared` and `param` lines, and anything else.
    Other,
}

/// One argument as written: its keyword (None: positional) and its text.
#[derive(Clone, Debug, PartialEq)]
pub struct ArgText {
    pub key: Option<String>,
    pub text: String,
}

/// One `NAME = call(..)` statement (or one `Suppress` kept as comments).
#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    pub name: String,
    pub func: String,
    pub role: Role,
    pub args: Vec<ArgText>,
    /// Out of the build (`#~NAME` lines).
    pub suppressed: bool,
    /// The setup an op, program or setup reads (its first positional argument's name).
    pub setup: Option<String>,
}

impl Stmt {
    /// The text of keyword argument `key`.
    pub fn arg(&self, key: &str) -> Option<&str> {
        self.args
            .iter()
            .find(|a| a.key.as_deref() == Some(key))
            .map(|a| a.text.as_str())
    }

    /// The text of the `k`th positional argument.
    pub fn pos(&self, k: usize) -> Option<&str> {
        self.args
            .iter()
            .filter(|a| a.key.is_none())
            .nth(k)
            .map(|a| a.text.as_str())
    }

    /// A string argument's value, its quotes taken off.
    pub fn arg_str(&self, key: &str) -> Option<String> {
        self.arg(key).map(unquote)
    }

    pub fn is_op(&self) -> bool {
        matches!(self.role, Role::Op(_))
    }
}

/// A string literal's value (`"x"` → x); anything else as written.
pub fn unquote(t: &str) -> String {
    let t = t.trim();
    if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

/// The op built-ins this workspace knows: forge-cam-lang's (milling), then forge-turn-lang's
/// (turning).
pub const OP_FUNCS: [&str; 16] = [
    "facing",
    "clear",
    "contour",
    "pocket",
    "drill",
    "engrave",
    "thread_mill",
    "edge_break",
    "turn_face",
    "turn_rough",
    "turn_finish",
    "bore_rough",
    "turn_groove",
    "turn_thread",
    "part_off",
    "turn_drill",
];

fn role_of(func: &str) -> Role {
    match func {
        "part" => Role::Part,
        "machine" => Role::Machine,
        "tool" | "lathe_tool" => Role::Tool,
        "setup" | "lathe_setup" => Role::Setup,
        "vise" | "soft_jaw" | "chuck" => Role::Fixture,
        "program" | "lathe_program" => Role::Program,
        f if OP_FUNCS.contains(&f) => Role::Op(f.to_string()),
        // an op built-in this workspace has no form for yet: shown, edited as text
        f if f.ends_with("_mill") || f.starts_with("rough") || f.starts_with("finish") => {
            Role::Op(f.to_string())
        }
        _ => Role::Other,
    }
}

/// The statements of `src`: its `NAME = call(..)` lines in order, the suppressed ones where
/// their comments stand. None when the text does not parse (the reply's errors say why).
pub fn read(src: &str) -> Option<Vec<Stmt>> {
    let prog = syntax::parse(src).ok()?;
    let mut out: Vec<(usize, Stmt)> = vec![];
    for s in &prog {
        if let Some(st) = stmt_of(src, s) {
            out.push((s.span.lo as usize, st));
        }
    }
    // suppressed statements: each tag's lines, read as the text they were
    let mut at = 0usize;
    let mut groups: Vec<(usize, String, String)> = vec![];
    for line in src.split_inclusive('\n') {
        if let Some((id, rest)) = suppressed_line(line) {
            match groups.last_mut() {
                Some(g) if g.1 == id && g.0 + g.2.len() == at => g.2.push_str(rest),
                _ => groups.push((at, id.to_string(), rest.to_string())),
            }
        }
        at += line.len();
    }
    for (pos, _id, text) in groups {
        if let Ok(p) = syntax::parse(&text) {
            for s in &p {
                if let Some(mut st) = stmt_of(&text, s) {
                    st.suppressed = true;
                    out.push((pos, st));
                }
            }
        }
    }
    out.sort_by_key(|(p, _)| *p);
    Some(out.into_iter().map(|(_, s)| s).collect())
}

/// A line `Suppress` kept: its id and the statement's text.
fn suppressed_line(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("#~")?;
    let n = rest.find(|c: char| !(c.is_alphanumeric() || c == '_'))?;
    let (id, rest) = rest.split_at(n);
    let rest = rest.trim_start_matches(['<', '>']);
    let rest = rest.strip_prefix(' ')?;
    (!id.is_empty()).then_some((id, rest))
}

fn stmt_of(src: &str, s: &syntax::Stmt) -> Option<Stmt> {
    let StmtKind::Assign { name, value } = &s.kind else {
        return None;
    };
    let ExprKind::Call(f, args) = &value.kind else {
        return None;
    };
    let ExprKind::Name(func) = &f.kind else {
        return None;
    };
    let text = |sp: syntax::Span| src[sp.lo as usize..sp.hi as usize].to_string();
    let args: Vec<ArgText> = args
        .iter()
        .map(|a| ArgText {
            key: a.name.as_ref().map(|(k, _)| k.clone()),
            text: text(a.value.span),
        })
        .collect();
    let role = role_of(func);
    let first = args.iter().find(|a| a.key.is_none()).map(|a| a.text.clone());
    let setup = match role {
        Role::Op(_) | Role::Program => first,
        Role::Setup => Some(name.clone()),
        _ => None,
    };
    Some(Stmt {
        name: name.clone(),
        func: func.clone(),
        role,
        args,
        suppressed: false,
        setup,
    })
}

/// One edit to the document, as the workspace asks for it.
#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    /// Keyword `arg` of statement `id` set to `text` (put in if missing).
    Set {
        id: String,
        arg: String,
        text: String,
    },
    /// Keyword `arg` of statement `id` taken out (the built-in's default applies): sent as
    /// `ReplaceFeature` of the statement less it.
    Unset {
        id: String,
        arg: String,
    },
    /// Statement `id` replaced by `text` (`ReplaceFeature`).
    Replace {
        id: String,
        text: String,
    },
    /// A new statement `text` (`NAME = call(..)`), before statement `before` or at the end.
    Add {
        before: Option<String>,
        text: String,
    },
    Delete {
        id: String,
    },
    /// Statement `id` moved before `before` (None: to the end).
    Move {
        id: String,
        before: Option<String>,
    },
    Suppress {
        id: String,
    },
    Unsuppress {
        id: String,
    },
    Rename {
        from: String,
        to: String,
    },
}

/// `src` with `e` made, or why not (nothing changed).
pub fn apply(src: &str, e: &Edit) -> Result<String, String> {
    let op = match e {
        Edit::Set { id, arg, text } => Op::SetArg {
            id: id.clone(),
            arg: arg.clone(),
            text: text.clone(),
        },
        Edit::Unset { id, arg } => return unset(src, id, arg),
        Edit::Replace { id, text } => Op::ReplaceFeature {
            id: id.clone(),
            text: text.clone(),
        },
        Edit::Add { before, text } => Op::AddFeature {
            before: before.clone(),
            text: text.clone(),
        },
        Edit::Delete { id } => Op::DeleteFeature { id: id.clone() },
        Edit::Move { id, before } => Op::MoveFeature {
            id: id.clone(),
            before: before.clone(),
        },
        Edit::Suppress { id } => Op::Suppress { id: id.clone() },
        Edit::Unsuppress { id } => Op::Unsuppress { id: id.clone() },
        Edit::Rename { from, to } => Op::Rename {
            from: from.clone(),
            to: to.clone(),
        },
    };
    forge_lang::edit::apply(src, &op).map_err(|e| e.text())
}

/// Keyword `arg` of `id`'s call taken out with its comma: the statement replaced by itself less
/// that argument (forge-lang's `ReplaceFeature`, so the editor sends only the language's own ops).
fn unset(src: &str, id: &str, arg: &str) -> Result<String, String> {
    let prog = syntax::parse(src).map_err(|e| format!("{e:?}"))?;
    for s in &prog {
        if let StmtKind::Assign { name, value } = &s.kind
            && name == id
            && let ExprKind::Call(_, args) = &value.kind
        {
            let Some(k) = args
                .iter()
                .position(|a| a.name.as_ref().is_some_and(|(n, _)| n == arg))
            else {
                return Ok(src.to_string());
            };
            let a = &args[k];
            let lo = a.name.as_ref().map_or(a.value.span.lo, |(_, sp)| sp.lo) as usize;
            let hi = a.value.span.hi as usize;
            // with the comma and space before it, or (the first) after it
            let (lo, hi) = if k > 0 {
                let before = src[..lo].trim_end();
                let c = before.strip_suffix(',').map_or(lo, |b| b.len());
                (c, hi)
            } else {
                let after = &src[hi..];
                let t = after.trim_start();
                let n = if let Some(t2) = t.strip_prefix(',') {
                    after.len() - t2.trim_start().len()
                } else {
                    0
                };
                (lo, hi + n)
            };
            let (slo, shi) = (s.span.lo as usize, s.span.hi as usize);
            let text = format!("{}{}", &src[slo..lo], &src[hi..shi]);
            return apply(src, &Edit::Replace { id: id.into(), text });
        }
    }
    Err(format!("no statement `{id}`"))
}

/// Statement `id`'s call replaced by `call` (`func(args)`), its name and the spacing before the
/// `=` as written.
pub fn replace_call(src: &str, id: &str, call: &str) -> Result<String, String> {
    let prog = syntax::parse(src).map_err(|e| format!("{e:?}"))?;
    let s = prog
        .iter()
        .find(|s| matches!(&s.kind, StmtKind::Assign { name, .. } if name == id))
        .ok_or_else(|| format!("no statement `{id}`"))?;
    let StmtKind::Assign { value, .. } = &s.kind else {
        unreachable!("an assignment")
    };
    let head = &src[s.span.lo as usize..value.span.lo as usize];
    apply(
        src,
        &Edit::Replace {
            id: id.into(),
            text: format!("{head}{call}"),
        },
    )
}

/// A name not used yet: `base`, else `base2`, `base3`, ...
pub fn free_name(stmts: &[Stmt], base: &str) -> String {
    let used = |n: &str| stmts.iter().any(|s| s.name == n) || OP_FUNCS.contains(&n);
    if !used(base) {
        return base.to_string();
    }
    (2..)
        .map(|k| format!("{base}{k}"))
        .find(|n| !used(n))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "machining
# a comment kept
shared tol = 0.01

f    = part(\"flange.fg\", group: \"N\")
m    = machine(\"generic-vmc3\")
t1   = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")
op1  = setup(f, machine: m, stock: box(margin: (3, 3, 2)), origin: \"top-center\", tol: tol)
skim = facing(op1, tool: t1, to: 18, step: 1)
prof = contour(op1, tool: t1, side: \"outside\", from: 12, to: -0.5, step: 4)
prog = program(op1, post: \"fanuc\", number: 1001)
";

    #[test]
    fn reads_the_statements() {
        let s = read(DOC).expect("parses");
        let names: Vec<&str> = s.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["f", "m", "t1", "op1", "skim", "prof", "prog"]);
        assert_eq!(s[4].role, Role::Op("facing".into()));
        assert_eq!(s[4].setup.as_deref(), Some("op1"));
        assert_eq!(s[4].arg("to"), Some("18"));
        assert_eq!(s[5].arg_str("side").as_deref(), Some("outside"));
        assert_eq!(s[3].arg("stock"), Some("box(margin: (3, 3, 2))"));
    }

    #[test]
    fn set_and_unset_round_trip() {
        let e = Edit::Set {
            id: "skim".into(),
            arg: "step".into(),
            text: "0.5".into(),
        };
        let t = apply(DOC, &e).expect("set");
        assert!(
            t.contains("skim = facing(op1, tool: t1, to: 18, step: 0.5)"),
            "{t}"
        );
        assert!(t.contains("# a comment kept"));
        let t2 = apply(
            &t,
            &Edit::Unset {
                id: "skim".into(),
                arg: "step".into(),
            },
        )
        .expect("unset");
        assert!(t2.contains("skim = facing(op1, tool: t1, to: 18)"), "{t2}");
        // a new keyword is put in
        let t3 = apply(
            DOC,
            &Edit::Set {
                id: "prof".into(),
                arg: "leave".into(),
                text: "0.2".into(),
            },
        )
        .expect("put in");
        assert_eq!(read(&t3).unwrap()[5].arg("leave"), Some("0.2"));
    }

    #[test]
    fn add_move_delete_suppress() {
        let t = apply(
            DOC,
            &Edit::Add {
                before: Some("prog".into()),
                text: "rough = clear(op1, tool: t1, from: 18, to: 12, step: 3, stepover: 4)".into(),
            },
        )
        .expect("add");
        let n: Vec<String> = read(&t).unwrap().into_iter().map(|s| s.name).collect();
        assert_eq!(n, ["f", "m", "t1", "op1", "skim", "prof", "rough", "prog"]);
        let t = apply(
            &t,
            &Edit::Move {
                id: "rough".into(),
                before: Some("prof".into()),
            },
        )
        .expect("move");
        let n: Vec<String> = read(&t).unwrap().into_iter().map(|s| s.name).collect();
        assert_eq!(n, ["f", "m", "t1", "op1", "skim", "rough", "prof", "prog"]);
        let s = apply(&t, &Edit::Suppress { id: "rough".into() }).expect("suppress");
        let st = read(&s).unwrap();
        let r = st.iter().find(|x| x.name == "rough").expect("still listed");
        assert!(r.suppressed);
        assert_eq!(st.iter().position(|x| x.name == "rough"), Some(5), "where it was");
        let u = apply(&s, &Edit::Unsuppress { id: "rough".into() }).expect("unsuppress");
        assert_eq!(u, t);
        let d = apply(&t, &Edit::Delete { id: "rough".into() }).expect("delete");
        assert_eq!(d, DOC);
    }

    #[test]
    fn free_names() {
        let s = read(DOC).unwrap();
        assert_eq!(free_name(&s, "skim"), "skim2");
        assert_eq!(free_name(&s, "pocket"), "pocket2");
        assert_eq!(free_name(&s, "bore"), "bore");
    }
}
