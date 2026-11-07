//! Wire harnesses: a drawn network of nodes and segments, the wires routed through it from pin to
//! pin, each segment's bundle and minimum bend radius, its centreline of lines and tangent arcs,
//! every wire's length, and the tubes. All of it closed form, in one fixed order:
//!
//! 1. each wire's path: the one given, or the shortest through the network (Dijkstra on the
//!    segments' polyline lengths, ties to the lower segment);
//! 2. each segment's bundle diameter from the wires in it, and from that its minimum radius;
//! 3. its centreline: the polyline (lead points off directed nodes, then the vias) with every
//!    corner an arc of that radius, or the tightest that fits with a `BendTooTight`;
//! 4. each wire's length, every term kept.
//!
//! The maths is checked in research/harness/derive.py.

pub mod tube;
pub mod wires;

use forge_math::{Point3, UnitVec3, dmath};
use forge_ops::sweep::PathSeg;

/// A point of the network. A directed node (a clip, a connector's exit) passes every route through
/// it along `dir`, either way.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: String,
    pub at: Point3,
    pub dir: Option<UnitVec3>,
    /// How far a route runs straight off a directed node before its first corner; None: the
    /// segment's minimum radius (room for a quarter turn).
    pub lead: Option<f64>,
    /// A clip seated at `at`, standing along this: the bundle runs through it, its axis the
    /// clip's base, its wall and half the thickest bundle through the node out along it.
    pub clip: Option<UnitVec3>,
}

/// Wires that run together as one: a twisted pair (or more), turning about their middle once
/// every `lay` along the run.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub id: String,
    pub wires: Vec<usize>,
    pub lay: f64,
}

/// A clip's wall and the room it leaves round its bundle (mm).
pub const CLIP_WALL: f64 = 1.2;

