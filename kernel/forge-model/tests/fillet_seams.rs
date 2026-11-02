//! Rounds whose springs cross the line where two faces of one surface meet.

use forge_bool::{Op, boolean};
use forge_math::{Frame, Point2, Point3, Vec2, Vec3};
use forge_mesh::{Quality, tessellate};
use forge_ops::{
    Axis2, FilletEdge, LoftGuides, LoftKind, Profile, ProfileLoop, Radius, Segment, fillet, fillet_kind,
    loft, revolve,
};
use forge_topo::Body;
use std::f64::consts::{FRAC_PI_2, TAU};

fn line(a: (f64, f64), b: (f64, f64), id: u64) -> (Segment, u64) {
    (
        Segment::Line {
            a: Point2::new(a.0, a.1),
            b: Point2::new(b.0, b.1),
        },
        id,
    )
}

/// An impeller's hub, revolved from an arc: a torus's two half-turn faces, parted at the azimuth of `x`
/// and opposite, with up the axis what `normal` x `x` says.
fn hub(normal: Vec3, x: Vec3) -> Body {
    let plane = Frame::from_zx(Point3::ORIGIN, normal.normalize().expect("unit"), x).expect("frame");
    let section = ProfileLoop::new(vec![
        line((6.0, 0.0), (50.0, 0.0), 0),
        line((50.0, 0.0), (50.0, 4.0), 1),
        (
            Segment::Arc {
                center: Point2::new(50.0, 38.0),
                radius: 34.0,
                start: -FRAC_PI_2,
                sweep: -FRAC_PI_2,
            },
            2,
        ),
        line((16.0, 38.0), (16.0, 46.0), 3),
        line((16.0, 46.0), (6.0, 46.0), 4),
        line((6.0, 46.0), (6.0, 0.0), 5),
    ]);
    let axis = Axis2 {
        origin: Point2::ORIGIN,
        dir: Vec2::Y,
    };
    let plane_profile = Profile {
        plane,
        loops: vec![section],
    };
    revolve(&plane_profile, axis, TAU, 1).expect("hub")
}

/// A blade lofted through twisted, tapering sections, its inner end a little off the hub's azimuth 0.
fn blade() -> Body {
    let section = |len: f64, m: [[f64; 4]; 3]| {
        let col = |k: usize| Vec3::new(m[0][k], m[1][k], m[2][k]);
        Profile {
            plane: Frame::from_axes(Point3::new(m[0][3], m[1][3], m[2][3]), col(0), col(1), col(2))
                .expect("frame"),
            loops: vec![ProfileLoop::new(vec![
                line((-2.0, -1.2), (len, -0.6), 0),
                line((len, -0.6), (len, 0.6), 1),
                line((len, 0.6), (-2.0, 1.2), 2),
                line((-2.0, 1.2), (-2.0, -1.2), 3),
            ])],
        }
    };
    let sections = [
        section(
            18.64,
            [
                [
                    0.8481127911179839,
                    -0.5277477641975217,
                    0.0467652747942349,
                    15.921588353457905,
                ],
                [
                    -0.5285422726207423,
                    -0.8488881194132837,
                    0.005659220076976541,
                    1.9267238963888775,
                ],
                [
                    0.03671184543119741,
                    -0.029517081554513942,
                    -0.99888987496197,
                    36.39838067587216,
                ],
            ],
        ),
        section(
            14.224,
            [
                [
                    0.8385460937405811,
                    0.028820436248521042,
                    0.5440678552596704,
                    20.852722840972767,
                ],
                [
                    -0.106545987399428,
                    -0.9706446535800789,
                    0.21563141943021577,
                    8.264598214569702,
                ],
                [
                    0.5343111464694459,
                    -0.23878513130187484,
                    -0.8108595808323737,
                    18.101814363132668,
                ],
            ],
        ),
        section(
            7.6,
            [
                [
                    0.08080024260490012,
                    0.6884819278648037,
                    0.720738479476816,
                    34.219763446172216,
                ],
                [
                    0.0289417988664519,
                    -0.7244184962984846,
                    0.6887526511738566,
                    32.70111624011256,
                ],
                [
                    0.9963100386292226,
                    -0.03479191320128538,
                    -0.0784590957278447,
                    4.104810653073649,
                ],
            ],
        ),
    ];
    let kind = LoftKind::Smooth {
        start: None,
        end: None,
    };
    loft(&sections, kind, &LoftGuides::None, 2).expect("blade")
}

fn rooted(hub: &Body, blade: &Body) -> Body {
    let joined = boolean(hub, blade, Op::Union, 3).expect("union");
    let root: Vec<FilletEdge> = joined
        .edge_ids()
        .filter(|&e| fillet_kind(&joined, e) == Some(false))
        .map(|edge| FilletEdge {
            edge,
            radius: Radius::Constant(1.2),
        })
        .collect();
    fillet(&joined, &root, 9).expect("rounds")
}

/// The round at a twisted blade's root reaches over the hub's azimuth 0, past the corner of the blade
/// that stands just short of it. Turning the hub about its axis moves that line clear and changes no
/// point of the part, so the hub turned a quarter is the reference for the volume.
#[test]
fn a_round_crosses_the_seam_of_a_revolved_hub() {
    let b = blade();
    let at_seam = rooted(&hub(-Vec3::Y, Vec3::X), &b);
    let clear = rooted(&hub(Vec3::X, Vec3::Y), &b);
    assert!(forge_topo::check(&at_seam).is_empty());
    assert!(forge_topo::check(&clear).is_empty());
    let volume = |body: &Body| forge_topo::mass::mass(body).expect("mass").volume;
    let (v, want) = (volume(&at_seam), volume(&clear));
    assert!((v - want).abs() < 1e-7 * want, "volume {v}, hub turned {want}");
    let mesh = tessellate(&at_seam, &Quality::default()).expect("meshes");
    assert!(mesh.open_edges().is_empty());
    assert!(
        (mesh.volume() - v).abs() < 2e-3 * v,
        "mesh {}, mass {v}",
        mesh.volume()
    );
}
