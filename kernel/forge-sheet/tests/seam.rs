//! Corner seams: a box with overlap and underlap seams against the closed form at each corner
//! (folded and flat), the overlapping flange flush with the other's far skin, a closed seam's
//! trims on a square corner, K moving nothing folded, typed refusals, and random corners never
//! crossing at a seam.
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, dmath};
use forge_ops::{ProfileLoop, Segment};
use forge_sheet::{
    BendRule, FeatureId, Gauge, HemKind, HemSpec, JointId, Map, Seam, SeamError, Sheet, SheetError, WallId,
    build_with,
};
use forge_topo::Body;

const Q: f64 = dmath::PI / 2.0;

fn gauge(t: f64, r: f64, k: f64) -> Gauge {
    Gauge {
        thickness: t,
        radius: r,
        rule: BendRule::K(k),
    }
}

fn rect(l: f64, w: f64) -> ProfileLoop {
    let p = Point2::new;
    let c = [p(0.0, 0.0), p(l, 0.0), p(l, w), p(0.0, w)];
    ProfileLoop::new(
        (0..4)
            .map(|i| {
                (
                    Segment::Line {
                        a: c[i],
                        b: c[(i + 1) % 4],
                    },
                    i as u64,
                )
            })
            .collect(),
    )
}

/// A base l x w with flanges up off all four edges, h high at `angle`.
fn tray(g: &Gauge, l: f64, w: f64, h: f64, angle: [f64; 4]) -> (Sheet, [JointId; 4]) {
    let (mut s, base) = Sheet::face(FeatureId(2), g.clone(), Frame::WORLD, &[rect(l, w)]).unwrap();
    // corners trimmed to the bends: the relief reaches t past the tangent lines, below any seam
    // whose gap is at least t, so the closed forms below add up wall by wall
    s.rule.corner.shape = forge_sheet::CornerShape::TrimToBend;
    let mut j = [JointId(0); 4];
    for e in 0..4 {
        j[e] = s.flange_r(base, e as u64, angle[e], h, g.radius).unwrap().1;
    }
    (s, j)
}

fn checked(b: &Body) {
    let problems = forge_topo::check::check(b);
    assert!(problems.is_empty(), "{problems:?}");
    let mesh = forge_mesh::tessellate(b, &forge_mesh::Quality::default()).expect("a mesh");
    assert!(mesh.open_edges().is_empty(), "open mesh");
}

fn vol(b: &Body) -> f64 {
    forge_topo::mass::mass(b).unwrap().volume
}

fn both(s: &Sheet) -> (f64, f64, Body) {
    let f = build_with(s, Map::Folded).unwrap();
    let l = build_with(s, Map::Flat).unwrap();
    checked(&f.body);
    checked(&l.body);
    (vol(&f.body), vol(&l.body), f.body)
}

#[test]
fn a_box_with_overlaps_and_underlaps() {
    let (t, r, k) = (1.5, 2.0, 0.4);
    let g = gauge(t, r, k);
    let (l, w, h) = (60.0, 40.0, 25.0);
    let (plain, _) = tray(&g, l, w, h, [Q; 4]);
    let (v0f, v0l, _) = both(&plain);
    let (mut s, j) = tray(&g, l, w, h, [Q; 4]);
    let gap = t;
    // edges run 0 (bottom), 1 (right), 2 (top), 3 (left): corners 0-1, 1-2, 2-3, 3-0
    s.seam(j[1], j[2], Seam::Overlap { gap }).unwrap();
    s.seam(j[3], j[2], Seam::Overlap { gap }).unwrap();
    s.seam(j[0], j[1], Seam::Overlap { gap }).unwrap();
    s.seam(j[0], j[3], Seam::Overlap { gap }).unwrap();
    let (vf, vl, fold) = both(&s);
    // per corner: the lapping flange gains t over (h - (R + gap)), the other loses the gap there
    let per = t * (t - gap) * (h - (r + gap));
    for (got, base) in [(vf, v0f), (vl, v0l)] {
        assert!(
            ((got - base) - 4.0 * per).abs() <= 1e-9 * got,
            "{} {}",
            got - base,
            4.0 * per
        );
    }
    // flush: the right flange's material reaches the top flange's far skin, y = w + t, exactly
    let ymax = fold
        .vertex_ids()
        .map(|v| fold.vertex(v).point)
        .filter(|p| p.x > l - 1e-9)
        .map(|p| p.y)
        .fold(f64::MIN, f64::max);
    assert!((ymax - (w + t)).abs() <= 1e-12 * w, "{ymax}");
    // the top flange stops the gap short of the right flange's inner skin, x = l - gap
    let xmax = fold
        .vertex_ids()
        .map(|v| fold.vertex(v).point)
        .filter(|p| p.y > w + 1e-9 && p.x < l - 1e-9 && p.z > r + gap + 1e-9)
        .map(|p| p.x)
        .fold(f64::MIN, f64::max);
    assert!((xmax - (l - gap)).abs() <= 1e-12 * l, "{xmax}");
}

