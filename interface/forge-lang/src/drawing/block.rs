//! Drawings in the same file: `drawing NAME(..) { .. }` blocks of a .fg, each a drawing of the
//! document in the drawing language. A block is read and made as a drawing file whose text is
//! `drawing`, the document's shared params, and the block's statements; what the drawing says
//! (errors, items) is put back where the block is, and an op on it writes inside the block.

use super::doc::{self, DrawError};
use super::ops::{self, DrawOp, Refused};
use super::regen::{self, Drawing};
use crate::syntax::{self, BlockKind, Span, StmtKind};
use std::path::Path;

/// A drawing block: its name, where it is written, and where its statements are (between its
/// braces).
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub name: String,
    pub span: Span,
    pub body: (usize, usize),
}

/// The document's drawing blocks, in order.
pub fn blocks(src: &str) -> Vec<Block> {
    let Ok(prog) = syntax::parse(src) else {
        return vec![];
    };
    prog.iter()
        .filter_map(|s| match &s.kind {
            StmtKind::Block {
                kind: BlockKind::Drawing,
                name,
                args,
                ..
            } => {
                let after = args
                    .last()
                    .map_or(s.span.lo as usize, |a| a.value.span.hi as usize);
                let open = after + src.get(after..)?.find('{')? + 1;
                let close = s.span.hi as usize - 1;
                (src.as_bytes().get(close) == Some(&b'}') && open <= close).then(|| Block {
                    name: name.clone(),
                    span: s.span,
                    body: (open, close),
                })
            }
            _ => None,
        })
        .collect()
}

/// A block as a drawing file's text, and how long its head is (`drawing` and the shared params,
/// before the block's own statements).
pub fn text(src: &str, name: &str) -> Option<(String, usize, Block)> {
    let b = blocks(src).into_iter().find(|b| b.name == name)?;
    let prog = syntax::parse(src).ok()?;
    let mut head = String::from("drawing\n");
    for s in &prog {
        if matches!(s.kind, StmtKind::Shared { .. }) {
            head.push_str(&src[s.span.lo as usize..s.span.hi as usize]);
            head.push('\n');
        }
    }
    let n = head.len();
    head.push_str(&src[b.body.0..b.body.1]);
    Some((head, n, b))
}

/// A span of the block's drawing text as where it is in the document (the head's own: the block).
fn back(sp: Span, head: usize, b: &Block) -> Span {
    if (sp.lo as usize) < head {
        return b.span;
    }
    let at = |x: u32| x as usize - head + b.body.0;
    Span::new(at(sp.lo), at(sp.hi.max(sp.lo)))
}

/// The block's drawing as written, not made (no model built): its items, sheets and errors,
/// spans in the document.
pub fn read(src: &str, own: &str, name: &str) -> Option<doc::Doc> {
    let (t, head, b) = text(src, name)?;
    let mut d = doc::read_with(&t, Some(own));
    for i in &mut d.items {
        i.span = back(i.span, head, &b);
    }
    for e in &mut d.errors {
        e.span = back(e.span, head, &b);
    }
    Some(d)
}

/// The block's drawing made: the document's model drawn (`own`, the .fg's name beside `dir`),
/// errors where they are in the document.
pub fn make(src: &str, dir: &Path, root: Option<&Path>, own: &str, name: &str) -> Option<Drawing> {
    let (t, head, b) = text(src, name)?;
    let mut d = regen::make_own(&t, dir, root, own);
    for i in &mut d.doc.items {
        i.span = back(i.span, head, &b);
    }
    for e in &mut d.errors {
        e.span = back(e.span, head, &b);
    }
    Some(d)
}

