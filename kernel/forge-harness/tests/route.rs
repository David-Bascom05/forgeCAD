use forge_harness::*;
use forge_math::{Point3, UnitVec3, Vec3, dmath};
use forge_ops::sweep::PathSeg;

fn p(x: f64, y: f64, z: f64) -> Point3 {
    Point3::new(x, y, z)
}

fn node(id: &str, at: Point3, dir: Option<UnitVec3>) -> Node {
    Node {
        id: id.into(),
        at,
        dir,
        lead: None,
    }
}

fn seg(id: &str, a: usize, b: usize, via: Vec<Point3>) -> Segment {
    Segment {
        id: id.into(),
        a,
        b,
        via,
        k: 1.5,
        bend_k: 3.0,
        wall: 0.0,
        service_loop: 0.0,
        d: None,
    }
}

fn conn(id: &str, exit: usize, at: Point3, n: usize) -> Connector {
    Connector {
        id: id.into(),
        exit,
        pins: (0..n)
            .map(|i| (format!("{}", i + 1), at + Vec3::new(0.0, 0.0, 2.54 * i as f64)))
            .collect(),
        insert: 0.0,
    }
}

fn wire(id: &str, from: (usize, usize), to: (usize, usize)) -> Wire {
    Wire {
        id: id.into(),
        ty: 0,
        from: End {
            connector: from.0,
            pin: from.1,
        },
        to: End {
            connector: to.0,
            pin: to.1,
        },
        path: None,
    }
}

fn types() -> Vec<WireType> {
    vec![WireType {
        id: "w22".into(),
        od: 1.6,
        min_bend: 0.0,
    }]
}

/// Two connectors, one segment with one corner: a quarter turn with long legs.
fn elbow(r_wire: f64) -> Network {
    Network {
        nodes: vec![
            node("a", p(0.0, 0.0, 0.0), None),
            node("b", p(50.0, 50.0, 0.0), None),
        ],
        segments: vec![seg("s", 0, 1, vec![p(50.0, 0.0, 0.0)])],
        types: vec![WireType {
            id: "w".into(),
            od: 2.0,
            min_bend: r_wire,
        }],
        connectors: vec![
            conn("J1", 0, p(-5.0, 0.0, 0.0), 1),
            conn("J2", 1, p(50.0, 55.0, 0.0), 1),
        ],
        wires: vec![wire("W1", (0, 0), (1, 0))],
        settings: Settings::default(),
    }
}

fn ends(s: &PathSeg) -> (Point3, Point3) {
    match *s {
        PathSeg::Line { a, b } => (a, b),
        PathSeg::Arc { circle, start, sweep } => {
            let at = |t: f64| {
                let (sn, cs) = dmath::sin_cos(t);
                circle.frame.origin
                    + (circle.frame.x().get() * cs + circle.frame.y().get() * sn) * circle.radius
            };
            (at(start), at(start + sweep))
        }
        PathSeg::Spline(_) => unreachable!(),
    }
}

#[test]
fn a_quarter_turn_is_an_arc_of_the_minimum_radius_and_its_length_is_closed_form() {
    let r = route(&elbow(0.0));
    let s = &r.segments[0];
    // one 2 mm wire: D = 2, R = 3 D
    assert_eq!(s.d, 2.0);
    assert_eq!(s.r_min, 6.0);
    assert_eq!(s.pieces.len(), 3);
    let want = 100.0 - 2.0 * 6.0 + 6.0 * dmath::FRAC_PI_2;
    assert!((s.length - want).abs() < 1e-12, "{} {want}", s.length);
    for w in s.pieces.windows(2) {
        assert!(ends(&w[0]).1.distance(ends(&w[1]).0) < 1e-12);
    }
    assert!(r.notes.is_empty(), "{:?}", r.notes);
}

