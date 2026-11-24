//! Our feeds and speeds against tool makers' published numbers (David: "sanity-check our speeds
//! and feeds against manufacturers' published numbers").
//!
//! The makers' rows are CAM-HAAS's raw extractions, each cited (research/cam/tooling/raw/*.json):
//! Harvey, Helical and Lakeshore end mills (speed and chip per material, op class, ae, ap) and
//! OSG, Guhring and YG-1 drills (speed, feed per revolution). For each row we run our resolver
//! with the maker's own preset OFF (the generic group table, the material class and the tool's
//! substrate only), at the maker's own engagement (ae, ap given as the user's), with no machine
//! clamps, and compare: spindle speed (the machine setting), programmed feed per tooth (or per
//! revolution for a drill) and the feed rate. Ratios are ours / theirs.
//!
//! Ranges are taken at their middle. The maker's material text is mapped to an ISO 513 group by
//! `iso_group` (a keyword table, listed in the report); rows it cannot class (plastics, rows
//! with no group) are counted and skipped.
pub mod report;

use forge_agent::json::Json;
use forge_tooling::Library;
use forge_tooling::feeds::Substrate;
use forge_tooling::resolve::{self, Ask, DrillAsk, DrillTool, Spindle, ToolData};
use std::path::Path;

/// One compared row.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub vendor: String,
    pub tool: String,
    /// flat | bull | ball | drill.
    pub kind: String,
    pub d: f64,
    pub z: u32,
    pub substrate: &'static str,
    pub material: String,
    pub group: &'static str,
    /// The maker's op class (slot, rough, finish, max, milling (general), drill).
    pub op: String,
    pub ae: Option<f64>,
    pub ap: Option<f64>,
    pub theirs_rpm: f64,
    pub ours_rpm: f64,
    /// mm/tooth (mm/rev for a drill); None where the maker states none.
    pub theirs_f: Option<f64>,
    pub ours_f: f64,
    /// Which of our layers set the speed and the feed.
    pub ours_src: String,
    /// The row's citation.
    pub url: String,
    /// Our default for this row was set from these very rows (a maker's own table, or the
    /// pooled median of the makers' rows): not an independent check.
    pub circular: bool,
}

impl Row {
    pub fn r_rpm(&self) -> f64 {
        self.ours_rpm / self.theirs_rpm
    }
    pub fn r_f(&self) -> Option<f64> {
        self.theirs_f.map(|t| self.ours_f / t)
    }
    pub fn r_vf(&self) -> Option<f64> {
        self.r_f().map(|f| f * self.r_rpm())
    }
}

/// The maker's material text → ISO 513 group: (keywords, group), first match wins, so the more
/// specific come first. Hardness classes over about 45 HRC are H; 36–50 HRC rows stay P (ISO 513
/// H starts at hardened steels, about 45 HRC; the row straddles it). Plastics: none (no ISO class).
pub const CLASSES: &[(&[&str], &str)] = &[
    (&["plastic"], ""),
    (&["> 50 hrc", "46 rc+", "hardened"], "H"),
    (&["stainless"], "M"),
    (&["cast iron"], "K"),
    (&["titanium", "monel", "high nickel", "inconel", "nickel alloy", "heat resistant", "superalloy"], "S"),
    (&["alumin", "magnesium", "zinc", "copper", "brass", "bronze"], "N"),
    (&["steel", "stl", "carbon"], "P"),
];

pub fn iso_group(text: &str) -> Option<&'static str> {
    let t = text.to_ascii_lowercase();
    for (keys, g) in CLASSES {
        if keys.iter().any(|k| t.contains(k)) {
            return if g.is_empty() { None } else { Some(g) };
        }
    }
    None
}

fn mid(v: Option<&Json>) -> Option<f64> {
    match v? {
        Json::Num(x) => Some(*x),
        Json::Arr(a) => {
            let xs: Vec<f64> = a.iter().filter_map(|x| x.num()).collect();
            if xs.is_empty() {
                None
            } else {
                Some(xs.iter().sum::<f64>() / xs.len() as f64)
            }
        }
        _ => None,
    }
}

