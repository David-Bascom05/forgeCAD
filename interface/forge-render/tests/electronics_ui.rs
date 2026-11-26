//! Exercise the editor through the same object browser and ribbon used in the window.
use forge_render::gui::Headless;
use std::path::Path;

#[test]
fn electronic_document_windows() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/electronics/analog");
    for (width, height, file, selection, name) in [
        (
            1600,
            1000,
            "sensor-interface.fg",
            "electronics:schchip:U1",
            "schematic",
        ),
        (1400, 860, "analog-symbols.fg", "electronics:schpin:VDD", "symbol"),
        (
            1024,
            700,
            "sensor-interface.fg",
            "electronics:schchip:C2",
            "compact",
        ),
    ] {
        let Ok(mut h) = Headless::new((width as f32, height as f32), 1.) else {
            eprintln!("No headless GPU adapter, skipping electronics window capture");
            return;
        };
        h.gui.open(base.join(file).to_str().unwrap());
        h.step();
        assert!(h.gui.shell.electronics.is_some(), "{}", h.gui.shell.status);
        assert!(h.click(selection), "{selection}");
        for _ in 0..3 {
            h.step();
        }
        if let Some(dir) = std::env::var_os("FORGE_UI_SHOTS") {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                Path::new(&dir).join(format!("native-{name}.png")),
                h.png().unwrap(),
            )
            .unwrap();
        }
        assert!(h.click(if name == "symbol" { "tool:Pin" } else { "tool:Wire" }));
    }
}
