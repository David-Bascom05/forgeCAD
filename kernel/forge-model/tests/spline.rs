//! Real curves in profiles: extrude and revolve walls, booleans against them, mass against meshes.

use forge_geom::spline::PlaneSpline;
use forge_geom::spline::Spline;
use forge_math::{Frame, Point2, dmath};
use forge_mesh::{Quality, tessellate};
use forge_ops::{
    Axis2, Handed, PathSeg, Profile, ProfileLoop, Segment, Winding, coil, extrude, revolve, sweep,
};
use forge_topo::{Body, check};
use std::sync::Arc;

fn s(c: PlaneSpline) -> Segment {
    Segment::Spline(Arc::new(c))
}

fn prof(loops: Vec<ProfileLoop>) -> Profile {
    Profile {
        plane: Frame::WORLD,
        loops,
    }
}

fn dome() -> ProfileLoop {
    let p = |x, y| Point2::new(x, y);
    let c = PlaneSpline::through(
        vec![p(0.0, 0.0), p(10.0, 8.0), p(20.0, 3.0), p(30.0, 0.0)],
        None,
        None,
    )
    .unwrap();
    ProfileLoop::new(vec![
        (s(c), 0),
        (
            Segment::Line {
                a: p(30.0, 0.0),
                b: p(0.0, 0.0),
            },
            1,
        ),
    ])
}

fn sound(b: &Body) {
    let problems = check(b);
    assert!(problems.is_empty(), "{problems:?}");
}

fn mesh_volume(b: &Body, chord: f64) -> f64 {
    let m = tessellate(
        b,
        &Quality {
            chord,
            ..Quality::default()
        },
    )
    .expect("meshes");
    m.volume()
}

#[test]
fn extruded_dome_is_valid_and_has_its_area() {
    let p = prof(vec![dome()]);
    let b = extrude(&p, 5.0, 1).unwrap();
    sound(&b);
    assert_eq!(b.faces.iter().filter(|f| f.alive).count(), 4);
    let want = p.area() * 5.0;
    let m = forge_topo::mass::mass(&b).unwrap();
    assert!((m.volume - want).abs() < 1e-8 * want, "{} {want}", m.volume);
    let mv = mesh_volume(&b, 1e-3);
    assert!((mv - want).abs() < 1e-3 * want, "{mv} {want}");
}

#[test]
fn a_whole_ellipse_extrudes_into_two_walls() {
    let e = PlaneSpline::ellipse(Point2::new(1.0, 2.0), 7.0, 3.0, 0.4, 0.0, dmath::TAU).unwrap();
    let p = prof(vec![ProfileLoop::new(vec![(s(e), 0)])]);
    let b = extrude(&p, 4.0, 1).unwrap();
    sound(&b);
    let want = dmath::PI * 21.0 * 4.0;
    let m = forge_topo::mass::mass(&b).unwrap();
    assert!((m.volume - want).abs() < 1e-8 * want, "{}", m.volume);
    assert_eq!(b.faces.iter().filter(|f| f.alive).count(), 4);
}

#[test]
fn a_kink_splits_the_wall() {
    let p2 = |x, y| Point2::new(x, y);
    // a corner at the middle knot, repeated to the degree
    let c = PlaneSpline::control(
        3,
        vec![
            p2(0.0, 0.0),
            p2(3.0, 5.0),
            p2(6.0, 5.0),
            p2(9.0, 9.0),
            p2(12.0, 5.0),
            p2(15.0, 5.0),
            p2(18.0, 0.0),
        ],
        Some(vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 2.0, 2.0, 2.0, 2.0]),
        None,
    )
    .unwrap();
    assert_eq!(c.c.corners().len(), 1);
    let l = ProfileLoop::new(vec![
        (s(c), 0),
        (
            Segment::Line {
                a: p2(18.0, 0.0),
                b: p2(0.0, 0.0),
            },
            1,
        ),
    ]);
    let b = extrude(&prof(vec![l]), 3.0, 1).unwrap();
    sound(&b);
    assert_eq!(b.faces.iter().filter(|f| f.alive).count(), 5);
}

