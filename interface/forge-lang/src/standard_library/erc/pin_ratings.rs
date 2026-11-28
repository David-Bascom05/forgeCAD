//! Only unconditional guaranteed bounds on known net voltages are actionable here.
use super::*;
use crate::electronics_library::evidence::{missing, require};

fn voltage_rule(s: &characteristics::Specification) -> Option<&'static str> {
    let kind = s.value.kind.as_str();
    if !matches!(
        kind,
        "pin_voltage"
            | "supply_voltage"
            | "input_voltage"
            | "forward_voltage"
            | "working_voltage"
            | "collector_emitter_voltage"
            | "bulk_drain_voltage"
            | "bulk_source_voltage"
            | "gate_kelvin_source_voltage"
            | "gate_drain_voltage"
            | "gate_source_voltage"
            | "drain_source_voltage"
            | "differential_voltage"
    ) {
        return None;
    }
    match s.class.as_str() {
        "operating" => Some("PinOperatingVoltage"),
        "absolute_max" => Some("PinAbsoluteVoltage"),
        _ => None,
    }
}

pub(super) fn check(doc: &Document, c: &mut Circuit) {
    let mut rails: Map<&str, VoltageRange> = Map::new();
    for (id, p) in &c.terminals {
        if !p.populated {
            continue;
        }
        for role in &p.gives {
            let role = &doc.roles[role];
            let range = role
                .provided_voltage()
                .or_else(|| match (role.kind, &role.rating) {
                    (RoleKind::Reference, Some(Rating::VoltageMv(mv))) => Some(VoltageRange {
                        min: mv / 1000.,
                        max: mv / 1000.,
                    }),
                    _ => None,
                });
            if let Some(r) = range {
                rails
                    .entry(&c.net_of[id])
                    .and_modify(|s| {
                        s.min = s.min.min(r.min);
                        s.max = s.max.max(r.max);
                    })
                    .or_insert(r);
            }
        }
    }
    for (id, p) in &c.terminals {
        if !p.populated {
            continue;
        }
        let sym = &doc.symbols[&p.symbol];
        let pin = &sym.pins[&p.pin];
        let mode = p.function.as_ref().and_then(|f| pin.functions.get(f));
        let owner = id.rsplit_once('.').unwrap().0;
        if sym
            .pins
            .first_key_value()
            .is_some_and(|(first, _)| first == &p.pin)
        {
            for spec in sym.specs.iter().filter(|s| s.value.pins.is_empty()) {
                if let Some(rule) = voltage_rule(spec) {
                    missing(rule, owner, &format!("{} measurement terminals", spec.value.kind));
                }
            }
        }
        let device = sym.specs.iter().filter(|s| s.value.pins.first() == Some(&p.pin));
        let mut declared = [false; 2];
        for s in pin
            .specs
            .iter()
            .chain(mode.into_iter().flat_map(|m| &m.specs))
            .chain(device)
        {
            let v = &s.value;
            let Some(rule) = voltage_rule(s) else {
                continue;
            };
            declared[usize::from(s.class == "absolute_max")] = true;
            if !s.conditions.is_empty() {
                for cond in &s.conditions {
                    missing(
                        rule,
                        id,
                        &format!("operating condition: {} {}", cond.kind, cond.pins.join(" → ")),
                    );
                }
                continue;
            }
            if v.min.is_none() && v.max.is_none() {
                missing(rule, id, &format!("{}.min or max", v.kind));
                continue;
            }
            let (plus, minus) = match v.pins.as_slice() {
                [] => (id.clone(), None),
                [a] => (format!("{owner}.{a}"), None),
                [a, b] => (format!("{owner}.{a}"), Some(format!("{owner}.{b}"))),
                _ => continue,
            };
            let Some(pos) = c.net_of.get(&plus) else { continue };
            let neg = minus.as_ref().and_then(|pin| c.net_of.get(pin));
            if minus.is_some() && neg.is_none() {
                continue;
            }
            let zero = VoltageRange { min: 0., max: 0. };
            let observed = if neg == Some(pos) {
                zero
            } else {
                let Some(a) = require(rule, &plus, "net.provided_voltage", rails.get(pos.as_str())) else {
                    continue;
                };
                let b = match neg {
                    Some(net) => {
                        let Some(b) = require(
                            rule,
                            minus.as_deref().unwrap(),
                            "net.provided_voltage",
                            rails.get(net.as_str()),
                        ) else {
                            continue;
                        };
                        b
                    }
                    None => &zero,
                };
                VoltageRange {
                    min: a.min - b.max,
                    max: a.max - b.min,
                }
            };
            // Working-voltage limits for two-terminal passives specify the voltage magnitude.
            let observed = if v.kind == "working_voltage"
                && minus.is_some()
                && matches!(sym.part_kind.as_deref(), Some("resistor" | "capacitor"))
            {
                VoltageRange {
                    min: if observed.min <= 0. && observed.max >= 0. {
                        0.
                    } else {
                        observed.min.abs().min(observed.max.abs())
                    },
                    max: observed.min.abs().max(observed.max.abs()),
                }
            } else {
                observed
            };
            let scale = v.unit_scale();
            if v.min.is_some_and(|n| observed.min < n * scale)
                || v.max.is_some_and(|n| observed.max > n * scale)
            {
                let limit = match (v.min, v.max) {
                    (Some(a), Some(b)) => format!("{a}–{b}"),
                    (Some(a), None) => format!("≥ {a}"),
                    (None, Some(b)) => format!("≤ {b}"),
                    _ => continue,
                };
                let mut objects = vec![id.clone(), plus];
                objects.extend(minus);
                objects.sort();
                objects.dedup();
                let source = if s.source.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", s.source)
                };
                c.issues.push(Issue {
                    kind: rule,
                    unknown: false,
                    objects,
                    message: format!(
                        "{id} sees {observed}. The {} {} limit is {limit} {}{source}.",
                        s.class, v.kind, v.unit
                    ),
                });
            }
        }
        if p.power_net.is_none()
            && matches!(
                p.electrical,
                Electrical::Input | Electrical::Supply | Electrical::Bidirectional
            )
        {
            for (i, rule) in ["PinOperatingVoltage", "PinAbsoluteVoltage"]
                .into_iter()
                .enumerate()
            {
                if !declared[i] {
                    missing(
                        rule,
                        id,
                        if i == 0 {
                            "operating voltage min or max"
                        } else {
                            "absolute voltage min or max"
                        },
                    );
                }
            }
        }
    }
}
