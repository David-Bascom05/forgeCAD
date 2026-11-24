//! Cutting data per ISO 513 material group and the feeds engine.
//!
//! The formulas are the standard ones (libraries-machines-posts-feeds.md §3.2):
//! - spindle n = 1000 vc / (π D_eff) rpm, D_eff = D, or 2 √(ap (D − ap)) for a ball at depth ap;
//! - radial chip thinning: for ae < D/2 the mean chip is thinner than fz by
//!   k = 2 √(ae/D − (ae/D)²), so the feed per tooth that keeps the chip is fz / k (capped);
//! - feed vf = fz n z (mm/min);
//! - engagement angle φ = acos(1 − 2 ae/D) (the chord at radial depth ae; φ = π for a slot);
//! - peak chip hex = fz sin φ for ae < D/2, fz beyond (the thinning factor k above is sin φ);
//! - mean chip hm = (1/φ) ∫ fz sin θ dθ over the arc = fz (1 − cos φ)/φ = 2 fz (ae/D)/φ, the same
//!   for up and down milling (2 fz/π for a slot). This is the chip Kienzle's kc is taken at
//!   (Sandvik Coromant, "Milling formulas and definitions": Pc uses kc at hm; the integral is
//!   Martellotti's circular-path chip h(θ) = fz sin θ, derived in
//!   research/cam/toolpath-quality/feeds-speeds.md §1.1);
//! - power Pc = ap ae vf kc / 6e7 kW, kc = kc1.1 hm^(−mc) (Kienzle 1952), hm the MEAN chip;
//! - torque Mc = Pc 30e3 / (π n) N·m (Sandvik, same page).
//!
//! kc falls as the chip grows and hm < hex, so taking kc at hex would understate the power by
//! (hex/hm)^mc: ×1.12 for a slot and ×1.18 at ae = D/10 with mc = 0.25. Tests below check the
//! worked example of feeds-speeds.md §11 (1/2" 3-flute carbide in 6061).
//!
//! The data is per group, in our own numbers restated from public handbook ranges; it is a
//! starting point. Every value is marked "verify with the tool maker".
use crate::toml::{Table, num, nums, text};
use crate::{Cite, LibError, cite};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Group {
    /// P M K N S H.
    pub group: String,
    pub name: String,
    /// Cutting speed for carbide and for HSS, m/min.
    pub vc_carbide: f64,
    pub vc_hss: f64,
    /// Feed per tooth at 1 mm and at 20 mm diameter (linear between, clamped outside), mm.
    pub fz_d1: f64,
    pub fz_d20: f64,
    /// Specific cutting force at 1 mm chip, N/mm², and its exponent.
    pub kc1_1: f64,
    pub mc: f64,
    /// Measured spread of kc1.1 and mc for the group's cited material, where one is cited: the
    /// power is also checked at the upper ends (h^(−mc) rises with mc for h < 1 mm, and the
    /// power rises with kc, so the upper ends bound it within the data: feeds-speeds.md §10.5).
    pub kc1_1_range: Option<[f64; 2]>,
    pub mc_range: Option<[f64; 2]>,
    pub source: String,
    pub note: String,
    /// Every value cited (cite.rs): `real = true` records must cite all of them.
    pub real: bool,
    pub cites: Vec<Cite>,
}

