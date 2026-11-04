//! What a thermoforming shop forms, as data: each sheet material's design limits and the machine's.
//! Only the shrink reaches geometry (the tool is grown by it); the rest is read by the checks.
//! Sources are in research/thermoform/README.md; a row or field marked `stand_in` is a guess in the
//! range the guides give, not a sourced number.

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
    /// The sheet temperature range it forms in, deg C (reported, not checked).
    pub forming: (f64, f64),
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
    /// The thinnest wall a formed part may come to, as a fraction of the sheet (for the thickness model).
    pub min_wall: f64,
    /// The sheet's law at its forming temperature, for the thickness model.
    pub law: crate::stretch::Law,
    /// Fields that are guesses, by name: shown on every check that reads them.
    pub stand_in: Vec<&'static str>,
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

fn row(name: &str, forming: (f64, f64), shrink: f64, max_adr: f64) -> Material {
    Material {
        name: name.into(),
        forming,
        shrink,
        draft_male: 4f64.to_radians(),
        draft_female: 2f64.to_radians(),
        min_radius: PerT {
            per_t: 1.0,
            plus: 0.8,
        },
        min_inside_radius: PerT {
            per_t: 0.0,
            plus: 0.4,
        },
        max_hd: 1.0,
        max_adr,
        min_gap_ratio: 1.0,
        max_vent: PerT {
            per_t: 0.5,
            plus: 0.0,
        },
        min_wall: 0.25,
        law: crate::stretch::Law::neo_hookean(0.5),
        stand_in: vec!["max_adr", "min_wall", "law"],
    }
}

/// The table every thermoform reads unless it is given a shop. The design guides give one set of
/// rules for every material (draft 4 deg male, 2 deg female; radii t + 0.8 outside, 0.4 inside;
/// H:D 1; a gap as wide as the depth between raised parts; vents under t / 2: Universal Plastics,
/// Eastman, Formech); forming temperatures and shrink are Formech's and Spartech's per material
/// (shrink the middle of their ranges). The areal draw ratios are one vendor's "typical safe"
/// figures (RapidMade), the thinnest wall a guess: both stand-ins. A placeholder machine: no
/// limits, a 25 mm clamp (a stand-in), any gauge.
pub fn placeholder() -> FormShop {
    FormShop {
        materials: vec![
            row("ABS", (150.0, 180.0), 0.006, 3.0),
            row("HIPS", (150.0, 175.0), 0.005, 3.5),
            row("PETG", (120.0, 160.0), 0.005, 2.5),
            row("PC", (170.0, 205.0), 0.007, 2.0),
            row("PMMA", (150.0, 175.0), 0.006, 2.0),
            row("HDPE", (150.0, 180.0), 0.03, 1.6),
        ],
        machine: Machine {
            clamp: 25.0,
            ..Machine::default()
        },
    }
}
