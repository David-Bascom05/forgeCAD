//! What a thermoforming shop forms, as data: each material's `thermo.*` values in forge-material's
//! records (forming window, shrink, design limits, its stretch law), every one with its own source
//! or marked a stand-in, and the machine's limits. Only the shrink reaches geometry (the mould is
//! grown by it); the law feeds the thickness model and the rest the checks.

/// A length as a multiple of the sheet's thickness plus a constant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PerT {
    pub per_t: f64,
    pub plus: f64,
}

impl PerT {
    pub fn at(self, t: f64) -> f64 {
        self.per_t * t + self.plus
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    pub name: String,
    /// What sheet the record is of (a grade, a maker's sheet, or a class).
    pub grade: String,
    /// The sheet temperature range it forms in, and the normal one where the source gives it, deg C.
    pub forming: (f64, f64),
    pub normal: Option<f64>,
    /// Linear mould shrinkage, as a fraction: the tool is this much larger than the part.
    pub shrink: f64,
    /// Least draft on a male tool (the part shrinks onto it) and a female one (away from it), radians.
    pub draft_male: f64,
    pub draft_female: f64,
    /// Least radius of a convex edge of the tool's surface (the sheet stretches round it) and of
    /// a concave one (it bridges into it).
    pub min_radius: PerT,
    pub min_inside_radius: PerT,
    /// Largest depth over the opening's least width (H:D), and areal draw ratio.
    pub max_hd: f64,
    pub max_adr: f64,
    /// Least gap between raised features over the depth between them (webbing).
    pub min_gap_ratio: f64,
    /// Largest vent hole across, before it marks the part.
    pub max_vent: PerT,
    /// The thinnest wall a formed part may come to, as a fraction of the sheet.
    pub min_wall: f64,
    /// Constitutive fits, each at its temperature, and measured modulus curves.
    pub fits: Vec<Fit>,
    pub curves: Vec<Curve>,
    /// For the shrink band (stage 4A): the glass transition and the hottest a part is taken off
    /// the mould at, C, and the linear expansion at room temperature, 1/K.
    pub tg: Option<f64>,
    pub demould: Option<f64>,
    pub cte: Option<f64>,
    /// Fields that are guesses, by name: shown on every check that reads them.
    pub stand_in: Vec<&'static str>,
}

/// A fitted law at a temperature, as published.
#[derive(Clone, Debug, PartialEq)]
pub struct Fit {
    pub law: crate::stretch::Law,
    pub temperature: f64,
    /// The test it was fitted to (bubble inflation, biaxial, uniaxial, DMA) and the source.
    pub test: String,
    pub source: String,
    pub stand_in: bool,
}

/// Measured points: the storage (Young's) modulus E' in MPa against temperature in deg C.
#[derive(Clone, Debug, PartialEq)]
pub struct Curve {
    pub points: Vec<(f64, f64)>,
    pub source: String,
}

/// The law the thickness model uses at a temperature, and where it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct LawAt {
    pub law: crate::stretch::Law,
    pub source: String,
    pub stand_in: bool,
}

/// A fit this far from the temperature asked is still taken (the nearest); further, a curve's.
const FIT_REACH: f64 = 15.0;

impl Material {
    /// The part's linear shrink after forming, as a band (stage 4A, amorphous): it contracts
    /// freely from where it set, somewhere between being taken off the mould and its glass
    /// transition, to room temperature (23 C). None without those (a semi-crystalline sheet's
    /// shrink is its crystallising, not this: the vendor's `shrink` stands).
    pub fn shrink_band(&self) -> Option<(f64, f64)> {
        let (tg, off, a) = (self.tg?, self.demould?, self.cte?);
        Some((a * (off.min(tg) - 23.0), a * (tg - 23.0)))
    }

    /// The sheet's normal forming temperature (its source's), else the window's middle.
    pub fn forming_temperature(&self) -> f64 {
        self.normal.unwrap_or(0.5 * (self.forming.0 + self.forming.1))
    }

    /// The law at `temp`: the nearest fit within 15 deg C, else neo-Hookean with G = E'/3 read
    /// off a modulus curve (the rubbery plateau), else the nearest fit, else a stand-in.
    pub fn law_at(&self, temp: f64) -> LawAt {
        let near = self.fits.iter().min_by(|a, b| {
            (a.temperature - temp)
                .abs()
                .total_cmp(&(b.temperature - temp).abs())
        });
        let of = |f: &Fit| LawAt {
            law: f.law.clone(),
            source: format!("{} ({}, {} C)", f.source, f.test, f.temperature),
            stand_in: f.stand_in,
        };
        if let Some(f) = near
            && (f.temperature - temp).abs() <= FIT_REACH
        {
            return of(f);
        }
        if let Some(c) = self.curves.first()
            && let Some(e) = interpolate(&c.points, temp)
        {
            return LawAt {
                law: crate::stretch::Law::neo_hookean(e / 3.0),
                source: format!("{} (E' at {temp} C, G = E'/3)", c.source),
                stand_in: false,
            };
        }
        match near {
            Some(f) => of(f),
            None => LawAt {
                law: crate::stretch::Law::neo_hookean(0.5),
                source: "stand-in: no fit or modulus curve for this material".into(),
                stand_in: true,
            },
        }
    }
}

/// Linear between points (by increasing x), held at the ends.
fn interpolate(p: &[(f64, f64)], x: f64) -> Option<f64> {
    let (first, last) = (p.first()?, p.last()?);
    if x <= first.0 {
        return Some(first.1);
    }
    if x >= last.0 {
        return Some(last.1);
    }
    p.windows(2)
        .find(|w| x >= w[0].0 && x <= w[1].0)
        .map(|w| w[0].1 + (x - w[0].0) / (w[1].0 - w[0].0) * (w[1].1 - w[0].1))
}

