//! Convergence under every interleaving we can generate: several clients edit one document at
//! once (SetParam, Rename races, Delete vs Replace, Moves, Adds, undo), their messages delivered
//! in a random order (each connection in order, as a WebSocket is), and at the end every client's
//! text, the server's head, and every build's hash are identical.

use forge_lang::error::{Literal, Op};
use forge_sync::room::{Conn, Room};
use forge_sync::wire::{Down, Up};
use forge_sync::{Client, MemStore, Sequencer, body_hash};
use std::collections::VecDeque;

const DOC: &str = "\
shared w = 40
shared h = 12
sketch plan(on: xy) {
  o = rect(center: (0, 0), size: (w, 30))
}
base = extrude(plan, h)
sketch ring(on: base.end) {
  c = circle((0, 0), 6)
}
param boss.d = 8
boss = extrude(ring, d)
sketch tab(on: xy) {
  o = rect(center: (40, 0), size: (8, 8))
}
param lid.t = 2
lid = extrude(tab, t, new: true)
post = extrude(tab, 9, new: true)
";

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// Ops a user could make on `text` (valid or not: the client checks them).
fn candidates(text: &str, rng: &mut Rng, tag: &str) -> Vec<Op> {
    let mut ops = vec![];
    for (k, lit) in forge_lang::edit::handles(text) {
        let f = 0.5 + (rng.below(100) as f64) / 50.0;
        ops.push(Op::SetParam {
            param: k,
            value: Literal {
                value: lit.value * f + 1.0,
                dim: lit.dim,
            },
        });
    }
    let o = forge_lang::Options {
        build: false,
        ..Default::default()
    };
    let (run, _) = forge_lang::run(text, o, Default::default());
    for f in run.feats.iter().filter(|f| f.top_level) {
        let id = f.id.clone();
        ops.push(Op::Rename {
            from: id.clone(),
            to: format!("{id}_{tag}"),
        });
        // deletes rarer than the rest, so the document stays worth editing
        if rng.below(4) == 0 {
            ops.push(Op::DeleteFeature { id: id.clone() });
        }
        ops.push(Op::MoveFeature {
            id: id.clone(),
            before: None,
        });
        ops.push(Op::ReplaceFeature {
            id: id.clone(),
            text: format!("{id} = extrude(tab, {}, new: true)", 1 + rng.below(9)),
        });
    }
    ops.push(Op::AddFeature {
        before: None,
        text: format!("extra_{tag} = extrude(plan, 2, new: true)"),
    });
    ops.push(Op::AddFeature {
        before: Some("post".into()),
        text: format!("shared s_{tag} = 5"),
    });
    ops
}

struct Sim {
    room: Room,
    store: MemStore,
    clients: Vec<Client>,
    up: Vec<VecDeque<Up>>,
    down: Vec<VecDeque<Down>>,
    joined: Vec<bool>,
    refused: usize,
}

impl Sim {
    fn new(n: usize) -> Sim {
        let mut store = MemStore::default();
        let seq = Sequencer::open(&mut store, "part.fg", DOC, None).unwrap();
        let clients: Vec<Client> = (0..n)
            .map(|k| Client::new("part.fg", &format!("c{k}"), "dev"))
            .collect();
        Sim {
            room: Room::new(seq),
            store,
            clients,
            up: vec![VecDeque::new(); n],
            down: vec![VecDeque::new(); n],
            joined: vec![false; n],
            refused: 0,
        }
    }
    fn route(&mut self, out: Vec<(Conn, Down)>) {
        for (c, d) in out {
            // through the wire's text form, as a socket carries it
            let d = Down::parse(&d.text()).unwrap();
            self.down[c as usize].push_back(d);
        }
    }
    fn flush(&mut self, k: usize) {
        for m in self.clients[k].outbox.drain(..) {
            self.up[k].push_back(Up::parse(&m.text()).unwrap());
        }
    }
    fn deliver_up(&mut self, k: usize) {
        let Some(m) = self.up[k].pop_front() else { return };
        let out = match m {
            Up::Hello { client, seq, .. } => {
                self.joined[k] = true;
                self.room.join(k as Conn, &client, &format!("user{k}"), seq)
            }
            m => {
                let (out, _) = self.room.receive(&mut self.store, k as Conn, m);
                out
            }
        };
        self.route(out);
    }
    fn deliver_down(&mut self, k: usize) {
        let Some(m) = self.down[k].pop_front() else { return };
        for e in self.clients[k].receive(m) {
            match e {
                forge_sync::Event::TextDiverged { seq } => panic!("client {k} text diverged at {seq}"),
                forge_sync::Event::Refused { .. } => self.refused += 1,
                _ => {}
            }
        }
        self.flush(k);
    }
    fn quiet(&self) -> bool {
        self.up.iter().all(VecDeque::is_empty) && self.down.iter().all(VecDeque::is_empty)
    }
    fn drain(&mut self, rng: &mut Rng) {
        while !self.quiet() {
            let k = rng.below(self.clients.len());
            if rng.below(2) == 0 {
                self.deliver_up(k)
            } else {
                self.deliver_down(k)
            }
        }
    }
}