/// A drawing op on the block, written inside it: refused as the op is on a drawing file, or if it
/// would change anything outside the block's statements.
pub fn apply_checked(src: &str, name: &str, op: &DrawOp) -> Result<String, Refused> {
    let Some((t, head, b)) = text(src, name) else {
        return Err(Refused::NoItem {
            item: name.to_string(),
        });
    };
    // the op's own refusals (no such item, a name taken, not a call), as on a drawing file
    let plain = ops::apply(&t, op)?;
    let out = match op {
        // its lines, indented as the block's are
        DrawOp::AddItem { text } => {
            let mut out = t.clone();
            if !out.ends_with('\n') {
                out.push('\n');
            }
            for l in text.trim_end().lines() {
                out.push_str("  ");
                out.push_str(l.trim_start());
                out.push('\n');
            }
            out
        }
        // the statement's whole lines, its indent too: the next line as it was
        DrawOp::DeleteItem { item } => {
            let prog = syntax::parse(&t).map_err(|e| Refused::Breaks {
                problem: doc::Problem::Syntax { message: e.text() },
            })?;
            let s = prog
                .iter()
                .find(|s| matches!(&s.kind, StmtKind::Assign { name, .. } if name == item))
                .ok_or(Refused::NoItem { item: item.clone() })?;
            // `span`, not `full`: full runs on to the next statement
            let lo = t[..s.span.lo as usize].rfind('\n').map_or(0, |k| k + 1);
            let hi = s.span.hi as usize;
            let hi = t[hi..].find('\n').map_or(t.len(), |k| hi + k + 1);
            format!("{}{}", &t[..lo], &t[hi..])
        }
        DrawOp::SetArg { .. } => plain,
    };
    if out.get(..head) != Some(&t[..head]) {
        return Err(Refused::Breaks {
            problem: doc::Problem::Syntax {
                message: "an edit of a drawing block stays inside its braces".into(),
            },
        });
    }
    // checked as the block reads (`part()` is this document), not as a drawing file would read
    // its text: there `part()` fails and hides what reads past it
    let read = |x: &str| doc::read_with(x, Some(SELF)).errors;
    let before = read(&t);
    if let Some(e) = read(&out)
        .iter()
        .find(|e| !before.iter().any(|x| x.problem == e.problem))
    {
        return Err(Refused::Breaks {
            problem: e.problem.clone(),
        });
    }
    let mut body = out[head..].to_string();
    if !body.ends_with('\n') {
        body.push('\n');
    }
    Ok(format!("{}{}{}", &src[..b.body.0], body, &src[b.body.1..]))
}

/// The document a block's checks read `part()` as: its own file, whatever its name.
const SELF: &str = "this.fg";

/// What `fg run --json` says of the blocks, without drawing them: each one's sheets, views (and
/// the model each draws) and errors.
pub fn summary(src: &str, own: &str) -> forge_agent::json::Json {
    use forge_agent::json::Json;
    let line = |lo: u32| src[..(lo as usize).min(src.len())].matches('\n').count() + 1;
    Json::Arr(
        blocks(src)
            .iter()
            .filter_map(|b| {
                let d = read(src, own, &b.name)?;
                let sheets = d
                    .items
                    .iter()
                    .filter_map(|i| match &i.kind {
                        doc::ItemKind::Sheet(s) => Some(Json::obj(vec![
                            ("id", i.id.as_str().into()),
                            ("size", s.size.as_str().into()),
                        ])),
                        _ => None,
                    })
                    .collect();
                let views = d
                    .items
                    .iter()
                    .filter_map(|i| match &i.kind {
                        doc::ItemKind::View(v) => {
                            let model = match &v.kind {
                                doc::ViewKind::Base { model, .. } | doc::ViewKind::Flat { model } => {
                                    Json::from(model.as_str())
                                }
                                _ => Json::Null,
                            };
                            Some(Json::obj(vec![
                                ("id", i.id.as_str().into()),
                                ("sheet", v.sheet.as_deref().map_or(Json::Null, Json::from)),
                                ("model", model),
                            ]))
                        }
                        _ => None,
                    })
                    .collect();
                let errors: Vec<Json> = d.errors.iter().map(|e: &DrawError| e.json(src)).collect();
                Some(Json::obj(vec![
                    ("id", b.name.as_str().into()),
                    ("line", line(b.span.lo).into()),
                    ("sheets", Json::Arr(sheets)),
                    ("views", Json::Arr(views)),
                    ("errors", Json::Arr(errors)),
                ]))
            })
            .collect(),
    )
}

/// The statements of block `name` as written (between its braces).
pub fn body<'a>(src: &'a str, name: &str) -> Option<&'a str> {
    let b = blocks(src).into_iter().find(|b| b.name == name)?;
    src.get(b.body.0..b.body.1)
}

/// `src` with block `name`'s statements those of `from`'s block of that name.
pub fn with_body(src: &str, name: &str, from: &str) -> Option<String> {
    let b = blocks(src).into_iter().find(|b| b.name == name)?;
    let new = body(from, name)?;
    Some(format!("{}{new}{}", &src[..b.body.0], &src[b.body.1..]))
}

/// Unsaved text `mine` (edited from `base`) and the file now on disk `disk`, where only drawing
/// blocks changed from `base`: `mine` with those blocks as on disk. None if the disk changed
/// anything else, or a block both sides changed: then there is no one text both edits make.
pub fn merge(base: &str, mine: &str, disk: &str) -> Option<String> {
    let names: Vec<String> = blocks(disk).into_iter().map(|b| b.name).collect();
    if names.len() != blocks(base).len() {
        return None;
    }
    let mut out = mine.to_string();
    let mut back = disk.to_string();
    for n in &names {
        if body(base, n) == body(disk, n) {
            continue;
        }
        // the disk's change to this block, onto mine: refused if mine changed it too
        if body(mine, n) != body(base, n) {
            return None;
        }
        out = with_body(&out, n, disk)?;
        back = with_body(&back, n, base)?;
    }
    // with its blocks put back as they were, the disk is the base: nothing else changed there
    (back == base).then_some(out)
}
