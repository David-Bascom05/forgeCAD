//! forge-server, phase 1 of research/hosted/DESIGN.md: one process serving one project folder
//! (plain files + git) to collaborating clients over WebSocket.
//!
//! Threads, no async runtime: an accept loop; per connection a reader (frames → messages → the
//! hub) and a writer (a bounded queue → frames); one **hub** thread that owns every document's
//! [`Room`] (forge-sync's sequencer and members) and the [`Store`]. So each document's ops are
//! ordered by one thread, validated before they are appended, appended durably before anyone
//! hears of them, and broadcast in seq order.
//!
//! Hardening, after forge-cli's `forgecad web` (interface/forge-cli/src/web.rs) and the security
//! review of this crate:
//! - binds 127.0.0.1; any other address only with `insecure_lan` (no TLS in phase 1: the token
//!   would cross the network in clear); on loopback only a loopback `Host` is answered;
//! - a browser's `Origin` must be the server's own (or one named with `--origin`);
//! - every client needs a token (`Hello`'s, compared in constant time);
//! - an **absolute** deadline for the upgrade and the Hello together, and a small message limit
//!   until the Hello is accepted; connections capped in all, per address and per document;
//!   frames (not just messages) rate-limited; a slow reader's queue capped; idle and write
//!   deadlines; a bounded inbox to the hub;
//! - a document is the one canonical spelling of a `.fg` file inside the folder (no links, no dot
//!   names, case as on disk: `fsx`); validation reads no file outside the folder, and runs under
//!   a time budget; a document's text and log are capped;
//! - a client's ops are keyed by (account, client id), so one account cannot touch another's;
//! - build hashes are checked against the **server's own** build of that seq, at most one build
//!   per document per `HASH_BUILD_GAP`; a mismatch is told only to its reporter, and logged;
//! - a panic while handling a document quarantines that document; a panic of the hub ends the
//!   process (it never keeps accepting with no one to serve);
//! - the folder is locked: one server per folder.

pub mod fsx;
pub mod git;
pub mod store;

use forge_lang::error::Op;
use forge_sync::room::{Action, Conn, Room};
use forge_sync::wire::{Down, Up};
use forge_sync::{Check, MemStore, Refusal, Sequencer, Store};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The upgrade and the Hello together must be done within this (absolute, not per read).
pub const HANDSHAKE: Duration = Duration::from_secs(5);
/// The most a message may hold before the Hello is accepted, and after (an op's texts are at
/// most forge-sync's `MAX_OP_TEXT`).
pub const PRE_HELLO_MAX: usize = 4 << 10;
pub const UP_MAX: usize = 512 << 10;
/// A connection with nothing read for this long is closed (clients ping every 20 s).
pub const IDLE: Duration = Duration::from_secs(90);
/// A write that takes longer than this ends the connection.
pub const WRITE: Duration = Duration::from_secs(10);
/// Messages queued for one connection before it counts as too slow and is dropped.
pub const QUEUE: usize = 4096;
/// Frames per second one connection may send (sustained), and its burst.
pub const RATE: f64 = 200.0;
pub const BURST: f64 = 400.0;
/// Connections from one address.
pub const PER_IP: usize = 32;
/// Events waiting for the hub (readers wait when it is full).
pub const INBOX: usize = 256;
/// The largest document text the server opens.
pub const MAX_DOC: u64 = 4 << 20;
/// How long one op's validation may take, and how many validations past their budget may still
/// be running (a stuck one cannot be stopped, only abandoned) before the server refuses more.
pub const VALIDATE_BUDGET: Duration = Duration::from_secs(5);
pub const MAX_ABANDONED: usize = 4;
/// At most one version per document this often (each is a git commit, made on the hub).
pub const VERSION_GAP: Duration = Duration::from_secs(2);
/// At most one server build (to check a reported hash) per document this often.
pub const HASH_BUILD_GAP: Duration = Duration::from_secs(10);
/// Kept timings (a ring).
const STATS_KEPT: usize = 10_000;

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
    /// Serve on a non-loopback address with no TLS (the token and every op in clear): a LAN demo
    /// only, said loudly.
    pub insecure_lan: bool,
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
            insecure_lan: false,
        }
    }
}

