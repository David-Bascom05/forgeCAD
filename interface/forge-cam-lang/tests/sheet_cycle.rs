//! The setup sheet's cycle time: the lower bound and the estimate together, as the app shows them,
//! from the one sum (`Cam::cycle_times`) forge-cam-quality reads too.
mod common;

use forge_cam_lang::cycle::{CYCLE_NOTE, cycle_words, mmss};

#[test]
fn the_sheet_shows_the_lower_bound_beside_the_estimate() {
    let src = std::fs::read_to_string(common::dir().join("flange_cam.fg")).unwrap();
    let cam = forge_cam_lang::run(&src, &common::dir());
    assert!(cam.ok, "{:?}", cam.errors);
    let ct = cam.cycle_times(&forge_machine::library::Library::with_builtin(), None);
    assert_eq!(ct.ops.len(), cam.ops.len());
    let p = &ct.progs[0];
    assert!(p.error.is_none(), "{:?}", p.error);
    assert!(p.t_min > 0.0 && p.t_scurve >= p.t_min, "{p:?}");
    // the ops' shares add up to their program's (every op is posted by the one program)
    let lo: f64 = ct.ops.iter().map(|t| t.t_min).sum();
    assert!(lo > 0.0 && lo <= p.t_min + 1e-6, "{lo} vs {}", p.t_min);
    let sheet = cam.sheet();
    for (o, t) in cam.ops.iter().zip(&ct.ops) {
        if t.t_min > 0.0 {
            assert!(sheet.contains(&cycle_words(t.t_min, t.t_scurve)), "{}", o.name);
        }
    }
    assert!(sheet.contains(&format!(
        "{}, {} tool change",
        cycle_words(p.t_min, p.t_scurve),
        p.tool_changes
    )));
    assert!(sheet.contains("est. "));
    assert!(
        sheet.contains(
            &CYCLE_NOTE
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
        )
    );
}

#[test]
fn a_time_rounds_to_whole_seconds() {
    assert_eq!(mmss(59.6), "1:00");
    assert_eq!(mmss(125.4), "2:05");
    assert_eq!(cycle_words(360.0, 390.0), "\u{2265} 6:00 \u{b7} est. 6:30");
    // never an estimate under the bound
    assert_eq!(cycle_words(60.0, 50.0), "\u{2265} 1:00 \u{b7} est. 1:00");
}
