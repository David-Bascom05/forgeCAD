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
    pub fn new(text: impl Into<String>) -> Arg {
        Arg {
            text: text.into(),
            to: None,
        }
    }
}

/// A change to a document, by stable id: what a tool produces, what the log records and what
/// the protocol will carry (`wire` is its binary form).
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    /// Sets a parameter's expression.
    SetParam {
        item: ItemId,
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
    /// Replaces the item with this id, whole.
    SetItem {
        item: Item,
    },
    /// Inserts items, ids already given, before `before` (or at the end).
    Insert {
        before: Option<ItemId>,
        items: Vec<Item>,
    },
    Delete {
        item: ItemId,
    },
    /// Moves an item before `before` (or to the end).
    Move {
        item: ItemId,
        before: Option<ItemId>,
    },
    /// The whole file, as read: where a log starts, and an edit made by hand.
    Text {
        src: String,
    },
}

/// The words of a selector that could name a feature: a name with a number on its end.
fn selector_words(text: &str) -> impl Iterator<Item = &str> {
    text.split([' ', '(', ')']).filter(|w| {
        let name = w.trim_end_matches(|c: char| c.is_ascii_digit());
        name.len() < w.len() && name.bytes().next().is_some_and(|c| c.is_ascii_alphabetic())
    })
}

