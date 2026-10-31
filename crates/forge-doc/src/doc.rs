//! The document: every line of a part in its unevaluated form (parameters, sketches with their
//! rows, feature lines with their expressions and references), its lossless print as the part
//! file, and the edits a client makes to it.

use crate::text::{self, Line};
use forge_model::Unit;

/// The solid features whose op, unwritten, is `new` on the first of them and `add` after.
const SOLIDS: [&str; 6] = ["extrude", "revolve", "ball", "sweep", "coil", "loft"];

/// An item's identity, kept through edits that move it. A feature line's is also the id its
/// faces, edges and vertices are named from, kept in the file (`id N` where it is not the
/// line's place among the feature lines), so inserting, deleting or moving lines renames
/// nothing. Blank and comment lines have ids from `BLANK` up, never written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemId(pub u64);

#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub items: Vec<Item>,
    /// The next blank line's id.
    next: u64,
    /// The highest feature id this document has held, so an id taken out is not handed out
    /// again while it is open.
    high: u64,
}

/// Where blank lines' ids start.
pub const BLANK: u64 = 1 << 63;

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub id: ItemId,
    pub label: Option<String>,
    pub body: Body,
    /// The comment after it, without the `#`.
    pub note: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    /// A blank line, or a comment alone on its line (the note).
    Blank,
    /// `name = expr [unit]`.
    Param { name: String, expr: String, unit: Unit },
    /// A sketch: its header after `sketch` (a frame or a reference) and its rows.
    Sketch { head: Vec<Arg>, rows: Vec<Row> },
    /// Any other feature: its kind and arguments.
    Feature { kind: String, args: Vec<Arg> },
}

/// A sketch row: geometry, a constraint or a dimension (none: a comment alone).
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub args: Vec<String>,
    pub note: Option<String>,
}

/// One argument as written: a word, a number or expression, a reference.
#[derive(Clone, Debug, PartialEq)]
pub struct Arg {
    pub text: String,
    /// The item it names, once read as a feature: printed by that item's place, so it follows
    /// the item when lines are inserted, deleted or moved.
    pub to: Option<ItemId>,
}

impl Arg {
    fn new(text: impl Into<String>) -> Arg {
        Arg {
            text: text.into(),
            to: None,
        }
    }
}

/// A change to a document.
#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    /// Sets a parameter's expression.
    SetParam {
        name: String,
        expr: String,
    },
    /// Sets a dimension: the value at the end of a sketch row.
    SetDimension {
        item: ItemId,
        row: usize,
        value: String,
    },
    /// Sets one argument of a feature or of a sketch's header: a value, a reference, a word.
    SetArg {
        item: ItemId,
        arg: usize,
        text: String,
    },
    /// Inserts the item (or items) a text reads as, before the item at `at` (or at the end).
    Insert {
        at: usize,
        text: String,
    },
    Delete {
        item: ItemId,
    },
    /// Moves an item to position `to`.
    Move {
        item: ItemId,
        to: usize,
    },
}

/// A line's code and its comment (a `#` outside a quoted string on).
fn split_note(raw: &str) -> (&str, Option<String>) {
    let code = text::uncommented(raw);
    let note = raw[code.len()..].strip_prefix('#').map(|n| n.trim().to_string());
    (code, note)
}

impl Item {
    /// Its arguments: a feature's, or a sketch's header.
    pub fn args(&self) -> &[Arg] {
        match &self.body {
            Body::Sketch { head, .. } => head,
            Body::Feature { args, .. } => args,
            _ => &[],
        }
    }
}

impl Document {
    /// Reads a part file. Every line becomes an item; what a line means is found when it is
    /// evaluated, so a line in error still reads and prints back.
    pub fn parse(src: &str) -> Document {
        let mut d = Document {
            items: Vec::new(),
            next: BLANK,
            high: 0,
        };
        d.items = d.read(src, true);
        d.high = d
            .items
            .iter()
            .map(|it| it.id.0)
            .filter(|&i| i < BLANK)
            .max()
            .unwrap_or(0);
        d
    }

