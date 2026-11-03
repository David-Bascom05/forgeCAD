//! The structural passes, before any numerics: anchors resolved by name in one pass over the
//! joints, rigid clusters by union-find over fastened joints, partitions as connected components of
//! the cluster graph, and each partition's biconnected blocks (prior-art §1.5, passes 1 to 3).
//! Every order here is by instance and joint id.

use crate::diag::{AnchorReason, Diagnostic, Item};
use crate::joint;
use crate::model::*;
use forge_math::Frame;
use std::collections::BTreeMap;

/// Frames closer than this (mm, and rad × size) are the same: what decides redundant from
/// conflicting in closed form.
pub(crate) const SAME: f64 = 1e-7;

/// A top-level instance: one rigid body to the solver (a part, or a rigid subassembly).
#[derive(Clone, Debug)]
pub(crate) struct Body {
    pub id: InstanceId,
    pub written: Frame,
    pub ground: bool,
    pub cluster: usize,
    /// The body's frame in its cluster's frame.
    pub offset: Frame,
}

#[derive(Clone, Debug)]
pub(crate) struct Cluster {
    /// Members by body index, least id first; the first places the cluster.
    pub members: Vec<usize>,
    pub ground: bool,
    /// The cluster's frame as written: its placing member's written pose.
    pub written: Frame,
}

/// What a link is in the document.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LinkId {
    Joint(JointId),
    Loose(u32),
}

/// One primitive constraint of a link, its ends in cluster frames.
#[derive(Clone, Debug)]
pub(crate) struct Prim {
    pub id: ConstraintId,
    pub kind: ConstraintKind,
    pub local: [Frame; 2],
}

/// A joint or a loose constraint: an edge between two bodies, made of primitives.
#[derive(Clone, Debug)]
pub(crate) struct PLink {
    pub id: LinkId,
    /// Some for a joint.
    pub joint: Option<JointKind>,
    pub body: [usize; 2],
    /// Each end's reference in its cluster's frame (A turned by `flip` already).
    pub local: [Frame; 2],
    pub prims: Vec<Prim>,
    pub drive: Vec<(Freedom, f64)>,
    pub limits: Vec<(Freedom, f64, f64)>,
}