/// Timings and counts the hub records (budget B2's server terms), in ms.
#[derive(Default, Debug, Clone)]
pub struct Stats {
    /// Rebase + validation + durable append of each committed op (the latest `STATS_KEPT`).
    pub commit_ms: Vec<f64>,
    pub refused: usize,
    pub committed: usize,
    /// Build hashes reported, checked against the server's own build, and found different.
    pub hashes: usize,
    pub hash_checks: usize,
    pub divergences: usize,
    pub versions: usize,
    pub checkpoints: usize,
    /// Documents quarantined after a panic.
    pub quarantined: usize,
}

/// A running server: where it listens and its counters.
pub struct Handle {
    pub addr: SocketAddr,
    pub stats: Arc<Mutex<Stats>>,
    stop: Arc<AtomicBool>,
    hub: Option<std::thread::JoinHandle<()>>,
}

impl Handle {
    /// Stops serving: no new connection, every connection closed, each document's head saved
    /// to git if it changed, the folder's lock let go. Returns when the hub has ended.
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // wake the accept loop
        let _ = TcpStream::connect(self.addr);
        if let Some(h) = self.hub.take() {
            let _ = h.join();
        }
    }
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
    /// The server's own build of `doc` at `seq` (None: it did not build).
    Built {
        doc: String,
        seq: u64,
        hash: Option<u64>,
    },
}

/// One connection's place, given back when it ends however it ends (a panic included).
struct Slot {
    live: Arc<AtomicUsize>,
    per_ip: Arc<Mutex<HashMap<IpAddr, usize>>>,
    ip: IpAddr,
}

impl Drop for Slot {
    fn drop(&mut self) {
        self.live.fetch_sub(1, Ordering::SeqCst);
        let mut m = self.per_ip.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(n) = m.get_mut(&self.ip) {
            *n -= 1;
            if *n == 0 {
                m.remove(&self.ip);
            }
        }
    }
}

/// Sockets with an absolute deadline: a watchdog shuts each down when its time is up, unless it
/// was marked done first.
type Deadlines = Arc<Mutex<Vec<(Instant, TcpStream, Arc<AtomicBool>)>>>;

fn watchdog(list: Deadlines) {
    loop {
        std::thread::sleep(Duration::from_millis(100));
        let now = Instant::now();
        let mut l = list.lock().unwrap_or_else(|e| e.into_inner());
        l.retain(|(at, s, done)| {
            if done.load(Ordering::SeqCst) {
                return false;
            }
            if now >= *at {
                let _ = s.shutdown(std::net::Shutdown::Both);
                return false;
            }
            true
        });
    }
}

/// Validation on its own thread under [`VALIDATE_BUDGET`]: a run past it is refused and
/// abandoned (forge-lang cannot be stopped midway); with [`MAX_ABANDONED`] still running, every
/// validation is refused until they end.
struct Timed {
    abandoned: Arc<AtomicUsize>,
}

