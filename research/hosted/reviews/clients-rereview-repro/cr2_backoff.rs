//! clients re-review scratch (not committed): reconnect rate after a server-side close.
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
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
    let remote = forge_remote::Remote { name: "x".into(), url: format!("ws://{addr}/sync"), token: "t".into() };
    let s = forge_remote::Session::start(&remote, "part.fg", None).unwrap();
    std::thread::sleep(Duration::from_secs(2));
    let ev = s.poll();
    drop(s);
    (n.load(Ordering::SeqCst), ev)
}

#[test]
fn reconnect_rate_after_a_retry_close() {
    let (n, ev) = count(r#"{"t":"closing","text":"this document is too busy: try again","retry":true}"#);
    eprintln!("retry:true -> {n} connections in 2 s; events {}", ev.len());
    // v1 client's view of a v2 server: an unparseable message
    let (m, ev2) = count(r#"{"t":"nonsense"}"#);
    eprintln!("unparseable -> {m} connections in 2 s; events {}", ev2.len());
    // v2 client vs a v1 server refusing its protocol
    let (k, ev3) = count(r#"{"t":"error","text":"protocol 2: this server speaks 1"}"#);
    eprintln!("v1 error -> {k} connections in 2 s; events {:?}", ev3.first());
    assert!(n > 15 && m > 15 && k > 15, "{n} {m} {k}");
}
