//! A file's path in the status line is said by its name, in Manufacture's own line too ("Wrote
//! <dir>/bracket_cam-sheet.html" reads "Wrote bracket_cam-sheet.html"), the whole line on hover.

use forge_render::gui::{Gui, Headless};

#[test]
fn manufactures_wrote_line_says_the_file_by_its_name() {
    let ex = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/machined"));
    let dir = std::env::temp_dir()
        .join(format!("forge-status-paths-{}", std::process::id()))
        .join("customer-acme-industrial/brackets-and-flanges/revision-c");
    std::fs::create_dir_all(&dir).unwrap();
    for e in std::fs::read_dir(ex).unwrap() {
        let e = e.unwrap();
        if e.path().is_file() {
            std::fs::copy(e.path(), dir.join(e.file_name())).unwrap();
        }
    }
    let Ok(mut h) = Headless::new((1400.0, 860.0), 1.0) else {
        eprintln!("no GPU adapter: skipped");
        return;
    };
    h.step();
    h.gui.open(dir.join("bracket_cam.fg").to_str().unwrap());
    let generated = |g: &Gui| {
        g.view.is_some()
            && g.fg
                .as_ref()
                .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
            && g.shell.cam.as_ref().is_some_and(|w| {
                w.running.is_none()
                    && !w.want_run
                    && w.run
                        .as_ref()
                        .is_some_and(|r| r.cam.is_none() || r.quality.is_some())
            })
    };
    assert!(h.until(generated), "never generated: {}", h.gui.shell.status);
    h.gui
        .shell
        .cam
        .as_mut()
        .unwrap()
        .command(forge_render::shell::cam::Cmd::SetupSheet);
    for _ in 0..30 {
        h.step();
    }
    assert!(
        h.gui.shell.status.contains("/revision-c/"),
        "{}",
        h.gui.shell.status
    );
    assert_eq!(h.gui.shell.status_shown, "Wrote bracket_cam-sheet.html");
    let _ = std::fs::remove_dir_all(dir.ancestors().nth(3).unwrap());
}
