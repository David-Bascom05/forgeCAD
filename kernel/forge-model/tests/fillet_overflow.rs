//! Rounds that stopped short at a vertex where the geometry carries on.

use forge_bool::{Op, boolean};
use forge_geom::CurveGeom;
use forge_math::{Frame, Point2, Point3, UnitVec3, Vec2, Vec3};
use forge_mesh::{Quality, tessellate};
use forge_ops::{
    Axis2, FilletEdge, LoftGuides, LoftKind, Profile, ProfileLoop, Radius, Segment, extrude, fillet, loft,
    revolve,
};
use forge_topo::{Body, EdgeId};
use std::f64::consts::{PI, TAU};

fn line(a: (f64, f64), b: (f64, f64), id: u64) -> (Segment, u64) {
    (
        Segment::Line {
            a: Point2::new(a.0, a.1),
            b: Point2::new(b.0, b.1),
        },
        id,
    )
}

/// A slot standing off centre on a cone, its flat walls tangent to its round ends.
fn slot_on_a_cone() -> Body {
    let (w, h) = (12.0, 4.0);
    let arc = |x: f64, start: f64, id| {
        (
            Segment::Arc {
                center: Point2::new(x, 0.0),
                radius: h,
                start,
                sweep: PI,
            },
            id,
        )
    };
    let slot = ProfileLoop::new(vec![
        line((-w, -h), (w, -h), 0),
        arc(w, -PI / 2.0, 1),
        line((w, h), (-w, h), 2),
        arc(-w, PI / 2.0, 3),
    ]);
    let boss = extrude(
        &Profile {
            plane: Frame::from_zx(Point3::new(9.0, 7.0, 0.0), UnitVec3::Z, Vec3::X).expect("frame"),
            loops: vec![slot],
        },
        60.0,
        1,
    )
    .expect("boss");
    let hub = revolve(
        &Profile {
            plane: Frame::from_zx(Point3::ORIGIN, -UnitVec3::Y, Vec3::X).expect("frame"),
            loops: vec![ProfileLoop::new(vec![
                line((0.0, 0.0), (70.0, 0.0), 0),
                line((70.0, 0.0), (70.0, 10.0), 1),
                line((70.0, 10.0), (0.0, 30.0), 2),
                line((0.0, 30.0), (0.0, 0.0), 3),
            ])],
        },
        Axis2 {
            origin: Point2::new(0.0, 0.0),
            dir: Vec2::new(0.0, 1.0),
        },
        TAU,
        2,
    )
    .expect("hub");
    boolean(&hub, &boss, Op::Union, 3).expect("union")
}

/// The rim at the slot's foot, all its pieces: those between the boss's walls and the cone.
fn rim(b: &Body) -> Vec<EdgeId> {
    b.edge_ids()
        .filter(|&e| {
            let x = b.edge(e);
            let faces: Vec<_> = x.coedges.iter().map(|&c| b.coedge_face(c)).collect();
            let cone = |f| matches!(b.face(f).surface, forge_geom::Surface::Cone(_));
            let m = x.curve.eval(x.range.mid());
            let near = (m.x - 9.0).abs() < 20.0 && (m.y - 7.0).abs() < 10.0;
            near && faces.len() == 2 && faces.iter().filter(|&&f| cone(f)).count() == 1
        })
        .collect()
}

/// The rim's pieces are tangent where a flat wall meets a round end, but they are traced curves, whose
/// end tangents are only as good as the trace. One piece picked rounds the whole rim, exactly as all of
/// them picked do.
#[test]
fn a_traced_rim_rounds_as_a_chain_from_one_pick() {
    let b = slot_on_a_cone();
    let edges = rim(&b);
    assert_eq!(edges.len(), 4, "rim pieces");
    let round = |list: &[EdgeId]| {
        let picks: Vec<FilletEdge> = list
            .iter()
            .map(|&edge| FilletEdge {
                edge,
                radius: Radius::Constant(1.5),
            })
            .collect();
        fillet(&b, &picks, 9).expect("rounds")
    };
    let all = round(&edges);
    assert!(forge_topo::check(&all).is_empty());
    let v_all = forge_topo::mass::mass(&all).expect("mass").volume;
    let mesh = tessellate(&all, &Quality::default()).expect("meshes");
    assert!(mesh.open_edges().is_empty());
    assert!((mesh.volume() - v_all).abs() < 2e-3 * v_all);
    for &one in &edges {
        let v = forge_topo::mass::mass(&round(&[one])).expect("mass").volume;
        assert!(
            (v - v_all).abs() < 1e-9 * v_all,
            "picking {one:?}: {v}, all {v_all}"
        );
    }
}

