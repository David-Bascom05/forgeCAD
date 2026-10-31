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
                radius: Radius::Points(vec![
                    (0.0, 0.6531669178000771),
                    (0.44452457961533676, 0.29178062703404073),
                    (1.0, 0.39993918839377435),
                ]),
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
    let body = forge_model::suite::fuzz(95, 4).regenerate().expect("regenerates");
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
