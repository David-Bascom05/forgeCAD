use forge_bool::{Op, boolean};
use forge_draw::{Class, Instance, Options, Piece, View, hlr};
use forge_math::{Frame, Point2, Point3, Vec3, dmath};
use forge_ops::{Profile, ProfileLoop, Segment, extrude, revolve};
use forge_topo::Body;

fn block(origin: Point3, w: f64, h: f64, d: f64, feature: u64) -> Body {
    let plane = Frame::WORLD.with_origin(origin);
    let loops = vec![Profile::rectangle(Point2::ORIGIN, w, h, 0)];
    extrude(&Profile { plane, loops }, d, feature).unwrap()
}

fn rod(centre: Point3, r: f64, len: f64, feature: u64) -> Body {
    let plane = Frame::WORLD.with_origin(centre);
    let loops = vec![Profile::circle(Point2::ORIGIN, r, 0)];
    extrude(&Profile { plane, loops }, len, feature).unwrap()
}

fn ball(centre: Point3, r: f64, feature: u64) -> Body {
    let q = dmath::FRAC_PI_2;
    let half = ProfileLoop::new(vec![
        (
            Segment::Arc {
                center: Point2::ORIGIN,
                radius: r,
                start: -q,
                sweep: 2.0 * q,
            },
            0,
        ),
        (
            Segment::Line {
                a: Point2::new(0.0, r),
                b: Point2::new(0.0, -r),
            },
            1,
        ),
    ]);
    let plane = Frame::from_zx(centre, Vec3::Z.normalize().unwrap(), Vec3::Y).unwrap();
    revolve(
        &Profile {
            plane,
            loops: vec![half],
        },
        forge_ops::Axis2 {
            origin: Point2::ORIGIN,
            dir: forge_math::Vec2::new(0.0, 1.0),
        },
        dmath::TAU,
        feature,
    )
    .unwrap()
}

/// A ring: a circle of radius `r` centred `big` from the z axis, swept round it.
fn ring(big: f64, r: f64, feature: u64) -> Body {
    let plane = Frame::from_zx(
        Point3::ORIGIN,
        Vec3::new(0.0, -1.0, 0.0).normalize().unwrap(),
        Vec3::X,
    )
    .unwrap();
    let loops = vec![Profile::circle(Point2::new(big, 0.0), r, 0)];
    revolve(
        &Profile { plane, loops },
        forge_ops::Axis2 {
            origin: Point2::ORIGIN,
            dir: forge_math::Vec2::new(0.0, 1.0),
        },
        dmath::TAU,
        feature,
    )
    .unwrap()
}

fn lengths(p: &[Piece]) -> (f64, f64) {
    let seen = p.iter().filter(|x| x.visible).map(|x| x.geom.length()).sum();
    let hidden = p.iter().filter(|x| !x.visible).map(|x| x.geom.length()).sum();
    (seen, hidden)
}

fn view(insts: &[Instance], name: &str) -> Vec<Piece> {
    hlr(insts, &View::named(name).unwrap(), &Options::default())
}

#[test]
fn block_front_is_its_rectangle() {
    let b = block(Point3::ORIGIN, 10.0, 20.0, 5.0, 1);
    let insts = [Instance {
        body: &b,
        pose: Frame::WORLD,
    }];
    let (seen, hidden) = lengths(&view(&insts, "front"));
    assert!((seen - 30.0).abs() < 1e-6, "{seen}");
    assert!(hidden.abs() < 1e-6, "{hidden}");
    let (seen, hidden) = lengths(&view(&insts, "top"));
    assert!(
        (seen - 60.0).abs() < 1e-6 && hidden.abs() < 1e-6,
        "{seen} {hidden}"
    );
    // iso: 9 of the 12 edges seen, 3 hidden
    let (seen, hidden) = lengths(&view(&insts, "iso"));
    let k = (2.0f64 / 3.0).sqrt();
    let s = |x: f64, y: f64, z: f64| {
        let v = View::named("iso").unwrap();
        v.project_vec(Vec3::new(x, y, z)).length()
    };
    let want_hidden = s(10.0, 0.0, 0.0) + s(0.0, 20.0, 0.0) + s(0.0, 0.0, 5.0);
    let _ = k;
    assert!((hidden - want_hidden).abs() < 1e-6, "{hidden} vs {want_hidden}");
    let want_seen = 2.0 * (s(10.0, 0.0, 0.0) + s(0.0, 20.0, 0.0) + s(0.0, 0.0, 5.0)) + want_hidden;
    assert!((seen - want_seen).abs() < 1e-6, "{seen} vs {want_seen}");
}

#[test]
fn hole_shows_hidden_in_front_and_a_circle_on_top() {
    let b = block(Point3::ORIGIN, 10.0, 20.0, 5.0, 1);
    let h = rod(Point3::new(5.0, 10.0, -1.0), 2.0, 7.0, 2);
    let b = boolean(&b, &h, Op::Subtract, 3).unwrap();
    let insts = [Instance {
        body: &b,
        pose: Frame::WORLD,
    }];
    let p = view(&insts, "front");
    let (seen, hidden) = lengths(&p);
    assert!((seen - 30.0).abs() < 1e-6, "{seen}");
    assert!((hidden - 10.0).abs() < 1e-6, "{hidden}");
    assert!(p.iter().filter(|x| !x.visible).all(|x| x.class == Class::Outline));
    let (seen, hidden) = lengths(&view(&insts, "top"));
    assert!((seen - 60.0 - 4.0 * dmath::PI).abs() < 1e-3, "{seen}");
    assert!(hidden.abs() < 1e-6);
}

