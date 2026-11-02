//! Two rounds meeting at a sharp edge between walls all but tangent.

use forge_geom::{CurveGeom, Surface, SurfaceGeom};
use forge_math::dmath::{cos, sin};
use forge_math::{Frame, Point2, Point3, UnitVec3};
use forge_mesh::{Quality, tessellate};
use forge_ops::{FilletEdge, LoftGuides, LoftKind, Profile, ProfileLoop, Radius, Segment, fillet, loft};
use forge_topo::{Body, FaceId};
use std::f64::consts::{FRAC_PI_2, PI};

/// A section with its corner at (x, y): an arc of radius `ra` runs into the corner, turning half a
/// degree left there onto an arc of radius `rb`, each 30 long; two lines close it, leaving acute
/// corners at the arcs' far ends.
fn section(z: f64, x: f64, y: f64, ra: f64, rb: f64) -> Profile {
    let half = 0.25 * PI / 180.0;
    let (sa, sb) = (30.0 / ra, 30.0 / rb);
    let centre = |h: f64, r: f64| Point2::new(x + r * cos(h + FRAC_PI_2), y + r * sin(h + FRAC_PI_2));
    let at = |c: Point2, r: f64, t: f64| Point2::new(c.x + r * cos(t), c.y + r * sin(t));
    let (ca, cb) = (centre(-half, ra), centre(half, rb));
    let (a0, b0) = (-half - FRAC_PI_2 - sa, half - FRAC_PI_2);
    let apex = Point2::new(x + 16.0, y + 16.0);
    let arc = |center, radius, start, sweep| Segment::Arc {
        center,
        radius,
        start,
        sweep,
    };
    Profile {
        plane: Frame::from_z(Point3::new(0.0, 0.0, z), UnitVec3::Z),
        loops: vec![ProfileLoop::new(vec![
            (arc(ca, ra, a0, sa), 0),
            (arc(cb, rb, b0, sb), 1),
            (
                Segment::Line {
                    a: at(cb, rb, b0 + sb),
                    b: apex,
                },
                2,
            ),
            (
                Segment::Line {
                    a: apex,
                    b: at(ca, ra, a0),
                },
                3,
            ),
        ])],
    }
}

/// Lofted so the corner leans back along the arcs. Arcs of different radius, near tangent, cross
/// twice: past the leaning corner, the first wall's spring meets the second wall's surface once on
/// the corner and again beside it, nearer the vertex, and the springs on the floor cross twice too.
fn block() -> Body {
    let sections = [
        section(0.0, 0.0, 0.0, 30.0, 80.0),
        section(30.0, -12.0, -3.0, 40.0, 70.0),
    ];
    loft(&sections, LoftKind::Ruled, &LoftGuides::None, 1).expect("loft")
}

/// Against OCCT for the volume (its surfaces stand off the true ball by up to 0.015 at r 6, see
/// research/fillets/README.md), and against the ball itself: a point of the round stepped r along
/// its normal must stand r from the floor and r from a wall.
#[test]
fn rounds_meet_on_the_corner_between_near_tangent_walls() {
    let b = block();
    let v0 = forge_topo::mass::mass(&b).expect("mass").volume;
    let floor = |p: Point3| p.z.abs() < 1e-9;
    let rim: Vec<_> = b
        .edge_ids()
        .filter(|&e| {
            let x = b.edge(e);
            let mid = x.curve.eval(x.range.mid());
            floor(x.curve.eval(x.range.lo)) && floor(mid) && mid.x > -10.0
        })
        .collect();
    assert_eq!(rim.len(), 2);
    let ends: Vec<Point3> = rim
        .iter()
        .flat_map(|&e| {
            let x = b.edge(e);
            [x.curve.eval(x.range.lo), x.curve.eval(x.range.hi)]
        })
        .filter(|p| p.x * p.x + p.y * p.y > 1.0)
        .collect();
    assert_eq!(ends.len(), 2);
    let walls: Vec<FaceId> = rim
        .iter()
        .flat_map(|&e| {
            b.edge(e)
                .coedges
                .iter()
                .map(|&c| b.coedge_face(c))
                .collect::<Vec<_>>()
        })
        .filter(|&f| !matches!(b.face(f).surface, Surface::Plane(_)))
        .collect();
    assert_eq!(walls.len(), 2);
    for (r, occt) in [(2.0, 37.506), (4.0, 148.34), (6.0, 329.887)] {
        let picks: Vec<FilletEdge> = rim
            .iter()
            .map(|&edge| FilletEdge {
                edge,
                radius: Radius::Constant(r),
            })
            .collect();
        let f = fillet(&b, &picks, 2).expect("rounds");
        assert!(forge_topo::check(&f).is_empty());
        let mesh = tessellate(&f, &Quality::default()).expect("meshes");
        assert!(mesh.open_edges().is_empty());
        let v = forge_topo::mass::mass(&f).expect("mass").volume;
        assert!(
            ((v0 - v) - occt).abs() < 3e-3 * occt,
            "r {r}: took {}, OCCT {occt}",
            v0 - v
        );
        assert!(
            (v - mesh.volume()).abs() < 1e-3 * v,
            "r {r}: volume {v}, mesh {}",
            mesh.volume()
        );
        let old: Vec<_> = b.face_ids().map(|g| b.face(g).name).collect();
        let mut seen = 0;
        for (i, t) in mesh.triangles.iter().enumerate() {
            let g = FaceId(mesh.face_of[i]);
            let q = mesh.positions[t[0] as usize];
            if old.contains(&f.face(g).name) || ends.iter().any(|e| e.distance(q) < 2.5 * r) {
                continue;
            }
            let s = &f.face(g).surface;
            let n = s.normal(s.project(q)).expect("normal");
            // the side of the round the ball is on: on a leaning wall, the normal is nearly level
            let c = [q + n * r, q - n * r]
                .into_iter()
                .min_by(|a, b| (a.z - r).abs().total_cmp(&(b.z - r).abs()))
                .expect("two");
            let off = walls
                .iter()
                .map(|&w| {
                    let s = &b.face(w).surface;
                    (s.eval(s.project(c)).distance(c) - r).abs()
                })
                .fold(f64::INFINITY, f64::min);
            assert!(
                (c.z - r).abs() < 1e-7,
                "r {r}: centre at height {} from {q:?}",
                c.z
            );
            assert!(off < 1e-7, "r {r}: centre {off} off its wall from {q:?}");
            seen += 1;
        }
        assert!(seen > 100, "r {r}: only {seen} points");
    }
}
