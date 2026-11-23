//! Fuzzes the readers. The supervisor runs workers of this same program, each on a stride of the
//! cases; a worker says before each case which it starts and after it how it went, so a case that
//! hangs, aborts or runs out of memory is known, kept and stepped past.
//!
//!   read-fuzz run TARGET [--seconds N | --cases N] [--seed S] [--jobs J] [--timeout-ms T]
//!                        [--mem-mb M] [--slow-ms W] [--big-mb B] [--max-seed-kb K]
//!                        [--seeds DIR].. [--out DIR]
//!   read-fuzz replay TARGET FILE..   each file once: outcome, time, peak memory
//!   read-fuzz targets
//!
//! TARGET is one of `read-fuzz targets`, or `all` (the time split between them). Seeds are the
//! files under each `--seeds` DIR with the target's extensions (default: the repository), plus
//! what the target makes itself. Case k < number of seeds is the seed itself; after that a mutant
//! of a seed, all drawn from (S, k), so `--seed S` and the case number reproduce it. A finding is
//! a panic, a hang past T ms, a worker killed (abort, stack overflow, out of memory past M MB) or a
//! case past B MB of peak memory (and 16 times its own size); each is written to `--out` (default target/read-fuzz) and
//! the run exits 1. Cases past W ms are listed but not findings.

use forge_math::rng::Rng;
use forge_read_fuzz::{
    Outcome, TARGETS, Target, mutate::mutate, peak_kb, quiet_panics, reset_peak, run, seeds,
};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

struct Cfg {
    seconds: Option<u64>,
    cases: u64,
    seed: u64,
    jobs: u64,
    timeout_ms: u64,
    mem_mb: u64,
    slow_ms: u64,
    big_mb: u64,
    max_seed_kb: usize,
    seeds: Vec<PathBuf>,
    out: PathBuf,
}

fn repo_root() -> PathBuf {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    here.ancestors().nth(2).map(PathBuf::from).unwrap_or(here)
}

fn parse(args: &[String]) -> Cfg {
    let mut c = Cfg {
        seconds: None,
        cases: 2000,
        seed: 1,
        jobs: 1,
        timeout_ms: 20_000,
        mem_mb: 3072,
        slow_ms: 2000,
        big_mb: 1024,
        max_seed_kb: 512,
        seeds: vec![],
        out: repo_root().join("target/read-fuzz"),
    };
    let mut i = 0;
    while i < args.len() {
        let v = args.get(i + 1).cloned().unwrap_or_default();
        let num = |s: &str| {
            s.parse::<u64>()
                .unwrap_or_else(|_| die(&format!("{}: not a number: {s}", args[i])))
        };
        match args[i].as_str() {
            "--seconds" => c.seconds = Some(num(&v)),
            "--cases" => c.cases = num(&v),
            "--seed" => c.seed = num(&v),
            "--jobs" => c.jobs = num(&v).max(1),
            "--timeout-ms" => c.timeout_ms = num(&v),
            "--mem-mb" => c.mem_mb = num(&v),
            "--slow-ms" => c.slow_ms = num(&v),
            "--big-mb" => c.big_mb = num(&v),
            "--max-seed-kb" => c.max_seed_kb = num(&v) as usize,
            "--seeds" => c.seeds.push(PathBuf::from(v)),
            "--out" => c.out = PathBuf::from(v),
            a => die(&format!("unknown option {a}")),
        }
        i += 2;
    }
    if c.seeds.is_empty() {
        c.seeds.push(repo_root());
    }
    c
}

fn die(s: &str) -> ! {
    eprintln!("read-fuzz: {s}");
    std::process::exit(2)
}

fn target(s: &str) -> Target {
    Target::by_name(s).unwrap_or_else(|| die(&format!("no target {s}; one of: {}", names())))
}

fn names() -> String {
    TARGETS.iter().map(|t| t.0).collect::<Vec<_>>().join(" ")
}

