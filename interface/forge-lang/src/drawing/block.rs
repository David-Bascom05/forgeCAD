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
    let out = ops::apply_checked(&t, op)?;
    if out.get(..head) != Some(&t[..head]) {
        return Err(Refused::Breaks {
            problem: doc::Problem::Syntax {
                message: "an edit of a drawing block stays inside its braces".into(),
            },
        });
    }
    // the block's lines indented as written inside it
    let mut body = String::new();
    for (k, l) in out[head..].split_inclusive('\n').enumerate() {
        // a line a delete left only its indent of
        if k > 0 && l.trim().is_empty() && l != "\n" {
            continue;
        }
        let fresh = !t[head..].contains(l.trim_end());
        if k > 0 && fresh && !l.trim().is_empty() && !l.starts_with([' ', '\t']) {
            body.push_str("  ");
        }
        body.push_str(l);
    }
    if !body.ends_with('\n') {
        body.push('\n');
    }
    Ok(format!("{}{}{}", &src[..b.body.0], body, &src[b.body.1..]))
}

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
