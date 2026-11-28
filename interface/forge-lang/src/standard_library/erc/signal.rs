//! Direct, nominal signal contracts between the active functions of fitted pins.
use super::graph::{Issue, Terminal};
use super::*;
use forge_agent::json::Json;

#[derive(Default, Debug)]
pub(super) struct Signals {
    inputs: Map<String, Set<RoleId>>,
    outputs: Map<String, Set<RoleId>>,
    incoming: Map<String, Vec<PathCheck>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Compatibility {
    Compatible,
    Mismatch,
    Unknown,
}

impl Compatibility {
    fn word(self) -> &'static str {
        match self {
            Self::Compatible => "compatible",
            Self::Mismatch => "mismatch",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug)]
struct PathCheck {
    source: String,
    status: Compatibility,
    alternatives: Vec<String>,
}

impl Signals {
    pub(super) fn covers(&self, pin: &str, role: &RoleId) -> bool {
        self.incoming.contains_key(pin) && self.inputs[pin].contains(role)
    }

    pub(super) fn peers(&self, pin: &str, role: &RoleId, need: bool, other: &str) -> Option<Evidence> {
        let (source, receiver) = if need { (other, pin) } else { (pin, other) };
        let roles = if need {
            self.inputs.get(pin)
        } else {
            self.outputs.get(pin)
        }?;
        if !roles.contains(role) {
            return None;
        }
        let path = self.incoming.get(receiver)?.iter().find(|p| p.source == source)?;
        Some(match path.status {
            Compatibility::Unknown => Evidence::Candidate,
            _ if self.inputs[receiver].contains(role) && self.outputs[source].contains(role) => {
                Evidence::Verified
            }
            _ if need => Evidence::Invalid,
            _ => return None, // A receiver need not consume every capability a driver offers.
        })
    }
}

fn input(kind: Electrical) -> bool {
    matches!(kind, Electrical::Input | Electrical::Bidirectional)
}

fn signal_role(doc: &Document, id: &RoleId) -> bool {
    let role = &doc.roles[id];
    role.connection.is_none()
        && matches!(
            role.kind,
            RoleKind::Signal | RoleKind::Clock | RoleKind::Interface
        )
}

fn roles(doc: &Document, declared: &Set<RoleId>) -> Set<RoleId> {
    declared
        .iter()
        .filter(|id| signal_role(doc, id))
        .cloned()
        .collect()
}

fn alternatives(doc: &Document, receiver: &Terminal, offered: &Set<RoleId>) -> Vec<String> {
    let pin = &doc.symbols[&receiver.symbol].pins[&receiver.pin];
    let mut result = Vec::new();
    let mut consider = |name: &str, kind, wants: &Set<RoleId>| {
        if input(kind) && receiver.function.as_deref() != Some(name) {
            let mut required = roles(doc, wants);
            required.extend(
                receiver
                    .instance_wants
                    .iter()
                    .filter(|id| signal_role(doc, id))
                    .cloned(),
            );
            if !required.is_empty() && required.is_subset(offered) {
                result.push(name.to_string());
            }
        }
    };
    for (name, mode) in &pin.functions {
        consider(name, mode.electrical, &mode.wants);
    }
    result
}

impl Circuit {
    pub(super) fn check_signals(&mut self, doc: &Document) {
        for members in self.nets.values() {
            let mut typed_sources = Vec::new();
            let mut untyped_sources = Vec::new();
            let mut receivers = Vec::new();
            for id in members {
                let Some(pin) = self.terminals.get(id).filter(|t| t.populated) else {
                    continue;
                };
                if !matches!(
                    pin.electrical,
                    Electrical::Input
                        | Electrical::Output
                        | Electrical::Bidirectional
                        | Electrical::OpenDrain
                ) {
                    continue;
                }
                let wants = roles(doc, &pin.wants);
                let gives = roles(doc, &pin.gives);
                let duplex = pin.electrical == Electrical::Bidirectional;
                if input(pin.electrical) && !(duplex && wants.is_empty() && !gives.is_empty()) {
                    receivers.push((id, wants.clone()));
                }
                if matches!(
                    pin.electrical,
                    Electrical::Output | Electrical::Bidirectional | Electrical::OpenDrain
                ) && !(duplex && gives.is_empty() && !wants.is_empty())
                {
                    if gives.is_empty() {
                        untyped_sources.push((id, gives));
                    } else {
                        typed_sources.push((id, gives));
                    }
                }
            }
            // Untyped nets contribute no signal evidence. On mixed nets only enumerate pairs
            // with at least one declared contract.
            for (receiver, wants) in receivers {
                let untyped = if wants.is_empty() {
                    &[][..]
                } else {
                    untyped_sources.as_slice()
                };
                for &(source, ref gives) in typed_sources.iter().chain(untyped) {
                    if receiver == source {
                        continue;
                    }
                    let status = if wants.is_empty() || gives.is_empty() {
                        Compatibility::Unknown
                    } else if wants.is_subset(gives) {
                        Compatibility::Compatible
                    } else {
                        Compatibility::Mismatch
                    };
                    let sink = &self.terminals[receiver];
                    let mut suggestions = Vec::new();
                    if status != Compatibility::Compatible {
                        suggestions = alternatives(doc, sink, gives);
                        let modes = |pin: &Terminal| pin.function.as_deref().unwrap_or("base").to_owned();
                        let names =
                            |rs: &Set<RoleId>| rs.iter().map(|r| r.0.as_str()).collect::<Vec<_>>().join(", ");
                        let mut message = format!(
                            "{source} ({}) offers [{}]; {receiver} ({}) requires [{}].",
                            modes(&self.terminals[source]),
                            names(gives),
                            modes(sink),
                            names(&wants)
                        );
                        if status == Compatibility::Unknown {
                            if gives.is_empty() {
                                crate::electronics_library::evidence::missing(
                                    "PinFunctionMismatch",
                                    source,
                                    "active pin provides signal contract",
                                );
                            }
                            if wants.is_empty() {
                                crate::electronics_library::evidence::missing(
                                    "PinFunctionMismatch",
                                    receiver,
                                    "active pin wants signal contract",
                                );
                            }
                            message.push_str(" One endpoint has no active signal contract.");
                        }
                        if !suggestions.is_empty() {
                            message.push_str(&format!(
                                " Compatible receiver functions: {}.",
                                suggestions.join(", ")
                            ));
                        }
                        self.issues.push(Issue {
                            kind: if status == Compatibility::Unknown {
                                "UnknownPinFunctionCompatibility"
                            } else {
                                "PinFunctionMismatch"
                            },
                            unknown: status == Compatibility::Unknown,
                            objects: vec![source.clone(), receiver.clone()],
                            message,
                        });
                    }
                    self.signals
                        .outputs
                        .entry(source.clone())
                        .or_insert_with(|| gives.clone());
                    self.signals
                        .inputs
                        .entry(receiver.clone())
                        .or_insert_with(|| wants.clone());
                    self.signals
                        .incoming
                        .entry(receiver.clone())
                        .or_default()
                        .push(PathCheck {
                            source: source.clone(),
                            status,
                            alternatives: suggestions,
                        });
                }
            }
        }
    }

