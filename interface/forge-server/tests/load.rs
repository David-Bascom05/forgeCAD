//! A load soak on the real server binary: many documents, several app sessions each, edits at a
//! steady rate for minutes, sessions closed and resumed all along (connection churn). The
//! server's memory (VmRSS) and CPU are sampled from /proc. It fails on a leak (memory still
//! growing after warm-up by more than its in-memory op log accounts for), on an edit lost, or on
//! acks far over their budget. Ignored by default (minutes, release):
//!   cargo test --release -p forge-server --test load -- --ignored --nocapture
//! Sizes: FORGE_LOAD_DOCS (100), FORGE_LOAD_PER_DOC (3), FORGE_LOAD_SECS (120),
//! FORGE_LOAD_RATE (edits per second per session, 2). FORGE_LOAD_REPORT: a JSON report there.
#![cfg(target_os = "linux")] // /proc

use forge_lang::error::{Literal, Op, ParamKey};
use forge_remote::{Remote, Session};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

const USERS: usize = 8;

fn env(k: &str, d: f64) -> f64 {
    std::env::var(k).ok().and_then(|s| s.parse().ok()).unwrap_or(d)
}

fn token(u: usize) -> String {
    format!("load-token-0123456789-{u}")
}

/// (VmRSS in KiB, CPU seconds used).
fn proc_stat(pid: u32) -> (u64, f64) {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).unwrap();
    let rss = status
        .lines()
        .find(|l| l.starts_with("VmRSS:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
    let f: Vec<&str> = stat.rsplit(')').next().unwrap().split_whitespace().collect();
    let ticks: f64 = f[11].parse::<f64>().unwrap() + f[12].parse::<f64>().unwrap();
    (rss, ticks / 100.0)
}

fn pct(v: &mut [f64], p: f64) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[((v.len() - 1) as f64 * p).round() as usize]
}

struct App {
    user: usize,
    doc: String,
    param: usize,
    s: Session,
    last: f64,
    due: Instant,
}

