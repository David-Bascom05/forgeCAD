//! Sheets and thin walls in .fg: the measures (a part's volume and area its solids', the sheets'
//! area apart), which bodies are sheets, and the refusals.

fn run(src: &str) -> forge_lang::Run {
    forge_lang::run(src, forge_lang::Options::default(), Default::default()).0
}

const EXAMPLE: &str = include_str!("../../../examples/surfaces/sheets.fg");
const THIN: &str = include_str!("../../../examples/surfaces/thin.fg");

#[test]
fn the_example_measures_its_solids_and_its_sheets_apart() {
    let r = run(EXAMPLE);
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let m = r.measure.as_ref().expect("measured");
    assert_eq!(m.problems, 0);
    // the base with the boss through it: 40 x 30 x (6 + 3)
    assert!((m.volume - 10800.0).abs() < 1e-9, "{}", m.volume);
    let pi = std::f64::consts::PI;
    // the wall: 40 then a quarter of r 10, 20 high; the shade: a quarter turn of 40 at r 10
    let want = 20.0 * (40.0 + 5.0 * pi) + 40.0 * 5.0 * pi;
    let got = forge_lang::report::sheet_area(&r).expect("sheets");
    assert!((got - want).abs() < 1e-9 * want, "{got} {want}");
    let sheets: Vec<bool> = r.solids.iter().map(|s| s.sheet).collect();
    assert_eq!(sheets.iter().filter(|&&s| s).count(), 2, "{sheets:?}");
}

#[test]
fn thin_walls_are_solids_mitred_at_their_corners() {
    let r = run(THIN);
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let m = r.measure.as_ref().expect("measured");
    let pi = std::f64::consts::PI;
    let (t, h, l, w, rr) = (2.0, 10.0, 40.0, 25.0, 5.0);
    let want = h * ((l + w) * t + t * t) + h * t * ((l - rr) + rr * pi / 2.0);
    assert!((m.volume - want).abs() < 1e-9 * want, "{} {want}", m.volume);
    assert!(r.solids.iter().all(|s| !s.sheet));
}

#[test]
fn refusals() {
    let head = "shared t = 2\nsketch s(on: xy) {\n  path w(start: (0, 0)) {\n    line(to: (10, 0))\n  }\n}\n";
    for (tail, word) in [
        ("x = extrude(s.w, 5, surface: true, taper: 2deg)\n", "NotWith"),
        ("x = extrude(s.w, 5, surface: true, thin: t)\n", "NotWith"),
        ("x = extrude(s.w, 5, side: \"in\")\n", "NotWith"),
        ("x = extrude(s.w, thru, surface: true)\n", "NotWith"),
        ("x = cut(s.w, 5, surface: true)\n", "NotWith"),
    ] {
        let r = run(&format!("{head}{tail}"));
        let e = format!("{:?}", r.errors);
        assert!(e.contains(word), "{tail}: {e}");
    }
}

#[test]
fn a_list_of_loops_is_a_tube_each_whatever_else_the_sketch_draws() {
    // the rect and the circle are tubes of their own, each thickened to its outside; the open
    // lines beside them leave them be
    let r = run(r#"shared t = 1
shared h = 5
sketch s(on: xy) {
  plate = rect(corner: (0, 0), size: (40, 30))
  bore = circle((20, 15), 6)
  l = line((50, 0), (60, 10))
  ax = line((-10, 0), (-10, 10))
}
a = extrude(s.plate, h, surface: true)
b = extrude([s.plate, s.bore], h, thin: t, side: "out")
c = revolve(s.bore, axis: s.ax, angle: 90deg, surface: true)
"#);
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let m = r.measure.as_ref().expect("measured");
    assert_eq!(m.problems, 0);
    let pi = std::f64::consts::PI;
    let want = 5.0 * (42.0 * 32.0 - 40.0 * 30.0 + pi * (49.0 - 36.0));
    assert!((m.volume - want).abs() < 1e-9 * want, "{} {want}", m.volume);
    let want = 140.0 * 5.0 + 2.0 * pi * 6.0 * 30.0 * pi / 2.0;
    let got = forge_lang::report::sheet_area(&r).expect("sheets");
    assert!((got - want).abs() < 1e-9 * want, "{got} {want}");
}
