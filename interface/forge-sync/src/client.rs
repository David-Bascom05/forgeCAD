//! The client: the same state machine on the desktop and in the browser (DESIGN.md §14).
//!
//! It keeps the server's text at the last seq it applied (`confirmed`) and the ops it sent that
//! are not yet acked (`pending`). What the user sees is `confirmed` with the pending ops replayed
//! on it, each rebased exactly as the server will rebase it (the same [`rebase`] function), so an
//! edit shows at once (budget B1: no network on the edit path) and every client converges on the
//! server's text once its ops are acked or refused.

use crate::rebase::rebase;
use crate::wire::{Down, Up};
use crate::{Entry, Refusal, text_hash};
use forge_lang::edit::apply;
use forge_lang::error::Op;

/// What the host should do after a message (besides sending the client's `outbox`).
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The view's text changed (another author's op, a refusal undoing an optimistic op, a
    /// snapshot): show and rebuild `text()`.
    Changed,
    /// The client's op `id` was refused: tell the user why.
    Refused {
        id: u64,
        refusal: Refusal,
    },
    /// The server's text hash differs from this client's after `seq`: the client asked for a
    /// snapshot (a bug: logged, never silent).
    TextDiverged {
        seq: u64,
    },
    /// A build hash disagreed (reported by the server).
    BuildDiverged {
        seq: u64,
        expected: u64,
        got: u64,
        by: String,
    },
    Versioned {
        name: String,
        seq: u64,
        commit: String,
    },
    /// A round trip measured (ms).
    Rtt(f64),
    /// The server closed the session with this.
    Closed(String),
}

#[derive(Clone, Debug)]
pub struct Pending {
    pub id: u64,
    pub base: u64,
    pub op: Op,
}

pub struct Client {
    pub doc: String,
    /// This client's stable id (ops are deduplicated by it across reconnects).
    pub id: String,
    token: String,
    /// The author name the server commits this client's ops under (from Welcome).
    pub you: String,
    confirmed: String,
    seq: u64,
    pending: Vec<Pending>,
    /// Entries applied since the oldest pending op's base (the rebase of pending ops reads them).
    recent: Vec<Entry>,
    view: String,
    next_id: u64,
    /// Own committed ops with an inverse, newest last: (seq, inverse).
    undo: Vec<(u64, Op)>,
    /// Messages for the server, in order; the host drains and sends them.
    pub outbox: Vec<Up>,
    /// True until the first Welcome.
    pub joining: bool,
    /// True once this connection's Welcome came: ops wait for it, then all pending ops are sent
    /// (an op sent before the server had placed this connection could be lost with it).
    welcomed: bool,
}

impl Client {
    /// A client for `doc`; it sends Hello at once (in `outbox`).
    pub fn new(doc: &str, id: &str, token: &str) -> Client {
        let mut c = Client {
            doc: doc.into(),
            id: id.into(),
            token: token.into(),
            you: String::new(),
            confirmed: String::new(),
            seq: 0,
            pending: vec![],
            recent: vec![],
            view: String::new(),
            next_id: 1,
            undo: vec![],
            outbox: vec![],
            joining: true,
            welcomed: false,
        };
        c.hello();
        c
    }

    /// After a reconnect: say hello again from the last seq applied; the server replays what
    /// was missed (Acks for this client's own committed ops), and the pending ops are re-sent on
    /// Welcome (the server answers those it already committed by id, never twice).
    pub fn reconnect(&mut self) {
        self.outbox.clear();
        self.welcomed = false;
        self.hello();
    }

    /// Sends an op now if this connection was welcomed (else Welcome sends every pending op).
    fn send_op(&mut self, base: u64, id: u64, op: &Op) {
        if self.welcomed {
            self.outbox.push(Up::Op {
                base,
                id,
                op: op.clone(),
            });
        }
    }

