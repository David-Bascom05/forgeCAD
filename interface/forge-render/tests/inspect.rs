//! The Inspect tools in the app, headless: Measure reads what's picked on the exact body, the
//! section's planes reach the renderer, the Physical tab's mass is the part's, a property typed
//! is an op on the file, and the Material Browser's Assign sets the part's material. GPU tests:
//! skipped where there is none.

use forge_render::gui::{Gui, Headless};
use forge_render::render::Entity;
use forge_render::shell::Action;
use forge_render::shell::inspect::{InspectCmd, SectionKind};

const BLOCK: &str = "properties(part_number: \"T-1\", material: \"6061-T6\")
shared a = 40
shared b = 20
shared c = 10
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (a, b))
}
block = extrude(plan, c)
";

fn ready(g: &Gui) -> bool {
    g.view.is_some()
        && g.fg
            .as_ref()
            .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
}

fn open(tag: &str) -> Option<(Headless, std::path::PathBuf)> {
    let dir = std::env::temp_dir().join(format!("fg-inspect-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("block.fg");
    std::fs::write(&file, BLOCK).unwrap();
    let mut h = match Headless::new((1300.0, 900.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    h.step();
    h.gui.open(file.to_str().unwrap());
    assert!(h.until(ready), "never ready: {}", h.gui.shell.status);
    for _ in 0..3 {
        h.step();
    }
    Some((h, file))
}

#[test]
fn measure_reads_a_picked_face_on_the_exact_body() {
    let Some((mut h, _)) = open("measure") else { return };
    h.gui.shell.out.push(Action::Inspect(InspectCmd::Measure));
    h.step();
    // the top face: the one whose area is 40 x 20
    let faces = h.gui.fg.as_ref().map_or(0, |_| 6);
    let mut found = false;
    for f in 0..faces {
        if let Some(m) = &mut h.gui.shell.inspect.measure {
            m.picks = vec![(0, Entity::Face(f))];
        }
        h.until(|g| {
            g.shell
                .inspect
                .measure
                .as_ref()
                .is_some_and(|m| m.reading.is_some())
        });
        let m = h.gui.shell.inspect.measure.as_ref().unwrap();
        let r = m.reading.as_ref().unwrap().as_ref().unwrap();
        let area = r.get(forge_agent::measure::Quantity::Area).unwrap();
        assert!(!m.names.is_empty());
        if (area - 800.0).abs() < 1e-6 {
            found = true;
            break;
        }
    }
    assert!(found, "no face of 800 mm²");
    // the picks are what the view marks
    let v = h.gui.view.as_ref().unwrap();
    assert_eq!(v.nav.selected.len(), 1);
}

#[test]
fn a_section_reaches_the_renderer_and_ends() {
    let Some((mut h, _)) = open("section") else { return };
    h.gui.shell.out.push(Action::Inspect(InspectCmd::Section));
    h.step();
    h.step();
    let r = &h.gui.view.as_ref().unwrap().renderer;
    // the XZ plane through the block's middle, the near side cut away
    let s = r.section.expect("a plane");
    assert!((s[1] + 1.0).abs() < 1e-6 && (s[3] - 10.0).abs() < 1e-4, "{s:?}");
    assert!(r.section2.is_none() && !r.section_end);
    if let Some(c) = &mut h.gui.shell.inspect.cut {
        c.kind = SectionKind::Quarter;
    }
    h.step();
    h.step();
    let r = &h.gui.view.as_ref().unwrap().renderer;
    assert_eq!(r.section2.map(|x| x.1), Some(true));
    h.gui.shell.inspect.cut = None;
    h.step();
    h.step();
    assert!(h.gui.view.as_ref().unwrap().renderer.section.is_none());
}

#[test]
fn the_physical_tab_has_the_parts_mass_and_assign_changes_the_material() {
    let Some((mut h, file)) = open("physical") else {
        return;
    };
    h.gui.shell.out.push(Action::Inspect(InspectCmd::Mass));
    h.step();
    assert!(h.until(|g| g.shell.inspect.physical.is_some()));
    let p = h.gui.shell.inspect.physical.clone().unwrap().unwrap();
    let rho = forge_material::find("6061-T6").unwrap().density;
    assert!((p.mass.unwrap() - 8000.0 * rho * 1e-9).abs() < 1e-12);
    assert_eq!(h.gui.shell.inspect.doc.get("part_number"), Some("T-1"));
    // the browser's Assign: an op on the file, and the mass follows the new density
    h.gui.shell.inspect.props = None;
    h.gui.shell.out.push(Action::Inspect(InspectCmd::Materials));
    h.step();
    h.step();
    assert!(h.gui.shell.inspect.browser.is_some(), "the browser");
    assert!(h.click("materials:item:1018 cold drawn"), "no such row");
    h.step();
    assert!(h.click("materials:assign"), "no Assign button");
    h.step();
    let ok = h.until(|g| g.shell.inspect.doc.material.as_deref() == Some("1018 cold drawn") && ready(g));
    assert!(
        ok,
        "{:?} {:?} {}",
        h.gui.shell.inspect.doc.material,
        h.gui.shell.fg.as_ref().and_then(|f| f.refused.clone()),
        h.gui.shell.status
    );
    h.gui.shell.inspect.want_physical = true;
    h.step();
    let p = h.gui.shell.inspect.physical.clone().unwrap().unwrap();
    let steel = forge_material::find("1018 cold drawn").unwrap().density;
    assert!(
        (p.mass.unwrap() - 8000.0 * steel * 1e-9).abs() < 1e-12,
        "{:?}",
        p.mass
    );
    h.gui.fg_save();
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("material: \"1018 cold drawn\""), "{text}");
}
