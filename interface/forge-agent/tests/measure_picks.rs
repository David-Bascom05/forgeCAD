//! Measure on picks, as the app's Measure tool reads them: one entity (length, loop, radius,
//! area, perimeter, a point), two on one body (gaps, centre to centre, axes, angles), and two on
//! bodies placed apart (an assembly's occurrences).

use forge_agent::measure::{Pick, Quantity as Q, read_one, read_two};
use forge_agent::select::Ent;
use forge_geom::SurfaceGeom;
use forge_math::{Frame, Point3, Vec3};
use forge_mesh::{Quality, TriMesh};
use forge_topo::Body;

fn part(src: &str) -> (Body, TriMesh) {
    let m = forge_agent::bench::model(src);
    let b = m.body().expect("it builds").clone();
    let mesh = forge_mesh::tessellate(&b, &Quality::default()).unwrap();
    (b, mesh)
}

fn face_at(b: &Body, p: Point3) -> Ent {
    Ent::Face(
        b.face_ids()
            .find(|&f| {
                let bx = b.face_box(f);
                let within = |i: forge_math::Interval, v: f64| i.lo - 1e-6 <= v && v <= i.hi + 1e-6;
                b.face(f).surface.distance(p) < 1e-9
                    && within(bx.x, p.x)
                    && within(bx.y, p.y)
                    && within(bx.z, p.z)
            })
            .expect("a face there"),
    )
}

fn edge_between(b: &Body, p: Point3, q: Point3) -> Ent {
    Ent::Edge(
        b.edge_ids()
            .find(|&e| {
                let x = b.edge(e);
                let (s, t) = (b.vertex(x.start).point, b.vertex(x.end).point);
                (s.distance(p) < 1e-9 && t.distance(q) < 1e-9)
                    || (s.distance(q) < 1e-9 && t.distance(p) < 1e-9)
            })
            .expect("an edge there"),
    )
}

/// The hole's top rim: the circle edge at z 20 round (x, 0).
fn rim(b: &Body, x: f64) -> Ent {
    Ent::Edge(
        b.edge_ids()
            .find(|&e| match &b.edge(e).curve {
                forge_geom::Curve::Circle(c) => c.frame.origin.distance(Point3::new(x, 0.0, 20.0)) < 1e-9,
                _ => false,
            })
            .expect("a rim"),
    )
}

fn near(a: Option<f64>, b: f64) -> bool {
    a.is_some_and(|a| (a - b).abs() < 1e-6)
}

const PLATE: &str =
    "extrude 20 rect -50 -30 100 60\nhole 10 -xy at -20 0 20 thru\nhole 10 -xy at 20 0 20 thru";

#[test]
fn one_pick_reads_its_own_sizes() {
    let (b, m) = part(PLATE);
    let pick = |ent| Pick {
        body: &b,
        mesh: &m,
        at: Frame::WORLD,
        ent,
    };
    let top = read_one(&pick(face_at(&b, Point3::new(0.0, 20.0, 20.0))));
    let holes = 2.0 * std::f64::consts::PI * 25.0;
    assert!(near(top.get(Q::Area), 6000.0 - holes), "{top:?}");
    assert!(
        near(top.get(Q::Perimeter), 320.0 + 2.0 * std::f64::consts::PI * 10.0),
        "{top:?}"
    );
    let e = read_one(&pick(edge_between(
        &b,
        Point3::new(-50.0, -30.0, 20.0),
        Point3::new(50.0, -30.0, 20.0),
    )));
    assert!(near(e.get(Q::Length), 100.0));
    assert!(near(e.get(Q::LoopLength), 320.0), "{e:?}");
    let r = read_one(&pick(rim(&b, 20.0)));
    assert!(near(r.get(Q::Diameter), 10.0) && near(r.get(Q::Radius), 5.0));
    assert!(near(r.get(Q::LoopLength), 10.0 * std::f64::consts::PI), "{r:?}");
    assert!(near(r.get(Q::X), 20.0));
}

