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
pub mod turning;

use forge_agent::json::Json;
use forge_tooling::Library;
use forge_tooling::feeds::{Generic, Quantiles, Substrate, ToolClass, TurnGeneric};
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
    /// Presets ON, the real default path: the tool's own preset as an op takes it
    /// (`Tool::preset_for` with the op's class), at the maker's engagement. None where the
    /// library has no such tool or no preset for the group.
    pub on: Option<On>,
    /// The tool's coating as printed, its helix (degrees, where printed) and corner radius.
    pub coating: String,
    pub helix: Option<f64>,
    pub re: f64,
    /// The maker's chip (its fz thinned at its own ae and ap, `resolve::preset_chip`) over our
    /// group's slot chip at the same diameter: what the generic classes are made of.
    pub chip_ratio: Option<f64>,
    /// Which generic class our presets-OFF numbers came from (empty: none, the group table).
    pub generic: String,
    /// The maker's printed range (low, high): rpm, and feed where stated.
    pub theirs_rpm_span: (f64, f64),
    pub theirs_f_span: Option<(f64, f64)>,
    /// The tool's preset for this very row (its material text and engagement): ours must give
    /// the maker's own numbers back (a check of the conversion and of reading a preset's feed).
    pub own: Option<(f64, f64)>,
}

/// One row run with presets ON.
#[derive(Clone, Debug, PartialEq)]
pub struct On {
    pub preset: String,
    pub rpm: f64,
    pub f: f64,
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
    pub fn on_rpm(&self) -> Option<f64> {
        self.on.as_ref().map(|o| o.rpm / self.theirs_rpm)
    }
    pub fn on_f(&self) -> Option<f64> {
        Some(self.on.as_ref()?.f / self.theirs_f?)
    }
    pub fn own_rpm(&self) -> Option<f64> {
        self.own.map(|o| o.0 / self.theirs_rpm)
    }
    pub fn own_f(&self) -> Option<f64> {
        Some(self.own?.1 / self.theirs_f?)
    }
    /// Ours (with the row's own preset) inside the maker's printed range, speed and feed.
    pub fn own_in_range(&self) -> Option<bool> {
        let (n, f) = self.own?;
        let within = |x: f64, (lo, hi): (f64, f64)| x >= lo * (1.0 - 1e-3) && x <= hi * (1.0 + 1e-3);
        Some(within(n, self.theirs_rpm_span) && self.theirs_f_span.is_none_or(|sp| within(f, sp)))
    }
}

/// The op of the language a maker's row is: a slot row a slot, a rough row a pocket, a
/// max/HEM row a clearing, a finishing row a contour, the rest general.
pub fn lang_kind(op: &str) -> &'static str {
    match op {
        "slot" => "slot",
        "rough" => "pocket",
        "max" => "clear",
        "finish" => "contour",
        _ => "general",
    }
}

/// The tool's preset written for this very maker row: same group, the maker's material text,
/// the row's width and depth.
fn own_preset<'a>(
    t: &'a forge_tooling::Tool,
    g: &str,
    op: &str,
    text: &str,
    ae: Option<(f64, f64)>,
    ap: Option<(f64, f64)>,
) -> Option<(&'a forge_tooling::Preset, Option<&'a forge_tooling::Cite>)> {
    // the preset's width and depth are a value of the row's printed range (the presets take its
    // low end)
    let near = |a: Option<f64>, b: Option<(f64, f64)>| match (a, b) {
        (Some(a), Some((lo, hi))) => a >= lo - 1e-3 && a <= hi + 1e-3,
        (None, None) => true,
        _ => false,
    };
    let p = t.presets.iter().find(|p| {
        let class = match op {
            "slot" | "rough" | "finish" => op,
            "max" => "rough",
            _ => "general",
        };
        p.group == g
            && p.name.split(' ').nth(1) == Some(class)
            && p.name.splitn(3, ' ').nth(2) == Some(text)
            && near(p.ae, ae)
            && near(p.ap, ap)
    })?;
    let path = format!("preset.{}", p.name);
    Some((p, t.cites.iter().find(|c| c.path == path)))
}

