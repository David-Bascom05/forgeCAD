//! shape.rs's screen once took an arc within 1e-12 rad of a whole turn as whole. That is sound
//! only for a whole circle alone in its loop: with another segment closing the profile, the ray's
//! parity counted the arc across its gap and the closing segment both, and the disc's centre read
//! Out at any radius. Two guards, each enough: the profile's refusal (`Segment::degenerate`,
//! forge-ops profile.rs: an arc within `ANGULAR` = 1e-11 rad of whole), at any radius, so no
//! shape holds such an arc; and the screen itself, which now takes only an exact whole turn as
//! whole. Both held at 1 mm up to `InputSize`'s 1.6e7.

use forge_bool::{Member, Seg2, Shape};
use forge_math::{Frame, Point2, Point3};
use forge_ops::{Profile, ProfileLoop, Segment};
use std::f64::consts::TAU;

/// An arc of radius `r` from angle 0 through `TAU - g`, closed by a line from its end.
fn profile(r: f64, g: f64) -> (Profile, Point2) {
    let e = Point2::new(
        r * forge_math::dmath::cos(TAU - g),
        r * forge_math::dmath::sin(TAU - g),
    );
    let segs = vec![
        (
            Segment::Arc {
                center: Point2::ORIGIN,
                radius: r,
                start: 0.0,
                sweep: TAU - g,
            },
            1,
        ),
        (
            Segment::Line {
                a: e,
                b: Point2::new(r, 0.0),
            },
            2,
        ),
    ];
    (
        Profile {
            plane: Frame::WORLD,
            loops: vec![ProfileLoop::new(segs)],
        },
        e,
    )
}

#[test]
fn a_profile_refuses_an_arc_a_hair_short_of_whole_at_any_size() {
    for r in [1.0, 5e5, 1.2e7, 1.6e7] {
        for g in [1e-12, 5e-12] {
            assert!(profile(r, g).0.normalized().is_err(), "r {r:e} g {g:e}");
        }
    }
}

#[test]
fn the_screen_answers_an_arc_the_profile_admits_exactly() {
    // short of whole by twice ANGULAR: admitted, and the disc's centre, in the gap's band (the ray
    // crosses the closing line, not the arc), is In
    for r in [1.2e7, 1.6e7] {
        let (p, e) = profile(r, 2e-11);
        let p = p
            .normalized()
            .expect("an arc 2e-11 rad short of whole is a profile");
        let segs: std::sync::Arc<[Seg2]> = p.loops[0]
            .segments
            .iter()
            .map(|(s, _)| match *s {
                Segment::Arc {
                    center,
                    radius,
                    start,
                    sweep,
                } => Seg2::Arc {
                    c: center,
                    r: radius,
                    start,
                    sweep,
                },
                Segment::Line { a, b } => Seg2::Line { a, b },
                _ => unreachable!(),
            })
            .collect();
        let sh = Shape::Prism {
            frame: Frame::WORLD,
            segs,
            lo: 0.0,
            hi: 1.0,
        };
        assert_eq!(
            sh.member(Point3::new(0.0, e.y / 2.0, 0.5), 1e-5),
            Member::In,
            "r {r:e}"
        );
    }
}

#[test]
fn the_screen_takes_only_an_exact_whole_turn_as_whole() {
    // a shape built by hand past the profile's refusal: never Out at the disc's centre (In, or
    // Near where the ray grazes the arc's ends)
    for r in [1.0, 1e3, 1e5, 5e5, 1.2e7, 1.6e7] {
        for g in [1e-12, 5e-13, 1e-13] {
            let e = Point2::new(
                r * forge_math::dmath::cos(TAU - g),
                r * forge_math::dmath::sin(TAU - g),
            );
            let segs: std::sync::Arc<[Seg2]> = vec![
                Seg2::Arc {
                    c: Point2::ORIGIN,
                    r,
                    start: 0.0,
                    sweep: TAU - g,
                },
                Seg2::Line {
                    a: e,
                    b: Point2::new(r, 0.0),
                },
            ]
            .into();
            let sh = Shape::Prism {
                frame: Frame::WORLD,
                segs,
                lo: 0.0,
                hi: 1.0,
            };
            let m = sh.member(Point3::new(0.0, e.y / 2.0, 0.5), 1e-5);
            assert_ne!(m, Member::Out, "r {r:e} g {g:e}");
        }
    }
}
