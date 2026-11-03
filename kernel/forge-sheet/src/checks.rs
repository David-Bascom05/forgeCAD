//! A sheet against what the shop can make (`ShopProfile`): its thickness stocked, each bend's
//! radius tooled, each flange long enough for the die. Violations name the feature and entity
//! with the measured value and the limit; they never change geometry.
//!
//! Not checked yet: holes and slots against bends, edges and the shop's minimum slot
//! (`HoleNearBend`, `HoleNearEdge`, `SlotTooNarrow` are typed but nothing produces them).
use crate::error::{Side, Violation};
use crate::graph::{JointKind, Sheet, straight};
use crate::shop::ShopProfile;
use forge_math::Point2;

/// Radii match the tooled ones to this.
const RADIUS_MATCH: f64 = 1e-9;

pub fn check(sheet: &Sheet, shop: &ShopProfile) -> Vec<Violation> {
    let feature = sheet.feature;
    let t = sheet.gauge.thickness;
    let mut out = Vec::new();
    let Some(stock) = shop.stock(t) else {
        out.push(Violation::ThicknessNotStocked {
            feature,
            thickness: t,
            nearest: shop.nearest_thickness(t).unwrap_or(f64::NAN),
        });
        return out;
    };
    let min_flange = stock.min_flange.at(t);
    for j in &sheet.joints {
        let JointKind::Bend { radius, .. } = j.kind else {
            continue;
        };
        if !stock.radii.iter().any(|r| (r - radius).abs() <= RADIUS_MATCH) {
            let nearest = stock
                .radii
                .iter()
                .copied()
                .min_by(|a, b| (a - radius).abs().total_cmp(&(b - radius).abs()))
                .unwrap_or(f64::NAN);
            out.push(Violation::RadiusNotTooled {
                feature,
                joint: j.id,
                radius,
                nearest,
            });
        }
        for side in [Side::A, Side::B] {
            let (wall, segment) = match side {
                Side::A => (j.a, j.on_a),
                Side::B => (j.b, j.on_b),
            };
            let Some(w) = sheet.wall(wall) else { continue };
            let Ok((p0, p1)) = straight(w, segment) else {
                continue;
            };
            let length = reach(w, p0, p1);
            if length < min_flange {
                out.push(Violation::FlangeTooShort {
                    feature,
                    wall,
                    joint: j.id,
                    length,
                    limit: min_flange,
                });
            }
        }
    }
    out
}

/// How far the wall's outline reaches from the mold line p0 -> p1 (the wall on its left).
fn reach(w: &crate::graph::Wall, p0: Point2, p1: Point2) -> f64 {
    let d = (p1 - p0) * (1.0 / p0.distance(p1));
    let left = |q: Point2| d.cross(q - p0);
    w.outline
        .iter()
        .flat_map(|l| &l.segments)
        .flat_map(|(s, _)| s.samples(16))
        .map(left)
        .fold(0.0, f64::max)
}
