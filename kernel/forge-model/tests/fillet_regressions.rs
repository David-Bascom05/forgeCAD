//! Fillets the random stress test found leaving open meshes.

use forge_geom::CurveGeom;
use forge_math::Point3;
use forge_mesh::{Quality, tessellate};
use forge_ops::{FilletEdge, Radius, fillet};

/// A fillet running out where a hole breaks through the wall it rounds: the run-out meets the
/// hole's cylinder nearly tangentially, and both faces once cut the same sliver triangle there.
#[test]
#[allow(clippy::excessive_precision, clippy::unreadable_literal)]
fn fillet_run_out_into_a_hole_is_watertight() {
    let body = forge_model::suite::fuzz(140, 4)
        .regenerate()
        .expect("regenerates");
    let corner = Point3::new(111.08863288027584, 0.0, 16.065239812785975);
    let edge = body
        .edge_ids()
        .find(|&e| {
            let ed = body.edge(e);
            ed.curve.eval(ed.range.lo).distance(corner) < 1e-6 && ed.curve.eval(ed.range.hi).y > 20.0
        })
        .expect("the top edge running into the hole");
    let b = fillet(
        &body,
        &[FilletEdge {
            edge,
            radius: Radius::Constant(0.1362436110600404),
        }],
        999,
    )
    .expect("fillets");
    assert!(forge_topo::check(&b).is_empty());
    let mesh = tessellate(&b, &Quality::default()).expect("tessellates");
    assert!(mesh.open_edges().is_empty(), "open mesh");
}

/// A hole's rim rounded beside a variable-radius round of the wall's top edge: the rim's blend
/// reaches past where the other blend trims the wall, so the two springs cross on it. Refused
/// (the blends would cut into each other), never built with crossing loops.
#[test]
#[allow(clippy::excessive_precision, clippy::unreadable_literal)]
fn crossing_springs_on_one_face_are_refused() {
    let body = forge_model::suite::fuzz(22, 4).regenerate().expect("regenerates");
    let edge_from = |a: Point3, b: Point3| {
        body.edge_ids()
            .find(|&e| {
                let ed = body.edge(e);
                ed.curve.eval(ed.range.lo).distance(a) < 1e-6 && ed.curve.eval(ed.range.hi).distance(b) < 1e-6
            })
            .expect("the edge")
    };
    let (y, z) = (122.81808499611236, 15.497288355585663);
    let bottom = edge_from(Point3::new(0.0, y, 0.0), Point3::ORIGIN);
    let top = edge_from(Point3::new(0.0, y, z), Point3::new(0.0, 0.0, z));
    let rim = edge_from(
        Point3::new(0.0, 84.0380589784699, 14.240203667452008),
        Point3::new(0.0, 84.0380589784699, 6.9762402330431215),
    );
    let r = fillet(
        &body,
        &[
            FilletEdge {
                edge: bottom,
                radius: Radius::Constant(0.7864539099322084),
            },
            FilletEdge {
                edge: top,
                // found running from its end at (0, y, z)
                radius: Radius::Points(vec![
                    (0.0, 0.6531669178000771),
                    (0.44452457961533676, 0.29178062703404073),
                    (1.0, 0.39993918839377435),
                ])
                .reversed(),
            },
            FilletEdge {
                edge: rim,
                radius: Radius::Constant(0.24750327074860923),
            },
        ],
        999,
    );
    assert!(
        matches!(r, Err(forge_ops::FilletError::Overflow { .. })),
        "{:?}",
        r.map(|b| b.faces.len())
    );
}

