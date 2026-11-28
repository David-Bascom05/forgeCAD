//! Deterministic pre-route floorplanning on a rectangular, single-side placement area.
//! Positions and envelopes are millimetres. Every result still needs the board's native DRC.
use super::*;
mod circuits;

pub type XY = [f64; 2];

#[derive(Clone, Copy, Debug)]
pub struct Seed {
    pub at: XY,
    /// Additional quarter turns about the original footprint origin.
    pub turns: u8,
    pub fixed: bool,
}
pub fn rotate([x, y]: XY, turns: u8) -> XY {
    match turns % 4 {
        0 => [x, y],
        1 => [-y, x],
        2 => [-x, -y],
        _ => [y, -x],
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub lo: XY,
    pub hi: XY,
}
impl Rect {
    fn shift(self, at: XY) -> Self {
        Self {
            lo: [self.lo[0] + at[0], self.lo[1] + at[1]],
            hi: [self.hi[0] + at[0], self.hi[1] + at[1]],
        }
    }
    fn overlap(self, other: Self, gap: f64) -> bool {
        self.hi[0] + gap > other.lo[0]
            && other.hi[0] + gap > self.lo[0]
            && self.hi[1] + gap > other.lo[1]
            && other.hi[1] + gap > self.lo[1]
    }
    fn area(self) -> f64 {
        (self.hi[0] - self.lo[0]) * (self.hi[1] - self.lo[1])
    }
    fn turn(self, n: u8) -> Self {
        let a = rotate(self.lo, n);
        let b = rotate(self.hi, n);
        Self {
            lo: [a[0].min(b[0]), a[1].min(b[1])],
            hi: [a[0].max(b[0]), a[1].max(b[1])],
        }
    }
}
/// Fixed bins keep a proposed pad neighbourhood independent of the number of
/// packages elsewhere on the board. Use exact inflated rectangle overlap after lookup.
struct Occupancy {
    origin: XY,
    dims: [usize; 2],
    bins: Vec<Vec<(usize, Rect)>>,
}
impl Occupancy {
    const WIDTH: f64 = 8.;
    fn new(room: Rect) -> Self {
        let dims: [usize; 2] =
            std::array::from_fn(|k| ((room.hi[k] - room.lo[k]) / Self::WIDTH).ceil().max(1.) as usize);
        Self {
            origin: room.lo,
            dims,
            bins: vec![Vec::new(); dims[0] * dims[1]],
        }
    }
    fn cells(&self, r: Rect, gap: f64) -> impl Iterator<Item = usize> + use<> {
        let bin =
            |x: f64, k: usize| (((x - self.origin[k]) / Self::WIDTH).floor() as usize).min(self.dims[k] - 1);
        let left = bin(r.lo[0] - gap, 0);
        let right = bin(r.hi[0] + gap, 0);
        let width = self.dims[0];
        (bin(r.lo[1] - gap, 1)..=bin(r.hi[1] + gap, 1))
            .flat_map(move |y| (left..=right).map(move |x| y * width + x))
    }
    fn clear(&self, id: usize, r: Rect, gap: f64) -> bool {
        self.cells(r, gap).all(|cell| {
            self.bins[cell]
                .iter()
                .all(|(other, v)| *other == id || !v.overlap(r, gap))
        })
    }
    fn insert(&mut self, id: usize, r: Rect) {
        for cell in self.cells(r, 0.) {
            self.bins[cell].push((id, r))
        }
    }
    fn remove(&mut self, id: usize, r: Rect) {
        for cell in self.cells(r, 0.) {
            self.bins[cell].retain(|(other, _)| *other != id)
        }
    }
}
struct Contact {
    peer: usize,
    local: XY,
    remote: XY,
    weight: f64,
}
pub struct Model {
    pub ids: Vec<String>,
    boxes: Vec<Rect>,
    local_pairs: Vec<(String, String)>,
    guides: Vec<Vec<Contact>>,
}
impl Model {
    /// The same scored branches and net-tree edges used by the placement comparator.
    pub fn new(b: &Board) -> Result<Self, String> {
        let src = b.schematic.as_ref().ok_or("packing needs a linked schematic")?;
        if !b.tracks.is_empty() || !b.vias.is_empty() {
            return Err("packing requires a pre-route board".into());
        }
        let ids: Vec<_> = b.layout.placements.keys().cloned().collect();
        let mut boxes = Vec::new();
        for id in &ids {
            let p = &b.layout.placements[id];
            if p.pose.side != Side::Top {
                return Err(format!("single-side packing cannot place {id}"));
            }
            let bounds = p
                .courtyard
                .as_ref()
                .ok_or_else(|| format!("missing courtyard for {id}"))?
                .bounds2();
            let local = |pt: Point| std::array::from_fn(|k| (pt[k] as f64 / 2. - p.pose.at[k] as f64) / 1e6);
            boxes.push(Rect {
                lo: local(bounds[0]),
                hi: local(bounds[1]),
            });
        }
        let mut model = Self {
            local_pairs: Vec::new(),
            guides: (0..ids.len()).map(|_| Vec::new()).collect(),
            ids,
            boxes,
        };
        let measured = super::floorplan::measure(b, &src.document, &src.circuit);
        model.local_pairs = measured.local_pairs;
        for (a, z, w) in measured.guides {
            model.add_guide(b, &a, &z, w)?;
        }
        Ok(model)
    }
    /// A design can supply additional measured pad-pair objectives, e.g. the individual
    /// branches of a verified converter switching loop.
    pub fn add_guide(&mut self, b: &Board, from: &str, to: &str, weight: f64) -> Result<(), String> {
        if !weight.is_finite() || weight <= 0. {
            return Err("guide weight must be positive".into());
        }
        let endpoint = |name: &str| -> Result<(usize, XY), String> {
            let p = b
                .layout
                .pads
                .get(name)
                .ok_or_else(|| format!("unknown pad {name}"))?;
            let i = self
                .ids
                .binary_search(&p.placement)
                .map_err(|_| "pad belongs to another board")?;
            let origin = b.layout.placements[&p.placement].pose.at;
            Ok((i, std::array::from_fn(|k| (p.at[k] - origin[k]) as f64 / 1e6)))
        };
        let (a, local) = endpoint(from)?;
        let (z, remote) = endpoint(to)?;
        if a != z {
            self.guides[a].push(Contact {
                peer: z,
                local,
                remote,
                weight,
            });
            self.guides[z].push(Contact {
                peer: a,
                local: remote,
                remote: local,
                weight,
            });
        }
        Ok(())
    }
    fn validate_domain(&self, seeds: &[Seed], room: Rect, keepouts: &[Rect], gap: f64) -> Result<(), String> {
        if seeds.len() != self.ids.len()
            || !gap.is_finite()
            || gap < 0.
            || (0..2).any(|k| {
                !room.lo[k].is_finite()
                    || !room.hi[k].is_finite()
                    || room.hi[k] <= room.lo[k]
                    || room.hi[k] - room.lo[k] > 1000.
            })
            || keepouts
                .iter()
                .any(|r| (0..2).any(|k| !r.lo[k].is_finite() || !r.hi[k].is_finite() || r.lo[k] > r.hi[k]))
            || seeds.iter().any(|s| s.at.iter().any(|x| !x.is_finite()))
        {
            return Err("invalid packing domain or seeds".into());
        }
        Ok(())
    }
    /// Fit all packages, preserving fixed poses, keepouts and inter-courtyard room.
    /// 'room' is the usable area after imposing the edge margin.
    pub fn pack(&self, seeds: &[Seed], room: Rect, keepouts: &[Rect], gap: f64) -> Result<Vec<XY>, String> {
        self.validate_domain(seeds, room, keepouts, gap)?;
        let boxes: Vec<_> = self
            .boxes
            .iter()
            .zip(seeds)
            .map(|(b, s)| b.turn(s.turns))
            .collect();
        let mut order: Vec<_> = (0..seeds.len()).collect();
        order.sort_by(|&a, &b| {
            seeds[b]
                .fixed
                .cmp(&seeds[a].fixed)
                .then_with(|| boxes[b].area().total_cmp(&boxes[a].area()))
                .then(a.cmp(&b))
        });
        let mut placed = vec![None; seeds.len()];
        let mut occupied = Occupancy::new(room);
        let legal = |i: usize, at: XY, occupied: &Occupancy| {
            let r = boxes[i].shift(at);
            (0..2).all(|k| r.lo[k] >= room.lo[k] && r.hi[k] <= room.hi[k])
                && !keepouts.iter().any(|v| r.overlap(*v, gap))
                && occupied.clear(i, r, gap)
        };
        let loss = |i: usize, at: XY, placed: &[Option<XY>]| {
            // Keep a soft attraction to the proposed functional group, while actual
            // pad connectivity determines which side of that group to occupy.
            let s = seeds[i];
            let mut cost = 5. * ((at[0] - s.at[0]).abs() + (at[1] - s.at[1]).abs());
            for edge in &self.guides[i] {
                let mate = seeds[edge.peer];
                let loc = placed[edge.peer].unwrap_or(mate.at);
                let a = rotate(edge.local, s.turns);
                let b = rotate(edge.remote, mate.turns);
                cost += edge.weight
                    * ((at[0] + a[0] - loc[0] - b[0]).abs() + (at[1] + a[1] - loc[1] - b[1]).abs());
            }
            cost
        };
        for &i in &order {
            let s = seeds[i];
            let mut best = None;
            if s.fixed {
                if legal(i, s.at, &occupied) {
                    best = Some((0., s.at))
                }
            } else {
                let center = s.at.map(|x| (x * 2.).round() / 2.);
                let limit = ((room.hi[0] - room.lo[0]).max(room.hi[1] - room.lo[1]) * 2.).ceil() as i32;
                let mut first = None;
                for r in 0..=limit {
                    // Traverse each square ring once.
                    for x in -r..=r {
                        let mut consider = |y: i32| {
                            let at = [center[0] + 0.5 * f64::from(x), center[1] + 0.5 * f64::from(y)];
                            if legal(i, at, &occupied) {
                                first.get_or_insert(r);
                                let value = loss(i, at, &placed);
                                if best.is_none_or(|(v, _)| value < v) {
                                    best = Some((value, at))
                                }
                            }
                        };
                        if x.abs() == r {
                            for y in -r..=r {
                                consider(y)
                            }
                        } else {
                            consider(-r);
                            consider(r)
                        }
                    }
                    if first.is_some_and(|f| r >= f + 4) {
                        break;
                    }
                }
            }
            let (_, at) = best.ok_or_else(|| format!("no legal pose for {}", self.ids[i]))?;
            placed[i] = Some(at);
            occupied.insert(i, boxes[i].shift(at));
        }
        for step in [3., 1.5, 0.5] {
            for &i in order.iter().rev() {
                if seeds[i].fixed {
                    continue;
                }
                let now = placed[i].unwrap();
                let mut best = (loss(i, now, &placed), now);
                for dx in -2..=2 {
                    for dy in -2..=2 {
                        let at = [now[0] + step * f64::from(dx), now[1] + step * f64::from(dy)];
                        if legal(i, at, &occupied) {
                            let v = loss(i, at, &placed);
                            if v < best.0 {
                                best = (v, at)
                            }
                        }
                    }
                }
                placed[i] = Some(best.1);
                if best.1 != now {
                    occupied.remove(i, boxes[i].shift(now));
                    occupied.insert(i, boxes[i].shift(best.1));
                }
            }
        }
        Ok(placed.into_iter().map(Option::unwrap).collect())
    }
    /// Local refinement with explicit per-footprint rotation permission. Rotation
    /// about the courtyard centre gives an asymmetric package the same occupied
    /// rectangle after a half turn. Rank the candidate with the physical pad offsets.
    pub fn polish(
        &self,
        poses: &mut [Seed],
        room: Rect,
        keepouts: &[Rect],
        gap: f64,
        rotatable: &[bool],
    ) -> Result<(), String> {
        self.validate_domain(poses, room, keepouts, gap)?;
        if rotatable.len() != poses.len() {
            return Err("one rotation permission per footprint is required".into());
        }
        let envelope = |i: usize, p: Seed| self.boxes[i].turn(p.turns).shift(p.at);
        let mut occupied = Occupancy::new(room);
        // The board uses integer nanometres. Allow one nanometre of round-off
        // when reading an existing legal pose back into the millimetre model.
        let tolerance = 0.000001;
        for (i, &p) in poses.iter().enumerate() {
            let r = envelope(i, p);
            if (0..2).any(|k| r.lo[k] < room.lo[k] - tolerance || r.hi[k] > room.hi[k] + tolerance)
                || keepouts.iter().any(|v| r.overlap(*v, gap - tolerance))
                || !occupied.clear(i, r, gap - tolerance)
            {
                return Err(format!(
                    "refinement needs a legal initial pose for {}",
                    self.ids[i]
                ));
            }
            occupied.insert(i, r);
        }
        let anchors = poses.to_vec();
        let mut order: Vec<_> = (0..poses.len()).collect();
        order.sort_by(|&a, &b| self.boxes[b].area().total_cmp(&self.boxes[a].area()));
        for step in [2., 1., 0.5] {
            for &i in &order {
                let start = poses[i];
                if start.fixed {
                    continue;
                }
                let original = envelope(i, start);
                let anchor = anchors[i].at;
                let loss = |p: Seed| {
                    let mut cost = (anchor[0] - p.at[0]).abs() + (anchor[1] - p.at[1]).abs();
                    for e in &self.guides[i] {
                        let peer = poses[e.peer];
                        let a = rotate(e.local, p.turns);
                        let b = rotate(e.remote, peer.turns);
                        cost += e.weight
                            * ((p.at[0] + a[0] - peer.at[0] - b[0]).abs()
                                + (p.at[1] + a[1] - peer.at[1] - b[1]).abs());
                    }
                    cost
                };
                let mut best = (loss(start), start);
                for extra in 0..if rotatable[i] { 4 } else { 1 } {
                    let turns = (start.turns % 4 + extra) % 4;
                    let b = self.boxes[i].turn(turns);
                    let center: XY =
                        std::array::from_fn(|k| (original.lo[k] + original.hi[k] - b.lo[k] - b.hi[k]) / 2.);
                    for x in -4..=4 {
                        for y in -4..=4 {
                            let p = Seed {
                                at: [center[0] + step * f64::from(x), center[1] + step * f64::from(y)],
                                turns,
                                fixed: false,
                            };
                            let r = envelope(i, p);
                            if (0..2).any(|k| r.lo[k] < room.lo[k] || r.hi[k] > room.hi[k])
                                || keepouts.iter().any(|v| r.overlap(*v, gap))
                                || !occupied.clear(i, r, gap)
                            {
                                continue;
                            }
                            let value = loss(p);
                            if value < best.0 - 1e-8 {
                                best = (value, p)
                            }
                        }
                    }
                }
                occupied.remove(i, original);
                poses[i] = best.1;
                occupied.insert(i, envelope(i, best.1));
            }
        }
        Ok(())
    }
    pub fn estimate(&self, at: &[XY], seeds: &[Seed]) -> f64 {
        self.guides
            .iter()
            .enumerate()
            .flat_map(|(i, edges)| {
                edges.iter().filter(move |e| e.peer > i).map(move |e| {
                    let a = rotate(e.local, seeds[i].turns);
                    let b = rotate(e.remote, seeds[e.peer].turns);
                    e.weight
                        * ((at[i][0] + a[0] - at[e.peer][0] - b[0]).abs()
                            + (at[i][1] + a[1] - at[e.peer][1] - b[1]).abs())
                })
            })
            .sum()
    }
}

struct BlockVersion {
    name: String,
    poses: Vec<(usize, XY, u8)>,
}

/// BLOCK:VERSION@REF=X,Y,ROTATION. Coordinates are absolute board millimetres.
/// Versions of one block may specify different subsets; unspecified poses inherit
/// their source seed. Different blocks must own disjoint placements.
fn version<'a>(spec: &'a str, ids: &[String]) -> Result<(&'a str, BlockVersion), String> {
    let mut fields = spec.split('@');
    let (block, name) = fields
        .next()
        .unwrap()
        .split_once(':')
        .ok_or("expected block:version")?;
    let valid = |s: &str| {
        !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    };
    if !valid(block) || !valid(name) {
        return Err("invalid block or version name".into());
    }
    let mut poses = Vec::new();
    for field in fields {
        let (id, xyz) = field.split_once('=').ok_or("expected REF=X,Y,DEGREES")?;
        let i = ids
            .binary_search_by(|s| s.as_str().cmp(id))
            .map_err(|_| format!("unknown placement {id}"))?;
        let args: Vec<_> = xyz.split(',').collect();
        let [x, y, angle] = args.as_slice() else {
            return Err("expected X,Y,DEGREES".into());
        };
        let distance = |s: &str| {
            s.parse::<f64>()
                .ok()
                .filter(|v| v.is_finite() && v.abs() <= 500.)
                .ok_or("invalid coordinate")
        };
        let degrees = angle
            .parse::<i32>()
            .ok()
            .filter(|v| v % 90 == 0)
            .ok_or("expected quarter turns in degrees")?;
        if poses.iter().any(|(j, _, _)| *j == i) {
            return Err(format!("duplicate pose for {id}"));
        }
        poses.push((
            i,
            [distance(x)?, distance(y)?],
            (degrees.rem_euclid(360) / 90) as u8,
        ));
    }
    Ok((
        block,
        BlockVersion {
            name: name.to_owned(),
            poses,
        },
    ))
}
pub(super) fn query(
    board: &Board,
    text: &str,
    dir: Option<&std::path::Path>,
    root: Option<&std::path::Path>,
    words: &[String],
) -> crate::electronics::Result<forge_agent::json::Json> {
    use crate::electronics::Fault;
    let err = |s: String| Fault::new("Packing", crate::syntax::Span::default(), s);
    let model = Model::new(board).map_err(&err)?;
    let mut fixed = std::collections::BTreeSet::new();
    let mut gap = 0.5;
    let mut rotate_free = false;
    let mut preserve_local = true;
    let mut explicit = Vec::new();
    let mut blocks: BTreeMap<&str, Vec<BlockVersion>> = BTreeMap::new();
    let mut owners = BTreeMap::new();
    for word in words {
        if matches!(word.as_str(), "local" | "independent") {
            preserve_local = word == "local";
            continue;
        }
        if let Some(names) = word.strip_prefix("circuit=") {
            explicit.push(names.split(',').map(str::to_owned).collect::<Vec<_>>());
            continue;
        }
        if word == "rotate" {
            rotate_free = true;
            continue;
        }
        if let Some(ids) = word.strip_prefix("fixed=") {
            for id in ids.split(',') {
                if !board.layout.placements.contains_key(id) {
                    return Err(err(format!("unknown fixed placement {id}")));
                }
                fixed.insert(id);
            }
        } else if let Some(value) = word.strip_prefix("gap=") {
            gap = value.parse().map_err(|_| err("gap needs millimetres".into()))?
        } else {
            let (block, v) = version(word, &model.ids).map_err(&err)?;
            for &(i, _, _) in &v.poses {
                if owners.insert(i, block).is_some_and(|previous| previous != block) {
                    return Err(err(format!("{} belongs to multiple blocks", model.ids[i])));
                }
            }
            let versions = blocks.entry(block).or_default();
            if versions.iter().any(|other| other.name == v.name) {
                return Err(err(format!("duplicate version {word}")));
            }
            versions.push(v);
        }
    }
    let mm = |v: i64| v as f64 / 1e6;
    let bounds = |points: &[Point]| Rect {
        lo: std::array::from_fn(|k| mm(points.iter().map(|p| p[k]).min().unwrap())),
        hi: std::array::from_fn(|k| mm(points.iter().map(|p| p[k]).max().unwrap())),
    };
    let room = bounds(&board.outline);
    let mut keep: Vec<_> = board.layout.cutouts.values().map(|p| bounds(p)).collect();
    for h in board.layout.holes.values() {
        let mut r = bounds(&[h.at, h.to.unwrap_or(h.at)]);
        for k in 0..2 {
            r.lo[k] -= mm(h.drill) / 2.;
            r.hi[k] += mm(h.drill) / 2.;
        }
        keep.push(r);
    }
    let seeds: Vec<_> = model
        .ids
        .iter()
        .map(|id| Seed {
            at: board.layout.placements[id].pose.at.map(mm),
            turns: 0,
            fixed: fixed.contains(id.as_str()),
        })
        .collect();
    let count = blocks
        .values()
        .try_fold(1usize, |n, vs| n.checked_mul(vs.len()))
        .filter(|c| *c <= 256)
        .ok_or_else(|| err("split queries exceeding 256 combinations".into()))?;
    let circuits = model.circuit_groups(preserve_local, &explicit).map_err(&err)?;
    let mut specs = Vec::new();
    let mut failures = Vec::new();
    for ordinal in 0..count {
        let mut trial = seeds.clone();
        let mut label = String::new();
        let mut index = ordinal;
        for (block, versions) in &blocks {
            let v = &versions[index % versions.len()];
            index /= versions.len();
            if !label.is_empty() {
                label.push('+')
            }
            label += &format!("{block}:{}", v.name);
            for &(i, at, turns) in &v.poses {
                trial[i].at = at;
                trial[i].turns = turns;
            }
        }
        if label.is_empty() {
            label.push_str("packed");
        }
        let allowed: Vec<_> = trial
            .iter()
            .enumerate()
            .map(|(i, p)| rotate_free && !p.fixed && !owners.contains_key(&i))
            .collect();
        // Named or versioned templates already describe an intended internal arrangement.
        // Synthesize just the fully inferred templates from the pad graph.
        let synthesized = circuits
            .iter()
            .filter(|group| {
                group.iter().all(|i| {
                    !owners.contains_key(i) && !explicit.iter().flatten().any(|name| name == &model.ids[*i])
                })
            })
            .try_for_each(|group| model.compact_inferred(&mut trial, group, gap));
        match synthesized.and_then(|_| {
            model.pack_circuits(
                &mut trial,
                &circuits,
                room,
                &keep,
                gap,
                rotate_free.then_some(&allowed),
            )
        }) {
            Ok(()) => (),
            Err(e) => {
                failures.push(forge_agent::json::Json::obj(vec![
                    ("name", label.into()),
                    ("reason", e.into()),
                ]));
                continue;
            }
        }
        let positions = trial.iter().map(|s| s.at);
        let mut spec = label.clone();
        for (i, (id, at)) in model.ids.iter().zip(positions).enumerate() {
            let orig = seeds[i].at;
            if at != orig || trial[i].turns != 0 {
                spec += &format!(
                    "@{id}={:.6},{:.6},{}",
                    at[0] - orig[0],
                    at[1] - orig[1],
                    90 * u16::from(trial[i].turns)
                );
            }
        }
        if label == spec
            && let Some(id) = model.ids.first()
        {
            spec += &format!("@{id}=0,0");
        }
        if spec != label {
            specs.push(spec);
        }
    }
    let mut reply = super::floorplan::compare_reserved(
        board,
        text,
        dir,
        root,
        &specs,
        &model.intrusions(&seeds, &circuits),
    )?;
    if let forge_agent::json::Json::Obj(ref mut fields) = reply
        && let Some((_, forge_agent::json::Json::Obj(result))) =
            fields.iter_mut().find(|(k, _)| k == "result")
    {
        result.push((
            "local_circuits".into(),
            forge_agent::json::Json::Arr(
                circuits
                    .iter()
                    .map(|g| {
                        forge_agent::json::Json::Arr(g.iter().map(|i| model.ids[*i].clone().into()).collect())
                    })
                    .collect(),
            ),
        ));
        result.push(("packing_failures".into(), forge_agent::json::Json::Arr(failures)));
    }
    Ok(reply)
}
