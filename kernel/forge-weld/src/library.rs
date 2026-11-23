//! The standard section library: sizes restated from the public dimension tables of the European
//! standards (our numbers, rounded as the tables give them), looked up by designation.
//!
//! - SHS, RHS: cold-formed square and rectangular hollow sections, EN 10219-2 sizes, corner
//!   radii by its rule for wall t (outer 2t, inner t up to 6 mm; 2.5t, 1.5t to 10; 3t, 2t over).
//! - CHS: cold-formed circular hollow sections, EN 10219-2 outside diameters.
//! - L: hot-rolled angles, EN 10056-1: root radius r1, toe radius r2 (about r1 / 2).
//! - UPE: parallel-flange channels, EN 10365.
//! - IPE: parallel-flange I-beams, EN 10365.
//! - FB: flat bar, EN 10058 widths and thicknesses, sharp edges.
//!
//! Designations: `SHS 40x3` or `SHS 40x40x3`, `RHS 60x40x3`, `CHS 48.3x3.2`, `L 50x5` or
//! `L 50x50x5`, `L 60x40x5`, `UPE 100`, `IPE 200`, `FB 40x5`.

use crate::section::{Section, SectionError, cold_formed_radii};

/// SHS: (side, wall).
const SHS: &[(f64, f64)] = &[
    (20.0, 2.0),
    (25.0, 2.0),
    (25.0, 2.5),
    (30.0, 2.0),
    (30.0, 3.0),
    (40.0, 2.0),
    (40.0, 3.0),
    (40.0, 4.0),
    (50.0, 3.0),
    (50.0, 4.0),
    (50.0, 5.0),
    (60.0, 3.0),
    (60.0, 4.0),
    (60.0, 5.0),
    (70.0, 4.0),
    (80.0, 4.0),
    (80.0, 5.0),
    (80.0, 6.0),
    (90.0, 5.0),
    (100.0, 4.0),
    (100.0, 5.0),
    (100.0, 6.0),
    (100.0, 8.0),
    (120.0, 5.0),
    (120.0, 6.0),
    (120.0, 8.0),
    (140.0, 6.0),
    (150.0, 6.0),
    (150.0, 8.0),
    (150.0, 10.0),
    (200.0, 8.0),
    (200.0, 10.0),
    (200.0, 12.5),
];

/// RHS: (height, width, wall); the first size the longer.
const RHS: &[(f64, f64, f64)] = &[
    (40.0, 20.0, 2.0),
    (40.0, 20.0, 3.0),
    (50.0, 25.0, 2.0),
    (50.0, 25.0, 3.0),
    (50.0, 30.0, 3.0),
    (60.0, 30.0, 3.0),
    (60.0, 40.0, 3.0),
    (60.0, 40.0, 4.0),
    (70.0, 40.0, 3.0),
    (80.0, 40.0, 3.0),
    (80.0, 40.0, 4.0),
    (80.0, 60.0, 4.0),
    (90.0, 50.0, 4.0),
    (100.0, 50.0, 4.0),
    (100.0, 50.0, 5.0),
    (100.0, 60.0, 4.0),
    (120.0, 60.0, 4.0),
    (120.0, 60.0, 5.0),
    (120.0, 80.0, 5.0),
    (140.0, 80.0, 5.0),
    (150.0, 100.0, 6.0),
    (160.0, 80.0, 6.0),
    (200.0, 100.0, 6.0),
    (200.0, 100.0, 8.0),
];

/// CHS: (outside diameter, wall).
const CHS: &[(f64, f64)] = &[
    (21.3, 2.0),
    (21.3, 2.6),
    (26.9, 2.0),
    (26.9, 2.6),
    (33.7, 2.6),
    (33.7, 3.2),
    (42.4, 2.6),
    (42.4, 3.2),
    (48.3, 2.6),
    (48.3, 3.2),
    (48.3, 4.0),
    (60.3, 3.2),
    (60.3, 4.0),
    (76.1, 3.2),
    (76.1, 4.0),
    (88.9, 3.2),
    (88.9, 4.0),
    (88.9, 5.0),
    (114.3, 4.0),
    (114.3, 5.0),
    (139.7, 5.0),
    (168.3, 5.0),
    (168.3, 6.3),
];

