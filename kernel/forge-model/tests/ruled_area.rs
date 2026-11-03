//! A ruled loft between two circles, one turned: a hyperboloid, whose wall's area along its rulings is
//! no polynomial. The area is right to rounding, and the reported error covers it closely.
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
    // 40-digit quadrature of the wall plus the caps: research/mass-props/ruled-area/reference.py
    let want = 11368.879628378854;
    let (miss, err) = ((m.area - want).abs(), m.err[1]);
    assert!(
        miss <= 1e-14 * want && miss <= err && err <= 1e-12 * want,
        "{miss} {err}"
    );
}

/// A square lofted to a smaller turned square: four hyperbolic paraboloids.
#[test]
fn a_twisted_squares_area_is_inside_its_reported_error() {
    let square = |z: f64, w: f64, turn: f64| Profile {
        loops: vec![Profile::rectangle(Point2::new(-w / 2.0, -w / 2.0), w, w, 0)],
        ..ring(z, 1.0, turn)
    };
    let body = loft(
        &[square(0.0, 20.0, 0.0), square(15.0, 12.0, 0.6)],
        LoftKind::Ruled,
        &LoftGuides::None,
        1,
    )
    .unwrap();
    let m = mass(&body).unwrap();
    let want = 1524.6209890747455;
    let (miss, err) = ((m.area - want).abs(), m.err[1]);
    assert!(
        miss <= 1e-14 * want && miss <= err && err <= 1e-12 * want,
        "{miss} {err}"
    );
}
