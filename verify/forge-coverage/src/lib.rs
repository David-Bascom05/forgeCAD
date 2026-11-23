//! The boolean's coverage on flat faces (research/kernel-audit/coverage.md).
//!
//! The box-box family: two axis-aligned boxes. On each axis the sign pattern of the endpoint
//! comparisons the boolean evaluates is one of Allen's 13 interval relations, so 13^3 patterns and
//! three ops are every consistent sign pattern of the family (`Rel::ALL`, in the order the Lean model
//! `Coverage.Allen` lists them). A pattern is realised by putting each endpoint at ten times its rank.
//! `exact` is the regularised boolean of the pattern computed on its cells; `kernel` runs
//! `forge_bool::boolean` on the realisation; both give an `Outcome`.
pub mod draft;
pub mod tilt;

use forge_bool::{BoolError, Op};
use forge_math::{Frame, Point2, Point3};
use forge_ops::{Profile, extrude};
use forge_topo::Body;

/// One axis's sign pattern: the ranks of A's interval ends and B's, in the merged order.
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

impl Rel {
    /// Allen's relations of A to B, as `Coverage.Allen.all` lists them.
    pub const ALL: [Rel; 13] = [
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

    pub fn by_name(name: &str) -> Option<Rel> {
        Rel::ALL.into_iter().find(|r| r.name == name)
    }
}

/// A pattern: one relation per axis, and the op.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pattern {
    pub rel: [Rel; 3],
    pub op: Op,
}

impl Pattern {
    /// "x/y/z op", as the Lean model and coverage.md write it.
    pub fn label(&self) -> String {
        format!(
            "{}/{}/{} {}",
            self.rel[0].name,
            self.rel[1].name,
            self.rel[2].name,
            op_name(self.op)
        )
    }

    pub fn all() -> Vec<Pattern> {
        let mut out = Vec::new();
        for op in [Op::Union, Op::Subtract, Op::Intersect] {
            for x in Rel::ALL {
                for y in Rel::ALL {
                    for z in Rel::ALL {
                        out.push(Pattern { rel: [x, y, z], op });
                    }
                }
            }
        }
        out
    }

    /// The boxes realising the pattern, scaled by `unit` (each end at unit times its rank).
    pub fn boxes(&self, unit: f64) -> ([[f64; 2]; 3], [[f64; 2]; 3]) {
        let s = |r: [u8; 2]| [unit * r[0] as f64, unit * r[1] as f64];
        (self.rel.map(|r| s(r.a)), self.rel.map(|r| s(r.b)))
    }
}

pub fn op_name(op: Op) -> &'static str {
    match op {
        Op::Union => "union",
        Op::Subtract => "subtract",
        Op::Intersect => "intersect",
    }
}

/// An axis-aligned box as a solid: a rectangle on z = z0 extruded up.
pub fn block(b: [[f64; 2]; 3], feature: u64) -> Body {
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

/// What a boolean came to, in the words the scoreboard uses.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// A valid body of this volume.
    Ok(f64),
    /// Nothing left.
    Empty,
    /// A ZeroThickness refusal.
    ZeroThickness,
    /// A body `forge_topo::check` faults, with its first problem's kind.
    Invalid(String),
    /// Lost its way (after the retry ladder), with the kind.
    Internal(String),
    Unsupported(String),
    Panic(String),
}

impl Outcome {
    pub fn word(&self) -> String {
        match self {
            Outcome::Ok(v) => format!("ok {v:.0}"),
            Outcome::Empty => "empty".into(),
            Outcome::ZeroThickness => "zero-thickness".into(),
            Outcome::Invalid(k) => format!("invalid {k}"),
            Outcome::Internal(k) => format!("internal {k}"),
            Outcome::Unsupported(k) => format!("unsupported {k}"),
            Outcome::Panic(m) => format!("panic {m}"),
        }
    }

    /// The same outcome: volumes within 1e-6 relative.
    pub fn agrees(&self, other: &Outcome) -> bool {
        match (self, other) {
            (Outcome::Ok(a), Outcome::Ok(b)) => (a - b).abs() <= 1e-6 * a.abs().max(b.abs()).max(1.0),
            (a, b) => a == b,
        }
    }
}

fn kind(d: String) -> String {
    d.split(|c: char| !c.is_alphanumeric())
        .next()
        .unwrap_or("")
        .to_string()
}

/// The kernel on the pattern's realisation.
pub fn kernel(p: &Pattern, unit: f64) -> Outcome {
    let (ba, bb) = p.boxes(unit);
    kernel_on(block(ba, 1), block(bb, 2), p.op)
}

