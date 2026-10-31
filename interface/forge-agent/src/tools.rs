//! The one command layer: every change to a part, from the UI or an agent, is a `Call`. A call
//! makes document ops by stable id; a commit applies them all or none, appends them to the part's
//! op log and writes the `.fc` file. The UI builds calls with typed arguments. An agent sends the
//! same call as a line of text (`line`), and every commit prints back as such lines (`describe`).

use crate::json::Json;
use crate::model::Model;
use crate::select::Ent;
use crate::short;
use forge_doc::wire::{self, Entry, Kind};
use forge_doc::{Body, Document, Item, ItemId, Op};
use forge_topo::{Name, check};
use std::path::{Path, PathBuf};

/// A change or a question, with its arguments typed: nothing here is parsed.
#[derive(Clone, Debug, PartialEq)]
pub enum Call {
    SetParam {
        item: ItemId,
        expr: String,
    },
    /// `row` among the sketch's rows: a dimension's value.
    SetDim {
        item: ItemId,
        row: usize,
        value: String,
    },
    /// One argument of a feature, or of a sketch's header.
    SetArg {
        item: ItemId,
        arg: usize,
        text: String,
    },
    /// New items (parameters, sketches, features) with ids from `Document::read_items`.
    Add {
        before: Option<ItemId>,
        items: Vec<Item>,
    },
    /// A feature's whole line replaced: `item.id` says which, and keeps its names.
    Edit {
        item: Item,
    },
    Delete {
        item: ItemId,
    },
    Move {
        item: ItemId,
        before: Option<ItemId>,
    },
    Undo,
    Redo,
    /// What later reads of the selection mean: handles or selectors.
    Select {
        refs: Vec<String>,
    },
}

/// The text form of each call, one line each: the registry `help` and the docs read.
pub struct Tool {
    pub name: &'static str,
    pub syntax: &'static str,
    pub what: &'static str,
}

pub const TOOLS: [Tool; 10] = [
    Tool {
        name: "add",
        syntax: "LINE",
        what: "any .fc line goes on the end; a sketch runs to `end`",
    },
    Tool {
        name: "param",
        syntax: "NAME = EXPR [mm|deg]",
        what: "sets the parameter, or adds it on the end",
    },
    Tool {
        name: "before",
        syntax: "before FEAT LINE",
        what: "a line put in above FEAT",
    },
    Tool {
        name: "edit",
        syntax: "edit FEAT LINE",
        what: "replaces FEAT's line; its id and label stay, so references hold",
    },
    Tool {
        name: "set",
        syntax: "set FEAT K TEXT",
        what: "FEAT's Kth word after the kind",
    },
    Tool {
        name: "dim",
        syntax: "dim SKETCH ROW VALUE",
        what: "a dimension: `dim plate dist l1 50` (the row without its value, or its number)",
    },
    Tool {
        name: "rm",
        syntax: "rm FEAT",
        what: "takes the line out; refused while another line names it",
    },
    Tool {
        name: "mv",
        syntax: "mv FEAT FEAT2|end",
        what: "moves FEAT above FEAT2; ids and names stay",
    },
    Tool {
        name: "undo",
        syntax: "undo | redo",
        what: "alone in its act",
    },
    Tool {
        name: "select",
        syntax: "select REF..",
        what: "alone in its act; `measure F sel` reads it",
    },
];

/// A number a drag can change, and where it lives.
#[derive(Clone, Debug, PartialEq)]
pub struct Dim {
    pub name: String,
    pub value: f64,
    pub at: Target,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Param(ItemId),
    Arg(ItemId, usize),
    Row(ItemId, usize),
}

impl Target {
    /// The op that gives it this value.
    pub fn op(self, value: String) -> Op {
        match self {
            Target::Param(item) => Op::SetParam { item, expr: value },
            Target::Arg(item, arg) => Op::SetArg {
                item,
                arg,
                text: value,
            },
            Target::Row(item, row) => Op::SetDimension { item, row, value },
        }
    }
}

/// What a part is, for saying what a commit did.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stat {
    pub faces: usize,
    pub volume: f64,
    pub bodies: usize,
    pub valid: bool,
}

/// What `act` and `call` answer.
#[derive(Clone, Debug)]
pub struct Done {
    pub ok: bool,
    /// One line when it worked; the line, the reason and a replacement that builds when not.
    pub text: String,
    /// The commit as `.fc` lines.
    pub lines: Vec<String>,
    pub version: u64,
}

impl Done {
    pub fn json(&self) -> Json {
        Json::obj(vec![
            ("ok", self.ok.into()),
            ("text", self.text.clone().into()),
            ("version", (self.version as usize).into()),
            (
                "lines",
                Json::Arr(self.lines.iter().map(|l| l.clone().into()).collect()),
            ),
        ])
    }
}

/// A model, its op log and the file they are kept in.
#[derive(Default)]
pub struct Part {
    pub model: Model,
    log: Vec<Entry>,
    /// Entries of `log` the next undo and redo take back, newest last.
    undos: Vec<usize>,
    redos: Vec<usize>,
    /// The part file; none: the log stays in memory and nothing is written.
    path: Option<PathBuf>,
    opened: bool,
    who: u64,
    selection: Vec<Name>,
    /// A value being dragged: what undoes the drag's first op.
    live: Option<Vec<Op>>,
    /// The numbers of the text with this hash.
    known: Option<(u64, Stat)>,
}

/// A commit being made.
#[derive(Default)]
struct Tx {
    ops: Vec<Op>,
    undo: Vec<Op>,
    /// The items touched, with the input line that did.
    touched: Vec<(ItemId, usize)>,
}

/// Input text cut into calls' lines: a sketch runs to its `end`. Each with its first line's number.
fn blocks(text: &str) -> Vec<(usize, String)> {
    let mut out: Vec<(usize, String)> = Vec::new();
    let mut open = false;
    for (k, raw) in text.lines().enumerate() {
        let code = crate::text::uncommented(raw).trim();
        if open {
            let b = out.last_mut().expect("a block is open");
            b.1.push('\n');
            b.1.push_str(raw);
            open = code != "end";
            continue;
        }
        if code.is_empty() && !raw.trim_start().starts_with('#') {
            continue;
        }
        let toks = crate::text::tokens(code);
        let mut t: &[String] = &toks;
        if t.first().is_some_and(|w| w == "before" || w == "edit") && t.len() > 2 {
            t = &t[2..];
        }
        if t.first().is_some_and(|w| w.ends_with(':') && w.len() > 1) {
            t = &t[1..];
        }
        open = t.first().is_some_and(|w| w == "sketch");
        out.push((k + 1, raw.to_string()));
    }
    out
}

/// The item a reference names, or why not, with what there is.
fn feat(doc: &Document, r: &str) -> Result<ItemId, String> {
    doc.find(r).ok_or_else(|| {
        let mut names: Vec<String> = doc.features().filter_map(|it| doc.name(it.id)).collect();
        names.retain(|n| close(n, r) <= 2 || names_share_kind(n, r));
        match names.len() {
            0 => format!("no feature '{r}'"),
            _ => format!("no feature '{r}'; there are {}", names.join(" ")),
        }
    })
}

