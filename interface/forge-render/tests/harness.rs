//! Wire harnesses in the app: examples/harness/robot.fg opened, its Harness tab and tools, the
//! browser's Harness branch, the tubes placed and lit by their rows, the status line, and a wire
//! added through Add Wire's dialog (one undo step).

use forge_render::gui::{Gui, Headless};
use forge_render::shell::harness_ui::{Cmd, Dialog};
use forge_render::shell::{FgCmd, HARNESS_TAB};
use std::path::{Path, PathBuf};

const EXAMPLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/harness");

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fg-harness-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    for f in ["header.fg", "plate.fg", "robot.fg"] {
        std::fs::copy(Path::new(EXAMPLE).join(f), d.join(f)).unwrap();
    }
    d
}

fn ready(g: &Gui) -> bool {
    g.view.is_some()
        && g.fg_asm.is_some()
        && g.fg.as_ref().is_some_and(|s| s.done == s.seq)
        && g.shell
            .fg
            .as_ref()
            .is_some_and(|p| p.asm.as_ref().is_some_and(|a| !a.harnesses.is_empty()))
}

fn settle(h: &mut Headless) {
    let t = std::time::Instant::now();
    while t.elapsed().as_secs_f64() < 20.0 {
        if h.until(ready) {
            break;
        }
    }
    assert!(ready(&h.gui), "never ready: {}", h.gui.shell.status);
    for _ in 0..3 {
        h.step();
    }
}

