//! Sketch text in the app: the sketcher's Text tool places a text at the click, its dialog sets
//! its lines, font, size, style, alignment and angle, Finish writes `text(..)` in the block (its
//! numbers params of the sketch), and an opened sketch with text stays editable: a moved text
//! sets its params, a changed one is rewritten, a deleted one taken out. `FORGE_UI_SHOTS=DIR`
//! keeps pictures.

use forge_render::gui::{Gui, Headless};
use forge_render::shell::Action;
use forge_render::shell::fg::FgAct;
use forge_render::shell::sketching::SkTool as T;

fn ready(g: &Gui) -> bool {
    g.view.is_some()
        && g.fg
            .as_ref()
            .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
}

fn settle(h: &mut Headless) {
    let t = std::time::Instant::now();
    while t.elapsed().as_secs_f64() < 30.0 {
        if h.until(ready) {
            break;
        }
    }
    assert!(ready(&h.gui), "never ready: {}", h.gui.shell.status);
    let _ = h.until(|g| g.view.as_ref().is_some_and(|v| !v.nav.moving()));
    for _ in 0..4 {
        h.step();
    }
}

fn shot(h: &mut Headless, name: &str) {
    for _ in 0..12 {
        h.step();
    }
    if let Ok(dir) = std::env::var("FORGE_UI_SHOTS")
        && let Ok(png) = h.png()
    {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(std::path::Path::new(&dir).join(format!("{name}.png")), png).unwrap();
    }
}

const PART: &str = "shared t = 4
sketch base(on: xy) {
  r = rect(corner: (0, 0), size: (60, 30))
}
plate = extrude(base.r, t)
sketch lab(on: plate.end) {
  l1 = line(near(0, 0), near(5, 0))
}
";

fn open() -> Option<Headless> {
    let dir = std::env::temp_dir().join(format!("fg-sktext-ui-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("plate.fg");
    std::fs::write(&file, PART).unwrap();
    let _ = std::fs::remove_file(forge_lang::undo::log_path(&file));
    let mut h = match Headless::new((1500.0, 1000.0), 1.0) {
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

fn source(h: &Headless) -> String {
    h.gui.shell.fg.as_ref().unwrap().source.clone()
}

fn edit_sketch(h: &mut Headless, id: &str) {
    h.gui
        .shell
        .fg
        .as_mut()
        .unwrap()
        .out
        .push(FgAct::EditSketch(id.into()));
    for _ in 0..4 {
        h.step();
    }
    assert!(h.gui.shell.sketch.is_some(), "{}", h.gui.shell.status);
    settle(h);
}

fn finish(h: &mut Headless) {
    h.gui.shell.out.push(Action::FinishSketch);
    h.step();
    assert!(
        h.gui.shell.sketch.is_none(),
        "{:?} {}",
        h.gui.shell.fg.as_ref().unwrap().refused,
        h.gui.shell.status
    );
    settle(h);
}

fn errors(h: &Headless) -> Vec<String> {
    h.gui
        .shell
        .fg
        .as_ref()
        .unwrap()
        .issues
        .iter()
        .filter(|i| i.severity == forge_render::shell::fg::Severity::Error)
        .map(|i| i.text.clone())
        .collect()
}

#[test]
fn text_is_placed_styled_written_reopened_moved_and_deleted() {
    let Some(mut h) = open() else { return };
    // the sketch on the plate's top, the Text tool, a click where the baseline starts
    edit_sketch(&mut h, "lab");
    h.gui.shell.out.push(Action::SketchTool(T::SketchText));
    h.step();
    h.gui.shell.sketch.as_mut().unwrap().click((10.0, 12.0), 0.5);
    for _ in 0..4 {
        h.step();
    }
    assert!(h.gui.shell.place("sktext:dialog").is_some(), "no Text dialog");
    {
        let m = h.gui.shell.sketch.as_mut().unwrap();
        assert_eq!(m.more.sktexts.len(), 1);
        let s = &mut m.more.sktexts[0];
        assert_eq!((s.x, s.y), (10.0, 12.0));
        s.text = "AB-1\nCD".into();
        s.size = 6.0;
    }
    assert!(h.click("sktext:bold"));
    h.step();
    assert!(h.click("sktext:align:Center"));
    h.step();
    shot(&mut h, "sktext-1-dialog");
    assert!(h.click("sktext:done"));
    for _ in 0..3 {
        h.step();
    }
    assert_eq!(h.gui.shell.sketch.as_ref().unwrap().more.sktext_open, None);
    assert!(h.gui.shell.place("sktext:dialog").is_none());
    finish(&mut h);
    let src = source(&h);
    assert!(
        src.contains(
            "t1 = text([\"AB-1\", \"CD\"], at: (t1_x, t1_y), size: t1_size, bold: true, align: \"center\")"
        ),
        "{src}"
    );
    for p in [
        "param lab.t1_x = 10",
        "param lab.t1_y = 12",
        "param lab.t1_size = 6",
    ] {
        assert!(src.contains(p), "{p}\n{src}");
    }
    assert!(errors(&h).is_empty(), "{:?}", errors(&h));

    // opened again: editable, the text held with the params it reads
    edit_sketch(&mut h, "lab");
    assert!(
        !h.gui.shell.status.contains("read-only"),
        "{}",
        h.gui.shell.status
    );
    {
        let m = h.gui.shell.sketch.as_mut().unwrap();
        assert_eq!(m.more.sktexts.len(), 1);
        let s = &mut m.more.sktexts[0];
        assert_eq!(s.name.as_deref(), Some("t1"));
        assert!(s.bold);
        s.x = 14.0;
        s.angle = 15.0;
        s.text = "AB-2".into();
    }
    shot(&mut h, "sktext-2-reopened");
    finish(&mut h);
    let src = source(&h);
    assert!(src.contains("param lab.t1_x = 14"), "{src}");
    assert!(
        src.contains("t1 = text(\"AB-2\", at: (t1_x, t1_y), size: t1_size, bold: true, align: \"center\", angle: 15deg)"),
        "{src}"
    );
    assert!(errors(&h).is_empty(), "{:?}", errors(&h));

    // a click on its start opens it; Delete takes it out
    edit_sketch(&mut h, "lab");
    h.gui.shell.out.push(Action::SketchTool(T::SketchText));
    h.step();
    h.gui.shell.sketch.as_mut().unwrap().click((14.0, 12.0), 0.5);
    for _ in 0..4 {
        h.step();
    }
    assert_eq!(h.gui.shell.sketch.as_ref().unwrap().more.sktext_open, Some(0));
    assert!(h.click("sktext:delete"));
    h.step();
    finish(&mut h);
    let src = source(&h);
    assert!(!src.contains("text("), "{src}");
    assert!(src.contains("l1 = line("), "{src}");
}

#[test]
fn a_quote_in_the_text_holds_finish_and_says_why() {
    let Some(mut h) = open() else { return };
    edit_sketch(&mut h, "lab");
    h.gui.shell.out.push(Action::SketchTool(T::SketchText));
    h.step();
    h.gui.shell.sketch.as_mut().unwrap().click((5.0, 5.0), 0.5);
    h.step();
    h.gui.shell.sketch.as_mut().unwrap().more.sktexts[0].text = "say \"hi\"".into();
    h.gui.shell.out.push(Action::FinishSketch);
    h.step();
    // refused: the sketch stays open, the file as it was
    assert!(h.gui.shell.sketch.is_some());
    assert!(!source(&h).contains("text("));
}
