//! Placement: closed form along tree edges, Newton only on loop blocks (prior-art §1.5 pass 3 and
//! 4), with the numeric diagnosis of redundancy and conflict at the solution (§5.3).

use crate::diag::{Diagnostic, Item};
use crate::joint::{self, Ends, Row};
use crate::model::*;
use crate::plan::{self, Plan};
use crate::qr::Qr;
use forge_math::{Frame, Vec3, dmath};
use std::collections::BTreeMap;

const MAX_ITERATIONS: usize = 60;
/// Residual (mm, angles × block size) per unit of block size at which Newton stops.
const TIGHT: f64 = 1e-13;
/// Residual per unit of block size that still counts as solved where no step improves it (the
/// rounding floor); above it, a stuck solve is a conflict.
const SOLVED: f64 = 1e-9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    /// Threads for partitions. Results do not depend on it.
    pub threads: usize,
}

impl Default for Options {
    fn default() -> Self {
        #[cfg(target_family = "wasm")]
        let threads = 1;
        #[cfg(not(target_family = "wasm"))]
        let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).min(8);
        Options { threads }
    }
}

/// A part occurrence, placed.
#[derive(Clone, Debug, PartialEq)]
pub struct Occurrence {
    pub path: Path,
    pub part: PartRef,
    pub pose: Frame,
    pub reference: bool,
}

/// One joint after the solve: its values, and how many of its freedoms the assembly can still
/// move, each with its world direction for a positive step.
#[derive(Clone, Debug, PartialEq)]
pub struct JointReport {
    pub joint: JointId,
    pub kind: JointKind,
    pub values: Vec<(Freedom, f64)>,
    /// Independent motions of the assembly that change this joint's values.
    pub free: usize,
    /// World axis of each freedom: the direction (or rotation axis) a positive value moves A.
    pub directions: Vec<(Freedom, Vec3)>,
    /// In a loop block (solved numerically) rather than a tree edge.
    pub in_loop: bool,
    /// Left out of the solve after a conflict, a broken anchor or a bad freedom.
    pub failed: bool,
}

/// A loop block (or blocks merged by a relation) as Newton saw it.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockReport {
    pub block: usize,
    pub links: Vec<Item>,
    pub instances: Vec<InstanceId>,
    /// Unknowns minus rows: the structural (Grübler) count, a hint only.
    pub structural_mobility: i64,
    /// Unknowns minus the Jacobian's rank at the solution: the numeric answer.
    pub mobility: usize,
    pub rows: usize,
    pub rank: usize,
    pub iterations: usize,
}

/// One primitive constraint (or a joint's drive or held limit) after the solve.
#[derive(Clone, Debug, PartialEq)]
pub struct ConstraintReport {
    pub id: ConstraintId,
    pub joint: Option<JointId>,
    /// None for a drive or a limit (one value).
    pub kind: Option<ConstraintKind>,
    /// Freedoms it removes when independent.
    pub removes: usize,
    /// Of those, how many the others already remove.
    pub redundant: usize,
    /// Left out after a conflict.
    pub failed: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub partitions_solved: usize,
    pub partitions_reused: usize,
    /// Loop blocks run through Newton (not reused from the last solve).
    pub blocks_solved: usize,
    pub blocks_reused: usize,
    pub bridges_placed: usize,
    pub newton_iterations: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Solution {
    /// Each top-level instance's solved pose, by id.
    pub poses: Vec<(InstanceId, Frame)>,
    pub occurrences: Vec<Occurrence>,
    pub diagnostics: Vec<Diagnostic>,
    pub joints: Vec<JointReport>,
    pub constraints: Vec<ConstraintReport>,
    pub blocks: Vec<BlockReport>,
    /// Degrees of freedom left in the whole assembly.
    pub mobility: usize,
    /// Every part used and the content hash it had: what an assembly commit records.
    pub hashes: Vec<(PartRef, Option<ContentHash>)>,
    pub stats: Stats,
}

impl Solution {
    pub fn pose(&self, id: InstanceId) -> Option<Frame> {
        self.poses
            .binary_search_by_key(&id, |p| p.0)
            .ok()
            .map(|i| self.poses[i].1)
    }
    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity() == crate::diag::Severity::Error)
    }
}

// ---- one partition ----

/// What a solved partition gives: its clusters' poses and its reports.
#[derive(Clone, Debug, Default)]
pub(crate) struct PartOut {
    pub poses: Vec<(usize, Frame)>,
    pub diagnostics: Vec<Diagnostic>,
    pub joints: Vec<JointReport>,
    pub blocks: Vec<BlockReport>,
    pub mobility: usize,
    pub stats: Stats,
}

#[derive(Clone, Debug)]
struct UnitOut {
    key: Vec<u64>,
    poses: Vec<(usize, Frame)>,
    diagnostics: Vec<Diagnostic>,
    joints: Vec<JointReport>,
    block: Option<BlockReport>,
    mobility: usize,
    iterations: usize,
}

/// The last solve's units of one partition, reused when their inputs are bit-identical.
#[derive(Clone, Debug, Default)]
pub(crate) struct PartMemo {
    units: Vec<UnitOut>,
}

struct Unit {
    /// Joint indices (plan), by id.
    joints: Vec<usize>,
    /// Vertices held while this unit is placed, and those it places.
    fixed: Vec<usize>,
    vars: Vec<usize>,
    /// Relations solved inside (both values unknown before).
    rels: Vec<usize>,
    /// A tree edge placed in closed form.
    closed: Option<Closed>,
    block: usize,
}

/// A tree edge placed in closed form: a joint, or loose constraints that together are one.
#[derive(Clone, Debug)]
struct Closed {
    kind: JointKind,
    body: [usize; 2],
    /// A's and B's frames in their clusters' frames, as the joint kind takes them.
    local: [Frame; 2],
    /// The joint's link, when it is a joint.
    joint: Option<usize>,
}

fn same_frame(p: &Frame, q: &Frame) -> bool {
    let mut a = Vec::new();
    let mut b = Vec::new();
    bits(p, &mut a);
    bits(q, &mut b);
    a == b
}

/// Loose constraints between the same two bodies that are exactly a joint's bundle (same frames,
/// offsets moved into B's frame) are placed as that joint.
fn recognise(plan: &Plan, links: &[usize]) -> Option<Closed> {
    let first = &plan.links[*links.first()?];
    let cl = |b: usize| plan.bodies[b].cluster;
    let mut prims: Vec<(ConstraintKind, Frame, Frame)> = Vec::new();
    for &k in links {
        let l = &plan.links[k];
        if l.joint.is_some() || cl(l.body[0]) != cl(first.body[0]) || cl(l.body[1]) != cl(first.body[1]) {
            return None;
        }
        for p in &l.prims {
            let up =
                |o: f64| p.local[1].compose(&Frame::WORLD.with_origin(forge_math::Point3::new(0.0, 0.0, o)));
            let (kind, b) = match p.kind {
                ConstraintKind::PointOnPlane { offset } => {
                    (ConstraintKind::PointOnPlane { offset: 0.0 }, up(offset))
                }
                ConstraintKind::PlanePlane { offset } => {
                    (ConstraintKind::PlanePlane { offset: 0.0 }, up(offset))
                }
                ConstraintKind::Insert { offset } => (ConstraintKind::Insert { offset: 0.0 }, up(offset)),
                k => (k, p.local[1]),
            };
            prims.push((kind, p.local[0], b));
        }
    }
    let (a, b) = (prims[0].1, prims[0].2);
    if !prims.iter().all(|p| same_frame(&p.1, &a) && same_frame(&p.2, &b)) {
        return None;
    }
    let mut names: Vec<&str> = prims.iter().map(|p| p.0.name()).collect();
    names.sort_unstable();
    let twist0 = prims
        .iter()
        .all(|p| !matches!(p.0, ConstraintKind::Twist { angle } if angle != 0.0));
    let kind = match names[..] {
        ["coincident"] => JointKind::Ball,
        ["coincident", "parallel"] | ["insert"] => JointKind::Revolute,
        ["concentric"] | ["parallel", "point_on_line"] => JointKind::Cylindrical,
        ["concentric", "twist"] | ["parallel", "point_on_line", "twist"] if twist0 => JointKind::Slider,
        ["plane_plane"] | ["parallel", "point_on_plane"] => JointKind::Planar,
        _ => return None,
    };
    Some(Closed {
        kind,
        body: first.body,
        local: [a, b],
        joint: None,
    })
}

