//! A SERVED project on the desktop (research/hosted/DESIGN.md §14–§15).
//!
//! A project is SERVED iff its folder holds `.forgecad/remote.toml`; the app checks [`remote`]
//! **once, when the project opens**. In a LOCAL project nothing here runs: no thread, no socket.
//!
//! A [`Session`] is one document's connection: a forge-sync [`Client`] behind a mutex, a thread
//! that connects, reads, reconnects with backoff and pings, and the app's calls (an edit the app
//! already applied and checked, a build's hash, undo, a version). Events are queued for the app to
//! [`Session::poll`] (and a waker, if given, is called so the app redraws).

use forge_lang::error::Op;
use forge_sync::wire::{Down, MAX_MESSAGE, Up};
use forge_sync::{Client, Event};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// `.forgecad/remote.toml`: where the project is served.
#[derive(Clone, Debug, PartialEq)]
pub struct Remote {
    /// `ws://HOST:PORT/sync` (phase 1: no TLS, so loopback or a LAN).
    pub url: String,
    /// The dev token (phase 1), or read from the environment variable `token_env` names.
    pub token: String,
}

/// The project's remote, if it is SERVED: None for a LOCAL project (no file), Err for a file
/// that is there but unreadable or wrong (the app says so rather than silently going LOCAL).
pub fn remote(project: &Path) -> Option<Result<Remote, String>> {
    let p = project.join(".forgecad").join("remote.toml");
    let text = match std::fs::read_to_string(&p) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => return Some(Err(format!("{}: {e}", p.display()))),
    };
    Some(parse_remote(&text).map_err(|e| format!("{}: {e}", p.display())))
}

/// The few `key = "value"` lines remote.toml holds (comments and blank lines skipped).
pub fn parse_remote(text: &str) -> Result<Remote, String> {
    let mut get = std::collections::HashMap::new();
    for (k, line) in text.lines().enumerate() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') || l.starts_with('[') {
            continue;
        }
        let (key, v) = l
            .split_once('=')
            .ok_or(format!("line {}: key = \"value\"", k + 1))?;
        let v = v.trim();
        let v = v
            .strip_prefix('"')
            .and_then(|x| x.strip_suffix('"'))
            .ok_or(format!("line {}: a quoted value", k + 1))?;
        get.insert(key.trim().to_string(), v.to_string());
    }
    let url = get.remove("url").ok_or("no url")?;
    if !url.starts_with("ws://") {
        return Err(format!("url {url}: ws://HOST:PORT/PATH"));
    }
    let token = match (get.remove("token"), get.remove("token_env")) {
        (Some(t), _) => t,
        (None, Some(var)) => std::env::var(&var).map_err(|_| format!("token_env {var} is not set"))?,
        (None, None) => return Err("no token (token = \"..\" or token_env = \"VAR\")".into()),
    };
    Ok(Remote { url, token })
}

/// (host:port, /path) of a ws:// URL.
fn split_url(url: &str) -> Result<(String, String), String> {
    let rest = url.strip_prefix("ws://").ok_or("ws:// only")?;
    let (host, path) = rest
        .split_once('/')
        .map_or((rest, "/".to_string()), |(h, p)| (h, format!("/{p}")));
    Ok((host.to_string(), path))
}

/// A fresh client id (a connection's ops are deduplicated by it across reconnects).
pub fn client_id() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(Instant::now().elapsed().as_nanos());
    format!("n{:016x}", h.finish())
}

struct Shared {
    client: Client,
    events: Vec<Event>,
    writer: Option<forge_ws::Writer>,
    connected: bool,
    /// When this client last sent anything (it pings when quiet: the server closes connections
    /// it has heard nothing from).
    last_sent: Instant,
    /// When each own op was sent (for the round trip to its ack).
    sent: std::collections::HashMap<u64, Instant>,
    /// Round trips from send to ack, in ms.
    pub acks_ms: Vec<f64>,
}

impl Shared {
    fn flush(&mut self) {
        let msgs: Vec<Up> = std::mem::take(&mut self.client.outbox);
        if let Some(w) = &mut self.writer {
            if !msgs.is_empty() {
                self.last_sent = Instant::now();
            }
            for m in msgs {
                if w.send(&m.text()).is_err() {
                    self.writer = None;
                    self.connected = false;
                    break;
                }
            }
        }
        // not connected: dropped; the reconnect re-sends Hello and every pending op
    }
}

/// One document's live connection.
pub struct Session {
    shared: Arc<Mutex<Shared>>,
    stop: Arc<AtomicBool>,
    pub doc: String,
}

pub type Waker = Arc<dyn Fn() + Send + Sync>;

/// A quiet client pings this often (the server's idle limit is 90 s).
pub const PING: Duration = Duration::from_secs(20);

