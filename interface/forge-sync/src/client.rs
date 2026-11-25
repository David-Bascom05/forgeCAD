//! The client: the same state machine on the desktop and in the browser (DESIGN.md §14).
//!
//! It keeps the server's text at the last seq it applied (`confirmed`) and the ops it sent that
//! are not yet acked (`pending`). What the user sees is `confirmed` with the pending ops replayed
//! on it, each rebased exactly as the server will rebase it (the same [`rebase`] function), so an
//! edit shows at once (budget B1: no network on the edit path) and every client converges on the
//! server's text once its ops are acked or refused.

use crate::rebase::rebase_group;
use crate::wire::{Down, Up};
use crate::{Entry, Refusal, apply_all, text_hash};
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
    /// The server closed the connection with this; the host reconnects.
    Closed(String),
    /// The server will not serve this client as it is (a bad token, another kernel, a
    /// quarantined document): the host stops reconnecting and stops syncing, and says why.
    Stopped(String),
    /// Something the server said about the last message (the connection stays).
    Said(String),
}

/// A group sent and not yet answered (most are one op).
#[derive(Clone, Debug)]
pub struct Pending {
    pub id: u64,
    pub base: u64,
    pub ops: Vec<Op>,
    /// An undo (its Ack's inverse is a redo, not another step to undo).
    pub undo: bool,
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
    /// Own committed groups, newest last: (seq, the inverse group; None: it has none, an undo
    /// barrier, said when reached, never skipped to older history).
    undo: Vec<(u64, Option<Vec<Op>>)>,
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

    /// What must outlive the app: this client's id, its next op id and its pending ops, as one
    /// JSON text (DESIGN.md §14: `.forgecad/sync/<doc>.pending`). Restored, the same ids are
    /// sent again, so the server answers one it already committed by (client, op id) and never
    /// applies it twice.
    pub fn saved(&self) -> String {
        use crate::wire::write;
        use forge_agent::json::Json;
        let ops = self
            .pending
            .iter()
            .map(|p| {
                Json::Str(
                    Up::Op {
                        base: p.base,
                        id: p.id,
                        ops: p.ops.clone(),
                    }
                    .text(),
                )
            })
            .collect();
        write(&Json::obj(vec![
            ("client", self.id.as_str().into()),
            ("next", (self.next_id as f64).into()),
            ("ops", Json::Arr(ops)),
        ]))
    }

    /// A client for `doc` that takes up a [`saved`](Self::saved) one: its id, its op ids and its
    /// pending ops (sent on its first Welcome). None: not a saved client's text.
    pub fn restore(doc: &str, token: &str, saved: &str) -> Option<Client> {
        use crate::wire::read;
        use forge_agent::json::Json;
        let j = read(saved).ok()?;
        let id = j.get("client")?.str()?;
        let next = j.get("next")?.num()? as u64;
        let mut c = Client::new(doc, id, token);
        if let Some(Json::Arr(ops)) = j.get("ops") {
            for o in ops {
                match Up::parse(o.str()?) {
                    Ok(Up::Op { base, id, ops }) if id < next => c.pending.push(Pending {
                        base,
                        id,
                        ops,
                        undo: false,
                    }),
                    _ => return None,
                }
            }
        }
        c.next_id = next.max(1);
        Some(c)
    }

    /// After a reconnect: say hello again from the last seq applied; the server replays what
    /// was missed (Acks for this client's own committed ops), and the pending ops are re-sent on
    /// Welcome (the server answers those it already committed by id, never twice).
    pub fn reconnect(&mut self) {
        self.outbox.clear();
        self.welcomed = false;
        self.hello();
    }