/// `text` with the whole word `from` as `to`, where it stands alone between spaces and brackets.
fn rename_word(text: &str, from: &str, to: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut word = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        if matches!(c, ' ' | '(' | ')') {
            out.push_str(if word == from { to } else { &word });
            word.clear();
            out.push(c);
        } else {
            word.push(c);
        }
    }
    out.pop();
    out
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
            .fold(d.high, u64::max);
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
            // `next N`: ids below N may have been used, by lines since taken out
            if placed
                && note.is_none()
                && toks.len() == 2
                && toks[0] == "next"
                && let Ok(n) = toks[1].parse::<u64>()
                && (1..BLANK).contains(&n)
            {
                self.high = self.high.max(n - 1);
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
                (_, None) if placed => 0,
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
        if placed {
            // a line without an id takes its place, unless a line says that id is its own
            let given: Vec<u64> = out
                .iter()
                .map(|it| it.id.0)
                .filter(|&i| i > 0 && i < BLANK)
                .collect();
            let lines = out.iter().filter(|it| it.id.0 < BLANK).count() as u64;
            let mut fresh = given.iter().copied().fold(self.high.max(lines), u64::max) + 1;
            let mut place = 0;
            for it in out.iter_mut().filter(|it| it.id.0 < BLANK) {
                place += 1;
                if it.id.0 == 0 {
                    it.id = ItemId(if given.contains(&place) {
                        fresh += 1;
                        fresh - 1
                    } else {
                        place
                    });
                }
            }
        }
        out
    }

    /// The part file.
    pub fn print(&self) -> String {
        let mut out = String::new();
        if let Some(n) = self.next_line() {
            out.push_str(&format!("next {n}\n"));
        }
        for k in 0..self.items.len() {
            self.print_item(&mut out, k, true);
        }
        out
    }

    fn print_item(&self, out: &mut String, k: usize, ids: bool) {
        let it = &self.items[k];
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
        if let Some(n) = self.written_id(k).filter(|_| ids) {
            out.push_str(&format!("id {n} "));
        }
        out.push_str(&self.code(k).join(" "));
        note(out, &it.note, !matches!(it.body, Body::Blank));
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
                note(out, &r.note, !r.args.is_empty());
            }
            out.push_str("\nend");
        }
        out.push('\n');
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

    /// The id the next new line takes, when the file must say it: some id above every line's
    /// was used by a line taken out, and must never name another.
    fn next_line(&self) -> Option<u64> {
        let top = self
            .items
            .iter()
            .map(|it| it.id.0)
            .filter(|&i| i < BLANK)
            .max()
            .unwrap_or(0);
        (self.high > top).then_some(self.high + 1)
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
        let mut no = 1 + self.next_line().is_some() as usize;
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
        let mut first = 1 + self.next_line().is_some() as usize;
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
    /// making a new body: its `new` is written. The op that undoes that, if it did.
    fn keep_new(&mut self, was: Option<(ItemId, bool)>) -> Option<Op> {
        let (id, false) = was? else { return None };
        if self.first_solid().is_some_and(|f| f.0 == id) {
            return None;
        }
        let it = self.items.iter_mut().find(|it| it.id == id)?;
        let before = it.clone();
        if let Body::Feature { args, .. } = &mut it.body {
            args.insert(0, Arg::new("new"));
        }
        Some(Op::SetItem { item: before })
    }

    /// The id of the item after position `k`, the place to come back to.
    fn after(&self, k: usize) -> Option<ItemId> {
        self.items.get(k).map(|it| it.id)
    }

    /// The position of `before`, or the end.
    fn place(&self, before: Option<ItemId>) -> Result<usize, String> {
        before.map_or(Ok(self.items.len()), |id| self.index(id))
    }

    /// New items from text, with ids of their own: the items of an `Op::Insert`.
    pub fn read_items(&mut self, text: &str) -> Vec<Item> {
        self.read(text, false)
    }

    /// The parameter of that name.
    pub fn param(&self, name: &str) -> Option<ItemId> {
        self.items.iter().find_map(|it| match &it.body {
            Body::Param { name: n, .. } if n == name => Some(it.id),
            _ => None,
        })
    }

    /// The item a feature reference names: `#ID`, a label, `fN`, or kind and count (`hole2`).
    pub fn find(&self, name: &str) -> Option<ItemId> {
        if let Some(id) = name.strip_prefix('#').and_then(|n| n.parse().ok()) {
            return self.items.iter().find(|it| it.id == ItemId(id)).map(|it| it.id);
        }
        let lines = || self.items.iter().filter(|it| !matches!(it.body, Body::Blank));
        if let Some(it) = lines().find(|it| it.label.as_deref() == Some(name)) {
            return Some(it.id);
        }
        if let Some(id) = self.param(name) {
            return Some(id);
        }
        let kind = name.trim_end_matches(|c: char| c.is_ascii_digit());
        let n: usize = name[kind.len()..].parse().ok()?;
        let n = n.checked_sub(1)?;
        if kind == "f" {
            return lines().nth(n).map(|it| it.id);
        }
        lines()
            .filter(|it| match &it.body {
                Body::Feature { kind: k, .. } => k == kind,
                Body::Sketch { .. } => kind == "sketch",
                Body::Param { .. } => kind == "param",
                Body::Blank => false,
            })
            .nth(n)
            .map(|it| it.id)
    }

    /// The name an item goes by here: its label, else kind and count.
    pub fn name(&self, id: ItemId) -> Option<String> {
        let it = self.items.iter().find(|it| it.id == id)?;
        if let Body::Param { name, .. } = &it.body {
            return Some(name.clone());
        }
        match &it.label {
            Some(l) => Some(l.clone()),
            None => self.name_of(id, ""),
        }
    }

    /// An item as it prints, label and sketch rows and all, its id left off.
    pub fn text_of(&self, id: ItemId) -> Option<String> {
        let k = self.index(id).ok()?;
        let mut one = String::new();
        self.print_item(&mut one, k, false);
        Some(one.trim_end().to_string())
    }

    /// Which features each selector in the file names by their place (`of hole1`): the places
    /// change when lines go in, out or move, and these must follow.
    fn selector_refs(&self) -> Vec<(ItemId, usize, String, ItemId)> {
        let mut out = Vec::new();
        for it in &self.items {
            for (k, a) in it.args().iter().enumerate() {
                if a.text.starts_with('(') {
                    for w in selector_words(&a.text) {
                        if let Some(to) = self.find(w) {
                            out.push((it.id, k, w.to_string(), to));
                        }
                    }
                }
            }
        }
        out
    }

    /// The feature lines whose selectors name this item.
    pub fn named_by(&self, id: ItemId) -> Option<ItemId> {
        let mut refs = self.selector_refs().into_iter();
        refs.find(|r| r.3 == id && r.0 != id).map(|r| r.0)
    }

    /// Every item that names this one, by a reference or in a selector.
    pub fn users(&self, id: ItemId) -> Vec<ItemId> {
        let mut out: Vec<ItemId> = self
            .items
            .iter()
            .filter(|it| it.id != id && it.args().iter().any(|a| a.to == Some(id)))
            .map(|it| it.id)
            .collect();
        for r in self.selector_refs() {
            if r.3 == id && r.0 != id && !out.contains(&r.0) {
                out.push(r.0);
            }
        }
        out
    }

    /// Selectors that named a feature by its place name it by its new place.
    fn rename_selectors(&mut self, was: Vec<(ItemId, usize, String, ItemId)>) {
        for (item, k, word, to) in was {
            let Some(new) = self.name_of(to, &word).filter(|n| *n != word) else {
                continue;
            };
            let Some(it) = self.items.iter_mut().find(|it| it.id == item) else {
                continue;
            };
            let args = match &mut it.body {
                Body::Sketch { head, .. } => head,
                Body::Feature { args, .. } => args,
                _ => continue,
            };
            let text = &mut args[k].text;
            *text = rename_word(text, &word, &new);
        }
    }

    /// Applies an op. What comes back undoes it, applied in order (and what undoing gives back,
    /// last first, redoes it).
    pub fn apply(&mut self, op: Op) -> Result<Vec<Op>, String> {
        // lines going in, out or moving change places; a value never does
        let places = matches!(op, Op::Insert { .. } | Op::Delete { .. } | Op::Move { .. });
        let was = if places { self.selector_refs() } else { Vec::new() };
        let undo = self.apply_op(op)?;
        self.rename_selectors(was);
        Ok(undo)
    }

    fn apply_op(&mut self, op: Op) -> Result<Vec<Op>, String> {
        let mut undo = Vec::new();
        match op {
            Op::SetParam { item, expr } => {
                let k = self.index(item)?;
                let Body::Param { expr: e, .. } = &mut self.items[k].body else {
                    return Err(format!("item {} is not a parameter", item.0));
                };
                undo.push(Op::SetParam {
                    item,
                    expr: std::mem::replace(e, expr),
                });
            }
            Op::SetDimension { item, row, value } => {
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
                let last = r.args.last_mut().expect("a dimension has a value");
                undo.push(Op::SetDimension {
                    item,
                    row,
                    value: std::mem::replace(last, value),
                });
            }
            Op::SetArg { item, arg, text } => {
                let k = self.index(item)?;
                let before = self.items[k].clone();
                let args = match &mut self.items[k].body {
                    Body::Sketch { head, .. } => head,
                    Body::Feature { args, .. } => args,
                    _ => return Err(format!("item {} has no arguments", item.0)),
                };
                *args.get_mut(arg).ok_or_else(|| format!("no argument {arg}"))? = Arg::new(text);
                undo.push(Op::SetItem { item: before });
            }
            Op::SetItem { item } => {
                let k = self.index(item.id)?;
                let first = self.first_solid();
                self.high = self.high.max(item.id.0 * (item.id.0 < BLANK) as u64);
                undo.push(Op::SetItem {
                    item: std::mem::replace(&mut self.items[k], item),
                });
                undo.extend(self.keep_new(first));
            }
            Op::Insert { before, items } => {
                let first = self.first_solid();
                let k = self.place(before)?;
                for it in &items {
                    if it.id.0 < BLANK {
                        self.high = self.high.max(it.id.0);
                    } else {
                        self.next = self.next.max(it.id.0 + 1);
                    }
                }
                undo.extend(items.iter().map(|it| Op::Delete { item: it.id }));
                self.items.splice(k..k, items);
                undo.extend(self.keep_new(first));
            }
            Op::Delete { item } => {
                let k = self.index(item)?;
                let it = self.items.remove(k);
                undo.push(Op::Insert {
                    before: self.after(k),
                    items: vec![it],
                });
            }
            Op::Move { item, before } => {
                let first = self.first_solid();
                let k = self.index(item)?;
                self.place(before)?;
                let it = self.items.remove(k);
                undo.push(Op::Move {
                    item,
                    before: self.after(k),
                });
                let to = if before == Some(item) {
                    k
                } else {
                    self.place(before)?
                };
                self.items.insert(to, it);
                undo.extend(self.keep_new(first));
            }
            Op::Text { src } => {
                undo.push(Op::Text { src: self.print() });
                let mut d = Document::parse(&src);
                d.next = d.next.max(self.next);
                d.high = d.high.max(self.high);
                *self = d;
            }
        }
        Ok(undo)
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

    /// The op's undo gives back the document, and undoing that gives the op's result.
    fn undone(d: &mut Document, op: Op) {
        // ids handed out stay taken, so the `next` line may stay
        let text = |d: &Document| {
            let p = d.print();
            p.strip_prefix("next ")
                .and_then(|r| r.split_once('\n'))
                .map_or(p.clone(), |r| r.1.to_string())
        };
        let before = text(d);
        let undo = d.apply(op).expect("applies");
        let after = text(d);
        let mut redo = Vec::new();
        for u in undo {
            redo.splice(0..0, d.apply(u).expect("undoes"));
        }
        assert_eq!(text(d), before);
        redo.into_iter().for_each(|r| drop(d.apply(r).expect("redoes")));
        assert_eq!(text(d), after);
    }

    #[test]
    fn edits() {
        let mut d = Document::parse(SRC);
        let id = |d: &Document, k: usize| d.items[k].id;
        let w = d.param("w").expect("w");
        d.apply(Op::SetParam {
            item: w,
            expr: "50".into(),
        })
        .expect("w");
        assert!(d.print().contains("\nw = 50  # mm\n"));
        assert!(d.param("q").is_none());
        assert!(
            d.apply(Op::SetParam {
                item: id(&d, 4),
                expr: "1".into()
            })
            .is_err()
        );
        d.apply(Op::SetDimension {
            item: id(&d, 4),
            row: 2,
            value: "w/2".into(),
        })
        .expect("dist");
        assert!(d.print().contains("  dist l1 w/2\n"));
        assert!(
            d.apply(Op::SetDimension {
                item: id(&d, 4),
                row: 0,
                value: "1".into()
            })
            .is_err()
        );
        // the hole names the sketch: it follows when a line goes in above
        d.link(4, &[(3, 2)]);
        let items = d.read_items("sketch\n  rect 0 0 1 1\nend");
        d.apply(Op::Insert {
            before: Some(id(&d, 4)),
            items,
        })
        .expect("insert");
        assert!(
            d.print().ends_with("hole 3 on sketch2 l1.a thru\n"),
            "{}",
            d.print()
        );
        let hole = id(&d, 7);
        d.apply(Op::Move {
            item: hole,
            before: Some(id(&d, 5)),
        })
        .expect("move");
        assert!(
            d.print()
                .contains("id 5 hole 3 on sketch2 l1.a thru\nsk: id 3 sketch"),
            "{}",
            d.print()
        );
        d.apply(Op::SetArg {
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
        d.apply(Op::Delete { item: hole }).expect("delete");
        assert!(!d.print().contains("hole"));
        // the hole's id is not handed out again
        let items = d.read_items("ball 1");
        d.apply(Op::Insert { before: None, items }).expect("insert");
        assert!(d.print().ends_with("id 7 ball 1\n"), "{}", d.print());
    }

    #[test]
    fn every_op_undoes() {
        let mut d = Document::parse(SRC);
        let id = |d: &Document, k: usize| d.items[k].id;
        let w = d.param("w").expect("w");
        let [a, b, c, e, f] = [1, 2, 4, 5, 6].map(|k| id(&d, k));
        undone(
            &mut d,
            Op::SetParam {
                item: w,
                expr: "7".into(),
            },
        );
        undone(
            &mut d,
            Op::SetDimension {
                item: c,
                row: 2,
                value: "9".into(),
            },
        );
        undone(
            &mut d,
            Op::SetArg {
                item: e,
                arg: 0,
                text: "9".into(),
            },
        );
        let mut item = d.items[5].clone();
        item.label = Some("b".into());
        undone(&mut d, Op::SetItem { item });
        let items = d.read_items("h = 3\nball 2");
        undone(
            &mut d,
            Op::Insert {
                before: Some(b),
                items,
            },
        );
        undone(&mut d, Op::Delete { item: e });
        undone(
            &mut d,
            Op::Move {
                item: f,
                before: Some(a),
            },
        );
        undone(
            &mut d,
            Op::Move {
                item: a,
                before: None,
            },
        );
        undone(
            &mut d,
            Op::Text {
                src: "ball 1\n".into(),
            },
        );
        // refused moves leave it as it was
        let before = d.clone();
        assert!(
            d.apply(Op::Move {
                item: a,
                before: Some(ItemId(99))
            })
            .is_err()
        );
        assert_eq!(d, before);
    }

    /// A selector names a feature by its place: when the place changes the selector follows, so
    /// it never finds another feature that came to stand there.
    #[test]
    fn selectors_follow_their_features() {
        let mut d =
            Document::parse("extrude 9 rect 0 0 9 9\nhole 3 at 1 1 9 thru\nfillet 1 (edges of hole1 top)\n");
        let [first, hole, fillet] = [0, 1, 2].map(|k| d.items[k].id);
        let items = d.read_items("hole 2 at 5 5 9 thru");
        let undo = d
            .apply(Op::Insert {
                before: Some(hole),
                items,
            })
            .expect("insert");
        assert!(d.print().contains("(edges of hole2 top)"), "{}", d.print());
        undo.into_iter().for_each(|u| drop(d.apply(u).expect("undoes")));
        assert!(d.print().contains("(edges of hole1 top)"), "{}", d.print());
        assert_eq!(d.named_by(hole), Some(fillet));
        assert_eq!(d.named_by(first), None);
        assert_eq!(
            rename_word("(a hole1 hole11)", "hole1", "hole2"),
            "(a hole2 hole11)"
        );
    }

    #[test]
    fn names_find_their_items() {
        let d = Document::parse(SRC);
        let sk = d.find("sk").expect("label");
        assert_eq!(d.find("sketch1"), Some(sk));
        assert_eq!(d.find("f3"), Some(sk));
        assert_eq!(d.find(&format!("#{}", sk.0)), Some(sk));
        assert_eq!(d.find("hole1"), Some(d.items[6].id));
        assert_eq!(d.find("hole2"), None);
        assert_eq!(d.name(sk).as_deref(), Some("sk"));
        assert_eq!(d.name(d.items[6].id).as_deref(), Some("hole1"));
        assert_eq!(
            d.text_of(sk).expect("sk").lines().next(),
            Some("sk: sketch on plane1 at 0 0 6")
        );
    }

    /// A solid moved above the first leaves the first making a new body, its `new` written.
    #[test]
    fn moves_keep_the_bodies() {
        let mut d = Document::parse("extrude 1 rect 0 0 1 1\nball new 2 at 5 0 0\n");
        let ball = d.items[1].id;
        let first = d.items[0].id;
        d.apply(Op::Move {
            item: ball,
            before: Some(first),
        })
        .expect("move");
        assert_eq!(
            d.print(),
            "id 2 ball new 2 at 5 0 0\nid 1 extrude new 1 rect 0 0 1 1\n"
        );
        // written ops are left as they are
        d.apply(Op::Move {
            item: ball,
            before: None,
        })
        .expect("move back");
        assert_eq!(d.print(), "extrude new 1 rect 0 0 1 1\nball new 2 at 5 0 0\n");
    }
}
