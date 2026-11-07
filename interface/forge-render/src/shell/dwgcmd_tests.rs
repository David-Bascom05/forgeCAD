//! The drafting commands driven as a user drives them: typed input and picked points fed to a
//! document made from a real drawing (`mix_2018.dwg`: a rectangle of lines 176..224 x 114..186,
//! a circle at (100, 150) radius 36, an arc, an ellipse, a polyline), the result checked, then
//! undone and the drawing checked as it was. `FORGE_DWG_EDIT_OUT=DIR` keeps the edited files
//! (for the ODA converter).

use super::dwgcmd::want;
use super::dwgdoc::DwgDoc;
use forge_dwg::draft::entity::read_entity;
use forge_dwg::draft::{Curve, V2, v2};
use forge_dwg::types::Data;
use std::collections::BTreeSet;

fn doc() -> DwgDoc {
    let path = format!(
        "{}/../../kernel/forge-dwg/tests/data/mix_2018.dwg",
        env!("CARGO_MANIFEST_DIR")
    );
    let bytes = std::fs::read(path).expect("the test drawing");
    let mut d = DwgDoc::from_bytes(None, "mix_2018.dwg", &bytes).expect("read");
    d.fit(egui::vec2(1600.0, 900.0));
    let s = d.view.expect("fitted").scale;
    d.ed.px = s;
    // typed points are exact; snaps are tested apart
    d.ed.set.osnap = false;
    d.ed.set.polar = false;
    d
}

/// One step of a script, as the user gives it.
enum S<'a> {
    /// A command, or what's typed at a prompt (Enter after it).
    T(&'a str),
    /// A click on the canvas at a world point.
    C(f64, f64),
    /// A window (left to right) or crossing (right to left) from one corner to the other.
    W(f64, f64, f64, f64),
    Enter,
    Esc,
}

fn run(d: &mut DwgDoc, script: &[S]) {
    for s in script {
        match s {
            S::T(t) => {
                let _ = d.run_command(t);
            }
            S::C(x, y) => {
                let p = v2(*x, *y);
                let scale = d.ed.px;
                d.hover(p, scale);
                d.click(p, false);
            }
            S::W(x0, y0, x1, y1) => {
                d.window(v2(*x0, *y0), v2(*x1, *y1), x1 < x0);
            }
            S::Enter => {
                if d.ed.active() {
                    d.enter();
                } else {
                    let _ = d.run_command("");
                }
            }
            S::Esc => d.escape(),
        }
    }
}

fn handles(d: &DwgDoc) -> BTreeSet<u32> {
    d.drawing.objects.keys().copied().collect()
}

/// The entities a script made (in the space).
fn made(d: &DwgDoc, before: &BTreeSet<u32>) -> Vec<u32> {
    d.drawing
        .objects
        .iter()
        .filter(|(h, o)| {
            !before.contains(h) && o.ent.is_some() && d.drawing.block_of(**h) == Some(d.drawing.model_space())
        })
        .map(|(h, _)| *h)
        .collect()
}

fn curve(d: &DwgDoc, h: u32) -> Curve {
    read_entity(&d.drawing.objects[&h].data).expect("a curve").curve
}

/// Undoes everything and checks the drawing is as it was read.
fn undo_all(d: &mut DwgDoc, orig: &forge_dwg::file::Drawing) {
    while !d.ed.hist.undo.is_empty() {
        d.undo();
    }
    assert!(d.drawing.objects.keys().eq(orig.objects.keys()), "objects back");
    for (h, o) in &orig.objects {
        assert_eq!(d.drawing.objects[h].data, o.data, "{h:#x}");
    }
    assert_eq!(d.drawing.header, orig.header);
}

/// The edited drawing written and read back, kept when asked.
fn save_check(d: &DwgDoc, name: &str) -> forge_dwg::file::Drawing {
    let bytes = d.bytes().expect("written");
    if let Ok(dir) = std::env::var("FORGE_DWG_EDIT_OUT") {
        std::fs::write(format!("{dir}/{name}.dwg"), &bytes).expect("kept");
    }
    forge_dwg::file::read(&bytes).expect("read back")
}

#[test]
fn line_with_close_and_undo() {
    let mut d = doc();
    let orig = d.drawing.clone();
    let before = handles(&d);
    run(
        &mut d,
        &[
            S::T("L"),
            S::T("0,0"),
            S::T("10,0"),
            S::T("@0,10"),
            S::T("@-10,0"),
            S::T("u"),
            S::T("@-10,0"),
            S::T("C"),
        ],
    );
    assert!(!d.ed.active());
    let new = made(&d, &before);
    assert_eq!(new.len(), 4, "{new:?}");
    let ls: Vec<Curve> = new.iter().map(|h| curve(&d, *h)).collect();
    assert!(ls.contains(&Curve::line(v2(0.0, 0.0), v2(10.0, 0.0))));
    assert!(ls.contains(&Curve::line(v2(10.0, 0.0), v2(10.0, 10.0))));
    assert!(ls.contains(&Curve::line(v2(10.0, 10.0), v2(0.0, 10.0))));
    assert!(ls.contains(&Curve::line(v2(0.0, 10.0), v2(0.0, 0.0))));
    // one undo step
    assert_eq!(d.ed.hist.undo.len(), 1);
    // drawn: the edit's own scene has them
    for h in &new {
        assert!(d.delta.by_entity.contains_key(h));
    }
    let back = save_check(&d, "line");
    assert!(new.iter().all(|h| back.objects.contains_key(h)));
    undo_all(&mut d, &orig);
    assert!(d.delta.by_entity.is_empty());
    // and redo puts them back
    d.redo();
    assert_eq!(made(&d, &before).len(), 4);
}

