//! A draft on one wall of a stadium: the wall's arcs and far side go with it, and the part is the
//! stadium's offset at each height.
use forge_math::{Frame, Point2, Point3, Vec3, dmath};
use forge_ops::{Profile, ProfileLoop, Segment, draft, extrude};
use forge_topo::{check, mass::mass};

#[test]
fn a_draft_runs_on_round_a_smooth_wall() {
    let (l, r, h) = (30.0, 10.0, 12.0);
    let arc = |x: f64, start: f64, id| {
        (
            Segment::Arc {
                center: Point2::new(x, 0.0),
                radius: r,
                start,
                sweep: dmath::PI,
            },
            id,
        )
    };
    let line = |a: (f64, f64), b: (f64, f64), id| {
        (
            Segment::Line {
                a: Point2::new(a.0, a.1),
                b: Point2::new(b.0, b.1),
            },
            id,
        )
    };
    let outline = vec![
        line((0.0, -r), (l, -r), 0),
        arc(l, -dmath::FRAC_PI_2, 1),
        line((l, r), (0.0, r), 2),
        arc(0.0, dmath::FRAC_PI_2, 3),
    ];
    let part = extrude(
        &Profile {
            plane: Frame::WORLD,
            loops: vec![ProfileLoop::new(outline)],
        },
        h,
        1,
    )
    .unwrap();
    let wall = part
        .face_ids()
        .find(|&f| {
            let c = part.face_box(f).center();
            (c.y + r).abs() < 1e-9 && (c.z - h / 2.0).abs() < 1e-9
        })
        .unwrap();
    let a = 3f64.to_radians();
    let neutral = Frame::from_axes(Point3::ORIGIN, Vec3::X, Vec3::Y, Vec3::Z).unwrap();
    let got = draft(&part, &[part.face(wall).name], &neutral, a).unwrap();
    assert_eq!(check(&got), vec![]);
    // the area at height z of the outline offset in by t z: integrated
    let t = dmath::sin(a) / dmath::cos(a);
    let (area, perim) = (2.0 * r * l + dmath::PI * r * r, 2.0 * l + dmath::TAU * r);
    let want = area * h - perim * t * h * h / 2.0 + dmath::PI * t * t * h * h * h / 3.0;
    let v = mass(&got).unwrap().volume;
    assert!((v - want).abs() < 1e-6, "{v} {want}");
}
