use forge_bool::{Op, boolean, boolean_into, unify};
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
    // (in place: `boolean` hands the result out unified already)
    let mut joined = a.clone();
    boolean_into(&mut joined, &b, Op::Union, 3).unwrap();
    // (the decide-once path writes the union unified: one box at once)
    #[cfg(not(feature = "decide_once"))]
    assert!(joined.face_ids().count() > 6);
    assert_eq!(counts(&boolean(&a, &b, Op::Union, 3).unwrap()), (8, 12, 6));
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
/// round it, not of the bore.
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

/// A face that is nothing but a tree of edges, each used by it on both sides (zero area): a
/// boolean left one, the V of a drill point's tip imprinted twice, a hair apart, on a plane through
/// its apex (Haas GM2-5AX, solid #650885, intersected with itself shifted; research/booleans/
/// drift.md §10). `unify` dropped its loop as a slit and left the face with none (`check`:
/// FaceNoLoops); the face goes with its loops.
#[test]
fn a_face_of_slits_alone_goes_whole() {
    use forge_geom::{Curve, Line};
    use forge_math::{Interval, UnitVec3};
    use forge_topo::Name;
    let mut b = block(0.0, 0.0, 10.0, 6.0, 4.0, 1);
    let top = b
        .face_ids()
        .find(|&f| matches!(&b.face(f).surface, Surface::Plane(p) if p.frame.origin.z > 3.9))
        .expect("the block's top");
    let surface = b.face(top).surface.clone();
    let sense = b.face(top).same_sense;
    let v = |b: &mut Body, x: f64, y: f64| b.add_vertex(Point3::new(x, y, 4.0), Name::feature(9));
    let (apex, a1, a2) = (v(&mut b, 5.0, 3.0), v(&mut b, 6.0, 2.0), v(&mut b, 6.0, 4.0));
    let line = |b: &mut Body, from: forge_topo::VertexId, to: forge_topo::VertexId, k: u64| {
        let (p, q) = (b.vertex(from).point, b.vertex(to).point);
        let dir = UnitVec3::new_unchecked((q - p) * (1.0 / p.distance(q)));
        b.add_edge(
            Curve::Line(Line::new(p, dir)),
            Interval::new(0.0, p.distance(q)),
            from,
            to,
            Name::feature(10 + k),
        )
    };
    let (e1, e2) = (line(&mut b, apex, a1, 1), line(&mut b, apex, a2, 2));
    let shell = forge_topo::ShellId(0);
    let f = b.add_face(shell, surface, sense, Name::feature(20));
    b.add_loop(f, &[(e1, true), (e1, false), (e2, true), (e2, false)]);
    let before = b.face_ids().count();
    let r = unified(b);
    assert_eq!(r.face_ids().count(), before - 1, "the slit face is gone");
}

/// A cone to its apex in two halves, one of the rulings between them split in two by a vertex (as
/// a boolean's imprint leaves it, Haas TRT210 #65625): the halves stay apart. Joined across the
/// piece off the apex, the piece at the apex went too, walked there and back, and the cone was one
/// face bounded by its circle alone (FaceWrongSide, no region for mass or mesh).
#[test]
fn a_split_ruling_keeps_a_cones_tip_in_halves() {
    use forge_geom::CurveGeom;
    let pts = [(0.0, 0.0), (5.0, 0.0), (0.0, 10.0)];
    let mirrored: Vec<(f64, f64)> = pts.iter().map(|p| (-p.0, p.1)).collect();
    let mut halves = boolean(
        &revolved_by(&pts, forge_math::dmath::PI, 1),
        &revolved_by(&mirrored, forge_math::dmath::PI, 2),
        Op::Union,
        3,
    )
    .unwrap();
    let cones = |b: &Body| {
        b.face_ids()
            .filter(|&f| matches!(b.face(f).surface, Surface::Cone(_)))
            .count()
    };
    assert_eq!(cones(&halves), 2);
    // a ruling between the two half cones
    let e = halves
        .edge_ids()
        .find(|&e| {
            let x = halves.edge(e);
            matches!(x.curve, forge_geom::Curve::Line(_))
                && x.coedges.len() == 2
                && x.coedges
                    .iter()
                    .all(|&c| matches!(halves.face(halves.coedge_face(c)).surface, Surface::Cone(_)))
        })
        .expect("a ruling between the halves");
    let x = halves.edge(e).clone();
    let mid = x.range.mid();
    let m = halves.add_vertex(x.curve.eval(mid), forge_topo::Name::feature(901));
    let lo = halves.add_edge(
        x.curve.clone(),
        forge_math::Interval::new(x.range.lo, mid),
        x.start,
        m,
        forge_topo::Name::feature(902),
    );
    let hi = halves.add_edge(
        x.curve.clone(),
        forge_math::Interval::new(mid, x.range.hi),
        m,
        x.end,
        forge_topo::Name::feature(903),
    );
    let swaps: Vec<_> = x
        .coedges
        .iter()
        .map(|&c| {
            let lp = halves.coedge(c).lp;
            let f = halves.coedge_face(c);
            let uses: Vec<(forge_topo::EdgeId, bool)> = halves
                .lp(lp)
                .coedges()
                .flat_map(|d| {
                    let u = (halves.coedge(d).edge, halves.coedge(d).forward);
                    match (u.0 == e, u.1) {
                        (false, _) => vec![u],
                        (true, true) => vec![(lo, true), (hi, true)],
                        (true, false) => vec![(hi, false), (lo, false)],
                    }
                })
                .collect();
            (f, lp, uses)
        })
        .collect();
    for (f, lp, uses) in swaps {
        halves.replace_loop(f, lp, &uses);
    }
    let u = unified(halves);
    assert_eq!(cones(&u), 2);
}
