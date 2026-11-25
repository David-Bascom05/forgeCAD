//! Guards over every benchmark part, for every strategy:
//! - chip load: no cutting sample's chip (hex) is over its op's target by more than 10 %. A feed
//!   error breaks tools and no geometric certifier sees it (a link at its link feed grazing stock
//!   reached 12 x fz before this guard; a sample given the air feed while cutting, 3 x).
//! - equal work: the strategies of one part (`NAME_STRATEGY_cam.fg`, the same NAME) remove the
//!   same volume within 0.5 %, so a time compared between them compares the same job (adaptive
//!   once left 12 % of a level while its time was compared with clear's). Pairs that differ by
//!   design are listed with the reason.
//! - nothing fast meets material: no rapid piece and no feed piece at the link feed (f >=
//!   FAST_FEED) meets material in the quality model, on any part. A hop's ramp once grazed a skin
//!   finer than its map's cell at F9999 (hex 12 x fz); its map is conservative now, and this guard
//!   keeps every map and link rule honest without slowing any descent.
//! - no micro-moves: every adaptive op's mean move (all its moves, links and rapids too) is at
//!   least the tool's radius and at least the mean move of clear on the same part (adaptive was
//!   once posted as its 0.75 mm simulation steps: 5,348 blocks where clear has 1,171).
//! - the engagement cap: an adaptive op's level cutting (its moves at one height; entries, which
//!   change height, cut a full slot by design) engages the tool no more than its `engagement:`
//!   cap (60° when not given) plus 5° (the measure's rounding). The ops that break it today are
//!   listed (`CAP_KNOWN`) with their length over the cap and why: another op over its cap fails,
//!   as does a listed one got worse; a fix takes its entry off. (Main posted full slotting under
//!   a 40° cap, unreported, until forge-cam-lang's own warning and this guard.) Measured on the
//!   chip-load guard's own run, so it costs no measure of its own.
use forge_cam_quality::{Config, JobQ, measure};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Parts whose strategies cut different regions by design.
const UNEQUAL: &[(&str, &str)] = &[
    (
        "rptu4",
        "closed pockets: clear clears them, adaptive does not take region \"pockets\"",
    ),
    (
        "rptu5",
        "closed pockets: clear clears them, adaptive does not take region \"pockets\"",
    ),
];

/// Why every listed adaptive op breaks its cap today (2026-11-25, main 50af9c42): the end-of-level
/// cleanup rings (plain offsets half a diameter apart: 90° by construction) and passes that stall
/// against a stock corner, leaving their material to the rings. Owners: the pass start, CAM-CORE;
/// the rings (engagement-limited), TOOLPATH-QUALITY.
const CAP_WHY: &str =
    "rings at half a diameter and stalled passes (CAM-CORE pass start; TOOLPATH-QUALITY rings)";

/// (part, op, mm of level cutting over the cap when listed, why). Each comes off with its fix.
/// Lengths along each move as the measure walks it (a ring posted as one whole-circle arc counts
/// its whole circumference; deep_islands and rptu4 were first listed from the shorter way round).
const CAP_KNOWN: &[(&str, &str, f64, &str)] = &[
    ("boss_adaptive_cam", "rough", 662.0, CAP_WHY),
    ("deep_boss_n_adaptive_cam", "rough", 893.0, CAP_WHY),
    ("deep_boss_n_zigzag_cam", "rough", 799.0, CAP_WHY),
    ("deep_boss_p_adaptive_cam", "rough", 893.0, CAP_WHY),
    ("deep_boss_p_zigzag_cam", "rough", 799.0, CAP_WHY),
    ("deep_islands_p_adaptive_cam", "rough", 538.0, CAP_WHY),
    ("deep_islands_p_zigzag_cam", "rough", 478.0, CAP_WHY),
    ("deep_open_slot_p_adaptive_cam", "rough", 408.0, CAP_WHY),
    ("deep_open_slot_p_zigzag_cam", "rough", 328.0, CAP_WHY),
    ("rptu3_adaptive_cam", "step", 440.0, CAP_WHY),
    ("rptu3_zigzag_cam", "step", 441.0, CAP_WHY),
    ("rptu4_adaptive_cam", "step", 772.0, CAP_WHY),
    ("rptu4_zigzag_cam", "step", 744.0, CAP_WHY),
    ("rptu5_adaptive_cam", "step", 617.0, CAP_WHY),
    ("rptu5_zigzag_cam", "step", 636.0, CAP_WHY),
    ("rptu6_adaptive_cam", "step", 440.0, CAP_WHY),
    ("rptu6_zigzag_cam", "step", 441.0, CAP_WHY),
];

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

