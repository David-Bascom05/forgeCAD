//! Regression tests for HOSTED-DESIGN's review of N1–N9 (research/hosted/reviews/N1-N9.md): a
//! document's close, a Close lost to a full inbox, a Resync within the gap, a hash build past its
//! budget, and the op history kept across a close.
#![cfg(all(feature = "test-hooks", not(target_os = "wasi")))] // the hooks; sockets and threads

use forge_lang::error::{Literal, Op, ParamKey};
use forge_server::{Config, StoreKind, start_on};
use forge_sync::wire::{Down, Up};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const DOC: &str = "shared w = 40\nsketch plan(on: xy) {\n  o = rect(center: (0, 0), size: (w, 30))\n}\nbase = extrude(plan, 10)\n";
const ADA: &str = "0123456789abcdef-ada";

fn project(name: &str, doc: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("forge-close-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join(doc), DOC).unwrap();
    d
}

fn serve_with(root: &Path, f: impl FnOnce(&mut Config)) -> (String, forge_server::Handle) {
    let mut c = Config::new(root.to_path_buf(), vec![("ada".into(), ADA.into())]);
    c.store = StoreKind::File;
    f(&mut c);
    let h = start_on(TcpListener::bind("127.0.0.1:0").unwrap(), c).unwrap();
    (h.addr.to_string(), h)
}

fn hello(doc: &str, client: &str, seq: Option<u64>) -> String {
    Up::Hello {
        doc: doc.into(),
        client: client.into(),
        token: ADA.into(),
        seq,
        proto: forge_sync::PROTOCOL,
        kernel: forge_sync::KERNEL.into(),
    }
    .text()
}

fn connect(addr: &str) -> (forge_ws::Reader, forge_ws::Writer) {
    let (r, w) = forge_ws::connect(addr, "/sync", &[], 1 << 20).unwrap();
    r.stream().set_read_timeout(Some(Duration::from_secs(8))).unwrap();
    (r, w)
}

/// The next message, or None when none comes in time.
fn next(r: &mut forge_ws::Reader) -> Option<Down> {
    match r.read() {
        Ok(forge_ws::Msg::Text(t)) => Some(Down::parse(&t).unwrap()),
        _ => None,
    }
}

fn joined(addr: &str, doc: &str, client: &str) -> (forge_ws::Reader, forge_ws::Writer, u64) {
    let (mut r, mut w) = connect(addr);
    w.send(&hello(doc, client, None)).unwrap();
    match next(&mut r) {
        Some(Down::Welcome { seq, .. }) => (r, w, seq),
        d => panic!("{d:?}"),
    }
}

fn set_w(v: f64) -> Op {
    Op::SetParam {
        param: ParamKey::parse("w").unwrap(),
        value: Literal {
            value: v,
            dim: forge_lang::values::Dim::Length,
        },
    }
}

fn op(base: u64, id: u64, v: f64) -> String {
    Up::Op {
        base,
        id,
        ops: vec![set_w(v)],
    }
    .text()
}

/// The Ack of op `id`, skipping anything else; None if it never comes.
fn ack_of(r: &mut forge_ws::Reader, id: u64) -> Option<u64> {
    loop {
        match next(r)? {
            Down::Ack { id: k, seq, .. } if k == id => return Some(seq),
            Down::Refused { id: k, refusal } if k == id => panic!("op {id} refused: {refusal:?}"),
            _ => {}
        }
    }
}

/// H1: a client reconnecting while its document saves (its only member left), its op sent right
/// behind its Hello: the op waits with the join and is committed by the new actor.
#[test]
fn an_op_sent_while_the_document_closes_is_committed() {
    let doc = "test_hook_slow_close.fg";
    let root = project("h1", doc);
    let (addr, h) = serve_with(&root, |_| {});
    let (mut r, mut w, seq) = joined(&addr, doc, "c1");
    w.send(&op(seq, 1, 41.0)).unwrap();
    let s1 = ack_of(&mut r, 1).expect("op 1 acked");
    drop((r, w)); // the only member leaves: the actor saves (1.5 s, the hook)
    std::thread::sleep(Duration::from_millis(200));
    let (mut r, mut w) = connect(&addr);
    w.send(&hello(doc, "c1", Some(s1))).unwrap();
    w.send(&op(s1, 2, 42.0)).unwrap(); // not waiting for Welcome
    let t0 = Instant::now();
    assert!(matches!(next(&mut r), Some(Down::Welcome { .. })));
    assert_eq!(
        ack_of(&mut r, 2),
        Some(s1 + 1),
        "the op sent during the close was committed"
    );
    assert!(
        t0.elapsed() >= Duration::from_millis(800),
        "it did wait for the save"
    );
    h.stop();
    let _ = std::fs::remove_dir_all(&root);
}

/// H2: a Close lost (the hook drops it, as a full inbox would): the actor still ends on its flag,
/// and the document opens again for the next join.
#[test]
fn a_lost_close_still_ends_the_actor() {
    let doc = "test_hook_lost_close.fg";
    let root = project("h2", doc);
    let (addr, h) = serve_with(&root, |_| {});
    let (r, w, _) = joined(&addr, doc, "c1");
    drop((r, w));
    std::thread::sleep(Duration::from_millis(300));
    let t0 = Instant::now();
    let (mut r, mut w) = connect(&addr);
    w.send(&hello(doc, "c2", None)).unwrap();
    assert!(
        matches!(next(&mut r), Some(Down::Welcome { .. })),
        "served after a lost Close"
    );
    assert!(t0.elapsed() < Duration::from_secs(3));
    h.stop();
    let _ = std::fs::remove_dir_all(&root);
}

/// M2: two Resyncs within RESYNC_GAP: both answered, the second when the gap has passed.
#[test]
fn a_second_resync_is_deferred_not_dropped() {
    let doc = "part.fg";
    let root = project("m2", doc);
    let (addr, h) = serve_with(&root, |_| {});
    let (mut r, mut w, _) = joined(&addr, doc, "c1");
    let t0 = Instant::now();
    w.send(&Up::Resync.text()).unwrap();
    w.send(&Up::Resync.text()).unwrap();
    assert!(matches!(next(&mut r), Some(Down::Welcome { snap: true, .. })));
    assert!(t0.elapsed() < Duration::from_secs(1), "the first at once");
    assert!(
        matches!(next(&mut r), Some(Down::Welcome { snap: true, .. })),
        "the second too"
    );
    let t = t0.elapsed();
    assert!(
        t >= forge_server::RESYNC_GAP && t < forge_server::RESYNC_GAP * 2,
        "{t:?}"
    );
    h.stop();
    let _ = std::fs::remove_dir_all(&root);
}

/// N9: a hash build past its budget gives its slot back once (by the actor), the build's own
/// late end does not give it back again, and its late result answers nothing.
#[test]
fn a_build_past_its_budget_frees_its_slot_once() {
    let doc = "test_hook_slow_build.fg";
    let root = project("n9", doc);
    let (addr, h) = serve_with(&root, |c| c.build_budget = Duration::from_millis(300));
    let (mut r, mut w, seq) = joined(&addr, doc, "c1");
    w.send(&Up::BuildHash { seq, hash: 1 }.text()).unwrap();
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(h.builds(), 1, "the build holds a slot");
    std::thread::sleep(Duration::from_millis(600));
    assert_eq!(h.builds(), 0, "given up on: the actor freed it");
    std::thread::sleep(Duration::from_millis(1500));
    assert_eq!(h.builds(), 0, "the build's late end did not free it again");
    r.stream()
        .set_read_timeout(Some(Duration::from_millis(300)))
        .unwrap();
    assert!(next(&mut r).is_none(), "a late result judges nothing");
    drop(w);
    h.stop();
    let _ = std::fs::remove_dir_all(&root);
}

/// M1: a document closed (its snapshot taken) still has its ops in the log: the snapshot is a
/// record appended after them.
#[test]
fn a_close_keeps_the_op_history() {
    let doc = "part.fg";
    let root = project("m1", doc);
    let (addr, h) = serve_with(&root, |_| {});
    let (mut r, mut w, seq) = joined(&addr, doc, "c1");
    for k in 1..=3 {
        w.send(&op(seq + k - 1, k, 40.0 + k as f64)).unwrap();
        ack_of(&mut r, k).expect("acked");
    }
    drop((r, w));
    std::thread::sleep(Duration::from_millis(600)); // the close: checkpoint and snapshot
    let logs: Vec<_> = std::fs::read_dir(root.join(".forgecad/server/log"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(logs.len(), 1);
    let b = std::fs::read(&logs[0]).unwrap();
    let records = b.windows(4).filter(|x| x == b"FGL1").count();
    assert!(records >= 5, "first text, 3 ops, a snapshot: {records}");
    let text = String::from_utf8_lossy(&b);
    assert_eq!(
        text.matches("\"first\"").count(),
        2,
        "the first text and one snapshot"
    );
    // and it reopens from that snapshot, at the same seq
    let (_r, _w, again) = joined(&addr, doc, "c2");
    assert_eq!(again, seq + 3);
    h.stop();
    let _ = std::fs::remove_dir_all(&root);
}
