//! M5 (the clients review): an apply-all is one group, committed atomically: one log record, one
//! seq, one check of the group's result, deduplicated as one, its inverse a group. HOSTED-DESIGN's
//! cases (research/hosted/DESIGN.md §4.2).
use forge_lang::error::{Literal, Op, ParamKey};
use forge_sync::room::Room;
use forge_sync::wire::{Down, Up};
use forge_sync::{Client, Event, MemStore, Refusal, Sequencer, Store};

const DOC: &str = "\
shared w = 40
shared h = 12
sketch plan(on: xy) {
  o = rect(center: (0, 0), size: (w, 30))
}
base = extrude(plan, h)
sketch tab(on: xy) {
  o = rect(center: (40, 0), size: (8, 8))
}
post = extrude(tab, 9, new: true)
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

fn rename(from: &str, to: &str) -> Op {
    Op::Rename {
        from: from.into(),
        to: to.into(),
    }
}

/// A room with clients joined at conn = their index.
struct World {
    st: MemStore,
    room: Room,
    cs: Vec<Client>,
}

impl World {
    fn new(names: &[&str]) -> World {
        let mut st = MemStore::default();
        let seq = Sequencer::open(&mut st, "part.fg", DOC, None).unwrap();
        let mut w = World {
            st,
            room: Room::new(seq),
            cs: vec![],
        };
        for (k, n) in names.iter().enumerate() {
            let mut c = Client::new("part.fg", &format!("{n}-c"), "t");
            c.outbox.clear();
            w.cs.push(c);
            w.join(k, None);
        }
        w
    }
    fn join(&mut self, k: usize, seq: Option<u64>) {
        let id = self.cs[k].id.clone();
        let author = id.trim_end_matches("-c").to_string();
        for (to, d) in self.room.join(k as u64, &id, &author, seq) {
            self.cs[to as usize].receive(d);
        }
    }
    /// One message from client `k` to the room, and the room's answers delivered.
    fn deliver(&mut self, k: usize, u: Up) -> Vec<Vec<Event>> {
        let mut evs = vec![vec![]; self.cs.len()];
        let (out, _) = self.room.receive(&mut self.st, k as u64, u);
        for (to, d) in out {
            evs[to as usize].extend(self.cs[to as usize].receive(d));
        }
        evs
    }
    /// Delivers every client's outbox (not Hellos: joins are explicit), and the answers, until
    /// nothing is left; returns the events each client saw.
    fn pump(&mut self) -> Vec<Vec<Event>> {
        let mut evs = vec![vec![]; self.cs.len()];
        loop {
            let mut any = false;
            for k in 0..self.cs.len() {
                for u in std::mem::take(&mut self.cs[k].outbox) {
                    if matches!(u, Up::Hello { .. }) {
                        continue;
                    }
                    any = true;
                    for (to, e) in self.deliver(k, u).into_iter().enumerate() {
                        evs[to].extend(e);
                    }
                }
            }
            if !any {
                return evs;
            }
        }
    }
    fn log(&mut self) -> Vec<forge_sync::Entry> {
        self.st.read_since("part.fg", 0).unwrap()
    }
    fn converged(&self) -> &str {
        let head = self.room.seq.head();
        for c in &self.cs {
            assert_eq!(c.text(), head, "{} converged on the head", c.id);
            assert!(c.pending().is_empty());
        }
        head
    }
}

