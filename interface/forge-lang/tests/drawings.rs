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

fn rules(d: &Drawing) -> Vec<(String, &'static str)> {
    d.errors
        .iter()
        .filter_map(|e| match &e.problem {
            forge_lang::drawing::doc::Problem::Gdt { item, rule, .. } => Some((item.clone(), rule.word())),
            _ => None,
        })
        .collect()
}

#[test]
fn gdt_is_checked_against_the_standard() {
    let dir = dir("drawing_gdt");
    std::fs::write(dir.join("plate.fg"), PART).unwrap();
    let base = "drawing\nm = part(\"plate.fg\")\ns = sheet(\"A3\")\ntop = base_view(m, sheet: s, dir: \"top\", at: (150, 150))\n";
    let d = regen::make(base, &dir);
    let rs = refs(&d, "top");
    let circle = rs.iter().find(|r| r.1 == "circle").unwrap().0.clone();
    let line_at = |x: f64| {
        rs.iter()
            .find(|r| r.1 == "line" && r.2[0] == r.2[2] && (r.2[0] - x).abs() < 1e-9)
            .unwrap()
            .0
            .clone()
    };
    let (left, right) = (line_at(0.0), line_at(60.0));
    let src = format!(
        "{base}da = datum(top, {left}, \"A\", at: (-10, 10))
db = datum(top, {right}, \"A\", at: (70, 10))
dc = datum(top, {right}, \"I\", at: (70, 30))
f1 = fcf(top, {left}, \"flatness\", \"0.1\", [\"A\"], at: (-30, 0))
f2 = fcf(top, {left}, \"position\", \"dia 0.2 M\", [\"A\", \"D\"], at: (-30, -10))
f3 = fcf(top, {circle}, \"circular_runout\", \"0.1\", [], at: (70, 50))
f4 = fcf(top, {circle}, \"concentricity\", \"dia 0.1\", [\"A\"], at: (70, 60))
f5 = fcf(top, {left}, \"flatness\", \"dia 0.1\", at: (-30, 20))
hole_x = dim(top, {left}, {circle}, at: (10, 30))
f6 = fcf(top, {circle}, \"position\", \"dia 0.1\", [\"A\"], at: (70, 70))
bore = diameter(top, {circle}, fit: \"H7\", at: (40, 50))
f7 = fcf(top, bore, \"perpendicularity\", \"dia 0.05 M\", [\"A\"], at: (40, 60))
"
    );
    let d = regen::make(&src, &dir);
    let got = rules(&d);
    let want = [
        ("db", "DatumDuplicate"),
        ("dc", "DatumLetter"),
        ("f1", "NoDatumsAllowed"),
        ("f2", "ModifierNeedsSize"),
        ("f2", "DatumUndefined"),
        ("f3", "DatumsNeeded"),
        ("f4", "NotInStandard"),
        ("f5", "DiameterNotAllowed"),
        ("f6", "PositionNotBasic"),
    ];
    for w in want {
        assert!(
            got.iter().any(|g| g.0 == w.0 && g.1 == w.1),
            "{w:?} not in {got:?}"
        );
    }
    // f7 is on the bore's size (a feature of size): MMC is allowed there
    assert!(!got.iter().any(|g| g.0 == "f7"), "{got:?}");
    assert_eq!(d.items.iter().find(|i| i.id == "bore").unwrap().value, Some(8.0));
    // every fix offered applies and clears its own problem
    for e in d.errors.iter().filter(|e| !e.fixes.is_empty()) {
        let forge_lang::drawing::doc::Problem::Gdt { item, rule, .. } = &e.problem else {
            continue;
        };
        let op = e
            .fixes
            .iter()
            .find(|f| !matches!(f, DrawOp::DeleteItem { .. }))
            .unwrap_or(&e.fixes[0]);
        let fixed = forge_lang::drawing::ops::apply(&src, op).unwrap();
        let again = rules(&regen::make(&fixed, &dir));
        assert!(
            !again.iter().any(|g| &g.0 == item && g.1 == rule.word()),
            "{op:?} left {item} {}",
            rule.word()
        );
    }
    // ISO keeps concentricity
    let iso = src.replace("s = sheet(\"A3\")", "s = sheet(\"A3\", standard: \"ISO\")");
    assert!(
        !rules(&regen::make(&iso, &dir))
            .iter()
            .any(|g| g.1 == "NotInStandard")
    );
    // the frame's edge going away makes it sick, like any annotation
    std::fs::write(
        dir.join("plate.fg"),
        PART.replace("bore = hole(at: hs.c, d: d, depth: thru)\n", ""),
    )
    .unwrap();
    let d = regen::make(&src, &dir);
    assert!(d.items.iter().find(|i| i.id == "f6").unwrap().sick);
}

#[test]
fn tolerances_are_written_and_fits_computed() {
    use forge_draw::fits::deviations;
    let (hi, lo) = deviations(8.0, "H7").unwrap();
    assert_eq!(((hi * 1000.0).round(), lo), (15.0, 0.0));
    let dir = dir("drawing_tol");
    std::fs::write(dir.join("plate.fg"), PART).unwrap();
    let base = "drawing\nm = part(\"plate.fg\")\ns = sheet(\"A3\")\ntop = base_view(m, sheet: s, dir: \"top\", at: (150, 150))\n";
    let d = regen::make(base, &dir);
    let circle = refs(&d, "top").into_iter().find(|r| r.1 == "circle").unwrap().0;
    let texts = |extra: &str| {
        let d = regen::make(
            &format!("{base}x = diameter(top, {circle}, at: (40, 50){extra})\n"),
            &dir,
        );
        assert!(d.ok(), "{:?}", d.errors);
        d.pages[0]
            .texts
            .iter()
            .map(|t| t.text.clone())
            .collect::<Vec<_>>()
    };
    assert!(texts(", tol: 0.05").iter().any(|t| t == "\u{2300}8 \u{b1}0.05"));
    let fit = texts(", fit: \"H7\"");
    assert!(
        fit.iter().any(|t| t == "\u{2300}8 H7")
            && fit.iter().any(|t| t == "+0.015")
            && fit.iter().any(|t| t == "0"),
        "{fit:?}"
    );
    let lim = texts(", limits: (0.1, -0.1)");
    assert!(
        lim.iter().any(|t| t == "\u{2300}8.1") && lim.iter().any(|t| t == "\u{2300}7.9"),
        "{lim:?}"
    );
    assert!(texts(", reference: true").iter().any(|t| t == "(\u{2300}8)"));
    assert!(texts(", bound: \"max\"").iter().any(|t| t == "\u{2300}8 MAX"));
    let bad = regen::make(
        &format!("{base}x = diameter(top, {circle}, fit: \"Q7\", at: (40, 50))\n"),
        &dir,
    );
    assert!(!bad.ok());
}

#[test]
fn position_needs_basic_location_both_ways_and_dims_shun_hidden_lines() {
    let dir = dir("drawing_located");
    std::fs::write(dir.join("plate.fg"), PART).unwrap();
    let base = "drawing\nm = part(\"plate.fg\")\ns = sheet(\"A3\")\ntop = base_view(m, sheet: s, dir: \"top\", at: (150, 150))\n";
    let d = regen::make(base, &dir);
    let rs = refs(&d, "top");
    let circle = rs.iter().find(|r| r.1 == "circle").unwrap().0.clone();
    let left = rs
        .iter()
        .find(|r| r.1 == "line" && r.2[0] == r.2[2] && r.2[0].abs() < 1e-9)
        .unwrap()
        .0
        .clone();
    let bottom = rs
        .iter()
        .find(|r| r.1 == "line" && r.2[1] == r.2[3] && r.2[1].abs() < 1e-9)
        .unwrap()
        .0
        .clone();
    let one_way = format!(
        "{base}da = datum(top, {left}, \"A\", at: (-10, 10))\nx = dim(top, {left}, {circle}, basic: true, at: (10, 50))\np = fcf(top, {circle}, \"position\", \"dia 0.1\", [\"A\"], at: (70, 40))\n"
    );
    let got = rules(&regen::make(&one_way, &dir));
    assert!(got.contains(&("p".into(), "PositionNotLocated")), "{got:?}");
    let both = format!("{one_way}y = dim(top, {bottom}, {circle}, basic: true, at: (-15, 10))\n");
    let got = rules(&regen::make(&both, &dir));
    assert!(!got.iter().any(|g| g.0 == "p"), "{got:?}");
    // a blind hole seen from below is only a hidden circle: dimensioning it there is warned
    std::fs::write(dir.join("plate.fg"), PART.replace("depth: thru", "depth: 4")).unwrap();
    let under = "drawing\nm = part(\"plate.fg\")\ns = sheet(\"A3\")\nbot = base_view(m, sheet: s, dir: \"bottom\", at: (150, 150))\n";
    let d = regen::make(under, &dir);
    let hidden = refs(&d, "bot")
        .into_iter()
        .find(|r| r.1 == "circle")
        .expect("the hidden rim")
        .0;
    let d = regen::make(
        &format!("{under}q = diameter(bot, {hidden}, at: (30, 30))\n"),
        &dir,
    );
    assert!(
        d.errors
            .iter()
            .any(|e| e.problem.kind() == "HiddenOnly" && e.warning),
        "{:?}",
        d.errors
    );
}

#[test]
fn frames_hang_under_their_dimension() {
    let dir = dir("drawing_hang");
    std::fs::write(dir.join("plate.fg"), PART).unwrap();
    let base = "drawing\nm = part(\"plate.fg\")\ns = sheet(\"A3\")\ntop = base_view(m, sheet: s, dir: \"top\", at: (150, 150))\n";
    let d = regen::make(base, &dir);
    let circle = refs(&d, "top").into_iter().find(|r| r.1 == "circle").unwrap().0;
    let src = format!(
        "{base}dd = diameter(top, {circle}, fit: \"H7\", at: (40, 50))\nf = fcf(top, dd, \"perpendicularity\", \"dia 0.02\", [\"A\"], at: (200, 200))\nda = datum(top, dd, \"A\")\n"
    );
    let d = regen::make(&src, &dir);
    let it = |id: &str| d.items.iter().find(|i| i.id == id).unwrap().clone();
    let (dd, f, da) = (it("dd"), it("f"), it("da"));
    // the frame sits right under the callout's text whatever `at` says, the datum under it
    assert!(
        f.hi.y <= dd.anchor.y - dd.half.y + 1e-6 && f.hi.y > dd.anchor.y - dd.half.y - 3.0,
        "{f:?} {dd:?}"
    );
    assert!((f.lo.x - (dd.anchor.x - dd.half.x)).abs() < 1.0, "{f:?} {dd:?}");
    assert!(da.lo.y < f.lo.y);
}

#[test]
fn mass_comes_from_the_material_library() {
    let dir = dir("drawing_mass");
    std::fs::write(dir.join("plate.fg"), PART).unwrap();
    let d = regen::make(
        "drawing\nm = part(\"plate.fg\", material: \"6061-T6\")\ns = sheet(\"A3\")\nv = base_view(m, sheet: s, at: (100, 100))\n",
        &dir,
    );
    let m = &d.models[0];
    let want = (60.0 * 40.0 * 10.0 - std::f64::consts::PI * 16.0 * 10.0) * 1e-9 * 2700.0;
    assert!((m.mass().unwrap() - want).abs() < 1e-6, "{:?}", m.mass());
    assert!(
        d.pages[0]
            .texts
            .iter()
            .any(|t| t.text == format!("{} kg", forge_draw::annot::number(want, 3)))
    );
    assert!(d.pages[0].texts.iter().any(|t| t.text == "6061-T6"));
    // an unknown material is said, with the names near it
    let d = regen::make(
        "drawing\nm = part(\"plate.fg\", material: \"6016-T6\")\nv = base_view(m, at: (100, 100))\n",
        &dir,
    );
    assert!(
        d.errors
            .iter()
            .any(|e| e.warning && e.problem.text().contains("6061-T6")),
        "{:?}",
        d.errors
    );
}

#[test]
fn breaks_close_up_and_crops_clip_but_dimensions_stay_true() {
    let dir = dir("drawing_break");
    std::fs::write(dir.join("plate.fg"), PART).unwrap();
    let base = "drawing\nm = part(\"plate.fg\")\ns = sheet(\"A3\")\ntop = base_view(m, sheet: s, dir: \"top\", at: (150, 150))\n";
    let d = regen::make(base, &dir);
    let rs = refs(&d, "top");
    let line_at = |x: f64| {
        rs.iter()
            .find(|r| r.1 == "line" && r.2[0] == r.2[2] && (r.2[0] - x).abs() < 1e-9)
            .unwrap()
            .0
            .clone()
    };
    let (left, right) = (line_at(0.0), line_at(60.0));
    let whole = d.view("top").unwrap().hi.x - d.view("top").unwrap().lo.x;
    let src = format!("{base}w = dim(top, {left}, {right}, at: (30, 50))\n")
        .replace("at: (150, 150))", "at: (150, 150), break: (\"x\", 30, 50))");
    let d = regen::make(&src, &dir);
    assert!(d.ok(), "{:?}", d.errors);
    let v = d.view("top").unwrap();
    // 20 taken out, 6 mm left between the sides
    assert!(
        (v.hi.x - v.lo.x - (whole - 20.0 + 6.0)).abs() < 1e-6,
        "{} vs {whole}",
        v.hi.x - v.lo.x
    );
    assert_eq!(d.items.iter().find(|i| i.id == "w").unwrap().value, Some(60.0));
    let cropped = regen::make(
        &base.replace("at: (150, 150))", "at: (150, 150), crop: [(0, 0), (30, 40)])"),
        &dir,
    );
    let v = cropped.view("top").unwrap();
    assert!((v.hi.x - v.lo.x - 30.0).abs() < 1e-6);
}

/// The item numbers a drawing's balloons leave out, from its one warning about them.
fn unballooned(d: &Drawing) -> Vec<usize> {
    let w: Vec<String> = d
        .errors
        .iter()
        .map(|e| e.problem.text())
        .filter(|t| t.contains("seen in no ballooned view"))
        .collect();
    assert!(w.len() <= 1, "{w:?}");
    w.first().map_or(vec![], |t| {
        t.split("items ")
            .nth(1)
            .unwrap()
            .split(" are")
            .next()
            .unwrap()
            .split(", ")
            .map(|n| n.parse().unwrap())
            .collect()
    })
}

#[test]
fn balloons_hidden_parts_in_a_section_numbered_by_the_first_view() {
    let caster =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/rack18u/casters/caster_lock.fg");
    let base = format!(
        "drawing\nc = assembly(\"{}\")\ns = sheet(\"C\")\nex = base_view(c, sheet: s, dir: \"iso\", explode: 1, hidden: false, at: (150, 250))\nb = auto_balloons(ex)\n",
        caster.display()
    );
    let d = regen::make(&base, &dir("drawing_balloons"));
    assert!(d.ok(), "{:?}", d.errors);
    // the bearing's balls, rings and cage are inside the wheel: no balloon in the iso
    let inside = unballooned(&d);
    assert!(inside.len() >= 4, "{inside:?}");
    let src = format!(
        "{base}side = base_view(c, sheet: s, dir: \"front\", scale: 1/2, at: (40, 340))\nsec = section(side, line: [(14, 30), (14, -45), (1, -45), (1, -150)], at: (300, 340), scale: 1)\nsb = auto_balloons(sec, from: ex)\n"
    );
    let d = regen::make(&src, &dir("drawing_balloons"));
    assert!(d.ok(), "{:?}", d.errors);
    let left = unballooned(&d);
    // the section shows most of them, and balloons none twice
    assert!(
        left.len() < inside.len() && left.iter().all(|n| inside.contains(n)),
        "{inside:?} -> {left:?}"
    );
}

/// A part file every model of a drawing uses is loaded once, though the models ask it for
/// different references (the rack and the caster and door drawn on their own): the rack's general
/// assembly loads each of its 68 part files once.
#[test]
fn each_part_file_of_a_drawing_is_loaded_once() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/rack18u/drawings");
    let src = std::fs::read_to_string(dir.join("rack_ga.fg")).expect("rack_ga.fg");
    let d = regen::make(&src, &dir);
    assert!(d.ok(), "{:?}", d.errors);
    let mut files = d.part_loads.clone();
    files.sort();
    files.dedup();
    assert_eq!(
        d.part_loads.len(),
        files.len(),
        "a file loaded twice: {:?}",
        d.part_loads
    );
    assert_eq!(files.len(), 68);
}
