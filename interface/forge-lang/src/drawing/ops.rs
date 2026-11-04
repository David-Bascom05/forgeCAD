//! Typed edits of a drawing, pure functions of (source, op): what the app writes for every
//! placement and every drag, and what errors offer as fixes. Each is checked: the result must
//! read with no more errors than the source had.

use super::doc;
use crate::syntax::{self, ExprKind, StmtKind};
use forge_agent::json::Json;

#[derive(Clone, Debug, PartialEq)]
pub enum DrawOp {
    /// Sets one argument of an item's call (`at`, `scale`, `text`..), adding it if absent;
    /// `value` is the argument's source text.
    SetArg {
        item: String,
        arg: String,
        value: String,
    },
    /// Adds an item: a whole line, `name = func(..)`.
    AddItem {
        text: String,
    },
    DeleteItem {
        item: String,
    },
}

impl DrawOp {
    pub fn json(&self) -> Json {
        match self {
            DrawOp::SetArg { item, arg, value } => Json::obj(vec![
                ("op", "SetArg".into()),
                ("item", item.as_str().into()),
                ("arg", arg.as_str().into()),
                ("value", value.as_str().into()),
            ]),
            DrawOp::AddItem { text } => {
                Json::obj(vec![("op", "AddItem".into()), ("text", text.as_str().into())])
            }
            DrawOp::DeleteItem { item } => {
                Json::obj(vec![("op", "DeleteItem".into()), ("item", item.as_str().into())])
            }
        }
    }

    pub fn from_json(j: &Json) -> Option<DrawOp> {
        let s = |k: &str| j.get(k).and_then(Json::str).map(str::to_string);
        Some(match j.get("op").and_then(Json::str)? {
            "SetArg" => DrawOp::SetArg {
                item: s("item")?,
                arg: s("arg")?,
                value: s("value")?,
            },
            "AddItem" => DrawOp::AddItem { text: s("text")? },
            "DeleteItem" => DrawOp::DeleteItem { item: s("item")? },
            _ => return None,
        })
    }
}

/// Why an op was refused; nothing is written.
#[derive(Clone, Debug, PartialEq)]
pub enum Refused {
    NoItem {
        item: String,
    },
    /// The item's line is not a call with arguments.
    NotACall {
        item: String,
    },
    /// The result reads with a problem the source did not have.
    Breaks {
        problem: doc::Problem,
    },
    Taken {
        item: String,
    },
}

impl Refused {
    pub fn text(&self) -> String {
        match self {
            Refused::NoItem { item } => format!("no item `{item}`"),
            Refused::NotACall { item } => format!("`{item}` is not an item call"),
            Refused::Breaks { problem } => {
                format!("the edit would make the drawing wrong: {}", problem.text())
            }
            Refused::Taken { item } => format!("`{item}` is already an item"),
        }
    }
}

fn splice(src: &str, lo: usize, hi: usize, with: &str) -> String {
    format!("{}{with}{}", &src[..lo], &src[hi..])
}

/// The op applied to the source, unchecked.
pub fn apply(src: &str, op: &DrawOp) -> Result<String, Refused> {
    let prog = syntax::parse(src).map_err(|e| Refused::Breaks {
        problem: doc::Problem::Syntax { message: e.text() },
    })?;
    let find = |item: &str| {
        prog.iter()
            .find(|s| matches!(&s.kind, StmtKind::Assign { name, .. } if name == item))
            .ok_or(Refused::NoItem {
                item: item.to_string(),
            })
    };
    match op {
        DrawOp::SetArg { item, arg, value } => {
            let s = find(item)?;
            let StmtKind::Assign { value: call, .. } = &s.kind else {
                unreachable!("found an assignment")
            };
            let ExprKind::Call(_, args) = &call.kind else {
                return Err(Refused::NotACall { item: item.clone() });
            };
            // a positional argument may be set by its index
            let named = args.iter().find(|a| a.name.as_ref().is_some_and(|n| n.0 == *arg));
            let positional = arg
                .parse::<usize>()
                .ok()
                .and_then(|k| args.iter().filter(|a| a.name.is_none()).nth(k));
            match named.or(positional) {
                Some(a) => Ok(splice(
                    src,
                    a.value.span.lo as usize,
                    a.value.span.hi as usize,
                    value,
                )),
                None => {
                    let close = call.span.hi as usize - 1;
                    let sep = if args.is_empty() { "" } else { ", " };
                    Ok(splice(src, close, close, &format!("{sep}{arg}: {value}")))
                }
            }
        }
        DrawOp::AddItem { text } => {
            if let Some(name) = text.split('=').next().map(str::trim)
                && find(name).is_ok()
            {
                return Err(Refused::Taken {
                    item: name.to_string(),
                });
            }
            let mut out = src.to_string();
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(text.trim_end());
            out.push('\n');
            Ok(out)
        }
        DrawOp::DeleteItem { item } => {
            let s = find(item)?;
            Ok(splice(src, s.full.lo as usize, s.full.hi as usize, ""))
        }
    }
}

/// The op applied, refused if the result reads with a problem the source did not have (a
/// reference's sickness is the model's, and is not counted).
pub fn apply_checked(src: &str, op: &DrawOp) -> Result<String, Refused> {
    let out = apply(src, op)?;
    let before = doc::read(src).errors;
    let after = doc::read(&out).errors;
    if let Some(e) = after
        .iter()
        .find(|e| !before.iter().any(|b| b.problem == e.problem))
    {
        return Err(Refused::Breaks {
            problem: e.problem.clone(),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str =
        "drawing\nm = part(\"a.fg\")\ns1 = sheet(\"B\")\nv = base_view(m, sheet: s1, at: (100, 100))\n";

    #[test]
    fn set_add_delete() {
        let out = apply_checked(
            SRC,
            &DrawOp::SetArg {
                item: "v".into(),
                arg: "at".into(),
                value: "(120, 90)".into(),
            },
        )
        .unwrap();
        assert!(out.contains("at: (120, 90))"));
        let out = apply_checked(
            &out,
            &DrawOp::SetArg {
                item: "v".into(),
                arg: "scale".into(),
                value: "0.5".into(),
            },
        )
        .unwrap();
        assert!(out.contains("at: (120, 90), scale: 0.5)"), "{out}");
        let out = apply_checked(
            &out,
            &DrawOp::AddItem {
                text: "r = projected(v, at: (250, 90))".into(),
            },
        )
        .unwrap();
        assert!(doc::read(&out).errors.is_empty());
        // a view another item reads cannot go
        let gone = apply_checked(&out, &DrawOp::DeleteItem { item: "v".into() });
        assert!(matches!(gone, Err(Refused::Breaks { .. })), "{gone:?}");
        let out = apply_checked(&out, &DrawOp::DeleteItem { item: "r".into() }).unwrap();
        assert_eq!(doc::read(&out).items.len(), 3);
        let bad = apply_checked(
            SRC,
            &DrawOp::SetArg {
                item: "v".into(),
                arg: "scale".into(),
                value: "\"big\"".into(),
            },
        );
        assert!(matches!(bad, Err(Refused::Breaks { .. })));
    }
}
