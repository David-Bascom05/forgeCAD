//! The sequencer: one per document, on one server at a time (DESIGN.md §4.2). It holds the head
//! text and seq, orders every op it accepts, and refuses the rest with their typed errors.

use crate::rebase::{rebase, span_op};
use crate::{Entry, Refusal, Store, check_limits, inverse::inverse, text_hash};
use forge_lang::error::Op;
use std::collections::HashMap;
use std::path::PathBuf;

/// How an op is checked on the head before it is appended: by default forge-lang's own check
/// ([`crate::validate`]) in this thread; a server gives one with a time budget.
pub trait Check {
    fn check(
        &mut self,
        head: &str,
        op: &Op,
        dir: Option<PathBuf>,
        root: Option<PathBuf>,
    ) -> Result<String, Refusal>;
}

/// [`crate::validate`], in the caller's thread.
pub struct Direct;

impl Check for Direct {
    fn check(
        &mut self,
        head: &str,
        op: &Op,
        dir: Option<PathBuf>,
        root: Option<PathBuf>,
    ) -> Result<String, Refusal> {
        crate::validate(head, op, dir, root)
    }
}

/// What one document may grow to.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// The head text's bytes (below the wire's `MAX_MESSAGE`, so a snapshot always fits).
    pub max_text: usize,
    /// Entries in the log (phase 1 keeps the whole log; compaction with snapshots is phase 2).
    pub max_log: usize,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits {
            max_text: 4 << 20,
            max_log: 200_000,
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
    first: String,
    head: String,
    log: Vec<Entry>,
    /// (client, op id) → seq, for idempotent retries.
    seen: HashMap<(String, u64), u64>,
}

impl Sequencer {
    /// Opens a document from its store, creating it with `text` if it does not exist.
    pub fn open(
        store: &mut dyn Store,
        doc: &str,
        text: &str,
        dir: Option<std::path::PathBuf>,
    ) -> std::io::Result<Sequencer> {
        let (first, log) = match store.load(doc)? {
            Some(x) => x,
            None => {
                store.create(doc, text)?;
                (text.to_string(), vec![])
            }
        };
        let mut head = first.clone();
        let mut seen = HashMap::new();
        for (k, e) in log.iter().enumerate() {
            if e.seq != k as u64 + 1 {
                return Err(std::io::Error::other(format!(
                    "{doc}: entry {} at place {}",
                    e.seq,
                    k + 1
                )));
            }
            head = forge_lang::edit::apply(&head, &e.op).map_err(|err| {
                std::io::Error::other(format!("{doc}: seq {} does not apply: {}", e.seq, err.kind()))
            })?;
            if text_hash(&head) != e.th {
                return Err(std::io::Error::other(format!(
                    "{doc}: seq {} text hash differs",
                    e.seq
                )));
            }
            seen.insert((e.client.clone(), e.id), e.seq);
        }
        Ok(Sequencer {
            doc: doc.into(),
            dir,
            first,
            head,
            log,
            seen,
            root: None,
            limits: Limits::default(),
            checker: Box::new(Direct),
        })
    }

    pub fn seq(&self) -> u64 {
        self.log.len() as u64
    }
    pub fn head(&self) -> &str {
        &self.head
    }
    /// The entries after `seq` (a reconnecting client's catch-up).
    pub fn since(&self, seq: u64) -> &[Entry] {
        &self.log[(seq as usize).min(self.log.len())..]
    }
    pub fn entry(&self, seq: u64) -> Option<&Entry> {
        self.log.get((seq as usize).checked_sub(1)?)
    }
    /// The document's text at `seq` (replayed from the first text).
    pub fn text_at(&self, seq: u64) -> Option<String> {
        let mut t = self.first.clone();
        for e in self.log.get(..seq as usize)? {
            t = forge_lang::edit::apply(&t, &e.op).ok()?;
        }
        Some(t)
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
        if let Some(&seq) = self.seen.get(&(client.to_string(), id)) {
            return Ok(Commit {
                entry: self.log[seq as usize - 1].clone(),
                duplicate: true,
            });
        }
        check_limits(op)?;
        let head = self.seq();
        if base > head {
            return Err(Refusal::BadBase { base, head });
        }
        let since = &self.log[base as usize..];
        let op = rebase(op, client, since)?;
        // a span op is valid only on its base: rebase refused it if another author's op landed;
        // the author's own ops since base moved its bytes too unless there are none
        if span_op(&op) && !since.is_empty() {
            return Err(Refusal::StaleSpan);
        }
        if self.log.len() >= self.limits.max_log {
            return Err(Refusal::Invalid {
                why: format!(
                    "this document's log is full ({} ops): it needs compaction (phase 2)",
                    self.limits.max_log
                ),
            });
        }
        let text = self
            .checker
            .check(&self.head, &op, self.dir.clone(), self.root.clone())?;
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
        self.seen.insert((client.into(), id), entry.seq);
        self.log.push(entry.clone());
        Ok(Commit {
            entry,
            duplicate: false,
        })
    }
}
