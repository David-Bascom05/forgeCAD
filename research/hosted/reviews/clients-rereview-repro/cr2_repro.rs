//! clients re-review scratch (not committed)
use forge_lang::error::{Literal, Op, ParamKey};
use forge_sync::room::Room;
use forge_sync::wire::{Down, Up};
use forge_sync::{Client, Event, MemStore, Sequencer};

const DOC: &str = "\
shared w = 40
shared h = 12
sketch plan(on: xy) {
  o = rect(center: (0, 0), size: (w, 30))
}
base = extrude(plan, h)
post = extrude(plan, 9, new: true)
";

fn set(p: &str, v: f64) -> Op {
    Op::SetParam { param: ParamKey::parse(p).unwrap(), value: Literal { value: v, dim: forge_lang::values::Dim::Length } }
}

/// H2(a) path: an op kept in .pending while offline, restored on the next open. The restored
/// client joins with no seq, so the Welcome is always a snapshot and the op's base is rewritten
/// to the head: a Rename in between is skipped, and the op lands on the new holder of the name.
/// Only 2 ops in between: no CATCH_UP limit needed.
#[test]
fn restored_pending_lands_on_the_renamed_target() {
    let mut st = MemStore::default();
    let seq = Sequencer::open(&mut st, "part.fg", DOC, None).unwrap();
    let mut room = Room::new(seq);
    let mut ada = Client::new("part.fg", "ada-c", "t");
    ada.outbox.clear();
    for (_, d) in room.join(0, "ada-c", "ada", None) { ada.receive(d); }
    ada.edit(set("w", 41.0)).unwrap();
    let kept = ada.saved();
    eprintln!("kept: {kept}");
    drop(ada); // the app closes while offline
    let s = &mut room.seq;
    s.submit(&mut st, "bob", "bob/b", 0, 1, &Op::Rename { from: "w".into(), to: "width".into() }).unwrap();
    s.submit(&mut st, "bob", "bob/b", 1, 2, &Op::AddFeature { before: Some("post".into()), text: "shared w = 5".into() }).unwrap();
    // next open
    let mut back = Client::restore("part.fg", "t", &kept).unwrap();
    back.outbox.clear();
    let out = room.join(1, "ada-c", "ada", None);
    assert!(matches!(out[0].1, Down::Welcome { snap: true, .. }));
    for (_, d) in out { back.receive(d); }
    let ups: Vec<Up> = back.outbox.drain(..).collect();
    eprintln!("resent: {ups:?}");
    for u in ups {
        let (out, _) = room.receive(&mut st, 1, u);
        for (_, d) in out { back.receive(d); }
    }
    let head = room.seq.head().to_string();
    eprintln!("HEAD:\n{head}");
    assert!(head.contains("shared width = 40"), "ada's edit did not reach her w (renamed width)");
    assert!(head.contains("shared w = 41"), "it landed on bob's new w");
}

/// Two app instances (or a copied project folder) restore the same .pending: one client id,
/// one op-id counter. The second instance's edit reuses an id the first committed and is
/// answered as a retry: dropped, and the user is not told.
#[test]
fn two_restores_of_one_pending_file_drop_an_edit_silently() {
    let mut st = MemStore::default();
    let seq = Sequencer::open(&mut st, "part.fg", DOC, None).unwrap();
    let mut room = Room::new(seq);
    let first = Client::new("part.fg", "shared-id", "t");
    let kept = first.saved();
    let mut a = Client::restore("part.fg", "t", &kept).unwrap();
    let mut b = Client::restore("part.fg", "t", &kept).unwrap();
    a.outbox.clear();
    b.outbox.clear();
    for (_, d) in room.join(1, "shared-id", "ada", None) { a.receive(d); }
    for (_, d) in room.join(2, "shared-id", "ada", None) { b.receive(d); }
    a.outbox.clear();
    b.outbox.clear();
    let ia = a.edit(set("w", 41.0)).unwrap();
    let ib = b.edit(set("h", 99.0)).unwrap();
    assert_eq!(ia, ib, "same op id");
    let mut b_events = vec![];
    for u in a.outbox.drain(..).collect::<Vec<_>>() {
        let (out, _) = room.receive(&mut st, 1, u);
        for (c, d) in out { if c == 1 { a.receive(d); } else { b_events.extend(b.receive(d)); } }
    }
    for u in b.outbox.drain(..).collect::<Vec<_>>() {
        let (out, _) = room.receive(&mut st, 2, u);
        eprintln!("answer to b: {:?}", out.iter().map(|x| &x.1).collect::<Vec<_>>());
        for (c, d) in out { if c == 2 { b_events.extend(b.receive(d)); } else { a.receive(d); } }
    }
    let head = room.seq.head().to_string();
    eprintln!("HEAD:\n{head}\nb events: {b_events:?}\nb pending {}", b.pending().len());
    assert!(!head.contains("shared h = 99"), "b's edit was dropped");
    assert!(b.pending().is_empty(), "and is no longer pending");
    assert!(!b_events.iter().any(|e| matches!(e, Event::Refused { .. })), "and b was not told");
}
