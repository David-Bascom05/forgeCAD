//! The Haas VF-2 and UMC-750 as the Machine Builder shows them (front 3/4, framed), for review:
//! `FORGE_UI_SHOTS=DIR cargo test --release --test haas_shots`. Their bodies are envelopes from
//! the published spec sheets (the records' own cites say so), not the maker's CAD.

use forge_render::gui::{Gui, Headless};
use std::path::{Path, PathBuf};

const EXAMPLES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples");

fn ready(g: &Gui) -> bool {
    g.view.is_some()
        && g.fg
            .as_ref()
            .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
}

fn project() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fg-haas-shots-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for e in std::fs::read_dir(Path::new(EXAMPLES).join("demo"))
        .unwrap()
        .flatten()
    {
        if e.path().is_file() {
            std::fs::copy(e.path(), dir.join(e.file_name())).unwrap();
        }
    }
    dir.join("part_tools.fg")
}

#[test]
fn haas_vf2_and_umc750() {
    let mut h = match Headless::new((1600.0, 1000.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return;
        }
    };
    h.step();
    let part = project();
    h.gui.open(part.to_str().unwrap());
    let t = std::time::Instant::now();
    while t.elapsed().as_secs_f64() < 60.0 && !h.until(ready) {}
    assert!(h.click("tab:Manage"));
    h.step();
    assert!(h.click("tool:Machine Builder"));
    h.step();
    for name in ["haas-vf-2", "haas-umc-750"] {
        h.gui.shell.machine.as_mut().unwrap().pick_record(name).unwrap();
        for _ in 0..14 {
            h.step();
        }
        assert_eq!(
            h.gui.shell.machine.as_ref().unwrap().record_name().as_deref(),
            Some(name)
        );
        if let Ok(dir) = std::env::var("FORGE_UI_SHOTS") {
            std::fs::create_dir_all(&dir).unwrap();
            if let Ok(png) = h.png() {
                std::fs::write(Path::new(&dir).join(format!("{name}.png")), png).unwrap();
            }
        }
    }
    let _ = std::fs::remove_dir_all(part.parent().unwrap());
}
