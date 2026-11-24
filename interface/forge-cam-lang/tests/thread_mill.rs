//! Thread milling (Fusion parity): radial passes, spring passes, hand, starts and a tangent arc
//! lead, each certified against P ∖ Δ like the single pass.
mod common;
use common::dir;
use forge_cam::ops::PMove;

fn run(args: &str) -> forge_cam_lang::Cam {
    let src = std::fs::read_to_string(dir().join("block_m6_cam.fg"))
        .unwrap()
        .replace(
            "thread_mill(op1, f.m6, tool: t6)",
            &format!("thread_mill(op1, f.m6, tool: t6{args})"),
        );
    forge_cam_lang::run(&src, &dir())
}

/// The helix arcs (z moving) of the op, as (radius from the hole's axis, rising).
fn helix(cam: &forge_cam_lang::Cam) -> Vec<(f64, bool)> {
    assert!(
        cam.ok,
        "{:?} {:?}",
        cam.errors,
        cam.ops.iter().map(|o| (&o.name, &o.refusal)).collect::<Vec<_>>()
    );
    let o = cam.ops.iter().find(|o| o.name == "thr").unwrap();
    assert_eq!(o.status, "built", "{:?}", o.refusal);
    let mv = &o.paths[0].moves;
    let mut out = vec![];
    let mut z = f64::NAN;
    for m in mv {
        match m {
            PMove::Arc { to, c, .. } => {
                if to[2] != z && !z.is_nan() {
                    out.push((
                        ((to[0] - c[0]).powi(2) + (to[1] - c[1]).powi(2)).sqrt(),
                        to[2] > z,
                    ));
                }
                z = to[2];
            }
            PMove::Line { to, .. } | PMove::Rapid { to } => z = to[2],
            _ => {}
        }
    }
    out
}

#[test]
fn radial_and_spring_passes_left_hand_and_arc_leads() {
    let one = helix(&run(""));
    assert!(one.iter().all(|h| h.1), "right hand: up");
    let three = helix(&run(", passes: 3, spring: 1"));
    assert_eq!(three.len(), 4 * one.len());
    let rs: Vec<f64> = three.iter().map(|h| h.0).collect();
    let rmax = one[0].0;
    assert!(
        rs[0] < rs[one.len()] && rs[one.len()] < rs[2 * one.len()],
        "{rs:?}"
    );
    assert!((rs[2 * one.len()] - rmax).abs() < 1e-9 && (rs[3 * one.len()] - rmax).abs() < 1e-9);
    let left = helix(&run(", hand: \"left\""));
    assert!(left.iter().all(|h| !h.1), "left hand climbing: down");
    let arc = helix(&run(", lead: \"arc\""));
    assert_eq!(arc.len(), one.len());
    let two = helix(&run(", starts: 2"));
    assert!(!two.is_empty());
    for bad in [", passes: 0", ", hand: \"up\"", ", lead: \"zig\""] {
        let cam = run(bad);
        let o = cam.ops.iter().find(|o| o.name == "thr");
        assert!(
            !cam.errors.is_empty() || o.is_some_and(|o| o.status == "refused"),
            "{bad}"
        );
    }
    let cam = forge_cam_lang::run(
        &common::doc("top = clear(op1, tool: t1, from: 18, to: 12, passes: 2)\n"),
        &dir(),
    );
    assert!(
        cam.errors
            .iter()
            .any(|e| e.contains("clear does not take `passes:`")),
        "{:?}",
        cam.errors
    );
}
