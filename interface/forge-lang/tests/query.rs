//! Queries on the load tray example (examples/sheetmetal): every number checked against the
//! drawing's arithmetic, and the same answer every run.

use forge_agent::json::Json;
use forge_lang::query::ask_file;
use forge_math::dmath;
use std::path::PathBuf;

fn ex(f: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/sheetmetal")
        .join(f)
}

fn ask(f: &str, q: &str) -> Json {
    let a = ask_file(&ex(f), None, q);
    assert!(a.ok, "{q}: {}", a.text);
    a.json
}

fn n(j: &Json, k: &str) -> f64 {
    j.get(k)
        .and_then(Json::num)
        .unwrap_or_else(|| panic!("{k} in {j}"))
}

fn v3(j: &Json, k: &str) -> [f64; 3] {
    match j.get(k) {
        Some(Json::Arr(x)) => [0, 1, 2].map(|i| x[i].num().expect("a number")),
        _ => panic!("{k} in {j}"),
    }
}

fn arr<'a>(j: &'a Json, k: &str) -> &'a [Json] {
    match j.get(k) {
        Some(Json::Arr(x)) => x,
        _ => panic!("{k} in {j}"),
    }
}

fn close(a: [f64; 3], b: [f64; 3]) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < 1e-6)
}

#[test]
fn every_hole_of_the_tray_where_its_sketch_put_it() {
    let j = ask("load_tray.fg", "holes");
    let h = arr(&j, "holes");
    assert_eq!(h.len(), 9);
    // t 1.5 below the floor's top skin; walls' outer skins on x = 0 and 160; wall holes 40 up the
    // wall from the floor's top skin, 25 and 85 from the front; the tab's 7.5 down, mid-tab
    let want: [(&str, [f64; 3], [f64; 3], f64); 9] = [
        ("left_holes.faces#0", [0.75, 25.0, 40.0], [1.0, 0.0, 0.0], 4.2),
        ("left_holes.faces#1", [0.75, 85.0, 40.0], [1.0, 0.0, 0.0], 4.2),
        ("tab_hole.faces", [80.0, -17.25, -7.5], [0.0, 1.0, 0.0], 4.2),
        ("right_holes.faces#0", [159.25, 25.0, 40.0], [1.0, 0.0, 0.0], 4.2),
        ("right_holes.faces#1", [159.25, 85.0, 40.0], [1.0, 0.0, 0.0], 4.2),
        ("tray.faces#0", [20.0, 20.0, -0.75], [0.0, 0.0, 1.0], 5.0),
        ("tray.faces#1", [20.0, 100.0, -0.75], [0.0, 0.0, 1.0], 5.0),
        ("tray.faces#2", [140.0, 20.0, -0.75], [0.0, 0.0, 1.0], 5.0),
        ("tray.faces#3", [140.0, 100.0, -0.75], [0.0, 0.0, 1.0], 5.0),
    ];
    for (h, (name, c, a, d)) in h.iter().zip(want) {
        assert_eq!(h.get("name").and_then(Json::str), Some(name), "{h}");
        assert!(close(v3(h, "centre"), c), "{h}");
        assert!(close(v3(h, "axis"), a), "{h}");
        assert!(
            (n(h, "d") - d).abs() < 1e-9 && (n(h, "depth") - 1.5).abs() < 1e-9,
            "{h}"
        );
        assert_eq!(h.get("thru"), Some(&Json::Bool(true)));
    }
    // the same text, the same answer
    let mut a = String::new();
    let mut b = String::new();
    j.write(&mut a);
    ask("load_tray.fg", "holes").write(&mut b);
    assert_eq!(a, b);
    // in the assembly: the rack's, at its pose (x - 1.7, the plate's front skin at y = -25)
    let j = ask("tray_asm.fg", "holes of rack d 4.2");
    let h = arr(&j, "holes");
    assert_eq!(h.len(), 4);
    assert_eq!(
        h[0].get("name").and_then(Json::str),
        Some("rack/left_holes.faces#0")
    );
    assert!(close(v3(&h[0], "centre"), [-0.95, 85.0, 40.0]), "{}", h[0]);
    // asked in the rack's own frame: its ear's sketch numbers (50 and 110 back, 60 up)
    let j = ask("tray_asm.fg", "holes of rack in rack");
    let c: Vec<[f64; 3]> = arr(&j, "holes").iter().map(|h| v3(h, "centre")).collect();
    assert!(c.iter().any(|x| close(*x, [162.65, 60.0, -50.0])), "{c:?}");
    assert!(c.iter().any(|x| close(*x, [162.65, 60.0, -110.0])), "{c:?}");
}

