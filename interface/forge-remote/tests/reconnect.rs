//! HOSTED-DESIGN's clients re-review, finding 4: the reconnect rate after a server-side close.
//! It passed while 39 connections in 2 s were made; it now asserts the backoff holds (it resets
//! only after a Welcome) and that a message this client cannot read stops it.
#![cfg(not(target_os = "wasi"))] // sockets and threads
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

fn count(reply: &'static str) -> (usize, Vec<forge_sync::Event>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    let n = Arc::new(AtomicUsize::new(0));
    let n2 = n.clone();
    std::thread::spawn(move || {
        for s in l.incoming() {
            let Ok(s) = s else { continue };
            n2.fetch_add(1, Ordering::SeqCst);
            std::thread::spawn(move || {
                if let Ok((mut r, mut w, _)) = forge_ws::accept(s, 1 << 20, |_| Ok(())) {
                    let _ = r.read(); // hello
                    let _ = w.send(reply);
                    w.close();
                }
            });
        }
    });
    let remote = forge_remote::Remote {
        name: "x".into(),
        url: format!("ws://{addr}/sync"),
        token: "t".into(),
    };
    let s = forge_remote::Session::start(&remote, "part.fg", None).unwrap();
    std::thread::sleep(Duration::from_millis(1200));
    let ev = s.poll();
    drop(s);
    (n.load(Ordering::SeqCst), ev)
}

#[test]
fn reconnects_back_off_and_an_unreadable_server_stops_the_link() {
    let retry = std::thread::spawn(|| count(r#"{"t":"closing","text":"busy: try again","retry":true}"#));
    let unreadable = std::thread::spawn(|| count(r#"{"t":"nonsense"}"#));
    let old = std::thread::spawn(|| count(r#"{"t":"error","text":"protocol 2: this server speaks 1"}"#));
    // closes at once (retry): 50, 100, 200, 400 ms apart: at most 5 in 1.2 s (it was ~25)
    let (n, _) = retry.join().unwrap();
    assert!(n <= 5, "retry:true -> {n} connections in 1.2 s");
    // a message of another protocol: one connection, and the link stops, said
    let (m, ev) = unreadable.join().unwrap();
    assert_eq!(m, 1, "unreadable -> {m} connections");
    assert!(
        ev.iter()
            .any(|e| matches!(e, forge_sync::Event::Stopped(t) if t.contains("protocol"))),
        "{ev:?}"
    );
    // an old server's error (a message only, now) then its close: backed off too
    let (k, _) = old.join().unwrap();
    assert!(k <= 5, "v1 error -> {k} connections in 1.2 s");
}