#[test]
fn direct_distance_and_polar_input() {
    let mut d = doc();
    let before = handles(&d);
    run(&mut d, &[S::T("LINE"), S::T("0,0")]);
    // the cursor east of the base: 25 typed goes 25 that way
    d.hover(v2(7.0, 0.0), d.ed.px);
    run(&mut d, &[S::T("25"), S::T("@10<90"), S::Enter]);
    let ls: Vec<Curve> = made(&d, &before).iter().map(|h| curve(&d, *h)).collect();
    assert!(ls.contains(&Curve::line(v2(0.0, 0.0), v2(25.0, 0.0))));
    let Curve::Line { a, b } = ls.iter().find(|c| c.start() == v2(25.0, 0.0)).unwrap() else {
        panic!()
    };
    assert!(a.dist(v2(25.0, 0.0)) < 1e-12 && b.dist(v2(25.0, 10.0)) < 1e-9);
}

#[test]
fn draw_commands() {
    let mut d = doc();
    let orig = d.drawing.clone();
    let before = handles(&d);
    run(
        &mut d,
        &[
            S::T("C"),
            S::T("0,0"),
            S::T("5"),
            S::T("CIRCLE"),
            S::T("2P"),
            S::T("20,0"),
            S::T("30,0"),
            S::T("CIRCLE"),
            S::T("3P"),
            S::T("40,0"),
            S::T("50,10"),
            S::T("60,0"),
            S::T("ARC"),
            S::T("0,20"),
            S::T("10,30"),
            S::T("20,20"),
            S::T("ARC"),
            S::T("C"),
            S::T("50,50"),
            S::T("60,50"),
            S::T("A"),
            S::T("90"),
            S::T("REC"),
            S::T("100,0"),
            S::T("120,10"),
            S::T("POLYGON"),
            S::T("6"),
            S::T("150,0"),
            S::T("I"),
            S::T("10"),
            S::T("ELLIPSE"),
            S::T("0,-50"),
            S::T("40,-50"),
            S::T("10"),
            S::T("SPLINE"),
            S::T("0,-80"),
            S::T("10,-70"),
            S::T("20,-85"),
            S::T("30,-75"),
            S::Enter,
            S::T("POINT"),
            S::T("5,-100"),
            S::T("PL"),
            S::T("200,0"),
            S::T("210,0"),
            S::T("A"),
            S::T("210,10"),
            S::T("L"),
            S::T("200,10"),
            S::T("C"),
            S::T("TEXT"),
            S::T("0,-120"),
            S::T("3"),
            S::T("0"),
            S::T("Hello there"),
            S::T("second line"),
            S::T(""),
        ],
    );
    assert!(!d.ed.active(), "{:?}", d.history.last());
    let new = made(&d, &before);
    let kinds: Vec<String> = new.iter().map(|h| d.drawing.objects[h].data.name()).collect();
    let count = |k: &str| kinds.iter().filter(|x| *x == k).count();
    assert_eq!(count("Circle"), 3, "{kinds:?}");
    assert_eq!(count("Arc"), 2, "{kinds:?}");
    assert_eq!(count("LwPolyline"), 3, "{kinds:?}");
    assert_eq!(count("Ellipse"), 1);
    assert_eq!(count("Spline"), 1);
    assert_eq!(count("Point"), 1);
    assert_eq!(count("Text"), 2);
    let circles: Vec<Curve> = new
        .iter()
        .filter(|h| matches!(d.drawing.objects[h].data, Data::Circle(_)))
        .map(|h| curve(&d, *h))
        .collect();
    assert!(circles.contains(&Curve::circle(v2(0.0, 0.0), 5.0)));
    assert!(circles.contains(&Curve::circle(v2(25.0, 0.0), 5.0)));
    assert!(circles.iter().any(
        |c| matches!(c, Curve::Circle { c, r } if c.dist(v2(50.0, 0.0)) < 1e-9 && (r - 10.0).abs() < 1e-9)
    ));
    // the polyline with a tangent arc: a half circle bulge 1 from (210,0) to (210,10)
    let pl = new
        .iter()
        .find_map(|h| match &d.drawing.objects[h].data {
            Data::LwPolyline(p) if p.points.len() == 4 && p.points[0] == [200.0, 0.0] => Some(p.clone()),
            _ => None,
        })
        .expect("the pline");
    assert!(pl.flag & 512 != 0);
    assert!((pl.bulges[1] - 1.0).abs() < 1e-9, "{:?}", pl.bulges);
    // the hexagon: six vertices ten from its centre
    let hex = new
        .iter()
        .find_map(|h| match &d.drawing.objects[h].data {
            Data::LwPolyline(p) if p.points.len() == 6 => Some(p.clone()),
            _ => None,
        })
        .expect("the hexagon");
    assert!(
        hex.points
            .iter()
            .all(|q| (V2::from(*q).dist(v2(150.0, 0.0)) - 10.0).abs() < 1e-9)
    );
    let texts: Vec<String> = new
        .iter()
        .filter_map(|h| match &d.drawing.objects[h].data {
            Data::Text(t) => Some(t.text.text()),
            _ => None,
        })
        .collect();
    assert!(texts.contains(&"Hello there".to_string()) && texts.contains(&"second line".to_string()));
    let back = save_check(&d, "draw");
    assert!(new.iter().all(|h| back.objects.contains_key(h)));
    undo_all(&mut d, &orig);
}