#[test]
fn a_hole_pair_reads_centre_to_centre_and_the_gap() {
    let (b, m) = part(PLATE);
    let pick = |ent| Pick {
        body: &b,
        mesh: &m,
        at: Frame::WORLD,
        ent,
    };
    let rims = read_two(&pick(rim(&b, -20.0)), &pick(rim(&b, 20.0)));
    assert!(near(rims.get(Q::CentreDistance), 40.0), "{rims:?}");
    assert!(near(rims.get(Q::Distance), 30.0), "{rims:?}");
    assert!(near(rims.get(Q::Dx), 30.0) && near(rims.get(Q::Dz), 0.0));
    let wall = |x: f64| face_at(&b, Point3::new(x, 0.0, 10.0));
    let bores = read_two(&pick(wall(-25.0)), &pick(wall(15.0)));
    assert!(near(bores.get(Q::AxisDistance), 40.0), "{bores:?}");
    assert!((bores.get(Q::Distance).unwrap() - 30.0).abs() < 1e-6, "{bores:?}");
    // the top and a side: square
    let side = face_at(&b, Point3::new(50.0, 0.0, 10.0));
    let top = face_at(&b, Point3::new(0.0, 20.0, 20.0));
    assert!(near(read_two(&pick(top), &pick(side)).get(Q::Angle), 90.0));
    // two edges from a corner
    let c = Point3::new(-50.0, -30.0, 20.0);
    let x = edge_between(&b, c, Point3::new(50.0, -30.0, 20.0));
    let z = edge_between(&b, c, Point3::new(-50.0, -30.0, 0.0));
    assert!(near(read_two(&pick(x), &pick(z)).get(Q::Angle), 90.0));
}

#[test]
fn picks_on_placed_bodies_are_measured_where_they_are() {
    let (b, m) = part(PLATE);
    let up = Frame::from_axes(Point3::new(0.0, 0.0, 50.0), Vec3::X, Vec3::Y, Vec3::Z).unwrap();
    let turned = Frame::from_axes(Point3::new(300.0, 0.0, 0.0), Vec3::Y, -Vec3::X, Vec3::Z).unwrap();
    let a = Pick {
        body: &b,
        mesh: &m,
        at: Frame::WORLD,
        ent: face_at(&b, Point3::new(0.0, 20.0, 20.0)),
    };
    let bottom = Pick {
        at: up,
        ent: face_at(&b, Point3::new(0.0, 20.0, 0.0)),
        ..a
    };
    let r = read_two(&a, &bottom);
    assert!(near(r.get(Q::Distance), 30.0), "{r:?}");
    assert!(near(r.get(Q::Dz), 30.0));
    // a rim of a turned copy: its centre turned with it
    let far = Pick {
        at: turned,
        ent: rim(&b, 20.0),
        ..a
    };
    let one = read_one(&far);
    assert!(near(one.get(Q::X), 300.0) && near(one.get(Q::Y), 20.0), "{one:?}");
    let pair = read_two(
        &Pick {
            ent: rim(&b, 20.0),
            ..a
        },
        &far,
    );
    let want = (280.0f64 * 280.0 + 20.0 * 20.0).sqrt();
    assert!(near(pair.get(Q::CentreDistance), want), "{pair:?}");
    // a vertex's coordinates, in the world
    let v = b
        .vertex_ids()
        .find(|&v| b.vertex(v).point.distance(Point3::new(50.0, 30.0, 20.0)) < 1e-9)
        .unwrap();
    let p = read_one(&Pick {
        ent: Ent::Vert(v),
        ..far
    });
    assert!(
        near(p.get(Q::X), 270.0) && near(p.get(Q::Y), 50.0) && near(p.get(Q::Z), 20.0),
        "{p:?}"
    );
}

use forge_agent::measure::{Free, Shown, Target, Whole, read, read_pair};

