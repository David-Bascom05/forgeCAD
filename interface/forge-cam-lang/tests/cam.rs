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
    // the bore with a short-fluted tool and nothing cleared above: the shank reaches the stock
    let cam = forge_cam_lang::run(
        &doc(
            "bore = pocket(op1, f.bore_h, tool: t2, stepover: 2.5, step: 3)\nprog = program(op1, post: \"haas\")\n",
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
    assert!(o.fix.as_deref().unwrap().contains("flutes"));
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
