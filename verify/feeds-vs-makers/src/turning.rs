//! Turning: our insert presets against the makers' grade tables (CAM-HAAS's raw/turn-vc rows,
//! each cited: Sumitomo, Tungaloy, Korloy, Ingersoll). For every (row, insert it covers) we pick
//! the insert's preset as an op does (`Insert::preset_for`) and compare cutting speed, feed per
//! revolution and depth of cut with the maker's start value (Sumitomo's optimum, Korloy's
//! recommended, else the low end of the printed range: the presets' convention):
//! - `own`: the row's grade and its very material text: must land inside the printed range;
//! - `graded`: the row's grade, no stock material named (the grade's lead row);
//! - `default`: no grade stated, no material (the slowest grade's lead row: the default path).
use forge_agent::json::Json;
use forge_tooling::Library;
use std::path::Path;

/// One path's ratios for a row: (vc, f where printed).
type PathRatio = dyn Fn(&TurnRow) -> Option<(f64, Option<f64>)>;

#[derive(Clone, Debug, PartialEq)]
pub struct TurnRow {
    pub maker: String,
    pub insert: String,
    pub group: String,
    pub grade: String,
    pub material: String,
    /// The maker's start value and printed range: vc m/min, f mm/rev, ap mm (None: not printed).
    pub vc: (f64, f64, f64),
    pub f: Option<(f64, f64, f64)>,
    pub ap: Option<(f64, f64, f64)>,
    /// Ours (vc, f, ap) per path; None where the insert has no preset for it.
    pub own: Option<(f64, f64, Option<f64>)>,
    pub graded: Option<(f64, f64, Option<f64>)>,
    pub default: Option<(f64, f64, Option<f64>)>,
    pub default_why: String,
    pub url: String,
}

/// (start, low, high) of a printed value.
fn printed(j: Option<&Json>) -> Option<(f64, f64, f64)> {
    let j = j?;
    let v: Vec<f64> = match j.get("v")? {
        Json::Num(x) => vec![*x],
        Json::Arr(a) => a.iter().filter_map(|x| x.num()).collect(),
        _ => return None,
    };
    let (lo, hi) = (
        v.iter().copied().fold(f64::INFINITY, f64::min),
        v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    );
    if !lo.is_finite() {
        return None;
    }
    let start = j
        .get("optimum")
        .or_else(|| j.get("recommended"))
        .or_else(|| j.get("nominal"))
        .and_then(|x| x.num())
        .unwrap_or(lo);
    Some((start, lo, hi))
}

pub fn rows(raw: &Path) -> Vec<TurnRow> {
    let lib = Library::builtin();
    let mut out = Vec::new();
    for maker in ["sumitomo", "tungaloy", "korloy", "ingersoll"] {
        let Ok(src) = std::fs::read_to_string(raw.join(format!("turn-vc/{maker}.json"))) else {
            continue;
        };
        let Ok(doc) = Json::parse(&src) else { continue };
        let Some(Json::Arr(rs)) = doc.get("rows") else {
            continue;
        };
        for r in rs {
            let s = |k: &str| r.get(k).and_then(|x| x.str()).unwrap_or("").to_string();
            let Some(vc) = printed(r.get("vc")) else { continue };
            let (group, grade, material) = (s("iso_group"), s("grade"), s("material_stated"));
            let Some(Json::Arr(ids)) = r.get("insert_ids") else {
                continue;
            };
            for id in ids.iter().filter_map(|x| x.str()) {
                let lid = format!("{maker}-{}", id.replace(' ', ""));
                let Some(ins) = lib.inserts.iter().find(|i| i.id == lid) else {
                    continue;
                };
                let run = |grade: Option<&str>, mat: Option<&str>| {
                    ins.preset_for(&group, grade, mat)
                        .ok()
                        .flatten()
                        .map(|p| (p.preset.vc, p.preset.fz, p.preset.ap))
                };
                // the row's own preset: its grade and its very material text
                let own_name = format!("{group} {} {grade} {material}", ins.chipbreaker);
                let own = ins
                    .presets
                    .iter()
                    .find(|p| p.name == own_name)
                    .map(|p| (p.vc, p.fz, p.ap));
                let dflt = ins.preset_for(&group, None, None).ok().flatten();
                out.push(TurnRow {
                    maker: maker.into(),
                    insert: lid.clone(),
                    group: group.clone(),
                    grade: grade.clone(),
                    material: material.clone(),
                    vc,
                    f: printed(r.get("f")),
                    ap: printed(r.get("ap")),
                    own,
                    graded: run(Some(&grade), None),
                    default: dflt.as_ref().map(|p| (p.preset.vc, p.preset.fz, p.preset.ap)),
                    default_why: dflt.map_or(String::new(), |p| p.why),
                    url: s("url"),
                });
            }
        }
    }
    out
}

