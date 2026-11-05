//! The wall's colours on the screen, headless: the pixels by the MIN marker are the legend's MIN
//! colour, on the modelled part and the simulated one, the marker in the same place both ways.

use forge_render::gui::{Gui, Headless};
use forge_render::shell::thermo::BANDS;
use forge_render::shell::thermo_ui::Cmd;

fn settled(g: &Gui) -> bool {
    g.view.is_some() && g.fg.as_ref().is_some_and(|s| s.done == s.seq) && !g.is_loading()
}

/// Whether a pixel within a ring round `at` is `want` (sRGB bytes) to within `tol`.
fn ring_has(h: &Headless, at: (f32, f32), want: [u8; 3], tol: i32) -> bool {
    let px = h.gui_pixels();
    (6..16).any(|r| {
        (0..36).any(|k| {
            let a = k as f32 * std::f32::consts::TAU / 36.0;
            let (x, y) = (at.0 + r as f32 * a.cos(), at.1 + r as f32 * a.sin());
            if x < 0.0 || y < 0.0 {
                return false;
            }
            let c = px.at(x as usize, y as usize);
            (0..3).all(|i| (c[i] as i32 - want[i] as i32).abs() <= tol)
        })
    })
}

#[test]
fn the_min_markers_pixels_are_the_legends_min_colour() {
    let mut h = match Headless::new((1400.0, 900.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return;
        }
    };
    h.step();
    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/thermoform/cover.fg");
    h.gui.open(file);
    let t = std::time::Instant::now();
    while t.elapsed().as_secs() < 600 && !h.until(settled) {}
    let want = BANDS[9].map(|x| (x * 255.0).round() as u8);
    let mut markers = vec![];
    for sim in [false, true] {
        if let Some(p) = h.gui.shell.fg.as_mut() {
            p.thermo.simulated = sim;
            p.thermo.analysis = forge_render::shell::thermo_ui::Analysis::None;
            p.thermo_command(Cmd::Colours);
        }
        for _ in 0..30 {
            h.step();
        }
        let fg = h.gui.shell.fg.as_ref().unwrap();
        let at = fg.thermos[0].min.1;
        markers.push(at);
        let (camera, size) = h.gui.shell.camera.expect("a view");
        let v = h.gui.shell.view_rect;
        let px = camera.pixel_of(at, size.0, size.1).expect("on the screen");
        let screen = (v.min.x + px.0, v.min.y + px.1);
        // the frame, to look at when it fails
        let _ = std::fs::write(
            format!("{}/thermo-screen-{sim}.ppm", env!("CARGO_TARGET_TMPDIR")),
            h.ppm(),
        );
        assert!(
            ring_has(&h, screen, want, 10),
            "simulated {sim}: no MIN colour by the marker at {screen:?}"
        );
    }
    // the model's thinnest point is the same sheet point either way, off it by t or by the wall:
    // the two within the sheet's thickness and a half of each other (t = 2)
    let d: f32 = (0..3)
        .map(|k| (markers[0][k] - markers[1][k]).powi(2))
        .sum::<f32>()
        .sqrt();
    assert!(d < 3.0, "{d}");
}