/// A lofted boss lying across a cone and running out over its rim. The round at its foot ends on the
/// disc's outer wall carried up over the boss, whose traced edge with that wall has to be walked on
/// past the vertex to where the round's spring reaches it.
#[test]
fn a_round_ends_on_a_traced_edge_carried_past_its_vertex() {
    let hub = revolve(
        &Profile {
            plane: Frame::from_zx(Point3::ORIGIN, -UnitVec3::Y, Vec3::X).expect("frame"),
            loops: vec![ProfileLoop::new(vec![
                line((0.0, 0.0), (50.0, 0.0), 0),
                line((50.0, 0.0), (50.0, 10.0), 1),
                line((50.0, 10.0), (0.0, 30.0), 2),
                line((0.0, 30.0), (0.0, 0.0), 3),
            ])],
        },
        Axis2 {
            origin: Point2::new(0.0, 0.0),
            dir: Vec2::new(0.0, 1.0),
        },
        TAU,
        1,
    )
    .expect("hub");
    let section = |x: f64, r: f64| Profile {
        plane: Frame::from_zx(Point3::new(x, 0.0, 14.0), UnitVec3::X, -Vec3::Z).expect("frame"),
        loops: vec![Profile::circle(Point2::new(0.0, 0.0), r, 0)],
    };
    let boss = loft(
        &[section(20.0, 8.0), section(60.0, 11.0), section(70.0, 12.0)],
        LoftKind::Smooth {
            start: None,
            end: None,
        },
        &LoftGuides::None,
        2,
    )
    .expect("loft");
    let b = boolean(&hub, &boss, Op::Union, 3).expect("union");
    assert!(forge_topo::check(&b).is_empty());
    let foot: Vec<EdgeId> = b
        .edge_ids()
        .filter(|&e| {
            let x = b.edge(e);
            let kinds: Vec<bool> = x
                .coedges
                .iter()
                .map(|&c| matches!(b.face(b.coedge_face(c)).surface, forge_geom::Surface::Cone(_)))
                .collect();
            let other: Vec<bool> = x
                .coedges
                .iter()
                .map(|&c| {
                    matches!(
                        b.face(b.coedge_face(c)).surface,
                        forge_geom::Surface::Procedural(_)
                    )
                })
                .collect();
            kinds.contains(&true) && other.contains(&true)
        })
        .collect();
    assert!(!foot.is_empty());
    let picks: Vec<FilletEdge> = foot
        .iter()
        .map(|&edge| FilletEdge {
            edge,
            radius: Radius::Constant(2.0),
        })
        .collect();
    let done = fillet(&b, &picks, 9).expect("rounds");
    assert!(forge_topo::check(&done).is_empty());
    let (v0, v) = (
        forge_topo::mass::mass(&b).expect("mass").volume,
        forge_topo::mass::mass(&done).expect("mass").volume,
    );
    let mesh = tessellate(&done, &Quality::default()).expect("meshes");
    assert!(mesh.open_edges().is_empty());
    assert!(
        (mesh.volume() - v).abs() < 2e-3 * v,
        "mesh {} mass {v}",
        mesh.volume()
    );
    assert!(v > v0);
}

/// A thin blade with rounded corners standing on a disc and running out over its rim. At the blade's
/// narrow end the rounds on its two sides mitre past the end's corner, one reaching further up it than
/// the other: the nearer's spring carries on past the corner on its surface carried on, to meet the
/// other blend, where it is off its face as that was and meant to be.
#[test]
fn a_mitre_carries_a_spring_past_its_corner() {
    let hub = revolve(
        &Profile {
            plane: Frame::from_zx(Point3::ORIGIN, -UnitVec3::Y, Vec3::X).expect("frame"),
            loops: vec![ProfileLoop::new(vec![
                line((0.0, -5.0), (86.0, -5.0), 0),
                line((86.0, -5.0), (86.0, 2.0), 1),
                line((86.0, 2.0), (30.0, 20.0), 2),
                line((30.0, 20.0), (0.0, 20.0), 3),
                line((0.0, 20.0), (0.0, -5.0), 4),
            ])],
        },
        Axis2 {
            origin: Point2::new(0.0, 0.0),
            dir: Vec2::new(0.0, 1.0),
        },
        TAU,
        1,
    )
    .expect("hub");
    let (a, b, r) = (31.0 - 1.9, 2.0 - 1.9, 1.9);
    let arc = |x: f64, y: f64, start: f64, id| {
        (
            Segment::Arc {
                center: Point2::new(x, y),
                radius: r,
                start,
                sweep: PI / 2.0,
            },
            id,
        )
    };
    let outline = ProfileLoop::new(vec![
        line((-a, -2.0), (a, -2.0), 0),
        arc(a, -b, -PI / 2.0, 1),
        line((31.0, -b), (31.0, b), 2),
        arc(a, b, 0.0, 3),
        line((a, 2.0), (-a, 2.0), 4),
        arc(-a, b, PI / 2.0, 5),
        line((-31.0, b), (-31.0, -b), 6),
        arc(-a, -b, PI, 7),
    ]);
    let x = Vec3::new(1.0, 1.0, 0.0);
    let centre = Point3::ORIGIN + x.normalize().expect("unit").get() * 56.0;
    let blade = extrude(
        &Profile {
            plane: Frame::from_zx(centre, UnitVec3::Z, x).expect("frame"),
            loops: vec![outline],
        },
        60.0,
        2,
    )
    .expect("blade");
    let joined = boolean(&hub, &blade, Op::Union, 3).expect("union");
    let root: Vec<FilletEdge> = joined
        .edge_ids()
        .filter(|&e| forge_ops::fillet_kind(&joined, e) == Some(false))
        .map(|edge| FilletEdge {
            edge,
            radius: Radius::Constant(1.0),
        })
        .collect();
    let done = fillet(&joined, &root, 9).expect("rounds");
    assert!(forge_topo::check(&done).is_empty());
    let v = forge_topo::mass::mass(&done).expect("mass").volume;
    let mesh = tessellate(&done, &Quality::default()).expect("meshes");
    assert!(mesh.open_edges().is_empty());
    assert!(
        (mesh.volume() - v).abs() < 2e-3 * v,
        "mesh {}, mass {v}",
        mesh.volume()
    );
    let before = forge_topo::mass::mass(&joined).expect("mass").volume;
    assert!(v > before);
}

