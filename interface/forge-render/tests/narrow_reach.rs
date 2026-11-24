//! At 480 px (a narrow window, the UI audit's second size) every ribbon command on every tab is
//! reachable by a click: its own button on screen, or, when its group is folded, its row on
//! screen once the group's menu is open. Nothing on the overflow ladder (words, icons, a group's
//! menu) may lose a command.

use forge_render::gui::Headless;

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

#[test]
fn at_480_px_every_ribbon_command_can_be_clicked() {
    let Ok(mut h) = Headless::new((W, H), 1.0) else {
        eprintln!("skipped: no GPU");
        return;
    };
    let d = std::env::temp_dir().join(format!("fg-narrow-reach-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("plate.fg"), PLATE).unwrap();
    h.gui.open(d.join("plate.fg").to_str().unwrap());
    assert!(h.until(|g| g.fg.as_ref().is_some_and(|s| s.seq > 0 && s.done == s.seq)));
    for _ in 0..6 {
        h.step();
    }
    let (mut lost, mut checked, mut folded) = (Vec::new(), 0, 0);
    for k in h.gui.shell.tab_order() {
        h.gui.shell.tab = k;
        for _ in 0..4 {
            h.step();
        }
        for (group, tool, _) in h.gui.shell.ribbon_live() {
            checked += 1;
            if h.gui.shell.place(&format!("tool:{tool}")).is_some_and(on_screen) {
                continue;
            }
            // folded: open the group's menu, find the row, close it again
            let g = format!("ribbon:group:{group}");
            let reached = h.gui.shell.place(&g).is_some_and(on_screen) && {
                h.click(&g);
                for _ in 0..3 {
                    h.step();
                }
                let there = h.gui.shell.place(&format!("tool:{tool}")).is_some_and(on_screen);
                h.key(egui::Key::Escape);
                for _ in 0..3 {
                    h.step();
                }
                there
            };
            if reached {
                folded += 1;
            } else {
                lost.push(format!("tab {k}: {group}/{tool}"));
            }
        }
    }
    eprintln!("{checked} commands, {folded} through a folded group's menu");
    assert!(checked > 20, "only {checked} ribbon commands seen");
    assert!(lost.is_empty(), "unreachable at 480 px: {lost:#?}");
}
