use forge_math::{Frame, Point2};
use forge_ops::{Profile, ProfileLoop, Segment, extrude};
use forge_topo::check;

/// A pie slice of a 60 mm circle over 0.94 rad with a hole whose corner lies 1.3e-4 inside the arc,
/// midway between two of the arc's samples at eight chords a sample: the chord there sags 2.9e-3,
/// so the hole's corner is past it, though not past the arc. The face is sound; `check` drew it
/// finer (64 and 512 times) before believing its loops cross (Haas HBF-12 #24138, a corner 1.3e-4
/// off an arc a boolean's unify had joined whole).
#[test]
fn a_corner_a_hair_inside_a_long_arc_is_not_a_crossing() {
    let (r, a0, sweep) = (60.0, 0.3, 0.94);
    let polar = |rr: f64, a: f64| Point2::new(rr * forge_math::dmath::cos(a), rr * forge_math::dmath::sin(a));
    let line = |a: Point2, b: Point2| Segment::Line { a, b };
    let outer = ProfileLoop::new(vec![
        (line(Point2::ORIGIN, polar(r, a0)), 1),
        (
            Segment::Arc {
                center: Point2::ORIGIN,
                radius: r,
                start: a0,
                sweep,
            },
            2,
        ),
        (line(polar(r, a0 + sweep), Point2::ORIGIN), 3),
    ]);
    // the hole's tip midway between two samples at 8 x 6 chords (6: the default for 0.94 rad)
    let at = a0 + sweep * 4.5 / 48.0;
    let tip = polar(r - 1.3e-4, at);
    let (left, right) = (polar(r - 5.0, at + 0.05), polar(r - 5.0, at - 0.05));
    let hole = ProfileLoop::new(vec![
        (line(tip, left), 4),
        (line(left, right), 5),
        (line(right, tip), 6),
    ]);
    let body = extrude(
        &Profile {
            plane: Frame::WORLD,
            loops: vec![outer, hole],
        },
        5.0,
        1,
    )
    .unwrap();
    assert!(check(&body).is_empty(), "{:?}", check(&body));
}
