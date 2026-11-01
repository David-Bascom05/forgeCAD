//! Real curves in profiles: extrude and revolve walls, booleans against them, mass against meshes.

use forge_geom::spline::PlaneSpline;
use forge_math::{Frame, Point2, dmath};
use forge_mesh::{Quality, tessellate};
use forge_ops::{Axis2, Profile, ProfileLoop, Segment, extrude, revolve};
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
