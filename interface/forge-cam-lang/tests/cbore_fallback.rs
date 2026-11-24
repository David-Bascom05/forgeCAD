//! The counterbore's fallback pinned: with no counterbore tool in the library (catalogue
//! counterbores, e.g. CAM-HAAS's DIN 373 Ø11, would otherwise be chosen), the largest end mill under
//! the counterbore's diameter pockets its circle to its depth after the drill.
mod common;
use common::dir;
use forge_tooling::{Kind, Library};

#[test]
fn without_a_counterbore_tool_the_counterbore_is_pocketed() {
    let mut lib = Library::builtin();
    lib.tools.retain(|t| t.kind != Kind::Counterbore);
    let src = "machining\nf = part(\"blind_holes.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 1), below: 3), origin: \"top-corner\", tol: 0.01, at: (-20, -20, -250))\nh = drill(op1, f.cbh, plan: \"drill\")\np = program(op1, post: \"fanuc\", number: 1)\n";
    let cam = forge_cam_lang::run_with_library(src, &dir(), lib);
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
    // the drill before it
    let d = o
        .plan
        .iter()
        .position(|p| p.starts_with("drill "))
        .expect("a drill step");
    let c = o.plan.iter().position(|p| p.starts_with("counterbore ")).unwrap();
    assert!(d < c, "{:?}", o.plan);
}
