//! The thickness model against parts formed and measured in published work (data/benchmarks):
//! each shape built as described, its sheet formed, the thickness read along the measured cut, and
//! the error printed and bounded. Run with --nocapture to see the tables.
use crate::*;
use forge_math::{Frame, Point2, Point3, UnitVec3, Vec2, Vec3};
use forge_ops::{Axis2, Profile, ProfileLoop, Segment};
use forge_topo::Body;

fn measured(name: &str) -> Vec<(f64, f64)> {
    let path = format!("{}/data/benchmarks/{name}.csv", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path)
        .expect("benchmark file")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with('s'))
        .map(|l| {
            let (a, b) = l.split_once(',').expect("two columns");
            (a.trim().parse().unwrap(), b.trim().parse().unwrap())
        })
        .collect()
}

/// A female cavity revolved from its half-section (r, z), z down from 0 at the rim: the opening
/// radius `top`, the floor's `bottom`, `depth`, the floor's edge rounded to `rf`.
fn cavity(top: f64, bottom: f64, depth: f64, rf: f64) -> Body {
    // the wall's direction down, and where the round meets it and the floor
    let w = Vec2::new(bottom - top, -depth);
    let wl = (w.x * w.x + w.y * w.y).sqrt();
    let (ux, uy) = (w.x / wl, w.y / wl);
    // the angle between the floor and the wall at the corner: the round's tangent points sit
    // rf * cot(half of it) back from the corner
    let turn = forge_math::dmath::acos(ux.clamp(-1.0, 1.0)).max(1e-9);
    let back = rf * forge_math::dmath::cos(turn / 2.0) / forge_math::dmath::sin(turn / 2.0);
    let corner = Point2::new(bottom, -depth);
    let on_wall = Point2::new(corner.x - ux * back, corner.y - uy * back);
    let on_floor = Point2::new(corner.x - back, corner.y);
    let centre = Point2::new(on_floor.x, on_floor.y + rf);
    let a0 = forge_math::dmath::atan2(on_wall.y - centre.y, on_wall.x - centre.x);
    let segs = vec![
        (
            Segment::Line {
                a: Point2::new(0.0, 0.0),
                b: Point2::new(0.0, -depth),
            },
            1,
        ),
        (
            Segment::Line {
                a: Point2::new(0.0, -depth),
                b: on_floor,
            },
            2,
        ),
        (
            Segment::Arc {
                center: centre,
                radius: rf,
                start: -forge_math::dmath::FRAC_PI_2,
                sweep: a0 + forge_math::dmath::FRAC_PI_2,
            },
            3,
        ),
        (
            Segment::Line {
                a: on_wall,
                b: Point2::new(top, 0.0),
            },
            4,
        ),
        (
            Segment::Line {
                a: Point2::new(top, 0.0),
                b: Point2::new(0.0, 0.0),
            },
            5,
        ),
    ];
    let plane = Frame::from_axes(Point3::ORIGIN, Vec3::X, Vec3::Z, -Vec3::Y).unwrap();
    let p = Profile {
        plane,
        loops: vec![ProfileLoop::new(segs)],
    };
    let axis = Axis2 {
        origin: Point2::new(0.0, 0.0),
        dir: Vec2::new(0.0, 1.0),
    };
    forge_ops::revolve(&p, axis, forge_math::dmath::TAU, 5).expect("revolves")
}

/// A female box cavity: a rounded square `a` across, `depth` deep, its floor edges rounded `rf`.
fn box_cavity(a: f64, depth: f64, rf: f64) -> Body {
    let p = Profile {
        plane: Frame::WORLD,
        loops: vec![Profile::rounded_rectangle(
            Point2::new(-a / 2.0, -a / 2.0),
            a,
            a,
            rf,
            0,
        )],
    };
    let b = forge_ops::extrude(&p, -depth, 5).unwrap();
    let floor: Vec<forge_ops::FilletEdge> = b
        .edge_ids()
        .filter(|&e| {
            let q = b.edge_points(e, 1);
            q.iter().all(|p| (p.z + depth).abs() < 1e-6)
        })
        .map(|edge| forge_ops::FilletEdge {
            edge,
            radius: forge_ops::Radius::Constant(rf),
        })
        .collect();
    forge_ops::fillet(&b, &floor, 9).expect("rounds")
}

/// The thickness along a row of the sheet's grid from its middle node outward (`diagonal`: along
/// the grid's diagonal), as (arc length along the formed sheet, thickness).
fn along(st: &Stretch, t0: f64, diagonal: bool) -> Vec<(f64, f64)> {
    let (nx, ny) = st.grid;
    let (ci, cj) = (nx / 2, ny / 2);
    let id = |i: usize, j: usize| j * (nx + 1) + i;
    let mut out = vec![];
    let mut s = 0.0;
    let mut prev: Option<Point3> = None;
    for k in 0..=(nx - ci).min(ny - cj) * usize::from(diagonal) + (nx - ci) * usize::from(!diagonal) {
        let n = if diagonal {
            id(ci + k, cj + k)
        } else {
            id(ci + k, cj)
        };
        let p = st.nodes[n];
        if let Some(q) = prev {
            s += (p - q).length();
        }
        prev = Some(p);
        out.push((s, st.node_ratio[n] * t0));
    }
    out
}

