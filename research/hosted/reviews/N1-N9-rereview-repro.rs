//! re-review scratch (not committed)
use forge_lang::error::{Literal, Op, ParamKey};
use forge_lang::syntax::Span;
use forge_sync::room::Room;
use forge_sync::wire::{Down, Up};
use forge_sync::{Client, MemStore, Sequencer, Refusal};

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

fn set(p: &str, v: f64) -> Op {
    Op::SetParam { param: ParamKey::parse(p).unwrap(), value: Literal { value: v, dim: forge_lang::values::Dim::Length } }
}

/// 470e7e90: a snapshot Welcome rewrites a pending op's base to the snapshot seq, so the server
/// no longer rebases it through the ops between its real base and the snapshot.
fn rename_then_readd(send_real_base: bool) -> String {
    let mut st = MemStore::default();
    let seq = Sequencer::open(&mut st, "part.fg", DOC, None).unwrap();
    let mut room = Room::new(seq);
    let mut ada = Client::new("part.fg", "ada-c", "t");
    ada.outbox.clear();
    for (_, d) in room.join(0, "ada-c", "ada", None) { ada.receive(d); }
    ada.edit(set("w", 41.0)).unwrap();
    ada.outbox.clear(); // lost with the link
    ada.reconnect();
    ada.outbox.clear();
    // bob: renames w, adds a new w, then > CATCH_UP small edits (no compaction)
    let s = &mut room.seq;
    let mut n = 0;
    let mut sub = |s: &mut Sequencer, st: &mut MemStore, op: &Op| { n += 1; s.submit(st, "bob", "bob/b", s.seq(), n, op).unwrap(); };
    sub(s, &mut st, &Op::Rename { from: "w".into(), to: "width".into() });
    sub(s, &mut st, &Op::AddFeature { before: Some("post".into()), text: "shared w = 5".into() });
    for k in 0..1100 { sub(s, &mut st, &set("h", 12.0 + (k % 2) as f64)); }
    let out = room.join(0, "ada-c", "ada", Some(0));
    assert!(matches!(out[0].1, Down::Welcome { snap: true, .. }), "a snapshot welcome");
    for (_, d) in out { ada.receive(d); }
    let ups: Vec<Up> = ada.outbox.drain(..).collect();
    for u in ups {
        let u = match (u, send_real_base) { (Up::Op { id, op, .. }, true) => Up::Op { base: 0, id, op }, (u, _) => u };
        let (out, _) = room.receive(&mut st, 0, u);
        eprintln!("answer: {:?}", out.iter().map(|x| &x.1).collect::<Vec<_>>());
    }
    room.seq.head().to_string()
}

#[test]
fn snapshot_welcome_base_rewrite_hits_the_new_name() {
    let tip = rename_then_readd(false);
    let real = rename_then_readd(true);
    eprintln!("TIP (base rewritten):\n{tip}\nREAL BASE:\n{real}");
    assert!(tip.contains("shared w = 41"), "tip: the edit lands on bob's new w");
    assert!(tip.contains("shared width = 40"));
    assert!(real.contains("shared width = 41"), "with its real base the server rebases it through the rename");
}

/// A Resync's snapshot Welcome resends an op already in flight on the same connection; its
/// first copy is refused (StaleSpan), the resend (base rewritten to the snapshot) is re-checked
/// as a refused id and commits, after the client was told it was refused.
#[test]
fn refused_then_committed_after_resync_welcome() {
    let mut st = MemStore::default();
    let seq = Sequencer::open(&mut st, "part.fg", DOC, None).unwrap();
    let mut room = Room::new(seq);
    let mut ada = Client::new("part.fg", "ada-c", "t");
    ada.outbox.clear();
    for (_, d) in room.join(0, "ada-c", "ada", None) { ada.receive(d); }
    // bob's op lands (ada has not seen it yet)
    room.seq.submit(&mut st, "bob", "bob/b", 0, 1, &set("h", 13.0)).unwrap();
    // ada's Resync reaches the server first and is answered at once
    let (welcome, _) = room.receive(&mut st, 0, Up::Resync);
    // then ada makes a span op on what she sees (base 0) and sends it
    let at = DOC.find("extrude(plan, h)").unwrap() + "extrude(plan, ".len();
    let id = ada.edit(Op::Respell { span: Span { lo: at as u32, hi: at as u32 + 1 }, to: "w".into() }).unwrap();
    let first: Vec<Up> = ada.outbox.drain(..).collect();
    assert_eq!(first.len(), 1);
    let (refused, _) = room.receive(&mut st, 0, first.into_iter().next().unwrap());
    assert!(matches!(refused[0].1, Down::Refused { refusal: Refusal::StaleSpan, .. }), "{:?}", refused[0].1);
    // ada reads: the Welcome, then the Refused
    for (_, d) in welcome { ada.receive(d); }
    let resend: Vec<Up> = ada.outbox.drain(..).collect();
    eprintln!("resend: {resend:?}");
    let mut told_refused = false;
    for (_, d) in refused { for e in ada.receive(d) { if matches!(e, forge_sync::Event::Refused { .. }) { told_refused = true; } } }
    assert!(told_refused && ada.pending().is_empty());
    // the resend reaches the server
    let mut acked = false;
    for u in resend {
        let (out, _) = room.receive(&mut st, 0, u);
        for (_, d) in out { if matches!(d, Down::Ack { id: k, .. } if k == id) { acked = true; } ada.receive(d); }
    }
    eprintln!("server seq {} head has respell: {}; ada seq {}", room.seq.seq(), room.seq.head().contains("extrude(plan, w)"), ada.seq());
    assert!(acked, "the op the user was told was refused was committed");
    assert!(room.seq.head().contains("extrude(plan, w)"));
    assert_ne!(ada.text(), room.seq.head(), "and ada does not have it");
}