impl Session {
    /// Starts the connection thread for `doc` (the document's path relative to the project).
    pub fn start(remote: &Remote, doc: &str, waker: Option<Waker>) -> Result<Session, String> {
        let (addr, path) = split_url(&remote.url)?;
        let shared = Arc::new(Mutex::new(Shared {
            client: Client::new(doc, &client_id(), &remote.token),
            events: vec![],
            writer: None,
            connected: false,
            last_sent: Instant::now(),
            sent: Default::default(),
            acks_ms: vec![],
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let (s2, stop2) = (shared.clone(), stop.clone());
        std::thread::Builder::new()
            .name(format!("forge-remote {doc}"))
            .spawn(move || run(&addr, &path, &s2, &stop2, waker))
            .map_err(|e| e.to_string())?;
        Ok(Session {
            shared,
            stop,
            doc: doc.into(),
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Shared> {
        self.shared.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// True once the first snapshot arrived (before it, the app shows its files' text).
    pub fn ready(&self) -> bool {
        !self.lock().client.joining
    }
    pub fn connected(&self) -> bool {
        self.lock().connected
    }
    /// The text to show: the server's with this client's pending ops on it.
    pub fn text(&self) -> String {
        self.lock().client.text().to_string()
    }
    /// The last seq applied from the server.
    pub fn seq(&self) -> u64 {
        self.lock().client.seq()
    }
    pub fn pending(&self) -> usize {
        self.lock().client.pending().len()
    }
    /// The app applied and checked `op` on `text()` and got `after` (its edit path, unchanged):
    /// queue and send it. Returns the op's id.
    pub fn local(&self, op: Op, after: String) -> u64 {
        let mut s = self.lock();
        let id = s.client.local(op, after);
        s.sent.insert(id, Instant::now());
        s.flush();
        id
    }
    /// `apply_checked` on `text()`, then [`local`](Self::local).
    pub fn edit(&self, op: Op) -> Result<u64, forge_lang::error::Error> {
        let mut s = self.lock();
        let id = s.client.edit(op)?;
        s.sent.insert(id, Instant::now());
        s.flush();
        Ok(id)
    }
    pub fn undo(&self) -> Option<Result<u64, forge_sync::Refusal>> {
        let mut s = self.lock();
        let r = s.client.undo();
        s.flush();
        r
    }
    /// The exact hash of the body the app built from the text at `seq`.
    pub fn built(&self, seq: u64, hash: u64) {
        let mut s = self.lock();
        s.client.built(seq, hash);
        s.flush();
    }
    pub fn version(&self, name: &str) {
        let mut s = self.lock();
        s.client.outbox.push(Up::Version { name: name.into() });
        s.flush();
    }
    /// Events since the last poll.
    pub fn poll(&self) -> Vec<Event> {
        std::mem::take(&mut self.lock().events)
    }
    /// Send → ack round trips so far (ms).
    pub fn ack_ms(&self) -> Vec<f64> {
        self.lock().acks_ms.clone()
    }
    /// Drops the connection (the thread reconnects): for tests of reconnects.
    pub fn drop_connection(&self) {
        let mut s = self.lock();
        if let Some(w) = &s.writer {
            let _ = w.stream().shutdown(std::net::Shutdown::Both);
        }
        s.writer = None;
        s.connected = false;
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.drop_connection();
    }
}

fn run(addr: &str, path: &str, shared: &Mutex<Shared>, stop: &AtomicBool, waker: Option<Waker>) {
    let lock = || shared.lock().unwrap_or_else(|e| e.into_inner());
    let wake = || {
        if let Some(w) = &waker {
            w()
        }
    };
    let mut backoff = Duration::from_millis(50);
    while !stop.load(Ordering::SeqCst) {
        let Ok((mut r, w)) = forge_ws::connect(addr, path, &[], MAX_MESSAGE) else {
            std::thread::sleep(backoff);
            backoff = (backoff * 2).min(Duration::from_secs(5));
            continue;
        };
        backoff = Duration::from_millis(50);
        let _ = r.stream().set_read_timeout(Some(PING));
        {
            let mut s = lock();
            s.writer = Some(w);
            s.connected = true;
            // Hello (from the last seq applied) and every pending op again
            s.client.reconnect();
            s.flush();
        }
        loop {
            if stop.load(Ordering::SeqCst) {
                return;
            }
            match r.read() {
                Ok(forge_ws::Msg::Text(t)) => {
                    let Ok(d) = Down::parse(&t) else { break };
                    let mut s = lock();
                    let acked = match &d {
                        Down::Ack { id, .. } => Some(*id),
                        _ => None,
                    };
                    let ev = s.client.receive(d);
                    if let Some(id) = acked
                        && let Some(t0) = s.sent.remove(&id)
                    {
                        s.acks_ms.push(t0.elapsed().as_secs_f64() * 1e3);
                    }
                    let closed = ev.iter().any(|e| matches!(e, Event::Closed(_)));
                    s.events.extend(ev);
                    if s.last_sent.elapsed() >= PING {
                        s.client.outbox.push(Up::Ping { t: 0.0 });
                    }
                    s.flush();
                    drop(s);
                    wake();
                    if closed {
                        break;
                    }
                }
                Ok(forge_ws::Msg::Ping(p)) => {
                    if let Some(w) = &mut lock().writer {
                        let _ = w.pong(&p);
                    }
                }
                // a quiet link: ping (the server closes connections idle past its limit)
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    let mut s = lock();
                    s.client.outbox.push(Up::Ping { t: 0.0 });
                    s.flush();
                    if !s.connected {
                        break;
                    }
                }
                Ok(forge_ws::Msg::Close) | Err(_) => break,
            }
        }
        let mut s = lock();
        s.writer = None;
        s.connected = false;
        drop(s);
        if !stop.load(Ordering::SeqCst) {
            std::thread::sleep(backoff);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_toml() {
        let r = parse_remote("# served\nurl = \"ws://127.0.0.1:7878/sync\"\ntoken = \"abc\"\n").unwrap();
        assert_eq!(r.url, "ws://127.0.0.1:7878/sync");
        assert_eq!(
            split_url(&r.url).unwrap(),
            ("127.0.0.1:7878".into(), "/sync".into())
        );
        assert!(parse_remote("url = \"http://x\"\ntoken = \"a\"").is_err());
        assert!(parse_remote("url = \"ws://x/sync\"").is_err());
        let d = std::env::temp_dir().join(format!("forge-remote-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        assert!(remote(&d).is_none(), "no file: LOCAL");
        let _ = std::fs::remove_dir_all(&d);
    }
}