fn open(file: &Path) -> Option<Headless> {
    let mut h = match Headless::new((1400.0, 1000.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    h.step();
    h.gui.open(file.to_str().unwrap());
    settle(&mut h);
    Some(h)
}

fn asm(h: &Headless) -> &forge_render::shell::fgasm::AsmPanel {
    h.gui.shell.fg.as_ref().unwrap().asm.as_ref().unwrap()
}

#[test]
fn the_harness_shows_in_the_tab_the_browser_the_view_and_the_status_bar() {
    let d = scratch("show");
    let Some(mut h) = open(&d.join("robot.fg")) else {
        return;
    };
    // the tab and its tools
    assert!(h.click("tab:Harness"), "no Harness tab");
    h.step();
    h.step();
    assert_eq!(h.gui.shell.tab, HARNESS_TAB);
    for tool in [
        "Create Harness",
        "Add Connector or\u{a0}Pin",
        "Route Segment",
        "Add Wire",
        "Route Wires",
        "Import Wire List",
        "Export Wire List",
    ] {
        assert!(h.gui.shell.place(&format!("tool:{tool}")).is_some(), "no {tool}");
    }
    // the browser's branch: the harness, its segments with their wires, its wires
    for key in ["h:loom", "h:loom:c", "h:loom:s", "h:loom:w"] {
        assert!(asm(&h).tree.find(key).is_some(), "no row {key}");
    }
    let trunk = asm(&h).tree.find("h:loom:s:trunk").unwrap();
    assert_eq!(asm(&h).tree.nodes[trunk].kids.len(), 14);
    assert!(
        asm(&h).tree.nodes[trunk]
            .detail
            .starts_with("PET braid 8 · Ø8.6 · R25.8"),
        "{}",
        asm(&h).tree.nodes[trunk].detail
    );
    // each wire its own tube, in its colour; the braided trunk's sleeve
    use forge_render::fg::HarnessPiece as P;
    let placed = asm(&h).harness_placed.clone();
    assert_eq!(
        placed.iter().filter(|p| matches!(p.1, P::Wire(_))).count(),
        15,
        "the loom's 14 and the power cable"
    );
    assert!(placed.iter().any(|p| p.1 == P::Sleeve(0)));
    assert!(placed.iter().all(|p| !matches!(p.1, P::Sleeve(k) if k > 0)));
    assert!(
        h.gui.shell.status.contains("15 wires routed"),
        "{}",
        h.gui.shell.status
    );
    let tint = |h: &Headless, piece: P| {
        let m = placed.iter().find(|p| p.1 == piece).unwrap().2;
        h.gui.view.as_ref().unwrap().scene.placements[m].tint
    };
    let red = forge_lang::harness::rgb("RD", "W1");
    assert_eq!(tint(&h, P::Wire(0)), [red[0], red[1], red[2], 0.0]);
    assert_ne!(tint(&h, P::Wire(0)), tint(&h, P::Wire(1)));
    // a segment's row lights its wires
    asm_select(&mut h, "h:loom:s:to_a");
    for k in 0..14 {
        let lit = tint(&h, P::Wire(k)) == forge_render::fg::PICKED_TINT;
        assert_eq!(lit, k < 4, "wire {k}");
    }
    // the harnesses' eyes hide their tubes (the loom's, and the power cable's)
    let hidden = &mut h
        .gui
        .shell
        .fg
        .as_mut()
        .unwrap()
        .asm
        .as_mut()
        .unwrap()
        .harness_hidden;
    hidden.insert("loom".into());
    hidden.insert("power".into());
    h.gui.asm_show();
    let view = h.gui.view.as_ref().unwrap();
    assert!(
        placed
            .iter()
            .all(|&(_, _, m)| view.scene.rows_of(m) == [[0.0; 4]; 3])
    );
}

fn asm_select(h: &mut Headless, key: &str) {
    h.gui.shell.fg.as_mut().unwrap().asm.as_mut().unwrap().selected = Some(key.into());
    h.gui.asm_show();
    h.step();
}

#[test]
fn add_wire_writes_its_line_in_the_harness_and_it_routes() {
    let d = scratch("wire");
    // a header two pins longer, and a sensor plug too: pins with no wire yet
    let robot = d.join("robot.fg");
    let src = std::fs::read_to_string(&robot).unwrap();
    let src = src
        .replace("shared n_ctrl = count(14)", "shared n_ctrl = count(16)")
        .replace("shared n_sens = count(6)", "shared n_sens = count(8)");
    std::fs::write(&robot, src).unwrap();
    let Some(mut h) = open(&robot) else {
        return;
    };
    h.gui.shell.fg_command(FgCmd::Harness(Cmd::Wire));
    h.step();
    {
        let p = h.gui.shell.fg.as_mut().unwrap();
        let Some(Dialog::Wire {
            name, from, to, ty, ..
        }) = &mut p.harness.dialog
        else {
            panic!("no Add Wire dialog: {:?}", p.harness.dialog)
        };
        assert_eq!(name, "W15");
        // its ends offered at the first pins no wire is on yet
        assert_eq!((from.as_str(), to.as_str()), ("J1", "J4"));
        *ty = "s26".into();
    }
    let (f, t) = match &h.gui.shell.fg.as_ref().unwrap().harness.dialog {
        Some(Dialog::Wire {
            from,
            from_pin,
            to,
            to_pin,
            ..
        }) => (format!("{from}.{from_pin}"), format!("{to}.{to_pin}")),
        _ => unreachable!(),
    };
    let before: Vec<(String, Vec<String>)> = asm(&h).harnesses[0]
        .wires
        .iter()
        .map(|w| (w.from.clone(), w.path.clone()))
        .collect();
    let ends: Vec<String> = asm(&h).harnesses[0]
        .wires
        .iter()
        .flat_map(|w| [w.from.clone(), w.to.clone()])
        .collect();
    assert!(
        !ends.contains(&f) && !ends.contains(&t),
        "{f} or {t} has a wire on it already"
    );
    h.step();
    assert!(h.click("harness:dialog:ok"), "no OK");
    h.step();
    settle(&mut h);
    let src = h.gui.fg.as_ref().unwrap().src.clone();
    assert!(src.contains("  W15 = wire("), "{src}");
    let w = &asm(&h).harnesses[0].wires;
    assert_eq!(w.len(), 15);
    assert_eq!((w[14].from.as_str(), w[14].to.as_str()), (f.as_str(), t.as_str()));
    assert!(w[14].routed.is_some(), "the new wire routes");
    // the others keep the ways they took
    for (a, b) in before.iter().zip(w) {
        assert_eq!(a.1, b.path);
    }
    assert!(h.gui.shell.fg.as_ref().unwrap().harness.dialog.is_none());
}