#[test]
fn mtext_hatch_and_dimensions() {
    let mut d = doc();
    let orig = d.drawing.clone();
    let before = handles(&d);
    run(&mut d, &[S::T("MTEXT"), S::T("0,-20"), S::T("40,-40")]);
    assert!(d.ed.prompt.as_ref().is_some_and(|p| p.wants(want::LINES)));
    d.feed(super::dwgcmd::Inp::Text("first\nsecond".into()));
    // a hatch in the rectangle of lines, solid; one in the circle, ANSI31
    run(
        &mut d,
        &[
            S::T("H"),
            S::T("P"),
            S::T("SOLID"),
            S::C(200.0, 130.0),
            S::Enter,
            S::T("HATCH"),
            S::T("P"),
            S::T("ANSI31"),
            S::C(100.0, 150.0),
            S::Enter,
            S::T("DIMLINEAR"),
            S::T("176,114"),
            S::T("224,114"),
            S::T("200,100"),
            S::T("DIMALIGNED"),
            S::T("176,114"),
            S::T("224,186"),
            S::T("230,140"),
            S::T("DIMRADIUS"),
            S::C(136.0, 150.0),
            S::T("150,170"),
            S::T("DIMDIAMETER"),
            S::C(64.0, 150.0),
            S::T("90,150"),
        ],
    );
    assert!(
        !d.ed.active(),
        "{:?}",
        &d.history[d.history.len().saturating_sub(4)..]
    );
    let new = made(&d, &before);
    let kinds: Vec<String> = new.iter().map(|h| d.drawing.objects[h].data.name()).collect();
    let count = |k: &str| kinds.iter().filter(|x| *x == k).count();
    assert_eq!(count("MText"), 1, "{kinds:?}");
    assert_eq!(count("Hatch"), 2, "{kinds:?}");
    assert_eq!(count("DimLinear"), 1);
    assert_eq!(count("DimAligned"), 1);
    assert_eq!(count("DimRadius"), 1);
    assert_eq!(count("DimDiameter"), 1);
    let mt = new
        .iter()
        .find_map(|h| match &d.drawing.objects[h].data {
            Data::MText(t) => Some(t.text.text()),
            _ => None,
        })
        .unwrap();
    assert_eq!(mt, "first\\Psecond");
    for h in &new {
        match &d.drawing.objects[h].data {
            Data::Hatch(x) if x.solid => {
                // the rectangle's six lines round what's inside it (islands)
                assert_eq!(x.paths[0].edges.len(), 6);
            }
            Data::Hatch(x) => {
                assert_eq!(x.name.text(), "ANSI31");
                assert_eq!(x.paths[0].edges.len(), 1, "the circle");
            }
            Data::DimLinear(x) => {
                assert!((x.dim.measurement - 48.0).abs() < 1e-3);
                assert!(forge_dwg::draft::commit::dim_block(&d.drawing, *h).is_some());
            }
            Data::DimRadius(x) => assert!((x.dim.measurement - 36.0).abs() < 1e-9),
            Data::DimDiameter(x) => assert!((x.dim.measurement - 72.0).abs() < 1e-9),
            _ => {}
        }
        // each is drawn
        assert!(
            d.delta.by_entity.contains_key(h),
            "{h:#x} {}",
            d.drawing.objects[h].data.name()
        );
    }
    let back = save_check(&d, "annotate");
    assert!(new.iter().all(|h| back.objects.contains_key(h)));
    undo_all(&mut d, &orig);
}

