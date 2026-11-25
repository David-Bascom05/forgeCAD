//! forge-server, phase 1 of research/hosted/DESIGN.md: one process serving one project folder
//! (plain files + git) to collaborating clients over WebSocket.
//!
//! Threads, no async runtime: an accept loop; per connection a reader (frames → messages → the
//! hub) and a writer (a bounded queue → frames); one **hub** thread that owns every document's
//! [`Room`] (forge-sync's sequencer and members) and the [`Store`]. So each document's ops are
//! ordered by one thread, validated before they are appended, appended durably before anyone
//! hears of them, and broadcast in seq order.
//!
//! Hardening, after forge-cli's `forgecad web` (interface/forge-cli/src/web.rs):
//! - binds 127.0.0.1 unless told otherwise; on loopback only a loopback `Host` is answered (no
//!   DNS rebinding);
//! - a browser's `Origin` must be the server's own (or one named with `--origin`);
//! - every client needs a token (`Hello`'s, compared in constant time); the server makes one if
//!   none is given;
//! - caps: connections in all and per document, the upgrade head, each message (forge-sync's
//!   `MAX_MESSAGE`, nesting depth), messages per second per connection, a slow reader's queue;
//! - deadlines: the upgrade and the Hello (5 s), idle connections (`IDLE`), every write;
//! - a document is a relative `.fg` path inside the folder: no `..`, no absolute path, no dot
//!   files or folders, no link leading out;
//! - every op is checked (limits, rebase, `apply_checked` on the head) before it is appended and
//!   broadcast; a malformed message closes the connection.

pub mod git;
pub mod store;

use forge_sync::room::{Action, Conn, Room};
use forge_sync::wire::{Down, MAX_MESSAGE, Up};
use forge_sync::{MemStore, Sequencer, Store};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, SyncSender, TrySendError, channel, sync_channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The upgrade and the Hello must arrive within this.
pub const HANDSHAKE: Duration = Duration::from_secs(5);
/// A connection with nothing read for this long is closed (clients ping every 20 s).
pub const IDLE: Duration = Duration::from_secs(90);
/// A write that takes longer than this ends the connection.
pub const WRITE: Duration = Duration::from_secs(10);
/// Messages queued for one connection before it counts as too slow and is dropped.
pub const QUEUE: usize = 4096;
/// Messages per second one connection may send (sustained), and its burst.
pub const RATE: f64 = 200.0;
pub const BURST: f64 = 400.0;
/// The largest document text the server opens.
pub const MAX_DOC: u64 = 8 << 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreKind {
    /// The durable per-document log in `<root>/.forgecad/server/log`.
    File,
    /// In memory: everything is lost when the process ends (tests, demos).
    Mem,
}

#[derive(Clone, Debug)]
pub struct Config {
    /// The project folder: its `.fg` files are the documents' first texts, its git the versions.
    pub root: PathBuf,
    pub bind: IpAddr,
    pub port: u16,
    /// (author name, token): phase 1's dev accounts.
    pub users: Vec<(String, String)>,
    pub store: StoreKind,
    /// Origins besides the server's own a browser page may connect from (the web app served
    /// elsewhere, e.g. `http://localhost:8080`).
    pub origins: Vec<String>,
    pub max_conns: usize,
    pub max_per_doc: usize,
    /// After this long with no op, a document's head is saved as a git commit (no version).
    pub checkpoint_idle: Duration,
}

impl Config {
    pub fn new(root: PathBuf, users: Vec<(String, String)>) -> Config {
        Config {
            root,
            bind: IpAddr::from([127, 0, 0, 1]),
            port: 7878,
            users,
            store: StoreKind::File,
            origins: vec![],
            max_conns: 256,
            max_per_doc: 64,
            checkpoint_idle: Duration::from_secs(300),
        }
    }
}

