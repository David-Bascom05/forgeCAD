//! Compare placement alternatives against the same board definition. Only the placement pose
//! is substituted. Each alternative uses the regular compiler, fill engine and DRC.
use super::*;
use crate::electronics::{Fault, Result};
use crate::schematic::{Circuit, Document, Electrical, RoleKind};
use crate::syntax::{self, ExprKind, StmtKind};
use forge_agent::json::Json;
use std::collections::BTreeSet;
use std::path::Path;

fn error(message: impl Into<String>) -> Fault {
    Fault::new("FloorplanCandidate", Span::default(), message)
}
fn mm(n: i64) -> f64 {
    n as f64 / 1e6
}
fn strings(items: Vec<String>) -> Json {
    Json::Arr(items.into_iter().map(Into::into).collect())
}

/// Fraction of the straight guide inside a filled triangle, in guide parameter space.
fn clip(a: Point, b: Point, tri: &[Point]) -> Option<(f64, f64)> {
    let cross = |u: Point, v: Point, p: Point| {
        (v[0] - u[0]) as i128 * (p[1] - u[1]) as i128 - (v[1] - u[1]) as i128 * (p[0] - u[0]) as i128
    };
    let orientation = cross(tri[0], tri[1], tri[2]).signum();
    if orientation == 0 {
        return None;
    }
    let (mut low, mut high) = (0.0_f64, 1.0_f64);
    for [u, v] in geometry::sides(tri) {
        let x = orientation * cross(u, v, a);
        let y = orientation * cross(u, v, b);
        if x < 0 && y < 0 {
            return None;
        }
        if (x < 0) != (y < 0) {
            let t = x as f64 / (x - y) as f64;
            if x < 0 {
                low = low.max(t);
            } else {
                high = high.min(t);
            }
        }
    }
    (low < high).then_some((low, high))
}

/// An adjacent plane with a single declared zone net supplies a candidate reference.
/// Where ambiguous, leave coverage unknown. Merge intervals across triangles and islands.
fn reference_gap(board: &Board, a: &Pad, b: &Pad) -> Option<(String, f64)> {
    if a.side != b.side {
        return None;
    }
    let mut layers = board
        .layers
        .iter()
        .filter(|l| matches!(l.material, LayerMaterial::Copper { .. }));
    let plane = match a.side {
        Side::Top => layers.nth(1)?,
        Side::Bottom => layers.rev().nth(1)?,
    };
    if plane.laminate.usage.as_deref() != Some("plane") {
        return None;
    }
    let nets: BTreeSet<_> = board
        .layout
        .zones
        .values()
        .filter(|z| z.layer == plane.id)
        .map(|z| &z.net)
        .collect();
    if nets.len() != 1 {
        return None;
    }
    let reference = *nets.first()?;
    let mut intervals: Vec<_> = board
        .layout
        .fills
        .values()
        .filter(|i| i.layer == plane.id && &i.net == reference)
        .flat_map(|i| i.triangles.iter().filter_map(|tri| clip(a.at, b.at, tri)))
        .collect();
    intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (mut end, mut covered) = (0.0_f64, 0.0_f64);
    for (lo, hi) in intervals {
        covered += (hi - lo.max(end)).max(0.);
        end = end.max(hi);
    }
    let length = forge_math::Vec2::new(mm(a.at[0] - b.at[0]), mm(a.at[1] - b.at[1])).length();
    Some((
        format!("{}/{}", plane.id, reference),
        length * (1. - covered).clamp(0., 1.),
    ))
}