impl Group {
    pub fn from_table(t: &Table) -> Result<Group, LibError> {
        let g = text(t, "group")
            .ok_or_else(|| LibError::Missing {
                record: "[[group]]".into(),
                field: "group",
                fix: "give the ISO 513 letter".into(),
            })?
            .to_string();
        let rec = format!("cutting group {g}");
        let n = |k: &'static str| {
            num(t, k).ok_or_else(|| LibError::Missing {
                record: rec.clone(),
                field: k,
                fix: format!("give `{k}`"),
            })
        };
        let range = |k: &'static str| -> Result<Option<[f64; 2]>, LibError> {
            match nums(t, k) {
                None => Ok(None),
                Some(v) if v.len() == 2 && v[0] <= v[1] => Ok(Some([v[0], v[1]])),
                Some(_) => Err(LibError::Bad {
                    record: rec.clone(),
                    field: k,
                    why: "is [low, high]".into(),
                }),
            }
        };
        let (kc1_1_range, mc_range) = (range("kc1_1_range")?, range("mc_range")?);
        let mut required: Vec<String> = ["vc_carbide", "vc_hss", "fz_d1", "fz_d20", "kc1_1", "mc"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        if kc1_1_range.is_some() {
            required.push("kc1_1_range".into());
        }
        if mc_range.is_some() {
            required.push("mc_range".into());
        }
        let (real, cites) = cite::read(t, &rec, &required, &[])?;
        Ok(Group {
            kc1_1_range,
            mc_range,
            real,
            cites,
            name: text(t, "name").unwrap_or("").to_string(),
            vc_carbide: n("vc_carbide")?,
            vc_hss: n("vc_hss")?,
            fz_d1: n("fz_d1")?,
            fz_d20: n("fz_d20")?,
            kc1_1: n("kc1_1")?,
            mc: n("mc")?,
            source: text(t, "source").unwrap_or("").to_string(),
            note: text(t, "note").unwrap_or("").to_string(),
            group: g,
        })
    }

    /// kc1.1 and mc at the upper ends of their cited ranges (the point values where none).
    pub fn upper(&self) -> (f64, f64) {
        (
            self.kc1_1_range.map_or(self.kc1_1, |r| r[1].max(self.kc1_1)),
            self.mc_range.map_or(self.mc, |r| r[1].max(self.mc)),
        )
    }

    pub fn fz_at(&self, d: f64) -> f64 {
        let f = ((d - 1.0) / 19.0).clamp(0.0, 1.0);
        self.fz_d1 + f * (self.fz_d20 - self.fz_d1)
    }
}

/// A cutting tool's substrate, read from the record's `material` text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Substrate {
    Carbide,
    /// High-speed steel, cobalt and powder-metal grades included.
    Hss,
    /// Cermet, ceramic, CBN, PCD: no generic speed column (the maker's data or the user's).
    Other,
    Unknown,
}

impl Substrate {
    /// "carbide", "Solid Carbide", "Micrograin Carbide" → Carbide; "HSS", "HSS-Co", "PM-HSS",
    /// "high speed steel" → Hss; cermet, ceramic, CBN, PCD, diamond → Other; else Unknown.
    pub fn of(text: &str) -> Substrate {
        let t = text.to_ascii_lowercase();
        if t.contains("carbide") {
            Substrate::Carbide
        } else if t.contains("hss") || t.contains("high speed") || t.contains("high-speed") {
            Substrate::Hss
        } else if ["cermet", "ceramic", "cbn", "pcd", "diamond"]
            .iter()
            .any(|k| t.contains(k))
        {
            Substrate::Other
        } else {
            Substrate::Unknown
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Substrate::Carbide => "carbide",
            Substrate::Hss => "HSS",
            Substrate::Other => "other",
            Substrate::Unknown => "unknown",
        }
    }
}

/// The ISO 513 group of a forge-material family, by the standard's own classes (P steel,
/// M stainless steel, K cast iron, N non-ferrous, S heat-resistant alloys and titanium,
/// H hardened steel). None for families ISO 513 does not class (plastics, rubber, wood).
/// A hardened steel's H, or a superalloy filed under another family, comes from a `[[stock]]`
/// record, never from this.
pub fn iso_group_of_family(family: &str) -> Option<&'static str> {
    match family {
        "steel" => Some("P"),
        "stainless" => Some("M"),
        "iron" => Some("K"),
        "aluminium" | "copper" => Some("N"),
        "titanium" => Some("S"),
        _ => None,
    }
}

/// kc1.1 and mc at a speed, their spread over the rows ([low, high] each), and how they were read.
pub type Kienzle = (f64, f64, [f64; 2], [f64; 2], String);

/// Cutting data for one stock material (a forge-material name): its ISO 513 group and, where
/// measured, kc1.1 and mc against the cutting speed (rows, linear between, held at the ends).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stock {
    pub material: String,
    pub group: String,
    pub vc_rows: Vec<f64>,
    pub kc1_1_rows: Vec<f64>,
    pub mc_rows: Vec<f64>,
    pub note: String,
    pub real: bool,
    pub cites: Vec<Cite>,
}

