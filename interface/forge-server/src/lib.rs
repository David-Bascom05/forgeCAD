//! forge-server, phase 1 of research/hosted/DESIGN.md: one process serving one project folder
//! (plain files + git) to collaborating clients over WebSocket.
//!
//! Threads, no async runtime:
//! - an accept loop; per connection a reader (frames → messages) and a writer (a queue bounded
//!   in bytes → frames);
//! - a **hub** that only routes: which connection is in which document, opening and closing
//!   documents;
//! - one **actor thread per open document** that owns its [`Room`] (forge-sync's sequencer and
//!   members) and its store: each document's ops are ordered by one thread, validated before they
//!   are appended (on a thread of their own, under a time budget), appended durably before anyone
//!   hears of them, and broadcast in seq order. A slow document (a heavy part's every check takes
//!   seconds) slows only itself.
//!
//! Hardening, after forge-cli's `forgecad web` (interface/forge-cli/src/web.rs) and two security
//! reviews of this crate (their findings are tests/security.rs):
//! - binds 127.0.0.1; any other address only with `insecure_lan` (no TLS in phase 1); on loopback
//!   only a loopback `Host`; a browser's `Origin` must be the server's own (or `--origin`);
//! - every client needs a token (`Hello`'s, compared in constant time), the server's protocol and
//!   kernel build (geometry is bit-identical only within one kernel);
//! - an absolute deadline for the upgrade and the Hello; a small message limit and a small cap
//!   on connections until the Hello is accepted; per-address caps (IPv6 by /64; loopback exempt
//!   by default); frames rate-limited; each connection's queue bounded in bytes;
//! - a document is the one canonical spelling of a `.fg` file inside the folder (no links, hard
//!   ones included; no dot names; case as on disk); validation reads no file outside the folder;
//! - a document's text, log (by bytes: compacted to a snapshot), catch-ups and hash checks are
//!   bounded; validation and hash-check builds run under time budgets, abandoned past them,
//!   counted per document;
//! - ops are keyed by (account, client id); versions are rate-limited per account;
//! - build hashes are checked against the server's own build, never another client's word;
//! - a file changed on disk outside the server is never overwritten;
//! - a panic in a document's actor quarantines that document; a panic of the hub marks the
//!   server dead (no more accepting): the binary then exits with 70.

pub mod fsx;
pub mod git;
pub mod store;

use forge_lang::error::Op;
use forge_sync::room::{Action, Conn, Room};
use forge_sync::wire::{Down, Up};
use forge_sync::{Check, Counts, MemStore, Refusal, Sequencer, Store};
use std::collections::{HashMap, HashSet};
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
/// Connections that have not finished their Hello, in all.
pub const PRE_AUTH: usize = 64;
/// A connection with nothing read for this long is closed (clients ping every 20 s).
pub const IDLE: Duration = Duration::from_secs(90);
/// A write that takes longer than this ends the connection.
pub const WRITE: Duration = Duration::from_secs(10);
/// Bytes queued for one connection before it counts as too slow and is dropped (it reconnects
/// and catches up, or takes a snapshot).
pub const QUEUE_BYTES: usize = 16 << 20;
/// Frames per second one connection may send (sustained), and its burst.
pub const RATE: f64 = 200.0;
pub const BURST: f64 = 400.0;
/// Messages waiting for the hub, and for one document's actor.
pub const INBOX: usize = 256;
pub const DOC_INBOX: usize = 1024;
/// The largest document text the server opens.
pub const MAX_DOC: u64 = 4 << 20;
/// How long one op's validation may take, and how many of one document's validations past their
/// budget may still be running (a stuck one cannot be stopped, only abandoned) before that
/// document refuses edits.
pub const VALIDATE_BUDGET: Duration = Duration::from_secs(5);
pub const MAX_ABANDONED: usize = 4;
/// One account's versions: at most one this often (each is a git commit).
pub const VERSION_GAP: Duration = Duration::from_secs(2);
/// Hash checks: at most one server build per document this often, each within
/// `BUILD_BUDGET`, and at most `BUILDS` running in the whole server.
pub const HASH_BUILD_GAP: Duration = Duration::from_secs(10);
pub const BUILD_BUDGET: Duration = Duration::from_secs(30);
pub const BUILDS: usize = 2;
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
    /// Connections from one address (IPv6: one /64); loopback is exempt unless `cap_loopback`
    /// (on one machine, SSH tunnels and proxies, every client is loopback).
    pub per_ip: usize,
    pub cap_loopback: bool,
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
            per_ip: 32,
            cap_loopback: false,
            checkpoint_idle: Duration::from_secs(300),
            insecure_lan: false,
        }
    }
}