/// The forming machine's limits, each optional.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Machine {
    /// The largest sheet it clamps: (shorter, longer).
    pub max_sheet: Option<(f64, f64)>,
    /// The deepest draw (the platen's travel).
    pub max_depth: Option<f64>,
    /// The sheet held under the clamp frame round the forming area, per side.
    pub clamp: f64,
    /// Sheet gauges stocked (none: any).
    pub gauges: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FormShop {
    pub materials: Vec<Material>,
    pub machine: Machine,
}

impl FormShop {
    pub fn material(&self, name: &str) -> Option<&Material> {
        self.materials.iter().find(|m| m.name.eq_ignore_ascii_case(name))
    }
}

/// The thermoforming keys a material record carries (`thermo.<key>`), as stand-in names.
pub const KEYS: [&str; 18] = [
    "tg",
    "demould",
    "window",
    "normal",
    "shrink",
    "max_adr",
    "draft_male",
    "draft_female",
    "min_radius",
    "min_inside_radius",
    "max_hd",
    "min_gap",
    "max_vent",
    "min_wall",
    "law",
    "mu",
    "alpha",
    "law_temp",
];

/// What in a record could not be read: the material and the key.
#[derive(Clone, Debug, PartialEq)]
pub struct DataError {
    pub record: String,
    pub field: String,
}

/// A record's thermoforming values (forge-material's `thermo.*` keys), or None if it has none.
pub fn from_record(r: &forge_material::Material) -> Result<Option<Material>, DataError> {
    use forge_material::{Kind, Value};
    if r.extra("thermo.window").is_none() {
        return Ok(None);
    }
    let fail = |k: &str| DataError {
        record: r.name.clone(),
        field: k.into(),
    };
    let nums = |k: &str| -> Result<Vec<f64>, DataError> {
        match r.extra(&format!("thermo.{k}")) {
            Some(Value::Num(v)) => Ok(vec![*v]),
            Some(Value::Nums(v)) => Ok(v.clone()),
            _ => Err(fail(k)),
        }
    };
    let one = |k: &str| nums(k).map(|v| v[0]);
    let per_t = |k: &str| -> Result<PerT, DataError> {
        let v = nums(k)?;
        Ok(PerT {
            per_t: v[0],
            plus: v.get(1).copied().unwrap_or(0.0),
        })
    };
    let window = nums("window")?;
    if window.len() != 2 {
        return Err(fail("window"));
    }
    let law = match r.extra("thermo.law") {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Err(fail("law")),
    };
    let (mu, alpha) = (nums("mu")?, nums("alpha")?);
    if mu.len() != alpha.len() || (law == "neo_hookean" && mu.len() != 1) {
        return Err(fail("mu"));
    }
    let fits = vec![Fit {
        law: crate::stretch::Law {
            terms: mu.into_iter().zip(alpha).collect(),
        },
        temperature: one("law_temp").unwrap_or(0.5 * (window[0] + window[1])),
        test: match r.extra("thermo.law_test") {
            Some(Value::Str(s)) => s.clone(),
            _ => String::new(),
        },
        source: r.source_of("thermo.law").to_string(),
        stand_in: r.kind_of("thermo.law") == Kind::StandIn,
    }];
    Ok(Some(Material {
        name: r.name.clone(),
        grade: r.note.clone().unwrap_or_default(),
        forming: (window[0], window[1]),
        normal: one("normal").ok(),
        shrink: one("shrink")?,
        draft_male: one("draft_male")?.to_radians(),
        draft_female: one("draft_female")?.to_radians(),
        min_radius: per_t("min_radius")?,
        min_inside_radius: per_t("min_inside_radius")?,
        max_hd: one("max_hd")?,
        max_adr: one("max_adr")?,
        min_gap_ratio: one("min_gap")?,
        max_vent: per_t("max_vent")?,
        min_wall: one("min_wall")?,
        fits,
        curves: vec![],
        tg: one("tg").ok(),
        demould: one("demould").ok(),
        cte: r.cte.map(|c| c * 1e-6),
        stand_in: KEYS
            .into_iter()
            .filter(|k| r.kind_of(&format!("thermo.{k}")) == Kind::StandIn)
            .collect(),
    }))
}

/// The thermoforming keys of a record that carry no source of their own and are not marked
/// stand-ins: the record's source is a datasheet's, not theirs.
pub fn unsourced(r: &forge_material::Material) -> Vec<String> {
    r.extra
        .iter()
        .map(|(k, _)| k)
        .filter(|k| k.starts_with("thermo.") && k.as_str() != "thermo.law_test")
        .filter(|k| r.sources.iter().all(|s| &s.0 != *k) && r.kind_of(k) != forge_material::Kind::StandIn)
        .cloned()
        .collect()
}

/// The shop every thermoform reads unless it is given one: the materials ForgeCAD's library has
/// thermoforming values for, and a placeholder machine (no limits, a 25 mm clamp, any gauge).
pub fn placeholder() -> FormShop {
    shop_of(forge_material::library().iter())
}

/// A shop of the records given (a library's, searched in order), the first of a name kept.
pub fn shop_of<'a>(records: impl Iterator<Item = &'a forge_material::Material>) -> FormShop {
    let mut materials: Vec<Material> = vec![];
    for r in records {
        if let Ok(Some(m)) = from_record(r)
            && !materials.iter().any(|x| x.name.eq_ignore_ascii_case(&m.name))
        {
            materials.push(m);
        }
    }
    FormShop {
        materials,
        machine: Machine {
            clamp: 25.0,
            ..Machine::default()
        },
    }
}
