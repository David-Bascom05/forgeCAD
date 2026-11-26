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
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{SyncSender, sync_channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Where a project is served, from the **user's** configuration (never the project's: a cloned
/// repository must not be able to send this machine's tokens anywhere).
#[derive(Clone, Debug, PartialEq)]
pub struct Remote {
    /// The remote's name, as the project's `.forgecad/remote.toml` names it.
    pub name: String,
    /// `ws://HOST:PORT/PATH`: a loopback host, unless the user's entry says `insecure_lan`
    /// (phase 1 has no TLS: a token sent elsewhere would cross the network in clear).
    pub url: String,
    /// The dev token (phase 1): the user's file holds it, or a `FORGECAD_*` variable it names.
    pub token: String,
}

/// The project's remote, if it is SERVED: None for a LOCAL project (no `.forgecad/remote.toml`),
/// Err for one that is there but cannot be used (said, never silently LOCAL). The project file
/// may say only `remote = "NAME"`; NAME must be a remote the user configured in
/// `<config>/forgecad/remotes.toml` (that entry is the user's trust in it).
pub fn remote(project: &Path) -> Option<Result<Remote, String>> {
    let p = project.join(".forgecad").join("remote.toml");
    let text = match std::fs::read_to_string(&p) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => return Some(Err(format!("{}: {e}", p.display()))),
    };
    let user_text = user_file()
        .and_then(|u| std::fs::read_to_string(u).ok())
        .unwrap_or_default();
    let here = project.canonicalize().unwrap_or_else(|_| project.to_path_buf());
    Some(resolve_for(&text, &user_text, Some(&here)).map_err(|e| format!("{}: {e}", p.display())))
}

static USER_FILE: std::sync::Mutex<Option<std::path::PathBuf>> = std::sync::Mutex::new(None);

/// The user's remotes file: `<config>/forgecad/remotes.toml` (a relative `XDG_CONFIG_HOME` is
/// ignored, as the XDG spec says: it would make the file depend on the working folder, which a
/// project chooses), or the one [`set_user_file`] named.
pub fn user_file() -> Option<std::path::PathBuf> {
    let set = USER_FILE.lock().unwrap_or_else(|e| e.into_inner()).clone();
    set.or_else(|| {
        let c = forge_dirs::config().filter(|c| c.is_absolute()).or_else(|| {
            let h = forge_dirs::home().filter(|h| h.is_absolute())?;
            Some(if cfg!(windows) || cfg!(target_os = "macos") {
                forge_dirs::config()?
            } else {
                h.join(".config")
            })
        })?;
        c.is_absolute().then(|| c.join("forgecad").join("remotes.toml"))
    })
}

/// Uses `path` as the user's remotes file in this process (tests; an app setting).
pub fn set_user_file(path: Option<std::path::PathBuf>) {
    *USER_FILE.lock().unwrap_or_else(|e| e.into_inner()) = path;
}

/// `key = "value"` lines, grouped by `[section]` (the few TOML forms these files use).
type Sections = Vec<(String, std::collections::HashMap<String, String>)>;

fn sections(text: &str) -> Result<Sections, String> {
    let mut out = vec![(String::new(), std::collections::HashMap::new())];
    for (k, line) in text.lines().enumerate() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        if let Some(h) = l.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
            out.push((h.trim().to_string(), Default::default()));
            continue;
        }
        let (key, v) = l
            .split_once('=')
            .ok_or(format!("line {}: key = \"value\"", k + 1))?;
        let v = v.trim();
        let v = match v {
            "true" | "false" => v,
            _ => v
                .strip_prefix('"')
                .and_then(|x| x.strip_suffix('"'))
                .ok_or(format!("line {}: a quoted value", k + 1))?,
        };
        out.last_mut()
            .expect("one")
            .1
            .insert(key.trim().to_string(), v.to_string());
    }
    Ok(out)
}

/// The project's `remote.toml` resolved against the user's `remotes.toml`.
pub fn resolve(project: &str, user: &str) -> Result<Remote, String> {
    resolve_for(project, user, None)
}

