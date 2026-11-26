//! Resolve a pin bias from active internal pulls and fitted resistor paths to rated rails.
use super::*;

impl Role {
    fn volts(&self) -> Option<VoltageRange> {
        match self.rating {
            Some(Rating::VoltageRange(range)) => Some(range),
            Some(Rating::VoltageMv(v)) => Some(VoltageRange {
                min: v / 1000.,
                max: v / 1000.,
            }),
            _ => None,
        }
    }
}

struct PullSource {
    target: String,
    bridge: Vec<String>,
    description: String,
    volts: Option<VoltageRange>,
    ohms: Option<[PartValue; 2]>,
    internal: Option<RoleId>,
    fitted: bool,
}

/// On a multi-branch signal net, a demonstrated rail conflict takes precedence over a
/// valid pull. Unrated branches may lead to unrelated signal pins.
#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum PullEvidence {
    Missing,
    Omitted,
    UnknownRail,
    UnknownValue,
    WrongValue,
    Verified,
    RailConflict,
}

impl PullSource {
    fn evaluate(&self, want: &Role, doc: &Document) -> (PullEvidence, String) {
        use PullEvidence::*;
        if !self.fitted {
            return (
                Omitted,
                format!("{} is omitted from this assembly", self.description),
            );
        }
        let Some(v) = self.volts else {
            return (
                UnknownRail,
                format!(
                    "{} has no declared rail voltage at {}",
                    self.description, self.target
                ),
            );
        };
        let path = format!("{} to {v} ({})", self.description, self.target);
        if self
            .internal
            .as_ref()
            .is_some_and(|id| doc.roles[id].kind != want.kind)
            || !want.volts().unwrap().accepts(v)
        {
            return (
                RailConflict,
                format!("{path} conflicts with the required {} rail", want.kind.word()),
            );
        }
        match (want.resistance, self.ohms) {
            (_, Some([lo, _])) if lo.magnitude == 0. => {
                (RailConflict, format!("{path}: zero-ohm connection to rail"))
            }
            (bounds, None) if bounds.is_some() || self.internal.is_none() => {
                (UnknownValue, format!("{path}: resistance unknown"))
            }
            (Some([lo, hi]), Some([a, b])) if !lo.at_most(a) || !b.at_most(hi) => (
                WrongValue,
                format!("{path}: resistance outside {}–{} Ω", lo.magnitude, hi.magnitude),
            ),
            _ => (Verified, path),
        }
    }
}