fn names_share_kind(a: &str, b: &str) -> bool {
    let kind = |s: &str| s.trim_end_matches(|c: char| c.is_ascii_digit()).len();
    kind(a) > 0 && kind(a) == kind(b) && a[..kind(a)] == b[..kind(b)]
}

/// Edit distance.
fn close(a: &str, b: &str) -> usize {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, x) in a.iter().enumerate() {
        let mut diag = row[0];
        row[0] = i + 1;
        for (j, y) in b.iter().enumerate() {
            let cost = diag + (x != y) as usize;
            diag = row[j + 1];
            row[j + 1] = cost.min(row[j] + 1).min(diag + 1);
        }
    }
    row[b.len()]
}

/// A commit's op as a line of text, as this document names things.
fn describe(doc: &Document, op: &Op) -> String {
    let name = |id: ItemId| doc.name(id).unwrap_or(format!("#{}", id.0));
    match op {
        Op::SetParam { item, expr } => format!("{} = {expr}", name(*item)),
        Op::SetDimension { item, row, value } => {
            let words = doc
                .items
                .iter()
                .find(|it| it.id == *item)
                .and_then(|it| match &it.body {
                    Body::Sketch { rows, .. } => rows.get(*row),
                    _ => None,
                })
                .map_or(String::new(), |r| r.args[..r.args.len() - 1].join(" "));
            format!("dim {} {words} {value}", name(*item))
        }
        Op::SetArg { item, arg, text } => format!("set {} {} {text}", name(*item), arg + 1),
        Op::SetItem { item } => format!(
            "edit {} {}",
            name(item.id),
            doc.text_of(item.id).unwrap_or_default()
        ),
        Op::Insert { before, items } => {
            let lines: Vec<String> = items.iter().filter_map(|it| doc.text_of(it.id)).collect();
            match before {
                Some(b) => format!("before {} {}", name(*b), lines.join("\n")),
                None => lines.join("\n"),
            }
        }
        Op::Delete { item } => format!("rm {}", name(*item)),
        Op::Move { item, before } => format!("mv {} {}", name(*item), before.map_or("end".to_string(), name)),
        Op::Text { .. } => "# the file was edited by hand".to_string(),
    }
}

impl Part {
    /// A part in memory: no file, no log on disk.
    pub fn new(src: &str) -> Part {
        Part {
            model: Model::new(src),
            ..Part::default()
        }
    }

    /// A part in memory on a model already made.
    pub fn of(model: Model) -> Part {
        Part {
            model,
            ..Part::default()
        }
    }

    /// The version a client is looking at: entries committed. Zero until the first commit.
    pub fn version(&self) -> u64 {
        self.log.len() as u64
    }

    /// Commits write to this file and to its log beside it, which is read now if it's there.
    pub fn open(&mut self, path: &Path) {
        if self.opened && self.path.as_deref() == Some(path) {
            return;
        }
        self.opened = true;
        self.path = Some(path.to_path_buf());
        let log = log_path(path);
        let Some(bytes) = read_plain(&log) else { return };
        let Some((entries, _)) = wire::read_log(&bytes) else {
            return;
        };
        self.log = entries;
        (self.undos, self.redos) = (Vec::new(), Vec::new());
        for (i, e) in self.log.iter().enumerate() {
            match e.kind {
                Kind::Do => {
                    self.undos.push(i);
                    self.redos.clear();
                }
                Kind::Undo => {
                    self.undos.pop();
                    self.redos.push(i);
                }
                Kind::Redo => {
                    self.redos.pop();
                    self.undos.push(i);
                }
            }
        }
    }

    /// The log is whole and ends on the text there is: if not (a first commit, or a hand edit),
    /// the text goes in first, and undo stops there.
    fn sync_log(&mut self) -> Result<(), String> {
        let src = self.model.src().to_string();
        if self
            .log
            .last()
            .is_some_and(|e| e.hash == wire::hash(src.as_bytes()))
        {
            return Ok(());
        }
        let s = self.stat();
        let e = Entry {
            kind: Kind::Do,
            who: self.who,
            hash: wire::hash(src.as_bytes()),
            ops: vec![Op::Text { src }],
            undo: Vec::new(),
            faces: s.faces as u32,
            volume: s.volume,
        };
        self.append(e)?;
        self.undos.clear();
        self.redos.clear();
        Ok(())
    }

    /// Onto the log, on disk first where there is a disk (the part file checked before). Where
    /// on disk it starts.
    fn append(&mut self, e: Entry) -> Result<Option<u64>, String> {
        let mut at = None;
        if let Some(p) = &self.path {
            let part = crate::cmd::writable_file(p)?;
            at =
                Some(append_entry(&log_path(p), mode_of(&part), &e).map_err(|w| format!("the op log: {w}"))?);
        }
        self.log.push(e);
        Ok(at)
    }

    /// The part's numbers now.
    pub fn stat(&mut self) -> Stat {
        let hash = wire::hash(self.model.src().as_bytes());
        if let Some((h, s)) = self.known
            && h == hash
        {
            return s;
        }
        let s = self.measure_stat();
        self.known = Some((hash, s));
        s
    }

    fn measure_stat(&mut self) -> Stat {
        let m = &mut self.model;
        let volume = m.mass().map(|p| p.volume).unwrap_or(f64::NAN);
        let n = m.built();
        let (faces, valid, bodies);
        match m.body() {
            Some(b) => {
                faces = b.face_ids().count();
                let solids = m.solids_after(n);
                bodies = solids.len();
                valid = solids.iter().all(|s| check(&s.body).is_empty());
            }
            None => (faces, valid, bodies) = (0, true, 0),
        }
        let volume = if volume.is_nan() { m.stats().0 } else { volume };
        Stat {
            faces,
            volume,
            bodies,
            valid,
        }
    }

    /// `what changed since version n`, one line.
    pub fn since(&self, n: u64) -> String {
        let doc = &self.model.doc;
        let now = self.version();
        if n >= now {
            return format!("v{now}: nothing since");
        }
        let was = n
            .checked_sub(1)
            .and_then(|k| self.log.get(k as usize))
            .map(|e| (e.faces as i64, e.volume));
        let tail = &self.log[n as usize..];
        let lines: Vec<String> = tail
            .iter()
            .flat_map(|e| &e.ops)
            .map(|op| describe(doc, op))
            .collect();
        let mut out = format!("v{n} -> v{now}: {}", lines.join(" | "));
        if let (Some(was), Some(last)) = (was, tail.last()) {
            out.push_str(&format!(
                "; {}",
                change(was, (last.faces as i64, last.volume), None)
            ));
        }
        out
    }