/// The local graph of a partition: vertex i < clusters.len() is cluster `clusters[i]`; the last
/// vertex is ground when the partition touches it.
struct Local<'a> {
    plan: &'a Plan,
    clusters: &'a [usize],
    ground: Option<usize>,
}

impl Local<'_> {
    fn vertex(&self, cluster: usize) -> usize {
        if self.plan.clusters[cluster].ground {
            return self.ground.expect("a grounded partition");
        }
        self.clusters
            .binary_search(&cluster)
            .expect("a cluster of this partition")
    }
    fn ends(&self, k: usize) -> [usize; 2] {
        let j = &self.plan.links[k];
        [0, 1].map(|s| self.vertex(self.plan.bodies[j.body[s]].cluster))
    }
}

/// Poses of a partition's clusters while it is placed.
struct Poses<'a> {
    plan: &'a Plan,
    local: &'a Local<'a>,
    at: Vec<Frame>,
}

impl Poses<'_> {
    fn cluster(&self, c: usize) -> Frame {
        if self.plan.clusters[c].ground {
            self.plan.clusters[c].written
        } else {
            self.at[self.local.vertex(c)]
        }
    }
    fn ends(&self, k: usize) -> Ends {
        let j = &self.plan.links[k];
        self.ends_of(k, &j.local)
    }
    fn prim_ends(&self, k: usize, p: usize) -> Ends {
        self.ends_of(k, &self.plan.links[k].prims[p].local)
    }
    fn ends_of(&self, k: usize, local: &[Frame; 2]) -> Ends {
        let j = &self.plan.links[k];
        let pa = self.cluster(self.plan.bodies[j.body[0]].cluster);
        let pb = self.cluster(self.plan.bodies[j.body[1]].cluster);
        Ends {
            a: pa.compose(&local[0]),
            b: pb.compose(&local[1]),
            ta: pa.origin,
            tb: pb.origin,
        }
    }
}

/// A joint end's world frame from the written poses.
fn written_rel(plan: &Plan, k: usize) -> Frame {
    let j = &plan.links[k];
    let w = |s: usize| {
        plan.clusters[plan.bodies[j.body[s]].cluster]
            .written
            .compose(&j.local[s])
    };
    joint::relative(&w(0), &w(1))
}

fn mid(j: &plan::PLink, f: Freedom) -> f64 {
    j.limits
        .iter()
        .find(|l| l.0 == f)
        .map_or(0.0, |l| 0.5 * (l.1 + l.2))
}

/// Written value of a freedom (angles near the limits' middle).
fn written_value(plan: &Plan, k: usize, f: Freedom) -> f64 {
    let v = joint::measure(f, &written_rel(plan, k));
    if f.is_angle() {
        joint::unwrap_near(v, mid(&plan.links[k], f))
    } else {
        v
    }
}

fn relation_offset(plan: &Plan, r: &plan::PRelation) -> f64 {
    r.offset.unwrap_or_else(|| {
        written_value(plan, r.follower.0, r.follower.1)
            - r.kind.slope() * written_value(plan, r.leader.0, r.leader.1)
    })
}

fn bits(f: &Frame, key: &mut Vec<u64>) {
    for v in [f.origin.to_vec(), f.x().get(), f.z().get()] {
        key.extend([v.x.to_bits(), v.y.to_bits(), v.z.to_bits()]);
    }
}

/// Biconnected blocks of the partition's joint graph (iterative Tarjan, edges by id). Returns the
/// blocks as lists of positions in `edges`.
fn blocks(nv: usize, edges: &[[usize; 2]]) -> Vec<Vec<usize>> {
    let mut adj: Vec<Vec<(usize, usize)>> = vec![Vec::new(); nv];
    for (e, &[u, v]) in edges.iter().enumerate() {
        adj[u].push((e, v));
        adj[v].push((e, u));
    }
    let mut disc = vec![usize::MAX; nv];
    let mut low = vec![0; nv];
    let mut time = 0;
    let mut estack: Vec<usize> = Vec::new();
    let mut out = Vec::new();
    for root in 0..nv {
        if disc[root] != usize::MAX {
            continue;
        }
        disc[root] = time;
        low[root] = time;
        time += 1;
        // (vertex, edge it was reached by, next adjacency index)
        let mut st: Vec<(usize, usize, usize)> = vec![(root, usize::MAX, 0)];
        while let Some(top) = st.last_mut() {
            let (v, pe, i) = *top;
            if i < adj[v].len() {
                top.2 += 1;
                let (e, w) = adj[v][i];
                if e == pe {
                    continue;
                }
                if disc[w] == usize::MAX {
                    estack.push(e);
                    disc[w] = time;
                    low[w] = time;
                    time += 1;
                    st.push((w, e, 0));
                } else if disc[w] < disc[v] {
                    estack.push(e);
                    low[v] = low[v].min(disc[w]);
                }
            } else {
                st.pop();
                if let Some(&(p, _, _)) = st.last() {
                    low[p] = low[p].min(low[v]);
                    if low[v] >= disc[p] {
                        let mut b = Vec::new();
                        while let Some(e) = estack.pop() {
                            b.push(e);
                            if e == pe {
                                break;
                            }
                        }
                        b.sort_unstable();
                        out.push(b);
                    }
                }
            }
        }
    }
    out.sort_by_key(|b| b[0]);
    out
}

/// Values each joint freedom has before any loop is solved: drives, values that follow a known
/// leader through a relation, and tree joints' written values. None: solved in a loop.
type Known = BTreeMap<(usize, Freedom), (f64, Option<Item>)>;

