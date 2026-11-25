//! The desktop app as a client of the hosted server (research/hosted/DESIGN.md §14–§15), headless:
//! two of the app's own part sessions (`fg::Session`, the code behind the 3D Model tab) open one
//! file of a SERVED project, edit it at once through their usual typed-op path, and see each
//! other's edits; a Rename race resolves the same on both; hosted undo keeps the collaborator's
//! edit; every build of the server's text reports its body hash and the server sees no
//! divergence. A LOCAL project (no remote.toml) gets no link at all.

use forge_lang::error::{Literal, Op, ParamKey};
use forge_render::fg::{Msg, Session};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

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
";

fn folder(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("forge-served-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("parts")).unwrap();
    std::fs::write(d.join("parts/part.fg"), DOC).unwrap();
    d
}

const TOKEN: &str = "0123456789abcdef-served";

fn open(path: &std::path::Path) -> Session {
    let waker: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    Session::open(path.to_str().unwrap(), None, waker).unwrap()
}

/// Polls both sessions until `done` holds (builds and link news both arrive through `poll`).
fn until(ss: &mut [&mut Session], what: &str, done: impl Fn(&[&mut Session]) -> bool) -> Vec<Msg> {
    let t0 = Instant::now();
    let mut got = vec![];
    loop {
        for s in ss.iter_mut() {
            got.extend(s.poll());
        }
        if done(ss) {
            return got;
        }
        assert!(
            t0.elapsed() < Duration::from_secs(30),
            "timed out waiting for {what}"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn synced(ss: &[&mut Session]) -> bool {
    let l: Vec<_> = ss.iter().map(|s| s.sync.as_ref().unwrap()).collect();
    l.iter().all(|l| l.ready() && l.pending() == 0)
        && l.iter().all(|x| x.seq() == l[0].seq())
        && ss.iter().all(|s| s.src == ss[0].src)
}

#[test]
fn two_app_sessions_edit_one_served_part() {
    let root = folder("two");
    let mut c = forge_server::Config::new(root.clone(), vec![("dev".into(), TOKEN.into())]);
    c.port = 0;
    let h = forge_server::start_on(std::net::TcpListener::bind("127.0.0.1:0").unwrap(), c).unwrap();
    // the project only names its remote; the address and token are the user's
    std::fs::create_dir_all(root.join(".forgecad")).unwrap();
    std::fs::write(root.join(".forgecad/remote.toml"), "remote = \"lab\"\n").unwrap();
    let user = root
        .parent()
        .unwrap()
        .join(format!("forge-served-remotes-{}.toml", std::process::id()));
    std::fs::write(
        &user,
        format!(
            "[remote.lab]\nurl = \"ws://{}/sync\"\ntoken = \"{TOKEN}\"\n",
            h.addr
        ),
    )
    .unwrap();
    forge_remote::set_user_file(Some(user.clone()));
    let file = root.join("parts/part.fg");
    let (mut a, mut b) = (open(&file), open(&file));
    assert!(
        a.sync.is_some() && b.sync.is_some(),
        "a SERVED project gets a link"
    );
    until(&mut [&mut a, &mut b], "the snapshots", synced);

    // both edit at once through the app's own edit path
    let lit = |v| Literal {
        value: v,
        dim: forge_lang::values::Dim::Length,
    };
    a.apply(Op::SetParam {
        param: ParamKey::parse("w").unwrap(),
        value: lit(55.0),
    })
    .unwrap();
    b.apply(Op::SetParam {
        param: ParamKey::parse("h").unwrap(),
        value: lit(7.0),
    })
    .unwrap();
    until(&mut [&mut a, &mut b], "both params", synced);
    assert!(
        a.src.contains("shared w = 55") && a.src.contains("shared h = 7"),
        "{}",
        a.src
    );
    // the server checks reported hashes against its own builds

    // a Rename race: one wins, both show the same
    a.apply(Op::Rename {
        from: "boss".into(),
        to: "hub".into(),
    })
    .unwrap();
    b.apply(Op::Rename {
        from: "boss".into(),
        to: "pin".into(),
    })
    .unwrap();
    until(&mut [&mut a, &mut b], "the rename race", synced);
    assert!(
        a.src.contains("hub = extrude") ^ a.src.contains("pin = extrude"),
        "{}",
        a.src
    );
    assert!(
        a.sync_note.is_some() || b.sync_note.is_some(),
        "the loser is told"
    );

    // hosted undo: a's last op taken back, b's edit stays
    let renamed = if a.src.contains("hub = ") { "hub" } else { "pin" };
    b.apply(Op::SetParam {
        param: ParamKey::parse(&format!("{renamed}.d")).unwrap(),
        value: lit(9.5),
    })
    .unwrap();
    until(&mut [&mut a, &mut b], "b's edit", synced);
    let before_undo = a.src.clone();
    a.apply(Op::SetParam {
        param: ParamKey::parse("w").unwrap(),
        value: lit(61.0),
    })
    .unwrap();
    until(&mut [&mut a, &mut b], "a's edit", synced);
    assert!(a.undo());
    until(&mut [&mut a, &mut b], "the undo", synced);
    assert_eq!(a.src, before_undo, "undo took back only a's op");
    assert!(b.src.contains(&format!("param {renamed}.d = 9.5")));

    // a raw text change is not shared in phase 1: refused and said
    assert!(!a.set_units("in") || a.src == b.src);

    // builds of the server's text report their hashes: let both settle their builds
    // and the server checks one against its own build of that seq
    let t0 = Instant::now();
    while h.stats.lock().unwrap().hash_checks < 1 {
        assert!(
            t0.elapsed() < Duration::from_secs(30),
            "no reported hash was checked"
        );
        a.poll();
        b.poll();
        std::thread::sleep(Duration::from_millis(5));
    }
    let st = h.stats.lock().unwrap().clone();
    assert_eq!(st.divergences, 0);
    assert!(
        st.hashes >= 2,
        "builds of the server's text reported their hashes: {}",
        st.hashes
    );
    eprintln!(
        "served app sessions: committed {}, refused {}, build hashes {}, checked against the server's build {}",
        st.committed, st.refused, st.hashes, st.hash_checks
    );

    // an assembly reports its placed parts' hash, and it matches the server's own build
    std::fs::copy(t_repo().join("examples/unified/hinged_box.fg"), root.join("parts/box.fg")).unwrap();
    let mut asm = open(&root.join("parts/box.fg"));
    let checks = h.stats.lock().unwrap().hash_checks;
    let t0 = Instant::now();
    let mut built_ok = vec![];
    while h.stats.lock().unwrap().hash_checks <= checks {
        for m in asm.poll() {
            if let Msg::Built(b) = m {
                built_ok.push((b.seq, b.ok, b.checked, b.inspect.as_ref().map(|i| (i.asm.is_some(), i.placed.len(), i.bodies.len()))));
            }
        }
        assert!(t0.elapsed() < Duration::from_secs(60), "asm builds: {built_ok:?}");
        assert!(
            t0.elapsed() < Duration::from_secs(60),
            "the assembly's hash was never checked: {:?}; note {:?}",
            h.stats.lock().unwrap(),
            asm.sync_note
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(h.stats.lock().unwrap().divergences, 0, "the app's assembly hash is the server's");
    drop(asm);

    // the served file follows the server's text...
    let file_now = || std::fs::read_to_string(&file).unwrap();
    let t0 = Instant::now();
    while file_now() != a.src {
        assert!(t0.elapsed() < Duration::from_secs(10), "the file never took the server's text");
        a.poll();
        b.poll();
        std::thread::sleep(Duration::from_millis(20));
    }
    // ...and a hand edit to it pauses syncing, the edit kept, never written over
    let hand = format!("{}# a hand edit\n", a.src);
    std::fs::write(&file, &hand).unwrap();
    let t0 = Instant::now();
    while a.sync.is_some() {
        assert!(t0.elapsed() < Duration::from_secs(10), "a hand edit was never noticed");
        a.poll();
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(a.sync_note.as_deref().is_some_and(|n| n.contains("edited outside")), "{:?}", a.sync_note);
    assert_eq!(file_now(), hand, "the hand edit is kept");
    // reopening it now: still paused (the mark says the file is not what the app wrote)
    let again = open(&file);
    assert!(again.sync.is_none() && again.sync_note.is_some());
    let _ = std::fs::remove_dir_all(&root);
}

fn t_repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn a_local_project_gets_no_link() {
    let root = folder("local");
    let s = open(&root.join("parts/part.fg"));
    assert!(s.sync.is_none());
    // and a remote.toml that is there but wrong is said, not ignored
    std::fs::create_dir_all(root.join(".forgecad")).unwrap();
    std::fs::write(
        root.join(".forgecad/remote.toml"),
        "url = \"ws://evil.example/sync\"\n",
    )
    .unwrap();
    let waker: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    assert!(Session::open(root.join("parts/part.fg").to_str().unwrap(), None, waker).is_err());
    let _ = std::fs::remove_dir_all(&root);
}