    /// Commits text lines, each one call, atomically: all or none, one reply. `base`: refused
    /// if the part has moved on from that version.
    pub fn act(&mut self, text: &str, base: Option<u64>) -> Done {
        if let Some(d) = self.ready(base) {
            return d;
        }
        let input = blocks(text);
        if input.is_empty() {
            return self.refused("error: nothing to do".into());
        }
        let first = crate::text::tokens(crate::text::uncommented(&input[0].1));
        if let ["undo" | "redo" | "select", ..] = first.iter().map(String::as_str).collect::<Vec<_>>()[..] {
            if input.len() > 1 {
                return self.refused(format!("error line 1: {} goes alone in its act", first[0]));
            }
            let call = match first[0].as_str() {
                "undo" => Call::Undo,
                "redo" => Call::Redo,
                _ => Call::Select {
                    refs: first[1..].to_vec(),
                },
            };
            return self.call(vec![call], None);
        }
        let mut tx = Tx::default();
        let saved = self.model.save();
        let was = self.stat();
        let had = self.model.error.as_ref().map(|f| f.at);
        for (no, block) in &input {
            let done = line_call(&mut self.model.doc, block).and_then(|call| {
                let k = tx.touched.len();
                push(&mut self.model.doc, call, &mut tx)?;
                tx.touched[k..].iter_mut().for_each(|t| t.1 = *no);
                Ok(())
            });
            if let Err(why) = done {
                self.model.restore(saved);
                let head = block.lines().next().unwrap_or("");
                return self.refused(format!("error line {no}: {head}\n{why}\nnothing applied"));
            }
        }
        self.finish(tx, saved, was, had, Kind::Do)
    }

    /// Commits typed calls, atomically. The UI's way in.
    pub fn call(&mut self, calls: Vec<Call>, base: Option<u64>) -> Done {
        if let Some(d) = self.ready(base) {
            return d;
        }
        match &calls[..] {
            [Call::Undo] => return self.undo_redo(true),
            [Call::Redo] => return self.undo_redo(false),
            [Call::Select { refs }] => return self.select(refs),
            _ => {}
        }
        let mut tx = Tx::default();
        let saved = self.model.save();
        let was = self.stat();
        let had = self.model.error.as_ref().map(|f| f.at);
        for (no, call) in calls.into_iter().enumerate() {
            let k = tx.touched.len();
            if let Err(why) = push(&mut self.model.doc, call, &mut tx) {
                self.model.restore(saved);
                return self.refused(format!("error call {}: {why}\nnothing applied", no + 1));
            }
            tx.touched[k..].iter_mut().for_each(|t| t.1 = no + 1);
        }
        self.finish(tx, saved, was, had, Kind::Do)
    }

    fn refused(&self, text: String) -> Done {
        Done {
            ok: false,
            text,
            lines: Vec::new(),
            version: self.version(),
        }
    }

    /// A log that ends on the text there is, and the base not left behind; or why not.
    fn ready(&mut self, base: Option<u64>) -> Option<Done> {
        if let Err(why) = self.sync_log() {
            return Some(self.refused(format!("error: {why}; nothing applied")));
        }
        let b = base?;
        (b != self.version()).then(|| self.refused(format!("moved: {}; nothing applied", self.since(b))))
    }

    fn select(&mut self, refs: &[String]) -> Done {
        let mut names = Vec::new();
        for r in refs {
            match self.model.select_ref(r) {
                Ok(n) => names.extend(n),
                Err(e) => return self.refused(format!("error: {r}: {e}")),
            }
        }
        let text = format!("ok; {} selected", names.len());
        self.selection = names;
        Done {
            ok: true,
            text,
            lines: Vec::new(),
            version: self.version(),
        }
    }

    /// What is selected, as entities of the finished part.
    pub fn selected(&self) -> Vec<Ent> {
        let Some(b) = self.model.body() else {
            return Vec::new();
        };
        let all = |n: Name| {
            b.face_by_name(n)
                .map(Ent::Face)
                .or_else(|| b.edge_by_name(n).map(Ent::Edge))
                .or_else(|| {
                    (0..b.vertices.len() as u32)
                        .map(forge_topo::VertexId)
                        .find(|&v| b.vertex(v).name == n)
                        .map(Ent::Vert)
                })
        };
        self.selection.iter().filter_map(|&n| all(n)).collect()
    }

    fn undo_redo(&mut self, undo: bool) -> Done {
        let word = if undo { "undo" } else { "redo" };
        let stack = if undo { &self.undos } else { &self.redos };
        let Some(&i) = stack.last() else {
            return self.refused(format!("error: nothing to {word}"));
        };
        let ops = self.log[i].undo.clone();
        if ops.is_empty() {
            return self.refused(format!(
                "error: nothing to {word}: the file was edited by hand there"
            ));
        }
        let saved = self.model.save();
        let was = self.stat();
        let had = self.model.error.as_ref().map(|f| f.at);
        let mut tx = Tx::default();
        for op in ops {
            match self.model.doc.apply(op.clone()) {
                Ok(u) => {
                    tx.ops.push(op);
                    // undone in the reverse of the order they were made
                    tx.undo.splice(0..0, u);
                }
                Err(e) => {
                    self.model.restore(saved);
                    return self.refused(format!("error: {word}: {e}"));
                }
            }
        }
        self.finish(tx, saved, was, had, if undo { Kind::Undo } else { Kind::Redo })
    }

    /// Builds what the document now says and commits it, or goes back and says why.
    fn finish(
        &mut self,
        mut tx: Tx,
        saved: crate::model::Saved,
        was: Stat,
        had: Option<usize>,
        kind: Kind,
    ) -> Done {
        self.model.sync_doc();
        // a part that didn't build may be mended a line at a time
        let failed = self.model.error.clone().filter(|f| had.is_none_or(|h| f.at <= h));
        if let Some(f) = failed {
            let text = self.explain_failure(&f, &tx);
            self.model.restore(saved);
            return self.refused(text);
        }
        if let Some(text) = self.empty_pick(&tx) {
            self.model.restore(saved);
            return self.refused(text);
        }
        let picks = self.pick_words(&tx);
        // a varying fillet's start, once found, goes in with the commit
        let before: Vec<Item> = self.model.doc.items.clone();
        if !self.model.fill_law_starts().is_empty() {
            for (old, new) in before.iter().zip(&self.model.doc.items) {
                if old != new {
                    tx.ops.push(Op::SetItem { item: new.clone() });
                    tx.undo.insert(0, Op::SetItem { item: old.clone() });
                }
            }
        }
        let now = self.stat();
        if !now.valid && was.valid {
            let text = self.invalid_text();
            self.model.restore(saved);
            return self.refused(text);
        }
        let src = self.model.src().to_string();
        if let Err(why) = self.commit(tx.ops.clone(), tx.undo, kind, now, &src) {
            self.model.restore(saved);
            return self.refused(format!("error: {why}; nothing applied"));
        }
        let mut text = format!(
            "ok v{}; {}",
            self.version(),
            change(
                (was.faces as i64, was.volume),
                (now.faces as i64, now.volume),
                Some(now.valid)
            )
        );
        if now.bodies != was.bodies && now.bodies.max(was.bodies) > 1 {
            text.push_str(&format!(", {} bodies", now.bodies));
        }
        if !picks.is_empty() {
            text.push_str("; picked ");
            text.push_str(&picks);
        }
        if let Some(f) = &self.model.error {
            let no = self.model.lines[f.at].no;
            text.push_str(&format!("; line {no} still fails: {}", f.msg));
        }
        let doc = &self.model.doc;
        Done {
            ok: true,
            text,
            lines: tx.ops.iter().map(|op| describe(doc, op)).collect(),
            version: self.version(),
        }
    }

