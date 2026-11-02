//! Rounds along a rim of many lobes, up to nearly the radius of the lobes' tips.

use forge_geom::CurveGeom;
use forge_geom::spline::PlaneSpline;
use forge_math::{Frame, Point2, dmath};
use forge_mesh::{Quality, tessellate};
use forge_ops::{FilletEdge, Profile, ProfileLoop, Radius, Segment, extrude, fillet};
use forge_topo::Body;
use std::f64::consts::{FRAC_PI_4, PI, TAU};
use std::sync::Arc;

/// A cycloidal disc: a closed spline through an offset epitrochoid, 15 lobes, tips of radius 6.2,
/// valleys of radius 4.6.
fn disc() -> Body {
    let (pins, ring, roller, e) = (16.0, 48.0, 4.05, 2.0);
    let at = |t: f64| {
        let psi = dmath::atan2(
            dmath::sin((1.0 - pins) * t),
            ring / (e * pins) - dmath::cos((1.0 - pins) * t),
        );
        Point2::new(
            ring * dmath::cos(t) - roller * dmath::cos(t + psi) - e * dmath::cos(pins * t),
            ring * dmath::sin(t) - roller * dmath::sin(t + psi) - e * dmath::sin(pins * t),
        )
    };
    let mut pts: Vec<Point2> = (0..240).map(|i| at(TAU * i as f64 / 240.0)).collect();
    pts.push(pts[0]);
    let spline = PlaneSpline::through(pts, None, None).expect("a spline");
    let profile = Profile {
        plane: Frame::WORLD,
        loops: vec![ProfileLoop::new(vec![(Segment::Spline(Arc::new(spline)), 0)])],
    };
    extrude(&profile, 12.0, 1).expect("disc")
}

/// A constant round on a convex plane rim takes away its section, a square corner less a quarter
/// circle, swept along the rim: by Pappus with the rim's turning, r^2 (1 - pi/4) (L - 2 pi k r), k the
/// section's centroid off the wall over r, whatever the rim's shape. The traced spine must bend as
/// tight as tip radius less r, and the spring runs far faster than the spine round a tip. The radius
/// passes the valleys' less a margin: moving in, they open out, and only the tips can pinch.
#[test]
fn rounds_on_a_lobed_rim_take_their_closed_form() {
    let b = disc();
    let before = forge_topo::mass::mass(&b).expect("mass");
    let length = (before.area - 2.0 * before.volume / 12.0) / 12.0;
    let k = (10.0 - 3.0 * PI) / (12.0 - 3.0 * PI);
    let rim: Vec<_> = b
        .edge_ids()
        .filter(|&e| {
            let x = b.edge(e);
            (x.curve.eval(x.range.mid()).z - 12.0).abs() < 1e-6
        })
        .collect();
    // (at 6 the spine's radius is about 0.2, and the wall's speed along it nearly a pole)
    #[cfg(not(target_arch = "wasm32"))]
    let radii = [(4.0, 1e-9), (5.5, 1e-9), (6.0, 1e-6)];
    // (unoptimized there, each takes most of a minute)
    #[cfg(target_arch = "wasm32")]
    let radii = [(5.5, 1e-9)];
    for (r, within) in radii {
        let picks: Vec<FilletEdge> = rim
            .iter()
            .map(|&edge| FilletEdge {
                edge,
                radius: Radius::Constant(r),
            })
            .collect();
        let done = fillet(&b, &picks, 9).expect("rounds");
        assert!(forge_topo::check(&done).is_empty());
        let want = r * r * (1.0 - FRAC_PI_4) * (length - TAU * k * r);
        let got = before.volume - forge_topo::mass::mass(&done).expect("mass").volume;
        assert!(
            (got - want).abs() < within * want,
            "r {r}: took {got}, closed form {want}"
        );
        let mesh = tessellate(&done, &Quality::default()).expect("meshes");
        assert!(mesh.open_edges().is_empty());
    }
}
