//! HOSTED-DESIGN's clients re-review (research/hosted/reviews/clients-rereview.md), finding 2,
//! as a regression test: it passed while the bug reproduced; it now asserts the fix.
use forge_lang::error::{Literal, Op, ParamKey};
use forge_sync::room::Room;
use forge_sync::wire::{Down, Up};
use forge_sync::{Client, MemStore, Sequencer};

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
    Op::SetParam {
        param: ParamKey::parse(p).unwrap(),
        value: Literal {
            value: v,
            dim: forge_lang::values::Dim::Length,
        },
    }
}

/// H2(a) path: an op kept in .pending while offline, restored on the next open. The restored
/// client joins with no seq, so the Welcome is always a snapshot and the op's base is rewritten
/// to the head: a Rename in between is skipped, and the op lands on the new holder of the name.
/// Only 2 ops in between: no CATCH_UP limit needed.
#[test]
fn a_restored_pending_op_follows_a_rename() {
    let mut st = MemStore::default();
    let seq = Sequencer::open(&mut st, "part.fg", DOC, None).unwrap();
    let mut room = Room::new(seq);
    let mut ada = Client::new("part.fg", "ada-c", "t");
    ada.outbox.clear();
    for (_, d) in room.join(0, "ada-c", "ada", None) {
        ada.receive(d);
    }
    ada.edit(set("w", 41.0)).unwrap();
    let kept = ada.saved();
    eprintln!("kept: {kept}");
    drop(ada); // the app closes while offline
    let s = &mut room.seq;
    s.submit(
        &mut st,
        "bob",
        "bob/b",
        0,
        1,
        &Op::Rename {
            from: "w".into(),
            to: "width".into(),
        },
    )
    .unwrap();
    s.submit(
        &mut st,
        "bob",
        "bob/b",
        1,
        2,
        &Op::AddFeature {
            before: Some("post".into()),
            text: "shared w = 5".into(),
        },
    )
    .unwrap();
    // next open
    let mut back = Client::restore("part.fg", "t", &kept).unwrap();
    back.outbox.clear();
    let out = room.join(1, "ada-c", "ada", None);
    assert!(matches!(out[0].1, Down::Welcome { snap: true, .. }));
    for (_, d) in out {
        back.receive(d);
    }
    let ups: Vec<Up> = std::mem::take(&mut back.outbox);
    eprintln!("resent: {ups:?}");
    for u in ups {
        let (out, _) = room.receive(&mut st, 1, u);
        for (_, d) in out {
            back.receive(d);
        }
    }
    let head = room.seq.head().to_string();
    eprintln!("HEAD:\n{head}");
    // sent with its real base, the kept op is rebased through bob's rename
    assert!(
        head.contains("shared width = 41"),
        "ada's edit reaches her w (renamed width)"
    );
    assert!(head.contains("shared w = 5"), "bob's new w is untouched");
}
