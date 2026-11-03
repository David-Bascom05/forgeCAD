//! Assemblies: instances of parts and subassemblies placed by joints between named anchor frames.
//!
//! The solve is structure first, numerics last (research: forgecad-gate/assemblies/prior-art.md):
//! 1. anchors looked up by name, one pass over the joints;
//! 2. rigid clusters by union-find over fastened joints;
//! 3. partitions, the connected parts of the cluster graph with ground cut out, solved in
//!    parallel and merged in id order;
//! 4. biconnected blocks: tree edges placed in closed form from the written poses and the joint
//!    values, and only loop blocks solved by minimum-norm Newton with column-pivoted QR.
//!
//! The solve starts from the written poses every time, so a document fully determines its result.
//! Every order is by instance and joint id; trig goes through `forge_math::dmath`; no FMA.

pub mod bom;
pub mod check;
pub mod diag;
mod joint;
pub mod model;
mod plan;
mod qr;
mod solve;
pub mod step;

pub use diag::{AnchorReason, Diagnostic, Fix, Intent, Item, Severity};
pub use model::*;
pub use solve::{BlockReport, ConstraintReport, JointReport, Occurrence, Options, Solution, Stats};

use std::collections::BTreeMap;

/// Solves an assembly once.
pub fn solve<P: Parts>(asm: &Assembly, parts: &P, opts: &Options) -> Solution {
    Solver::new(asm.clone(), parts, *opts).solution
}

/// The anchor pass alone: every joint end looked up by name, and nothing solved.
pub fn check_anchors<P: Parts>(asm: &Assembly, parts: &P) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let mut subs = BTreeMap::new();
    let (bodies, of, _, suppressed) = plan::level(asm, parts, &mut subs, &mut Vec::new(), &mut diags);
    diags.clear();
    plan::resolve_links(asm, parts, &bodies, &of, &suppressed, &subs, &mut diags);
    diags.retain(|d| matches!(d, Diagnostic::BrokenAnchor { .. }));
    diags
}

/// A live assembly: solved once, then re-solved in part as joint values change.
pub struct Solver<P: Parts> {
    parts: P,
    asm: Assembly,
    plan: plan::Plan,
    memo: solve::Memo,
    opts: Options,
    solution: Solution,
}

impl<P: Parts> Solver<P> {
    pub fn new(asm: Assembly, parts: P, opts: Options) -> Self {
        let plan = plan::Plan::new(&asm, &parts, &mut BTreeMap::new(), &mut Vec::new());
        let mut s = Solver {
            parts,
            asm,
            plan,
            memo: solve::Memo::default(),
            opts,
            solution: Solution::default(),
        };
        s.run(None);
        s
    }

    pub fn solution(&self) -> &Solution {
        &self.solution
    }

    pub fn assembly(&self) -> &Assembly {
        &self.asm
    }

    pub fn parts(&self) -> &P {
        &self.parts
    }

    fn run(&mut self, dirty: Option<&[bool]>) {
        let mut sol = match dirty {
            Some(d) => solve::run(&self.plan, &self.opts, Some((&mut self.memo, d))),
            None => {
                let all = vec![true; self.plan.partitions.len()];
                solve::run(&self.plan, &self.opts, Some((&mut self.memo, &all)))
            }
        };
        solve::hashes(&mut sol, &self.parts);
        self.solution = sol;
    }

    fn link_of(&self, id: JointId) -> Option<usize> {
        let jl = &self.plan.joint_link;
        jl.binary_search_by_key(&id, |x| x.0).ok().map(|i| jl[i].1)
    }

    /// Partitions a joint's change can reach.
    fn partition_of(&self, id: JointId) -> Option<usize> {
        let k = self.link_of(id)?;
        let j = &self.plan.links[k];
        j.body
            .iter()
            .map(|&b| self.plan.part_of[self.plan.bodies[b].cluster])
            .find(|&p| p != usize::MAX)
    }

    /// Sets one joint value (a param driving motion). Only that joint's partition is re-solved,
    /// and in it only the blocks whose inputs changed.
    pub fn set_drive(&mut self, id: JointId, f: Freedom, v: f64) -> &Solution {
        if let Some(j) = self.asm.joints.iter_mut().find(|j| j.id == id) {
            match j.drive.iter_mut().find(|d| d.0 == f) {
                Some(d) => d.1 = v,
                None => j.drive.push((f, v)),
            }
        }
        let Some(k) = self.link_of(id) else {
            return self.replan();
        };
        let had = self.plan.links[k].drive.iter().any(|d| d.0 == f);
        if !had || !self.plan.links[k].freedoms().contains(&f) {
            return self.replan();
        }
        for d in self.plan.links[k].drive.iter_mut().filter(|d| d.0 == f) {
            d.1 = v;
        }
        let mut dirty = vec![false; self.plan.partitions.len()];
        if let Some(p) = self.partition_of(id) {
            dirty[p] = true;
        }
        // relations reach across partitions only through ground: the plan unions them otherwise
        self.run(Some(&dirty));
        &self.solution
    }

    /// Replaces a joint (same id). Drives and limits only: its partition is re-solved; anything
    /// else re-plans the level.
    pub fn edit_joint(&mut self, joint: Joint) -> &Solution {
        let Some(old) = self.asm.joints.iter_mut().find(|j| j.id == joint.id) else {
            self.asm.joints.push(joint);
            return self.replan();
        };
        let same_shape = old.kind == joint.kind
            && old.a == joint.a
            && old.b == joint.b
            && old.flip == joint.flip
            && old.suppressed == joint.suppressed
            && old.drive.iter().map(|d| d.0).eq(joint.drive.iter().map(|d| d.0));
        *old = joint.clone();
        let k = self.link_of(joint.id);
        match k {
            Some(k) if same_shape => {
                self.plan.links[k].drive = joint.drive.clone();
                self.plan.links[k].limits = joint.limits.clone();
                let mut dirty = vec![false; self.plan.partitions.len()];
                if let Some(p) = self.partition_of(joint.id) {
                    dirty[p] = true;
                }
                self.run(Some(&dirty));
                &self.solution
            }
            _ => self.replan(),
        }
    }

    /// Replaces the whole document (a text edit) and re-plans.
    pub fn set_assembly(&mut self, asm: Assembly) -> &Solution {
        self.asm = asm;
        self.replan()
    }

    fn replan(&mut self) -> &Solution {
        let mut subs = std::mem::take(&mut self.plan.subs);
        self.plan = plan::Plan::new(&self.asm, &self.parts, &mut subs, &mut Vec::new());
        self.memo = solve::Memo::default();
        self.run(None);
        &self.solution
    }
}
