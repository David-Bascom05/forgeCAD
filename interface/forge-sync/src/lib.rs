//! forge-sync: hosted forgeCAD's protocol, with no I/O (research/hosted/DESIGN.md §4, §14).
//!
//! A document is a **server-ordered log of typed ops** (forge-lang's `Op`). Clients apply their
//! own ops at once (no network on the edit path) and send them with the seq they were made on;
//! the [`Sequencer`] brings a late op to the head ([`rebase`]: Rename's name substitution, then
//! re-validation with `apply_checked`), appends it durably through a [`Store`], and the host
//! broadcasts it. Every client that applies the log in seq order ends at the server's text
//! (`apply` is a deterministic function of (text, op)); each message carries the text's hash so
//! a divergence is detected, never assumed away, and clients report the hash of each body they
//! build ([`body_hash`]) so a geometry divergence is detected too.
//!
//! The crate has no threads, sockets or files of its own (the [`Store`] and [`Transport`] traits
//! are the host's), so the same code runs in forge-server, the native desktop app and the
//! browser build. Nothing in the speed gate's binaries links it, and LOCAL projects never run it.

pub mod client;
pub mod inverse;
pub mod rebase;
pub mod room;
pub mod sequencer;
pub mod wire;

pub use client::{Client, Event};
pub use rebase::rebase;
pub use room::{Action, Room};
pub use sequencer::{Check, Commit, Direct, Limits, Sequencer};
pub use wire::{Down, Up};

use forge_lang::error::Op;

/// One committed op, as the log stores it.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// Its place in the document's order (1, 2, ...; 0 is the document's first text).
    pub seq: u64,
    /// Who sent it (the account; in phase 1 the dev token's user).
    pub author: String,
    /// The client (a connection's stable id) it came from: its ops are deduplicated by
    /// (client, id), and its later ops are not transformed through it.
    pub client: String,
    /// The client's op id.
    pub id: u64,
    /// The seq the client made it on.
    pub base: u64,
    /// The op as applied (rebased).
    pub op: Op,
    /// Its exact inverse on the text it was applied to (undo), if it has one.
    pub inv: Option<Op>,
    /// The hash of the text after it.
    pub th: u64,
}

/// Why an op was not committed. Nothing was written.
#[derive(Clone, Debug, PartialEq)]
pub enum Refusal {
    /// forge-lang refused the op on the head: its typed error (kind and rendered text).
    Lang { kind: String, text: String },
    /// A Rename of a name another author renamed first.
    RenameConflict { name: String },
    /// An op addressed by byte span whose base is no longer the head.
    StaleSpan,
    /// The op's base is not a seq this document has.
    BadBase { base: u64, head: u64 },
    /// The op is outside the protocol's limits (sizes, non-finite numbers, names).
    Invalid { why: String },
    /// The store could not append it.
    Store { why: String },
    /// A retry of an op committed (at `seq`) before the latest snapshot: the client's snapshot
    /// already holds it (the client drops it silently).
    AlreadyCommitted { seq: u64 },
    /// The client's protocol or kernel differs from the server's (bit-identical geometry holds
    /// only for one kernel build).
    Mismatch { why: String },
}

