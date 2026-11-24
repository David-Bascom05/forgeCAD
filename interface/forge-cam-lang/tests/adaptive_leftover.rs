//! Adaptive clearing never leaves stock silently: what a level leaves after every pass and ring,
//! more than a cell inside the region, is reported on the op (and G4 lists it). The case: a
//! round stock box 3 mm round the boss's 70 mm plate, where the passes run out of entries.
mod common;
use std::path::Path;

#[test]
fn stock_left_after_every_pass_and_ring_is_reported() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../verify/forge-cam-quality/parts");
    let src = "machining\nf = part(\"boss.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"flat-6\", t: 1, stickout: 30, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: box(min: (-3, -3, 0), max: (73, 73, 26)), origin: \"top-corner\", tol: 0.01)\nskim = facing(op1, tool: t1, to: 25, step: 1)\nrough = adaptive(op1, tool: t1, from: 25, to: 10.2, step: 15, engagement: 40, passes: \"two_way\")\nprog = program(op1, post: \"fanuc\", number: 5007)\n";
    let cam = forge_cam_lang::run(src, &dir);
    let w: Vec<&String> = cam
        .warnings
        .iter()
        .filter(|w| {
            w.starts_with("rough: adaptive: at z 10.200 ")
                && w.contains("left uncut after every pass and ring")
        })
        .collect();
    assert_eq!(w.len(), 1, "{:?}", cam.warnings);
}