impl Stock {
    pub fn from_table(t: &Table) -> Result<Stock, LibError> {
        let material = text(t, "material")
            .ok_or_else(|| LibError::Missing {
                record: "[[stock]]".into(),
                field: "material",
                fix: "give the forge-material name".into(),
            })?
            .to_string();
        let rec = format!("stock `{material}`");
        let group = text(t, "group")
            .ok_or_else(|| LibError::Missing {
                record: rec.clone(),
                field: "group",
                fix: "give the ISO 513 letter".into(),
            })?
            .to_string();
        let rows = |k: &str| nums(t, k).unwrap_or_default();
        let (vc_rows, kc1_1_rows, mc_rows) = (rows("vc_rows"), rows("kc1_1_rows"), rows("mc_rows"));
        if kc1_1_rows.len() != vc_rows.len()
            || mc_rows.len() != vc_rows.len()
            || vc_rows.windows(2).any(|w| w[0] >= w[1])
        {
            return Err(LibError::Bad {
                record: rec,
                field: "vc_rows",
                why: "vc_rows, kc1_1_rows and mc_rows are the same length, vc increasing".into(),
            });
        }
        let mut required = vec!["group".to_string()];
        if !vc_rows.is_empty() {
            required.extend(["vc_rows", "kc1_1_rows", "mc_rows"].map(String::from));
        }
        let (real, cites) = cite::read(t, &rec, &required, &[])?;
        Ok(Stock {
            note: text(t, "note").unwrap_or("").to_string(),
            material,
            group,
            vc_rows,
            kc1_1_rows,
            mc_rows,
            real,
            cites,
        })
    }

    /// kc1.1 and mc at cutting speed vc (linear between rows; the end row outside them, said
    /// in the note), and the rows' full spread. None when the record has no rows.
    pub fn kienzle_at(&self, vc: f64) -> Option<Kienzle> {
        let n = self.vc_rows.len();
        if n == 0 {
            return None;
        }
        let spread = |v: &[f64]| {
            [
                v.iter().copied().fold(f64::INFINITY, f64::min),
                v.iter().copied().fold(0.0, f64::max),
            ]
        };
        let (lo, hi) = (self.vc_rows[0], self.vc_rows[n - 1]);
        let (kc, mc, how) = if vc <= lo || n == 1 {
            (
                self.kc1_1_rows[0],
                self.mc_rows[0],
                format!("the row at vc {lo} (asked {vc:.0}: under the data)"),
            )
        } else if vc >= hi {
            (
                self.kc1_1_rows[n - 1],
                self.mc_rows[n - 1],
                format!("the row at vc {hi} (asked {vc:.0}: over the data)"),
            )
        } else {
            let j = self.vc_rows.iter().position(|&x| x >= vc).unwrap_or(n - 1);
            let (a, b) = (self.vc_rows[j - 1], self.vc_rows[j]);
            let u = (vc - a) / (b - a);
            (
                self.kc1_1_rows[j - 1] + u * (self.kc1_1_rows[j] - self.kc1_1_rows[j - 1]),
                self.mc_rows[j - 1] + u * (self.mc_rows[j] - self.mc_rows[j - 1]),
                format!("between the rows at vc {a} and {b}"),
            )
        };
        Some((kc, mc, spread(&self.kc1_1_rows), spread(&self.mc_rows), how))
    }

    /// The group's record with this material's kc1.1/mc at vc (and their spread as the ranges).
    pub fn apply(&self, g: &Group, vc: f64) -> Group {
        match self.kienzle_at(vc) {
            Some((kc, mc, kr, mr, how)) => {
                let src = self
                    .cites
                    .iter()
                    .find(|c| c.path == "kc1_1_rows")
                    .map_or("", |c| c.source.as_str());
                Group {
                    kc1_1: kc,
                    mc,
                    kc1_1_range: Some(kr),
                    mc_range: Some(mr),
                    source: format!("{} kc1.1/mc {how}: {src}", self.material),
                    ..g.clone()
                }
            }
            None => g.clone(),
        }
    }
}

