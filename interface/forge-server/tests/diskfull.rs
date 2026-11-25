//! A full disk under a running server: an edit that cannot be written durably is refused
//! ("not saved", never acked), the server keeps serving, the log stays whole (a part-written
//! record is cut back), and once there is room again edits commit, and a restart reopens the log
//! with every committed op once and its seqs without a gap.
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
    // room again: edits commit
    std::fs::remove_file(&filler).unwrap();
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
