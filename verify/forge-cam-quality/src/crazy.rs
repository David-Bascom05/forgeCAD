//! No crazy paths: bounds that no sane toolpath comes near, whatever the user set (feeds,
//! engagement, step-downs and tools are the user's; these are not cycle-time targets). Each built
//! op keeps within:
//! - air: at most 60 % of a cutting move's length in air at the cutting feed (links, hops and
//!   plunges are not cutting moves), once over 50 mm;
//! - re-cut: at most 25 % passing again, at the cutting feed, over stock the op itself cleared
//!   (the tool's axis stood in stock when the op began), once over 50 mm;
//! - links: at most twice the cutting length, or 500 mm;
//! - rapids: at most the cutting length, or 300 mm;
//! - retracts: at most one per 10 mm cut, or 20.
//!
//! Checked over every benchmark part by tests/guards.rs (in its one measure of them) and over every
//! machined example by tests/crazy.rs (a corpus guard per document). Rapids or link-feed moves into
//! material, the engagement cap, stock left and micro-moves have their own guards. The ops that
//! break a bound today are in KNOWN with the value when listed: any other, or a listed one more
//! than 10 % worse, fails; a fixed one must come off the list.
use crate::OpQ;

/// (document stem, op, rule, value when listed, why and owner).
pub const KNOWN: &[(&str, &str, &str, f64, &str)] = &[
    (
        "manifold_cam",
        "crn",
        "air",
        0.995,
        "pocket: 86 mm cut in 17 m of cutting moves (CAM-2D-FINISH)",
    ),
    (
        "manifold_cam",
        "crn",
        "links",
        2963.0,
        "pocket: 3 m of links for 86 mm cut (CAM-2D-FINISH)",
    ),
    (
        "manifold_cam",
        "shal",
        "air",
        0.76,
        "pocket: the shallow pocket's passes mostly over cleared floor (CAM-2D-FINISH)",
    ),
    (
        "manifold_cam",
        "grv",
        "recut",
        0.33,
        "slot: a third of the groove's length passes again over what it cut (CAM-2D-FINISH)",
    ),
    (
        "rptu1_clear_cam",
        "nut",
        "recut",
        0.44,
        "clear round the nut: 44 % of its length again over what it cut (CAM-CORE)",
    ),
    (
        "rptu1_clear_cam",
        "nut",
        "rapids",
        593.0,
        "clear round the nut: 593 mm of rapids for 178 mm cut (CAM-CORE)",
    ),
];

/// Each bound an op breaks: (rule, value, limit).
pub fn broken(q: &OpQ) -> Vec<(&'static str, f64, f64)> {
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

/// What a document's ops break against KNOWN: (failures, every bound broken). A failure is a
/// violation not listed, a listed one more than 10 % worse, or a listed one that no longer happens.
pub fn check(stem: &str, ops: &[OpQ]) -> (Vec<String>, Vec<String>) {
    let mut bad = Vec::new();
    let mut table = Vec::new();
    for q in ops {
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
    for k in KNOWN.iter().filter(|k| k.0 == stem) {
        if !table
            .iter()
            .any(|t| t.starts_with(&format!("{}/{} {}:", k.0, k.1, k.2)))
        {
            bad.push(format!(
                "{}/{} {}: no longer broken, take it off crazy::KNOWN",
                k.0, k.1, k.2
            ));
        }
    }
    (bad, table)
}