/// Generic drilling data for one ISO 513 group and drill substrate: the cutting speed and the
/// feed per revolution at stated diameters (cutting.toml `[[drill]]`, cited per value).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrillRow {
    pub group: String,
    /// "carbide" | "hss".
    pub substrate: String,
    pub vc: f64,
    pub d_rows: Vec<f64>,
    /// mm/rev at each of d_rows.
    pub f_rows: Vec<f64>,
    pub real: bool,
    pub cites: Vec<Cite>,
}

impl DrillRow {
    pub fn from_table(t: &Table) -> Result<DrillRow, LibError> {
        let s = |k: &'static str| {
            text(t, k).map(str::to_string).ok_or_else(|| LibError::Missing {
                record: "[[drill]]".into(),
                field: k,
                fix: format!("give `{k}`"),
            })
        };
        let (group, substrate) = (s("group")?, s("substrate")?);
        let rec = format!("drill data {group}/{substrate}");
        let vc = num(t, "vc").ok_or_else(|| LibError::Missing {
            record: rec.clone(),
            field: "vc",
            fix: "give `vc`, m/min".into(),
        })?;
        let (d_rows, f_rows) = (
            nums(t, "d_rows").unwrap_or_default(),
            nums(t, "f_rows").unwrap_or_default(),
        );
        if d_rows.is_empty() || d_rows.len() != f_rows.len() || d_rows.windows(2).any(|w| w[0] >= w[1]) {
            return Err(LibError::Bad {
                record: rec,
                field: "d_rows",
                why: "d_rows and f_rows are the same length (at least one), d increasing".into(),
            });
        }
        let required = ["vc", "d_rows", "f_rows"].map(String::from);
        let (real, cites) = cite::read(t, &rec, &required, &["group", "substrate"])?;
        Ok(DrillRow {
            group,
            substrate,
            vc,
            d_rows,
            f_rows,
            real,
            cites,
        })
    }

    /// Feed per revolution at diameter d: linear between the stated diameters; outside them the
    /// lesser of the linear extension and the nearest row scaled by d (said in the note).
    pub fn f_at(&self, d: f64) -> (f64, String) {
        let (ds, fs) = (&self.d_rows, &self.f_rows);
        let n = ds.len();
        if n == 1 {
            return (fs[0] * d / ds[0], format!("scaled by diameter from Ø{}", ds[0]));
        }
        let lin = |j: usize| fs[j] + (fs[j + 1] - fs[j]) * (d - ds[j]) / (ds[j + 1] - ds[j]);
        if d < ds[0] {
            let f = lin(0).min(fs[0] * d / ds[0]).max(0.0);
            (
                f,
                format!(
                    "under the data's Ø{}: the lesser of the linear and the proportional extension",
                    ds[0]
                ),
            )
        } else if d > ds[n - 1] {
            let f = lin(n - 2).min(fs[n - 1] * d / ds[n - 1]);
            (
                f,
                format!(
                    "over the data's Ø{}: the lesser of the linear and the proportional extension",
                    ds[n - 1]
                ),
            )
        } else {
            let j = ds.iter().position(|&x| x >= d).unwrap_or(n - 1).max(1) - 1;
            (lin(j), format!("between Ø{} and Ø{}", ds[j], ds[j + 1]))
        }
    }
}

/// Generic tapping speed for one ISO 513 group and tap substrate (cutting.toml `[[tap]]`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TapRow {
    pub group: String,
    pub substrate: String,
    pub vc: f64,
    pub real: bool,
    pub cites: Vec<Cite>,
}

impl TapRow {
    pub fn from_table(t: &Table) -> Result<TapRow, LibError> {
        let s = |k: &'static str| {
            text(t, k).map(str::to_string).ok_or_else(|| LibError::Missing {
                record: "[[tap]]".into(),
                field: k,
                fix: format!("give `{k}`"),
            })
        };
        let (group, substrate) = (s("group")?, s("substrate")?);
        let rec = format!("tap data {group}/{substrate}");
        let vc = num(t, "vc").ok_or_else(|| LibError::Missing {
            record: rec.clone(),
            field: "vc",
            fix: "give `vc`, m/min".into(),
        })?;
        let (real, cites) = cite::read(t, &rec, &["vc".to_string()], &["group", "substrate"])?;
        Ok(TapRow {
            group,
            substrate,
            vc,
            real,
            cites,
        })
    }
}