/// L: (long leg, short leg, thickness, root radius, toe radius).
const ANGLE: &[(f64, f64, f64, f64, f64)] = &[
    (20.0, 20.0, 3.0, 3.5, 2.0),
    (25.0, 25.0, 3.0, 3.5, 2.0),
    (25.0, 25.0, 4.0, 3.5, 2.0),
    (30.0, 30.0, 3.0, 5.0, 2.5),
    (30.0, 30.0, 4.0, 5.0, 2.5),
    (35.0, 35.0, 4.0, 5.0, 2.5),
    (40.0, 40.0, 4.0, 6.0, 3.0),
    (40.0, 40.0, 5.0, 6.0, 3.0),
    (45.0, 45.0, 4.5, 7.0, 3.5),
    (50.0, 50.0, 4.0, 7.0, 3.5),
    (50.0, 50.0, 5.0, 7.0, 3.5),
    (50.0, 50.0, 6.0, 7.0, 3.5),
    (60.0, 60.0, 5.0, 8.0, 4.0),
    (60.0, 60.0, 6.0, 8.0, 4.0),
    (60.0, 60.0, 8.0, 8.0, 4.0),
    (70.0, 70.0, 7.0, 9.0, 4.5),
    (80.0, 80.0, 8.0, 10.0, 5.0),
    (90.0, 90.0, 9.0, 11.0, 5.5),
    (100.0, 100.0, 10.0, 12.0, 6.0),
    (120.0, 120.0, 12.0, 13.0, 6.5),
    (40.0, 25.0, 4.0, 4.0, 2.0),
    (45.0, 30.0, 4.0, 4.5, 2.0),
    (50.0, 30.0, 5.0, 5.0, 2.5),
    (60.0, 30.0, 5.0, 5.0, 2.5),
    (60.0, 40.0, 5.0, 6.0, 3.0),
    (60.0, 40.0, 6.0, 6.0, 3.0),
    (65.0, 50.0, 5.0, 6.0, 3.0),
    (75.0, 50.0, 6.0, 7.0, 3.5),
    (80.0, 40.0, 6.0, 7.0, 3.5),
    (80.0, 60.0, 7.0, 8.0, 4.0),
    (100.0, 50.0, 8.0, 9.0, 4.5),
    (100.0, 65.0, 7.0, 10.0, 5.0),
    (100.0, 75.0, 8.0, 10.0, 5.0),
    (120.0, 80.0, 10.0, 11.0, 5.5),
];

/// UPE: (size, h, b, tw, tf, r).
const UPE: &[(u32, f64, f64, f64, f64, f64)] = &[
    (80, 80.0, 50.0, 4.0, 7.0, 10.0),
    (100, 100.0, 55.0, 4.5, 7.5, 10.0),
    (120, 120.0, 60.0, 5.0, 8.0, 12.0),
    (140, 140.0, 65.0, 5.0, 9.0, 12.0),
    (160, 160.0, 70.0, 5.5, 9.5, 12.0),
    (180, 180.0, 75.0, 5.5, 10.5, 12.0),
    (200, 200.0, 80.0, 6.0, 11.0, 13.0),
    (220, 220.0, 85.0, 6.5, 12.0, 13.0),
    (240, 240.0, 90.0, 7.0, 12.5, 15.0),
    (270, 270.0, 95.0, 7.5, 13.5, 15.0),
    (300, 300.0, 100.0, 9.5, 15.0, 15.0),
];

