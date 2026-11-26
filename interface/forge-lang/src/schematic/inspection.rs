//! Inspect electrical evidence independently of the sheet projection or viewport.
use super::*;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Reverse,
    Conflict,
}

fn distances<'a>(c: &'a Circuit, roots: impl IntoIterator<Item = &'a str>) -> Map<&'a str, usize> {
    let mut queue: VecDeque<_> = roots.into_iter().collect();
    let mut result: Map<_, _> = queue.iter().map(|&key| (key, 0)).collect();
    while let Some(a) = queue.pop_front() {
        let next = result[a] + 1;
        for b in &c.adjacency[a] {
            if let std::collections::btree_map::Entry::Vacant(slot) = result.entry(b.as_str()) {
                slot.insert(next);
                queue.push_back(b);
            }
        }
    }
    result
}
/// Trace a declared direction along all electrically shortest routes between its endpoints.
/// Wire storage order supplies an orientation for drawing, never evidence for a direction.
pub fn wire_directions(c: &Circuit) -> Map<String, Direction> {
    let mut by_net: Map<&str, Vec<_>> = Map::new();
    for (wire, (a, b, _)) in &c.wires {
        by_net
            .entry(&c.net_of[a])
            .or_default()
            .push((wire, a.as_str(), b.as_str()));
    }
    let mut result = Map::new();
    for (net, wires) in &by_net {
        let sources: Vec<_> = c.nets[*net]
            .iter()
            .filter(|p| {
                c.terminals
                    .get(*p)
                    .is_some_and(|t| matches!(t.electrical, Electrical::Output | Electrical::Source))
            })
            .map(String::as_str)
            .collect();
        if sources.is_empty() {
            continue;
        }
        let competing = sources.len() > 1;
        let d = distances(c, sources);
        for &(wire, a, b) in wires {
            let direction = if competing {
                Direction::Conflict
            } else if d[a] < d[b] {
                Direction::Forward
            } else if d[a] > d[b] {
                Direction::Reverse
            } else {
                continue;
            };
            result.insert(wire.clone(), direction);
        }
    }
    for (from, to) in c.flows.values() {
        let net = &c.net_of[from];
        if c.net_of[to] != *net || from == to {
            continue;
        }
        let Some(wires) = by_net.get(net.as_str()) else {
            continue;
        };
        let start = distances(c, [from.as_str()]);
        let end = distances(c, [to.as_str()]);
        let length = start[to.as_str()];
        for &(wire, a, b) in wires {
            let dir = if start[a] + 1 + end[b] == length {
                Direction::Forward
            } else if start[b] + 1 + end[a] == length {
                Direction::Reverse
            } else {
                continue;
            };
            result
                .entry(wire.clone())
                .and_modify(|previous| {
                    if *previous != dir {
                        *previous = Direction::Conflict;
                    }
                })
                .or_insert(dir);
        }
    }
    result
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Evidence {
    Verified,
    Candidate,
    Invalid,
}
#[derive(Clone, Debug)]
pub struct RoleCheck {
    pub role: RoleId,
    pub need: bool,
    pub measured: bool,
    pub peers: Vec<(String, Evidence)>,
}
#[derive(Default, Debug)]
pub struct PinInspection {
    pub checks: Vec<RoleCheck>,
    /// One connected power symbol per rail name, preferring the local drawn path.
    pub rails: Map<String, String>,
    pub pins: Set<String>,
    pub wires: Set<String>,
}

/// Return both sides of role assignments, keeping same-net possibilities distinct from verified links.
pub fn inspect_pin(c: &Circuit, pin: &str) -> Option<PinInspection> {
    let p = c.terminals.get(pin)?;
    let net = &c.nets[&c.net_of[pin]];
    let mut inspection = PinInspection::default();
    inspection.pins.insert(pin.to_owned());
    let mut targets = Set::new();
    let invalid: Set<_> = c
        .issues
        .iter()
        .filter(|i| matches!(i.kind, "DisconnectedFulfillment" | "ReturnMismatch"))
        .map(|i| &i.objects[0])
        .collect();
    for (need, roles) in [(true, &p.wants), (false, &p.gives)] {
        for role in roles {
            if need && let Some(check) = c.connections.get(pin).and_then(|w| w.get(role)) {
                let mut peers = Vec::new();
                let state = if check.satisfied {
                    Evidence::Verified
                } else {
                    Evidence::Invalid
                };
                for id in std::iter::once(&check.target).chain(&check.bridge_pins) {
                    if c.terminals.contains_key(id) {
                        peers.push((id.clone(), state));
                        inspection.pins.insert(id.clone());
                        if c.net_of[id] == c.net_of[pin] {
                            targets.insert(id.clone());
                        }
                    }
                }
                if let [_, far] = check.bridge_pins.as_slice() {
                    trace_paths(c, far, &Set::from([check.target.clone()]), &mut inspection, false);
                }
                inspection.checks.push(RoleCheck {
                    role: role.clone(),
                    need,
                    measured: true,
                    peers,
                });
                continue;
            }
            let mut peers = Map::new();
            for i in c.relations.get(pin).into_iter().flatten() {
                let link = &c.links[*i];
                let (this, other) = if need {
                    (&link.want, &link.give)
                } else {
                    (&link.give, &link.want)
                };
                if this != pin || link.role != *role {
                    continue;
                }
                let state = if invalid.contains(&link.id) {
                    Evidence::Invalid
                } else {
                    Evidence::Verified
                };
                peers.insert(other.clone(), state);
            }
            for k in net {
                if k != pin
                    && c.terminals
                        .get(k)
                        .is_some_and(|t| (if need { &t.gives } else { &t.wants }).contains(role))
                {
                    peers.entry(k.clone()).or_insert(Evidence::Candidate);
                }
            }
            let has_link = c
                .relations
                .get(pin)
                .into_iter()
                .flatten()
                .any(|i| c.links[*i].role == *role);
            let required = c.voltages.get(pin).and_then(|v| v.get(role)).filter(|_| need);
            if required.is_some() {
                peers.clear();
            }
            let mut measured = required.is_some();
            let mut record = |other: &str, accepted: bool| {
                let state = if accepted {
                    Evidence::Verified
                } else {
                    Evidence::Invalid
                };
                let entry = peers.entry(other.to_owned()).or_insert(state);
                if *entry != Evidence::Invalid {
                    *entry = state;
                }
            };
            if let Some(check) = required {
                for source in &check.providers {
                    record(&source.pin, check.required.accepts(source.range));
                }
            } else if !need {
                for other in net {
                    for check in c.voltages.get(other).into_iter().flat_map(|v| v.values()) {
                        for source in &check.providers {
                            if source.pin == pin && source.role == *role {
                                measured = !has_link;
                                record(other, check.required.accepts(source.range));
                            }
                        }
                    }
                }
            }
            targets.extend(peers.keys().filter(|k| c.net_of[*k] == c.net_of[pin]).cloned());
            inspection.checks.push(RoleCheck {
                role: role.clone(),
                need,
                measured,
                peers: peers.into_iter().collect(),
            });
        }
    }
    trace_paths(c, pin, &targets, &mut inspection, true);
    Some(inspection)
}

fn trace_paths(
    c: &Circuit,
    pin: &str,
    targets: &Set<String>,
    inspection: &mut PinInspection,
    include_rails: bool,
) {
    // Start on the selected side. A return still affects ERC evidence, but belongs to
    // a different net. Prefer visible wires before crossing a label or sheet boundary.
    let scope = pin.rsplit_once('/').unwrap().0;
    let mut parent = Map::from([(pin, pin)]);
    let mut local = VecDeque::from([pin]);
    let mut remote = VecDeque::new();
    let mut on_sheet = true;
    while let Some(node) = local.pop_front().or_else(|| {
        on_sheet = false;
        remote.pop_front()
    }) {
        if include_rails && let Some(name) = c.terminals.get(node).and_then(|t| t.power_net.as_ref()) {
            inspection
                .rails
                .entry(name.clone())
                .or_insert_with(|| node.into());
        }
        for next in &c.adjacency[node] {
            let next = next.as_str();
            if let std::collections::btree_map::Entry::Vacant(slot) = parent.entry(next) {
                slot.insert(node);
                if on_sheet && next.rsplit_once('/').is_some_and(|(s, _)| s == scope) {
                    local.push_back(next);
                } else {
                    remote.push_back(next);
                }
            }
        }
    }
    let rails = inspection.rails.values().filter(|_| include_rails);
    let mut edges = Set::new();
    for target in targets.iter().chain(rails) {
        let mut here = target.as_str();
        while let Some(&prev) = parent.get(here) {
            if c.terminals.contains_key(here) {
                inspection.pins.insert(here.into());
            }
            if prev == here {
                break;
            }
            edges.insert((prev.min(here).to_owned(), prev.max(here).to_owned()));
            here = prev;
        }
    }
    for (id, (a, b, _)) in &c.wires {
        if edges.contains(&(a.min(b).clone(), a.max(b).clone())) {
            inspection.wires.insert(id.clone());
        }
    }
}