#[test]
fn a_section_is_the_outline_in_numbers() {
    // across the tray at y = 60: floor 0..-1.5, walls 0..1.5 and 158.5..160 up to 58.5, lips in
    // to 12 and 148, bends r 1.5 inside, 3 outside
    let j = ask("load_tray.fg", "section y=60");
    let l = arr(&j, "loops");
    assert_eq!(l.len(), 1);
    assert_eq!(l[0].get("kind").and_then(Json::str), Some("outline"));
    assert!(close(v3(&l[0], "min"), [0.0, 60.0, -1.5]) && close(v3(&l[0], "max"), [160.0, 60.0, 58.5]));
    // straight runs: the floor 154, two walls 54 each, two lips 9 each, the lips' ends, all both
    // skins; four bends each side as quarter circles
    let pi = dmath::PI;
    let floor = 157.0 - 3.0;
    let straight = 2.0 * floor / 2.0 * 2.0 + 4.0 * 54.0 + 4.0 * 9.0 + 2.0 * 1.5;
    let arcs = 4.0 * (pi / 2.0) * (1.5 + 3.0);
    assert!(
        (n(&l[0], "length") - (straight + arcs)).abs() < 1e-6,
        "{}",
        n(&l[0], "length")
    );
    let mid = 1.5 * (floor + 2.0 * 54.0 + 2.0 * 9.0) + 4.0 * (pi / 4.0) * (9.0 - 2.25);
    assert!((n(&j, "area") - mid).abs() < 1e-6, "{} {mid}", n(&j, "area"));
    let arcs: Vec<&Json> = arr(&l[0], "pieces")
        .iter()
        .filter(|p| p.get("kind").and_then(Json::str) == Some("arc"))
        .collect();
    assert_eq!(arcs.len(), 8);
    assert!(
        arcs.iter()
            .any(|a| close(v3(a, "centre"), [3.0, 60.0, 1.5]) && n(a, "r") == 3.0)
    );
    // at z = 40, through the wall holes: each wall in three pieces, gaps 4.2 wide about y 25, 85
    let j = ask("tray_asm.fg", "section z=40 of tray");
    let l = arr(&j, "loops");
    assert_eq!(
        l.len(),
        6,
        "three pieces of each side wall; the rear wall stops at 38.5"
    );
    let left: Vec<[f64; 3]> = l
        .iter()
        .filter(|x| v3(x, "max")[0] == 1.5)
        .map(|x| [v3(x, "min")[1], v3(x, "max")[1], 0.0])
        .collect();
    for y in [[0.0, 22.9, 0.0], [27.1, 82.9, 0.0], [87.1, 118.1, 0.0]] {
        assert!(left.iter().any(|x| close(*x, y)), "{left:?}");
    }
}

#[test]
fn holes_on_one_axis_across_parts() {
    let j = ask("tray_asm.fg", "aligned");
    let axes = arr(&j, "axes");
    // y 25 and 85, z 40: each through both walls and both ears
    assert_eq!(axes.len(), 2);
    for a in axes {
        assert_eq!(arr(a, "holes").len(), 4);
        assert_eq!(n(a, "offset"), 0.0);
    }
    let j = ask(
        "tray_asm.fg",
        "aligned tray/right_holes.faces#1 rack/right_holes.faces#0",
    );
    assert_eq!(j.get("coaxial"), Some(&Json::Bool(true)));
    // the left wall drawn with the right wall's numbers: its y runs from the rear, so 25 lands at
    // 95 and 85 at 35, 10 off the ear's 85 and 25
    let j = ask("probes/tray_misread_asm.fg", "aligned");
    assert_eq!(arr(&j, "axes").len(), 0);
    let j = ask(
        "probes/tray_misread_asm.fg",
        "aligned tray/left_holes.faces#0 rack/left_holes.faces#1",
    );
    assert_eq!(j.get("coaxial"), Some(&Json::Bool(false)));
    assert!((n(&j, "offset") - 10.0).abs() < 1e-9);
}