#[test]
fn move_copy_rotate_scale_mirror() {
    let mut d = doc();
    let orig = d.drawing.clone();
    // noun-verb: the circle chosen, then MOVE
    d.choose(&[0x69], true);
    run(&mut d, &[S::T("M"), S::T("0,0"), S::T("10,5")]);
    let Curve::Circle { c, .. } = curve(&d, 0x69) else {
        panic!()
    };
    assert!(c.dist(v2(109.998_004_878_626_49, 155.0)) < 1e-9);
    assert!(d.hidden.contains(&0x69) && d.edited.contains(&0x69));
    // verb-noun: COPY, pick the line, Enter, two copies
    let before = handles(&d);
    run(
        &mut d,
        &[
            S::T("CO"),
            S::C(224.0, 170.0),
            S::Enter,
            S::T("0,0"),
            S::T("100,0"),
            S::T("200,0"),
            S::Enter,
        ],
    );
    let copies = made(&d, &before);
    assert_eq!(copies.len(), 2);
    let xs: Vec<f64> = copies.iter().map(|h| curve(&d, *h).start().x).collect();
    assert!(xs.contains(&324.0) && xs.contains(&424.0), "{xs:?}");
    // ROTATE by a reference: the line 0x6e (horizontal) turned to 90
    run(
        &mut d,
        &[
            S::T("RO"),
            S::C(200.0, 186.0),
            S::Enter,
            S::T("200,186"),
            S::T("R"),
            S::T("0"),
            S::T("90"),
        ],
    );
    let Curve::Line { a, b } = curve(&d, 0x6e) else {
        panic!()
    };
    assert!(
        (a.x - 200.0).abs() < 1e-3 && (b.x - 200.0).abs() < 1e-3,
        "{a:?} {b:?}"
    );
    // SCALE a text by 2 about its insertion
    run(
        &mut d,
        &[
            S::T("SC"),
            S::W(30.0, 10.0, 34.0, 15.0),
            S::Enter,
            S::T("31.616666666666664,11.25"),
            S::T("2"),
        ],
    );
    let Data::Text(t) = &d.drawing.objects[&0x70].data else {
        panic!()
    };
    assert!((t.height - 5.0).abs() < 1e-12);
    // MIRROR the arc, keeping the source
    let before = handles(&d);
    run(
        &mut d,
        &[
            S::T("MI"),
            S::C(24.0, 30.7),
            S::Enter,
            S::T("0,0"),
            S::T("0,10"),
            S::Enter,
        ],
    );
    let m = made(&d, &before);
    assert_eq!(m.len(), 1);
    let Curve::Arc(a) = curve(&d, m[0]) else { panic!() };
    assert!(a.c.dist(v2(-20.0, 30.0)) < 1e-9);
    assert!(d.drawing.objects.contains_key(&0x9e));
    let back = save_check(&d, "modify1");
    assert!(back.objects.contains_key(&m[0]));
    undo_all(&mut d, &orig);
}

#[test]
fn offset_trim_extend_fillet_chamfer() {
    let mut d = doc();
    let orig = d.drawing.clone();
    // OFFSET the rectangle's right side 5 outward
    let before = handles(&d);
    run(
        &mut d,
        &[
            S::T("O"),
            S::T("5"),
            S::C(224.0, 170.0),
            S::T("240,170"),
            S::Enter,
        ],
    );
    let o = made(&d, &before);
    assert_eq!(o.len(), 1);
    assert_eq!(curve(&d, o[0]), Curve::line(v2(229.0, 150.0), v2(229.0, 186.0)));
    // a line across the circle, then TRIM its middle out at the circle
    run(&mut d, &[S::T("L"), S::T("40,150"), S::T("160,150"), S::Enter]);
    let across = made(&d, &before).into_iter().find(|h| !o.contains(h)).unwrap();
    run(&mut d, &[S::T("TR"), S::C(100.0, 150.0), S::Enter]);
    let pieces: Vec<Curve> = made(&d, &before)
        .into_iter()
        .filter(|h| !o.contains(h))
        .map(|h| curve(&d, h))
        .collect();
    assert_eq!(pieces.len(), 2, "{pieces:?}");
    assert!(
        pieces
            .iter()
            .any(|c| c.end().dist(v2(64.0, 150.0)) < 1e-2 || c.start().dist(v2(64.0, 150.0)) < 1e-2)
    );
    let _ = across;
    // TRIM the circle's top by the rectangle? no: trim the circle's left between the line's pieces
    // EXTEND: the offset line up to... the top edge is at 186 and the offset line ends at 186:
    // shorten it first with LENGTHEN, then EXTEND it back to the circle? Extend the bottom
    // right line (224, 150 -> 114) down: nothing below, says so; extend the offset line's
    // bottom to the rectangle's bottom line's extension: none either
    run(
        &mut d,
        &[S::T("LEN"), S::T("DE"), S::T("-10"), S::C(229.0, 184.0), S::Enter],
    );
    assert_eq!(curve(&d, o[0]), Curve::line(v2(229.0, 150.0), v2(229.0, 176.0)));
    // a horizontal line at y = 200 above it, then EXTEND the offset line up to it
    run(&mut d, &[S::T("L"), S::T("150,200"), S::T("260,200"), S::Enter]);
    run(&mut d, &[S::T("EX"), S::C(229.0, 175.0), S::Enter]);
    assert_eq!(curve(&d, o[0]), Curve::line(v2(229.0, 150.0), v2(229.0, 200.0)));
    // FILLET the rectangle's top left corner, radius 5
    run(
        &mut d,
        &[
            S::T("F"),
            S::T("R"),
            S::T("5"),
            S::C(176.0, 170.0),
            S::C(190.0, 186.0),
        ],
    );
    let Curve::Line { b, .. } = curve(&d, 0x6c) else {
        panic!()
    };
    assert!(b.dist(v2(176.0, 181.0)) < 1e-6, "{b:?}");
    // CHAMFER the bottom right corner, 3 and 3
    run(
        &mut d,
        &[
            S::T("CHA"),
            S::T("D"),
            S::T("3"),
            S::T("3"),
            S::C(224.0, 130.0),
            S::C(210.0, 114.0),
        ],
    );
    let Curve::Line { b, .. } = curve(&d, 0x6b) else {
        panic!()
    };
    assert!(b.dist(v2(224.0, 117.0)) < 1e-6, "{b:?}");
    assert_eq!(d.ed.hist.undo.len(), 8);
    let _ = save_check(&d, "modify2");
    undo_all(&mut d, &orig);
}

