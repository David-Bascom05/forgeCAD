//! Two-body cases: one `forge_bool::boolean` of two solids against the reference.
//!
//! The box-box family (the coverage worker's, branch claude/repo-review-mbdrlj-coverage,
//! verify/forge-exact/src/lib.rs): two axis boxes whose ends on each axis stand in one of Allen's
//! 13 interval relations, each end at ten times its rank, so 13³ patterns × three ops are every
//! consistent sign pattern of the family. And its tolerance band (examples/box_band.rs there):
//! every pattern with coincident ends, B's coincident ends on the chosen axes moved outward by
//! δ·LINEAR.

use crate::{Answer, Class, Judged, Kernel, Reference, dump_solid, exact, guarded, judge_body, rule};
use forge_bool::Op;
use forge_math::{Frame, Point2, Point3};
use forge_model::Combine;
use forge_ops::{Profile, extrude};
use forge_topo::Body;

/// One axis's relation: the ranks of A's interval ends and of B's in the merged order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rel {
    pub name: &'static str,
    pub a: [u8; 2],
    pub b: [u8; 2],
}

const fn rel(name: &'static str, a0: u8, a1: u8, b0: u8, b1: u8) -> Rel {
    Rel {
        name,
        a: [a0, a1],
        b: [b0, b1],
    }
}

/// Allen's relations of A to B, in the order the coverage model lists them.
pub const RELS: [Rel; 13] = [
    rel("before", 0, 1, 2, 3),
    rel("meets", 0, 1, 1, 2),
    rel("overlaps", 0, 2, 1, 3),
    rel("finishedBy", 0, 2, 1, 2),
    rel("contains", 0, 3, 1, 2),
    rel("starts", 0, 1, 0, 2),
    rel("equal", 0, 1, 0, 1),
    rel("startedBy", 0, 2, 0, 1),
    rel("during", 1, 2, 0, 3),
    rel("finishes", 1, 2, 0, 2),
    rel("overlappedBy", 1, 3, 0, 2),
    rel("metBy", 1, 2, 0, 1),
    rel("after", 2, 3, 0, 1),
];

pub const OPS: [Op; 3] = [Op::Union, Op::Subtract, Op::Intersect];

pub fn op_word(op: Op) -> &'static str {
    match op {
        Op::Union => "union",
        Op::Subtract => "subtract",
        Op::Intersect => "intersect",
    }
}

fn combine(op: Op) -> Combine {
    match op {
        Op::Union => Combine::Add,
        Op::Subtract => Combine::Cut,
        Op::Intersect => Combine::Keep,
    }
}

pub type Boxes = [[f64; 2]; 3];

/// A pattern: one relation per axis and the op.
#[derive(Clone, Copy, Debug)]
pub struct Pattern {
    pub rel: [Rel; 3],
    pub op: Op,
}

impl Pattern {
    pub fn all() -> Vec<Pattern> {
        let mut out = Vec::new();
        for op in OPS {
            for x in RELS {
                for y in RELS {
                    for z in RELS {
                        out.push(Pattern { rel: [x, y, z], op });
                    }
                }
            }
        }
        out
    }

    pub fn label(&self) -> String {
        format!(
            "{}/{}/{} {}",
            self.rel[0].name,
            self.rel[1].name,
            self.rel[2].name,
            op_word(self.op)
        )
    }

    /// A's box and B's, each end at `unit` times its rank.
    pub fn boxes(&self, unit: f64) -> (Boxes, Boxes) {
        let s = |r: [u8; 2]| [unit * r[0] as f64, unit * r[1] as f64];
        (self.rel.map(|r| s(r.a)), self.rel.map(|r| s(r.b)))
    }

