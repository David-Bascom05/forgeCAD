//! Vendor profiles in the language (forge-vendor): `vendor("sendcutsend", material: "5052")` is
//! a shop, and `sheet(.., vendor: "sendcutsend", material: "5052-H32 aluminum")` takes its stock.
//! The sheet's radius and bend rule default from the vendor's stock of its thickness, so the flat
//! pattern develops by the vendor's own K and radius; a thickness the vendor does not stock in
//! that material, a material it does not offer and a vendor not found are refused, with the
//! options listed.
use crate::error::Error;
use crate::eval::R;
use crate::sheet::{RuleV, SheetErr, ShopV, StockV};
use crate::syntax::Span;
use crate::values::{Dim, Num};
use forge_agent::json::Json;
use forge_sheet as fs;
use forge_vendor::{Profiles, Vendor};
use std::rc::Rc;
use std::sync::Arc;

/// What a shop made from a vendor carries besides its rules: who and what it is, for the report
/// and the drawing.
#[derive(Clone, Debug, PartialEq)]
pub struct VendorTag {
    pub id: String,
    pub name: String,
    /// The vendor's own name for the material.
    pub material: String,
    /// forge-material's name for it (mass, the drawing's note).
    pub library: Option<String>,
    pub source: Option<String>,
    pub taken: Option<String>,
    pub note: Option<String>,
    pub tolerance: Vec<(String, String)>,
    /// Stocked thicknesses (mm), with gauges.
    pub thicknesses: Vec<(f64, Option<String>)>,
    /// Where the profile was read from (none: built in).
    pub file: Option<String>,
    /// Stocked thicknesses (mm) the vendor gives no bend rule for: a sheet of one says its own.
    pub no_rule: Vec<f64>,
}