#[test]
fn break_join_stretch_array_explode_matchprop_erase() {
    let mut d = doc();
    let orig = d.drawing.clone();
    // BREAK the rectangle's top between two points
    let before = handles(&d);
    run(&mut d, &[S::T("BR"), S::C(190.0, 186.0), S::T("200,186")]);
    let pieces = made(&d, &before);
    assert_eq!(pieces.len(), 1, "one piece added beside the one kept");
    // JOIN them back
    run(
        &mut d,
        &[S::T("J"), S::C(185.0, 186.0), S::C(215.0, 186.0), S::Enter],
    );
    assert!(!d.drawing.objects.contains_key(&pieces[0]) || !d.drawing.objects.contains_key(&0x6e));
    // STRETCH the rectangle's right side 10 right by a crossing round it
    run(
        &mut d,
        &[
            S::T("S"),
            S::W(230.0, 100.0, 215.0, 200.0),
            S::Enter,
            S::T("0,0"),
            S::T("10,0"),
        ],
    );
    let Curve::Line { a, b } = curve(&d, 0x6a) else {
        panic!()
    };
    assert!((a.x - 234.0).abs() < 1e-9 && (b.x - 234.0).abs() < 1e-9);
    let Curve::Line { a, .. } = curve(&d, 0x6f) else {
        panic!()
    };
    assert!(
        (a.x - 234.00004).abs() < 1e-6,
        "the bottom's right end went too: {a:?}"
    );
    // ARRAY the arc 2 rows by 3 columns
    let before = handles(&d);
    run(
        &mut d,
        &[
            S::T("AR"),
            S::C(24.0, 30.7),
            S::Enter,
            S::T("R"),
            S::T("2"),
            S::T("3"),
            S::T("20"),
            S::T("30"),
        ],
    );
    assert_eq!(made(&d, &before).len(), 5);
    // polar: 4 about the origin
    let before = handles(&d);
    run(
        &mut d,
        &[
            S::T("AR"),
            S::C(24.0, 30.7),
            S::Enter,
            S::T("PO"),
            S::T("0,0"),
            S::T("4"),
            S::T("360"),
            S::T("Y"),
        ],
    );
    assert_eq!(made(&d, &before).len(), 3);
    // EXPLODE the polyline
    let before = handles(&d);
    run(&mut d, &[S::T("X"), S::C(3.0, 2.0), S::Enter]);
    assert_eq!(made(&d, &before).len(), 2);
    assert!(!d.drawing.objects.contains_key(&0x9c));
    // MATCHPROP: the arc's layer onto the circle
    let arc_layer = d.drawing.objects[&0x9e].ent.as_ref().unwrap().layer;
    run(
        &mut d,
        &[S::T("MA"), S::C(24.0, 30.7), S::C(136.0, 150.0), S::Enter],
    );
    assert_eq!(d.drawing.objects[&0x69].ent.as_ref().unwrap().layer, arc_layer);
    // ERASE by a window round the ellipse
    run(&mut d, &[S::T("E"), S::W(35.0, 45.0, 65.0, 75.0), S::Enter]);
    assert!(!d.drawing.objects.contains_key(&0x9d));
    let _ = save_check(&d, "modify3");
    undo_all(&mut d, &orig);
}

#[test]
fn grips_and_escape() {
    let mut d = doc();
    let orig = d.drawing.clone();
    d.choose(&[0x6a], true);
    assert_eq!(d.ed.grips.len(), 3);
    // the top grip dragged up: STRETCH
    run(&mut d, &[S::C(224.0, 186.0), S::T("224,200")]);
    assert_eq!(curve(&d, 0x6a), Curve::line(v2(224.0, 150.0), v2(224.0, 200.0)));
    // the middle grip, Enter to MOVE, then a point
    d.choose(&[0x6a], true);
    run(&mut d, &[S::C(224.0, 175.0), S::Enter, S::T("234,175")]);
    assert_eq!(curve(&d, 0x6a), Curve::line(v2(234.0, 150.0), v2(234.0, 200.0)));
    // Esc calls a command off, and what it made stays (LINE's segments)
    let before = handles(&d);
    run(&mut d, &[S::T("L"), S::T("0,0"), S::T("5,5"), S::Esc]);
    assert!(!d.ed.active());
    assert_eq!(made(&d, &before).len(), 1);
    undo_all(&mut d, &orig);
}