#[test]
fn work_features_origin_items_and_sketch_geometry_measure_against_faces() {
    let (b, m) = part(PLATE);
    let face = |p| {
        Target::Ent(Pick {
            body: &b,
            mesh: &m,
            at: Frame::WORLD,
            ent: face_at(&b, p),
        })
    };
    let top = face(Point3::new(0.0, 20.0, 20.0));
    // the XY origin plane: the plate's top is 20 above it, parallel
    let xy = Target::Free(Free::Plane(Point3::ORIGIN, Vec3::Z));
    let r = read_pair(&top, &xy);
    assert!(near(r.get(Q::Distance), 20.0), "{r:?}");
    assert!(near(r.get(Q::Dz), 20.0) && near(r.get(Q::Dx), 0.0));
    // a work point above the top: its height off it, exactly
    let p = Target::Free(Free::Point(Point3::new(3.0, 4.0, 32.5)));
    assert!(near(read_pair(&top, &p).get(Q::Distance), 12.5));
    // the Z axis and the side face at x = 50: 50 apart, at 0 degrees (the axis lies along it)
    let z = Target::Free(Free::Axis(Point3::ORIGIN, Vec3::Z));
    let side = face(Point3::new(50.0, 0.0, 10.0));
    let r = read_pair(&z, &side);
    assert!(near(r.get(Q::Distance), 50.0) && near(r.get(Q::Angle), 0.0), "{r:?}");
    // the Z axis and a hole's bore: axis to axis, 20
    let bore = face(Point3::new(25.0, 0.0, 10.0));
    assert!(near(read_pair(&z, &bore).get(Q::AxisDistance), 20.0));
    // a sketch line square to the top face: 90 degrees, its arc drawn at their corner
    let up = Target::Free(Free::Segment(Point3::new(0.0, 0.0, 30.0), Point3::new(0.0, 0.0, 40.0)));
    let r = read_pair(&up, &top);
    assert!(near(r.get(Q::Angle), 90.0) && near(r.get(Q::Distance), 10.0), "{r:?}");
    assert!(r.shown.iter().any(|s| matches!(s, Shown::Angle(_, _, _, d) if (d - 90.0).abs() < 1e-9)));
    // a sketch circle: its own sizes
    let c = read(&Target::Free(Free::Circle(Point3::ORIGIN, Vec3::Z, 5.0)));
    assert!(near(c.get(Q::Diameter), 10.0) && near(c.get(Q::Length), std::f64::consts::TAU * 5.0));
}

#[test]
fn the_maximum_distance_of_points_lines_and_circles_in_one_plane() {
    let seg = |a: (f64, f64), b: (f64, f64)| {
        Target::Free(Free::Segment(Point3::new(a.0, a.1, 0.0), Point3::new(b.0, b.1, 0.0)))
    };
    let r = read_pair(&seg((0.0, 0.0), (10.0, 0.0)), &seg((0.0, 5.0), (10.0, 5.0)));
    assert!(near(r.get(Q::Distance), 5.0));
    assert!(near(r.get(Q::MaxDistance), 125f64.sqrt()), "{r:?}");
    let circle = |x: f64, rad: f64| Target::Free(Free::Circle(Point3::new(x, 0.0, 0.0), Vec3::Z, rad));
    let r = read_pair(&circle(0.0, 3.0), &circle(20.0, 2.0));
    assert!(near(r.get(Q::Distance), 15.0) && near(r.get(Q::CentreDistance), 20.0));
    assert!(near(r.get(Q::MaxDistance), 25.0), "{r:?}");
    // two points: one distance, no maximum
    let p = |x: f64| Target::Free(Free::Point(Point3::new(x, 0.0, 0.0)));
    assert!(read_pair(&p(0.0), &p(3.0)).get(Q::MaxDistance).is_none());
}

#[test]
fn two_faces_at_an_angle_draw_it_at_their_common_edge_and_a_face_is_tinted() {
    let (b, m) = part("extrude 20 poly 0 0 40 0 40 20 20 40 0 40");
    let pick = |p| {
        Target::Ent(Pick {
            body: &b,
            mesh: &m,
            at: Frame::WORLD,
            ent: face_at(&b, p),
        })
    };
    // the side at x = 40 and the bevel from (40, 20) to (20, 40): 45 degrees between them
    let (side, bevel) = (pick(Point3::new(40.0, 10.0, 10.0)), pick(Point3::new(30.0, 30.0, 10.0)));
    let r = read_pair(&side, &bevel);
    assert!(near(r.get(Q::Angle), 45.0), "{r:?}");
    let arc = r.shown.iter().find_map(|s| match s {
        Shown::Angle(v, u, w, d) => Some((*v, *u, *w, *d)),
        _ => None,
    });
    let (v, u, w, d) = arc.expect("the angle's arc");
    // on the edge the two faces share (x 40, y 20), its arms along each face away from it
    assert!((v.x - 40.0).abs() < 1e-6 && (v.y - 20.0).abs() < 1e-6, "{v:?}");
    assert!((u.dot(w) - forge_math::dmath::cos(d.to_radians())).abs() < 1e-9);
    // one face: its area, and its triangles to tint
    let top = read(&pick(Point3::new(10.0, 10.0, 20.0)));
    let area = 40.0 * 40.0 - 0.5 * 20.0 * 20.0;
    assert!(near(top.get(Q::Area), area), "{top:?}");
    let tint: f64 = top
        .shown
        .iter()
        .filter_map(|s| match s {
            Shown::Face(t, _) => Some(t.iter().map(|[a, b, c]| (*b - *a).cross(*c - *a).length() / 2.0).sum::<f64>()),
            _ => None,
        })
        .sum();
    assert!((tint - area).abs() < 1e-6, "{tint}");
}

