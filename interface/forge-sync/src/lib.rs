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
pub use sequencer::{Commit, Sequencer};
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
            _ => Refusal::Lang {
                kind: kind.into(),
                text: text.into(),
            },
        }
    }
}

/// What the sequencer's host keeps durably: the document's first text and its log.
pub trait Store {
    /// The first text and every entry since, or None if the document was never created.
    fn load(&mut self, doc: &str) -> std::io::Result<Option<(String, Vec<Entry>)>>;
    /// Creates the document with its first text (seq 0).
    fn create(&mut self, doc: &str, text: &str) -> std::io::Result<()>;
    /// Appends one entry; returns only when it is durable (the store's definition).
    fn append(&mut self, doc: &str, e: &Entry) -> std::io::Result<()>;
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
    docs: std::collections::BTreeMap<String, (String, Vec<Entry>)>,
}

impl Store for MemStore {
    fn load(&mut self, doc: &str) -> std::io::Result<Option<(String, Vec<Entry>)>> {
        Ok(self.docs.get(doc).cloned())
    }
    fn create(&mut self, doc: &str, text: &str) -> std::io::Result<()> {
        if self.docs.contains_key(doc) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                doc.to_string(),
            ));
        }
        self.docs.insert(doc.into(), (text.into(), vec![]));
        Ok(())
    }
    fn append(&mut self, doc: &str, e: &Entry) -> std::io::Result<()> {
        let d = self.docs.get_mut(doc).ok_or(std::io::ErrorKind::NotFound)?;
        if e.seq != d.1.len() as u64 + 1 {
            return Err(std::io::Error::other(format!(
                "append of seq {} after {}",
                e.seq,
                d.1.len()
            )));
        }
        d.1.push(e.clone());
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
/// hash). A linear pass over the body; clients report it after each build.
pub fn body_hash(run: &forge_lang::Run) -> u64 {
    let mut h = forge_math::hash::BitHasher::new();
    for e in &run.errors {
        for b in e.kind().bytes() {
            h.u64(b as u64);
        }
    }
    h.u64(run.errors.len() as u64);
    match &run.body {
        Some(b) => {
            for v in &b.vertices {
                h.p3(v.point);
            }
            for e in &b.edges {
                h.f64(e.range.lo);
                h.f64(e.range.hi);
            }
            h.u64(b.faces.len() as u64);
        }
        None => h.u64(u64::MAX),
    }
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
