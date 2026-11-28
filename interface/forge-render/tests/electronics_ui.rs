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
fn rename_a_library_pin_and_type_its_angle() {
    let Ok(mut h) = Headless::new((1400., 860.), 1.) else {
        return;
    };
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/electronics/analog");
    let dir = forge_dirs::temp_dir().join(format!("forge-pin-name-{}", forge_dirs::process_id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("analog-symbols.fg");
    std::fs::copy(base.join("analog-symbols.fg"), &path).unwrap();
    h.gui.open(path.to_str().unwrap());
    h.step();
    assert!(h.click("electronics:schpin:VDD"));
    h.step();
    for (key, value) in [
        ("electronics:pin-name", "AVDD"),
        ("electronics:pin-direction", "37.5"),
    ] {
        assert!(h.click(key));
        h.step();
        h.key_with(egui::Key::A, egui::Modifiers::COMMAND);
        h.text(value);
        h.step();
    }
    assert!(h.click("electronics:apply-pin"));
    h.step();
    assert!(
        h.placed("electronics:schpin:AVDD").is_some(),
        "{}",
        h.gui.shell.status
    );
    h.gui.fg_save();
    let source = std::fs::read_to_string(base.join("sensor-interface.fg")).unwrap();
    let (doc, circuit) = forge_lang::schematic::compile(&source, Some(&dir), None).unwrap();
    let sym = &doc.symbols["analog.Amplifier"];
    assert_eq!(sym.pins["AVDD"].angle, 37.5);
    assert_eq!(sym.pins["AVDD"].number, "5");
    assert!(!sym.pins.contains_key("VDD"));
    assert_eq!(circuit.links[0].want, "main/U1.AVDD");
    if let Some(out) = std::env::var_os("FORGE_UI_SHOTS") {
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(Path::new(&out).join("native-pin-edit.png"), h.png().unwrap()).unwrap();
    }
    std::fs::remove_dir_all(dir).unwrap();
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
    assert!(h.click("electronics:anchor:U1.VDD"));
    h.step();
    assert!(
        h.placed("electronics:check:main/U1.VDD:@analog.local_bypass")
            .is_some()
    );
    assert!(h.placed("electronics:rail:main/supply.net").is_some());
    h.key(egui::Key::Escape);
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
fn checklist_edits_the_assignment_from_either_pin() {
    let Ok(mut h) = Headless::new((1280., 800.), 1.) else {
        return;
    };
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/electronics/analog");
    let dir = forge_dirs::temp_dir().join(format!("forge-pin-checklist-{}", forge_dirs::process_id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(base.join("analog-symbols.fg"), dir.join("analog-symbols.fg")).unwrap();
    let original = std::fs::read_to_string(base.join("sensor-interface.fg")).unwrap();
    for (pin, peer) in [("U1.VDD", "C2.P"), ("C2.P", "U1.VDD")] {
        let path = dir.join(format!("sensor-{pin}.fg"));
        std::fs::write(&path, &original).unwrap();
        h.gui.open(path.to_str().unwrap());
        h.step();
        assert!(h.click(&format!("electronics:anchor:{pin}")));
        h.step();
        let check = format!("electronics:check:main/{pin}:@analog.local_bypass");
        assert!(h.click(&check));
        h.step();
        assert!(h.click("electronics:unassign:main/local_cap"));
        h.step();
        h.gui.fg_save();
        let src = std::fs::read_to_string(&path).unwrap();
        assert!(!src.contains("= fulfill("));
        assert!(h.click(&check));
        h.step();
        h.step(); // The dock enters Assign Role after the canvas has drawn.
        assert!(
            h.placed(&format!("electronics:assign:main/{peer}")).is_some(),
            "{}",
            h.gui.shell.status
        );
        // Deliver the earlier save's file notification while Assign Role is active.
        h.gui.shell.electronics.as_mut().unwrap().reload(true, Some(&src));
        if pin == "U1.VDD" {
            assert!(h.click(&format!("electronics:assign:main/{peer}")));
        } else {
            assert!(h.click(&format!("electronics:anchor:{peer}")));
        }
        h.step();
        assert!(
            h.placed(&format!("electronics:inspection:main/{pin}")).is_some(),
            "assigning {pin} and {peer}: {}",
            h.gui.shell.status
        );
        h.gui.fg_save();
        let src = std::fs::read_to_string(&path).unwrap();
        let (_, c) = forge_lang::schematic::compile(&src, Some(&dir), None).unwrap();
        assert_eq!(c.links.len(), 1);
        assert_eq!(c.links[0].want, "main/U1.VDD");
        assert_eq!(c.links[0].give, "main/C2.P");
        assert!(
            forge_lang::schematic::inspect_pin(&c, &format!("main/{pin}"))
                .unwrap()
                .checks
                .iter()
                .any(|r| r
                    .peers
                    .contains(&(format!("main/{peer}"), forge_lang::schematic::Evidence::Verified)))
        );
        for _ in 0..2 {
            h.gui.shell.electronics.as_mut().unwrap().step(false);
        }
        h.gui.fg_save();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
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

#[test]
fn symbol_prefix_drives_placement_and_allows_a_manual_reference() {
    let Ok(mut h) = Headless::new((1400., 860.), 1.) else {
        return;
    };
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/electronics/analog");
    let dir = forge_dirs::temp_dir().join(format!("forge-reference-prefix-{}", forge_dirs::process_id()));
    std::fs::create_dir_all(&dir).unwrap();
    let library = dir.join("analog-symbols.fg");
    std::fs::copy(base.join("analog-symbols.fg"), &library).unwrap();
    let sheet = dir.join("sensor-interface.fg");
    let source = std::fs::read_to_string(base.join("sensor-interface.fg")).unwrap()
        + "\nTP1 = junction(sheet: main, at: (-40,-20))\n";
    std::fs::write(&sheet, source).unwrap();
    h.gui.open(library.to_str().unwrap());
    h.step();
    assert!(h.click("electronics:schchip:Resistor"));
    h.step();
    if let Some(out) = std::env::var_os("FORGE_UI_SHOTS") {
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(
            Path::new(&out).join("native-reference-prefix.png"),
            h.png().unwrap(),
        )
        .unwrap();
    }
    assert!(h.click("electronics:ref-prefix"));
    h.step();
    h.key_with(egui::Key::A, egui::Modifiers::COMMAND);
    h.text("TP");
    h.step();
    assert!(h.click("electronics:apply-properties"));
    h.step();
    h.gui.fg_save();
    let saved = std::fs::read_to_string(&library).unwrap();
    let (doc, _) = forge_lang::schematic::compile(&saved, Some(&dir), None).unwrap();
    assert_eq!(doc.symbols["Resistor"].ref_prefix, "TP");
    h.gui.open(sheet.to_str().unwrap());
    h.step();
    assert!(h.click("electronics:dock:Parts"));
    h.step();
    for (x, manual) in [(550., false), (640., true)] {
        assert!(h.click("electronics:part:analog.Resistor"));
        h.step();
        if manual {
            assert!(h.click("electronics:place-reference"));
            h.step();
            h.key_with(egui::Key::A, egui::Modifiers::COMMAND);
            h.text("TP99");
            h.step();
        }
        h.click_at(x, 620.);
        h.step();
        assert!(h.click("electronics:dock:Parts"));
        h.step();
    }
    h.gui.fg_save();
    let saved = std::fs::read_to_string(&sheet).unwrap();
    let (doc, _) = forge_lang::schematic::compile(&saved, Some(&dir), None).unwrap();
    for name in ["R1", "TP2", "TP99"] {
        assert!(
            doc.rows.iter().any(|row| row.id == name
                && matches!(&row.item,
            forge_lang::schematic::Item::Place {symbol,..} if symbol == "analog.Resistor")),
            "{name}"
        );
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn checklist_explains_the_connected_supply_voltage() {
    let Ok(mut h) = Headless::new((1400., 860.), 1.) else {
        return;
    };
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/electronics/analog");
    let dir = forge_dirs::temp_dir().join(format!("forge-pin-voltage-{}", forge_dirs::process_id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(base.join("analog-symbols.fg"), dir.join("analog-symbols.fg")).unwrap();
    let original = std::fs::read_to_string(base.join("powered-amplifier.fg")).unwrap();
    for (mv, fits) in [(3300, true), (12000, false)] {
        let path = dir.join(format!("amplifier-{mv}mV.fg"));
        std::fs::write(
            &path,
            original.replace("voltage_mv: 3300", &format!("voltage_mv: {mv}")),
        )
        .unwrap();
        h.gui.open(path.to_str().unwrap());
        h.step();
        assert!(h.click("electronics:anchor:U1.VDD"));
        h.step();
        assert!(
            h.placed(&format!("electronics:voltage:main/U1.VDD:main/J1.P1:{fits}"))
                .is_some()
        );
        if let Some(out) = std::env::var_os("FORGE_UI_SHOTS") {
            std::fs::create_dir_all(&out).unwrap();
            std::fs::write(
                Path::new(&out).join(format!("native-supply-{mv}.png")),
                h.png().unwrap(),
            )
            .unwrap();
        }
        if fits {
            // Pin inspection refits the canvas on the next frame.
            h.step();
            h.step();
            assert!(h.click("electronics:anchor:J1.P1"));
            h.step();
            assert!(h.placed("electronics:inspection:main/J1.P1").is_some());
            assert!(h.click("electronics:placement-roles"));
            for _ in 0..15 {
                h.step();
            }
            let toggle = "electronics:placement-role:false:source";
            if let Some(out) = std::env::var_os("FORGE_UI_SHOTS") {
                std::fs::write(
                    Path::new(&out).join("native-connector-roles.png"),
                    h.png().unwrap(),
                )
                .unwrap();
            }
            assert!(h.click(toggle));
            // The supply row appears or disappears and moves the controls below it.
            for _ in 0..3 {
                h.step();
            }
            h.gui.fg_save();
            let saved = std::fs::read_to_string(&path).unwrap();
            let (doc, circuit) = forge_lang::schematic::compile(&saved, Some(&dir), None).unwrap();
            assert!(circuit.terminals["main/J1.P1"].gives.is_empty());
            assert_eq!(
                circuit.voltages["main/U1.VDD"][&doc.role_names["analog.supply_voltage"]].status(),
                "unknown"
            );
            assert!(h.click(toggle));
            // The supply row appears or disappears and moves the controls below it.
            for _ in 0..3 {
                h.step();
            }
            h.gui.fg_save();
            let saved = std::fs::read_to_string(&path).unwrap();
            let (doc, circuit) = forge_lang::schematic::compile(&saved, Some(&dir), None).unwrap();
            assert_eq!(
                circuit.voltages["main/U1.VDD"][&doc.role_names["analog.supply_voltage"]].status(),
                "satisfied"
            );
            assert!(doc.symbols["analog.Connector2"].pins["P1"].gives.is_empty());
        }
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn inspect_wiring_through_a_typed_component() {
    let Ok(mut h) = Headless::new((1400., 860.), 1.) else {
        return;
    };
    let file =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/electronics/analog/switched-load.fg");
    h.gui.open(file.to_str().unwrap());
    h.step();
    assert!(h.click("electronics:anchor:J1.P1"));
    h.step();
    assert!(
        h.placed("electronics:connection:main/J1.P1:load_return:true")
            .is_some()
    );
    if let Some(out) = std::env::var_os("FORGE_UI_SHOTS") {
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(
            Path::new(&out).join("native-pin-connection.png"),
            h.png().unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn browse_nested_library_and_place_the_leaf_symbol() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/electronics/load-switch");
    let Ok(mut h) = Headless::new((1600., 1000.), 1.) else {
        eprintln!("No headless GPU adapter");
        return;
    };
    let dir = std::env::temp_dir().join(format!("fg-nested-window-{}", std::process::id()));
    for file in [
        "switch-card.fg",
        "components/catalog.fg",
        "components/discrete/catalog.fg",
        "components/discrete/switches.fg",
    ] {
        let target = dir.join(file);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(base.join(file), target).unwrap();
    }
    let sheet = dir.join("switch-card.fg");
    h.gui.open(sheet.to_str().unwrap());
    h.step();
    assert!(h.click("electronics:sheet:overview/left"));
    h.step();
    assert!(h.click("electronics:dock:Parts"));
    h.step();
    assert!(h.click("electronics:part:components.discrete.switches.PowerFET"));
    h.step();
    if let Some(out) = std::env::var_os("FORGE_UI_SHOTS") {
        std::fs::create_dir_all(&out).unwrap();
        let left = h.placed("electronics:anchor:J1.IN").unwrap().center();
        let right = h.placed("electronics:anchor:Q1.source").unwrap().center();
        h.move_to((left.x + right.x) / 2., (left.y + right.y) / 2.);
        h.step();
        h.wheel(800.);
        for _ in 0..24 {
            h.step();
        }
        std::fs::write(Path::new(&out).join("nested-library.png"), h.png().unwrap()).unwrap();
    }
    h.click_at(750., 560.);
    h.step();
    h.gui.fg_save();
    let src = std::fs::read_to_string(&sheet).unwrap();
    let (_, c) = forge_lang::schematic::compile(&src, Some(&dir), None).unwrap();
    assert!(c.terminals.contains_key("overview/left/Q2.drain"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn inspect_i2c_pull_witness_in_the_native_window() {
    let Ok(mut h) = Headless::new((1500., 950.), 1.) else {
        return;
    };
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/electronics/i2c-bias.fg");
    h.gui.open(file.to_str().unwrap());
    h.step();
    assert!(h.click("electronics:anchor:U1.SDA"));
    h.step();
    assert!(
        h.placed("electronics:connection:main/U1.SDA:logic_high:true")
            .is_some()
    );
    if let Some(out) = std::env::var_os("FORGE_UI_SHOTS") {
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(Path::new(&out).join("i2c-pull.png"), {
            for _ in 0..3 {
                h.step();
            }
            h.png().unwrap()
        })
        .unwrap();
    }
}

#[test]
fn confirm_differential_net_labels_and_reopen() {
    let Ok(mut h) = Headless::new((1500., 950.), 1.) else {
        return;
    };
    let dir = std::env::temp_dir().join(format!("fg-diff-pair-ui-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("usb.fg");
    let sample = include_str!("../../../examples/electronics/usb-diff-pair.fg");
    let (unpaired, _) = sample.rsplit_once("usb = diff_pair").unwrap();
    std::fs::write(&file, unpaired).unwrap();
    h.gui.open(file.to_str().unwrap());
    h.step();
    h.key(egui::Key::F8);
    h.step();
    assert!(h.click("electronics:pair:USB_D+:USB_D-"));
    h.step();
    if let Some(out) = std::env::var_os("FORGE_UI_SHOTS") {
        std::fs::create_dir_all(&out).unwrap();
        for _ in 0..3 {
            h.step();
        }
        std::fs::write(Path::new(&out).join("differential-pair.png"), h.png().unwrap()).unwrap();
    }
    assert!(h.click("electronics:create-record"));
    h.step();
    h.gui.fg_save();
    let src = std::fs::read_to_string(file).unwrap();
    let (_, c) = forge_lang::schematic::compile(&src, Some(&dir), None).unwrap();
    assert_eq!(c.diff_pairs["main/pair1"], ["main/U1.DP", "main/U1.DM"]);
    std::fs::remove_dir_all(dir).unwrap();
}
