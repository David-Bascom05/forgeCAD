//! The machining document end to end: built, certified on the posted path, and the refusals
//! typed with fixes.
use std::path::Path;

fn doc(body: &str) -> String {
    format!(
        "machining\nf = part(\"flange.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")\nt2 = tool(\"flat-6\", t: 2, stickout: 26, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 2)), origin: \"top-center\", tol: 0.01)\n{body}"
    )
}

#[test]
fn the_flange_program_is_certified() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let src = std::fs::read_to_string(dir.join("flange_cam.fg")).unwrap();
    let cam = forge_cam_lang::run(&src, &dir);
    assert!(
        cam.ok,
        "{:?} {:?}",
        cam.errors,
        cam.ops.iter().map(|o| (&o.name, &o.refusal)).collect::<Vec<_>>()
    );
    assert_eq!(cam.ops.len(), 5);
    assert!(cam.ops.iter().all(|o| o.status == "built" && o.pairs > 0));
    let (txt, post) = cam.program_text().unwrap();
    assert_eq!(post, "fanuc");
    assert!(
        txt.contains("G98 G83") || txt.contains("G98 G81"),
        "the bolt holes are a canned cycle"
    );
    let mut j = String::new();
    cam.json().write(&mut j);
    assert!(j.contains("\"schema\":\"forgecad.cam/1\""));
    assert!(cam.sheet().contains("certified"));
    // G4: every band decided, nothing left over
    assert!(cam.leftover_done && cam.leftover.is_empty(), "{:?}", cam.leftover);
}

#[test]
fn leftover_is_listed() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    // facing alone: the plate's sides, the bore and the holes are all still stock
    let cam = forge_cam_lang::run(&doc("skim = facing(op1, tool: t1, to: 18, step: 1)\n"), &dir);
    assert!(cam.leftover_done);
    assert!(!cam.leftover.is_empty());
    assert!(
        cam.leftover.iter().any(|l| l.z0 < 12.0 && l.z1 > 0.0),
        "{:?}",
        cam.leftover
    );
}

#[test]
fn refusals_name_the_check_and_a_fix() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    // the bore with a stick-out so short the holder's nose dips below the stock's top
    let cam = forge_cam_lang::run(
        &doc(
            "t3 = tool(\"flat-6\", t: 3, stickout: 19, holder: \"bt40-er32\")\nbore = pocket(op1, f.bore_h, tool: t3, stepover: 2.5, step: 3)\nprog = program(op1, post: \"haas\")\n",
        ),
        &dir,
    );
    assert!(!cam.ok);
    let o = &cam.ops[0];
    assert_eq!(o.status, "refused");
    assert!(
        o.checks.iter().any(|c| c.rule == "holder_collision"),
        "{:?}",
        o.checks
    );
    assert!(o.fix.is_some());
    assert!(
        cam.program_text().is_none(),
        "no program is released with a refused op"
    );
    // the bolt holes before the plate top is cleared: the rapids would enter stock
    let cam = forge_cam_lang::run(
        &doc(
            "skim = facing(op1, tool: t1, to: 18, step: 1)\nbolts = drill(op1, f.bolts, plan: \"auto\")\nprog = program(op1)\n",
        ),
        &dir,
    );
    let b = cam.ops.iter().find(|o| o.name == "bolts").unwrap();
    assert_eq!(b.status, "refused", "{:?}", b.checks);
    assert!(b.checks.iter().any(|c| c.rule == "rapid_clear"));
    // a typed error for an unknown name
    let cam = forge_cam_lang::run(&doc("x = drill(op1, f.nope)\n"), &dir);
    assert!(
        cam.errors.iter().any(|e| e.contains("no hole feature `nope`")),
        "{:?}",
        cam.errors
    );
}

#[test]
fn fillets_and_chamfers_are_lowered_and_a_gouge_into_a_fillet_is_refused() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let src = "machining\nf = part(\"boss_fillet.fg\", group: \"N\")\nt1 = tool(\"flat-6\", t: 1, stickout: 26, holder: \"bt40-er32\")\nop1 = setup(f, stock: box(margin: (2, 2, 1)), origin: \"top-center\")\nskim = facing(op1, tool: t1, to: 18, step: 1)\nboss = contour(op1, tool: t1, side: \"outside\", from: 18, to: 10.5, step: 4)\nprog = program(op1, post: \"linuxcnc\")\n";
    let cam = forge_cam_lang::run(src, &dir);
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    let b = cam.ops.iter().find(|o| o.name == "boss").unwrap();
    // the contour is made from the section above the fillet: at its foot the tool meets the
    // fillet (a torus face), a real gouge
    assert_eq!(b.status, "refused", "{:?}", b.checks);
    assert!(b.checks.iter().any(|c| c.rule == "gouge"), "{:?}", b.checks);
}

#[test]
fn a_pocket_of_every_hole_of_the_section() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let cam = forge_cam_lang::run(
        &doc(
            "skim = facing(op1, tool: t1, to: 18, step: 1)\ntop = clear(op1, tool: t1, from: 18, to: 12, step: 3, stepover: 4)\nholes = pocket(op1, tool: t2, from: 12, to: -0.5, stepover: 2, step: 4)\nprog = program(op1)\n",
        ),
        &dir,
    );
    let o = cam.ops.iter().find(|o| o.name == "holes").unwrap();
    // the bore and the four bolt holes, each its own pocket
    assert_eq!(o.paths.len(), 5, "{:?}", o.refusal);
    assert_eq!(o.status, "built", "{:?}", o.checks);
}

