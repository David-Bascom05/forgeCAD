//! research/cam-speed: one machining document, the app's path (`forge_cam_lang::run` and the
//! reply's JSON), cold in this fresh process, rerun unchanged, then after edits (one op's
//! parameter, the first op's, a tool's, the part's), each in the same process as the app does.
//! Prints one JSON object: per run its wall and CPU seconds, the most threads seen, the reply's
//! hash (reply JSON and every program's text) and, with FORGE_CAM_PROF set, each phase span.
//!
//! cargo run --release -p forge-cam-lang --example cam_bench -- DOC.fg [OUT_DIR]
//!
//! OUT_DIR: the cold run's reply and program texts are written there (`DOC.out`) for
//! byte-for-byte comparison across builds.
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const NOT_OPS: &[&str] = &[
    "part", "machine", "tool", "setup", "program", "box", "fixture", "vise", "jaws", "soft_jaw", "stock",
    "probe", "holder",
];

/// CPU seconds of this process so far (user + system, /proc's clock ticks).
fn cpu() -> f64 {
    let s = std::fs::read_to_string("/proc/self/stat").unwrap_or_default();
    // fields after the command's closing parenthesis; utime and stime are fields 14 and 15
    let rest = s.rsplit_once(')').map(|x| x.1).unwrap_or("");
    let f: Vec<&str> = rest.split_whitespace().collect();
    let tick = 100.0;
    let u: f64 = f.get(11).and_then(|x| x.parse().ok()).unwrap_or(0.0);
    let k: f64 = f.get(12).and_then(|x| x.parse().ok()).unwrap_or(0.0);
    (u + k) / tick
}

fn threads() -> usize {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("Threads:"))
                .and_then(|l| l[8..].trim().parse().ok())
        })
        .unwrap_or(0)
}

static MAXT: AtomicUsize = AtomicUsize::new(0);
static STOP: AtomicBool = AtomicBool::new(false);

fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
}

struct R {
    what: String,
    wall: f64,
    cpu: f64,
    json_s: f64,
    threads: usize,
    ok: bool,
    hash: u64,
    text: String,
    prof: Vec<(String, f64, u64)>,
}

fn run(what: &str, dir: &std::path::Path, doc: &str) -> R {
    let src = std::fs::read_to_string(dir.join(doc)).expect("read");
    MAXT.store(threads(), Ordering::Relaxed);
    let c0 = cpu();
    let t0 = std::time::Instant::now();
    let mut text = String::new();
    // a turning document runs on forge-turn-lang (as forge-render's run_unmeasured does)
    let (ok, t1, t2) = if forge_turn_lang::is_turning(&src) {
        let cam = forge_turn_lang::run(&src, dir);
        let t1 = std::time::Instant::now();
        cam.json().write(&mut text);
        let t2 = std::time::Instant::now();
        for pr in &cam.programs {
            text.push_str(&format!("\n{:?}", pr.text));
        }
        (cam.ok, t1, t2)
    } else {
        let cam = forge_cam_lang::run(&src, dir);
        let t1 = std::time::Instant::now();
        // the app's reply (forge-render's run_unmeasured writes it)
        cam.json().write(&mut text);
        let t2 = std::time::Instant::now();
        for pr in &cam.programs {
            text.push_str(&format!("\n{:?}", pr.text));
        }
        (cam.ok, t1, t2)
    };
    let json_s = (t2 - t1).as_secs_f64();
    let wall = (t2 - t0).as_secs_f64();
    let cpu = cpu() - c0;
    R {
        what: what.into(),
        wall,
        cpu,
        json_s,
        threads: MAXT.load(Ordering::Relaxed),
        ok,
        hash: fnv(&text),
        text,
        prof: forge_cam_lang::prof::take(),
    }
}

/// A statement `NAME = f(..)`: (line index, name, f).
type Stmt = (usize, String, String);

