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

/// A project's own copy of a machine (a `cam/*.toml` beside the document) is not shadowed
/// silently by the built-in record: where it disagrees (here the VF-2's X travel), the name is
/// refused saying so; where it agrees it is one machine with the record.
#[test]
fn a_projects_disagreeing_copy_is_refused_not_shadowed() {
    let dir = std::env::temp_dir().join(format!("fg-machines-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("cam")).unwrap();
    std::fs::copy(machined().join("flange.fg"), dir.join("flange.fg")).unwrap();
    let src = std::fs::read_to_string(machined().join("flange_cam.fg"))
        .unwrap()
        .replace("machine(\"generic-vmc3\")", "machine(\"haas-vf-2\")")
        .replace("at: (0, 0, -250)", "at: (-381, -203, -400)")
        .replace("post: \"fanuc\"", "post: \"haas\"");
    let copy = |x: [i32; 2]| {
        format!(
            "[[machine]]\nname = \"haas-vf-2\"\nkind = \"vmc3\"\ncontrol = \"haas\"\nrpm = [1, 8100]\ncurve_rpm = [0, 1753.4, 8100]\ncurve_kw = [0, 22.4, 22.4]\nmax_feed = 16500\ncoolants = [\"flood\"]\nsource = \"a project's copy\"\n\n[[machine.joint]]\nname = \"X\"\naxis = [1, 0, 0]\nrange = [{}, {}]\nrapid = 25400\n\n[[machine.joint]]\nname = \"Y\"\naxis = [0, 1, 0]\nrange = [-406, 0]\nrapid = 25400\n\n[[machine.joint]]\nname = \"Z\"\naxis = [0, 0, 1]\nrange = [-508, 0]\nrapid = 25400\n",
            x[0], x[1]
        )
    };
    // agreeing: one machine, it plans
    std::fs::write(dir.join("cam").join("vf2.toml"), copy([-762, 0])).unwrap();
    let cam = run(&src, &dir);
    assert!(cam.ok, "{:?}", cam.errors);
    // disagreeing: refused, naming the joint's travel
    std::fs::write(dir.join("cam").join("vf2.toml"), copy([-500, 0])).unwrap();
    let cam = run(&src, &dir);
    assert!(!cam.ok);
    let e = cam.errors.join("\n");
    assert!(
        e.contains("joint X travel") && e.contains("[-762.0, 0.0]") && e.contains("[-500.0, 0.0]"),
        "{e}"
    );
}

/// Every record's planner axes: where a machine has linear X, Y and Z joints, they are unit
/// vectors along the axes, right-handed (det +1); a vertical machine's (its milling spindle
/// along +z) exactly +x, +y, +z. One record with a joint's side or sign flipped would mirror
/// every program planned on it.
#[test]
fn every_records_planner_axes_are_right_handed() {
    let mut checked = 0;
    let mut bad = vec![];
    for d in forge_machine::library::builtin() {
        let Ok(m) = machines::combine(&d, None) else {
            bad.push(format!("{}: does not combine", d.name));
            continue;
        };
        let ax = |n: &str| m.joints.iter().find(|j| j.name == n).map(|j| j.axis);
        let (Some(x), Some(y), Some(z)) = (ax("X"), ax("Y"), ax("Z")) else {
            continue;
        };
        checked += 1;
        let unit = |a: [f64; 3]| {
            a.iter().filter(|v| **v != 0.0).count() == 1 && a.iter().all(|v| [0.0, 1.0, -1.0].contains(v))
        };
        let det = x[0] * (y[1] * z[2] - y[2] * z[1]) - x[1] * (y[0] * z[2] - y[2] * z[0])
            + x[2] * (y[0] * z[1] - y[1] * z[0]);
        if !(unit(x) && unit(y) && unit(z)) || det != 1.0 {
            bad.push(format!(
                "{} ({}): X {x:?} Y {y:?} Z {z:?}, det {det}",
                d.name,
                d.kind.name()
            ));
            continue;
        }
        let vertical = d
            .spindles
            .iter()
            .find(|s| s.kind == forge_machine::def::SpindleKind::Milling)
            .is_some_and(|s| s.axis == [0.0, 0.0, 1.0]);
        if vertical && !d.kind.turns() && (x, y, z) != ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]) {
            bad.push(format!(
                "{} ({}, vertical): X {x:?} Y {y:?} Z {z:?}",
                d.name,
                d.kind.name()
            ));
        }
    }
    eprintln!("{checked} records with X, Y and Z checked");
    assert!(checked > 100);
    assert!(bad.is_empty(), "{} records:\n{}", bad.len(), bad.join("\n"));
}
