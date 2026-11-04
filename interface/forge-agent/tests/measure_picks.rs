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
