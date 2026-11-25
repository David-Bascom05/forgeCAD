//! A cut that comes within the fixture clearance of a modelled vise's jaws is refused, naming
//! the jaw, and is never certified: a real violation, not a tie. (A pass exactly at the fixture
//! clearance is proven clear by forge-cam-geom's axisplane.rs, tested there.)
mod common;
use common::dir;

#[test]
fn a_contour_down_beside_the_jaws_is_refused_naming_the_jaw() {
    let src = "machining\nf = part(\"well_island.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: box(min: (-3, -3, 0), max: (63, 43, 13)), origin: \"top-corner\", tol: 0.01, at: (0, 0, -250), fixture: vise(\"generic-vise-150\", across: \"x\", clamp: 4))\nprof = contour(op1, tool: t1, side: \"outside\", from: 13, to: 0, step: 5)\np = program(op1, post: \"fanuc\", number: 1)\n";
    let cam = forge_cam_lang::run(src, &dir());
    let o = cam.ops.iter().find(|o| o.name == "prof").unwrap();
    assert_eq!(o.status, "refused", "{:?}", o.refusal);
    assert_eq!(o.code.as_deref(), Some("fixture_collision"));
    let w = o.refusal.as_deref().unwrap();
    assert!(
        w.contains("of the workholding") && w.contains("(fixed jaw)") && w.contains("1 mm clear"),
        "{w}"
    );
}