/// The statements `NAME = f(..)` of a document: (line index, name, f).
fn stmts(src: &str) -> Vec<(usize, String, String)> {
    let mut v = vec![];
    for (i, l) in src.lines().enumerate() {
        let t = l.trim_start();
        if t.starts_with('#') {
            continue;
        }
        let Some((n, rest)) = t.split_once('=') else {
            continue;
        };
        let n = n.trim();
        if n.is_empty() || !n.chars().all(|c| c.is_alphanumeric() || c == '_') {
            continue;
        }
        let rest = rest.trim_start();
        let Some((f, _)) = rest.split_once('(') else {
            continue;
        };
        if f.chars().all(|c| c.is_alphanumeric() || c == '_') {
            v.push((i, n.to_string(), f.to_string()));
        }
    }
    v
}

/// `key: NUMBER` in `line` replaced by `g(NUMBER)`; None when there is none.
fn bump(line: &str, key: &str, g: impl Fn(f64) -> f64) -> Option<String> {
    let pat = format!("{key}:");
    let mut from = 0;
    while let Some(k) = line[from..].find(&pat) {
        let at = from + k;
        // a whole argument name
        let before = line[..at].chars().last();
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
            from = at + pat.len();
            continue;
        }
        let s = at + pat.len();
        let num_start = s + line[s..].len() - line[s..].trim_start().len();
        let end = line[num_start..]
            .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
            .map(|e| num_start + e)
            .unwrap_or(line.len());
        let x: f64 = line[num_start..end].parse().ok()?;
        // a unit after the number is kept
        return Some(format!("{}{}{}", &line[..num_start], fmt(g(x)), &line[end..]));
    }
    None
}

fn fmt(x: f64) -> String {
    let s = format!("{:.4}", x);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_string()
}

fn op_edit(
    src: &str,
    which: &dyn Fn(&[Stmt]) -> Option<usize>,
) -> Option<(String, String)> {
    let st: Vec<_> = stmts(src)
        .into_iter()
        .filter(|s| !NOT_OPS.contains(&s.2.as_str()))
        .collect();
    let k = which(&st)?;
    let (li, name, f) = &st[k];
    let lines: Vec<&str> = src.lines().collect();
    let l = lines[*li];
    let new = bump(l, "stepover", |x| x * 0.9)
        .or_else(|| bump(l, "step", |x| x * 0.9))
        .or_else(|| bump(l, "leave", |x| x + 0.05))
        .or_else(|| bump(l, "to", |x| x + 0.1))
        .or_else(|| bump(l, "depth", |x| x * 0.95))?;
    let mut out: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
    out[*li] = new;
    Some((out.join("\n") + "\n", format!("{name} ({f})")))
}

fn tool_edit(src: &str) -> Option<(String, String)> {
    let st: Vec<_> = stmts(src).into_iter().filter(|s| s.2 == "tool").collect();
    let (li, name, _) = st.last()?;
    let lines: Vec<&str> = src.lines().collect();
    let new = bump(lines[*li], "stickout", |x| x + 1.0)?;
    let mut out: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
    out[*li] = new;
    Some((out.join("\n") + "\n", format!("{name} stickout +1")))
}

/// The part's first `shared`/`param` number scaled by 1.02.
fn part_edit(src: &str) -> Option<(String, String)> {
    let lines: Vec<&str> = src.lines().collect();
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim_start();
        if !(t.starts_with("shared ") || t.starts_with("param ")) {
            continue;
        }
        let Some((lhs, rhs)) = t.split_once('=') else {
            continue;
        };
        let r = rhs.trim();
        let end = r
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(r.len());
        let Ok(x) = r[..end].parse::<f64>() else { continue };
        if x == 0.0 {
            continue;
        }
        let mut out: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
        out[i] = format!("{}= {}{}", lhs, fmt(x * 1.02), &r[end..]);
        return Some((out.join("\n") + "\n", format!("part {} x1.02", lhs.trim())));
    }
    None
}

fn part_file(src: &str) -> Option<String> {
    for l in src.lines() {
        if let Some(k) = l.find("part(\"") {
            let r = &l[k + 6..];
            return r.find('"').map(|e| r[..e].to_string());
        }
    }
    None
}