    fn commit(
        &mut self,
        ops: Vec<Op>,
        undo: Vec<Op>,
        kind: Kind,
        now: Stat,
        src: &str,
    ) -> Result<(), String> {
        let hash = wire::hash(src.as_bytes());
        let entry = Entry {
            kind,
            who: self.who,
            ops,
            undo,
            hash,
            faces: now.faces as u32,
            volume: now.volume,
        };
        let at = self.append(entry)?;
        if let Some(p) = &self.path
            && let Err(why) = crate::cmd::replace(p, src)
        {
            // the file stays as it was, so the log must too
            if let Some(at) = at {
                cut_log(&log_path(p), at);
            }
            self.log.pop();
            return Err(why);
        }
        let i = self.log.len() - 1;
        match kind {
            Kind::Do => {
                self.undos.push(i);
                self.redos.clear();
            }
            Kind::Undo => {
                self.undos.pop();
                self.redos.push(i);
            }
            Kind::Redo => {
                self.redos.pop();
                self.undos.push(i);
            }
        }
        Ok(())
    }

    fn explain_failure(&mut self, f: &crate::model::Failure, tx: &Tx) -> String {
        let line = &self.model.lines[f.at];
        let echo = line.toks.split(|t| t == ";").next().unwrap_or_default().join(" ");
        let id = ItemId(line.id);
        let place = match tx.touched.iter().find(|t| t.0 == id) {
            Some((_, no)) => format!("error line {no}: {echo}"),
            None => format!("error at file line {} after this change: {echo}", line.no),
        };
        let fix = self.replacement(f, id);
        let tail = match fix {
            Some(t) => format!("try: {t}"),
            None => format!("hint: {}", f.hint),
        };
        format!("{place}\n{}\n{tail}\nnothing applied", f.msg)
    }

    /// A selector that picked nothing on a line this commit touched.
    fn empty_pick(&self, tx: &Tx) -> Option<String> {
        let m = &self.model;
        let (no, _) = m.picks().into_iter().find(|&(no, n)| {
            n == 0
                && m.lines
                    .iter()
                    .any(|l| l.no == no && tx.touched.iter().any(|t| t.0.0 == l.id))
        })?;
        let line = m.lines.iter().find(|l| l.no == no)?;
        let echo = line.toks.split(|t| t == ";").next().unwrap_or_default().join(" ");
        let input = tx.touched.iter().find(|t| t.0.0 == line.id)?.1;
        Some(format!(
            "error line {input}: {echo}\na selector in it picked nothing; `query F SEL` shows what one finds\nnothing applied"
        ))
    }

    fn invalid_text(&self) -> String {
        let m = &self.model;
        let first = m
            .solids_after(m.built())
            .iter()
            .flat_map(|s| check(&s.body))
            .next()
            .map_or(String::new(), |p| format!("{p:?}"));
        format!("error: the result is not a valid solid: {first}\nnothing applied")
    }

    /// "fillet1:4 chamfer1:2": how many each selector on a touched line picked.
    fn pick_words(&self, tx: &Tx) -> String {
        let m = &self.model;
        let mut out = Vec::new();
        for (no, n) in m.picks() {
            let Some(i) = m.lines.iter().position(|l| l.no == no) else {
                continue;
            };
            if tx.touched.iter().any(|t| t.0.0 == m.lines[i].id) && i < m.labels.len() {
                out.push(format!("{}:{n}", m.feat_name(i)));
            }
        }
        out.join(" ")
    }

    /// A line that does build in place of the one that failed, tried for real, if a small change
    /// makes one: a name misspelt, or a size that fits. The model is left as `finish` restores it.
    fn replacement(&mut self, f: &crate::model::Failure, id: ItemId) -> Option<String> {
        let at = self.model.doc.items.iter().position(|it| it.id == id)?;
        let kind = match &self.model.doc.items[at].body {
            Body::Feature { kind, .. } => kind.clone(),
            _ => String::new(),
        };
        let mut tries: Vec<Item> = Vec::new();
        let with = |item: &Item, arg: Option<usize>, text: &str| -> Item {
            let mut it = item.clone();
            match (&mut it.body, arg) {
                (Body::Feature { kind, .. }, None) => *kind = text.to_string(),
                (Body::Feature { args, .. }, Some(k)) => args[k] = forge_doc::Arg::new(text),
                _ => {}
            }
            it
        };
        let item = self.model.doc.items[at].clone();
        if !kind.is_empty() && crate::text::kind(&kind).is_none() {
            let best = crate::text::KINDS.iter().min_by_key(|k| close(k.name, &kind));
            if let Some(best) = best.filter(|b| close(b.name, &kind) <= 2) {
                tries.push(with(&item, None, best.name));
            }
        }
        // a name that is not there: the nearest that is, among what comes above
        if let Some(bad) = f.msg.split('\'').nth(1)
            && let Body::Feature { args, .. } = &item.body
        {
            let names: Vec<String> = self.model.doc.items[..at]
                .iter()
                .filter_map(|it| self.model.doc.name(it.id))
                .collect();
            if let Some(best) = names
                .iter()
                .min_by_key(|n| close(n, bad))
                .filter(|n| close(n, bad) <= 2)
                && let Some(k) = args.iter().position(|a| a.text.contains(bad))
            {
                tries.push(with(&item, Some(k), &args[k].text.replace(bad, best)));
            }
        }
        // a round or chamfer or wall too big for where it goes
        if matches!(kind.as_str(), "fillet" | "chamfer" | "shell" | "facefillet")
            && let Body::Feature { args, .. } = &item.body
            && let Some((k, size)) = args
                .iter()
                .enumerate()
                .find_map(|(k, a)| Some((k, a.text.parse::<f64>().ok()?)))
        {
            let max = f
                .msg
                .rsplit("radius under ")
                .next()
                .and_then(|m| m.parse::<f64>().ok());
            let first = max.map_or(size * 0.5, |m| m * 0.9);
            for v in [first, first * 0.5, first * 0.25] {
                tries.push(with(&item, Some(k), &round2(v)));
            }
        }
        for t in tries.into_iter().take(4) {
            let text = {
                self.model.doc.items[at] = t;
                self.model.doc.text_of(id)
            };
            self.model.sync_doc();
            let fine = self
                .model
                .error
                .as_ref()
                .is_none_or(|e| self.model.lines[e.at].id != id.0);
            if fine {
                return text;
            }
        }
        None
    }

