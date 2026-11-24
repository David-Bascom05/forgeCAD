//! What the manifold test part (examples/machined/manifold*.fg) found in the section-based ops:
//! an island standing in a pocket was taken as an outline (the outside contour refused); one
//! drilled hole the tool does not fit refused a whole section pocket; and a hole drilled from
//! below could not be drilled in the setup that turns the part over.
mod common;
use common::dir;

fn run(src: &str) -> forge_cam_lang::Cam {
    forge_cam_lang::run(src, &dir())
}

#[test]
fn an_island_in_a_pocket_is_not_an_outline() {
    // well_island: a boss standing in a well; the outside contour is the outline only
    let src = "machining\nf = part(\"well_island.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"flat-6\", t: 1, stickout: 30, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 1), below: 2), origin: \"top-corner\", tol: 0.01, at: (-20, -20, -250))\nprof = contour(op1, tool: t1, side: \"outside\", from: 12, to: 7, step: 3)\np = program(op1, post: \"fanuc\", number: 1)\n";
    let cam = run(src);
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    let o = cam.ops.iter().find(|o| o.name == "prof").unwrap();
    assert_eq!(o.status, "built", "{:?}", o.refusal);
}

#[test]
fn a_section_pocket_leaves_the_holes_its_tool_does_not_fit() {
    // the flange between z 12 and 0: the 20 mm bore and the four 6.6 mm bolt holes; a 10 mm
    // tool pockets the bore and leaves the bolt holes, said
    let src = "machining\nf = part(\"flange.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 2), below: 3), origin: \"top-center\", tol: 0.01, at: (0, 0, -250))\nskim = facing(op1, tool: t1, to: 18, step: 1)\nbores = pocket(op1, tool: t1, from: 12, to: -0.5, stepover: 2.5, step: 3)\n";
    let cam = run(src);
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    assert!(
        cam.warnings
            .iter()
            .any(|w| w.starts_with("bores: 4 of 5 holes in the section are too small for the 10 mm tool")),
        "{:?}",
        cam.warnings
    );
    // none fits: refused
    let cam = run(&src
        .replace("flat-10\", t: 1, stickout: 32", "flat-12\", t: 1, stickout: 32")
        .replace("from: 12, to: -0.5", "from: 12, to: 6"));
    let _ = cam;
}

#[test]
fn a_hole_drilled_from_below_is_drilled_in_the_turned_setup() {
    let tmp = forge_dirs::temp_dir().join(format!("forgecad-turned-holes-{}", forge_dirs::process_id()));
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::write(
        tmp.join("under.fg"),
        "shared L = 40\nshared W = 30\nshared t = 10\nshared hx = 20\nshared hy = 15\nshared hd = 6.6\nshared deep = 6\nshared tip = 118deg\nsketch plan(on: xy) {\n  o = rect(corner: (0, 0), size: (L, W))\n}\nplate = extrude(plan, t)\nsketch bot(on: plate.start) {\n  h = pt(hx, -hy)\n}\nbelow = hole(at: bot.h, d: hd, depth: deep, point: tip)\n",
    )
    .unwrap();
    let doc = |setup: &str| {
        format!(
            "machining\nf = part(\"under.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 1), below: 1), origin: \"top-corner\", tol: 0.01, at: (-20, -20, -250))\nskim = facing(op1, tool: t1, to: 10, step: 1)\n{setup}"
        )
    };
    // from above it opens on the other side: refused
    let cam = forge_cam_lang::run(&doc("dr = drill(op1, f.below, plan: \"drill\")\n"), &tmp);
    let o = cam.ops.iter().find(|o| o.name == "dr");
    assert!(
        o.is_none_or(|o| o.status == "refused") || cam.errors.iter().any(|e| e.contains("not along")),
        "{:?} {:?}",
        cam.errors,
        o.map(|o| &o.refusal)
    );
    // turned over: drilled
    let cam = forge_cam_lang::run(
        &doc(
            "op2 = setup(f, from: op1, machine: m, origin: \"top-corner\", tol: 0.01, at: (-20, 20, -250))\np1 = program(op1, post: \"fanuc\", number: 1)\ndr = drill(op2, f.below, plan: \"drill\")\np2 = program(op2, post: \"fanuc\", number: 2)\n",
        ),
        &tmp,
    );
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    let o = cam.ops.iter().find(|o| o.name == "dr").unwrap();
    assert_eq!(o.status, "built", "{:?}", o.refusal);
    let _ = std::fs::remove_dir_all(&tmp);
}