fn solve_partition(plan: &Plan, p: usize, memo: Option<&PartMemo>) -> (PartOut, PartMemo) {
    let part = &plan.partitions[p];
    let local = Local {
        plan,
        clusters: &part.clusters,
        ground: part.grounded.then_some(part.clusters.len()),
    };
    let nv = part.clusters.len() + part.grounded as usize;
    let edges: Vec<[usize; 2]> = part.edges.iter().map(|&k| local.ends(k)).collect();
    let bl = blocks(nv, &edges);

    // the block-cut tree from the roots: ground first, else the least cluster
    let mut vblocks: Vec<Vec<usize>> = vec![Vec::new(); nv];
    let mut bverts: Vec<Vec<usize>> = Vec::new();
    for (b, es) in bl.iter().enumerate() {
        let mut vs: Vec<usize> = es.iter().flat_map(|&e| edges[e]).collect();
        vs.sort_unstable();
        vs.dedup();
        for &v in &vs {
            vblocks[v].push(b);
        }
        bverts.push(vs);
    }
    let mut placed = vec![false; nv];
    let mut placed_by = vec![usize::MAX; nv];
    let mut entry = vec![usize::MAX; bl.len()];
    let mut border: Vec<usize> = Vec::new();
    let mut roots = Vec::new();
    let mut starts: Vec<usize> = local.ground.into_iter().chain(0..nv).collect();
    starts.dedup();
    for s in starts {
        if placed[s] {
            continue;
        }
        roots.push(s);
        placed[s] = true;
        let mut queue = std::collections::VecDeque::from([s]);
        while let Some(v) = queue.pop_front() {
            for &b in &vblocks[v] {
                if entry[b] != usize::MAX {
                    continue;
                }
                entry[b] = v;
                border.push(b);
                for &w in &bverts[b] {
                    if !placed[w] {
                        placed[w] = true;
                        placed_by[w] = b;
                        queue.push_back(w);
                    }
                }
            }
        }
    }
    let parent_block = |b: usize| -> Option<usize> {
        let v = entry[b];
        (placed_by[v] != usize::MAX).then_some(placed_by[v])
    };
    let mut block_of = BTreeMap::new();
    for (b, es) in bl.iter().enumerate() {
        for &e in es {
            block_of.insert(part.edges[e], b);
        }
    }
    let rels: Vec<&plan::PRelation> = part.relations.iter().map(|&r| &plan.relations[r]).collect();

    // relations whose leader is solved in a loop pull their follower's block into that loop
    let mut guf = plan::Uf::new(bl.len());
    let known = loop {
        let mut gsize = vec![0usize; bl.len()];
        for b in 0..bl.len() {
            gsize[guf.find(b)] += 1;
        }
        let tree = |k: usize, guf: &mut plan::Uf| -> bool {
            block_of
                .get(&k)
                .is_some_and(|&b| bl[b].len() == 1 && gsize[guf.find(b)] == 1)
        };
        let mut known: Known = BTreeMap::new();
        let mut diags = Vec::new();
        for &k in &part.edges {
            for &(f, v) in &plan.links[k].drive {
                known.insert((k, f), (v, None));
            }
        }
        let followers: Vec<(usize, Freedom)> = rels.iter().map(|r| r.follower).collect();
        for &k in &part.edges {
            if tree(k, &mut guf) {
                for &f in plan.links[k].freedoms() {
                    if !known.contains_key(&(k, f)) && !followers.contains(&(k, f)) {
                        known.insert((k, f), (written_value(plan, k, f), None));
                    }
                }
            }
        }
        // followers of known leaders, in relation order, until nothing changes
        let mut done = vec![false; rels.len()];
        loop {
            let mut changed = false;
            for (i, r) in rels.iter().enumerate() {
                if done[i] {
                    continue;
                }
                let Some(&(lv, _)) = known.get(&r.leader) else {
                    continue;
                };
                done[i] = true;
                changed = true;
                let v = r.kind.slope() * lv + relation_offset(plan, r);
                match known.get(&r.follower) {
                    Some(&(had, by)) => {
                        let fj = &plan.links[r.follower.0];
                        let mut implied = vec![drive_item(fj, r.follower.1)];
                        if let Some(by) = by {
                            implied.push(by);
                        }
                        let scale = if r.follower.1.is_angle() { plan.size } else { 1.0 };
                        plan::closed_form(&mut diags, Item::Relation(r.id), implied, (had - v).abs() * scale);
                    }
                    None => {
                        known.insert(r.follower, (v, Some(Item::Relation(r.id))));
                    }
                }
            }
            if !changed {
                break;
            }
        }
        // merge
        let mut merged = false;
        for (i, r) in rels.iter().enumerate() {
            if done[i] {
                continue;
            }
            let (Some(&bl_l), Some(&bl_f)) = (block_of.get(&r.leader.0), block_of.get(&r.follower.0)) else {
                continue;
            };
            if guf.find(bl_l) == guf.find(bl_f) {
                continue;
            }
            let chain = |mut b: usize| {
                let mut v = vec![b];
                while let Some(pb) = parent_block(b) {
                    v.push(pb);
                    b = pb;
                }
                v
            };
            let (cl, cf) = (chain(bl_l), chain(bl_f));
            for (c, o) in [(&cl, &cf), (&cf, &cl)] {
                let mut prev = c[0];
                for &b in c.iter() {
                    guf.union(prev, b);
                    prev = b;
                    if o.contains(&b) {
                        break;
                    }
                }
            }
            guf.union(bl_l, bl_f);
            merged = true;
        }
        if !merged {
            break (known, diags);
        }
    };
    let (known, mut diagnostics) = known;

    // units in block order; a merged group runs at its first block
    let mut units: Vec<Unit> = Vec::new();
    let mut unit_done = vec![false; bl.len()];
    for &b in &border {
        let g = guf.find(b);
        if unit_done[g] {
            continue;
        }
        unit_done[g] = true;
        let members: Vec<usize> = (0..bl.len()).filter(|&x| guf.find(x) == g).collect();
        let mut joints: Vec<usize> = members
            .iter()
            .flat_map(|&x| bl[x].iter().map(|&e| part.edges[e]))
            .collect();
        joints.sort_by_key(|&k| plan.links[k].id);
        let closed = if members.len() != 1 {
            None
        } else if joints.len() == 1 && plan.links[joints[0]].joint.is_some() {
            let l = &plan.links[joints[0]];
            Some(Closed {
                kind: l.kind(),
                body: l.body,
                local: l.local,
                joint: Some(joints[0]),
            })
        } else if bverts[b].len() == 2 {
            recognise(plan, &joints)
        } else {
            None
        };
        let mut vs: Vec<usize> = members.iter().flat_map(|&x| bverts[x].iter().copied()).collect();
        vs.sort_unstable();
        vs.dedup();
        let inner: Vec<usize> = vs
            .iter()
            .copied()
            .filter(|&v| placed_by[v] != usize::MAX && members.contains(&placed_by[v]))
            .collect();
        let fixed: Vec<usize> = vs.iter().copied().filter(|v| !inner.contains(v)).collect();
        let urels: Vec<usize> = (0..rels.len())
            .filter(|&i| {
                !known.contains_key(&rels[i].follower)
                    && joints.contains(&rels[i].leader.0)
                    && joints.contains(&rels[i].follower.0)
            })
            .map(|i| part.relations[i])
            .collect();
        units.push(Unit {
            joints,
            fixed,
            vars: inner,
            rels: urels,
            closed,
            block: b,
        });
    }

    // place
    let mut poses = Poses {
        plan,
        local: &local,
        at: vec![Frame::WORLD; nv],
    };
    for (i, &c) in part.clusters.iter().enumerate() {
        poses.at[i] = plan.clusters[c].written;
    }
    let mut out = PartOut::default();
    for &r in &roots {
        if Some(r) != local.ground {
            out.mobility += 6;
        }
    }
    let mut new_memo = PartMemo::default();
    for (ui, u) in units.iter().enumerate() {
        let mut key = Vec::new();
        for &v in &u.fixed {
            if Some(v) != local.ground {
                bits(&poses.at[v], &mut key);
            }
        }
        for &k in &u.joints {
            for &f in plan.links[k].freedoms() {
                key.push(known.get(&(k, f)).map_or(u64::MAX, |v| v.0.to_bits()));
            }
        }
        let reuse = memo
            .and_then(|m| m.units.get(ui))
            .filter(|m| m.key == key)
            .cloned();
        let res = match reuse {
            Some(r) => {
                if u.closed.is_some() {
                    out.stats.bridges_placed += 0;
                } else {
                    out.stats.blocks_reused += 1;
                }
                r
            }
            None => {
                let mut r = if u.closed.is_some() {
                    out.stats.bridges_placed += 1;
                    place_bridge(plan, &poses, u, &known)
                } else {
                    out.stats.blocks_solved += 1;
                    solve_group(plan, &local, &poses, u, &known, &rels)
                };
                r.key = key;
                r
            }
        };
        for &(v, f) in &res.poses {
            poses.at[v] = f;
        }
        out.stats.newton_iterations += res.iterations;
        out.mobility += res.mobility;
        diagnostics.extend(res.diagnostics.iter().cloned());
        out.joints.extend(res.joints.iter().cloned());
        out.blocks.extend(res.block.iter().cloned());
        new_memo.units.push(res);
    }
    out.diagnostics = diagnostics;
    out.poses = part
        .clusters
        .iter()
        .enumerate()
        .map(|(i, &c)| (c, poses.at[i]))
        .collect();
    (out, new_memo)
}