impl Circuit {
    pub(super) fn check_pulls(&mut self, doc: &Document, sheets: &Map<&str, Vec<&Row>>) {
        if !doc.roles.values().any(|r| r.kind.is_pull()) {
            return;
        }
        let mut rails = Map::<&str, Vec<(&str, VoltageRange)>>::new();
        let mut sources = Map::<&str, Vec<PullSource>>::new();
        for (id, term) in &self.terminals {
            if !term.populated {
                continue;
            }
            let net = self.net_of[id].as_str();
            for role in &term.gives {
                let spec = &doc.roles[role];
                if spec.kind.is_pull() {
                    sources.entry(net).or_default().push(PullSource {
                        target: id.clone(),
                        bridge: vec![],
                        volts: spec.volts(),
                        ohms: spec.resistance,
                        description: format!(
                            "Internal {} on {id}{}",
                            spec.kind.word().replace('_', "-"),
                            spec.resistance
                                .map(|[lo, hi]| if lo == hi {
                                    format!(" ({} Ω)", lo.magnitude)
                                } else {
                                    format!(" ({}–{} Ω)", lo.magnitude, hi.magnitude)
                                })
                                .unwrap_or_default()
                        ),
                        internal: Some(role.clone()),
                        fitted: true,
                    });
                } else if matches!(spec.kind, RoleKind::Power | RoleKind::Reference)
                    && let Some(v) = spec.volts()
                {
                    rails.entry(net).or_default().push((id, v));
                }
            }
        }
        for (scope, sheet) in &self.occurrences {
            for row in sheets.get(sheet.as_str()).into_iter().flatten() {
                let Item::Place {
                    symbol,
                    value,
                    reference,
                    ..
                } = &row.item
                else {
                    continue;
                };
                let def = &doc.symbols[symbol];
                if def.part_kind.as_deref() != Some("resistor") {
                    continue;
                }
                let owner = format!("{scope}/{}", row.id);
                let display = doc.reference(&owner, reference.as_ref().unwrap_or(&row.id));
                let mut paths: Vec<_> = def.paths.values().cloned().collect();
                if paths.is_empty() && def.pins.len() == 2 {
                    let mut pins = def.pins.keys();
                    paths.push([pins.next().unwrap().clone(), pins.next().unwrap().clone()]);
                }
                for [a, b] in paths {
                    let ends = [format!("{owner}.{a}"), format!("{owner}.{b}")];
                    for (near, far) in [(&ends[0], &ends[1]), (&ends[1], &ends[0])] {
                        let (Some(net), Some(other)) = (self.net_of.get(near), self.net_of.get(far)) else {
                            continue;
                        };
                        if net == other {
                            continue;
                        }
                        let supply = rails.get(other.as_str());
                        let mut add = |target: &str, volts| {
                            sources.entry(net).or_default().push(PullSource {
                                target: target.into(),
                                bridge: vec![near.clone(), far.clone()],
                                volts,
                                ohms: PartValue::parse(value).filter(|v| v.unit == "Ω").map(|v| [v; 2]),
                                description: format!("{display} ({value})"),
                                internal: None,
                                fitted: self.terminals[near].populated,
                            });
                        };
                        if let Some(supply) = supply {
                            for (id, v) in supply {
                                add(id, Some(*v));
                            }
                        } else {
                            add(far, None);
                        }
                    }
                }
            }
        }
        for (pin, term) in &self.terminals {
            if !term.populated {
                continue;
            }
            for role in &term.wants {
                let want = &doc.roles[role];
                if !want.kind.is_pull() {
                    continue;
                }
                let mut check = ConnectionCheck {
                    source_role: None,
                    target: pin.clone(),
                    bridge_pins: vec![],
                    satisfied: false,
                    unknown: false,
                    detail: format!("Missing {} on {pin}", want.kind.word().replace('_', "-")),
                };
                // An incompatible powered rail overrides a good path. An unrated resistor
                // may be an unrelated signal path, so prefer demonstrated evidence to it.
                let mut best = PullEvidence::Missing;
                for s in sources.get(self.net_of[pin].as_str()).into_iter().flatten() {
                    let (rank, detail) = s.evaluate(want, doc);
                    if rank > best {
                        best = rank;
                        check = ConnectionCheck {
                            source_role: s.internal.clone(),
                            target: s.target.clone(),
                            bridge_pins: s.bridge.clone(),
                            satisfied: rank == PullEvidence::Verified,
                            unknown: matches!(rank, PullEvidence::UnknownRail | PullEvidence::UnknownValue),
                            detail,
                        };
                    }
                }
                if let Some((id, v)) = rails.get(self.net_of[pin].as_str()).and_then(|r| r.first()) {
                    check = ConnectionCheck {
                        source_role: None,
                        target: (*id).into(),
                        bridge_pins: vec![],
                        satisfied: false,
                        unknown: false,
                        detail: format!("Signal is tied directly to {v} at {id} without a pull resistor"),
                    };
                }
                if !check.satisfied {
                    let mut objects = vec![pin.clone()];
                    if check.target != *pin {
                        objects.push(check.target.clone());
                    }
                    objects.extend_from_slice(&check.bridge_pins);
                    self.issues.push(super::graph::Issue {
                        kind: "PullMismatch",
                        unknown: check.unknown,
                        objects,
                        message: check.detail.clone(),
                    });
                }
                self.connections
                    .entry(pin.clone())
                    .or_default()
                    .insert(role.clone(), check);
            }
        }
    }
}
