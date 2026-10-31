//! Meshes enclose the right volume: within what the chord tolerance allows of the exact one.
use forge_math::{Point2, dmath};
use forge_mesh::{Quality, tessellate};
use forge_model::adversarial::fr;
use forge_model::{Combine, Extent, Feature, Part};
use forge_ops::Profile;

fn meshed_volume(part: &Part, chord: f64) -> f64 {
    let b = part.regenerate().expect("regenerates");
    let m = tessellate(
        &b,
        &Quality {
            chord,
            ..Quality::default()
        },
    )
    .expect("meshes");
    assert!(m.open_edges().is_empty(), "closed mesh");
    m.volume()
}

/// A blind pocket's wall is concave: chords across it cut into the cavity and add volume, so
/// a cylinder face meshed with too few points across u reads too big.
#[test]
fn blind_pocket_volume_is_exact() {
    let (w, h, t, r, depth) = (60.0, 40.0, 20.0, 9.5, 12.0);
    let part = Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![Profile::rectangle(Point2::ORIGIN, w, h, 0)],
                },
                extent: Extent::Distance(t),
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([30.0, 20.0, t - depth], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
                },
                extent: Extent::Distance(depth + 1.0),
                op: Combine::Cut,
            },
        ],
    };
    let exact = w * h * t - dmath::PI * r * r * depth;
    let m = forge_topo::mass::mass(&part.regenerate().expect("regenerates")).expect("mass properties");
    assert!(
        (m.volume - exact).abs() <= 1e-12 * exact,
        "exact {} vs {exact}",
        m.volume
    );
    for chord in [0.25, 0.01, 0.002] {
        let v = meshed_volume(&part, chord);
        // chords lie within `chord` of the surface: the error is at most chord times the
        // pocket's area (wall plus floor)
        let bound = chord * (2.0 * dmath::PI * r * depth + dmath::PI * r * r);
        assert!(
            (v - exact).abs() <= bound,
            "chord {chord}: {v} vs {exact} (bound {bound})"
        );
    }
}

/// Found by the OpenCascade oracle: a tilted blind cut and a vertical blind pocket meeting
/// inside a block (fuzz 1, features 0, 27 and 31). OCCT: 766093.2; Monte Carlo: 766082 +- 15.
#[test]
fn tilted_cut_and_pocket_volume_matches_occt() {
    let part = Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![Profile::rectangle(
                        Point2::ORIGIN,
                        130.15888135323598,
                        160.93659345227428,
                        0,
                    )],
                },
                extent: Extent::Distance(37.56688330880711),
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [28.453547351198445, 56.19712074992299, -7.7814610973627545],
                        [0.13699240887329533, 0.05492737968542511, 0.9890480589293855],
                        [0.45993563287222833, 0.8807814276562592, -0.11262011503379223],
                    ),
                    loops: vec![Profile::circle(Point2::ORIGIN, 13.232964346192585, 0)],
                },
                extent: Extent::Distance(36.7207502682546),
                op: Combine::Cut,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [47.48417098255251, 64.61262623133184, 12.119852949701473],
                        [0.0, 0.0, 1.0],
                        [-0.9658543583338399, -0.2590856199937125, 0.0],
                    ),
                    loops: vec![Profile::circle(Point2::ORIGIN, 9.979993718994757, 0)],
                },
                extent: Extent::Distance(19.364888029871924),
                op: Combine::Cut,
            },
        ],
    };
    let v = meshed_volume(&part, 0.002);
    assert!((v - 766093.2).abs() < 20.0, "{v}");
}
