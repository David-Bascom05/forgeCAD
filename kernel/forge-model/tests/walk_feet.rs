//! A round on a lofted wall traces against an offset of it, whose every value is a search of the
//! wall. The search carries on from the walk's last parameters, so what a thread built before must
//! not reach the answer: built alone, or after another round on the same thread, or elsewhere at
//! once, the body has the same bits.
use forge_bool::{Op, boolean};
use forge_math::{Frame, Point2, Point3, Vec3, dmath};
use forge_ops::{
    FilletEdge, LoftGuides, LoftKind, Profile, ProfileLoop, Radius, Segment, extrude, fillet_with, loft,
};
use forge_topo::{Body, check};

/// A thick arc about (8.6, -34.2), 4.5 wide, its two ends rounded.
fn blade_section(z: f64, turn: f64) -> Profile {
    let (c, mean, half) = (Point2::new(8.5658, -34.1526), 61.0580, 2.25);
    let (from, to) = (1.6480, -0.3155);
    let at = |a: f64| {
        let (s, k) = dmath::sin_cos(a);
        Point2::new(c.x + mean * k, c.y + mean * s)
    };
    let arc = |center: Point2, radius: f64, start: f64, sweep: f64, id: u64| {
        (
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            },
            id,
        )
    };
    let pi = dmath::PI;
    let outline = ProfileLoop::new(vec![
        arc(at(from), half, from, pi, 0),
        arc(c, mean - half, from, to - from, 1),
        arc(at(to), half, to + pi, pi, 2),
        arc(c, mean + half, to, from - to, 3),
    ]);
    let (s, k) = dmath::sin_cos(turn);
    Profile {
        plane: Frame::from_axes(
            Point3::new(0.0, 0.0, z),
            Vec3::new(k, s, 0.0),
            Vec3::new(-s, k, 0.0),
            Vec3::Z,
        )
        .unwrap(),
        loops: vec![outline],
    }
}

fn plate_and_blade() -> Body {
    let plate = Profile {
        plane: Frame::WORLD,
        loops: vec![Profile::circle(Point2::ORIGIN, 125.0, 0)],
    };
    let plate = extrude(&plate, 5.0, 1).unwrap();
    let kind = LoftKind::Smooth {
        start: None,
        end: None,
    };
    let sections = [blade_section(0.0, 0.0), blade_section(20.0, 15f64.to_radians())];
    let blade = loft(&sections, kind, &LoftGuides::None, 2).unwrap();
    boolean(&plate, &blade, Op::Union, 3).unwrap()
}

fn round(joined: &Body, r: f64) -> Body {
    let foot: Vec<FilletEdge> = joined
        .edge_ids()
        .filter(|&e| {
            joined
                .edge_points(e, 6)
                .iter()
                .all(|p| (p.z - 5.0).abs() < 1e-6 && (p.x * p.x + p.y * p.y).sqrt() < 120.0)
        })
        .map(|edge| FilletEdge {
            edge,
            radius: Radius::Constant(r),
        })
        .collect();
    assert!(!foot.is_empty());
    fillet_with(joined, &foot, &Default::default(), 4).unwrap()
}

fn bits(b: &Body) -> Vec<u64> {
    assert_eq!(check(b), vec![]);
    let mut out = Vec::new();
    for e in b.edge_ids() {
        for p in b.edge_points(e, 16) {
            out.extend([p.x.to_bits(), p.y.to_bits(), p.z.to_bits()]);
        }
    }
    out
}

/// On a thread of its own where there are threads (wasm has one): what ran before stays behind.
fn fresh<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    #[cfg(not(target_arch = "wasm32"))]
    return std::thread::scope(|s| s.spawn(f).join().unwrap());
    #[cfg(target_arch = "wasm32")]
    f()
}

#[test]
fn a_round_on_a_lofted_wall_has_the_same_bits_on_any_thread_after_anything() {
    let joined = plate_and_blade();
    let alone = fresh(|| bits(&round(&joined, 2.0)));
    let after_another = fresh(|| {
        round(&joined, 1.5);
        bits(&round(&joined, 2.0))
    });
    assert_eq!(alone, after_another);
    #[cfg(not(target_arch = "wasm32"))]
    {
        let together: Vec<Vec<u64>> = std::thread::scope(|s| {
            let runs: Vec<_> = (0..4).map(|_| s.spawn(|| bits(&round(&joined, 2.0)))).collect();
            runs.into_iter().map(|r| r.join().unwrap()).collect()
        });
        assert!(together.iter().all(|b| *b == alone));
    }
    assert_eq!(bits(&round(&joined, 2.0)), alone);
}