#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    pub id: String,
    pub a: usize,
    pub b: usize,
    /// Corners between the ends, filleted.
    pub via: Vec<Point3>,
    /// The bundle's packing factor.
    pub k: f64,
    /// The minimum radius per bundle diameter.
    pub bend_k: f64,
    /// A covering's wall, on top of the bundle.
    pub wall: f64,
    /// Length added to every wire through the segment.
    pub service_loop: f64,
    /// A diameter given, not worked out from the wires.
    pub d: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WireType {
    pub id: String,
    /// The insulation's outside diameter.
    pub od: f64,
    /// The wire's own minimum bend radius; 0 where it has none.
    pub min_bend: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Connector {
    pub id: String,
    /// The node routes leave the connector from.
    pub exit: usize,
    pub pins: Vec<(String, Point3)>,
    /// How far a wire goes into a pin.
    pub insert: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct End {
    pub connector: usize,
    pub pin: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Wire {
    pub id: String,
    pub ty: usize,
    pub from: End,
    pub to: End,
    /// The segments it runs through, in order from `from`; None: the shortest.
    pub path: Option<Vec<usize>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// Added to every wire's route, as a fraction of it.
    pub slack: f64,
    /// Lengths rounded up to a multiple of this; 0: not rounded.
    pub round_up: f64,
    /// The diameter drawn for a segment no wire runs through.
    pub empty_d: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            slack: 0.0,
            round_up: 0.0,
            empty_d: 2.0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Network {
    pub nodes: Vec<Node>,
    pub segments: Vec<Segment>,
    pub types: Vec<WireType>,
    pub connectors: Vec<Connector>,
    pub wires: Vec<Wire>,
    pub groups: Vec<Group>,
    pub settings: Settings,
}

/// A corner of a segment's centreline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Corner {
    /// The polyline's vertex.
    pub at: Point3,
    /// The angle the route turns through (radians).
    pub turn: f64,
    pub needed: f64,
    pub got: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SegmentOut {
    /// The wires through it, by index.
    pub wires: Vec<usize>,
    pub d: f64,
    pub r_min: f64,
    /// Its centreline from node a to node b: tangent-continuous lines and arcs.
    pub pieces: Vec<PathSeg>,
    pub length: f64,
    pub corners: Vec<Corner>,
    /// The centreline's direction at a and at b, along a to b.
    pub tangents: [Option<UnitVec3>; 2],
    /// How far it runs straight off each end before its first corner.
    pub straight: [f64; 2],
}

/// A routed wire's length, term by term.
#[derive(Clone, Debug, PartialEq)]
pub struct WireOut {
    /// The segments, each with whether it is run from its a to its b.
    pub path: Vec<(usize, bool)>,
    /// Pin to exit node, at each end.
    pub stubs: [f64; 2],
    /// The stubs and the segments' centrelines.
    pub route: f64,
    pub loops: f64,
    pub slack: f64,
    pub insertion: f64,
    /// route + loops + slack + insertion.
    pub exact: f64,
    /// `exact` rounded up.
    pub length: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Note {
    /// A corner tighter than the segment's minimum radius: built with the tightest that fits.
    BendTooTight {
        segment: usize,
        corner: usize,
        at: Point3,
        needed: f64,
        got: f64,
    },
    /// A wire turns `angle` (radians) through a node with no direction, toward `toward`.
    KinkAtNode {
        node: usize,
        wire: usize,
        angle: f64,
        toward: UnitVec3,
    },
    /// No chain of segments joins the wire's connectors.
    NoPath {
        wire: usize,
    },
    /// The given path's segment `at` (its index in the path) does not start where the one before
    /// it ends.
    PathBroken {
        wire: usize,
        at: usize,
    },
    SegmentEmpty {
        segment: usize,
    },
    /// A group's wire does not run between the same connectors as the group's first: routed on
    /// its own.
    GroupSplit {
        group: usize,
        wire: usize,
    },
}

/// A clip as built: where its bundle's centre is, the bundle's direction there, the way the
/// clip stands, the bundle's diameter it holds and where it is seated.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipOut {
    pub node: usize,
    pub at: Point3,
    pub axis: UnitVec3,
    pub seat: UnitVec3,
    pub d: f64,
    pub base: Point3,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Routed {
    /// Each node where it ends up (a clip's: lifted to hold its bundle).
    pub nodes_at: Vec<Point3>,
    pub clips: Vec<ClipOut>,
    pub segments: Vec<SegmentOut>,
    /// By wire; None where it has no path.
    pub wires: Vec<Option<WireOut>>,
    pub notes: Vec<Note>,
}

/// A conductor's diameter (mm) by AWG, ASTM B258's defining formula: 0.127 mm at 36, 92 times
/// that at 0000 (n = -3), geometric between.
pub fn awg_d(n: f64) -> f64 {
    0.127 * dmath::powf(92.0, (36.0 - n) / 39.0)
}

/// A bundle's diameter: one wire is its own; from two up, sqrt(k sum d^2), never under the two
/// largest side by side (derive.py section 5).
pub fn bundle_d(ods: &[f64], k: f64) -> f64 {
    match ods {
        [] => 0.0,
        [d] => *d,
        _ => {
            let (mut d1, mut d2) = (0.0f64, 0.0f64);
            let mut sq = 0.0;
            for &d in ods {
                sq += d * d;
                if d > d1 {
                    d2 = d1;
                    d1 = d;
                } else if d > d2 {
                    d2 = d;
                }
            }
            (k * sq).sqrt().max(d1 + d2)
        }
    }
}

/// Lengths under this are none.
const TINY: f64 = 1e-9;
/// Turns under this (radians) are straight.
const STRAIGHT: f64 = 1e-9;

fn tan_half(theta: f64) -> f64 {
    let (s, c) = dmath::sin_cos(theta / 2.0);
    if c <= TINY { f64::INFINITY } else { s / c }
}

fn poly_length(net: &Network, s: &Segment) -> f64 {
    let mut at = net.nodes[s.a].at;
    let mut l = 0.0;
    for p in s.via.iter().copied().chain([net.nodes[s.b].at]) {
        l += at.distance(p);
        at = p;
    }
    l
}

/// The shortest chain of segments from node `from` to node `to`, each with its direction.
fn shortest(net: &Network, weight: &[f64], from: usize, to: usize) -> Option<Vec<(usize, bool)>> {
    let n = net.nodes.len();
    let mut dist = vec![f64::INFINITY; n];
    let mut prev: Vec<Option<(usize, bool, usize)>> = vec![None; n];
    let mut done = vec![false; n];
    dist[from] = 0.0;
    loop {
        let mut u = None;
        for i in 0..n {
            if !done[i] && dist[i].is_finite() && u.is_none_or(|j: usize| dist[i] < dist[j]) {
                u = Some(i);
            }
        }
        let Some(u) = u else { break };
        if u == to {
            break;
        }
        done[u] = true;
        for (k, s) in net.segments.iter().enumerate() {
            for (here, there, fwd) in [(s.a, s.b, true), (s.b, s.a, false)] {
                if here == u && !done[there] && dist[u] + weight[k] < dist[there] {
                    dist[there] = dist[u] + weight[k];
                    prev[there] = Some((k, fwd, u));
                }
            }
        }
    }
    if !dist[to].is_finite() {
        return None;
    }
    let mut path = vec![];
    let mut at = to;
    while let Some((k, fwd, p)) = prev[at] {
        path.push((k, fwd));
        at = p;
    }
    path.reverse();
    Some(path)
}

/// A given path, oriented from node `from`; Err: the index of the first segment that does not
/// join the one before.
fn walk(net: &Network, segs: &[usize], from: usize, to: usize) -> Result<Vec<(usize, bool)>, usize> {
    let mut at = from;
    let mut out = vec![];
    for (i, &k) in segs.iter().enumerate() {
        let s = net.segments.get(k).ok_or(i)?;
        if s.a == at {
            out.push((k, true));
            at = s.b;
        } else if s.b == at {
            out.push((k, false));
            at = s.a;
        } else {
            return Err(i);
        }
    }
    if at != to {
        return Err(segs.len());
    }
    Ok(out)
}

/// The polyline a segment's centreline rounds: node a, its lead point, the vias, node b's lead
/// point, node b; repeated points dropped.
fn polyline(nodes: &[Node], s: &Segment, r_min: f64) -> Vec<Point3> {
    let (a, b) = (&nodes[s.a], &nodes[s.b]);
    let lead = |n: &Node, toward: Point3| {
        n.dir.map(|d| {
            let sign = if d.dot(toward - n.at) < 0.0 { -1.0 } else { 1.0 };
            n.at + d.get() * (sign * n.lead.unwrap_or(r_min))
        })
    };
    let mut pts = vec![a.at];
    pts.extend(lead(a, s.via.first().copied().unwrap_or(b.at)));
    pts.extend(s.via.iter().copied());
    pts.extend(lead(b, s.via.last().copied().unwrap_or(a.at)));
    pts.push(b.at);
    pts.dedup_by(|p, q| p.distance(*q) <= TINY);
    pts
}

/// A segment's centreline: corners rounded at `r_min`, or the tightest that fit, and its length.
fn centreline(pts: &[Point3], r_min: f64) -> (Vec<PathSeg>, f64, Vec<Corner>) {
    let legs: Vec<(UnitVec3, f64)> = pts
        .windows(2)
        .filter_map(|w| Some(((w[1] - w[0]).normalize()?, w[0].distance(w[1]))))
        .collect();
    let n = legs.len().saturating_sub(1);
    let turns: Vec<f64> = (0..n)
        .map(|i| dmath::acos_clamped(legs[i].0.dot(legs[i + 1].0)))
        .collect();
    let want: Vec<f64> = turns
        .iter()
        .map(|&t| if t <= STRAIGHT { 0.0 } else { r_min * tan_half(t) })
        .collect();
    // each leg holds its two corners' tangents, both scaled down where it cannot (derive.py 2)
    let scale: Vec<f64> = (0..legs.len())
        .map(|j| {
            let need = if j >= 1 { want[j - 1] } else { 0.0 } + if j < n { want[j] } else { 0.0 };
            if need > legs[j].1 { legs[j].1 / need } else { 1.0 }
        })
        .collect();
    let mut pieces = vec![];
    let mut corners = vec![];
    let mut length = 0.0;
    let mut cur = pts[0];
    let line = |a: Point3, b: Point3, pieces: &mut Vec<PathSeg>, length: &mut f64| {
        if a.distance(b) > TINY {
            *length += a.distance(b);
            pieces.push(PathSeg::Line { a, b });
        }
    };
    for i in 0..n {
        let (u, w) = (legs[i].0, legs[i + 1].0);
        let v = pts[i + 1];
        let theta = turns[i];
        if theta <= STRAIGHT {
            continue;
        }
        let t = if want[i].is_finite() {
            want[i] * scale[i].min(scale[i + 1])
        } else {
            0.0
        };
        let th = tan_half(theta);
        let got = if th.is_finite() { t / th } else { 0.0 };
        corners.push(Corner {
            at: v,
            turn: theta,
            needed: r_min,
            got,
        });
        let p_in = v - u.get() * t;
        line(cur, p_in, &mut pieces, &mut length);
        cur = p_in;
        let arc = u.cross(w).normalize().and_then(|z| {
            let c = p_in + z.cross(u) * got;
            let frame = forge_math::Frame::from_zx(c, z, p_in - c)?;
            (got > TINY).then_some(PathSeg::Arc {
                circle: forge_geom::Circle { frame, radius: got },
                start: 0.0,
                sweep: theta,
            })
        });
        if let Some(arc) = arc {
            length += got * theta;
            pieces.push(arc);
            cur = v + w.get() * t;
        }
    }
    line(cur, pts[pts.len() - 1], &mut pieces, &mut length);
    (pieces, length, corners)
}

/// Routes the network: paths, bundles, centrelines, lengths and notes.
pub fn route(net: &Network) -> Routed {
    let mut notes = vec![];
    let weight: Vec<f64> = net.segments.iter().map(|s| poly_length(net, s)).collect();
    let mut paths: Vec<Option<Vec<(usize, bool)>>> = Vec::with_capacity(net.wires.len());
    for (k, w) in net.wires.iter().enumerate() {
        let from = net.connectors[w.from.connector].exit;
        let to = net.connectors[w.to.connector].exit;
        let p = match &w.path {
            Some(segs) => match walk(net, segs, from, to) {
                Ok(p) => Some(p),
                Err(at) => {
                    notes.push(Note::PathBroken { wire: k, at });
                    None
                }
            },
            None => {
                let p = shortest(net, &weight, from, to);
                if p.is_none() {
                    notes.push(Note::NoPath { wire: k });
                }
                p
            }
        };
        paths.push(p);
    }
    // a group runs as one: its wires on its first's path, where they join the same connectors
    let exits = |w: &Wire| {
        let (a, b) = (
            net.connectors[w.from.connector].exit,
            net.connectors[w.to.connector].exit,
        );
        (a.min(b), a.max(b))
    };
    for (g, grp) in net.groups.iter().enumerate() {
        let Some(&lead) = grp.wires.first() else { continue };
        let (from_lead, lead_path) = (
            net.connectors[net.wires[lead].from.connector].exit,
            paths[lead].clone(),
        );
        for &w in &grp.wires[1..] {
            if exits(&net.wires[w]) != exits(&net.wires[lead]) {
                notes.push(Note::GroupSplit { group: g, wire: w });
                continue;
            }
            // the same chain, run the other way when the wire starts at the lead's far end
            let same_way = net.connectors[net.wires[w].from.connector].exit == from_lead;
            paths[w] = lead_path.clone().map(|p| {
                if same_way {
                    p
                } else {
                    p.into_iter().rev().map(|(s, f)| (s, !f)).collect()
                }
            });
        }
    }
    let mut through: Vec<Vec<usize>> = vec![vec![]; net.segments.len()];
    for (k, p) in paths.iter().enumerate() {
        for &(s, _) in p.iter().flatten() {
            if through[s].last() != Some(&k) {
                through[s].push(k);
            }
        }
    }
    // each segment's bundle: a group one item of its wires packed
    let mut sizes: Vec<(Vec<usize>, f64, f64)> = Vec::with_capacity(net.segments.len());
    for (k, s) in net.segments.iter().enumerate() {
        let wires = std::mem::take(&mut through[k]);
        let ods = item_ods(net, &wires);
        let d = match s.d {
            Some(d) => d,
            None if wires.is_empty() => {
                notes.push(Note::SegmentEmpty { segment: k });
                net.settings.empty_d
            }
            None => bundle_d(&ods, s.k) + 2.0 * s.wall,
        };
        let r_min = wires
            .iter()
            .map(|&w| net.types[net.wires[w].ty].min_bend)
            .fold(s.bend_k * d, f64::max);
        sizes.push((wires, d, r_min));
    }
    // clips lifted to hold the thickest bundle through them
    let mut nodes = net.nodes.clone();
    let mut clips = vec![];
    for (k, n) in net.nodes.iter().enumerate() {
        let Some(seat) = n.clip else { continue };
        let d = net
            .segments
            .iter()
            .zip(&sizes)
            .filter(|(s, _)| s.a == k || s.b == k)
            .map(|(_, z)| z.1)
            .fold(0.0, f64::max);
        nodes[k].at = n.at + seat.get() * (CLIP_WALL + d / 2.0);
        clips.push((k, seat, d));
    }
    let mut segments = Vec::with_capacity(net.segments.len());
    for (k, s) in net.segments.iter().enumerate() {
        let (wires, d, r_min) = std::mem::take(&mut sizes[k]);
        let pts = polyline(&nodes, s, r_min);
        let (pieces, length, corners) = centreline(&pts, r_min);
        for (c, corner) in corners.iter().enumerate() {
            if corner.got < corner.needed * (1.0 - 1e-9) {
                notes.push(Note::BendTooTight {
                    segment: k,
                    corner: c,
                    at: corner.at,
                    needed: corner.needed,
                    got: corner.got,
                });
            }
        }
        let tangents = [
            pts.get(1).and_then(|&p| (p - pts[0]).normalize()),
            pts.len()
                .checked_sub(2)
                .and_then(|i| (pts[i + 1] - pts[i]).normalize()),
        ];
        let line_len = |p: Option<&PathSeg>| match p {
            Some(PathSeg::Line { a, b }) => a.distance(*b),
            _ => 0.0,
        };
        let straight = [line_len(pieces.first()), line_len(pieces.last())];
        segments.push(SegmentOut {
            wires,
            d,
            r_min,
            pieces,
            length,
            corners,
            tangents,
            straight,
        });
    }
    let st = &net.settings;
    let mut wires = Vec::with_capacity(net.wires.len());
    for (k, (w, p)) in net.wires.iter().zip(paths).enumerate() {
        let Some(path) = p else {
            wires.push(None);
            continue;
        };
        let pin = |e: End| {
            let c = &net.connectors[e.connector];
            c.pins[e.pin].1.distance(net.nodes[c.exit].at)
        };
        let stubs = [pin(w.from), pin(w.to)];
        let route = stubs[0] + stubs[1] + path.iter().map(|&(s, _)| segments[s].length).sum::<f64>();
        let loops: f64 = path.iter().map(|&(s, _)| net.segments[s].service_loop).sum();
        let slack = (route + loops) * st.slack;
        let insertion = net.connectors[w.from.connector].insert + net.connectors[w.to.connector].insert;
        let exact = route + loops + slack + insertion;
        let length = if st.round_up > 0.0 {
            (exact / st.round_up - 1e-9).ceil() * st.round_up
        } else {
            exact
        };
        // turns through the nodes between segments
        for pair in path.windows(2) {
            let ((s1, f1), (s2, f2)) = (pair[0], pair[1]);
            let node = if f1 {
                net.segments[s1].b
            } else {
                net.segments[s1].a
            };
            let t1 = &segments[s1].tangents;
            let t2 = &segments[s2].tangents;
            let into = if f1 { t1[1] } else { t1[0].map(|t| -t) };
            let out = if f2 { t2[0] } else { t2[1].map(|t| -t) };
            let (Some(into), Some(out)) = (into, out) else {
                continue;
            };
            let angle = dmath::acos_clamped(into.dot(out));
            if angle > 1e-6 {
                let toward = (out.get() - into.get() * into.dot(out))
                    .normalize()
                    .unwrap_or_else(|| into.any_perpendicular());
                notes.push(Note::KinkAtNode {
                    node,
                    wire: k,
                    angle,
                    toward,
                });
            }
        }
        wires.push(Some(WireOut {
            path,
            stubs,
            route,
            loops,
            slack,
            insertion,
            exact,
            length,
        }));
    }
    let clips = clips
        .into_iter()
        .filter_map(|(k, seat, d)| {
            // the bundle's way through: its segments' tangent at the node
            let axis = segments.iter().zip(&net.segments).find_map(|(o, s)| {
                if s.a == k {
                    o.tangents[0]
                } else if s.b == k {
                    o.tangents[1]
                } else {
                    None
                }
            })?;
            Some(ClipOut {
                node: k,
                at: nodes[k].at,
                axis,
                seat,
                d,
                base: net.nodes[k].at,
            })
        })
        .collect();
    Routed {
        nodes_at: nodes.iter().map(|n| n.at).collect(),
        clips,
        segments,
        wires,
        notes,
    }
}

/// The diameters a segment's bundle is made of: a wire's own, or a group's packed.
pub fn item_ods(net: &Network, wires: &[usize]) -> Vec<f64> {
    let mut out = vec![];
    let mut done = vec![];
    for &w in wires {
        match net.groups.iter().position(|g| g.wires.contains(&w)) {
            Some(g) if !done.contains(&g) => {
                done.push(g);
                let r: Vec<f64> = net.groups[g]
                    .wires
                    .iter()
                    .filter(|x| wires.contains(x))
                    .map(|&x| net.types[net.wires[x].ty].od / 2.0)
                    .collect();
                out.push(wires::pack(&r).1);
            }
            Some(_) => {}
            None => out.push(net.types[net.wires[w].ty].od),
        }
    }
    out
}