/// IPE: (size, h, b, tw, tf, r).
const IPE: &[(u32, f64, f64, f64, f64, f64)] = &[
    (80, 80.0, 46.0, 3.8, 5.2, 5.0),
    (100, 100.0, 55.0, 4.1, 5.7, 7.0),
    (120, 120.0, 64.0, 4.4, 6.3, 7.0),
    (140, 140.0, 73.0, 4.7, 6.9, 7.0),
    (160, 160.0, 82.0, 5.0, 7.4, 9.0),
    (180, 180.0, 91.0, 5.3, 8.0, 9.0),
    (200, 200.0, 100.0, 5.6, 8.5, 12.0),
    (220, 220.0, 110.0, 5.9, 9.2, 12.0),
    (240, 240.0, 120.0, 6.2, 9.8, 15.0),
    (270, 270.0, 135.0, 6.6, 10.2, 15.0),
    (300, 300.0, 150.0, 7.1, 10.7, 15.0),
    (330, 330.0, 160.0, 7.5, 11.5, 18.0),
    (360, 360.0, 170.0, 8.0, 12.7, 18.0),
    (400, 400.0, 180.0, 8.6, 13.5, 21.0),
];

/// FB widths and thicknesses stocked (every pair with the thickness under the width).
const FB_W: &[f64] = &[
    10.0, 12.0, 15.0, 16.0, 20.0, 25.0, 30.0, 35.0, 40.0, 45.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0, 120.0,
    150.0, 200.0,
];
const FB_T: &[f64] = &[2.0, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0, 12.0, 15.0, 20.0, 25.0];

fn nums(s: &str) -> Option<Vec<f64>> {
    s.split(['x', 'X', '*'])
        .map(|p| p.trim().parse::<f64>().ok().filter(|v| v.is_finite()))
        .collect()
}

