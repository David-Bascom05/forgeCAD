//! The machine's homing (referencing) as motion: each joint to its reference end of travel, in
//! the order the maker publishes (`[[machine.homing]]`), else a generic reference return that
//! says it is one. The segments are ordinary rapids of the timeline, so the collision and removal
//! checks run on them like any other move.
//!
//! - A joint goes to its `ref` end (the travel's + end for `ref = "plus"`, − for `"minus"`).
//!   Without a stated `ref` the end is unknown: the joint goes to its `home` value, and the
//!   reply says the direction is not checked (`joint.N.ref`).
//! - Published order: the record's groups in turn, each group's joints together; the joints it
//!   does not name afterwards one at a time, in the record's joint order, which is NOT the
//!   maker's (only that they come after): reported not checked (`homing.rest`).
//! - No published order: every joint together (a generic reference return), reported not
//!   checked (`homing`).
//!
//! The search speed, the switch or hard-stop approach and the back-off to the index are not
//! modelled: each move is a rapid to the end, so its time is a lower bound only if the rapid
//! rate bounds the homing speed (it does on every control we know of, but is not claimed).
use crate::def::JointType;
use crate::sim::{Segment, Setup, Timeline, Unchecked};

#[derive(Clone, Debug, PartialEq)]
pub struct Homing {
    /// The joint groups in the order they moved (indexes into def.joints).
    pub steps: Vec<Vec<usize>>,
    /// The order is the maker's (`[[machine.homing]]`), not generic.
    pub published: bool,
    pub segments: Vec<Segment>,
    /// The machine after homing.
    pub end: Vec<f64>,
    /// What this homing could not check.
    pub unchecked: Vec<Unchecked>,
}

/// Where a joint's reference is: its `ref` end of travel, else its `home` value (None: direction
/// not stated).
fn reference_of(s: &Setup, j: usize) -> (f64, bool) {
    let jt = &s.def.joints[j];
    match (jt.reference, jt.range) {
        (Some(r), Some([lo, hi])) => (if r > 0 { hi } else { lo }, true),
        _ => (jt.home, false),
    }
}

/// Homing from joint values `from`, at program time `t0`, as timeline segments (line 0).
/// A machine homes because its position is unknown (power-on, after an emergency stop): unless
/// the caller knows `from` (`from_known`: homing after a program that ended at a known pose),
/// the sweep from it is reported not checked (`homing.start`).
pub fn home(s: &Setup, from: &[f64], from_known: bool, t0: f64) -> Homing {
    let def = s.def;
    let n = def.joints.len();
    let mut steps: Vec<Vec<usize>> = Vec::new();
    let mut unchecked: Vec<Unchecked> = Vec::new();
    let published = def.homing.is_some();
    if !from_known {
        unchecked.push(Unchecked {
            path: "homing.start".into(),
            why: "unknown",
            line: 0,
            check: "the machine's position before homing (swept from the given pose; the machine homes from wherever it is)",
        });
    }
    match &def.homing {
        Some(order) => {
            for g in order {
                steps.push(g.iter().filter_map(|nm| def.joint_index(nm)).collect());
            }
            let named: Vec<usize> = steps.iter().flatten().copied().collect();
            let rest: Vec<usize> = (0..n).filter(|j| !named.contains(j)).collect();
            if !rest.is_empty() {
                unchecked.push(Unchecked {
                    path: "homing.rest".into(),
                    why: "unknown",
                    line: 0,
                    check: "the order of the joints the published homing does not name",
                });
            }
            steps.extend(rest.into_iter().map(|j| vec![j]));
        }
        None => {
            steps.push((0..n).collect());
            unchecked.push(Unchecked {
                path: "homing".into(),
                why: "unknown",
                line: 0,
                check: "the homing order (a generic reference return: every joint together)",
            });
        }
    }
    let mut tl = Timeline {
        segments: Vec::new(),
        marks: Vec::new(),
        violations: Vec::new(),
        t_min: 0.0,
        t_stop: 0.0,
        joints: def.joints.iter().map(|j| j.name.clone()).collect(),
        accel_missing: false,
        after_skip: Vec::new(),
        skip_unmodelled: Vec::new(),
    };
    let mut q = from.to_vec();
    let mut t = t0;
    for g in &steps {
        let mut q1 = q.clone();
        for &j in g {
            let (r, known) = reference_of(s, j);
            if !known {
                unchecked.push(Unchecked {
                    path: format!("joint.{}.ref", def.joints[j].name),
                    why: "unknown",
                    line: 0,
                    check: "the homing direction (the joint goes to its home value)",
                });
            }
            // a cyclic rotary homes to its reference angle, not past its range
            q1[j] = if def.joints[j].ty == JointType::Rotary && def.joints[j].cyclic {
                def.joints[j].home
            } else {
                r
            };
        }
        for seg in crate::sim::rapid(s, 0, &q, &q1, t, s.start_tool, None, &mut tl) {
            t += seg.t_min;
            tl.segments.push(seg);
        }
        q = q1;
    }
    Homing {
        steps,
        published,
        segments: tl.segments,
        end: q,
        unchecked,
    }
}
