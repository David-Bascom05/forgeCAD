//! The sheet-metal kernel against its Lean spec (verify/sheet): every sheet part in the repository
//! and seeded random strips are built folded and flat, and what the kernel publishes (the gauge,
//! each wall's folded frame and developed place, each bend's numbers and developed place, and the
//! flat vertices with their folded namesakes) is written as exact `f64` bits for the Lean oracle
//! (`lake exe sheet-oracle`, verify/sheet/Sheet/Oracle.lean), which recomputes the fold in exact
//! rationals with proven sin/cos enclosures and judges audit2 §2.0's conditions 1, 3/4, 5 and 7.
//!
//! The oracle's answers are kept as a digest in tests/sheet_lean_diff.digest, one line per corpus:
//! its name, the number of inputs, an FNV-1a of the input and one of the oracle's answers.
//! `SHEET_LEAN=1` runs the oracle (built with `cd verify/sheet && lake build sheet-oracle`),
//! prints every answer that is not `pass`, and rewrites the digest.
use forge_lang::{Options, Run};
use forge_math::{Frame, Point3};
use forge_sheet::build::{Built, Map, build_with};
use std::collections::BTreeMap;
use std::fmt::Write;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
const DIGEST_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/sheet_lean_diff.digest");

fn h(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn p3(p: Point3) -> String {
    format!("{} {} {}", h(p.x), h(p.y), h(p.z))
}

fn frame(f: &Frame) -> String {
    let (x, y, z) = (f.x().get(), f.y().get(), f.z().get());
    format!(
        "{} {} {} {} {} {} {} {} {} {}",
        p3(f.origin),
        h(x.x),
        h(x.y),
        h(x.z),
        h(y.x),
        h(y.y),
        h(y.z),
        h(z.x),
        h(z.y),
        h(z.z)
    )
}

fn place(p: &forge_sheet::build::Place2) -> String {
    format!("{} {} {} {}", h(p.origin.x), h(p.origin.y), h(p.x.x), h(p.x.y))
}

/// One sheet's records, or None where the kernel refuses either map (nothing to judge).
fn dump(name: &str, sheet: &forge_sheet::Sheet, fold: &Built, flat: &Built) -> Option<String> {
    let mut s = String::new();
    let _ = writeln!(s, "sheet {name}");
    let _ = writeln!(s, "t {}", h(sheet.gauge.thickness));
    let mut flat_of = BTreeMap::new();
    for l in &flat.layout.walls {
        flat_of.insert(l.wall, l.frame);
    }
    for w in &fold.layout.walls {
        let _ = writeln!(s, "wall {} {} {}", w.wall.0, frame(&w.frame), place(&w.developed));
    }
    for b in &fold.layout.bends {
        let _ = writeln!(
            s,
            "bend {} {} {} {} {} {} {} {} {}",
            b.joint.0,
            b.a.0,
            b.b.0,
            h(b.angle),
            h(b.radius),
            h(b.allowance),
            h(b.setback),
            h(b.length),
            place(&b.developed)
        );
    }
    for j in &sheet.joints {
        if matches!(j.kind, forge_sheet::JointKind::Hem(_)) {
            let _ = writeln!(s, "hem {}", j.id.0);
        }
    }
    let w0 = fold.layout.walls.first()?;
    let _ = writeln!(s, "flat0 {}", frame(flat_of.get(&w0.wall)?));
    let _ = writeln!(s, "dev0 {}", place(&w0.developed));
    // a form's vertices are formed folded and pierced flat by design: not a fold of each other
    if sheet.forms.is_empty() {
        let folded: BTreeMap<_, Point3> = fold.body.vertices.iter().map(|v| (v.name, v.point)).collect();
        for v in &flat.body.vertices {
            if let Some(&f) = folded.get(&v.name) {
                let _ = writeln!(s, "vert {} {}", p3(v.point), p3(f));
            }
        }
    }
    s.push_str("end\n");
    Some(s)
}

fn sheets_of(name: &str, src: &str, out: &mut Vec<String>, refused: &mut Vec<String>) {
    let r: Run = forge_lang::run(src, Options::default(), Default::default()).0;
    for (i, so) in r.sheets.iter().enumerate() {
        let tag = format!("{name}#{i}");
        match (
            build_with(&so.sheet, Map::Folded),
            build_with(&so.sheet, Map::Flat),
        ) {
            (Ok(fold), Ok(flat)) => match dump(&tag, &so.sheet, &fold, &flat) {
                Some(d) => out.push(d),
                None => refused.push(tag),
            },
            _ => refused.push(tag),
        }
    }
}

fn fg_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut es: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    es.sort();
    for p in es {
        if p.is_dir() {
            fg_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "fg") {
            out.push(p);
        }
    }
}

/// FNV-1a.
fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

