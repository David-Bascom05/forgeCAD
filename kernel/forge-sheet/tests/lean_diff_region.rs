//! region.rs's point-in-region against the Lean spec's exact rule (verify/sheet/Sheet/Region.lean:
//! the polygon over the loop's vertices by the half-open crossing rule, XOR each arc's lens, every
//! sign exact over the doubles' rationals). The trims' seeded case sets are subtracted, and each
//! result's components (those with no spline pieces; the Lean model has lines and arcs) are
//! asked about random points and points just off every vertex. The kernel's `holds` and the
//! model's `inLoop` are compared by `lake exe sheet-oracle`; the answers' digest is kept in
//! tests/lean_diff_region.digest. `SHEET_LEAN=1` runs the oracle, prints every disagreement and
//! rewrites the digest.
mod common;
use common::cases::{self, H, Kind, W};
use forge_math::Point2;
use forge_math::rng::Rng;
use forge_sheet::region::{Bound, Piece, holds, subtract};
use std::fmt::Write;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
const DIGEST_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/lean_diff_region.digest");

fn h(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn lp(s: &mut String, l: &[Bound]) -> bool {
    let _ = writeln!(s, "loop {}", l.len());
    for b in l {
        match b.piece {
            Piece::Line { .. } => {
                let _ = writeln!(s, "edge {} {} line", h(b.start.x), h(b.start.y));
            }
            Piece::Arc { c, r, t0, t1 } => {
                let _ = writeln!(
                    s,
                    "edge {} {} arc {} {} {} {}",
                    h(b.start.x),
                    h(b.start.y),
                    h(c.x),
                    h(c.y),
                    h(r),
                    u8::from(t1 > t0)
                );
            }
            Piece::Spline { .. } => return false,
        }
    }
    true
}

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
            "{corpus}: no Lean answers for these inputs ({key}); run the oracle: (cd verify/sheet && lake build sheet-oracle) then SHEET_LEAN=1 cargo test --release -p forge-sheet --test lean_diff_region"
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
    let mut disagree = 0usize;
    for l in out.lines() {
        if l.contains(" disagree") && !l.contains(", 0 disagree") || l.starts_with("  ") {
            eprintln!("{l}");
        }
        if l.starts_with("  disagree") {
            disagree += 1;
        }
    }
    eprintln!(
        "{corpus}: {} regions, {disagree} disagreements",
        out.lines().filter(|l| l.starts_with("region")).count()
    );
    // the kernel's answers are the exact rule's, ties and all
    assert_eq!(disagree, 0, "{corpus}: the kernel and the Lean rule disagree");
    static FILE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _one = FILE.lock().unwrap_or_else(|e| e.into_inner());
    let have = std::fs::read_to_string(DIGEST_PATH).unwrap_or_default();
    let mut lines: Vec<String> = have
        .lines()
        .filter(|l| l.split(' ').next() != Some(corpus))
        .map(str::to_string)
        .collect();
    lines.push(format!("{key} {:016x} {disagree}-disagree", fnv(&out)));
    lines.sort();
    std::fs::write(DIGEST_PATH, lines.join("\n") + "\n").expect("write the digest");
}

#[test]
fn holds_against_the_exact_rule() {
    let mut input = String::new();
    let mut regions = 0;
    let mut rng = Rng::new(0x1ea_d1ff);
    for kind in [
        Kind::OverlapLines,
        Kind::Isolated,
        Kind::Boundary,
        Kind::Tangent,
        Kind::Coincident,
        Kind::Mix,
        Kind::Nudged,
    ] {
        let runs: Vec<(usize, u64)> = if kind == Kind::OverlapLines {
            (0..40).map(|i| ([4, 8, 16, 32][i % 4], 100 + i as u64)).collect()
        } else {
            vec![(4, 1), (8, 2), (16, 3), (32, 4)]
        };
        for (n, seed) in runs {
            let case = cases::case(kind, n, seed);
            let Ok(region) = subtract(&case.wall, &case.cuts) else {
                continue;
            };
            let mut s = format!("region {}-{n}-{seed}\n", kind.name());
            let mut ok = true;
            for c in &region.parts {
                s.push_str("comp\n");
                ok &= lp(&mut s, &c.outer);
                for hl in &c.holes {
                    ok &= lp(&mut s, hl);
                }
            }
            if !ok {
                continue;
            }
            // random points, and points just off every vertex
            let mut qs: Vec<Point2> = (0..400)
                .map(|_| Point2::new(rng.range(-5.0, W + 5.0), rng.range(-5.0, H + 5.0)))
                .collect();
            for c in &region.parts {
                for b in c.outer.iter().chain(c.holes.iter().flatten()) {
                    for d in [1e-6, 1e-3] {
                        let a = rng.range(0.0, forge_math::dmath::TAU);
                        let (sn, cs) = forge_math::dmath::sin_cos(a);
                        qs.push(Point2::new(b.start.x + d * cs, b.start.y + d * sn));
                    }
                }
            }
            // just off every piece's middle, on both sides
            for c in &region.parts {
                for b in c.outer.iter().chain(c.holes.iter().flatten()) {
                    let (m, t) = (b.piece.at(0.5), b.piece.tangent(0.5));
                    let l = (t.x * t.x + t.y * t.y).sqrt().max(1e-300);
                    for d in [1e-9, 1e-6, -1e-9, -1e-6] {
                        qs.push(Point2::new(m.x - t.y / l * d, m.y + t.x / l * d));
                    }
                }
            }
            // exact ties: level with every vertex (on it, and off it along x, where the ray from
            // the point runs through it), and each arc's centre, and level with it on its circle
            // and past it
            for c in &region.parts {
                for b in c.outer.iter().chain(c.holes.iter().flatten()) {
                    for dx in [0.0, 1.0, -1.0, 7.5, -7.5] {
                        qs.push(Point2::new(b.start.x + dx, b.start.y));
                    }
                    if let Piece::Arc { c: o, r, .. } = b.piece {
                        for dx in [0.0, r, -r, 2.0 * r, -2.0 * r] {
                            qs.push(Point2::new(o.x + dx, o.y));
                        }
                        qs.push(Point2::new(o.x, o.y + r));
                        qs.push(Point2::new(o.x, o.y - r));
                    }
                }
            }
            for q in qs {
                let k = region.parts.iter().any(|c| holds(c, q));
                let _ = writeln!(s, "query {} {} {}", h(q.x), h(q.y), u8::from(k));
            }
            s.push_str("end\n");
            input.push_str(&s);
            regions += 1;
        }
    }
    assert!(regions >= 30, "too few regions without splines: {regions}");
    oracle("regions", &input, regions);
}