fn directions(kind: JointKind, b: &Frame) -> Vec<(Freedom, Vec3)> {
    kind.freedoms()
        .iter()
        .map(|&f| {
            let v = match f {
                Freedom::Tx | Freedom::Rx => b.x().get(),
                Freedom::Ty | Freedom::Ry => b.y().get(),
                Freedom::Tz | Freedom::Rz => b.z().get(),
            };
            (f, v)
        })
        .collect()
}

fn place_bridge(plan: &Plan, poses: &Poses, u: &Unit, known: &Known) -> UnitOut {
    let c = u.closed.as_ref().expect("a closed-form edge");
    let link = c.joint.map(|k| (k, &plan.links[k]));
    let cl = |s: usize| plan.bodies[c.body[s]].cluster;
    let written = |s: usize| plan.clusters[cl(s)].written.compose(&c.local[s]);
    let rel_w = joint::relative(&written(0), &written(1));
    let mut diagnostics = Vec::new();
    let mut values = Vec::new();
    let mut free = 0;
    for &f in c.kind.freedoms() {
        let Some((k, j)) = link else {
            // loose constraints: the written values
            free += 1;
            values.push(joint::measure(f, &rel_w));
            continue;
        };
        let mut v = match known.get(&(k, f)) {
            Some(&(v, by)) => {
                let driven = j.drive.iter().any(|d| d.0 == f) || by.is_some();
                free += (!driven) as usize;
                v
            }
            None => {
                free += 1;
                0.0
            }
        };
        // a value at a limit to within the solve's rounding is at the limit, not past it
        let slack = if f.is_angle() {
            1e-9
        } else {
            forge_math::tol::LINEAR
        };
        if let Some(&(_, lo, hi)) = j.limits.iter().find(|l| l.0 == f)
            && (lo - slack..=hi + slack).contains(&v)
        {
            v = v.clamp(lo, hi);
        }
        if let Some(&(_, lo, hi)) = j.limits.iter().find(|l| l.0 == f)
            && !(lo..=hi).contains(&v)
        {
            let joint = j.joint_id().expect("a joint");
            let driven = j.drive.iter().any(|d| d.0 == f);
            diagnostics.push(Diagnostic::LimitViolated {
                joint,
                freedom: f,
                value: v,
                range: (lo, hi),
                fix: driven.then_some(crate::diag::Fix::SetDrive {
                    joint,
                    freedom: f,
                    value: v.clamp(lo, hi),
                }),
            });
            v = v.clamp(lo, hi);
        }
        values.push(v);
    }
    let m = joint::motion(c.kind, &values, &rel_w);
    let va = poses.local.vertex(cl(0));
    let exit = u.vars[0];
    // World(A) = World(B) ∘ M, with World(end) = P(cluster) ∘ local
    let pose = if exit == va {
        poses
            .cluster(cl(1))
            .compose(&c.local[1])
            .compose(&m)
            .compose(&c.local[0].inverse())
    } else {
        poses
            .cluster(cl(0))
            .compose(&c.local[0])
            .compose(&m.inverse())
            .compose(&c.local[1].inverse())
    };
    let b_world = if exit == va {
        poses.cluster(cl(1)).compose(&c.local[1])
    } else {
        pose.compose(&c.local[1])
    };
    if c.kind == JointKind::Ball {
        values.clear();
    }
    let joints = link
        .map(|(_, j)| JointReport {
            joint: j.joint_id().expect("a joint"),
            kind: c.kind,
            values: c.kind.freedoms().iter().copied().zip(values).collect(),
            free,
            directions: directions(c.kind, &b_world),
            in_loop: false,
            failed: false,
        })
        .into_iter()
        .collect();
    UnitOut {
        key: Vec::new(),
        poses: vec![(exit, pose)],
        diagnostics,
        joints,
        block: None,
        mobility: free,
        iterations: 0,
    }
}

// ---- loop blocks ----

fn drive_item(j: &plan::PLink, f: Freedom) -> Item {
    Item::of(ConstraintId::Joint {
        joint: j.joint_id().expect("a joint"),
        part: JointPart::Drive(f),
    })
}

/// What a loop block's rows come from: each one item of a diagnosis.
#[derive(Clone, Debug, PartialEq)]
enum Src {
    /// Link k's p-th primitive.
    Prim {
        k: usize,
        p: usize,
    },
    /// A joint value held at `target` (a drive, a relation's known value, or a limit).
    Value {
        k: usize,
        f: Freedom,
        target: f64,
        item: Item,
    },
    Relation {
        r: usize,
        offset: f64,
    },
}

impl Src {
    fn item(&self, plan: &Plan) -> Item {
        match self {
            Src::Prim { k, p } => plan.links[*k].item(*p),
            Src::Value { item, .. } => *item,
            Src::Relation { r, .. } => Item::Relation(plan.relations[*r].id),
        }
    }
    fn link(&self, plan: &Plan) -> Option<usize> {
        match self {
            Src::Prim { k, .. } | Src::Value { k, .. } => Some(*k),
            Src::Relation { r, .. } => Some(plan.relations[*r].follower.0),
        }
    }
}

/// A row with its derivative per cluster.
struct GRow {
    src: usize,
    r: f64,
    g: Vec<(usize, [f64; 6])>,
}

struct Block<'a> {
    plan: &'a Plan,
    /// Variable clusters, by vertex order.
    vars: Vec<usize>,
    slot: BTreeMap<usize, usize>,
    size: f64,
    /// The value each joint freedom is measured near: its written value at the start, then
    /// tracked through every accepted step, so angles keep their whole turns.
    near: std::cell::RefCell<BTreeMap<(usize, Freedom), f64>>,
    near0: BTreeMap<(usize, Freedom), f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Solved,
    /// A point where no step lowers the residual: the rows conflict here.
    Stuck,
    /// Still improving at the iteration cap.
    Capped,
}