/// A tube revolved part way round a cylinder, sunk into its wall. The rounds along the two rims turn the
/// tube's two flat ends, each end's new corners named from the same faces. Along the rims what they add
/// is the area of the section between wall, tube and ball, by Pappus, and along the ends a quarter
/// round's: integrated here on a grid, sharing nothing with the kernel, and within 1% of the whole, the
/// rest being the corners.
#[test]
fn rounds_ending_on_one_flat_end_are_told_apart() {
    let (wall, major, tube, z0, r) = (44.0, 55.0, 12.0, 16.0, 2.0);
    let (height, turn) = (50.0, 200f64.to_radians());
    let drum = extrude(
        &Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::circle(Point2::new(0.0, 0.0), wall, 0)],
        },
        height,
        1,
    )
    .expect("drum");
    let ring = revolve(
        &Profile {
            plane: Frame::from_zx(Point3::ORIGIN, -UnitVec3::Y, Vec3::X).expect("frame"),
            loops: vec![Profile::circle(Point2::new(major, z0), tube, 0)],
        },
        Axis2 {
            origin: Point2::new(0.0, 0.0),
            dir: Vec2::new(0.0, 1.0),
        },
        turn,
        2,
    )
    .expect("ring");
    let joined = boolean(&drum, &ring, Op::Union, 3).expect("union");
    let rims: Vec<FilletEdge> = joined
        .edge_ids()
        .filter(|&e| forge_ops::fillet_kind(&joined, e) == Some(false))
        .map(|edge| FilletEdge {
            edge,
            radius: Radius::Constant(r),
        })
        .collect();
    let done = fillet(&joined, &rims, 9).expect("rounds");
    assert!(forge_topo::check(&done).is_empty());
    let added = forge_topo::mass::mass(&done).expect("mass").volume
        - forge_topo::mass::mass(&joined).expect("mass").volume;
    // what is filled: below the ball's centre, on the wall's side of the line from it to the tube's
    // centre (on which it touches the tube), outside tube and ball
    let (cr, cz) = (
        wall + r,
        z0 + ((tube + r) * (tube + r) - (major - wall - r) * (major - wall - r)).sqrt(),
    );
    let side = |x: f64, z: f64| (major - cr) * (z - cz) - (z0 - cz) * (x - cr);
    let corner_side = side(wall, z0 + (tube * tube - (major - wall) * (major - wall)).sqrt());
    let (dr, dz) = (2.0 * r / 4000.0, (cz - z0) / 8000.0);
    let mut moment = 0.0;
    for i in 0..4000 {
        let x = wall + (i as f64 + 0.5) * dr;
        for j in 0..8000 {
            let z = z0 + (j as f64 + 0.5) * dz;
            let off_tube = (x - major) * (x - major) + (z - z0) * (z - z0) > tube * tube;
            let off_ball = (x - cr) * (x - cr) + (z - cz) * (z - cz) > r * r;
            if off_tube && off_ball && z < cz && side(x, z) * corner_side > 0.0 {
                moment += x * dr * dz;
            }
        }
    }
    let chord = 2.0 * (tube * tube - (major - wall) * (major - wall)).sqrt();
    let want = 2.0 * turn * moment + 2.0 * chord * r * r * (1.0 - std::f64::consts::FRAC_PI_4);
    assert!(
        (added - want).abs() < 1e-2 * want,
        "added {added}, reference {want}"
    );
}
