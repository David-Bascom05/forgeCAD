//! Every floating panel and dialog: what it names is where it's drawn from its first seen
//! frame, and the view never takes the pointer through it. GPU tests: skipped where there's none.

use forge_render::gui::{Gui, Headless};
use forge_render::shell::Action;
use forge_render::shell::inspect::InspectCmd;

fn ready(g: &Gui) -> bool {
    g.view.is_some()
        && g.fg
            .as_ref()
            .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
}

fn open(tag: &str) -> Option<Headless> {
    let dir = std::env::temp_dir().join(format!("fg-windows-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let demo = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/demo"));
    for e in std::fs::read_dir(demo).unwrap().flatten() {
        if e.path().is_file() {
            std::fs::copy(e.path(), dir.join(e.file_name())).unwrap();
        }
    }
    let mut h = match Headless::new((1400.0, 900.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    h.step();
    h.gui.open(dir.join("part_tools.fg").to_str().unwrap());
    assert!(h.until(ready), "never ready: {}", h.gui.shell.status);
    for _ in 0..5 {
        h.step();
    }
    Some(h)
}

/// A panel opened: from the first frame its `name` is placed, that place stays put, and egui
/// says the panel, not the view, is under it.
fn check(h: &mut Headless, what: &str, name: &str) {
    let mut first = None;
    for _ in 0..10 {
        h.step();
        let now = h.gui.shell.place(name);
        if let Some(r) = now {
            let was = *first.get_or_insert(r);
            assert_eq!(was, r, "{what}: {name} moved after it was first named");
        }
    }
    let r = first.unwrap_or_else(|| panic!("{what}: nothing named {name}"));
    let layer = h.gui.ctx.layer_id_at(r.center());
    assert!(
        layer.is_some_and(|l| l.order != egui::Order::Background),
        "{what}: the view, not the panel, is under {name}: {layer:?}"
    );
}

#[test]
fn every_panel_is_placed_from_its_first_frame_and_is_over_the_view() {
    let Some(mut h) = open("all") else { return };
    h.gui.shell.out.push(Action::Inspect(InspectCmd::Measure));
    check(&mut h, "Measure", "measure:filter:Faces");
    h.gui.shell.inspect.measure = None;
    h.gui.shell.out.push(Action::Inspect(InspectCmd::Properties));
    check(&mut h, "iProperties", "iprops:close:close");
    h.gui.shell.out.push(Action::Inspect(InspectCmd::Properties));
    h.gui.shell.out.push(Action::Inspect(InspectCmd::Materials));
    check(&mut h, "Material Browser", "materials:search");
    h.gui.shell.out.push(Action::Inspect(InspectCmd::Materials));
    h.gui.shell.prefs.open = true;
    check(&mut h, "Settings", "settings:close:close");
    h.gui.shell.prefs.open = false;
    h.gui.shell.content.open = true;
    check(&mut h, "Content Center", "cc:close");
    h.gui.shell.content.open = false;
    h.gui.shell.fg.as_mut().unwrap().params_open = true;
    check(&mut h, "Parameters", "fg:params:close");
    h.gui.shell.fg.as_mut().unwrap().params_open = false;
    h.gui.fg_open_tool("extrude");
    check(&mut h, "Extrude", "fg:tool:name");
}
