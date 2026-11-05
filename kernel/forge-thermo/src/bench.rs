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
    tweak: &dyn Fn(&mut Process),
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
    let mut pr = Process {
        law: law.law.clone(),
        pressure: 0.09,
        resolution: 80,
        surface,
        ..Process::default()
    };
    tweak(&mut pr);
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
        &|_| {},
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
        &|_| {},
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
        &|_| {},
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
        &|_| {},
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
            &|_| {},
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
            &|_| {},
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
        // measured, the sheet's middle on the tool half its thickness now off it along the normal:
        // cone +0.19, cylinder +0.73, box +1.41 (axis) +0.86 (diagonal) % t0 over the height
        // field's vertical quarter of t0; these parts' error is the slip and draw-in the model
        // leaves out (stage 2), not the contact's distance. Within 3 % t0
        assert!(d <= h + 0.03, "{name}: {d} {h}");
    }
}

/// Draw-in: the sheet on the mould's top sliding in against Coulomb friction (mu times the
/// vacuum). Measured against the parts, the mean error of t0 for each mu (stuck, 2, 1, 0.5, 0.25):
/// cone 1.20 3.27 1.58 1.18 1.58; cylinder 16.63 16.29 16.47 16.65 16.89; box axis 12.43 11.00
/// 12.62 12.09 11.48; diagonal 12.14 10.79 11.97 12.14 11.55. Slip on the top moves the Erdogan
/// error by at most 1.4 % t0, not under 8: the material they hold beyond the opening is not drawn
/// over the mould's top (the clamp frame's own draw-in and pre-forming sag are what's left).
#[test]
fn the_sheet_drawn_in_over_the_mould_top() {
    let cone = cavity(65.0, 55.0, 100.0, 3.0);
    let cyl = cavity(100.0, 100.0 - 1e-3, 100.0, 10.0);
    let bx = box_cavity(150.0, 75.0, 10.0);
    for (name, shape, t0, flange, temp, diagonal, upto) in [
        ("duarte2022_cone", &cone, 1.0, 50.0, 140.0, false, 150.0),
        ("erdogan2014_cylinder", &cyl, 2.5, 40.0, 180.0, false, 185.0),
        ("erdogan2014_box_axis", &bx, 2.0, 65.0, 180.0, false, 125.0),
        ("erdogan2014_box_diagonal", &bx, 2.0, 65.0, 180.0, true, 145.0),
    ] {
        let mut row = vec![];
        for mu in [None, Some(2.0), Some(1.0), Some(0.5), Some(0.25)] {
            let e = compare(
                name,
                Surface::Height,
                &|p: &mut Process| {
                    p.slide = mu;
                    p.clamp = None;
                    p.wrinkle = false;
                },
                shape,
                t0,
                flange,
                "HIPS",
                temp,
                diagonal,
                upto,
            );
            row.push(format!("{mu:?} {:.2}", e * 100.0));
        }
        eprintln!("DRAW {name}: {}", row.join(" | "));
        assert_eq!(row.len(), 5);
    }
}

