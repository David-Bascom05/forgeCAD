//! Pictures of Capture for research/render/record/ (UX-LEAD's list), only with CAPTURE_SHOTS=DIR:
//! the ribbon with Capture in Bend Simulation and in Manufacture at 1400 and 1024 wide, the
//! settings window, a recording's pill, the result line after a photo, and the photo itself.

use forge_render::gui::{Gui, Headless};
use forge_render::shell::Action;
use forge_render::shell::capture::{Capture, Format, Size};
use forge_render::shell::env::EnvId;

fn settled(g: &Gui) -> bool {
    g.view.is_some() && g.fg.as_ref().is_some_and(|s| s.done == s.seq) && !g.loading()
}

fn at(size: (f32, f32), file: &str, env: EnvId) -> Option<Headless> {
    let mut h = Headless::new(size, 1.0).ok()?;
    h.step();
    h.gui.open(&format!("{}/../../examples/{file}", env!("CARGO_MANIFEST_DIR")));
    assert!(h.until(settled), "{file} never built: {}", h.gui.shell.status);
    h.gui.shell.out.push(Action::EnterEnv(env));
    for _ in 0..6 {
        h.step();
    }
    assert_eq!(h.gui.shell.env, Some(env), "{}", h.gui.shell.status);
    Some(h)
}

fn shot(h: &Headless, dir: &str, name: &str) {
    std::fs::write(format!("{dir}/{name}.png"), h.png().unwrap()).unwrap();
}

#[test]
fn shots_of_capture() {
    let Ok(dir) = std::env::var("CAPTURE_SHOTS") else {
        return;
    };
    for (w, h) in [(1400.0, 860.0), (1024.0, 700.0)] {
        let tag = format!("{}", w as u32);
        if let Some(hh) = at((w, h), "sheetmetal/load_tray.fg", EnvId::BendSim) {
            shot(&hh, &dir, &format!("01-ribbon-bendsim-{tag}"));
        }
        if let Some(hh) = at((w, h), "machined/flange_cam.fg", EnvId::Manufacture) {
            shot(&hh, &dir, &format!("02-ribbon-manufacture-{tag}"));
        }
    }
    let Some(mut h) = at((1400.0, 860.0), "sheetmetal/load_tray.fg", EnvId::BendSim) else {
        return;
    };
    let out = std::env::temp_dir().join(format!("fg-capture-shots-{}", std::process::id()));
    std::fs::create_dir_all(&out).unwrap();
    let c = &mut h.gui.shell.capture;
    c.folder = out.to_string_lossy().into_owned();
    c.settings_open = true;
    // past the window's fade-in
    for _ in 0..30 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    shot(&h, &dir, "03-settings");
    h.gui.shell.capture.settings_open = false;
    // a recording, caught part way
    let c = &mut h.gui.shell.capture;
    c.size = Size::Custom(320, 200);
    c.fps = 24;
    c.orbit_seconds = 4.0;
    c.format = Format::Frames;
    c.asks.push(Capture::Record);
    for _ in 0..60 {
        h.step();
    }
    shot(&h, &dir, "04-recording");
    h.gui.shell.capture.asks.push(Capture::Cancel);
    h.step();
    // a photo and its line
    h.gui.shell.capture.photo_size = Size::Custom(1280, 720);
    h.gui.shell.capture.asks.push(Capture::Photo);
    h.step();
    while h.gui.capture_busy() {
        h.step();
    }
    for _ in 0..3 {
        h.step();
    }
    shot(&h, &dir, "05-photo-saved");
    let photo = std::fs::read_dir(&out)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|x| x == "png"))
        .expect("the photo");
    std::fs::copy(photo, format!("{dir}/06-the-photo.png")).unwrap();
}
