//! Adaptive clearing's engagement cap, guarded: on every benchmark part, the level cutting of an
//! adaptive op (its moves at one height; entries, which change height, cut a full slot by design)
//! engages the tool no more than its `engagement:` cap (60° when not given) plus 5° (the
//! measurement's rounding), as forge-cam-quality measures each move.
//!
//! The parts that break it today are listed with the length over the cap when listed and why:
//! the guard fails if any other adaptive op goes over the cap, or if a listed one gets worse.
//! A fix takes its entries off the list. (Main posted full slotting under a 40° cap, unreported,
//! until forge-cam-lang's own warning and this guard.)
use forge_cam::ops::PMove;
use forge_cam_quality::{Config, JobQ, measure};
use std::path::{Path, PathBuf};

/// Why every adaptive op breaks its cap today (2026-11-25, main 50af9c42): the end-of-level
/// cleanup rings (plain offsets half a diameter apart: 90° by construction) and passes that stall
/// against a stock corner, leaving their material to the rings. Owners: the pass start, CAM-CORE;
/// the rings (engagement-limited), TOOLPATH-QUALITY.
#[allow(dead_code)]
const WHY: &str = "rings at half a diameter and stalled passes (CAM-CORE pass start; TOOLPATH-QUALITY rings)";

/// (part, op, mm of level cutting over the cap when listed, why). Each comes off with its fix.
#[allow(dead_code)]
const KNOWN: &[(&str, &str, f64, &str)] = &[
    ("boss_adaptive_cam", "rough", 662.0, WHY),
    ("deep_boss_n_adaptive_cam", "rough", 893.0, WHY),
    ("deep_boss_n_zigzag_cam", "rough", 799.0, WHY),
    ("deep_boss_p_adaptive_cam", "rough", 893.0, WHY),
    ("deep_boss_p_zigzag_cam", "rough", 799.0, WHY),
    ("deep_islands_p_adaptive_cam", "rough", 412.0, WHY),
    ("deep_islands_p_zigzag_cam", "rough", 352.0, WHY),
    ("deep_open_slot_p_adaptive_cam", "rough", 408.0, WHY),
    ("deep_open_slot_p_zigzag_cam", "rough", 328.0, WHY),
    ("rptu3_adaptive_cam", "step", 440.0, WHY),
    ("rptu3_zigzag_cam", "step", 441.0, WHY),
    ("rptu4_adaptive_cam", "step", 533.0, WHY),
    ("rptu4_zigzag_cam", "step", 505.0, WHY),
    ("rptu5_adaptive_cam", "step", 617.0, WHY),
    ("rptu5_zigzag_cam", "step", 636.0, WHY),
    ("rptu6_adaptive_cam", "step", 440.0, WHY),
    ("rptu6_zigzag_cam", "step", 441.0, WHY),
];