/// [`resolve`], for the project folder `at`: an entry with `projects = "/a, /b"` serves only
/// projects inside one of those folders (so a project elsewhere naming it is refused: the
/// user's trust in a remote is for their own projects, not any folder that names it).
pub fn resolve_for(project: &str, user: &str, at: Option<&Path>) -> Result<Remote, String> {
    let proj = sections(project)?;
    let top = &proj[0].1;
    if proj.len() > 1 || top.keys().any(|k| k != "remote") {
        return Err(
            "a project's remote.toml names a remote and nothing else: remote = \"NAME\" \
                    (its address and token are the user's, in remotes.toml)"
                .into(),
        );
    }
    let name = top.get("remote").ok_or("no remote = \"NAME\"")?.clone();
    let users = sections(user)?;
    let want = format!("remote.{name}");
    let e = users
        .iter()
        .find(|(h, _)| *h == want)
        .map(|x| &x.1)
        .ok_or(format!(
            "this project is served by the remote {name:?}, which is not in your remotes.toml: \
         add [remote.{name}] with its url and token if you trust it"
        ))?;
    let url = e.get("url").ok_or(format!("[remote.{name}]: no url"))?.clone();
    let (host, _) = split_url(&url)?;
    let insecure = e.get("insecure_lan").is_some_and(|v| v == "true");
    let bare = host
        .rsplit_once(':')
        .map_or(host.as_str(), |x| x.0)
        .trim_start_matches('[')
        .trim_end_matches(']');
    let loopback = bare.eq_ignore_ascii_case("localhost")
        || bare.parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback());
    if !loopback && !insecure {
        return Err(format!(
            "[remote.{name}]: {host} is not this machine, and phase 1 has no TLS: the token would cross the network \
             in clear (insecure_lan = true to send it anyway)"
        ));
    }
    // the user's trust in a remote is for their own projects: an entry names the folders it
    // serves, and a project elsewhere naming it is refused (by default: no list, no project)
    let list = e.get("projects").ok_or(format!(
        "[remote.{name}] lists no projects: add projects = \"/path/to/your/cad\" (the folders it may serve)"
    ))?;
    let inside = at.is_some_and(|at| {
        list.split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .filter(|p| Path::new(p).is_absolute())
            // both sides canonical (`at` is): a symlinked projects folder still matches; a
            // folder that does not exist matches only as written
            .any(|p| at.starts_with(std::fs::canonicalize(p).unwrap_or_else(|_| p.into())))
    });
    if !inside {
        return Err(format!(
            "[remote.{name}] serves only the projects under {list:?}: this one is elsewhere"
        ));
    }
    let token = match (e.get("token"), e.get("token_env")) {
        (Some(t), _) => t.clone(),
        (None, Some(var)) if var.starts_with("FORGECAD_") => {
            std::env::var(var).map_err(|_| format!("[remote.{name}]: {var} is not set"))?
        }
        (None, Some(var)) => {
            return Err(format!(
                "[remote.{name}]: token_env must name a FORGECAD_ variable, not {var}"
            ));
        }
        (None, None) => {
            return Err(format!(
                "[remote.{name}]: no token (token = \"..\" or token_env = \"FORGECAD_..\")"
            ));
        }
    };
    Ok(Remote { name, url, token })
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

enum Outgoing {
    Messages(Vec<Up>),
    Pong(Vec<u8>),
}

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);
const OUTBOX_CAPACITY: usize = 64;

struct Shared {
    client: Client,
    events: Vec<Event>,
    writer: Option<SyncSender<Outgoing>>,
    /// A shutdown handle, installed before reading the HTTP upgrade.
    socket: Option<TcpStream>,
    connected: bool,
    /// When this client last sent anything (it pings when quiet: the server closes connections
    /// it has heard nothing from).
    last_sent: Instant,
    /// When this client last heard anything (a link quiet for 3 pings is dead: dropped, and the
    /// connection thread reconnects).
    last_heard: Instant,
    /// How often a quiet client pings.
    ping: Duration,
    /// When each own op was sent (for the round trip to its ack).
    sent: std::collections::HashMap<u64, Instant>,
    /// Round trips from send to ack, in ms.
    pub acks_ms: Vec<f64>,
}

impl Shared {
    fn disconnect(&mut self) {
        if let Some(socket) = self.socket.take() {
            let _ = socket.shutdown(Shutdown::Both);
        }
        self.writer = None;
        self.connected = false;
    }