    /// The items a text reads as. A feature line without an `id` takes its place among the
    /// feature lines (`placed`, a whole file) or the next id unused (lines inserted).
    fn read(&mut self, src: &str, placed: bool) -> Vec<Item> {
        let mut out: Vec<Item> = Vec::new();
        let mut fresh = self.high + 1;
        let mut block = false;
        for raw in src.lines() {
            let (code, note) = split_note(raw);
            let mut toks = text::tokens(code);
            if block {
                let Some(Item {
                    body: Body::Sketch { rows, .. },
                    ..
                }) = out.last_mut()
                else {
                    unreachable!("a block has its header")
                };
                if toks.len() == 1 && toks[0] == "end" {
                    block = false;
                } else if !toks.is_empty() || note.is_some() {
                    rows.push(Row { args: toks, note });
                }
                continue;
            }
            let label = match toks.first() {
                Some(t) if t.len() > 1 && t.ends_with(':') => {
                    Some(toks.remove(0).trim_end_matches(':').to_string())
                }
                _ => None,
            };
            let given = match (
                toks.first().map(String::as_str),
                toks.get(1).and_then(|t| t.parse::<u64>().ok()),
            ) {
                (Some("id"), Some(n)) if n > 0 && n < BLANK && toks.len() > 2 => {
                    toks.drain(..2);
                    Some(n)
                }
                _ => None,
            };
            let body = if toks.is_empty() {
                Body::Blank
            } else if toks.len() > 1 && toks[1] == "=" {
                let mut rest = toks.split_off(2);
                let unit = match rest.last().map(|s| s.as_str()) {
                    Some("mm") => Unit::Mm,
                    Some("deg") => Unit::Deg,
                    _ => Unit::None,
                };
                if unit != Unit::None {
                    rest.pop();
                }
                Body::Param {
                    name: toks.swap_remove(0),
                    expr: rest.join(" "),
                    unit,
                }
            } else if toks[0] == "sketch" {
                block = true;
                Body::Sketch {
                    head: toks.drain(1..).map(Arg::new).collect(),
                    rows: Vec::new(),
                }
            } else {
                let kind = toks.remove(0);
                Body::Feature {
                    kind,
                    args: toks.into_iter().map(Arg::new).collect(),
                }
            };
            let id = match (&body, given) {
                (Body::Blank, _) => {
                    self.next += 1;
                    self.next - 1
                }
                (_, Some(n)) => n,
                (_, None) if placed => out.iter().filter(|it| it.id.0 < BLANK).count() as u64 + 1,
                _ => {
                    fresh += 1;
                    fresh - 1
                }
            };
            let id = ItemId(id);
            out.push(Item {
                id,
                label,
                body,
                note,
            });
        }
        out
    }

    /// The part file.
    pub fn print(&self) -> String {
        let mut out = String::new();
        for (k, it) in self.items.iter().enumerate() {
            let note = |out: &mut String, note: &Option<String>, code: bool| {
                if let Some(n) = note {
                    out.push_str(if code { "  # " } else { "# " });
                    out.push_str(n);
                }
            };
            if let Some(l) = &it.label {
                out.push_str(l);
                out.push_str(": ");
            }
            if let Some(n) = self.written_id(k) {
                out.push_str(&format!("id {n} "));
            }
            out.push_str(&self.code(k).join(" "));
            note(&mut out, &it.note, !matches!(it.body, Body::Blank));
            if let Body::Sketch { rows, .. } = &it.body {
                for r in rows {
                    out.push_str("\n ");
                    for a in &r.args {
                        out.push(' ');
                        out.push_str(a);
                    }
                    if r.args.is_empty() {
                        out.push(' ');
                    }
                    note(&mut out, &r.note, !r.args.is_empty());
                }
                out.push_str("\nend");
            }
            out.push('\n');
        }
        out
    }

    /// An item's first line as tokens: its kind and arguments (references by the places of
    /// the items they name now). Blank items have none.
    fn code(&self, k: usize) -> Vec<String> {
        let arg = |a: &Arg| match a.to.and_then(|id| self.name_of(id, &a.text)) {
            Some(n) => n,
            None => a.text.clone(),
        };
        match &self.items[k].body {
            Body::Blank => vec![],
            Body::Param { name, expr, unit } => {
                let mut t = vec![name.clone(), "=".into()];
                t.extend(text::tokens(expr));
                match unit {
                    Unit::None => {}
                    Unit::Mm => t.push("mm".into()),
                    Unit::Deg => t.push("deg".into()),
                }
                t
            }
            Body::Sketch { head, .. } => std::iter::once("sketch".into())
                .chain(head.iter().map(arg))
                .collect(),
            Body::Feature { kind, args } => std::iter::once(kind.clone())
                .chain(args.iter().map(arg))
                .collect(),
        }
    }

