//! A full disk under a running server: an edit that cannot be written durably is refused
//! ("not saved", never acked), the server keeps serving, the log stays whole (a part-written
//! record is cut back); a close while full (its checkpoint and its snapshot record failing
//! mid-write) loses nothing acked; once there is room again edits commit, and a restart reopens
//! the log with every committed op once and its seqs without a gap.
//!
//! Needs a small filesystem of its own to fill (it is filled for real): ignored by default.
//!   mkdir -p /tmp/tiny && mount -t tmpfs -o size=4m tmpfs /tmp/tiny
//!   FORGE_FULL_DIR=/tmp/tiny cargo test -p forge-server --test diskfull -- --ignored
#![cfg(not(target_os = "wasi"))] // sockets, threads

use forge_lang::error::{Literal, Op, ParamKey};
use forge_remote::{Remote, Session};
use forge_server::{Config, StoreKind, start_on};
use std::io::Write;
use std::net::TcpListener;
use std::path::Path;
use std::time::{Duration, Instant};

const DOC: &str = "shared w = 40\nsketch plan(on: xy) {\n  o = rect(center: (0, 0), size: (w, 30))\n}\nbase = extrude(plan, 10)\n";
const TOKEN: &str = "0123456789abcdef-full";

fn set(v: f64) -> Op {
    Op::SetParam {
        param: ParamKey::parse("w").unwrap(),
        value: Literal {
            value: v,
            dim: forge_lang::values::Dim::Length,
        },
    }
}

fn serve(root: &Path, at: &str) -> forge_server::Handle {
    let mut c = Config::new(root.to_path_buf(), vec![("dev".into(), TOKEN.into())]);
    c.store = StoreKind::File;
    start_on(TcpListener::bind(at).unwrap(), c).unwrap()
}