    fn queue(&mut self, message: Outgoing) -> bool {
        let Some(writer) = &self.writer else { return false };
        if writer.try_send(message).is_err() {
            // A stalled writer cannot accumulate messages indefinitely. The next
            // connection re-sends any unacknowledged edits under their original IDs.
            self.disconnect();
            return false;
        }
        self.last_sent = Instant::now();
        true
    }

    fn flush(&mut self) {
        let messages = std::mem::take(&mut self.client.outbox);
        if !messages.is_empty() {
            // A reconnect can replay more edits than there are queue slots.
            // Keep that ordered replay together in the one connection's queue.
            self.queue(Outgoing::Messages(messages));
        }
    }

}

/// One document's live connection.
pub struct Session {
    shared: Arc<Mutex<Shared>>,
    stop: Arc<AtomicBool>,
    pub doc: String,
    /// The server's address: what a saved client is kept for.
    url: String,
}

pub type Waker = Arc<dyn Fn() + Send + Sync>;

/// A quiet client pings this often (the server's idle limit is 90 s).
pub const PING: Duration = Duration::from_secs(20);

impl Session {
    /// Starts the connection thread for `doc` (the document's path relative to the project).
    pub fn start(remote: &Remote, doc: &str, waker: Option<Waker>) -> Result<Session, String> {
        Session::start_with(remote, doc, waker, PING)
    }

    /// [`start`](Self::start), taking up a client the app saved ([`saved`](Self::saved)) when
    /// it closed: its pending ops are sent again under the same ids (never applied twice). A
    /// saved text that is not one starts a new client.
    pub fn resume(
        remote: &Remote,
        doc: &str,
        saved: Option<&str>,
        waker: Option<Waker>,
    ) -> Result<Session, String> {
        // kept for this server only: another server's history is not this client's (its op ids
        // and bases mean nothing there)
        let client = saved
            .and_then(|t| forge_sync::wire::read(t).ok())
            .filter(|j| j.get("server").and_then(|v| v.str()) == Some(remote.url.as_str()))
            .and_then(|j| j.get("client").and_then(|v| v.str()).map(str::to_string))
            .and_then(|t| Client::restore(doc, &remote.token, &t));
        Session::start_client(remote, doc, client, waker, PING)
    }

    /// This client as the app keeps it across a close: its id, op ids and pending ops.
    /// Kept with the server's address: taken up again only against the same server.
    pub fn saved(&self) -> String {
        use forge_sync::wire::{Json, write};
        let client = self
            .shared
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .client
            .saved();
        write(&Json::obj(vec![
            ("server", self.url.as_str().into()),
            ("client", client.as_str().into()),
        ]))
    }

    /// [`start`](Self::start), pinging every `ping` when quiet (tests).
    pub fn start_with(
        remote: &Remote,
        doc: &str,
        waker: Option<Waker>,
        ping: Duration,
    ) -> Result<Session, String> {
        Session::start_client(remote, doc, None, waker, ping)
    }