fn val<'a>(j: &'a Json, k: &str) -> Option<&'a Json> {
    j.get(k)?.get("v")
}

/// What was skipped and why: (reason, count).
#[derive(Clone, Debug, Default)]
pub struct Skips(pub Vec<(String, usize)>);

impl Skips {
    fn add(&mut self, why: &str) {
        match self.0.iter_mut().find(|x| x.0 == why) {
            Some(x) => x.1 += 1,
            None => self.0.push((why.into(), 1)),
        }
    }
}

const FREE: Spindle<'static> = Spindle {
    rpm: [0.0, 1e12],
    max_feed: 0.0,
    power: &[],
};

/// All compared rows from the raw files under `raw`.
pub fn rows(raw: &Path) -> (Vec<Row>, Skips) {
    let lib = Library::builtin();
    let mut out = Vec::new();
    let mut skips = Skips::default();
    for vendor in ["harvey", "helical", "lakeshore", "osg", "guhring", "yg1"] {
        let Ok(src) = std::fs::read_to_string(raw.join(format!("{vendor}.json"))) else {
            skips.add(&format!("{vendor}.json not found"));
            continue;
        };
        let Ok(doc) = Json::parse(&src) else {
            skips.add(&format!("{vendor}.json does not parse"));
            continue;
        };
        let Some(Json::Arr(recs)) = doc.get("records") else { continue };
        for r in recs {
            let kind = r.get("kind").and_then(|k| k.str()).unwrap_or("").to_string();
            let vals = r.get("values").unwrap_or(&Json::Null);
            let Some(d) = val(vals, "dc").and_then(|x| x.num()) else { continue };
            let z = val(vals, "nof").and_then(|x| x.num()).unwrap_or(2.0) as u32;
            let mat = val(vals, "material").and_then(|x| x.str()).unwrap_or("");
            let sub = Substrate::of(mat);
            let id = r.get("id").and_then(|x| x.str()).unwrap_or("?").to_string();
            let re = val(vals, "re").and_then(|x| x.num()).unwrap_or(0.0);
            let Some(Json::Arr(cut)) = r.get("cutting") else { continue };
            for c in cut {
                let text = c
                    .get("material")
                    .or_else(|| c.get("material_stated"))
                    .and_then(|x| x.str())
                    .unwrap_or("")
                    .to_string();
                let op = c.get("operation").and_then(|x| x.str()).unwrap_or("").to_string();
                let url = c.get("url").and_then(|x| x.str()).unwrap_or("").to_string();
                let Some(vc) = mid(val(c, "vc")) else {
                    skips.add("no speed stated");
                    continue;
                };
                let Some(g) = iso_group(&text) else {
                    skips.add("material not classed (plastics or unknown text)");
                    continue;
                };
                let Some(group) = lib.group(g) else { continue };
                let theirs_rpm = 1000.0 * vc / (std::f64::consts::PI * d);
                match kind.as_str() {
                    "flat" | "bull" | "ball" => {
                        if op == "chamfer" || op == "drilling" {
                            skips.add("chamfer or drilling row of an end mill");
                            continue;
                        }
                        let (ae, ap) = (mid(val(c, "ae")), mid(val(c, "ap")));
                        let ask = Ask {
                            ae,
                            ap,
                            ..Ask::default()
                        };
                        let tool = ToolData {
                            d,
                            flutes: z.max(1),
                            ball: kind == "ball",
                            re: if kind == "bull" { re } else { 0.0 },
                            substrate: sub,
                            coating: "",
                            beam: None,
                            preset: None,
                        };
                        // with no engagement stated the maker's chip is compared as a slot's
                        let nominal = (d, ap.unwrap_or(0.5 * d));
                        let Some(res) = resolve::milling(&ask, &tool, Some(group), &FREE, nominal) else {
                            skips.add("our resolver has no data (substrate without a generic column)");
                            continue;
                        };
                        out.push(Row {
                            vendor: vendor.into(),
                            tool: id.clone(),
                            kind: kind.clone(),
                            d,
                            z,
                            substrate: sub.name(),
                            material: text,
                            group: g,
                            op,
                            ae,
                            ap,
                            theirs_rpm,
                            ours_rpm: res.rpm.v,
                            theirs_f: mid(val(c, "fz")),
                            ours_f: res.fz.v,
                            ours_src: format!("vc {} / chip {}", res.vc.source.name(), res.chip.source.name()),
                            url,
                            circular: false,
                        });
                    }
                    "drill" => {
                        let f = c.get("fn_or_fz").and_then(|x| {
                            let u = x.get("unit").and_then(|u| u.str()).unwrap_or("");
                            if u.contains("rev") { mid(x.get("v")) } else { None }
                        });
                        let dt = DrillTool {
                            d,
                            flutes: z.max(2),
                            point: val(vals, "sig").and_then(|x| x.num()).unwrap_or(0.0),
                            substrate: sub,
                            preset: None,
                        };
                        let rowdata = match sub {
                            Substrate::Carbide => lib.drilling(g, "carbide"),
                            Substrate::Hss => lib.drilling(g, "hss"),
                            _ => None,
                        };
                        let Some(res) =
                            resolve::drilling(&DrillAsk::default(), &dt, Some(group), None, rowdata, &FREE, d)
                        else {
                            skips.add("our drilling model has no row for the group and substrate");
                            continue;
                        };
                        let circular = rowdata.is_some_and(|rw| {
                            rw.cites.iter().any(|ci| {
                                (vendor == "guhring" && ci.source.contains("guhring"))
                                    || (vendor == "osg" && ci.source.contains("osgtool"))
                                    || ci.source.contains("feeds-vs-makers")
                            })
                        });
                        out.push(Row {
                            vendor: vendor.into(),
                            tool: id.clone(),
                            kind: kind.clone(),
                            d,
                            z,
                            substrate: sub.name(),
                            material: text,
                            group: g,
                            op: "drill".into(),
                            ae: None,
                            ap: None,
                            theirs_rpm,
                            ours_rpm: res.rpm.v,
                            theirs_f: f,
                            ours_f: res.f.v,
                            ours_src: format!("vc {} / f {}", res.vc.source.name(), res.f.source.name()),
                            url,
                            circular,
                        });
                    }
                    _ => skips.add("tool kind not compared (chamfer, spot, tap, thread mill)"),
                }
            }
        }
    }
    (out, skips)
}