/// The engagement-cap guard over measured jobs: what fails it (an unlisted op over its cap, or a
/// listed one worse than its figure), and every op's figure.
fn cap_violations(jobs: &[JobQ]) -> (Vec<String>, Vec<String>) {
    let (mut bad, mut table) = (Vec::new(), Vec::new());
    for j in jobs {
        let part = PathBuf::from(&j.file)
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let src = std::fs::read_to_string(&j.file).unwrap();
        for (op, cap) in adaptive_caps(&src) {
            let over: f64 = j
                .ops
                .iter()
                .filter(|q| q.name == op)
                .flat_map(|q| q.moves_q.iter())
                .filter(|m| m.level && m.phi > cap + 5.0)
                .map(|m| m.len_xy)
                .sum();
            table.push(format!("{part}/{op} cap {cap}°: {over:.0} mm over"));
            match CAP_KNOWN.iter().find(|k| k.0 == part && k.1 == op) {
                None if over > 0.5 => bad.push(format!(
                    "{part}/{op}: {over:.0} mm of level cutting over its {cap}° cap (not a known violation)"
                )),
                Some(&(_, _, was, why)) if over > was * 1.02 + 1.0 => bad.push(format!(
                    "{part}/{op}: {over:.0} mm over its {cap}° cap, worse than the {was:.0} mm listed ({why})"
                )),
                _ => {}
            }
        }
    }
    (bad, table)
}

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

fn measure_all() -> Vec<JobQ> {
    let cfg = Config {
        cell: Some(0.1),
        machine: None,
        feeds: Vec::new(),
        lookahead: None,
    };
    let fs = files();
    // WASI has no threads: the same measures, one after another
    if cfg!(target_family = "wasm") {
        return fs.iter().map(|f| measure(f, &cfg)).collect();
    }
    std::thread::scope(|s| {
        let hs: Vec<_> = fs.iter().map(|f| s.spawn(|| measure(f, &cfg))).collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    })
}

#[test]
fn chip_load_and_equal_work() {
    let jobs = measure_all();
    // the engagement cap, on the same measure (asserted last, so the others still report)
    let (cap_bad, cap_table) = cap_violations(&jobs);
    let mut over = Vec::new();
    let mut fast = Vec::new();
    // (part, strategy) -> removed, for the parts named NAME_STRATEGY_cam
    let mut work: BTreeMap<String, Vec<(String, f64)>> = BTreeMap::new();
    // part -> (strategy, op, mean move, tool radius), the clearing ops
    let mut means: BTreeMap<String, Vec<(String, String, f64, f64)>> = BTreeMap::new();
    for j in &jobs {
        let stem = PathBuf::from(&j.file)
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert!(j.errors.is_empty(), "{stem}: {:?}", j.errors);
        for q in &j.ops {
            if q.rapid_cuts > 0 || q.fast_cuts > 0 {
                fast.push(format!(
                    "{stem}/{}: {} rapid and {} link-feed pieces meet material (first at {:?})",
                    q.name, q.rapid_cuts, q.fast_cuts, q.fast_cut_at
                ));
            }
            if q.fz_target > 0.0 && q.hex_max > 1.10 * q.fz_target {
                over.push(format!(
                    "{stem}/{}: hex_max {:.4} against fz {:.4}",
                    q.name, q.hex_max, q.fz_target
                ));
            }
        }
        let name = stem.trim_end_matches("_cam");
        for strat in ["_clear", "_adaptive", "_zigzag"] {
            if let Some(part) = name.strip_suffix(strat) {
                let removed: f64 = j.ops.iter().map(|q| q.removed).sum();
                work.entry(part.to_string())
                    .or_default()
                    .push((strat[1..].to_string(), removed));
                for q in j.ops.iter().filter(|q| q.kind == "adaptive" || q.kind == "clear") {
                    let len = q.len_cut + q.len_air + q.len_rapid;
                    means.entry(part.to_string()).or_default().push((
                        q.kind.clone(),
                        q.name.clone(),
                        len / (q.moves.max(1) as f64),
                        0.5 * q.d,
                    ));
                }
            }
        }
    }
    assert!(over.is_empty(), "chip load over its target:\n{}", over.join("\n"));
    assert!(fast.is_empty(), "fast moves meet material:\n{}", fast.join("\n"));
    let mut unequal = Vec::new();
    for (part, v) in &work {
        if v.len() < 2 || UNEQUAL.iter().any(|(p, _)| p == part) {
            continue;
        }
        let lo = v.iter().map(|x| x.1).fold(f64::INFINITY, f64::min);
        let hi = v.iter().map(|x| x.1).fold(0.0, f64::max);
        if hi > lo * 1.005 {
            unequal.push(format!("{part}: {v:?}"));
        }
    }
    assert!(
        unequal.is_empty(),
        "strategies of one part remove different volumes:\n{}",
        unequal.join("\n")
    );
    let mut micro = Vec::new();
    for (part, v) in &means {
        let clear = v
            .iter()
            .filter(|x| x.0 == "clear")
            .map(|x| x.2)
            .fold(0.0, f64::max);
        for (kind, name, mean, r) in v.iter().filter(|x| x.0 == "adaptive") {
            if *mean < *r || *mean < clear {
                micro.push(format!(
                    "{part}/{name} ({kind}): mean move {mean:.2} mm, r {r}, clear's {clear:.2}"
                ));
            }
        }
    }
    assert!(
        micro.is_empty(),
        "adaptive posts short moves:\n{}",
        micro.join("\n")
    );
    assert!(
        cap_bad.is_empty(),
        "adaptive over its engagement cap:\n{}\n\nall:\n{}",
        cap_bad.join("\n"),
        cap_table.join("\n")
    );
}

