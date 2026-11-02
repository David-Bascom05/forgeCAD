//! Rounds whose ball rests on a sharp rim of no simple shape: an ellipse, where a sloping top cuts a
//! round wall.

use forge_bool::{Op, boolean};
use forge_geom::{CurveGeom, Surface, SurfaceGeom};
use forge_math::{Frame, Point2, Point3, UnitVec3, Vec3};
use forge_mesh::{Quality, tessellate};
use forge_ops::{FilletEdge, Profile, ProfileLoop, Radius, Segment, extrude, fillet};
use forge_topo::Body;

/// The top: its height where the straight wall meets the round end, at x = 20, and how it falls.
#[derive(Clone, Copy)]
struct Top(f64, f64);

impl Top {
    fn at(&self, x: f64) -> f64 {
        self.0 + self.1 * (x - 20.0)
    }
}

/// A wall along x, 10 thick, ending in a half round, its top a plane falling along x, on a plate.
fn scene(top: Top) -> Body {
    let pi = std::f64::consts::PI;
    let at = |z: f64| Frame::from_zx(Point3::new(0.0, 0.0, z), UnitVec3::Z, Vec3::X).expect("frame");
    let line = |a: (f64, f64), b: (f64, f64), id| {
        (
            Segment::Line {
                a: Point2::new(a.0, a.1),
                b: Point2::new(b.0, b.1),
            },
            id,
        )
    };
    let plate = extrude(
        &Profile {
            plane: at(-5.0),
            loops: vec![Profile::rectangle(Point2::new(-30.0, -40.0), 100.0, 80.0, 0)],
        },
        5.0,
        1,
    )
    .expect("plate");
    let wall = extrude(
        &Profile {
            plane: at(0.0),
            loops: vec![ProfileLoop::new(vec![
                line((0.0, -5.0), (20.0, -5.0), 0),
                (
                    Segment::Arc {
                        center: Point2::new(20.0, 0.0),
                        radius: 5.0,
                        start: -pi / 2.0,
                        sweep: pi,
                    },
                    1,
                ),
                line((20.0, 5.0), (0.0, 5.0), 2),
                line((0.0, 5.0), (0.0, -5.0), 3),
            ])],
        },
        30.0,
        2,
    )
    .expect("wall");
    let above = extrude(
        &Profile {
            plane: Frame::from_zx(Point3::new(0.0, -20.0, 0.0), -UnitVec3::Y, Vec3::X).expect("frame"),
            loops: vec![ProfileLoop::new(vec![
                line((-40.0, top.at(-40.0)), (40.0, top.at(40.0)), 0),
                line((40.0, top.at(40.0)), (40.0, 60.0), 1),
                line((40.0, 60.0), (-40.0, 60.0), 2),
                line((-40.0, 60.0), (-40.0, top.at(-40.0)), 3),
            ])],
        },
        -40.0,
        5,
    )
    .expect("above");
    let cut = boolean(&wall, &above, Op::Subtract, 6).expect("cut");
    boolean(&plate, &cut, Op::Union, 3).expect("union")
}

/// The distance to the wall, worked out from its shape alone: the nearest point of two convex sets'
/// meet, the round footprint's prism and what lies under the top, by Dykstra's projections.
fn to_wall(top: Top, c: Point3) -> f64 {
    let footprint = |p: Point3| {
        let (x, y) = (p.x.max(0.0), p.y);
        let (x, y) = if x <= 20.0 {
            (x, y.clamp(-5.0, 5.0))
        } else {
            let dx = x - 20.0;
            let l = (dx * dx + y * y).sqrt();
            let k = (5.0 / l).min(1.0);
            (20.0 + dx * k, y * k)
        };
        Point3::new(x, y, p.z)
    };
    let under = |p: Point3| {
        let n = Vec3::new(-top.1, 0.0, 1.0);
        let over = (p.z - top.at(p.x)) / n.dot(n);
        if over > 0.0 { p - n * over } else { p }
    };
    let (mut p, mut a, mut b) = (c, Vec3::ZERO, Vec3::ZERO);
    for _ in 0..4000 {
        let q = under(p + a);
        a = p + a - q;
        p = footprint(q + b);
        b = q + b - p;
    }
    c.distance(p)
}

/// Every ball the round's surface says it holds touches the plate and the wall at its radius, with
/// nothing in common with the kernel but the point it starts from.
fn rounds_with_true_balls(top: Top, r: f64, want: f64) {
    let body = scene(top);
    let foot: Vec<FilletEdge> = body
        .edge_ids()
        .filter(|&e| {
            let x = body.edge(e);
            [x.range.lo, x.range.mid()]
                .iter()
                .all(|&t| x.curve.eval(t).z.abs() < 1e-9)
                && forge_ops::fillet_kind(&body, e) == Some(false)
        })
        .map(|edge| FilletEdge {
            edge,
            radius: Radius::Constant(r),
        })
        .collect();
    let b = fillet(&body, &foot, 9).expect("rounds");
    assert!(forge_topo::check(&b).is_empty());
    let volume = |x: &Body| forge_topo::mass::mass(x).expect("mass").volume;
    let added = volume(&b) - volume(&body);
    assert!((added - want).abs() < 0.01, "added {added}");

    let mut seen = 0;
    for f in b.face_ids() {
        let face = b.face(f);
        if !matches!(face.surface, Surface::Procedural(_) | Surface::Torus(_)) {
            continue;
        }
        let loops = forge_topo::uv::face_uv_loops(&b, f);
        let (mut lo, mut hi) = (Point2::new(f64::MAX, f64::MAX), Point2::new(f64::MIN, f64::MIN));
        for p in loops.iter().flat_map(|l| l.points.iter()) {
            lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
        }
        for i in 1..40 {
            for j in 1..12 {
                let uv = Point2::new(
                    lo.x + (hi.x - lo.x) * f64::from(i) / 40.0,
                    lo.y + (hi.y - lo.y) * f64::from(j) / 12.0,
                );
                if !forge_topo::uv::point_in_loops(uv, &loops) {
                    continue;
                }
                let n = b.face_normal(f, uv).expect("normal");
                let c = face.surface.eval(uv) + n.get() * r;
                assert!((c.z - r).abs() < 1e-5, "off the plate by {} at {c:?}", c.z - r);
                let d = to_wall(top, c);
                assert!((d - r).abs() < 1e-4, "{d} from the wall at {c:?}");
                seen += 1;
            }
        }
    }
    assert!(seen > 500, "{seen}");

    let fine = Quality {
        chord: 0.01,
        ..Default::default()
    };
    let mesh = tessellate(&b, &fine).expect("meshes");
    assert!(mesh.open_edges().is_empty());
    assert!((mesh.volume() - volume(&b)).abs() < 1e-3 * volume(&b));
}

/// The ball comes to the rim on the straight wall and stays on it round the end. A ball's closing on
/// a grid (research/fillets/closing4.rs) goes to about 111 as its cells shrink, and the mitres at the
/// far end's two corners add 1.1 to what it counts.
#[test]
fn ball_on_a_line_and_an_ellipse_of_a_sloping_top() {
    rounds_with_true_balls(Top(2.0, -0.3), 3.0, 113.7552);
}