#[test]
fn the_gap_between_an_ear_and_its_wall() {
    let j = ask("tray_asm.fg", "gap rack/right_ear.bottom tray/right.bottom");
    assert!((n(&j, "gap") - 0.2).abs() < 1e-9, "{j}");
    assert_eq!(j.get("parallel"), Some(&Json::Bool(true)));
    // by a hole's name: the ear's hole wall to the wall's, across the 0.2
    let j = ask(
        "tray_asm.fg",
        "gap rack/left_holes.faces#1 tray/left_holes.faces#0",
    );
    assert!((n(&j, "gap") - 0.2).abs() < 1e-6, "{j}");
}

#[test]
fn where_a_clash_is_and_what_it_touches() {
    let j = ask("probes/tray_clash.fg", "interference");
    let c = arr(&j, "interferences");
    assert_eq!(c.len(), 1);
    // 0.5 deep over the wall's straight 54 and part of each bend (r 3 outside, where its skin is
    // within 0.5 of x = 0), 95 along, less the two shared holes
    let u = 2.75f64.sqrt();
    let f = |u: f64| u / 2.0 * (9.0 - u * u).sqrt() + 4.5 * dmath::asin(u / 3.0);
    let bend = f(u) - f(0.0) - 2.5 * u;
    let want = 95.0 * (0.5 * 54.0 + 2.0 * bend) - 2.0 * dmath::PI * 2.1 * 2.1 * 0.5;
    assert!(
        (n(&c[0], "volume") - want).abs() < 1e-6 * want,
        "{} {want}",
        n(&c[0], "volume")
    );
    assert!(
        close(v3(&c[0], "min"), [0.0, 0.0, 1.5 - u]),
        "{:?}",
        v3(&c[0], "min")
    );
    assert!((v3(&c[0], "max")[1] - 95.0).abs() < 1e-6);
    let faces: Vec<&str> = arr(&c[0], "faces_a").iter().filter_map(Json::str).collect();
    assert!(
        faces.contains(&"tray/left.bottom") && faces.contains(&"tray/left.bend"),
        "{faces:?}"
    );
    let faces: Vec<&str> = arr(&c[0], "faces_b").iter().filter_map(Json::str).collect();
    assert!(faces.contains(&"rack/left_ear.bottom"), "{faces:?}");
    assert_eq!(n(&ask("tray_asm.fg", "interference"), "volume"), 0.0);
}

#[test]
fn where_a_sketch_on_a_wall_lands() {
    // the right wall's sketch: origin at the front of its inner skin, x up, y to the rear
    let j = ask("load_tray.fg", "frame rh");
    assert!(close(v3(&j, "origin"), [158.5, 0.0, 0.0]) && close(v3(&j, "x"), [0.0, 0.0, 1.0]));
    assert!(close(v3(&j, "y"), [0.0, 1.0, 0.0]));
    // the left wall's: from the rear, y to the front
    let j = ask("load_tray.fg", "frame lh");
    assert!(close(v3(&j, "origin"), [1.5, 120.0, 0.0]) && close(v3(&j, "y"), [0.0, -1.0, 0.0]));
    let s = j.get("sheet").expect("on the sheet");
    assert_eq!(s.get("wall").and_then(Json::str), Some("left"));
    // on the flat its x runs back the way the floor's does (turned 180)
    let fx = match s.get("flat_x") {
        Some(Json::Arr(x)) => [x[0].num().unwrap(), x[1].num().unwrap()],
        _ => panic!(),
    };
    assert!((fx[0] + 1.0).abs() < 1e-9 && fx[1].abs() < 1e-9);
    // in the assembly, by occurrence: the rack's left ear, its y down from the plate's top
    let j = ask("tray_asm.fg", "frame rack/left_ear.top");
    assert!(close(v3(&j, "origin"), [-1.7, -25.0, 80.0]), "{j}");
    assert!(
        close(v3(&j, "y"), [0.0, 0.0, -1.0]) && close(v3(&j, "x"), [0.0, 1.0, 0.0]),
        "{j}"
    );
}