    fn hello(&mut self) {
        self.outbox.push(Up::Hello {
            doc: self.doc.clone(),
            client: self.id.clone(),
            token: self.token.clone(),
            seq: (!self.joining).then_some(self.seq),
            proto: crate::PROTOCOL,
            kernel: crate::KERNEL.into(),
        });
    }

    /// The text the user sees: the server's, with this client's pending ops on it.
    pub fn text(&self) -> &str {
        &self.view
    }
    /// The last seq applied from the server.
    pub fn seq(&self) -> u64 {
        self.seq
    }
    pub fn confirmed(&self) -> &str {
        &self.confirmed
    }
    pub fn pending(&self) -> &[Pending] {
        &self.pending
    }

    /// The user's op, already applied and checked by the host on `text()` (the host's edit
    /// path: `apply_checked` succeeded and gave `after`). Queued and sent; returns its id.
    pub fn local(&mut self, op: Op, after: String) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.send_op(self.seq, id, &op);
        self.pending.push(Pending {
            id,
            base: self.seq,
            op,
        });
        self.view = after;
        id
    }

    /// Applies and checks `op` on `text()` itself, then [`local`](Self::local). Refused: the
    /// typed error, nothing sent.
    pub fn edit(&mut self, op: Op) -> Result<u64, forge_lang::error::Error> {
        let (after, _) = forge_lang::edit::apply_checked(&self.view, &op)?;
        Ok(self.local(op, after))
    }

    /// Undo this client's last committed op that has an inverse: the inverse, made on the text
    /// at that op's seq, rebased by the server like any op (so collaborators' later edits stay).
    /// None: nothing to undo. The inverse is applied locally at once (Err if it no longer applies
    /// here; it is then not sent).
    pub fn undo(&mut self) -> Option<Result<u64, Refusal>> {
        let (seq, inv) = self.undo.pop()?;
        // the inverse as it reads now: through others' renames since that seq
        let op = match rebase(&inv, &self.id, self.recent.iter().filter(|e| e.seq > seq)) {
            Ok(op) => op,
            Err(r) => return Some(Err(r)),
        };
        let checked = forge_lang::edit::apply_checked(&self.view, &op).map_err(|e| Refusal::Lang {
            kind: e.kind().into(),
            text: e.text(),
        });
        Some(checked.map(|(after, _)| {
            let id = self.next_id;
            self.next_id += 1;
            self.send_op(seq.max(self.seq), id, &op);
            self.pending.push(Pending {
                id,
                base: seq.max(self.seq),
                op,
            });
            self.view = after;
            id
        }))
    }

    /// Records the hash of a body built from the text at `seq` (sent to the server).
    pub fn built(&mut self, seq: u64, hash: u64) {
        self.outbox.push(Up::BuildHash { seq, hash });
    }

    /// One message from the server.
    pub fn receive(&mut self, m: Down) -> Vec<Event> {
        let mut ev = vec![];
        match m {
            Down::Welcome { seq, text, you, snap } => {
                self.you = you;
                // a catch-up: the entries follow
                self.joining = false;
                if snap {
                    // a snapshot: the head as it is (pending ops, if any, replay on it)
                    self.joining = false;
                    self.confirmed = text;
                    self.seq = seq;
                    self.recent.clear();
                    // the pending ops now stand on this text (they replay on it here): sent from
                    // it, they cannot fall behind the server's latest snapshot
                    for p in &mut self.pending {
                        p.base = seq;
                    }
                    self.replay();
                    ev.push(Event::Changed);
                }
                // every Welcome (a join, a reconnect, a Resync's snapshot) sends every pending op
                // again, in order: the server answers one it committed with its Ack again or
                // AlreadyCommitted (by client and op id), so none is lost and none is held
                // forever (DESIGN.md §4.2)
                {
                    self.welcomed = true;
                    for p in &self.pending {
                        self.outbox.push(Up::Op {
                            base: p.base,
                            id: p.id,
                            op: p.op.clone(),
                        });
                    }
                }
            }
            Down::Ack { id, seq, op, inv, th } => {
                let Some(k) = self.pending.iter().position(|p| p.id == id) else {
                    return ev;
                };
                if seq <= self.seq {
                    // already in the snapshot this client started over from
                    self.pending.remove(k);
                    self.replay();
                    ev.push(Event::Changed);
                    return ev;
                }
                if seq != self.seq + 1 {
                    return self.diverged(seq);
                }
                let p = self.pending.remove(k);
                let e = Entry {
                    seq,
                    author: self.you.clone(),
                    client: self.id.clone(),
                    id,
                    base: p.base,
                    op,
                    inv: inv.clone(),
                    th,
                };
                if !self.commit(e) {
                    return self.diverged(seq);
                }
                if let Some(inv) = inv {
                    self.undo.push((seq, inv));
                }
                // only the client's own op landed: the view is unchanged unless the server's
                // rebase of it differed from the local one
                let before = self.view.clone();
                self.replay();
                if self.view != before {
                    ev.push(Event::Changed);
                }
            }
            Down::Refused { id, refusal } => {
                if let Some(k) = self.pending.iter().position(|p| p.id == id) {
                    self.pending.remove(k);
                    self.replay();
                    ev.push(Event::Changed);
                }
                // a retry of an op the snapshot already holds: nothing to tell
                if !matches!(refusal, Refusal::AlreadyCommitted { .. }) {
                    ev.push(Event::Refused { id, refusal });
                }
            }
            Down::Applied { seq, author, op, th } => {
                if seq <= self.seq {
                    return ev;
                }
                if seq != self.seq + 1 {
                    return self.diverged(seq);
                }
                let e = Entry {
                    seq,
                    author: author.clone(),
                    client: format!("other:{author}"),
                    id: 0,
                    base: 0,
                    op,
                    inv: None,
                    th,
                };
                if !self.commit(e) {
                    return self.diverged(seq);
                }
                self.replay();
                ev.push(Event::Changed);
            }
            Down::Diverged {
                seq,
                expected,
                got,
                by,
            } => ev.push(Event::BuildDiverged {
                seq,
                expected,
                got,
                by,
            }),
            Down::Versioned { name, seq, commit } => ev.push(Event::Versioned { name, seq, commit }),
            Down::Pong { t } => ev.push(Event::Rtt(t)),
            Down::Error { text } => ev.push(Event::Closed(text)),
        }
        ev
    }

    /// Applies a committed entry to the confirmed text; false if it does not apply or the text's
    /// hash differs from the server's.
    fn commit(&mut self, e: Entry) -> bool {
        let Ok(t) = apply(&self.confirmed, &e.op) else {
            return false;
        };
        if text_hash(&t) != e.th {
            return false;
        }
        self.confirmed = t;
        self.seq = e.seq;
        self.recent.push(e);
        // keep only what a pending op's rebase can still read
        match self.pending.iter().map(|p| p.base).min() {
            Some(b) => self.recent.retain(|x| x.seq > b),
            // undo rebases through entries after its op's seq: keep a bounded tail
            None => {
                let keep = 1024;
                if self.recent.len() > keep {
                    self.recent.drain(..self.recent.len() - keep);
                }
            }
        }
        true
    }

    /// The view: the confirmed text with every pending op rebased as the server will and applied;
    /// one that no longer applies is skipped (the server will refuse it).
    fn replay(&mut self) {
        let mut t = self.confirmed.clone();
        for p in &self.pending {
            let since = self.recent.iter().filter(|e| e.seq > p.base);
            if let Ok(op) = rebase(&p.op, &self.id, since)
                && let Ok(x) = apply(&t, &op)
            {
                t = x;
            }
        }
        self.view = t;
    }

    /// The text differs from the server's: ask for a snapshot on this connection (pending ops
    /// replay on it; the server drops retries of the ones it committed).
    fn diverged(&mut self, seq: u64) -> Vec<Event> {
        self.outbox.push(Up::Resync);
        vec![Event::TextDiverged { seq }]
    }
}
