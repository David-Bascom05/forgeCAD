//! At 480 px (a narrow window, the UI audit's second size) every ribbon command on every tab is
//! reachable by a click: its own button on screen, or, when its group is folded, its row on
//! screen once the group's menu is open. Nothing on the overflow ladder (words, icons, a group's
//! menu) may lose a command.

use forge_render::gui::Headless;
use forge_render::shell::{HARNESS_TAB, THERMO_TAB};

const PLATE: &str = "shared t = 6\nsketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (60, 40))\n}\n\
plate = extrude(plan, t)\n";
const W: f32 = 480.0;
const H: f32 = 860.0;

fn on_screen(r: egui::Rect) -> bool {
    r.width() > 4.0
        && r.height() > 4.0
        && r.min.x >= -0.5
        && r.min.y >= -0.5
        && r.max.x <= W + 0.5
        && r.max.y <= H + 0.5
}

fn click(h: &mut Headless, name: &str) -> bool {
    let Some(r) = h.placed(name).filter(|r| on_screen(*r)) else {
        return false;
    };
    h.move_to(r.center().x, r.center().y);
    h.step();
    h.button(true);
    h.step();
    h.button(false);
    // one frame, as a quick hand: a menu opened by it must already say where its rows are
    h.step();
    true
}

/// Every tab by a click (in the row, or through »), and on it every ribbon command: its own
/// button on screen, or its row on screen once its folded group's menu is open.
fn every_command(h: &mut Headless, what: &str) -> usize {
    let (mut lost, mut checked) = (Vec::new(), 0);
    // the tabs: those in the row, then those behind »
    let tabs = |h: &Headless| -> Vec<String> {
        h.gui
            .shell
            .places
            .iter()
            .filter_map(|p| p.0.strip_prefix("tab:").map(str::to_string))
            .collect()
    };
    let mut names = tabs(h);
    if click(h, "tabs:more") {
        for _ in 0..3 {
            h.step();
        }
        names.extend(
            tabs(h)
                .into_iter()
                .filter(|n| !names.contains(n))
                .collect::<Vec<_>>(),
        );
        h.key(egui::Key::Escape);
        for _ in 0..3 {
            h.step();
        }
    }
    // the tabs the row draws: the order, less Harness and Thermoform where there are none
    let s = &h.gui.shell;
    let want = s.tab_order().len()
        - usize::from(s.tab_order().contains(&HARNESS_TAB) && !s.harness_tab())
        - usize::from(s.tab_order().contains(&THERMO_TAB) && !s.thermo_tab());
    assert!(
        names.len() >= want,
        "{what}: {} tabs reachable of {want}: {names:?}",
        names.len()
    );
    for name in names {
        // the last choice's menu gone, and its rows with it
        for _ in 0..2 {
            h.step();
        }
        let tab = format!("tab:{name}");
        let was = h.gui.shell.tab;
        if h.gui.shell.place(&tab).is_none_or(|r| !on_screen(r)) {
            click(h, "tabs:more");
        }
        let k = h.gui.shell.tab;
        if !click(h, &tab)
            || (h.gui.shell.tab == was && k == was && !h.gui.shell.place(&tab).is_some_and(on_screen))
        {
            let row: Vec<String> = h
                .gui
                .shell
                .places
                .iter()
                .filter(|p| p.0.starts_with("tab"))
                .map(|p| format!("{} {:.0},{:.0}", p.0, p.1.min.x, p.1.min.y))
                .collect();
            lost.push(format!(
                "{what}: the tab {name} (on {}, row {row:?})",
                h.gui.shell.tab
            ));
            h.key(egui::Key::Escape);
            continue;
        }
        for (group, tool, _) in h.gui.shell.ribbon_live() {
            checked += 1;
            if h.gui.shell.place(&format!("tool:{tool}")).is_some_and(on_screen) {
                continue;
            }
            // folded: open the group's menu, find the row, close it again
            let reached = click(h, &format!("ribbon:group:{group}"))
                && h.placed(&format!("tool:{tool}")).is_some_and(on_screen);
            h.key(egui::Key::Escape);
            for _ in 0..3 {
                h.step();
            }
            if !reached {
                lost.push(format!("{what}: {name} › {group} › {tool}"));
            }
        }
    }
    assert!(lost.is_empty(), "unreachable at 480 px: {lost:#?}");
    checked
}

#[test]
fn at_480_px_every_tab_and_ribbon_command_can_be_clicked() {
    let Ok(mut h) = Headless::new((W, H), 1.0) else {
        eprintln!("skipped: no GPU");
        return;
    };
    for _ in 0..6 {
        h.step();
    }
    let empty = every_command(&mut h, "no document");
    let d = std::env::temp_dir().join(format!("fg-narrow-reach-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("plate.fg"), PLATE).unwrap();
    h.gui.open(d.join("plate.fg").to_str().unwrap());
    assert!(h.until(|g| g.fg.as_ref().is_some_and(|s| s.seq > 0 && s.done == s.seq)));
    for _ in 0..6 {
        h.step();
    }
    let plate = every_command(&mut h, "a part");
    eprintln!("{empty} commands with no document, {plate} on a part");
    assert!(empty > 20 && plate > 20);
}