impl Block<'_> {
    fn rows(&self, poses: &Poses, srcs: &[Src], active: &[bool]) -> Vec<GRow> {
        let mut out = Vec::new();
        let mut tmp: Vec<Row> = Vec::new();
        let clus = |k: usize, s: usize| self.plan.bodies[self.plan.links[k].body[s]].cluster;
        let scale = |row: &Row| if row.angular { self.size } else { 1.0 };
        let push = |out: &mut Vec<GRow>, si: usize, row: Row, k: usize| {
            let s = scale(&row);
            let ga = row.ga.map(|x| x * s);
            let gb = row.gb.map(|x| x * s);
            out.push(GRow {
                src: si,
                r: row.r * s,
                g: vec![(clus(k, 0), ga), (clus(k, 1), gb)],
            });
        };
        for (si, src) in srcs.iter().enumerate() {
            if !active[si] {
                continue;
            }
            match src {
                Src::Prim { k, p } => {
                    let e = poses.prim_ends(*k, *p);
                    tmp.clear();
                    e.rows(self.plan.links[*k].prims[*p].kind, &mut tmp);
                    for &row in &tmp {
                        push(&mut out, si, row, *k);
                    }
                }
                Src::Value { k, f, target, .. } => {
                    push(&mut out, si, poses.ends(*k).value_row(*f, *target), *k);
                }
                Src::Relation { r, offset } => {
                    let rel = &self.plan.relations[*r];
                    let (lk, lf) = rel.leader;
                    let (fk, ff) = rel.follower;
                    let near = |k, f| self.near.borrow().get(&(k, f)).copied().unwrap_or(0.0);
                    let lv = poses.ends(lk).value(lf, near(lk, lf));
                    let fv = poses.ends(fk).value(ff, near(fk, ff));
                    let slope = rel.kind.slope();
                    let s = if ff.is_angle() { self.size } else { 1.0 };
                    let g = vec![
                        (clus(fk, 0), fv.ga.map(|x| x * s)),
                        (clus(fk, 1), fv.gb.map(|x| x * s)),
                        (clus(lk, 0), lv.ga.map(|x| -x * slope * s)),
                        (clus(lk, 1), lv.gb.map(|x| -x * slope * s)),
                    ];
                    out.push(GRow {
                        src: si,
                        r: (fv.r - slope * lv.r - offset) * s,
                        g,
                    });
                }
            }
        }
        out
    }

    /// Moves each tracked value to where it is now, continuously from where it was.
    fn track(&self, poses: &Poses) {
        let mut near = self.near.borrow_mut();
        for (&(k, f), v) in near.iter_mut() {
            *v = poses.ends(k).value(f, *v).r;
        }
    }
    fn reset(&self) {
        *self.near.borrow_mut() = self.near0.clone();
    }
    fn tracked(&self, k: usize, f: Freedom) -> f64 {
        self.near.borrow().get(&(k, f)).copied().unwrap_or(0.0)
    }

    /// The signs the rows leave open: aligned (not opposed) axes, and angles on their own side
    /// rather than half a turn round.
    fn signs(&self, poses: &Poses, srcs: &[Src], active: &[bool]) -> Vec<f64> {
        let mut out = Vec::new();
        let toward = |e: &Ends, t: f64| {
            let (sn, cs) = dmath::sin_cos(t);
            e.a.x().get().dot(e.b.x().get() * cs + e.b.y().get() * sn)
        };
        for (si, src) in srcs.iter().enumerate() {
            if !active[si] {
                continue;
            }
            match src {
                Src::Prim { k, p } => {
                    let e = poses.prim_ends(*k, *p);
                    let z = e.a.z().get().dot(e.b.z().get());
                    match self.plan.links[*k].prims[*p].kind {
                        ConstraintKind::Parallel
                        | ConstraintKind::PlanePlane { .. }
                        | ConstraintKind::Concentric
                        | ConstraintKind::Insert { .. } => out.push(z),
                        ConstraintKind::Lock => {
                            out.push(z);
                            out.push(toward(&e, 0.0));
                        }
                        ConstraintKind::Twist { angle } => out.push(toward(&e, angle)),
                        _ => {}
                    }
                }
                Src::Value { k, f, target, .. } if f.is_angle() => out.push(toward(&poses.ends(*k), *target)),
                _ => {}
            }
        }
        out
    }

    /// Row-major Jacobian over the variables (ω scaled by the block size).
    fn jacobian(&self, rows: &[GRow]) -> Vec<f64> {
        let n = 6 * self.vars.len();
        let mut j = vec![0.0; rows.len() * n];
        for (i, row) in rows.iter().enumerate() {
            for (c, g) in &row.g {
                if let Some(&s) = self.slot.get(c) {
                    for t in 0..3 {
                        j[i * n + 6 * s + t] += g[t] / self.size;
                        j[i * n + 6 * s + 3 + t] += g[3 + t];
                    }
                }
            }
        }
        j
    }

    fn apply(&self, poses: &mut Poses, d: &[f64], a: f64) {
        for (s, &v) in self.vars.iter().enumerate() {
            let w = Vec3::new(d[6 * s], d[6 * s + 1], d[6 * s + 2]) * (a / self.size);
            let t = Vec3::new(d[6 * s + 3], d[6 * s + 4], d[6 * s + 5]) * a;
            poses.at[v] = joint::step(&poses.at[v], w, t);
        }
    }

    /// Minimum-norm Gauss-Newton from the poses as they are.
    fn newton(&self, poses: &mut Poses, srcs: &[Src], active: &[bool]) -> (Status, f64, usize) {
        let n = 6 * self.vars.len();
        let ss = |rows: &[GRow]| rows.iter().map(|r| r.r * r.r).sum::<f64>();
        let worst = |rows: &[GRow]| rows.iter().fold(0.0f64, |m, r| m.max(r.r.abs()));
        let tol = TIGHT * self.size.max(1.0);
        let ok = SOLVED * self.size.max(1.0);
        let mut rows = self.rows(poses, srcs, active);
        for it in 0..MAX_ITERATIONS {
            let e = worst(&rows);
            if e <= tol {
                return (Status::Solved, e, it);
            }
            let m = rows.len();
            let jac = self.jacobian(&rows);
            let mut jt = vec![0.0; n * m];
            for i in 0..m {
                for c in 0..n {
                    jt[c * m + i] = jac[i * n + c];
                }
            }
            let qr = Qr::new(jt, n, m, 1e-12);
            let b: Vec<f64> = rows.iter().map(|r| -r.r).collect();
            let d = qr.min_norm(&b);
            let f0 = ss(&rows);
            let saved = poses.at.clone();
            let before = self.signs(poses, srcs, active);
            let mut a = 1.0;
            let mut took = false;
            for _ in 0..16 {
                self.apply(poses, &d, a);
                let trial = self.rows(poses, srcs, active);
                // never step onto the other branch of a sign the rows cannot see
                let kept = self
                    .signs(poses, srcs, active)
                    .iter()
                    .zip(&before)
                    .all(|(now, was)| *was <= 0.0 || *now > 0.0);
                if kept && ss(&trial) < f0 {
                    rows = trial;
                    self.track(poses);
                    took = true;
                    break;
                }
                poses.at.clone_from(&saved);
                a *= 0.5;
            }
            if !took {
                let st = if e <= ok { Status::Solved } else { Status::Stuck };
                return (st, e, it);
            }
        }
        let e = worst(&rows);
        let st = if e <= ok { Status::Solved } else { Status::Capped };
        (st, e, MAX_ITERATIONS)
    }
}