impl Check for Timed {
    fn check(
        &mut self,
        head: &str,
        op: &Op,
        dir: Option<PathBuf>,
        root: Option<PathBuf>,
    ) -> Result<String, Refusal> {
        if self.abandoned.load(Ordering::SeqCst) >= MAX_ABANDONED {
            return Err(Refusal::Invalid {
                why: "the server is busy with edits that took too long: try again later".into(),
            });
        }
        // 0 running, 1 finished in time, 2 abandoned: whichever side moves it from 0 decides
        // who counts, so a run ending right at the deadline is never counted twice or lost
        let (tx, rx) = std::sync::mpsc::channel();
        let state = Arc::new(AtomicU8::new(0));
        let (head, op, gone, st) = (
            head.to_string(),
            op.clone(),
            self.abandoned.clone(),
            state.clone(),
        );
        std::thread::Builder::new()
            .name("forge-server validate".into())
            .spawn(move || {
                let r = catch_unwind(AssertUnwindSafe(|| forge_sync::validate(&head, &op, dir, root)))
                    .unwrap_or_else(|_| {
                        Err(Refusal::Invalid {
                            why: "validation failed".into(),
                        })
                    });
                if st
                    .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst)
                    .is_err()
                {
                    // it was abandoned: no one waits; it no longer runs
                    gone.fetch_sub(1, Ordering::SeqCst);
                }
                let _ = tx.send(r);
            })
            .map_err(|e| Refusal::Invalid { why: e.to_string() })?;
        match rx.recv_timeout(VALIDATE_BUDGET) {
            Ok(r) => r,
            Err(_) => {
                if state
                    .compare_exchange(0, 2, Ordering::SeqCst, Ordering::SeqCst)
                    .is_err()
                {
                    // it finished just now
                    return rx.recv().unwrap_or_else(|_| {
                        Err(Refusal::Invalid {
                            why: "validation failed".into(),
                        })
                    });
                }
                self.abandoned.fetch_add(1, Ordering::SeqCst);
                Err(Refusal::Invalid {
                    why: format!("checking this edit took over {} s", VALIDATE_BUDGET.as_secs()),
                })
            }
        }
    }
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
    if !addr.ip().is_loopback() && !c.insecure_lan {
        return Err(format!(
            "{addr} is not loopback: phase 1 has no TLS, so the token and every op would cross the network in clear \
             (insecure_lan / --insecure-lan to do it anyway)"
        ));
    }
    let root = c
        .root
        .canonicalize()
        .map_err(|e| format!("{}: {e}", c.root.display()))?;
    let c = Config { root, ..c };
    let lock = store::lock(&c.root).map_err(|e| e.to_string())?;
    let store: Box<dyn Store + Send> = match c.store {
        StoreKind::Mem => Box::new(MemStore::default()),
        StoreKind::File => Box::new(
            store::FileLogStore::new(c.root.join(".forgecad/server/log"))
                .map_err(|e| format!("the store: {e}"))?,
        ),
    };
    let marks = fsx::DiskMarks::new(c.root.join(".forgecad/server/disk")).map_err(|e| e.to_string())?;
    let stats = Arc::new(Mutex::new(Stats::default()));
    let (tx, rx) = sync_channel::<Ev>(INBOX);
    let c = Arc::new(c);
    let stop = Arc::new(AtomicBool::new(false));
    let hub_thread = {
        let (c, stats, tx, stop) = (c.clone(), stats.clone(), tx.clone(), stop.clone());
        std::thread::Builder::new()
            .name("forge-server hub".into())
            .spawn(move || {
                let _lock = lock;
                let r = catch_unwind(AssertUnwindSafe(|| hub(&c, store, marks, rx, tx, &stats, &stop)));
                if r.is_err() {
                    // never keep accepting with no hub: end the process
                    eprintln!("forge-server: the hub stopped on a panic; exiting");
                    std::process::exit(70);
                }
            })
            .map_err(|e| e.to_string())?
    };
    let deadlines: Deadlines = Arc::new(Mutex::new(vec![]));
    {
        let d = deadlines.clone();
        std::thread::Builder::new()
            .name("forge-server deadlines".into())
            .spawn(move || watchdog(d))
            .map_err(|e| e.to_string())?;
    }
    let live = Arc::new(AtomicUsize::new(0));
    let per_ip: Arc<Mutex<HashMap<IpAddr, usize>>> = Arc::new(Mutex::new(HashMap::new()));
    let stop2 = stop.clone();
    std::thread::Builder::new()
        .name("forge-server accept".into())
        .spawn(move || {
            let mut next: Conn = 0;
            for s in l.incoming() {
                if stop2.load(Ordering::SeqCst) {
                    return;
                }
                let Ok(mut s) = s else { continue };
                let Ok(peer) = s.peer_addr() else { continue };
                next += 1;
                let ip = peer.ip();
                let over = {
                    let mut m = per_ip.lock().unwrap_or_else(|e| e.into_inner());
                    let n = m.entry(ip).or_insert(0);
                    let over = live.load(Ordering::SeqCst) >= c.max_conns || *n >= PER_IP;
                    if !over {
                        *n += 1;
                        live.fetch_add(1, Ordering::SeqCst);
                    }
                    over
                };
                if over {
                    let _ = std::io::Write::write_all(
                        &mut s,
                        b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    );
                    continue;
                }
                let slot = Slot {
                    live: live.clone(),
                    per_ip: per_ip.clone(),
                    ip,
                };
                let (c, tx, d) = (c.clone(), tx.clone(), deadlines.clone());
                let conn = next;
                let _ = std::thread::Builder::new()
                    .name(format!("forge-server conn {conn}"))
                    .spawn(move || {
                        let _slot = slot;
                        connection(s, conn, &c, &tx, &d);
                    });
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(Handle {
        addr,
        stats,
        stop,
        hub: Some(hub_thread),
    })
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

fn connection(s: TcpStream, conn: Conn, c: &Config, tx: &SyncSender<Ev>, deadlines: &Deadlines) {
    let _ = s.set_read_timeout(Some(HANDSHAKE));
    let _ = s.set_write_timeout(Some(WRITE));
    let _ = s.set_nodelay(true);
    // the upgrade and the Hello: done by the deadline, or the socket is shut
    let done = Arc::new(AtomicBool::new(false));
    if let Ok(s2) = s.try_clone() {
        deadlines.lock().unwrap_or_else(|e| e.into_inner()).push((
            Instant::now() + HANDSHAKE,
            s2,
            done.clone(),
        ));
    } else {
        return;
    }
    let accepted = forge_ws::accept(s, PRE_HELLO_MAX, |q| {
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
    if let Err(why) = fsx::canonical_doc(&c.root, &doc) {
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
    done.store(true, Ordering::SeqCst);
    r.set_max(UP_MAX);
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
    // ops are keyed by (account, client id): one account can never reach another's
    let key = format!("{author}/{client}");
    if tx
        .send(Ev::Join {
            conn,
            doc,
            client: key,
            author,
            seq,
            out: otx.clone(),
        })
        .is_err()
    {
        return;
    }
    let (mut tokens, mut last, mut frames) = (BURST, Instant::now(), r.frames);
    loop {
        let got = r.read();
        // every frame costs, pings and empty continuations too
        let now = Instant::now();
        let used = (r.frames - frames) as f64;
        frames = r.frames;
        tokens = (tokens + now.duration_since(last).as_secs_f64() * RATE).min(BURST) - used;
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
        let m = match got {
            Ok(forge_ws::Msg::Text(t)) => t,
            Ok(forge_ws::Msg::Ping(p)) => {
                let _ = otx.try_send(Out::Pong(p));
                continue;
            }
            Ok(forge_ws::Msg::Close) | Err(_) => break,
        };
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
    last_version: Option<Instant>,
    /// The server's own build hash per seq (bounded), and reports waiting for one.
    built: BTreeMap<u64, Option<u64>>,
    waiting: Vec<(u64, u64, String, Conn)>,
    last_build: Option<Instant>,
}

type Conns = HashMap<Conn, (String, SyncSender<Out>)>;

/// Sends each message to its connection; those too slow (queue full) or gone are returned.
fn send(conns: &Conns, out: Vec<(Conn, Down)>) -> Vec<Conn> {
    let mut slow = vec![];
    for (k, d) in out {
        if let Some((_, tx)) = conns.get(&k) {
            match tx.try_send(Out::Text(d.text())) {
                Ok(()) => {}
                Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => slow.push(k),
            }
        }
    }
    slow
}

struct Hub<'a> {
    c: &'a Config,
    store: Box<dyn Store + Send>,
    marks: fsx::DiskMarks,
    stats: &'a Mutex<Stats>,
    docs: HashMap<String, Doc>,
    conns: Conns,
    quarantined: HashSet<String>,
    abandoned: Arc<AtomicUsize>,
    /// Builds for hash checks run here, off the hub.
    builder: SyncSender<(String, u64, String, Option<PathBuf>, PathBuf)>,
}

fn hub(
    c: &Config,
    store: Box<dyn Store + Send>,
    marks: fsx::DiskMarks,
    rx: Receiver<Ev>,
    tx: SyncSender<Ev>,
    stats: &Mutex<Stats>,
    stop: &AtomicBool,
) {
    let (btx, brx) = sync_channel::<(String, u64, String, Option<PathBuf>, PathBuf)>(4);
    std::thread::Builder::new()
        .name("forge-server builds".into())
        .spawn(move || {
            for (doc, seq, text, dir, root) in brx {
                let hash = catch_unwind(AssertUnwindSafe(|| {
                    let o = forge_lang::Options {
                        dir,
                        root: Some(root),
                        ..forge_lang::Options::default()
                    };
                    let (run, _) = forge_lang::run(&text, o, Default::default());
                    run.body.as_ref().map(forge_sync::hash_body)
                }))
                .ok()
                .flatten();
                // the hub may be busy: a check dropped is only a check not made
                let _ = tx.try_send(Ev::Built { doc, seq, hash });
            }
        })
        .expect("a thread");
    let mut h = Hub {
        c,
        store,
        marks,
        stats,
        docs: HashMap::new(),
        conns: HashMap::new(),
        quarantined: HashSet::new(),
        abandoned: Arc::new(AtomicUsize::new(0)),
        builder: btx,
    };
    loop {
        if stop.load(Ordering::SeqCst) {
            let docs: Vec<String> = h.docs.keys().cloned().collect();
            for d in docs {
                h.checkpoint(&d);
            }
            let all: Vec<Conn> = h.conns.keys().copied().collect();
            h.drop_conns(all);
            return;
        }
        let ev = match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(ev) => Some(ev),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
        };
        if let Some(ev) = ev {
            let doc_of = match &ev {
                Ev::Join { doc, .. } | Ev::Built { doc, .. } => Some(doc.clone()),
                Ev::Up { conn, .. } | Ev::Leave { conn } => h.conns.get(conn).map(|x| x.0.clone()),
            };
            // a panic handling one document's event quarantines that document, not the server
            if catch_unwind(AssertUnwindSafe(|| h.event(ev))).is_err()
                && let Some(d) = doc_of
            {
                eprintln!("forge-server: {d}: a panic while handling it: quarantined");
                h.quarantine(&d);
            }
        }
        h.idle();
    }
}

impl Hub<'_> {
    fn lock_stats(&self) -> std::sync::MutexGuard<'_, Stats> {
        self.stats.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn quarantine(&mut self, doc: &str) {
        self.quarantined.insert(doc.to_string());
        self.docs.remove(doc);
        let gone: Vec<Conn> = self
            .conns
            .iter()
            .filter(|(_, v)| v.0 == doc)
            .map(|(k, _)| *k)
            .collect();
        for k in gone {
            if let Some((_, tx)) = self.conns.remove(&k) {
                let _ = tx.try_send(Out::Text(
                    Down::Error {
                        text: "this document is quarantined after a server error".into(),
                    }
                    .text(),
                ));
                let _ = tx.try_send(Out::Close);
            }
        }
        self.lock_stats().quarantined += 1;
    }

    fn drop_conns(&mut self, ks: Vec<Conn>) {
        for k in ks {
            if let Some((doc, tx)) = self.conns.remove(&k) {
                let _ = tx.try_send(Out::Close);
                if let Some(d) = self.docs.get_mut(&doc) {
                    d.room.leave(k);
                }
            }
        }
    }

    fn event(&mut self, ev: Ev) {
        match ev {
            Ev::Join {
                conn,
                doc,
                client,
                author,
                seq,
                out,
            } => {
                let refuse = |out: &SyncSender<Out>, e: String| {
                    let _ = out.try_send(Out::Text(Down::Error { text: e }.text()));
                    let _ = out.try_send(Out::Close);
                };
                if self.quarantined.contains(&doc) {
                    return refuse(&out, "this document is quarantined after a server error".into());
                }
                if !self.docs.contains_key(&doc) {
                    match self.open_doc(&doc) {
                        Ok(room) => {
                            self.docs.insert(
                                doc.clone(),
                                Doc {
                                    saved: room.seq.seq(),
                                    room,
                                    last_op: Instant::now(),
                                    last_version: None,
                                    built: BTreeMap::new(),
                                    waiting: vec![],
                                    last_build: None,
                                },
                            );
                        }
                        Err(e) => return refuse(&out, e),
                    }
                }
                let d = self.docs.get_mut(&doc).expect("opened");
                if d.room.members() >= self.c.max_per_doc {
                    return refuse(&out, "too many people in this document".into());
                }
                let o = d.room.join(conn, &client, &author, seq);
                self.conns.insert(conn, (doc, out));
                let slow = send(&self.conns, o);
                self.drop_conns(slow);
            }
            Ev::Up { conn, up } => {
                let Some((doc, _)) = self.conns.get(&conn) else {
                    return;
                };
                let doc = doc.clone();
                if matches!(up, Up::BuildHash { .. }) {
                    self.lock_stats().hashes += 1;
                }
                let Some(d) = self.docs.get_mut(&doc) else { return };
                let is_op = matches!(up, Up::Op { .. });
                let t0 = Instant::now();
                let (out, actions) = d.room.receive(self.store.as_mut(), conn, up);
                if is_op {
                    let ms = t0.elapsed().as_secs_f64() * 1e3;
                    let refused = matches!(out.first(), Some((_, Down::Refused { .. })));
                    if !refused {
                        d.last_op = Instant::now();
                    }
                    let mut s = self.stats.lock().unwrap_or_else(|e| e.into_inner());
                    if refused {
                        s.refused += 1;
                    } else {
                        s.committed += 1;
                        if s.commit_ms.len() >= STATS_KEPT {
                            s.commit_ms.remove(0);
                        }
                        s.commit_ms.push(ms);
                    }
                }
                let slow = send(&self.conns, out);
                self.drop_conns(slow);
                for a in actions {
                    self.action(&doc, a);
                }
            }
            Ev::Leave { conn } => {
                if let Some((doc, _)) = self.conns.remove(&conn)
                    && let Some(d) = self.docs.get_mut(&doc)
                {
                    d.room.leave(conn);
                    if d.room.members() == 0 {
                        // no one left: save what changed, then let it go (its log stays)
                        self.checkpoint(&doc);
                        self.docs.remove(&doc);
                    }
                }
            }
            Ev::Built { doc, seq, hash } => {
                let Some(d) = self.docs.get_mut(&doc) else { return };
                d.built.insert(seq, hash);
                while d.built.len() > 1024 {
                    d.built.pop_first();
                }
                let (now, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut d.waiting)
                    .into_iter()
                    .partition(|w| w.0 == seq);
                d.waiting = rest;
                for (seq, got, author, conn) in now {
                    self.judge(&doc, seq, hash, got, &author, conn);
                }
            }
        }
    }

    /// A reported hash against the server's own build of that seq.
    fn judge(&mut self, doc: &str, seq: u64, mine: Option<u64>, got: u64, author: &str, conn: Conn) {
        let Some(mine) = mine else { return };
        self.lock_stats().hash_checks += 1;
        if mine == got {
            return;
        }
        self.lock_stats().divergences += 1;
        eprintln!(
            "forge-server: DIVERGENCE {doc} seq {seq}: {author} built {got:016x}, the server {mine:016x}"
        );
        let d = Down::Diverged {
            seq,
            expected: mine,
            got,
            by: author.to_string(),
        };
        // told to the one that built differently: it shows the server's result (DESIGN.md §3)
        let slow = send(&self.conns, vec![(conn, d)]);
        self.drop_conns(slow);
    }

    fn action(&mut self, doc: &str, a: Action) {
        match a {
            Action::Version { name, author, conn } => {
                let Some(d) = self.docs.get_mut(doc) else { return };
                if d.last_version.is_some_and(|t| t.elapsed() < VERSION_GAP) {
                    let slow = send(
                        &self.conns,
                        vec![(
                            conn,
                            Down::Error {
                                text: "one version every 2 s".into(),
                            },
                        )],
                    );
                    return self.drop_conns(slow);
                }
                d.last_version = Some(Instant::now());
                let (head, seq) = (d.room.seq.head().to_string(), d.room.seq.seq());
                match git::version(&self.c.root, doc, &head, &name, &author, seq) {
                    Ok(commit) => {
                        let _ = self.marks.set(doc, &head);
                        let d = self.docs.get_mut(doc).expect("there");
                        d.saved = seq;
                        let o = d.room.versioned(&name, seq, &commit);
                        self.lock_stats().versions += 1;
                        let slow = send(&self.conns, o);
                        self.drop_conns(slow);
                    }
                    Err(e) => {
                        let o = vec![(
                            conn,
                            Down::Error {
                                text: format!("the version was not written: {e}"),
                            },
                        )];
                        let slow = send(&self.conns, o);
                        self.drop_conns(slow);
                    }
                }
            }
            Action::CheckHash {
                seq,
                hash,
                author,
                conn,
            } => {
                let Some(d) = self.docs.get_mut(doc) else { return };
                if let Some(&mine) = d.built.get(&seq) {
                    return self.judge(doc, seq, mine, hash, &author, conn);
                }
                if d.waiting.len() >= 64 {
                    return;
                }
                let already = d.waiting.iter().any(|w| w.0 == seq);
                d.waiting.push((seq, hash, author, conn));
                if already || d.last_build.is_some_and(|t| t.elapsed() < HASH_BUILD_GAP) {
                    // rate-limited: kept only while that seq's build is on its way
                    if !already {
                        d.waiting.pop();
                    }
                    return;
                }
                let Some(text) = d.room.seq.text_at(seq) else {
                    return;
                };
                if self
                    .builder
                    .try_send((
                        doc.to_string(),
                        seq,
                        text,
                        d.room.seq.dir.clone(),
                        self.c.root.clone(),
                    ))
                    .is_ok()
                {
                    d.last_build = Some(Instant::now());
                } else {
                    d.waiting.retain(|w| w.0 != seq);
                }
            }
        }
    }

    fn checkpoint(&mut self, doc: &str) {
        let Some(d) = self.docs.get_mut(doc) else { return };
        if d.room.seq.seq() <= d.saved {
            return;
        }
        let (head, seq) = (d.room.seq.head().to_string(), d.room.seq.seq());
        d.saved = seq;
        match git::checkpoint(&self.c.root, doc, &head, seq) {
            Ok(()) => {
                let _ = self.marks.set(doc, &head);
                self.lock_stats().checkpoints += 1;
            }
            Err(e) => eprintln!("forge-server: {doc}: checkpoint not written: {e}"),
        }
    }

    /// Idle checkpoints: a document quiet for a while gets its head saved in git.
    fn idle(&mut self) {
        let due: Vec<String> = self
            .docs
            .iter()
            .filter(|(_, d)| d.room.seq.seq() > d.saved && d.last_op.elapsed() >= self.c.checkpoint_idle)
            .map(|(k, _)| k.clone())
            .collect();
        for doc in due {
            self.checkpoint(&doc);
        }
    }

    fn open_doc(&mut self, doc: &str) -> Result<Room, String> {
        let path = fsx::canonical_doc(&self.c.root, doc).map_err(str::to_string)?;
        let len = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
        if len > MAX_DOC {
            return Err(format!("{doc}: {len} bytes (at most {MAX_DOC})"));
        }
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{doc}: {e}"))?;
        // a file changed outside the server since it last wrote it: never overwritten silently
        match self.marks.get(doc) {
            Some(h) if h != forge_sync::text_hash(&text) => {
                return Err(format!(
                    "{doc} changed on disk outside the server (an editor, a git pull): bringing such a change in \
                     is not in phase 1 (DESIGN.md §15.4)"
                ));
            }
            Some(_) => {}
            None => self.marks.set(doc, &text).map_err(|e| e.to_string())?,
        }
        let mut seq = Sequencer::open(
            self.store.as_mut(),
            doc,
            &text,
            path.parent().map(Path::to_path_buf),
        )
        .map_err(|e| e.to_string())?;
        seq.root = Some(self.c.root.clone());
        seq.checker = Box::new(Timed {
            abandoned: self.abandoned.clone(),
        });
        Ok(Room::new(seq))
    }
}
