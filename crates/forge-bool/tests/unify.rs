use forge_bool::{Op, boolean, unify};
use forge_geom::Surface;
use forge_math::{Frame, Point2, Point3, Vec2};
use forge_mesh::{Quality, tessellate};
use forge_ops::{Axis2, Profile, ProfileLoop, Segment, extrude, revolve};
use forge_topo::{Body, check};

fn block(x: f64, y: f64, w: f64, h: f64, d: f64, feature: u64) -> Body {
    let plane = Frame::WORLD.with_origin(Point3::new(x, y, 0.0));
    let loops = vec![Profile::rectangle(Point2::ORIGIN, w, h, 0)];
    extrude(&Profile { plane, loops }, d, feature).unwrap()
}

fn unified(mut b: Body) -> Body {
    unify(&mut b);
    // compacted in place, the same as copied
    let in_place = b.clone().compact_in_shells();
    let b = b.compact();
    assert!(in_place == b);
    assert!(check(&b).is_empty(), "{:?}", check(&b));
    b
}

fn volume(b: &Body) -> f64 {
    let q = Quality {
        chord: 0.01,
        max_angle: 0.2,
    };
    tessellate(b, &q).expect("meshes").volume()
}

fn counts(b: &Body) -> (usize, usize, usize) {
    (b.vertices.len(), b.edge_ids().count(), b.face_ids().count())
}

#[test]
fn blocks_side_by_side_are_one_box() {
    let (a, b) = (
        block(0.0, 0.0, 10.0, 6.0, 4.0, 1),
        block(10.0, 0.0, 5.0, 6.0, 4.0, 2),
    );
    let joined = boolean(&a, &b, Op::Union, 3).unwrap();
    assert!(joined.face_ids().count() > 6);
    let pieces: Vec<_> = joined.face_ids().map(|f| joined.face(f).name).collect();
    let u = unified(joined);
    assert_eq!(counts(&u), (8, 12, 6));
    assert!((volume(&u) - 15.0 * 6.0 * 4.0).abs() < 1e-6);
    // every piece's name still finds its face, the whole taking the least of its pieces' names
    for n in pieces {
        let f = u.face_by_name(n).expect("a piece's name resolves");
        assert!(u.face(f).name <= n);
    }
}

#[test]
fn a_ring_of_blocks_has_one_top_with_a_hole() {
    let mut b = block(0.0, 0.0, 30.0, 10.0, 5.0, 1);
    for (k, (x, y, w, h)) in [
        (0.0, 10.0, 10.0, 10.0),
        (20.0, 10.0, 10.0, 10.0),
        (0.0, 20.0, 30.0, 10.0),
    ]
    .into_iter()
    .enumerate()
    {
        b = boolean(
            &b,
            &block(x, y, w, h, 5.0, 2 + k as u64),
            Op::Union,
            10 + k as u64,
        )
        .unwrap();
    }
    let u = unified(b);
    // top, bottom, four outside walls, four inside walls
    assert_eq!(u.face_ids().count(), 10);
    for f in u.face_ids() {
        let face = u.face(f);
        if let Surface::Plane(p) = &face.surface
            && p.frame.z().get().z.abs() > 0.5
        {
            assert_eq!(face.loops.len(), 2);
            // the outer loop first: 4 edges each, so look at its size
            let outer = u.lp(face.loops[0]).coedges().map(|c| u.coedge(c).edge);
            let len: f64 = outer.map(|e| u.edge(e).range.width()).sum();
            assert!(
                (len - 2.0 * (30.0 + 30.0)).abs() < 1e-9,
                "outer loop first: {len}"
            );
        }
    }
    assert!((volume(&u) - (900.0 - 100.0) * 5.0).abs() < 1e-6);
}

/// Two half-turn revolves of a profile with cones in it, one each side of the axis, added: the
/// cylinder and the frustum each join into one face with a seam, the disc and ring into one with
/// none; the cone to its apex stays in halves (one face there would have no seam).
#[test]
fn revolved_halves_join() {
    let pts = [
        (0.0, 0.0),
        (3.0, 2.0),
        (3.0, 8.0),
        (5.0, 8.0),
        (7.0, 12.0),
        (0.0, 12.0),
    ];
    let mirrored: Vec<(f64, f64)> = pts.iter().map(|p| (-p.0, p.1)).collect();
    let halves = boolean(
        &revolved_by(&pts, forge_math::dmath::PI, 1),
        &revolved_by(&mirrored, forge_math::dmath::PI, 2),
        Op::Union,
        3,
    )
    .unwrap();
    let whole = revolved(&pts, 4);
    let u = unified(halves);
    // two half cones, cylinder, ring, frustum, end disc
    assert_eq!((u.face_ids().count(), whole.face_ids().count()), (6, 5));
    assert!((volume(&u) - volume(&whole)).abs() < 1e-3 * volume(&whole));
    let seamed: Vec<&Surface> = u
        .face_ids()
        .filter(|&f| {
            u.face(f)
                .loops
                .iter()
                .flat_map(|&l| u.lp(l).coedges())
                .any(|c| u.partner(c).is_some_and(|p| u.coedge_face(p) == f))
        })
        .map(|f| &u.face(f).surface)
        .collect();
    assert!(
        matches!(
            seamed[..],
            [Surface::Cylinder(_), Surface::Cone(_)] | [Surface::Cone(_), Surface::Cylinder(_)]
        ),
        "{seamed:?}"
    );
}

