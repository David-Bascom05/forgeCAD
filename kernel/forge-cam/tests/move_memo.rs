//! The certification's per-move memo (movememo.rs): every input a move's checks read is in its
//! key. Changing any one of them (the move, its phase, the tool, a part face, a stock face, a
//! fixture face, the tolerance, the clearance, the fixture's clearance, the column route) works
//! the move out anew, and every summary equals a certification with the memo off; renumbering the
//! faces takes the checks from the memo with the faces' new names. A stale summary (a Clear for
//! inputs it was not made for) is the bug this guards.
use forge_cam::movememo;
use forge_cam::ops::{self, CheckSummary, OpPath, PMove, Phase, Stock, ToolUse};
use forge_cam_geom::check::Face;
use forge_tooling::Library;
use std::sync::atomic::Ordering;

fn tool(id: &str, stickout: f64) -> ToolUse {
    let l = Library::builtin();
    let t = l.tool(id).unwrap().clone();
    let shape = t.shape(stickout, Some(l.holder("bt40-er32").unwrap())).unwrap();
    ToolUse {
        t: 1,
        tool: t,
        holder: Some("bt40-er32".into()),
        stickout,
        shape,
        probe: false,
    }
}

/// Over the part's top, along it, with one cut 0.05 into it (a gouge: a failing check, so the
/// memo's failing checks and their faces' names are exercised too), then a rapid up.
fn path(x: f64) -> OpPath {
    let l = |to: [f64; 3], phase| PMove::Line { to, f: 600.0, phase };
    OpPath {
        tool: 0,
        start: [x, 5.0, 30.0],
        moves: vec![
            l([x, 5.0, 12.0], Phase::Plunge),
            l([x + 20.0, 5.0, 12.0], Phase::Cut),
            l([x + 20.0, 15.0, 12.0], Phase::Cut),
            l([x + 20.0, 15.0, 9.95], Phase::Plunge),
            l([x + 25.0, 15.0, 9.95], Phase::Cut),
            PMove::Rapid {
                to: [x + 25.0, 15.0, 30.0],
            },
        ],
        rpm: 8000.0,
        coolant: forge_post::Coolant::Off,
    }
}

type Args = (
    OpPath,
    ToolUse,
    Vec<Face>,
    Vec<(usize, Vec<Face>)>,
    Vec<Face>,
    f64,
    f64,
    f64,
    bool,
);

fn certify(a: &Args) -> CheckSummary {
    ops::certify_with(&a.0, &a.1, &a.2, &a.3, &a.4, a.5, a.6, a.7, a.8)
}

fn fresh(a: &Args) -> CheckSummary {
    movememo::ON.store(false, Ordering::SeqCst);
    let s = certify(a);
    movememo::ON.store(true, Ordering::SeqCst);
    s
}

fn text(s: &CheckSummary) -> String {
    format!("{s:?}")
}

#[test]
fn every_input_class_is_in_a_moves_key() {
    let bx = |min: [f64; 3], max: [f64; 3]| Stock { min, max }.faces();
    let part = bx([0.0, 0.0, 0.0], [40.0, 30.0, 10.0]);
    let base: Args = (
        path(5.0),
        tool("flat-6", 30.0),
        part.clone(),
        vec![(0, bx([-2.0, -2.0, 0.0], [42.0, 32.0, 14.0]))],
        // a jaw beside the path, within its clearance (a fixture check that fails too)
        bx([0.0, -1.0, 0.0], [40.0, -0.2, 10.0]),
        2.5,
        0.01,
        2.0,
        false,
    );
    let s0 = certify(&base);
    assert!(!s0.clear, "the gouge fails: {s0:?}");
    assert_eq!(text(&s0), text(&fresh(&base)));
    // the same again: every move from the memo, the same summary
    let (w0, t0) = ops::move_memo_counts();
    assert_eq!(text(&certify(&base)), text(&s0));
    let (w1, t1) = ops::move_memo_counts();
    assert_eq!(w1, w0, "the same arguments worked out again");
    assert!(t1 > t0);

    let mut edits: Vec<(&str, Args)> = Vec::new();
    let mut e = base.clone();
    e.0 = path(5.000001);
    edits.push(("path", e));
    let mut e = base.clone();
    // the gouging cut as a plunge: the same geometry and faces, the same limits, another phase
    // (its checks name it)
    e.0.moves[4] = PMove::Line {
        to: [30.0, 15.0, 9.95],
        f: 600.0,
        phase: Phase::Plunge,
    };
    edits.push(("phase", e));
    let mut e = base.clone();
    // and as a rapid: another rule and limits
    e.0.moves[4] = PMove::Rapid {
        to: [30.0, 15.0, 9.95],
    };
    edits.push(("phase (rapid)", e));
    let mut e = base.clone();
    e.1 = tool("flat-6", 30.5);
    edits.push(("tool stickout", e));
    let mut e = base.clone();
    e.1 = tool("flat-10", 30.0);
    edits.push(("tool", e));
    let mut e = base.clone();
    e.1.probe = true;
    edits.push(("probe", e));
    let mut e = base.clone();
    e.2 = bx([0.0, 0.0, 0.0], [40.0, 30.0, 10.02]);
    edits.push(("part face", e));
    // the top face (the one the moves are near; a face no move is near is read by none)
    let top = (0..part.len())
        .max_by(|&a, &b| part[a].bbox[2].lo.total_cmp(&part[b].bbox[2].lo))
        .unwrap();
    let mut e = base.clone();
    e.2[top].mu = e.2[top].mu * 2.0 + 1e-9;
    edits.push(("part face's mu", e));
    let mut e = base.clone();
    e.3 = vec![(0, bx([-2.0, -2.0, 0.0], [42.0, 32.0, 13.0]))];
    edits.push(("stock", e));
    let mut e = base.clone();
    e.4 = bx([0.0, -1.0, 0.0], [40.0, -0.3, 10.0]);
    edits.push(("fixture", e));
    let mut e = base.clone();
    e.5 = 2.0;
    edits.push(("fixture clearance", e));
    let mut e = base.clone();
    e.6 = 0.02;
    edits.push(("tolerance", e));
    let mut e = base.clone();
    e.7 = 2.5;
    edits.push(("clearance", e));
    let mut e = base.clone();
    e.8 = true;
    edits.push(("column", e));
    for (what, e) in &edits {
        let (w, _) = ops::move_memo_counts();
        let s = certify(e);
        assert!(ops::move_memo_counts().0 > w, "{what}: no move worked out anew");
        assert_eq!(text(&s), text(&fresh(e)), "{what}: not the fresh summary");
    }
    // the faces renumbered: every move from the memo, the checks naming the new numbers, as a
    // fresh certification does
    let mut e = base.clone();
    for f in &mut e.2 {
        f.id += 100;
    }
    let (w, _) = ops::move_memo_counts();
    let s = certify(&e);
    assert_eq!(ops::move_memo_counts().0, w, "renumbered faces worked out again");
    assert_eq!(text(&s), text(&fresh(&e)));
    assert!(
        s.checks.iter().any(|c| c.other.starts_with("part face 1")) && text(&s) != text(&s0),
        "the new names: {s:?}"
    );
    // and the base again, unchanged
    assert_eq!(text(&certify(&base)), text(&s0));
}
