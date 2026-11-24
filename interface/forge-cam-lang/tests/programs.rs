//! NC programs (CAM-UI's export asks): `ops:` posts some ops of a program's setups. Each op is
//! certified on the stock every earlier op of its setup leaves, so those must be posted before
//! it (an earlier program, or earlier in its own), or the program is refused with the fix; an
//! op no program posts is not certified.
mod common;
use common::{dir, doc};

const OPS: &str = "skim = facing(op1, tool: t1, to: 18, step: 1)\ntop = clear(op1, tool: t1, from: 18, to: 12, step: 3, stepover: 4)\nprof = contour(op1, tool: t1, side: \"outside\", from: 12, to: -0.5, step: 4)\n";

#[test]
fn a_program_posts_the_ops_it_names_in_order() {
    // two programs splitting the setup, in order: both posted and certified
    let cam = forge_cam_lang::run(
        &doc(&format!(
            "{OPS}p1 = program(op1, ops: [skim, top], post: \"fanuc\", number: 1)\np2 = program(op1, ops: prof, post: \"fanuc\", number: 2)\n"
        )),
        &dir(),
    );
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    assert!(
        cam.ops.iter().all(|o| o.status == "built"),
        "{:?}",
        cam.ops.iter().map(|o| &o.refusal).collect::<Vec<_>>()
    );
    assert_eq!(cam.certification(), "full");
    let t1 = cam.programs[0].text.as_deref().unwrap();
    let t2 = cam.programs[1].text.as_deref().unwrap();
    assert!(t1.contains("(skim facing)") && t1.contains("(top clear)") && !t1.contains("(prof contour)"));
    assert!(t2.contains("(prof contour)") && !t2.contains("(skim facing)"));
    assert_eq!(cam.programs[0].ops, vec![0, 1]);
    assert_eq!(cam.programs[1].ops, vec![2]);
}

#[test]
fn an_op_before_the_ops_its_stock_assumes_is_refused() {
    // `top` was certified on the stock `skim` leaves: posting it first is refused, with the fix
    let cam = forge_cam_lang::run(
        &doc(&format!(
            "{OPS}p1 = program(op1, ops: [top, prof], post: \"fanuc\", number: 1)\np2 = program(op1, ops: skim, post: \"fanuc\", number: 2)\n"
        )),
        &dir(),
    );
    assert!(!cam.ok);
    let p1 = &cam.programs[0];
    assert!(p1.text.is_none());
    assert!(
        p1.error
            .as_deref()
            .is_some_and(|e| e.contains("posts `top`") && e.contains("fix: post `skim` first")),
        "{:?}",
        p1.error
    );
    // its ops are then posted by no program: not certified
    assert_eq!(cam.ops[1].status, "unposted");
}

#[test]
fn an_op_no_program_names_is_not_certified_and_ops_are_the_programs_own() {
    let cam = forge_cam_lang::run(
        &doc(&format!(
            "{OPS}p1 = program(op1, ops: [skim, top], post: \"fanuc\", number: 1)\n"
        )),
        &dir(),
    );
    let prof = cam.ops.iter().find(|o| o.name == "prof").unwrap();
    assert_eq!(prof.status, "unposted");
    assert!(
        prof.refusal
            .as_deref()
            .unwrap()
            .contains("no program(..) posts this op")
    );
    // an op of another setup
    let cam = forge_cam_lang::run(
        &doc(&format!(
            "{OPS}op2 = setup(f, from: op1, machine: m, origin: \"top-center\", tol: 0.01, at: (0, 0, -250))\nback = facing(op2, tool: t1, to: 18, step: 1)\np1 = program(op1, ops: [skim, back], post: \"fanuc\", number: 1)\n"
        )),
        &dir(),
    );
    assert!(
        cam.errors
            .iter()
            .any(|e| e.contains("`back` is an op of setup `op2`, which this program does not post")),
        "{:?}",
        cam.errors
    );
}

/// Post options the program carries (CAM-UI's export asks): its number formats and coolant.
/// The text is quantised, read back and certified at them, as at the dialect's own; a coolant
/// the machine does not have is refused.
#[test]
fn a_program_carries_its_decimals_and_coolant() {
    let run = |extra: &str| {
        let cam = forge_cam_lang::run(
            &doc(&format!(
                "{OPS}p1 = program(op1, post: \"fanuc\", number: 1{extra})\n"
            )),
            &dir(),
        );
        assert!(cam.errors.is_empty(), "{extra}: {:?}", cam.errors);
        assert_eq!(cam.certification(), "full", "{extra}");
        cam.programs[0].text.clone().unwrap()
    };
    let plain = run("");
    assert!(plain.contains("M08") && !plain.contains("M07"));
    let fine = run(", decimals: 4, feed_decimals: 2");
    assert_ne!(fine, plain);
    // a feed word with two decimals where the dialect writes one
    assert!(
        fine.lines().any(|l| l
            .split_whitespace()
            .any(|w| w.starts_with('F') && w.split('.').nth(1).is_some_and(|d| d.len() == 2))),
        "{fine}"
    );
    let mist = run(", coolant: \"mist\"");
    assert!(mist.contains("M07") && !mist.contains("M08"));
    let cam = forge_cam_lang::run(
        &doc(&format!(
            "{OPS}p1 = program(op1, post: \"fanuc\", number: 1, coolant: \"through\")\n"
        )),
        &dir(),
    );
    assert!(
        cam.errors
            .iter()
            .any(|e| e.contains("the machine `generic-vmc3` has no through coolant")),
        "{:?}",
        cam.errors
    );
}

/// `split: "setup"` (CAM-UI's export: one file per setup): one program per setup, named
/// `PROGRAM/SETUP`, numbered on from `number:`, each with its setup's name; each certified as a
/// program of its own.
#[test]
fn a_split_program_is_one_program_per_setup() {
    let cam = forge_cam_lang::run(
        &doc(
            "skim = facing(op1, tool: t1, to: 18, step: 1)\nop2 = setup(f, from: op1, machine: m, origin: \"top-center\", tol: 0.01, at: (0, 0, -250))\nback = facing(op2, tool: t1, to: 18, step: 1)\np = program([op1, op2], post: \"fanuc\", number: 10, split: \"setup\")\n",
        ),
        &dir(),
    );
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    assert_eq!(cam.programs.len(), 2);
    let names: Vec<(&str, Option<&str>)> = cam
        .programs
        .iter()
        .map(|p| (p.name.as_str(), p.setup.as_deref()))
        .collect();
    assert_eq!(names, vec![("p/op1", Some("op1")), ("p/op2", Some("op2"))]);
    let (t0, t1) = (
        cam.programs[0].text.as_deref().unwrap(),
        cam.programs[1].text.as_deref().unwrap(),
    );
    assert!(
        t0.contains("O0010") && t0.contains("(skim facing)") && !t0.contains("(back facing)"),
        "{t0}"
    );
    assert!(
        t1.contains("O0011") && t1.contains("(back facing)") && !t1.contains("(skim facing)"),
        "{t1}"
    );
    assert_eq!(cam.certification(), "full");
    let e = forge_cam_lang::run(
        &doc("skim = facing(op1, tool: t1, to: 18, step: 1)\np = program(op1, split: \"op\")\n"),
        &dir(),
    )
    .errors;
    assert!(e.iter().any(|e| e.contains("split: \"setup\"")), "{e:?}");
}
