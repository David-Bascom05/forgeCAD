//! A tube square to a torus's axis, the axes not meeting. The roots of a mirror pair meet at one u, and
//! the discriminant lies flat there: the closed form must not link a root to its mirror image, a chain
//! on both surfaces at every point but jumping across the torus between two of them.
use forge_geom::intersect::SurfaceSurface;
use forge_geom::meet::intersect;
use forge_geom::{Curve, Cylinder, Surface, Torus, meet};
use forge_math::{Box3, Frame, Point3, Vec3};

#[test]
fn a_tube_square_to_a_torus_axis_has_no_chain_across_the_torus() {
    let torus = Surface::Torus(
        Torus::new(
            Frame::from_axes(Point3::new(150.5, 20.0, -13.0), Vec3::Z, -Vec3::Y, Vec3::X).unwrap(),
            5.5,
            2.5,
        )
        .unwrap(),
    );
    // (the rounding in the axes is what puts the event on the wrong side)
    let tube = Surface::Cylinder(Cylinder {
        frame: Frame::from_axes(
            Point3::new(143.0, -18.0, -12.000000000000002),
            Vec3::X,
            Vec3::new(0.0, 6.123233995736766e-17, -1.0),
            Vec3::new(0.0, 1.0, 6.123233995736766e-17),
        )
        .unwrap(),
        radius: 8.25,
    });
    let region = Box3::from_points([
        Point3::new(142.49999, 11.99999, -20.25002000000001),
        Point3::new(151.25002000000006, 14.500020000000008, -4.99999),
    ]);
    // the tracer's to answer if the closed form will not
    let Some(SurfaceSurface::Curves(cs)) = meet::intersect(&tube, &torus, region) else {
        return;
    };
    for (c, _) in &cs {
        let Curve::Intersection(ic) = c else {
            panic!("a chain")
        };
        for w in ic.pts.windows(2) {
            assert!(w[0].distance(w[1]) < 3.0, "{:?} to {:?}", w[0], w[1]);
        }
    }
}

/// A cylinder lying across a torus's flat top: going round the cylinder the root count runs 0, 4, 2
/// in a band a few hundredths of a turn wide, which the quarter-point count checks do not see. The
/// two outer roots must not be joined as a turn over the stretch of curve between them.
#[test]
fn a_band_of_roots_too_thin_to_see_does_not_lose_its_stretch_of_curve() {
    let torus = Surface::Torus(Torus {
        frame: Frame::from_axes(Point3::ORIGIN, Vec3::X, -Vec3::Z, Vec3::Y).unwrap(),
        major: 56.0,
        minor: 8.0,
    });
    let cylinder = Surface::Cylinder(Cylinder {
        frame: Frame::from_axes(Point3::new(0.0, -4.0, -64.35), Vec3::Z, Vec3::Y, -Vec3::X).unwrap(),
        radius: 11.0,
    });
    let region = Box3::from_points([Point3::new(-40.0, -8.0, -70.0), Point3::new(40.0, 7.0, -40.0)]);
    let step = 0.5 * region.diagonal() / 64.0;
    if let Some(SurfaceSurface::Curves(curves)) = intersect(&torus, &cylinder, region) {
        for (c, _) in &curves {
            let Curve::Intersection(c) = c else { continue };
            let n = c.pts.len();
            for i in 0..c.segments() {
                let (p, q) = (c.pts[i], c.pts[(i + 1) % n]);
                if region.contains(p) || region.contains(q) {
                    assert!(p.distance(q) <= 3.0 * step, "{p:?} {q:?}");
                }
            }
        }
    }
}
