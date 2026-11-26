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
    pub peers: Vec<(String, Evidence)>,
}
#[derive(Default, Debug)]
pub struct PinInspection {
    pub checks: Vec<RoleCheck>,
    pub pins: Set<String>,
    pub wires: Set<String>,
}

/// Return both sides of role assignments, keeping same-net possibilities distinct from verified links.
pub fn inspect_pin(c: &Circuit, pin: &str) -> Option<PinInspection> {
    let p = c.terminals.get(pin)?;
    let net = &c.nets[&c.net_of[pin]];
    let mut inspection = PinInspection::default();
    inspection.pins.insert(pin.to_owned());
    let mut routes: Map<String, Set<String>> = Map::new();
    let invalid: Set<_> = c
        .issues
        .iter()
        .filter(|i| matches!(i.kind, "DisconnectedFulfillment" | "ReturnMismatch"))
        .map(|i| &i.objects[0])
        .collect();
    for (need, roles) in [(true, &p.wants), (false, &p.gives)] {
        for role in roles {
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
                if let Some((a, b)) = &link.returns {
                    inspection.pins.extend([a.clone(), b.clone()]);
                    routes.entry(a.clone()).or_default().insert(b.clone());
                }
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
            inspection.pins.extend(peers.keys().cloned());
            routes
                .entry(pin.into())
                .or_default()
                .extend(peers.keys().cloned());
            inspection.checks.push(RoleCheck {
                role: role.clone(),
                need,
                peers: peers.into_iter().collect(),
            });
        }
    }
    // Build one predecessor tree per source. Virtual label/hierarchy edges participate
    // in reachability; only actual wire segments become highlighted drawing objects.
    let mut edges = Set::new();
    for (root, targets) in &routes {
        let mut parent = Map::from([(root.as_str(), root.as_str())]);
        let mut queue = VecDeque::from([root.as_str()]);
        while let Some(a) = queue.pop_front() {
            for b in &c.adjacency[a] {
                if let std::collections::btree_map::Entry::Vacant(slot) = parent.entry(b.as_str()) {
                    slot.insert(a);
                    queue.push_back(b);
                }
            }
        }
        for target in targets {
            let mut here = target.as_str();
            while let Some(&prev) = parent.get(here) {
                if prev == here {
                    break;
                }
                for node in [prev, here] {
                    if c.terminals.contains_key(node) {
                        inspection.pins.insert(node.into());
                    }
                }
                edges.insert((prev.min(here).to_owned(), prev.max(here).to_owned()));
                here = prev;
            }
        }
    }
    for (id, (a, b, _)) in &c.wires {
        if edges.contains(&(a.min(b).clone(), a.max(b).clone())) {
            inspection.wires.insert(id.clone());
        }
    }
    Some(inspection)
}