impl PLink {
    /// A fastened joint or a lock: joins a rigid cluster.
    pub fn rigid(&self) -> bool {
        self.prims.len() == 1 && self.prims[0].kind == ConstraintKind::Lock
    }
    /// A joint's kind (only called on joints).
    pub fn kind(&self) -> JointKind {
        self.joint.expect("a joint")
    }
    pub fn item(&self, p: usize) -> Item {
        Item::of(self.prims[p].id)
    }
    pub fn joint_id(&self) -> Option<JointId> {
        match self.id {
            LinkId::Joint(j) => Some(j),
            LinkId::Loose(_) => None,
        }
    }
    pub fn freedoms(&self) -> &'static [Freedom] {
        self.joint.map_or(&[], |k| k.freedoms())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PRelation {
    pub id: RelationId,
    pub kind: RelationKind,
    /// (joint index, freedom)
    pub leader: (usize, Freedom),
    pub follower: (usize, Freedom),
    pub offset: Option<f64>,
}

/// A connected part of the cluster graph, ground cut out: solvable on its own.
#[derive(Clone, Debug, Default)]
pub(crate) struct Partition {
    pub clusters: Vec<usize>,
    /// Joint indices of its non-fastened edges, by id.
    pub edges: Vec<usize>,
    pub relations: Vec<usize>,
    pub grounded: bool,
}

/// A rigid subassembly's solved inside: each part occurrence's pose in the subassembly's frame.
#[derive(Clone, Debug, Default)]
pub(crate) struct Sub {
    pub occ: BTreeMap<Path, (PartRef, Frame, bool)>,
    pub diagnostics: Vec<Diagnostic>,
}

pub(crate) struct Plan {
    pub bodies: Vec<Body>,
    pub of: Vec<Component>,
    pub reference: Vec<bool>,
    pub clusters: Vec<Cluster>,
    pub links: Vec<PLink>,
    /// Joint id to link index, by id.
    pub joint_link: Vec<(JointId, usize)>,
    pub relations: Vec<PRelation>,
    pub partitions: Vec<Partition>,
    /// Partition of each cluster (usize::MAX for a lone ungrounded cluster with no edges).
    pub part_of: Vec<usize>,
    /// Non-fastened joints with both ends in one cluster, or both on ground: checked, not solved.
    pub rigid_joints: Vec<usize>,
    /// Each fastened joint's parent body chain, to name what implies what.
    pub fastened_parent: Vec<Option<(usize, Item)>>,
    pub diagnostics: Vec<Diagnostic>,
    pub subs: BTreeMap<DocId, Sub>,
    /// Size of the assembly, for scaling angles to lengths.
    pub size: f64,
}

pub(crate) struct Uf(Vec<usize>);

impl Uf {
    pub fn new(n: usize) -> Uf {
        Uf((0..n).collect())
    }
    pub fn find(&mut self, mut i: usize) -> usize {
        while self.0[i] != i {
            self.0[i] = self.0[self.0[i]];
            i = self.0[i];
        }
        i
    }
    /// Joins two sets; the smaller index is the root, so roots do not depend on join order.
    pub fn union(&mut self, a: usize, b: usize) -> bool {
        let (a, b) = (self.find(a), self.find(b));
        if a == b {
            return false;
        }
        self.0[a.max(b)] = a.min(b);
        true
    }
}

/// A subassembly document solved once and cached; None if it is cyclic or missing.
pub(crate) fn sub<P: Parts>(
    doc: &DocId,
    parts: &P,
    subs: &mut BTreeMap<DocId, Sub>,
    stack: &mut Vec<DocId>,
    diags: &mut Vec<Diagnostic>,
) -> bool {
    if subs.contains_key(doc) {
        return true;
    }
    if stack.contains(doc) {
        diags.push(Diagnostic::CyclicAssembly { doc: doc.clone() });
        return false;
    }
    let Some(asm) = parts.assembly(doc) else {
        return false;
    };
    stack.push(doc.clone());
    let mut plan = Plan::new(asm, parts, subs, stack);
    stack.pop();
    let sol = crate::solve::run(&plan, &crate::solve::Options { threads: 1 }, None);
    *subs = std::mem::take(&mut plan.subs);
    let mut out = Sub {
        diagnostics: sol.diagnostics.clone(),
        ..Sub::default()
    };
    for o in sol.occurrences {
        out.occ.insert(o.path, (o.part, o.pose, o.reference));
    }
    subs.insert(doc.clone(), out);
    true
}

type Level = (Vec<Body>, Vec<Component>, Vec<bool>, Vec<InstanceId>);

/// A level's bodies by id (suppressed ones listed apart, by id); the subassemblies they use are
/// solved into `subs`.
pub(crate) fn level<P: Parts>(
    asm: &Assembly,
    parts: &P,
    subs: &mut BTreeMap<DocId, Sub>,
    stack: &mut Vec<DocId>,
    diags: &mut Vec<Diagnostic>,
) -> Level {
    // bodies by id; duplicates reported, the first kept
    let mut order: Vec<usize> = (0..asm.instances.len()).collect();
    order.sort_by_key(|&i| (asm.instances[i].id, i));
    let mut bodies = Vec::new();
    let mut of = Vec::new();
    let mut reference = Vec::new();
    let mut suppressed = Vec::new();
    for &i in &order {
        let inst = &asm.instances[i];
        if bodies.last().is_some_and(|b: &Body| b.id == inst.id) || suppressed.binary_search(&inst.id).is_ok()
        {
            diags.push(Diagnostic::DuplicateInstance { id: inst.id });
            continue;
        }
        if inst.suppressed {
            suppressed.push(inst.id);
            continue;
        }
        if let Component::Assembly(d) = &inst.of {
            // missing or cyclic: joints into it report a broken anchor
            sub(d, parts, subs, stack, diags);
        }
        bodies.push(Body {
            id: inst.id,
            written: inst.pose,
            ground: inst.ground,
            cluster: usize::MAX,
            offset: Frame::WORLD,
        });
        of.push(inst.of.clone());
        reference.push(inst.reference);
    }
    (bodies, of, reference, suppressed)
}

impl Plan {
    /// The plan of one assembly level; subassemblies it uses are solved into `subs` first, and
    /// the plan takes them.
    pub fn new<P: Parts>(
        asm: &Assembly,
        parts: &P,
        subs: &mut BTreeMap<DocId, Sub>,
        stack: &mut Vec<DocId>,
    ) -> Plan {
        let mut diags = Vec::new();
        let (mut bodies, of, reference, suppressed) = level(asm, parts, subs, stack, &mut diags);
        let mut joints = resolve_links(asm, parts, &bodies, &of, &suppressed, subs, &mut diags);

        let any_ground = bodies.iter().any(|b| b.ground);
        if !any_ground && !bodies.is_empty() {
            diags.push(Diagnostic::NoGround);
        }
        let mut size = 1.0f64;
        for b in &bodies {
            size = size.max(b.written.origin.to_vec().abs_max());
        }
        for j in &joints {
            for l in &j.local {
                size = size.max(l.origin.to_vec().abs_max());
            }
        }

        // pass 1: rigid clusters
        let n = bodies.len();
        let mut uf = Uf::new(n);
        for j in joints.iter().filter(|j| j.rigid()) {
            uf.union(j.body[0], j.body[1]);
        }
        let mut cl_of = vec![usize::MAX; n];
        let mut clusters: Vec<Cluster> = Vec::new();
        for i in 0..n {
            let r = uf.find(i);
            if cl_of[r] == usize::MAX {
                cl_of[r] = clusters.len();
                clusters.push(Cluster {
                    members: Vec::new(),
                    ground: false,
                    written: Frame::WORLD,
                });
            }
            let c = cl_of[r];
            cl_of[i] = c;
            clusters[c].members.push(i);
            clusters[c].ground |= bodies[i].ground;
        }
        for (i, b) in bodies.iter_mut().enumerate() {
            b.cluster = cl_of[i];
        }
        // fastened adjacency by joint id
        let mut fadj: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];
        for (k, j) in joints.iter().enumerate().filter(|(_, j)| j.rigid()) {
            fadj[j.body[0]].push((k, j.body[1]));
            fadj[j.body[1]].push((k, j.body[0]));
        }
        let mut parent: Vec<Option<(usize, Item)>> = vec![None; n];
        let mut placed = vec![false; n];
        let mut tree_edge = vec![false; joints.len()];
        for cl in clusters.iter_mut() {
            let root = if cl.ground {
                *cl.members
                    .iter()
                    .find(|&&m| bodies[m].ground)
                    .expect("a grounded member")
            } else {
                cl.members[0]
            };
            cl.written = bodies[root].written;
            placed[root] = true;
            let mut queue = std::collections::VecDeque::from([root]);
            while let Some(u) = queue.pop_front() {
                for &(k, v) in &fadj[u] {
                    if placed[v] {
                        continue;
                    }
                    let j = &joints[k];
                    // the anchor frames meet: O_a ∘ la = O_b ∘ lb
                    let ou = bodies[u].offset;
                    let off = if j.body[0] == v {
                        ou.compose(&j.local[1]).compose(&j.local[0].inverse())
                    } else {
                        ou.compose(&j.local[0]).compose(&j.local[1].inverse())
                    };
                    bodies[v].offset = off;
                    placed[v] = true;
                    tree_edge[k] = true;
                    parent[v] = Some((u, j.item(0)));
                    queue.push_back(v);
                }
            }
        }
        // local anchors into cluster frames
        for j in joints.iter_mut() {
            for k in 0..2 {
                let off = bodies[j.body[k]].offset;
                j.local[k] = off.compose(&j.local[k]);
                for p in j.prims.iter_mut() {
                    p.local[k] = off.compose(&p.local[k]);
                }
            }
        }
        // closing fastened joints and extra grounds: implied or conflicting, decided in closed form
        let path = |a: usize, b: usize| -> Vec<Item> {
            let chain = |mut x: usize| {
                let mut v = vec![x];
                while let Some((p, _)) = parent[x] {
                    v.push(p);
                    x = p;
                }
                v
            };
            let (ca, cb) = (chain(a), chain(b));
            let mut items: Vec<Item> = Vec::new();
            for (c, o) in [(&ca, &cb), (&cb, &ca)] {
                for &x in c.iter().take_while(|x| !o.contains(x)) {
                    if let Some((_, it)) = parent[x] {
                        items.push(it);
                    }
                }
            }
            items.sort();
            items.dedup();
            items
        };
        for (k, j) in joints.iter().enumerate() {
            if !j.rigid() || tree_edge[k] {
                continue;
            }
            let gap = joint::frame_gap(&j.local[0], &j.local[1], size);
            let implied = path(j.body[0], j.body[1]);
            closed_form(&mut diags, j.item(0), implied, gap);
        }
        for cl in &clusters {
            let root = cl.members.iter().copied().find(|&m| bodies[m].ground);
            for &m in &cl.members {
                if Some(m) == root || !bodies[m].ground {
                    continue;
                }
                let at = cl.written.compose(&bodies[m].offset);
                let gap = joint::frame_gap(&at, &bodies[m].written, size);
                let mut implied = path(m, root.expect("grounded"));
                implied.push(Item::Ground(bodies[root.expect("grounded")].id));
                closed_form(&mut diags, Item::Ground(bodies[m].id), implied, gap);
            }
        }