#[test]
fn a_wire_with_a_larger_minimum_sets_the_radius() {
    let r = route(&elbow(9.0));
    assert_eq!(r.segments[0].r_min, 9.0);
}

#[test]
fn a_corner_too_tight_builds_with_the_tightest_arc_and_says_so() {
    let mut net = elbow(0.0);
    net.segments[0].via = vec![p(5.0, 0.0, 0.0)];
    net.nodes[1].at = p(5.0, 5.0, 0.0);
    let r = route(&net);
    let s = &r.segments[0];
    // both legs 5 long: the tangent takes the whole leg, R = 5 / tan(45deg)
    assert!((s.corners[0].got - 5.0).abs() < 1e-12);
    assert_eq!(
        r.notes,
        vec![Note::BendTooTight {
            segment: 0,
            corner: 0,
            at: p(5.0, 0.0, 0.0),
            needed: 6.0,
            got: s.corners[0].got,
        }]
    );
    assert!((s.length - 5.0 * dmath::FRAC_PI_2).abs() < 1e-12);
}

#[test]
fn bundles() {
    assert_eq!(bundle_d(&[1.6], 1.5), 1.6);
    assert_eq!(bundle_d(&[1.6, 1.6], 1.5), 3.2);
    let three = bundle_d(&[1.0, 1.0, 1.0], 1.5);
    assert!((three - 4.5f64.sqrt()).abs() < 1e-15);
    // many small wires: the area term
    let many = bundle_d(&[1.0; 20], 1.5);
    assert!((many - 30f64.sqrt()).abs() < 1e-14);
    // a big wire and a small one: side by side
    assert_eq!(bundle_d(&[3.0, 2.5], 1.5), 5.5);
}

#[test]
fn awg_conductors() {
    assert!((awg_d(36.0) - 0.127).abs() < 1e-15);
    assert!((awg_d(22.0) - 0.6438).abs() < 1e-4);
    assert!((awg_d(-3.0) - 11.684).abs() < 1e-3);
}

/// J1 at the left; a branch node; J2 up one branch, J3 down a longer one; a loop back segment.
fn branched(directed_branch: bool) -> Network {
    let dir = directed_branch.then_some(UnitVec3::X);
    Network {
        nodes: vec![
            node("n1", p(0.0, 0.0, 0.0), Some(UnitVec3::X)),
            node("n2", p(100.0, 0.0, 0.0), dir),
            node("n3", p(160.0, 60.0, 0.0), Some(UnitVec3::Y)),
            node("n4", p(160.0, -80.0, 20.0), Some(-UnitVec3::Y)),
        ],
        segments: vec![
            seg("trunk", 0, 1, vec![]),
            seg("up", 1, 2, vec![p(160.0, 0.0, 0.0)]),
            seg("down", 1, 3, vec![p(140.0, -40.0, 20.0)]),
            seg("around", 0, 2, vec![p(40.0, 200.0, 0.0), p(160.0, 200.0, 0.0)]),
        ],
        types: types(),
        connectors: vec![
            conn("J1", 0, p(-10.0, 0.0, 0.0), 6),
            conn("J2", 2, p(160.0, 70.0, 0.0), 4),
            conn("J3", 3, p(160.0, -90.0, 20.0), 4),
        ],
        wires: vec![
            wire("W1", (0, 0), (1, 0)),
            wire("W2", (0, 1), (1, 1)),
            wire("W3", (0, 2), (2, 0)),
            wire("W4", (0, 3), (2, 1)),
            wire("W5", (0, 4), (2, 2)),
        ],
        settings: Settings {
            slack: 0.03,
            round_up: 5.0,
            empty_d: 2.0,
        },
    }
}