/// What the engine was asked for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cut {
    pub d: f64,
    pub flutes: u32,
    pub ball: bool,
    pub carbide: bool,
    /// Axial and radial engagement, mm.
    pub ap: f64,
    pub ae: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Feeds {
    pub rpm: f64,
    pub vc: f64,
    /// Programmed feed per tooth (after thinning) and the peak chip it gives.
    pub fz: f64,
    pub hex: f64,
    /// Mean chip over the engaged arc, mm: the chip kc and the power are taken at.
    pub hm: f64,
    /// Engagement angle, radians (π for a slot).
    pub phi: f64,
    /// mm/min.
    pub vf: f64,
    /// kW at the spindle (cutting power, before spindle efficiency).
    pub power: f64,
    /// N·m at the spindle.
    pub torque: f64,
    /// The effective diameter the speed was set for.
    pub d_eff: f64,
    /// rpm limited by the machine (the speed is then lower than asked).
    pub rpm_limited: bool,
}

/// The largest factor chip thinning may raise the feed per tooth by.
pub const THIN_CAP: f64 = 3.0;

pub fn feeds(g: &Group, c: &Cut, rpm_range: [f64; 2]) -> Feeds {
    let d_eff = effective_diameter(c.d, c.ball, c.ap);
    let vc0 = if c.carbide { g.vc_carbide } else { g.vc_hss };
    let n0 = 1000.0 * vc0 / (std::f64::consts::PI * d_eff.max(1e-6));
    let rpm = n0.clamp(rpm_range[0], rpm_range[1]);
    let target = g.fz_at(c.d);
    let k = thinning((c.ae / c.d).clamp(0.0, 1.0));
    let fz = if k > 0.0 {
        (target / k).min(THIN_CAP * target)
    } else {
        target
    };
    let mut f = at(g, c, fz, rpm, d_eff);
    f.rpm_limited = rpm != n0;
    f
}

/// The diameter that cuts: D, or for a ball at depth ap < D/2, 2 √(ap (D − ap)).
pub fn effective_diameter(d: f64, ball: bool, ap: f64) -> f64 {
    if ball && ap < 0.5 * d {
        2.0 * (ap * (d - ap)).max(0.0).sqrt()
    } else {
        d
    }
}

/// Peak-chip factor hex/fz = sin φ = 2 √(r − r²) for r = ae/D < 1/2, else 1.
pub fn thinning(r: f64) -> f64 {
    if r < 0.5 {
        2.0 * (r - r * r).max(0.0).sqrt()
    } else {
        1.0
    }
}

/// Where the edge cuts at axial depth ap: the effective diameter and the axial thinning sin κ
/// (the chip normal to the edge is fz sin κ sin θ).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contact {
    pub d_eff: f64,
    pub sin_kappa: f64,
}

/// The contact of a tool of diameter d and corner radius re (0 flat, d/2 ball) at depth ap,
/// tilted by `tilt` degrees (a ball only; other tools are taken untilted). feeds-speeds.md
/// §1.2–1.3 (derived, and checked there against Sandvik's round-insert and ball formulas):
/// for ap < re, D_eff = D − 2 re + 2 √(re² − (re − ap)²) and sin κ = √(1 − (1 − ap/re)²), the
/// local entering angle at the bottom of the cut (for a ball, Sandvik's fz = D hex / D_cap:
/// sin κ = D_eff/D); for a ball tilted by β the contact band runs from polar angle β to β + α,
/// α = acos(1 − 2 ap/D), so D_eff = D sin(min(α + β, 90°)) and sin κ = D_eff/D; else D and 1.
pub fn contact(d: f64, re: f64, ap: f64, tilt: f64) -> Contact {
    let re = re.clamp(0.0, 0.5 * d);
    if re <= 0.0 || ap >= re {
        return Contact {
            d_eff: d,
            sin_kappa: 1.0,
        };
    }
    let ap = ap.max(0.0);
    let ball = re >= 0.5 * d;
    if ball && tilt != 0.0 {
        let alpha = forge_math::dmath::acos_clamped(1.0 - 2.0 * ap / d);
        let a = (alpha + tilt.abs().to_radians()).min(std::f64::consts::FRAC_PI_2);
        let s = forge_math::dmath::sin(a);
        return Contact {
            d_eff: d * s,
            sin_kappa: s,
        };
    }
    let u = 1.0 - ap / re;
    let sk = (1.0 - u * u).max(0.0).sqrt();
    Contact {
        d_eff: d - 2.0 * re + 2.0 * re * sk,
        sin_kappa: sk,
    }
}