/// A variable round of a long arc, beside a variable round of an upright edge: carried on round
/// the circle, the arc's run-outs once met behind it, the blend lay over itself, and its face
/// drew inside out.
#[test]
#[allow(clippy::excessive_precision, clippy::unreadable_literal)]
fn run_outs_do_not_wrap_round_a_circle() {
    // the part as its booleans leave it (unjoined), where this arc is an edge of its own
    let part = forge_model::suite::fuzz(95, 4);
    let mut body = None;
    for i in 0..part.features.len() {
        body =
            forge_model::step(forge_model::History::new(&part.features, None), i, body).expect("regenerates");
    }
    let body = body.expect("a part").compact();
    let edge_from = |a: Point3, b: Point3| {
        body.edge_ids()
            .find(|&e| {
                let ed = body.edge(e);
                ed.curve.eval(ed.range.lo).distance(a) < 1e-6 && ed.curve.eval(ed.range.hi).distance(b) < 1e-6
            })
            .expect("the edge")
    };
    let upright = edge_from(Point3::ORIGIN, Point3::new(0.0, 0.0, 22.5986290334029));
    let arc = edge_from(
        Point3::new(45.582834624123194, 122.60770634512093, 25.580418309135823),
        Point3::new(37.18671118964335, 119.3332829097344, 22.598629033402908),
    );
    let b = fillet(
        &body,
        &[
            FilletEdge {
                edge: upright,
                radius: Radius::Linear(0.6642782433564213, 0.5511827989444841),
            },
            FilletEdge {
                edge: arc,
                radius: Radius::Linear(0.07959531712698743, 0.13589652606735247),
            },
        ],
        999,
    )
    .expect("fillets");
    assert!(forge_topo::check(&b).is_empty(), "{:?}", forge_topo::check(&b));
    let mesh = tessellate(&b, &Quality::default()).expect("tessellates");
    assert!(mesh.open_edges().is_empty(), "open mesh");
}

/// A round at the foot of a stadium boss, whose tangent edge between flat and arc has a vertex with
/// nothing else there, nearer the plate than the radius: the spring crosses that edge's upper piece,
/// so the edge counts as whole. The volume added has a closed form (Pappus for the arcs).
#[test]
fn seam_split_below_the_spring_is_one_edge() {
    use forge_bool::{Op, boolean};
    use forge_math::{Frame, Point2, Vec3};
    use forge_ops::{Profile, ProfileLoop, Segment, extrude};
    use forge_topo::Body;
    let (l, rb, h, r) = (20.0, 5.0, 10.0, 3.0);
    let at_z = |z: f64| Frame::from_z(Point3::new(0.0, 0.0, z), forge_math::UnitVec3::Z);
    let plate = extrude(
        &Profile {
            plane: at_z(0.0),
            loops: vec![Profile::rectangle(Point2::new(-30.0, -20.0), 60.0, 40.0, 0)],
        },
        5.0,
        1,
    )
    .expect("plate");
    let pi = std::f64::consts::PI;
    let boss = extrude(
        &Profile {
            plane: at_z(5.0),
            loops: vec![ProfileLoop::new(vec![
                (
                    Segment::Line {
                        a: Point2::new(0.0, -rb),
                        b: Point2::new(l, -rb),
                    },
                    0,
                ),
                (
                    Segment::Arc {
                        center: Point2::new(l, 0.0),
                        radius: rb,
                        start: -pi / 2.0,
                        sweep: pi,
                    },
                    1,
                ),
                (
                    Segment::Line {
                        a: Point2::new(l, rb),
                        b: Point2::new(0.0, rb),
                    },
                    2,
                ),
                (
                    Segment::Arc {
                        center: Point2::ORIGIN,
                        radius: rb,
                        start: pi / 2.0,
                        sweep: pi,
                    },
                    3,
                ),
            ])],
        },
        h,
        2,
    )
    .expect("boss");
    let mut body: Body = boolean(&plate, &boss, Op::Union, 3).expect("union");
    let before = forge_topo::mass::mass(&body).expect("mass").volume;
    // an edge up the wall where flat meets arc, in two with its lower piece 1 long
    let seam = body
        .edge_ids()
        .find(|&e| {
            let x = body.edge(e);
            let (a, b) = (body.vertex(x.start).point, body.vertex(x.end).point);
            (a - b).cross(Vec3::Z).length() < 1e-9 && a.z.min(b.z) > 4.0
        })
        .expect("the seam");
    let x = body.edge(seam).clone();
    let up = x.curve.deriv(x.range.mid()).d1.z > 0.0;
    let cut = if up { x.range.lo + 1.0 } else { x.range.hi - 1.0 };
    let m = body.add_vertex(x.curve.eval(cut), x.name.with_str("half"));
    let other = body.add_edge(
        x.curve.clone(),
        forge_math::Interval::new(cut, x.range.hi),
        m,
        x.end,
        x.name.with_str("half"),
    );
    {
        let edge = &mut body.edges[seam.index()];
        edge.range.hi = cut;
        edge.end = m;
        edge.bbox.clear();
    }
    for c in x.coedges.iter().copied().collect::<Vec<_>>() {
        let (f, lp) = (body.coedge_face(c), body.coedge(c).lp);
        let uses: Vec<_> = body
            .lp(lp)
            .coedges()
            .flat_map(|d| {
                let co = body.coedge(d);
                match (co.edge == seam, co.forward) {
                    (true, true) => vec![(seam, true), (other, true)],
                    (true, false) => vec![(other, false), (seam, false)],
                    _ => vec![(co.edge, co.forward)],
                }
            })
            .collect();
        body.replace_loop(f, lp, &uses);
    }
    let foot: Vec<FilletEdge> = body
        .edge_ids()
        .filter(|&e| {
            let x = body.edge(e);
            let (a, b) = (x.curve.eval(x.range.lo), x.curve.eval(x.range.mid()));
            (a.z - 5.0).abs() < 1e-9
                && (b.z - 5.0).abs() < 1e-9
                && forge_ops::fillet_kind(&body, e) == Some(false)
        })
        .map(|edge| FilletEdge {
            edge,
            radius: Radius::Constant(r),
        })
        .collect();
    assert_eq!(foot.len(), 4);
    let b = fillet(&body, &foot, 999).expect("rounds");
    assert!(forge_topo::check(&b).is_empty());
    assert!(
        tessellate(&b, &Quality::default())
            .expect("meshes")
            .open_edges()
            .is_empty()
    );
    let area = r * r * (1.0 - pi / 4.0);
    let moment = r * r * r * (5.0 / 6.0 - pi / 4.0);
    let added = 2.0 * l * area + 2.0 * pi * (rb * area + moment);
    let got = forge_topo::mass::mass(&b).expect("mass").volume - before;
    assert!((got - added).abs() < 1e-6, "added {got}, want {added}");
}

