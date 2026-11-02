//! Rounds whose ball rolls over a narrow strip with no edge of its own under it.

use forge_bool::{Op, boolean};
use forge_geom::CurveGeom;
use forge_math::{Frame, Point2, Point3, UnitVec3, Vec3};
use forge_mesh::{Quality, tessellate};
use forge_ops::{FilletEdge, Profile, Radius, extrude, fillet};

/// A boss standing over a block's rounded rim, not reaching the top, a round at its foot wider than
/// the rim's. Up the wall the ball rests on the wall, at the middle on the top beyond the rim, and in
/// between on the rim, where no edge of the boss's foot lies. Reference: a ball's closing on a grid
/// (research/fillets/closing.rs), which shares nothing with the kernel, gives 1100.8.
#[test]
fn foot_of_a_boss_over_a_rounded_rim() {
    let at = |z: f64| Frame::from_zx(Point3::new(0.0, 0.0, z), UnitVec3::Z, Vec3::X).expect("frame");
    let block = extrude(
        &Profile {
            plane: at(0.0),
            loops: vec![Profile::rectangle(Point2::new(-50.0, 0.0), 100.0, 30.0, 0)],
        },
        30.0,
        1,
    )
    .expect("block");
    let rim = block
        .edge_ids()
        .find(|&e| {
            let x = block.edge(e);
            [x.range.lo, x.range.hi].iter().all(|&t| {
                let p = x.curve.eval(t);
                (p.y - 30.0).abs() < 1e-9 && (p.z - 30.0).abs() < 1e-9
            })
        })
        .expect("the rim");
    let rounded = fillet(
        &block,
        &[FilletEdge {
            edge: rim,
            radius: Radius::Constant(3.0),
        }],
        2,
    )
    .expect("the rim's round");
    let boss = extrude(
        &Profile {
            plane: at(0.0),
            loops: vec![Profile::circle(Point2::new(0.0, 40.0), 12.0, 0)],
        },
        60.0,
        3,
    )
    .expect("boss");
    let body = boolean(&rounded, &boss, Op::Union, 4).expect("union");
    let foot: Vec<FilletEdge> = body
        .edge_ids()
        .filter(|&e| forge_ops::fillet_kind(&body, e) == Some(false))
        .map(|edge| FilletEdge {
            edge,
            radius: Radius::Constant(5.0),
        })
        .collect();
    assert_eq!(foot.len(), 3);
    let b = fillet(&body, &foot, 9).expect("rounds");
    assert!(forge_topo::check(&b).is_empty());
    let volume = |x: &forge_topo::Body| forge_topo::mass::mass(x).expect("mass").volume;
    let added = volume(&b) - volume(&body);
    assert!((added - 1100.8).abs() < 2.0, "added {added}");
    let fine = Quality {
        chord: 0.001,
        ..Default::default()
    };
    let mesh = tessellate(&b, &fine).expect("meshes");
    assert!(mesh.open_edges().is_empty());
    assert!((mesh.volume() - volume(&b)).abs() < 1e-4 * volume(&b));
}