    /// How an item is named here, in the style `was` names it: its label, `fN`, or kind and
    /// count. None if it is gone.
    fn name_of(&self, id: ItemId, was: &str) -> Option<String> {
        let k = self.items.iter().position(|it| it.id == id)?;
        let it = &self.items[k];
        fn kind(it: &Item) -> Option<&str> {
            match &it.body {
                Body::Feature { kind, .. } => Some(kind.as_str()),
                Body::Sketch { .. } => Some("sketch"),
                Body::Param { .. } => Some("param"),
                Body::Blank => None,
            }
        }
        let place = self.items[..k].iter().filter(|i| kind(i).is_some()).count();
        if was.starts_with('f') && was[1..].bytes().all(|c| c.is_ascii_digit()) {
            return Some(format!("f{}", place + 1));
        }
        if it.label.as_deref() == Some(was) {
            return Some(was.to_string());
        }
        let kd = kind(it)?;
        let n = self.items[..=k].iter().filter(|i| kind(i) == Some(kd)).count();
        Some(format!("{kd}{n}"))
    }

    /// Item `k`'s id when the file must say it: a feature line not at the place its id names.
    fn written_id(&self, k: usize) -> Option<u64> {
        let id = self.items[k].id.0;
        let place = self.items[..k].iter().filter(|it| it.id.0 < BLANK).count() as u64 + 1;
        (id < BLANK && id != place).then_some(id)
    }

    /// The lines a part history is read from, one per item that is not blank, numbered by
    /// where they print.
    pub fn lines(&self) -> Vec<Line> {
        let mut out = Vec::new();
        let mut no = 1;
        for (k, it) in self.items.iter().enumerate() {
            let mut toks = self.code(k);
            if let Body::Sketch { rows, .. } = &it.body {
                for r in rows.iter().filter(|r| !r.args.is_empty()) {
                    toks.push(";".into());
                    toks.extend(r.args.iter().cloned());
                }
            }
            if !toks.is_empty() {
                out.push(Line {
                    id: it.id.0,
                    no,
                    label: it.label.clone(),
                    toks,
                });
            }
            no += 1 + match &it.body {
                Body::Sketch { rows, .. } => rows.len() + 1,
                _ => 0,
            };
        }
        out
    }

    /// The items lines come from, in order (the blank ones left out).
    pub fn features(&self) -> impl Iterator<Item = &Item> {
        self.items.iter().filter(|it| !matches!(it.body, Body::Blank))
    }

    /// Records what feature line `line`'s tokens name (as `text::parse_line_refs` says), so the
    /// references follow those items through later edits.
    pub fn link(&mut self, line: usize, refs: &[(usize, usize)]) {
        let ids: Vec<ItemId> = self.features().map(|it| it.id).collect();
        let Some(&id) = ids.get(line) else { return };
        let it = self.items.iter_mut().find(|it| it.id == id).expect("listed");
        let args = match &mut it.body {
            Body::Sketch { head, .. } => head,
            Body::Feature { args, .. } => args,
            _ => return,
        };
        for &(tok, feature) in refs {
            if let (Some(a), Some(&to)) = (tok.checked_sub(1).and_then(|k| args.get_mut(k)), ids.get(feature))
            {
                a.to = Some(to);
            }
        }
    }

    /// Takes the identities and links of `old`'s first `lines` feature lines, which read the
    /// same as this document's (a re-read of an edited file keeps what it can).
    pub fn adopt(&mut self, old: &Document, lines: usize) {
        self.next = self.next.max(old.next);
        self.high = self.high.max(old.high);
        let olds: Vec<(ItemId, Vec<Option<ItemId>>)> = old
            .features()
            .take(lines)
            .map(|it| (it.id, it.args().iter().map(|a| a.to).collect()))
            .collect();
        for (it, (id, tos)) in self.items.iter_mut().filter(|it| it.id.0 < BLANK).zip(olds) {
            if it.id != id {
                continue;
            }
            if let Body::Sketch { head: args, .. } | Body::Feature { args, .. } = &mut it.body {
                for (a, to) in args.iter_mut().zip(tos) {
                    a.to = to;
                }
            }
        }
    }