        // relations
        let joint_link: Vec<(JointId, usize)> = joints
            .iter()
            .enumerate()
            .filter_map(|(k, j)| j.joint_id().map(|id| (id, k)))
            .collect();
        let jix = |id: JointId| {
            joint_link
                .binary_search_by_key(&id, |x| x.0)
                .ok()
                .map(|i| joint_link[i].1)
        };
        let mut rels: Vec<&Relation> = asm.relations.iter().filter(|r| !r.suppressed).collect();
        rels.sort_by_key(|r| r.id);
        let mut relations = Vec::new();
        for r in rels {
            let ok = (|| {
                let (l, f) = (jix(r.leader.joint)?, jix(r.follower.joint)?);
                let (la, fa) = r.kind.dims();
                let fine = joints[l].freedoms().contains(&r.leader.freedom)
                    && joints[f].freedoms().contains(&r.follower.freedom)
                    && joints[l].kind() != JointKind::Ball
                    && joints[f].kind() != JointKind::Ball
                    && r.leader.freedom.is_angle() == la
                    && r.follower.freedom.is_angle() == fa
                    && (l, r.leader.freedom) != (f, r.follower.freedom);
                fine.then_some((l, f))
            })();
            match ok {
                Some((l, f)) => relations.push(PRelation {
                    id: r.id,
                    kind: r.kind,
                    leader: (l, r.leader.freedom),
                    follower: (f, r.follower.freedom),
                    offset: r.offset,
                }),
                None => diags.push(Diagnostic::BadRelation { relation: r.id }),
            }
        }