fn copy_dir(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).expect("scratch");
    for e in std::fs::read_dir(from).expect("dir").flatten() {
        let p = e.path();
        if p.is_file() {
            std::fs::copy(&p, to.join(e.file_name())).expect("copy");
        } else if p.is_dir() && e.file_name() != "out" && e.file_name() != "renders" {
            copy_dir(&p, &to.join(e.file_name()));
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let docp = std::path::PathBuf::from(args.next().expect("DOC.fg"));
    let out_dir = args.next();
    let src_dir = docp.parent().unwrap_or(std::path::Path::new(".")).to_path_buf();
    let doc = docp.file_name().unwrap().to_string_lossy().into_owned();
    let stem = docp.file_stem().unwrap().to_string_lossy().into_owned();
    let scratch = std::env::temp_dir().join(format!("cam_bench_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    // the document's directory copied (the edits are made there) at its place under the
    // checkout, every other entry along its path linked: a part's relative imports
    // (`../../cam-parts/x.step`) reach what they reach in the checkout
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("checkout");
    let abs = src_dir.canonicalize().expect("doc dir");
    let rel = abs.strip_prefix(&root).expect("the document is in the checkout").to_path_buf();
    let work = scratch.join(&rel);
    copy_dir(&src_dir, &work);
    let mut at = rel.as_path();
    while let Some(parent) = at.parent() {
        let keep = at.file_name().unwrap();
        for e in std::fs::read_dir(root.join(parent)).expect("dir").flatten() {
            if e.file_name() != keep && e.file_name() != "target" && e.file_name() != ".git" {
                let _ = std::os::unix::fs::symlink(e.path(), scratch.join(parent).join(e.file_name()));
            }
        }
        at = parent;
    }
    let sampler = std::thread::spawn(|| {
        while !STOP.load(Ordering::Relaxed) {
            MAXT.fetch_max(threads(), Ordering::Relaxed);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    });
    let mut rs = vec![];
    let cold = run("cold", &work, &doc);
    if let Some(o) = &out_dir {
        std::fs::write(std::path::Path::new(o).join(format!("{stem}.out")), &cold.text).expect("out");
    }
    rs.push(cold);
    // CAM_BENCH_COLD=1: the cold run only (for a profiler)
    let cold_only = std::env::var_os("CAM_BENCH_COLD").is_some();
    if cold_only {
        STOP.store(true, Ordering::Relaxed);
    }
    if !cold_only {
        rs.push(run("rerun", &work, &doc));
        let orig = std::fs::read_to_string(work.join(&doc)).unwrap();
        let edit = |what: &str, file: &str, new: Option<(String, String)>, rs: &mut Vec<R>| {
            let Some((text, label)) = new else { return };
            let p = work.join(file);
            let before = std::fs::read_to_string(&p).unwrap();
            std::fs::write(&p, text).unwrap();
            rs.push(run(&format!("{what}: {label}"), &work, &doc));
            std::fs::write(&p, before).unwrap();
        };
        edit(
            "edit_last_op",
            &doc,
            op_edit(&orig, &|s| s.len().checked_sub(1)),
            &mut rs,
        );
        edit(
            "edit_first_op",
            &doc,
            op_edit(&orig, &|s| (!s.is_empty()).then_some(0)),
            &mut rs,
        );
        edit("edit_tool", &doc, tool_edit(&orig), &mut rs);
        if let Some(pf) = part_file(&orig) {
            let ps = std::fs::read_to_string(work.join(&pf)).unwrap_or_default();
            edit("edit_part", &pf, part_edit(&ps), &mut rs);
        }
    }
    STOP.store(true, Ordering::Relaxed);
    let _ = sampler.join();
    let _ = std::fs::remove_dir_all(&scratch);
    let mut o = format!("{{\"doc\":\"{}\",\"runs\":[", esc(&docp.display().to_string()));
    for (i, r) in rs.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        o.push_str(&format!(
            "{{\"what\":\"{}\",\"wall\":{:.6},\"cpu\":{:.3},\"json\":{:.6},\"threads\":{},\"ok\":{},\"hash\":\"{:016x}\",\"prof\":[",
            esc(&r.what),
            r.wall,
            r.cpu,
            r.json_s,
            r.threads,
            r.ok,
            r.hash
        ));
        for (k, (n, s, c)) in r.prof.iter().enumerate() {
            if k > 0 {
                o.push(',');
            }
            o.push_str(&format!("[\"{}\",{:.6},{}]", esc(n), s, c));
        }
        o.push_str("]}");
    }
    o.push_str("]}");
    println!("{o}");
}
