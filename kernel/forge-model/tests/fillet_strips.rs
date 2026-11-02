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

/// The same over a narrower rim and a smaller ball, the boss almost reaching the rim's far edge.
/// Reference as above: the closing's mean over several grid steps stood 1.5% over the exact volume
/// of the scene above, and stands 1.3% over the number below (840.1).
#[test]
fn walk_over_a_rim_narrower_than_the_ball() {
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
            radius: Radius::Constant(2.0),
        }],
        2,
    )
    .expect("the rim's round");
    let boss = extrude(
        &Profile {
            plane: at(0.0),
            loops: vec![Profile::circle(Point2::new(0.0, 40.0), 11.5, 0)],
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
            radius: Radius::Constant(4.0),
        })
        .collect();
    let b = fillet(&body, &foot, 9).expect("rounds");
    assert!(forge_topo::check(&b).is_empty());
    let volume = |x: &forge_topo::Body| forge_topo::mass::mass(x).expect("mass").volume;
    let added = volume(&b) - volume(&body);
    assert!((added - 829.6546).abs() < 0.01, "added {added}");
    let fine = Quality {
        chord: 0.001,
        ..Default::default()
    };
    let mesh = tessellate(&b, &fine).expect("meshes");
    assert!(mesh.open_edges().is_empty());
    assert!((mesh.volume() - volume(&b)).abs() < 1e-4 * volume(&b));
}

/// A rectangular boss's foot rounded all round, the mitres at its corners running up the corners'
/// edges, one of which a vertex splits nearer the plate than the radius. The result is that of the
/// edge whole.
#[test]
fn mitre_past_a_stray_vertex_on_the_sharp_edge() {
    let at = |z: f64| Frame::from_zx(Point3::new(0.0, 0.0, z), UnitVec3::Z, Vec3::X).expect("frame");
    let plate = extrude(
        &Profile {
            plane: at(0.0),
            loops: vec![Profile::rectangle(Point2::new(-40.0, -40.0), 80.0, 80.0, 0)],
        },
        5.0,
        1,
    )
    .expect("plate");
    let boss = extrude(
        &Profile {
            plane: at(5.0),
            loops: vec![Profile::rectangle(Point2::new(-10.0, -10.0), 20.0, 20.0, 0)],
        },
        10.0,
        2,
    )
    .expect("boss");
    let whole = boolean(&plate, &boss, Op::Union, 3).expect("union");
    let foot = |b: &forge_topo::Body| -> Vec<FilletEdge> {
        b.edge_ids()
            .filter(|&e| {
                let x = b.edge(e);
                (x.curve.eval(x.range.mid()).z - 5.0).abs() < 1e-9
                    && forge_ops::fillet_kind(b, e) == Some(false)
            })
            .map(|edge| FilletEdge {
                edge,
                radius: Radius::Constant(3.0),
            })
            .collect()
    };
    let want = fillet(&whole, &foot(&whole), 9).expect("rounds whole");
    let mut split = whole.clone();
    let up = split
        .edge_ids()
        .find(|&e| {
            let x = split.edge(e);
            let (p, q) = (x.curve.eval(x.range.lo), x.curve.eval(x.range.hi));
            (p.z - q.z).abs() > 9.0 && p.x > 9.0 && p.y > 9.0 && p.x < 11.0
        })
        .expect("the corner's edge");
    let x = split.edge(up).clone();
    let rising = x.curve.deriv(x.range.mid()).d1.z > 0.0;
    let cut = if rising {
        x.range.lo + 1.0
    } else {
        x.range.hi - 1.0
    };
    let m = split.add_vertex(x.curve.eval(cut), x.name.with_str("half"));
    let other = split.add_edge(
        x.curve.clone(),
        forge_math::Interval::new(cut, x.range.hi),
        m,
        x.end,
        x.name.with_str("half"),
    );
    {
        let edge = &mut split.edges[up.index()];
        edge.range.hi = cut;
        edge.end = m;
        edge.bbox.clear();
    }
    for c in x.coedges.iter().copied().collect::<Vec<_>>() {
        let (f, lp) = (split.coedge_face(c), split.coedge(c).lp);
        let uses: Vec<_> = split
            .lp(lp)
            .coedges()
            .flat_map(|d| {
                let co = split.coedge(d);
                match (co.edge == up, co.forward) {
                    (true, true) => vec![(up, true), (other, true)],
                    (true, false) => vec![(other, false), (up, false)],
                    _ => vec![(co.edge, co.forward)],
                }
            })
            .collect();
        split.replace_loop(f, lp, &uses);
    }
    let got = fillet(&split, &foot(&split), 9).expect("rounds split");
    assert!(forge_topo::check(&got).is_empty());
    let volume = |x: &forge_topo::Body| forge_topo::mass::mass(x).expect("mass").volume;
    assert!((volume(&got) - volume(&want)).abs() < 1e-9 * volume(&want));
}

