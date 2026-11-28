//! Compare a placement's purchasing/design requirements against the selected part.
use super::*;
use crate::electronics_library::evidence::missing;
use sch::characteristics::{Reading, Specification};
const RULE: &str = "DatasheetRequirement";

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 8. * f64::EPSILON * a.abs().max(b.abs())
}
fn same_condition(a: &Reading, b: &Reading) -> bool {
    if a.kind != b.kind
        || a.pins != b.pins
        || a.state != b.state
        || a.kind.starts_with("custom.") && a.unit != b.unit
    {
        return false;
    }
    [(a.min, b.min), (a.typ, b.typ), (a.max, b.max)]
        .into_iter()
        .all(|pair| match pair {
            (Some(x), Some(y)) => near(a.canonical_value(x), b.canonical_value(y)),
            (None, None) => true,
            _ => false,
        })
}
fn conditions_match(a: &Specification, b: &Specification) -> bool {
    if a.conditions.len() != b.conditions.len() {
        return false;
    }
    let mut left: Vec<_> = b.conditions.iter().collect();
    for c in &a.conditions {
        let Some(i) = left.iter().position(|d| same_condition(c, d)) else {
            return false;
        };
        left.swap_remove(i);
    }
    true
}
/// None means the manufacturer did not publish a comparable guarantee.
fn accepts(w: &Reading, g: &Reading) -> Option<bool> {
    if !w.state.is_empty() {
        if w.state == g.state {
            return Some(true);
        }
        if w.kind == "protection" && w.state != "absent" && g.state != "absent" {
            return None;
        }
        return Some(false);
    }
    if w.kind.starts_with("custom.") && w.unit != g.unit {
        return None;
    }
    let mut unknown = false;
    for (want, got, sense) in [(w.min, g.min, -1), (w.typ, g.typ, 0), (w.max, g.max, 1)] {
        if let Some(want) = want {
            let Some(got) = got else {
                unknown = true;
                continue;
            };
            let (a, b) = (g.canonical_value(got), w.canonical_value(want));
            if !near(a, b) && (sense == 0 || sense < 0 && a < b || sense > 0 && a > b) {
                return Some(false);
            }
        }
    }
    (!unknown).then_some(true)
}
pub(super) fn check(doc: &Document, c: &Circuit, out: &mut Vec<Issue>) {
    for (scope, sheet) in &c.occurrences {
        for row in doc.rows.iter().filter(|r| &r.sheet == sheet) {
            let sch::Item::Place {
                symbol,
                spec_limits,
                populated,
                pins,
                ..
            } = &row.item
            else {
                continue;
            };
            let functions = &pins.functions;
            if spec_limits.is_empty() {
                continue;
            }
            let owner = format!("{scope}/{}", row.id);
            if !c.is_fitted(&owner, *populated) {
                continue;
            }
            let sym = &doc.symbols[symbol];
            for want in spec_limits {
                let pins = &want.value.pins;
                let obj = pins
                    .first()
                    .map_or_else(|| owner.clone(), |p| format!("{owner}.{p}"));
                let device = sym.specs.iter().filter(|s| &s.value.pins == pins);
                let at_pin = pins
                    .first()
                    .and_then(|p| sym.pins.get_key_value(p))
                    .into_iter()
                    .flat_map(|(p, pin)| {
                        pin.specs.iter().chain(
                            functions
                                .get(p)
                                .and_then(|f| pin.functions.get(f))
                                .into_iter()
                                .flat_map(|f| &f.specs),
                        )
                    })
                    .filter(|s| &s.value.pins == pins || pins.len() == 1 && s.value.pins.is_empty());
                let mut mismatch = None;
                let mut passed = false;
                for fact in device.chain(at_pin).filter(|s| {
                    s.class == want.class && s.value.kind == want.value.kind && conditions_match(want, s)
                }) {
                    if !want.curve_basis.is_empty() && want.curve_basis != fact.curve_basis {
                        continue;
                    }
                    if want.curve_basis.is_empty()
                        && (want.value.min.is_some() || want.value.max.is_some())
                        && matches!(
                            fact.curve_basis.as_str(),
                            "typical_curve" | "bench_measurement" | "simulation"
                        )
                    {
                        continue;
                    }
                    match accepts(&want.value, &fact.value) {
                        Some(true) => {
                            passed = true;
                            break;
                        }
                        Some(false) => mismatch = Some(fact),
                        None => {}
                    }
                }
                if passed {
                    continue;
                }
                let field = want.describe();
                if let Some(got) = mismatch {
                    out.push(Issue {
                        kind: RULE,
                        unknown: false,
                        objects: vec![obj],
                        message: format!(
                            "Datasheet cannot meet the specified {}. Required {}. Published {}.",
                            want.value.kind,
                            want.describe(),
                            got.describe()
                        ),
                    });
                } else {
                    missing(RULE, &obj, &field);
                }
            }
        }
    }
}