fn vised(clamp: f64, body: &str) -> String {
    format!(
        "machining\nf = part(\"flange.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 2)), origin: \"top-center\", tol: 0.01, fixture: vise(\"generic-vise-150\", across: \"x\", clamp: {clamp}, parallel: 6, clearance: 1))\n{body}prog = program(op1, post: \"fanuc\")\n"
    )
}

#[test]
fn the_vise_is_checked_by_its_clearance() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    // clearing the plate's top down to z 12 runs out to the stock's faces, where the jaws are:
    // jaws to z 10.9 keep the 1 mm clearance under the cutter's tip, jaws to 11.1 do not
    let body = "skim = facing(op1, tool: t1, to: 18, step: 1)\ntop = clear(op1, tool: t1, from: 18, to: 12, step: 3, stepover: 4)\n";
    let cam = forge_cam_lang::run(&vised(10.9, body), &dir);
    assert!(
        cam.ok,
        "{:?} {:?}",
        cam.errors,
        cam.ops.iter().map(|o| (&o.name, &o.refusal)).collect::<Vec<_>>()
    );
    let cam = forge_cam_lang::run(&vised(11.1, body), &dir);
    assert!(!cam.ok);
    let o = cam.ops.iter().find(|o| o.name == "top").unwrap();
    assert_eq!(o.status, "refused");
    let c = o.checks.iter().find(|c| c.rule == "fixture_collision").expect("a fixture check");
    assert!(c.other.contains("jaw"), "{}", c.other);
    let fix = o.fix.as_deref().unwrap();
    assert!(fix.starts_with("SetArg clamp: grip at most 11.00"), "{fix}");
    assert!(cam.program_text().is_none());
    // the facing passes overhang the stock: with the jaws gripping its full height, they would
    // cut the jaws
    let cam = forge_cam_lang::run(&vised(20.0, "skim = facing(op1, tool: t1, to: 18, step: 1)\n"), &dir);
    assert!(!cam.ok);
    assert!(cam.ops[0].checks.iter().any(|c| c.rule == "fixture_collision"));
    // the profile goes through the plate: the jaws hold what it would cut; split the setup
    let cam = forge_cam_lang::run(
        &vised(
            5.0,
            "skim = facing(op1, tool: t1, to: 18, step: 1)\ntop = clear(op1, tool: t1, from: 18, to: 12, step: 3, stepover: 4)\nprof = contour(op1, tool: t1, side: \"outside\", from: 12, to: -0.5, step: 4)\n",
        ),
        &dir,
    );
    let o = cam.ops.iter().find(|o| o.name == "prof").unwrap();
    assert_eq!(o.status, "refused");
    assert!(o.fix.as_deref().unwrap().contains("second setup"), "{:?}", o.fix);
    // a clamp deeper than the stock is refused at the setup
    let cam = forge_cam_lang::run(&vised(25.0, ""), &dir);
    assert!(cam.errors.iter().any(|e| e.contains("clamp depth")), "{:?}", cam.errors);
}

#[test]
fn rapids_across_the_jaws_keep_the_clearance() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    // jaws flush with the stock's top; the bore's rapids at 0.5 above it come within 1 mm
    let doc = |retract: f64| {
        format!(
            "machining\nf = part(\"flange.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 2)), origin: \"top-center\", tol: 0.01, clear: {retract}, retract: {retract}, fixture: vise(\"generic-vise-150\", across: \"x\", clamp: 20, clearance: 1))\nskim = facing(op1, tool: t1, to: 19.95, step: 1)\nprog = program(op1, post: \"fanuc\")\n"
        )
    };
    let cam = forge_cam_lang::run(&doc(0.5), &dir);
    assert!(!cam.ok);
    assert!(
        cam.ops[0].checks.iter().any(|c| c.rule == "fixture_collision" && c.phase == "rapid"),
        "{:?}",
        cam.ops[0].checks
    );
}

#[test]
fn what_the_jaws_hold_is_left_over_by_name() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let cam = forge_cam_lang::run(
        &vised(
            5.0,
            "skim = facing(op1, tool: t1, to: 18, step: 1)\ntop = clear(op1, tool: t1, from: 18, to: 12, step: 3, stepover: 4)\nprof = contour(op1, tool: t1, side: \"outside\", from: 12, to: 6.5, step: 3)\n",
        ),
        &dir,
    );
    assert!(cam.ok, "{:?}", cam.ops.iter().map(|o| (&o.name, &o.refusal)).collect::<Vec<_>>());
    assert!(cam.leftover_done);
    assert!(cam.leftover.iter().any(|l| l.why == "held by fixture" && l.z1 <= 5.0));
    assert!(cam.leftover.iter().all(|l| l.why == "uncut" || l.z1 <= 5.0));
    let mut j = String::new();
    cam.json().write(&mut j);
    assert!(j.contains("\"why\":\"held by fixture\"") && j.contains("\"jaw_top\":5"), "{j}");
    assert!(cam.sheet().contains("Held in the vise"));
}
