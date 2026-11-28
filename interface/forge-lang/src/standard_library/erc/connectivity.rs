//! Standard electrical checks over resolved schematic facts.
use super::*;
#[path = "drive_ratings.rs"]
mod drive_ratings;
#[path = "pin_ratings.rs"]
mod pin_ratings;
use crate::electronics_library::Sheets;
pub(crate) fn buses(_: &Document, c: &mut Circuit, _: &Sheets, _: &Map<String, bool>) {
    let mut assigned = Map::new();
    for (id, [p, n]) in &c.diff_pairs {
        let (a, b) = (&c.net_of[p], &c.net_of[n]);
        if a == b {
            c.issues.push(Issue {
                kind: "DiffPairShort",
                unknown: false,
                objects: vec![id.clone(), p.clone(), n.clone()],
                message: "Both differential pair legs resolve to the same net".into(),
            });
            continue;
        }
        for (net, mate, positive) in [(a, b, true), (b, a, false)] {
            if let Some((other, polarity, first)) = assigned.get(net) {
                if *other != mate || *polarity != positive {
                    c.issues.push(Issue {
                        kind: "DiffPairConflict",
                        unknown: false,
                        objects: vec![String::clone(first), id.clone(), net.clone()],
                        message: "Net has conflicting differential mates or polarity".into(),
                    });
                    break;
                }
            } else {
                assigned.insert(net, (mate, positive, id.clone()));
            }
        }
    }
    for (id, members) in &c.buses {
        let mut seen = Map::new();
        for (lane, pin) in members {
            if let Some(other) = seen.insert(&c.net_of[pin], lane) {
                c.issues.push(Issue {
                    unknown: false,
                    kind: "BusMemberShort",
                    objects: vec![id.clone(), members[other].clone(), pin.clone()],
                    message: format!("{other} and {lane} share a net"),
                });
            }
        }
    }
}
pub(crate) fn electrical(doc: &Document, c: &mut Circuit, _: &Sheets, _: &Map<String, bool>) {
    c.check_voltages(doc);
    pin_ratings::check(doc, c);
    drive_ratings::check(doc, c);
}
pub(crate) fn relations(doc: &Document, c: &mut Circuit, _: &Sheets, _: &Map<String, bool>) {
    c.check();
    c.check_sources_and_labels(doc);
}
pub(crate) fn placement(doc: &Document, c: &Circuit, findings: &mut Vec<Issue>) {
    for sheet in &c.occurrences {
        let (scope, id) = sheet;
        for row in doc.rows.iter().filter(|r| &r.sheet == id) {
            if let Item::Place { symbol, pins, .. } = &row.item {
                let functions = &pins.functions;
                for (group, signals) in &doc.symbols[symbol].peripherals {
                    for (signal, pins) in signals {
                        let active: Vec<_> = pins
                            .iter()
                            .filter(|p| functions.get(*p) == Some(signal))
                            .map(|p| format!("{scope}/{}.{p}", row.id))
                            .collect();
                        if active.len() > 1 {
                            findings.push(Issue {
                                unknown: false,
                                kind: "PeripheralConflict",
                                objects: active,
                                message: format!("More than one pin selects {group}.{signal}"),
                            });
                        }
                    }
                }
            }
        }
    }
}
pub(super) fn missing_units(c: &mut Circuit, scope: &str, owner: &str, missing: Set<&String>) {
    for unit in missing {
        c.issues.push(Issue {
            unknown: false,
            kind: "UnplacedGate",
            objects: vec![format!("{scope}/{owner}"), unit.clone()],
            message: "sub part has no schematic placement".into(),
        });
    }
}
impl Circuit {
    fn check_sources_and_labels(&mut self, doc: &Document) {
        let mut spellings = Map::new();
        for (key, object) in &self.authored_labels {
            let name = self.labels[key].as_str();
            let scope = &key[..key.len() - name.len()];
            let group = (scope, name.to_lowercase());
            if let Some((previous, other)) = spellings.get(&group) {
                if previous != &name {
                    self.issues.push(Issue {
                        unknown: false,
                        kind: "SimilarLabelNames",
                        objects: vec![String::clone(other), object.clone()],
                        message: format!(
                            "Labels {previous} and {name} differ only in case in the same scope"
                        ),
                    });
                }
            } else {
                spellings.insert(group, (name, object.clone()));
            }
        }
        for members in self.nets.values() {
            let pins: Vec<_> = members
                .iter()
                .filter_map(|id| self.terminals.get(id).filter(|p| p.populated).map(|p| (id, p)))
                .collect();
            if pins.len() < 2
                && let Some(label) = members.iter().find_map(|id| self.authored_labels.get(id))
            {
                self.issues.push(Issue {
                    unknown: false,
                    kind: "DanglingLabel",
                    objects: vec![label.clone()],
                    message: "This label reaches fewer than two fitted pins".into(),
                });
            }
            let power = pins.iter().any(|(_, p)| {
                p.electrical == Electrical::Source
                    || p.gives
                        .iter()
                        .any(|r| matches!(doc.roles[r].kind, RoleKind::Power | RoleKind::Reference))
            });
            let driver = power
                || pins.iter().any(|(_, p)| {
                    p.power_net.is_none()
                        && matches!(
                            p.electrical,
                            Electrical::Output
                                | Electrical::Bidirectional
                                | Electrical::OpenDrain
                                | Electrical::Passive
                        )
                });
            for (kind, electrical, driven, msg) in [
                (
                    "UndrivenPowerInput",
                    Electrical::Supply,
                    power,
                    "Power inputs have no declared power source on this net",
                ),
                (
                    "UndrivenInput",
                    Electrical::Input,
                    driver,
                    "Signal inputs have no driver on this net",
                ),
            ] {
                if driven {
                    continue;
                }
                let objects: Vec<_> = pins
                    .iter()
                    .filter(|(id, p)| {
                        p.electrical == electrical && self.attached.contains(*id) && !self.nc.contains(*id)
                    })
                    .map(|(id, _)| (*id).clone())
                    .collect();
                if !objects.is_empty() {
                    self.issues.push(Issue {
                        kind,
                        unknown: false,
                        objects,
                        message: msg.into(),
                    });
                }
            }
        }
    }
    fn check_voltages(&mut self, doc: &Document) {
        for members in self.nets.values() {
            let mut sources = Vec::new();
            let mut wants = Vec::new();
            for id in members {
                let Some(pin) = self.terminals.get(id).filter(|p| p.populated) else {
                    continue;
                };
                for role in &pin.gives {
                    let Some(voltage) = doc.roles[role].provided_voltage() else {
                        continue;
                    };
                    sources.push((id, role, voltage));
                }
                for role in &pin.wants {
                    if doc.roles[role].kind == RoleKind::Power
                        && let Some(Rating::VoltageRange(required)) = doc.roles[role].rating
                    {
                        wants.push((id, role, required));
                    }
                }
            }
            for (pin, role, required) in wants {
                let mut check = VoltageCheck {
                    required,
                    providers: Vec::new(),
                };
                for &(peer, give, offered) in &sources {
                    if peer == pin {
                        continue;
                    }
                    check.providers.push(VoltageProvider {
                        pin: peer.clone(),
                        role: give.clone(),
                        range: offered,
                    });
                    if !required.accepts(offered) {
                        self.issues.push(Issue {
                            unknown: false,
                            kind: "VoltageOutOfRange",
                            objects: vec![pin.clone(), peer.clone()],
                            message: format!("{pin} wants {required}. {peer} provides {offered}."),
                        });
                    }
                }
                if check.providers.is_empty() {
                    crate::electronics_library::evidence::missing(
                        "VoltageOutOfRange",
                        pin,
                        "net.provided_voltage",
                    );
                    self.issues.push(Issue {
                        unknown: true,
                        kind: "UnknownSupplyVoltage",
                        objects: vec![pin.clone()],
                        message: format!(
                            "{pin} wants {required}. No connected pin declares a supply voltage."
                        ),
                    });
                }
                self.voltages
                    .entry(pin.clone())
                    .or_default()
                    .insert(role.clone(), check);
            }
        }
    }
    fn check(&mut self) {
        let issue = |kind, objects, message: &str| Issue {
            unknown: kind == "UnfulfilledWant",
            kind,
            objects,
            message: message.into(),
        };
        for (id, pin) in &self.terminals {
            if !pin.populated {
                continue;
            }
            let connected = self.attached.contains(id);
            if connected && (self.nc.contains(id) || pin.electrical == Electrical::Unused) {
                self.issues.push(issue(
                    "ConnectedNoConnect",
                    vec![id.clone()],
                    "a no-connect pin is attached",
                ));
            } else if !connected && !self.nc.contains(id) && pin.electrical != Electrical::Unused {
                self.issues
                    .push(issue("UnconnectedPin", vec![id.clone()], "pin has no attachment"));
            }
        }
        for members in self.nets.values() {
            let outputs: Vec<_> = members
                .iter()
                .filter(|id| {
                    self.terminals.get(*id).is_some_and(|p| {
                        p.populated && matches!(p.electrical, Electrical::Output | Electrical::Source)
                    })
                })
                .cloned()
                .collect();
            if outputs.len() > 1 {
                self.issues.push(issue(
                    "DriverConflict",
                    outputs,
                    "multiple push-pull or power outputs drive one net",
                ));
            }
            let names: Set<_> = members.iter().filter_map(|id| self.labels.get(id)).collect();
            if names.len() > 1 {
                self.issues.push(issue(
                    "NetLabelConflict",
                    members
                        .iter()
                        .filter(|k| self.labels.contains_key(*k))
                        .cloned()
                        .collect(),
                    "different net names are electrically joined",
                ));
            }
        }
        let mut satisfied = Set::new();
        for (i, link) in self.links.iter().enumerate() {
            for pin in [&link.want, &link.give] {
                self.relations.entry(pin.clone()).or_default().push(i);
            }
            if !self.terminals[&link.want].populated {
                continue;
            }
            if !self.terminals[&link.give].populated || self.net_of[&link.want] != self.net_of[&link.give] {
                self.issues.push(issue(
                    "DisconnectedFulfillment",
                    vec![link.id.clone(), link.want.clone(), link.give.clone()],
                    "functional relationship spans disconnected supply or signal pins",
                ));
            } else if link.returns.as_ref().is_some_and(|(a, b)| {
                a == b || self.net_of[a] != self.net_of[b] || self.net_of[a] == self.net_of[&link.want]
            }) {
                self.issues.push(issue(
                    "ReturnMismatch",
                    vec![link.id.clone()],
                    "distinct return endpoints need a common net separate from the supply",
                ));
            } else {
                satisfied.insert((&link.want, &link.role));
            }
        }
        for (id, p) in &self.terminals {
            for r in &p.wants {
                if p.populated
                    && !self.voltages.get(id).is_some_and(|v| v.contains_key(r))
                    && !self.connections.get(id).is_some_and(|v| v.contains_key(r))
                    && !self.signals.covers(id, r)
                    && !satisfied.contains(&(id, r))
                {
                    self.issues.push(issue(
                        "UnfulfilledWant",
                        vec![id.clone(), r.0.clone()],
                        "pin wants a role without a verified fulfillment",
                    ));
                }
            }
        }
    }
}
