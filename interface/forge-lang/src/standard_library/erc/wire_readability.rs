//! Sheet-space warnings for misleading wire geometry. These never change electrical attachments.
use super::*;

const NEAR_MM: f64 = 0.5;
const EPS_MM: f64 = 1e-7;

#[derive(Clone, Copy)]
struct Bounds {
    lo: Point,
    hi: Point,
}
impl Bounds {
    fn new(p: Point) -> Self {
        Self { lo: p, hi: p }
    }
    fn include(&mut self, p: Point) {
        for (i, x) in p.into_iter().enumerate() {
            self.lo[i] = self.lo[i].min(x);
            self.hi[i] = self.hi[i].max(x);
        }
    }
    fn near(self, b: Self) -> bool {
        (0..2).all(|i| self.lo[i] <= b.hi[i] + NEAR_MM && b.lo[i] <= self.hi[i] + NEAR_MM)
    }
}
struct Segment<'a> {
    wire: &'a str,
    net: &'a str,
    changed: bool,
    a: Point,
    b: Point,
}
impl Segment<'_> {
    fn left(&self) -> f64 {
        self.a[0].min(self.b[0])
    }
    fn right(&self) -> f64 {
        self.a[0].max(self.b[0])
    }
    fn overlaps(&self, other: &Self) -> bool {
        let dx = self.b[0] - self.a[0];
        let dy = self.b[1] - self.a[1];
        let length = dx.hypot(dy);
        if length <= EPS_MM {
            return false;
        }
        let projected = |p: Point| {
            let (x, y) = (p[0] - self.a[0], p[1] - self.a[1]);
            ((dx * y - dy * x).abs() <= EPS_MM * length).then_some((dx * x + dy * y) / length)
        };
        if let (Some(u), Some(v)) = (projected(other.a), projected(other.b)) {
            return length.min(u.max(v)) - 0f64.max(u.min(v)) > EPS_MM;
        }
        false
    }
}
pub(super) fn findings(c: &Circuit, scope: &str, changed: Option<&Set<String>>) -> Vec<super::graph::Issue> {
    let prefix = format!("{scope}/");
    let local = |id: &str| !id[prefix.len()..].contains('/');
    // Only geometry near a moved pin or its incident wires can acquire a finding.
    // Prior findings for those objects have already been removed by the move cache.
    let mut affected: Option<Bounds> = None;
    if let Some(ids) = changed {
        let mut include = |p| affected.get_or_insert(Bounds::new(p)).include(p);
        for id in ids.iter().filter(|id| id.starts_with(&prefix) && local(id)) {
            if let Some(t) = c.terminals.get(id) {
                include(t.at);
            }
            if let Some((a, b, via)) = c.wires.get(id) {
                include(c.locations[a]);
                include(c.locations[b]);
                for &p in via {
                    include(p);
                }
            }
        }
        if affected.is_none() {
            return vec![];
        }
    }
    let mut segments = Vec::new();
    let mut pairs = Set::new();
    let mut pins: Vec<_> = c
        .terminals
        .range(prefix.clone()..)
        .take_while(|(id, _)| id.starts_with(&prefix))
        .filter(|(id, t)| local(id) && affected.is_none_or(|r| r.near(Bounds::new(t.at))))
        .collect();
    pins.sort_by(|a, b| a.1.at[0].total_cmp(&b.1.at[0]));
    let mut moved_pins: Vec<_> = changed
        .into_iter()
        .flatten()
        .filter_map(|id| Some((id, c.terminals.get(id)?)))
        .filter(|(id, _)| id.starts_with(&prefix) && local(id))
        .collect();

    moved_pins.sort_by(|a, b| a.1.at[0].total_cmp(&b.1.at[0]));
    for (wire, (a, b, via)) in c
        .wires
        .range(prefix.clone()..)
        .take_while(|(id, _)| id.starts_with(&prefix))
        .filter(|(id, _)| local(id))
    {
        let (start, end) = (c.locations[a], c.locations[b]);
        if let Some(region) = affected {
            let mut bounds = Bounds::new(start);
            bounds.include(end);
            for &p in via {
                bounds.include(p);
            }
            if !region.near(bounds) {
                continue;
            }
        }
        let wire_changed = changed.is_none_or(|ids| ids.contains(wire));
        let candidates = if wire_changed { &pins } else { &moved_pins };
        let net = c.net_of[a].as_str();
        for p in [start, end] {
            let lo = candidates.partition_point(|(_, t)| t.at[0] < p[0] - NEAR_MM);
            for &(id, t) in candidates[lo..]
                .iter()
                .take_while(|(_, t)| t.at[0] <= p[0] + NEAR_MM)
            {
                if (p[1] - t.at[1]).abs() <= NEAR_MM
                    && (p[0] - t.at[0]).hypot(p[1] - t.at[1]) <= NEAR_MM
                    && c.net_of[id] != net
                {
                    pairs.insert(("WireNearPin", wire.as_str(), id.as_str()));
                }
            }
        }
        let mut prev = start;
        for &next in via.iter().chain(std::iter::once(&end)) {
            segments.push(Segment {
                wire,
                net,
                changed: wire_changed,
                a: prev,
                b: next,
            });
            prev = next;
        }
    }
    segments.sort_by(|a, b| a.left().total_cmp(&b.left()));
    let mut active: Vec<&Segment> = Vec::new();
    let mut active_changes: Vec<&Segment> = Vec::new();
    for s in &segments {
        active.retain(|p| p.right() + EPS_MM >= s.left());
        active_changes.retain(|p| p.right() + EPS_MM >= s.left());
        for other in if s.changed { &active } else { &active_changes } {
            if s.a[1].min(s.b[1]) <= other.a[1].max(other.b[1]) + EPS_MM
                && other.a[1].min(other.b[1]) <= s.a[1].max(s.b[1]) + EPS_MM
                && s.net != other.net
                && s.overlaps(other)
            {
                pairs.insert(("WireOverlap", s.wire.min(other.wire), s.wire.max(other.wire)));
            }
        }
        if s.changed && changed.is_some() {
            active_changes.push(s);
        }
        active.push(s);
    }
    pairs
        .into_iter()
        .map(|(kind, a, b)| super::graph::Issue {
            kind,
            unknown: false,
            objects: vec![a.into(), b.into()],
            message: if kind == "WireNearPin" {
                format!("Wire {a} ends within {NEAR_MM} mm of pin {b} on a different net")
            } else {
                format!("Wires {a} and {b} overlap along their length on different nets")
            },
        })
        .collect()
}