#[test]
fn revolved_curve_has_pappus_volume() {
    let p2 = |x, y| Point2::new(x, y);
    // off the axis, a closed loop round: volume is 2 pi times the first moment of area
    let c = PlaneSpline::through(
        vec![p2(5.0, 0.0), p2(8.0, 3.0), p2(6.0, 8.0), p2(5.0, 10.0)],
        None,
        None,
    )
    .unwrap();
    let l = ProfileLoop::new(vec![
        (s(c.clone()), 0),
        (
            Segment::Line {
                a: p2(5.0, 10.0),
                b: p2(5.0, 0.0),
            },
            1,
        ),
    ]);
    let axis = Axis2 {
        origin: Point2::ORIGIN,
        dir: forge_math::Vec2::Y,
    };
    let b = revolve(&prof(vec![l]), axis, dmath::TAU, 1).unwrap();
    sound(&b);
    // integral of x dA = 1/2 integral of x^2 dy round the loop: the line part is exact, the curve by Gauss
    let rule = forge_math::poly::gauss_legendre(32);
    let mut moment = 0.0;
    for k in 0..c.c.pieces() {
        let br = c.c.breaks();
        let (lo, h) = (br[k], br[k + 1] - br[k]);
        for &(x, w) in &rule {
            let q = c.c.deriv(lo + h * 0.5 * (x + 1.0));
            moment += 0.25 * h * w * q.p.x * q.p.x * q.d1.y;
        }
    }
    moment += 0.5 * 25.0 * -10.0;
    let want = dmath::TAU * moment;
    let m = forge_topo::mass::mass(&b).unwrap();
    assert!((m.volume - want).abs() < 1e-6 * want, "{} {want}", m.volume);
    let mv = mesh_volume(&b, 1e-3);
    assert!((mv - want).abs() < 2e-3 * want, "{mv} {want}");
}

/// Length of a curve, by Gauss.
fn length(c: &Spline) -> f64 {
    let rule = forge_math::poly::gauss_legendre(32);
    let br = c.breaks();
    (0..c.pieces())
        .map(|k| {
            let (lo, h) = (br[k], br[k + 1] - br[k]);
            rule.iter()
                .map(|&(x, w)| 0.5 * h * w * c.deriv(lo + 0.5 * h * (x + 1.0)).d1.length())
                .sum::<f64>()
        })
        .sum()
}

