//! Rows the Issues window gains from the kernel's reports: an assembly loop the solve couldn't
//! rank clearly (`NearSingular`, a warning with no line of its own). `FORGE_UI_SHOTS=DIR` keeps
//! a picture.

use forge_render::gui::{Gui, Headless};
use forge_render::shell::fg::Severity;

fn ready(g: &Gui) -> bool {
    g.view.is_some()
        && g.fg
            .as_ref()
            .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
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

fn open(text: &str, tag: &str) -> Option<Headless> {
    let dir = std::env::temp_dir().join(format!("fg-issue-rows-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("p.fg");
    std::fs::write(&file, text).unwrap();
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
    let t = std::time::Instant::now();
    while t.elapsed().as_secs_f64() < 30.0 && !h.until(ready) {}
    assert!(ready(&h.gui), "never ready: {}", h.gui.shell.status);
    for _ in 0..4 {
        h.step();
    }
    Some(h)
}

#[test]
fn a_near_singular_loop_is_a_warning_row_with_no_line() {
    let n = forge_lang::asm::NearSingular {
        instances: vec!["crank".into(), "rod".into()],
        rank: 5,
        generic_rank: 6,
        mobility: 1,
    };
    let is = forge_render::fg::near_singular(&n);
    assert_eq!(is.severity, Severity::Warning);
    assert_eq!(is.kind, "NearSingular");
    assert_eq!(is.line, 0);
    assert!(is.text.starts_with("crank, rod: near-singular"), "{}", is.text);
    assert!(is.text.contains("5 counted, at most 6; 1 free"), "{}", is.text);

    // in the window: its row, its kind with no line
    let Some(mut h) = open(
        "sketch s(on: xy) {\n  r = rect(corner: (0, 0), size: (20, 10))\n}\nplate = extrude(s.r, 2)\n",
        "near",
    ) else {
        return;
    };
    {
        let p = h.gui.shell.fg.as_mut().unwrap();
        p.issues.push(is);
        p.issues_list = true;
    }
    for _ in 0..4 {
        h.step();
    }
    let k = h.gui.shell.fg.as_ref().unwrap().issues.len() - 1;
    assert!(h.gui.shell.place(&format!("fg:issue:{k}")).is_some(), "no row");
    shot(&mut h, "issues-near-singular");
}

#[test]
fn a_taper_that_closes_short_of_its_depth_shows_its_fixes_and_one_clears_it() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../forge-lang/tests/kernel_parts/taper_collapses.fg"
    ))
    .unwrap();
    let Some(mut h) = open(&src, "taper") else {
        return;
    };
    let issues = h.gui.shell.fg.as_ref().unwrap().issues.clone();
    let (k, is) = issues
        .iter()
        .enumerate()
        .find(|(_, i)| i.kind == "TaperCloses")
        .expect("a TaperCloses lint");
    assert_eq!(is.severity, Severity::Lint);
    assert!(
        is.text.starts_with("pit: the taper closes at depth"),
        "{}",
        is.text
    );
    assert_eq!(is.fixes.len(), 2);
    let labels: Vec<String> = is
        .fixes
        .iter()
        .map(|f| forge_render::shell::fg::fix_label(&is.kind, f))
        .collect();
    assert!(
        labels[0].starts_with("Depth to where it closes: Set pit.d = 0.305"),
        "{labels:?}"
    );
    assert!(
        labels[1].starts_with("Largest taper that reaches the depth: Set pit.a = 73.8"),
        "{labels:?}"
    );
    // the Issues window: its row and both fixes; the first applied clears it
    h.gui.shell.fg.as_mut().unwrap().issues_list = true;
    for _ in 0..4 {
        h.step();
    }
    assert!(h.gui.shell.place(&format!("fg:issue:{k}")).is_some());
    assert!(h.gui.shell.place(&format!("fg:fix:{k}:1")).is_some());
    shot(&mut h, "issues-taper-closes");
    assert!(h.click(&format!("fg:fix:{k}:0")), "no first fix");
    h.step();
    let t = std::time::Instant::now();
    while t.elapsed().as_secs_f64() < 30.0 && !h.until(ready) {}
    for _ in 0..4 {
        h.step();
    }
    let p = h.gui.shell.fg.as_ref().unwrap();
    assert!(p.source.contains("param pit.d = 0.305"), "{}", p.source);
    assert!(p.issues.iter().all(|i| i.kind != "TaperCloses"), "{:?}", p.issues);
}