/// The clamp draw-in: the blank's outer band under the clamp frame slides in against the frame's
/// grip (a shear stress, MPa), the sheet on the mould's top against Coulomb friction (mu times the
/// vacuum), the membrane wrinkling rather than taking compression. The mean error of t0 for each.
#[test]
fn the_sheet_drawn_in_from_under_the_clamp() {
    let cyl = cavity(100.0, 100.0 - 1e-3, 100.0, 10.0);
    let bx = box_cavity(150.0, 75.0, 10.0);
    let grip = |hold: f64| Some(Clamp { band: 20.0, hold });
    // (mu on the mould's top, the clamp's grip, wrinkling)
    let ways: [(Option<f64>, Option<Clamp>, bool); 6] = [
        (None, None, false),
        (None, None, true),
        (Some(0.3), grip(0.3), true),
        (Some(0.3), grip(0.03), true),
        (Some(0.1), grip(0.01), true),
        (Some(0.0), grip(0.0), true),
    ];
    for (name, shape, t0, flange, temp, diagonal, upto) in [
        ("erdogan2014_cylinder", &cyl, 2.5, 40.0, 180.0, false, 185.0),
        ("karabeyoglu2017_cylinder", &cyl, 2.0, 40.0, 150.0, false, 185.0),
        ("erdogan2014_box_axis", &bx, 2.0, 65.0, 180.0, false, 125.0),
        ("erdogan2014_box_diagonal", &bx, 2.0, 65.0, 180.0, true, 145.0),
    ] {
        let row: Vec<String> = ways
            .iter()
            .map(|&(mu, clamp, wrinkle)| {
                let e = compare(
                    name,
                    Surface::Height,
                    &|p: &mut Process| {
                        p.slide = mu;
                        p.clamp = clamp;
                        p.wrinkle = wrinkle;
                    },
                    shape,
                    t0,
                    flange,
                    "HIPS",
                    temp,
                    diagonal,
                    upto,
                );
                format!("{:.2}", e * 100.0)
            })
            .collect();
        eprintln!("CLAMP {name}: {}", row.join(" | "));
        assert_eq!(row.len(), ways.len());
    }
}

/// A rate-independent membrane's wall does not depend on how stiff the sheet is, once its load
/// steps are fine enough: the pressure only scales the stress. So a time-temperature shift of the
/// modulus (WLF, Arrhenius) cannot move the forming; only the law's shape, or rate and time, can.
/// The Erdogan cylinder with the HIPS law at a third, once and three times its stiffness: at 10
/// steps the third is 4 % t0 off (12.2 against 16.6, the steps' path), at 160 all three agree.
#[test]
fn the_sheets_stiffness_does_not_move_a_converged_forming() {
    let shape = cavity(100.0, 100.0 - 1e-3, 100.0, 10.0);
    let run = |scale: f64, steps: usize| {
        compare(
            "erdogan2014_cylinder",
            Surface::Height,
            &|p: &mut Process| {
                p.law.terms.iter_mut().for_each(|t| t.0 *= scale);
                p.steps = steps;
            },
            &shape,
            2.5,
            40.0,
            "HIPS",
            180.0,
            false,
            185.0,
        )
    };
    let fine: Vec<f64> = [0.3, 1.0, 3.0].iter().map(|&k| run(k, 160)).collect();
    eprintln!("STIFF 160 steps: {fine:?}");
    let spread = fine.iter().copied().fold(f64::NEG_INFINITY, f64::max)
        - fine.iter().copied().fold(f64::INFINITY, f64::min);
    assert!(
        spread < 0.005,
        "stiffness moves the converged forming by {spread} of t0"
    );
}

/// A long female V groove, its walls at 90 degrees, `d` from its bottom to each edge along the
/// wall, `len` long.
fn v_groove(d: f64, len: f64) -> Body {
    let h = d / 2f64.sqrt();
    let pts = [Point2::new(-h, 0.0), Point2::new(0.0, -h), Point2::new(h, 0.0)];
    let segs = (0..3)
        .map(|k| {
            (
                Segment::Line {
                    a: pts[k],
                    b: pts[(k + 1) % 3],
                },
                k as u64 + 1,
            )
        })
        .collect();
    let plane = Frame::from_axes(Point3::new(-len / 2.0, 0.0, 0.0), Vec3::Y, Vec3::Z, Vec3::X).unwrap();
    let p = Profile {
        plane,
        loops: vec![ProfileLoop::new(segs)],
    };
    forge_ops::extrude(&p, len, 5).expect("extrudes")
}

