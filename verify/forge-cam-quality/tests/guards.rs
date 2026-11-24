//! Two guards over every benchmark part, for every strategy:
//! - chip load: no cutting sample's chip (hex) is over its op's target by more than 10 %. A feed
//!   error breaks tools and no geometric certifier sees it (a link at its link feed grazing stock
//!   reached 12 x fz before this guard; a sample given the air feed while cutting, 3 x).
//! - equal work: the strategies of one part (`NAME_STRATEGY_cam.fg`, the same NAME) remove the
//!   same volume within 0.5 %, so a time compared between them compares the same job (adaptive
//!   once left 12 % of a level while its time was compared with clear's). Pairs that differ by
//!   design are listed with the reason.
use forge_cam_quality::{Config, JobQ, measure};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Parts whose strategies cut different regions by design.
const UNEQUAL: &[(&str, &str)] = &[
    ("rptu4", "closed pockets: clear clears them, adaptive does not take region \"pockets\""),
    ("rptu5", "closed pockets: clear clears them, adaptive does not take region \"pockets\""),
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

fn measure_all() -> Vec<JobQ> {
    let cfg = Config {
        cell: Some(0.1),
        machine: None,
        feeds: Vec::new(),
        lookahead: None,
    };
    let fs = files();
    std::thread::scope(|s| {
        let hs: Vec<_> = fs.iter().map(|f| s.spawn(|| measure(f, &cfg))).collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    })
}

#[test]
fn chip_load_and_equal_work() {
    let jobs = measure_all();
    let mut over = Vec::new();
    // (part, strategy) -> removed, for the parts named NAME_STRATEGY_cam
    let mut work: BTreeMap<String, Vec<(String, f64)>> = BTreeMap::new();
    for j in &jobs {
        let stem = PathBuf::from(&j.file).file_stem().unwrap().to_string_lossy().into_owned();
        assert!(j.errors.is_empty(), "{stem}: {:?}", j.errors);
        for q in &j.ops {
            if q.fz_target > 0.0 && q.hex_max > 1.10 * q.fz_target {
                over.push(format!("{stem}/{}: hex_max {:.4} against fz {:.4}", q.name, q.hex_max, q.fz_target));
            }
        }
        let name = stem.trim_end_matches("_cam");
        for strat in ["_clear", "_adaptive", "_zigzag"] {
            if let Some(part) = name.strip_suffix(strat) {
                let removed: f64 = j.ops.iter().map(|q| q.removed).sum();
                work.entry(part.to_string()).or_default().push((strat[1..].to_string(), removed));
            }
        }
    }
    assert!(over.is_empty(), "chip load over its target:\n{}", over.join("\n"));
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
    assert!(unequal.is_empty(), "strategies of one part remove different volumes:\n{}", unequal.join("\n"));
}
