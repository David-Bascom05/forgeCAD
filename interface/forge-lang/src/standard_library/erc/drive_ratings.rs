//! DC comparisons on directed nets. Bidirectional or shared drive needs a selected drive state.
use super::*;
use crate::electronics_library::evidence::{missing, require};
use crate::schematic::characteristics::Specification;

fn records<'a>(doc: &'a Document, p: &Terminal) -> impl Iterator<Item = &'a Specification> {
    let sym = &doc.symbols[&p.symbol];
    let pin = &sym.pins[&p.pin];
    let mode = p.function.as_ref().and_then(|f| pin.functions.get(f));
    pin.specs
        .iter()
        .chain(mode.into_iter().flat_map(|f| &f.specs))
        .chain(
            sym.specs
                .iter()
                .filter(|s| s.value.pins.len() == 1 && s.value.pins[0] == p.pin),
        )
}
fn scalar(doc: &Document, p: &Terminal, k: &str, minimum: bool) -> Option<f64> {
    records(doc, p)
        .filter(|s| {
            s.value.kind == k
                && (!matches!(k, "source_current" | "sink_current") || s.class == "operating")
                && s.conditions.is_empty()
                && matches!(s.class.as_str(), "operating" | "characteristic")
                && (s.value.pins.is_empty() || s.value.pins.as_slice() == [p.pin.as_str()])
        })
        .filter_map(|s| {
            let v = &s.value;
            let scale = v.unit_scale();
            (if minimum { v.min } else { v.max }).map(|n| n * scale)
        })
        .reduce(|a, b| if minimum { a.max(b) } else { a.min(b) })
}
fn logic(doc: &Document, p: &Terminal) -> Option<bool> {
    let mut digital = false;
    for s in records(doc, p).filter(|s| s.conditions.is_empty()) {
        if s.value.kind == "signal_domain" {
            return Some(s.value.state == "digital");
        }
        digital |= matches!(
            s.value.kind.as_str(),
            "output_high_voltage" | "output_low_voltage" | "logic_high_threshold" | "logic_low_threshold"
        );
    }
    digital.then_some(true)
}
pub(super) fn check(doc: &Document, c: &mut Circuit) {
    for members in c.nets.values() {
        let pins: Vec<_> = members
            .iter()
            .filter_map(|id| {
                c.terminals
                    .get(id)
                    .filter(|p| p.populated && p.power_net.is_none() && p.electrical != Electrical::Unused)
                    .map(|p| (id, p))
            })
            .collect();
        let mut sources = pins.iter().copied().filter(|(_, p)| {
            matches!(
                p.electrical,
                Electrical::Output | Electrical::Source | Electrical::OpenDrain
            )
        });
        let Some((source, driver)) = sources.next() else {
            continue;
        };
        let sinks: Vec<_> = pins.iter().copied().filter(|(id, _)| *id != source).collect();
        if sinks.is_empty() {
            continue;
        }
        // Exclusive drive cannot be inferred across another active or bidirectional driver.
        if sources.next().is_some()
            || sinks
                .iter()
                .any(|(_, p)| p.electrical == Electrical::Bidirectional)
        {
            for (id, p) in &pins {
                if !matches!(
                    p.electrical,
                    Electrical::Output
                        | Electrical::Source
                        | Electrical::OpenDrain
                        | Electrical::Bidirectional
                ) {
                    continue;
                }
                for rule in [
                    "PinSourceCurrent",
                    "PinSinkCurrent",
                    "LogicHighLevel",
                    "LogicLowLevel",
                ] {
                    missing(rule, id, "active driver and load-sharing state");
                }
            }
            continue;
        }
        if driver.electrical != Electrical::Source {
            for (sink, receiver) in sinks
                .iter()
                .copied()
                .filter(|(_, p)| p.electrical == Electrical::Input)
            {
                let domains = (logic(doc, driver), logic(doc, receiver));
                if matches!(domains, (Some(false), _) | (_, Some(false))) {
                    continue;
                }
                for (high, rule, out, input) in [
                    (
                        true,
                        "LogicHighLevel",
                        "output_high_voltage",
                        "logic_high_threshold",
                    ),
                    (
                        false,
                        "LogicLowLevel",
                        "output_low_voltage",
                        "logic_low_threshold",
                    ),
                ] {
                    if high && driver.electrical == Electrical::OpenDrain {
                        continue;
                    }
                    if domains == (None, None) {
                        missing(rule, source, "signal_domain or guaranteed logic levels");
                        missing(rule, sink, "signal_domain or logic thresholds");
                        continue;
                    }
                    let a = require(
                        rule,
                        source,
                        if high {
                            "output_high_voltage.min"
                        } else {
                            "output_low_voltage.max"
                        },
                        scalar(doc, driver, out, high),
                    );
                    let b = require(
                        rule,
                        sink,
                        if high {
                            "logic_high_threshold.min"
                        } else {
                            "logic_low_threshold.max"
                        },
                        scalar(doc, receiver, input, high),
                    );
                    if let (Some(a), Some(b)) = (a, b)
                        && (if high { a < b } else { a > b })
                    {
                        c.issues.push(Issue {
                            kind: rule,
                            unknown: false,
                            objects: vec![source.clone(), sink.clone()],
                            message: format!(
                                "{source} guarantees {a} V. {sink} requires {} {b} V.",
                                if high { "at least" } else { "at most" }
                            ),
                        });
                    }
                }
            }
        }
        for (pull, limit) in [
            ("load_current", "source_current"),
            ("pullup_load_current", "sink_current"),
        ] {
            if driver.electrical == Electrical::OpenDrain && limit == "source_current"
                || driver.electrical == Electrical::Source && limit == "sink_current"
            {
                continue;
            }
            let rule = if limit == "source_current" {
                "PinSourceCurrent"
            } else {
                "PinSinkCurrent"
            };
            let capacity = require(
                rule,
                source,
                &format!("{limit}.max (continuous operating magnitude)"),
                scalar(doc, driver, limit, false).filter(|n| *n >= 0.),
            );
            let mut complete = true;
            let mut total = 0.;
            for (id, p) in &sinks {
                if let Some(value) = require(rule, id, &format!("{pull}.max"), scalar(doc, p, pull, false)) {
                    if value < 0. {
                        missing(rule, id, "nonnegative load-current magnitude");
                        complete = false;
                    } else {
                        total += value;
                    }
                } else {
                    complete = false;
                }
            }
            if complete
                && let Some(max) = capacity
                && total > max
            {
                let mut objects = vec![source.clone()];
                objects.extend(sinks.iter().map(|(id, _)| (*id).clone()));
                c.issues.push(Issue{kind:rule,unknown:false,objects,message:format!("{source} has a {limit} limit of {max} A. The declared worst-case loads total {total} A.")});
            }
        }
    }
}
