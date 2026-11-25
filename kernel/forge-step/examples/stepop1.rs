//! One boolean on a solid read from a small STEP file, for the speed gate's instruction count:
//! `stepop1 <case>`. `stepop1 noop` does nothing (the process alone, which the gate takes off).
//! The gate's other parts are made by the kernel itself, so a change to the tracer's spline paths
//! (a file's NURBS faces) went unmeasured: a change that traced on where it used to refuse took
//! a refusal on a real file from 0.2 s to 30 s and the gate saw nothing (the Haas sweep,
//! KERNEL-STRESS, 2026-11-25). Each case is the sweep's quarter box (`box_sub`, then `box_int`)
//! on a solid out of the Haas models, the file read and the two ops counted; a refusal counts
//! as what it cost.
//!   k018-box   ST-28L #645206: a block with a ruled NURBS face (23 KB)
//!   ring-box   HA5C-T #51342: a ring of 12 torus patches, an O-ring's groove (25 KB)
use forge_bool::{Op, boolean};
use forge_math::{Frame, Point2, Point3, UnitVec3};

fn main() {
    let case = std::env::args().nth(1).unwrap_or_default();
    let text: &[u8] = match case.as_str() {
        "noop" => return,
        "k018-box" => include_bytes!("../tests/data/haas/nurbs_block_k018.step"),
        "ring-box" => include_bytes!("../tests/data/haas/torus_ring_12faces.step"),
        c => panic!("stepop1: no case {c:?} (k018-box, ring-box, noop)"),
    };
    let imp = forge_step::import(text, &forge_step::Limits::default()).expect("the file reads");
    let b = &imp.solids.first().expect("a solid").body;
    let bx = b.faces_box();
    let (c, m) = (bx.center(), 0.1 * bx.diagonal() + 1.0);
    let (lo, hi) = (
        Point3::new(c.x, bx.y.lo - m, c.z),
        Point3::new(bx.x.hi + m, bx.y.hi + m, bx.z.hi + m),
    );
    let p = forge_ops::Profile {
        plane: Frame::from_z(lo, UnitVec3::Z),
        loops: vec![forge_ops::Profile::rectangle(
            Point2::ORIGIN,
            hi.x - lo.x,
            hi.y - lo.y,
            8000,
        )],
    };
    let tool = forge_ops::extrude(&p, hi.z - lo.z, 8010).expect("the box");
    for op in [Op::Subtract, Op::Intersect] {
        // an answer or a refusal: either is counted
        std::hint::black_box(boolean(b, &tool, op, 9000).is_ok());
    }
}
