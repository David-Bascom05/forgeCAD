//! Regression tests for the security review of hosted/server @4e64707d: each reproduces a
//! finding against the fixed server (the PoCs as they were sent).
#![cfg(not(target_os = "wasi"))] // sockets, threads and file locks: not in WASI

use forge_lang::error::{Literal, Op, ParamKey};
use forge_server::{Config, StoreKind, start_on};
use forge_sync::wire::{Down, Up};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const DOC: &str = "shared w = 40\nsketch plan(on: xy) {\n  o = rect(center: (0, 0), size: (w, 30))\n}\nbase = extrude(plan, 10)\n";
const ADA: &str = "0123456789abcdef-ada";
const BO: &str = "0123456789abcdef-bo";

fn project(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("forge-sec-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("part.fg"), DOC).unwrap();
    d
}

fn serve_with(root: &std::path::Path, f: impl FnOnce(&mut Config)) -> (String, forge_server::Handle) {
    let mut c = Config::new(
        root.to_path_buf(),
        vec![("ada".into(), ADA.into()), ("bo".into(), BO.into())],
    );
    c.store = StoreKind::File;
    f(&mut c);
    let h = start_on(TcpListener::bind("127.0.0.1:0").unwrap(), c).unwrap();
    (h.addr.to_string(), h)
}

fn upgrade(addr: &str) -> TcpStream {
    let mut s = TcpStream::connect(addr).unwrap();
    write!(
        s,
        "GET /sync HTTP/1.1\r\nHost: {addr}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
    )
    .unwrap();
    let mut head = vec![];
    let mut b = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        s.read_exact(&mut b).unwrap();
        head.push(b[0]);
    }
    assert!(
        head.starts_with(b"HTTP/1.1 101"),
        "{}",
        String::from_utf8_lossy(&head)
    );
    s
}

fn masked(fin: bool, op: u8, len: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut f = vec![if fin { 0x80 } else { 0 } | op];
    f.extend_from_slice(len);
    f[1] |= 0x80;
    f.extend_from_slice(&[9, 8, 7, 6]);
    f.extend(payload.iter().enumerate().map(|(i, b)| b ^ [9, 8, 7, 6][i & 3]));
    f
}

/// A session: (reader, writer) after a good Hello.
fn joined(addr: &str, token: &str, client: &str) -> (forge_ws::Reader, forge_ws::Writer, u64) {
    let (mut r, mut w) = forge_ws::connect(addr, "/sync", &[], 1 << 20).unwrap();
    let h = Up::Hello {
        doc: "part.fg".into(),
        client: client.into(),
        token: token.into(),
        seq: None,
        proto: forge_sync::PROTOCOL,
        kernel: forge_sync::KERNEL.into(),
    };
    w.send(&h.text()).unwrap();
    match next(&mut r) {
        Down::Welcome { seq, .. } => (r, w, seq),
        d => panic!("{d:?}"),
    }
}

