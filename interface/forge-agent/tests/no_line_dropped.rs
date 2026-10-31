//! A line of a part file is built or refused. It never quietly does less than it asks: a selector
//! that picks nothing, a finishing feature naming nothing, a character the font has no glyph for.
//! A line that builds and changes nothing is said to.
use forge_agent::history::model;

const BASE: &str = "b: extrude 10 rect 0 0 40 30\nh: hole 5 -xy at 10 10 10 thru\n";

#[test]
fn a_line_asking_for_nothing_is_refused() {
    for line in [
        "fillet 1",
        "chamfer 1",
        "chamfer 1 (edges of h along +x)",
        "shell 1 (faces of h along +x)",
        "draft 3 xy (faces of h along +x)",
        "draft 3 xy",
        "offset 1 (faces of h along +x)",
        "offset 1",
        "delete (faces of h along +x)",
        "delete",
        "move (faces of h along +x) by 0 0 1",
        "move by 0 0 1",
        "thicken 1 (faces of h along +x)",
        "facefillet 1 (faces of h along +x) to (faces plane near 0 0 10)",
        "extrude 5 rect 0 0 1 1 junk",
        "pattern nosuch lin 1 0 0 3 5",
        "pattern h pts",
        "combine add b nosuch",
        "text add \"\u{263a}\" font \"forgecad-test.ttf\" 5 1 xy at 0 0 10",
    ] {
        let m = model(&format!("{BASE}{line}\n"));
        assert!(m.error.is_some(), "`{line}` built with no error");
    }
}

/// A character without a glyph among ones that have one fails the line: the text would be short.
#[test]
fn a_character_the_font_lacks_is_not_left_out() {
    let m = model("extrude 10 rect 0 0 80 30\ntext add \"D\" font \"forgecad-test.ttf\" 8 1 xy at 5 5 10\n");
    assert!(m.error.is_none(), "{:?}", m.error.map(|e| e.msg));
    let m = model(
        "extrude 10 rect 0 0 80 30\ntext add \"D\u{263a}\" font \"forgecad-test.ttf\" 8 1 xy at 5 5 10\n",
    );
    let e = m.error.expect("refused");
    assert!(e.msg.contains("NoGlyph"), "{}", e.msg);
}

#[test]
fn a_line_that_changes_nothing_is_named() {
    let m = model(&format!(
        "{BASE}extrude cut 5 at 100 100 0 rect 0 0 1 1\nball cut 3 at 200 200 200\nextrude add 2 at 5 5 2 rect 0 0 1 1\nextrude cut 3 at 2 2 8 rect 0 0 1 1\n"
    ));
    assert!(m.error.is_none());
    let no = |i: usize| m.lines[i].no;
    assert_eq!(m.no_effect(), vec![no(2), no(3), no(4)]);
}

/// The error says what is open and which feature made it; the line is the one to blame.
#[test]
fn a_line_leaving_no_closed_solid_is_refused() {
    let m = model(
        "extrude new 20 at -20 -20 -20 rect 0 0 40 40\nrevolve keep z 0 0 -1 x -1 0 0 at 0 0 -14 circle 10 0 4\n",
    );
    if let Some(e) = &m.error {
        assert_eq!(e.at, 1);
        assert!(e.msg.contains("no closed solid (feature 1)"), "{}", e.msg);
        assert!(e.msg.contains("not 2"), "{}", e.msg);
    } else {
        assert!(forge_topo::check::check(m.body().expect("a part")).is_empty());
    }
}
