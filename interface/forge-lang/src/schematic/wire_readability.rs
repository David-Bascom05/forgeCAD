//! Sheet-space warnings for misleading wire geometry. These never change electrical attachments.
use super::*;

const NEAR_MM: f64 = 0.5;
const EPS_MM: f64 = 1e-7;

struct Segment<'a> {
    wire: &'a str,
    net: &'a str,
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
pub(super) fn findings(c: &Circuit, scope: &str) -> Vec<super::graph::Issue> {
    let prefix = format!("{scope}/");
    let local = |id: &str| !id[prefix.len()..].contains('/');
    let mut segments = Vec::new();
    let mut pairs = Set::new();
    let mut pins: Vec<_> = c
        .terminals
        .range(prefix.clone()..)
        .take_while(|(id, _)| id.starts_with(&prefix))
        .filter(|(id, _)| local(id))
        .collect();
    pins.sort_by(|a, b| a.1.at[0].total_cmp(&b.1.at[0]));
    for (wire, (a, b, via)) in c
        .wires
        .range(prefix.clone()..)
        .take_while(|(id, _)| id.starts_with(&prefix))
        .filter(|(id, _)| local(id))
    {
        let net = c.net_of[a].as_str();
        for end in [a, b] {
            let p = c.locations[end];
            let lo = pins.partition_point(|(_, t)| t.at[0] < p[0] - NEAR_MM);
            for &(id, t) in pins[lo..].iter().take_while(|(_, t)| t.at[0] <= p[0] + NEAR_MM) {
                if c.net_of[id] != net && (p[0] - t.at[0]).hypot(p[1] - t.at[1]) <= NEAR_MM {
                    pairs.insert(("WireNearPin", wire.as_str(), id.as_str()));
                }
            }
        }
        let mut prev = c.locations[a];
        for &next in via.iter().chain(std::iter::once(&c.locations[b])) {
            segments.push(Segment {
                wire,
                net,
                a: prev,
                b: next,
            });
            prev = next;
        }
    }
    segments.sort_by(|a, b| a.left().total_cmp(&b.left()));
    let mut active: Vec<&Segment> = Vec::new();
    for s in &segments {
        active.retain(|p| p.right() + EPS_MM >= s.left());
        for other in &active {
            if s.net != other.net && s.overlaps(other) {
                pairs.insert(("WireOverlap", s.wire.min(other.wire), s.wire.max(other.wire)));
            }
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