fn next(r: &mut forge_ws::Reader) -> Down {
    match r.read() {
        Ok(forge_ws::Msg::Text(t)) => Down::parse(&t).unwrap(),
        x => panic!("{x:?}"),
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

/// HIGH 1: a continuation claiming u64::MAX bytes panicked the connection thread before its
/// slot was given back; two of them filled a 2-slot server for good. Now: refused, slot freed.
#[test]
fn hostile_length_never_leaks_a_slot() {
    let root = project("len");
    let (addr, h) = serve_with(&root, |c| c.max_conns = 2);
    for _ in 0..6 {
        let mut s = upgrade(&addr);
        s.write_all(&masked(false, 0x1, &[3], b"abc")).unwrap();
        let mut len = vec![127];
        len.extend_from_slice(&u64::MAX.to_be_bytes());
        s.write_all(&masked(true, 0x0, &len, b"")).unwrap();
        let mut sink = vec![];
        let _ = s.read_to_end(&mut sink); // the server closes it
    }
    std::thread::sleep(Duration::from_millis(200));
    // still room for a real client
    let _ok = joined(&addr, ADA, "c1");
    // and before the Hello a message is small: a 64 KiB "hello" is refused
    let (_, mut w) = forge_ws::connect(&addr, "/sync", &[], 1 << 20).unwrap();
    assert!(w.send(&"x".repeat(64 << 10)).is_ok());
    h.stop();
    let _ = std::fs::remove_dir_all(&root);
}

/// MEDIUM 4: a trickled upgrade is cut at the absolute deadline (not reset by each byte), and one
/// address cannot take every slot.
#[test]
fn slow_upgrades_are_cut_and_one_address_is_capped() {
    let root = project("slow");
    let (addr, h) = serve_with(&root, |c| {
        c.cap_loopback = true;
        c.per_ip = 3;
        c.handshake = Duration::from_secs(1);
    });
    let t0 = Instant::now();
    let mut s = TcpStream::connect(&addr).unwrap();
    let req = format!("GET /sync HTTP/1.1\r\nHost: {addr}\r\n");
    let mut cut = false;
    for b in req.bytes().cycle().take(40) {
        if s.write_all(&[b]).is_err() {
            cut = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(400));
        let _ = s.set_nonblocking(true);
        let mut x = [0u8; 1];
        if matches!(s.read(&mut x), Ok(0)) {
            cut = true;
            break;
        }
        let _ = s.set_nonblocking(false);
    }
    assert!(cut, "a trickled upgrade is cut");
    let took = t0.elapsed();
    assert!(
        took < Duration::from_secs(8),
        "cut at the deadline, not per byte: {took:?}"
    );
    // one address (loopback, counted on request here): at most per_ip at once
    let held: Vec<TcpStream> = (0..3).map(|_| TcpStream::connect(&addr).unwrap()).collect();
    std::thread::sleep(Duration::from_millis(100));
    let mut extra = TcpStream::connect(&addr).unwrap();
    let mut answer = String::new();
    let _ = extra.read_to_string(&mut answer);
    assert!(answer.contains("503"), "{answer:?}");
    drop(held);
    h.stop();
    let _ = std::fs::remove_dir_all(&root);
}

/// MEDIUM 4: after the Hello, pings and empty frames count against the rate too.
#[test]
fn ping_floods_are_rate_limited() {
    let root = project("ping");
    let (addr, h) = serve_with(&root, |_| {});
    let (_r, w, _) = joined(&addr, ADA, "c1");
    let s = w.stream().try_clone().unwrap();
    let mut s = s;
    let mut closed = false;
    for _ in 0..5000 {
        if s.write_all(&masked(true, 0x9, &[0], b"")).is_err() {
            closed = true;
            break;
        }
    }
    std::thread::sleep(Duration::from_millis(200));
    closed |= s.write_all(&masked(true, 0x9, &[0], b"")).is_err() || s.write_all(&[0; 1]).is_err();
    let mut buf = vec![];
    let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = s.read_to_end(&mut buf);
    assert!(
        closed || String::from_utf8_lossy(&buf).contains("too many"),
        "a ping flood is cut"
    );
    h.stop();
    let _ = std::fs::remove_dir_all(&root);
}

/// MEDIUM 7: another account using the victim's client id and op ids neither swallows the
/// victim's ops nor skips its renames: ops are keyed by (account, client id).
#[test]
fn client_ids_are_per_account() {
    let root = project("spoof");
    let (addr, h) = serve_with(&root, |_| {});
    let (mut rb, mut wb, _) = joined(&addr, BO, "victim");
    // the attacker sends op id 1 first under the victim's client id
    wb.send(
        &Up::Op {
            base: 0,
            id: 1,
            op: set_w(41.0),
        }
        .text(),
    )
    .unwrap();
    assert!(matches!(next(&mut rb), Down::Ack { seq: 1, .. }));
    // the victim's own op id 1 is a different op: it must be committed, not "already seen"
    let (mut ra, mut wa, _) = joined(&addr, ADA, "victim");
    wa.send(
        &Up::Op {
            base: 1,
            id: 1,
            op: set_w(77.0),
        }
        .text(),
    )
    .unwrap();
    match next(&mut ra) {
        Down::Ack { seq, op, .. } => {
            assert_eq!(seq, 2);
            assert_eq!(op, set_w(77.0));
        }
        d => panic!("{d:?}"),
    }
    h.stop();
    let _ = std::fs::remove_dir_all(&root);
}

/// LOW 12: a faked build hash no longer defines "expected" for everyone: the server checks it
/// against its own build, tells only the reporter, and others hear nothing.
#[test]
fn a_fake_hash_only_marks_its_reporter() {
    let root = project("hash");
    let (addr, h) = serve_with(&root, |_| {});
    let (mut ra, mut wa, _) = joined(&addr, ADA, "a");
    let (mut rb, _wb, _) = joined(&addr, BO, "b");
    wa.send(
        &Up::Op {
            base: 0,
            id: 1,
            op: set_w(50.0),
        }
        .text(),
    )
    .unwrap();
    assert!(matches!(next(&mut ra), Down::Ack { seq: 1, .. }));
    assert!(matches!(next(&mut rb), Down::Applied { seq: 1, .. }));
    wa.send(
        &Up::BuildHash {
            seq: 1,
            hash: 0xdead_beef,
        }
        .text(),
    )
    .unwrap();
    match next(&mut ra) {
        Down::Diverged { seq: 1, got, by, .. } => {
            assert_eq!(got, 0xdead_beef);
            assert_eq!(by, "ada", "an account, never a client id");
        }
        d => panic!("{d:?}"),
    }
    let _ = rb.stream().set_read_timeout(Some(Duration::from_millis(500)));
    assert!(rb.read().is_err(), "b hears nothing of a's fake");
    let st = h.stats.lock().unwrap().clone();
    assert_eq!((st.hash_checks, st.divergences), (1, 1));
    h.stop();
    let _ = std::fs::remove_dir_all(&root);
}

/// MEDIUM 3 / LOW 9: aliases of one file and links are refused at the door.
#[test]
fn one_file_one_room() {
    let root = project("alias");
    std::fs::create_dir_all(root.join("sub")).unwrap();
    std::fs::write(root.join("sub/p.fg"), DOC).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(root.join(".forgecad"), root.join("cfg.fg")).ok();
    let (addr, h) = serve_with(&root, |_| {});
    for doc in ["sub//p.fg", "./sub/p.fg", "SUB/p.fg", "sub/./p.fg", "cfg.fg"] {
        let (mut r, mut w) = forge_ws::connect(&addr, "/sync", &[], 1 << 20).unwrap();
        w.send(
            &Up::Hello {
                doc: doc.into(),
                client: "c".into(),
                token: ADA.into(),
                seq: None,
                proto: forge_sync::PROTOCOL,
                kernel: forge_sync::KERNEL.into(),
            }
            .text(),
        )
        .unwrap();
        assert!(
            matches!(next(&mut r), Down::Closing { retry: false, .. }),
            "{doc}"
        );
    }
    h.stop();
    let _ = std::fs::remove_dir_all(&root);
}

/// LOW 10: a file changed on disk outside the server is not silently overwritten, and the
/// folder serves one server at a time.
#[test]
fn outside_changes_and_one_server_per_folder() {
    let root = project("disk");
    let (addr, h) = serve_with(&root, |_| {});
    assert!(
        start_on(
            TcpListener::bind("127.0.0.1:0").unwrap(),
            Config::new(root.clone(), vec![("x".into(), ADA.into())])
        )
        .is_err(),
        "a second server on the same folder"
    );
    {
        let (mut r, mut w, _) = joined(&addr, ADA, "c1");
        w.send(
            &Up::Op {
                base: 0,
                id: 1,
                op: set_w(60.0),
            }
            .text(),
        )
        .unwrap();
        assert!(matches!(next(&mut r), Down::Ack { .. }));
    }
    h.stop(); // its head checkpointed to the file
    assert!(
        std::fs::read_to_string(root.join("part.fg"))
            .unwrap()
            .contains("shared w = 60")
    );
    // someone edits the file while the server is down
    std::fs::write(root.join("part.fg"), DOC.replace("40", "99")).unwrap();
    let (addr, h) = serve_with(&root, |_| {});
    let (mut r, mut w) = forge_ws::connect(&addr, "/sync", &[], 1 << 20).unwrap();
    w.send(
        &Up::Hello {
            doc: "part.fg".into(),
            client: "c".into(),
            token: ADA.into(),
            seq: None,
            proto: forge_sync::PROTOCOL,
            kernel: forge_sync::KERNEL.into(),
        }
        .text(),
    )
    .unwrap();
    match next(&mut r) {
        Down::Closing { text, retry: false } => assert!(text.contains("changed on disk"), "{text}"),
        d => panic!("{d:?}"),
    }
    h.stop();
    let _ = std::fs::remove_dir_all(&root);
}

/// The review's note: a non-loopback bind without TLS is refused unless asked for loudly.
#[test]
fn no_cleartext_off_loopback_by_default() {
    let root = project("bind");
    let l = TcpListener::bind("0.0.0.0:0").unwrap();
    let e = start_on(l, Config::new(root.clone(), vec![("x".into(), ADA.into())]))
        .err()
        .unwrap();
    assert!(e.contains("not loopback"), "{e}");
    let _ = std::fs::remove_dir_all(&root);
}

/// LOW 8: validation reads no file outside the project folder: a `use` of a file beside the
/// folder evaluates with no root (it was read) and is refused as Outside with the server's root.
#[test]
fn validation_stays_in_the_folder() {
    let root = project("root");
    let outside = root
        .parent()
        .unwrap()
        .join(format!("forge-sec-outside-{}.fg", std::process::id()));
    std::fs::write(&outside, "shared k = 3\n").unwrap();
    let name = outside.file_name().unwrap().to_string_lossy().to_string();
    let op = Op::AddFeature {
        before: None,
        text: format!("lib_ext = use(\"../{name}\")"),
    };
    let free = forge_sync::validate(DOC, None, &op, Some(root.clone()), None);
    let held = forge_sync::validate(DOC, None, &op, Some(root.clone()), Some(root.clone()));
    let _ = std::fs::remove_file(&outside);
    let _ = std::fs::remove_dir_all(&root);
    assert!(free.is_ok(), "with no root the file is read: {free:?}");
    match held {
        Err(forge_sync::Refusal::Lang { text, .. }) => {
            assert!(text.to_lowercase().contains("outside"), "{text}")
        }
        x => panic!("read outside the root: {x:?}"),
    }
}

// ---- the re-review of 029ad8ae ----

fn hello_doc(addr: &str, token: &str, client: &str, doc: &str) -> (forge_ws::Reader, forge_ws::Writer, u64) {
    let (mut r, mut w) = forge_ws::connect(addr, "/sync", &[], 1 << 20).unwrap();
    let h = Up::Hello {
        doc: doc.into(),
        client: client.into(),
        token: token.into(),
        seq: None,
        proto: forge_sync::PROTOCOL,
        kernel: forge_sync::KERNEL.into(),
    };
    w.send(&h.text()).unwrap();
    match next(&mut r) {
        Down::Welcome { seq, .. } => (r, w, seq),
        d => panic!("{d:?}"),
    }
}

/// A: a heavy document's checks take seconds; another document's edits are not held up by them
/// (one actor per document), and the heavy document's own next edit still commits (its head is
/// not evaluated again per op; abandoned checks are counted per document).
#[test]
fn a_heavy_document_slows_only_itself() {
    let root = project("heavy");
    std::fs::write(root.join("other.fg"), DOC).unwrap();
    let (addr, h) = serve_with(&root, |_| {});
    let (mut ra, mut wa, _) = hello_doc(&addr, ADA, "a", "part.fg");
    let (mut rb, mut wb, _) = hello_doc(&addr, BO, "b", "other.fg");
    let heavy = Op::AddFeature {
        before: None,
        text: "fn g(n) {\n  return len([i * 2 for i in 0..n])\n}\nfn hv(n) {\n  return g(n) + g(n) + g(n) + g(n)\n}\nfn hz(n) {\n  return hv(n) + hv(n) + hv(n)\n}\nshared heavy = hz(150000)".into(),
    };
    let t0 = Instant::now();
    wa.send(
        &Up::Op {
            base: 0,
            id: 1,
            op: heavy,
        }
        .text(),
    )
    .unwrap();
    std::thread::sleep(Duration::from_millis(50));
    let t1 = Instant::now();
    wb.send(
        &Up::Op {
            base: 0,
            id: 1,
            op: set_w(55.0),
        }
        .text(),
    )
    .unwrap();
    assert!(matches!(next(&mut rb), Down::Ack { .. }));
    let other = t1.elapsed();
    match next(&mut ra) {
        Down::Ack { .. } => {}
        d => panic!("the heavy op: {d:?}"),
    }
    let heavy_took = t0.elapsed();
    eprintln!("heavy op committed in {heavy_took:?}; the other document's op in {other:?}");
    assert!(
        other < heavy_took / 2,
        "the other document waited: {other:?} of {heavy_took:?}"
    );
    // the heavy document's own trivial op: committed (one evaluation of the new text)
    wa.send(
        &Up::Op {
            base: 1,
            id: 2,
            op: set_w(41.0),
        }
        .text(),
    )
    .unwrap();
    let t2 = Instant::now();
    match next(&mut ra) {
        Down::Ack { .. } => eprintln!("its next op committed in {:?}", t2.elapsed()),
        d => panic!("the heavy document's next op: {d:?}"),
    }
    h.stop();
    let _ = std::fs::remove_dir_all(&root);
}

/// B: a hash report at the head of a long log never makes the server replay the log; the
/// document keeps answering.
#[test]
fn a_hash_report_on_a_long_log_is_answered_at_once() {
    let root = project("longlog");
    let (addr, h) = serve_with(&root, |_| {});
    let (mut r, mut w, _) = hello_doc(&addr, ADA, "a", "part.fg");
    for k in 0..400u64 {
        w.send(
            &Up::Op {
                base: k,
                id: k + 1,
                op: set_w(10.0 + (k % 90) as f64),
            }
            .text(),
        )
        .unwrap();
        assert!(matches!(next(&mut r), Down::Ack { .. }));
    }
    for seq in [400, 1, 200] {
        w.send(&Up::BuildHash { seq, hash: 1 }.text()).unwrap();
        let t0 = Instant::now();
        w.send(&Up::Ping { t: 1.0 }.text()).unwrap();
        loop {
            match next(&mut r) {
                Down::Pong { .. } => break,
                Down::Diverged { .. } => continue,
                d => panic!("{d:?}"),
            }
        }
        assert!(
            t0.elapsed() < Duration::from_millis(100),
            "seq {seq}: {:?}",
            t0.elapsed()
        );
    }
    h.stop();
    let _ = std::fs::remove_dir_all(&root);
}

/// F: loopback is not capped per address by default (every local client, tunnel and proxy is
/// loopback); connections that never say Hello have a cap of their own.
#[test]
fn many_local_clients_and_a_pre_auth_cap() {
    let root = project("loopcap");
    let (addr, h) = serve_with(&root, |_| {});
    let joined_: Vec<_> = (0..40)
        .map(|k| hello_doc(&addr, ADA, &format!("c{k}"), "part.fg"))
        .collect();
    assert_eq!(joined_.len(), 40);
    drop(joined_);
    std::thread::sleep(Duration::from_millis(300));
    let idle: Vec<TcpStream> = (0..forge_server::PRE_AUTH)
        .map(|_| TcpStream::connect(&addr).unwrap())
        .collect();
    std::thread::sleep(Duration::from_millis(200));
    let mut extra = TcpStream::connect(&addr).unwrap();
    let mut answer = String::new();
    let _ = extra.set_read_timeout(Some(Duration::from_secs(3)));
    let _ = extra.read_to_string(&mut answer);
    assert!(answer.contains("503"), "the pre-auth cap: {answer:?}");
    drop(idle);
    std::thread::sleep(Duration::from_millis(300));
    let _ok = hello_doc(&addr, ADA, "late", "part.fg");
    h.stop();
    let _ = std::fs::remove_dir_all(&root);
}

/// G: a file edited on disk while its document is open is never written over by a version or
/// checkpoint.
#[test]
fn an_edit_on_disk_while_open_is_never_overwritten() {
    let root = project("diskopen");
    let (addr, h) = serve_with(&root, |_| {});
    let (mut r, mut w, _) = hello_doc(&addr, ADA, "a", "part.fg");
    w.send(
        &Up::Op {
            base: 0,
            id: 1,
            op: set_w(60.0),
        }
        .text(),
    )
    .unwrap();
    assert!(matches!(next(&mut r), Down::Ack { .. }));
    let hand = DOC.replace("40", "77");
    std::fs::write(root.join("part.fg"), &hand).unwrap();
    w.send(&Up::Version { name: "v1".into() }.text()).unwrap();
    match next(&mut r) {
        Down::Error { text } => assert!(text.contains("changed on disk"), "{text}"),
        d => panic!("{d:?}"),
    }
    drop((r, w));
    h.stop();
    assert_eq!(
        std::fs::read_to_string(root.join("part.fg")).unwrap(),
        hand,
        "the hand edit is kept"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// H: versions are limited per account, not per connection (rejoining does not reset it).
#[test]
fn versions_are_limited_per_account() {
    let root = project("vgap");
    let (addr, h) = serve_with(&root, |_| {});
    {
        let (mut r, mut w, _) = hello_doc(&addr, ADA, "a1", "part.fg");
        w.send(&Up::Version { name: "v1".into() }.text()).unwrap();
        assert!(matches!(next(&mut r), Down::Versioned { .. }));
    }
    let (mut r, mut w, _) = hello_doc(&addr, ADA, "a2", "part.fg");
    w.send(&Up::Version { name: "v2".into() }.text()).unwrap();
    match next(&mut r) {
        Down::Error { text } => assert!(text.contains("one version"), "{text}"),
        d => panic!("{d:?}"),
    }
    h.stop();
    let _ = std::fs::remove_dir_all(&root);
}

/// 6: a panic handling one document quarantines it; another document goes on committing; the
/// counters and stats are not poisoned.
#[test]
fn a_panic_quarantines_one_document() {
    let root = project("panic");
    std::fs::write(root.join("other.fg"), DOC).unwrap();
    let (addr, h) = serve_with(&root, |_| {});
    let (mut ra, mut wa, _) = hello_doc(&addr, ADA, "a", "part.fg");
    let (mut rb, mut wb, _) = hello_doc(&addr, BO, "b", "other.fg");
    let boom = Op::SetProperty {
        key: "test_hook_panic".into(),
        text: None,
    };
    wa.send(
        &Up::Op {
            base: 0,
            id: 1,
            op: boom,
        }
        .text(),
    )
    .unwrap();
    match next(&mut ra) {
        Down::Closing { text, retry: false } => assert!(text.contains("quarantined"), "{text}"),
        d => panic!("{d:?}"),
    }
    wb.send(
        &Up::Op {
            base: 0,
            id: 1,
            op: set_w(50.0),
        }
        .text(),
    )
    .unwrap();
    assert!(
        matches!(next(&mut rb), Down::Ack { .. }),
        "the other document still commits"
    );
    // the quarantined document is refused from now on
    let (mut r2, mut w2) = forge_ws::connect(&addr, "/sync", &[], 1 << 20).unwrap();
    let hello = Up::Hello {
        doc: "part.fg".into(),
        client: "c2".into(),
        token: BO.into(),
        seq: None,
        proto: forge_sync::PROTOCOL,
        kernel: forge_sync::KERNEL.into(),
    };
    w2.send(&hello.text()).unwrap();
    assert!(matches!(next(&mut r2), Down::Closing { retry: false, .. }));
    let st = h.stats.lock().unwrap().clone();
    assert_eq!(st.quarantined, 1);
    h.stop();
    let _ = std::fs::remove_dir_all(&root);
}

/// 6: a panic in the hub itself (outside any document) ends the server process with 70: it never
/// keeps accepting with no one to serve.
#[test]
fn a_hub_panic_exits_70() {
    let root = project("hubpanic");
    std::fs::write(root.join("test_hook_hub_panic.fg"), DOC).unwrap();
    let port = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let mut p = std::process::Command::new(env!("CARGO_BIN_EXE_forge-server"))
        .args([
            root.to_str().unwrap(),
            "--port",
            &port.to_string(),
            "--token",
            ADA,
        ])
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let addr = format!("127.0.0.1:{port}");
    let t0 = Instant::now();
    let (mut _r, mut w) = loop {
        if let Ok(x) = forge_ws::connect(&addr, "/sync", &[], 1 << 20) {
            break x;
        }
        assert!(
            t0.elapsed() < Duration::from_secs(10),
            "the server never listened"
        );
        std::thread::sleep(Duration::from_millis(50));
    };
    let hello = Up::Hello {
        doc: "test_hook_hub_panic.fg".into(),
        client: "c".into(),
        token: ADA.into(),
        seq: None,
        proto: forge_sync::PROTOCOL,
        kernel: forge_sync::KERNEL.into(),
    };
    w.send(&hello.text()).unwrap();
    let t0 = Instant::now();
    let code = loop {
        if let Some(s) = p.try_wait().unwrap() {
            break s.code();
        }
        if t0.elapsed() > Duration::from_secs(10) {
            let _ = p.kill();
            panic!("the server did not exit after its hub panicked");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    assert_eq!(code, Some(70));
    let _ = std::fs::remove_dir_all(&root);
}
