//! Sheets (surface bodies) from `extrude_sheet`: valid by `check`'s sheet rules, their counts, and
//! their area exact against the chain's length times the depth.

use forge_geom::SurfaceGeom;
use forge_geom::spline::PlaneSpline;
use forge_math::{Frame, Point2, dmath};
use forge_ops::{ExtrudeError, ProfileError, Segment, extrude_sheet};
use forge_topo::mass::{mass, sheet_area};
use forge_topo::{Body, Problem, check};
use std::sync::Arc;

fn line(a: (f64, f64), b: (f64, f64)) -> Segment {
    Segment::Line {
        a: Point2::new(a.0, a.1),
        b: Point2::new(b.0, b.1),
    }
}

fn valid(b: &Body) {
    let p = check(b);
    assert!(p.is_empty(), "{p:?}");
    assert!(b.is_sheet());
    // what copies a body keeps its faces' mark
    assert!(b.compact().is_sheet() && b.clone().into_canonical().is_sheet());
}

fn counts(b: &Body) -> (usize, usize, usize, usize) {
    (b.vertices.len(), b.edges.len(), b.faces.len(), b.shells.len())
}

fn area(b: &Body) -> f64 {
    sheet_area(b).unwrap().0
}

#[test]
fn an_l_of_lines_is_a_valid_open_sheet() {
    let chain = vec![
        (line((0.0, 0.0), (40.0, 0.0)), 1),
        (line((40.0, 0.0), (40.0, 25.0)), 2),
    ];
    let b = extrude_sheet(&Frame::WORLD, &[chain], 10.0, 7).unwrap();
    valid(&b);
    assert_eq!(counts(&b), (6, 7, 2, 1));
    // boundary: the four rim edges and the two end uprights
    let once = b.edge_ids().filter(|&e| b.edge(e).coedges.len() == 1).count();
    assert_eq!(once, 6);
    assert!((area(&b) - 650.0).abs() < 1e-9, "{}", area(&b));
}

#[test]
fn the_front_is_the_right_of_the_chain_both_ways() {
    // a line along +x: its right, with z toward you, is -y
    for d in [10.0, -10.0] {
        let b = extrude_sheet(&Frame::WORLD, &[vec![(line((0.0, 0.0), (5.0, 0.0)), 1)]], d, 1).unwrap();
        valid(&b);
        let f = b.face_ids().next().unwrap();
        let p = b.vertex(b.edge(b.edge_ids().next().unwrap()).start).point;
        let n = b.face_normal(f, b.face(f).surface.project(p)).unwrap();
        assert!(n.get().y < -0.999, "{d}: {n:?}");
    }
}

#[test]
fn arcs_and_splines_make_exact_walls() {
    let arc = Segment::Arc {
        center: Point2::new(0.0, 0.0),
        radius: 10.0,
        start: 0.0,
        sweep: dmath::PI / 2.0,
    };
    let s = PlaneSpline::through(
        vec![
            Point2::new(0.0, 10.0),
            Point2::new(-5.0, 14.0),
            Point2::new(-12.0, 13.0),
            Point2::new(-20.0, 18.0),
        ],
        None,
        None,
    )
    .unwrap();
    // its length by Gauss-Legendre on many pieces
    let d = s.c.domain();
    let (g, w) = (
        [
            -0.906179845938664,
            -0.538469310105683,
            0.0,
            0.538469310105683,
            0.906179845938664,
        ],
        [
            0.236926885056189,
            0.478628670499366,
            0.568888888888889,
            0.478628670499366,
            0.236926885056189,
        ],
    );
    let n = 4000;
    let h = d.width() / n as f64;
    let len: f64 = (0..n)
        .map(|k| {
            let m = d.lo + h * (k as f64 + 0.5);
            (0..5)
                .map(|i| w[i] * 0.5 * h * s.c.deriv(m + 0.5 * h * g[i]).d1.length())
                .sum::<f64>()
        })
        .sum();
    let chain = vec![(arc, 1), (Segment::Spline(Arc::new(s)), 2)];
    let b = extrude_sheet(&Frame::WORLD, &[chain], 4.0, 3).unwrap();
    valid(&b);
    let want = 4.0 * (10.0 * dmath::PI / 2.0 + len);
    assert!((area(&b) - want).abs() < 1e-7 * want, "{} {want}", area(&b));
}