/// The maker's material text → ISO 513 group: (keywords, group), first match wins, so the more
/// specific come first. Hardness classes over about 45 HRC are H; 36–50 HRC rows stay P (ISO 513
/// H starts at hardened steels, about 45 HRC; the row straddles it). Plastics: none (no ISO class).
pub const CLASSES: &[(&[&str], &str)] = &[
    (&["plastic"], ""),
    (&["> 50 hrc", "46 rc+", "hardened"], "H"),
    (&["stainless"], "M"),
    (&["cast iron"], "K"),
    (
        &[
            "titanium",
            "monel",
            "high nickel",
            "inconel",
            "nickel alloy",
            "heat resistant",
            "superalloy",
        ],
        "S",
    ),
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

/// A stated value's printed range (a single value: both ends).
fn span(v: Option<&Json>) -> Option<(f64, f64)> {
    match v? {
        Json::Num(x) => Some((*x, *x)),
        Json::Arr(a) => {
            let xs: Vec<f64> = a.iter().filter_map(|x| x.num()).collect();
            if xs.is_empty() {
                return None;
            }
            Some((
                xs.iter().copied().fold(f64::INFINITY, f64::min),
                xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            ))
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
        let Some(Json::Arr(recs)) = doc.get("records") else {
            continue;
        };
        for r in recs {
            let kind = r.get("kind").and_then(|k| k.str()).unwrap_or("").to_string();
            let vals = r.get("values").unwrap_or(&Json::Null);
            let Some(d) = val(vals, "dc").and_then(|x| x.num()) else {
                continue;
            };
            let z = val(vals, "nof").and_then(|x| x.num()).unwrap_or(2.0) as u32;
            let mat = val(vals, "material").and_then(|x| x.str()).unwrap_or("");
            let sub = Substrate::of(mat);
            let id = r.get("id").and_then(|x| x.str()).unwrap_or("?").to_string();
            let re = val(vals, "re").and_then(|x| x.num()).unwrap_or(0.0);
            let Some(Json::Arr(cut)) = r.get("cutting") else {
                continue;
            };
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
                let vc_span = span(val(c, "vc")).unwrap_or((vc, vc));
                let theirs_rpm_span = (
                    1000.0 * vc_span.0 / (std::f64::consts::PI * d),
                    1000.0 * vc_span.1 / (std::f64::consts::PI * d),
                );
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
                        let coating = val(vals, "coating")
                            .and_then(|x| x.str())
                            .unwrap_or("")
                            .to_string();
                        let helix = val(vals, "helix").and_then(|x| x.num());
                        let tc = ToolClass {
                            family: forge_tooling::feeds::family_of_text(&text),
                            tier: Some(tier_of(vendor)),
                            flutes: Some(z.max(1)),
                            coated: ToolClass::coated(&coating),
                            helix,
                        };
                        // the generic default path: the op's class as the language classes it
                        // (a generic tool: a contour at 10% D or under finishes), its generic
                        // class and the class's chip factor (or the [[op_class]] one)
                        let class = generic_class(&op, ae, d);
                        let gu = lib.generic_use(g, class, &tc);
                        let factor = match (class, gu.and_then(|u| u.class_factor())) {
                            ("slot" | "general", _) => None,
                            (c, Some(f)) => Some((c, f)),
                            (c, None) => lib.op_class(c).map(|r| (c, r.chip_factor)),
                        };
                        let tool = ToolData {
                            d,
                            flutes: z.max(1),
                            ball: kind == "ball",
                            re: if kind == "bull" { re } else { 0.0 },
                            substrate: sub,
                            coating: "",
                            beam: None,
                            class: factor.map(|(c, f)| (c, f, "")),
                            preset: None,
                            generic: gu,
                        };
                        let chip_ratio = mid(val(c, "fz")).map(|fz| {
                            let p = forge_tooling::Preset {
                                name: String::new(),
                                group: g.into(),
                                vc,
                                fz,
                                ap,
                                ae,
                            };
                            let rr = if kind == "ball" {
                                0.5 * d
                            } else if kind == "bull" {
                                re
                            } else {
                                0.0
                            };
                            resolve::preset_chip(&p, d, rr).0 / group.fz_at(d)
                        });
                        // with no engagement stated the maker's chip is compared as a slot's
                        let nominal = (d, ap.unwrap_or(0.5 * d));
                        let Some(res) = resolve::milling(&ask, &tool, Some(group), &FREE, nominal) else {
                            skips.add("our resolver has no data (substrate without a generic column)");
                            continue;
                        };
                        let lt = lib.tool(&format!("{vendor}-{id}")).ok();
                        let on = lt.and_then(|t| {
                            let class = t.class_for(lang_kind(&op), ae.unwrap_or(d), g);
                            let (p, c, classed) = t.preset_for(g, class)?;
                            let class_rec = if classed || class == "slot" || class == "general" {
                                None
                            } else {
                                lib.op_class(class)
                            };
                            let res = resolve::milling(
                                &ask,
                                &ToolData {
                                    class: class_rec.map(|c| (c.name.as_str(), c.chip_factor, "")),
                                    preset: Some((p, c)),
                                    generic: None,
                                    ..tool
                                },
                                Some(group),
                                &FREE,
                                nominal,
                            )?;
                            Some(On {
                                preset: p.name.clone(),
                                rpm: res.rpm.v,
                                f: res.fz.v,
                            })
                        });
                        let own = lt.and_then(|t| {
                            let pc = own_preset(t, g, &op, &text, span(val(c, "ae")), span(val(c, "ap")))?;
                            // at the preset's own width and depth (a value of the row's range)
                            let own_ask = Ask {
                                ae: pc.0.ae.or(ae),
                                ap: pc.0.ap.or(ap),
                                ..Ask::default()
                            };
                            let res = resolve::milling(
                                &own_ask,
                                &ToolData {
                                    preset: Some(pc),
                                    class: None,
                                    generic: None,
                                    ..tool
                                },
                                Some(group),
                                &FREE,
                                nominal,
                            )?;
                            Some((res.rpm.v, res.fz.v))
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
                            op,
                            ae,
                            ap,
                            theirs_rpm,
                            ours_rpm: res.rpm.v,
                            theirs_f: mid(val(c, "fz")),
                            ours_f: res.fz.v,
                            ours_src: format!(
                                "vc {} / chip {}",
                                res.vc.source.name(),
                                res.chip.source.name()
                            ),
                            url,
                            circular: false,
                            on,
                            own,
                            coating,
                            helix,
                            re,
                            chip_ratio,
                            generic: gu.map_or(String::new(), |u| u.op.class()),
                            theirs_rpm_span,
                            theirs_f_span: span(val(c, "fz")),
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
                        let Some(res) = resolve::drilling(
                            &DrillAsk::default(),
                            &dt,
                            Some(group),
                            None,
                            rowdata,
                            &FREE,
                            d,
                        ) else {
                            skips.add("our drilling model has no row for the group and substrate");
                            continue;
                        };
                        let lt = lib.tool(&format!("{vendor}-{id}")).ok();
                        let pset = lt.and_then(|t| t.preset_for(g, "general"));
                        let on = pset.and_then(|(p, c, _)| {
                            let res = resolve::drilling(
                                &DrillAsk::default(),
                                &DrillTool {
                                    preset: Some((p, c)),
                                    ..dt
                                },
                                Some(group),
                                None,
                                rowdata,
                                &FREE,
                                d,
                            )?;
                            Some(On {
                                preset: p.name.clone(),
                                rpm: res.rpm.v,
                                f: res.f.v,
                            })
                        });
                        let own = lt.and_then(|t| {
                            let pc = own_preset(t, g, "general", &text, None, None)?;
                            let res = resolve::drilling(
                                &DrillAsk::default(),
                                &DrillTool {
                                    preset: Some(pc),
                                    ..dt
                                },
                                Some(group),
                                None,
                                rowdata,
                                &FREE,
                                d,
                            )?;
                            Some((res.rpm.v, res.f.v))
                        });
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
                            on,
                            own,
                            coating: String::new(),
                            helix: None,
                            re: 0.0,
                            chip_ratio: None,
                            generic: String::new(),
                            theirs_rpm_span,
                            theirs_f_span: c.get("fn_or_fz").and_then(|x| {
                                let u = x.get("unit").and_then(|u| u.str()).unwrap_or("");
                                if u.contains("rev") { span(x.get("v")) } else { None }
                            }),
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
) -> Vec<(
    String,
    Option<(f64, f64, f64, usize)>,
    Option<(f64, f64, f64, usize)>,
    Option<(f64, f64, f64, usize)>,
)> {
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
        "{}\t{}\t{}\t{}\t{}\t{}\t{:.4}\t{}\ton {}\t{}\t{}",
        r.vendor,
        r.tool,
        r.group,
        r.op,
        r.material,
        r.ae.map_or("-".into(), |a| format!("{a:.3}")),
        r.r_rpm(),
        f(r.r_f()),
        r.on.as_ref().map_or("-", |o| o.preset.as_str()),
        f(r.on_rpm()),
        f(r.on_f())
    )
}

/// The maker's own cutting speed for a row, m/min.
pub fn theirs_vc(r: &Row) -> f64 {
    r.theirs_rpm * std::f64::consts::PI * r.d / 1000.0
}

/// The makers' tier, from their own product lines: Harvey's "… Material Specific End Mills"
/// and Helical's "End Mills for Aluminum Alloys / for Steels" lines are material-specific
/// (high-performance) tools; Lakeshore's sheet is "GENERAL PURPOSE EM SPECS"
/// (lakeshorecarbide.com/pdf/GENERAL PURPOSE EM SPECS.pdf); the tool names are in the raw
/// records (research/cam/tooling/raw/*.json `name`).
pub fn tier_of(vendor: &str) -> &'static str {
    match vendor {
        "lakeshore" => "general-purpose",
        _ => "material-specific",
    }
}

/// A maker's op as a generic tool's op class: slot; rough and max (HEM) rough; finish at a
/// finishing width (<= 10% D, `Tool::class_for` for a tool with no finish preset), else rough;
/// the rest general.
pub fn generic_class(op: &str, ae: Option<f64>, d: f64) -> &'static str {
    match op {
        "slot" => "slot",
        "rough" | "max" => "rough",
        "finish" if ae.is_some_and(|a| a <= 0.1 * d * (1.0 + 1e-9)) => "finish",
        "finish" => "rough",
        _ => "general",
    }
}

/// Fewest rows, and fewest makers, a generic class is written with.
pub const GENERIC_MIN_ROWS: usize = 10;
pub const GENERIC_MIN_MAKERS: usize = 2;

/// The generic classes from the makers' end-mill rows (carbide: every compared row is): for
/// each level of `feeds::GENERIC_LEVELS`, each class with at least GENERIC_MIN_ROWS rows from
/// at least GENERIC_MIN_MAKERS makers, the min, 10th and 25th percentiles, median and max of the rows'
/// speeds (the middle of a printed range) and chips (relative to the group's slot chip).
/// `without`: leave one maker's rows out (the hold-out).
pub fn generic_table(rows: &[Row], without: Option<&str>) -> Vec<Generic> {
    use std::collections::BTreeMap;
    let mut classes: BTreeMap<String, Vec<&Row>> = BTreeMap::new();
    for r in rows
        .iter()
        .filter(|r| r.kind != "drill" && r.substrate == "carbide" && Some(r.vendor.as_str()) != without)
    {
        let tc = ToolClass {
            family: forge_tooling::feeds::family_of_text(&r.material),
            tier: Some(tier_of(&r.vendor)),
            flutes: Some(r.z.max(1)),
            coated: ToolClass::coated(&r.coating),
            helix: r.helix,
        };
        let op = generic_class(&r.op, r.ae, r.d);
        for keep in forge_tooling::feeds::GENERIC_LEVELS {
            if let Some(k) = forge_tooling::feeds::generic_key(r.group, op, &tc, keep) {
                classes.entry(k).or_default().push(r);
            }
        }
    }
    let mut out = Vec::new();
    for (k, sel) in classes {
        let mut makers: Vec<String> = sel.iter().map(|r| r.vendor.clone()).collect();
        makers.sort();
        makers.dedup();
        if sel.len() < GENERIC_MIN_ROWS || makers.len() < GENERIC_MIN_MAKERS {
            continue;
        }
        let Some(vc) = Quantiles::of(sel.iter().map(|r| theirs_vc(r)).collect()) else {
            continue;
        };
        let Some(chip) = Quantiles::of(sel.iter().filter_map(|r| r.chip_ratio).collect()) else {
            continue;
        };
        let f: Vec<&str> = k.split('|').collect();
        out.push(Generic {
            group: f[0].into(),
            family: f[1].into(),
            op: f[2].into(),
            tier: f[3].into(),
            flutes: f[4].parse().unwrap_or(0),
            coating: f[5].into(),
            helix: f[6].into(),
            n: sel.len(),
            makers,
            vc,
            chip,
            cites: Vec::new(),
        });
    }
    out
}

/// data/generic.toml's text for a table.
pub fn generic_toml(table: &[Generic]) -> String {
    let mut s = String::from(
        "# Generated by `cargo run -p feeds-vs-makers -- --generic` from the makers' cited end-mill rows\n\
         # (research/cam/tooling/raw/{harvey,helical,lakeshore}.json); do not edit. Each class: the makers'\n\
         # speeds and chips over its rows as [min, P10, P25, median, max]; the generic default is P10\n\
         # (a speed or chip that is too high breaks tools, one too low only costs time).\n\
         # chip is relative to the group's slot chip at the row's diameter (cutting.toml fz rows).\n",
    );
    let f4 = |a: Quantiles| {
        format!(
            "[{:.4}, {:.4}, {:.4}, {:.4}, {:.4}]",
            a.min, a.p10, a.p25, a.median, a.max
        )
    };
    for g in table {
        s.push_str(&format!(
            "\n[[generic]]\ngroup = \"{}\"\nfamily = \"{}\"\nop = \"{}\"\ntier = \"{}\"\nflutes = {}\ncoating = \"{}\"\nhelix = \"{}\"\nn = {}\nmakers = [{}]\nvc = {}\nchip = {}\nreal = true\n",
            g.group,
            g.family,
            g.op,
            g.tier,
            g.flutes,
            g.coating,
            g.helix,
            g.n,
            g.makers.iter().map(|m| format!("\"{m}\"")).collect::<Vec<_>>().join(", "),
            f4(g.vc),
            f4(g.chip)
        ));
        for (path, what) in [
            ("vc", "cutting speeds (the middle of each printed range)"),
            (
                "chip",
                "chips (each row's fz thinned at its own ae and ap, over the group's slot chip at its diameter)",
            ),
        ] {
            s.push_str(&format!(
                "[[generic.cite]]\npath = \"{path}\"\nstatus = \"estimate\"\nsource = \"research/cam/tooling/raw/*.json (each row's maker URL in verify/feeds-vs-makers/rows.tsv)\"\ntaken = \"2026-11-24\"\nbasis = \"min, 10th and 25th percentiles, median, max of {} makers' {what} over {} rows of class {}; the tier (material-specific / general-purpose) is an estimate from the makers' product-line names (no maker states a tier)\"\n",
                g.makers.len(),
                g.n,
                g.class()
            ));
        }
    }
    s
}

/// Leave one maker out of the generic classes: each of its rows scored against the classes
/// built from the other makers only, at the class the row's tool would resolve to there.
/// Returns, per row of a maker: (row index, default speed / maker's, default chip / maker's,
/// class), the default being the class's 10th percentile.
pub fn generic_held_out(rows: &[Row]) -> Vec<(usize, f64, Option<f64>, String)> {
    let mut makers: Vec<&str> = rows
        .iter()
        .filter(|r| r.kind != "drill")
        .map(|r| r.vendor.as_str())
        .collect();
    makers.sort();
    makers.dedup();
    let mut out = Vec::new();
    for m in makers {
        let table = generic_table(rows, Some(m));
        for (i, r) in rows
            .iter()
            .enumerate()
            .filter(|(_, r)| r.vendor == m && r.kind != "drill")
        {
            let tc = ToolClass {
                family: forge_tooling::feeds::family_of_text(&r.material),
                tier: Some(tier_of(&r.vendor)),
                flutes: Some(r.z.max(1)),
                coated: ToolClass::coated(&r.coating),
                helix: r.helix,
            };
            let op = generic_class(&r.op, r.ae, r.d);
            let Some(gv) = forge_tooling::feeds::generic_in(&table, r.group, op, &tc) else {
                continue;
            };
            let gs = forge_tooling::feeds::generic_in(&table, r.group, "slot", &tc);
            let chip = match (gs, r.chip_ratio) {
                (Some(gs), Some(c)) => Some(gs.chip.default_value() / c),
                _ => None,
            };
            out.push((i, gv.vc.default_value() / theirs_vc(r), chip, gv.class()));
        }
    }
    out
}

/// One maker row of turning data: (maker, vc, f, ap).
type TurnSample = (String, f64, f64, Option<f64>);

/// The generic turning classes from the inserts' maker presets (each cited to its maker's
/// grade table): one row per (maker, group, chipbreaker, grade, material) the makers print, so an
/// insert sold in many codes counts once. Per group × material family, and per whole group,
/// with at least GENERIC_MIN_ROWS rows from at least GENERIC_MIN_MAKERS makers: the speeds',
/// feeds' and depths' min, P10, P25, median, max.
pub fn turn_generic_table(lib: &Library) -> Vec<TurnGeneric> {
    turn_generic_table_without(lib, None)
}

/// As `turn_generic_table`, leaving one maker's rows out (the hold-out).
pub fn turn_generic_table_without(lib: &Library, without: Option<&str>) -> Vec<TurnGeneric> {
    use std::collections::{BTreeMap, BTreeSet};
    let mut seen = BTreeSet::new();
    // (group, family) -> rows (maker, vc, f, ap)
    let mut classes: BTreeMap<(String, String), Vec<TurnSample>> = BTreeMap::new();
    for ins in lib.inserts.iter().filter(|i| Some(i.maker.as_str()) != without) {
        for p in &ins.presets {
            let head = format!("{} {} ", p.group, ins.chipbreaker);
            let Some(rest) = p.name.strip_prefix(&head) else {
                continue;
            };
            let Some((grade, mat)) = rest.split_once(' ') else {
                continue;
            };
            if !seen.insert((
                ins.maker.clone(),
                p.group.clone(),
                ins.chipbreaker.clone(),
                grade.to_string(),
                mat.to_string(),
            )) {
                continue;
            }
            let row = (ins.maker.clone(), p.vc, p.fz, p.ap);
            if let Some(f) = forge_tooling::feeds::family_of_text(mat) {
                classes
                    .entry((p.group.clone(), f.to_string()))
                    .or_default()
                    .push(row.clone());
            }
            classes
                .entry((p.group.clone(), "*".to_string()))
                .or_default()
                .push(row);
        }
    }
    let mut out = Vec::new();
    for ((group, family), rows) in classes {
        let mut makers: Vec<String> = rows.iter().map(|r| r.0.clone()).collect();
        makers.sort();
        makers.dedup();
        if rows.len() < GENERIC_MIN_ROWS || makers.len() < GENERIC_MIN_MAKERS {
            continue;
        }
        let (Some(vc), Some(f), Some(ap)) = (
            Quantiles::of(rows.iter().map(|r| r.1).collect()),
            Quantiles::of(rows.iter().map(|r| r.2).collect()),
            Quantiles::of(rows.iter().filter_map(|r| r.3).collect()),
        ) else {
            continue;
        };
        out.push(TurnGeneric {
            group,
            family,
            n: rows.len(),
            makers,
            vc,
            f,
            ap,
            cites: Vec::new(),
        });
    }
    out
}

/// The `[[turn_generic]]` records' text.
pub fn turn_generic_toml(table: &[TurnGeneric]) -> String {
    let f5 = |a: Quantiles| {
        format!(
            "[{:.4}, {:.4}, {:.4}, {:.4}, {:.4}]",
            a.min, a.p10, a.p25, a.median, a.max
        )
    };
    let mut s = String::new();
    for g in table {
        s.push_str(&format!(
            "\n[[turn_generic]]\ngroup = \"{}\"\nfamily = \"{}\"\nn = {}\nmakers = [{}]\nvc = {}\nf = {}\nap = {}\nreal = true\n",
            g.group,
            g.family,
            g.n,
            g.makers.iter().map(|m| format!("\"{m}\"")).collect::<Vec<_>>().join(", "),
            f5(g.vc),
            f5(g.f),
            f5(g.ap)
        ));
        for (path, what) in [
            ("vc", "cutting speeds"),
            ("f", "feeds per revolution"),
            ("ap", "depths of cut"),
        ] {
            s.push_str(&format!(
                "[[turn_generic.cite]]\npath = \"{path}\"\nstatus = \"estimate\"\nsource = \"the inserts' [[insert.preset]] (kernel/forge-tooling/data/turning/*.toml), each cited to its maker's grade table\"\ntaken = \"2026-11-24\"\nbasis = \"min, 10th and 25th percentiles, median, max of {} makers' {what} over {} grade-table rows of class {}\"\n",
                g.makers.len(),
                g.n,
                g.class()
            ));
        }
    }
    s
}
