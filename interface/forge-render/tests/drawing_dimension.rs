//! Annotate > Dimension in a drawing file: two edges of the top view, then the place. The
//! dimension is written into the file. Before: placing it hung the app (DrawingEnv::doc called
//! itself for a drawing file, without end), on the desktop and in the page.

use forge_render::gui::Headless;
use std::sync::mpsc;
use std::time::Duration;

const PART: &str = "sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (60, 30))
}
base = extrude(plan, 6)
";
const DRAWING: &str = r#"drawing
# plate drawing: made by File > New Drawing.

m = part("plate.fg")
s1 = sheet("A4", standard: "ASME", title: "PLATE")
front = base_view(m, sheet: s1, dir: "front", scale: 1, at: (74.25, 79.8))
top = projected(front, at: (74.25, 157.5))
right = projected(front, at: (163.35, 79.8))
iso = projected(front, at: (184.14, 151.2))
"#;

/// Opens the drawing, picks Annotate > Dimension, two edges and a place; the file's text after.
fn place_dimension() -> Option<String> {
    let Ok(mut h) = Headless::new((1280.0, 800.0), 1.0) else {
        eprintln!("no GPU adapter: skipped");
        return None;
    };
    let dir = std::env::temp_dir().join(format!("forge-drawing-dim-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("plate.fg"), PART).unwrap();
    let file = dir.join("plate_dwg.fg");
    std::fs::write(&file, DRAWING).unwrap();
    h.gui.open(file.to_str().unwrap());
    assert!(
        h.until(|g| g
            .shell
            .drawing
            .as_ref()
            .is_some_and(|d| d.made.is_some() && !d.busy)),
        "the sheet never made: {}",
        h.gui.shell.status
    );
    // Annotate, then Dimension
    for (x, y) in [(209.0, 55.0), (40.0, 105.0)] {
        h.click_at(x, y);
        h.step();
    }
    // the top view's two long edges, then where the dimension goes
    for (x, y) in [(600.0, 290.0), (600.0, 365.0), (700.0, 330.0)] {
        h.move_to(x, y);
        h.step();
        h.step();
        h.click_at(x, y);
        for _ in 0..5 {
            h.step();
        }
    }
    let text = std::fs::read_to_string(&file).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    Some(text)
}

#[test]
fn a_dimension_placed_in_a_drawing_file_is_written() {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let _ = tx.send(place_dimension());
        })
        .unwrap();
    match rx.recv_timeout(Duration::from_secs(240)) {
        Ok(Some(text)) => assert!(text.contains("= dim(top, "), "no dimension written:\n{text}"),
        Ok(None) => {}
        Err(mpsc::RecvTimeoutError::Timeout) => panic!("placing the dimension hung the app"),
        Err(mpsc::RecvTimeoutError::Disconnected) => panic!("placing the dimension crashed the app"),
    }
}
