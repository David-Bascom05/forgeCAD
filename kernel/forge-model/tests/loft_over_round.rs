//! A loft running out over a round where a curved plate wall meets a barrel. The loft's sides cross
//! the round's springs at a shallow angle, where a fence and a spring part by a few hundredths.
use forge_bool::{Op, boolean};
use forge_math::{Frame, Point2, Point3, Vec2, Vec3, dmath};
use forge_ops::{Axis2, FilletEdge, LoftGuides, LoftKind, Profile, ProfileLoop, Radius, Segment};
use forge_ops::{extrude, fillet_kind, fillet_with, loft, revolve};
use forge_topo::{Body, check, mass::mass};

fn arc(c: (f64, f64), a: (f64, f64), b: (f64, f64), ccw: bool, id: u64) -> (Segment, u64) {
    let start = dmath::atan2(a.1 - c.1, a.0 - c.0);
    let mut sweep = dmath::atan2(b.1 - c.1, b.0 - c.0) - start;
    if ccw && sweep < 0.0 {
        sweep += dmath::TAU;
    }
    if !ccw && sweep > 0.0 {
        sweep -= dmath::TAU;
    }
    let radius = ((a.0 - c.0) * (a.0 - c.0) + (a.1 - c.1) * (a.1 - c.1)).sqrt();
    let center = Point2::new(c.0, c.1);
    (
        Segment::Arc {
            center,
            radius,
            start,
            sweep,
        },
        id,
    )
}

fn line(a: (f64, f64), b: (f64, f64), id: u64) -> (Segment, u64) {
    let (a, b) = (Point2::new(a.0, a.1), Point2::new(b.0, b.1));
    (Segment::Line { a, b }, id)
}

fn part() -> Body {
    let (p, q, r) = (
        (34.589532537714, -20.088908353179),
        (81.244590715619, 13.366283345820),
        (77.748913997209, 39.918584156444),
    );
    let m = |a: (f64, f64)| (-a.0, a.1);
    let flat = |plane, l| Profile {
        plane,
        loops: vec![ProfileLoop::new(l)],
    };
    let wall = (95.121214478714, -55.244497971241);
    let plate = extrude(
        &flat(
            Frame::WORLD,
            vec![
                arc(wall, p, q, false, 0),
                arc((105.0, 30.0), q, r, false, 1),
                arc((0.0, -555.0226788034), r, m(r), true, 2),
                arc(m((105.0, 30.0)), m(r), m(q), false, 3),
                arc(m(wall), m(q), m(p), false, 4),
                arc((0.0, 0.0), m(p), p, true, 5),
            ],
        ),
        12.0,
        1,
    )
    .unwrap();
    let side = Frame::from_axes(Point3::new(105.0, 30.0, 12.0), Vec3::X, Vec3::Z, -Vec3::Y).unwrap();
    let y = Axis2 {
        origin: Point2::ORIGIN,
        dir: Vec2::Y,
    };
    let barrel = revolve(
        &flat(
            side,
            vec![
                line((0.0, -18.0), (33.5, -18.0), 0),
                line((33.5, -18.0), (34.832441509613, -16.509390762591), 1),
                arc(
                    (259.583333333333, 8.0),
                    (34.832441509613, -16.509390762591),
                    (35.0, 34.0),
                    false,
                    2,
                ),
                line((35.0, 34.0), (1.5, 34.0), 3),
                line((1.5, 34.0), (0.0, 32.5), 4),
                line((0.0, 32.5), (0.0, -18.0), 5),
            ],
        ),
        y,
        dmath::TAU,
        2,
    )
    .unwrap();
    let joined = boolean(&plate, &barrel, Op::Union, 3).unwrap();
    let concave: Vec<FilletEdge> = joined
        .edge_ids()
        .filter(|&e| fillet_kind(&joined, e) == Some(false))
        .map(|edge| FilletEdge {
            edge,
            radius: Radius::Constant(3.0),
        })
        .collect();
    let rounded = fillet_with(&joined, &concave, &Default::default(), 4).unwrap();
    let (c, s) = (0.961523947640823f64, 0.274721127897378f64);
    let n = 1.0 / (c * c + s * s).sqrt();
    let (x, y, z) = (
        Vec3::Z,
        Vec3::new(s * n, -c * n, 0.0),
        Vec3::new(c * n, s * n, 0.0),
    );
    let section = |o: Point3, w: f64, h: f64| {
        let f = Frame::from_axes(o, x, y, z).unwrap();
        let (r, l) = (4.0, -2.0);
        let loop_ = vec![
            arc((l + r, -h + r), (l, -h + r), (l + r, -h), true, 0),
            line((l + r, -h), (w - r, -h), 1),
            arc((w - r, -h + r), (w - r, -h), (w, -h + r), true, 2),
            line((w, -h + r), (w, h - r), 3),
            arc((w - r, h - r), (w, h - r), (w - r, h), true, 4),
            line((w - r, h), (l + r, h), 5),
            arc((l + r, h - r), (l + r, h), (l, h - r), true, 6),
            line((l, h - r), (l, -h + r), 7),
        ];
        flat(f, loop_)
    };
    let arm = loft(
        &[
            section(Point3::new(21.153526848, 6.043864814, 12.0), 28.0, 15.0),
            section(Point3::new(35.384477503, 10.109850715, 12.0), 15.0, 8.5),
            section(Point3::new(49.615428157, 14.175836616, 12.0), 12.0, 7.5),
            section(Point3::new(63.846378812, 18.241822518, 12.0), 16.0, 8.5),
            section(Point3::new(78.077329466, 22.307808419, 12.0), 30.0, 14.0),
        ],
        LoftKind::Smooth {
            start: None,
            end: None,
        },
        &LoftGuides::None,
        5,
    )
    .unwrap();
    let or = boolean(&rounded, &arm, Op::Union, 6).unwrap();
    let and = boolean(&rounded, &arm, Op::Intersect, 7).unwrap();
    assert_eq!(check(&or), vec![]);
    assert_eq!(check(&and), vec![]);
    let v = |b: &Body| mass(b).unwrap().volume;
    assert!(
        (v(&or) + v(&and) - v(&rounded) - v(&arm)).abs() < 1e-3,
        "{} {}",
        v(&or),
        v(&and)
    );
    or
}

#[test]
fn a_loft_over_a_round_crosses_its_springs() {
    part();
}