#[test]
fn along_a_curve_the_volume_is_section_area_by_length() {
    // Guldin: a section centred on a planar path, however it turns
    let k = forge_geom::spline::uniform_knots(3, 4);
    let pts: Vec<_> = [(0.0, 0.0), (5.0, 0.0), (20.0, 6.0), (30.0, 0.0)]
        .iter()
        .map(|&(x, y)| forge_math::Point3::new(x, y, 0.0))
        .collect();
    let path = Spline::from_nurbs(3, &k, &pts, None).unwrap();
    let plane = forge_model::adversarial::fr([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    let p = Profile {
        plane,
        loops: vec![Profile::rectangle(Point2::new(-1.0, -1.0), 2.0, 2.0, 0)],
    };
    let b = sweep(&p, &[PathSeg::Spline(Arc::new(path.clone()))], 1).unwrap();
    sound(&b);
    let want = 4.0 * length(&path);
    let m = forge_topo::mass::mass(&b).unwrap();
    assert!((m.volume - want).abs() < 1e-8 * want, "{} {want}", m.volume);
    let mv = mesh_volume(&b, 1e-3);
    assert!((mv - want).abs() < 2e-3 * want, "{mv} {want}");
}

#[test]
fn round_a_bend_and_in_a_coil_a_curved_section_keeps_pappus() {
    let p2 = |x, y| Point2::new(x, y);
    let c = PlaneSpline::through(vec![p2(8.0, 0.0), p2(10.0, 2.0), p2(12.0, 0.0)], None, None).unwrap();
    let area = -c.c.area_term() - 0.5 * (p2(12.0, 0.0).to_vec().cross(p2(8.0, 0.0).to_vec()));
    let l = ProfileLoop::new(vec![
        (s(c.clone()), 0),
        (
            Segment::Line {
                a: p2(12.0, 0.0),
                b: p2(8.0, 0.0),
            },
            1,
        ),
    ]);
    let plane = forge_model::adversarial::fr([0.0, 0.0, 0.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0]);
    let prof = Profile {
        plane,
        loops: vec![l],
    };
    // the section is symmetric about x = 10
    let want = |angle: f64| area.abs() * 10.0 * angle;
    let axis = Axis2 {
        origin: Point2::ORIGIN,
        dir: forge_math::Vec2::Y,
    };
    let w = Winding {
        pitch: 5.0,
        turns: 3.0,
        taper: 0.0,
        handed: Handed::Right,
    };
    let b = coil(&prof, axis, &w, 1).unwrap();
    sound(&b);
    let m = forge_topo::mass::mass(&b).unwrap();
    let v = want(3.0 * dmath::TAU);
    assert!((m.volume - v).abs() < 1e-7 * v, "{} {v}", m.volume);

    // a bend of 90 degrees about the y axis through the origin, the profile 10 out on x
    let circle = forge_geom::Circle {
        frame: forge_model::adversarial::fr([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
        radius: 10.0,
    };
    let at = forge_model::adversarial::fr([0.0, 0.0, 0.0], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0]);
    let bent = Profile {
        plane: at,
        loops: prof.loops.clone(),
    };
    let turn = PathSeg::Arc {
        circle,
        start: 0.0,
        sweep: dmath::FRAC_PI_2,
    };
    let b = sweep(&bent, &[turn], 1).unwrap();
    sound(&b);
    let m = forge_topo::mass::mass(&b).unwrap();
    let v = want(dmath::FRAC_PI_2);
    assert!((m.volume - v).abs() < 1e-7 * v, "{} {v}", m.volume);
}

/// A body's volume by mass, against its mesh.
fn checked(b: &Body) -> f64 {
    sound(b);
    let m = forge_topo::mass::mass(b).unwrap();
    let mv = mesh_volume(b, 1e-3);
    assert!(
        (m.volume - mv).abs() < 2e-3 * m.volume.abs(),
        "mass {} mesh {mv}",
        m.volume
    );
    m.volume
}

#[test]
fn a_loft_closes_on_a_point() {
    use forge_math::{Point3, Vec3};
    use forge_ops::{Apex, LoftGuides, LoftKind, LoftOptions, loft_and_bounds};
    let at = |x: f64, y: f64, z: f64| Apex {
        point: Point3::new(x, y, z),
        dome: None,
    };
    let circle = prof(vec![Profile::circle(Point2::ORIGIN, 5.0, 0)]);
    let square = prof(vec![Profile::rectangle(Point2::new(-5.0, -5.0), 10.0, 10.0, 0)]);
    let none = LoftGuides::None;
    let ruled = LoftKind::Ruled;
    let free = LoftKind::Smooth {
        start: None,
        end: None,
    };
    let with = |a: [Option<Apex>; 2]| LoftOptions {
        apex: a,
        ..LoftOptions::default()
    };
    let one = |p: &Profile, k, a: [Option<Apex>; 2]| {
        loft_and_bounds(std::slice::from_ref(p), k, &none, &with(a), 1)
            .unwrap()
            .0
    };
    // cone and oblique cone, pyramid and oblique pyramid: a third of base by height
    for (x, y) in [(0.0, 0.0), (3.0, 2.0)] {
        let v = checked(&one(&circle, ruled, [None, Some(at(x, y, 20.0))]));
        let want = dmath::PI * 25.0 * 20.0 / 3.0;
        assert!((v - want).abs() < 1e-8 * want, "{v} {want}");
        let v = checked(&one(&square, ruled, [Some(at(x, y, -20.0)), None]));
        let want = 100.0 * 20.0 / 3.0;
        assert!((v - want).abs() < 1e-8 * want, "{v} {want}");
    }
    // a smooth loft through a section on to a point
    let high = Profile {
        plane: Frame::WORLD.with_origin(Point3::new(0.0, 0.0, 6.0)),
        loops: vec![Profile::circle(Point2::ORIGIN, 4.0, 0)],
    };
    let b = loft_and_bounds(
        &[circle.clone(), high],
        free,
        &none,
        &with([None, Some(at(0.0, 0.0, 12.0))]),
        1,
    )
    .unwrap()
    .0;
    assert!(checked(&b) > 0.0);
    // ...with the point off the axis: the walls lean, and mass still walks their loops through it
    let low = Profile {
        plane: Frame::WORLD.with_origin(Point3::new(0.0, 0.0, 5.5714)),
        loops: vec![Profile::circle(Point2::ORIGIN, 5.7375, 0)],
    };
    let base = prof(vec![Profile::circle(Point2::ORIGIN, 5.7375, 0)]);
    let b = loft_and_bounds(
        &[base, low],
        free,
        &none,
        &with([None, Some(at(-0.4319, -1.8793, 11.1429))]),
        1,
    )
    .unwrap()
    .0;
    assert!(checked(&b) > 0.0);
    // a dome: walls arrive square to the axis, so the end is a smooth round nose
    let mut dome = at(0.0, 0.0, 8.0);
    dome.dome = Some((Vec3::Z, 2.0));
    let b = loft_and_bounds(
        std::slice::from_ref(&circle),
        free,
        &none,
        &with([None, Some(dome)]),
        1,
    )
    .unwrap()
    .0;
    let v = checked(&b);
    // of revolution: r(s), z(s) a natural cubic with the end slope given (research/spline-loops)
    let (l, r0): (f64, f64) = (8.0, 5.0);
    let radial = |s: f64| {
        // r = r0 + b s + d s^3; r(l) = 0, r'(l) = -2
        let d = (r0 - 2.0 * l) / (2.0 * l * l * l);
        let b = -2.0 - 3.0 * d * l * l;
        (r0 + b * s + d * s * s * s, b + 3.0 * d * s * s)
    };
    let axial = |s: f64| {
        // z = a s + c s^3; z(l) = 8, z'(l) = 0
        let c = -8.0 / (2.0 * l * l * l);
        let a = -3.0 * c * l * l;
        (a * s + c * s * s * s, a + 3.0 * c * s * s)
    };
    let rule = forge_math::poly::gauss_legendre(32);
    let want: f64 = rule
        .iter()
        .map(|&(x, w)| {
            let s = 0.5 * l * (x + 1.0);
            dmath::PI * radial(s).0.powi(2) * axial(s).1 * 0.5 * l * w
        })
        .sum();
    assert!((v - want).abs() < 1e-7 * want, "{v} {want}");
    let (_, dr) = radial(l);
    assert!((dr + 2.0).abs() < 1e-12 && axial(l).0 > 7.999);
}

#[test]
fn sections_line_up_by_their_starts() {
    use forge_math::Point3;
    use forge_ops::{LoftGuides, LoftKind, LoftOptions, loft_and_bounds};
    // a measured case's shape (research/spline-loops): two parallel circles far off one axis
    let high = Profile {
        plane: Frame::WORLD.with_origin(Point3::new(0.0, 96.0, 400.0)),
        loops: vec![Profile::circle(Point2::ORIGIN, 10.0, 0)],
    };
    let low = prof(vec![Profile::circle(Point2::ORIGIN, 6.0, 0)]);
    let plain = LoftOptions::default();
    let auto = LoftOptions {
        align: true,
        ..plain.clone()
    };
    let run = |o: &LoftOptions| {
        let b = loft_and_bounds(
            &[high.clone(), low.clone()],
            LoftKind::Ruled,
            &LoftGuides::None,
            o,
            1,
        )
        .unwrap()
        .0;
        checked(&b)
    };
    let (as_written, lined) = (run(&plain), run(&auto));
    // as written the rulings are parallel to the axis of an oblique cone: the frustum's volume
    let frustum = dmath::PI * 400.0 / 3.0 * (100.0 + 36.0 + 60.0);
    assert!(
        (as_written - frustum).abs() < 1e-6 * frustum,
        "{as_written} {frustum}"
    );
    // the second circle starts nearest the first's start: twisted, so less material
    assert!(lined < 0.8 * as_written, "{lined} {as_written}");
    // a seam given where the rule would put it is the same solid
    let d = (10.0f64, 96.0f64);
    let l = (d.0 * d.0 + d.1 * d.1).sqrt();
    let seamed = LoftOptions {
        seams: vec![None, Some(Point2::new(6.0 * d.0 / l, 6.0 * d.1 / l))],
        ..plain.clone()
    };
    assert!((run(&seamed) - lined).abs() < 1e-6 * lined);
}

#[test]
fn a_chamfer_runs_along_a_spline_edge() {
    use forge_ops::{ChamferSize, chamfer};
    let p = prof(vec![dome()]);
    let b = extrude(&p, 5.0, 1).unwrap();
    // the top edge of the wall: named from the cap and the side
    let root = forge_topo::Name::feature(1);
    let side = root.with_str("side").with(0);
    let name = forge_topo::Name::NONE.with_set(&[root.with_str("end"), side]);
    let e = b.edge_by_name(name).expect("the top edge");
    let c = chamfer(&b, &[b.edge(e).name], ChamferSize::Equal(0.5), 2).unwrap();
    let v = checked(&c);
    let v0 = forge_topo::mass::mass(&b).unwrap().volume;
    // a triangle of area 1/8 along about the edge's length, the wall's offset side only slightly less
    let cut = v0 - v;
    let len = {
        let Segment::Spline(s) = &dome().segments[0].0 else {
            panic!()
        };
        let br = s.c.breaks();
        let rule = forge_math::poly::gauss_legendre(16);
        (0..s.c.pieces())
            .map(|k| {
                let (lo, h) = (br[k], br[k + 1] - br[k]);
                rule.iter()
                    .map(|&(x, w)| 0.5 * h * w * s.c.deriv(lo + 0.5 * h * (x + 1.0)).d1.length())
                    .sum::<f64>()
            })
            .sum::<f64>()
    };
    assert!(cut > 0.8 * 0.125 * len && cut < 1.1 * 0.125 * len, "{cut} {len}");
}

#[test]
fn a_loft_runs_between_curved_sections() {
    use forge_math::Point3;
    use forge_ops::{LoftGuides, LoftKind, loft};
    let p2 = |x, y| Point2::new(x, y);
    let arch = |w: f64, h: f64, z: f64| {
        let c = PlaneSpline::control(2, vec![p2(-w, 0.0), p2(0.0, h), p2(w, 0.0)], None, None).unwrap();
        Profile {
            plane: Frame::WORLD.with_origin(Point3::new(0.0, 0.0, z)),
            loops: vec![ProfileLoop::new(vec![
                (s(c), 0),
                (
                    Segment::Line {
                        a: p2(w, 0.0),
                        b: p2(-w, 0.0),
                    },
                    1,
                ),
            ])],
        }
    };
    // a parabolic arch's area is 2/3 w h here, and a straight blend between same-shaped arches
    // has area (2/3) w(z) h(z) at each height: integrate it
    let (a, b) = (arch(6.0, 8.0, 0.0), arch(3.0, 4.0, 10.0));
    let plain = LoftGuides::None;
    let v = checked(&loft(&[a.clone(), b.clone()], LoftKind::Ruled, &plain, 1).unwrap());
    let want = 2.0 / 3.0 * 10.0 * (6.0 * 8.0 + 3.0 * 4.0 + 0.5 * (6.0 * 4.0 + 3.0 * 8.0)) / 3.0;
    assert!((v - want).abs() < 1e-7 * want, "{v} {want}");
    let smooth = LoftKind::Smooth {
        start: None,
        end: None,
    };
    let three = [a, arch(5.0, 6.0, 5.0), b];
    assert!(checked(&loft(&three, smooth, &plain, 1).unwrap()) > 0.0);
}