/// A ratchet on micro-moves: in every adaptive op, at most 12 % of the cutting moves (cut at one
/// level, or arcs) under 1 mm on the deep parts, at most a quarter on the RPTU parts (4 mm levels:
/// their passes are short; 21-24 % as of 2026-11-24, the deep parts at most 11.4 %).
#[test]
fn adaptive_cutting_moves_not_micro() {
    use forge_cam::ops::{PMove, Phase};
    let mut bad = Vec::new();
    for f in files() {
        let src = std::fs::read_to_string(&f).unwrap();
        if !src.contains("adaptive(") {
            continue;
        }
        let cam = forge_cam_lang::run(&src, f.parent().unwrap());
        for op in cam.ops.iter().filter(|o| o.kind == "adaptive") {
            let (mut n, mut short) = (0usize, 0usize);
            for p in &op.paths {
                let mut at = p.start;
                for m in &p.moves {
                    let (to, len) = match *m {
                        PMove::Line { to, phase, .. } => {
                            let l = ((to[0] - at[0]).powi(2) + (to[1] - at[1]).powi(2)).sqrt();
                            (
                                to,
                                if phase == Phase::Cut && (to[2] - at[2]).abs() < 1e-9 {
                                    Some(l)
                                } else {
                                    None
                                },
                            )
                        }
                        PMove::Arc { to, c, .. } => {
                            // the chord bounds the arc's length from below
                            let _ = c;
                            (
                                to,
                                Some(((to[0] - at[0]).powi(2) + (to[1] - at[1]).powi(2)).sqrt()),
                            )
                        }
                        PMove::Rapid { to } => (to, None),
                        _ => continue,
                    };
                    if let Some(l) = len {
                        n += 1;
                        if l < 1.0 {
                            short += 1;
                        }
                    }
                    at = to;
                }
            }
            let rptu = f.parent().is_some_and(|d| d.ends_with("rptu"));
            let limit = if rptu { 0.25 } else { 0.12 };
            if n > 0 && short as f64 > limit * n as f64 {
                bad.push(format!(
                    "{}/{}: {short} of {n} cutting moves under 1 mm (limit {:.0} %)",
                    f.display(),
                    op.name,
                    100.0 * limit
                ));
            }
        }
    }
    assert!(bad.is_empty(), "adaptive posts micro-moves:\n{}", bad.join("\n"));
}