    /// Every number a drag can change: parameters written as numbers, numeric arguments of
    /// feature lines, numeric sketch dimensions.
    pub fn dims(&self) -> Vec<Dim> {
        let doc = &self.model.doc;
        let mut out = Vec::new();
        for it in doc.features() {
            let name = doc.name(it.id).unwrap_or_default();
            match &it.body {
                Body::Param { expr, .. } => {
                    if let Ok(value) = expr.parse() {
                        out.push(Dim {
                            name,
                            value,
                            at: Target::Param(it.id),
                        });
                    }
                }
                Body::Feature { args, .. } => {
                    for (k, a) in args.iter().enumerate() {
                        if let Ok(value) = a.text.parse() {
                            out.push(Dim {
                                name: format!("{name}.{}", k + 1),
                                value,
                                at: Target::Arg(it.id, k),
                            });
                        }
                    }
                }
                Body::Sketch { rows, .. } => {
                    for (k, r) in rows.iter().enumerate() {
                        let dim = matches!(
                            r.args.first().map(String::as_str),
                            Some("dist" | "hdist" | "vdist" | "rad" | "dia" | "ang")
                        );
                        if let (true, Some(Ok(value))) = (dim, r.args.last().map(|v| v.parse())) {
                            let words = r.args[..r.args.len() - 1].join(" ");
                            out.push(Dim {
                                name: format!("{name}.{words}"),
                                value,
                                at: Target::Row(it.id, k),
                            });
                        }
                    }
                }
                Body::Blank => {}
            }
        }
        out
    }

    /// One step of a drag: the value goes into the document and the part rebuilds from what it
    /// changed. Nothing is logged or written until `drag_commit`.
    pub fn drag(&mut self, op: Op) -> Result<(), String> {
        if self.live.is_none() {
            self.sync_log()?;
        }
        let undo = self.model.edit(op)?;
        self.live.get_or_insert(undo);
        Ok(())
    }

    /// The drag ends on this value, or on `fallback` (the last that built) if it doesn't build:
    /// one commit, one line. If neither does it ends where it started.
    pub fn drag_commit(&mut self, op: Op, fallback: Option<Op>) -> Done {
        let Some(undo) = self.live.take() else {
            return self.refused("error: nothing is being dragged".into());
        };
        let mut chosen = None;
        for o in std::iter::once(op).chain(fallback) {
            if self.model.edit(o.clone()).is_ok() && self.model.error.is_none() {
                chosen = Some(o);
                break;
            }
        }
        let Some(op) = chosen else {
            for u in undo {
                let _ = self.model.doc.apply(u);
            }
            self.model.sync_doc();
            return self.refused("that value does not build; back as it was".into());
        };
        let was = self.log.last().map(|e| (e.faces as i64, e.volume));
        let now = self.stat();
        let src = self.model.src().to_string();
        let ops = vec![op];
        if let Err(why) = self.commit(ops.clone(), undo.clone(), Kind::Do, now, &src) {
            for u in undo {
                let _ = self.model.doc.apply(u);
            }
            self.model.sync_doc();
            return self.refused(format!("error: {why}; back as it was"));
        }
        let at = (now.faces as i64, now.volume);
        Done {
            ok: true,
            text: format!(
                "ok v{}; {}",
                self.version(),
                change(was.unwrap_or(at), at, Some(now.valid))
            ),
            lines: ops.iter().map(|op| describe(&self.model.doc, op)).collect(),
            version: self.version(),
        }
    }

    /// Back to before the drag.
    pub fn drag_cancel(&mut self) {
        if let Some(undo) = self.live.take() {
            for op in undo {
                let _ = self.model.doc.apply(op);
            }
            self.model.sync_doc();
        }
    }
}

/// What a commit did: `+4 faces, volume -120.4 mm3, valid`.
fn change(was: (i64, f64), now: (i64, f64), valid: Option<bool>) -> String {
    let mut parts = Vec::new();
    let df = now.0 - was.0;
    if df != 0 {
        parts.push(format!("{df:+} faces"));
    }
    let dv = now.1 - was.1;
    if dv.abs() > 1e-9 {
        parts.push(format!(
            "volume {}{} mm3",
            if dv > 0.0 { "+" } else { "" },
            short(dv)
        ));
    }
    if parts.is_empty() {
        parts.push("shape unchanged".into());
    }
    match valid {
        Some(true) => parts.push("valid".into()),
        Some(false) => parts.push("INVALID".into()),
        None => {}
    }
    parts.join(", ")
}

/// A size to try: two figures is plenty for a suggestion.
fn round2(v: f64) -> String {
    let mut d = 1.0;
    while v * d < 10.0 && d < 1e6 {
        d *= 10.0;
    }
    let scaled = (v * d) as u64 as f64;
    short(scaled / d)
}