#[test]
fn snaps_find_ends_and_centres() {
    let mut d = doc();
    d.ed.set.osnap = true;
    run(&mut d, &[S::T("L")]);
    // near the rectangle's corner: its endpoint
    d.hover(v2(224.3, 185.8), d.ed.px);
    match &d.ed.track {
        Some(super::dwgcmd::Track::Snap(s)) => {
            assert_eq!(s.kind, forge_dwg::draft::snap::SnapKind::Endpoint);
            assert!(s.point.dist(v2(224.0, 186.0)) < 1e-3);
        }
        t => panic!("{t:?}"),
    }
    assert!(d.ed.cursor.unwrap().dist(v2(224.0, 186.0)) < 1e-3);
    run(&mut d, &[S::Esc]);
}

#[test]
fn properties_and_layers() {
    let mut d = doc();
    let orig = d.drawing.clone();
    d.choose(&[0x69], true);
    d.set_prop("Radius", "40").unwrap();
    let Curve::Circle { r, .. } = curve(&d, 0x69) else {
        panic!()
    };
    assert_eq!(r, 40.0);
    d.set_prop("Color", "Red").unwrap();
    assert_eq!(d.drawing.objects[&0x69].ent.as_ref().unwrap().color.index, 1);
    // a new layer, the circle onto it, it turned off, frozen and locked
    let h = d.new_layer().unwrap();
    let name = d.layer_rows().into_iter().find(|r| r.handle == h).unwrap().name;
    d.set_prop("Layer", &name).unwrap();
    assert_eq!(d.drawing.objects[&0x69].ent.as_ref().unwrap().layer.handle, h);
    d.layer_switch(h, super::dwgprops::Switch::On).unwrap();
    assert!(!d.layer_rows().into_iter().find(|r| r.handle == h).unwrap().on);
    assert_eq!(d.layer_state.get(&h), Some(&(false, false)));
    // it can't be deleted with the circle on it
    assert!(d.delete_layer(h).is_err());
    // the current layer, and the ribbon's dropdown with nothing chosen
    d.clear_selection();
    d.layer_picked(h).unwrap();
    assert_eq!(d.current_layer(), h);
    d.layer_colour(h, 3).unwrap();
    assert_eq!(d.layer_rows().into_iter().find(|r| r.handle == h).unwrap().aci, 3);
    let back = save_check(&d, "props");
    assert!(back.objects.contains_key(&h));
    undo_all(&mut d, &orig);
    // a layer nothing is on goes
    let h = d.new_layer().unwrap();
    d.delete_layer(h).unwrap();
    assert!(!d.drawing.objects.contains_key(&h));
}

