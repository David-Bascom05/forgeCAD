//! The sequencer: one per document, on one server at a time (DESIGN.md §4.2). It holds the head
//! text and seq, orders every op it accepts, and refuses the rest with their typed errors.
//!
//! What it keeps is bounded in bytes, not only in count: the log in memory is what came after
//! the latest snapshot, and once it holds [`Limits::max_log_bytes`] it is **compacted** (the
//! head becomes the store's snapshot, the entries before it go); the texts kept for hash checks
//! are a short ring; the head's error counts are kept, so an op's check evaluates only the new
//! text, not the head again.

use crate::rebase::{rebase, span_op};
use crate::{Counts, Entry, Refusal, Store, check_limits, inverse::inverse, text_hash};
use forge_lang::error::Op;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;

/// How an op is checked on the head before it is appended: by default forge-lang's own check
/// ([`crate::validate`]) in this thread; a server gives one with a time budget. `counts` are the
/// head's error counts if known (then the head is not evaluated again); it returns the new text
/// and its counts.
pub trait Check {
    fn check(
        &mut self,
        head: &str,
        counts: Option<&Counts>,
        op: &Op,
        dir: Option<PathBuf>,
        root: Option<PathBuf>,
    ) -> Result<(String, Counts), Refusal>;
}

/// [`crate::validate`], in the caller's thread.
pub struct Direct;

impl Check for Direct {
    fn check(
        &mut self,
        head: &str,
        counts: Option<&Counts>,
        op: &Op,
        dir: Option<PathBuf>,
        root: Option<PathBuf>,
    ) -> Result<(String, Counts), Refusal> {
        crate::validate(head, counts, op, dir, root)
    }
}

/// What one document may grow to.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// The head text's bytes (below the wire's `MAX_MESSAGE`, so a snapshot always fits).
    pub max_text: usize,
    /// The log's bytes in memory (and in the store) before it is compacted to a snapshot.
    pub max_log_bytes: usize,
    /// Texts kept for hash checks: at most this many, and at most `recent_bytes` in all.
    pub recent: usize,
    pub recent_bytes: usize,
    /// Clients remembered for retries beyond compaction and restarts, per account (one
    /// account's clients cannot push another's out). Each is a high-water mark plus its refused
    /// ids ([`Mark`]), so this is O(clients), not O(ops).
    pub seen: usize,
    /// Refused op ids remembered per client (the newest: a client resends only its pending ops,
    /// which are its newest).
    pub refused: usize,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits {
            max_text: 4 << 20,
            max_log_bytes: 16 << 20,
            recent: 64,
            recent_bytes: 16 << 20,
            seen: 1_000,
            refused: 1_000,
        }
    }
}

/// An op the sequencer committed: the host acks its author and broadcasts it to the others.
#[derive(Clone, Debug, PartialEq)]
pub struct Commit {
    pub entry: Entry,
    /// True when (client, id) had already been committed: the same entry again, nothing written
    /// (a client's retry after a lost connection).
    pub duplicate: bool,
}

pub struct Sequencer {
    pub doc: String,
    /// The folder the document's sibling files are read from while validating (a `part(FILE)`).
    pub dir: Option<PathBuf>,
    /// The project's folder: validation reads no file outside it.
    pub root: Option<PathBuf>,
    pub limits: Limits,
    pub checker: Box<dyn Check + Send>,
    /// The seq of the latest snapshot: the log holds the entries after it.
    floor: u64,
    head: String,
    head_counts: Option<Counts>,
    log: VecDeque<Entry>,
    /// Each entry's bytes as stored, and their sum.
    sizes: VecDeque<usize>,
    log_bytes: usize,
    /// Recent texts by seq (hash checks build them).
    recent: VecDeque<(u64, Arc<str>)>,
    recent_bytes: usize,
    /// (client, op id) → seq of the entries in `log` (a retry gets the same entry again).
    in_log: HashMap<(String, u64), u64>,
    /// Per client: what it had committed, for retries of ops before the log (after a
    /// compaction or a restart); least recently used last per account, for eviction.
    marks: HashMap<String, Mark>,
    mark_order: HashMap<String, VecDeque<String>>,
    /// Times the log was compacted (tests, stats).
    pub compactions: usize,
}