#[test]
fn a_loop_is_a_tube_and_a_circle_a_cylinder() {
    let sq = vec![
        (line((0.0, 0.0), (0.0, 10.0)), 1),
        (line((0.0, 10.0), (10.0, 10.0)), 2),
        (line((10.0, 10.0), (10.0, 0.0)), 3),
        (line((10.0, 0.0), (0.0, 0.0)), 4),
    ];
    // drawn clockwise, turned: the front is outside
    let b = extrude_sheet(&Frame::WORLD, &[sq], 5.0, 1).unwrap();
    valid(&b);
    assert_eq!(counts(&b), (8, 12, 4, 1));
    assert!((area(&b) - 200.0).abs() < 1e-9);
    let c = Segment::Circle {
        center: Point2::ORIGIN,
        radius: 3.0,
        ccw: true,
    };
    let b = extrude_sheet(&Frame::WORLD, &[vec![(c, 1)]], 5.0, 1).unwrap();
    valid(&b);
    assert_eq!(counts(&b), (2, 3, 1, 1));
    assert!((area(&b) - 30.0 * dmath::PI).abs() < 1e-9);
}

#[test]
fn two_chains_are_two_shells() {
    let a = vec![(line((0.0, 0.0), (10.0, 0.0)), 1)];
    let b = vec![
        (line((0.0, 5.0), (10.0, 5.0)), 2),
        (line((10.0, 5.0), (10.0, 9.0)), 3),
    ];
    let s = extrude_sheet(&Frame::WORLD, &[a, b], 2.0, 1).unwrap();
    valid(&s);
    assert_eq!(s.shells.len(), 2);
}

#[test]
fn refusals() {
    let cross = vec![
        (line((0.0, 0.0), (10.0, 0.0)), 1),
        (line((10.0, 0.0), (10.0, 5.0)), 2),
        (line((10.0, 5.0), (5.0, -5.0)), 3),
    ];
    assert_eq!(
        extrude_sheet(&Frame::WORLD, &[cross], 2.0, 1),
        Err(ExtrudeError::Profile(ProfileError::SelfIntersecting {
            loop_index: 0
        }))
    );
    let a = vec![(line((0.0, 0.0), (10.0, 0.0)), 1)];
    let b = vec![(line((5.0, -5.0), (5.0, 5.0)), 2)];
    assert_eq!(
        extrude_sheet(&Frame::WORLD, &[a.clone(), b], 2.0, 1),
        Err(ExtrudeError::Profile(ProfileError::SelfIntersecting {
            loop_index: 1
        }))
    );
    let gap = vec![
        (line((0.0, 0.0), (10.0, 0.0)), 1),
        (line((10.0, 1.0), (10.0, 5.0)), 2),
    ];
    assert!(matches!(
        extrude_sheet(&Frame::WORLD, &[gap], 2.0, 1),
        Err(ExtrudeError::Profile(ProfileError::Open { id: 1, .. }))
    ));
    assert_eq!(
        extrude_sheet(&Frame::WORLD, &[a], 0.0, 1),
        Err(ExtrudeError::TooShort)
    );
}

#[test]
fn a_sheet_has_no_volume_and_check_keeps_solids_closed() {
    let chain = vec![
        (line((0.0, 0.0), (40.0, 0.0)), 1),
        (line((40.0, 0.0), (40.0, 25.0)), 2),
    ];
    let mut b = extrude_sheet(&Frame::WORLD, &[chain], 10.0, 7).unwrap();
    let m = mass(&b).unwrap();
    assert_eq!((m.volume, m.area), (0.0, 0.0));
    assert!((m.sheet_area - 650.0).abs() < 1e-9);
    // the same topology called a solid: its boundary edges are each used once, not twice
    for f in &mut b.faces {
        f.sheet = false;
    }
    let p = check(&b);
    assert_eq!(
        p.iter()
            .filter(|p| matches!(p, Problem::EdgeUses { uses: 1, .. }))
            .count(),
        6,
        "{p:?}"
    );
}

mod revolved {
    use super::*;
    use forge_math::Vec2;
    use forge_ops::{Axis2, revolve_sheet};

    const Y: Axis2 = Axis2 {
        origin: Point2::ORIGIN,
        dir: Vec2 { x: 0.0, y: 1.0 },
    };

