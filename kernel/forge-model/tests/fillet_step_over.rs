//! A curved blade lofted with a twist through a plate, rounded where it meets the plate. The round
//! tracks round the blade's narrow end, where a trace that finds no step ahead tries longer ones: one
//! landing on the far arm of the same curve must not count as carrying on.
use forge_bool::{Op, boolean};
use forge_math::{Frame, Point2, Point3, Vec3, dmath};
use forge_mesh::tessellate::{Quality, tessellate};
use forge_ops::{
    FilletEdge, LoftGuides, LoftKind, Profile, ProfileLoop, Radius, Segment, extrude, fillet_with, loft,
};
use forge_topo::{Body, check};

/// A thick arc about (8.6, -34.2), 4.5 wide, its two ends rounded.
fn blade_section(z: f64, turn: f64) -> Profile {
    let (c, mean, half) = (Point2::new(8.5658, -34.1526), 61.0580, 2.25);
    let (from, to) = (1.6480, -0.3155);
    let at = |a: f64| {
        let (s, k) = dmath::sin_cos(a);
        Point2::new(c.x + mean * k, c.y + mean * s)
    };
    let arc = |center: Point2, radius: f64, start: f64, sweep: f64, id: u64| {
        (
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            },
            id,
        )
    };
    let pi = dmath::PI;
    let outline = ProfileLoop::new(vec![
        arc(at(from), half, from, pi, 0),
        arc(c, mean - half, from, to - from, 1),
        arc(at(to), half, to + pi, pi, 2),
        arc(c, mean + half, to, from - to, 3),
    ]);
    let (s, k) = dmath::sin_cos(turn);
    Profile {
        plane: Frame::from_axes(
            Point3::new(0.0, 0.0, z),
            Vec3::new(k, s, 0.0),
            Vec3::new(-s, k, 0.0),
            Vec3::Z,
        )
        .unwrap(),
        loops: vec![outline],
    }
}

fn plate_and_blade() -> Body {
    let plate = Profile {
        plane: Frame::WORLD,
        loops: vec![Profile::circle(Point2::ORIGIN, 125.0, 0)],
    };
    let plate = extrude(&plate, 5.0, 1).unwrap();
    let kind = LoftKind::Smooth {
        start: None,
        end: None,
    };
    let sections = [blade_section(0.0, 0.0), blade_section(20.0, 15f64.to_radians())];
    let blade = loft(&sections, kind, &LoftGuides::None, 2).unwrap();
    boolean(&plate, &blade, Op::Union, 3).unwrap()
}

#[test]
fn a_round_about_a_blade_s_narrow_end_stays_on_its_own_arm() {
    let r = 2.0;
    let joined = plate_and_blade();
    let foot: Vec<FilletEdge> = joined
        .edge_ids()
        .filter(|&e| {
            joined
                .edge_points(e, 6)
                .iter()
                .all(|p| (p.z - 5.0).abs() < 1e-6 && (p.x * p.x + p.y * p.y).sqrt() < 120.0)
        })
        .map(|edge| FilletEdge {
            edge,
            radius: Radius::Constant(r),
        })
        .collect();
    assert!(!foot.is_empty());
    let rounded = fillet_with(&joined, &foot, &Default::default(), 4).unwrap();
    assert_eq!(check(&rounded), vec![]);
    let volume = |b: &Body| {
        let q = Quality {
            chord: 0.05,
            ..Default::default()
        };
        tessellate(b, &q).unwrap().volume()
    };
    let added = volume(&rounded) - volume(&joined);
    let run = 2.0 * 61.058 * 1.9635 + 2.0 * dmath::PI * 2.25;
    let section = r * r * (1.0 - dmath::PI / 4.0);
    assert!(
        added > run * section && added < 1.5 * run * section,
        "{added} against {}",
        run * section
    );
}