/// Timings and counts (budget B2's server terms, in ms).
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
    pub compactions: usize,
    /// Documents quarantined after a panic, and documents open now.
    pub quarantined: usize,
    pub open_docs: usize,
}

/// A running server: where it listens, its counters, and whether its hub died.
pub struct Handle {
    pub addr: SocketAddr,
    pub stats: Arc<Mutex<Stats>>,
    stop: Arc<AtomicBool>,
    dead: Arc<AtomicBool>,
    hub: Option<std::thread::JoinHandle<()>>,
}

impl Handle {
    /// Stops serving: no new connection, every document's head saved and its actor ended, every
    /// connection closed, the folder's lock let go. Returns when the hub has ended.
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
        if let Some(h) = self.hub.take() {
            let _ = h.join();
        }
    }
    /// True if the hub stopped on a panic: the server serves no one and should exit (the binary
    /// exits with 70).
    pub fn dead(&self) -> bool {
        self.dead.load(Ordering::SeqCst)
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

/// The key one address's connections are counted under: an IPv6 address by its /64 (one host's
/// usual allotment), an IPv4 one whole.
pub fn ip_key(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return IpAddr::V4(v4);
            }
            let mut s = v6.segments();
            s[4..].iter_mut().for_each(|x| *x = 0);
            IpAddr::from(s)
        }
        v4 => v4,
    }
}

enum Out {
    Text(String),
    Pong(Vec<u8>),
    Close,
}

/// A connection's queue to its writer, bounded in bytes.
#[derive(Clone)]
struct Outbox {
    tx: SyncSender<(Out, usize)>,
    bytes: Arc<AtomicUsize>,
}

impl Outbox {
    /// Queued, or false: the connection is too slow (or gone) and must be dropped.
    fn send(&self, m: Out) -> bool {
        let n = match &m {
            Out::Text(t) => t.len(),
            Out::Pong(p) => p.len(),
            Out::Close => 0,
        };
        if self.bytes.fetch_add(n, Ordering::SeqCst) + n > QUEUE_BYTES && n > 0 {
            self.bytes.fetch_sub(n, Ordering::SeqCst);
            return false;
        }
        match self.tx.try_send((m, n)) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                self.bytes.fetch_sub(n, Ordering::SeqCst);
                false
            }
        }
    }
    fn down(&self, d: &Down) -> bool {
        self.send(Out::Text(d.text()))
    }
    fn close(&self, why: Option<&str>) {
        if let Some(w) = why {
            let _ = self.down(&Down::Error { text: w.into() });
        }
        let _ = self.send(Out::Close);
    }
}

enum Ev {
    Join {
        conn: Conn,
        doc: String,
        client: String,
        author: String,
        seq: Option<u64>,
        out: Outbox,
    },
    Up {
        conn: Conn,
        up: Up,
    },
    Leave {
        conn: Conn,
    },
    /// A document's actor ended on a panic.
    Quarantine {
        doc: String,
    },
}

/// One connection's place, given back when it ends however it ends (a panic included).
struct Slot {
    live: Arc<AtomicUsize>,
    pre_auth: Arc<AtomicUsize>,
    authed: bool,
    per_ip: Arc<Mutex<HashMap<IpAddr, usize>>>,
    ip: Option<IpAddr>,
}