    #[test]
    fn a_line_turned_part_way_is_an_open_cylinder() {
        let b = revolve_sheet(
            &Frame::WORLD,
            &[vec![(line((5.0, 0.0), (5.0, 8.0)), 1)]],
            Y,
            dmath::PI / 2.0,
            1,
        )
        .unwrap();
        valid(&b);
        assert!(
            (area(&b) - 5.0 * dmath::PI / 2.0 * 8.0).abs() < 1e-9,
            "{}",
            area(&b)
        );
    }

    #[test]
    fn a_whole_turn_of_a_line_and_a_cone() {
        // a cylinder wall and a cone: two boundary circles, the seam inside
        let chain = vec![
            (line((5.0, 0.0), (5.0, 8.0)), 1),
            (line((5.0, 8.0), (2.0, 12.0)), 2),
        ];
        let b = revolve_sheet(&Frame::WORLD, &[chain], Y, dmath::TAU, 1).unwrap();
        valid(&b);
        let want = dmath::TAU * 5.0 * 8.0 + dmath::PI * (5.0 + 2.0) * 5.0;
        assert!((area(&b) - want).abs() < 1e-8 * want, "{} {want}", area(&b));
    }

    #[test]
    fn a_half_circle_from_pole_to_pole_is_a_closed_sphere() {
        let arc = Segment::Arc {
            center: Point2::ORIGIN,
            radius: 4.0,
            start: -dmath::PI / 2.0,
            sweep: dmath::PI,
        };
        let b = revolve_sheet(&Frame::WORLD, &[vec![(arc, 1)]], Y, dmath::TAU, 1).unwrap();
        valid(&b);
        assert_eq!(b.edge_ids().filter(|&e| b.edge(e).coedges.len() == 1).count(), 0);
        assert!((area(&b) - 64.0 * dmath::PI).abs() < 1e-8, "{}", area(&b));
        // a quarter: an open dome, one boundary circle
        let arc = Segment::Arc {
            center: Point2::ORIGIN,
            radius: 4.0,
            start: 0.0,
            sweep: dmath::PI / 2.0,
        };
        let b = revolve_sheet(&Frame::WORLD, &[vec![(arc, 1)]], Y, dmath::TAU, 1).unwrap();
        valid(&b);
        assert!((area(&b) - 32.0 * dmath::PI).abs() < 1e-8, "{}", area(&b));
    }

    #[test]
    fn a_flat_ring_turned_whole() {
        let b = revolve_sheet(
            &Frame::WORLD,
            &[vec![(line((2.0, 0.0), (6.0, 0.0)), 1)]],
            Y,
            dmath::TAU,
            1,
        )
        .unwrap();
        valid(&b);
        assert!((area(&b) - dmath::PI * 32.0).abs() < 1e-9, "{}", area(&b));
    }
}

mod thickened {
    use super::*;
    use forge_ops::thicken;

    fn solid(b: &Body) -> f64 {
        let p = check(b);
        assert!(p.is_empty(), "{p:?}");
        assert!(!b.has_sheet());
        mass(b).unwrap().volume
    }

    fn faces(b: &Body) -> Vec<forge_topo::Name> {
        b.face_ids().map(|f| b.face(f).name).collect()
    }

    #[test]
    fn an_l_wall_mitred_at_its_corner() {
        let chain = vec![
            (line((0.0, 0.0), (40.0, 0.0)), 1),
            (line((40.0, 0.0), (40.0, 25.0)), 2),
        ];
        let s = extrude_sheet(&Frame::WORLD, &[chain], 10.0, 7).unwrap();
        let t = 2.0;
        // out: the front (right of the chain), the corner's outside squared off
        let v = solid(&thicken(&s, &faces(&s), 0.0, t, 8).unwrap());
        assert!((v - 10.0 * (65.0 * t + t * t)).abs() < 1e-9, "{v}");
        let v = solid(&thicken(&s, &faces(&s), -t, 0.0, 8).unwrap());
        assert!((v - 10.0 * (65.0 * t - t * t)).abs() < 1e-9, "{v}");
        let v = solid(&thicken(&s, &faces(&s), -t / 2.0, t / 2.0, 8).unwrap());
        assert!((v - 10.0 * 65.0 * t).abs() < 1e-9, "{v}");
    }

