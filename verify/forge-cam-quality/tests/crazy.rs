//! No crazy paths: on every benchmark part and every machined example, each built op keeps within
//! bounds that no sane toolpath comes near, whatever the user set (feeds, engagement, step-downs
//! and tools are the user's; these are not cycle-time targets):
//! - air: at most 60 % of a cutting move's length in air at the cutting feed (links, hops and
//!   plunges are not cutting moves), once over 50 mm;
//! - re-cut: at most 25 % passing again, at the cutting feed, over stock the op itself cleared
//!   (the tool's axis stood in stock when the op began), once over 50 mm;
//! - links: at most twice the cutting length, or 500 mm;
//! - rapids: at most the cutting length, or 300 mm;
//! - retracts: at most one per 10 mm cut, or 20.
//!
//! Rapids or link-feed moves into material, the engagement cap, stock left and micro-moves have
//! their own guards (guards.rs, engagement_cap.rs). The ops that break a bound today are listed
//! with the value when listed: the guard fails if any other op breaks one, or a listed one gets
//! more than 10 % worse. A fix takes its entry off the list.
use forge_cam_quality::{Config, JobQ, OpQ, measure};
use std::path::PathBuf;

/// (document stem, op, rule, value when listed, why).
const KNOWN: &[(&str, &str, &str, f64, &str)] = &[
    (
        "manifold_cam",
        "crn",
        "air",
        0.995,
        "pocket: 86 mm cut in 17 m of cutting moves (the corner rest pocket, 2026-11-25)",
    ),
    (
        "manifold_cam",
        "crn",
        "links",
        2963.0,
        "pocket: 3 m of links for 86 mm cut",
    ),
    (
        "manifold_cam",
        "shal",
        "air",
        0.76,
        "pocket: the shallow pocket's passes mostly over cleared floor",
    ),
    (
        "manifold_cam",
        "grv",
        "recut",
        0.33,
        "slot: a third of the groove's length passes again over what it cut",
    ),
    (
        "mold_core_cam",
        "left",
        "air",
        0.76,
        "rest machining: most of its length in air",
    ),
    (
        "mold_core_cam",
        "left",
        "retracts",
        35.0,
        "rest machining: 35 retracts for 85 mm cut",
    ),
    (
        "rptu1_clear_cam",
        "nut",
        "recut",
        0.44,
        "clear round the nut: 44 % of its length again over what it cut",
    ),
    (
        "rptu1_clear_cam",
        "nut",
        "rapids",
        593.0,
        "clear round the nut: 593 mm of rapids for 178 mm cut",
    ),
];

fn files() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let parts = root.join("parts");
    let examples = root.join("../../examples/machined");
    let mut out = Vec::new();
    for dir in [parts.clone(), parts.join("rptu"), examples] {
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

/// Each bound an op breaks: (rule, value, limit).
fn broken(q: &OpQ) -> Vec<(&'static str, f64, f64)> {
    let mut out = Vec::new();
    if q.status != "built" || q.len_cut <= 0.0 {
        return out;
    }
    let moving = q.len_cut + q.len_air_cut;
    if q.len_air_cut > 50.0 && q.len_air_cut > 0.60 * moving {
        out.push(("air", q.len_air_cut / moving, 0.60));
    }
    if q.len_recut > 50.0 && q.len_recut > 0.25 * moving {
        out.push(("recut", q.len_recut / moving, 0.25));
    }
    let links = (2.0 * q.len_cut).max(500.0);
    if q.len_link > links {
        out.push(("links", q.len_link, links));
    }
    let rapids = q.len_cut.max(300.0);
    if q.len_rapid > rapids {
        out.push(("rapids", q.len_rapid, rapids));
    }
    let retracts = (q.len_cut / 10.0).max(20.0);
    if q.retracts as f64 > retracts {
        out.push(("retracts", q.retracts as f64, retracts));
    }
    out
}

#[test]
fn no_crazy_paths() {
    let cfg = Config {
        cell: Some(0.1),
        machine: None,
        feeds: Vec::new(),
        lookahead: None,
    };
    let fs = files();
    let jobs: Vec<JobQ> = if cfg!(target_family = "wasm") {
        fs.iter().map(|f| measure(f, &cfg)).collect()
    } else {
        std::thread::scope(|s| {
            let hs: Vec<_> = fs.iter().map(|f| s.spawn(|| measure(f, &cfg))).collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        })
    };
    let mut bad = Vec::new();
    let mut table = Vec::new();
    for (f, j) in fs.iter().zip(&jobs) {
        let stem = f.file_stem().unwrap().to_string_lossy().into_owned();
        // (an op refused by its document is not measured: `broken` reads built ops only)
        for q in &j.ops {
            for (rule, v, limit) in broken(q) {
                table.push(format!("{stem}/{} {rule}: {v:.2} (limit {limit:.2})", q.name));
                match KNOWN.iter().find(|k| k.0 == stem && k.1 == q.name && k.2 == rule) {
                    None => bad.push(format!(
                        "{stem}/{} ({}): {rule} {v:.2} over its bound {limit:.2} (not a known violation)",
                        q.name, q.kind
                    )),
                    Some(&(_, _, _, was, why)) if v > was * 1.10 => bad.push(format!(
                        "{stem}/{}: {rule} {v:.2}, worse than the {was:.2} listed ({why})",
                        q.name
                    )),
                    _ => {}
                }
            }
        }
    }
    // a listed violation that no longer happens: its entry comes off
    let stale: Vec<String> = KNOWN
        .iter()
        .filter(|k| {
            !table
                .iter()
                .any(|t| t.starts_with(&format!("{}/{} {}:", k.0, k.1, k.2)))
        })
        .map(|k| format!("{}/{} {}: no longer broken, take it off KNOWN", k.0, k.1, k.2))
        .collect();
    bad.extend(stale);
    assert!(bad.is_empty(), "{}\n\nall:\n{}", bad.join("\n"), table.join("\n"));
}
