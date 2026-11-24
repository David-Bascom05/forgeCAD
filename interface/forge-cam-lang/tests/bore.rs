//! Helical bore (Fusion's Bore): a helix down a round hole's wall, a circle at the bottom, a
//! finishing helix and spring passes; certified against the part.
mod common;
use common::{dir, doc};
use forge_cam::ops::PMove;

const PRE: &str = "skim = facing(op1, tool: t1, to: 18, step: 1)\n";

fn run(extra: &str) -> forge_cam_lang::Cam {
    let body = format!(
        "{PRE}b = bore(op1, f.bore_h, tool: t1{extra})\nprog = program(op1, post: \"fanuc\", number: 1)\n"
    );
    forge_cam_lang::run(&doc(&body), &dir())
}

fn helices(cam: &forge_cam_lang::Cam) -> usize {
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    let o = cam.ops.iter().find(|o| o.name == "b").unwrap();
    assert_eq!(o.status, "built", "{:?} {:?}", o.refusal, o.fix);
    o.paths[0]
        .moves
        .iter()
        .filter(|m| matches!(m, PMove::Rapid { .. }))
        .count()
        - 1
}

#[test]
fn a_bore_is_helices_down_the_hole() {
    assert_eq!(helices(&run("")), 1);
    assert_eq!(helices(&run(", pitch: 2, finish_leave: 0.5, spring: 1")), 3);
    let cam = run(", ramp: 10");
    helices(&cam);
    let (txt, _) = cam.program_text().unwrap();
    assert!(
        txt.lines().any(|l| l.starts_with("G3") || l.contains(" G3 ")),
        "{txt}"
    );
    // a tool under half the hole across leaves a core
    let body = format!("{PRE}b = bore(op1, f.bore_h, tool: t2)\n");
    let cam = forge_cam_lang::run(&doc(&body), &dir());
    let msgs: Vec<String> = cam
        .errors
        .iter()
        .cloned()
        .chain(cam.ops.iter().filter_map(|o| o.refusal.clone()))
        .collect();
    assert!(msgs.iter().any(|m| m.contains("a core of")), "{msgs:?}");
    for (extra, msg) in [(", pitch: 1, ramp: 3", "not both"), (", ramp: 50", "ramp:")] {
        let cam = run(extra);
        let msgs: Vec<String> = cam
            .errors
            .iter()
            .cloned()
            .chain(cam.ops.iter().filter_map(|o| o.refusal.clone()))
            .collect();
        assert!(msgs.iter().any(|m| m.contains(msg)), "{extra}: {msgs:?}");
    }
}
