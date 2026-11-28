//! The client: the same state machine on the desktop and in the browser (DESIGN.md §14).
//!
//! It keeps the server's text at the last seq it applied (`confirmed`) and the ops it sent that
//! are not yet acked (`pending`). What the user sees is `confirmed` with the pending ops replayed
//! on it, each rebased exactly as the server will rebase it (the same [`rebase`] function), so an
//! edit shows at once (budget B1: no network on the edit path) and every client converges on the
//! server's text once its ops are acked or refused.

use crate::Edit;
use crate::rebase::rebase_edit as rebase;
use crate::wire::{Down, Up};
use crate::{Entry, Refusal, text_hash};
use forge_lang::edit::At;
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

#[derive(Clone, Debug)]
pub struct Pending {
    pub id: u64,
    pub base: u64,
    pub op: Edit,
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
    undo: Vec<(u64, Edit)>,
    /// The inverse in flight. Its Ack consumes the undo entry; a refusal keeps it.
    pending_undo: Option<u64>,
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
            pending_undo: None,
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
                        op: p.op.clone(),
                    }
                    .text(),
                )
            })
            .collect();
        write(&Json::obj(vec![
            ("client", self.id.as_str().into()),
            ("next", (self.next_id as f64).into()),
            ("ops", Json::Arr(ops)),
            (
                "pending_undo",
                self.pending_undo.map_or(Json::Null, |id| (id as f64).into()),
            ),
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
                    Ok(Up::Op { base, id, op }) if id < next => c.pending.push(Pending { base, id, op }),
                    _ => return None,
                }
            }
        }
        c.pending_undo = j.get("pending_undo").and_then(Json::num).map(|n| n as u64);
        if c.pending_undo
            .is_some_and(|id| !c.pending.iter().any(|p| p.id == id))
        {
            return None;
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

    /// Sends an op now if this connection was welcomed (else Welcome sends every pending op).
    fn send_op(&mut self, base: u64, id: u64, op: &Edit) {
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
    pub fn local(&mut self, op: impl Into<Edit>, after: String) -> Result<u64, forge_lang::error::Error> {
        if self.missing_pending_history() {
            return Err(forge_lang::error::Error::StaleEdit);
        }
        let op = op.into();
        let id = self.next_id;
        self.next_id += 1;
        self.send_op(self.seq, id, &op);
        self.pending.push(Pending {
            id,
            base: self.seq,
            op,
        });
        self.view = after;
        Ok(id)
    }

    /// Queue only if the editor still sees the exact revision and optimistic text it checked.
    pub fn local_on(
        &mut self,
        base: u64,
        before: &str,
        op: Edit,
        after: String,
    ) -> Result<u64, forge_lang::error::Error> {
        if self.joining || base != self.seq || before != self.view {
            return Err(forge_lang::error::Error::StaleEdit);
        }
        self.local(op, after)
    }

    /// Applies and checks `op` on `text()` itself, then [`local`](Self::local). Refused: the
    /// typed error, nothing sent.
    pub fn edit(&mut self, op: Op) -> Result<u64, forge_lang::error::Error> {
        let (after, _) = forge_lang::edit::apply_checked(&self.view, &op)?;
        self.local(op, after)
    }

    /// Undo this client's last committed op that has an inverse: the inverse, made on the text
    /// at that op's seq, rebased by the server like any op (so collaborators' later edits stay).
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
        let (seq, inv) = self.undo.last()?.clone();
        // the inverse as it reads now: through others' renames since that seq
        let op = match rebase(&inv, &self.id, self.recent.iter().filter(|e| e.seq > seq)) {
            Ok(op) => op,
            Err(r) => return Some(Err(r)),
        };
        let checked = op.checked(&self.view).map_err(|e| Refusal::Lang {
            kind: e.kind().into(),
            text: e.text(),
        });
        Some(checked.and_then(|after| {
            let id = self.local(op, after).map_err(|e| Refusal::Lang { kind: e.kind().into(), text: e.text() })?;
            self.pending_undo = Some(id);
            Ok(id)
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
                    if seq != self.seq || text != self.confirmed {
                        // The snapshot omits the intervening renames needed by old inverses.
                        self.recent.clear();
                        self.undo.clear();
                    }
                    self.confirmed = text;
                    self.seq = seq;
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
                            op: p.op.clone(),
                        });
                    }
                }
            }
            Down::Ack { id, seq, op, inv, th } => {
                let Some(k) = self.pending.iter().position(|p| p.id == id) else {
                    return ev;
                };
                let was_undo = self.pending_undo == Some(id);
                if was_undo {
                    self.pending_undo = None;
                    self.undo.pop();
                }
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
                    op,
                    inv: inv.clone(),
                    th,
                };
                if !self.commit(e) {
                    return self.diverged(seq);
                }
                if let Some(inv) = inv.filter(|_| !was_undo) {
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
                if self.pending_undo == Some(id) {
                    self.pending_undo = None;
                    if matches!(refusal, Refusal::AlreadyCommitted { .. }) {
                        self.undo.pop();
                    }
                }
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
            Down::Error { text } => ev.push(Event::Said(text)),
            Down::Closing { text, retry: true } => ev.push(Event::Closed(text)),
            Down::Closing { text, retry: false } => ev.push(Event::Stopped(text)),
        }
        ev
    }

    /// Applies a committed entry to the confirmed text; false if it does not apply or the text's
    /// hash differs from the server's.
    fn commit(&mut self, e: Entry) -> bool {
        let Ok(t) = e.op.apply(&self.confirmed, &At::default()) else {
            return false;
        };
        if text_hash(&t) != e.th {
            return false;
        }
        self.confirmed = t;
        self.seq = e.seq;
        self.recent.push(e);
        // Keep the undo window as well as any older history needed by a pending edit.
        // A discarded rename can make an old inverse target a newly reused name.
        let floor = self
            .pending
            .iter()
            .map(|p| p.base)
            .min()
            .unwrap_or(self.seq)
            .min(self.seq.saturating_sub(1024));
        let expired = self.recent.partition_point(|entry| entry.seq <= floor);
        self.recent.drain(..expired);
        let expired = self.undo.partition_point(|(seq, _)| *seq < floor);
        self.undo.drain(..expired);
        true
    }

    /// A fresh snapshot may omit renames between a restored op's base and the
    /// confirmed text. Keep the confirmed view until the server settles those ops.
    fn missing_pending_history(&self) -> bool {
        let floor = self.recent.first().map_or(self.seq, |entry| entry.seq - 1);
        self.pending.iter().any(|pending| pending.base < floor)
    }

    /// The view: the confirmed text with every pending op rebased as the server will and applied;
    /// one that no longer applies is skipped (the server will refuse it).
    fn replay(&mut self) {
        if self.missing_pending_history() {
            self.view.clone_from(&self.confirmed);
            return;
        }
        let mut t = self.confirmed.clone();
        for p in &self.pending {
            let since = self.recent.iter().filter(|e| e.seq > p.base);
            if let Ok(op) = rebase(&p.op, &self.id, since)
                && let Ok(x) = op.apply(&t, &At::default())
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
