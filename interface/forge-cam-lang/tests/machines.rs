//! One machine list (forge_cam_lang::machines): a machine forge-machine defines can be named in
//! a machining document, its record the source for what it carries; a tool-library record of
//! the same name that disagrees is refused, not silently overruled.

use forge_cam_lang::{machines, run, run_with_library};
use std::path::{Path, PathBuf};

fn machined() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined")
}

/// The flange on the Haas VF-2 (CAM-HAAS's cited record): it plans, every op is certified, and
/// its program posts in the machine's own dialect.
#[test]
fn the_flange_plans_certifies_and_posts_on_a_haas_vf2() {
    let src = std::fs::read_to_string(machined().join("flange_cam.fg"))
        .unwrap()
        .replace("machine(\"generic-vmc3\")", "machine(\"haas-vf-2\")")
        .replace("at: (0, 0, -250)", "at: (-381, -203, -400)")
        .replace("post: \"fanuc\"", "post: \"haas\"");
    let t0 = std::time::Instant::now();
    let cam = run(&src, &machined());
    eprintln!(
        "the flange on the VF-2: {:.0} ms",
        t0.elapsed().as_secs_f64() * 1e3
    );
    assert!(cam.ok, "{:?}", cam.errors);
    for op in &cam.ops {
        assert_eq!(op.status, "built", "{}: {:?}", op.name, op.refusal);
    }
    let prog = cam.programs.first().expect("a program");
    let text = prog.text.as_deref().expect("posted");
    assert_eq!(prog.post, "haas", "posted in the machine's control's dialect");
    assert!(text.lines().count() > 100);
    // the planner's machine is the record's: travel, rapids and the spindle as cited
    let m = machines::combine(machines::record("haas-vf-2").unwrap(), None).unwrap();
    let x = m.joints.iter().find(|j| j.name == "X").unwrap();
    assert_eq!(
        (x.axis, x.range, x.rapid),
        ([1.0, 0.0, 0.0], [-762.0, 0.0], 25400.0)
    );
    assert_eq!(m.rpm, [1.0, 8100.0]);
    assert_eq!(m.max_feed, 16500.0);
    assert_eq!(m.control, "haas");
}

/// The generic VMC's two records (forge-tooling's and forge-machine's) agree, so it is one
/// machine; a tool library whose copy disagrees is refused where a document names it, saying
/// what differs.
#[test]
fn two_records_that_disagree_are_refused() {
    let lib = forge_tooling::Library::builtin();
    assert!(
        machines::conflict(&lib, "generic-vmc3").is_none(),
        "the built-in copies agree"
    );
    let mut lib = forge_tooling::Library::builtin();
    let m = lib
        .machines
        .iter_mut()
        .find(|m| m.name == "generic-vmc3")
        .unwrap();
    m.joints.iter_mut().find(|j| j.name == "Z").unwrap().range = [-400.0, 0.0];
    let src = std::fs::read_to_string(machined().join("flange_cam.fg")).unwrap();
    let cam = run_with_library(&src, &machined(), lib);
    assert!(!cam.ok);
    let e = cam.errors.join("\n");
    assert!(e.contains("disagree") && e.contains("joint Z travel"), "{e}");
}