    /// The item printed on file line `no` (from 1), and the sketch row if it is one.
    pub fn at_line(&self, no: usize) -> Option<(usize, Option<usize>)> {
        let mut first = 1;
        for (k, it) in self.items.iter().enumerate() {
            let rows = match &it.body {
                Body::Sketch { rows, .. } => rows.len() + 1,
                _ => 0,
            };
            if no >= first && no <= first + rows {
                let row = (no > first && no < first + rows).then(|| no - first - 1);
                return Some((k, row));
            }
            first += 1 + rows;
        }
        None
    }

    fn index(&self, id: ItemId) -> Result<usize, String> {
        self.items
            .iter()
            .position(|it| it.id == id)
            .ok_or_else(|| format!("no item {}", id.0))
    }

    /// The first solid feature line and whether its op is written (unwritten, it is `new` by
    /// its place alone).
    fn first_solid(&self) -> Option<(ItemId, bool)> {
        self.items.iter().find_map(|it| match &it.body {
            Body::Feature { kind, args } if SOLIDS.contains(&kind.as_str()) => Some((
                it.id,
                args.first()
                    .is_some_and(|a| text::OPS.iter().any(|o| o.0 == a.text)),
            )),
            _ => None,
        })
    }

    /// A first solid line whose op was unwritten, moved off first place by an edit, keeps
    /// making a new body: its `new` is written.
    fn keep_new(&mut self, was: Option<(ItemId, bool)>) {
        let Some((id, false)) = was else { return };
        if self.first_solid().is_some_and(|f| f.0 == id) {
            return;
        }
        if let Some(Body::Feature { args, .. }) = self
            .items
            .iter_mut()
            .find(|it| it.id == id)
            .map(|it| &mut it.body)
        {
            args.insert(0, Arg::new("new"));
        }
    }