#[test]
fn a_closed_seam_on_a_square_corner() {
    let (t, r) = (2.0, 2.0);
    let g = gauge(t, r, 0.42);
    let (l, w, h) = (50.0, 30.0, 20.0);
    let (plain, _) = tray(&g, l, w, h, [Q; 4]);
    let (v0f, v0l, _) = both(&plain);
    let (mut s, j) = tray(&g, l, w, h, [Q; 4]);
    let gap = t;
    s.seam(j[1], j[2], Seam::Closed { gap }).unwrap();
    let (vf, vl, _) = both(&s);
    // both stop at the bisector on their inner skins, the half gap back: each loses g / sqrt 2
    let per = -2.0 * t * (gap / 2.0f64.sqrt()) * (h - (r + gap));
    for (got, base) in [(vf, v0f), (vl, v0l)] {
        assert!(((got - base) - per).abs() <= 1e-9 * got, "{} {per}", got - base);
    }
}

#[test]
fn k_moves_no_seam() {
    // seams are cut in folded terms: the walls' outlines do not read K (the corner relief, cut
    // on the flat, does move: it is flat-anchored)
    let walls = |k: f64| {
        let (mut s, j) = tray(&gauge(1.5, 2.0, k), 60.0, 40.0, 25.0, [Q, 1.2, Q, 1.9]);
        s.seam(j[1], j[2], Seam::Overlap { gap: 1.5 }).unwrap();
        s.seam(j[0], j[1], Seam::Closed { gap: 1.5 }).unwrap();
        build_with(&s, Map::Folded).unwrap();
        s.walls
    };
    assert_eq!(walls(0.33), walls(0.47));
}

#[test]
fn refusals_are_typed() {
    let g = gauge(1.0, 1.0, 0.4);
    let (s0, j) = tray(&g, 40.0, 30.0, 15.0, [Q; 4]);
    let why = |r: Result<(), SheetError>| match r {
        Err(SheetError::Seam { why, .. }) => why,
        o => panic!("{o:?}"),
    };
    let mut s = s0.clone();
    assert_eq!(
        why(s.seam(j[0], j[2], Seam::Overlap { gap: 0.5 })),
        SeamError::NotACorner
    );
    assert!(matches!(
        why(s.seam(j[0], j[1], Seam::Overlap { gap: 0.001 })),
        SeamError::Gap { .. }
    ));
    assert_eq!(s, s0, "a refusal changes nothing");
    s.hem(
        WallId(2),
        1,
        HemSpec {
            kind: HemKind::Open { gap: 1.0 },
            leg: 5.0,
            up: false,
        },
        None,
    )
    .unwrap();
    assert!(matches!(
        why(s.seam(j[1], j[2], Seam::Overlap { gap: 0.5 })),
        SeamError::TipTaken { .. }
    ));
}

#[test]
fn fuzz_random_corners() {
    let mut rng = Rng::new(0x5ea);
    let (mut built, mut refused, mut clashes) = (0, 0, 0);
    for seed in 0..200 {
        let t = rng.range(0.5, 2.5);
        let g = gauge(t, rng.range(0.5, 2.0) * t, rng.range(0.3, 0.5));
        let a = [0; 4].map(|_| rng.range(0.5, 2.2) * if rng.below(5) == 0 { -1.0 } else { 1.0 });
        let (mut s, j) = tray(
            &g,
            rng.range(30.0, 80.0),
            rng.range(30.0, 80.0),
            rng.range(12.0, 40.0),
            a,
        );
        let mut ok = true;
        for c in 0..4 {
            let (x, y) = (j[c], j[(c + 1) % 4]);
            let (x, y) = if rng.bool() { (x, y) } else { (y, x) };
            let gap = rng.range(0.05, 1.0);
            let seam = if rng.bool() {
                Seam::Overlap { gap }
            } else {
                Seam::Closed { gap }
            };
            match s.seam(x, y, seam) {
                Ok(()) => {}
                Err(SheetError::Seam {
                    why: SeamError::NoRoom { .. } | SeamError::Parallel { .. },
                    ..
                }) => ok = false,
                Err(e) => panic!("seed {seed}: {e:?}"),
            }
        }
        if !ok {
            refused += 1;
            continue;
        }
        match build_with(&s, Map::Folded) {
            Ok(f) => {
                checked(&f.body);
                let l = build_with(&s, Map::Flat).unwrap_or_else(|e| panic!("seed {seed}: {e:?}"));
                checked(&l.body);
                built += 1;
            }
            // flanges leaning over the base can meet away from their seams (bend zones, tips)
            Err(SheetError::FoldedClash { a, b, .. }) => {
                assert!(a != b);
                clashes += 1;
            }
            Err(e) => panic!("seed {seed}: {e:?}"),
        }
    }
    println!("{built} built, {refused} refused, {clashes} clashes");
    assert!(built > 80, "{built}");
}