/// One text line as the call it is. Names resolve on the document as it stands at this line.
fn line_call(doc: &mut Document, block: &str) -> Result<Call, String> {
    let head = block.lines().next().unwrap_or("");
    let code = crate::text::uncommented(head);
    let t = crate::text::tokens(code);
    let w = |k: usize| t.get(k).map(String::as_str);
    let rest = |k: usize| t[k.min(t.len())..].join(" ");
    let block_from = |k: usize| -> String {
        // the head's words from k on, then whatever follows it (sketch rows, its comment)
        let tail = block.split_once('\n').map_or("", |x| x.1);
        let note = head.get(code.len()..).unwrap_or("");
        let first = format!("{}{note}", t[k.min(t.len())..].join(" "));
        if tail.is_empty() {
            first
        } else {
            format!("{first}\n{tail}")
        }
    };
    match w(0).unwrap_or("") {
        "before" if t.len() > 2 => {
            let before = feat(doc, &t[1])?;
            let items = doc.read_items(&block_from(2));
            Ok(Call::Add {
                before: Some(before),
                items,
            })
        }
        "before" => Err("before FEAT LINE".into()),
        "edit" if t.len() > 2 => {
            let id = feat(doc, &t[1])?;
            let old = doc.items.iter().find(|i| i.id == id).expect("found").clone();
            let mut read = doc.read_items(&block_from(2));
            let [mut item] = <[Item; 1]>::try_from(std::mem::take(&mut read))
                .map_err(|_| "edit takes one line (or one sketch)".to_string())?;
            item.id = id;
            item.label = item.label.or(old.label);
            item.note = item.note.or(old.note);
            Ok(Call::Edit { item })
        }
        "edit" => Err("edit FEAT LINE".into()),
        "set" => {
            let (Some(f), Some(k), true) = (w(1), w(2).and_then(|k| k.parse::<usize>().ok()), t.len() > 3)
            else {
                return Err("set FEAT K TEXT: K counts the words after the kind, from 1".into());
            };
            let item = feat(doc, f)?;
            Ok(Call::SetArg {
                item,
                arg: k.wrapping_sub(1),
                text: rest(3),
            })
        }
        "dim" if t.len() > 3 => {
            let item = feat(doc, &t[1])?;
            let Some(Body::Sketch { rows, .. }) = doc.items.iter().find(|i| i.id == item).map(|i| &i.body)
            else {
                return Err(format!("{} is not a sketch", t[1]));
            };
            let value = t[t.len() - 1].clone();
            let want = &t[2..t.len() - 1];
            let row = match want {
                [n] if n.parse::<usize>().is_ok() => n.parse::<usize>().expect("checked").wrapping_sub(1),
                _ => rows
                    .iter()
                    .position(|r| r.args.len() == want.len() + 1 && r.args[..want.len()] == *want)
                    .ok_or_else(|| {
                        let dims: Vec<String> = rows
                            .iter()
                            .filter(|r| {
                                matches!(
                                    r.args.first().map(String::as_str),
                                    Some("dist" | "hdist" | "vdist" | "rad" | "dia" | "ang")
                                )
                            })
                            .map(|r| r.args[..r.args.len() - 1].join(" "))
                            .collect();
                        format!("no row '{}'; dimensions: {}", want.join(" "), dims.join(", "))
                    })?,
            };
            Ok(Call::SetDim { item, row, value })
        }
        "dim" => Err("dim SKETCH ROW VALUE".into()),
        "rm" if t.len() == 2 => Ok(Call::Delete {
            item: feat(doc, &t[1])?,
        }),
        "rm" => Err("rm FEAT".into()),
        "mv" if t.len() == 3 => Ok(Call::Move {
            item: feat(doc, &t[1])?,
            before: match t[2].as_str() {
                "end" => None,
                b => Some(feat(doc, b)?),
            },
        }),
        "mv" => Err("mv FEAT FEAT2|end".into()),
        _ if w(1) == Some("=") && t.len() > 2 => match doc.param(&t[0]) {
            Some(item) => {
                let mut e = t[2..].to_vec();
                if matches!(e.last().map(String::as_str), Some("mm" | "deg")) {
                    e.pop();
                }
                Ok(Call::SetParam {
                    item,
                    expr: e.join(" "),
                })
            }
            None => Ok(Call::Add {
                before: None,
                items: doc.read_items(block),
            }),
        },
        _ => Ok(Call::Add {
            before: None,
            items: doc.read_items(block),
        }),
    }
}

/// The call's ops, applied to the document as they are made.
fn push(doc: &mut Document, call: Call, tx: &mut Tx) -> Result<(), String> {
    let (touched, ops): (Vec<ItemId>, Vec<Op>) = match call {
        Call::SetParam { item, expr } => (vec![item], vec![Op::SetParam { item, expr }]),
        Call::SetDim { item, row, value } => (vec![item], vec![Op::SetDimension { item, row, value }]),
        Call::SetArg { item, arg, text } => (vec![item], vec![Op::SetArg { item, arg, text }]),
        Call::Add { before, items } => (
            items.iter().map(|i| i.id).collect(),
            vec![Op::Insert { before, items }],
        ),
        Call::Edit { item } => (vec![item.id], vec![Op::SetItem { item }]),
        Call::Delete { item } => {
            let using = doc
                .items
                .iter()
                .find(|it| it.id != item && it.args().iter().any(|a| a.to == Some(item)))
                .map(|it| it.id)
                .or_else(|| doc.named_by(item));
            if let Some(u) = using {
                let (this, that) = (
                    doc.name(item).unwrap_or_default(),
                    doc.name(u).unwrap_or_default(),
                );
                return Err(format!("{that} names {this}: rm or edit {that} first"));
            }
            (vec![], vec![Op::Delete { item }])
        }
        Call::Move { item, before } => (vec![item], vec![Op::Move { item, before }]),
        Call::Undo | Call::Redo | Call::Select { .. } => {
            return Err("undo, redo and select go alone".into());
        }
    };
    tx.touched.extend(touched.into_iter().map(|t| (t, 0)));
    for op in ops {
        let u = doc.apply(op.clone())?;
        tx.ops.push(op);
        tx.undo.splice(0..0, u);
    }
    Ok(())
}

fn log_path(part: &Path) -> PathBuf {
    let mut p = part.as_os_str().to_owned();
    p.push(".log");
    PathBuf::from(p)
}

/// The log's bytes, if it is a plain file with one name, read through no link.
#[cfg(unix)]
fn read_plain(path: &Path) -> Option<Vec<u8>> {
    use std::io::Read;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    // a pipe would hang the open
    if !std::fs::symlink_metadata(path).ok()?.is_file() {
        return None;
    }
    let f = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(crate::cmd::NOFOLLOW)
        .open(path)
        .ok()?;
    let held = f.metadata().ok()?;
    if !held.is_file() || held.nlink() != 1 || held.len() > wire::LOG_LIMIT {
        return None;
    }
    let mut out = Vec::new();
    f.take(wire::LOG_LIMIT).read_to_end(&mut out).ok()?;
    Some(out)
}

#[cfg(not(unix))]
fn read_plain(_: &Path) -> Option<Vec<u8>> {
    None
}

/// One entry onto the log, which holds the part's text and so is as private as the part. Made if
/// it isn't there, through no link; a torn tail cut off; anything but a log with one name left
/// alone and refused. Where the entry starts, to take it back.
#[cfg(unix)]
fn append_entry(path: &Path, part_mode: u32, e: &Entry) -> Result<u64, String> {
    use std::io::{ErrorKind, Read, Seek, Write};
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
    let refused = "not an op log with one name; move it away to go on";
    let cannot = |_| "cannot write".to_string();
    let part_mode = part_mode & 0o777;
    let opened = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(crate::cmd::NOFOLLOW)
        .open(path);
    let mut f = match opened {
        Ok(f) => f,
        Err(x) if x.kind() == ErrorKind::NotFound => std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(part_mode)
            .open(path)
            .map_err(|_| "cannot write".to_string())?,
        Err(_) => return Err(refused.into()),
    };
    let held = f.metadata().map_err(cannot)?;
    if !held.is_file() || held.nlink() != 1 || held.len() > wire::LOG_LIMIT {
        return Err(refused.into());
    }
    let mut bytes = Vec::new();
    f.read_to_end(&mut bytes).map_err(cannot)?;
    let mut out = Vec::new();
    let start = if bytes.is_empty() {
        out = wire::log_header();
        0
    } else {
        let (_, good) = wire::read_log(&bytes).ok_or(refused)?;
        good as u64
    };
    if held.mode() & 0o777 & !part_mode != 0 {
        let tighter = std::fs::Permissions::from_mode(held.mode() & 0o777 & part_mode);
        f.set_permissions(tighter).map_err(cannot)?;
    }
    wire::put_entry(&mut out, e);
    f.set_len(start).map_err(cannot)?;
    f.seek(std::io::SeekFrom::End(0)).map_err(cannot)?;
    if f.write_all(&out).is_err() {
        let _ = f.set_len(start);
        return Err("cannot write".into());
    }
    Ok(start)
}