        // pass 2: partitions over the cluster graph, ground cut out
        let nc = clusters.len();
        let mut puf = Uf::new(nc);
        let vert = |c: usize| if clusters[c].ground { None } else { Some(c) };
        let mut rigid_joints = Vec::new();
        let mut edge_list = Vec::new();
        for (k, j) in joints.iter().enumerate() {
            if j.rigid() {
                continue;
            }
            let (ca, cb) = (bodies[j.body[0]].cluster, bodies[j.body[1]].cluster);
            match (vert(ca), vert(cb)) {
                _ if ca == cb => rigid_joints.push(k),
                (None, None) => rigid_joints.push(k),
                (Some(a), Some(b)) => {
                    puf.union(a, b);
                    edge_list.push(k);
                }
                _ => edge_list.push(k),
            }
        }
        let jvert = |k: usize| {
            let j = &joints[k];
            vert(bodies[j.body[0]].cluster).or(vert(bodies[j.body[1]].cluster))
        };
        for r in &relations {
            if let (Some(a), Some(b)) = (jvert(r.leader.0), jvert(r.follower.0)) {
                puf.union(a, b);
            }
        }
        let mut part_of = vec![usize::MAX; nc];
        let mut partitions: Vec<Partition> = Vec::new();
        let mut slot = vec![usize::MAX; nc];
        fn touch(c: usize, slot: &mut [usize], parts: &mut Vec<Partition>, puf: &mut Uf) -> usize {
            let r = puf.find(c);
            if slot[r] == usize::MAX {
                slot[r] = parts.len();
                parts.push(Partition::default());
            }
            slot[r]
        }
        for &k in &edge_list {
            let c = jvert(k).expect("an edge has a free end");
            let p = touch(c, &mut slot, &mut partitions, &mut puf);
            partitions[p].edges.push(k);
        }
        for (ri, r) in relations.iter().enumerate() {
            if let Some(c) = jvert(r.leader.0).or(jvert(r.follower.0)) {
                let p = touch(c, &mut slot, &mut partitions, &mut puf);
                partitions[p].relations.push(ri);
            }
        }
        for c in 0..nc {
            let r = puf.find(c);
            if !clusters[c].ground && slot[r] != usize::MAX {
                part_of[c] = slot[r];
                partitions[slot[r]].clusters.push(c);
            }
        }
        // partitions in order of their least cluster, which is the order of least instance id
        let mut porder: Vec<usize> = (0..partitions.len()).collect();
        porder.sort_by_key(|&p| partitions[p].clusters.first().copied().unwrap_or(usize::MAX));
        let mut renum = vec![0; partitions.len()];
        for (new, &old) in porder.iter().enumerate() {
            renum[old] = new;
        }
        let mut sorted: Vec<Partition> = porder
            .iter()
            .map(|&p| std::mem::take(&mut partitions[p]))
            .collect();
        for p in &mut sorted {
            p.grounded = p.edges.iter().any(|&k| {
                let j = &joints[k];
                clusters[bodies[j.body[0]].cluster].ground || clusters[bodies[j.body[1]].cluster].ground
            });
        }
        for po in part_of.iter_mut().filter(|p| **p != usize::MAX) {
            *po = renum[*po];
        }