impl Slot {
    fn authenticated(&mut self) {
        if !self.authed {
            self.authed = true;
            self.pre_auth.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        self.live.fetch_sub(1, Ordering::SeqCst);
        if !self.authed {
            self.pre_auth.fetch_sub(1, Ordering::SeqCst);
        }
        if let Some(ip) = self.ip {
            let mut m = self.per_ip.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(n) = m.get_mut(&ip) {
                *n -= 1;
                if *n == 0 {
                    m.remove(&ip);
                }
            }
        }
    }
}

/// Sockets with an absolute deadline: a watchdog shuts each down when its time is up, unless it
/// was marked done first.
type Deadlines = Arc<Mutex<Vec<(Instant, TcpStream, Arc<AtomicBool>)>>>;

fn watchdog(list: Deadlines, stop: Arc<AtomicBool>, dead: Arc<AtomicBool>) {
    while !stop.load(Ordering::SeqCst) && !dead.load(Ordering::SeqCst) {
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
/// abandoned (forge-lang cannot be stopped midway); with [`MAX_ABANDONED`] of this document's
/// still running, its edits are refused until they end. (A step and memory budget inside
/// forge-lang, so a run can be stopped, is the kernel's to give: KERNEL-STRESS's op budget.)
struct Timed {
    abandoned: Arc<AtomicUsize>,
}

impl Check for Timed {
    fn check(
        &mut self,
        head: &str,
        counts: Option<&Counts>,
        op: &Op,
        dir: Option<PathBuf>,
        root: Option<PathBuf>,
    ) -> Result<(String, Counts), Refusal> {
        if self.abandoned.load(Ordering::SeqCst) >= MAX_ABANDONED {
            return Err(Refusal::Invalid {
                why: "this document is busy with edits that took too long: try again later".into(),
            });
        }
        // 0 running, 1 finished in time, 2 abandoned: whichever side moves it from 0 decides
        // who counts, so a run ending right at the deadline is never counted twice or lost
        let (tx, rx) = std::sync::mpsc::channel();
        let state = Arc::new(AtomicU8::new(0));
        let (head, counts, op, gone, st) = (
            head.to_string(),
            counts.cloned(),
            op.clone(),
            self.abandoned.clone(),
            state.clone(),
        );
        std::thread::Builder::new()
            .name("forge-server validate".into())
            .spawn(move || {
                let r = catch_unwind(AssertUnwindSafe(|| {
                    forge_sync::validate(&head, counts.as_ref(), &op, dir, root)
                }))
                .unwrap_or_else(|_| {
                    Err(Refusal::Invalid {
                        why: "validation failed".into(),
                    })
                });
                if st
                    .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst)
                    .is_err()
                {
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

/// The in-memory store shared by every document's actor.
struct SharedMem(Arc<Mutex<MemStore>>);

impl Store for SharedMem {
    fn latest_snapshot(&mut self, doc: &str) -> std::io::Result<Option<(u64, String)>> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .latest_snapshot(doc)
    }
    fn read_since(&mut self, doc: &str, seq: u64) -> std::io::Result<Vec<forge_sync::Entry>> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .read_since(doc, seq)
    }
    fn create(&mut self, doc: &str, text: &str) -> std::io::Result<()> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).create(doc, text)
    }
    fn append(&mut self, doc: &str, e: &forge_sync::Entry) -> std::io::Result<()> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).append(doc, e)
    }
    fn compact(&mut self, doc: &str, seq: u64, text: &str) -> std::io::Result<()> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .compact(doc, seq, text)
    }
}

/// What every document's actor shares.
struct Shared {
    c: Config,
    stats: Arc<Mutex<Stats>>,
    marks: fsx::DiskMarks,
    mem: Arc<Mutex<MemStore>>,
    /// Each account's last version.
    versions: Mutex<HashMap<String, Instant>>,
    /// Hash-check builds running in the whole server.
    builds: Arc<AtomicUsize>,
}