    #[test]
    fn a_closed_square_is_a_tube() {
        let sq = vec![
            (line((0.0, 0.0), (10.0, 0.0)), 1),
            (line((10.0, 0.0), (10.0, 10.0)), 2),
            (line((10.0, 10.0), (0.0, 10.0)), 3),
            (line((0.0, 10.0), (0.0, 0.0)), 4),
        ];
        let s = extrude_sheet(&Frame::WORLD, &[sq], 5.0, 1).unwrap();
        let v = solid(&thicken(&s, &faces(&s), 0.0, 1.0, 2).unwrap());
        assert!((v - 5.0 * (144.0 - 100.0)).abs() < 1e-9, "{v}");
        let v = solid(&thicken(&s, &faces(&s), -1.0, 0.0, 2).unwrap());
        assert!((v - 5.0 * (100.0 - 64.0)).abs() < 1e-9, "{v}");
    }

    #[test]
    fn a_spline_wall_both_ways_is_its_length_times_t() {
        // offsets t/2 either side of a plane curve bound t times its length exactly (the curvature
        // terms of the two sides cancel), so the wall's volume is h t L
        let sp = PlaneSpline::through(
            vec![
                Point2::new(0.0, 0.0),
                Point2::new(8.0, 3.0),
                Point2::new(16.0, -2.0),
                Point2::new(24.0, 4.0),
            ],
            None,
            None,
        )
        .unwrap();
        let d = sp.c.domain();
        let (g, w) = (
            [
                -0.906179845938664,
                -0.538469310105683,
                0.0,
                0.538469310105683,
                0.906179845938664,
            ],
            [
                0.236926885056189,
                0.478628670499366,
                0.568888888888889,
                0.478628670499366,
                0.236926885056189,
            ],
        );
        let n = 4000;
        let hh = d.width() / n as f64;
        let len: f64 = (0..n)
            .map(|k| {
                let m = d.lo + hh * (k as f64 + 0.5);
                (0..5)
                    .map(|i| w[i] * 0.5 * hh * sp.c.deriv(m + 0.5 * hh * g[i]).d1.length())
                    .sum::<f64>()
            })
            .sum();
        let s = extrude_sheet(&Frame::WORLD, &[vec![(Segment::Spline(Arc::new(sp)), 1)]], 5.0, 1).unwrap();
        let t = 0.5;
        let v = solid(&thicken(&s, &faces(&s), -t / 2.0, t / 2.0, 2).unwrap());
        assert!((v - 5.0 * t * len).abs() < 1e-7 * v, "{v} {}", 5.0 * t * len);
        // one side: still valid, and between the two (the curve bends both ways)
        let out = solid(&thicken(&s, &faces(&s), 0.0, t, 2).unwrap());
        let inn = solid(&thicken(&s, &faces(&s), -t, 0.0, 2).unwrap());
        assert!((out + inn - 2.0 * 5.0 * t * len).abs() < 1e-7 * v, "{out} {inn}");
    }

    #[test]
    fn arcs_offset_to_arcs() {
        // a line then a quarter arc tangent to it: the wall t thick, out and in
        let chain = vec![
            (line((0.0, 0.0), (10.0, 0.0)), 1),
            (
                Segment::Arc {
                    center: Point2::new(10.0, 5.0),
                    radius: 5.0,
                    start: -dmath::PI / 2.0,
                    sweep: dmath::PI / 2.0,
                },
                2,
            ),
        ];
        let s = extrude_sheet(&Frame::WORLD, &[chain], 3.0, 1).unwrap();
        let t = 1.0;
        let len = 10.0 + 5.0 * dmath::PI / 2.0;
        // outward of a ccw arc is away from its centre: the ring there is (r + t/2) wide on average
        let v = solid(&thicken(&s, &faces(&s), 0.0, t, 2).unwrap());
        let want = 3.0 * (10.0 * t + dmath::PI / 4.0 * ((5.0 + t) * (5.0 + t) - 25.0));
        assert!((v - want).abs() < 1e-8, "{v} {want}");
        let v = solid(&thicken(&s, &faces(&s), -t / 2.0, t / 2.0, 2).unwrap());
        assert!((v - 3.0 * len * t).abs() < 1e-8, "{v}");
    }
}
