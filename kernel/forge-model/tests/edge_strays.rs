//! A traced curve that strays from its chain: an edge on it must be sound or refused.
use forge_bool::{Op, boolean};
use forge_math::{Frame, Point2, Point3, Vec2, Vec3, dmath};
use forge_mesh::tessellate::{Quality, tessellate};
use forge_ops::{Axis2, LoftGuides, LoftKind, Profile, ProfileLoop, Segment, loft, revolve};
use forge_topo::Body;

/// A segment of a loop: a line, or an arc from a to b about c, counter-clockwise or not.
#[derive(Clone, Copy)]
enum S {
    L([f64; 2], [f64; 2]),
    A([f64; 2], [f64; 2], [f64; 2], bool),
}
use S::{A, L};

const HUB: &[S] = &[
    L([118.000000, 16.000000], [118.514708, 20.470467]),
    L([118.514708, 20.470467], [63.014708, 26.860467]),
    A(
        [63.014708, 26.860467],
        [42.202627, 50.206240],
        [65.702627, 50.206240],
        false,
    ),
    L([42.202627, 50.206240], [42.200000, 66.000000]),
    L([42.200000, 66.000000], [37.700000, 66.000000]),
    L([37.700000, 66.000000], [37.702627, 50.206240]),
    A(
        [37.702627, 50.206240],
        [62.500000, 22.390000],
        [65.702627, 50.206240],
        true,
    ),
    L([62.500000, 22.390000], [118.000000, 16.000000]),
];
const ENVELOPE: &[S] = &[
    L([0.000000, -20.000000], [124.000000, -20.000000]),
    L([124.000000, -20.000000], [124.000000, 17.986874]),
    L([124.000000, 17.986874], [62.728759, 24.376874]),
    A(
        [62.728759, 24.376874],
        [39.702627, 50.206240],
        [65.702627, 50.206240],
        false,
    ),
    L([39.702627, 50.206240], [39.702627, 70.000000]),
    L([39.702627, 70.000000], [0.000000, 70.000000]),
    L([0.000000, 70.000000], [0.000000, -20.000000]),
];
const BLADE_0: &[S] = &[
    A(
        [3.681286, 28.966688],
        [4.028483, 24.480102],
        [3.854885, 26.723395],
        true,
    ),
    A(
        [4.028483, 24.480102],
        [64.437835, -52.501773],
        [8.565808, -34.152564],
        false,
    ),
    A(
        [64.437835, -52.501773],
        [68.713176, -53.905859],
        [66.575506, -53.203816],
        true,
    ),
    A(
        [68.713176, -53.905859],
        [3.681286, 28.966688],
        [8.565808, -34.152564],
        true,
    ),
];
const BLADE_1: &[S] = &[
    A(
        [6.269335, 31.588306],
        [6.262810, 27.088311],
        [6.266072, 29.338308],
        true,
    ),
    A(
        [6.262810, 27.088311],
        [65.131326, -53.223304],
        [6.173205, -34.708526],
        false,
    ),
    A(
        [65.131326, -53.223304],
        [69.424609, -54.571535],
        [67.277967, -53.897419],
        true,
    ),
    A(
        [69.424609, -54.571535],
        [6.269335, 31.588306],
        [6.173205, -34.708526],
        true,
    ),
];
const BLADE_2: &[S] = &[
    A(
        [24.632876, 32.995796],
        [22.913957, 28.837032],
        [23.773416, 30.916414],
        true,
    ),
    A(
        [22.913957, 28.837032],
        [60.826824, -68.183362],
        [-4.650311, -37.852159],
        false,
    ),
    A(
        [60.826824, -68.183362],
        [64.910004, -70.074828],
        [62.868414, -69.129095],
        true,
    ),
    A(
        [64.910004, -70.074828],
        [24.632876, 32.995796],
        [-4.650311, -37.852159],
        true,
    ),
];
const BLADE_3: &[S] = &[
    A(
        [53.565740, 19.950023],
        [49.941307, 17.282911],
        [51.753523, 18.616467],
        true,
    ),
    A(
        [49.941307, 17.282911],
        [48.676407, -91.827197],
        [-23.055457, -36.433233],
        false,
    ),
    A(
        [48.676407, -91.827197],
        [52.238035, -94.577617],
        [50.457221, -93.202407],
        true,
    ),
    A(
        [52.238035, -94.577617],
        [53.565740, 19.950023],
        [-23.055457, -36.433233],
        true,
    ),
];
const BLADE_Z: [f64; 4] = [4.0, 20.0, 28.0, 40.0];

fn outline(segs: &[S]) -> ProfileLoop {
    let at = |p: [f64; 2]| Point2::new(p[0], p[1]);
    ProfileLoop::new(
        segs.iter()
            .enumerate()
            .map(|(i, &s)| {
                let seg = match s {
                    L(a, b) => Segment::Line { a: at(a), b: at(b) },
                    A(a, b, c, ccw) => {
                        let start = dmath::atan2(a[1] - c[1], a[0] - c[0]);
                        let end = dmath::atan2(b[1] - c[1], b[0] - c[0]);
                        let mut sweep = end - start;
                        if ccw && sweep <= 0.0 {
                            sweep += dmath::TAU;
                        }
                        if !ccw && sweep >= 0.0 {
                            sweep -= dmath::TAU;
                        }
                        let radius = ((a[0] - c[0]) * (a[0] - c[0]) + (a[1] - c[1]) * (a[1] - c[1])).sqrt();
                        Segment::Arc {
                            center: at(c),
                            radius,
                            start,
                            sweep,
                        }
                    }
                };
                (seg, i as u64)
            })
            .collect(),
    )
}

fn turned(segs: &[S], feature: u64) -> Body {
    let plane = Frame::from_axes(Point3::ORIGIN, Vec3::X, Vec3::Z, -Vec3::Y).unwrap();
    let p = Profile {
        plane,
        loops: vec![outline(segs)],
    };
    let axis = Axis2 {
        origin: Point2::ORIGIN,
        dir: Vec2::new(0.0, 1.0),
    };
    revolve(&p, axis, dmath::TAU, feature).unwrap()
}

/// A blade trimmed to an envelope, joined to a hub. The curve where they meet passes a tip no chain
/// step draws, and the edge on it wanders off its face by half a millimetre between its 16 samples:
/// `check` passed it and nothing could mesh it. It refuses, or builds what meshes.
#[test]
fn an_edge_straying_from_its_chain_is_refused_not_left_to_the_mesher() {
    let sections: Vec<Profile> = [BLADE_0, BLADE_1, BLADE_2, BLADE_3]
        .iter()
        .zip(BLADE_Z)
        .map(|(segs, z)| Profile {
            plane: Frame::from_axes(Point3::new(0.0, 0.0, z), Vec3::X, Vec3::Y, Vec3::Z).unwrap(),
            loops: vec![outline(segs)],
        })
        .collect();
    let kind = LoftKind::Smooth {
        start: None,
        end: None,
    };
    let blade = loft(&sections, kind, &LoftGuides::None, 2).unwrap();
    let trimmed = boolean(&blade, &turned(ENVELOPE, 3), Op::Intersect, 4).unwrap();
    if let Ok(joined) = boolean(&turned(HUB, 1), &trimmed, Op::Union, 5) {
        let q = Quality {
            chord: 0.05,
            ..Default::default()
        };
        tessellate(&joined, &q).unwrap();
    }
}