#[derive(Default)]
pub(super) struct Score {
    crossings: usize,
    pub(super) local_pairs: Vec<(String, String)>,
    pub(super) guides: Vec<(String, String, f64)>,
    terms: BTreeMap<String, (f64, Json)>,
    blocks: BTreeMap<(String, String), f64>,
    missing: Vec<String>,
    blocked: Vec<String>,
    opens: BTreeMap<String, usize>,
}
impl Score {
    fn between(&mut self, a: &str, b: &str, cost: f64) {
        if a != b && !a.is_empty() && !b.is_empty() {
            *self
                .blocks
                .entry((a.min(b).to_owned(), a.max(b).to_owned()))
                .or_default() += cost;
        }
    }
    fn total(&self) -> f64 {
        self.terms.values().map(|(v, _)| v).sum()
    }
}
struct Evidence<'a> {
    board: &'a Board,
    pads: BTreeMap<String, Vec<String>>,
    score: Score,
}
impl Evidence<'_> {
    fn branch(
        &mut self,
        id: String,
        (source, target): (&str, &str),
        weight: f64,
        prefer: Option<f64>,
        reason: &str,
        local: bool,
    ) {
        let nearest = self
            .pads
            .get(source)
            .into_iter()
            .flatten()
            .flat_map(|a| self.pads.get(target).into_iter().flatten().map(move |b| (a, b)))
            .map(|(a, b)| {
                let p = self.board.layout.pads[a].at;
                let q = self.board.layout.pads[b].at;
                ((p[0] - q[0]).abs() + (p[1] - q[1]).abs(), a, b)
            })
            .min();
        let Some((length, a, b)) = nearest else {
            self.score
                .missing
                .push(format!("{id}: missing placed endpoint ({source}, {target})"));
            return;
        };
        let length = mm(length);
        let reference = reference_gap(self.board, &self.board.layout.pads[a], &self.board.layout.pads[b]);
        let gap = reference.as_ref().map(|(_, g)| *g);
        let over = prefer.map_or(0., |limit| (length - limit).max(0.));
        let cost = weight * (length + over + gap.unwrap_or(0.));
        let scope = |s: &str| {
            s.rsplit_once('/')
                .map_or(String::new(), |(group, _)| group.to_owned())
        };
        let mut crossings = Vec::new();
        if local {
            let pa = &self.board.layout.pads[a];
            let pb = &self.board.layout.pads[b];
            self.score
                .local_pairs
                .push((pa.placement.clone(), pb.placement.clone()));
            if pa.side == pb.side {
                for (id, p) in &self.board.layout.placements {
                    if *id == pa.placement || *id == pb.placement || p.pose.side != pa.side {
                        continue;
                    }
                    if let Some(Shape::Region(points)) = &p.courtyard
                        && (geometry::inside(points, pa.at)
                            || geometry::sides(points).any(|edge| geometry::intersects(edge, [pa.at, pb.at])))
                    {
                        crossings.push(id.clone());
                    }
                }
            }
        }
        // A crossed package is evidence against direct local access. The guide
        // is an estimate: the routed connection might use another layer.
        self.score.crossings += crossings.len();
        let cost = cost + weight * 5. * crossings.len() as f64;
        self.score.guides.push((a.clone(), b.clone(), weight));
        self.score.between(&scope(source), &scope(target), cost);
        self.score.terms.insert(
            id,
            (
                cost,
                Json::obj(vec![
                    ("basis", reason.into()),
                    ("source", source.into()),
                    ("receiver", target.into()),
                    ("local_circuit", local.into()),
                    ("crossed_courtyards", strings(crossings)),
                    ("source_pad", a.clone().into()),
                    ("receiver_pad", b.clone().into()),
                    ("length_mm", length.into()),
                    ("weight", weight.into()),
                    ("prefer_mm", prefer.map_or(Json::Null, Into::into)),
                    ("reference_plane", reference.map_or(Json::Null, |(s, _)| s.into())),
                    ("uncovered_guide_mm", gap.map_or(Json::Null, Into::into)),
                ]),
            ),
        );
    }
}

