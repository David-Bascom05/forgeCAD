use forge_harness::wires::{pack, paths, slots, tube_mesh};
use forge_harness::*;
use forge_math::{Point3, UnitVec3, Vec3};
use std::time::Instant;

fn p(x: f64, y: f64, z: f64) -> Point3 {
    Point3::new(x, y, z)
}

fn no_overlap(r: &[f64], at: &[[f64; 2]], d: f64) {
    for i in 0..r.len() {
        let reach = (at[i][0].powi(2) + at[i][1].powi(2)).sqrt() + r[i];
        assert!(reach <= d / 2.0 + 1e-9);
        for j in 0..i {
            let gap = ((at[i][0] - at[j][0]).powi(2) + (at[i][1] - at[j][1]).powi(2)).sqrt();
            assert!(gap >= r[i] + r[j] - 1e-9, "{i} {j} overlap");
        }
    }
}

#[test]
fn packings_are_tight_and_never_overlap() {
    let (at, d) = pack(&[0.8]);
    assert_eq!((at, d), (vec![[0.0, 0.0]], 1.6));
    let (_, d) = pack(&[1.0, 1.0]);
    assert!((d - 4.0).abs() < 1e-12);
    // three equal: the triangle, 1 + 2/sqrt(3) of the diameter
    let (_, d) = pack(&[1.0; 3]);
    assert!((d - 2.0 * (1.0 + 2.0 / 3f64.sqrt())).abs() < 2e-3, "{d}");
    // seven equal: the hexagon, three diameters
    let (at, d) = pack(&[1.0; 7]);
    assert!((d - 6.0).abs() < 2e-3, "{d}");
    no_overlap(&[1.0; 7], &at, d);
    let mixed: Vec<f64> = (0..40).map(|k| [0.55, 0.8, 1.2][k % 3]).collect();
    let (at, d) = pack(&mixed);
    no_overlap(&mixed, &at, d);
    // within a quarter of the area rule's diameter
    let area = (1.5 * mixed.iter().map(|r| 4.0 * r * r).sum::<f64>()).sqrt();
    assert!(d < 1.25 * area && d > 0.8 * area, "{d} {area}");
    assert_eq!(pack(&mixed), pack(&mixed));
}

/// A trunk to a directed hub, two branches; `n` wires each way, three gauges.
fn net(n: usize) -> Network {
    let node = |id: &str, at: Point3, dir: Option<UnitVec3>| Node {
        id: id.into(),
        at,
        dir,
        lead: None,
        clip: None,
    };
    let seg = |a, b| Segment {
        id: format!("{a}-{b}"),
        a,
        b,
        via: vec![],
        k: 1.5,
        bend_k: 3.0,
        wall: 0.0,
        service_loop: 0.0,
        d: None,
    };
    let row = |c: Point3, m: usize| -> Vec<(String, Point3)> {
        (0..m)
            .map(|i| {
                (
                    format!("{}", i + 1),
                    c + Vec3::new(2.54 * (i as f64 - (m - 1) as f64 / 2.0), 0.0, 0.0),
                )
            })
            .collect()
    };
    let conn = |id: &str, exit, at, m| Connector {
        id: id.into(),
        exit,
        pins: row(at, m),
        insert: 4.0,
    };
    Network {
        nodes: vec![
            node("a", p(0.0, 0.0, 20.0), Some(UnitVec3::Z)),
            node("hub", p(150.0, 0.0, 60.0), Some(UnitVec3::X)),
            node("b", p(300.0, 80.0, 20.0), Some(UnitVec3::Z)),
            node("c", p(280.0, -90.0, 20.0), Some(UnitVec3::Z)),
        ],
        segments: vec![seg(0, 1), seg(1, 2), seg(1, 3)],
        types: [1.1, 1.6, 2.4]
            .iter()
            .map(|&od| WireType {
                id: format!("{od}"),
                od,
                min_bend: 0.0,
            })
            .collect(),
        connectors: vec![
            conn("J1", 0, p(0.0, 0.0, 10.0), 2 * n),
            conn("J2", 2, p(300.0, 80.0, 10.0), n),
            conn("J3", 3, p(280.0, -90.0, 10.0), n),
        ],
        wires: (0..2 * n)
            .map(|i| Wire {
                id: format!("W{i}"),
                ty: i % 3,
                from: End { connector: 0, pin: i },
                to: End {
                    connector: 1 + i / n,
                    pin: i % n,
                },
                path: None,
            })
            .collect(),
        groups: vec![],
        settings: Settings::default(),
    }
}

