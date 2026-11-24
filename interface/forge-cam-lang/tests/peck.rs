//! Peck and chip-break cycles: certified (their re-entry down the hole they drilled is a link,
//! the tool passing only where it has been: it was refused as a rapid against the wall the drill
//! touches), and with the same verdicts for any peck retract or chip-break lift the control may
//! be set to (forge_cam_lang::CYCLE_PARAMS). Its own binary: the switch is process-wide.
use std::path::Path;

#[test]
fn peck_cycles_certify_for_any_retract() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let dir = forge_dirs::temp_dir().join(format!("forge-cam-peck-{}", forge_dirs::process_id()));
    std::fs::create_dir_all(&dir).unwrap();
    // block_m6 made deep: a 20 mm hole of 5 mm (past 3 D: pecked, q = 5)
    let part = std::fs::read_to_string(src_dir.join("block_m6.fg"))
        .unwrap()
        .replace("shared t = 14", "shared t = 26")
        .replace("shared hd = 12", "shared hd = 20");
    std::fs::write(dir.join("block_m6.fg"), part).unwrap();
    let doc = std::fs::read_to_string(src_dir.join("block_m6_cam.fg"))
        .unwrap()
        .replace("to: 14, step", "to: 26, step")
        .replace("from: 14, to: -0.5", "from: 26, to: -0.5")
        .lines()
        .filter(|l| !l.starts_with("thr "))
        .collect::<Vec<_>>()
        .join("\n");
    let run = |params: (f64, f64)| {
        *forge_cam_lang::CYCLE_PARAMS.lock().unwrap() = params;
        let cam = forge_cam_lang::run(&doc, &dir);
        let ops: Vec<String> = cam
            .ops
            .iter()
            .map(|o| {
                let checks: Vec<String> = o
                    .checks
                    .iter()
                    .map(|c| format!("{} {}", c.rule, c.verdict))
                    .collect();
                format!("{} {} {:?}", o.name, o.status, checks)
            })
            .collect();
        (cam.ok, ops, cam.program_text().map(|t| t.0.to_string()))
    };
    let base = run((forge_post::PECK_CLEAR, forge_post::CHIP_LIFT));
    assert!(base.0, "{:?}", base.1);
    let txt = base.2.clone().unwrap();
    assert!(txt.contains("G83"), "a peck cycle");
    // any retract from 0 to a whole peck (q = 5)
    for p in [(0.0, 0.0), (0.1, 0.05), (1.0, 0.5), (2.5, 1.0), (5.0, 5.0)] {
        assert_eq!(run(p), base, "{p:?}");
    }
    std::fs::remove_dir_all(&dir).ok();
}