/// A law on a closed rim, given through points on one half only, runs round the rim through them; a
/// half holding one point of it does not hold that radius (it took the rim at 1.4 all round).
#[test]
fn a_law_on_a_closed_rim_is_periodic() {
    use forge_math::{Frame, Point2, Point3, Vec3};
    use forge_ops::{FilletEdge, Profile, Radius, extrude, fillet_with};
    let plane = Frame::from_axes(Point3::ORIGIN, Vec3::X, Vec3::Y, Vec3::Z).unwrap();
    let post = extrude(
        &Profile {
            plane,
            loops: vec![Profile::circle(Point2::ORIGIN, 3.0, 0)],
        },
        5.0,
        1,
    )
    .unwrap();
    let rim = post
        .edge_ids()
        .find(|&e| post.edge_points(e, 8).iter().all(|p| (p.z - 5.0).abs() < 1e-9))
        .unwrap();
    let round = |radius| {
        let list = [FilletEdge { edge: rim, radius }];
        fillet_with(&post, &list, &Default::default(), 2)
    };
    let volume = |b: &forge_topo::Body| forge_topo::mass::mass(b).unwrap().volume;
    let half = volume(&round(Radius::Points(vec![(0.0, 0.6), (0.5, 1.4)])).unwrap());
    let whole = volume(&round(Radius::Points(vec![(0.0, 0.6), (0.5, 1.4), (1.0, 0.6)])).unwrap());
    assert!((half - whole).abs() < 1e-6, "{half} {whole}");
    // between the rims at the two radii, not at either
    let at = |r: f64| volume(&round(Radius::Constant(r)).unwrap());
    assert!(half > at(1.4) + 1.0 && half < at(0.6) - 1.0, "{half}");
}