fn oracle(corpus: &str, input: &str, n: usize) {
    let key = format!("{corpus} {n} {:016x}", fnv(input));
    if std::env::var_os("SHEET_LEAN").is_none() {
        let have = std::fs::read_to_string(DIGEST_PATH).unwrap_or_default();
        assert!(
            have.lines().any(|l| l.starts_with(&key)),
            "{corpus}: no Lean answers for these inputs ({key}); run the oracle: (cd verify/sheet && lake build sheet-oracle) then SHEET_LEAN=1 cargo test --release -p forge-lang --test sheet_lean_diff"
        );
        return;
    }
    let path = format!("{ROOT}/verify/sheet/.lake/build/bin/sheet-oracle");
    let mut child = std::process::Command::new(&path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("{path}: {e}; lake build sheet-oracle first"));
    let mut stdin = child.stdin.take().expect("stdin");
    let feed = input.to_string();
    let t = std::thread::spawn(move || {
        use std::io::Write as _;
        stdin.write_all(feed.as_bytes()).expect("write to the oracle");
    });
    let out = child.wait_with_output().expect("the oracle runs");
    t.join().expect("fed");
    let out = String::from_utf8(out.stdout).expect("utf8");
    let not_pass: Vec<&str> = out.lines().filter(|l| !l.ends_with(": pass")).collect();
    eprintln!(
        "{corpus}: {} answers, {} not pass",
        out.lines().count(),
        not_pass.len()
    );
    for l in &not_pass {
        eprintln!("  {l}");
    }
    static FILE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _one = FILE.lock().unwrap_or_else(|e| e.into_inner());
    let have = std::fs::read_to_string(DIGEST_PATH).unwrap_or_default();
    let mut lines: Vec<String> = have
        .lines()
        .filter(|l| l.split(' ').next() != Some(corpus))
        .map(str::to_string)
        .collect();
    lines.push(format!(
        "{key} {:016x} {}",
        fnv(&out),
        if not_pass.is_empty() {
            "all-pass".to_string()
        } else {
            format!("{}-not-pass", not_pass.len())
        }
    ));
    lines.sort();
    std::fs::write(DIGEST_PATH, lines.join("\n") + "\n").expect("write the digest");
}

/// Every sheet part in the repository: parts/sheet and examples/.
#[test]
fn every_sheet_part() {
    let mut files = Vec::new();
    fg_files(
        std::path::Path::new(&format!("{ROOT}/interface/forge-lang/parts/sheet")),
        &mut files,
    );
    fg_files(std::path::Path::new(&format!("{ROOT}/examples")), &mut files);
    let (mut out, mut refused) = (Vec::new(), Vec::new());
    for f in &files {
        let src = std::fs::read_to_string(f).expect("read");
        if !src.contains("sheet(") {
            continue;
        }
        let name = f
            .strip_prefix(ROOT)
            .unwrap_or(f)
            .to_string_lossy()
            .replace(' ', "_");
        sheets_of(&name, &src, &mut out, &mut refused);
    }
    eprintln!(
        "{} sheets dumped, {} refused by the kernel",
        out.len(),
        refused.len()
    );
    assert!(!out.is_empty(), "no sheet parts found");
    oracle("parts", &out.concat(), out.len());
}

/// Seeded random strips: a base, and two flanges at random angle, sense, radius, K and gauge.
#[test]
fn random_strips() {
    let mut rng = forge_math::rng::Rng::new(0x5_4ee7);
    let (mut out, mut refused) = (Vec::new(), Vec::new());
    for i in 0..60 {
        let t = [0.5, 1.0, 1.5, 2.0, 3.0][rng.below(5)];
        let r = t * rng.range(0.3, 4.0);
        let k = rng.range(0.2, 0.5);
        let a1 = rng.range(20.0, 160.0);
        let a2 = rng.range(20.0, 160.0);
        let s1 = if rng.below(2) == 0 { "up" } else { "down" };
        let s2 = if rng.below(2) == 0 { "up" } else { "down" };
        let (l, w) = (rng.range(30.0, 80.0), rng.range(20.0, 60.0));
        let (h1, h2) = (rng.range(10.0, 40.0), rng.range(10.0, 40.0));
        let src = format!(
            "shared t = {t}\nshared r = {r}\nshared k = ratio({k})\nparam plan.L = {l}\nparam plan.W = {w}\n\
             param right.h = {h1}\nparam right.a = {a1}deg\nparam left.h = {h2}\nparam left.a = {a2}deg\n\
             sketch plan(on: xy) {{\n  outline = rect(corner: (0, 0), size: (L, W))\n}}\n\
             base = sheet(plan, t: t, r: r, bend: kfactor(k))\n\
             right = flange(base.edge(plan.outline[1]), height: h, angle: a, side: \"{s1}\", measure: \"inner\")\n\
             left = flange(base.edge(plan.outline[3]), height: h, angle: a, side: \"{s2}\", measure: \"outer\")\n"
        );
        sheets_of(&format!("strip{i}"), &src, &mut out, &mut refused);
    }
    eprintln!("{} strips dumped, {} refused", out.len(), refused.len());
    assert!(out.len() >= 40, "too few strips built: {refused:?}");
    oracle("strips", &out.concat(), out.len());
}