    /// Applies an edit.
    pub fn apply(&mut self, e: Edit) -> Result<(), String> {
        match e {
            Edit::SetParam { name, expr } => {
                let p = self.items.iter_mut().find_map(|it| match &mut it.body {
                    Body::Param { name: n, expr, .. } if *n == name => Some(expr),
                    _ => None,
                });
                *p.ok_or_else(|| format!("no parameter '{name}'"))? = expr;
            }
            Edit::SetDimension { item, row, value } => {
                let k = self.index(item)?;
                let Body::Sketch { rows, .. } = &mut self.items[k].body else {
                    return Err(format!("item {} is not a sketch", item.0));
                };
                let r = rows
                    .get_mut(row)
                    .ok_or_else(|| format!("the sketch has no row {row}"))?;
                if !matches!(
                    r.args.first().map(|s| s.as_str()),
                    Some("dist" | "hdist" | "vdist" | "rad" | "dia" | "ang")
                ) {
                    return Err(format!("row {row} is not a dimension"));
                }
                *r.args.last_mut().expect("a dimension has a value") = value;
            }
            Edit::SetArg { item, arg, text } => {
                let k = self.index(item)?;
                let args = match &mut self.items[k].body {
                    Body::Sketch { head, .. } => head,
                    Body::Feature { args, .. } => args,
                    _ => return Err(format!("item {} has no arguments", item.0)),
                };
                *args.get_mut(arg).ok_or_else(|| format!("no argument {arg}"))? = Arg::new(text);
            }
            Edit::Insert { at, text } => {
                let first = self.first_solid();
                let new = self.read(&text, false);
                self.high = new
                    .iter()
                    .map(|it| it.id.0)
                    .filter(|&i| i < BLANK)
                    .fold(self.high, u64::max);
                let at = at.min(self.items.len());
                self.items.splice(at..at, new);
                self.keep_new(first);
            }
            Edit::Delete { item } => {
                let k = self.index(item)?;
                self.items.remove(k);
            }
            Edit::Move { item, to } => {
                let first = self.first_solid();
                let k = self.index(item)?;
                let it = self.items.remove(k);
                self.items.insert(to.min(self.items.len()), it);
                self.keep_new(first);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "# a plate\nw = 40  # mm\nd = w*3 + 1 mm\n\nsk: sketch on plane1 at 0 0 6\n  rect 0 0 w (d - 1)  # outline\n  # a comment row\n  dist l1 w\nend\nbase: extrude 6 sk\nhole 3 on sketch1 l1.a thru\n";

    #[test]
    fn prints_back() {
        let d = Document::parse(SRC);
        assert_eq!(d.print(), SRC);
        assert_eq!(Document::parse(&d.print()), d);
        let lines = d.lines();
        assert_eq!(lines.iter().map(|l| l.no).collect::<Vec<_>>(), [2, 3, 5, 10, 11]);
        assert_eq!(
            [4, 5, 8, 9, 10, 12].map(|n| d.at_line(n)),
            [
                Some((3, None)),
                Some((4, None)),
                Some((4, Some(2))),
                Some((4, None)),
                Some((5, None)),
                None
            ]
        );
        assert_eq!(
            lines[2].toks.join(" "),
            "sketch on plane1 at 0 0 6 ; rect 0 0 w (d - 1) ; dist l1 w"
        );
        assert_eq!(lines[1].toks.join(" "), "d = w*3 + 1 mm");
    }

    #[test]
    fn edits() {
        let mut d = Document::parse(SRC);
        let id = |d: &Document, k: usize| d.items[k].id;
        d.apply(Edit::SetParam {
            name: "w".into(),
            expr: "50".into(),
        })
        .expect("w");
        assert!(d.print().contains("\nw = 50  # mm\n"));
        assert!(
            d.apply(Edit::SetParam {
                name: "q".into(),
                expr: "1".into()
            })
            .is_err()
        );
        d.apply(Edit::SetDimension {
            item: id(&d, 4),
            row: 2,
            value: "w/2".into(),
        })
        .expect("dist");
        assert!(d.print().contains("  dist l1 w/2\n"));
        assert!(
            d.apply(Edit::SetDimension {
                item: id(&d, 4),
                row: 0,
                value: "1".into()
            })
            .is_err()
        );
        // the hole names the sketch: it follows when a line goes in above
        d.link(4, &[(3, 2)]);
        d.apply(Edit::Insert {
            at: 4,
            text: "sketch\n  rect 0 0 1 1\nend".into(),
        })
        .expect("insert");
        assert!(
            d.print().ends_with("hole 3 on sketch2 l1.a thru\n"),
            "{}",
            d.print()
        );
        let hole = id(&d, 7);
        d.apply(Edit::Move { item: hole, to: 5 }).expect("move");
        assert!(
            d.print()
                .contains("id 5 hole 3 on sketch2 l1.a thru\nsk: id 3 sketch"),
            "{}",
            d.print()
        );
        d.apply(Edit::SetArg {
            item: hole,
            arg: 0,
            text: "4".into(),
        })
        .expect("arg");
        assert!(d.print().contains("hole 4 on sketch2"), "{}", d.print());
        // ids read back as written; the rest take their places
        let back = Document::parse(&d.print());
        assert_eq!(back.print(), d.print());
        let ids = |d: &Document| d.features().map(|it| it.id).collect::<Vec<_>>();
        assert_eq!(ids(&back), ids(&d));
        d.apply(Edit::Delete { item: hole }).expect("delete");
        assert!(!d.print().contains("hole"));
        // the hole's id is not handed out again
        d.apply(Edit::Insert {
            at: usize::MAX,
            text: "ball 1".into(),
        })
        .expect("insert");
        assert!(d.print().ends_with("id 7 ball 1\n"), "{}", d.print());
    }

    /// A solid moved above the first leaves the first making a new body, its `new` written.
    #[test]
    fn moves_keep_the_bodies() {
        let mut d = Document::parse("extrude 1 rect 0 0 1 1\nball new 2 at 5 0 0\n");
        let ball = d.items[1].id;
        d.apply(Edit::Move { item: ball, to: 0 }).expect("move");
        assert_eq!(d.print(), "ball new 2 at 5 0 0\nextrude new 1 rect 0 0 1 1\n");
        // written ops are left as they are
        d.apply(Edit::Move { item: ball, to: 1 }).expect("move back");
        assert_eq!(d.print(), "extrude new 1 rect 0 0 1 1\nball new 2 at 5 0 0\n");
    }
}