fn same(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn fmt(v: f64) -> String {
    let s = format!("{v}");
    s.strip_suffix(".0").map(str::to_string).unwrap_or(s)
}

/// The library's section by its designation (any case, spaces between the family and the sizes
/// optional): its name written the library's way.
pub fn lookup(designation: &str) -> Result<Section, SectionError> {
    let unknown = || SectionError::UnknownSize(designation.to_string());
    let d = designation.trim();
    let split = d.find(|c: char| c.is_ascii_digit()).ok_or_else(unknown)?;
    let (fam, rest) = d.split_at(split);
    let fam = fam.trim().to_ascii_uppercase();
    let n = nums(rest).ok_or_else(unknown)?;
    match (fam.as_str(), &n[..]) {
        ("SHS", &[a, t]) | ("SHS", &[a, _, t]) if n.len() == 2 || same(n[0], n[1]) => {
            SHS.iter()
                .find(|r| same(r.0, a) && same(r.1, t))
                .ok_or_else(unknown)?;
            let (ro, ri) = cold_formed_radii(t);
            Section::rhs(format!("SHS {}x{}x{}", fmt(a), fmt(a), fmt(t)), a, a, t, ro, ri)
        }
        ("RHS", &[h, w, t]) => {
            RHS.iter()
                .find(|r| same(r.0, h) && same(r.1, w) && same(r.2, t))
                .ok_or_else(unknown)?;
            let (ro, ri) = cold_formed_radii(t);
            // the longer side up (y), as a beam stands
            Section::rhs(format!("RHS {}x{}x{}", fmt(h), fmt(w), fmt(t)), w, h, t, ro, ri)
        }
        ("CHS", &[od, t]) => {
            CHS.iter()
                .find(|r| same(r.0, od) && same(r.1, t))
                .ok_or_else(unknown)?;
            Section::chs(format!("CHS {}x{}", fmt(od), fmt(t)), od, t)
        }
        ("L", &[a, t]) | ("L", &[a, _, t]) if n.len() == 2 || same(n[0], n[1]) => {
            angle(a, a, t).ok_or_else(unknown)?
        }
        ("L", &[a, b, t]) => angle(a, b, t).ok_or_else(unknown)?,
        ("UPE", &[h]) => {
            let r = UPE.iter().find(|r| same(r.0 as f64, h)).ok_or_else(unknown)?;
            Section::channel(format!("UPE {}", r.0), r.1, r.2, r.3, r.4, r.5)
        }
        ("IPE", &[h]) => {
            let r = IPE.iter().find(|r| same(r.0 as f64, h)).ok_or_else(unknown)?;
            Section::beam(format!("IPE {}", r.0), r.1, r.2, r.3, r.4, r.5)
        }
        ("FB", &[w, t]) => {
            if !(FB_W.iter().any(|&x| same(x, w)) && FB_T.iter().any(|&x| same(x, t)) && t < w) {
                return Err(unknown());
            }
            Section::flat(format!("FB {}x{}", fmt(w), fmt(t)), w, t)
        }
        _ => Err(unknown()),
    }
}

fn angle(a: f64, b: f64, t: f64) -> Option<Result<Section, SectionError>> {
    let r = ANGLE
        .iter()
        .find(|r| same(r.0, a) && same(r.1, b) && same(r.2, t))?;
    Some(Section::angle(
        format!("L {}x{}x{}", fmt(a), fmt(b), fmt(t)),
        r.0,
        r.1,
        r.2,
        r.3,
        r.4,
    ))
}

/// Every designation the library holds, in its families' order.
pub fn designations() -> Vec<String> {
    let mut out = Vec::new();
    out.extend(
        SHS.iter()
            .map(|r| format!("SHS {}x{}x{}", fmt(r.0), fmt(r.0), fmt(r.1))),
    );
    out.extend(
        RHS.iter()
            .map(|r| format!("RHS {}x{}x{}", fmt(r.0), fmt(r.1), fmt(r.2))),
    );
    out.extend(CHS.iter().map(|r| format!("CHS {}x{}", fmt(r.0), fmt(r.1))));
    out.extend(
        ANGLE
            .iter()
            .map(|r| format!("L {}x{}x{}", fmt(r.0), fmt(r.1), fmt(r.2))),
    );
    out.extend(UPE.iter().map(|r| format!("UPE {}", r.0)));
    out.extend(IPE.iter().map(|r| format!("IPE {}", r.0)));
    for &w in FB_W {
        for &t in FB_T.iter().filter(|&&t| t < w) {
            out.push(format!("FB {}x{}", fmt(w), fmt(t)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_designation_builds_and_reads_back() {
        for d in designations() {
            let s = lookup(&d).unwrap_or_else(|e| panic!("{d}: {e:?}"));
            assert_eq!(s.name, d);
            assert!(s.area() > 0.0, "{d}");
            let (lo, hi) = s.bbox();
            // centred on its box
            assert!((lo.x + hi.x).abs() < 1e-9 && (lo.y + hi.y).abs() < 1e-9, "{d}");
        }
    }

    #[test]
    fn short_forms_and_refusals() {
        assert_eq!(lookup("shs 40x3").unwrap().name, "SHS 40x40x3");
        assert_eq!(lookup("L50x5").unwrap().name, "L 50x50x5");
        assert_eq!(lookup("IPE200").unwrap().name, "IPE 200");
        for bad in ["SHS 41x3", "IPE 210", "XYZ 10", "SHS", "FB 10x12", "SHS 40x30x3"] {
            assert!(matches!(lookup(bad), Err(SectionError::UnknownSize(_))), "{bad}");
        }
        // EN 10219 corners on an SHS 40 x 3: outer 6, inner 3
        let s = lookup("SHS 40x3").unwrap();
        let pi = forge_math::dmath::PI;
        let want = 1600.0 - (4.0 - pi) * 36.0 - (34.0 * 34.0 - (4.0 - pi) * 9.0);
        assert!((s.area() - want).abs() < 1e-9);
    }
}