/// Timings the hub records (budget B2's server terms), in ms.
#[derive(Default, Debug, Clone)]
pub struct Stats {
    /// Rebase + validation + durable append of each committed op.
    pub commit_ms: Vec<f64>,
    pub refused: usize,
    pub committed: usize,
    pub divergences: usize,
    pub versions: usize,
    pub checkpoints: usize,
}

/// A running server: where it listens and its counters.
pub struct Handle {
    pub addr: SocketAddr,
    pub stats: Arc<Mutex<Stats>>,
}

/// A fresh random token (the OS's CSPRNG device where there is one).
pub fn random_token() -> String {
    #[cfg(unix)]
    {
        use std::io::Read;
        let mut b = [0u8; 16];
        if std::fs::File::open("/dev/urandom")
            .and_then(|mut f| f.read_exact(&mut b))
            .is_ok()
        {
            return b.iter().map(|x| format!("{x:02x}")).collect();
        }
    }
    use std::hash::{BuildHasher, Hasher};
    (0..2u64)
        .map(|i| {
            let mut h = std::collections::hash_map::RandomState::new().build_hasher();
            h.write_u64(i);
            format!("{:016x}", h.finish())
        })
        .collect()
}

/// Equal without saying where they differ.
pub fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

fn host_name(h: &str) -> &str {
    if let Some(rest) = h.strip_prefix('[') {
        return rest.split(']').next().unwrap_or_default();
    }
    h.rsplit_once(':').map_or(h, |(n, p)| {
        if p.bytes().all(|b| b.is_ascii_digit()) {
            n
        } else {
            h
        }
    })
}

fn is_loopback_name(h: &str) -> bool {
    let n = host_name(h);
    n.eq_ignore_ascii_case("localhost") || n.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

/// A document name a client may open: a relative `.fg` path inside the folder, no `..`, no
/// dot components, ASCII letters, digits, `_ - . /` only.
pub fn clean_doc(root: &Path, doc: &str) -> Result<PathBuf, &'static str> {
    if doc.is_empty() || doc.len() > 512 || !doc.ends_with(".fg") {
        return Err("a document is a .fg path");
    }
    if !doc
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b"_-./".contains(&b))
    {
        return Err("a document's path is letters, digits and _ - . /");
    }
    let p = Path::new(doc);
    for c in p.components() {
        match c {
            Component::Normal(n) if !n.to_string_lossy().starts_with('.') => {}
            _ => return Err("a document's path stays inside the project"),
        }
    }
    let full = root.join(p);
    // no link leading out of the folder
    let real = full.canonicalize().map_err(|_| "no such document")?;
    let real_root = root.canonicalize().map_err(|_| "the project folder is gone")?;
    if !real.starts_with(&real_root) {
        return Err("a document's path stays inside the project");
    }
    Ok(full)
}

enum Out {
    Text(String),
    Pong(Vec<u8>),
    Close,
}

enum Ev {
    Join {
        conn: Conn,
        doc: String,
        client: String,
        author: String,
        seq: Option<u64>,
        out: SyncSender<Out>,
    },
    Up {
        conn: Conn,
        up: Up,
    },
    Leave {
        conn: Conn,
    },
}

/// Binds and serves in background threads; returns at once.
pub fn start(c: Config) -> Result<Handle, String> {
    let l = TcpListener::bind(SocketAddr::new(c.bind, c.port))
        .map_err(|e| format!("{}:{}: {e}", c.bind, c.port))?;
    start_on(l, c)
}