fn at(curve: &[(f64, f64)], s: f64) -> f64 {
    for w in curve.windows(2) {
        if s >= w[0].0 && s <= w[1].0 {
            return w[0].1 + (s - w[0].0) / (w[1].0 - w[0].0).max(1e-12) * (w[1].1 - w[0].1);
        }
    }
    curve.last().map_or(f64::NAN, |c| c.1)
}

/// Builds and forms the part, compares along the cut up to `upto` (the rim), and returns
/// (mean absolute error over t0, the measured and predicted minima and where).
#[allow(clippy::too_many_arguments)]
fn compare(
    name: &str,
    surface: Surface,
    shape: &Body,
    t0: f64,
    flange: f64,
    mat: &str,
    temp: f64,
    diagonal: bool,
    upto: f64,
) -> f64 {
    let spec = Spec {
        tool: Tool::Female,
        pull: UnitVec3::Z,
        t: t0,
        trim: None,
        flange,
        shrink: 0.0,
        base: 10.0,
        margin: 10.0,
        vent: None,
        vents: None,
        vent_pitch: 25.0,
        sheet: None,
    };
    let f = analyse(shape, &spec).expect("analyses");
    let m = placeholder().material(mat).cloned().unwrap();
    let law = m.law_at(temp);
    let pr = Process {
        law: law.law.clone(),
        pressure: 0.09,
        resolution: 80,
        surface,
    };
    let st = stretch(&f, &spec, &pr);
    let model = along(&st, t0, diagonal);
    let data = measured(name);
    let mut err = 0.0;
    let mut n = 0;
    eprintln!(
        "\n{name} ({surface:?}): {mat} law {:?} ({})",
        law.law.terms, law.source
    );
    eprintln!("  s     measured  model");
    let (mut mmin, mut pmin) = ((f64::INFINITY, 0.0), (f64::INFINITY, 0.0));
    for &(s, t) in data.iter().filter(|d| d.0 <= upto) {
        let p = at(&model, s);
        eprintln!("  {s:5.0} {t:8.3} {p:8.3}");
        err += (p - t).abs();
        n += 1;
        if t < mmin.0 {
            mmin = (t, s);
        }
        if p < pmin.0 {
            pmin = (p, s);
        }
    }
    let mae = err / n as f64 / t0;
    eprintln!(
        "  mean |error| {:.1}% of t0; measured min {:.3} at s {}, model {:.3} at s {}; model's own min {:.3}",
        mae * 100.0,
        mmin.0,
        mmin.1,
        at(&model, mmin.1),
        pmin.1,
        st.min.0 * t0
    );
    mae
}

#[test]
fn erdogan_cylinder() {
    let shape = cavity(100.0, 100.0 - 1e-3, 100.0, 10.0);
    let e = compare(
        "erdogan2014_cylinder",
        Surface::Height,
        &shape,
        2.5,
        40.0,
        "HIPS",
        180.0,
        false,
        185.0,
    );
    assert!(e < 0.2, "{e}");
}

#[test]
fn erdogan_box() {
    let shape = box_cavity(150.0, 75.0, 10.0);
    let a = compare(
        "erdogan2014_box_axis",
        Surface::Height,
        &shape,
        2.0,
        65.0,
        "HIPS",
        180.0,
        false,
        125.0,
    );
    let d = compare(
        "erdogan2014_box_diagonal",
        Surface::Height,
        &shape,
        2.0,
        65.0,
        "HIPS",
        180.0,
        true,
        145.0,
    );
    assert!(a < 0.15 && d < 0.15, "{a} {d}");
}

#[test]
fn duarte_cone() {
    let shape = cavity(65.0, 55.0, 100.0, 3.0);
    let e = compare(
        "duarte2022_cone",
        Surface::Height,
        &shape,
        1.0,
        50.0,
        "HIPS",
        140.0,
        false,
        150.0,
    );
    assert!(e < 0.03, "{e}");
}

/// The 3-D distance field against the height field on the measured parts: the same parts, the
/// gap now along the surface's normal.
#[test]
fn the_distance_field_forms_the_measured_parts_as_well() {
    let cone = cavity(65.0, 55.0, 100.0, 3.0);
    let cyl = cavity(100.0, 100.0 - 1e-3, 100.0, 10.0);
    let bx = box_cavity(150.0, 75.0, 10.0);
    for (name, shape, t0, flange, temp, diagonal, upto) in [
        ("duarte2022_cone", &cone, 1.0, 50.0, 140.0, false, 150.0),
        ("erdogan2014_cylinder", &cyl, 2.5, 40.0, 180.0, false, 185.0),
        ("erdogan2014_box_axis", &bx, 2.0, 65.0, 180.0, false, 125.0),
        ("erdogan2014_box_diagonal", &bx, 2.0, 65.0, 180.0, true, 145.0),
    ] {
        let h = compare(
            name,
            Surface::Height,
            shape,
            t0,
            flange,
            "HIPS",
            temp,
            diagonal,
            upto,
        );
        let d = compare(
            name,
            Surface::Distance,
            shape,
            t0,
            flange,
            "HIPS",
            temp,
            diagonal,
            upto,
        );
        eprintln!(
            "{name}: height {:.2} % t0, distance {:.2} % t0",
            h * 100.0,
            d * 100.0
        );
        // measured: cone +0.24, cylinder +0.17, box +2.4 (axis) % t0. On the box's steep walls the
        // height field's vertical reach was a normal one of reach cos(wall): the sheet stuck later
        // there; the field sticks it a quarter of t off the wall itself. Within 3 % t0
        assert!(d <= h + 0.03, "{name}: {d} {h}");
    }
}