/// The membrane filling a 90 degree corner under sticking contact, against the closed form
/// (validation/s1_ref_corner_fill.csv, its spec): an incompressible neo-Hookean strip pushed into
/// the corner deposits t/t0 = (x/d)^k / lambda_c on the walls, x from the corner, k = 4/pi - 1,
/// lambda_c = 1.1107, whatever the pressure it ends at. Here a long groove (plane strain in its
/// middle), the sheet stuck on the mould's top at the groove's edges. Mean and worst error of
/// t/t0 on the walls (three nodes from the corner, two from the clamp's edge), printed and bounded.
/// Measured, mean / worst of t/t0: 80 nodes across the window and 10 steps 0.33 / 1.04 %; 160 and
/// 10, 0.44 / 1.64 %; 160 and 100, 0.35 / 1.39 %; R/d 0.018 (the pressure raised till the corner
/// is full).
#[test]
fn the_membrane_fills_a_corner_as_the_closed_form_says() {
    let d = 20.0;
    let shape = v_groove(d, 120.0);
    let spec = Spec {
        tool: Tool::Female,
        pull: UnitVec3::Z,
        t: 1.0,
        trim: None,
        flange: 5.0,
        shrink: 0.0,
        base: 10.0,
        margin: 5.0,
        vent: None,
        vents: None,
        vent_pitch: 25.0,
        sheet: None,
    };
    let f = analyse(&shape, &spec).expect("analyses");
    let (k, lc) = (4.0 / std::f64::consts::PI - 1.0, 1.110_720_7);
    let mut radius = vec![];
    for (res, steps, bending) in [
        (80, 10, false),
        (160, 10, false),
        (160, 100, false),
        (160, 10, true),
    ] {
        let pr = Process {
            law: Law::neo_hookean(1.0),
            pressure: 0.05,
            resolution: res,
            steps,
            bending,
            ..Process::default()
        };
        let st = stretch(&f, &spec, &pr);
        let (nx, ny) = st.grid;
        let id = |i: usize, j: usize| j * (nx + 1) + i;
        let h = d / 2f64.sqrt();
        let local = |p: Point3| f.frame.point_to_local(p);
        // the middle row across the groove: each wall point's distance from the corner
        let row: Vec<(f64, f64, bool)> = (0..=ny)
            .map(|j| id(nx / 2, j))
            .map(|n| {
                let p = local(st.nodes[n]);
                let x = (p.y * p.y + (p.z + h) * (p.z + h)).sqrt();
                (x, st.node_ratio[n], st.touched[n])
            })
            .filter(|w| w.0 < d - 1e-6)
            .collect();
        let r = row
            .iter()
            .filter(|w| w.2)
            .map(|w| w.0)
            .fold(f64::INFINITY, f64::min);
        // the grid's spacing (on the flat flange, unmoved): a node's wall is its triangles' by
        // area, so the three nodes at the corner and the two at the clamp's edge read the
        // elements either side
        let h = (local(st.nodes[id(nx / 2, 1)]) - local(st.nodes[id(nx / 2, 0)])).length();
        let errs: Vec<f64> = row
            .iter()
            .filter(|w| w.2 && w.0 >= (2.0 * r).max(3.0 * h) && w.0 <= d - 2.0 * h)
            .map(|w| (w.1 - forge_math::dmath::powf(w.0 / d, k) / lc).abs())
            .collect();
        let mean = errs.iter().sum::<f64>() / errs.len().max(1) as f64;
        let worst = errs.iter().copied().fold(0.0, f64::max);
        eprintln!(
            "CORNER res {res} steps {steps} bending {bending}: R/d {:.3}, {} wall points, t/t0 error mean {:.4} worst {:.4}",
            r / d,
            errs.len(),
            mean,
            worst
        );
        radius.push(r);
        if bending {
            // a plate cannot follow the corner as tight as a membrane: its radius is larger, and
            // the wall it lays down away from the corner is the membrane's
            assert!(
                r > radius[1],
                "bending radius {r} vs the membrane's {}",
                radius[1]
            );
            assert!(mean < 0.02, "mean {mean}");
            continue;
        }
        assert!(errs.len() >= 8);
        // the closed form's acceptance is 1 %: the mean holds it, the worst (next to the corner)
        // within 2 %
        assert!(mean < 0.01 && worst < 0.02, "mean {mean} worst {worst}");
    }
}

