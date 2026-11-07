use forge_harness::tube::tube;
use forge_harness::*;
use forge_math::{Point3, UnitVec3};
use std::time::Instant;

fn p(x: f64, y: f64, z: f64) -> Point3 {
    Point3::new(x, y, z)
}

/// A segment with `n` corners zig-zagging in 3D between two directed nodes.
fn zigzag(n: usize, r_wire: f64) -> Network {
    let via = (0..n)
        .map(|i| {
            p(
                30.0 * (i + 1) as f64,
                if i % 2 == 0 { 25.0 } else { -25.0 },
                10.0 * i as f64,
            )
        })
        .collect();
    let end = p(30.0 * (n + 1) as f64, 0.0, 10.0 * n as f64);
    Network {
        nodes: vec![
            Node {
                id: "a".into(),
                at: p(0.0, 0.0, 0.0),
                dir: Some(UnitVec3::X),
                lead: Some(8.0),
                clip: None,
            },
            Node {
                id: "b".into(),
                at: end,
                dir: Some(UnitVec3::X),
                lead: Some(8.0),
                clip: None,
            },
        ],
        segments: vec![Segment {
            id: "s".into(),
            a: 0,
            b: 1,
            via,
            k: 1.5,
            bend_k: 3.0,
            wall: 0.0,
            service_loop: 0.0,
            d: None,
        }],
        types: vec![WireType {
            id: "w".into(),
            od: 1.6,
            min_bend: r_wire,
        }],
        connectors: vec![
            Connector {
                id: "J1".into(),
                exit: 0,
                pins: (0..6).map(|i| (format!("{i}"), p(-6.0, 0.0, i as f64))).collect(),
                insert: 0.0,
            },
            Connector {
                id: "J2".into(),
                exit: 1,
                pins: (0..6)
                    .map(|i| (format!("{i}"), end + (p(6.0, 0.0, i as f64) - p(0.0, 0.0, 0.0))))
                    .collect(),
                insert: 0.0,
            },
        ],
        wires: (0..6)
            .map(|i| Wire {
                id: format!("W{i}"),
                ty: 0,
                from: End { connector: 0, pin: i },
                to: End { connector: 1, pin: i },
                path: None,
            })
            .collect(),
        groups: vec![],
        settings: Settings::default(),
    }
}

#[test]
fn tubes_are_valid_closed_form_bodies() {
    let net = zigzag(4, 0.0);
    let r = route(&net);
    let s = &r.segments[0];
    assert!(r.notes.is_empty(), "{:?}", r.notes);
    let bodies = tube(&s.pieces, s.d, 1);
    assert_eq!(bodies.len(), 1);
    let b = &bodies[0];
    assert!(forge_topo::check::check(b).is_empty());
    assert!(b.faces.iter().all(|f| matches!(
        f.surface,
        forge_geom::Surface::Cylinder(_) | forge_geom::Surface::Torus(_) | forge_geom::Surface::Plane(_)
    )));
}

#[test]
fn a_corner_tighter_than_the_tube_still_builds() {
    let mut net = zigzag(2, 0.0);
    // the corner gets so little room its arc is under the tube's radius
    net.segments[0].via = vec![p(30.0, 0.0, 0.0), p(30.4, 0.4, 0.0)];
    net.segments[0].d = Some(6.0);
    let r = route(&net);
    assert!(r.notes.iter().any(|n| matches!(n, Note::BendTooTight { .. })));
    let s = &r.segments[0];
    assert!(!tube(&s.pieces, s.d, 1).is_empty());
}

/// What a segment costs: its route, its tube, and the tube's mesh (printed: `--nocapture`).
#[test]
fn a_segment_routes_and_sweeps_in_milliseconds() {
    for n in [1usize, 3, 6] {
        let net = zigzag(n, 0.0);
        let t0 = Instant::now();
        let r = route(&net);
        let t1 = Instant::now();
        let s = &r.segments[0];
        let bodies = tube(&s.pieces, s.d, 1);
        let t2 = Instant::now();
        let q = forge_mesh::tessellate::Quality {
            chord: 0.05,
            max_angle: forge_math::dmath::PI / 12.0,
        };
        let tris: usize = bodies
            .iter()
            .map(|b| forge_mesh::tessellate::tessellate(b, &q).unwrap().triangles.len())
            .sum();
        let t3 = Instant::now();
        println!(
            "corners {n}: pieces {} route {:.3} ms  sweep {:.3} ms  mesh {:.3} ms ({tris} tris)",
            s.pieces.len(),
            (t1 - t0).as_secs_f64() * 1e3,
            (t2 - t1).as_secs_f64() * 1e3,
            (t3 - t2).as_secs_f64() * 1e3
        );
        assert!((t3 - t0).as_secs_f64() < 0.5);
    }
}
