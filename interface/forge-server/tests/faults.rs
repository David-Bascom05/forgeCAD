//! Fault injection on the real server binary: app sessions (forge-remote) edit several documents
//! while the server is killed (SIGKILL, mid-op) and restarted on the same port, and sessions are
//! closed mid-edit and taken up again from their saved client (as the app does across a close).
//! Afterwards every session has converged on the server's head, every edit made is in it (none
//! lost), and the log holds each (client, op id) once (none applied twice).
//!
//! The smoke run is short (one kill, one resume). The soak runs for FORGE_SOAK_SECS (default 60):
//!   cargo test --release -p forge-server --test faults -- --ignored --nocapture
#![cfg(not(target_os = "wasi"))] // processes, sockets, threads

use forge_lang::error::{Literal, Op, ParamKey};
use forge_remote::{Remote, Session};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

const USERS: usize = 3;

fn doc_text(params: usize) -> String {
    let mut t = String::new();
    for k in 0..params {
        t.push_str(&format!("shared p{k} = 1\n"));
    }
    t.push_str(
        "sketch plan(on: xy) {\n  o = rect(center: (0, 0), size: (p0, 30))\n}\nbase = extrude(plan, 10)\n",
    );
    t
}

fn token(u: usize) -> String {
    format!("fault-token-0123456789-{u}")
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

struct Server {
    root: PathBuf,
    port: u16,
    child: Option<Child>,
}

impl Server {
    fn start(&mut self) {
        let mut c = Command::new(env!("CARGO_BIN_EXE_forge-server"));
        c.arg(&self.root)
            .args(["--port", &self.port.to_string(), "--store", "file"])
            .stderr(std::process::Stdio::null());
        for u in 0..USERS {
            c.args(["--user", &format!("u{u}:{}", token(u))]);
        }
        self.child = Some(c.spawn().unwrap());
        // listening (a refused connect until then)
        let t0 = Instant::now();
        while std::net::TcpStream::connect(("127.0.0.1", self.port)).is_err() {
            assert!(t0.elapsed() < Duration::from_secs(10), "the server did not start");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    /// SIGKILL: no checkpoint, no snapshot, mid-op.
    fn kill(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.kill();
    }
}

/// One app session: who it is, which document, its next value, the last value it set.
struct App {
    user: usize,
    doc: String,
    param: usize,
    s: Session,
    last: f64,
    edits: usize,
}

fn remote(port: u16, u: usize) -> Remote {
    Remote {
        name: "faults".into(),
        url: format!("ws://127.0.0.1:{port}/sync"),
        token: token(u),
    }
}

fn set(param: usize, v: f64) -> Op {
    Op::SetParam {
        param: ParamKey::parse(&format!("p{param}")).unwrap(),
        value: Literal {
            value: v,
            dim: forge_lang::values::Dim::Length,
        },
    }
}

fn wait(what: &str, secs: u64, mut done: impl FnMut() -> bool) {
    let t0 = Instant::now();
    while !done() {
        assert!(
            t0.elapsed() < Duration::from_secs(secs),
            "timed out waiting for {what}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// The run: `docs` documents, `per_doc` sessions on each, for `secs`; the server killed every
/// `kill_every` and a session closed and resumed every `resume_every`.
fn run(name: &str, docs: usize, per_doc: usize, secs: f64, kill_every: f64, resume_every: f64) {
    let root = std::env::temp_dir().join(format!("forge-faults-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    for d in 0..docs {
        std::fs::write(root.join(format!("d{d}.fg")), doc_text(per_doc)).unwrap();
    }
    let mut srv = Server {
        root: root.clone(),
        port: free_port(),
        child: None,
    };
    srv.start();
    let mut apps: Vec<App> = vec![];
    for d in 0..docs {
        for k in 0..per_doc {
            let user = (d + k) % USERS;
            let doc = format!("d{d}.fg");
            let s = Session::start(&remote(srv.port, user), &doc, None).unwrap();
            apps.push(App {
                user,
                doc,
                param: k,
                s,
                last: 1.0,
                edits: 0,
            });
        }
    }
    wait("every session's first snapshot", 20, || {
        apps.iter().all(|a| a.s.ready())
    });
    let (t0, mut kills, mut resumes, mut next_kill, mut next_resume) =
        (Instant::now(), 0, 0, kill_every, resume_every);
    let mut v = 2.0;
    while t0.elapsed().as_secs_f64() < secs {
        // every session edits its own param (no conflicts: what lands is known exactly)
        for a in apps.iter_mut() {
            if a.s.ready() && a.s.pending() < 8 {
                v += 1.0;
                if a.s.edit(set(a.param, v)).is_ok() {
                    a.last = v;
                    a.edits += 1;
                }
            }
            for e in a.s.poll() {
                if let forge_sync::Event::Refused { refusal, .. } = e {
                    panic!("{}: an edit was refused: {refusal:?}", a.doc);
                }
                if let forge_sync::Event::Stopped(why) = e {
                    panic!("{}: the server refused the session for good: {why}", a.doc);
                }
            }
        }
        let now = t0.elapsed().as_secs_f64();
        if now >= next_kill {
            next_kill += kill_every;
            srv.kill();
            kills += 1;
            std::thread::sleep(Duration::from_millis(50));
            srv.start();
        }
        if now >= next_resume {
            next_resume += resume_every;
            // the app closes mid-edit and opens again: its saved client taken up
            let k = resumes % apps.len();
            let a = &mut apps[k];
            let saved = a.s.saved();
            let s = Session::resume(&remote(srv.port, a.user), &a.doc, Some(&saved), None).unwrap();
            a.s = s; // the old one dropped: its connection closed
            resumes += 1;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    // quiet: everything lands
    wait(
        "every edit acked and every session at its document's head",
        60,
        || {
            apps.iter().all(|a| a.s.ready() && a.s.pending() == 0)
                && apps.iter().all(|a| {
                    apps.iter()
                        .filter(|b| b.doc == a.doc)
                        .all(|b| b.s.seq() == a.s.seq())
                })
        },
    );
    let edits: usize = apps.iter().map(|a| a.edits).sum();
    // converged: each session's text is its document's, and every session's last edit is in it
    let mut heads: HashMap<String, String> = HashMap::new();
    for a in &apps {
        let t = a.s.text();
        let h = heads.entry(a.doc.clone()).or_insert_with(|| t.clone());
        assert_eq!(&t, h, "{}: sessions differ", a.doc);
        assert!(
            t.contains(&format!("shared p{} = {}", a.param, a.last)),
            "{}: p{} lost its last edit {} (kills {kills}, resumes {resumes}):\n{t}",
            a.doc,
            a.param,
            a.last
        );
    }
    // and a fresh session reads the same heads from the server
    for (doc, h) in &heads {
        let f = Session::start(&remote(srv.port, 0), doc, None).unwrap();
        wait("a fresh session", 20, || f.ready());
        assert_eq!(&f.text(), h, "{doc}: the server's head");
    }
    // the log: every (client, op id) once
    let store = forge_server::store::FileLogStore::new(root.join(".forgecad/server/log")).unwrap();
    let mut committed = 0;
    for doc in heads.keys() {
        let es = store.history(doc).unwrap().expect("a log");
        let mut seen = HashSet::new();
        for (k, e) in es.iter().enumerate() {
            assert_eq!(e.seq, k as u64 + 1, "{doc}: seqs run without a gap");
            assert!(
                seen.insert((e.client.clone(), e.id)),
                "{doc}: ({}, {}) committed twice",
                e.client,
                e.id
            );
        }
        committed += es.len();
    }
    assert!(committed <= edits, "committed {committed} of {edits} edits");
    eprintln!(
        "faults {name}: {docs} documents x {per_doc} sessions, {secs} s: {edits} edits, {committed} committed \
         (a later edit of the same param may supersede a pending one), {kills} kills, {resumes} resumes: \
         converged, nothing lost, nothing twice"
    );
    drop(apps);
    drop(srv);
    let _ = std::fs::remove_dir_all(&root);
}

/// One kill mid-op and one close-and-resume, quickly.
#[test]
fn a_kill_and_a_resume_lose_nothing_and_apply_nothing_twice() {
    run("smoke", 2, 2, 1.2, 0.5, 0.8);
}

/// The soak: FORGE_SOAK_SECS (default 60) of edits on 8 documents, 3 sessions each, the server
/// killed every 3 s and a session resumed every 0.7 s.
#[test]
#[ignore]
fn soak_with_kills_and_resumes() {
    let secs = std::env::var("FORGE_SOAK_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60.0);
    run("soak", 8, 3, secs, 3.0, 0.7);
}

#[test]
fn recovered_ack_reaches_disk_on_close_without_another_edit() {
    recovered_close(false);
}

#[test]
fn recovered_disk_rename_finishes_git_checkpoint_after_restart() {
    recovered_close(true);
}

fn recovered_close(renamed: bool) {
    let root = std::env::temp_dir().join(format!("forge-recovered-close-{}-{renamed}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("part.fg");
    std::fs::write(&path, doc_text(1)).unwrap();
    let mut server = Server { root: root.clone(), port: free_port(), child: None };
    server.start();
    let connection = remote(server.port, 0);
    let session = Session::start(&connection, "part.fg", None).unwrap();
    wait("initial snapshot", 10, || session.ready());
    session.edit(set(0, 42.0)).unwrap();
    wait("acknowledged edit", 10, || session.pending() == 0 && session.seq() == 1);
    let expected = session.text();
    server.kill();
    drop(session);
    assert_ne!(std::fs::read_to_string(&path).unwrap(), expected);
    if renamed {
        // State left by an older writer killed after its atomic rename, before its disk mark.
        std::fs::write(&path, &expected).unwrap();
    }
    server.start();
    let reopened = Session::start(&connection, "part.fg", None).unwrap();
    wait("replayed head", 10, || reopened.ready() && reopened.text() == expected);
    drop(reopened);
    wait("checkpoint of recovered edit", 10, || std::fs::read_to_string(&path).unwrap() == expected);
    wait("git checkpoint of recovered edit", 10, || {
        let Some(history) = forge_history::History::open(&path) else { return false };
        let Ok(Some(head)) = history.head() else { return false };
        history.content("part.fg", Some(&head)).ok().flatten().as_deref() == Some(expected.as_bytes())
    });
    drop(server);
    std::fs::remove_dir_all(root).unwrap();
}
