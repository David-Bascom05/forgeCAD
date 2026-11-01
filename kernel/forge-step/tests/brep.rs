//! OpenCascade's `.brep` text, written out by hand: placed copies, reversed shapes, a cone with a
//! negative angle, and every way of cutting a file short.
use forge_math::dmath;
use forge_step::{Imported, Limits, import};
use forge_topo::{check, mass};

fn read(text: &str) -> Imported {
    import(text.as_bytes(), &Limits::default()).unwrap_or_else(|e| panic!("{e}"))
}

/// A closed solid of revolution about z, from z = 0 to z = h: a cylinder or cone wall with a seam,
/// its two rims and two planes. `surface` is the wall's line in the Surfaces section, `r1` and
/// `r2` the rims' radii, `s` the seam's length; `root` the last lines.
fn turned(locations: &str, surface: &str, (r1, r2): (f64, f64), h: f64, s: f64, shapes: &str) -> String {
    format!(
        "DBRep_DrawableShape

CASCADE Topology V1, (c) Matra-Datavision
{locations}
Curve2ds 0
Curves 3
1 {r1} 0 0 {dx} 0 {dz}
2 0 0 0 0 0 1 1 0 0 0 1 0 {r1}
2 0 0 {h} 0 0 1 1 0 0 0 1 0 {r2}
Polygon3D 0
PolygonOnTriangulations 0
Surfaces 3
{surface}
1 0 0 0 0 0 -1 0 1 0 1 0 0
1 0 0 {h} 0 0 1 1 0 0 0 1 0
Triangulations 0

TShapes 13
Ve
1e-07
{r1} 0 0
0 0

0101101
*
Ve
1e-07
{r2} 0 {h}
0 0

0101101
*
Ed
 1e-07 1 1 0
1  1 0 0 {s}
0

0101000
+13 0 -12 0 *
Ed
 1e-07 1 1 0
1  2 0 0 6.283185307179586
0

0101100
+13 0 -13 0 *
Ed
 1e-07 1 1 0
1  3 0 0 6.283185307179586
0

0101100
+12 0 -12 0 *
Wi

0101000
+11 0 -9 0 -11 0 +10 0 *
Fa
0  1e-07 1 0

0101000
+8 0 *
Wi

0101100
-10 0 *
Fa
0  1e-07 2 0

0101000
+6 0 *
Wi

0101100
+9 0 *
Fa
0  1e-07 3 0

0101000
+4 0 *
Sh

0101100
+7 0 +5 0 +3 0 *
So

1100100
+2 0 *
{shapes}",
        dx = (r2 - r1) / s,
        dz = h / s,
    )
}

const NO_LOCATIONS: &str = "Locations 0";

fn cylinder(r: f64, h: f64, shapes: &str, locations: &str) -> String {
    turned(
        locations,
        &format!("2 0 0 0 0 0 1 1 0 0 0 1 0 {r}"),
        (r, r),
        h,
        h,
        shapes,
    )
}

#[test]
fn a_cylinder_with_its_seam() {
    let (r, h) = (5.0, 7.0);
    let got = read(&cylinder(r, h, "\n+1 0", NO_LOCATIONS));
    assert!(got.refusals.is_empty(), "{:?}", got.refusals);
    assert_eq!(got.solids.len(), 1);
    let b = &got.solids[0].body;
    assert_eq!(check(b), vec![]);
    let m = mass::mass_rel(b, 1e-9).unwrap();
    let pi = dmath::PI;
    assert!((m.volume - pi * r * r * h).abs() < 1e-6, "{}", m.volume);
    assert!(
        (m.area - (2.0 * pi * r * r + 2.0 * pi * r * h)).abs() < 1e-6,
        "{}",
        m.area
    );
}

#[test]
fn a_location_moves_a_solid_and_powers_compose() {
    let locs = "Locations 2
1
 0 -1 0 10
 1 0 0 0
 0 0 1 0
2  1 2 0";
    let once = read(&cylinder(5.0, 7.0, "\n+1 1", locs));
    let c = once.solids[0].body.faces_box().center();
    assert!((c.x - 10.0).abs() < 1e-6 && c.y.abs() < 1e-6, "{c:?}");
    // the same turn and move twice: (10, 0) rotates to (0, 10), then moves by 10 in x
    let twice = read(&cylinder(5.0, 7.0, "\n+1 2", locs));
    let c = twice.solids[0].body.faces_box().center();
    assert!((c.x - 10.0).abs() < 1e-6 && (c.y - 10.0).abs() < 1e-6, "{c:?}");
    assert_eq!(check(&twice.solids[0].body), vec![]);
}

#[test]
fn a_scale_is_refused_by_name() {
    let locs = "Locations 1
1
 2 0 0 0
 0 2 0 0
 0 0 2 0";
    let got = read(&cylinder(5.0, 7.0, "\n+1 1", locs));
    assert!(got.solids.is_empty());
    assert!(got.refusals[0].why.contains("scale"), "{:?}", got.refusals);
}

#[test]
fn a_cone_with_a_negative_angle_opens_down_its_axis() {
    // r = 6 at z = 0, narrowing to r = 2 at z = 4: 45 degrees, negative
    let (r1, r2, h) = (6.0, 2.0, 4.0);
    let half = -dmath::FRAC_PI_2 / 2.0;
    let s = 4.0 * 2f64.sqrt();
    let text = turned(
        NO_LOCATIONS,
        &format!("3 0 0 0 0 0 1 1 0 0 0 1 0 {r1} {half}"),
        (r1, r2),
        h,
        s,
        "\n+1 0",
    );
    let got = read(&text);
    assert!(got.refusals.is_empty(), "{:?}", got.refusals);
    let b = &got.solids[0].body;
    assert_eq!(check(b), vec![]);
    let m = mass::mass_rel(b, 1e-9).unwrap();
    let want = dmath::PI * h / 3.0 * (r1 * r1 + r1 * r2 + r2 * r2);
    assert!((m.volume - want).abs() < 1e-6, "{} {want}", m.volume);
}

/// Whatever is left of a file, it reads or is refused.
#[test]
fn cut_short_or_damaged_it_never_panics() {
    let text = cylinder(5.0, 7.0, "\n+1 0", NO_LOCATIONS);
    let at: Vec<usize> = text.char_indices().map(|x| x.0).collect();
    for &k in &at {
        let _ = import(&text.as_bytes()[..k], &Limits::default());
    }
    let words: Vec<&str> = text.split_ascii_whitespace().collect();
    for i in 0..words.len() {
        for bad in ["-1", "99999999999", "1e400", "x"] {
            let mut w = words.clone();
            w[i] = bad;
            let _ = import(w.join(" ").as_bytes(), &Limits::default());
        }
    }
}