    /// The band's boxes: B's ends equal to one of A's, on `axes`, moved outward by `delta`
    /// LINEARs; None if no end moved.
    pub fn banded(&self, unit: f64, axes: &[usize], delta: f64) -> Option<(Boxes, Boxes)> {
        let (a, mut b) = self.boxes(unit);
        let mut moved = false;
        for &k in axes {
            for (s, end) in b[k].iter_mut().enumerate() {
                if a[k].contains(end) {
                    *end += if s == 0 { -delta } else { delta } * forge_math::tol::LINEAR;
                    moved = true;
                }
            }
        }
        moved.then_some((a, b))
    }
}

/// An axis box as a solid: a rectangle on its bottom extruded up.
pub fn block(b: Boxes, feature: u64) -> Body {
    let profile = Profile {
        plane: Frame::WORLD.with_origin(Point3::new(b[0][0], b[1][0], b[2][0])),
        loops: vec![Profile::rectangle(
            Point2::ORIGIN,
            b[0][1] - b[0][0],
            b[1][1] - b[1][0],
            feature * 10,
        )],
    };
    extrude(&profile, b[2][1] - b[2][0], feature).expect("a box extrudes")
}

/// A two-body case.
pub struct Pair {
    pub name: String,
    pub a: Boxes,
    pub b: Boxes,
    pub op: Op,
}

impl Pair {
    /// The repro's Rust text.
    pub fn source(&self) -> String {
        format!(
            "pair({:?}, {:?}, Op::{:?})",
            self.a.map(|r| r.map(f64::to_bits)),
            self.b.map(|r| r.map(f64::to_bits)),
            self.op
        )
    }
}

/// The reference's input for A op B.
pub fn pair_input(a: &Body, b: &Body, op: Op) -> Result<String, String> {
    pair_input_rule(a, b, op, &rule())
}

/// `pair_input` under the named rule.
pub fn pair_input_rule(a: &Body, b: &Body, op: Op, rule: &str) -> Result<String, String> {
    Ok(format!(
        "{{\"linear\":{},\"rule\":\"{}\",\"solids\":[{},{}]}}",
        exact(forge_math::tol::LINEAR),
        rule,
        dump_solid(a, Combine::New)?,
        dump_solid(b, combine(op))?
    ))
}

/// The kernel's boolean, a body with no faces read as the empty result.
pub fn kernel_pair(a: &Body, b: &Body, op: Op) -> Kernel {
    match guarded(|| forge_bool::boolean(a, b, op, 3).map_err(|e| format!("{e:?}"))) {
        Ok(body) if body.face_ids().next().is_none() => Kernel::Refused("Empty (no faces)".into()),
        r => judge_body(r),
    }
}

/// The pair judged: its class and the reference's answer.
pub fn judge_pair(reference: &mut Reference, p: &Pair) -> Judged {
    let (a, b) = (block(p.a, 1), block(p.b, 2));
    let (class, answer) = match pair_input(&a, &b, p.op) {
        Err(e) => (Class::Scope(e), None),
        Ok(input) => {
            let ans: Answer = reference.ask(&input);
            (crate::classify(&ans, &kernel_pair(&a, &b, p.op)), Some(ans))
        }
    };
    Judged {
        name: p.name.clone(),
        path: "boolean",
        class,
        answer,
    }
}

/// Every box-box pattern at unit 10.
pub fn box_family() -> Vec<Pair> {
    Pattern::all()
        .into_iter()
        .map(|p| {
            let (a, b) = p.boxes(10.0);
            Pair {
                name: format!("boxes/{}", p.label()),
                a,
                b,
                op: p.op,
            }
        })
        .collect()
}

/// One band sweep: every pattern with a coincident end on `axes`, moved by `delta` LINEARs.
pub fn band(axes: &[usize], delta: f64) -> Vec<Pair> {
    let ax: String = axes.iter().map(|a| a.to_string()).collect();
    Pattern::all()
        .into_iter()
        .filter_map(|p| {
            let (a, b) = p.banded(10.0, axes, delta)?;
            Some(Pair {
                name: format!("band axes {ax} δ {delta}/{}", p.label()),
                a,
                b,
                op: p.op,
            })
        })
        .collect()
}
