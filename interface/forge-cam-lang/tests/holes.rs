//! The drill plan against the hole's floor (examples/machined/blind_holes.fg): a blind hole
//! with a flat floor (a counterbored hole's, as the manifold's) was drilled with the drill's
//! shoulder at the floor's depth, its point through the floor: a gouge. The point now stops on
//! the floor, and the ring it leaves is said.
mod common;
use common::dir;

fn doc(body: &str) -> String {
    format!(
        "machining\nf = part(\"blind_holes.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 1), below: 3), origin: \"top-corner\", tol: 0.01, at: (-20, -20, -250))\n{body}\np = program(op1, post: \"fanuc\", number: 1)\n"
    )
}

#[test]
fn a_flat_floored_hole_is_drilled_to_its_floor_not_through_it() {
    let cam = forge_cam_lang::run(&doc("h = drill(op1, f.flat, plan: \"drill\")"), &dir());
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    let o = cam.ops.iter().find(|o| o.name == "h").unwrap();
    assert_eq!(o.status, "built", "{:?}", o.refusal);
    assert!(
        cam.warnings
            .iter()
            .any(|w| w.starts_with("h: `flat` has a flat floor: the drill stops with its point on it")),
        "{:?}",
        cam.warnings
    );
}

#[test]
fn a_pointed_hole_is_drilled_to_its_point() {
    let cam = forge_cam_lang::run(&doc("h = drill(op1, f.pointed, plan: \"drill\")"), &dir());
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    let o = cam.ops.iter().find(|o| o.name == "h").unwrap();
    assert_eq!(o.status, "built", "{:?}", o.refusal);
    assert!(
        !cam.warnings.iter().any(|w| w.contains("flat floor")),
        "{:?}",
        cam.warnings
    );
}

#[test]
fn a_counterbore_is_cut_after_the_drill() {
    // no counterbore tool of 11 mm in the library: the largest end mill under it pockets its
    // circle to its depth (which one depends on the library: not pinned)
    let cam = forge_cam_lang::run(&doc("h = drill(op1, f.cbh, plan: \"drill\")"), &dir());
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    let o = cam.ops.iter().find(|o| o.name == "h").unwrap();
    assert_eq!(o.status, "built", "{:?}", o.refusal);
    assert!(
        o.plan
            .iter()
            .any(|p| p.starts_with("counterbore by ") && p.ends_with(" (pocketed, helical entry) to 4")),
        "{:?}",
        o.plan
    );
}

#[test]
fn a_countersink_is_cut_after_the_drill() {
    // a 90 deg tool of at least 9 mm plunged until its cone is 9 mm across at the mouth
    let cam = forge_cam_lang::run(&doc("h = drill(op1, f.csk, plan: \"drill\")"), &dir());
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    let o = cam.ops.iter().find(|o| o.name == "h").unwrap();
    assert_eq!(o.status, "built", "{:?}", o.refusal);
    assert!(
        o.plan.iter().any(|p| p.starts_with("countersink ")),
        "{:?}",
        o.plan
    );
}

#[test]
fn a_through_hole_stops_short_of_the_support_or_says_what_stock_it_needs() {
    // 3 mm under the part: the 6.6 drill's point (1.98) and a break-through fit above the
    // support; 1 mm does not: refused, naming the stock it needs
    let cam = forge_cam_lang::run(&doc("h = drill(op1, f.csk, plan: \"drill\")"), &dir());
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    assert_eq!(cam.ops.iter().find(|o| o.name == "h").unwrap().status, "built");
    let thin = doc("h = drill(op1, f.csk, plan: \"drill\")").replace("below: 3", "below: 1");
    let cam = forge_cam_lang::run(&thin, &dir());
    let o = cam.ops.iter().find(|o| o.name == "h").unwrap();
    assert_eq!(o.status, "refused");
    let r = o.refusal.clone().unwrap_or_default() + &cam.errors.join(" ");
    assert!(
        r.contains("goes through") && r.contains("of stock under the part"),
        "{r}"
    );
}

#[test]
fn a_hole_with_an_iso_class_is_drilled_under_size_and_reamed() {
    let cam = forge_cam_lang::run(&doc("h = drill(op1, f.pin, plan: \"auto\")"), &dir());
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    let o = cam.ops.iter().find(|o| o.name == "h").unwrap();
    assert_eq!(o.status, "built", "{:?}", o.refusal);
    assert!(
        o.plan.iter().any(|p| p.starts_with("drill drill-9.8")),
        "{:?}",
        o.plan
    );
    assert!(
        o.plan.iter().any(|p| p == "ream reamer-10h7 (H7)"),
        "{:?}",
        o.plan
    );
}

#[test]
fn a_dowel_sized_hole_with_no_class_is_said() {
    let cam = forge_cam_lang::run(&doc("h = drill(op1, f.pointed, plan: \"drill\")"), &dir());
    assert!(
        cam.warnings
            .iter()
            .any(|w| w.starts_with("h: `pointed` is a dowel's size (5 mm) with no fit")),
        "{:?}",
        cam.warnings
    );
}

#[test]
fn a_tapped_hole_is_certified_against_its_declared_thread() {
    // block_m6: the M6 hole is modelled at its 5 mm tap drill; the tap cuts to 6. The thread is
    // declared (Δ, in removals) and the op certified against P ∖ Δ, as thread_mill is
    let src = "machining\nf = part(\"block_m6.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 1), below: 2), origin: \"top-corner\", tol: 0.01, at: (-20, -20, -250))\nskim = facing(op1, tool: t1, to: 14, step: 1)\nside = clear(op1, tool: t1, from: 14, to: -0.5, step: 3, stepover: 4)\ntap = drill(op1, f.m6, plan: \"auto\")\nprog = program(op1, post: \"fanuc\", number: 1)\n";
    let cam = forge_cam_lang::run(src, &dir());
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    let o = cam.ops.iter().find(|o| o.name == "tap").unwrap();
    assert_eq!(o.status, "built", "{:?}", o.refusal);
    assert!(o.plan.iter().any(|p| p.starts_with("tap ")), "{:?}", o.plan);
    assert!(
        o.removals
            .iter()
            .any(|r| r.starts_with("cosmetic_thread M6x1 of `m6`")),
        "{:?}",
        o.removals
    );
}

#[test]
fn the_manifold_taps_are_certified_against_their_threads() {
    // the manifold's top face (z 26) trims its pocket's corner arcs end first and its boss's
    // foot anticlockwise: its trim is chained and its loops' senses set by nesting, so the
    // thread's disk comes out of it (the face kept whole, the tap gouged it by 0.5)
    let src = "machining\nf = part(\"manifold.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 2.5, 0), below: 4.5), origin: \"top-corner\", tol: 0.01, at: (-60, -40, -260))\ntap = drill(op1, f.taps, plan: \"auto\")\nprog = program(op1, post: \"fanuc\", number: 1)\n";
    let cam = forge_cam_lang::run(src, &dir());
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    let o = cam.ops.iter().find(|o| o.name == "tap").unwrap();
    assert_eq!(o.removals.len(), 4, "{:?}", o.removals);
    // (the stock is left above the holes here, so the tap's shank may meet it: only the part
    // is this test's subject)
    assert!(
        !o.checks.iter().any(|c| c.rule == "gouge" && c.verdict != "pass"),
        "{:?}",
        o.checks
            .iter()
            .filter(|c| c.rule == "gouge")
            .take(3)
            .collect::<Vec<_>>()
    );
}