#[cfg(not(unix))]
fn append_entry(_: &Path, _: u32, _: &Entry) -> Result<u64, String> {
    Err("writing part files needs a unix system".into())
}

/// Takes the last entry back off the log.
#[cfg(unix)]
fn cut_log(path: &Path, to: u64) {
    use std::os::unix::fs::OpenOptionsExt;
    let f = std::fs::OpenOptions::new()
        .write(true)
        .custom_flags(crate::cmd::NOFOLLOW)
        .open(path);
    if let Ok(f) = f {
        let _ = f.set_len(to);
    }
}

#[cfg(not(unix))]
fn cut_log(_: &Path, _: u64) {}

#[cfg(unix)]
fn mode_of(m: &std::fs::Metadata) -> u32 {
    std::os::unix::fs::MetadataExt::mode(m)
}

#[cfg(not(unix))]
fn mode_of(_: &std::fs::Metadata) -> u32 {
    0o600
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLOCK: &str = "w = 40\nbase: extrude 10 rect 0 0 w 30\n";

    fn ok(p: &mut Part, lines: &str) -> Done {
        let d = p.act(lines, None);
        assert!(d.ok, "{lines}: {}", d.text);
        d
    }

    #[test]
    fn a_reply_is_a_diff_and_a_commit_prints_back() {
        let mut p = Part::new(BLOCK);
        let d = ok(&mut p, "fillet 2 (edges of base along +z)\nw = 50");
        assert!(d.text.starts_with("ok v2; +4 faces, volume +"), "{}", d.text);
        assert!(d.text.ends_with("valid; picked fillet1:4"), "{}", d.text);
        assert_eq!(d.lines, ["fillet 2 (edges of base along +z)", "w = 50"]);
        assert_eq!(
            p.model.src(),
            "w = 50\nbase: extrude 10 rect 0 0 w 30\nfillet 2 (edges of base along +z)\n"
        );
    }

    #[test]
    fn all_or_none() {
        let mut p = Part::new(BLOCK);
        let was = p.model.src().to_string();
        let d = p.act("w = 60\nextrude cut 1 rect 0 0 1 1\nnope 3", None);
        assert!(!d.ok && d.text.starts_with("error line 3: nope 3"), "{}", d.text);
        assert!(d.text.ends_with("nothing applied"), "{}", d.text);
        assert_eq!(p.model.src(), was);
        assert_eq!(p.model.lines.len(), 2);
        assert!((p.stat().volume - 12000.0).abs() < 1e-6);
        // what built before still does, and more goes on
        ok(&mut p, "w = 60");
        assert!((p.stat().volume - 18000.0).abs() < 1e-6);
    }

    #[test]
    fn a_change_breaking_a_line_below_says_which() {
        let mut p = Part::new("w = 40\nbase: extrude 10 rect 0 0 w 30\nhole 5 -xy at w/2 15 10 thru\n");
        let d = p.act("w = bad", None);
        assert!(!d.ok && d.text.contains("nothing applied"), "{}", d.text);
        assert!(p.model.src().contains("w = 40"));
    }

    #[test]
    fn a_replacement_that_builds_comes_with_the_error() {
        let mut p = Part::new(BLOCK);
        // too big for the block: the suggestion was built
        let d = p.act("fillet 100 (edges of base along +z)", None);
        assert!(!d.ok, "{}", d.text);
        let tried = d
            .text
            .lines()
            .find_map(|l| l.strip_prefix("try: "))
            .expect(&d.text);
        assert!(
            tried.starts_with("fillet ") && tried.ends_with("(edges of base along +z)"),
            "{tried}"
        );
        ok(&mut p, tried);
        // a misspelt kind
        let mut p = Part::new(BLOCK);
        let d = p.act("chamfr 1 (edges of base along +z)", None);
        assert!(
            d.text.contains("try: chamfer 1 (edges of base along +z)"),
            "{}",
            d.text
        );
        // a name off by a digit
        let mut p = Part::new("base: extrude 10 rect 0 0 40 30\nhole 5 -xy at 10 10 10 thru\n");
        let d = p.act("pattern hole2 lin 1 0 0 2 15", None);
        assert!(d.text.contains("try: pattern hole1 lin 1 0 0 2 15"), "{}", d.text);
    }

    #[test]
    fn a_selector_that_picks_nothing_is_refused() {
        let mut p = Part::new(BLOCK);
        let d = p.act("fillet 2 (edges of base r=77)", None);
        assert!(!d.ok, "{}", d.text);
        assert!(p.model.lines.len() == 2);
    }

    #[test]
    fn undo_and_redo_walk_the_commits() {
        let mut p = Part::new(BLOCK);
        let a = p.model.src().to_string();
        ok(&mut p, "w = 50\nh = 3");
        let b = p.model.src().to_string();
        ok(&mut p, "rm h");
        let c = p.model.src().to_string();
        ok(&mut p, "undo");
        assert_eq!(p.model.src(), b);
        ok(&mut p, "undo");
        assert_eq!(p.model.src().trim_start_matches("next 4\n"), a);
        assert!(!p.act("undo", None).ok, "the first version is where undo stops");
        ok(&mut p, "redo");
        ok(&mut p, "redo");
        assert_eq!(p.model.src(), c);
        assert!(!p.act("redo", None).ok);
        // a new commit ends the redos
        ok(&mut p, "undo");
        ok(&mut p, "w = 45");
        assert!(!p.act("redo", None).ok);
    }

    #[test]
    fn a_base_left_behind_is_refused_with_what_changed() {
        let mut p = Part::new(BLOCK);
        ok(&mut p, "w = 50");
        let v = p.version();
        ok(&mut p, "fillet 2 (edges of base along +z)");
        let d = p.act("w = 60", Some(v));
        assert!(
            !d.ok && d.text.starts_with("moved: v2 -> v3: fillet 2"),
            "{}",
            d.text
        );
        assert!(
            d.text.contains("+4 faces") && d.text.ends_with("nothing applied"),
            "{}",
            d.text
        );
        assert!(p.act("w = 60", Some(p.version())).ok);
        assert_eq!(p.since(p.version()), "v4: nothing since");
    }

    #[test]
    fn typed_calls_are_the_same_calls() {
        let mut p = Part::new(BLOCK);
        let w = p.model.doc.param("w").expect("w");
        let d = p.call(
            vec![Call::SetParam {
                item: w,
                expr: "50".into(),
            }],
            None,
        );
        assert_eq!(d.lines, ["w = 50"]);
        let mut q = Part::new(BLOCK);
        assert!(q.act(&d.lines.join("\n"), None).ok);
        assert_eq!(q.model.src(), p.model.src());
        // with nothing parsed: the body as it goes in the document
        let items = vec![Item {
            id: ItemId(9),
            label: Some("h".into()),
            body: Body::Feature {
                kind: "hole".into(),
                args: "5 -xy at 10 10 10 thru"
                    .split(' ')
                    .map(forge_doc::Arg::new)
                    .collect(),
            },
            note: None,
        }];
        let d = p.call(vec![Call::Add { before: None, items }], None);
        assert!(d.ok, "{}", d.text);
        // what it printed drives another part to the same place
        assert!(q.act(&d.lines.join("\n"), None).ok);
        assert_eq!(p.stat(), q.stat());
    }

    #[test]
    fn lines_edit_in_place_and_keep_their_names() {
        let mut p = Part::new(
            "base: extrude 10 rect 0 0 40 30\nhole 5 -xy at 10 10 10 thru\nsk: sketch xy\n  rect 0 0 5 5\n  dist l1 5\nend\n",
        );
        ok(&mut p, "set hole1 1 6");
        assert!(p.model.src().contains("hole 6 -xy at"), "{}", p.model.src());
        ok(&mut p, "edit hole1 hole 7 -xy at 20 10 10 thru");
        assert!(p.model.src().contains("hole 7 -xy at 20"), "{}", p.model.src());
        ok(&mut p, "dim sk dist l1 8");
        assert!(p.model.src().contains("dist l1 8"), "{}", p.model.src());
        ok(&mut p, "before hole1 d = 3");
        ok(&mut p, "mv hole1 end");
        assert!(p.model.src().trim_end().ends_with("thru"), "{}", p.model.src());
        let d = p.act("dim sk dist l9 8", None);
        assert!(d.text.contains("dimensions: dist l1"), "{}", d.text);
        let d = p.act("rm hole7", None);
        assert!(
            d.text.contains("no feature 'hole7'; there are hole1"),
            "{}",
            d.text
        );
    }

    #[test]
    fn a_line_going_in_above_keeps_what_selectors_mean() {
        let mut p = Part::new(BLOCK);
        ok(
            &mut p,
            "hole 5 -xy at 10 10 10 thru\nfillet 2 (edges of hole1 circle top)",
        );
        let v = p.stat().volume;
        // a hole above, far off: the round stays on the first hole, now the second
        ok(&mut p, "before hole1 hole 5 -xy at 30 10 10 thru");
        assert!(
            p.model.src().contains("(edges of hole2 circle top)"),
            "{}",
            p.model.src()
        );
        assert!(
            (p.stat().volume - (v - 196.34954)).abs() < 1e-3,
            "{}",
            p.stat().volume - v
        );
    }

    #[test]
    fn rm_is_refused_while_a_line_names_it() {
        let mut p = Part::new(BLOCK);
        ok(
            &mut p,
            "hole 5 -xy at 10 10 10 thru\npattern hole1 lin 1 0 0 2 15",
        );
        let d = p.act("rm hole1", None);
        assert!(d.text.contains("pattern1 names hole1"), "{}", d.text);
        ok(&mut p, "rm pattern1\nrm hole1");
    }

    #[test]
    fn a_sketch_goes_in_whole() {
        let mut p = Part::new("");
        ok(&mut p, "plate: sketch xy\n  rect 0 0 10 10\nend\nextrude 2 plate");
        assert_eq!(p.model.body().map(|b| b.face_ids().count()), Some(6));
        ok(&mut p, "before extrude1 q: sketch xy\n  circle 5 5 1\nend");
        assert!(
            p.model.src().contains("q: id 3 sketch xy\n  circle 5 5 1\nend"),
            "{}",
            p.model.src()
        );
    }

    #[test]
    fn dims_drag_and_commit_as_one_line() {
        let mut p = Part::new(BLOCK);
        ok(&mut p, "hole 5 -xy at 10 10 10 thru");
        let v = p.version();
        let names: Vec<String> = p.dims().iter().map(|d| d.name.clone()).collect();
        assert!(names.contains(&"w".to_string()) && names.contains(&"hole1.1".to_string()));
        let w = p.dims().iter().find(|d| d.name == "w").expect("w").at;
        for x in ["41", "42", "43"] {
            p.drag(w.op(x.into())).expect("a drag");
            assert!(p.model.error.is_none());
        }
        assert_eq!(p.version(), v, "nothing logged until it ends");
        let d = p.drag_commit(w.op("44".into()), None);
        assert_eq!(d.lines, ["w = 44"]);
        assert_eq!(p.version(), v + 1);
        ok(&mut p, "undo");
        assert!(p.model.src().starts_with("w = 40\n"), "{}", p.model.src());
    }

    #[test]
    fn select_reads_entities() {
        let mut p = Part::new(BLOCK);
        let d = p.act("select (faces plane normal +z)", None);
        assert_eq!(d.text, "ok; 1 selected");
        assert_eq!(p.selected().len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn the_log_outlives_the_process() {
        use crate::cmd::{Args, Session};
        let dir = std::env::temp_dir().join(format!("forge-tools-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let f = dir.join("a.fc");
        let file = f.to_string_lossy().to_string();
        let act = |lines: &[&str], base: Option<u64>| {
            let a = Args {
                file: file.clone(),
                words: lines.iter().map(|s| s.to_string()).collect(),
                base,
                ..Args::default()
            };
            // every command its own session, as the command line runs them
            Session::default().writable(true).run("act", &a)
        };
        let r = act(&["w = 3", "extrude w rect 0 0 1 1"], None);
        assert_eq!(r.text, "ok v2; +6 faces, volume +3 mm3, valid", "{}", r.text);
        assert!(act(&["w = 4"], Some(2)).ok);
        let r = act(&["w = 5"], Some(2));
        assert!(!r.ok && r.text.contains("moved: v2 -> v3: w = 4"), "{}", r.text);
        // undo in a fresh session, from the log
        assert!(act(&["undo"], None).ok);
        assert_eq!(
            std::fs::read_to_string(&f).expect("read"),
            "w = 3\nextrude w rect 0 0 1 1\n"
        );
        // a hand edit goes in as text, and undo stops there
        std::fs::write(&f, "w = 7\nextrude w rect 0 0 1 1\n").expect("write");
        let r = act(&["w = 8"], None);
        assert!(r.ok && r.text.starts_with("ok v6;"), "{}", r.text);
        assert!(act(&["undo"], None).ok);
        assert_eq!(
            std::fs::read_to_string(&f).expect("read"),
            "w = 7\nextrude w rect 0 0 1 1\n"
        );
        let r = act(&["undo"], None);
        assert!(!r.ok && r.text.contains("edited by hand"), "{}", r.text);
        // a log torn at its end keeps what is whole
        let log = dir.join("a.fc.log");
        let bytes = std::fs::read(&log).expect("log");
        let (entries, _) = wire::read_log(&bytes).expect("a log");
        std::fs::write(&log, &bytes[..bytes.len() - 2]).expect("write");
        let r = act(&["w = 9"], None);
        assert!(r.ok, "{}", r.text);
        let (now, _) = wire::read_log(&std::fs::read(&log).expect("log")).expect("a log");
        assert!(now.len() >= entries.len() - 1, "{} {}", now.len(), entries.len());
        std::fs::remove_dir_all(&dir).expect("clean up");
    }
}
