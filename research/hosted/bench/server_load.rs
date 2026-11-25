//! Phase-2 capacity measurement (HOSTED-DESIGN, research/hosted/DESIGN.md §17.6): the
//! forge-server binary as its own process (file store, loopback), loaded by forge-remote
//! sessions from this process. Measures the server's resident memory per open document and per
//! connection, and its CPU seconds per committed op, from /proc. Ignored by default:
//!   cargo build --release -p forge-server
//!   cargo test --release -p forge-server --test load -- --ignored --nocapture
//! Writes a JSON report to $FORGE_LOAD_REPORT if set.

use forge_lang::error::{Literal, Op};
use forge_remote::{Remote, Session};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
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
sketch tab(on: xy) {
  o = rect(center: (40, 0), size: (8, 8))
}
param lid.t = 2
lid = extrude(tab, t, new: true)
post = extrude(tab, 9, new: true)
";
const DOCS: usize = 240;
const USERS: usize = 8;

fn token(u: usize) -> String {
    format!("load-token-0123456789-{u}")
}

fn proc_stat(pid: u32) -> (u64, f64) {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).unwrap();
    let rss_kb = status
        .lines()
        .find(|l| l.starts_with("VmRSS:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
    let f: Vec<&str> = stat.rsplit(')').next().unwrap().split_whitespace().collect();
    // utime, stime: fields 14 and 15 (1-based), here 11 and 12 after the ')' split
    let ticks: f64 = f[11].parse::<f64>().unwrap() + f[12].parse::<f64>().unwrap();
    (rss_kb, ticks / 100.0)
}

fn settle_rss(pid: u32) -> u64 {
    std::thread::sleep(Duration::from_millis(1500));
    proc_stat(pid).0
}

fn server(root: &Path, port: u16) -> Child {
    let bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/release/forge-server");
    let mut c = Command::new(bin);
    c.arg(root).args(["--port", &port.to_string(), "--store", "file", "--checkpoint", "3600"]);
    for u in 0..USERS {
        c.args(["--user", &format!("u{u}:{}", token(u))]);
    }
    let child = c.spawn().expect("target/release/forge-server: build it first");
    std::thread::sleep(Duration::from_millis(500));
    child
}

fn open(port: u16, doc: usize, u: usize) -> Session {
    let remote = Remote {
        name: "load".into(),
        url: format!("ws://127.0.0.1:{port}/sync"),
        token: token(u),
    };
    let s = Session::start(&remote, &format!("d{doc}.fg"), None).unwrap();
    let t0 = Instant::now();
    while !s.ready() {
        assert!(t0.elapsed() < Duration::from_secs(20), "no snapshot for d{doc}");
        std::thread::sleep(Duration::from_millis(2));
    }
    s
}

fn pct(v: &mut [f64], p: f64) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    if v.is_empty() {
        return f64::NAN;
    }
    v[((v.len() - 1) as f64 * p).round() as usize]
}

#[test]
#[ignore]
fn server_memory_and_cpu_per_op() {
    let root = std::env::temp_dir().join(format!("forge-load-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    for d in 0..DOCS {
        std::fs::write(root.join(format!("d{d}.fg")), DOC).unwrap();
    }
    let port = 47_000 + (std::process::id() % 1000) as u16;
    let mut child = server(&root, port);
    let pid = child.id();
    let mut report = vec![];
    let base = settle_rss(pid);
    report.push(format!("\"rss_idle_kb\": {base}"));

    // documents: one connection each, 0 -> 60 -> 240 documents open
    let mut sessions = vec![];
    let mut points = vec![(0usize, base)];
    for &n in &[60usize, 240] {
        while sessions.len() < n {
            let d = sessions.len();
            sessions.push(open(port, d, d % USERS));
        }
        points.push((n, settle_rss(pid)));
    }
    let per_doc = (points[2].1 as f64 - points[1].1 as f64) / 180.0;
    report.push(format!("\"rss_docs_kb\": {:?}", points));
    report.push(format!("\"rss_per_doc_and_conn_kb\": {per_doc:.1}"));

    // connections: 16 more on documents already open (no new actor)
    let before = settle_rss(pid);
    let mut extra = vec![];
    for k in 0..15 {
        extra.push(open(port, k, (k + 1) % USERS));
    }
    let after = settle_rss(pid);
    let per_conn = (after as f64 - before as f64) / 15.0;
    report.push(format!("\"rss_per_extra_conn_kb\": {per_conn:.1}"));
    drop(extra);

    // load: every session sends SetParam ops at `rate` a second for `secs`
    for &(rate, secs) in &[(1.0f64, 20u64), (4.0, 20)] {
        let (_, cpu0) = proc_stat(pid);
        let t0 = Instant::now();
        let mut sent = 0u64;
        let gap = Duration::from_secs_f64(1.0 / rate);
        let mut next = Instant::now();
        let mut k = 0u64;
        // the document's first handle (a length), as the e2e test picks its edits
        let (key, lit) = forge_lang::edit::handles(DOC).into_iter().next().expect("a handle");
        while t0.elapsed() < Duration::from_secs(secs) {
            for s in &sessions {
                k += 1;
                let _ = s.edit(Op::SetParam {
                    param: key.clone(),
                    value: Literal {
                        value: lit.value * (0.8 + (k % 200) as f64 / 500.0),
                        dim: lit.dim,
                    },
                });
                sent += 1;
            }
            next += gap;
            if let Some(d) = next.checked_duration_since(Instant::now()) {
                std::thread::sleep(d);
            }
        }
        // drain
        let t1 = Instant::now();
        while sessions.iter().any(|s| s.pending() > 0) && t1.elapsed() < Duration::from_secs(60) {
            std::thread::sleep(Duration::from_millis(20));
        }
        let wall = t0.elapsed().as_secs_f64();
        let (_, cpu1) = proc_stat(pid);
        let mut acks: Vec<f64> = sessions.iter().flat_map(|s| s.ack_ms()).collect();
        let pending: usize = sessions.iter().map(|s| s.pending()).sum();
        report.push(format!(
            "\"load_{rate}_per_s\": {{\"docs\": {}, \"sent\": {sent}, \"wall_s\": {wall:.1}, \"ops_per_s\": {:.0}, \"server_cpu_s\": {:.2}, \"cpu_ms_per_op\": {:.3}, \"ack_p50_ms\": {:.2}, \"ack_p99_ms\": {:.2}, \"still_pending\": {pending}}}",
            sessions.len(),
            sent as f64 / wall,
            cpu1 - cpu0,
            (cpu1 - cpu0) * 1e3 / sent as f64,
            pct(&mut acks, 0.5),
            pct(&mut acks, 0.99),
        ));
    }
    let (rss_end, _) = proc_stat(pid);
    report.push(format!("\"rss_end_kb\": {rss_end}"));
    let json = format!("{{\n  {}\n}}\n", report.join(",\n  "));
    println!("{json}");
    if let Ok(p) = std::env::var("FORGE_LOAD_REPORT") {
        std::fs::write(p, &json).unwrap();
    }
    drop(sessions);
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&root);
}