fn solve_group(
    plan: &Plan,
    local: &Local,
    start: &Poses,
    u: &Unit,
    known: &Known,
    _rels: &[&plan::PRelation],
) -> UnitOut {
    // start: the written poses, carried with the block's fixed vertex
    let mut poses = Poses {
        plan,
        local,
        at: start.at.clone(),
    };
    let carry = u
        .fixed
        .iter()
        .find(|&&v| Some(v) != local.ground)
        .map(|&v| poses.at[v].compose(&plan.clusters[local.clusters[v]].written.inverse()));
    for &v in &u.vars {
        let w = plan.clusters[local.clusters[v]].written;
        poses.at[v] = match carry {
            Some(c) => c.compose(&w),
            None => w,
        };
    }
    let begin = poses.at.clone();
    let mut size = 1.0f64;
    let mut near = BTreeMap::new();
    let mut srcs: Vec<Src> = Vec::new();
    for &k in &u.joints {
        let j = &plan.links[k];
        for l in &j.local {
            size = size.max(l.origin.to_vec().abs_max());
        }
        for p in 0..j.prims.len() {
            srcs.push(Src::Prim { k, p });
        }
        for &f in j.freedoms() {
            near.insert((k, f), written_value(plan, k, f));
            if let Some(&(v, by)) = known.get(&(k, f)) {
                srcs.push(Src::Value {
                    k,
                    f,
                    target: v,
                    item: by.unwrap_or_else(|| drive_item(j, f)),
                });
            }
        }
    }
    for &r in &u.rels {
        srcs.push(Src::Relation {
            r,
            offset: relation_offset(plan, &plan.relations[r]),
        });
    }
    for &v in &u.vars {
        size = size.max(poses.at[v].origin.to_vec().abs_max());
    }
    let size = size.min(plan.size.max(1.0)).max(1.0);
    let slot: BTreeMap<usize, usize> = u
        .vars
        .iter()
        .enumerate()
        .map(|(s, &v)| (local.clusters[v], s))
        .collect();
    let blk = Block {
        plan,
        vars: u.vars.clone(),
        slot,
        size,
        near: std::cell::RefCell::new(near.clone()),
        near0: near,
    };
    let mut active = vec![true; srcs.len()];
    let mut diagnostics = Vec::new();
    let mut iterations = 0;
    // values far from where the written poses hold them are reached in steps, so the loop stays
    // on the written branch (a function of the document only)
    let from: Vec<Option<f64>> = srcs
        .iter()
        .map(|x| match x {
            Src::Value { k, f, .. } => Some(poses.ends(*k).value(*f, blk.tracked(*k, *f)).r),
            _ => None,
        })
        .collect();
    let mut steps = 1usize;
    for (x, f0) in srcs.iter().zip(&from) {
        if let (Src::Value { f, target, .. }, Some(f0)) = (x, f0) {
            let unit = if f.is_angle() { 0.25 } else { 0.1 * size };
            steps = steps.max(((target - f0).abs() / unit).ceil() as usize);
        }
    }
    let steps = steps.min(256);
    for i in 1..steps {
        let t = i as f64 / steps as f64;
        let ramp: Vec<Src> = srcs
            .iter()
            .zip(&from)
            .map(|(x, f0)| match (x, f0) {
                (Src::Value { k, f, target, item }, Some(f0)) => Src::Value {
                    k: *k,
                    f: *f,
                    target: f0 + (target - f0) * t,
                    item: *item,
                },
                _ => x.clone(),
            })
            .collect();
        iterations += blk.newton(&mut poses, &ramp, &active).2;
    }
    let (mut status, _, it) = blk.newton(&mut poses, &srcs, &active);
    iterations += it;
    let mut failed: Vec<usize> = Vec::new();
    if status != Status::Solved {
        // diagnosis: add sources in item order; the first that cannot be met is a culprit, and a
        // deletion filter over what came before gives a minimal conflicting set
        let mut order: Vec<usize> = (0..srcs.len()).collect();
        order.sort_by_key(|&i| plan.blame(srcs[i].item(plan)));
        let mut on = vec![false; srcs.len()];
        let solves = |on: &[bool], poses: &mut Poses| {
            poses.at.clone_from(&begin);
            blk.reset();
            blk.newton(poses, &srcs, on)
        };
        for &i in &order {
            on[i] = true;
            let (st, _, _) = solves(&on, &mut poses);
            if st == Status::Solved {
                continue;
            }
            let mut set: Vec<usize> = order.iter().copied().filter(|&x| on[x]).collect();
            for &x in set.clone().iter().rev() {
                if x == i {
                    continue;
                }
                let trial: Vec<bool> = (0..srcs.len()).map(|y| set.contains(&y) && y != x).collect();
                if solves(&trial, &mut poses).0 != Status::Solved {
                    set.retain(|&y| y != x);
                }
            }
            let only: Vec<bool> = (0..srcs.len()).map(|y| set.contains(&y)).collect();
            let (st2, _, _) = solves(&only, &mut poses);
            let rows = blk.rows(&poses, &srcs, &only);
            let mut items = Vec::new();
            let mut residuals = Vec::new();
            for &x in &set {
                items.push(srcs[x].item(plan));
                residuals.push(
                    rows.iter()
                        .filter(|r| r.src == x)
                        .fold(0.0f64, |m, r| m.max(r.r.abs())),
                );
            }
            if st2 == Status::Capped {
                diagnostics.push(Diagnostic::NotConverged {
                    block: u.block,
                    items: srcs.iter().map(|x| x.item(plan)).collect(),
                    residual: residuals.iter().fold(0.0f64, |m, r| m.max(*r)),
                });
            } else {
                diagnostics.push(Diagnostic::Conflict { items, residuals });
            }
            on[i] = false;
            failed.push(i);
        }
        active = on;
        let r = solves(&active, &mut poses);
        status = r.0;
        iterations += r.2;
        if status != Status::Solved {
            poses.at.clone_from(&begin);
            blk.reset();
            diagnostics.push(Diagnostic::NotConverged {
                block: u.block,
                items: srcs.iter().map(|x| x.item(plan)).collect(),
                residual: r.1,
            });
        }
    }
    // limits: hold a value that lies outside at the nearer limit, and keep it if the loop closes
    for _ in 0..4 {
        let mut add = Vec::new();
        for &k in &u.joints {
            let j = &plan.links[k];
            if !srcs
                .iter()
                .enumerate()
                .any(|(i, x)| active[i] && x.link(plan) == Some(k))
            {
                continue;
            }
            for &(f, lo, hi) in &j.limits {
                let held = srcs.iter().enumerate().any(|(i, x)| {
                    active[i] && matches!(x, Src::Value { k: kk, f: ff, .. } if *kk == k && *ff == f)
                });
                if held {
                    continue;
                }
                let v = poses.ends(k).value(f, blk.tracked(k, f)).r;
                if v < lo - 1e-9 || v > hi + 1e-9 {
                    add.push((k, f, v, if v < lo { lo } else { hi }, (lo, hi)));
                }
            }
        }
        if add.is_empty() {
            break;
        }
        let saved = (srcs.len(), poses.at.clone(), blk.near.borrow().clone());
        for &(k, f, _, b, _) in &add {
            let item = Item::of(ConstraintId::Joint {
                joint: plan.links[k].joint_id().expect("a joint"),
                part: JointPart::Limit(f),
            });
            srcs.push(Src::Value {
                k,
                f,
                target: b,
                item,
            });
            active.push(true);
        }
        let (st, _, it) = blk.newton(&mut poses, &srcs, &active);
        iterations += it;
        if st != Status::Solved {
            srcs.truncate(saved.0);
            active.truncate(saved.0);
            poses.at = saved.1;
            *blk.near.borrow_mut() = saved.2;
            for &(k, f, v, _, range) in &add {
                diagnostics.push(Diagnostic::LimitViolated {
                    joint: plan.links[k].joint_id().expect("a joint"),
                    freedom: f,
                    value: v,
                    range,
                    fix: None,
                });
            }
            break;
        }
    }

    // rank at the solution: redundancy, mobility, each joint's free motions
    let rows = blk.rows(&poses, &srcs, &active);
    let m = rows.len();
    let n = 6 * blk.vars.len();
    let jac = blk.jacobian(&rows);
    let mut jt = vec![0.0; n * m];
    for i in 0..m {
        for c in 0..n {
            jt[c * m + i] = jac[i * n + c];
        }
    }
    let qr = Qr::new(jt, n, m, 1e-9);
    let rank = qr.rank;
    if rank < m && status == Status::Solved {
        redundancy(plan, &srcs, &rows, &jac, n, &mut diagnostics);
    }
    let null = qr.null_space();
    let mut joints = Vec::new();
    for &k in &u.joints {
        let j = &plan.links[k];
        if j.joint.is_none() {
            continue;
        }
        let k = &k;
        let si = srcs.iter().position(|x| x.link(plan) == Some(*k)).unwrap_or(0);
        let e = poses.ends(*k);
        let mut grads = Vec::new();
        let mut values = Vec::new();
        for &f in j.freedoms() {
            let row = e.value(f, blk.tracked(*k, f));
            let s = if row.angular { size } else { 1.0 };
            let gr = GRow {
                src: si,
                r: row.r,
                g: vec![
                    (plan.bodies[j.body[0]].cluster, row.ga.map(|x| x * s)),
                    (plan.bodies[j.body[1]].cluster, row.gb.map(|x| x * s)),
                ],
            };
            grads.extend(blk.jacobian(std::slice::from_ref(&gr)));
            if j.kind() != JointKind::Ball {
                values.push((f, row.r));
            }
        }
        let nf = j.freedoms().len();
        let free = if j.kind() == JointKind::Ball {
            // a ball's freedom is any rotation: count the motions that turn A relative to B
            let mut g = Vec::new();
            for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
                let mut row = vec![0.0; n];
                for (side, sign) in [(0, 1.0), (1, -1.0)] {
                    if let Some(&s) = blk.slot.get(&plan.bodies[j.body[side]].cluster) {
                        for t in 0..3 {
                            row[6 * s + t] = sign * axis[t] / size;
                        }
                    }
                }
                g.extend(row);
            }
            projected_rank(&g, 3, n, &null)
        } else {
            projected_rank(&grads, nf, n, &null)
        };
        joints.push(JointReport {
            joint: j.joint_id().expect("a joint"),
            kind: j.kind(),
            values,
            free,
            directions: directions(j.kind(), &e.b),
            in_loop: true,
            failed: failed.iter().any(|&x| srcs[x].link(plan) == Some(*k)),
        });
    }
    let mut instances: Vec<InstanceId> = u
        .vars
        .iter()
        .flat_map(|&v| {
            plan.clusters[local.clusters[v]]
                .members
                .iter()
                .map(|&b| plan.bodies[b].id)
        })
        .collect();
    instances.sort();
    let block = BlockReport {
        block: u.block,
        links: srcs.iter().map(|x| x.item(plan)).collect(),
        instances,
        structural_mobility: n as i64 - m as i64,
        mobility: n - rank,
        rows: m,
        rank,
        iterations,
    };
    UnitOut {
        key: Vec::new(),
        poses: u.vars.iter().map(|&v| (v, poses.at[v])).collect(),
        diagnostics,
        joints,
        block: Some(block),
        mobility: n - rank,
        iterations,
    }
}