/// A client's committed ops, exactly, in O(1) per client: its op ids are sent in increasing order
/// (`Client::next_id`), so every id up to `hwm` was committed unless it is in `refused`.
#[derive(Clone, Debug, Default)]
struct Mark {
    /// The highest op id committed, and its seq.
    hwm: u64,
    seq: u64,
    /// Ids refused (never committed); a retry of one is checked again, never "already
    /// committed".
    refused: std::collections::BTreeSet<u64>,
}

fn account_of(client: &str) -> String {
    // the account is the client key's first part (`author/client`, as the server keys it)
    client.split('/').next().unwrap_or("").to_string()
}

fn entry_bytes(e: &Entry) -> usize {
    crate::wire::write(&crate::wire::entry_json(e)).len()
}

impl Sequencer {
    /// Opens a document from its store, creating it with `text` if it does not exist.
    pub fn open(
        store: &mut dyn Store,
        doc: &str,
        text: &str,
        dir: Option<PathBuf>,
    ) -> std::io::Result<Sequencer> {
        let snap = match store.latest_snapshot(doc)? {
            Some(x) => x,
            None => {
                store.create(doc, text)?;
                crate::Snapshot {
                    seq: 0,
                    text: text.to_string(),
                    seen: vec![],
                    refused: vec![],
                }
            }
        };
        let (floor, base) = (snap.seq, snap.text);
        let mut s = Sequencer {
            doc: doc.into(),
            dir,
            root: None,
            limits: Limits::default(),
            checker: Box::new(Direct),
            floor,
            head: base,
            head_counts: None,
            log: VecDeque::new(),
            sizes: VecDeque::new(),
            log_bytes: 0,
            recent: VecDeque::new(),
            recent_bytes: 0,
            in_log: HashMap::new(),
            marks: HashMap::new(),
            mark_order: HashMap::new(),
            compactions: 0,
        };
        // (a log from before marks wrote one triple per op: the highest per client is its mark);
        // in seq order, so each account's least recently used client is evicted first, as before
        // the reopen (a client offline with pending ops is the one to keep)
        let mut seen = snap.seen;
        seen.sort_by_key(|x| x.2);
        for (client, id, seq) in seen {
            s.mark_committed(&client, id, seq);
        }
        for (client, id) in snap.refused {
            s.mark_refused(&client, id);
        }
        for e in store.read_since(doc, floor)? {
            if e.seq != s.seq() + 1 {
                return Err(std::io::Error::other(format!(
                    "{doc}: entry {} after {}",
                    e.seq,
                    s.seq()
                )));
            }
            let head = forge_lang::edit::apply(&s.head, &e.op).map_err(|err| {
                std::io::Error::other(format!("{doc}: seq {} does not apply: {}", e.seq, err.kind()))
            })?;
            if text_hash(&head) != e.th {
                return Err(std::io::Error::other(format!(
                    "{doc}: seq {} text hash differs",
                    e.seq
                )));
            }
            s.head = head;
            s.remember(&e);
            s.push(e);
        }
        Ok(s)
    }

    pub fn seq(&self) -> u64 {
        self.floor + self.log.len() as u64
    }
    pub fn head(&self) -> &str {
        &self.head
    }
    /// The seq of the latest snapshot: a client behind it needs a snapshot, not a catch-up.
    pub fn floor(&self) -> u64 {
        self.floor
    }
    /// The entries after `seq`, if the log still has them all (a reconnecting client's catch-up).
    pub fn since(&self, seq: u64) -> Option<impl Iterator<Item = &Entry>> {
        if seq < self.floor || seq > self.seq() {
            return None;
        }
        Some(self.log.iter().skip((seq - self.floor) as usize))
    }
    /// The bytes of the entries after `seq` (what a catch-up would send).
    pub fn bytes_since(&self, seq: u64) -> usize {
        let skip = seq.saturating_sub(self.floor) as usize;
        self.sizes.iter().skip(skip).sum()
    }
    pub fn log_bytes(&self) -> usize {
        self.log_bytes
    }
    /// The text at `seq`, if it is the head or one of the recent ones (never a replay of the log:
    /// a client must not be able to make the server do that).
    pub fn text_at(&self, seq: u64) -> Option<Arc<str>> {
        if seq == self.seq() {
            return Some(Arc::from(self.head.as_str()));
        }
        self.recent.iter().find(|r| r.0 == seq).map(|r| r.1.clone())
    }

