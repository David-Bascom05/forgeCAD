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
fn rename(a: &str, b: &str) -> Op {
    Op::Rename {
        from: a.into(),
        to: b.into(),
    }
}

/// The app attaches a served part before the first Welcome and edits the file's text (the
/// server's text at seq N): the client queues the op with base 0, not N.
fn edit_before_welcome(history: Vec<Op>, op: Op) -> (String, Vec<Event>, u64) {
    let mut st = MemStore::default();
    let seq = Sequencer::open(&mut st, "part.fg", DOC, None).unwrap();
    let mut room = Room::new(seq);
    for (k, o) in history.into_iter().enumerate() {
        let b = room.seq.seq();
        room.seq.submit(&mut st, "bob", "bob/c", b, k as u64 + 1, &[o]).unwrap();
    }
    let file = room.seq.head().to_string(); // the file on disk == the server's head
    let mut c = Client::new("part.fg", "ada/c", "t");
    c.outbox.clear();
    let after = forge_lang::edit::apply_checked(&file, &op).unwrap().0;
    c.local(op, after);
    let base = c.pending()[0].base;
    let mut evs = vec![];
    for (to, d) in room.join(0, "ada/c", "ada", None) {
        assert_eq!(to, 0);
        evs.extend(c.receive(d));
    }
    for u in std::mem::take(&mut c.outbox) {
        if matches!(u, Up::Hello { .. }) {
            continue;
        }
        let (out, _) = room.receive(&mut st, 0, u);
        for (_, d) in out {
            evs.extend(c.receive(d));
        }
    }
    (room.seq.head().to_string(), evs, base)
}

#[test]
fn review_edit_before_welcome_lands_on_another_name() {
    // bob: w renamed width, then a new shared w added. ada's file is that text; she sets the
    // NEW w before her link's first Welcome.
    let (head, evs, base) = edit_before_welcome(
        vec![
            rename("w", "width"),
            Op::AddFeature {
                before: Some("plan".into()),
                text: "shared w = 5".into(),
            },
        ],
        set("w", 7.0),
    );
    eprintln!("base sent: {base}\nhead:\n{head}\nevents: {evs:?}");
    assert_eq!(base, 0);
    assert!(head.contains("shared width = 7"), "landed on width: {head}");
    assert!(head.contains("shared w = 5"), "the w she set is untouched");
}

#[test]
fn review_edit_before_welcome_spurious_rename_conflict() {
    let (head, evs, _) = edit_before_welcome(
        vec![rename("w", "width"), rename("width", "w")],
        rename("w", "wide"),
    );
    eprintln!("head:\n{head}\nevents: {evs:?}");
    assert!(evs.iter().any(|e| matches!(e, Event::Refused { .. })), "{evs:?}");
}

#[test]
fn review_restore_with_empty_group_drops_all_pending() {
    let mut c = Client::new("part.fg", "ada/c", "t");
    c.receive(Down::Welcome {
        seq: 0,
        text: DOC.into(),
        you: "ada".into(),
        snap: true,
    });
    let t = c.text().to_string();
    let a = forge_lang::edit::apply(&t, &set("w", 41.0)).unwrap();
    c.local(set("w", 41.0), a.clone());
    c.local_group(vec![], a); // apply_all(vec![]) in the app
    let saved = c.saved();
    eprintln!("{saved}");
    assert!(Client::restore("part.fg", "t", &saved).is_none(), "every pending op lost");
}

