//! The ribbon never clips (docs/ux/STANDARD.md §10): at 1024 and 900 px wide, every tool on each
//! tab is inside the window, or reached through its group folded into one button.
use forge_render::gui::Headless;

fn machined() -> std::path::PathBuf {
    std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/machined")).to_path_buf()
}

fn check(h: &mut Headless, what: &str, width: f32) {
    for _ in 0..3 {
        h.step();
    }
    let tools = h.gui.shell.ribbon_tools();
    assert!(!tools.is_empty(), "{what}: no ribbon");
    let inside = |r: egui::Rect| r.min.x >= -0.5 && r.max.x <= width + 0.5;
    for (group, tool) in tools {
        let direct = h.gui.shell.place(&format!("tool:{tool}")).filter(|r| inside(*r));
        let folded = h
            .gui
            .shell
            .place(&format!("ribbon:in:{group}:{tool}"))
            .filter(|r| inside(*r));
        assert!(
            direct.is_some() || folded.is_some(),
            "{what} at {width}: `{tool}` ({group}) clips or is missing"
        );
    }
}

#[test]
fn every_tool_fits_or_folds() {
    for width in [1024.0, 900.0, 760.0, 480.0] {
        let dir = std::env::temp_dir().join(format!("fg-ribbon-fit-{}-{width}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for f in ["flange.fg", "flange_cam.fg"] {
            std::fs::copy(machined().join(f), dir.join(f)).unwrap();
        }
        let mut h = match Headless::new((width, 700.0), 1.0) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("skipped: {e}");
                return;
            }
        };
        h.step();
        // a part: its Model, Inspect and View tabs (the sketch tab has its own ribbon)
        h.gui.open(dir.join("flange.fg").to_str().unwrap());
        h.until(|g| g.shell.fg.is_some());
        // (480 px: Manufacture only, where its groups fold; a part's ribbon is not asked to fit)
        if width > 700.0 {
            for tab in [0usize, 2, 3] {
                h.gui.shell.tab = tab;
                check(&mut h, &format!("part tab {tab}"), width);
            }
        }
        // a machining document: Manufacture
        h.gui.open(dir.join("flange_cam.fg").to_str().unwrap());
        h.until(|g| g.shell.cam.is_some());
        h.gui.shell.tab = forge_render::shell::CAM_TAB;
        check(&mut h, "Manufacture", width);
        if let (Ok(d), Ok(png)) = (std::env::var("FORGE_UI_SHOTS"), h.png()) {
            std::fs::write(
                std::path::Path::new(&d).join(format!("ribbon-cam-{width}.png")),
                png,
            )
            .unwrap();
        }
        assert!(h.gui.shell.cam.is_some(), "the machining document opened");
        // folded groups (the narrowest): one opens its menu, its tools placed in it
        let folded: Vec<String> = h
            .gui
            .shell
            .places
            .iter()
            .filter_map(|p| p.0.strip_prefix("ribbon:group:").map(str::to_string))
            .collect();
        if width < 500.0 {
            assert!(!folded.is_empty(), "nothing folded at {width}");
            let g = &folded[0];
            let inner: Vec<String> = h
                .gui
                .shell
                .ribbon_tools()
                .into_iter()
                .filter(|(gr, _)| gr == g)
                .map(|(_, t)| t.to_string())
                .collect();
            assert!(h.click(&format!("ribbon:group:{g}")));
            for _ in 0..3 {
                h.step();
            }
            for tool in &inner {
                let r = h
                    .gui
                    .shell
                    .place(&format!("tool:{tool}"))
                    .unwrap_or_else(|| panic!("`{tool}` not in {g}'s menu"));
                assert!(r.max.x <= width + 0.5, "{tool} at {r:?}");
            }
            if let (Ok(d), Ok(png)) = (std::env::var("FORGE_UI_SHOTS"), h.png()) {
                std::fs::write(std::path::Path::new(&d).join("ribbon-folded-menu.png"), png).unwrap();
            }
        }
    }
}