#[test]
fn a_whole_body_reads_its_volume_mass_and_box_where_it_stands() {
    let (b, m) = part("extrude 20 rect -50 -30 100 60");
    let at = Frame::WORLD.with_origin(Point3::new(100.0, 0.0, 0.0));
    // aluminium: 2.7e-6 kg/mm³
    let w = Target::Whole(Whole {
        body: &b,
        mesh: &m,
        at,
        density: Some(2.7e-6),
    });
    let r = read(&w);
    assert!(near(r.get(Q::Volume), 120_000.0), "{r:?}");
    assert!((r.get(Q::Mass).unwrap() - 0.324).abs() < 1e-9);
    assert!(near(r.get(Q::SizeX), 100.0) && near(r.get(Q::SizeY), 60.0) && near(r.get(Q::SizeZ), 20.0), "{r:?}");
    // two bodies, one moved 150 along x: 50 apart
    let w2 = Target::Whole(Whole {
        body: &b,
        mesh: &m,
        at: Frame::WORLD.with_origin(Point3::new(250.0, 0.0, 0.0)),
        density: None,
    });
    let r = read_pair(&w, &w2);
    assert!(near(r.get(Q::Distance), 50.0), "{r:?}");
}

#[test]
fn an_arc_reads_its_sweep_and_a_circle_none() {
    let (b, m) = part("extrude 20 rect -50 -30 100 60\nfillet 5 (edges along +z)\n");
    let pick = |ent| Pick {
        body: &b,
        mesh: &m,
        at: Frame::WORLD,
        ent,
    };
    let arc = b
        .edge_ids()
        .find(|&e| matches!(&b.edge(e).curve, forge_geom::Curve::Circle(c) if (c.radius - 5.0).abs() < 1e-9))
        .expect("a rounded corner's arc");
    let r = read_one(&pick(Ent::Edge(arc)));
    assert!(near(r.get(Q::Angle), 90.0), "{r:?}");
    assert!(near(r.get(Q::Radius), 5.0));
    // a whole circle: no sweep
    let (b, m) = part(PLATE);
    let r = read_one(&Pick {
        body: &b,
        mesh: &m,
        at: Frame::WORLD,
        ent: rim(&b, 20.0),
    });
    if (r.get(Q::Length).unwrap() - std::f64::consts::TAU * 5.0).abs() < 1e-9 {
        assert!(r.get(Q::Angle).is_none(), "{r:?}");
    }
}

#[test]
fn the_maximum_distance_is_drawn_between_its_two_points() {
    use forge_agent::measure::{Free, Shown, Target, read_pair};
    let circle = |x: f64, rad: f64| Target::Free(Free::Circle(Point3::new(x, 0.0, 0.0), Vec3::Z, rad));
    let r = read_pair(&circle(0.0, 3.0), &circle(20.0, 2.0));
    let line = r.shown.iter().find_map(|s| match s {
        Shown::Line(a, b, Q::MaxDistance) => Some((*a, *b)),
        _ => None,
    });
    let (a, b) = line.expect("the maximum drawn");
    assert!(a.distance(Point3::new(-3.0, 0.0, 0.0)) < 1e-9 && b.distance(Point3::new(22.0, 0.0, 0.0)) < 1e-9);
    // a point off a circle's plane: the far side of the rim, across the centre
    let p = Target::Free(Free::Point(Point3::new(4.0, 0.0, 3.0)));
    let r = read_pair(&circle(0.0, 2.0), &p);
    assert!(near(r.get(Q::MaxDistance), (36.0f64 + 9.0).sqrt()), "{r:?}");
}

#[test]
fn three_points_read_the_angle_at_the_middle_one() {
    use forge_agent::measure::{Free, Shown, Target, read_three};
    let p = |x: f64, y: f64| Target::Free(Free::Point(Point3::new(x, y, 0.0)));
    let r = read_three(&p(10.0, 0.0), &p(0.0, 0.0), &p(0.0, 5.0));
    assert!(near(r.get(Q::Angle), 90.0), "{r:?}");
    assert!(r.shown.iter().any(|s| matches!(s, Shown::Angle(v, ..) if v.distance(Point3::ORIGIN) < 1e-12)));
    let r = read_three(&p(1.0, 0.0), &p(0.0, 0.0), &p(1.0, 1.0));
    assert!(near(r.get(Q::Angle), 45.0), "{r:?}");
}
