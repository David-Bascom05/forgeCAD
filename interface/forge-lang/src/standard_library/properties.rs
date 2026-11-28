//! Rust-authored standard property types and the fitted maximum-supply check.
use super::*;
use sch::{AttributeType, AttributeUnit, ConversionGraph, ConversionTable, Item, Radix, StrapKind};
fn unit(name: &str, factor: f64, min: Option<f64>) -> AttributeUnit {
    AttributeUnit {
        name: name.into(),
        factor,
        min,
        max: None,
        radix: None,
    }
}
fn attribute(
    label: &str,
    kind: StrapKind,
    units: Vec<AttributeUnit>,
    modes: &[&str],
    edges: Option<Vec<ConversionTable>>,
) -> AttributeType {
    let links = edges.unwrap_or_else(|| ConversionGraph::defaults(&units));
    AttributeType {
        origin: None,
        label: label.into(),
        kind,
        conversions: ConversionGraph::build(kind, &units, links).expect("standard conversions"),
        units,
        modes: modes.iter().map(|s| s.to_string()).collect(),
        range: None,
    }
}
pub(crate) fn types() -> BTreeMap<String, AttributeType> {
    let single = || vec![unit("Unitless", 1., None)];
    let address = [
        ("Hex", 16, "0x", 2),
        ("Decimal", 10, "", 0),
        ("Binary", 2, "0b", 7),
    ]
    .map(|(n, b, p, d)| AttributeUnit {
        name: n.into(),
        factor: 1.,
        min: Some(0.),
        max: Some(127.),
        radix: Some(Radix {
            base: b,
            prefix: p.into(),
            digits: d,
        }),
    })
    .to_vec();
    let edges = [
        ("Hex to decimal", "Hex", "Decimal"),
        ("Decimal to binary", "Decimal", "Binary"),
    ]
    .map(|(name, a, b)| ConversionTable {
        name: name.into(),
        from: a.into(),
        to: b.into(),
        rows: vec![],
    })
    .to_vec();
    BTreeMap::from([
        (
            "address".into(),
            attribute("I²C address", StrapKind::Address7, address, &[], Some(edges)),
        ),
        (
            "current_limit".into(),
            attribute(
                "Current limit",
                StrapKind::Current,
                vec![
                    unit("A", 1., Some(0.)),
                    unit("mA", 0.001, Some(0.)),
                    unit("µA", 0.000001, Some(0.)),
                ],
                &[],
                None,
            ),
        ),
        (
            "text".into(),
            attribute("Text", StrapKind::Text, single(), &[], None),
        ),
        (
            "choice".into(),
            attribute("Choice", StrapKind::Choice, single(), &[], None),
        ),
        (
            "boot_mode".into(),
            attribute(
                "Boot mode",
                StrapKind::BootMode,
                single(),
                &["Flash", "ROM bootloader", "SRAM"],
                None,
            ),
        ),
    ])
}
const MAX_VOLTAGE: &str = "standard.properties.ratings.maximum_supply_voltage";
pub(super) fn ratings() -> BTreeMap<String, AttributeType> {
    let mut t = attribute(
        "Maximum supply voltage",
        StrapKind::Quantity,
        vec![unit("V", 1., Some(0.)), unit("mV", 0.001, Some(0.))],
        &[],
        None,
    );
    t.origin = Some(MAX_VOLTAGE);
    BTreeMap::from([("maximum_supply_voltage".into(), t)])
}
pub(super) fn supply(doc: &Document, c: &Circuit, out: &mut Vec<Issue>) {
    if !doc
        .attribute_types
        .values()
        .any(|t| t.origin == Some(MAX_VOLTAGE))
    {
        return;
    }
    for (scope, sheet) in &c.occurrences {
        for row in doc.rows.iter().filter(|r| &r.sheet == sheet) {
            let Item::Place {
                symbol,
                fields,
                populated,
                ..
            } = &row.item
            else {
                continue;
            };
            let owner = format!("{scope}/{}", row.id);
            if !c.is_fitted(&owner, *populated) {
                continue;
            }
            let sym = &doc.symbols[symbol];
            if !sym
                .field_types
                .values()
                .any(|ty| doc.attribute_types[ty].origin == Some(MAX_VOLTAGE))
            {
                for pin in sym.pins.keys() {
                    let target = format!("{owner}.{pin}");
                    if c.terminals
                        .get(&target)
                        .is_some_and(|p| p.electrical == sch::Electrical::Supply)
                    {
                        evidence::missing("SupplyAboveRating", &target, "maximum_supply_voltage");
                    }
                }
            }
            for (key, ty) in &sym.field_types {
                let t = &doc.attribute_types[ty];
                if t.origin != Some(MAX_VOLTAGE) {
                    continue;
                }
                let Some(max) = fields
                    .get(key)
                    .or_else(|| sym.fields.get(key))
                    .and_then(|v| t.value(v))
                    .and_then(|j| j.get("value")?.num())
                else {
                    evidence::missing("SupplyAboveRating", &owner, key);
                    continue;
                };
                for pin in sym.pins.keys() {
                    let target = format!("{owner}.{pin}");
                    let Some(term) = c.terminals.get(&target) else {
                        continue;
                    };
                    if term.electrical != sch::Electrical::Supply {
                        continue;
                    }
                    let mut supplied = false;
                    for other in &c.nets[&c.net_of[&target]] {
                        let Some(src) = c.terminals.get(other).filter(|s| s.populated) else {
                            continue;
                        };
                        for role in &src.gives {
                            supplied |= doc.roles[role].provided_voltage().is_some();
                            if let Some(range) = doc.roles[role].provided_voltage()
                                && range.max > max
                            {
                                out.push(Issue {
                                    kind: "SupplyAboveRating",
                                    unknown: false,
                                    objects: vec![
                                        target.clone(),
                                        other.clone(),
                                        format!("{owner}:custom.{key}"),
                                    ],
                                    message: format!(
                                        "{other} supplies up to {} V; {target} is rated for {max} V",
                                        range.max
                                    ),
                                });
                            }
                        }
                    }
                    if !supplied {
                        evidence::missing("SupplyAboveRating", &target, "net.provided_voltage");
                    }
                }
            }
        }
    }
}