    fn start_client(
        remote: &Remote,
        doc: &str,
        client: Option<Client>,
        waker: Option<Waker>,
        ping: Duration,
    ) -> Result<Session, String> {
        let (addr, path) = split_url(&remote.url)?;
        let shared = Arc::new(Mutex::new(Shared {
            client: client.unwrap_or_else(|| Client::new(doc, &client_id(), &remote.token)),
            events: vec![],
            writer: None,
            socket: None,
            connected: false,
            last_sent: Instant::now(),
            last_heard: Instant::now(),
            ping,
            sent: Default::default(),
            acks_ms: vec![],
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let (s2, stop2) = (shared.clone(), stop.clone());
        std::thread::Builder::new()
            .name(format!("forge-remote {doc}"))
            .spawn(move || run(&addr, &path, &s2, &stop2, waker))
            .map_err(|e| e.to_string())?;
        // the pinger: a quiet client pings by the clock, never from inside a read
        let (s3, stop3) = (shared.clone(), stop.clone());
        std::thread::Builder::new()
            .name(format!("forge-remote ping {doc}"))
            .spawn(move || {
                while !stop3.load(Ordering::SeqCst) {
                    let ping = s3.lock().unwrap_or_else(|e| e.into_inner()).ping;
                    std::thread::sleep((ping / 4).min(Duration::from_millis(500)));
                    let mut s = s3.lock().unwrap_or_else(|e| e.into_inner());
                    if s.connected && s.last_heard.elapsed() >= ping * 3
                        || !s.connected && s.socket.is_some() && s.last_heard.elapsed() >= HANDSHAKE_TIMEOUT
                    {
                        s.disconnect();
                    } else if s.connected && s.last_sent.elapsed() >= ping {
                        s.client.outbox.push(Up::Ping { t: 0.0 });
                        s.flush();
                    }
                }
            })
            .map_err(|e| {
                stop.store(true, Ordering::SeqCst);
                shared.lock().unwrap_or_else(|e| e.into_inner()).disconnect();
                e.to_string()
            })?;
        Ok(Session {
            shared,
            stop,
            doc: doc.into(),
            url: remote.url.clone(),
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
    pub fn snapshot(&self) -> (u64, String) {
        let s = self.lock();
        (s.client.seq(), s.client.text().into())
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
    pub fn local(&self, op: impl Into<forge_sync::Edit>, after: String) -> Result<u64, forge_lang::error::Error> {
        let mut s = self.lock();
        let id = s.client.local(op, after)?;
        s.sent.insert(id, Instant::now());
        s.flush();
        Ok(id)
    }
    pub fn local_on(
        &self,
        base: u64,
        before: &str,
        op: forge_sync::Edit,
        after: String,
    ) -> Result<(), forge_lang::error::Error> {
        let mut s = self.lock();
        let id = s.client.local_on(base, before, op, after)?;
        s.sent.insert(id, Instant::now());
        s.flush();
        Ok(())
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
        self.lock().disconnect();
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
        let connect = || -> std::io::Result<_> {
            let started = Instant::now();
            let mut last = std::io::Error::new(std::io::ErrorKind::AddrNotAvailable, "no address");
            for address in addr.to_socket_addrs()? {
                if stop.load(Ordering::SeqCst) {
                    return Err(std::io::ErrorKind::Interrupted.into());
                }
                let left = HANDSHAKE_TIMEOUT.saturating_sub(started.elapsed());
                if left.is_zero() {
                    return Err(std::io::ErrorKind::TimedOut.into());
                }
                let socket = match TcpStream::connect_timeout(&address, left) {
                    Ok(socket) => socket,
                    Err(error) => {
                        last = error;
                        continue;
                    }
                };
                socket.set_read_timeout(Some(HANDSHAKE_TIMEOUT))?;
                socket.set_write_timeout(Some(HANDSHAKE_TIMEOUT))?;
                let cancel = socket.try_clone()?;
                {
                    let mut s = lock();
                    if stop.load(Ordering::SeqCst) {
                        return Err(std::io::ErrorKind::Interrupted.into());
                    }
                    s.socket = Some(cancel);
                    s.last_heard = Instant::now();
                }
                let (reader, writer) = forge_ws::upgrade_client(socket, addr, path, &[], MAX_MESSAGE)?;
                // A read timeout in the middle of a WebSocket frame would leave the
                // framing out of step. Only the HTTP upgrade has a read timeout.
                writer.stream().set_read_timeout(None)?;
                writer.stream().set_write_timeout(Some(WRITE_TIMEOUT))?;
                return Ok((reader, writer));
            }
            Err(last)
        };
        let Ok((mut r, mut w)) = connect() else {
            lock().disconnect();
            if !stop.load(Ordering::SeqCst) {
                std::thread::sleep(backoff);
                backoff = (backoff * 2).min(Duration::from_secs(5));
            }
            continue;
        };
        let (send, receive) = sync_channel(OUTBOX_CAPACITY);
        let Ok(writer) = std::thread::Builder::new()
            .name("forge-remote write".into())
            .spawn(move || {
                while let Ok(message) = receive.recv() {
                    let result = match message {
                        Outgoing::Messages(messages) => messages.iter().try_for_each(|message| w.send(&message.text())),
                        Outgoing::Pong(data) => w.pong(&data),
                    };
                    if result.is_err() {
                        let _ = w.stream().shutdown(Shutdown::Both);
                        break;
                    }
                }
            })
        else {
            lock().disconnect();
            std::thread::sleep(backoff);
            backoff = (backoff * 2).min(Duration::from_secs(5));
            continue;
        };
        {
            let mut s = lock();
            // The document may have closed or the handshake deadline expired just
            // as the peer replied.
            if s.socket.is_some() && !stop.load(Ordering::SeqCst) {
                s.last_heard = Instant::now();
                s.writer = Some(send);
                s.connected = true;
                s.client.reconnect();
                s.flush();
            } else {
                drop(send);
            }
        }
        loop {
            if stop.load(Ordering::SeqCst) || !lock().connected {
                break;
            }
            match r.read() {
                Ok(forge_ws::Msg::Text(t)) => {
                    let Ok(d) = Down::parse(&t) else {
                        // a message this client cannot read: another protocol; retrying would
                        // fail the same way for ever
                        let mut s = lock();
                        s.events.push(Event::Stopped(
                            "the server speaks another protocol: update the app (or the server)".into(),
                        ));
                        drop(s);
                        stop.store(true, Ordering::SeqCst);
                        wake();
                        break;
                    };
                    if matches!(d, Down::Welcome { .. }) {
                        backoff = Duration::from_millis(50);
                    }
                    let mut s = lock();
                    s.last_heard = Instant::now();
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
                    let closed = ev
                        .iter()
                        .any(|e| matches!(e, Event::Closed(_) | Event::Stopped(_)));
                    // refused for good (a bad token, another kernel, a quarantined document):
                    // reconnecting would be refused the same way, for ever
                    let stopped = ev.iter().any(|e| matches!(e, Event::Stopped(_)));
                    s.events.extend(ev);
                    s.flush();
                    drop(s);
                    // the client applied the op here (forge-lang): its sketches let go
                    forge_sync::release_sketches();
                    if stopped {
                        stop.store(true, Ordering::SeqCst);
                    }
                    wake();
                    if closed {
                        break;
                    }
                }
                Ok(forge_ws::Msg::Ping(p)) => {
                    lock().queue(Outgoing::Pong(p));
                }
                Ok(forge_ws::Msg::Close) | Err(_) => break,
            }
        }
        lock().disconnect();
        let _ = writer.join();
        if !stop.load(Ordering::SeqCst) {
            std::thread::sleep(backoff);
            backoff = (backoff * 2).min(Duration::from_secs(5));
        }
    }
}

// sockets, threads, links and file locks: not in WASI (the wasm suite runs the workspace there)
#[cfg(all(test, not(target_os = "wasi")))]
mod tests {
    use super::*;

    #[test]
    fn remote_toml() {
        let user = "[remote.lab]\nurl = \"ws://127.0.0.1:7878/sync\"\ntoken = \"abc\"\nprojects = \"/home/me/cad\"\n\
                    [remote.far]\nurl = \"ws://10.0.0.9:7878/sync\"\ntoken = \"abc\"\nprojects = \"/home/me/cad\"\n\
                    [remote.envy]\nurl = \"ws://localhost:7878/sync\"\ntoken_env = \"GITHUB_TOKEN\"\nprojects = \"/home/me/cad\"\n\
                    [remote.open]\nurl = \"ws://127.0.0.1:7878/sync\"\ntoken = \"abc\"\n";
        let mine = Some(Path::new("/home/me/cad/p"));
        let r = resolve_for("remote = \"lab\"\n", user, mine).unwrap();
        assert_eq!(
            (r.url.as_str(), r.token.as_str()),
            ("ws://127.0.0.1:7878/sync", "abc")
        );
        assert_eq!(
            split_url(&r.url).unwrap(),
            ("127.0.0.1:7878".into(), "/sync".into())
        );
        // the security reviews' cases: the project cannot choose the address or the token, and a
        // remote serves only the folders its entry lists (none listed: none)
        for (proj, at, why) in [
            (
                "url = \"ws://evil.example/sync\"\ntoken_env = \"AWS_SECRET\"\n",
                mine,
                "nothing else",
            ),
            ("remote = \"lab\"\ntoken = \"x\"\n", mine, "nothing else"),
            ("remote = \"unknown\"\n", mine, "not in your remotes.toml"),
            ("remote = \"far\"\n", mine, "not this machine"),
            ("remote = \"envy\"\n", mine, "FORGECAD_"),
            ("remote = \"open\"\n", mine, "lists no projects"),
            ("remote = \"lab\"\n", Some(Path::new("/tmp/cloned")), "elsewhere"),
            ("remote = \"lab\"\n", None, "elsewhere"),
        ] {
            let e = resolve_for(proj, user, at).unwrap_err();
            assert!(e.contains(why), "{proj}: {e}");
        }
        let lan = user.replace("[remote.far]\n", "[remote.far]\ninsecure_lan = true\n");
        assert!(resolve_for("remote = \"far\"\n", &lan, mine).is_ok());
        let d = std::env::temp_dir().join(format!("forge-remote-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        assert!(remote(&d).is_none(), "no file: LOCAL");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn closing_cancels_an_unanswered_upgrade() {
        use std::io::Read;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let remote = Remote {
            name: "stalled".into(),
            url: format!("ws://{}/sync", listener.local_addr().unwrap()),
            token: "test".into(),
        };
        let session = Session::start(&remote, "p.fg", None).unwrap();
        let (mut socket, _) = listener.accept().unwrap();
        socket.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut request = [0; 4];
        socket.read_exact(&mut request).unwrap();
        assert_eq!(&request, b"GET ");
        assert!(!session.connected());
        drop(session);
        let mut rest = Vec::new();
        match socket.read_to_end(&mut rest) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => {}
            Err(e) => panic!("upgrade socket was not shut down: {e}"),
        }
    }

    #[test]
    fn a_full_writer_queue_disconnects_without_waiting() {
        let remote = Remote {
            name: "offline".into(),
            url: "ws://127.0.0.1:1/sync".into(),
            token: "test".into(),
        };
        let session = Session::start(&remote, "p.fg", None).unwrap();
        // Install a queue whose receiver deliberately makes no progress. A
        // second call must disconnect instead of waiting for the receiver.
        let (sender, _stalled) = sync_channel(1);
        let mut shared = session.lock();
        shared.writer = Some(sender);
        shared.connected = true;
        assert!(shared.queue(Outgoing::Pong(vec![1])));
        assert!(!shared.queue(Outgoing::Pong(vec![2])));
        assert!(!shared.connected && shared.writer.is_none());
    }

    /// The review's 11: a frame cut in two by a pause (longer than the ping interval) is read
    /// whole, never out of step; and a quiet client pings by the clock.
    #[test]
    fn a_split_frame_and_pings_by_the_clock() {
        use std::io::Write;
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let at = l.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (s, _) = l.accept().unwrap();
            let raw = s.try_clone().unwrap();
            let (mut r, _w, _) = forge_ws::accept(s, 1 << 20, |_| Ok(())).unwrap();
            let hello = match r.read().unwrap() {
                forge_ws::Msg::Text(t) => Up::parse(&t).unwrap(),
                x => panic!("{x:?}"),
            };
            assert!(matches!(hello, Up::Hello { .. }));
            let welcome = Down::Welcome {
                seq: 0,
                text: "shared w = 4\n".repeat(20),
                you: "dev".into(),
                snap: true,
            }
            .text();
            let mut f = vec![0x81u8, 126];
            f.extend_from_slice(&(welcome.len() as u16).to_be_bytes());
            f.extend_from_slice(welcome.as_bytes());
            let mut raw = raw;
            raw.write_all(&f[..f.len() / 2]).unwrap();
            std::thread::sleep(Duration::from_millis(500));
            raw.write_all(&f[f.len() / 2..]).unwrap();
            // a ping comes by the clock while the server says nothing
            let t0 = Instant::now();
            loop {
                match r.read().unwrap() {
                    forge_ws::Msg::Text(t) if matches!(Up::parse(&t), Ok(Up::Ping { .. })) => {
                        break t0.elapsed();
                    }
                    _ => {}
                }
            }
        });
        let remote = Remote {
            name: "t".into(),
            url: format!("ws://{at}/sync"),
            token: "x".into(),
        };
        let s = Session::start_with(&remote, "p.fg", None, Duration::from_millis(300)).unwrap();
        let t0 = Instant::now();
        while !s.ready() {
            assert!(
                t0.elapsed() < Duration::from_secs(5),
                "the split Welcome was never read"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(s.text(), "shared w = 4\n".repeat(20), "read whole, in step");
        let waited = server.join().unwrap();
        assert!(
            waited < Duration::from_millis(1500),
            "a ping within the interval: {waited:?}"
        );
    }
}