impl Shared {
    fn stats(&self) -> std::sync::MutexGuard<'_, Stats> {
        self.stats.lock().unwrap_or_else(|e| e.into_inner())
    }
    fn store(&self) -> Result<Box<dyn Store + Send>, String> {
        Ok(match self.c.store {
            StoreKind::Mem => Box::new(SharedMem(self.mem.clone())),
            StoreKind::File => Box::new(
                store::FileLogStore::new(self.c.root.join(".forgecad/server/log"))
                    .map_err(|e| format!("the store: {e}"))?,
            ),
        })
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
    let marks = fsx::DiskMarks::new(c.root.join(".forgecad/server/disk")).map_err(|e| e.to_string())?;
    let stats = Arc::new(Mutex::new(Stats::default()));
    let shared = Arc::new(Shared {
        c: c.clone(),
        stats: stats.clone(),
        marks,
        mem: Arc::new(Mutex::new(MemStore::default())),
        versions: Mutex::new(HashMap::new()),
        builds: Arc::new(AtomicUsize::new(0)),
    });
    let (tx, rx) = sync_channel::<Ev>(INBOX);
    let stop = Arc::new(AtomicBool::new(false));
    let dead = Arc::new(AtomicBool::new(false));
    let hub_thread = {
        let (shared, tx, stop, dead) = (shared.clone(), tx.clone(), stop.clone(), dead.clone());
        std::thread::Builder::new()
            .name("forge-server hub".into())
            .spawn(move || {
                let _lock = lock;
                if catch_unwind(AssertUnwindSafe(|| hub(&shared, rx, tx, &stop))).is_err() {
                    eprintln!("forge-server: the hub stopped on a panic: serving no one");
                    dead.store(true, Ordering::SeqCst);
                }
            })
            .map_err(|e| e.to_string())?
    };
    let deadlines: Deadlines = Arc::new(Mutex::new(vec![]));
    {
        let (d, s, x) = (deadlines.clone(), stop.clone(), dead.clone());
        std::thread::Builder::new()
            .name("forge-server deadlines".into())
            .spawn(move || watchdog(d, s, x))
            .map_err(|e| e.to_string())?;
    }
    let live = Arc::new(AtomicUsize::new(0));
    let pre_auth = Arc::new(AtomicUsize::new(0));
    let per_ip: Arc<Mutex<HashMap<IpAddr, usize>>> = Arc::new(Mutex::new(HashMap::new()));
    let (stop2, dead2) = (stop.clone(), dead.clone());
    let c = Arc::new(c);
    std::thread::Builder::new()
        .name("forge-server accept".into())
        .spawn(move || {
            let mut next: Conn = 0;
            for s in l.incoming() {
                if stop2.load(Ordering::SeqCst) || dead2.load(Ordering::SeqCst) {
                    return;
                }
                let Ok(mut s) = s else { continue };
                let Ok(peer) = s.peer_addr() else { continue };
                next += 1;
                let counted = (!peer.ip().is_loopback() || c.cap_loopback).then(|| ip_key(peer.ip()));
                let admitted = {
                    let mut m = per_ip.lock().unwrap_or_else(|e| e.into_inner());
                    let ok = live.load(Ordering::SeqCst) < c.max_conns
                        && pre_auth.load(Ordering::SeqCst) < PRE_AUTH
                        && counted.is_none_or(|k| m.get(&k).copied().unwrap_or(0) < c.per_ip);
                    if ok {
                        // counted only when admitted: a refused address leaves nothing behind
                        if let Some(k) = counted {
                            *m.entry(k).or_insert(0) += 1;
                        }
                        live.fetch_add(1, Ordering::SeqCst);
                        pre_auth.fetch_add(1, Ordering::SeqCst);
                    }
                    ok
                };
                if !admitted {
                    let _ = std::io::Write::write_all(
                        &mut s,
                        b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    );
                    continue;
                }
                let slot = Slot {
                    live: live.clone(),
                    pre_auth: pre_auth.clone(),
                    authed: false,
                    per_ip: per_ip.clone(),
                    ip: counted,
                };
                let (c, tx, d) = (c.clone(), tx.clone(), deadlines.clone());
                let conn = next;
                let _ = std::thread::Builder::new()
                    .name(format!("forge-server conn {conn}"))
                    .spawn(move || connection(s, conn, &c, &tx, &d, slot));
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(Handle {
        addr,
        stats,
        stop,
        dead,
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

fn connection(
    s: TcpStream,
    conn: Conn,
    c: &Config,
    tx: &SyncSender<Ev>,
    deadlines: &Deadlines,
    mut slot: Slot,
) {
    let _ = s.set_read_timeout(Some(HANDSHAKE));
    let _ = s.set_write_timeout(Some(WRITE));
    let _ = s.set_nodelay(true);
    // the upgrade and the Hello: done by the deadline, or the socket is shut
    let done = Arc::new(AtomicBool::new(false));
    let Ok(s2) = s.try_clone() else { return };
    deadlines
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push((Instant::now() + HANDSHAKE, s2, done.clone()));
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
        proto,
        kernel,
    }) = hello
    else {
        return fail(&mut w, "the first message is hello");
    };
    let Some(author) = author_of(c, &token) else {
        return fail(&mut w, "a valid token is needed");
    };
    if proto != forge_sync::PROTOCOL {
        return fail(
            &mut w,
            &format!("protocol {proto}: this server speaks {}", forge_sync::PROTOCOL),
        );
    }
    if kernel != forge_sync::KERNEL {
        // bit-identical geometry holds only within one kernel build: one per room
        return fail(
            &mut w,
            &format!(
                "kernel {}: this server's is {} (update the app to the server's build)",
                kernel.chars().take(80).collect::<String>(),
                forge_sync::KERNEL
            ),
        );
    }
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
    slot.authenticated();
    r.set_max(UP_MAX);
    let _ = r.stream().set_read_timeout(Some(IDLE));
    let (otx, orx) = sync_channel::<(Out, usize)>(65536);
    let bytes = Arc::new(AtomicUsize::new(0));
    let out = Outbox {
        tx: otx,
        bytes: bytes.clone(),
    };
    let writer = std::thread::Builder::new()
        .name(format!("forge-server write {conn}"))
        .spawn(move || {
            for (m, n) in orx {
                let ok = match m {
                    Out::Text(t) => w.send(&t).is_ok(),
                    Out::Pong(p) => w.pong(&p).is_ok(),
                    Out::Close => {
                        w.close();
                        false
                    }
                };
                bytes.fetch_sub(n, Ordering::SeqCst);
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
            out: out.clone(),
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
            out.close(Some("too many messages"));
            break;
        }
        let m = match got {
            Ok(forge_ws::Msg::Text(t)) => t,
            Ok(forge_ws::Msg::Ping(p)) => {
                let _ = out.send(Out::Pong(p));
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
                out.close(Some(&format!("a bad message: {e}")));
                break;
            }
        }
    }
    out.close(None);
    let _ = r.stream().shutdown(std::net::Shutdown::Read);
    let _ = tx.send(Ev::Leave { conn });
}

/// A message to a document's actor.
enum DocEv {
    Join {
        conn: Conn,
        client: String,
        author: String,
        seq: Option<u64>,
        out: Outbox,
    },
    Up {
        conn: Conn,
        up: Up,
    },
    Leave {
        conn: Conn,
    },
    /// A hash-check build finished: the server's own hash of the text at `seq`.
    Built {
        seq: u64,
        hash: Option<u64>,
    },
    /// Save and end (no members left, or the server stops).
    Close,
}

struct DocHandle {
    tx: SyncSender<DocEv>,
    thread: std::thread::JoinHandle<()>,
    members: usize,
}

fn hub(shared: &Arc<Shared>, rx: Receiver<Ev>, tx: SyncSender<Ev>, stop: &AtomicBool) {
    let mut docs: HashMap<String, DocHandle> = HashMap::new();
    let mut conns: HashMap<Conn, (String, Outbox)> = HashMap::new();
    let mut quarantined: HashSet<String> = HashSet::new();
    let close = |docs: &mut HashMap<String, DocHandle>, doc: &str| {
        if let Some(d) = docs.remove(doc) {
            let _ = d.tx.send(DocEv::Close);
            let _ = d.thread.join();
        }
    };
    loop {
        if stop.load(Ordering::SeqCst) {
            for (_, (_, out)) in conns.drain() {
                out.close(Some("the server is stopping"));
            }
            let all: Vec<String> = docs.keys().cloned().collect();
            for d in all {
                close(&mut docs, &d);
            }
            return;
        }
        let ev = match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(ev) => ev,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
        };
        match ev {
            Ev::Join {
                conn,
                doc,
                client,
                author,
                seq,
                out,
            } => {
                #[cfg(feature = "test-hooks")]
                if doc == "test_hook_hub_panic.fg" {
                    panic!("test hook: a panic in the hub");
                }
                if quarantined.contains(&doc) {
                    out.close(Some("this document is quarantined after a server error"));
                    continue;
                }
                if !docs.contains_key(&doc) {
                    match open_actor(shared, &doc, tx.clone()) {
                        Ok(h) => {
                            docs.insert(doc.clone(), h);
                        }
                        Err(e) => {
                            out.close(Some(&e));
                            continue;
                        }
                    }
                }
                let d = docs.get_mut(&doc).expect("opened");
                if d.members >= shared.c.max_per_doc {
                    out.close(Some("too many people in this document"));
                    continue;
                }
                let j = DocEv::Join {
                    conn,
                    client,
                    author,
                    seq,
                    out: out.clone(),
                };
                if d.tx.try_send(j).is_err() {
                    out.close(Some("this document is too busy: try again"));
                    continue;
                }
                d.members += 1;
                conns.insert(conn, (doc, out));
            }
            Ev::Up { conn, up } => {
                let Some((doc, out)) = conns.get(&conn) else {
                    continue;
                };
                if let Some(d) = docs.get(doc)
                    && d.tx.try_send(DocEv::Up { conn, up }).is_err()
                {
                    // the document's inbox is full: this connection is the one shed
                    out.close(Some("this document is too busy: reconnect"));
                }
            }
            Ev::Leave { conn } => {
                if let Some((doc, _)) = conns.remove(&conn)
                    && let Some(d) = docs.get_mut(&doc)
                {
                    let _ = d.tx.send(DocEv::Leave { conn });
                    d.members -= 1;
                    if d.members == 0 {
                        // no one left: its head saved, its actor ended (its log stays)
                        close(&mut docs, &doc);
                    }
                }
            }
            Ev::Quarantine { doc } => {
                quarantined.insert(doc.clone());
                if let Some(d) = docs.remove(&doc) {
                    let _ = d.thread.join();
                }
                let gone: Vec<Conn> = conns
                    .iter()
                    .filter(|(_, v)| v.0 == doc)
                    .map(|(k, _)| *k)
                    .collect();
                for k in gone {
                    if let Some((_, out)) = conns.remove(&k) {
                        out.close(Some("this document is quarantined after a server error"));
                    }
                }
                shared.stats().quarantined += 1;
            }
        }
        shared.stats().open_docs = docs.len();
    }
}

fn open_actor(shared: &Arc<Shared>, doc: &str, hub: SyncSender<Ev>) -> Result<DocHandle, String> {
    let room = open_doc(shared, doc)?;
    let (tx, rx) = sync_channel::<DocEv>(DOC_INBOX);
    let (shared2, doc2, self_tx) = (shared.clone(), doc.to_string(), tx.clone());
    let thread = std::thread::Builder::new()
        .name(format!("forge-server doc {doc}"))
        .spawn(move || {
            let saved = room.seq.seq();
            let store = shared2.store().ok();
            let mut a = Actor {
                s: shared2,
                doc: doc2.clone(),
                room,
                store,
                outs: HashMap::new(),
                self_tx,
                last_op: Instant::now(),
                saved,
                built: Default::default(),
                waiting: vec![],
                building: None,
                last_build: None,
                paused: None,
            };
            if catch_unwind(AssertUnwindSafe(|| a.run(rx))).is_err() {
                eprintln!("forge-server: {doc2}: a panic in its actor: quarantined");
                for out in a.outs.values() {
                    out.close(Some("this document is quarantined after a server error"));
                }
                let _ = hub.send(Ev::Quarantine { doc: doc2 });
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(DocHandle {
        tx,
        thread,
        members: 0,
    })
}

fn open_doc(shared: &Shared, doc: &str) -> Result<Room, String> {
    let c = &shared.c;
    let path = fsx::canonical_doc(&c.root, doc).map_err(str::to_string)?;
    let len = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
    if len > MAX_DOC {
        return Err(format!("{doc}: {len} bytes (at most {MAX_DOC})"));
    }
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{doc}: {e}"))?;
    // a file changed outside the server since it last wrote it: never overwritten silently
    match shared.marks.get(doc) {
        Some(h) if h != forge_sync::text_hash(&text) => {
            return Err(format!(
                "{doc} changed on disk outside the server (an editor, a git pull): bringing such a change in is not in \
                 phase 1 (DESIGN.md §15.4)"
            ));
        }
        Some(_) => {}
        None => shared.marks.set(doc, &text).map_err(|e| e.to_string())?,
    }
    let mut store = shared.store()?;
    let mut seq = Sequencer::open(store.as_mut(), doc, &text, path.parent().map(Path::to_path_buf))
        .map_err(|e| e.to_string())?;
    seq.root = Some(c.root.clone());
    seq.checker = Box::new(Timed {
        abandoned: Arc::new(AtomicUsize::new(0)),
    });
    Ok(Room::new(seq))
}

struct Actor {
    s: Arc<Shared>,
    doc: String,
    room: Room,
    store: Option<Box<dyn Store + Send>>,
    outs: HashMap<Conn, Outbox>,
    self_tx: SyncSender<DocEv>,
    last_op: Instant,
    /// The seq last written to git (a checkpoint or a version).
    saved: u64,
    /// The server's own build hash per seq (bounded), reports waiting for one, the build in
    /// flight (its seq, when it started).
    built: std::collections::BTreeMap<u64, Option<u64>>,
    waiting: Vec<(u64, u64, String, Conn)>,
    building: Option<(u64, Instant)>,
    last_build: Option<Instant>,
    /// Why the file is not written any more (changed on disk outside the server).
    paused: Option<String>,
}

impl Actor {
    fn send(&mut self, out: Vec<(Conn, Down)>) {
        let mut slow = vec![];
        for (k, d) in out {
            if let Some(o) = self.outs.get(&k)
                && !o.down(&d)
            {
                slow.push(k);
            }
        }
        for k in slow {
            // too slow to keep up: dropped; it reconnects and catches up or takes a snapshot
            if let Some(o) = self.outs.remove(&k) {
                o.close(None);
                self.room.leave(k);
            }
        }
    }

    fn run(&mut self, rx: Receiver<DocEv>) {
        loop {
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(DocEv::Close) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    self.checkpoint();
                    return;
                }
                Ok(ev) => self.event(ev),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            }
            self.idle();
        }
    }

    fn event(&mut self, ev: DocEv) {
        match ev {
            DocEv::Join {
                conn,
                client,
                author,
                seq,
                out,
            } => {
                self.outs.insert(conn, out);
                let o = self.room.join(conn, &client, &author, seq);
                self.send(o);
            }
            DocEv::Leave { conn } => {
                self.outs.remove(&conn);
                self.room.leave(conn);
            }
            DocEv::Up { conn, up } => {
                #[cfg(feature = "test-hooks")]
                if let Up::Op {
                    op: Op::SetProperty { key, .. },
                    ..
                } = &up
                    && key == "test_hook_panic"
                {
                    panic!("test hook: a panic in a document's actor");
                }
                if matches!(up, Up::BuildHash { .. }) {
                    self.s.stats().hashes += 1;
                }
                let is_op = matches!(up, Up::Op { .. });
                let t0 = Instant::now();
                let (out, actions) = match self.store.as_mut() {
                    Some(st) => self.room.receive(st.as_mut(), conn, up),
                    None => (
                        vec![(
                            conn,
                            Down::Error {
                                text: "the store is not available".into(),
                            },
                        )],
                        vec![],
                    ),
                };
                if is_op {
                    let ms = t0.elapsed().as_secs_f64() * 1e3;
                    let refused = matches!(out.first(), Some((_, Down::Refused { .. })));
                    if !refused {
                        self.last_op = Instant::now();
                    }
                    let compactions = self.room.seq.compactions;
                    let mut s = self.s.stats();
                    if refused {
                        s.refused += 1;
                    } else {
                        s.committed += 1;
                        if s.commit_ms.len() >= STATS_KEPT {
                            s.commit_ms.remove(0);
                        }
                        s.commit_ms.push(ms);
                    }
                    s.compactions = s.compactions.max(compactions);
                }
                self.send(out);
                for a in actions {
                    self.action(a);
                }
            }
            DocEv::Built { seq, hash } => {
                if self.building.is_some_and(|b| b.0 == seq) {
                    self.building = None;
                }
                self.built.insert(seq, hash);
                while self.built.len() > 1024 {
                    self.built.pop_first();
                }
                let (now, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut self.waiting)
                    .into_iter()
                    .partition(|w| w.0 == seq);
                self.waiting = rest;
                for (seq, got, author, conn) in now {
                    self.judge(seq, hash, got, &author, conn);
                }
            }
            DocEv::Close => {}
        }
    }

    /// A reported hash against the server's own build of that seq.
    fn judge(&mut self, seq: u64, mine: Option<u64>, got: u64, author: &str, conn: Conn) {
        let Some(mine) = mine else { return };
        self.s.stats().hash_checks += 1;
        if mine == got {
            return;
        }
        self.s.stats().divergences += 1;
        // the determinism gate replays it from this line: document, seq, both hashes, who
        eprintln!(
            "forge-server: DIVERGENCE doc={} seq={seq} reported={got:016x} server={mine:016x} by={author} kernel={}",
            self.doc,
            forge_sync::KERNEL
        );
        let d = Down::Diverged {
            seq,
            expected: mine,
            got,
            by: author.to_string(),
        };
        // told to the one that built differently: it shows the server's result (DESIGN.md §3)
        self.send(vec![(conn, d)]);
    }

    fn action(&mut self, a: Action) {
        match a {
            Action::Version { name, author, conn } => {
                {
                    let mut v = self.s.versions.lock().unwrap_or_else(|e| e.into_inner());
                    if v.get(&author).is_some_and(|t| t.elapsed() < VERSION_GAP) {
                        drop(v);
                        return self.send(vec![(
                            conn,
                            Down::Error {
                                text: "one version every 2 s".into(),
                            },
                        )]);
                    }
                    v.insert(author.clone(), Instant::now());
                }
                let seq = self.room.seq.seq();
                let done = self
                    .write_file()
                    .and_then(|path| git::version(&self.s.c.root, &self.doc, &path, &name, &author, seq));
                match done {
                    Ok(commit) => {
                        self.saved = seq;
                        self.s.stats().versions += 1;
                        let o = self.room.versioned(&name, seq, &commit);
                        self.send(o);
                    }
                    Err(e) => self.send(vec![(
                        conn,
                        Down::Error {
                            text: format!("the version was not written: {e}"),
                        },
                    )]),
                }
            }
            Action::CheckHash {
                seq,
                hash,
                author,
                conn,
            } => {
                if let Some(&mine) = self.built.get(&seq) {
                    return self.judge(seq, mine, hash, &author, conn);
                }
                if self.waiting.len() >= 64 {
                    return;
                }
                if self.building.is_some_and(|b| b.0 == seq) {
                    self.waiting.push((seq, hash, author, conn));
                    return;
                }
                if self.building.is_some() || self.last_build.is_some_and(|t| t.elapsed() < HASH_BUILD_GAP) {
                    return; // rate-limited: this report is not checked
                }
                let Some(text) = self.room.seq.text_at(seq) else {
                    return;
                };
                if self.s.builds.fetch_add(1, Ordering::SeqCst) >= BUILDS {
                    self.s.builds.fetch_sub(1, Ordering::SeqCst);
                    return;
                }
                let (dir, root, tx, builds) = (
                    self.room.seq.dir.clone(),
                    self.s.c.root.clone(),
                    self.self_tx.clone(),
                    self.s.builds.clone(),
                );
                let spawned = std::thread::Builder::new()
                    .name("forge-server hash build".into())
                    .spawn(move || {
                        let hash = catch_unwind(AssertUnwindSafe(|| {
                            let o = forge_lang::Options {
                                dir,
                                root: Some(root),
                                ..forge_lang::Options::default()
                            };
                            let (run, _) = forge_lang::run(&text, o, Default::default());
                            forge_sync::run_hash(&run)
                        }))
                        .ok()
                        .flatten();
                        builds.fetch_sub(1, Ordering::SeqCst);
                        let _ = tx.try_send(DocEv::Built { seq, hash });
                    });
                if spawned.is_err() {
                    self.s.builds.fetch_sub(1, Ordering::SeqCst);
                    return;
                }
                self.building = Some((seq, Instant::now()));
                self.last_build = Some(Instant::now());
                self.waiting.push((seq, hash, author, conn));
            }
        }
    }

    /// The head written to the document's file, if the file still holds what the server last
    /// wrote (else it was changed outside: never overwritten; the document's saving pauses).
    /// The mark is set right after the write, before git.
    fn write_file(&mut self) -> Result<PathBuf, String> {
        if let Some(p) = &self.paused {
            return Err(p.clone());
        }
        let path = fsx::canonical_doc(&self.s.c.root, &self.doc).map_err(str::to_string)?;
        let now = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        if self.s.marks.get(&self.doc) != Some(forge_sync::text_hash(&now)) {
            let why = format!(
                "{} changed on disk outside the server: it is not written over (bring the change in by hand)",
                self.doc
            );
            eprintln!("forge-server: {why}");
            self.paused = Some(why.clone());
            return Err(why);
        }
        let head = self.room.seq.head().to_string();
        fsx::put(&path, head.as_bytes()).map_err(|e| e.to_string())?;
        self.s.marks.set(&self.doc, &head).map_err(|e| e.to_string())?;
        Ok(path)
    }

    fn checkpoint(&mut self) {
        let seq = self.room.seq.seq();
        if seq <= self.saved {
            return;
        }
        self.saved = seq;
        match self
            .write_file()
            .and_then(|p| git::checkpoint(&self.s.c.root, &p, seq))
        {
            Ok(()) => self.s.stats().checkpoints += 1,
            Err(e) => eprintln!("forge-server: {}: checkpoint not written: {e}", self.doc),
        }
    }

    fn idle(&mut self) {
        // a hash-check build past its budget: abandoned, its reports dropped
        if let Some((seq, t)) = self.building
            && t.elapsed() > BUILD_BUDGET
        {
            self.building = None;
            self.waiting.retain(|w| w.0 != seq);
        }
        if self.room.seq.seq() > self.saved && self.last_op.elapsed() >= self.s.c.checkpoint_idle {
            self.checkpoint();
        }
    }
}
