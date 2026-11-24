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
    /// Presets ON, the real default path: the tool's own preset as an op takes it
    /// (`Tool::preset_for` with the op's class), at the maker's engagement. None where the
    /// library has no such tool or no preset for the group.
    pub on: Option<On>,
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
                        let tool = ToolData {
                            d,
                            flutes: z.max(1),
                            ball: kind == "ball",
                            re: if kind == "bull" { re } else { 0.0 },
                            substrate: sub,
                            coating: "",
                            beam: None,
                            class: None,
                            preset: None,
                        };
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
                            let class_rec = if classed || class != "rough" {
                                None
                            } else {
                                lib.op_class(class)
                            };
                            let res = resolve::milling(
                                &ask,
                                &ToolData {
                                    class: class_rec.map(|c| (c.name.as_str(), c.chip_factor, "")),
                                    preset: Some((p, c)),
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

fn median(mut v: Vec<f64>) -> Option<f64> {
    v.retain(|x| x.is_finite());
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    Some(v[(v.len() - 1) / 2])
}

/// A slot row (the chip is the peak chip: no thinning credit).
fn is_slot(r: &Row) -> bool {
    r.kind == "flat" && (r.op == "slot" || r.ae.is_some_and(|a| a >= 0.5 * r.d))
}

/// Leave one maker out: for each row, the generic speed (and, for the groups whose chip comes
/// from maker slot rows, the slot chip at its diameter) re-derived from the OTHER makers' rows
/// only, as the shipped table was derived from all; the row's ratios then scored against that.
/// Returns (row index, held-out rpm ratio, held-out chip ratio); a row whose group no other
/// maker covers is left out (no hold-out possible).
pub fn held_out(
    rows: &[Row],
    current_vc: &dyn Fn(&str, bool) -> f64,
    chip_groups: &[&str],
) -> Vec<(usize, f64, Option<f64>)> {
    let mut out = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let drill = r.kind == "drill";
        let others = |f: &dyn Fn(&Row) -> bool| -> Vec<&Row> {
            rows.iter()
                .filter(|o| {
                    o.vendor != r.vendor
                        && o.group == r.group
                        && (o.kind == "drill") == drill
                        && o.substrate == r.substrate
                        && f(o)
                })
                .collect()
        };
        let Some(vc_cal) = median(others(&|_| true).iter().map(|o| theirs_vc(o)).collect()) else {
            continue;
        };
        let k = vc_cal / current_vc(r.group, drill);
        let r_rpm = r.r_rpm() * k;
        // the chip: re-derived at the row's diameter from the others' slot rows (bins of the
        // same diameter, within 2%), where this group's chip came from slot rows
        let r_f = if !drill && chip_groups.contains(&r.group) {
            let same_d = others(&|o| is_slot(o) && (o.d - r.d).abs() <= 0.02 * r.d);
            let mine = rows
                .iter()
                .filter(|o| o.group == r.group && is_slot(o) && (o.d - r.d).abs() <= 0.02 * r.d);
            match (
                median(same_d.iter().filter_map(|o| o.theirs_f).collect()),
                median(mine.filter_map(|o| o.theirs_f).collect()),
            ) {
                (Some(cal), Some(all)) => r.r_f().map(|f| f * cal / all),
                _ => None,
            }
        } else if drill {
            // a drill's feed per rev is not re-derived without its maker: no hold-out
            None
        } else {
            r.r_f()
        };
        out.push((i, r_rpm, r_f));
    }
    out
}

/// Our shipped generic speed for a group (carbide end mills; or a carbide/HSS drill row).
pub fn shipped_vc(lib: &Library, group: &str, drill: bool, substrate: &str) -> f64 {
    let sub = if substrate == "HSS" { "hss" } else { "carbide" };
    if drill {
        lib.drilling(group, sub).map_or(f64::NAN, |r| r.vc)
    } else {
        lib.group(group)
            .map_or(f64::NAN, |g| if sub == "hss" { g.vc_hss } else { g.vc_carbide })
    }
}

/// The groups whose generic slot chip comes from the makers' slot rows (cutting.toml fz_rows).
pub fn chip_groups(lib: &Library) -> Vec<&'static str> {
    ["P", "M", "K", "N", "S", "H"]
        .into_iter()
        .filter(|g| lib.group(g).is_some_and(|x| !x.fz_rows.is_empty()))
        .collect()
}

/// Held-out stats per maker: (maker, rows held out, rpm stats, chip stats).
#[allow(clippy::type_complexity)]
pub fn held_out_by_maker(
    rows: &[Row],
) -> Vec<(
    String,
    usize,
    Option<(f64, f64, f64, usize)>,
    Option<(f64, f64, f64, usize)>,
)> {
    let lib = Library::builtin();
    let cg = chip_groups(&lib);
    let ho = held_out(
        rows,
        &|g: &str, drill: bool| {
            // the substrate is carbide for every compared end mill; drills look theirs up by row
            shipped_vc(&lib, g, drill, "carbide")
        },
        &cg,
    );
    let mut makers: Vec<String> = rows.iter().map(|r| r.vendor.clone()).collect();
    makers.sort();
    makers.dedup();
    makers
        .into_iter()
        .map(|m| {
            let sel: Vec<&(usize, f64, Option<f64>)> = ho.iter().filter(|x| rows[x.0].vendor == m).collect();
            (
                m,
                sel.len(),
                stats(sel.iter().map(|x| x.1).collect()),
                stats(sel.iter().filter_map(|x| x.2).collect()),
            )
        })
        .collect()
}
