//! M5 over the wire: an apply-all sent as one group reaches a collaborator as one, is one record
//! in the file log, and replays as one when the server reopens the document.
#![cfg(not(target_os = "wasi"))] // sockets, threads

use forge_lang::error::{Literal, Op, ParamKey};
use forge_remote::{Remote, Session};
use forge_server::{Config, StoreKind, start_on};
use std::net::TcpListener;
use std::path::Path;
use std::time::{Duration, Instant};

const DOC: &str = "shared w = 40\nshared h = 12\nsketch plan(on: xy) {\n  o = rect(center: (0, 0), size: (w, 30))\n}\nbase = extrude(plan, h)\n";
const TOKEN: &str = "0123456789abcdef-groups";

fn set(p: &str, v: f64) -> Op {
    Op::SetParam {
        param: ParamKey::parse(p).unwrap(),
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

#[test]
fn a_group_crosses_the_wire_as_one_and_replays_as_one() {
    let root = std::env::temp_dir().join(format!("forge-groups-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("part.fg"), DOC).unwrap();
    let h = serve(&root, "127.0.0.1:0");
    let at = h.addr.to_string();
    let remote = Remote {
        name: "g".into(),
        url: format!("ws://{at}/sync"),
        token: TOKEN.into(),
    };
    let ada = Session::start(&remote, "part.fg", None).unwrap();
    let bob = Session::start(&remote, "part.fg", None).unwrap();
    wait("joined", || ada.ready() && bob.ready());
    // through h = 0 (invalid alone) to the rename: checked as a whole, committed as one
    ada.edit_group(vec![
        set("h", 0.0),
        set("h", 7.0),
        Op::Rename {
            from: "w".into(),
            to: "width".into(),
        },
    ])
    .unwrap();
    wait("acked and seen", || {
        ada.pending() == 0 && bob.seq() == 1 && bob.text().contains("shared width = 40")
    });
    assert_eq!(ada.text(), bob.text());
    assert!(bob.text().contains("shared h = 7"));
    let last = bob.text();
    drop((ada, bob));
    h.stop();
    let store = forge_server::store::FileLogStore::new(root.join(".forgecad/server/log")).unwrap();
    let es = store.history("part.fg").unwrap().expect("a log");
    assert_eq!(es.len(), 1, "one record: {es:?}");
    assert_eq!(es[0].ops.len(), 3);
    assert_eq!(
        es[0].inv.as_ref().map(Vec::len),
        Some(3),
        "its inverse is a group"
    );
    // reopened: the group replays as one, at the same text
    let h = serve(&root, &at);
    let s = Session::start(&remote, "part.fg", None).unwrap();
    wait("rejoined", || s.ready());
    assert_eq!(s.text(), last);
    assert_eq!(s.seq(), 1);
    drop(s);
    h.stop();
    let _ = std::fs::remove_dir_all(&root);
}
