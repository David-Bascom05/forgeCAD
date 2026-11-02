//! A smooth loft through many sections, against its radius law integrated.
use forge_math::{Frame, Point2, Point3, Vec3, dmath};
use forge_ops::{LoftError, LoftGuides, LoftKind, Profile, loft};
use forge_topo::{check, mass::mass};

fn radius(z: f64) -> f64 {
    5.0 + dmath::sin(z / 4.0)
}

fn stack(n: usize) -> Vec<Profile> {
    (0..n)
        .map(|k| {
            let z = k as f64;
            Profile {
                plane: Frame::from_axes(Point3::new(0.0, 0.0, z), Vec3::X, Vec3::Y, Vec3::Z).unwrap(),
                loops: vec![Profile::circle(Point2::ORIGIN, radius(z), 0)],
            }
        })
        .collect()
}

#[test]
fn a_smooth_loft_through_thirty_sections() {
    let n = 30;
    let b = loft(
        &stack(n),
        LoftKind::Smooth {
            start: None,
            end: None,
        },
        &LoftGuides::None,
        1,
    )
    .unwrap();
    assert_eq!(check(&b), vec![]);
    // Simpson over the radius law, which the spline through 30 sections follows closely
    let m = 2000;
    let h = (n - 1) as f64 / m as f64;
    let area = |z: f64| dmath::PI * radius(z) * radius(z);
    let sum: f64 = (0..=m)
        .map(|i| {
            let w = if i == 0 || i == m {
                1.0
            } else if i % 2 == 1 {
                4.0
            } else {
                2.0
            };
            w * area(i as f64 * h)
        })
        .sum();
    let v = mass(&b).unwrap().volume;
    assert!((v - sum * h / 3.0).abs() < 2.0, "{v} not {}", sum * h / 3.0);
}

#[test]
fn past_the_most_a_loft_says_so() {
    let n = forge_geom::skinned::MAX_SECTIONS + 1;
    let got = loft(
        &stack(n),
        LoftKind::Smooth {
            start: None,
            end: None,
        },
        &LoftGuides::None,
        1,
    );
    assert_eq!(
        got.unwrap_err(),
        LoftError::TooManySections {
            sections: n,
            most: n - 1
        }
    );
}
