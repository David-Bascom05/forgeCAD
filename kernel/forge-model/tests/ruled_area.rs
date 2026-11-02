//! A ruled loft between two circles, one turned: a hyperboloid, whose wall's area is a one-dimensional
//! integral. The reported error has to cover what the rule along the rulings leaves out.
use forge_math::{Frame, Point2, Point3, Vec3, dmath};
use forge_ops::{LoftGuides, LoftKind, Profile, ProfileLoop, Segment, loft};
use forge_topo::mass::mass;

fn ring(z: f64, r: f64, turn: f64) -> Profile {
    let half = |start: f64, id: u64| {
        (
            Segment::Arc {
                center: Point2::ORIGIN,
                radius: r,
                start,
                sweep: dmath::PI,
            },
            id,
        )
    };
    let (sn, cs) = dmath::sin_cos(turn);
    Profile {
        plane: Frame::from_axes(
            Point3::new(0.0, 0.0, z),
            Vec3::new(cs, sn, 0.0),
            Vec3::new(-sn, cs, 0.0),
            Vec3::Z,
        )
        .unwrap(),
        loops: vec![ProfileLoop::new(vec![half(0.0, 0), half(dmath::PI, 1)])],
    }
}

#[test]
fn a_hyperboloids_area_is_inside_its_reported_error() {
    let (a, b, h, turn) = (30.0, 24.0, 40.0, 0.5);
    let body = loft(
        &[ring(0.0, a, 0.0), ring(h, b, turn)],
        LoftKind::Ruled,
        &LoftGuides::None,
        1,
    )
    .unwrap();
    let m = mass(&body).unwrap();
    let (s, c) = dmath::sin_cos(turn);
    let rho = |t: f64| (((1.0 - t) * a + t * b * c).powi(2) + (t * b * s).powi(2)).sqrt();
    let n = 200_000;
    let d = 1.0 / n as f64;
    let wall: f64 = (0..n)
        .map(|i| {
            let t = (i as f64 + 0.5) * d;
            let slope = (rho(t + 1e-6) - rho(t - 1e-6)) / 2e-6;
            dmath::TAU * rho(t) * (slope * slope + h * h).sqrt() * d
        })
        .sum();
    let want = wall + dmath::PI * (a * a + b * b);
    let (got, err) = (m.area, m.err[1]);
    assert!((got - want).abs() <= err.max(1e-6), "{got} {want}, {err}");
}