/// A whole-turn cone (one face, its seam ending at the apex) cut by a wedge whose two sides hold
/// its axis: the face is split along two lines through the apex, besides the seam, so the way on
/// at the apex is chosen among three.
#[test]
fn a_wedge_through_a_cones_apex() {
    let cone = || revolved(&[(0.0, 0.0), (5.0, 0.0), (0.0, 10.0)], 1);
    let pts = [(0.0, 0.0), (20.0, 5.0), (20.0, 15.0)];
    let segs = (0..3)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % 3]);
            let seg = Segment::Line {
                a: Point2::new(a.0, a.1),
                b: Point2::new(b.0, b.1),
            };
            (seg, i as u64)
        })
        .collect();
    let plane = Frame::from_axes(
        Point3::new(0.0, -1.0, 0.0),
        forge_math::Vec3::new(1.0, 0.0, 0.0),
        forge_math::Vec3::new(0.0, 0.0, -1.0),
        forge_math::Vec3::new(0.0, 1.0, 0.0),
    )
    .unwrap();
    let wedge = extrude(
        &Profile {
            plane,
            loops: vec![ProfileLoop::new(segs)],
        },
        12.0,
        2,
    )
    .unwrap();
    // (as meshed, a little under the cone's exact volume)
    let whole = volume(&cone());
    let share = (forge_math::dmath::atan(0.75) - forge_math::dmath::atan(0.25)) / forge_math::dmath::TAU;
    for (op, want) in [(Op::Subtract, 1.0 - share), (Op::Intersect, share)] {
        let b = boolean(&cone(), &wedge, op, 3).unwrap();
        assert!(check(&b).is_empty(), "{op:?} {:?}", check(&b));
        assert!(
            (volume(&b) - want * whole).abs() < 1e-3 * whole,
            "{op:?} {}",
            volume(&b) / whole
        );
    }
}

fn revolved(pts: &[(f64, f64)], feature: u64) -> Body {
    revolved_by(pts, forge_math::dmath::TAU, feature)
}

fn revolved_by(pts: &[(f64, f64)], angle: f64, feature: u64) -> Body {
    let segs = (0..pts.len())
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            let seg = Segment::Line {
                a: Point2::new(a.0, a.1),
                b: Point2::new(b.0, b.1),
            };
            (seg, i as u64)
        })
        .collect();
    let profile = Profile {
        plane: Frame::WORLD,
        loops: vec![ProfileLoop::new(segs)],
    };
    let axis = Axis2 {
        origin: Point2::ORIGIN,
        dir: Vec2::new(0.0, 1.0),
    };
    revolve(&profile, axis, angle, feature).unwrap()
}

/// A tube's end disc joined whole, cut by a bore ending on it and a seat flush with it in one
/// n-ary boolean: the seat's circle, a chord's sag outside the bore's, is a hole of the ring
/// round it, not of the bore (forgecad-gate/repros/nary-cut-into-joined-ring.fc).
#[test]
fn seat_cut_into_a_joined_tube_end() {
    let tube = unified(revolved(
        &[(0.0, 0.0), (25.0, 0.0), (30.0, 160.0), (0.0, 160.0)],
        1,
    ));
    let bore = unified(revolved(
        &[(0.0, 0.0), (23.5, 0.0), (28.5, 160.0), (0.0, 160.0)],
        2,
    ));
    let plane = Frame::from_z(Point3::new(0.0, 149.0, 0.0), forge_math::UnitVec3::Y);
    let seat = Profile {
        plane,
        loops: vec![Profile::circle(Point2::ORIGIN, 29.0, 0)],
    };
    let seat = extrude(&seat, 11.0, 3).unwrap();
    let r = forge_bool::nary(&[&tube, &bore, &seat], &[Op::Subtract, Op::Subtract], 1).unwrap();
    assert!(check(&r).is_empty(), "{:?}", check(&r));
    let seq = boolean(
        &boolean(&tube, &bore, Op::Subtract, 4).unwrap(),
        &seat,
        Op::Subtract,
        5,
    )
    .unwrap();
    assert!((volume(&r) - volume(&seq)).abs() < 1e-6 * volume(&seq));
}