fn run(seed: u64, n: usize, steps: usize) -> (usize, usize, usize) {
    let mut rng = Rng(seed);
    let mut sim = Sim::new(n);
    for k in 0..n {
        sim.flush(k);
    }
    let (mut edits, mut drops) = (0, 0);
    for step in 0..steps {
        let k = rng.below(n);
        match rng.below(10) {
            0..=2 if !sim.clients[k].joining => {
                let tag = format!("{k}x{step}");
                let view = sim.clients[k].text().to_string();
                let ops = candidates(&view, &mut rng, &tag);
                let op = ops[rng.below(ops.len())].clone();
                if sim.clients[k].edit(op).is_ok() {
                    edits += 1;
                }
                sim.flush(k);
            }
            3 if !sim.clients[k].joining => {
                if let Some(Ok(_)) = sim.clients[k].undo() {
                    edits += 1;
                }
                sim.flush(k);
            }
            4 if rng.below(8) == 0 => {
                // the connection drops: whatever was in flight either way is lost
                sim.room.leave(k as Conn);
                sim.up[k].clear();
                sim.down[k].clear();
                sim.clients[k].reconnect();
                sim.flush(k);
                drops += 1;
            }
            4..=6 => sim.deliver_up(k),
            _ => sim.deliver_down(k),
        }
    }
    sim.drain(&mut rng);
    let head = sim.room.seq.head().to_string();
    let o = forge_lang::Options::default();
    let (run, _) = forge_lang::run(&head, o.clone(), Default::default());
    let want = body_hash(&run);
    for (k, c) in sim.clients.iter().enumerate() {
        assert!(
            c.pending().is_empty(),
            "seed {seed}: client {k} still has pending ops"
        );
        assert_eq!(c.seq(), sim.room.seq.seq(), "seed {seed}: client {k} seq");
        assert_eq!(c.text(), head, "seed {seed}: client {k} text");
        assert_eq!(c.confirmed(), head, "seed {seed}: client {k} confirmed");
        let (r, _) = forge_lang::run(c.text(), o.clone(), Default::default());
        assert_eq!(body_hash(&r), want, "seed {seed}: client {k} body");
    }
    // the log replays to the head
    assert_eq!(sim.room.seq.text_at(sim.room.seq.seq()).unwrap(), head);
    (edits, sim.refused, drops)
}

#[test]
fn the_document_is_valid() {
    let (run, _) = forge_lang::run(DOC, forge_lang::Options::default(), Default::default());
    assert!(run.errors.is_empty(), "{:?}", run.errors);
    assert!(run.body.is_some());
}

#[test]
fn clients_converge_under_random_interleavings() {
    let (mut edits, mut refused, mut drops) = (0, 0, 0);
    for seed in 1..=40u64 {
        let (e, r, d) = run(
            seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1,
            2 + (seed as usize % 3),
            160,
        );
        edits += e;
        refused += r;
        drops += d;
    }
    assert!(drops > 10, "drops {drops}");
    // the test means something only if both paths ran
    assert!(edits > 200, "edits {edits}");
    assert!(refused > 5, "refused {refused}");
    eprintln!("edits {edits}, refused on the server {refused}, dropped connections {drops}");
}

