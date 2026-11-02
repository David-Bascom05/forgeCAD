//! Rims rolling round the top of a plate with one arc side: where the rim's plane-side cylinder and
//! arc-side torus meet they touch at the crown. The torus face's area is a one-dimensional integral.
use forge_geom::Surface;
use forge_math::{Frame, Point2, Point3, Vec3, dmath};
use forge_ops::{FilletEdge, Profile, ProfileLoop, Radius, Segment, extrude, fillet_with};
use forge_topo::{check, mass::mass};

fn plate(cx: f64, cy: f64, big: f64, xl: f64, xr: f64, tube: f64) -> (f64, f64, f64) {
    let major = big + tube;
    let ya = cy + (big * big - (xl - cx) * (xl - cx)).sqrt();
    let yb = cy + (big * big - (xr - cx) * (xr - cx)).sqrt();
    let (a0, a1) = (dmath::atan2(ya - cy, xl - cx), dmath::atan2(yb - cy, xr - cx));
    let top = ya.max(yb) + 40.0;
    let at = Point2::new;
    let line = |a, b, id| (Segment::Line { a, b }, id);
    let outline = ProfileLoop::new(vec![
        (
            Segment::Arc {
                center: at(cx, cy),
                radius: big,
                start: a1,
                sweep: a0 - a1,
            },
            0,
        ),
        line(at(xl, ya), at(xl, top), 1),
        line(at(xl, top), at(xr, top), 2),
        line(at(xr, top), at(xr, yb), 3),
    ]);
    let frame = Frame::from_axes(Point3::ORIGIN, Vec3::X, Vec3::Y, Vec3::Z).unwrap();
    let b = extrude(
        &Profile {
            plane: frame,
            loops: vec![outline],
        },
        24.0,
        1,
    )
    .unwrap();
    let rim: Vec<FilletEdge> = b
        .edge_ids()
        .filter(|&e| {
            [b.edge(e).start, b.edge(e).end]
                .iter()
                .all(|&v| (b.vertex(v).point.z - 24.0).abs() < 1e-9)
        })
        .map(|edge| FilletEdge {
            edge,
            radius: Radius::Constant(tube),
        })
        .collect();
    let b = fillet_with(&b, &rim, &Default::default(), 2).unwrap();
    assert_eq!(check(&b), vec![]);
    let m = mass(&b).unwrap();
    let n = 100_000;
    let mut want = 0.0;
    for i in 0..n {
        let v = (i as f64 + 0.5) / n as f64 * dmath::FRAC_PI_2;
        let c = dmath::cos(v);
        let rho = major - tube * c;
        let left = dmath::acos((xl + tube - tube * c - cx) / rho);
        let right = dmath::acos((xr - tube + tube * c - cx) / rho);
        want += tube * rho * (left - right) * dmath::FRAC_PI_2 / n as f64;
    }
    let got = m
        .faces
        .iter()
        .find(|(f, _)| matches!(b.face(*f).surface, Surface::Torus(_)))
        .expect("the arc's rim")
        .1;
    (got, want, m.err[0])
}

#[test]
fn a_rim_meeting_at_a_crown_has_its_area() {
    for (cx, cy, big, xl, xr, tube) in [
        (-70.0, -100.0, 117.5, -100.0, -25.0, 2.5),
        (-71.7, -102.7, 117.3, -100.0, -27.0, 2.5),
        (0.0, -80.0, 100.0, -50.0, 50.0, 1.0),
        (10.0, -40.0, 60.0, -30.0, 45.0, 3.0),
        (-5.0, -200.0, 230.0, -60.0, 40.0, 4.0),
        (20.0, -30.0, 50.0, -15.0, 50.0, 1.5),
    ] {
        let (got, want, err) = plate(cx, cy, big, xl, xr, tube);
        assert!(
            ((got - want) / want).abs() < 1e-7,
            "{big} {tube}: {got} not {want}"
        );
        // the bridges' stray is in the reported error, and it stays small
        assert!(err < 2e-2, "{big} {tube}: {err}");
    }
}
