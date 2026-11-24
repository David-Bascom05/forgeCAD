//! The staged stock is skipped where every body checked against it stays the clearance above
//! its top (Heights.lean): every example's checks, verdicts and program are the same with the
//! stock built anyway (forge_cam_lang::FULL_STOCK). Its own test binary: the switch is process-wide.
use std::path::Path;

#[test]
fn skipping_the_staged_stock_changes_no_verdict() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let mut docs: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.to_string_lossy().ends_with("_cam.fg"))
        .collect();
    docs.sort();
    assert!(docs.len() >= 7);
    let run = |full: bool| -> Vec<String> {
        forge_cam_lang::FULL_STOCK.store(full, std::sync::atomic::Ordering::Relaxed);
        docs.iter()
            .map(|p| {
                let cam = forge_cam_lang::run(&std::fs::read_to_string(p).unwrap(), &dir);
                let ops: Vec<String> = cam
                    .ops
                    .iter()
                    .map(|o| {
                        let checks: Vec<String> = o
                            .checks
                            .iter()
                            .map(|c| format!("{} {} {} {}", c.rule, c.verdict, c.move_index, c.other))
                            .collect();
                        format!("{} {} {:?}", o.name, o.status, checks)
                    })
                    .collect();
                format!("{} {:?} {:?}", p.display(), ops, cam.program_text().map(|t| t.0))
            })
            .collect()
    };
    let (skip, full) = (run(false), run(true));
    for (a, b) in skip.iter().zip(&full) {
        assert_eq!(a, b);
    }
}