/// The numbers of a cut at a given fz and rpm: chips, feed, power, torque.
pub fn at(g: &Group, c: &Cut, fz: f64, rpm: f64, d_eff: f64) -> Feeds {
    at_contact(
        g,
        c,
        fz,
        rpm,
        Contact {
            d_eff,
            sin_kappa: 1.0,
        },
        c.d,
    )
}

/// As `at`, with the contact's axial thinning, and the radial immersion taken against
/// `d_radial` (the diameter the width ae is cut at: D_eff for a ball, Sandvik's side-milling
/// form, D otherwise).
pub fn at_contact(g: &Group, c: &Cut, fz: f64, rpm: f64, k: Contact, d_radial: f64) -> Feeds {
    let d_eff = k.d_eff;
    let r = (c.ae / d_radial.max(1e-9)).clamp(0.0, 1.0);
    let hex = fz * k.sin_kappa * thinning(r);
    let vf = fz * rpm * c.flutes.max(1) as f64;
    let phi = engagement_angle(r);
    let hm = fz * k.sin_kappa * mean_chip_factor(r, phi);
    // the floor only guards kc's pole at a zero chip (ae → 0, where the power → 0 anyway)
    let kc = g.kc1_1 * forge_math::dmath::powf(hm.max(1e-4), -g.mc);
    let power = c.ap * c.ae * vf * kc / 6.0e7;
    let torque = if rpm > 0.0 {
        power * 30.0e3 / (std::f64::consts::PI * rpm)
    } else {
        0.0
    };
    Feeds {
        rpm,
        vc: rpm * std::f64::consts::PI * d_eff / 1000.0,
        fz,
        hex,
        hm,
        phi,
        vf,
        power,
        torque,
        d_eff,
        rpm_limited: false,
    }
}

/// Engagement angle φ = acos(1 − 2r), r = ae/D clamped to [0, 1]; radians.
pub fn engagement_angle(r: f64) -> f64 {
    // dmath: the same bits on native and wasm
    forge_math::dmath::acos_clamped(1.0 - 2.0 * r.clamp(0.0, 1.0))
}

