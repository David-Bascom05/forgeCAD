//! forge-det: the cross-platform determinism gate (research/hosted/DESIGN.md §13.2).
//!
//! Every platform that runs forgeCAD must build the same bits from the same document and the
//! same op stream: hosted collaborators rebuild locally, so a difference is a divergence between
//! them. This prints one line per (document, op) with the exact hashes; CI runs it on every
//! target and diffs the files against linux x86-64-v3, with no tolerance.
//!
//! usage:
//!   forge-det [--root DIR] [--corpus] [--streams] [--shard K/N] [FILE.fg ..]
//!   forge-det [--root DIR] --record      (re)write verify/forge-det/streams/*.ops
//!
//! Lines (stdout), all with paths relative to the root, '/'-separated (a stream's lines name its
//! document DOC+ops):
//!   DOC OP BODY MESH ERRS          OP 0 = the document as written, k = after the k-th op of its
//!                                  stream; BODY/MESH are 16 hex digits; ERRS the error kinds
//!                                  joined by '+' ('-' for none)
//!   DOC OP REFUSED KIND            the k-th op was refused (its typed error's kind)
//!   DOC OP WARM_DIFFERS BODY MESH  the warm rebuild (the Cache of the previous run, as an editing
//!                                  app keeps it) gave other bits than a cold build: a divergence
//!                                  by itself (BODY/MESH are the warm ones)
//!   consts/EXPR 0 FOLDED RUNTIME SAME  (`--consts`, and with shard 0 of `--corpus`) the bits of
//!                                  a constant expression folded at compile time (the build
//!                                  host's libm: LLVM folds powi and libm calls with it) and the
//!                                  same at run time (black_box); SAME is '-' or FOLD when they
//!                                  differ. Diffed across targets and build hosts like every line:
//!                                  a fold that depends on the build host shows here first
//! Timing goes to stderr only. Run with FORGECAD_CACHE=off (the disk cache would make a result
//! depend on what ran before); it refuses to run without it.
use forge_agent::json::Json;
use forge_lang::edit;
use forge_lang::error::Op;
use forge_math::hash::BitHasher;
use std::path::{Path, PathBuf};

fn main() {
    let mut root: Option<PathBuf> = None;
    let (mut corpus, mut streams, mut record, mut consts) = (false, false, false, false);
    let mut shard = (0usize, 1usize);
    let mut files: Vec<String> = vec![];
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--root" => root = args.next().map(PathBuf::from),
            "--corpus" => corpus = true,
            "--streams" => streams = true,
            "--record" => record = true,
            "--consts" => consts = true,
            "--shard" => {
                let s = args.next().unwrap_or_default();
                let (k, n) = s.split_once('/').unwrap_or(("0", "1"));
                shard = (k.parse().unwrap_or(0), n.parse().unwrap_or(1).max(1));
            }
            f => files.push(f.to_string()),
        }
    }
    if std::env::var("FORGECAD_CACHE").as_deref() != Ok("off") {
        eprintln!("forge-det: run with FORGECAD_CACHE=off (a cached result would hide the build)");
        std::process::exit(2);
    }
    let root = root.unwrap_or_else(|| std::env::current_dir().expect("a working directory"));
    if corpus || record {
        let mut all = vec![];
        walk(&root.join("examples"), &mut all);
        all.sort();
        files.extend(all.iter().map(|p| rel(&root, p)));
    }
    if record {
        record_streams(&root, &files);
        return;
    }
    if consts || (corpus && shard.0 == 0) {
        print_consts();
    }
    let t0 = forge_math::clock::system_now();
    let mut n = 0usize;
    for (k, f) in files.iter().enumerate() {
        if k % shard.1 == shard.0 {
            n += 1;
            replay(&root, f, &[], f);
        }
    }
    if streams {
        let dir = root.join("verify/forge-det/streams");
        let mut list = vec![];
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().is_some_and(|x| x == "ops") {
                    list.push(p);
                }
            }
        }
        list.sort();
        for (k, p) in list.iter().enumerate() {
            if k % shard.1 != shard.0 {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(p) else {
                println!("{} 0 UNREADABLE", rel(&root, p));
                continue;
            };
            let mut lines = text.lines().filter(|l| !l.trim().is_empty());
            let doc = lines
                .next()
                .and_then(|l| Json::parse(l).ok())
                .and_then(|j| j.get("doc").and_then(Json::str).map(str::to_string));
            let Some(doc) = doc else {
                println!("{} 0 BAD_STREAM", rel(&root, p));
                continue;
            };
            let ops: Vec<Option<Op>> = lines
                .map(|l| Json::parse(l).ok().and_then(|j| Op::from_json(&j)))
                .collect();
            n += 1;
            replay(&root, &doc, &ops, &format!("{doc}+ops"));
        }
    }
    let secs = forge_math::clock::system_now()
        .duration_since(t0)
        .map_or(0.0, |d| d.as_secs_f64());
    eprintln!(
        "forge-det: {n} runs in {secs:.1} s, {} threads",
        forge_math::par::threads()
    );
}