#[test]
fn a_query_that_names_nothing_is_a_typed_refusal() {
    for (q, kind) in [
        ("gap tray/nope rack/left_ear.top", "NoSuchFace"),
        ("holes of shelf", "NoSuchOccurrence"),
        ("frame tray/plan.outline", "NotAPlane"),
        ("sideways", "Query"),
    ] {
        let a = ask_file(&ex("tray_asm.fg"), None, q);
        assert!(!a.ok);
        let got = a
            .json
            .get("error")
            .and_then(|e| e.get("kind"))
            .and_then(Json::str);
        assert_eq!(got, Some(kind), "{q}: {}", a.text);
    }
}

fn run(f: &str) -> forge_lang::Run {
    let p = ex(f);
    let src = std::fs::read_to_string(&p).expect("the example");
    let opts = forge_lang::Options {
        dir: p.parent().map(|d| d.to_path_buf()),
        ..forge_lang::Options::default()
    };
    forge_lang::run(&src, opts, Default::default()).0
}

#[test]
fn an_assemblys_expects_check_gaps_and_shared_axes() {
    let r = run("tray_asm.fg");
    let a = r.asm.as_ref().expect("an assembly");
    let got: Vec<(&str, f64)> = a
        .expects
        .iter()
        .map(|e| (e.metric.as_str(), e.measured.expect("measured")))
        .collect();
    assert!(a.expects.iter().all(|e| e.ok), "{got:?}");
    assert!(
        got.iter()
            .filter(|g| g.0 == "gap")
            .all(|g| (g.1 - 0.2).abs() < 1e-9),
        "{got:?}"
    );
    assert!(
        got.iter()
            .filter(|g| g.0.starts_with("axis_"))
            .all(|g| g.1.abs() < 1e-9)
    );
    // the misread wall: its first hole 70 off the ear's (y 95 against 25)
    let r = run("probes/tray_misread_asm.fg");
    let e = &r.asm.as_ref().expect("an assembly").expects[0];
    assert!(!e.ok && (e.measured.expect("measured") - 70.0).abs() < 1e-9);
    // a part's own count of round holes
    let r = run("load_tray.fg");
    assert!(
        r.expects.iter().all(|e| e.ok) && r.errors.is_empty(),
        "{:?}",
        r.errors
    );
}

#[test]
fn a_vendor_step_is_read_like_a_part() {
    // vendor/bracket.step, made from bracket_src.fg: a 1.2 L bracket, two 3.5 holes on a 24 pitch
    // in its 40 x 30 foot, one 4.5 hole 12.5 up its upright at y 28.8..30
    let j = ask("vendor/vendor.fg", "holes");
    let h = arr(&j, "holes");
    assert_eq!(h.len(), 3);
    assert!(close(v3(&h[0], "centre"), [8.0, 15.0, -0.6]) && n(&h[0], "d") == 3.5);
    assert!(close(v3(&h[1], "centre"), [32.0, 15.0, -0.6]));
    assert!(close(v3(&h[2], "centre"), [20.0, 29.4, 12.5]) && close(v3(&h[2], "axis"), [0.0, 1.0, 0.0]));
    let j = ask("vendor/vendor.fg", "tabs");
    let t = arr(&j, "tabs");
    assert_eq!(t.len(), 2, "{j}");
    for x in t {
        assert!((n(x, "thickness") - 1.2).abs() < 1e-9);
    }
    let holes: Vec<usize> = t.iter().map(|x| arr(x, "holes").len()).collect();
    assert!(holes.contains(&2) && holes.contains(&1), "{holes:?}");
    // a section square to the upright, 10 above the foot: one strip 40 by 1.2
    let j = ask("vendor/vendor.fg", "section z=10");
    let l = arr(&j, "loops");
    assert_eq!(l.len(), 1);
    assert!((n(&j, "area") - 40.0 * 1.2).abs() < 1e-6);
}