pub(super) fn measure(b: &Board, doc: &Document, c: &Circuit) -> Score {
    let mut e = Evidence {
        board: b,
        pads: BTreeMap::new(),
        score: Score::default(),
    };
    let occurrences: BTreeMap<_, _> = b
        .layout
        .placements
        .iter()
        .filter_map(|(id, p)| Some((p.occurrence.as_deref()?, id)))
        .collect();
    for (term, t) in c.terminals.iter().filter(|(_, t)| t.populated) {
        let Some((owner, _)) = term.rsplit_once('.') else {
            continue;
        };
        if let Some(id) = occurrences.get(owner) {
            e.pads.insert(
                term.clone(),
                doc.symbols[&t.symbol].pins[&t.pin]
                    .pads
                    .iter()
                    .map(|n| format!("{id}.{n}"))
                    .collect(),
            );
        }
    }
    let unplaced: BTreeSet<_> = c
        .terminals
        .iter()
        .filter(|(_, t)| t.populated && doc.symbols[&t.symbol].power_net.is_none())
        .filter_map(|(id, _)| id.rsplit_once('.').map(|(owner, _)| owner))
        .filter(|owner| !occurrences.contains_key(owner))
        .collect();
    e.score
        .missing
        .extend(unplaced.into_iter().map(|owner| format!("no placement: {owner}")));
    for l in &c.links {
        if !c.terminals[&l.want].populated {
            continue;
        }
        if !c.terminals[&l.give].populated
            || c.net_of[&l.want] != c.net_of[&l.give]
            || l.returns.as_ref().is_some_and(|(a, b)| {
                a == b || c.net_of[a] != c.net_of[b] || c.net_of[a] == c.net_of[&l.want]
            })
        {
            e.score.missing.push(format!("unverified fulfillment: {}", l.id));
            continue;
        }
        let role = &doc.roles[&l.role];
        let preference = l.prefer.or(role.prefer);
        // A common supply or ground establishes connectivity alone.
        if preference.is_none()
            && l.returns.is_none()
            && matches!(role.kind, RoleKind::Power | RoleKind::Return)
        {
            continue;
        }
        e.branch(
            format!("link/{}", l.id),
            (&l.give, &l.want),
            4.,
            preference,
            &l.role.0,
            l.returns.is_some() || role.kind == RoleKind::Clock,
        );
        if let Some((from, to)) = &l.returns {
            e.branch(
                format!("return/{}", l.id),
                (from, to),
                4.,
                None,
                "declared return",
                true,
            );
        }
    }
    for (pin, checks) in &c.connections {
        for (role, check) in checks {
            let id = format!("connection/{pin}/{}", role.0);
            if !check.satisfied {
                e.score.missing.push(format!("{id}: {}", check.detail));
                continue;
            }
            let prefer = doc.roles[role].prefer;
            match check.bridge_pins.as_slice() {
                [near, far] => {
                    e.branch(format!("{id}/in"), (pin, near), 3., prefer, &check.detail, false);
                    e.branch(
                        format!("{id}/out"),
                        (far, &check.target),
                        3.,
                        prefer,
                        &check.detail,
                        false,
                    );
                    // An output through a verified component to a passive tap can feed
                    // inputs elsewhere on the sheet hierarchy. This proves topology only.
                    if doc.roles[role].kind == RoleKind::Signal
                        && c.terminals[pin].electrical == Electrical::Output
                        && c.terminals[&check.target].electrical == Electrical::Passive
                    {
                        for load in &c.nets[&c.net_of[&check.target]] {
                            if c.terminals
                                .get(load)
                                .is_some_and(|t| t.populated && t.electrical == Electrical::Input)
                            {
                                e.branch(
                                    format!("{id}/load/{load}"),
                                    (&check.target, load),
                                    2.,
                                    None,
                                    "input on the verified output network",
                                    false,
                                );
                            }
                        }
                    }
                }
                [] if pin != &check.target => {
                    e.branch(id, (pin, &check.target), 3., prefer, &check.detail, false)
                }
                _ => {}
            }
        }
    }
    let mut clock_receivers = BTreeMap::<&str, BTreeSet<&str>>::new();
    for (from, to, status, roles) in c.placement_signals() {
        if status == "compatible" && roles.iter().any(|r| doc.roles[r].kind == RoleKind::Clock) {
            clock_receivers.entry(from).or_default().insert(to);
        }
    }
    for (from, to, status, roles) in c.placement_signals() {
        let id = format!("signal/{from}/{to}");
        if status != "compatible" {
            e.score.missing.push(format!("{id}: {status}"));
            continue;
        }
        let clock = roles.iter().any(|r| doc.roles[r].kind == RoleKind::Clock);
        let prefer = roles
            .iter()
            .filter_map(|r| doc.roles[r].prefer)
            .min_by(f64::total_cmp);
        let function = |pin: &str| c.terminals[pin].function.as_deref().unwrap_or("base");
        let reason = format!(
            "{} → {} ({})",
            function(from),
            function(to),
            roles.iter().map(|r| r.0.as_str()).collect::<Vec<_>>().join(", ")
        );
        e.branch(
            id,
            (from, to),
            if clock { 4. } else { 2. },
            prefer,
            &reason,
            clock && clock_receivers.get(from).is_some_and(|s| s.len() == 1),
        );
    }
    let fitted_owners: BTreeSet<_> = c
        .terminals
        .iter()
        .filter(|(_, t)| t.populated)
        .filter_map(|(id, _)| id.rsplit_once('.').map(|(p, _)| p))
        .collect();
    let mut by_net: BTreeMap<&str, Vec<(&String, &Pad)>> = BTreeMap::new();
    for (key, p) in &b.layout.pads {
        if b.layout.placements[&p.placement]
            .occurrence
            .as_deref()
            .is_some_and(|occ| !fitted_owners.contains(occ))
        {
            continue;
        }
        by_net.entry(&p.net).or_default().push((key, p));
    }
    let pairs: BTreeSet<_> = c
        .diff_pairs
        .values()
        .flatten()
        .filter_map(|p| c.net_of.get(p))
        .collect();
    for (net, pads) in by_net {
        if pads.len() < 2 || pads.iter().all(|(_, p)| p.placement == pads[0].1.placement) {
            continue;
        }
        // The reference plane supplies distributed returns. Other multipin nets get
        // one rectilinear spanning tree, retaining the block boundaries it crosses.
        let is_plane = b.layout.zones.values().any(|z| {
            z.net == net
                && b.layers
                    .iter()
                    .any(|l| l.id == z.layer && l.laminate.usage.as_deref() == Some("plane"))
        });
        let has_load_class = b.classes.values().any(|class| class.nets.contains(net));
        if is_plane && !has_load_class && !pairs.contains(&b.nets[net].name) {
            continue;
        }
        let nodes: Vec<_> = pads.iter().map(|(id, _)| vec![id.as_str()]).collect();
        let tree = report::pad_tree(b, &nodes, |p, q| (p[0] - q[0]).abs() + (p[1] - q[1]).abs());
        let length = mm(tree.iter().map(|(_, _, d)| d).sum());
        let scope = |pad: &str| {
            let p = &b.layout.placements[&b.layout.pads[pad].placement];
            p.occurrence
                .as_deref()
                .and_then(|o| o.rsplit_once('/').map(|(s, _)| s))
                .unwrap_or("")
        };
        let class = b
            .classes
            .iter()
            .filter(|(_, c)| c.nets.contains(net))
            .max_by_key(|(_, c)| c.min_width_nm);
        let width_weight = class.zip(b.process.as_ref()).map_or(1., |((_, c), p)| {
            (c.min_width_nm as f64 / p.min_width_nm.max(1) as f64).clamp(1., 8.)
        });
        let paired = pairs.contains(&b.nets[net].name);
        let weight = width_weight.max(if paired { 3. } else { 1. });
        let mut uncovered = 0.;
        let segments = tree
            .into_iter()
            .map(|(a, z, d)| {
                let reference = if paired {
                    reference_gap(b, &b.layout.pads[a], &b.layout.pads[z])
                } else {
                    None
                };
                let gap = reference.as_ref().map(|(_, g)| *g);
                uncovered += gap.unwrap_or(0.);
                e.score.guides.push((a.to_owned(), z.to_owned(), weight));
                e.score
                    .between(scope(a), scope(z), weight * (mm(d) + gap.unwrap_or(0.)));
                Json::obj(vec![
                    ("uncovered_guide_mm", gap.map_or(Json::Null, Into::into)),
                    ("from", a.into()),
                    ("to", z.into()),
                    ("from_block", scope(a).into()),
                    ("to_block", scope(z).into()),
                    ("length_mm", mm(d).into()),
                ])
            })
            .collect();
        e.score.terms.insert(
            format!("net/{net}"),
            (
                weight * (length + uncovered),
                Json::obj(vec![
                    ("basis", "rectilinear spanning tree".into()),
                    ("length_mm", length.into()),
                    ("weight", weight.into()),
                    ("diff_pair", paired.into()),
                    ("class", class.map_or(Json::Null, |(name, _)| name.clone().into())),
                    ("segments", Json::Arr(segments)),
                ]),
            ),
        );
    }
    for (id, p) in &b.layout.placements {
        if let Some(shape) = &p.courtyard {
            if !shape.within(&b.outline) {
                e.score.blocked.push(format!("CourtyardOutsideBoard {id}"));
            }
        } else {
            e.score.missing.push(format!("no courtyard: {id}"));
        }
    }
    for f in b.check().findings {
        if let Witness::Disconnected { net, .. } = &f.witness {
            *e.score.opens.entry((*net).to_string()).or_default() += 1;
        }
        if f.rule != Rule::UnroutedNet && b.severity(f.rule) == "error" {
            let other = match &f.witness {
                Witness::Pair { other, .. } | Witness::Spacing { other, .. } => *other,
                _ => "",
            };
            e.score.blocked.push(
                format!("{} {} {other}", f.rule.name(), f.object)
                    .trim_end()
                    .into(),
            );
        }
    }
    e.score
}