fn wait(what: &str, mut f: impl FnMut() -> bool) {
    let t0 = Instant::now();
    while !f() {
        assert!(t0.elapsed() < Duration::from_secs(20), "timed out: {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Fills `dir`'s filesystem with a file until a write fails: what is left is a few blocks at most.
fn fill(dir: &Path) -> std::path::PathBuf {
    let p = dir.join("filler");
    let mut f = std::fs::File::create(&p).unwrap();
    let block = vec![0u8; 4096];
    while f.write_all(&block).and_then(|_| f.sync_data()).is_ok() {}
    p
}

#[test]
#[ignore]
fn a_full_disk_refuses_edits_keeps_the_log_whole_and_recovers() {
    let Ok(base) = std::env::var("FORGE_FULL_DIR") else {
        eprintln!("FORGE_FULL_DIR (a small filesystem to fill) is not set: skipped");
        return;
    };
    let root = Path::new(&base).join(format!("proj-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("part.fg"), DOC).unwrap();
    let h = serve(&root, "127.0.0.1:0");
    let at = h.addr.to_string();
    let remote = Remote {
        name: "full".into(),
        url: format!("ws://{at}/sync"),
        token: TOKEN.into(),
    };
    let s = Session::start(&remote, "part.fg", None).unwrap();
    wait("joined", || s.ready());
    // room: committed
    s.edit(set(41.0)).unwrap();
    wait("the first edit acked", || s.pending() == 0);
    assert!(s.text().contains("shared w = 41"));
    // no room: refused, said, not applied; the server goes on
    let filler = fill(&root);
    let mut refused = None;
    for k in 0..200 {
        s.edit(set(50.0 + k as f64)).unwrap();
        wait("an answer", || s.pending() == 0);
        if let Some(r) = s.poll().into_iter().find_map(|e| match e {
            forge_sync::Event::Refused { refusal, .. } => Some(refusal),
            _ => None,
        }) {
            refused = Some(r);
            break;
        }
    }
    let refused = refused.expect("an edit refused while the disk was full");
    assert!(
        matches!(refused, forge_sync::Refusal::Store { .. }),
        "refused as not saved: {refused:?}"
    );
    let at_full = s.text();
    // still full, the only member leaves: the document closes, and its checkpoint (the file, a
    // git commit) and its snapshot (a record appended to the log) both fail mid-write
    drop(s);
    std::thread::sleep(Duration::from_millis(800));
    // room again: the document opens from a whole log, at the last acked text
    std::fs::remove_file(&filler).unwrap();
    let s = Session::start(&remote, "part.fg", None).unwrap();
    wait("rejoined after the failed close", || s.ready());
    assert_eq!(s.text(), at_full, "no acked op lost, nothing unacked applied");
    // and edits commit
    s.edit(set(77.0)).unwrap();
    wait("acked once there is room", || {
        s.pending() == 0 && s.text().contains("shared w = 77")
    });
    assert_ne!(at_full, s.text());
    let last = s.text();
    drop(s);
    h.stop();
    // the log reopens whole: every op once, seqs without a gap, the head the last text
    let store = forge_server::store::FileLogStore::new(root.join(".forgecad/server/log")).unwrap();
    let es = store.history("part.fg").unwrap().expect("a log");
    let mut seen = std::collections::HashSet::new();
    for (k, e) in es.iter().enumerate() {
        assert_eq!(e.seq, k as u64 + 1, "seqs run without a gap");
        assert!(seen.insert((e.client.clone(), e.id)), "an op once");
    }
    let h = serve(&root, &at);
    let s = Session::start(&remote, "part.fg", None).unwrap();
    wait("rejoined", || s.ready());
    assert_eq!(s.text(), last, "the server reopened at the last committed text");
    drop(s);
    h.stop();
    let _ = std::fs::remove_dir_all(&root);
}

/// A compaction on a full disk: the ops still fit (each a few hundred bytes) but the snapshot
/// record (the whole document) does not, so every compaction fails mid-write while edits go on
/// being acked. Nothing acked is lost: the part-written snapshot is cut back, the log stays
/// whole, and a restart, with room again, reopens at the last acked text (and compacts then).
#[test]
#[ignore]
fn a_compaction_on_a_full_disk_loses_nothing_acked() {
    let Ok(base) = std::env::var("FORGE_FULL_DIR") else {
        eprintln!("FORGE_FULL_DIR (a small filesystem to fill) is not set: skipped");
        return;
    };
    let root = Path::new(&base).join(format!("compact-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    // a document whose snapshot (~70 KB) is far larger than an op's record
    let mut doc = DOC.to_string();
    for k in 0..4000 {
        doc.push_str(&format!("shared q{k} = 1\n"));
    }
    std::fs::write(root.join("part.fg"), &doc).unwrap();
    let serve_small = |at: &str| {
        let mut c = Config::new(root.to_path_buf(), vec![("dev".into(), TOKEN.into())]);
        c.store = StoreKind::File;
        c.max_log_bytes = Some(600);
        start_on(TcpListener::bind(at).unwrap(), c).unwrap()
    };
    let h = serve_small("127.0.0.1:0");
    let at = h.addr.to_string();
    let remote = Remote {
        name: "full".into(),
        url: format!("ws://{at}/sync"),
        token: TOKEN.into(),
    };
    let s = Session::start(&remote, "part.fg", None).unwrap();
    wait("joined", || s.ready());
    s.edit(set(41.0)).unwrap();
    wait("the first edit acked", || s.pending() == 0);
    // full, then 32 KiB given back: room for dozens of ops, not for one snapshot
    let filler = fill(&root);
    let len = std::fs::metadata(&filler).unwrap().len();
    std::fs::OpenOptions::new()
        .write(true)
        .open(&filler)
        .unwrap()
        .set_len(len.saturating_sub(32 << 10))
        .unwrap();
    let mut acked = 0;
    for k in 0..20 {
        s.edit(set(50.0 + k as f64)).unwrap();
        wait("an answer", || s.pending() == 0);
        let refused = s
            .poll()
            .into_iter()
            .any(|e| matches!(e, forge_sync::Event::Refused { .. }));
        if refused {
            break;
        }
        acked += 1;
    }
    assert!(
        acked >= 5,
        "edits kept being acked while compactions failed: {acked}"
    );
    let last = s.text();
    assert!(
        last.contains(&format!("shared w = {}", 50 + acked - 1)),
        "{acked}"
    );
    drop(s);
    h.stop();
    // the log as written: whole, every op once, seqs without a gap (the failed snapshots cut back)
    let store = forge_server::store::FileLogStore::new(root.join(".forgecad/server/log")).unwrap();
    let es = store.history("part.fg").unwrap().expect("a log");
    assert_eq!(es.len(), 1 + acked, "every acked op is in the log");
    // past the 600-byte log several times over: each of those compactions was tried and failed
    let bytes: usize = es
        .iter()
        .map(|e| forge_sync::wire::write(&forge_sync::wire::entry_json(e)).len())
        .sum();
    assert!(bytes > 3 * 600, "{bytes} bytes of log: compactions were due");
    let mut seen = std::collections::HashSet::new();
    for (k, e) in es.iter().enumerate() {
        assert_eq!(e.seq, k as u64 + 1, "seqs run without a gap");
        assert!(seen.insert((e.client.clone(), e.id)), "an op once");
    }
    // room again: reopened at the last acked text, and it compacts now
    std::fs::remove_file(&filler).unwrap();
    let h = serve_small(&at);
    let s = Session::start(&remote, "part.fg", None).unwrap();
    wait("rejoined", || s.ready());
    assert_eq!(s.text(), last, "reopened at the last acked text");
    for k in 0..10 {
        s.edit(set(80.0 + k as f64)).unwrap();
        wait("acked with room", || s.pending() == 0);
    }
    let last = s.text();
    drop(s);
    h.stop();
    let h = serve_small(&at);
    let s = Session::start(&remote, "part.fg", None).unwrap();
    wait("rejoined after a compaction", || s.ready());
    assert_eq!(s.text(), last);
    assert!(last.contains("shared w = 89"));
    drop(s);
    h.stop();
    let _ = std::fs::remove_dir_all(&root);
}