impl Refusal {
    pub fn kind(&self) -> String {
        match self {
            Refusal::Lang { kind, .. } => kind.clone(),
            Refusal::RenameConflict { .. } => "RenameConflict".into(),
            Refusal::StaleSpan => "StaleSpan".into(),
            Refusal::BadBase { .. } => "BadBase".into(),
            Refusal::Invalid { .. } => "Invalid".into(),
            Refusal::Store { .. } => "Store".into(),
            Refusal::AlreadyCommitted { .. } => "AlreadyCommitted".into(),
            Refusal::Mismatch { .. } => "Mismatch".into(),
        }
    }
    pub fn text(&self) -> String {
        match self {
            Refusal::Lang { text, .. } => text.clone(),
            Refusal::RenameConflict { name } => format!("`{name}` was renamed by someone else first"),
            Refusal::StaleSpan => "the text this edit points into changed: make it again".into(),
            Refusal::BadBase { base, head } => {
                format!("base {base} is not a version of this document (head {head})")
            }
            Refusal::Invalid { why } => why.clone(),
            Refusal::Store { why } => format!("not saved: {why}"),
            Refusal::AlreadyCommitted { seq } => format!("already committed at seq {seq}"),
            Refusal::Mismatch { why } => why.clone(),
        }
    }
    /// The refusal a client reads back from the wire.
    pub fn from_wire(kind: &str, text: &str) -> Refusal {
        match kind {
            "RenameConflict" => Refusal::RenameConflict {
                name: text.split('`').nth(1).unwrap_or("").to_string(),
            },
            "StaleSpan" => Refusal::StaleSpan,
            "Invalid" => Refusal::Invalid { why: text.into() },
            "Store" => Refusal::Store { why: text.into() },
            "AlreadyCommitted" => Refusal::AlreadyCommitted {
                seq: text.rsplit(' ').next().and_then(|x| x.parse().ok()).unwrap_or(0),
            },
            "Mismatch" => Refusal::Mismatch { why: text.into() },
            _ => Refusal::Lang {
                kind: kind.into(),
                text: text.into(),
            },
        }
    }
}

/// What the sequencer's host keeps durably. The shape is a database's, not a file's (DESIGN.md
/// §5: Postgres is the phase-2 store): a snapshot, the entries since it, an atomic append, and a
/// compaction that makes the head the snapshot.
pub trait Store {
    /// The latest snapshot: (its seq, the text at it), or None if the document was never created.
    fn latest_snapshot(&mut self, doc: &str) -> std::io::Result<Option<(u64, String)>>;
    /// Every entry after `seq`, in order.
    fn read_since(&mut self, doc: &str, seq: u64) -> std::io::Result<Vec<Entry>>;
    /// Creates the document with its first text (the snapshot at seq 0).
    fn create(&mut self, doc: &str, text: &str) -> std::io::Result<()>;
    /// Appends one entry; returns only when it is durable (the store's definition). Failed:
    /// nothing of it is kept.
    fn append(&mut self, doc: &str, e: &Entry) -> std::io::Result<()>;
    /// `text` is the document at `seq` (the last entry's): it becomes the snapshot, and the
    /// entries up to it may go. Atomic: after a crash, either the old log or the new snapshot.
    fn compact(&mut self, doc: &str, seq: u64, text: &str) -> std::io::Result<()>;
}

/// Where a client's or server's messages go (a WebSocket, a channel in a test).
pub trait Transport {
    fn send(&mut self, text: String);
}

/// The host's clock, in milliseconds (std's on native, `performance.now()` in the browser).
pub trait Clock {
    fn now_ms(&self) -> f64;
}

/// An in-memory [`Store`]: tests, and a server run with no durable store.
#[derive(Default)]
pub struct MemStore {
    docs: std::collections::BTreeMap<String, (u64, String, Vec<Entry>)>,
}

impl Store for MemStore {
    fn latest_snapshot(&mut self, doc: &str) -> std::io::Result<Option<(u64, String)>> {
        Ok(self.docs.get(doc).map(|d| (d.0, d.1.clone())))
    }
    fn read_since(&mut self, doc: &str, seq: u64) -> std::io::Result<Vec<Entry>> {
        Ok(self
            .docs
            .get(doc)
            .map(|d| d.2.iter().filter(|e| e.seq > seq).cloned().collect())
            .unwrap_or_default())
    }
    fn create(&mut self, doc: &str, text: &str) -> std::io::Result<()> {
        if self.docs.contains_key(doc) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                doc.to_string(),
            ));
        }
        self.docs.insert(doc.into(), (0, text.into(), vec![]));
        Ok(())
    }
    fn append(&mut self, doc: &str, e: &Entry) -> std::io::Result<()> {
        let d = self.docs.get_mut(doc).ok_or(std::io::ErrorKind::NotFound)?;
        let last = d.2.last().map_or(d.0, |x| x.seq);
        if e.seq != last + 1 {
            return Err(std::io::Error::other(format!(
                "append of seq {} after {last}",
                e.seq
            )));
        }
        d.2.push(e.clone());
        Ok(())
    }
    fn compact(&mut self, doc: &str, seq: u64, text: &str) -> std::io::Result<()> {
        let d = self.docs.get_mut(doc).ok_or(std::io::ErrorKind::NotFound)?;
        d.0 = seq;
        d.1 = text.into();
        d.2.retain(|e| e.seq > seq);
        Ok(())
    }
}

