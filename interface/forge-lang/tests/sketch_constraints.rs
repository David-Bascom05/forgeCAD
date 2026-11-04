//! Concentric, collinear and symmetric hold what they say, measured by what the sketch extrudes.

fn volume(src: &str) -> f64 {
    let r = forge_lang::run(src, forge_lang::Options::default(), Default::default()).0;
    assert!(r.errors.is_empty(), "{:?}\n{src}", r.errors);
    r.measure.expect("built").volume
}

#[test]
fn concentric_centres_a_circle_on_another() {
    let v = volume(
        "sketch s(on: xy) {
  a = circle((0, 0), 10)
  b = circle(near(3, 2), 4)
  concentric(a, b)
}
ring = extrude(s, 1)
",
    );
    assert!((v - std::f64::consts::PI * (100.0 - 16.0)).abs() < 1e-6, "{v}");
}

#[test]
fn collinear_puts_a_side_on_a_line() {
    let v = volume(
        "sketch s(on: xy) {
  h = line((0, 10), (5, 10), construction: true)
  o = poly([(0, 0), (20, 0), near(25, 9), (0, 10)])
  collinear(h, o[2])
  vertical(o[1])
}
slab = extrude(s.o, 1)
",
    );
    assert!((v - 200.0).abs() < 1e-6, "{v}");
}

#[test]
fn symmetric_mirrors_points_lines_and_circles_about_a_line() {
    let v = volume(
        "sketch s(on: xy) {
  ax = line((0, 0), (0, 10), construction: true)
  o = poly([(-5, 0), near(4, 1), (0, 6)])
  symmetric(o[0].a, o[0].b, ax)
}
tri = extrude(s.o, 1)
",
    );
    assert!((v - 30.0).abs() < 1e-6, "{v}");
    let v = volume(
        "sketch s(on: xy) {
  ax = line((0, -20), (0, 20), construction: true)
  a = circle((-10, 0), 3)
  b = circle(near(9, 1), 3)
  symmetric(a, b, ax)
  l = line((-6, -8), (-2, 8), construction: true)
  m = line(near(5, -7), near(1, 9), construction: true)
  symmetric(l, m, ax)
}
discs = extrude(s.b, 1)
",
    );
    assert!((v - std::f64::consts::PI * 9.0).abs() < 1e-6, "{v}");
}

#[test]
fn an_ellipse_is_a_profile_of_its_own() {
    let v = volume(
        "sketch s(on: xy) {
  e = ellipse(center: (0, 0), major: (10, 0), b: 4)
}
oval = extrude(s.e, 2)
",
    );
    assert!((v - std::f64::consts::PI * 10.0 * 4.0 * 2.0).abs() < 1e-6, "{v}");
}
