//! examples/machined/shaft_turn.fg: every op certified on the posted path, G5 in full, the
//! program written; and a turning document is told apart from a milling one.
use std::path::Path;

fn dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined")
}

#[test]
fn the_shaft_turns() {
    let d = dir();
    let src = std::fs::read_to_string(d.join("shaft_turn.fg")).unwrap();
    assert!(forge_turn_lang::is_turning(&src));
    let cam = forge_turn_lang::run(&src, &d);
    assert!(cam.ok, "{:?} {:?}", cam.errors, cam.ops);
    assert_eq!(cam.certification(), "full");
    assert_eq!(cam.ops.len(), 5);
    assert!(cam.ops.iter().all(|o| o.status == "built"));
    let text = cam.program_text().unwrap();
    assert_eq!(cam.report.as_ref().unwrap().stock_model, "columns");
    // and with the exact stock: the same verdicts, and the leftover (the shoulder's corner)
    let exact = forge_turn_lang::run_checked(&src, &d);
    assert!(exact.ok && exact.report.as_ref().unwrap().leftover_done);
    assert_eq!(exact.report.as_ref().unwrap().leftover.len(), 1);
    assert!(text.contains("G32") && text.contains("G96") && text.contains("T0505"));
}

#[test]
fn a_milling_document_is_not_turning() {
    let src = std::fs::read_to_string(dir().join("flange_cam.fg")).unwrap();
    assert!(!forge_turn_lang::is_turning(&src));
}

#[test]
fn a_groove_the_part_does_not_have_is_refused_and_no_program_is_written() {
    let d = dir();
    let src = std::fs::read_to_string(d.join("shaft_turn.fg"))
        .unwrap()
        .replace("(2, 0, 0), (4, 0, 0)", "(2, 0, 0), (3, 0, 0), (4, 0, 0)")
        .replace(
            "cut   = part_off(",
            "t3 = lathe_tool(\"groove-3\", t: 3)\ngr = turn_groove(op1, tool: t3, from: -20, to: -16, r: 8)\ncut   = part_off(",
        );
    let cam = forge_turn_lang::run(&src, &d);
    assert!(!cam.ok);
    assert_eq!(cam.certification(), "refused");
    assert!(cam.program_text().is_none());
    let g = cam.ops.iter().find(|o| o.name == "gr").unwrap();
    assert_eq!(g.status, "refused");
    assert!(g.refusal.as_ref().unwrap().starts_with("part"), "{:?}", g.refusal);
}

#[test]
fn a_negative_leave_is_refused() {
    let d = dir();
    let src = std::fs::read_to_string(d.join("shaft_turn.fg"))
        .unwrap()
        .replace("tool: t2, to: -55)", "tool: t2, leave: -0.1, to: -55)");
    let cam = forge_turn_lang::run(&src, &d);
    assert!(
        !cam.ok && cam.errors.iter().any(|e| e.contains("leave")),
        "{:?}",
        cam.errors
    );
}

#[test]
fn the_nut_is_bored_grooved_and_threaded_inside() {
    let d = dir();
    let src = std::fs::read_to_string(d.join("nut_turn.fg")).unwrap();
    let cam = forge_turn_lang::run(&src, &d);
    assert!(cam.ok, "{:?} {:?}", cam.errors, cam.ops);
    assert_eq!(cam.certification(), "full");
    assert_eq!(cam.ops.len(), 6);
    let text = cam.program_text().unwrap();
    // the relief groove to Ø32 and the thread's last pass at the major diameter, on the
    // imaginary tip
    assert!(
        text.contains("X32. F0.04") && text.contains("G32 X30. Z-20.5 F1.5"),
        "{text}"
    );
    // finishing the bore on through the relief groove: the boring insert's trailing edge would
    // run up through the groove's top wall
    let through = src.replace("to: -18.5", "to: -30");
    let cam = forge_turn_lang::run(&through, &d);
    assert!(!cam.ok && cam.ops.iter().any(|o| o.name == "fin" && o.status == "refused"));
}