/// The hash of a document's text (FNV-1a, 64 bits): every Ack and Applied carries the server's,
/// so a client whose text differs knows at once.
pub fn text_hash(t: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in t.as_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// The exact hash of a build: every vertex's point and every edge's range bit for bit, the
/// face count, and the kinds of the errors (as tools/speed's `bits` and the determinism gate
/// hash). A linear pass over the body.
pub fn body_hash(run: &forge_lang::Run) -> u64 {
    let mut h = forge_math::hash::BitHasher::new();
    for e in &run.errors {
        for b in e.kind().bytes() {
            h.u64(b as u64);
        }
    }
    h.u64(run.errors.len() as u64);
    match &run.body {
        Some(b) => h.u64(hash_body(b)),
        None => h.u64(u64::MAX),
    }
    h.finish()
}

/// The exact hash of a body alone (what the app reports after each successful build: it holds
/// the body, not the run).
pub fn hash_body(b: &forge_topo::Body) -> u64 {
    let mut h = forge_math::hash::BitHasher::new();
    for v in &b.vertices {
        h.p3(v.point);
    }
    for e in &b.edges {
        h.f64(e.range.lo);
        h.f64(e.range.hi);
    }
    h.u64(b.faces.len() as u64);
    h.finish()
}

/// The protocol's limits on one op (checked before anything else, on the server).
pub const MAX_OP_TEXT: usize = 256 << 10;
pub const MAX_NAME: usize = 256;

/// Refuses an op outside the protocol's limits: a text over [`MAX_OP_TEXT`], a name over
/// [`MAX_NAME`] or with characters a name cannot have, a non-finite value.
pub fn check_limits(op: &Op) -> Result<(), Refusal> {
    let bad = |why: String| Err(Refusal::Invalid { why });
    let mut names: Vec<&str> = vec![];
    let mut texts: Vec<&str> = vec![];
    match op {
        Op::SetParam { value, .. } | Op::AddLocalParam { value, .. } if !value.value.is_finite() => {
            return bad("a value that is not a finite number".into());
        }
        Op::AddLocalParam { name, .. } => names.push(name),
        Op::AddFeature { before, text } => {
            names.extend(before.as_deref());
            texts.push(text);
        }
        Op::DeleteFeature { id } | Op::Suppress { id } | Op::Unsuppress { id } => names.push(id),
        Op::ReplaceFeature { id, text } => {
            names.push(id);
            texts.push(text);
        }
        Op::MoveFeature { id, before } => {
            names.push(id);
            names.extend(before.as_deref());
        }
        Op::SetArg { id, arg, text } => {
            names.extend([id.as_str(), arg.as_str()]);
            texts.push(text);
        }
        Op::Rename { from, to } => names.extend([from.as_str(), to.as_str()]),
        Op::SetProperty { key, text } => {
            names.push(key);
            texts.extend(text.as_deref());
        }
        Op::LiftLiteral { name, .. } => names.push(name),
        Op::Respell { to, .. } => names.push(to),
        _ => {}
    }
    if let Some(n) = names.iter().find(|n| !path_ok(n)) {
        return bad(format!(
            "not a name: {:?}",
            n.chars().take(40).collect::<String>()
        ));
    }
    if let Some(t) = texts.iter().find(|t| t.len() > MAX_OP_TEXT) {
        return bad(format!("a text of {} bytes (at most {MAX_OP_TEXT})", t.len()));
    }
    Ok(())
}

/// A name, a feature path (`bolt[2].hub`) or a key: ASCII letters, digits, `_ . [ ] ,`, at most
/// [`MAX_NAME`] bytes.
pub fn path_ok(n: &str) -> bool {
    !n.is_empty()
        && n.len() <= MAX_NAME
        && n.chars()
            .all(|c| c.is_ascii_alphanumeric() || "_.[],".contains(c))
}

/// A run's errors counted by kind: what an op's check compares (a new error is one of a kind
/// the head had fewer of).
pub type Counts = std::collections::BTreeMap<&'static str, usize>;

fn counts(r: &forge_lang::Run) -> Counts {
    let mut c = Counts::new();
    for e in &r.errors {
        *c.entry(e.kind()).or_default() += 1;
    }
    c
}

/// forge-lang's check of an op on `head` (as `edit::apply_checked`, without the reach): applied,
/// then refused with the first new typed error the result has that `head` had not, counted by
/// kind. `head_counts`, if known, spare evaluating the head again. `root` confines what files the
/// evaluation may read (a server passes its project folder). Returns the new text and its counts.
pub fn validate(
    head: &str,
    head_counts: Option<&Counts>,
    op: &Op,
    dir: Option<std::path::PathBuf>,
    root: Option<std::path::PathBuf>,
) -> Result<(String, Counts), Refusal> {
    let lang = |e: forge_lang::error::Error| Refusal::Lang {
        kind: e.kind().into(),
        text: e.text(),
    };
    let text = forge_lang::edit::apply(head, op).map_err(lang)?;
    let opts = forge_lang::Options {
        build: false,
        dir,
        root,
        ..forge_lang::Options::default()
    };
    let before = match head_counts {
        Some(c) => c.clone(),
        None => counts(&forge_lang::run(head, opts.clone(), Default::default()).0),
    };
    let (after, _) = forge_lang::run(&text, opts, Default::default());
    let now = counts(&after);
    if let Some(e) = after
        .errors
        .iter()
        .find(|e| now.get(e.kind()) > before.get(e.kind()).or(Some(&0)))
    {
        return Err(lang(e.clone()));
    }
    Ok((text, now))
}

/// The wire protocol's version: a client of another version is refused at Hello.
pub const PROTOCOL: u32 = 1;

/// This build's kernel: the pinned toolchain and the hash of the kernel's and the language's
/// sources (build.rs). Geometry is bit-identical only between equal kernels, so a room holds one.
pub const KERNEL: &str = env!("FORGE_KERNEL_ID");

/// The exact hash of placed bodies (an assembly's occurrences: each part's body and its pose),
/// independent of their order: each occurrence's hash, sorted, then hashed.
pub fn hash_placed<'a>(
    placed: impl IntoIterator<Item = (&'a forge_topo::Body, &'a forge_math::Frame)>,
) -> u64 {
    let mut each: Vec<u64> = placed
        .into_iter()
        .map(|(b, f)| {
            let mut h = forge_math::hash::BitHasher::new();
            h.u64(hash_body(b));
            h.p3(f.origin);
            h.v3(f.x().get());
            h.v3(f.y().get());
            h.v3(f.z().get());
            h.finish()
        })
        .collect();
    each.sort_unstable();
    let mut h = forge_math::hash::BitHasher::new();
    h.u64(each.len() as u64);
    for x in each {
        h.u64(x);
    }
    h.finish()
}

/// What a client reports after a build of a run: the body's hash for a part, the placed
/// occurrences' for an assembly (whose run has no single body), none if it built nothing.
pub fn run_hash(run: &forge_lang::Run) -> Option<u64> {
    if let Some(a) = &run.asm
        && !a.draw.is_empty()
    {
        return Some(hash_placed(a.draw.iter().map(|(b, f)| (&**b, f))));
    }
    run.body.as_ref().map(hash_body)
}
