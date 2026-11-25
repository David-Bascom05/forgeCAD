//! A part the view could draw only in part (faces the mesher failed on, left out): the Issues
//! window says so in words, counted with the warnings; Show frames the faces left out; their
//! border is outlined in the warning colour (the gap reads "not drawn", not "hole").

use forge_render::gui::Headless;
use forge_render::shell::fg::{Undrawn, UndrawnFace};

fn built(g: &forge_render::gui::Gui) -> bool {
    g.fg.as_ref()
        .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
        && g.view.is_some()
}

fn open() -> Option<Headless> {
    let mut h = Headless::new((1400.0, 860.0), 1.0).ok()?;
    h.step();
    h.gui.open("crank");
    assert!(h.until(built), "{}", h.gui.shell.status);
    h.step();
    h.step();
    Some(h)
}

fn shot(h: &mut Headless, name: &str) {
    for _ in 0..12 {
        h.step();
    }
    if let Ok(dir) = std::env::var("FORGE_UI_SHOTS") {
        std::fs::create_dir_all(&dir).unwrap();
        if let Ok(png) = h.png() {
            std::fs::write(std::path::Path::new(&dir).join(format!("{name}.png")), png).unwrap();
        }
    }
}

fn two_faces() -> Undrawn {
    Undrawn {
        faces: vec![
            UndrawnFace {
                face: 7,
                why: "ConstraintsIntersect { .. }".into(),
                edges: vec![0, 1, 2, 3],
                center: [10.0, 0.0, 5.0],
                radius: 6.0,
            },
            UndrawnFace {
                face: 9,
                why: "ConstraintsIntersect { .. }".into(),
                edges: vec![4, 5],
                center: [-10.0, 0.0, 5.0],
                radius: 6.0,
            },
        ],
        none_drawn: false,
    }
}

#[test]
fn faces_left_out_of_the_drawing_are_a_warning_in_words_with_show() {
    let Some(mut h) = open() else { return };
    let (e0, w0) = h.gui.shell.fg.as_ref().unwrap().counts();
    {
        // a plane face of the crank left out, as the mesher would leave it: its real border
        let body = forge_render::load::body("crank").unwrap();
        let cap = body
            .face_ids()
            .find(|&f| matches!(body.face(f).surface, forge_geom::Surface::Plane(_)))
            .unwrap();
        let left = [forge_render::tess::LeftOut { face: cap.0, why: "ConstraintsIntersect { .. }".into() }];
        let fg = h.gui.shell.fg.as_mut().unwrap();
        fg.undrawn = forge_render::fg::undrawn_of(&body, &left);
        fg.issues_list = true;
    }
    for _ in 0..4 {
        h.step();
    }
    let fg = h.gui.shell.fg.as_ref().unwrap();
    assert_eq!(fg.counts(), (e0, w0 + 1), "one warning for the part, however many faces");
    let line = fg.undrawn.line("crank.fg");
    assert_eq!(
        line,
        "crank.fg: 1 face couldn't be drawn and is left out of the view. The part itself is unchanged."
    );
    assert!(!line.contains("Constraint"), "the mesher's reason is the hover's, not the line's");
    assert!(fg.undrawn.detail().contains(": Mesher: ConstraintsIntersect"));
    assert!(!fg.undrawn.faces[0].edges.is_empty());
    let show = h.gui.shell.place("fg:undrawn:show").expect("Show beside the warning");
    assert!(h.gui.shell.place("fg:undrawn:line").is_some());
    shot(&mut h, "undrawn-issues");
    // Show frames the faces left out: the view turns to their place
    let eye0 = h.gui.view.as_ref().map(|v| (v.nav.camera.target, v.nav.camera.dist));
    h.click_at(show.center().x, show.center().y);
    for _ in 0..30 {
        h.step();
    }
    let eye1 = h.gui.view.as_ref().map(|v| (v.nav.camera.target, v.nav.camera.dist));
    assert_ne!(eye0, eye1, "Show moved the view to the faces left out");
    h.gui.shell.fg.as_mut().unwrap().issues_list = false;
    shot(&mut h, "undrawn-show");
}

#[test]
fn past_the_limit_the_warning_says_nothing_is_drawn() {
    let u = Undrawn { faces: vec![], none_drawn: true };
    assert!(u.any());
    assert_eq!(
        u.line("big.fg"),
        "big.fg couldn't be drawn: its faces couldn't be meshed. The part itself is unchanged."
    );
    assert_eq!(u.place(), None, "nothing to frame");
    let one = Undrawn { faces: vec![two_faces().faces[0].clone()], none_drawn: false };
    assert!(one.line("p.fg").starts_with("p.fg: 1 face couldn't be drawn and is left out"));
    assert!(!Undrawn::default().any());
}