/// Median and the 10th/90th percentiles of a set of ratios (None when empty).
pub fn stats(mut v: Vec<f64>) -> Option<(f64, f64, f64, usize)> {
    v.retain(|x| x.is_finite());
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    let q = |p: f64| v[((v.len() - 1) as f64 * p).round() as usize];
    Some((q(0.5), q(0.1), q(0.9), v.len()))
}

/// One summary line per key: (key, rpm stats, feed-per-tooth stats, feed-rate stats).
#[allow(clippy::type_complexity)]
pub fn summary<F: Fn(&Row) -> String>(
    rows: &[Row],
    key: F,
) -> Vec<(String, Option<(f64, f64, f64, usize)>, Option<(f64, f64, f64, usize)>, Option<(f64, f64, f64, usize)>)> {
    let mut keys: Vec<String> = rows.iter().map(&key).collect();
    keys.sort();
    keys.dedup();
    keys.into_iter()
        .map(|k| {
            let sel: Vec<&Row> = rows.iter().filter(|r| key(r) == k).collect();
            (
                k,
                stats(sel.iter().map(|r| r.r_rpm()).collect()),
                stats(sel.iter().filter_map(|r| r.r_f()).collect()),
                stats(sel.iter().filter_map(|r| r.r_vf()).collect()),
            )
        })
        .collect()
}

/// The pinned per-row line: what a change must not move silently.
pub fn pin_line(r: &Row) -> String {
    let f = |x: Option<f64>| x.map_or("-".to_string(), |v| format!("{v:.4}"));
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{:.4}\t{}",
        r.vendor,
        r.tool,
        r.group,
        r.op,
        r.material,
        r.ae.map_or("-".into(), |a| format!("{a:.3}")),
        r.r_rpm(),
        f(r.r_f())
    )
}
