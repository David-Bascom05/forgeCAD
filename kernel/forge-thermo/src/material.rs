//! What a thermoforming shop forms, as data (data/materials.json): each sheet material's design
//! limits, forming window, shrink and constitutive fits, every number with its source or marked a
//! stand-in, and the machine's limits. Only the shrink reaches geometry (the tool is grown by it);
//! the laws feed the thickness model and the rest the checks.
use crate::data::{Val, parse};

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

/// The material data the kernel ships: every number with its source or marked a stand-in.
pub const DATA: &str = include_str!("../data/materials.json");

/// What in the data could not be read: the material (or "rules") and the field.
#[derive(Clone, Debug, PartialEq)]
pub struct DataError {
    pub record: String,
    pub field: String,
}

fn fail(record: &str, field: &str) -> DataError {
    DataError {
        record: record.into(),
        field: field.into(),
    }
}

/// A sourced value's number (`value`), and whether it is a stand-in.
fn sourced(v: &Val, rec: &str, k: &'static str) -> Result<(Val, bool), DataError> {
    let stand_in = v.get("stand_in").and_then(Val::bool).unwrap_or(false);
    let has = v.get("source").and_then(Val::str).is_some_and(|s| !s.is_empty());
    if !stand_in && !has {
        return Err(fail(rec, k));
    }
    Ok((v.clone(), stand_in))
}

fn value_of(v: &Val, rec: &str, k: &'static str) -> Result<f64, DataError> {
    v.get("value").and_then(Val::num).ok_or(fail(rec, k))
}

fn per_t_of(v: &Val, rec: &str, k: &'static str) -> Result<PerT, DataError> {
    Ok(PerT {
        per_t: v.get("per_t").and_then(Val::num).ok_or(fail(rec, k))?,
        plus: v.get("plus").and_then(Val::num).ok_or(fail(rec, k))?,
    })
}

fn nums(v: Option<&Val>) -> Vec<f64> {
    v.map(|a| a.arr().iter().filter_map(Val::num).collect())
        .unwrap_or_default()
}

/// A law record: Ogden (mu_mpa, alpha), Mooney-Rivlin (c10_mpa, c01_mpa) or neo-Hookean (mu_mpa
/// or c10_mpa), as an Ogden law.
fn law_of(v: &Val, rec: &str) -> Result<crate::stretch::Law, DataError> {
    let model = v.get("model").and_then(Val::str).unwrap_or("");
    let one = |k: &str| v.get(k).and_then(Val::num);
    let terms = match model {
        "ogden" => {
            let (mu, al) = (nums(v.get("mu_mpa")), nums(v.get("alpha")));
            if mu.is_empty() || mu.len() != al.len() {
                return Err(fail(rec, "ogden"));
            }
            mu.into_iter().zip(al).collect()
        }
        "mooney_rivlin" => {
            let (c10, c01) = (
                one("c10_mpa").ok_or(fail(rec, "c10_mpa"))?,
                one("c01_mpa").ok_or(fail(rec, "c01_mpa"))?,
            );
            vec![(2.0 * c10, 2.0), (-2.0 * c01, -2.0)]
        }
        "neo_hookean" => {
            let mu = one("mu_mpa")
                .or(one("c10_mpa").map(|c| 2.0 * c))
                .ok_or(fail(rec, "mu_mpa"))?;
            vec![(mu, 2.0)]
        }
        _ => return Err(fail(rec, "model")),
    };
    Ok(crate::stretch::Law { terms })
}