/// Bending (a plate D = G t^3 / 3 at the sheet's first thickness) against the membrane on the
/// measured parts, mean error of t0 each.
#[test]
fn the_sheet_bends_as_a_plate_on_the_measured_parts() {
    let cone = cavity(65.0, 55.0, 100.0, 3.0);
    let cyl = cavity(100.0, 100.0 - 1e-3, 100.0, 10.0);
    let bx = box_cavity(150.0, 75.0, 10.0);
    for (name, shape, t0, flange, temp, diagonal, upto) in [
        ("duarte2022_cone", &cone, 1.0, 50.0, 140.0, false, 150.0),
        ("erdogan2014_cylinder", &cyl, 2.5, 40.0, 180.0, false, 185.0),
        ("karabeyoglu2017_cylinder", &cyl, 2.0, 40.0, 150.0, false, 185.0),
        ("erdogan2014_box_axis", &bx, 2.0, 65.0, 180.0, false, 125.0),
        ("erdogan2014_box_diagonal", &bx, 2.0, 65.0, 180.0, true, 145.0),
    ] {
        let row: Vec<String> = [false, true]
            .iter()
            .map(|&b| {
                let e = compare(
                    name,
                    Surface::Height,
                    &|p: &mut Process| p.bending = b,
                    shape,
                    t0,
                    flange,
                    "HIPS",
                    temp,
                    diagonal,
                    upto,
                );
                format!("{:.2}", e * 100.0)
            })
            .collect();
        eprintln!("BEND {name}: membrane {} | plate {}", row[0], row[1]);
    }
}

/// The plug-only test (Atmani 2020, Fig. 13): a 1.1 mm HIPS sheet clamped on a 90 mm round window
/// (here the rim of a deep round cavity, the sheet stuck on its top), an R90-nosed d42 plug driven
/// in from 20 mm above, no pressure; dry (the sheet sticks to the plug) and lubricated (it slides).
/// The stroke is not printed: the one whose section is as long as the measured one is used. The
/// measured section, clamp to clamp, is compared through its middle, each side's arc length scaled
/// to the model's. Mean error of t0 for each.
#[test]
fn a_plug_drives_the_sheet_as_measured() {
    let shape = cavity(45.0, 45.0 - 1e-3, 150.0, 5.0);
    let spec = Spec {
        tool: Tool::Female,
        pull: UnitVec3::Z,
        t: 1.1,
        trim: None,
        flange: 45.0,
        shrink: 0.0,
        base: 10.0,
        margin: 10.0,
        vent: None,
        vents: None,
        vent_pitch: 25.0,
        sheet: None,
    };
    let f = analyse(&shape, &spec).expect("analyses");
    let law = placeholder().material("HIPS").cloned().unwrap().law_at(130.0);
    for (name, slip) in [
        ("atmani2020_plugonly_dry130", false),
        ("atmani2020_plugonly_lub130", true),
    ] {
        let data = measured(name);
        let total = data.last().map_or(0.0, |d| d.0);
        for stroke in [45.0, 55.0, 65.0] {
            let pr = Process {
                law: law.law.clone(),
                pressure: 0.0,
                resolution: 100,
                plug: Some(Plug {
                    at: Some((0.0, 0.0)),
                    radius: 21.0,
                    nose: 90.0,
                    edge: 10.0,
                    start: 20.0,
                    stroke,
                    steps: 40,
                    slip,
                }),
                ..Process::default()
            };
            let st = stretch(&f, &spec, &pr);
            let model = along(&st, spec.t, false);
            // the model's arc to the clamp: where the row comes back up to the trim plane
            let (nx, ny) = st.grid;
            let row = (0..=nx - nx / 2).map(|k| st.nodes[(ny / 2) * (nx + 1) + nx / 2 + k]);
            let half = row
                .zip(&model)
                .find(|(p, _)| f.frame.point_to_local(*p).z > -0.05)
                .map_or(0.0, |(_, m)| m.0);
            let mut err = 0.0;
            for &(s, t) in &data {
                let c = (s - total / 2.0).abs() / (total / 2.0) * half;
                err += (at(&model, c) - t).abs();
            }
            let mae = err / data.len() as f64 / spec.t;
            eprintln!(
                "PLUG {name} stroke {stroke}: section {:.1} mm (measured {total}), under the plug {:.3} mm (measured {:.2}), mean |error| {:.1} % of t0",
                2.0 * half,
                at(&model, 0.0),
                at(&data, total / 2.0),
                mae * 100.0
            );
        }
    }
}

