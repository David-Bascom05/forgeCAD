//! Drawings follow their model: dimensions re-measure after a model edit, and one whose edge is
//! gone turns sick (a typed warning with fixes), never measured against something else.

use forge_agent::json::Json;
use forge_lang::drawing::ops::{DrawOp, apply_checked};
use forge_lang::drawing::regen::{self, Drawing};
use std::path::PathBuf;

const PART: &str = "shared w = 60
shared d = 8
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (w, 40))
}
plate = extrude(plan, 10)
sketch hs(on: plate.end) {
  c = pt(20, 20)
}
bore = hole(at: hs.c, d: d, depth: thru)
";

fn dir(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The drawing's references on a view: (ref text, kind, view box).
fn refs(d: &Drawing, view: &str) -> Vec<(String, String, [f64; 4])> {
    let Json::Arr(rs) = regen::refs(d, view).unwrap() else {
        panic!()
    };
    rs.iter()
        .map(|r| {
            let b = |k: &str, i: usize| match r.get(k).unwrap() {
                Json::Arr(a) => a[i].num().unwrap(),
                _ => panic!(),
            };
            (
                r.get("ref").and_then(Json::str).unwrap().to_string(),
                r.get("kind").and_then(Json::str).unwrap().to_string(),
                [b("view_lo", 0), b("view_lo", 1), b("view_hi", 0), b("view_hi", 1)],
            )
        })
        .collect()
}

fn value(d: &Drawing, id: &str) -> Option<f64> {
    d.items.iter().find(|i| i.id == id).and_then(|i| i.value)
}

#[test]
fn dimensions_follow_the_model_and_turn_sick() {
    let dir = dir("drawing_follow");
    std::fs::write(dir.join("plate.fg"), PART).unwrap();
    let base = "drawing\nm = part(\"plate.fg\")\ns = sheet(\"A3\")\ntop = base_view(m, sheet: s, dir: \"top\", at: (150, 150))\nfront = projected(top, at: (150, 80))\n";
    let d = regen::make(base, &dir);
    assert!(d.ok(), "{:?}", d.errors);
    let rs = refs(&d, "top");
    let circle = rs
        .iter()
        .find(|r| r.1 == "circle")
        .expect("the hole's rim")
        .0
        .clone();
    let left = rs
        .iter()
        .find(|r| r.1 == "line" && r.2[0] == r.2[2] && r.2[0].abs() < 1e-9)
        .expect("the left edge")
        .0
        .clone();
    let right = rs
        .iter()
        .find(|r| r.1 == "line" && r.2[0] == r.2[2] && (r.2[0] - 60.0).abs() < 1e-9)
        .expect("the right edge")
        .0
        .clone();
    let src = format!(
        "{base}width = dim(top, {left}, {right}, at: (30, 50))\nhole_x = dim(top, {left}, {circle}, at: (10, 30))\nbore_d = diameter(top, {circle}, at: (40, 50))\nnote_h = hole_note(top, {circle}, at: (40, 60))\n"
    );
    let d = regen::make(&src, &dir);
    assert!(d.ok(), "{:?}", d.errors);
    assert_eq!(value(&d, "width"), Some(60.0));
    assert_eq!(value(&d, "hole_x"), Some(20.0));
    assert_eq!(value(&d, "bore_d"), Some(8.0));
    assert_eq!(value(&d, "note_h"), Some(8.0));
    assert!(d.errors.is_empty(), "{:?}", d.errors);

    // the model changes: the dimensions follow
    std::fs::write(
        dir.join("plate.fg"),
        PART.replace("w = 60", "w = 80").replace("d = 8", "d = 10"),
    )
    .unwrap();
    let d = regen::make(&src, &dir);
    assert_eq!(value(&d, "width"), Some(80.0));
    assert_eq!(value(&d, "hole_x"), Some(20.0));
    assert_eq!(value(&d, "bore_d"), Some(10.0));

    // the hole goes: what referred to it is sick, typed, with a fix; the rest is untouched
    std::fs::write(
        dir.join("plate.fg"),
        PART.replace("bore = hole(at: hs.c, d: d, depth: thru)\n", ""),
    )
    .unwrap();
    let d = regen::make(&src, &dir);
    assert_eq!(value(&d, "width"), Some(60.0));
    let sick: Vec<&str> = d.items.iter().filter(|i| i.sick).map(|i| i.id.as_str()).collect();
    assert_eq!(sick, ["hole_x", "bore_d", "note_h"]);
    assert!(d.ok(), "sick items are warnings: the drawing is still made");
    let e = d
        .errors
        .iter()
        .find(|e| matches!(&e.problem, forge_lang::drawing::doc::Problem::Sick { item, .. } if item == "bore_d"))
        .unwrap();
    assert!(e.warning);
    assert!(e.fixes.contains(&DrawOp::DeleteItem {
        item: "bore_d".into()
    }));
    // the fix applies and leaves a drawing with nothing sick in it
    let fixed = apply_checked(
        &src,
        &DrawOp::DeleteItem {
            item: "bore_d".into(),
        },
    )
    .unwrap();
    let d = regen::make(&fixed, &dir);
    assert!(d.items.iter().all(|i| i.id != "bore_d"));
}

#[test]
fn problems_are_typed_with_spans() {
    let dir = dir("drawing_problems");
    std::fs::write(dir.join("plate.fg"), PART).unwrap();
    let src = "drawing\nm = part(\"plate.fg\")\nv = base_view(m, at: (100, 100), scale: \"big\")\nw = projected(nothing, at: (1, 2))\nx = base_view(m, at: (1, 1), wobble: 3)\n";
    let d = regen::make(src, &dir);
    let kinds: Vec<&str> = d.errors.iter().map(|e| e.problem.kind()).collect();
    assert_eq!(kinds, ["WrongType", "UnknownName", "UnknownArg"]);
    let e = &d.errors[0];
    assert_eq!(&src[e.span.lo as usize..e.span.hi as usize], "\"big\"");
    let missing = regen::make("drawing\nm = part(\"nope.fg\")\n", &dir);
    assert_eq!(missing.errors[0].problem.kind(), "Model");
    assert!(!missing.ok());
}

#[test]
fn pages_write_as_pdf_dxf_and_svg() {
    let dir = dir("drawing_out");
    std::fs::write(dir.join("plate.fg"), PART).unwrap();
    let src = "drawing\nm = part(\"plate.fg\")\ns = sheet(\"A3\", title: \"PLATE\")\ntop = base_view(m, sheet: s, dir: \"top\", at: (150, 150))\na = section(top, [(-10, 20), (70, 20)], at: (150, 80))\nb = detail(top, (20, 20), r: 8, at: (300, 200))\nmarks = center_marks(top)\n";
    let d = regen::make(src, &dir);
    assert!(d.ok(), "{:?}", d.errors);
    let section = d.view("a").unwrap();
    assert!(
        section.extra.iter().any(|s| s.pen == forge_draw::Pen::Hatch),
        "the cut face is hatched"
    );
    let detail = d.view("b").unwrap();
    assert!(
        detail.pieces.iter().any(|p| p.geom.circle().is_some()),
        "the hole is in the detail"
    );
    let pdf = forge_draw::pdf::pdf(&d.pages);
    assert!(pdf.starts_with(b"%PDF"));
    let dxf = forge_draw::dxf::dxf(&d.pages[0]);
    assert!(dxf.contains("\nHATCH\n") && dxf.contains("\nCIRCLE\n"));
    let svg = forge_draw::svg::svg(&d.pages[0]);
    assert!(svg.contains("SECTION A-A"));
}