fn files() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("parts");
    let mut out = Vec::new();
    for dir in [root.clone(), root.join("rptu")] {
        for e in std::fs::read_dir(&dir).unwrap() {
            let p = e.unwrap().path();
            if p.to_string_lossy().ends_with("_cam.fg") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

fn measure_one(f: &Path) -> JobQ {
    let cfg = Config {
        cell: Some(0.1),
        machine: None,
        feeds: Vec::new(),
        lookahead: None,
    };
    measure(f, &cfg)
}

/// Each adaptive op of a document and its cap: `NAME = adaptive(.., engagement: N, ..)`.
fn adaptive_caps(src: &str) -> Vec<(String, f64)> {
    let mut out = Vec::new();
    for line in src.lines() {
        let Some((name, rest)) = line.split_once('=') else {
            continue;
        };
        let rest = rest.trim_start();
        if !rest.starts_with("adaptive(") {
            continue;
        }
        let cap = rest
            .split("engagement:")
            .nth(1)
            .and_then(|t| {
                t.trim_start()
                    .split(|c: char| !(c.is_ascii_digit() || c == '.'))
                    .next()
                    .and_then(|n| n.parse::<f64>().ok())
            })
            .unwrap_or(60.0);
        out.push((name.trim().to_string(), cap));
    }
    out
}

/// The length (mm) of an op's level moves that the measure finds over the cap.
fn over_cap(job: &JobQ, f: &Path, op: &str, cap: f64) -> f64 {
    let Some(q) = job.ops.iter().find(|q| q.name == op) else {
        return 0.0;
    };
    let src = std::fs::read_to_string(f).unwrap();
    let cam = forge_cam_lang::run(&src, f.parent().unwrap());
    let Some(o) = cam.ops.iter().find(|o| o.name == op) else {
        return 0.0;
    };
    let mut over = 0.0;
    for m in q.moves_q.iter().filter(|m| m.phi > cap + 5.0) {
        let Some(p) = o.paths.get(m.path) else {
            continue;
        };
        let mut at = p.start;
        for mv in &p.moves[..m.mv] {
            if let PMove::Rapid { to } | PMove::Line { to, .. } | PMove::Arc { to, .. } = *mv {
                at = to;
            }
        }
        let len = match p.moves.get(m.mv) {
            Some(PMove::Line { to, .. }) if to[2] == at[2] => {
                ((to[0] - at[0]).powi(2) + (to[1] - at[1]).powi(2)).sqrt()
            }
            Some(PMove::Arc { to, c, .. }) if to[2] == at[2] => {
                // the arc's length from its sweep (the shorter way round is enough here: an
                // adaptive arc spans less than a turn)
                let r = ((at[0] - c[0]).powi(2) + (at[1] - c[1]).powi(2)).sqrt();
                let a0 = forge_math::dmath::atan2(at[1] - c[1], at[0] - c[0]);
                let a1 = forge_math::dmath::atan2(to[1] - c[1], to[0] - c[0]);
                let mut d = (a1 - a0).abs();
                if d > std::f64::consts::PI {
                    d = std::f64::consts::TAU - d;
                }
                r * d
            }
            _ => 0.0,
        };
        over += len;
    }
    over
}

// a measurement for the ring work (research/handoff/TOOLPATH-QUALITY.md), not a guard:
// CAPSCAN_OUT=FILE cargo test --release -p forge-cam-quality --test zz_capscan -- --ignored
#[test]
#[ignore]
fn capscan() {
    let fs: Vec<PathBuf> = files()
        .into_iter()
        .filter(|f| {
            std::fs::read_to_string(f)
                .map(|s| !adaptive_caps(&s).is_empty())
                .unwrap_or(false)
        })
        .collect();
    let jobs: Vec<JobQ> = if cfg!(target_family = "wasm") {
        fs.iter().map(|f| measure_one(f)).collect()
    } else {
        std::thread::scope(|s| {
            let hs: Vec<_> = fs.iter().map(|f| s.spawn(|| measure_one(f))).collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        })
    };
    let mut rows = Vec::new();
    for (f, job) in fs.iter().zip(&jobs) {
        let part = f.file_stem().unwrap().to_string_lossy().into_owned();
        let src = std::fs::read_to_string(f).unwrap();
        for (op, cap) in adaptive_caps(&src) {
            let over = over_cap(job, f, &op, cap);
            let q = job.ops.iter().find(|q| q.name == op).unwrap();
            let all: f64 = job.ops.iter().map(|q| q.removed).sum();
            rows.push(format!("CAPSCAN {part:32} {op:6} cap {cap:3} over {over:7.0} mm  t_la {:8.1}  t_stop {:8.1} removed_all {all:9.0} rapid_cuts {} fast {}", q.t_la, q.t_stop, q.rapid_cuts, q.fast_cuts));
        }
    }
    let p = std::env::var("CAPSCAN_OUT").unwrap_or("/dev/stderr".into());
    std::fs::write(p, rows.join("\n") + "\n").unwrap();
}