/// A male dome: a hemisphere of radius `r` standing on the trim plane.
fn dome(r: f64) -> Body {
    let segs = vec![
        (
            Segment::Line {
                a: Point2::new(0.0, 0.0),
                b: Point2::new(r, 0.0),
            },
            1,
        ),
        (
            Segment::Arc {
                center: Point2::new(0.0, 0.0),
                radius: r,
                start: 0.0,
                sweep: forge_math::dmath::FRAC_PI_2,
            },
            2,
        ),
        (
            Segment::Line {
                a: Point2::new(0.0, r),
                b: Point2::new(0.0, 0.0),
            },
            3,
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

/// Stage 3's first test: Varedi 2024's ABS hemisphere (3 mm, male R125, 77.7 kPa), the sheet
/// 140.2 C at its centre and down to 127 C at its edge when it met the mould (Fig. 6b, the two
/// halves' mean), each triangle's stiffness the ABS law's (Erchiqui, 150 C) by ABS's WLF shift
/// (Cha 2019: 170 C, 5.8, 120.8 K) to a power: 0 is one temperature, 1 the flow regime. The wall
/// against the horizontal distance from the pole, mean error of t0.
#[test]
fn an_unevenly_heated_sheet_forms_as_measured() {
    let shape = dome(125.0);
    let spec = Spec {
        tool: Tool::Male,
        pull: UnitVec3::Z,
        t: 2.91,
        trim: None,
        flange: 50.0,
        shrink: 0.0,
        base: 10.0,
        margin: 40.0,
        vent: None,
        vents: None,
        vent_pitch: 25.0,
        sheet: None,
    };
    let f = analyse(&shape, &spec).expect("analyses");
    let law = placeholder().material("ABS").cloned().unwrap().law_at(140.0);
    let data = measured("varedi2024_hemisphere");
    let profile = vec![
        (0.0, 140.0),
        (15.0, 139.4),
        (45.0, 137.9),
        (75.0, 135.9),
        (105.0, 133.1),
        (135.0, 130.8),
        (165.0, 127.3),
    ];
    let mut row = vec![];
    for power in [0.0, 0.25, 0.5, 1.0] {
        let pr = Process {
            law: law.law.clone(),
            pressure: 0.0777,
            resolution: 100,
            heat: (power > 0.0).then(|| Heat {
                at: Some((0.0, 0.0)),
                profile: profile.clone(),
                shift: Shift::Wlf {
                    tref: 170.0,
                    c1: 5.8,
                    c2: 120.8,
                },
                temp: 140.0,
                power,
                edge: None,
            }),
            ..Process::default()
        };
        let st = stretch(&f, &spec, &pr);
        // the wall along the grid's middle row by horizontal distance from the pole
        let (nx, ny) = st.grid;
        let model: Vec<(f64, f64)> = (0..=nx - nx / 2)
            .map(|k| (ny / 2) * (nx + 1) + nx / 2 + k)
            .map(|n| {
                let p = f.frame.point_to_local(st.nodes[n]);
                ((p.x * p.x + p.y * p.y).sqrt(), st.node_ratio[n] * spec.t)
            })
            .collect();
        let err =
            data.iter().map(|&(y, t)| (at(&model, y) - t).abs()).sum::<f64>() / data.len() as f64 / spec.t;
        eprintln!(
            "HEAT power {power}: pole {:.2} (measured 2.80), at 120 mm {:.2} (1.95), mean |error| {:.1} % of t0",
            at(&model, 0.0),
            at(&model, 120.0),
            err * 100.0
        );
        if power == 0.0 {
            // the surface's stretch (stage 4): Varedi's DIC, from first contact, 5.6 % at the pole
            // and 72.8 % at the base's edge (the model's from the flat sheet, its drape included)
            let sv = crate::surface::stretches(&st, &f.frame);
            let near = |r0: f64, r1: f64| {
                st.tris
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| {
                        let c = t.iter().fold(Vec3::ZERO, |s, &i| {
                            s + (f.frame.point_to_local(st.nodes[i as usize]) - Point3::ORIGIN)
                        }) * (1.0 / 3.0);
                        let r = (c.x * c.x + c.y * c.y).sqrt();
                        r >= r0 && r < r1 && t.iter().all(|&i| st.touched[i as usize])
                    })
                    .map(|(k, _)| sv.l1[k] - 1.0)
                    .fold(0.0, f64::max)
            };
            let (pole, edge) = (near(0.0, 10.0), near(100.0, 125.0));
            eprintln!(
                "SURFACE strain, max principal: pole {:.1} % (measured 5.6), base edge {:.1} % (72.8)",
                pole * 100.0,
                edge * 100.0
            );
            assert!(edge > pole);
        }
        let line: Vec<String> = data
            .iter()
            .map(|&(y, t)| format!("{y:.0}:{t:.2}/{:.2}", at(&model, y)))
            .collect();
        eprintln!("  {}", line.join(" "));
        row.push(err);
    }
    assert_eq!(row.len(), 4);
}

/// A male block standing on the trim plane, `a` square, `h` tall, its vertical edges rounded `rf`.
fn male_block(a: f64, h: f64, rf: f64) -> Body {
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
    forge_ops::extrude(&p, h, 5).unwrap()
}

/// Webbing (stage 2g): a tall male block webs at its corners (Formech's rule: a male part over
/// about as tall as it is wide), a low one does not. Squeezed area and folds for each: 15 mm
/// tall, 72 triangles squeezed (144 mm2), no fold; 90 mm, 187 (374 mm2), 94 folds.
#[test]
fn a_tall_male_block_webs_and_a_low_one_does_not() {
    let mut found = vec![];
    for h in [15.0, 90.0] {
        let shape = male_block(80.0, h, 5.0);
        let spec = Spec {
            tool: Tool::Male,
            pull: UnitVec3::Z,
            t: 1.5,
            trim: None,
            flange: 30.0,
            shrink: 0.0,
            base: 10.0,
            margin: 10.0,
            vent: None,
            vents: None,
            vent_pitch: 25.0,
            sheet: None,
        };
        let f = analyse(&shape, &spec).expect("analyses");
        let law = placeholder().material("HIPS").cloned().unwrap().law_at(160.0);
        let pr = Process {
            law: law.law,
            resolution: 80,
            ..Process::default()
        };
        let st = stretch(&f, &spec, &pr);
        let w = crate::surface::webbing(&st, &f.frame, spec.t, 0.05);
        eprintln!(
            "WEB block {h} mm tall: {} squeezed triangles, {:.0} mm2 of the flat sheet, {} folds",
            w.squeezed.len(),
            w.squeezed_area,
            w.folds.len()
        );
        found.push(w);
    }
    assert!(found[1].squeezed_area > found[0].squeezed_area);
    assert!(found[0].folds.is_empty() && !found[1].folds.is_empty());
}