    fn remember(&mut self, e: &Entry) {
        self.in_log.insert((e.client.clone(), e.id), e.seq);
        self.mark_committed(&e.client, e.id, e.seq);
    }

    /// The client's mark, made most recently used (an account keeps its `limits.seen` newest).
    fn mark(&mut self, client: &str) -> &mut Mark {
        let order = self.mark_order.entry(account_of(client)).or_default();
        if let Some(k) = order.iter().position(|c| c == client) {
            let c = order.remove(k).expect("there");
            order.push_back(c);
        } else {
            order.push_back(client.to_string());
            while order.len() > self.limits.seen {
                if let Some(old) = order.pop_front() {
                    self.marks.remove(&old);
                }
            }
        }
        self.marks.entry(client.to_string()).or_default()
    }

    fn mark_committed(&mut self, client: &str, id: u64, seq: u64) {
        let m = self.mark(client);
        m.refused.remove(&id);
        if id > m.hwm {
            m.hwm = id;
            m.seq = seq;
        }
    }

    fn mark_refused(&mut self, client: &str, id: u64) {
        let keep = self.limits.refused;
        let m = self.mark(client);
        m.refused.insert(id);
        while m.refused.len() > keep {
            m.refused.pop_first();
        }
    }

    fn push(&mut self, e: Entry) {
        let n = entry_bytes(&e);
        self.log_bytes += n;
        self.sizes.push_back(n);
        self.log.push_back(e);
    }

    fn keep_recent(&mut self) {
        let t: Arc<str> = Arc::from(self.head.as_str());
        self.recent_bytes += t.len();
        self.recent.push_back((self.seq(), t));
        while self.recent.len() > self.limits.recent || self.recent_bytes > self.limits.recent_bytes {
            match self.recent.pop_front() {
                Some((_, t)) => self.recent_bytes -= t.len(),
                None => break,
            }
        }
    }

    /// Orders `op`, sent by `client` (account `author`) as its op `id`, made on the text at
    /// `base`: limits, rebase, validation on the head, durable append. Refused: nothing written.
    pub fn submit(
        &mut self,
        store: &mut dyn Store,
        author: &str,
        client: &str,
        base: u64,
        id: u64,
        op: &Op,
    ) -> Result<Commit, Refusal> {
        if let Some(&seq) = self.in_log.get(&(client.to_string(), id))
            && let Some(e) = self.log.get((seq - self.floor - 1) as usize)
        {
            return Ok(Commit {
                entry: e.clone(),
                duplicate: true,
            });
        }
        if let Some(m) = self.marks.get(client)
            && id <= m.hwm
            && !m.refused.contains(&id)
        {
            // committed before the log's start: the client's snapshot holds it
            return Err(Refusal::AlreadyCommitted { seq: m.seq });
        }
        let r = self.order(store, author, client, base, id, op);
        if r.is_err() {
            self.mark_refused(client, id);
        }
        r
    }