/// Case `k` of target `t` under seed `s`: the seed itself, or a mutant.
fn case(t: Target, pool: &[(String, Vec<u8>)], s: u64, k: u64) -> (String, Vec<u8>) {
    if pool.is_empty() {
        return ("empty".into(), vec![]);
    }
    if (k as usize) < pool.len() {
        return (pool[k as usize].0.clone(), pool[k as usize].1.clone());
    }
    let mut r =
        Rng::new(s.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ k.wrapping_mul(0xbf58_476d_1ce4_e5b9) ^ 0x5eed);
    let a = r.below(pool.len());
    let o = r.below(pool.len());
    (
        pool[a].0.clone(),
        mutate(t.format(), &pool[a].1, &pool[o].1, &mut r),
    )
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("worker") => worker(&args[1..]),
        Some("run") if args.len() >= 2 => {
            let cfg = parse(&args[2..]);
            let ts: Vec<Target> = if args[1] == "all" {
                // building a part from a mutant is the kernel's fuzzing, slow per case: its own runs
                TARGETS
                    .iter()
                    .map(|t| t.1)
                    .filter(|&t| t != Target::FgBuild)
                    .collect()
            } else {
                args[1].split(',').map(target).collect()
            };
            let per = cfg.seconds.map(|s| (s / ts.len() as u64).max(1));
            let mut found = 0;
            for t in ts {
                found += supervise(t, &cfg, per);
            }
            std::process::exit(if found > 0 { 1 } else { 0 });
        }
        Some("replay") if args.len() >= 3 => replay(target(&args[1]), &args[2..]),
        Some("targets") => println!("{}", names()),
        _ => die("usage: read-fuzz run TARGET [options] | replay TARGET FILE.. | targets"),
    }
}

fn replay(t: Target, files: &[String]) {
    quiet_panics();
    let mut bad = 0;
    for f in files {
        let data = std::fs::read(f).unwrap_or_else(|e| die(&format!("{f}: {e}")));
        reset_peak();
        let t0 = Instant::now();
        let o = run(t, &data);
        let ms = t0.elapsed().as_millis();
        if o.finding() {
            bad += 1;
        }
        println!("{f}\t{ms} ms\t{} MB\t{o:?}", peak_kb() / 1024);
    }
    std::process::exit(if bad > 0 { 1 } else { 0 });
}

/// `worker TARGET SEED FROM STRIDE TO MAX_SEED_KB DIR..`: cases FROM, FROM+STRIDE, .. below TO.
fn worker(a: &[String]) {
    quiet_panics();
    let t = target(&a[0]);
    let n = |i: usize| a[i].parse::<u64>().unwrap_or(0);
    let (s, from, stride, to, kb) = (n(1), n(2), n(3).max(1), n(4), n(5) as usize);
    let dirs: Vec<PathBuf> = a[6..].iter().map(PathBuf::from).collect();
    let pool = seeds(t, &dirs, kb << 10);
    let out = std::io::stdout();
    let mut k = from;
    while k < to {
        {
            let mut o = out.lock();
            let _ = writeln!(o, "S {k}");
            let _ = o.flush();
        }
        let (_, data) = case(t, &pool, s, k);
        reset_peak();
        let t0 = Instant::now();
        let r = run(t, &data);
        let ms = t0.elapsed().as_millis();
        let what = match &r {
            Outcome::Ok => "ok".to_string(),
            Outcome::Refused(_) => "refused".to_string(),
            Outcome::Panic(p) => format!("panic {}", p.replace('\n', " ")),
        };
        let mut o = out.lock();
        let _ = writeln!(o, "D {k} {ms} {} {} {what}", peak_kb(), data.len());
        let _ = o.flush();
        k += stride;
    }
}

struct Finding {
    kind: String,
    case: u64,
    what: String,
}

