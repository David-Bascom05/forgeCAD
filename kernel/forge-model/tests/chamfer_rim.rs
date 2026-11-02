//! A boss's end rim after the round at its root: a rim in two halves, either side of its faces'
//! seams, and a chamfer picked on them or on one of them.
use forge_math::{Frame, Point2, Point3, Vec2, Vec3, dmath};
use forge_ops::{Axis2, ChamferSize, FilletEdge, Profile, ProfileLoop, Radius, Segment};
use forge_ops::{chamfer, fillet_with, revolve};
use forge_topo::{Body, EdgeId, check, mass::mass};

fn line(a: (f64, f64), b: (f64, f64), id: u64) -> (Segment, u64) {
    let (a, b) = (Point2::new(a.0, a.1), Point2::new(b.0, b.1));
    (Segment::Line { a, b }, id)
}

fn at_radius(b: &Body, r: f64, z: f64) -> Vec<EdgeId> {
    b.edge_ids()
        .filter(|&e| {
            b.edge_points(e, 8)
                .iter()
                .all(|p| (p.z - z).abs() < 1e-9 && ((p.x * p.x + p.y * p.y).sqrt() - r).abs() < 1e-9)
        })
        .collect()
}

#[test]
fn a_chamfer_runs_round_a_rim_past_its_seam() {
    let plane = Frame::from_axes(Point3::ORIGIN, Vec3::X, Vec3::Z, -Vec3::Y).unwrap();
    let axis = Axis2 {
        origin: Point2::ORIGIN,
        dir: Vec2::Y,
    };
    let loops = vec![ProfileLoop::new(vec![
        line((0.0, -16.0), (24.0, -16.0), 0),
        line((24.0, -16.0), (24.0, 0.0), 1),
        line((24.0, 0.0), (92.0, 0.0), 2),
        line((92.0, 0.0), (92.0, 2.6), 3),
        line((92.0, 2.6), (59.0702281, 4.9026740), 4),
        (
            Segment::Arc {
                center: Point2::new(62.0, 46.8003641),
                radius: 42.0,
                start: dmath::atan2(4.9026740 - 46.8003641, 59.0702281 - 62.0),
                sweep: -dmath::PI - dmath::atan2(4.9026740 - 46.8003641, 59.0702281 - 62.0),
            },
            5,
        ),
        line((20.0, 46.8003641), (20.0, 51.8), 6),
        line((20.0, 51.8), (0.0, 51.8), 7),
        line((0.0, 51.8), (0.0, -16.0), 8),
    ])];
    let part = revolve(&Profile { plane, loops }, axis, dmath::TAU, 1).unwrap();
    let root: Vec<FilletEdge> = at_radius(&part, 24.0, 0.0)
        .into_iter()
        .map(|edge| FilletEdge {
            edge,
            radius: Radius::Constant(3.0),
        })
        .collect();
    let part = fillet_with(&part, &root, &Default::default(), 2).unwrap();
    assert_eq!(check(&part), vec![]);
    let rims = at_radius(&part, 24.0, -16.0);
    assert_eq!(rims.len(), 2);
    let (c, whole) = (1.0, mass(&part).unwrap().volume);
    // the triangle's area round the path of its centroid
    let want = 0.5 * c * c * dmath::TAU * (24.0 - c / 3.0);
    let got = chamfer(&part, &[part.edge(rims[0]).name], ChamferSize::Equal(c), 4).unwrap();
    assert_eq!(check(&got), vec![]);
    let took = whole - mass(&got).unwrap().volume;
    assert!((took - want).abs() < 1e-6, "{took} {want}");
}

/// A rim of two arcs between one plane and one cylinder, its vertices no seam of theirs.
#[test]
fn a_chamfer_runs_round_two_arcs_of_one_rim() {
    use forge_ops::extrude;
    let arc = |start: f64, id| {
        (
            Segment::Arc {
                center: Point2::ORIGIN,
                radius: 10.0,
                start,
                sweep: dmath::PI,
            },
            id,
        )
    };
    let post = extrude(
        &Profile {
            plane: Frame::WORLD,
            loops: vec![ProfileLoop::new(vec![arc(0.0, 0), arc(dmath::PI, 1)])],
        },
        8.0,
        1,
    )
    .unwrap();
    let rims = at_radius(&post, 10.0, 8.0);
    assert_eq!(rims.len(), 2);
    let (c, whole) = (1.0, mass(&post).unwrap().volume);
    let want = 0.5 * c * c * dmath::TAU * (10.0 - c / 3.0);
    let got = chamfer(&post, &[post.edge(rims[0]).name], ChamferSize::Equal(c), 2).unwrap();
    assert_eq!(check(&got), vec![]);
    let took = whole - mass(&got).unwrap().volume;
    assert!((took - want).abs() < 1e-6, "{took} {want}");
}