    fn order(
        &mut self,
        store: &mut dyn Store,
        author: &str,
        client: &str,
        base: u64,
        id: u64,
        op: &Op,
    ) -> Result<Commit, Refusal> {
        check_limits(op)?;
        let head = self.seq();
        if base > head {
            return Err(Refusal::BadBase { base, head });
        }
        let since: Vec<Entry> = match self.since(base) {
            Some(s) => s.cloned().collect(),
            // made before the in-memory log's start: the ops in between are read from the store's
            // history (kept: a snapshot deletes nothing), and the op is rebased through them like
            // any other. Never sent as if made on a newer text: that would skip a Rename's rebase
            // and land the edit on another name.
            None => {
                let older = store
                    .read_range(&self.doc, base, self.floor)
                    .map_err(|e| Refusal::Store { why: e.to_string() })?;
                let whole = older.as_ref().is_some_and(|o| {
                    o.len() as u64 == self.floor - base && o.iter().zip(base + 1..).all(|(e, k)| e.seq == k)
                });
                if !whole {
                    return Err(Refusal::RebaseGone { base });
                }
                older
                    .expect("whole")
                    .into_iter()
                    .chain(self.log.iter().cloned())
                    .collect()
            }
        };
        let op = rebase(op, client, since.iter())?;
        // a span op is valid only on its base: rebase refused it if another author's op landed;
        // the author's own ops since base moved its bytes too unless there are none
        if span_op(&op) && !since.is_empty() {
            return Err(Refusal::StaleSpan);
        }
        let checked = self.checker.check(
            &self.head,
            self.head_counts.as_ref(),
            &op,
            self.dir.clone(),
            self.root.clone(),
        );
        let (text, counts) = match checked {
            Ok(x) => x,
            Err(r) => return Err(name_the_taker(r, &op, client, &since)),
        };
        if text.len() > self.limits.max_text {
            return Err(Refusal::Invalid {
                why: format!(
                    "the document would be {} bytes (at most {})",
                    text.len(),
                    self.limits.max_text
                ),
            });
        }
        let inv = inverse(&self.head, &op, &text);
        let entry = Entry {
            seq: head + 1,
            author: author.into(),
            client: client.into(),
            id,
            base,
            op,
            inv,
            th: text_hash(&text),
        };
        store
            .append(&self.doc, &entry)
            .map_err(|e| Refusal::Store { why: e.to_string() })?;
        self.head = text;
        self.head_counts = Some(counts);
        self.remember(&entry);
        self.push(entry.clone());
        self.keep_recent();
        if self.log_bytes > self.limits.max_log_bytes {
            // the entry is durable already: a failed compaction only leaves the log longer
            if let Err(e) = self.compact(store) {
                eprintln!("forge-sync: {}: compaction failed: {e}", self.doc);
            }
        }
        Ok(Commit {
            entry,
            duplicate: false,
        })
    }

    /// The head becomes the store's snapshot; the entries before it go (clients behind it get a
    /// snapshot when they join; their retries of ops before it are `AlreadyCommitted`).
    pub fn compact(&mut self, store: &mut dyn Store) -> std::io::Result<()> {
        let mut seen: Vec<(String, u64, u64)> = self
            .marks
            .iter()
            .filter(|(_, m)| m.hwm > 0)
            .map(|(c, m)| (c.clone(), m.hwm, m.seq))
            .collect();
        seen.sort();
        let mut refused: Vec<(String, u64)> = self
            .marks
            .iter()
            .flat_map(|(c, m)| m.refused.iter().map(|i| (c.clone(), *i)))
            .collect();
        refused.sort();
        let snap = crate::Snapshot {
            seq: self.seq(),
            text: self.head.clone(),
            seen,
            refused,
        };
        store.compact(&self.doc, &snap)?;
        self.floor = self.seq();
        self.log.clear();
        self.in_log.clear();
        self.sizes.clear();
        self.log_bytes = 0;
        self.compactions += 1;
        Ok(())
    }
}

/// A NameTaken refusal of an op whose name another author's concurrent op took: the refusal
/// says which op (seq and author), so the person knows what happened.
fn name_the_taker(r: Refusal, op: &Op, client: &str, since: &[Entry]) -> Refusal {
    let Refusal::Lang { kind, text } = &r else {
        return r;
    };
    if kind != "NameTaken" {
        return r;
    }
    let binds = |t: &str| -> Vec<String> {
        forge_lang::syntax::parse(t)
            .map(|p| forge_lang::edit::bound(&p).into_iter().collect())
            .unwrap_or_default()
    };
    let mine: Vec<String> = match op {
        Op::Rename { to, .. } => vec![to.clone()],
        Op::AddFeature { text, .. } => binds(text),
        _ => vec![],
    };
    let taker = since
        .iter()
        .rev()
        .filter(|e| e.client != client)
        .find(|e| match &e.op {
            Op::Rename { to, .. } => mine.contains(to),
            Op::AddFeature { text, .. } => binds(text).iter().any(|n| mine.contains(n)),
            _ => false,
        });
    match taker {
        Some(e) => Refusal::Lang {
            kind: kind.clone(),
            text: format!("{text} (taken just now by {}'s edit at seq {})", e.author, e.seq),
        },
        None => r,
    }
}