/// Rank of the `f x n` gradient rows restricted to the null space.
fn projected_rank(g: &[f64], f: usize, n: usize, null: &[Vec<f64>]) -> usize {
    if null.is_empty() || f == 0 {
        return 0;
    }
    let k = null.len();
    let mut a = vec![0.0; f * k];
    for i in 0..f {
        for (c, z) in null.iter().enumerate() {
            a[i * k + c] = (0..n).map(|t| g[i * n + t] * z[t]).sum();
        }
    }
    crate::qr::rank(&a, f, k)
}

/// Rows that add no rank, taken in item order: each is named with the items its row is a
/// combination of.
fn redundancy(plan: &Plan, srcs: &[Src], rows: &[GRow], jac: &[f64], n: usize, out: &mut Vec<Diagnostic>) {
    let mut order: Vec<usize> = (0..rows.len()).collect();
    order.sort_by_key(|&i| (plan.blame(srcs[rows[i].src].item(plan)), i));
    // modified Gram-Schmidt in item order, twice for safety
    let mut basis: Vec<Vec<f64>> = Vec::new();
    let mut kept: Vec<usize> = Vec::new();
    let mut found: BTreeMap<Item, (usize, Vec<Item>)> = BTreeMap::new();
    for &i in &order {
        let row = &jac[i * n..(i + 1) * n];
        let norm0 = row.iter().map(|x| x * x).sum::<f64>().sqrt();
        let mut v = row.to_vec();
        for _ in 0..2 {
            for b in &basis {
                let d: f64 = v.iter().zip(b).map(|(x, y)| x * y).sum();
                for (x, y) in v.iter_mut().zip(b) {
                    *x -= d * y;
                }
            }
        }
        let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > 1e-8 * norm0.max(1e-300) && norm0 > 0.0 {
            for x in v.iter_mut() {
                *x /= norm;
            }
            basis.push(v);
            kept.push(i);
            continue;
        }
        // which kept rows it depends on
        let m = kept.len();
        let mut a = vec![0.0; n * m];
        for (c, &r) in kept.iter().enumerate() {
            for t in 0..n {
                a[t * m + c] = jac[r * n + t];
            }
        }
        let item = srcs[rows[i].src].item(plan);
        let mut by: Vec<Item> = Vec::new();
        if m > 0
            && let Some(c) = forge_math::linalg::solve_least_squares(&a, n, m, row)
        {
            let cmax = c.iter().fold(0.0f64, |s, x| s.max(x.abs()));
            for (ci, &r) in kept.iter().enumerate() {
                let it = srcs[rows[r].src].item(plan);
                if c[ci].abs() > 1e-9 * cmax.max(1.0) && it != item {
                    by.push(it);
                }
            }
        }
        let e = found.entry(item).or_insert((0, Vec::new()));
        e.0 += 1;
        e.1.extend(by);
    }
    for (item, (rows, mut by)) in found {
        by.sort();
        by.dedup();
        out.push(Diagnostic::Redundant {
            item,
            rows,
            implied_by: by,
        });
    }
}

// ---- the whole level ----

/// The memo of a whole level: each partition's last result and units.
#[derive(Clone, Debug, Default)]
pub(crate) struct Memo {
    pub parts: Vec<Option<(PartOut, PartMemo)>>,
}

fn par_map<T: Send>(items: &[usize], threads: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    #[cfg(target_family = "wasm")]
    let threads = {
        let _ = threads;
        1
    };
    if threads <= 1 || items.len() < 2 {
        return items.iter().map(|&i| f(i)).collect();
    }
    let chunk = items.len().div_ceil(threads);
    std::thread::scope(|s| {
        let hs: Vec<_> = items
            .chunks(chunk)
            .map(|c| {
                let f = &f;
                s.spawn(move || c.iter().map(|&i| f(i)).collect::<Vec<T>>())
            })
            .collect();
        hs.into_iter()
            .flat_map(|h| h.join().expect("a partition"))
            .collect()
    })
}

