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

#[test]
fn inspect_a_note_crossing_the_output_wire() {
    let Ok(mut h) = Headless::new((1600., 1000.), 1.) else {
        eprintln!("No headless GPU adapter, skipping readability capture");
        return;
    };
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/electronics/analog");
    let dir = forge_dirs::temp_dir().join(format!("forge-readability-ui-{}", forge_dirs::process_id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(base.join("analog-symbols.fg"), dir.join("analog-symbols.fg")).unwrap();
    let mut source = std::fs::read_to_string(base.join("sensor-interface.fg")).unwrap();
    source.push_str("\nscope_probe = note(sheet:main, at:(24,-0.75), value:\"Test point\")\n");
    let file = dir.join("sensor-readability.fg");
    std::fs::write(&file, &source).unwrap();
    let (_, circuit) = forge_lang::schematic::compile(&source, Some(&dir), None).unwrap();
    assert!(
        circuit
            .issues
            .iter()
            .any(|i| i.kind == "TextGeometryOverlap" && i.objects == ["main/scope_probe", "main/w7"])
    );
    h.gui.open(file.to_str().unwrap());
    h.step();
    assert!(h.click("tab:Inspect"));
    assert!(h.click("tool:Electrical Checks"));
    for _ in 0..3 {
        h.step();
    }
    if let Some(dir) = std::env::var_os("FORGE_UI_SHOTS") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(Path::new(&dir).join("native-readability.png"), h.png().unwrap()).unwrap();
    }
    std::fs::remove_dir_all(dir).unwrap();
}