#[test]
#[ignore]
fn load_soak_holds_memory_and_latency() {
    let docs = env("FORGE_LOAD_DOCS", 100.0) as usize;
    let per_doc = env("FORGE_LOAD_PER_DOC", 3.0) as usize;
    let secs = env("FORGE_LOAD_SECS", 120.0);
    let rate = env("FORGE_LOAD_RATE", 2.0);
    // sessions closed and resumed per second (connection churn)
    let churn = env("FORGE_LOAD_CHURN", 4.0);
    let root = std::env::temp_dir().join(format!("forge-load-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let mut text = String::new();
    for k in 0..per_doc {
        text.push_str(&format!("shared p{k} = 1\n"));
    }
    // FORGE_LOAD_GEOM=0: the params change no geometry (a measurement of what the kernel keeps
    // per new shape)
    let size = if env("FORGE_LOAD_GEOM", 1.0) > 0.0 {
        "p0"
    } else {
        "40"
    };
    text.push_str(&format!(
        "sketch plan(on: xy) {{\n  o = rect(center: (0, 0), size: ({size}, 30))\n}}\nbase = extrude(plan, 10)\n"
    ));
    for d in 0..docs {
        std::fs::write(root.join(format!("d{d}.fg")), &text).unwrap();
    }
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    // FORGE_LOAD_WRAP: the server run under a tool (e.g. "valgrind --tool=massif
    // --massif-out-file=OUT"), to find where its memory goes
    let wrap = std::env::var("FORGE_LOAD_WRAP").unwrap_or_default();
    let mut words = wrap.split_whitespace();
    let mut c = match words.next() {
        Some(tool) => {
            let mut c = Command::new(tool);
            c.args(words).arg(env!("CARGO_BIN_EXE_forge-server"));
            c
        }
        None => Command::new(env!("CARGO_BIN_EXE_forge-server")),
    };
    c.arg(&root)
        .args([
            "--port",
            &port.to_string(),
            "--store",
            "file",
            "--checkpoint",
            "30",
        ])
        .args(["--max-conns", &(docs * per_doc + 64).to_string()])
        .stderr(std::process::Stdio::null());
    for u in 0..USERS {
        c.args(["--user", &format!("u{u}:{}", token(u))]);
    }
    let child: Child = c.stdout(std::process::Stdio::null()).spawn().unwrap();
    // killed however the test ends (a failed assert too)
    struct Kill(Child);
    impl Drop for Kill {
        fn drop(&mut self) {
            // under a tool: a TERM, so it writes its report; else a KILL
            if std::env::var("FORGE_LOAD_WRAP").is_ok_and(|w| !w.is_empty()) {
                let _ = Command::new("kill")
                    .args(["-TERM", &self.0.id().to_string()])
                    .status();
            } else {
                let _ = self.0.kill();
            }
            let _ = self.0.wait();
        }
    }
    let pid = child.id();
    let _child = Kill(child);
    let t0 = Instant::now();
    while std::net::TcpStream::connect(("127.0.0.1", port)).is_err() {
        assert!(t0.elapsed() < Duration::from_secs(60));
        std::thread::sleep(Duration::from_millis(10));
    }
    let remote = |u: usize| Remote {
        name: "load".into(),
        url: format!("ws://127.0.0.1:{port}/sync"),
        token: token(u),
    };
    let mut apps: Vec<App> = vec![];
    for d in 0..docs {
        for k in 0..per_doc {
            let user = (d * per_doc + k) % USERS;
            let doc = format!("d{d}.fg");
            apps.push(App {
                user,
                s: Session::start(&remote(user), &doc, None).unwrap(),
                doc,
                param: k,
                last: 1.0,
                due: Instant::now(),
            });
        }
    }
    let t0 = Instant::now();
    while !apps.iter().all(|a| a.s.ready()) {
        assert!(t0.elapsed() < Duration::from_secs(60), "sessions did not join");
        std::thread::sleep(Duration::from_millis(20));
    }
    let (rss_joined, cpu0) = proc_stat(pid);
    let gap = Duration::from_secs_f64(1.0 / rate);
    let (t0, mut v, mut edits, mut resumes) = (Instant::now(), 2.0, 0usize, 0usize);
    let mut samples: Vec<(f64, u64, usize)> = vec![]; // (s, rss KiB, edits so far)
    let mut next_sample = 0.0;
    let mut next_resume = 0.0;
    while t0.elapsed().as_secs_f64() < secs {
        let now = Instant::now();
        for a in apps.iter_mut() {
            if now >= a.due && a.s.pending() < 4 {
                v += 1.0;
                if a.s.edit(set(a.param, v)).is_ok() {
                    a.last = v;
                    edits += 1;
                }
                a.due = now + gap;
            }
            for e in a.s.poll() {
                match e {
                    forge_sync::Event::Refused { refusal, .. } => panic!("{}: refused: {refusal:?}", a.doc),
                    forge_sync::Event::Stopped(why) => panic!("{}: stopped: {why}", a.doc),
                    _ => {}
                }
            }
        }
        let el = t0.elapsed().as_secs_f64();
        if churn > 0.0 && el >= next_resume {
            // connection churn: one session closes and comes back (its saved client)
            next_resume += 1.0 / churn;
            let k = resumes % apps.len();
            let saved = apps[k].s.saved();
            apps[k].s = Session::resume(&remote(apps[k].user), &apps[k].doc, Some(&saved), None).unwrap();
            resumes += 1;
        }
        if el >= next_sample {
            next_sample += 5.0;
            samples.push((el, proc_stat(pid).0, edits));
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    let t_end = Instant::now();
    while !apps.iter().all(|a| a.s.ready() && a.s.pending() == 0) {
        assert!(
            t_end.elapsed() < Duration::from_secs(60),
            "edits never all landed"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let (rss_end, cpu1) = proc_stat(pid);
    let mut acks: Vec<f64> = apps.iter().flat_map(|a| a.s.ack_ms()).collect();
    for a in &apps {
        assert!(
            a.s.text().contains(&format!("shared p{} = {}", a.param, a.last)),
            "{}: p{}'s last edit lost",
            a.doc,
            a.param
        );
    }
    // memory after warm-up (the first third) against the end
    let warm = samples[samples.len() / 3];
    let last = *samples.last().unwrap();
    let ops = (last.2 - warm.2).max(1) as f64;
    let per_op = (last.1 as f64 - warm.1 as f64) * 1024.0 / ops;
    let (p50, p99) = (pct(&mut acks, 0.5), pct(&mut acks, 0.99));
    let cpu_ms = (cpu1 - cpu0) * 1e3 / edits.max(1) as f64;
    let report = format!(
        "{{\"docs\":{docs},\"sessions\":{},\"secs\":{secs},\"edits\":{edits},\"resumes\":{resumes},\
         \"rss_joined_mib\":{:.1},\"rss_end_mib\":{:.1},\"rss_growth_bytes_per_op_after_warmup\":{per_op:.0},\
         \"server_cpu_ms_per_op\":{cpu_ms:.3},\"ack_p50_ms\":{p50:.2},\"ack_p99_ms\":{p99:.2},\
         \"samples\":[{}]}}",
        apps.len(),
        rss_joined as f64 / 1024.0,
        rss_end as f64 / 1024.0,
        samples
            .iter()
            .map(|(t, r, e)| format!("[{t:.0},{r},{e}]"))
            .collect::<Vec<_>>()
            .join(",")
    );
    eprintln!("load soak: {report}");
    if let Ok(p) = std::env::var("FORGE_LOAD_REPORT") {
        std::fs::write(p, &report).unwrap();
    }
    // what is meant to grow: the in-memory op log (about 0.7 KB an entry here, bounded per
    // document by compaction at Limits::max_log_bytes). The forge_keep leak this found was 38 KB
    assert!(
        per_op < 2048.0,
        "memory grows {per_op:.0} B per op after warm-up: a leak?\n{report}"
    );
    assert!(p99 < 1000.0, "ack p99 {p99:.1} ms\n{report}");
    drop(apps);
    let _ = std::fs::remove_dir_all(&root);
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