    /// Sends a group now if this connection was welcomed (else Welcome sends every pending one).
    fn send_op(&mut self, base: u64, id: u64, ops: &[Op]) {
        if self.welcomed {
            self.outbox.push(Up::Op {
                base,
                id,
                ops: ops.to_vec(),
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
    /// Refused (nothing queued) as [`local_group`](Self::local_group) refuses.
    pub fn local(&mut self, op: Op, after: String) -> Result<u64, Refusal> {
        self.local_group(vec![op], after)
    }

    /// The user's group of ops (an apply-all: several edits that stand or fall together),
    /// already applied in order and its result checked by the host on `text()`, giving `after`.
    /// Sent as one: the server commits all of them or none. Returns its id.
    ///
    /// Refused, nothing queued: before this client's first Welcome (the text the edit was made
    /// on is no version this client knows: an op's base is the seq of the text it was made on,
    /// never a guess, or the server would rebase it through ops it was made after and land it
    /// on another name), and a group outside the protocol's limits (its count, each op's, its
    /// message's bytes: sent, the server would refuse it, or close the connection on it).
    pub fn local_group(&mut self, ops: Vec<Op>, after: String) -> Result<u64, Refusal> {
        if self.joining {
            return Err(Refusal::Invalid {
                why: "not joined yet: an edit waits for the server's text".into(),
            });
        }
        crate::check_group(&ops)?;
        let id = self.next_id;
        let m = Up::Op {
            base: self.seq,
            id,
            ops,
        };
        let bytes = m.text().len();
        if bytes > crate::MAX_UP {
            return Err(Refusal::Invalid {
                why: format!("an edit of {bytes} bytes (at most {})", crate::MAX_UP),
            });
        }
        let Up::Op { base, id, ops } = m else {
            unreachable!()
        };
        self.next_id += 1;
        self.send_op(base, id, &ops);
        self.pending.push(Pending {
            id,
            base,
            ops,
            undo: false,
        });
        self.view = after;
        Ok(id)
    }

    /// Applies and checks `op` on `text()` itself, then [`local`](Self::local). Refused: why,
    /// nothing sent.
    pub fn edit(&mut self, op: Op) -> Result<u64, Refusal> {
        let (after, _) = forge_lang::edit::apply_checked(&self.view, &op).map_err(lang)?;
        self.local(op, after)
    }

    /// Applies `ops` in order on `text()` and checks the result (only the result: a group may
    /// pass through an invalid text), then [`local_group`](Self::local_group). Refused: why,
    /// nothing sent.
    pub fn edit_group(&mut self, ops: Vec<Op>) -> Result<u64, Refusal> {
        crate::check_group(&ops)?;
        let (after, _) = crate::validate(&self.view, None, &ops, None, None)?;
        self.local_group(ops, after)
    }

    /// Undo this client's last committed group that has an inverse: the inverse group, made on
    /// the text at that group's seq, rebased by the server like any group (so collaborators'
    /// later edits stay; a group's inverse is undone as one too).
    /// None: nothing to undo. The inverse is applied locally at once (Err if it no longer applies
    /// here; it is then not sent).
    pub fn undo(&mut self) -> Option<Result<u64, Refusal>> {
        // the user's latest edit is still on its way: undoing now would take back the op before
        // it (the undo list holds only committed ops), so it waits for the Ack
        if !self.pending.is_empty() && !self.undo.is_empty() {
            return Some(Err(Refusal::Invalid {
                why: "an edit is still on its way to the server: undo once it has landed".into(),
            }));
        }
        let (seq, inv) = self.undo.pop()?;
        let Some(inv) = inv else {
            // an edit with no exact inverse: undo stops here, and says so (going on to the one
            // before would take back an older edit while this one stays)
            self.undo.push((seq, None));
            return Some(Err(Refusal::Invalid {
                why: "the last edit cannot be undone here (it has no exact inverse): make the change by hand"
                    .into(),
            }));
        };
        // the inverse as it reads now: through others' renames since that seq
        let ops = match rebase_group(&inv, &self.id, self.recent.iter().filter(|e| e.seq > seq)) {
            Ok(ops) => ops,
            Err(r) => return Some(Err(r)),
        };
        let checked = match &ops[..] {
            [op] => forge_lang::edit::apply_checked(&self.view, op)
                .map(|(after, _)| after)
                .map_err(|e| Refusal::Lang {
                    kind: e.kind().into(),
                    text: e.text(),
                }),
            _ => crate::validate(&self.view, None, &ops, None, None).map(|(after, _)| after),
        };
        let after = match checked {
            Ok(a) => a,
            Err(r) => {
                // it does not apply here now: the step stays for a later try
                self.undo.push((seq, Some(inv)));
                return Some(Err(r));
            }
        };
        let id = self.next_id;
        self.next_id += 1;
        self.send_op(seq.max(self.seq), id, &ops);
        self.pending.push(Pending {
            id,
            base: seq.max(self.seq),
            ops,
            undo: true,
        });
        self.view = after;
        Some(Ok(id))
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
                    // (each pending op keeps its own base: the server rebases it from there,
                    // reading older ops from its history; sent as if made on this text, a Rename
                    // in between would be skipped and the edit land on another name)
                    self.replay();
                    ev.push(Event::Changed);
                }
                // a connection's first Welcome sends every pending op again, in order (DESIGN.md
                // §4.2): the server answers one it committed with its Ack again or
                // AlreadyCommitted, by (client, op id). A Resync's Welcome on a live connection
                // sends nothing: each op sent before the Resync was answered before it (one
                // connection is in order), and a resend of one refused could commit after the
                // user was told it was refused.
                if !self.welcomed {
                    self.welcomed = true;
                    for p in &self.pending {
                        self.outbox.push(Up::Op {
                            base: p.base,
                            id: p.id,
                            ops: p.ops.clone(),
                        });
                    }
                }
            }
            Down::Ack {
                id,
                seq,
                ops,
                inv,
                th,
            } => {
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
                    // committed, at a seq this client cannot place: it is no longer pending (the
                    // snapshot the Resync brings holds it)
                    self.pending.remove(k);
                    return self.diverged(seq);
                }
                let p = self.pending.remove(k);
                let e = Entry {
                    seq,
                    author: self.you.clone(),
                    client: self.id.clone(),
                    id,
                    base: p.base,
                    ops,
                    inv: inv.clone(),
                    th,
                };
                if !self.commit(e) {
                    return self.diverged(seq);
                }
                // an undo's inverse is a redo: not a step to undo (a second Undo would redo)
                if !p.undo {
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
            Down::Applied { seq, author, ops, th } => {
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
                    ops,
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
            Down::Error { text } => ev.push(Event::Said(text)),
            Down::Closing { text, retry: true } => ev.push(Event::Closed(text)),
            Down::Closing { text, retry: false } => ev.push(Event::Stopped(text)),
        }
        ev
    }

    /// Applies a committed entry to the confirmed text; false if it does not apply or the text's
    /// hash differs from the server's.
    fn commit(&mut self, e: Entry) -> bool {
        let Ok(t) = apply_all(&self.confirmed, &e.ops) else {
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

    /// The view: the confirmed text with every pending group rebased as the server will and
    /// applied; one that no longer applies is skipped whole (the server will refuse it whole).
    fn replay(&mut self) {
        let mut t = self.confirmed.clone();
        for p in &self.pending {
            let since = self.recent.iter().filter(|e| e.seq > p.base);
            if let Ok(ops) = rebase_group(&p.ops, &self.id, since)
                && let Ok(x) = apply_all(&t, &ops)
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

fn lang(e: forge_lang::error::Error) -> Refusal {
    Refusal::Lang {
        kind: e.kind().into(),
        text: e.text(),
    }
}