impl TurnRow {
    /// Ours/theirs (vc, f) for a path; f None where the maker prints no feed in the row.
    pub fn ratio(&self, ours: Option<(f64, f64, Option<f64>)>) -> Option<(f64, Option<f64>)> {
        let (v, f, _) = ours?;
        Some((v / self.vc.0, self.f.map(|t| f / t.0)))
    }
    /// The row's own preset inside the printed ranges (vc; f and ap where printed).
    pub fn own_in_range(&self) -> Option<bool> {
        let (v, f, ap) = self.own?;
        let within = |x: f64, (_, lo, hi): (f64, f64, f64)| x >= lo * (1.0 - 1e-3) && x <= hi * (1.0 + 1e-3);
        Some(
            within(v, self.vc)
                && self.f.is_none_or(|t| within(f, t))
                && match (ap, self.ap) {
                    (Some(a), Some(t)) => within(a, t),
                    _ => true,
                },
        )
    }
}

/// The report section.
pub fn markdown(rows: &[TurnRow]) -> String {
    use crate::stats;
    let fmt = |s: Option<(f64, f64, f64, usize)>| match s {
        Some((m, lo, hi, n)) => format!("{m:.2} ({lo:.2}–{hi:.2}, n {n})"),
        None => "–".into(),
    };
    let own: Vec<&TurnRow> = rows.iter().filter(|r| r.own.is_some()).collect();
    let inr = own.iter().filter(|r| r.own_in_range() == Some(true)).count();
    let mut s = String::from("\n## Turning: insert presets against the makers' grade tables\n\n");
    let mut inserts: Vec<&str> = rows.iter().map(|r| r.insert.as_str()).collect();
    inserts.sort();
    inserts.dedup();
    s.push_str(&format!(
        "{} (row, insert) pairs from Sumitomo, Tungaloy, Korloy and Ingersoll grade tables (research/cam/tooling/raw/turn-vc), over {} inserts of the library. Theirs is the maker's start value (Sumitomo's optimum, Korloy's recommended, else the low end of the printed range: the presets' convention); ours is the insert's preset as `Insert::preset_for` picks it. Ratios ours/theirs, median (10th–90th, n).\n\n",
        rows.len(),
        inserts.len()
    ));
    s.push_str(&format!(
        "- Reproduction: with the row's grade and material, {inr} of {} pairs land inside the maker's printed range (vc; f and ap where printed).\n",
        own.len()
    ));
    let line = |name: &str, f: &PathRatio| {
        let r: Vec<(f64, Option<f64>)> = rows.iter().filter_map(f).collect();
        format!(
            "| {name} | {} | {} | {} |\n",
            r.len(),
            fmt(stats(r.iter().map(|x| x.0).collect())),
            fmt(stats(r.iter().filter_map(|x| x.1).collect()))
        )
    };
    s.push_str("\n| path | pairs | vc ours/theirs | f ours/theirs |\n|---|---|---|---|\n");
    s.push_str(&line("own (grade + material)", &|r| r.ratio(r.own)));
    s.push_str(&line("graded (grade, no material: the grade's lead row)", &|r| {
        r.ratio(r.graded)
    }));
    s.push_str(&line(
        "default (no grade, no material: the slowest grade's lead row)",
        &|r| r.ratio(r.default),
    ));
    let above = rows
        .iter()
        .filter_map(|r| r.ratio(r.default))
        .filter(|x| x.0 > 1.0 + 1e-9)
        .count();
    let n = rows.iter().filter(|r| r.default.is_some()).count();
    s.push_str(&format!(
        "\n- The default path is above the maker's start speed for the row in {above} of {n} pairs: a row of a slower material than the grade's lead row (the lead row is the maker's first printed material, e.g. Sumitomo's mild steel), or a slower grade's row when the insert's grade is not stated cannot be slower than the slowest grade's lead row.\n"
    ));
    s.push_str("\n| maker | pairs | default vc ours/theirs | default f ours/theirs |\n|---|---|---|---|\n");
    for m in ["sumitomo", "tungaloy", "korloy", "ingersoll"] {
        let sel: Vec<&TurnRow> = rows.iter().filter(|r| r.maker == m).collect();
        let r: Vec<(f64, Option<f64>)> = sel.iter().filter_map(|r| r.ratio(r.default)).collect();
        s.push_str(&format!(
            "| {m} | {} | {} | {} |\n",
            sel.len(),
            fmt(stats(r.iter().map(|x| x.0).collect())),
            fmt(stats(r.iter().filter_map(|x| x.1).collect()))
        ));
    }
    s.push_str("\n| group | pairs | default vc ours/theirs | default f ours/theirs |\n|---|---|---|---|\n");
    for g in ["P", "M", "K", "N", "S", "H"] {
        let sel: Vec<&TurnRow> = rows.iter().filter(|r| r.group == g).collect();
        if sel.is_empty() {
            continue;
        }
        let r: Vec<(f64, Option<f64>)> = sel.iter().filter_map(|r| r.ratio(r.default)).collect();
        s.push_str(&format!(
            "| {g} | {} | {} | {} |\n",
            sel.len(),
            fmt(stats(r.iter().map(|x| x.0).collect())),
            fmt(stats(r.iter().filter_map(|x| x.1).collect()))
        ));
    }
    s.push_str("\n- Not compared: Korloy's 9 inserts (no s/re in the raw extraction, so no records), inserts and groups with no printed vc (see raw/turn-vc `missing`), and K/S/H/N tables for chipbreakers the inserts do not have. A tool with no preset for the part's group gets no turning default: the op asks for vc and f (no generic turning layer yet).\n");
    s
}