fn place_spans(src: &str) -> Result<BTreeMap<String, (Span, Option<Span>)>> {
    let ast = syntax::parse(src).map_err(|e| error(e.text()))?;
    let mut spans = BTreeMap::new();
    for stmt in ast {
        if let StmtKind::Assign { name, value } = stmt.kind
            && let ExprKind::Call(op, args) = value.kind
            && matches!(op.kind,ExprKind::Name(ref s) if s=="place")
            && let Some(at) = args
                .iter()
                .find(|a| a.name.as_ref().is_some_and(|(s, _)| s == "at"))
        {
            let angle = args
                .iter()
                .find(|a| a.name.as_ref().is_some_and(|(s, _)| s == "angle"));
            spans.insert(name, (at.value.span, angle.map(|a| a.value.span)));
        }
    }
    Ok(spans)
}

fn candidate(
    src: &str,
    b: &Board,
    spans: &BTreeMap<String, (Span, Option<Span>)>,
    moves: &[&str],
) -> Result<(String, Json, Json)> {
    let mut changes = BTreeMap::new();
    let mut positions = BTreeMap::new();
    let mut angles = BTreeMap::new();
    for spec in moves {
        let (group, d) = spec
            .split_once('=')
            .ok_or_else(|| error("expected group=dx,dy in mm"))?;
        let (x, y) = d
            .split_once(',')
            .ok_or_else(|| error("expected two millimetre offsets"))?;
        let (y, turn) = match y.split_once(',') {
            None => (y, 0i32),
            Some((y, s)) => (
                y,
                s.parse::<i32>()
                    .ok()
                    .filter(|d| d % 90 == 0)
                    .ok_or_else(|| error("angle offset must be a multiple of 90 degrees"))?,
            ),
        };
        let offset = |s: &str| {
            s.parse::<f64>()
                .ok()
                .filter(|x| x.is_finite() && x.abs() <= 1000.)
                .map(|n| (n * 1e6).round() as i64)
                .ok_or_else(|| error("invalid placement offset"))
        };
        let delta = [offset(x)?, offset(y)?];
        let prefix = format!("{group}/");
        let mut matched = false;
        for (id, p) in &b.layout.placements {
            if id != group
                && !p
                    .occurrence
                    .as_deref()
                    .is_some_and(|o| o == group || o.starts_with(&prefix))
            {
                continue;
            }
            matched = true;
            let (span, angle_span) = spans
                .get(id)
                .ok_or_else(|| error(format!("cannot locate {id}.at")))?;
            let at = [mm(p.pose.at[0] + delta[0]), mm(p.pose.at[1] + delta[1])];
            let mut coord = format!("({:.6}mm,{:.6}mm)", at[0], at[1]);
            if turn % 360 != 0 {
                let angle = (p.pose.angle + f64::from(turn)).rem_euclid(360.);
                if let Some(a) = angle_span {
                    changes.insert((a.lo, a.hi), format!("{angle}deg"));
                } else {
                    coord += &format!(", angle:{angle}deg");
                }
                angles.insert(id.clone(), Json::from(angle));
            }
            if changes.insert((span.lo, span.hi), coord).is_some() {
                return Err(error(format!("overlapping move groups at {id}")));
            }
            positions.insert(id.clone(), Json::Arr(at.into_iter().map(Into::into).collect()));
        }
        if !matched {
            return Err(error(format!("no placements match {group}")));
        }
    }
    let mut text = src.to_owned();
    for ((lo, hi), value) in changes.into_iter().rev() {
        text.replace_range(lo as usize..hi as usize, &value);
    }
    Ok((
        text,
        Json::Obj(positions.into_iter().collect()),
        Json::Obj(angles.into_iter().collect()),
    ))
}

