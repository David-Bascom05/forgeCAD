//! Exercise the real document shell, checked PCB edits, save, and camera on native egui.
use forge_render::gui::Headless;
use std::path::Path;

#[test]
fn layout_window_edits_and_saves_the_source() {
    let Ok(mut h) = Headless::new((1500., 900.), 1.) else {
        eprintln!("No GPU adapter");
        return;
    };
    let dir = forge_dirs::temp_dir().join(format!("forge-pcb-screen-{}", forge_dirs::process_id()));
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/electronics");
    std::fs::create_dir_all(dir.join("footprints")).unwrap();
    let path = dir.join("filter-bank.fg");
    std::fs::copy(base.join("filter-bank.fg"), &path).unwrap();
    std::fs::copy(
        base.join("footprints/filter-bank.fg"),
        dir.join("footprints/filter-bank.fg"),
    )
    .unwrap();
    h.gui.open(path.to_str().unwrap());
    h.step();
    assert!(h.gui.shell.pcb.is_some(), "{}", h.gui.shell.status);
    assert!(h.click("pcb:placement:R4"));
    h.step();
    let shot = |h: &mut Headless, name: &str| {
        if let Some(out) = std::env::var_os("FORGE_UI_SHOTS") {
            std::fs::create_dir_all(&out).unwrap();
            std::fs::write(Path::new(&out).join(name), h.png().unwrap()).unwrap();
        }
    };
    shot(&mut h, "board-layout.png");
    assert!(h.click("tool:Rotate"));
    h.step();
    assert!(h.gui.shell.pcb.as_ref().unwrap().dirty());
    h.gui.fg_save();
    let read = || {
        forge_lang::pcb::compile(&std::fs::read_to_string(&path).unwrap(), Some(&dir), Some(&dir)).unwrap()
    };
    assert_eq!(read().layout.placements["R4"].pose.angle, 90.);
    h.gui.shell.pcb.as_mut().unwrap().step(false);
    h.gui.fg_save();
    assert_eq!(read().layout.placements["R4"].pose.angle, 0.);
    assert!(h.click("pcb:panel:Stackup"));
    h.step();
    shot(&mut h, "board-stackup.png");
    assert!(h.click("pcb:visible:copper.bottom"));
    assert!(h.click("pcb:layer:copper.top"));
    assert!(h.click("tool:Route Track"));
    h.step();
    let view = h.gui.shell.view_rect;
    h.click_at(view.center().x - 20., view.center().y);
    h.click_at(view.center().x + 80., view.center().y - 20.);
    h.key(egui::Key::Enter);
    h.step();
    h.gui.fg_save();
    assert_eq!(read().tracks.len(), 25);
    assert!(h.click("pcb:panel:DRC"));
    h.step();
    std::fs::remove_dir_all(dir).unwrap();
}