/// The constant-expression lines (see the header): each written once as a literal expression
/// (folded by the compiler in release) and once through `black_box` (computed at run time).
// the very calls the kernel must not make (std's libm and powi): what they fold to is the check
#[allow(clippy::disallowed_methods)]
fn print_consts() {
    use std::hint::black_box as b;
    fn line(name: &str, folded: f64, runtime: f64) {
        let (f, r) = (folded.to_bits(), runtime.to_bits());
        let same = if f == r { "-" } else { "FOLD" };
        println!("consts/{name} 0 {f:016x} {r:016x} {same}");
    }
    macro_rules! c {
        ($name:expr, |$x:ident| $e:expr, $v:expr) => {{
            let folded = {
                let $x: f64 = $v;
                $e
            };
            let runtime = {
                let $x: f64 = b($v);
                $e
            };
            line($name, folded, runtime);
        }};
    }
    // powi: folded with the host's pow (one rounding), at run time a chain of multiplies
    c!("powi(95.65,3)", |x| x.powi(b(3)), 95.65);
    c!("powi(95.65,3)lit", |x| x.powi(3), 95.65);
    c!("powi(1.1,7)", |x| x.powi(7), 1.1);
    c!("powi(0.3,5)", |x| x.powi(5), 0.3);
    c!("powi(2.5,-3)", |x| x.powi(-3), 2.5);
    c!("powi(10,-2)", |x| x.powi(-2), 10.0);
    c!("powi(1.0000001,17)", |x| x.powi(17), 1.0000001);
    c!("powi(3.25,4)", |x| x.powi(4), 3.25);
    c!("powi(-0.7,9)", |x| x.powi(9), -0.7);
    // libm-shaped calls the compiler may fold with the host's libm
    c!("sin(1)", |x| x.sin(), 1.0);
    c!("cos(0.7)", |x| x.cos(), 0.7);
    c!("tan(0.3)", |x| x.tan(), 0.3);
    c!("asin(0.4)", |x| x.asin(), 0.4);
    c!("acos(0.6)", |x| x.acos(), 0.6);
    c!("atan(0.9)", |x| x.atan(), 0.9);
    c!("atan2(1,3)", |x| x.atan2(3.0), 1.0);
    c!("exp(1.3)", |x| x.exp(), 1.3);
    c!("exp_m1(0.01)", |x| x.exp_m1(), 0.01);
    c!("ln(7.7)", |x| x.ln(), 7.7);
    c!("ln_1p(0.02)", |x| x.ln_1p(), 0.02);
    c!("log10(42)", |x| x.log10(), 42.0);
    c!("sqrt(2)", |x| x.sqrt(), 2.0);
    c!("cbrt(3)", |x| x.cbrt(), 3.0);
    c!("powf(2.1,3.3)", |x| x.powf(3.3), 2.1);
    c!("hypot(3.1,4.2)", |x| x.hypot(4.2), 3.1);
    c!("sinh(0.5)", |x| x.sinh(), 0.5);
    c!("cosh(0.5)", |x| x.cosh(), 0.5);
    c!("tanh(0.5)", |x| x.tanh(), 0.5);
    c!("mul_add(1.1,2.2,3.3)", |x| x.mul_add(2.2, 3.3), 1.1);
}