/// `FORGE_DWG_BIG=FILE.dwg cargo test --release --lib edit_timings -- --ignored --nocapture`:
/// what each interactive step costs on a big drawing, as a table.
#[test]
#[ignore]
fn edit_timings() {
    let Ok(path) = std::env::var("FORGE_DWG_BIG") else {
        return;
    };
    let ms = |t: std::time::Instant| t.elapsed().as_secs_f64() * 1e3;
    let mut rows: Vec<(String, f64)> = vec![];
    let t = std::time::Instant::now();
    let bytes = std::fs::read(&path).unwrap();
    let mut d = DwgDoc::from_bytes(None, "big.dwg", &bytes).unwrap();
    rows.push(("open (read + scene + GPU buffers)".into(), ms(t)));
    d.fit(egui::vec2(1600.0, 900.0));
    let scale = d.view.unwrap().scale;
    d.ed.px = scale;
    let n_obj = d.drawing.objects.len();
    let n_items = d.geom.n_items();
    // the model's lines, and one in the middle of the list
    let lines: Vec<u32> = d
        .geom
        .by_entity
        .keys()
        .copied()
        .filter(|h| matches!(d.drawing.objects.get(h).map(|o| &o.data), Some(Data::Line(_))))
        .collect();
    assert!(!lines.is_empty(), "no lines in model space");
    let l0 = lines[lines.len() / 2];
    let c0 = curve(&d, l0);
    let (a, b) = (c0.start(), c0.end());
    let mid = a.mid(b);
    // zoom in on it, as one works
    d.view = Some(super::dwgdoc::View {
        center: mid.arr(),
        scale: 900.0 / (a.dist(b) * 4.0).max(1e-6),
    });
    let scale = d.view.unwrap().scale;
    d.ed.px = scale;
    let px = 1.0 / scale;

    // hovering with a command waiting for a point: snaps
    run(&mut d, &[S::T("L")]);
    let mut worst: f64 = 0.0;
    let t = std::time::Instant::now();
    for k in 0..200 {
        let p = a + (b - a) * (k as f64 / 200.0) + v2(px * 2.0, px * 1.0);
        let t1 = std::time::Instant::now();
        d.hover(p, scale);
        worst = worst.max(ms(t1));
    }
    rows.push(("hover + object snap (mean of 200)".into(), ms(t) / 200.0));
    let snapped = matches!(d.ed.track, Some(super::dwgcmd::Track::Snap(_)));
    eprintln!(
        "last hover snapped: {snapped} ({:?})",
        d.ed.track.as_ref().map(|t| match t {
            super::dwgcmd::Track::Snap(s) => format!("{:?}", s.kind),
            _ => "tracking".into(),
        })
    );
    rows.push(("hover + object snap (worst)".into(), worst));
    let t = std::time::Instant::now();
    run(&mut d, &[S::T(&format!("{},{}", a.x, a.y))]);
    rows.push(("LINE first point".into(), ms(t)));
    let far = a + v2(px * 300.0, px * 200.0);
    let t = std::time::Instant::now();
    run(&mut d, &[S::T(&format!("{},{}", far.x, far.y))]);
    rows.push(("LINE next point (adds + redraws)".into(), ms(t)));
    let t = std::time::Instant::now();
    run(&mut d, &[S::T(&format!("{},{}", far.x + px * 100.0, far.y))]);
    rows.push(("LINE next point (2nd segment)".into(), ms(t)));
    run(&mut d, &[S::Enter]);
    // the rubber band and the pick under the cursor
    let q = d.viewq();
    let t = std::time::Instant::now();
    for _ in 0..100 {
        let _ = q.pick(mid, px * 5.0);
    }
    rows.push(("pick under cursor (mean of 100)".into(), ms(t) / 100.0));
    // MOVE one, noun-verb
    d.choose(&[l0], true);
    let t = std::time::Instant::now();
    run(
        &mut d,
        &[S::T("M"), S::T("0,0"), S::T(&format!("{},{}", px * 10.0, 0.0))],
    );
    rows.push(("MOVE 1 entity".into(), ms(t)));
    // MOVE a thousand
    let many: Vec<u32> = lines.iter().copied().take(1000).collect();
    d.choose(&many, true);
    let t = std::time::Instant::now();
    run(
        &mut d,
        &[S::T("M"), S::T("0,0"), S::T(&format!("{},{}", px * 10.0, 0.0))],
    );
    rows.push((format!("MOVE {} entities", many.len()), ms(t)));
    // a preview of them dragged: what the overlay draws
    d.choose(&many, true);
    run(&mut d, &[S::T("M"), S::T("0,0")]);
    let t = std::time::Instant::now();
    let g = d.ghosts();
    let mut segs = vec![];
    if let Some(super::dwgcmd::Ghost::Ents(hs, _)) = g.first() {
        let q = d.viewq();
        for h in hs {
            q.segments(*h, 20_001, &mut segs);
        }
    }
    rows.push(("MOVE 1000: rubber band's segments".into(), ms(t)));
    run(&mut d, &[S::Esc]);
    // COPY, OFFSET, TRIM, FILLET
    d.choose(&[l0], true);
    let t = std::time::Instant::now();
    run(
        &mut d,
        &[
            S::T("CO"),
            S::T("0,0"),
            S::T(&format!("0,{}", px * 20.0)),
            S::Enter,
        ],
    );
    rows.push(("COPY 1 entity".into(), ms(t)));
    let c0 = curve(&d, l0);
    let mid = c0.start().mid(c0.end());
    let side = mid + (c0.end() - c0.start()).perp().unit_or_zero() * (px * 15.0);
    let t = std::time::Instant::now();
    run(
        &mut d,
        &[
            S::T("O"),
            S::T(&format!("{}", px * 10.0)),
            S::C(mid.x, mid.y),
            S::T(&format!("{},{}", side.x, side.y)),
            S::Enter,
        ],
    );
    rows.push(("OFFSET a line".into(), ms(t)));
    let t = std::time::Instant::now();
    run(&mut d, &[S::T("TR"), S::C(mid.x, mid.y), S::Enter]);
    rows.push(("TRIM (quick mode) one pick".into(), ms(t)));
    // two lines of our own meeting, filleted
    let o = far + v2(px * 400.0, 0.0);
    run(
        &mut d,
        &[
            S::T("L"),
            S::T(&format!("{},{}", o.x, o.y)),
            S::T(&format!("{},{}", o.x + px * 100.0, o.y)),
            S::T(&format!("{},{}", o.x + px * 100.0, o.y + px * 100.0)),
            S::Enter,
        ],
    );
    let t = std::time::Instant::now();
    run(
        &mut d,
        &[
            S::T("F"),
            S::T("R"),
            S::T(&format!("{}", px * 20.0)),
            S::C(o.x + px * 50.0, o.y),
            S::C(o.x + px * 100.0, o.y + px * 50.0),
        ],
    );
    rows.push(("FILLET two lines".into(), ms(t)));
    // a rectangle hatched, and a dimension
    let r0 = o + v2(px * 200.0, 0.0);
    run(
        &mut d,
        &[
            S::T("REC"),
            S::T(&format!("{},{}", r0.x, r0.y)),
            S::T(&format!("{},{}", r0.x + px * 100.0, r0.y + px * 60.0)),
        ],
    );
    let t = std::time::Instant::now();
    run(
        &mut d,
        &[S::T("H"), S::C(r0.x + px * 50.0, r0.y + px * 30.0), S::Enter],
    );
    rows.push(("HATCH pick inside a rectangle".into(), ms(t)));
    let t = std::time::Instant::now();
    run(
        &mut d,
        &[
            S::T("DLI"),
            S::T(&format!("{},{}", r0.x, r0.y)),
            S::T(&format!("{},{}", r0.x + px * 100.0, r0.y)),
            S::T(&format!("{},{}", r0.x + px * 50.0, r0.y - px * 20.0)),
        ],
    );
    rows.push(("DIMLINEAR (dimension + its block)".into(), ms(t)));
    let t = std::time::Instant::now();
    d.undo();
    rows.push(("U (undo the dimension)".into(), ms(t)));
    let t = std::time::Instant::now();
    d.redo();
    rows.push(("REDO".into(), ms(t)));
    // the Properties palette: a colour; the Layers palette: a layer off and on, its colour
    d.choose(&[l0], true);
    let t = std::time::Instant::now();
    d.set_prop("Color", "Red").unwrap();
    rows.push(("Properties: colour of 1 entity".into(), ms(t)));
    let layer = d.drawing.objects[&l0].ent.as_ref().unwrap().layer.handle;
    let look = |d: &DwgDoc| match &d.drawing.objects[&layer].data {
        Data::Layer(l) => format!(
            "{} {:?} {} {}",
            l.color.index, l.color.rgb, l.ltype.handle, l.flags
        ),
        _ => String::new(),
    };
    eprintln!("layer before: {}", look(&d));
    let t = std::time::Instant::now();
    d.layer_switch(layer, super::dwgprops::Switch::On).unwrap();
    eprintln!("layer after: {}", look(&d));
    rows.push(("Layers: a layer off".into(), ms(t)));
    eprintln!("layer off redrew {:?}", d.last_sync);
    d.layer_switch(layer, super::dwgprops::Switch::On).unwrap();
    let t = std::time::Instant::now();
    d.layer_colour(layer, 3).unwrap();
    rows.push(("Layers: a layer's colour (recoloured in place)".into(), ms(t)));
    // a window round everything, chosen (highlighted)
    let (lo, hi) = d.viewq().extents().unwrap();
    let t = std::time::Instant::now();
    let all = d.viewq().select_box(lo, hi, false);
    rows.push((format!("window selection of all ({})", all.len()), ms(t)));
    let t = std::time::Instant::now();
    d.choose(&all, true);
    rows.push(("  ... chosen and highlighted".into(), ms(t)));
    d.clear_selection();
    // ten thousand moved, then erased
    let many: Vec<u32> = lines.iter().copied().skip(1000).take(10_000).collect();
    d.choose(&many, true);
    let t = std::time::Instant::now();
    run(
        &mut d,
        &[S::T("M"), S::T("0,0"), S::T(&format!("{},{}", px * 10.0, 0.0))],
    );
    rows.push((format!("MOVE {} entities", many.len()), ms(t)));
    d.choose(&many, true);
    let t = std::time::Instant::now();
    run(&mut d, &[S::T("E")]);
    rows.push((format!("ERASE {} entities", many.len()), ms(t)));
    let t = std::time::Instant::now();
    d.undo();
    rows.push((format!("U (the ERASE of {})", many.len()), ms(t)));
    let t = std::time::Instant::now();
    let out = d.bytes().unwrap();
    rows.push(("save (write the file)".into(), ms(t)));
    eprintln!(
        "\n{n_obj} objects, {n_items} items drawn, {} bytes written",
        out.len()
    );
    eprintln!("| operation | ms |\n|---|---:|");
    for (k, v) in &rows {
        eprintln!("| {k} | {v:.2} |");
    }
    eprintln!("editor's own steps (last 20):");
    for (k, v) in d.ed.times.iter().rev().take(20) {
        eprintln!("  {k}: {v:.2} ms");
    }
}

/// FILLET picked as a hand picks: just off the first line, the second where the arc will cut it.
#[test]
fn fillet_picked_in_the_part_cut() {
    let mut d = doc();
    let before = handles(&d);
    run(
        &mut d,
        &[
            S::T("F"),
            S::T("R"),
            S::T("15"),
            S::C(175.68, 167.45),
            S::C(190.98, 185.92),
        ],
    );
    assert!(!d.ed.active(), "{:?}", d.history.last());
    let arcs: Vec<Curve> = made(&d, &before).iter().map(|h| curve(&d, *h)).collect();
    assert!(matches!(arcs.as_slice(), [Curve::Arc(a)] if a.c.dist(v2(191.0, 171.0)) < 1e-6));
}