/// hm / fz = (1 − cos φ)/φ = 2r/φ; 0 at no engagement.
pub fn mean_chip_factor(r: f64, phi: f64) -> f64 {
    if phi > 0.0 {
        2.0 * r.clamp(0.0, 1.0) / phi
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thinning_and_ball_diameter() {
        let g = Group {
            group: "N".into(),
            name: "al".into(),
            vc_carbide: 300.0,
            vc_hss: 100.0,
            fz_d1: 0.01,
            fz_d20: 0.1,
            kc1_1: 700.0,
            mc: 0.25,
            source: String::new(),
            note: String::new(),
            ..Group::default()
        };
        let full = feeds(
            &g,
            &Cut {
                d: 10.0,
                flutes: 3,
                ball: false,
                carbide: true,
                ap: 5.0,
                ae: 10.0,
            },
            [0.0, 1e9],
        );
        let light = feeds(
            &g,
            &Cut {
                d: 10.0,
                flutes: 3,
                ball: false,
                carbide: true,
                ap: 5.0,
                ae: 1.0,
            },
            [0.0, 1e9],
        );
        assert!((full.rpm - 9_549.296_585_513_72).abs() < 1e-6);
        // ae/D = 0.1: k = 2 sqrt(0.09) = 0.6
        assert!((light.fz / full.fz - 1.0 / 0.6).abs() < 1e-12);
        assert!((light.hex - full.fz).abs() < 1e-12);
        let ball = feeds(
            &g,
            &Cut {
                d: 10.0,
                flutes: 2,
                ball: true,
                carbide: true,
                ap: 1.0,
                ae: 1.0,
            },
            [0.0, 12_000.0],
        );
        assert!((ball.d_eff - 6.0).abs() < 1e-12 && ball.rpm_limited && ball.rpm == 12_000.0);
    }

    /// The group of feeds-speeds.md §11: 6061 with Helical H45AL-3 data (2100 SFM = 640.08 m/min;
    /// 1/2" slot IPT 0.0032" = 0.08128 mm, SF_H45AL-3.pdf) and the top of Sandvik's N-group kc1.1
    /// range (700 N/mm², mc 0.25, libraries-machines-posts-feeds.md §3.1). fz is flat in D so
    /// the target chip is the cited one at 12.7 mm.
    fn al6061() -> Group {
        Group {
            group: "N".into(),
            name: "6061".into(),
            vc_carbide: 640.08,
            vc_hss: 100.0,
            fz_d1: 0.08128,
            fz_d20: 0.08128,
            kc1_1: 700.0,
            mc: 0.25,
            source: String::new(),
            note: String::new(),
            ..Group::default()
        }
    }

    fn half_inch(ae: f64, ap: f64) -> Cut {
        Cut {
            d: 12.7,
            flutes: 3,
            ball: false,
            carbide: true,
            ap,
            ae,
        }
    }

    /// feeds-speeds.md §11 slot: n clamps 16,043 → 10,000 rpm, hm = 2 fz/π = 0.0517 mm,
    /// kc = 2.10 kc1.1, Q = 393 cm³/min, Pc = 9.6 kW (at kc1.1 = 700), Mc = 9.2 N·m.
    #[test]
    fn handbook_slot() {
        let f = feeds(&al6061(), &half_inch(12.7, 12.7), [0.0, 10_000.0]);
        assert!(f.rpm_limited && f.rpm == 10_000.0);
        assert!((1000.0 * 640.08 / (std::f64::consts::PI * 12.7) - 16_043.0).abs() < 1.0);
        assert!((f.phi - std::f64::consts::PI).abs() < 1e-12);
        assert!((f.hex - 0.08128).abs() < 1e-12);
        assert!((f.hm - 2.0 * 0.08128 / std::f64::consts::PI).abs() < 1e-12);
        assert!((f.hm - 0.0517).abs() < 5e-5);
        assert!((f.vf - 2438.4).abs() < 1e-9);
        let q = 12.7 * 12.7 * f.vf / 1000.0; // cm³/min
        assert!((q - 393.0).abs() < 1.0);
        let kc_ratio = forge_math::dmath::powf(f.hm, -0.25);
        assert!((kc_ratio - 2.10).abs() < 0.005);
        assert!((f.power - 9.6).abs() < 0.05, "{}", f.power);
        assert!((f.torque - 9.2).abs() < 0.05, "{}", f.torque);
    }

    /// feeds-speeds.md §11 adaptive (a): ae = 0.1 D, ap = 2 D, holding the slot chip.
    /// φ = 36.87°, RCTF 1.667, fz = 0.1355 mm, hm = 0.0421 mm, vf = 4064 mm/min,
    /// Q = 131 cm³/min, Pc = 3.4 kW, Mc = 3.2 N·m (kc1.1 = 700).
    #[test]
    fn handbook_adaptive() {
        let f = feeds(&al6061(), &half_inch(1.27, 25.4), [0.0, 10_000.0]);
        assert!((f.phi.to_degrees() - 36.87).abs() < 0.005);
        assert!((f.fz - 0.08128 / 0.6).abs() < 1e-12);
        assert!((f.hex - 0.08128).abs() < 1e-12);
        assert!((f.hm - 0.0421).abs() < 5e-5, "{}", f.hm);
        assert!((f.vf - 4064.0).abs() < 0.1);
        assert!((1.27 * 25.4 * f.vf / 1000.0 - 131.0).abs() < 0.5);
        assert!((f.power - 3.4).abs() < 0.05, "{}", f.power);
        assert!((f.torque - 3.2).abs() < 0.05, "{}", f.torque);
    }

    /// The bug this fixes: kc at the peak chip understated the power by (hex/hm)^mc,
    /// ×1.12 for a slot and ×1.18 at ae = D/10 (feeds-speeds.md §0 finding 1).
    #[test]
    fn power_at_mean_not_peak() {
        let g = al6061();
        for (ae, want) in [(12.7, 1.12), (1.27, 1.18)] {
            let f = feeds(&g, &half_inch(ae, 5.0), [0.0, 10_000.0]);
            let kc_peak = g.kc1_1 * forge_math::dmath::powf(f.hex, -g.mc);
            let old = 5.0 * ae * f.vf * kc_peak / 6.0e7;
            assert!((f.power / old - want).abs() < 0.005, "{}", f.power / old);
        }
    }

    /// The closed form hm = fz (1 − cos φ)/φ against the arc average of fz sin θ, summed
    /// numerically, down milling (θ from π − φ to π), for several immersions.
    #[test]
    fn mean_chip_is_the_arc_average() {
        for r in [0.01, 0.1, 0.25, 0.5, 0.7, 1.0] {
            let phi = engagement_angle(r);
            let n = 200_000;
            let mut sum = 0.0;
            for i in 0..n {
                let th = std::f64::consts::PI - phi + phi * (i as f64 + 0.5) / n as f64;
                sum += forge_math::dmath::sin(th);
            }
            let avg = sum / n as f64;
            assert!((avg - mean_chip_factor(r, phi)).abs() < 1e-9, "r {r}");
            // and the peak factor sin φ (k in the engine) below D/2
            if r < 0.5 {
                assert!((forge_math::dmath::sin(phi) - 2.0 * (r - r * r).sqrt()).abs() < 1e-12);
            }
        }
    }

    /// Ball nose effective diameter, CNCCookbook's example: 1/2" ball at 0.040" deep cuts out
    /// to 0.2713" (feeds-speeds.md §1.2; 2√(0.04·0.46) = 0.27129).
    #[test]
    fn ball_deff_cnccookbook() {
        let f = feeds(
            &al6061(),
            &Cut {
                d: 12.7,
                flutes: 2,
                ball: true,
                carbide: true,
                ap: 0.04 * 25.4,
                ae: 1.0,
            },
            [0.0, 1e9],
        );
        assert!((f.d_eff / 25.4 - 0.27129).abs() < 5e-6, "{}", f.d_eff / 25.4);
    }

    /// Contact geometry: a ball by the bull formula at re = D/2 is the CNCCookbook D_eff; its
    /// sin κ is D_eff/D (Sandvik's ball fz = D hex / D_cap); a bull nose's axial thinning is the
    /// round insert's √(1 − (1 − ap/re)²); a ball tilted 0 is untilted, tilted past 90° − α cuts
    /// at its full diameter.
    #[test]
    fn contact_flat_bull_ball_tilt() {
        let d = 12.7;
        let ap = 0.04 * 25.4;
        let b = contact(d, 0.5 * d, ap, 0.0);
        assert!((b.d_eff - effective_diameter(d, true, ap)).abs() < 1e-12);
        assert!((b.sin_kappa - b.d_eff / d).abs() < 1e-12);
        assert_eq!(
            contact(d, 0.0, 3.0, 0.0),
            Contact {
                d_eff: d,
                sin_kappa: 1.0
            }
        );
        let bull = contact(10.0, 1.0, 0.5, 0.0);
        assert!((bull.sin_kappa - (1.0f64 - 0.25).sqrt()).abs() < 1e-12);
        assert!((bull.d_eff - (8.0 + 2.0 * (1.0f64 - 0.25).sqrt())).abs() < 1e-12);
        assert_eq!(contact(10.0, 1.0, 2.0, 0.0).d_eff, 10.0);
        let t0 = contact(d, 0.5 * d, ap, 1e-300);
        assert!((t0.d_eff - b.d_eff).abs() < 1e-9);
        let t = contact(d, 0.5 * d, ap, 80.0);
        assert!((t.d_eff - d).abs() < 1e-12 && t.sin_kappa == 1.0);
        // 15° of tilt: D sin(α + 15°)
        let alpha = forge_math::dmath::acos(1.0 - 2.0 * ap / d);
        let t = contact(d, 0.5 * d, ap, 15.0);
        assert!((t.d_eff - d * forge_math::dmath::sin(alpha + 15f64.to_radians())).abs() < 1e-12);
    }
}
