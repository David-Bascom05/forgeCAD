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