        Plan {
            bodies,
            of,
            reference,
            clusters,
            links: joints,
            joint_link,
            relations,
            partitions: sorted,
            part_of,
            rigid_joints,
            fastened_parent: parent,
            diagnostics: diags,
            subs: std::mem::take(subs),
            size,
        }
    }
}

/// A constraint already decided by others: a warning if it agrees, else a conflict naming it and
/// what implies it.
pub(crate) fn closed_form(diags: &mut Vec<Diagnostic>, item: Item, implied: Vec<Item>, gap: f64) {
    if gap <= SAME {
        diags.push(Diagnostic::Redundant {
            item,
            rows: 0,
            implied_by: implied,
        });
    } else {
        let mut items = implied;
        items.push(item);
        let n = items.len();
        let mut residuals = vec![0.0; n];
        residuals[n - 1] = gap;
        diags.push(Diagnostic::Conflict { items, residuals });
    }
}

/// The anchor pass: every joint's and loose constraint's two ends looked up by name, once.
/// Broken ones are reported and left out. References come back in their bodies' frames.
pub(crate) fn resolve_links<P: Parts>(
    asm: &Assembly,
    parts: &P,
    bodies: &[Body],
    of: &[Component],
    suppressed: &[InstanceId],
    subs: &BTreeMap<DocId, Sub>,
    diags: &mut Vec<Diagnostic>,
) -> Vec<PLink> {
    let find = |id: InstanceId| bodies.binary_search_by_key(&id, |b| b.id).ok();
    let mut links: Vec<PLink> = Vec::new();
    let ends_of = |path_a: &End,
                   path_b: &End,
                   joint: Option<JointId>,
                   constraint: Option<ConstraintId>,
                   want: Option<[&'static [RefKind]; 2]>,
                   diags: &mut Vec<Diagnostic>|
     -> Option<[(usize, Frame); 2]> {
        let mut ends = [(0usize, Frame::WORLD); 2];
        let mut ok = true;
        for (k, (side, end)) in [(Side::A, path_a), (Side::B, path_b)].into_iter().enumerate() {
            let anchor = match &end.anchor {
                AnchorRef::Named(n) => n.clone(),
                AnchorRef::Local(_) => String::new(),
            };
            match resolve(end, bodies, of, &find, parts, subs, suppressed) {
                Ok((b, r)) => {
                    let need = want.map_or(&[RefKind::Frame][..], |w| w[k]);
                    if !need.contains(&r.kind) {
                        diags.push(Diagnostic::WrongReference {
                            constraint,
                            joint,
                            side,
                            anchor,
                            want: need,
                            found: r.kind,
                        });
                        ok = false;
                    }
                    ends[k] = (b, r.frame);
                }
                Err(reason) => {
                    diags.push(Diagnostic::BrokenAnchor {
                        constraint,
                        joint,
                        side,
                        anchor,
                        reason,
                    });
                    ok = false;
                }
            }
        }
        if ok && ends[0].0 == ends[1].0 && path_a.path == path_b.path {
            diags.push(Diagnostic::SelfJoint { constraint, joint });
            ok = false;
        }
        ok.then_some(ends)
    };

    let mut jorder: Vec<usize> = (0..asm.joints.len()).collect();
    jorder.sort_by_key(|&i| (asm.joints[i].id, i));
    let mut last: Option<JointId> = None;
    for &ji in &jorder {
        let j = &asm.joints[ji];
        if last == Some(j.id) {
            diags.push(Diagnostic::DuplicateJoint { id: j.id });
            continue;
        }
        last = Some(j.id);
        if j.suppressed {
            continue;
        }
        let mut ok = true;
        for f in j.drive.iter().map(|d| d.0).chain(j.limits.iter().map(|l| l.0)) {
            if !j.kind.freedoms().contains(&f) || j.kind == JointKind::Ball {
                diags.push(Diagnostic::BadFreedom {
                    joint: j.id,
                    freedom: f,
                });
                ok = false;
            }
        }
        let Some(ends) = ends_of(&j.a, &j.b, Some(j.id), None, None, diags) else {
            continue;
        };
        if !ok {
            continue;
        }
        let la = joint::effective(ends[0].1, j.flip);
        let lb = ends[1].1;
        let prims = j
            .kind
            .bundle()
            .iter()
            .enumerate()
            .map(|(i, &kind)| {
                // pin-slot's line is B's x
                let b = if j.kind == JointKind::PinSlot && i == 0 {
                    Frame::from_zx(lb.origin, lb.x(), lb.y().get()).expect("orthonormal axes")
                } else {
                    lb
                };
                Prim {
                    id: ConstraintId::Joint {
                        joint: j.id,
                        part: JointPart::Prim(i as u8),
                    },
                    kind,
                    local: [la, b],
                }
            })
            .collect();
        links.push(PLink {
            id: LinkId::Joint(j.id),
            joint: Some(j.kind),
            body: [ends[0].0, ends[1].0],
            local: [la, lb],
            prims,
            drive: j.drive.clone(),
            limits: j.limits.clone(),
        });
    }
    let mut corder: Vec<usize> = (0..asm.constraints.len()).collect();
    corder.sort_by_key(|&i| (asm.constraints[i].id, i));
    let mut last: Option<u32> = None;
    for &ci in &corder {
        let c = &asm.constraints[ci];
        if last == Some(c.id) {
            diags.push(Diagnostic::DuplicateConstraint { id: c.id });
            continue;
        }
        last = Some(c.id);
        if c.suppressed {
            continue;
        }
        let id = ConstraintId::Loose(c.id);
        let Some(ends) = ends_of(&c.a, &c.b, None, Some(id), Some(c.kind.accepts()), diags) else {
            continue;
        };
        let la = joint::effective(ends[0].1, c.flip);
        let lb = ends[1].1;
        links.push(PLink {
            id: LinkId::Loose(c.id),
            joint: None,
            body: [ends[0].0, ends[1].0],
            local: [la, lb],
            prims: vec![Prim {
                id,
                kind: c.kind,
                local: [la, lb],
            }],
            drive: Vec::new(),
            limits: Vec::new(),
        });
    }
    links
}

/// A joint end's anchor: the body it moves with, and the anchor in that body's frame.
#[allow(clippy::too_many_arguments)]
fn resolve<P: Parts>(
    end: &End,
    _bodies: &[Body],
    of: &[Component],
    find: &impl Fn(InstanceId) -> Option<usize>,
    parts: &P,
    subs: &BTreeMap<DocId, Sub>,
    suppressed: &[InstanceId],
) -> Result<(usize, Reference), AnchorReason> {
    let Some(&first) = end.path.first() else {
        return Err(AnchorReason::BadPath { at: 0 });
    };
    let Some(b) = find(first) else {
        return Err(if suppressed.binary_search(&first).is_ok() {
            AnchorReason::Suppressed { at: 0 }
        } else {
            AnchorReason::NoInstance { at: 0 }
        });
    };
    let (part, inside) = match &of[b] {
        Component::Part(p) => {
            if end.path.len() > 1 {
                return Err(AnchorReason::BadPath { at: 1 });
            }
            (p.clone(), Frame::WORLD)
        }
        Component::Assembly(d) => {
            let Some(s) = subs.get(d) else {
                return Err(AnchorReason::NoDocument { doc: d.clone() });
            };
            let rest: Path = end.path[1..].to_vec();
            if rest.is_empty() {
                return Err(AnchorReason::BadPath { at: 1 });
            }
            match s.occ.get(&rest) {
                Some((p, f, _)) => (p.clone(), *f),
                None => {
                    return Err(AnchorReason::NoInstance {
                        at: end.path.len() - 1,
                    });
                }
            }
        }
    };
    let local = match &end.anchor {
        AnchorRef::Local(r) => *r,
        AnchorRef::Named(name) => {
            let Some(info) = parts.part(&part) else {
                return Err(AnchorReason::NoDocument {
                    doc: part.doc.clone(),
                });
            };
            match info.refs.get(name) {
                None => return Err(AnchorReason::NoAnchor),
                Some(Err(fault)) => return Err(AnchorReason::Part(fault.clone())),
                Some(Ok(r)) => *r,
            }
        }
    };
    Ok((
        b,
        Reference {
            kind: local.kind,
            frame: inside.compose(&local.frame),
        },
    ))
}