#[test]
fn wires_take_the_shortest_path_and_bundles_follow() {
    let r = route(&branched(true));
    let w = |k: usize| r.wires[k].as_ref().unwrap();
    assert_eq!(w(0).path, vec![(0, true), (1, true)]);
    assert_eq!(w(2).path, vec![(0, true), (2, true)]);
    assert_eq!(r.segments[0].wires, vec![0, 1, 2, 3, 4]);
    assert_eq!(r.segments[1].wires, vec![0, 1]);
    assert_eq!(r.segments[2].wires, vec![2, 3, 4]);
    assert_eq!(r.segments[3].wires, Vec::<usize>::new());
    assert_eq!(r.segments[1].d, 3.2);
    assert!((r.segments[0].d - (1.5f64 * 5.0 * 1.6 * 1.6).sqrt()).abs() < 1e-12);
    assert_eq!(r.notes, vec![Note::SegmentEmpty { segment: 3 }]);
    // a directed node: every segment leaves along its direction
    for s in &r.segments[..3] {
        let t = s.tangents[0].unwrap();
        assert!(t.dot(UnitVec3::X).abs() > 1.0 - 1e-12, "{t:?}");
    }
}

#[test]
fn a_given_path_is_kept_and_a_broken_one_is_typed() {
    let mut net = branched(true);
    net.wires[0].path = Some(vec![3]);
    net.wires[1].path = Some(vec![0, 2]);
    let r = route(&net);
    assert_eq!(r.wires[0].as_ref().unwrap().path, vec![(3, true)]);
    assert!(r.wires[1].is_none());
    assert!(r.notes.contains(&Note::PathBroken { wire: 1, at: 2 }));
}

#[test]
fn no_path_is_typed() {
    let mut net = branched(true);
    net.segments.truncate(1);
    let r = route(&net);
    assert!(r.notes.contains(&Note::NoPath { wire: 0 }));
    assert!(r.wires[0].is_none());
}

#[test]
fn wire_lengths_keep_every_term_and_round_up() {
    let mut net = branched(true);
    net.connectors[0].insert = 4.0;
    net.connectors[1].insert = 6.0;
    net.segments[1].service_loop = 30.0;
    let r = route(&net);
    let w = r.wires[0].as_ref().unwrap();
    let stubs = 10.0 + 10.0;
    assert!((w.stubs[0] + w.stubs[1] - stubs).abs() < 1e-12);
    let route_len = stubs + r.segments[0].length + r.segments[1].length;
    assert!((w.route - route_len).abs() < 1e-12);
    assert_eq!(w.loops, 30.0);
    assert!((w.slack - (route_len + 30.0) * 0.03).abs() < 1e-12);
    assert_eq!(w.insertion, 10.0);
    assert!((w.exact - (route_len + 30.0) * 1.03 - 10.0).abs() < 1e-12);
    assert!(w.length >= w.exact && w.length - w.exact < 5.0);
    assert_eq!(w.length % 5.0, 0.0);
}

#[test]
fn a_turn_through_a_free_node_says_how_far_and_which_way() {
    let r = route(&branched(false));
    // W3 runs trunk (+x) then down toward (140, -40, 20): a turn toward -y and +z
    let kink = r
        .notes
        .iter()
        .find_map(|n| match n {
            Note::KinkAtNode {
                node: 1,
                wire: 2,
                angle,
                toward,
            } => Some((*angle, *toward)),
            _ => None,
        })
        .expect("a kink at n2");
    let out = Vec3::new(40.0, -40.0, 20.0);
    let want = dmath::acos_clamped(40.0 / out.length());
    assert!((kink.0 - want).abs() < 1e-12);
    let toward = Vec3::new(0.0, -40.0, 20.0).normalize().unwrap();
    assert!(kink.1.dot(toward) > 1.0 - 1e-12);
    // W1 runs trunk then up, straight on along x into its via: no kink
    assert!(
        !r.notes
            .iter()
            .any(|n| matches!(n, Note::KinkAtNode { wire: 0, .. }))
    );
}

#[test]
fn routing_is_deterministic() {
    let net = branched(true);
    assert_eq!(route(&net), route(&net));
}
