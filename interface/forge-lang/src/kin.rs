//! An assembly's kinematic structure as solved (`AsmOut::kin`, with `Options::kin`): its
//! bodies, joints, loose constraints and relations by name, each joint end's frame in the
//! world. What an export to a robot description reads (forge-urdf).

use forge_asm as fa;
use forge_asm::{ConstraintKind, Freedom, JointKind};
use forge_math::Frame;
use std::collections::BTreeMap;

/// An assembly's kinematic structure as solved: what the export reads (`AsmOut::kin`).
#[derive(Clone, Debug, Default)]
pub struct Kin {
    /// The top-level instances not suppressed, in document order.
    pub bodies: Vec<KinBody>,
    /// Joints and loose constraints not suppressed, in document order.
    pub links: Vec<KinLink>,
    pub relations: Vec<KinRelation>,
    /// Each of `AsmOut::occurrences`' top-level body (an index into `bodies`).
    pub occ_body: Vec<Option<usize>>,
}

#[derive(Clone, Debug)]
pub struct KinBody {
    pub name: String,
    pub ground: bool,
    /// Its solved pose.
    pub pose: Frame,
}

#[derive(Clone, Debug)]
pub struct KinLink {
    pub id: String,
    /// The joint's kind; None for a loose constraint (`constraint` names its kind).
    pub kind: Option<JointKind>,
    pub constraint: Option<ConstraintKind>,
    /// Each end's body (A, B), None where its instance is not placed.
    pub ends: [Option<usize>; 2],
    /// Each end's anchor frame in the world as solved: A's flipped when the joint says so.
    pub at: [Option<Frame>; 2],
    /// The solved values (mm, rad), and the limits as written.
    pub values: Vec<(Freedom, f64)>,
    pub limits: Vec<(Freedom, f64, f64)>,
}

impl KinLink {
    /// Rigid: a fastened joint or a lock constraint.
    pub fn rigid(&self) -> bool {
        self.kind == Some(JointKind::Fastened) || self.constraint == Some(ConstraintKind::Lock)
    }
}

#[derive(Clone, Debug)]
pub struct KinRelation {
    pub id: String,
    pub kind: &'static str,
    /// d follower / d leader, in forge's units (mm, rad).
    pub slope: f64,
    pub leader: (String, Freedom),
    pub follower: (String, Freedom),
    pub offset: Option<f64>,
}

type Namer<'a> = &'a dyn Fn(u64) -> Option<String>;

/// The structure of a solved assembly level, names given by id.
pub fn kin_of<P: fa::Parts>(
    asm: &fa::Assembly,
    sol: &fa::Solution,
    parts: &P,
    inst_name: Namer,
    link_name: Namer,
    rel_name: Namer,
) -> Kin {
    let mut kin = Kin::default();
    let mut index: BTreeMap<fa::InstanceId, usize> = BTreeMap::new();
    for inst in asm.instances.iter().filter(|i| !i.suppressed) {
        if index.contains_key(&inst.id) {
            continue;
        }
        index.insert(inst.id, kin.bodies.len());
        kin.bodies.push(KinBody {
            name: inst_name(inst.id.0).unwrap_or_else(|| format!("i{}", inst.id.0)),
            ground: inst.ground,
            pose: sol.pose(inst.id).unwrap_or(inst.pose),
        });
    }
    let world = |e: &fa::End| -> Option<Frame> {
        let local = |info: Option<&fa::PartInfo>| match &e.anchor {
            fa::AnchorRef::Local(r) => Some(r.frame),
            fa::AnchorRef::Named(n) => info?.refs.get(n)?.as_ref().ok().map(|r| r.frame),
        };
        if let Some(o) = sol.occurrences.iter().find(|o| o.path == e.path) {
            return Some(o.pose.compose(&local(parts.part(&o.part))?));
        }
        match (&e.anchor, e.path.as_slice()) {
            (fa::AnchorRef::Local(r), [only]) => Some(sol.pose(*only)?.compose(&r.frame)),
            _ => None,
        }
    };
    let body = |e: &fa::End| e.path.first().and_then(|i| index.get(i).copied());
    for j in asm.joints.iter().filter(|j| !j.suppressed) {
        let a = world(&j.a).map(|f| if j.flip { f.flipped() } else { f });
        kin.links.push(KinLink {
            id: link_name(j.id.0).unwrap_or_else(|| format!("j{}", j.id.0)),
            kind: Some(j.kind),
            constraint: None,
            ends: [body(&j.a), body(&j.b)],
            at: [a, world(&j.b)],
            values: sol
                .joints
                .iter()
                .find(|r| r.joint == j.id)
                .map(|r| r.values.clone())
                .unwrap_or_default(),
            limits: j.limits.clone(),
        });
    }
    for c in asm.constraints.iter().filter(|c| !c.suppressed) {
        let a = world(&c.a).map(|f| if c.flip { f.flipped() } else { f });
        kin.links.push(KinLink {
            id: link_name(c.id).unwrap_or_else(|| format!("c{}", c.id)),
            kind: None,
            constraint: Some(c.kind),
            ends: [body(&c.a), body(&c.b)],
            at: [a, world(&c.b)],
            values: vec![],
            limits: vec![],
        });
    }
    for r in asm.relations.iter().filter(|r| !r.suppressed) {
        let joint = |v: &fa::ValueRef| link_name(v.joint.0).unwrap_or_else(|| format!("j{}", v.joint.0));
        kin.relations.push(KinRelation {
            id: rel_name(r.id.0).unwrap_or_else(|| format!("r{}", r.id.0)),
            kind: r.kind.name(),
            slope: r.kind.slope(),
            leader: (joint(&r.leader), r.leader.freedom),
            follower: (joint(&r.follower), r.follower.freedom),
            offset: r.offset,
        });
    }
    kin.occ_body = sol
        .occurrences
        .iter()
        .map(|o| o.path.first().and_then(|i| index.get(i).copied()))
        .collect();
    kin
}