#[test]
fn review_group_inverse_cost() {
    let mut doc = String::new();
    for i in 0..8_000 {
        doc.push_str(&format!("shared p{i} = 1\n"));
    }
    eprintln!("doc bytes {}", doc.len());
    let ops: Vec<Op> = (0..256).map(|i| set(&format!("p{i}"), 2.0)).collect();
    let t = std::time::Instant::now();
    let one = forge_lang::edit::apply(&doc, &ops[0]).unwrap();
    eprintln!("one apply: {:?}", t.elapsed());
    let t = std::time::Instant::now();
    let (_, counts) = forge_sync::validate(&doc, None, &ops[..1], None, None).unwrap();
    eprintln!("validate head+1 op: {:?}", t.elapsed());
    let t = std::time::Instant::now();
    let (after, _) = forge_sync::validate(&doc, Some(&counts), &ops, None, None).unwrap();
    eprintln!("validate group of 256 (budgeted part): {:?}", t.elapsed());
    let t = std::time::Instant::now();
    let inv = forge_sync::sequencer::group_inverse(&doc, &ops, &after);
    eprintln!("group_inverse (unbudgeted, actor thread): {:?} some={}", t.elapsed(), inv.is_some());
    let _ = one;
}

/// Two hosted undos: the second takes back the first undo (its inverse was pushed), never the
/// edit before.
#[test]
fn review_second_undo_redoes_instead_of_going_back() {
    let mut st = MemStore::default();
    let seq = Sequencer::open(&mut st, "part.fg", DOC, None).unwrap();
    let mut room = Room::new(seq);
    let mut c = Client::new("part.fg", "ada/c", "t");
    c.outbox.clear();
    for (_, d) in room.join(0, "ada/c", "ada", None) {
        c.receive(d);
    }
    let pump = |c: &mut Client, room: &mut Room, st: &mut MemStore| {
        for u in std::mem::take(&mut c.outbox) {
            let (out, _) = room.receive(st, 0, u);
            for (_, d) in out {
                c.receive(d);
            }
        }
    };
    c.edit_group(vec![set("w", 41.0), set("h", 13.0)]).unwrap(); // edit 1
    pump(&mut c, &mut room, &mut st);
    c.edit(set("h", 20.0)).unwrap(); // edit 2
    pump(&mut c, &mut room, &mut st);
    c.undo().unwrap().unwrap(); // undo edit 2: h = 13
    pump(&mut c, &mut room, &mut st);
    let after_one = room.seq.head().to_string();
    c.undo().unwrap().unwrap(); // should undo edit 1: w = 40, h = 12
    pump(&mut c, &mut room, &mut st);
    let head = room.seq.head().to_string();
    eprintln!("after one undo:\n{after_one}\nafter two:\n{head}");
    assert!(head.contains("shared h = 20"), "the second undo re-did edit 2");
}

/// A group with one op lacking an inverse (SetArg) has none; the next undo silently takes back
/// the edit before it.
#[test]
fn review_undo_skips_a_group_without_inverse() {
    let mut st = MemStore::default();
    let seq = Sequencer::open(&mut st, "part.fg", DOC, None).unwrap();
    let mut room = Room::new(seq);
    let mut c = Client::new("part.fg", "ada/c", "t");
    c.outbox.clear();
    for (_, d) in room.join(0, "ada/c", "ada", None) {
        c.receive(d);
    }
    let pump = |c: &mut Client, room: &mut Room, st: &mut MemStore| {
        for u in std::mem::take(&mut c.outbox) {
            let (out, _) = room.receive(st, 0, u);
            for (_, d) in out {
                eprintln!("down: {}", d.text());
                c.receive(d);
            }
        }
    };
    c.edit(set("h", 13.0)).unwrap(); // edit 1
    pump(&mut c, &mut room, &mut st);
    c.edit_group(vec![
        set("w", 41.0),
        Op::SetArg {
            id: "base".into(),
            arg: "new".into(),
            text: "true".into(),
        },
    ])
    .unwrap(); // edit 2 (an apply-all)
    pump(&mut c, &mut room, &mut st);
    let r = c.undo();
    pump(&mut c, &mut room, &mut st);
    let head = room.seq.head().to_string();
    eprintln!("undo: {r:?}\n{head}");
    assert!(head.contains("shared w = 41") && head.contains("shared h = 12"), "edit 1 undone, edit 2 kept");
}