/// Every edge of a disc picked at once, its seam among them: the seam has nothing to round, and the
/// two rims round as they would alone. The volume taken has a closed form (Pappus).
#[test]
fn a_seam_picked_with_the_rest_is_left_alone() {
    let (big, h, r) = (20.0, 10.0, 2.0);
    let at = Frame::from_zx(Point3::ORIGIN, UnitVec3::Z, Vec3::X).expect("frame");
    let disc = extrude(
        &Profile {
            plane: at,
            loops: vec![Profile::circle(Point2::ORIGIN, big, 0)],
        },
        h,
        1,
    )
    .expect("disc");
    let all: Vec<FilletEdge> = disc
        .edge_ids()
        .map(|edge| FilletEdge {
            edge,
            radius: Radius::Constant(r),
        })
        .collect();
    assert!(all.len() > 2);
    let b = fillet(&disc, &all, 9).expect("rounds");
    assert!(forge_topo::check(&b).is_empty());
    let volume = |x: &forge_topo::Body| forge_topo::mass::mass(x).expect("mass").volume;
    let pi = std::f64::consts::PI;
    let area = r * r * (1.0 - pi / 4.0);
    let moment = r * r * r * (5.0 / 6.0 - pi / 4.0);
    let taken = 2.0 * std::f64::consts::TAU * (big * area - moment);
    let got = volume(&disc) - volume(&b);
    assert!((got - taken).abs() < 1e-6 * taken, "took {got}, want {taken}");
}

/// A rib with a rounded nose standing on a cone, the foot rounded with the nose's radius: along the
/// flat the ball rests on the flat, round the nose the arc is the ball's own, and the supports change
/// where no edge of the foot does. Reference: a ball's closing on a grid, sharing nothing with the
/// kernel.
#[test]
fn foot_of_a_rib_on_a_cone() {
    use forge_math::Vec2;
    use forge_ops::{Axis2, ProfileLoop, Segment, revolve};
    let line = |a: (f64, f64), b: (f64, f64), id| {
        (
            Segment::Line {
                a: Point2::new(a.0, a.1),
                b: Point2::new(b.0, b.1),
            },
            id,
        )
    };
    let plane = Frame::from_zx(Point3::ORIGIN, -UnitVec3::Y, Vec3::X).expect("frame");
    let hub = revolve(
        &Profile {
            plane,
            loops: vec![ProfileLoop::new(vec![
                line((0.0, -5.0), (84.0, -5.0), 0),
                line((84.0, -5.0), (84.0, 2.0), 1),
                line((84.0, 2.0), (30.0, 20.0), 2),
                line((30.0, 20.0), (0.0, 20.0), 3),
                line((0.0, 20.0), (0.0, -5.0), 4),
            ])],
        },
        Axis2 {
            origin: Point2::ORIGIN,
            dir: Vec2::new(0.0, 1.0),
        },
        std::f64::consts::TAU,
        1,
    )
    .expect("hub");
    let pi = std::f64::consts::PI;
    let (x0, x1, h) = (35.0, 75.0, 2.0);
    let rib = extrude(
        &Profile {
            plane: Frame::from_zx(Point3::ORIGIN, UnitVec3::Z, Vec3::new(1.0, 1.0, 0.0)).expect("frame"),
            loops: vec![ProfileLoop::new(vec![
                line((x0 + h, -h), (x1 - h, -h), 0),
                (
                    Segment::Arc {
                        center: Point2::new(x1 - h, 0.0),
                        radius: h,
                        start: -pi / 2.0,
                        sweep: pi,
                    },
                    1,
                ),
                line((x1 - h, h), (x0 + h, h), 2),
                (
                    Segment::Arc {
                        center: Point2::new(x0 + h, 0.0),
                        radius: h,
                        start: pi / 2.0,
                        sweep: pi,
                    },
                    3,
                ),
            ])],
        },
        30.0,
        2,
    )
    .expect("rib");
    let body = boolean(&hub, &rib, Op::Union, 3).expect("union");
    let foot: Vec<FilletEdge> = body
        .edge_ids()
        .filter(|&e| forge_ops::fillet_kind(&body, e) == Some(false))
        .map(|edge| FilletEdge {
            edge,
            radius: Radius::Constant(2.0),
        })
        .collect();
    let b = fillet(&body, &foot, 9).expect("rounds");
    assert!(forge_topo::check(&b).is_empty());
    let volume = |x: &forge_topo::Body| forge_topo::mass::mass(x).expect("mass").volume;
    // the closing's mean over eight grid steps is 79.6, spread 3.6
    let added = volume(&b) - volume(&body);
    assert!((75.0..80.0).contains(&added), "added {added}");
    assert!(
        tessellate(&b, &Quality::default())
            .expect("meshes")
            .open_edges()
            .is_empty()
    );
}
