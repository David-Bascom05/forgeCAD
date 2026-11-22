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
        })),
        shop,
    }
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