impl VendorTag {
    /// The thicknesses as a person reads them: `1.6002 mm (0.063 in, 14 ga)`.
    pub fn listed(&self, inches: bool) -> String {
        self.thicknesses
            .iter()
            .map(|(t, g)| {
                let mm = crate::values::fmt(*t);
                let inch = if inches {
                    format!(" = {} in", crate::values::fmt(t / 25.4))
                } else {
                    String::new()
                };
                match g {
                    Some(g) => format!("{mm}{inch} ({g})"),
                    None => format!("{mm}{inch}"),
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// The profiles a document sees: its project's, the user's, the built-in ones.
pub fn profiles(ev: &crate::eval::Eval) -> Arc<Profiles> {
    // a session kept to its root reads no file: the built-in ones only
    if ev.root.is_some() {
        return Arc::new(Profiles::builtin());
    }
    let dir = ev.dir.as_deref();
    if let Some(d) = dir {
        crate::deps::tomls(&forge_vendor::project_dir(d));
    }
    if let Some(u) = forge_vendor::user_dir() {
        crate::deps::tomls(&u);
    }
    Profiles::cached(dir)
}

fn near<'a>(all: impl Iterator<Item = &'a str>, name: &str) -> Vec<String> {
    let n = forge_material::norm(name);
    let mut v: Vec<(usize, &str)> = all.map(|x| (edits(&forge_material::norm(x), &n), x)).collect();
    v.sort();
    v.into_iter().take(3).map(|x| x.1.to_string()).collect()
}

fn edits(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for j in 0..b.len() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != b[j]))
                .min(row[j] + 1)
                .min(row[j + 1] + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

/// The shop vendor `id` is for material `material`, or the typed refusal. `sheet`: the sheet
/// feature naming them (its fixes SetArg it).
pub fn shop(
    ev: &crate::eval::Eval,
    id: &str,
    id_span: Span,
    material: Option<&str>,
    material_span: Span,
    sheet: Option<&str>,
) -> R<Rc<ShopV>> {
    let p = profiles(ev);
    let Some(e) = p.find(id) else {
        return Err(Error::Sheet(Box::new(SheetErr::UnknownVendor {
            span: id_span,
            name: id.into(),
            ids: p.ids().iter().map(|s| s.to_string()).collect(),
            near: near(p.ids().into_iter(), id),
            sheet: sheet.map(String::from),
        })));
    };
    let v = &e.vendor;
    let offered = || v.materials.iter().map(|m| m.name.clone()).collect::<Vec<_>>();
    let Some(material) = material else {
        return Err(Error::Sheet(Box::new(SheetErr::VendorMaterial {
            span: id_span,
            vendor: v.name.clone(),
            name: None,
            offered: offered(),
            near: vec![],
            sheet: sheet.map(String::from),
        })));
    };
    let Some(m) = v.material(material) else {
        return Err(Error::Sheet(Box::new(SheetErr::VendorMaterial {
            span: material_span,
            vendor: v.name.clone(),
            name: Some(material.into()),
            offered: offered(),
            near: near(
                v.materials.iter().flat_map(|m| {
                    std::iter::once(m.name.as_str()).chain(m.aliases.iter().map(String::as_str))
                }),
                material,
            ),
            sheet: sheet.map(String::from),
        })));
    };
    Ok(Rc::new(of(v, m, e.file.as_deref())))
}

/// The language's shop value of a vendor's material.
pub fn of(v: &Vendor, m: &forge_vendor::MaterialRow, file: Option<&std::path::Path>) -> ShopV {
    let shop = v.shop(m);
    let stocks = shop
        .stock
        .iter()
        .map(|s| {
            Rc::new(StockV {
                stock: s.clone(),
                t: Num::konst(s.thickness, Dim::Length),
                radii: s.radii.iter().map(|r| Num::konst(*r, Dim::Length)).collect(),
                rule: Rc::new(RuleV {
                    rule: s.rule.clone(),
                    k: match s.rule {
                        fs::BendRule::K(k) => Some(Num::konst(k, Dim::Ratio)),
                        _ => None,
                    },
                }),
            })
        })
        .collect();
    ShopV {
        stocks,
        hems: fs::HemTable::default(),
        vendor: Some(Rc::new(VendorTag {
            id: v.id.clone(),
            name: v.name.clone(),
            material: m.name.clone(),
            library: m.library.clone(),
            source: v.source.clone(),
            taken: v.taken.clone(),
            note: v.note.clone(),
            tolerance: v.tolerance.clone(),
            thicknesses: v.thicknesses(m),
            file: file.map(|f| f.display().to_string()),
            no_rule: m
                .stock
                .iter()
                .filter(|s| s.k.is_none() && s.ktable.is_none() && s.bend_deduction.is_none())
                .map(|s| s.t * v.units.mm())
                .collect(),
        })),
        shop,
    }
}

/// Whether the vendor states a bend rule for its stock of thickness `t` (or never bends it).
pub fn states_rule(shop: &ShopV, t: f64) -> bool {
    let Some(tag) = &shop.vendor else { return true };
    let Some(s) = shop.shop.stock(t) else { return true };
    // a stock only cut is never bent by its rule (each bend is OperationNotOffered)
    !shop.shop.offers_in(fs::Operation::Bend, t)
        || !tag.no_rule.iter().any(|x| (x - s.thickness).abs() <= 1e-9)
}

/// The stock of thickness `t`, or VendorThickness with the stocked ones and a fix to the nearest.
pub fn stock_of(shop: &ShopV, t: &Num, span: Span) -> R<()> {
    let Some(tag) = &shop.vendor else { return Ok(()) };
    if shop.shop.stock(t.v).is_some() {
        return Ok(());
    }
    let nearest = shop.shop.nearest_thickness(t.v);
    let fix = nearest.and_then(|x| crate::sheet::set_param(t, x));
    Err(Error::Sheet(Box::new(SheetErr::VendorThickness {
        span,
        vendor: tag.name.clone(),
        material: tag.material.clone(),
        thickness: t.v,
        offered: tag.thicknesses.clone(),
        fixes: fix.into_iter().collect(),
    })))
}

fn listing(p: &Profiles) -> Json {
    Json::Arr(
        p.list
            .iter()
            .map(|e| {
                let v = &e.vendor;
                Json::obj(vec![
                    ("id", v.id.as_str().into()),
                    ("name", v.name.as_str().into()),
                    ("library", e.lib.label().into()),
                    (
                        "file",
                        e.file
                            .as_ref()
                            .map_or(Json::Null, |f| f.display().to_string().into()),
                    ),
                    ("taken", v.taken.as_deref().map_or(Json::Null, Json::from)),
                    ("source", v.source.as_deref().map_or(Json::Null, Json::from)),
                    (
                        "materials",
                        Json::Arr(v.materials.iter().map(|m| m.name.as_str().into()).collect()),
                    ),
                ])
            })
            .collect(),
    )
}

/// One vendor's profile as JSON: its provenance, operations, limits, tolerances, and each
/// material's stocks with the rules they are checked by.
pub fn profile_json(v: &Vendor) -> Json {
    let materials = v
        .materials
        .iter()
        .map(|m| {
            let shop = v.shop(m);
            let stocks = shop
                .stock
                .iter()
                .zip(&m.stock)
                .map(|(s, row)| {
                    let t = s.thickness;
                    let r = s.radii.first().copied().unwrap_or(0.0);
                    let at = |x: fs::Scaled| {
                        (x != fs::Scaled::NONE)
                            .then(|| x.at(t, r))
                            .map_or(Json::Null, Json::Num)
                    };
                    Json::obj(vec![
                        ("thickness", t.into()),
                        ("gauge", row.gauge.as_deref().map_or(Json::Null, Json::from)),
                        (
                            "radii",
                            Json::Arr(s.radii.iter().map(|x| Json::Num(*x)).collect()),
                        ),
                        (
                            "k",
                            match s.rule {
                                fs::BendRule::K(k) if row.k.is_some() || row.bend_deduction.is_some() => {
                                    k.into()
                                }
                                _ => Json::Null,
                            },
                        ),
                        ("bends", (!s.not.contains(&fs::Operation::Bend)).into()),
                        ("min_flange", at(s.min_flange)),
                        ("hole_to_bend", at(s.hole_to_bend)),
                        ("hole_to_edge", at(s.hole_to_edge)),
                        ("bend_to_edge", at(s.bend_to_edge)),
                        ("min_hole", at(s.min_hole)),
                        ("min_slot", at(s.min_slot)),
                        ("min_web", at(s.min_web)),
                    ])
                })
                .collect();
            Json::obj(vec![
                ("name", m.name.as_str().into()),
                (
                    "aliases",
                    Json::Arr(m.aliases.iter().map(|a| a.as_str().into()).collect()),
                ),
                ("library", m.library.as_deref().map_or(Json::Null, Json::from)),
                ("stock", Json::Arr(stocks)),
            ])
        })
        .collect();
    Json::obj(vec![
        ("id", v.id.as_str().into()),
        ("name", v.name.as_str().into()),
        ("source", v.source.as_deref().map_or(Json::Null, Json::from)),
        ("taken", v.taken.as_deref().map_or(Json::Null, Json::from)),
        ("note", v.note.as_deref().map_or(Json::Null, Json::from)),
        ("verify", forge_vendor::VERIFY.into()),
        (
            "operations",
            v.operations.as_ref().map_or(Json::Null, |o| {
                Json::Arr(o.iter().map(|x| x.as_str().into()).collect())
            }),
        ),
        (
            "tolerance",
            Json::Obj(
                v.tolerance
                    .iter()
                    .map(|(k, t)| (k.clone(), Json::from(t.as_str())))
                    .collect(),
            ),
        ),
        ("materials", Json::Arr(materials)),
        ("text", v.to_text().into()),
    ])
}

/// `forgecad vendor ..`: list | show ID | export ID [-o FILE] | import FILE | new ID --from ID |
/// check FILE. `doc_dir`: the project folder whose `vendors/` is searched first. Returns the
/// JSON reply and its words.
pub fn tool(args: &[String], doc_dir: Option<&std::path::Path>) -> Result<(Json, String), String> {
    let p = Profiles::load(doc_dir);
    let find = |id: &str| {
        p.find(id)
            .ok_or_else(|| format!("no vendor profile '{id}'; there are {}", p.ids().join(", ")))
    };
    let flag = |k: &str| {
        args.iter()
            .position(|a| a == k)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let ok = |fields: Vec<(&str, Json)>| {
        let mut f = vec![("ok", Json::from(true))];
        f.extend(fields);
        Json::obj(f)
    };
    match args.first().map(String::as_str) {
        Some("list") | None => {
            let mut text = String::new();
            for e in &p.list {
                text.push_str(&format!(
                    "{:<14} {:<22} {:<22} {}\n",
                    e.vendor.id,
                    e.vendor.name,
                    e.lib.label(),
                    e.vendor.taken.as_deref().unwrap_or("")
                ));
            }
            for (f, e) in &p.errors {
                text.push_str(&format!("unread: {}: {e}\n", f.display()));
            }
            Ok((ok(vec![("vendors", listing(&p))]), text))
        }
        Some("show") => {
            let id = args.get(1).ok_or("vendor show takes an id")?;
            let e = find(id)?;
            Ok((ok(vec![("vendor", profile_json(&e.vendor))]), e.vendor.to_text()))
        }
        Some("export") => {
            let id = args.get(1).ok_or("vendor export takes an id")?;
            let e = find(id)?;
            match flag("-o") {
                Some(f) => {
                    forge_vendor::save(std::path::Path::new(&f), &e.vendor)?;
                    Ok((ok(vec![("file", f.as_str().into())]), format!("wrote {f}\n")))
                }
                None => Ok((ok(vec![("text", e.vendor.to_text().into())]), e.vendor.to_text())),
            }
        }
        Some("import") => {
            let f = args.get(1).ok_or("vendor import takes a file")?;
            let to = forge_vendor::import(std::path::Path::new(f))?;
            let to = to.display().to_string();
            Ok((ok(vec![("file", to.as_str().into())]), format!("saved as {to}\n")))
        }
        Some("new") => {
            let id = args.get(1).ok_or("vendor new takes an id")?;
            let mut v = match flag("--from") {
                Some(from) => find(&from)?.vendor.clone(),
                None => Vendor::parse(&format!("format = 1\nid = \"{id}\"\n")).map_err(|e| e.to_string())?,
            };
            v.id = id.clone();
            v.name = flag("--name").unwrap_or_else(|| id.clone());
            // the copy is the user's now: its data is still the vendor's, as read then
            let text = v.to_text();
            Vendor::parse(&text).map_err(|e| e.to_string())?;
            let dir = forge_vendor::user_dir().ok_or("no config folder on this machine")?;
            let to = dir.join(format!("{id}.toml"));
            forge_vendor::save(&to, &v)?;
            let to = to.display().to_string();
            Ok((ok(vec![("file", to.as_str().into())]), format!("saved as {to}\n")))
        }
        Some("check") => {
            let f = args.get(1).ok_or("vendor check takes a file")?;
            let text = std::fs::read_to_string(f).map_err(|e| format!("{f}: {e}"))?;
            let v = Vendor::parse(&text).map_err(|e| format!("{f}: {e}"))?;
            Ok((
                ok(vec![("vendor", profile_json(&v))]),
                format!("{f}: reads: {} ({} materials)\n", v.name, v.materials.len()),
            ))
        }
        Some(o) => Err(format!(
            "vendor {o}: list | show ID | export ID [-o FILE] | import FILE | new ID [--from ID] [--name NAME] | check FILE"
        )),
    }
}