/// The shop the data describes: its materials, and a placeholder machine.
pub fn from_data(text: &str) -> Result<FormShop, DataError> {
    let d = parse(text).map_err(|e| fail("", e.want))?;
    let rules = d.get("rules").ok_or(fail("rules", "rules"))?;
    let mut out = vec![];
    for m in d.get("materials").map(Val::arr).unwrap_or(&[]) {
        let name = m
            .get("name")
            .and_then(Val::str)
            .ok_or(fail("?", "name"))?
            .to_string();
        let mut stand_in = vec![];
        // the material's own field, else the rule every material shares
        let mut field = |k: &'static str| -> Result<Val, DataError> {
            let (v, guess) = match m.get(k) {
                Some(v) => sourced(v, &name, k)?,
                None => sourced(rules.get(k).ok_or(fail(&name, k))?, "rules", k)?,
            };
            if guess {
                stand_in.push(k);
            }
            Ok(v)
        };
        let forming = field("forming_c")?;
        let shrink = field("shrink")?;
        let mut mat = Material {
            grade: m.get("grade").and_then(Val::str).unwrap_or("").to_string(),
            forming: (
                forming
                    .get("min")
                    .and_then(Val::num)
                    .ok_or(fail(&name, "forming_c"))?,
                forming
                    .get("max")
                    .and_then(Val::num)
                    .ok_or(fail(&name, "forming_c"))?,
            ),
            normal: forming.get("normal").and_then(Val::num),
            shrink: value_of(&shrink, &name, "shrink")?,
            draft_male: value_of(&field("draft_male_deg")?, &name, "draft_male_deg")?.to_radians(),
            draft_female: value_of(&field("draft_female_deg")?, &name, "draft_female_deg")?.to_radians(),
            min_radius: per_t_of(&field("min_radius")?, &name, "min_radius")?,
            min_inside_radius: per_t_of(&field("min_inside_radius")?, &name, "min_inside_radius")?,
            max_hd: value_of(&field("max_hd")?, &name, "max_hd")?,
            max_adr: value_of(&field("max_adr")?, &name, "max_adr")?,
            min_gap_ratio: value_of(&field("min_gap_ratio")?, &name, "min_gap_ratio")?,
            max_vent: per_t_of(&field("max_vent")?, &name, "max_vent")?,
            min_wall: value_of(&field("min_wall")?, &name, "min_wall")?,
            fits: vec![],
            curves: vec![],
            name: name.clone(),
            stand_in: vec![],
        };
        for f in m.get("laws").map(Val::arr).unwrap_or(&[]) {
            let (f, guess) = sourced(f, &name, "laws")?;
            mat.fits.push(Fit {
                law: law_of(&f, &name)?,
                temperature: f
                    .get("temperature_c")
                    .and_then(Val::num)
                    .ok_or(fail(&name, "temperature_c"))?,
                test: f.get("test").and_then(Val::str).unwrap_or("").to_string(),
                source: f.get("source").and_then(Val::str).unwrap_or("").to_string(),
                stand_in: guess,
            });
        }
        for c in m.get("curves").map(Val::arr).unwrap_or(&[]) {
            let (c, _) = sourced(c, &name, "curves")?;
            if c.get("kind").and_then(Val::str) != Some("storage_modulus") {
                continue;
            }
            let mut points: Vec<(f64, f64)> = c
                .get("points")
                .map(|p| {
                    p.arr()
                        .iter()
                        .filter_map(|q| Some((q.arr().first()?.num()?, q.arr().get(1)?.num()?)))
                        .collect()
                })
                .unwrap_or_default();
            points.sort_by(|a, b| a.0.total_cmp(&b.0));
            mat.curves.push(Curve {
                points,
                source: c.get("source").and_then(Val::str).unwrap_or("").to_string(),
            });
        }
        if mat.law_at(mat.forming_temperature()).stand_in {
            stand_in.push("law");
        }
        mat.stand_in = stand_in;
        out.push(mat);
    }
    Ok(FormShop {
        materials: out,
        machine: Machine {
            clamp: 25.0,
            ..Machine::default()
        },
    })
}

/// The shop every thermoform reads unless it is given one: the shipped data, and a placeholder
/// machine (no limits, a 25 mm clamp, a stand-in; any gauge).
pub fn placeholder() -> FormShop {
    from_data(DATA).expect("the shipped material data reads")
}
