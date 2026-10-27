//! The reliability scoreboard: every adversarial family, every degenerate configuration, the suite
//! and fuzz histories, through both the sequential and the n-ary regen.
//! cargo run --release -p forge-model --example reliability -- [seeds] [--save FILE] [--baseline FILE] [--show]
//!
//! Each part on each path is ok (valid solid), reject (a clear ZeroThickness error, correct for an
//! exact zero-thickness touch), or a failure (error, invalid solid, panic). Both paths ok but with
//! meshed volumes more than 0.2% apart counts as a disagreement. With --baseline, exits 1 if any
//! part that was ok or reject before now fails or disagrees.
use forge_mesh::{Quality, tessellate};
use forge_model::adversarial::{canonical, degenerate_parts, families};
use forge_model::{Part, RegenError, suite};
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Verdict {
    Ok,
    Reject,
    Error,
    Invalid,
    Panic,
}

impl Verdict {
    fn word(self) -> &'static str {
        match self {
            Verdict::Ok => "ok",
            Verdict::Reject => "reject",
            Verdict::Error => "error",
            Verdict::Invalid => "invalid",
            Verdict::Panic => "panic",
        }
    }
    fn good(self) -> bool {
        matches!(self, Verdict::Ok | Verdict::Reject)
    }
}

struct Run {
    verdict: Verdict,
    detail: String,
    volume: Option<f64>,
    ms: f64,
}

fn run(f: impl FnOnce() -> Result<forge_topo::Body, RegenError>) -> Run {
    let t = Instant::now();
    let r = catch_unwind(AssertUnwindSafe(f));
    let ms = t.elapsed().as_secs_f64() * 1e3;
    let (verdict, detail, volume) = match r {
        Err(p) => {
            let m = p
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| p.downcast_ref::<String>().cloned())
                .unwrap_or_default();
            (Verdict::Panic, m.chars().take(80).collect(), None)
        }
        Ok(Err(e)) => {
            let s = format!("{e:?}");
            let v = if s.contains("ZeroThickness") {
                Verdict::Reject
            } else {
                Verdict::Error
            };
            (v, s.chars().take(100).collect(), None)
        }
        Ok(Ok(b)) => {
            let problems = forge_topo::check(&b);
            let q = Quality {
                chord: 0.02,
                max_angle: 0.5,
            };
            let vol = catch_unwind(AssertUnwindSafe(|| tessellate(&b, &q).ok().map(|m| m.volume())))
                .ok()
                .flatten();
            match problems.first() {
                None => (Verdict::Ok, String::new(), vol),
                Some(p) => (
                    Verdict::Invalid,
                    format!("{p:?}").chars().take(100).collect(),
                    vol,
                ),
            }
        }
    };
    Run {
        verdict,
        detail,
        volume,
        ms,
    }
}

