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

#[test]
fn the_collar_back_chamfer_is_finished_left_hand() {
    let d = dir();
    let src = std::fs::read_to_string(d.join("collar_turn.fg")).unwrap();
    let cam = forge_turn_lang::run(&src, &d);
    assert!(cam.ok, "{:?} {:?}", cam.errors, cam.ops);
    assert_eq!(cam.certification(), "full");
    let text = cam.program_text().unwrap();
    assert!(text.contains("T0909") && text.contains("Z-15.6"), "{text}");
    // the right-hand profiling tool on down the chamfer instead: its trailing edge meets the collar
    let rh = src.replace("to: -16)\nneck", "to: -21)\nneck");
    assert_ne!(rh, src);
    let cam = forge_turn_lang::run(&rh, &d);
    assert!(!cam.ok && cam.ops.iter().any(|o| o.name == "fin" && o.status == "refused"));
}

#[test]
fn a_view_reads_the_profile_the_setup_and_the_paths() {
    let d = dir();
    let src = std::fs::read_to_string(d.join("shaft_turn.fg")).unwrap();
    let cam = forge_turn_lang::run(&src, &d);
    assert_eq!(cam.parts().len(), 1);
    let (name, s) = &cam.setups()[0];
    assert_eq!((name.as_str(), s.bar_r, s.bar_front), ("op1", 16.0, 2.0));
    assert!(!s.chuck_region().loops.is_empty());
    let names: Vec<&str> = cam.paths().iter().map(|p| p.0.as_str()).collect();
    assert_eq!(names, ["face", "rough", "fin", "thr", "cut"]);
    assert!(cam.paths().iter().all(|p| p.1 == 0 && !p.2.steps.is_empty()));
}

#[test]
fn an_unnamed_operation_is_an_error() {
    let d = dir();
    let src = std::fs::read_to_string(d.join("shaft_turn.fg"))
        .unwrap()
        .replace("thr   = turn_thread(", "turn_thread(");
    let cam = forge_turn_lang::run(&src, &d);
    assert!(!cam.ok);
    assert!(
        cam.errors
            .iter()
            .any(|e| e.contains("an operation needs a name: `name = turn_thread(..)`")),
        "{:?}",
        cam.errors
    );
}

#[test]
fn maker_and_project_tooling_reach_turning_through_the_library() {
    let d = dir();
    let src = std::fs::read_to_string(d.join("shaft_turn.fg")).unwrap();
    // the rough and face tool from the makers' records: a Korloy PCLNR2525M12 with a Sumitomo
    // CNMG120408
    let makers = src.replace(
        "lathe_tool(\"od-rough-cnmg\", t: 1)",
        "lathe_tool(holder: \"korloy-PCLNR2525-M12\", insert: \"sumitomo-CNMG120408N-GU\", t: 1)",
    );
    assert_ne!(makers, src);
    let cam = forge_turn_lang::run(&makers, &d);
    assert!(cam.ok, "{:?} {:?}", cam.errors, cam.ops);
    // a project's own insert (a shop record, not built in): unknown to `run`, used through the
    // library the levels make
    let shop = makers.replace("sumitomo-CNMG120408N-GU", "shop-cnmg-0408");
    let cam = forge_turn_lang::run(&shop, &d);
    assert!(
        !cam.ok && cam.errors.iter().any(|e| e.contains("shop-cnmg-0408")),
        "{:?}",
        cam.errors
    );
    let mut lib = forge_tooling::Library::builtin();
    lib.add(
        "project/cam/turning/shop.toml",
        "[[insert]]\nid = \"shop-cnmg-0408\"\ncode = \"CNMG120408\"\nic = 12.7\ns = 4.76\nre = 0.8\nmaker = \"shop\"\nsource = \"shop\"\ntaken = \"2026-11-24\"\n",
    )
    .unwrap();
    let cam = forge_turn_lang::run_with_library(&shop, &d, false, lib);
    assert!(cam.ok, "{:?} {:?}", cam.errors, cam.ops);
    assert_eq!(cam.certification(), "full");
}

#[test]
fn the_hub_certifies_in_full() {
    // CAM-CORE's shop test part: a hollow hub, 14 ops covering every turning op
    let d = dir();
    let src = std::fs::read_to_string(d.join("hub_turn.fg")).unwrap();
    let cam = forge_turn_lang::run(&src, &d);
    assert!(cam.ok, "{:?} {:?}", cam.errors, cam.ops);
    assert_eq!(cam.certification(), "full");
    assert_eq!(cam.ops.len(), 14);
    // the finishes without their groove bridges dive into the relief grooves: refused
    let dive = src
        .replace(", skip: ((-23, -20), (-56, -52)))", ")")
        .replace(", skip: (-25, -22))", ")");
    assert_ne!(dive, src);
    let cam = forge_turn_lang::run(&dive, &d);
    assert!(
        cam.ops.iter().any(|o| o.name == "fin" && o.status == "refused")
            && cam.ops.iter().any(|o| o.name == "bfin" && o.status == "refused"),
        "{:?}",
        cam.ops
    );
    // a blade too short for the bar is refused with the library's blades that reach
    let short = src.replace(
        "lathe_tool(blade: \"tungaloy-CGP32-3D\", insert: \"tungaloy-SGM3-020\", t: 10)",
        "lathe_tool(\"part-3\", t: 10)",
    );
    assert_ne!(short, src);
    let cam = forge_turn_lang::run(&short, &d);
    assert!(
        cam.errors.iter().any(|e| e.contains("part-3-deep (reach 40)")),
        "{:?}",
        cam.errors
    );
}
