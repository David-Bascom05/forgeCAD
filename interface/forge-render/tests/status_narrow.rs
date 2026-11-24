//! The status bar in a narrow window (480 px): the status line ends short of Markup, cut with "…"
//! and whole on hover, and leads with the short state (valid, volume, faces) for the cut to keep.
//! Before: it ran on under Markup and the issues chip, both drawn over it.

use forge_render::gui::{Gui, Headless};

const PART: &str = "sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (60, 40))
}
base = extrude(plan, 24)
";

fn settled(g: &Gui) -> bool {
    g.view.is_some() && g.fg.as_ref().is_some_and(|s| s.done == s.seq) && !g.loading()
}

#[test]
fn the_status_line_stops_short_of_markup_in_a_narrow_window() {
    let Ok(mut h) = Headless::new((480.0, 860.0), 1.0) else {
        eprintln!("no GPU adapter: skipped");
        return;
    };
    let dir = std::env::temp_dir().join(format!("forge-status-narrow-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("plate.fg");
    std::fs::write(&file, PART).unwrap();
    h.gui.open(file.to_str().unwrap());
    assert!(h.until(settled), "never built: {}", h.gui.shell.status);
    for _ in 0..4 {
        h.step();
    }
    let status = h.gui.shell.status.clone();
    // the short state first, so the cut takes the details: valid, the volume in cm³, the faces
    assert!(status.starts_with("valid · 57.6 cm³ · 6 faces · "), "{status}");
    let said = h.placed("status:said").expect("the status line");
    // cut between its parts, the short state whole
    let shown = h.gui.shell.status_shown.clone();
    assert!(
        shown.starts_with("valid \u{b7} 57.6 cm³") && shown.ends_with(" \u{b7} \u{2026}"),
        "{shown}"
    );
    let markup = h.placed("status:markup").expect("Markup");
    assert!(
        said.right() <= markup.left(),
        "the status \"{status}\" ({said:?}) runs under Markup ({markup:?})"
    );
    if let Ok(d) = std::env::var("FORGE_UI_SHOTS")
        && let Ok(png) = h.png()
    {
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(std::path::Path::new(&d).join("status-480.png"), png).unwrap();
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A release refusal in Manufacture (the flange's setup unplaced, an NC export refused) in a
/// narrow window: the refusal's fix button stays whole and clickable, after the words and short of
/// Markup; the words are cut to leave it room, and left out (to the fix's hover) when even a few
/// of them wouldn't fit.
#[test]
fn a_refusal_s_fix_keeps_its_room_in_a_narrow_window() {
    let Some(narrow) = refusal_at(480.0) else {
        return;
    };
    assert_eq!(narrow, "", "words beside the fix at 480 px: {narrow}");
    let wider = refusal_at(760.0).unwrap();
    assert!(
        wider.starts_with("Program prog can") && wider.ends_with("(cut)"),
        "at 760 px: {wider}"
    );
}

/// The status line shown beside the fix in a window `w` px wide, its places checked.
fn refusal_at(w: f32) -> Option<String> {
    let ex = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/machined"));
    let dir = std::env::temp_dir().join(format!("forge-status-narrow-cam-{}-{w}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["flange.fg", "flange_cam.fg"] {
        std::fs::copy(ex.join(f), dir.join(f)).unwrap();
    }
    let file = dir.join("flange_cam.fg");
    let t = std::fs::read_to_string(&file)
        .unwrap()
        .replace(", at: (0, 0, -250)", "");
    std::fs::write(&file, t).unwrap();
    let Ok(mut h) = Headless::new((w, 860.0), 1.0) else {
        eprintln!("no GPU adapter: skipped");
        return None;
    };
    h.step();
    h.gui.open(file.to_str().unwrap());
    let generated = |g: &Gui| {
        g.view.is_some()
            && g.fg
                .as_ref()
                .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
            && g.shell.cam.as_ref().is_some_and(|w| {
                w.running.is_none()
                    && !w.want_run
                    && w.run
                        .as_ref()
                        .is_some_and(|r| r.cam.is_none() || r.quality.is_some())
            })
    };
    assert!(h.until(generated), "never generated: {}", h.gui.shell.status);
    h.gui
        .shell
        .cam
        .as_mut()
        .unwrap()
        .command(forge_render::shell::cam::Cmd::ExportNc(None));
    for _ in 0..4 {
        h.step();
    }
    let said = h.placed("status:said").expect("the status line");
    let fix = h.gui.shell.place("status:fix").expect("the refusal's fix");
    let markup = h.placed("status:markup").expect("Markup");
    assert!(
        said.right() <= fix.left(),
        "{w} px: the words {said:?} run under the fix {fix:?}"
    );
    assert!(
        fix.right() <= markup.left(),
        "{w} px: the fix {fix:?} under Markup {markup:?}"
    );
    if let Ok(d) = std::env::var("FORGE_UI_SHOTS")
        && let Ok(png) = h.png()
    {
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(
            std::path::Path::new(&d).join(format!("status-{w}-refusal.png")),
            png,
        )
        .unwrap();
    }
    let _ = std::fs::remove_dir_all(&dir);
    Some(h.gui.shell.status_shown.clone())
}