struct Row {
    family: String,
    name: String,
    seq: Run,
    nary: Run,
    disagree: bool,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let seeds: u64 = args.iter().find_map(|a| a.parse().ok()).unwrap_or(24);
    let flag = |f: &str| {
        args.iter()
            .position(|a| a == f)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let show = args.iter().any(|a| a == "--show");
    let threads = std::env::var("THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4usize);
    std::panic::set_hook(Box::new(|_| {}));
    let mut parts: Vec<(String, String, Part)> = Vec::new();
    for (fam, g) in families() {
        if fam.starts_with("degen_") {
            continue;
        }
        for s in 0..seeds {
            parts.push((fam.to_string(), format!("{fam}/{s}"), canonical(&g(s))));
        }
    }
    for (name, p) in degenerate_parts() {
        let fam = format!("degen_{}", name.split('/').next().unwrap_or(""));
        parts.push((fam, name, canonical(&p)));
    }
    for (name, p) in suite::parts() {
        parts.push(("suite".into(), name.to_string(), p));
    }
    for s in 1..=seeds {
        parts.push(("fuzz".into(), format!("fuzz/{s}"), suite::fuzz(s, 40)));
    }
    let t0 = Instant::now();
    let next = AtomicUsize::new(0);
    let rows: Mutex<Vec<(usize, Row)>> = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some((family, name, part)) = parts.get(i) else {
                        break;
                    };
                    let seq = run(|| part.regenerate());
                    let nary = run(|| part.regenerate_nary(1));
                    let disagree = match (seq.verdict, nary.verdict, seq.volume, nary.volume) {
                        (Verdict::Ok, Verdict::Ok, Some(a), Some(b)) => {
                            (a - b).abs() > 2e-3 * a.abs().max(b.abs()).max(1.0)
                        }
                        _ => false,
                    };
                    let row = Row {
                        family: family.clone(),
                        name: name.clone(),
                        seq,
                        nary,
                        disagree,
                    };
                    rows.lock().expect("rows").push((i, row));
                }
            });
        }
    });
    let mut rows = rows.into_inner().expect("rows");
    rows.sort_by_key(|r| r.0);
    let rows: Vec<Row> = rows.into_iter().map(|r| r.1).collect();

    let mut fams: BTreeMap<&str, [usize; 8]> = BTreeMap::new();
    let mut order: Vec<&str> = Vec::new();
    for r in &rows {
        if !fams.contains_key(r.family.as_str()) {
            order.push(&r.family);
        }
        let e = fams.entry(&r.family).or_default();
        e[0] += 1;
        e[1] += (r.seq.verdict == Verdict::Ok) as usize;
        e[2] += (r.seq.verdict == Verdict::Reject) as usize;
        e[3] += (r.nary.verdict == Verdict::Ok) as usize;
        e[4] += (r.nary.verdict == Verdict::Reject) as usize;
        e[5] += r.disagree as usize;
        e[6] += (r.seq.ms * 1e3) as usize;
        e[7] += (r.nary.ms * 1e3) as usize;
    }
    println!(
        "{:<26} {:>5} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>8} {:>8}",
        "family", "parts", "seq ok", "rej", "fail", "nry ok", "rej", "fail", "disagree", "ms s/n"
    );
    let mut tot = [0usize; 8];
    for f in &order {
        let e = fams[f];
        println!(
            "{f:<26} {:>5} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>8} {:>5.0}/{:.0}",
            e[0],
            e[1],
            e[2],
            e[0] - e[1] - e[2],
            e[3],
            e[4],
            e[0] - e[3] - e[4],
            e[5],
            e[6] as f64 / 1e3,
            e[7] as f64 / 1e3
        );
        for k in 0..8 {
            tot[k] += e[k];
        }
    }
    let good = |r: &Row| r.seq.verdict.good() && r.nary.verdict.good() && !r.disagree;
    let score = rows.iter().filter(|r| good(r)).count();
    println!(
        "{:<26} {:>5} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>8}",
        "TOTAL",
        tot[0],
        tot[1],
        tot[2],
        tot[0] - tot[1] - tot[2],
        tot[3],
        tot[4],
        tot[0] - tot[3] - tot[4],
        tot[5]
    );
    println!(
        "score: {score}/{} parts good on both paths ({:.1}%), {:.1} s",
        rows.len(),
        100.0 * score as f64 / rows.len() as f64,
        t0.elapsed().as_secs_f64()
    );
    if show {
        for r in rows.iter().filter(|r| !good(r)) {
            println!(
                "  {:<52} seq {:<7} nary {:<7}{}  {}",
                r.name,
                r.seq.verdict.word(),
                r.nary.verdict.word(),
                if r.disagree { " DISAGREE" } else { "" },
                if r.seq.verdict.good() {
                    &r.nary.detail
                } else {
                    &r.seq.detail
                }
            );
        }
    }
    let line = |r: &Row| {
        format!(
            "{} {} {}{}",
            r.name,
            r.seq.verdict.word(),
            r.nary.verdict.word(),
            if r.disagree { " disagree" } else { "" }
        )
    };
    if let Some(path) = flag("--save") {
        let out: Vec<String> = rows.iter().map(line).collect();
        std::fs::write(&path, out.join("\n") + "\n").expect("write baseline");
        println!("saved {path}");
    }
    if let Some(path) = flag("--baseline") {
        let text = std::fs::read_to_string(&path).expect("read baseline");
        let before: BTreeMap<&str, (&str, &str, bool)> = text
            .lines()
            .filter_map(|l| {
                let f: Vec<&str> = l.split(' ').collect();
                (f.len() >= 3).then(|| (f[0], (f[1], f[2], f.len() > 3)))
            })
            .collect();
        let ok = |w: &str| w == "ok" || w == "reject";
        let mut worse = Vec::new();
        let mut better = 0;
        for r in &rows {
            let Some(&(s, n, d)) = before.get(r.name.as_str()) else {
                continue;
            };
            let was = (ok(s), ok(n), !d);
            let now = (r.seq.verdict.good(), r.nary.verdict.good(), !r.disagree);
            if (was.0 && !now.0) || (was.1 && !now.1) || (was.2 && !now.2) {
                worse.push(format!("{}  (was {s} {n})", line(r)));
            } else if now != was {
                better += 1;
            }
        }
        println!("against {path}: {} worse, {better} better", worse.len());
        for w in &worse {
            println!("  WORSE {w}");
        }
        if !worse.is_empty() {
            std::process::exit(1);
        }
    }
}
