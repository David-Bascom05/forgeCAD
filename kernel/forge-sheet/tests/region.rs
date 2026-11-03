//! The trims' region boolean against an exact scanline of the inputs and against forge-bool on a
//! flat prism (the prototype's Option B), on the prototype's seeded case sets; and its refusals.
mod common;
use common::cases::{self, Case, H, Kind, W};
use common::reference;
use forge_bool::{Op, boolean_into};
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, Point3};
use forge_ops::extrude::extrude;
use forge_ops::{Profile, ProfileLoop, Segment};
use forge_sheet::region::{Region, RegionError, holds, subtract};

const T: f64 = 2.0;

fn prism(l: &[ProfileLoop], z: f64, h: f64) -> forge_topo::Body {
    let plane = Frame::WORLD.with_origin(Point3::new(0.0, 0.0, z));
    extrude(
        &Profile {
            plane,
            loops: l.to_vec(),
        },
        h,
        1,
    )
    .expect("an extrudable loop")
}

/// Option B's area, or None where forge-bool refuses.
fn by_prism(case: &Case) -> Option<f64> {
    let mut body = prism(&case.wall, 0.0, T);
    for (i, c) in case.cuts.iter().enumerate() {
        let tool = prism(std::slice::from_ref(c), -1.0, T + 2.0);
        boolean_into(&mut body, &tool, Op::Subtract, i as u64 + 2).ok()?;
    }
    Some(forge_topo::mass::mass(&body.into_compact()).ok()?.volume / T)
}

fn inside_input(l: &[ProfileLoop], p: Point2) -> bool {
    let mut odd = false;
    for x in l {
        let poly: Vec<Point2> = x.segments.iter().flat_map(|(s, _)| s.samples(200)).collect();
        let n = poly.len();
        for i in 0..n {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            if (a.y > p.y) != (b.y > p.y) && a.x + (b.x - a.x) * (p.y - a.y) / (b.y - a.y) > p.x {
                odd = !odd;
            }
        }
    }
    odd
}

/// Wrong points among `of` samples away from every input boundary.
fn wrong_points(case: &Case, r: &Region, seed: u64, of: usize) -> usize {
    let mut rng = Rng::new(seed ^ 0xabc);
    let mut wrong = 0;
    for _ in 0..of {
        let p = Point2::new(rng.range(-5.0, W + 5.0), rng.range(-5.0, H + 5.0));
        if case
            .wall
            .iter()
            .chain(&case.cuts)
            .any(|l| l.nearest(p).distance(p) <= 0.05)
        {
            continue;
        }
        let truth = inside_input(&case.wall, p)
            && !case.cuts.iter().any(|c| inside_input(std::slice::from_ref(c), p));
        if truth != r.parts.iter().any(|c| holds(c, p)) {
            wrong += 1;
        }
    }
    wrong
}

/// Each set: built cases must match the scanline (and forge-bool where it builds); refusals must
/// be pinches or near-coincidence, nothing else.
fn run(kind: Kind, n: usize, seeds: std::ops::Range<u64>) -> (usize, usize) {
    let (mut good, mut refused) = (0, 0);
    for seed in seeds {
        let case = cases::case(kind, n, seed);
        match subtract(&case.wall, &case.cuts) {
            Ok(r) => {
                good += 1;
                let (want, err) = reference::area(&case.wall, &case.cuts, 200_000);
                let got = r.area();
                assert!(
                    (got - want).abs() <= 4.0 * err + 1e-3,
                    "{kind:?} n {n} seed {seed}: area {got} against scanline {want} ({err})"
                );
                assert_eq!(
                    wrong_points(&case, &r, seed, 400),
                    0,
                    "{kind:?} n {n} seed {seed}"
                );
                if n <= 20
                    && let Some(b) = by_prism(&case)
                {
                    assert!(
                        (got - b).abs() <= 1e-3,
                        "{kind:?} n {n} seed {seed}: {got} against forge-bool {b}"
                    );
                }
            }
            Err(RegionError::Pinched { .. } | RegionError::NearCoincident { .. }) => refused += 1,
            Err(e) => panic!("{kind:?} n {n} seed {seed}: {e:?}"),
        }
    }
    (good, refused)
}

#[test]
fn isolated_and_boundary_cuts() {
    assert_eq!(run(Kind::Isolated, 2, 0..12), (12, 0));
    assert_eq!(run(Kind::Isolated, 20, 0..3), (3, 0));
    assert!(run(Kind::Boundary, 20, 0..4).0 > 0);
}