pub(super) fn compare(
    b: &Board,
    src: &str,
    dir: Option<&Path>,
    root: Option<&Path>,
    args: &[String],
) -> Result<Json> {
    compare_reserved(b, src, dir, root, args, &[])
}

pub(super) fn compare_reserved(
    b: &Board,
    src: &str,
    dir: Option<&Path>,
    root: Option<&Path>,
    args: &[String],
    base_intrusions: &[String],
) -> Result<Json> {
    let source = b
        .schematic
        .as_ref()
        .ok_or_else(|| error("floorplan needs a linked schematic"))?;
    let mut args = args;
    let variant = args.first().and_then(|s| s.strip_prefix("variant="));
    let fitted;
    let circuit = if let Some(name) = variant {
        args = &args[1..];
        let changes = source
            .document
            .variants
            .get(name)
            .ok_or_else(|| error(format!("unknown variant {name}")))?;
        fitted = Circuit::build_fitted(&source.document, changes)?;
        &fitted
    } else {
        &source.circuit
    };
    let spans = place_spans(src)?;
    let mut base = measure(b, &source.document, circuit);
    base.blocked.extend_from_slice(base_intrusions);
    let mut names = BTreeSet::from(["base".to_owned()]);
    let mut rows = Vec::new();
    let mut winner = None;
    let mut best = (usize::MAX, f64::INFINITY);
    let mut record = |name: &str, s: &Score, positions: Json, angles: Json| {
        let total = s.total();
        if !s.terms.is_empty() && s.blocked.is_empty() && (s.crossings, total) < best {
            winner = Some(name.to_owned());
            best = (s.crossings, total);
        }
        let terms = s
            .terms
            .iter()
            .map(|(id, (cost, evidence))| {
                Json::obj(vec![
                    ("id", id.clone().into()),
                    ("cost", (*cost).into()),
                    ("delta", (*cost - base.terms[id].0).into()),
                    ("evidence", evidence.clone()),
                ])
            })
            .collect();
        rows.push(Json::obj(vec![
            ("name", name.into()),
            ("local_guide_crossings", (s.crossings as f64).into()),
            (
                "score",
                if s.terms.is_empty() {
                    Json::Null
                } else {
                    total.into()
                },
            ),
            ("moves_mm", positions),
            ("angles_deg", angles),
            ("terms", Json::Arr(terms)),
            (
                "between_blocks",
                Json::Arr(
                    s.blocks
                        .iter()
                        .map(|((a, b), cost)| {
                            Json::obj(vec![
                                ("a", a.clone().into()),
                                ("b", b.clone().into()),
                                ("cost", (*cost).into()),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("blockers", strings(s.blocked.clone())),
            ("missing", strings(s.missing.clone())),
        ]));
    };
    record("base", &base, Json::Obj(Vec::new()), Json::Obj(Vec::new()));
    for spec in args {
        let mut fields = spec.split('@');
        let name = fields.next().unwrap_or_default();
        let moves: Vec<_> = fields.collect();
        if name.is_empty() || moves.is_empty() || !names.insert(name.into()) {
            return Err(error("expected a unique name followed by @group=dx,dy"));
        }
        let (text, positions, angles) = candidate(src, b, &spans, &moves)?;
        match compile(&text, dir, root) {
            Ok(candidate) => {
                let mut score = measure(&candidate, &source.document, circuit);
                for (net, opens) in &score.opens {
                    if b.tracks.values().any(|t| &t.net == net)
                        && *opens > base.opens.get(net).copied().unwrap_or(0)
                    {
                        score.blocked.push(format!("ExistingRouteDisconnected {net}"));
                    }
                }
                record(name, &score, positions, angles);
            }
            Err(fault) => {
                let mut score = Score::default();
                score.blocked.push(format!("{}: {}", fault.kind, fault.message));
                record(name, &score, positions, angles);
            }
        }
    }
    Ok(Json::obj(vec![
        ("schema", SCHEMA.into()),
        ("ok", true.into()),
        (
            "result",
            Json::obj(vec![
                ("model", "placement-estimate/1".into()),
                ("variant", variant.map_or(Json::Null, Into::into)),
                ("winner", winner.map_or(Json::Null, Into::into)),
                ("candidates", Json::Arr(rows)),
            ]),
        ),
    ]))
}