/// Only the group's result is checked: it may pass through an invalid text (a depth of 0 on
/// the way to 5), which a per-op check would refuse.
#[test]
fn a_group_is_checked_as_a_whole() {
    let uses = set("h", 0.0);
    let adds = set("h", 5.0);
    let mut st = MemStore::default();
    let mut s = Sequencer::open(&mut st, "part.fg", DOC, None).unwrap();
    // alone, the first op is refused (the intermediate text is invalid)
    assert!(
        s.submit(&mut st, "ada", "ada-c", 0, 1, std::slice::from_ref(&uses))
            .is_err()
    );
    // as a group: committed, one entry, one seq
    let c = s
        .submit(&mut st, "ada", "ada-c", 0, 2, &[uses.clone(), adds.clone()])
        .unwrap();
    assert_eq!((c.entry.seq, c.entry.ops.len()), (1, 2));
    assert!(s.head().contains("shared h = 5"), "{}", s.head());
    assert_eq!(st.read_since("part.fg", 0).unwrap().len(), 1, "one record");
    // the client's own check is the same: the group is accepted, the op alone is not
    let mut c = Client::new("part.fg", "bob-c", "t");
    c.receive(Down::Welcome {
        seq: 0,
        text: DOC.into(),
        you: "bob".into(),
        snap: true,
    });
    assert!(c.edit(uses.clone()).is_err());
    c.edit_group(vec![uses, adds]).unwrap();
    assert!(c.text().contains("shared h = 5"));
    // limits: an empty group and one over MAX_GROUP_OPS are refused
    assert!(matches!(
        s.submit(&mut st, "ada", "ada-c", 1, 3, &[]),
        Err(Refusal::Invalid { .. })
    ));
    let many = vec![set("w", 41.0); forge_sync::MAX_GROUP_OPS + 1];
    assert!(matches!(
        s.submit(&mut st, "ada", "ada-c", 1, 4, &many),
        Err(Refusal::Invalid { .. })
    ));
    // and on the wire: a message with no ops, or too many, is not a message
    let text = Up::Op {
        base: 0,
        id: 1,
        ops: many,
    }
    .text();
    assert!(Up::parse(&text).is_err());
    assert!(Up::parse(r#"{"t":"op","base":0,"id":1,"ops":[]}"#).is_err());
}

/// A group refused as a whole leaves the head and the log as they were: none of its ops lands.
#[test]
fn a_refused_group_leaves_head_and_log_untouched() {
    let mut st = MemStore::default();
    let mut s = Sequencer::open(&mut st, "part.fg", DOC, None).unwrap();
    s.submit(&mut st, "bob", "bob-c", 0, 1, &[set("h", 13.0)])
        .unwrap();
    let (head, seq, log) = (
        s.head().to_string(),
        s.seq(),
        st.read_since("part.fg", 0).unwrap(),
    );
    // its second op does not apply (no such feature): the first, valid alone, must not land
    let r = s.submit(
        &mut st,
        "ada",
        "ada-c",
        1,
        1,
        &[set("w", 41.0), Op::DeleteFeature { id: "nosuch".into() }],
    );
    assert!(r.is_err(), "{r:?}");
    assert_eq!(s.head(), head);
    assert_eq!(s.seq(), seq);
    assert_eq!(st.read_since("part.fg", 0).unwrap(), log);
    // and a client's view drops the whole group when it is refused
    let mut w = World::new(&["ada", "bob"]);
    w.cs[0]
        .edit_group(vec![set("w", 41.0), rename("h", "height")])
        .unwrap();
    // bob renames h first: ada's group (its rename of h) is refused, and with it w = 41
    w.cs[1].edit(rename("h", "hh")).unwrap();
    // (bob's op reaches the server first)
    let ada_out = std::mem::take(&mut w.cs[0].outbox);
    let ev = w.pump();
    assert!(ev[1].iter().all(|e| !matches!(e, Event::Refused { .. })));
    for u in ada_out {
        w.deliver(0, u);
    }
    let head = w.converged().to_string();
    assert!(
        head.contains("shared w = 40"),
        "none of the group landed:\n{head}"
    );
    assert!(head.contains("shared hh = 12"));
    assert_eq!(w.log().len(), 1, "only bob's op is in the log");
}

/// A group with Rename a→b and an op on b, against a concurrent Rename a→c: one consistent
/// outcome in either order (the later rename refused, or the group refused as a unit), never
/// half a group.
#[test]
fn a_rename_group_against_a_concurrent_rename() {
    for group_first in [true, false] {
        let mut w = World::new(&["ada", "bob"]);
        w.cs[0]
            .edit_group(vec![rename("w", "width"), set("width", 55.0)])
            .unwrap();
        w.cs[1].edit(rename("w", "wide")).unwrap();
        let (first, second) = if group_first { (0, 1) } else { (1, 0) };
        let out_second = std::mem::take(&mut w.cs[second].outbox);
        w.pump();
        let mut refused = vec![];
        for u in out_second {
            for e in w.deliver(second, u).concat() {
                if matches!(e, Event::Refused { .. }) {
                    refused.push(e);
                }
            }
        }
        let head = w.converged().to_string();
        assert!(
            matches!(
                refused.as_slice(),
                [Event::Refused {
                    refusal: Refusal::RenameConflict { .. },
                    ..
                }]
            ),
            "the later one is refused: {refused:?}"
        );
        if first == 0 {
            assert!(
                head.contains("shared width = 55") && !head.contains("wide"),
                "{head}"
            );
        } else {
            assert!(
                head.contains("shared wide = 40") && !head.contains("width"),
                "{head}"
            );
        }
        assert_eq!(w.log().len(), 1, "one record");
    }
}

/// A group resent after a reconnect (its Ack lost) is deduplicated as one: committed once.
#[test]
fn a_group_resent_after_a_reconnect_commits_once() {
    let mut w = World::new(&["ada", "bob"]);
    w.cs[0]
        .edit_group(vec![set("w", 41.0), set("h", 13.0), rename("post", "peg")])
        .unwrap();
    // delivered (bob gets it), ada's answer lost with the link
    for u in std::mem::take(&mut w.cs[0].outbox) {
        let (out, _) = w.room.receive(&mut w.st, 0, u);
        for (to, d) in out.into_iter().filter(|x| x.0 != 0) {
            w.cs[to as usize].receive(d);
        }
    }
    w.room.leave(0);
    w.cs[0].reconnect();
    w.cs[0].outbox.clear();
    let seen = w.cs[0].seq();
    w.join(0, Some(seen));
    let ev = w.pump();
    assert!(
        ev[0].iter().all(|e| !matches!(e, Event::Refused { .. })),
        "{ev:?}"
    );
    let head = w.converged().to_string();
    assert!(head.contains("shared w = 41") && head.contains("shared h = 13") && head.contains("peg ="));
    let log = w.log();
    assert_eq!(log.len(), 1, "committed once: {log:?}");
    assert_eq!(log[0].ops.len(), 3);
    // and once more at the sequencer, the same (client, id): the same entry, nothing written
    let again = w
        .room
        .seq
        .submit(&mut w.st, "ada", "ada-c", 0, 1, &log[0].ops)
        .unwrap();
    assert!(again.duplicate);
    assert_eq!(w.log().len(), 1);
}

/// A group made on a version older than the in-memory log (compacted since) is rebased from the
/// store's kept ops like a single op: through a collaborator's rename, onto the right name.
#[test]
fn a_group_straddling_a_compaction_rebases_from_kept_ops() {
    let mut st = MemStore::default();
    let mut s = Sequencer::open(&mut st, "part.fg", DOC, None).unwrap();
    s.limits.max_log_bytes = 2_000;
    s.submit(&mut st, "bob", "bob-c", 0, 1, &[rename("w", "width")])
        .unwrap();
    for k in 0..40 {
        s.submit(
            &mut st,
            "bob",
            "bob-c",
            s.seq(),
            2 + k,
            &[set("h", 12.0 + (k % 3) as f64)],
        )
        .unwrap();
    }
    assert!(s.compactions > 0 && s.floor() > 1, "the rename is before the log");
    // ada's group, made at seq 0 (before bob's rename)
    let c = s
        .submit(&mut st, "ada", "ada-c", 0, 1, &[set("w", 41.0), set("h", 20.0)])
        .unwrap();
    assert_eq!(c.entry.ops, vec![set("width", 41.0), set("h", 20.0)]);
    assert!(s.head().contains("shared width = 41") && s.head().contains("shared h = 20"));
    assert!(!s.head().contains("shared w ="));
}

/// The group's inverse is a group: undone after a collaborator's later op that uses the group's
/// new name, it is rebased like any group and lands on the right names (or is refused): never a
/// wrong target.
#[test]
fn undo_of_a_rename_group_after_a_collaborator_uses_the_new_name() {
    let mut w = World::new(&["ada", "bob"]);
    w.cs[0]
        .edit_group(vec![rename("w", "width"), set("width", 41.0)])
        .unwrap();
    w.pump();
    // bob uses the new name
    w.cs[1]
        .edit(Op::AddFeature {
            before: Some("plan".into()),
            text: "shared k = width + 1".into(),
        })
        .unwrap();
    w.pump();
    assert!(w.converged().contains("shared k = width + 1"));
    // ada undoes her group: one group, both ops back, in reverse order
    let r = w.cs[0].undo().expect("something to undo");
    let ev = w.pump();
    let head = w.converged().to_string();
    // (bob's op renamed nothing: the inverse group rebases unchanged and lands; were it a
    // rename of `width`, the inverse would be refused as a unit, never put on another name)
    assert!(r.is_ok(), "{r:?}");
    assert!(
        ev[0].iter().all(|e| !matches!(e, Event::Refused { .. })),
        "{ev:?}"
    );
    // the rename back rewrote bob's use too; the value is the one before the group
    assert!(head.contains("shared w = 40"), "{head}");
    assert!(head.contains("shared k = w + 1"), "{head}");
    assert!(!head.contains("width"), "{head}");
    let log = w.log();
    assert_eq!(log.last().unwrap().ops.len(), 2, "the undo is one group");
    assert_eq!(log.len(), 3);
}

/// A log written before groups (one `op` per entry) still opens.
#[test]
fn a_protocol_2_log_entry_reads_as_a_group_of_one() {
    use forge_sync::wire::write;
    let old = format!(
        r#"{{"seq":1,"author":"dev","client":"c","id":1,"base":0,"op":{},"inv":{},"th":"0000000000000007"}}"#,
        write(&rename("a", "b").json()),
        write(&rename("b", "a").json())
    );
    let j = forge_sync::wire::read(&old).unwrap();
    let e = forge_sync::wire::entry_from(&j).expect("reads");
    assert_eq!(e.ops, vec![rename("a", "b")]);
    assert_eq!(e.inv, Some(vec![rename("b", "a")]));
    // written back as a group
    let back = forge_sync::wire::write(&forge_sync::wire::entry_json(&e));
    assert!(back.contains("\"ops\":["), "{back}");
    assert_eq!(
        forge_sync::wire::entry_from(&forge_sync::wire::read(&back).unwrap()),
        Some(e)
    );
}

// HOSTED-DESIGN's M5 review (research/hosted/reviews/m5-groups.md).

/// High: an edit before the first Welcome is made on a text whose version this client does not
/// know; sent with base 0 the server would rebase it through ops it was made after (a set of
/// the new `w` landed on `width`). Refused, nothing queued; after the Welcome its base is the
/// Welcome's seq and it lands on the right name.
#[test]
fn an_edit_before_the_first_welcome_is_refused_not_sent_with_a_guessed_base() {
    let mut st = MemStore::default();
    let mut s = Sequencer::open(&mut st, "part.fg", DOC, None).unwrap();
    s.submit(&mut st, "bob", "bob-c", 0, 1, &[rename("w", "width")])
        .unwrap();
    s.submit(
        &mut st,
        "bob",
        "bob-c",
        1,
        2,
        &[Op::AddFeature {
            before: Some("plan".into()),
            text: "shared w = 5".into(),
        }],
    )
    .unwrap();
    let mut room = Room::new(s);
    let mut ada = Client::new("part.fg", "ada-c", "t");
    ada.outbox.clear();
    let after = DOC.replace("shared w = 40", "shared w = 7");
    assert!(matches!(
        ada.local(set("w", 7.0), after),
        Err(Refusal::Invalid { .. })
    ));
    assert!(ada.edit(set("w", 7.0)).is_err());
    assert!(ada.edit_group(vec![set("w", 7.0)]).is_err());
    assert!(
        ada.pending().is_empty() && ada.outbox.is_empty(),
        "nothing queued or sent"
    );
    for (_, d) in room.join(0, "ada-c", "ada", None) {
        ada.receive(d);
    }
    ada.edit(set("w", 7.0)).unwrap();
    assert_eq!(ada.pending()[0].base, 2, "made on the Welcome's text");
    for u in std::mem::take(&mut ada.outbox) {
        for (_, d) in room.receive(&mut st, 0, u).0 {
            ada.receive(d);
        }
    }
    let head = room.seq.head();
    assert!(
        head.contains("shared w = 7") && head.contains("shared width = 40"),
        "{head}"
    );
    assert_eq!(ada.text(), head);
}

/// Medium 2: a group outside the limits is refused by the client itself, nothing queued (sent,
/// the server would refuse it or, over its message size, close the connection on every resend).
#[test]
fn the_client_refuses_a_group_outside_the_limits() {
    let mut w = World::new(&["ada"]);
    let c = &mut w.cs[0];
    let view = c.text().to_string();
    assert!(c.local_group(vec![], view.clone()).is_err());
    assert!(
        c.local_group(vec![set("w", 41.0); forge_sync::MAX_GROUP_OPS + 1], view.clone())
            .is_err()
    );
    let big: Vec<Op> = (0..3)
        .map(|k| Op::AddFeature {
            before: Some("plan".into()),
            text: format!("shared big{k} = 1 // {}", "x".repeat(200 << 10)),
        })
        .collect();
    let r = c.local_group(big, view.clone());
    assert!(
        matches!(&r, Err(Refusal::Invalid { why }) if why.contains("bytes")),
        "{r:?}"
    );
    assert!(c.pending().is_empty() && c.outbox.is_empty());
    assert_eq!(c.text(), view);
    // the next edit gets the next id as if nothing happened
    assert_eq!(c.edit(set("w", 41.0)).unwrap(), 1);
}

/// Medium 1: the group's inverse is the checker's (worked out inside the check and its time
/// budget), not computed again on the document's thread after it.
#[test]
fn the_inverse_comes_from_the_check() {
    struct NoInverse;
    impl forge_sync::Check for NoInverse {
        fn check(
            &mut self,
            head: &str,
            counts: Option<&forge_sync::Counts>,
            ops: &[Op],
            dir: Option<std::path::PathBuf>,
            root: Option<std::path::PathBuf>,
        ) -> Result<forge_sync::sequencer::Checked, Refusal> {
            let (t, c) = forge_sync::validate(head, counts, ops, dir, root)?;
            Ok((t, c, None))
        }
    }
    let mut st = MemStore::default();
    let mut s = Sequencer::open(&mut st, "part.fg", DOC, None).unwrap();
    let c = s
        .submit(&mut st, "ada", "ada-c", 0, 1, &[set("w", 41.0)])
        .unwrap();
    assert_eq!(
        c.entry.inv,
        Some(vec![set("w", 40.0)]),
        "the default check inverts"
    );
    s.checker = Box::new(NoInverse);
    let c = s
        .submit(&mut st, "ada", "ada-c", 1, 2, &[set("w", 42.0)])
        .unwrap();
    assert_eq!(c.entry.inv, None, "the checker's inverse is the one kept");
}

/// Medium 3: a group with no exact inverse is an undo barrier, said: Undo never skips it to
/// take back the edit before it.
#[test]
fn a_group_with_no_inverse_is_an_undo_barrier() {
    let two = Op::AddFeature {
        before: Some("plan".into()),
        text: "shared a = 1\nshared b = 2".into(),
    };
    let after = forge_lang::edit::apply(DOC, &two).expect("applies");
    assert!(
        forge_sync::inverse::inverse(DOC, &two, &after).is_none(),
        "the precondition"
    );
    let mut w = World::new(&["ada"]);
    w.cs[0].edit(set("w", 41.0)).unwrap();
    w.pump();
    w.cs[0].edit_group(vec![set("h", 13.0), two]).unwrap();
    w.pump();
    let before = w.converged().to_string();
    for _ in 0..2 {
        let r = w.cs[0].undo().expect("a step is there");
        assert!(
            matches!(&r, Err(Refusal::Invalid { why }) if why.contains("cannot be undone")),
            "{r:?}"
        );
        w.pump();
        assert_eq!(w.converged(), before, "w = 41 stays: nothing older is undone");
    }
}

/// Medium 4 (older than M5): a second Undo takes back the edit before, never redoes the first.
#[test]
fn a_second_undo_does_not_redo_the_first() {
    let mut w = World::new(&["ada"]);
    w.cs[0].edit(set("w", 41.0)).unwrap();
    w.pump();
    w.cs[0].edit(set("h", 13.0)).unwrap();
    w.pump();
    w.cs[0].undo().unwrap().unwrap();
    w.pump();
    assert!(w.converged().contains("shared h = 12") && w.converged().contains("shared w = 41"));
    w.cs[0].undo().unwrap().unwrap();
    w.pump();
    let head = w.converged();
    assert!(
        head.contains("shared h = 12") && head.contains("shared w = 40"),
        "{head}"
    );
    assert!(w.cs[0].undo().is_none(), "nothing left: the undos are not steps");
}

/// A concurrent rename that two ops of one group read: both rebased onto the new name.
#[test]
fn a_concurrent_rename_reaches_every_op_of_a_group() {
    let mut w = World::new(&["ada", "bob"]);
    w.cs[1].edit(rename("w", "width")).unwrap();
    w.cs[0]
        .edit_group(vec![
            set("w", 41.0),
            Op::AddFeature {
                before: Some("plan".into()),
                text: "shared k = w + 1".into(),
            },
        ])
        .unwrap();
    let ada_out = std::mem::take(&mut w.cs[0].outbox);
    w.pump(); // bob's rename first
    for u in ada_out {
        w.deliver(0, u);
    }
    w.pump();
    let head = w.converged().to_string();
    assert!(
        head.contains("shared width = 41") && head.contains("shared k = width + 1"),
        "{head}"
    );
    assert!(!head.contains("shared w "), "{head}");
}

/// Undo through a collaborator's rename: the inverse lands on the renamed parameter.
#[test]
fn undo_through_a_collaborators_rename() {
    let mut w = World::new(&["ada", "bob"]);
    w.cs[0].edit(set("w", 41.0)).unwrap();
    w.pump();
    w.cs[1].edit(rename("w", "width")).unwrap();
    w.pump();
    w.cs[0].undo().unwrap().unwrap();
    let ev = w.pump();
    assert!(
        ev[0].iter().all(|e| !matches!(e, Event::Refused { .. })),
        "{ev:?}"
    );
    let head = w.converged();
    assert!(
        head.contains("shared width = 40") && !head.contains("shared w "),
        "{head}"
    );
}