#[test]
fn rod_and_ball_outlines() {
    let r = rod(Point3::ORIGIN, 3.0, 10.0, 1);
    let insts = [Instance {
        body: &r,
        pose: Frame::WORLD,
    }];
    let (seen, hidden) = lengths(&view(&insts, "front"));
    assert!(
        (seen - 32.0).abs() < 1e-4 && hidden.abs() < 1e-6,
        "{seen} {hidden}"
    );
    let b = ball(Point3::new(1.0, 2.0, 3.0), 4.0, 1);
    let insts = [Instance {
        body: &b,
        pose: Frame::WORLD,
    }];
    for name in ["front", "iso", "top"] {
        let p = view(&insts, name);
        let (seen, _) = lengths(&p);
        let outline: f64 = p
            .iter()
            .filter(|x| x.visible && x.class == Class::Outline)
            .map(|x| x.geom.length())
            .sum();
        assert!(
            (outline - 8.0 * dmath::PI).abs() < 1e-3,
            "{name}: {outline} {seen}"
        );
    }
}

#[test]
fn ring_outline_seen_from_above_and_aslant() {
    let t = ring(10.0, 2.0, 1);
    let insts = [Instance {
        body: &t,
        pose: Frame::WORLD,
    }];
    // from above: circles of radius 12 and 8
    let p = view(&insts, "top");
    let outline: f64 = p
        .iter()
        .filter(|x| x.visible && x.class == Class::Outline)
        .map(|x| x.geom.length())
        .sum();
    assert!((outline - 40.0 * dmath::PI).abs() < 1e-2, "{outline}");
    // from the front: the rims, two lines of length 20, and the tube's two sections, their outer
    // halves seen and inner halves hidden behind the tube's near side
    let p = view(&insts, "front");
    let (seen, hidden) = lengths(&p);
    let want = 4.0 * dmath::PI + 2.0 * 20.0;
    // where a section touches a rim it lies within a micrometre of it for a few micrometres: one line there
    assert!((seen - want).abs() < 1e-3, "{seen} vs {want}");
    assert!((hidden - 4.0 * dmath::PI).abs() < 2e-2, "{hidden}");
}

#[test]
fn one_part_hides_another() {
    let a = block(Point3::ORIGIN, 10.0, 10.0, 10.0, 1);
    let b = block(Point3::new(2.0, 20.0, 2.0), 6.0, 6.0, 6.0, 1);
    let insts = [
        Instance {
            body: &a,
            pose: Frame::WORLD,
        },
        Instance {
            body: &b,
            pose: Frame::WORLD,
        },
    ];
    let (seen, hidden) = lengths(&view(&insts, "front"));
    assert!((seen - 40.0).abs() < 1e-6, "{seen}");
    assert!((hidden - 24.0).abs() < 1e-6, "{hidden}");
    // the same part placed by a pose
    let pose = Frame::WORLD.with_origin(Point3::new(2.0, 20.0, 2.0));
    let c = block(Point3::ORIGIN, 6.0, 6.0, 6.0, 1);
    let insts = [
        Instance {
            body: &a,
            pose: Frame::WORLD,
        },
        Instance { body: &c, pose },
    ];
    let (seen, hidden) = lengths(&view(&insts, "front"));
    assert!(
        (seen - 40.0).abs() < 1e-6 && (hidden - 24.0).abs() < 1e-6,
        "{seen} {hidden}"
    );
}

#[test]
fn notch_seen_edge_on() {
    // a half-round notch in the block's side, seen along the notch's flat: its arcs edge-on
    let b = block(Point3::ORIGIN, 40.0, 20.0, 3.0, 1);
    let n = rod(Point3::new(20.0, 0.0, -1.0), 4.0, 5.0, 2);
    let b = boolean(&b, &n, Op::Subtract, 3).unwrap();
    let insts = [Instance {
        body: &b,
        pose: Frame::WORLD,
    }];
    let p = view(&insts, "right");
    let (seen, hidden) = lengths(&p);
    // right view: the block's 20 x 3 end, the notch's arcs edge-on along the bottom and top
    assert!((seen - 46.0).abs() < 1e-6, "{seen}");
    // the notch's far ruling, behind the block
    assert!((hidden - 3.0).abs() < 1e-6, "{hidden}");
    let p = view(&insts, "front");
    let (seen, hidden) = lengths(&p);
    // front: the block's outline and the notch's two sides
    assert!((seen - 2.0 * 40.0 - 2.0 * 3.0 - 2.0 * 3.0).abs() < 1e-6, "{seen}");
    assert!(hidden.abs() < 1e-6, "{hidden}");
}