/// Runs one target's workers; the number of findings.
fn supervise(t: Target, c: &Cfg, seconds: Option<u64>) -> usize {
    let exe = std::env::current_exe().unwrap_or_else(|e| die(&e.to_string()));
    let pool = seeds(t, &c.seeds, c.max_seed_kb << 10);
    let end = seconds.map(|s| Instant::now() + Duration::from_secs(s));
    let to = if end.is_some() { u64::MAX } else { c.cases };
    let (tx, rx) = mpsc::channel::<(u64, Option<String>)>();
    let mut next: Vec<u64> = (0..c.jobs).collect();
    let mut current: Vec<Option<(u64, Instant)>> = vec![None; c.jobs as usize];
    let mut children: Vec<Option<std::process::Child>> = (0..c.jobs).map(|_| None).collect();
    let mut round: Vec<u64> = vec![0; c.jobs as usize];
    let mut findings: Vec<Finding> = vec![];
    let (mut done, mut slow, mut refused) = (0u64, vec![], 0u64);
    let mut worst = (0u64, 0u64, 0u64, 0u64);
    let start = Instant::now();
    let spawn = |j: u64, from: u64, g: u64, tx: mpsc::Sender<(u64, Option<String>)>| -> std::process::Child {
        let mut args: Vec<String> = vec![
            "worker".into(),
            t.name().into(),
            c.seed.to_string(),
            from.to_string(),
            c.jobs.to_string(),
            to.to_string(),
            c.max_seed_kb.to_string(),
        ];
        args.extend(c.seeds.iter().map(|p| p.display().to_string()));
        // the memory cap: a shell's ulimit, so nothing here needs to call the OS itself
        let mut cmd = if cfg!(unix) && c.mem_mb > 0 {
            let mut x = Command::new("sh");
            x.arg("-c")
                .arg(format!("ulimit -v {} && exec \"$0\" \"$@\"", c.mem_mb * 1024))
                .arg(&exe)
                .args(&args);
            x
        } else {
            let mut x = Command::new(&exe);
            x.args(&args);
            x
        };
        let mut ch = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| die(&e.to_string()));
        let so = ch.stdout.take();
        std::thread::spawn(move || {
            if let Some(so) = so {
                for l in BufReader::new(so).lines().map_while(Result::ok) {
                    if tx.send((j | (g << 32), Some(l))).is_err() {
                        return;
                    }
                }
            }
            let _ = tx.send((j | (g << 32), None));
        });
        ch
    };
    for j in 0..c.jobs {
        children[j as usize] = Some(spawn(j, next[j as usize], 0, tx.clone()));
    }
    let mut live = c.jobs;
    let stderr_of = |ch: &mut std::process::Child| -> String {
        let mut s = String::new();
        if let Some(mut e) = ch.stderr.take() {
            use std::io::Read;
            let _ = e.read_to_string(&mut s);
        }
        let lines: Vec<&str> = s.lines().filter(|l| !l.trim().is_empty()).collect();
        lines[lines.len().saturating_sub(3)..]
            .join(" | ")
            .chars()
            .take(300)
            .collect()
    };
    while live > 0 {
        let timed_out = end.is_some_and(|e| Instant::now() >= e);
        if timed_out {
            for ch in children.iter_mut().flatten() {
                let _ = ch.kill();
                let _ = ch.wait();
            }
            break;
        }
        let msg = rx.recv_timeout(Duration::from_millis(200));
        // a case past its time: the worker is stopped and the case kept
        for j in 0..c.jobs as usize {
            if let Some((k, t0)) = current[j]
                && t0.elapsed() > Duration::from_millis(c.timeout_ms)
            {
                if let Some(mut ch) = children[j].take() {
                    let _ = ch.kill();
                    let _ = ch.wait();
                }
                findings.push(Finding {
                    kind: "hang".into(),
                    case: k,
                    what: format!("past {} ms", c.timeout_ms),
                });
                current[j] = None;
                next[j] = k + c.jobs;
                round[j] += 1;
                if next[j] < to {
                    children[j] = Some(spawn(j as u64, next[j], round[j], tx.clone()));
                } else {
                    live -= 1;
                }
            }
        }
        let Ok((tag, line)) = msg else { continue };
        let (j, g) = ((tag & 0xffff_ffff) as usize, tag >> 32);
        if g != round[j] {
            continue;
        }
        match line {
            Some(l) => {
                let mut w = l.splitn(6, ' ');
                let (kind, k) = (
                    w.next().unwrap_or(""),
                    w.next().and_then(|x| x.parse::<u64>().ok()),
                );
                let Some(k) = k else { continue };
                if kind == "S" {
                    current[j] = Some((k, Instant::now()));
                } else if kind == "D" {
                    current[j] = None;
                    next[j] = k + c.jobs;
                    done += 1;
                    let ms: u64 = w.next().and_then(|x| x.parse().ok()).unwrap_or(0);
                    let kb: u64 = w.next().and_then(|x| x.parse().ok()).unwrap_or(0);
                    let len: u64 = w.next().and_then(|x| x.parse().ok()).unwrap_or(0);
                    let rest = w.next().unwrap_or("");
                    if ms > worst.0 {
                        worst.0 = ms;
                        worst.1 = k;
                    }
                    if kb > worst.2 {
                        worst.2 = kb;
                        worst.3 = k;
                    }
                    if rest == "refused" {
                        refused += 1;
                    }
                    if let Some(p) = rest.strip_prefix("panic ") {
                        findings.push(Finding {
                            kind: "panic".into(),
                            case: k,
                            what: p.into(),
                        });
                    } else if kb > c.big_mb * 1024 && kb / 1024 > 16 * (len >> 20).max(1) {
                        findings.push(Finding {
                            kind: "memory".into(),
                            case: k,
                            what: format!("{} MB peak", kb / 1024),
                        });
                    } else if ms > c.slow_ms {
                        slow.push((k, ms));
                    }
                }
            }
            None => {
                // the worker ended: done, or killed by the case it was on
                let mut why = String::new();
                if let Some(mut ch) = children[j].take() {
                    let st = ch.wait().ok();
                    why = format!(
                        "{} {}",
                        st.map(|s| s.to_string()).unwrap_or_default(),
                        stderr_of(&mut ch)
                    );
                }
                match current[j].take() {
                    Some((k, _)) => {
                        findings.push(Finding {
                            kind: "crash".into(),
                            case: k,
                            what: why,
                        });
                        next[j] = k + c.jobs;
                        round[j] += 1;
                        if next[j] < to {
                            children[j] = Some(spawn(j as u64, next[j], round[j], tx.clone()));
                        } else {
                            live -= 1;
                        }
                    }
                    None => live -= 1,
                }
            }
        }
    }
    let _ = std::fs::create_dir_all(&c.out);
    // one file per kind and place: a panic by where it was, others by kind
    let mut by: std::collections::BTreeMap<String, (usize, &Finding)> = Default::default();
    for f in &findings {
        let sig = match f.kind.as_str() {
            "panic" => format!("panic {}", f.what.split(": ").next().unwrap_or("")),
            k => k.to_string(),
        };
        by.entry(sig).or_insert((0, f)).0 += 1;
    }
    for (sig, (n, f)) in &by {
        let (from, data) = case(t, &pool, c.seed, f.case);
        let base = c
            .out
            .join(format!("{}-{}-s{}-c{}", t.name(), f.kind, c.seed, f.case));
        let _ = std::fs::write(base.with_extension("bin"), &data);
        let _ = std::fs::write(
            base.with_extension("txt"),
            format!(
                "target {}\nkind {}\nseed {}\ncase {}\nfrom {from}\n{}\n",
                t.name(),
                f.kind,
                c.seed,
                f.case,
                f.what
            ),
        );
        println!(
            "FINDING {} {sig} x{n}: case {} (from {from}): {}",
            t.name(),
            f.case,
            f.what
        );
    }
    slow.sort_by_key(|x| std::cmp::Reverse(x.1));
    for (k, ms) in slow.iter().take(5) {
        let (from, _) = case(t, &pool, c.seed, *k);
        println!("slow {} case {k} {ms} ms (from {from})", t.name());
    }
    println!(
        "{}: {done} cases in {:.0} s from {} seeds, {refused} refused, {} slow, {} findings; slowest {} ms (case {}), most memory {} MB (case {})",
        t.name(),
        start.elapsed().as_secs_f64(),
        pool.len(),
        slow.len(),
        findings.len(),
        worst.0,
        worst.1,
        worst.2 / 1024,
        worst.3
    );
    findings.len()
}
