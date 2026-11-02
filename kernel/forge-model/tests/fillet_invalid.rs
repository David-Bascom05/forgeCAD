//! A round that builds a body failing `check` is refused rather than returned.
use forge_math::{Frame, Point2, Point3, Vec3, dmath};
use forge_ops::{
    FilletEdge, FilletError, LoftGuides, LoftKind, Profile, ProfileLoop, Radius, Segment, fillet, loft,
};
use forge_topo::{Body, EdgeId, check};

const TILT: f64 = 0.0422;

fn section(o: (f64, f64), grow: f64) -> Profile {
    let (a, b, c) = (-57.65 - grow, 63.65 + grow, 8.25 + grow);
    let d = c - 3.9;
    let p = [(a, -c), (b, -d), (b, d), (a, c)].map(|(x, y)| Point2::new(x, y));
    let side = |i: usize| {
        (
            Segment::Line {
                a: p[i],
                b: p[(i + 1) % 4],
            },
            i as u64,
        )
    };
    Profile {
        plane: Frame::from_axes(
            Point3::new(o.0, 0.0, o.1),
            Vec3::new(dmath::cos(TILT), 0.0, dmath::sin(TILT)),
            -Vec3::Y,
            Vec3::new(dmath::sin(TILT), 0.0, -dmath::cos(TILT)),
        )
        .unwrap(),
        loops: vec![ProfileLoop::new((0..4).map(side).collect())],
    }
}

fn nearest(b: &Body, to: Point3) -> EdgeId {
    let d = |e: EdgeId| {
        b.edge_points(e, 8)
            .iter()
            .map(|p| p.distance(to))
            .fold(f64::MAX, f64::min)
    };
    b.edge_ids().min_by(|&x, &y| d(x).total_cmp(&d(y))).unwrap()
}

fn round(b: &Body, near: &[Point3], r: f64, feature: u64) -> Result<Body, FilletError> {
    let list: Vec<FilletEdge> = near
        .iter()
        .map(|&p| FilletEdge {
            edge: nearest(b, p),
            radius: Radius::Constant(r),
        })
        .collect();
    fillet(b, &list, feature)
}

#[test]
fn a_round_stands_valid_or_is_refused() {
    // a long tapered plate: its four long edges rounded, then the round of one rounded
    let plate = loft(
        &[section((84.83, 11.16), 0.0), section((85.04, 6.17), 0.44)],
        LoftKind::Ruled,
        &LoftGuides::None,
        1,
    )
    .unwrap();
    let long = [
        Point3::new(27.0, 8.4, 6.7),
        Point3::new(27.0, -8.4, 6.7),
        Point3::new(148.7, 4.5, 11.9),
        Point3::new(148.7, -4.5, 11.9),
    ];
    let first = round(&plate, &long, 2.5, 2).unwrap();
    assert!(check(&first).is_empty());
    match round(&first, &[Point3::new(84.83, 6.6, 11.16)], 1.5, 3) {
        Ok(b) => assert_eq!(check(&b), vec![]),
        Err(FilletError::Invalid(p)) => assert!(!p.is_empty()),
        Err(_) => {}
    }
}