/// Every .fg under `dir`.
fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x == "fg") {
            out.push(p);
        }
    }
}

/// `p` relative to `root`, with '/' (the same name on every OS).
fn rel(root: &Path, p: &Path) -> String {
    let r = p.strip_prefix(root).unwrap_or(p);
    r.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn opts(path: &Path) -> forge_lang::Options {
    forge_lang::Options {
        dir: path.parent().map(Path::to_path_buf),
        ..forge_lang::Options::default()
    }
}

/// The document `doc` (relative to `root`), then each op of `ops` applied in turn, hashed after
/// each: cold, and warm from the previous run's Cache.
fn replay(root: &Path, doc: &str, ops: &[Option<Op>], label: &str) {
    let path = root.join(doc);
    let Ok(src) = std::fs::read_to_string(&path) else {
        println!("{label} 0 UNREADABLE");
        return;
    };
    let (run, mut cache) = forge_lang::run(&src, opts(&path), Default::default());
    println!("{label} 0 {}", hash_run(&run).line());
    let dir = path.parent().map(Path::to_path_buf);
    let mut text = src;
    for (k, op) in ops.iter().enumerate() {
        let k = k + 1;
        let Some(op) = op else {
            println!("{label} {k} BAD_OP");
            continue;
        };
        let next = match edit::apply_checked_in(&text, op, dir.clone()) {
            Ok((t, _)) => t,
            Err(e) => {
                println!("{label} {k} REFUSED {}", e.kind());
                continue;
            }
        };
        let (warm, c2) = forge_lang::run(&next, opts(&path), cache);
        cache = c2;
        let (cold, _) = forge_lang::run(&next, opts(&path), Default::default());
        let (hw, hc) = (hash_run(&warm), hash_run(&cold));
        println!("{label} {k} {}", hc.line());
        if hw != hc {
            println!("{label} {k} WARM_DIFFERS {:016x} {:016x}", hw.body, hw.mesh);
        }
        text = next;
    }
}

#[derive(PartialEq)]
struct Hashes {
    body: u64,
    mesh: u64,
    errs: String,
}

impl Hashes {
    fn line(&self) -> String {
        format!("{:016x} {:016x} {}", self.body, self.mesh, self.errs)
    }
}

fn body_bits(h: &mut BitHasher, b: &forge_topo::Body) {
    for v in &b.vertices {
        h.p3(v.point);
    }
    for e in &b.edges {
        h.f64(e.range.lo);
        h.f64(e.range.hi);
    }
    h.u64(b.faces.len() as u64);
}

fn mesh_bits(m: &mut BitHasher, b: &forge_topo::Body) {
    let q = forge_mesh::tessellate::Quality::default();
    let r = if b.from_file {
        forge_mesh::tessellate::tessellate_tolerant(b, &q)
    } else {
        forge_mesh::tessellate::tessellate(b, &q)
    };
    match r {
        Ok(mesh) => {
            for p in &mesh.positions {
                m.p3(*p);
            }
            for t in &mesh.triangles {
                m.u64(t[0] as u64 | (t[1] as u64) << 21 | (t[2] as u64) << 42);
            }
            m.u64(mesh.triangles.len() as u64);
        }
        Err(_) => m.u64(u64::MAX),
    }
}

/// The exact bits of a run (research/hosted/proto's hash, as tools/speed's `bits`): every
/// vertex's point and edge's range, the face count, the error kinds; the mesh at the default
/// quality. An assembly: each drawn occurrence's body bits and pose, its mobility and
/// interference count (its parts' meshes are not hashed: the part documents are, on their own).
fn hash_run(run: &forge_lang::Run) -> Hashes {
    let mut h = BitHasher::new();
    let mut m = BitHasher::new();
    let kinds: Vec<&str> = run.errors.iter().map(|e| e.kind()).collect();
    for k in &kinds {
        for b in k.bytes() {
            h.u64(b as u64);
        }
    }
    h.u64(kinds.len() as u64);
    match &run.body {
        Some(b) => {
            body_bits(&mut h, b);
            mesh_bits(&mut m, b);
        }
        None => h.u64(u64::MAX),
    }
    if let Some(a) = &run.asm {
        for (body, frame) in &a.draw {
            body_bits(&mut h, body);
            for b in format!("{frame:?}").bytes() {
                h.u64(b as u64);
            }
        }
        h.u64(a.mobility as u64);
        h.u64(a.interferences as u64);
    }
    Hashes {
        body: h.finish(),
        mesh: m.finish(),
        errs: if kinds.is_empty() {
            "-".into()
        } else {
            kinds.join("+")
        },
    }
}

/// Writes one stream per document that evaluates cleanly: up to 3 SetParams (each handle ×1.05),
/// a Rename of a top-level feature, a MoveFeature to the end, and a ReplaceFeature that keeps
/// the statement's text. Only ops `apply_checked` accepts in turn are kept, so the streams are
/// edits a user could make. Deterministic: the same corpus gives the same files.
fn record_streams(root: &Path, files: &[String]) {
    let dir = root.join("verify/forge-det/streams");
    let _ = std::fs::create_dir_all(&dir);
    let mut written = 0;
    for doc in files {
        let path = root.join(doc);
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        let pdir = path.parent().map(Path::to_path_buf);
        let o = forge_lang::Options {
            build: false,
            dir: pdir.clone(),
            ..Default::default()
        };
        let (run, _) = forge_lang::run(&src, o, Default::default());
        if !run.errors.is_empty() {
            continue;
        }
        let mut cands: Vec<Op> = vec![];
        for (key, lit) in edit::handles(&src).into_iter().take(3) {
            let mut v = lit;
            v.value = if v.value == 0.0 { 0.5 } else { v.value * 1.05 };
            cands.push(Op::SetParam { param: key, value: v });
        }
        let tops: Vec<_> = run.feats.iter().filter(|f| f.top_level).collect();
        if let Some(f) = tops.first() {
            cands.push(Op::Rename {
                from: f.id.clone(),
                to: format!("{}_r", f.id),
            });
        }
        if let Some(f) = tops.iter().rev().nth(1) {
            cands.push(Op::MoveFeature {
                id: f.id.clone(),
                before: None,
            });
        }
        if let Some(f) = tops.last()
            && let Some(stmt) = src.get(f.stmt.lo as usize..f.stmt.hi as usize)
        {
            cands.push(Op::ReplaceFeature {
                id: f.id.clone(),
                text: stmt.to_string(),
            });
        }
        let mut text = src.clone();
        let mut kept = vec![];
        for op in cands {
            // a Rename moves the names later candidates were made with: they are made on the
            // text before it, so take them through it as the hosted rebase would (they are kept
            // only if they still apply)
            if let Ok((t, _)) = edit::apply_checked_in(&text, &op, pdir.clone()) {
                text = t;
                kept.push(op);
            }
        }
        if kept.is_empty() {
            continue;
        }
        let mut out = write(&Json::obj(vec![("doc", doc.as_str().into())]));
        out.push('\n');
        for op in &kept {
            out.push_str(&write(&op.json()));
            out.push('\n');
        }
        let name = doc.trim_end_matches(".fg").replace('/', "__");
        if std::fs::write(dir.join(format!("{name}.ops")), out).is_ok() {
            written += 1;
        }
    }
    eprintln!("forge-det: {written} streams written");
}

/// JSON with every number exact (Rust's shortest round-trip form; forge-agent's own writer
/// rounds, fine for people, wrong for a param that must reach every platform bit for bit).
fn write(j: &Json) -> String {
    let mut out = String::new();
    write_into(j, &mut out);
    out
}

fn write_into(j: &Json, out: &mut String) {
    match j {
        Json::Num(n) if n.is_finite() => {
            use std::fmt::Write;
            let _ = write!(out, "{n}");
        }
        Json::Arr(a) => {
            out.push('[');
            for (i, v) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_into(v, out);
            }
            out.push(']');
        }
        Json::Obj(f) => {
            out.push('{');
            for (i, (k, v)) in f.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                Json::Str(k.clone()).write(out);
                out.push(':');
                write_into(v, out);
            }
            out.push('}');
        }
        other => other.write(out),
    }
}