/// Solves a planned level. Partitions marked clean in `memo` are reused whole; the others reuse
/// any unit whose inputs are bit-identical.
pub(crate) fn run(plan: &Plan, opts: &Options, memo: Option<(&mut Memo, &[bool])>) -> Solution {
    let np = plan.partitions.len();
    let (memo, dirty) = match memo {
        Some((m, d)) => (Some(m), Some(d)),
        None => (None, None),
    };
    let todo: Vec<usize> = (0..np)
        .filter(|&p| {
            dirty.is_none_or(|d| d[p])
                || memo
                    .as_ref()
                    .is_none_or(|m| m.parts.get(p).is_none_or(|x| x.is_none()))
        })
        .collect();
    let prev: Option<&Memo> = memo.as_deref();
    let solved = par_map(&todo, opts.threads, |p| {
        let pm = prev
            .and_then(|m| m.parts.get(p))
            .and_then(|x| x.as_ref())
            .map(|x| &x.1);
        solve_partition(plan, p, pm)
    });
    let mut stats = Stats::default();
    let mut results: Vec<Option<(PartOut, PartMemo)>> = match &memo {
        Some(m) if m.parts.len() == np => m.parts.clone(),
        _ => vec![None; np],
    };
    stats.partitions_reused = np - todo.len();
    stats.partitions_solved = todo.len();
    for (p, r) in todo.iter().zip(solved) {
        results[*p] = Some(r);
    }

    // cluster poses: written, then each partition's
    let mut cpose: Vec<Frame> = plan.clusters.iter().map(|c| c.written).collect();
    let mut sol = Solution {
        diagnostics: plan.diagnostics.clone(),
        ..Solution::default()
    };
    let mut lone = 0;
    for (c, cl) in plan.clusters.iter().enumerate() {
        if !cl.ground && plan.part_of[c] == usize::MAX {
            lone += 6;
        }
    }
    sol.mobility = lone;
    for (p, r) in results.iter().enumerate() {
        let Some(r) = r else { continue };
        let out = &r.0;
        let fresh = todo.binary_search(&p).is_ok();
        for &(c, f) in &out.poses {
            cpose[c] = f;
        }
        sol.diagnostics.extend(out.diagnostics.iter().cloned());
        sol.joints.extend(out.joints.iter().cloned());
        sol.blocks.extend(out.blocks.iter().cloned());
        sol.mobility += out.mobility;
        let s = if fresh { out.stats } else { Stats::default() };
        stats.blocks_solved += s.blocks_solved;
        stats.blocks_reused += s.blocks_reused;
        stats.bridges_placed += s.bridges_placed;
        stats.newton_iterations += s.newton_iterations;
    }
    if let Some(m) = memo {
        m.parts = results;
    }
    // joints held rigid by a cluster or by ground: checked against what holds them
    for &k in &plan.rigid_joints {
        let j = &plan.links[k];
        let w = |local: &[Frame; 2]| {
            let pa = cpose[plan.bodies[j.body[0]].cluster];
            let pb = cpose[plan.bodies[j.body[1]].cluster];
            Ends {
                a: pa.compose(&local[0]),
                b: pb.compose(&local[1]),
                ta: pa.origin,
                tb: pb.origin,
            }
        };
        let mut gaps: Vec<(Item, f64)> = (0..j.prims.len())
            .map(|p| (j.item(p), w(&j.prims[p].local).gap(j.prims[p].kind, plan.size)))
            .collect();
        let rel = joint::relative(&w(&j.local).a, &w(&j.local).b);
        for &(f, v) in &j.drive {
            let got = joint::measure(f, &rel);
            let d = if f.is_angle() {
                (joint::unwrap_near(got, v) - v).abs() * plan.size
            } else {
                (got - v).abs()
            };
            gaps.push((drive_item(j, f), d));
        }
        let (ca, cb) = (plan.bodies[j.body[0]].cluster, plan.bodies[j.body[1]].cluster);
        let mut implied = Vec::new();
        if ca == cb {
            let mut x = j.body[0];
            let mut chain = vec![x];
            while let Some((p, _)) = plan.fastened_parent[x] {
                chain.push(p);
                x = p;
            }
            let mut y = j.body[1];
            let mut other = vec![y];
            while let Some((p, _)) = plan.fastened_parent[y] {
                other.push(p);
                y = p;
            }
            for (c, o) in [(&chain, &other), (&other, &chain)] {
                for &z in c.iter().take_while(|z| !o.contains(z)) {
                    if let Some((_, it)) = plan.fastened_parent[z] {
                        implied.push(it);
                    }
                }
            }
        } else {
            for c in [ca, cb] {
                if let Some(&g) = plan.clusters[c].members.iter().find(|&&m| plan.bodies[m].ground) {
                    implied.push(Item::Ground(plan.bodies[g].id));
                }
            }
        }
        implied.sort();
        implied.dedup();
        for (item, gap) in gaps {
            plan::closed_form(&mut sol.diagnostics, item, implied.clone(), gap);
        }
    }

    // per constraint: what it removes and what of that is redundant
    let mut red: BTreeMap<Item, (usize, bool)> = BTreeMap::new();
    for d in &sol.diagnostics {
        match d {
            Diagnostic::Redundant { item, rows, .. } => {
                let e = red.entry(*item).or_insert((0, false));
                e.0 += if *rows == 0 { usize::MAX / 2 } else { *rows };
            }
            Diagnostic::Conflict { items, .. } => {
                if let Some(last) = items.last() {
                    red.entry(*last).or_insert((0, false)).1 = true;
                }
            }
            _ => {}
        }
    }
    for l in &plan.links {
        let mut add = |id: ConstraintId, kind: Option<ConstraintKind>, removes: usize| {
            let (r, failed) = red.get(&Item::of(id)).copied().unwrap_or((0, false));
            sol.constraints.push(ConstraintReport {
                id,
                joint: id.joint(),
                kind,
                removes,
                redundant: r.min(removes),
                failed,
            });
        };
        for p in &l.prims {
            add(p.id, Some(p.kind), p.kind.removes());
        }
        if let Some(j) = l.joint_id() {
            for &(f, _) in &l.drive {
                add(
                    ConstraintId::Joint {
                        joint: j,
                        part: JointPart::Drive(f),
                    },
                    None,
                    1,
                );
            }
        }
    }
    sol.constraints.sort_by_key(|c| c.id);

    // poses and occurrences
    for b in &plan.bodies {
        sol.poses.push((b.id, cpose[b.cluster].compose(&b.offset)));
    }
    let mut hashes: BTreeMap<PartRef, Option<ContentHash>> = BTreeMap::new();
    for (i, b) in plan.bodies.iter().enumerate() {
        let pose = sol.poses[i].1;
        match &plan.of[i] {
            Component::Part(p) => {
                sol.occurrences.push(Occurrence {
                    path: vec![b.id],
                    part: p.clone(),
                    pose,
                    reference: plan.reference[i],
                });
            }
            Component::Assembly(d) => {
                if let Some(s) = plan.subs.get(d) {
                    for (path, (p, f, r)) in &s.occ {
                        let mut full = vec![b.id];
                        full.extend(path.iter().copied());
                        sol.occurrences.push(Occurrence {
                            path: full,
                            part: p.clone(),
                            pose: pose.compose(f),
                            reference: plan.reference[i] || *r,
                        });
                    }
                    for dg in &s.diagnostics {
                        sol.diagnostics.push(Diagnostic::Inside {
                            path: vec![b.id],
                            diagnostic: Box::new(dg.clone()),
                        });
                    }
                }
            }
        }
    }
    sol.stats = stats;
    let _ = &mut hashes;
    sol
}

/// Records each part's built hash and warns where a pin disagrees.
pub(crate) fn hashes<P: Parts>(sol: &mut Solution, parts: &P) {
    let mut seen: BTreeMap<PartRef, Option<ContentHash>> = BTreeMap::new();
    for o in &sol.occurrences {
        if seen.contains_key(&o.part) {
            continue;
        }
        let h = parts.part(&o.part).and_then(|i| i.hash);
        if let Some(pin) = o.part.pin
            && h != Some(pin)
        {
            sol.diagnostics.push(Diagnostic::PartChanged {
                part: o.part.clone(),
                pinned: pin,
                found: h,
            });
        }
        seen.insert(o.part.clone(), h);
    }
    sol.hashes = seen.into_iter().collect();
}