/// Serves on a listener already bound (tests: port 0).
pub fn start_on(l: TcpListener, c: Config) -> Result<Handle, String> {
    if c.users.is_empty() {
        return Err("no tokens: every client needs one".into());
    }
    let addr = l.local_addr().map_err(|e| e.to_string())?;
    let store: Box<dyn Store + Send> = match c.store {
        StoreKind::Mem => Box::new(MemStore::default()),
        StoreKind::File => Box::new(
            store::FileLogStore::new(c.root.join(".forgecad/server/log"))
                .map_err(|e| format!("the store: {e}"))?,
        ),
    };
    let stats = Arc::new(Mutex::new(Stats::default()));
    let (tx, rx) = channel::<Ev>();
    let c = Arc::new(c);
    {
        let (c, stats) = (c.clone(), stats.clone());
        std::thread::Builder::new()
            .name("forge-server hub".into())
            .spawn(move || hub(&c, store, rx, &stats))
            .map_err(|e| e.to_string())?;
    }
    let live = Arc::new(AtomicUsize::new(0));
    std::thread::Builder::new()
        .name("forge-server accept".into())
        .spawn(move || {
            let mut next: Conn = 0;
            for s in l.incoming() {
                let Ok(s) = s else { continue };
                next += 1;
                let (c, tx, live) = (c.clone(), tx.clone(), live.clone());
                if live.fetch_add(1, Ordering::SeqCst) >= c.max_conns {
                    live.fetch_sub(1, Ordering::SeqCst);
                    let mut s = s;
                    let _ = std::io::Write::write_all(
                        &mut s,
                        b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    );
                    continue;
                }
                let conn = next;
                let _ = std::thread::Builder::new()
                    .name(format!("forge-server conn {conn}"))
                    .spawn(move || {
                        connection(s, conn, &c, &tx);
                        live.fetch_sub(1, Ordering::SeqCst);
                    });
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(Handle { addr, stats })
}

/// Who a token belongs to: every user is compared (no early exit on a match).
fn author_of(c: &Config, token: &str) -> Option<String> {
    let mut who = None;
    for (name, t) in &c.users {
        if same(token, t) && who.is_none() {
            who = Some(name.clone());
        }
    }
    who
}

fn connection(s: TcpStream, conn: Conn, c: &Config, tx: &Sender<Ev>) {
    let _ = s.set_read_timeout(Some(HANDSHAKE));
    let _ = s.set_write_timeout(Some(WRITE));
    let _ = s.set_nodelay(true);
    let accepted = forge_ws::accept(s, MAX_MESSAGE, |q| {
        if q.path != "/sync" {
            return Err(forge_ws::Refuse(404, "the sync endpoint is /sync"));
        }
        let host = q.header("host").unwrap_or("");
        if c.bind.is_loopback() && !is_loopback_name(host) {
            return Err(forge_ws::Refuse(421, "this server answers to localhost only"));
        }
        // a browser always says its page's origin; a native client says none
        if let Some(o) = q.header("origin") {
            let bare = o.split_once("://").map_or(o, |x| x.1);
            if !bare.eq_ignore_ascii_case(host) && !c.origins.iter().any(|x| x.eq_ignore_ascii_case(o)) {
                return Err(forge_ws::Refuse(403, "a page from another origin"));
            }
        }
        Ok(())
    });
    let Ok((mut r, mut w, _)) = accepted else { return };
    let fail = |w: &mut forge_ws::Writer, text: &str| {
        let _ = w.send(&Down::Error { text: text.into() }.text());
        w.close();
    };
    // the first message is Hello, within the handshake's deadline
    let hello = match r.read() {
        Ok(forge_ws::Msg::Text(t)) => Up::parse(&t),
        _ => return,
    };
    let Ok(Up::Hello {
        doc,
        client,
        token,
        seq,
    }) = hello
    else {
        return fail(&mut w, "the first message is hello");
    };
    let Some(author) = author_of(c, &token) else {
        return fail(&mut w, "a valid token is needed");
    };
    if let Err(why) = clean_doc(&c.root, &doc) {
        return fail(&mut w, why);
    }
    if client.is_empty()
        || client.len() > 128
        || !client
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
    {
        return fail(&mut w, "a client id is 1-128 letters, digits, _ and -");
    }
    let _ = r.stream().set_read_timeout(Some(IDLE));
    let (otx, orx) = sync_channel::<Out>(QUEUE);
    let writer = std::thread::Builder::new()
        .name(format!("forge-server write {conn}"))
        .spawn(move || {
            for m in orx {
                let ok = match m {
                    Out::Text(t) => w.send(&t).is_ok(),
                    Out::Pong(p) => w.pong(&p).is_ok(),
                    Out::Close => {
                        w.close();
                        false
                    }
                };
                if !ok {
                    break;
                }
            }
            let _ = w.stream().shutdown(std::net::Shutdown::Both);
        });
    if writer.is_err() {
        return;
    }
    let _ = tx.send(Ev::Join {
        conn,
        doc,
        client,
        author,
        seq,
        out: otx.clone(),
    });
    let (mut tokens, mut last) = (BURST, Instant::now());
    loop {
        let m = match r.read() {
            Ok(forge_ws::Msg::Text(t)) => t,
            Ok(forge_ws::Msg::Ping(p)) => {
                let _ = otx.try_send(Out::Pong(p));
                continue;
            }
            Ok(forge_ws::Msg::Close) | Err(_) => break,
        };
        let now = Instant::now();
        tokens = (tokens + now.duration_since(last).as_secs_f64() * RATE).min(BURST) - 1.0;
        last = now;
        if tokens < 0.0 {
            let _ = otx.try_send(Out::Text(
                Down::Error {
                    text: "too many messages".into(),
                }
                .text(),
            ));
            break;
        }
        match Up::parse(&m) {
            Ok(up) => {
                if tx.send(Ev::Up { conn, up }).is_err() {
                    break;
                }
            }
            Err(e) => {
                let _ = otx.try_send(Out::Text(
                    Down::Error {
                        text: format!("a bad message: {e}"),
                    }
                    .text(),
                ));
                break;
            }
        }
    }
    let _ = otx.try_send(Out::Close);
    let _ = r.stream().shutdown(std::net::Shutdown::Read);
    let _ = tx.send(Ev::Leave { conn });
}

struct Doc {
    room: Room,
    last_op: Instant,
    /// The seq last written to git (a checkpoint or a version).
    saved: u64,
}

fn hub(c: &Config, mut store: Box<dyn Store + Send>, rx: Receiver<Ev>, stats: &Mutex<Stats>) {
    let mut docs: HashMap<String, Doc> = HashMap::new();
    let mut conns: HashMap<Conn, (String, SyncSender<Out>)> = HashMap::new();
    let send = |conns: &mut HashMap<Conn, (String, SyncSender<Out>)>, out: Vec<(Conn, Down)>| {
        let mut slow = vec![];
        for (k, d) in out {
            if let Some((_, tx)) = conns.get(&k) {
                match tx.try_send(Out::Text(d.text())) {
                    Ok(()) => {}
                    // too slow to keep up (or gone): drop it; it reconnects and catches up
                    Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => slow.push(k),
                }
            }
        }
        slow
    };
    loop {
        let ev = rx.recv_timeout(Duration::from_secs(1));
        let mut drop_conns = vec![];
        match ev {
            Ok(Ev::Join {
                conn,
                doc,
                client,
                author,
                seq,
                out,
            }) => {
                let opened = match docs.get_mut(&doc) {
                    Some(d) => Ok(d),
                    None => open_doc(c, store.as_mut(), &doc).map(|room| {
                        docs.entry(doc.clone()).or_insert(Doc {
                            saved: room.seq.seq(),
                            room,
                            last_op: Instant::now(),
                        })
                    }),
                };
                let d = match opened {
                    Ok(d) if d.room.members() >= c.max_per_doc => {
                        Err("too many people in this document".to_string())
                    }
                    x => x.map_err(|e| e.to_string()),
                };
                match d {
                    Ok(d) => {
                        let o = d.room.join(conn, &client, &author, seq);
                        conns.insert(conn, (doc, out));
                        drop_conns = send(&mut conns, o);
                    }
                    Err(e) => {
                        let _ = out.try_send(Out::Text(Down::Error { text: e }.text()));
                        let _ = out.try_send(Out::Close);
                    }
                }
            }
            Ok(Ev::Up { conn, up }) => {
                let Some((doc, _)) = conns.get(&conn) else {
                    continue;
                };
                let doc = doc.clone();
                let Some(d) = docs.get_mut(&doc) else { continue };
                let is_op = matches!(up, Up::Op { .. });
                let t0 = Instant::now();
                let (out, actions) = d.room.receive(store.as_mut(), conn, up);
                if is_op {
                    let ms = t0.elapsed().as_secs_f64() * 1e3;
                    let mut s = stats.lock().unwrap_or_else(|e| e.into_inner());
                    if matches!(out.first(), Some((_, Down::Refused { .. }))) {
                        s.refused += 1;
                    } else {
                        s.committed += 1;
                        s.commit_ms.push(ms);
                        d.last_op = Instant::now();
                    }
                }
                drop_conns = send(&mut conns, out);
                for a in actions {
                    match a {
                        Action::Version { name, author, conn } => {
                            let r = git::version(
                                &c.root,
                                &doc,
                                d.room.seq.head(),
                                &name,
                                &author,
                                d.room.seq.seq(),
                            );
                            match r {
                                Ok(commit) => {
                                    d.saved = d.room.seq.seq();
                                    stats.lock().unwrap_or_else(|e| e.into_inner()).versions += 1;
                                    let o = d.room.versioned(&name, d.room.seq.seq(), &commit);
                                    drop_conns.extend(send(&mut conns, o));
                                }
                                Err(e) => {
                                    let o = vec![(
                                        conn,
                                        Down::Error {
                                            text: format!("the version was not written: {e}"),
                                        },
                                    )];
                                    drop_conns.extend(send(&mut conns, o));
                                }
                            }
                        }
                        Action::Diverged {
                            seq,
                            expected,
                            got,
                            by,
                            first,
                        } => {
                            stats.lock().unwrap_or_else(|e| e.into_inner()).divergences += 1;
                            eprintln!(
                                "forge-server: DIVERGENCE {doc} seq {seq}: {by} built {got:016x}, {first} built {expected:016x}"
                            );
                        }
                    }
                }
            }
            Ok(Ev::Leave { conn }) => {
                if let Some((doc, _)) = conns.remove(&conn)
                    && let Some(d) = docs.get_mut(&doc)
                {
                    d.room.leave(conn);
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
        }
        for k in drop_conns {
            if let Some((doc, tx)) = conns.remove(&k) {
                let _ = tx.try_send(Out::Close);
                if let Some(d) = docs.get_mut(&doc) {
                    d.room.leave(k);
                }
            }
        }
        // idle checkpoints: a document quiet for a while gets its head saved in git
        for (doc, d) in docs.iter_mut() {
            if d.room.seq.seq() > d.saved && d.last_op.elapsed() >= c.checkpoint_idle {
                match git::checkpoint(&c.root, doc, d.room.seq.head(), d.room.seq.seq()) {
                    Ok(()) => stats.lock().unwrap_or_else(|e| e.into_inner()).checkpoints += 1,
                    Err(e) => eprintln!("forge-server: {doc}: checkpoint not written: {e}"),
                }
                d.saved = d.room.seq.seq();
            }
        }
    }
}

fn open_doc(c: &Config, store: &mut dyn Store, doc: &str) -> Result<Room, String> {
    let path = clean_doc(&c.root, doc).map_err(str::to_string)?;
    let len = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
    if len > MAX_DOC {
        return Err(format!("{doc}: {len} bytes (at most {MAX_DOC})"));
    }
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{doc}: {e}"))?;
    let seq = Sequencer::open(store, doc, &text, path.parent().map(Path::to_path_buf))
        .map_err(|e| e.to_string())?;
    Ok(Room::new(seq))
}
