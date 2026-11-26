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
            "electronics:component:U1",
            "schematic",
        ),
        (1400, 860, "analog-symbols.fg", "electronics:schpin:VDD", "symbol"),
        (
            1024,
            700,
            "sensor-interface.fg",
            "electronics:component:C2",
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

#[test]
fn inspect_pin_and_compare_signal_displays() {
    let Ok(mut h) = Headless::new((1400., 860.), 1.) else {
        return;
    };
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/electronics/analog");
    let dir = forge_dirs::temp_dir().join(format!("forge-signal-{}", forge_dirs::process_id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(base.join("analog-symbols.fg"), dir.join("analog-symbols.fg")).unwrap();
    let source = std::fs::read_to_string(base.join("sensor-interface.fg")).unwrap()
        + "\nopposing_direction=flow(sheet:main,from:output,to:feedback)\n";
    let path = dir.join("signal-directions.fg");
    std::fs::write(&path, source).unwrap();
    h.gui.open(path.to_str().unwrap());
    h.step();
    let pin = h.placed("electronics:anchor:U1.VDD").unwrap().center();
    h.right_drag((pin.x, pin.y), (pin.x, pin.y));
    h.step();
    assert!(h.placed("electronics:inspection:main/U1.VDD").is_some());
    let shot = |h: &Headless, name: &str| {
        if let Some(out) = std::env::var_os("FORGE_UI_SHOTS") {
            std::fs::create_dir_all(&out).unwrap();
            std::fs::write(Path::new(&out).join(name), h.png().unwrap()).unwrap();
        }
    };
    shot(&h, "native-pin-inspection.png");
    h.key(egui::Key::Escape);
    h.step();
    assert!(h.click("electronics:dock:Parts"));
    h.step();
    assert!(h.click("electronics:part:analog.Amplifier"));
    h.step();
    shot(&h, "native-parts.png");
    assert!(h.click("electronics:dock:Power"));
    h.step();
    assert!(h.placed("electronics:schpower:+3V3").is_some());
    assert!(h.click("electronics:dock:Properties"));
    h.step();
    assert!(h.click("tab:View"));
    h.step();
    assert!(h.click("tool:Moving Dots"));
    h.step();
    assert!(h.click("tool:Direction Arrows"));
    h.step();
    shot(&h, "native-signal-directions.png");
    if let Some(folder) = std::env::var_os("FORGE_SIGNAL_FRAMES") {
        std::fs::create_dir_all(&folder).unwrap();
        for n in 0..32 {
            for _ in 0..3 {
                h.step();
            }
            std::fs::write(Path::new(&folder).join(format!("{n:02}.png")), h.png().unwrap()).unwrap();
        }
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn select_repeated_sheet_draw_and_change_grid() {
    let Ok(mut h) = Headless::new((1400., 860.), 1.) else {
        return;
    };
    let file =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/electronics/analog/filter-sheets.fg");
    h.gui.open(file.to_str().unwrap());
    h.step();
    assert!(h.click("electronics:sheet:main/left_filter"));
    h.step();
    assert!(h.placed("electronics:anchor:R1.B").is_some());
    assert!(h.click("tab:Draw"));
    h.step();
    assert!(h.click("tool:Rectangle"));
    h.step();
    assert!(h.click("electronics:grid"));
    for _ in 0..12 {
        h.step();
    }
    assert!(h.click("electronics:grid:50 mil"));
    for _ in 0..12 {
        h.step();
    }
    assert!(
        h.placed("electronics:pitch:1.27").is_some(),
        "50 mil pitch was not applied"
    );
    assert!(
        h.placed("electronics:grid:50 mil").is_none(),
        "Grid menu should close after selection"
    );
    if let Some(dir) = std::env::var_os("FORGE_UI_SHOTS") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(Path::new(&dir).join("native-drawing-grid.png"), h.png().unwrap()).unwrap();
    }
}