/// `forge_bool::boolean` on two bodies, as an outcome.
pub fn kernel_on(a: Body, b: Body, op: Op) -> Outcome {
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        forge_bool::boolean(&a, &b, op, 3)
    }));
    match r {
        Err(e) => Outcome::Panic(
            e.downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| e.downcast_ref::<String>().cloned())
                .unwrap_or_default()
                .chars()
                .take(60)
                .collect(),
        ),
        Ok(Err(BoolError::ZeroThickness { .. })) => Outcome::ZeroThickness,
        Ok(Err(BoolError::Internal(l))) => Outcome::Internal(kind(format!("{l:?}"))),
        Ok(Err(BoolError::Unsupported(u))) => Outcome::Unsupported(kind(format!("{u:?}"))),
        Ok(Ok(body)) => {
            if body.face_ids().next().is_none() {
                return Outcome::Empty;
            }
            let problems = forge_topo::check(&body);
            if let Some(pr) = problems.first() {
                return Outcome::Invalid(kind(format!("{pr:?}")));
            }
            let q = forge_mesh::Quality {
                chord: 0.02,
                max_angle: 0.5,
            };
            match forge_mesh::tessellate(&body, &q) {
                Ok(m) => Outcome::Ok(m.volume()),
                Err(e) => Outcome::Invalid(format!("mesh {}", kind(format!("{e:?}")))),
            }
        }
    }
}

/// The exact answer: the op decided on the pattern's open 3-cells (rank space, 3 cells an axis), the
/// regularised result their closure. Non-manifold (an edge or a vertex the kept cells meet only
/// diagonally at) is ZeroThickness, as research/kernel-audit/README.md 2.1 specifies.
pub fn exact(p: &Pattern, unit: f64) -> Outcome {
    let (a, b) = p.boxes(unit);
    exact_boxes(a, b, p.op)
}

/// The exact answer for any two boxes, on the cells their coordinates cut space into.
pub fn exact_boxes(a: [[f64; 2]; 3], b: [[f64; 2]; 3], op: Op) -> Outcome {
    let coords: Vec<Vec<f64>> = (0..3)
        .map(|k| {
            let mut c = vec![a[k][0], a[k][1], b[k][0], b[k][1]];
            c.sort_by(f64::total_cmp);
            c.dedup();
            c
        })
        .collect();
    let n: Vec<i32> = coords.iter().map(|c| c.len() as i32 - 1).collect();
    let mid = |k: usize, i: i32| 0.5 * (coords[k][i as usize] + coords[k][i as usize + 1]);
    let inside =
        |bx: &[[f64; 2]; 3], c: [i32; 3]| (0..3).all(|k| bx[k][0] < mid(k, c[k]) && mid(k, c[k]) < bx[k][1]);
    let kept = |c: [i32; 3]| -> bool {
        if (0..3).any(|k| c[k] < 0 || c[k] >= n[k]) {
            return false;
        }
        let (ia, ib) = (inside(&a, c), inside(&b, c));
        match op {
            Op::Union => ia || ib,
            Op::Subtract => ia && !ib,
            Op::Intersect => ia && ib,
        }
    };
    let width = |ax: usize, x: i32| coords[ax][x as usize + 1] - coords[ax][x as usize];
    let mut volume = 0.0;
    for i in 0..n[0] {
        for j in 0..n[1] {
            for k in 0..n[2] {
                if kept([i, j, k]) {
                    volume += width(0, i) * width(1, j) * width(2, k);
                }
            }
        }
    }
    if volume == 0.0 {
        return Outcome::Empty;
    }
    // every grid vertex: its 8 octants; an edge's 4 quadrants are within them
    for i in 0..=n[0] {
        for j in 0..=n[1] {
            for k in 0..=n[2] {
                let oct = |d: usize| {
                    kept([
                        i - 1 + (d & 1) as i32,
                        j - 1 + ((d >> 1) & 1) as i32,
                        k - 1 + ((d >> 2) & 1) as i32,
                    ])
                };
                if !manifold_octants(oct) {
                    return Outcome::ZeroThickness;
                }
            }
        }
    }
    Outcome::Ok(volume)
}

/// Whether the kept octants about a vertex (bit d of the octant index: +x, +y, +z) leave its link a
/// single circle or none: the kept ones and the others each face-connected.
pub fn manifold_octants(kept: impl Fn(usize) -> bool) -> bool {
    let set: Vec<bool> = (0..8).map(&kept).collect();
    let connected = |want: bool| {
        let members: Vec<usize> = (0..8).filter(|&d| set[d] == want).collect();
        let Some(&first) = members.first() else {
            return true;
        };
        let mut seen = vec![first];
        let mut i = 0;
        while i < seen.len() {
            let d = seen[i];
            for bit in [1, 2, 4] {
                let n = d ^ bit;
                if set[n] == want && !seen.contains(&n) {
                    seen.push(n);
                }
            }
            i += 1;
        }
        seen.len() == members.len()
    };
    connected(true) && connected(false)
}

/// A box's extents: [min, max] on each axis.
pub type Extents = [[f64; 2]; 3];

/// The pattern's boxes with B's ends that coincide with one of A's moved by `delta` x LINEAR on
/// `axes` (outwards on both of B's ends), as examples/box_band moves them; None if none moved.
pub fn band_boxes(p: &Pattern, unit: f64, delta: f64, axes: &[usize]) -> Option<(Extents, Extents)> {
    let (a, mut b) = p.boxes(unit);
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