#[test]
fn rename_race_one_wins_and_both_converge() {
    let mut sim = Sim::new(2);
    let mut rng = Rng(7);
    for k in 0..2 {
        sim.flush(k);
    }
    sim.drain(&mut rng);
    // both rename `boss` at once, and each then edits the renamed feature's param
    sim.clients[0]
        .edit(Op::Rename {
            from: "boss".into(),
            to: "hub".into(),
        })
        .unwrap();
    sim.clients[1]
        .edit(Op::Rename {
            from: "boss".into(),
            to: "pin".into(),
        })
        .unwrap();
    let lit = Literal {
        value: 11.0,
        dim: forge_lang::values::Dim::Length,
    };
    sim.clients[1]
        .edit(Op::SetParam {
            param: forge_lang::error::ParamKey::parse("pin.d").unwrap(),
            value: lit,
        })
        .unwrap();
    // and a third op of client 1 that does not depend on its rename: it must survive
    sim.clients[1]
        .edit(Op::SetParam {
            param: forge_lang::error::ParamKey::parse("lid.t").unwrap(),
            value: lit,
        })
        .unwrap();
    for k in 0..2 {
        sim.flush(k);
    }
    // client 0's rename reaches the server first
    sim.deliver_up(0);
    sim.drain(&mut rng);
    let head = sim.room.seq.head();
    assert!(head.contains("hub = extrude"), "{head}");
    assert!(head.contains("param lid.t = 11"), "{head}");
    assert!(!head.contains("pin"), "{head}");
    assert_eq!(sim.clients[0].text(), head);
    assert_eq!(sim.clients[1].text(), head);
    assert_eq!(sim.refused, 2);
}

#[test]
fn late_ops_follow_a_rename() {
    let mut sim = Sim::new(2);
    let mut rng = Rng(9);
    for k in 0..2 {
        sim.flush(k);
    }
    sim.drain(&mut rng);
    sim.clients[0]
        .edit(Op::Rename {
            from: "boss".into(),
            to: "hub".into(),
        })
        .unwrap();
    let lit = Literal {
        value: 6.5,
        dim: forge_lang::values::Dim::Length,
    };
    // made before client 1 saw the rename: it says `boss.d`
    sim.clients[1]
        .edit(Op::SetParam {
            param: forge_lang::error::ParamKey::parse("boss.d").unwrap(),
            value: lit,
        })
        .unwrap();
    for k in 0..2 {
        sim.flush(k);
    }
    sim.deliver_up(0);
    sim.drain(&mut rng);
    let head = sim.room.seq.head();
    assert!(head.contains("param hub.d = 6.5"), "{head}");
    assert_eq!(sim.clients[1].text(), head);
    assert_eq!(sim.refused, 0);
}

#[test]
fn reconnect_replays_and_dedups() {
    let mut sim = Sim::new(2);
    let mut rng = Rng(11);
    for k in 0..2 {
        sim.flush(k);
    }
    sim.drain(&mut rng);
    let lit = Literal {
        value: 50.0,
        dim: forge_lang::values::Dim::Length,
    };
    sim.clients[0]
        .edit(Op::SetParam {
            param: forge_lang::error::ParamKey::parse("w").unwrap(),
            value: lit,
        })
        .unwrap();
    sim.flush(0);
    sim.deliver_up(0); // committed, but the ack is lost with the connection
    sim.down[0].clear();
    sim.room.leave(0);
    sim.clients[1]
        .edit(Op::Rename {
            from: "post".into(),
            to: "mast".into(),
        })
        .unwrap();
    sim.flush(1);
    sim.deliver_up(1);
    // client 0 reconnects: Hello from its last seq, its pending op re-sent
    sim.clients[0].reconnect();
    sim.flush(0);
    sim.drain(&mut rng);
    let head = sim.room.seq.head();
    assert_eq!(sim.room.seq.seq(), 2, "the retried op was committed once");
    assert_eq!(sim.clients[0].text(), head);
    assert_eq!(sim.clients[1].text(), head);
}

#[test]
fn a_document_cannot_grow_past_its_limits() {
    let mut store = MemStore::default();
    let mut seq = Sequencer::open(&mut store, "part.fg", DOC, None).unwrap();
    seq.limits = forge_sync::Limits {
        max_text: DOC.len() + 60,
        max_log: 2,
    };
    let add = |k: usize| Op::AddFeature {
        before: None,
        text: format!("shared big_{k} = {}", "1".repeat(30)),
    };
    assert!(seq.submit(&mut store, "a", "a/c", 0, 1, &add(1)).is_ok());
    match seq.submit(&mut store, "a", "a/c", 1, 2, &add(2)) {
        Err(forge_sync::Refusal::Invalid { why }) => assert!(why.contains("bytes"), "{why}"),
        x => panic!("{x:?}"),
    }
    seq.limits.max_text = 1 << 20;
    assert!(seq.submit(&mut store, "a", "a/c", 1, 3, &add(3)).is_ok());
    match seq.submit(&mut store, "a", "a/c", 2, 4, &add(4)) {
        Err(forge_sync::Refusal::Invalid { why }) => assert!(why.contains("log is full"), "{why}"),
        x => panic!("{x:?}"),
    }
    assert_eq!(seq.seq(), 2, "nothing refused was written");
}
