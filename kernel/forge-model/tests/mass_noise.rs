//! Two rounds on a lens post's foot meet along a curve where both sides are blends. On walls that are
//! Ruled surfaces (the lens's arcs given as curves, which no wall recognises as a cylinder) that
//! curve's trace carries the noise of the walls' nearest-point searches, and refining it stops at
//! that noise. The same post with cylinder walls is the analytic twin: the same solid, known far
//! more closely. The true error stays inside the reported bound.
use forge_bool::{Op, boolean};
use forge_geom::spline::PlaneSpline;
use forge_math::{Frame, Point2, Point3, Vec3, dmath};
use forge_ops::{
    FilletEdge, LoftGuides, LoftKind, Profile, ProfileLoop, Radius, Segment, extrude, fillet_with, loft,
};
use forge_topo::{Body, check, mass::mass};
use std::sync::Arc;

/// A lens: two arcs of radius `big` meeting at sharp tips at x = +-w, as curves (so its walls stay
/// Ruled) or as arcs (cylinders).
fn lens(big: f64, w: f64, curves: bool) -> ProfileLoop {
    let c = (big * big - w * w).sqrt();
    let half = dmath::asin(w / big);
    let side = |cy: f64, mid: f64, id: u64| {
        let at = Point2::new(0.0, cy);
        let (start, sweep) = (mid - half, 2.0 * half);
        let s = if curves {
            Segment::Spline(Arc::new(
                PlaneSpline::ellipse(at, big, big, 0.0, start, sweep).unwrap(),
            ))
        } else {
            Segment::Arc {
                center: at,
                radius: big,
                start,
                sweep,
            }
        };
        (s, id)
    };
    ProfileLoop::new(vec![side(-c, dmath::FRAC_PI_2, 0), side(c, -dmath::FRAC_PI_2, 1)])
}

/// A lens post on a plate, the foot rounded by `f`: blends on two sides meeting along a mitre.
fn post(big: f64, w: f64, f: f64, lofted: bool) -> Option<Body> {
    let at = |z: f64, loops: Vec<ProfileLoop>| Profile {
        plane: Frame::from_axes(Point3::new(0.0, 0.0, z), Vec3::X, Vec3::Y, Vec3::Z).unwrap(),
        loops,
    };
    let plate = at(
        -20.0,
        vec![Profile::rectangle(Point2::new(-80.0, -80.0), 160.0, 160.0, 0)],
    );
    let plate = extrude(&plate, 20.0, 1).unwrap();
    let post = if lofted {
        let ends = [
            at(-5.0, vec![lens(big, w, true)]),
            at(40.0, vec![lens(big, w, true)]),
        ];
        loft(&ends, LoftKind::Ruled, &LoftGuides::None, 2).unwrap()
    } else {
        extrude(&at(-5.0, vec![lens(big, w, false)]), 45.0, 2).unwrap()
    };
    let u = boolean(&plate, &post, Op::Union, 3).unwrap();
    let foot: Vec<FilletEdge> = u
        .edge_ids()
        .filter(|&e| {
            u.edge_points(e, 8)
                .iter()
                .all(|p| p.z.abs() < 1e-9 && p.x.abs() < 70.0 && p.y.abs() < 70.0)
        })
        .map(|edge| FilletEdge {
            edge,
            radius: Radius::Constant(f),
        })
        .collect();
    assert_eq!(foot.len(), 2);
    let out = fillet_with(&u, &foot, &Default::default(), 4).ok()?;
    assert_eq!(check(&out), vec![]);
    Some(out)
}

#[test]
fn blends_meeting_on_a_lofted_post_are_their_analytic_twin() {
    for (big, w, f) in [
        (40.0, 20.0, 2.0),
        (40.0, 20.0, 1.0),
        (60.0, 25.0, 3.0),
        (30.0, 18.0, 1.5),
    ] {
        let exact = mass(&post(big, w, f, false).unwrap()).unwrap();
        let m = mass(&post(big, w, f, true).unwrap()).unwrap();
        let (dv, da) = ((m.volume - exact.volume).abs(), (m.area - exact.area).abs());
        assert!(dv > 0.0, "the twins should come by different paths");
        assert!(
            dv <= m.err[0] + exact.err[0],
            "volume off {dv}, bound {}",
            m.err[0]
        );
        assert!(da <= m.err[1] + exact.err[1], "area off {da}, bound {}", m.err[1]);
    }
}