    /// Active, fitted pin pairs as resolved by schematic ERC.
    pub(crate) fn placement_signals(&self) -> impl Iterator<Item = (&str, &str, &str, &Set<RoleId>)> {
        self.signals.incoming.iter().flat_map(move |(receiver, checks)| {
            checks.iter().map(move |path| {
                (
                    path.source.as_str(),
                    receiver.as_str(),
                    path.status.word(),
                    &self.signals.inputs[receiver],
                )
            })
        })
    }

    pub(super) fn signal_paths_json(&self, doc: &Document) -> Json {
        let endpoint = |id: &str, key, roles: &Set<RoleId>| {
            let pin = &self.terminals[id];
            Json::obj(vec![
                ("terminal", id.to_owned().into()),
                (
                    "active_function",
                    pin.function.clone().map_or(Json::Null, Into::into),
                ),
                ("electrical", pin.electrical.word().into()),
                (key, super::report::roles(roles, doc)),
            ])
        };
        let mut paths = Vec::new();
        for (receiver, checks) in &self.signals.incoming {
            for path in checks {
                paths.push(Json::obj(vec![
                    ("net", self.net_of[receiver].clone().into()),
                    ("status", path.status.word().into()),
                    (
                        "source",
                        endpoint(&path.source, "gives", &self.signals.outputs[&path.source]),
                    ),
                    (
                        "receiver",
                        endpoint(receiver, "wants", &self.signals.inputs[receiver]),
                    ),
                    (
                        "compatible_receiver_functions",
                        Json::Arr(path.alternatives.iter().cloned().map(Into::into).collect()),
                    ),
                ]));
            }
        }
        Json::Arr(paths)
    }
}