#[test]
fn each_wire_runs_pin_to_pin_through_its_slots() {
    let net = net(6);
    let r = route(&net);
    let sl = slots(&net, &r);
    assert_eq!(sl[0].at.len(), 12);
    let ws = paths(&net, &r, &sl, 3.0);
    for (k, w) in ws.iter().enumerate() {
        let w = w.as_ref().unwrap();
        let from = net.connectors[0].pins[k].1;
        let to_c = &net.connectors[net.wires[k].to.connector];
        let to = to_c.pins[net.wires[k].to.pin].1;
        // into the pin by its insertion (3 at most), through the pin, out of the other
        assert!(w.points[0].distance(from - Vec3::new(0.0, 0.0, 3.0)) < 1e-9);
        assert!(w.points[1].distance(from) < 1e-9);
        assert!(w.points.last().unwrap().distance(to - Vec3::new(0.0, 0.0, 3.0)) < 1e-9);
        assert!(w.points.iter().any(|q| q.distance(to) < 1e-9));
        // no jump longer than the longest straight
        for s in w.points.windows(2) {
            assert!(s[0].distance(s[1]) < 200.0);
        }
        assert!(w.breakout[0].is_some() && w.breakout[1].is_some());
    }
}

#[test]
fn a_wire_tube_is_rings_and_caps() {
    let m = tube_mesh(&[p(0.0, 0.0, 0.0), p(10.0, 0.0, 0.0), p(10.0, 10.0, 0.0)], 0.5, 8);
    assert_eq!(m.positions.len(), 3 * 8 + 2);
    assert_eq!(m.triangles.len(), 2 * 8 * 2 + 2 * 8);
    for (q, n) in m.positions.iter().zip(&m.normals).take(24) {
        assert!((n.length() - 1.0).abs() < 1e-12);
        // on the ring: half a millimetre off the line it rounds
        let axis = if q.x < 10.0 - 1e-9 {
            (q.y * q.y + q.z * q.z).sqrt()
        } else {
            ((q.x - 10.0).powi(2) + q.z * q.z).sqrt()
        };
        assert!((axis - 0.5).abs() < 1e-9 || q.x > 9.0, "{q:?}");
    }
}

/// A hundred wires: route, pack, every wire's path and its mesh (printed: `--nocapture`).
#[test]
fn a_hundred_wires_route_and_mesh_in_milliseconds() {
    let net = net(50);
    let t0 = Instant::now();
    let r = route(&net);
    let t1 = Instant::now();
    let sl = slots(&net, &r);
    let t2 = Instant::now();
    let ws = paths(&net, &r, &sl, 3.0);
    let t3 = Instant::now();
    let tris: usize = ws
        .iter()
        .flatten()
        .map(|w| tube_mesh(&w.points, w.r, 8).triangles.len())
        .sum();
    let t4 = Instant::now();
    let ms = |a: Instant, b: Instant| (b - a).as_secs_f64() * 1e3;
    println!(
        "100 wires: route {:.2} ms, pack {:.2} ms, paths {:.2} ms, meshes {:.2} ms ({tris} triangles)",
        ms(t0, t1),
        ms(t1, t2),
        ms(t2, t3),
        ms(t3, t4)
    );
    assert_eq!(ws.iter().flatten().count(), 100);
    assert!(ms(t0, t4) < 2000.0);
}

#[test]
fn a_clip_holds_its_bundle_and_ties_ring_the_bare_runs() {
    let mut n = net(6);
    // the hub a clip seated on a post top, standing up
    n.nodes[1].at = p(150.0, 0.0, 40.0);
    n.nodes[1].clip = Some(UnitVec3::Z);
    let r = route(&n);
    let d = (0..3).map(|k| r.segments[k].d).fold(0.0, f64::max);
    assert!((r.nodes_at[1].z - (40.0 + CLIP_WALL + d / 2.0)).abs() < 1e-12);
    assert_eq!(r.clips.len(), 1);
    assert_eq!(r.clips[0].base, p(150.0, 0.0, 40.0));
    let sl = slots(&n, &r);
    let ties = wires::ties(&n, &r, &sl, 50.0);
    // every bare segment has ties, none closer than a tie's width
    assert!(ties.len() >= 6, "{}", ties.len());
    for t in &ties {
        assert!(t.d > 2.0);
    }
    assert!(wires::ties(&n, &r, &sl, 0.0).is_empty());
}

#[test]
fn wires_through_each_other_are_found() {
    let a = wires::WirePath {
        points: vec![p(0.0, 0.0, 0.0), p(10.0, 0.0, 0.0)],
        r: 0.5,
        breakout: [None, None],
    };
    let b = wires::WirePath {
        points: vec![p(5.0, -5.0, 0.3), p(5.0, 5.0, 0.3)],
        r: 0.5,
        breakout: [None, None],
    };
    let clear = wires::WirePath {
        points: vec![p(5.0, -5.0, 1.2), p(5.0, 5.0, 1.2)],
        r: 0.5,
        breakout: [None, None],
    };
    let c = wires::clashes(&[Some(a.clone()), Some(b)], 0.1);
    assert_eq!(c.len(), 1);
    assert!(c[0].2.distance(p(5.0, 0.0, 0.15)) < 1e-9);
    assert!(wires::clashes(&[Some(a), Some(clear)], 0.1).is_empty());
}
