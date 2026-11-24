//! A face clicked in the view is drawn selected (the theme's chosen colour over it, its edges
//! outlined in it) after the
//! pointer leaves it, and the pointer over the part never lights an origin plane behind it.
//! Before: the frame's marks left out what was selected, so a clicked face showed nothing once the
//! pointer left; and with the face under the pointer selected (its hover mark gone) the origin
//! planes counted as hit there, so the XZ plane behind the face lit up green instead.

use forge_render::gui::{Gui, Headless};

const PART: &str = "sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (60, 30))
}
base = extrude(plan, 6)
";

fn settled(g: &Gui) -> bool {
    g.view.is_some() && g.fg.as_ref().is_some_and(|s| s.done == s.seq) && !g.loading()
}

/// Warm (amber, the chosen colour) over neutral: red well over blue.
fn warm(p: [u8; 3]) -> bool {
    p[0] as i32 - p[2] as i32 > 25
}

/// The selection's outline colour, full strength: the amber of the theme's chosen.
fn outline(p: [u8; 3]) -> bool {
    p[0] > 200 && (120..200).contains(&p[1]) && p[2] < 90
}

/// Outline pixels around the plate's top face.
fn outlined(px: &forge_render::gui::Pixels) -> usize {
    (300..620)
        .flat_map(|y| (580..1000).map(move |x| (x, y)))
        .filter(|&(x, y)| outline(px.at(x, y)))
        .count()
}

#[test]
fn a_clicked_face_is_drawn_selected_and_no_plane_lights_behind_it() {
    let Ok(mut h) = Headless::new((1280.0, 800.0), 1.0) else {
        eprintln!("no GPU adapter: skipped");
        return;
    };
    let dir = std::env::temp_dir().join(format!("forge-selection-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("plate.fg");
    std::fs::write(&file, PART).unwrap();
    h.gui.open(file.to_str().unwrap());
    assert!(h.until(settled), "never built: {}", h.gui.shell.status);
    // the top face, as the view fits this plate at 1280 x 800; a point of the XZ plane's square
    // off the part, to see it stays as it was
    let (face, plane) = ((760.0, 440.0), (450usize, 300usize));
    h.move_to(300.0, 650.0);
    assert!(h.settle_hover());
    let before = h.gui_pixels();
    assert!(
        !warm(before.at(face.0 as usize, face.1 as usize)),
        "the face warm before any click"
    );
    h.move_to(face.0, face.1);
    assert!(h.settle_hover());
    h.click_at(face.0, face.1);
    assert!(h.settle_hover());
    let over = h.gui_pixels();
    let picked = h.gui.view.as_ref().map_or(0, |v| v.nav.selected.len());
    assert_eq!(picked, 1, "{}", h.gui.shell.status);
    assert_eq!(
        over.at(plane.0, plane.1),
        before.at(plane.0, plane.1),
        "the origin plane behind the face lit up with the pointer on the face"
    );
    h.move_to(300.0, 650.0);
    assert!(h.settle_hover());
    let after = h.gui_pixels();
    let p = after.at(face.0 as usize, face.1 as usize);
    assert!(
        warm(p),
        "the selected face not drawn selected: {p:?} (before {:?})",
        before.at(face.0 as usize, face.1 as usize)
    );
    // and outlined: its edges in the selection's colour
    let (o0, o1) = (outlined(&before), outlined(&after));
    assert!(
        o1 > 500 && o1 > o0 * 10,
        "the selected face's outline: {o1} amber pixels (before {o0})"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