#[test]
fn overlapping_cuts() {
    run(Kind::Overlap, 20, 0..3);
    run(Kind::OverlapLines, 20, 0..3);
    run(Kind::Mix, 20, 0..4);
}

#[test]
fn coincident_and_tangent_cuts() {
    run(Kind::Coincident, 2, 0..10);
    run(Kind::Coincident, 20, 0..3);
    // a cut tangent inside another cut leaves a valid region; every other tangency pinches
    let (good, refused) = run(Kind::Tangent, 2, 0..10);
    assert_eq!(good + refused, 10);
    assert!(refused >= 5);
}

#[test]
fn nudged_cuts_are_refused_or_right() {
    run(Kind::Nudged, 2, 0..10);
}

fn rect(x: f64, y: f64, w: f64, h: f64, id0: u64) -> ProfileLoop {
    let p = Point2::new;
    let c = [p(x, y), p(x + w, y), p(x + w, y + h), p(x, y + h)];
    ProfileLoop::new(
        (0..4)
            .map(|i| {
                (
                    Segment::Line {
                        a: c[i],
                        b: c[(i + 1) % 4],
                    },
                    id0 + i as u64,
                )
            })
            .collect(),
    )
}

#[test]
fn a_pinch_names_both_edges() {
    // a square cut whose corner sits on the wall's bottom edge (segment 0)
    let wall = [rect(0.0, 0.0, 100.0, 50.0, 0)];
    let s = 10.0;
    let diamond = ProfileLoop::new(
        [(50.0, 0.0), (50.0 + s, s), (50.0, 2.0 * s), (50.0 - s, s)]
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let q = [(50.0, 0.0), (50.0 + s, s), (50.0, 2.0 * s), (50.0 - s, s)];
                let (a, b) = (q[i], q[(i + 1) % 4]);
                (
                    Segment::Line {
                        a: Point2::new(a.0, a.1),
                        b: Point2::new(b.0, b.1),
                    },
                    10 + i as u64,
                )
            })
            .collect(),
    );
    let Err(RegionError::Pinched { pinches }) = subtract(&wall, &[diamond]) else {
        panic!("a corner on the edge pinches");
    };
    assert_eq!(pinches.len(), 1);
    let p = pinches[0];
    assert!(p.at.distance(Point2::new(50.0, 0.0)) < 1e-9);
    let mut ids = [p.a, p.b];
    ids.sort();
    assert_eq!(
        ids,
        [0, 10],
        "the wall's bottom and the cut edge leaving the corner, run backwards"
    );
}

#[test]
fn near_coincident_edges_are_refused_by_name() {
    let wall = [rect(0.0, 0.0, 100.0, 50.0, 0)];
    // a notch flush with the right edge (segment 1), 3e-6 inside it
    let notch = rect(80.0, 10.0, 20.0 - 3e-6, 20.0, 20);
    match subtract(&wall, &[notch]) {
        Err(RegionError::NearCoincident { a, b, gap, limit, .. }) => {
            let mut ids = [a, b];
            ids.sort();
            assert_eq!(ids, [1, 21]);
            assert!(gap > 1e-6 && gap < limit);
        }
        other => panic!("{other:?}"),
    }
    // exactly flush is a notch
    let r = subtract(&wall, &[rect(80.0, 10.0, 20.0, 20.0, 20)]).unwrap();
    assert_eq!(r.parts.len(), 1);
    assert!((r.area() - (5000.0 - 400.0)).abs() < 1e-9);
}

#[test]
fn a_cut_across_splits_the_wall_and_names_follow_the_input() {
    let wall = [rect(0.0, 0.0, 100.0, 50.0, 0)];
    let r = subtract(&wall, &[rect(40.0, -5.0, 20.0, 60.0, 10)]).unwrap();
    assert_eq!(r.parts.len(), 2);
    for c in &r.parts {
        assert_eq!(c.outer.len(), 4);
        // each part keeps pieces of the wall's bottom and top and one side of the cut
        let mut ids: Vec<u64> = c.outer.iter().map(|b| b.id).collect();
        ids.sort();
        assert!(ids == [0, 2, 3, 13] || ids == [0, 1, 2, 11], "{ids:?}");
        for (i, b) in c.outer.iter().enumerate() {
            let next = &c.outer[(i + 1) % c.outer.len()];
            assert_eq!(b.piece.at(1.0), next.start, "lines end exactly on their vertices");
        }
    }
    assert!((r.area() - 4000.0).abs() < 1e-9);
}
