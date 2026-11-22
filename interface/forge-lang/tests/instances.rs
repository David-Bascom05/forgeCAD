//! A pattern's copies and a hole's places by index (`P[k]`): each its own faces, so an edge alike
//! in everything but which copy holds it is named apart (`edges(of: P[2], ..)`).

use forge_lang::anchors::Asked;
use forge_lang::{Options, Run};

fn run(src: &str, asks: &[&str]) -> Run {
    let opts = Options {
        asks: asks.iter().map(|s| s.to_string()).collect(),
        ..Options::default()
    };
    forge_lang::run(src, opts, Default::default()).0
}

fn faces(r: &Run, k: usize) -> Vec<forge_topo::Name> {
    match &r.asked[k].1 {
        Asked::Faces(fs, _) => fs.clone(),
        o => panic!("{}: {o:?}", r.asked[k].0),
    }
}

fn edges(r: &Run, k: usize) -> Vec<forge_topo::Name> {
    match &r.asked[k].1 {
        Asked::Edges(es) => es.clone(),
        o => panic!("{}: {o:?}", r.asked[k].0),
    }
}

/// A 90 x 30 plate with a round boss copied twice along x, 25 apart.
const PEGS: &str = "shared t = 5
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (90, 30))
}
base = extrude(plan, t)
param top.r = 6
sketch top(on: base.end) {
  c = circle((20, 15), r)
}
param peg.h = 8
peg = extrude(top.c, h)
param pegs.n = count(3)
param pegs.s = 25
pegs = pattern(of: peg, layout: lin(along: base.side(plan.outline[1]).normal, n: n, step: s))
";

/// The x of the centre of the faces' box (their vertices' extremes).
fn mid_x(r: &Run, names: &[forge_topo::Name]) -> f64 {
    let b = r.body.as_ref().unwrap();
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for f in b.face_ids().filter(|&f| names.contains(&b.face(f).name)) {
        let bx = b.face_box(f);
        lo = lo.min(bx.x.lo);
        hi = hi.max(bx.x.hi);
    }
    (lo + hi) / 2.0
}

#[test]
fn each_copy_of_a_pattern_is_its_own_faces() {
    let r = run(
        PEGS,
        &[
            "pegs[0]",
            "pegs[1]",
            "pegs[2]",
            "faces(of: pegs)",
            "edges(of: pegs[2], crease: \"convex\")",
            "edges(of: pegs, crease: \"convex\")",
            "faces(of: pegs[1], shape: \"plane\")",
        ],
    );
    assert!(
        r.ok(),
        "{:?}",
        r.errors.iter().map(|e| e.text()).collect::<Vec<_>>()
    );
    let (c0, c1, c2) = (faces(&r, 0), faces(&r, 1), faces(&r, 2));
    // the original is the peg's own; each copy a top and a wall, none shared
    assert_eq!(c0.len(), 2, "{c0:?}");
    assert_eq!(c1.len(), 2, "{c1:?}");
    assert_eq!(c2.len(), 2, "{c2:?}");
    assert!(c1.iter().all(|f| !c2.contains(f) && !c0.contains(f)));
    let mut both = c1.clone();
    both.extend(&c2);
    both.sort();
    let mut all = faces(&r, 3);
    all.sort();
    assert_eq!(both, all, "the pattern's faces are its copies'");
    assert!((mid_x(&r, &c0) - 20.0).abs() < 1e-6);
    assert!((mid_x(&r, &c1) - 45.0).abs() < 1e-6);
    assert!((mid_x(&r, &c2) - 70.0).abs() < 1e-6);
    assert_eq!(edges(&r, 4).len(), 1, "one copy's rim");
    assert_eq!(edges(&r, 5).len(), 2, "both copies' rims");
    assert_eq!(faces(&r, 6).len(), 1, "a copy's top");
}

#[test]
fn a_fillet_on_one_copy_rounds_that_copy_alone() {
    let one = format!("{PEGS}round = fillet(edges(of: pegs[2], crease: \"convex\"), 2)\n");
    let other = format!("{PEGS}round = fillet(edges(of: pegs[1], crease: \"convex\"), 2)\n");
    let (a, b) = (run(&one, &[]), run(&other, &[]));
    assert!(a.ok() && b.ok(), "{:?} {:?}", a.errors, b.errors);
    let plain = run(PEGS, &[]);
    let (ma, mb, m0) = (a.measure.unwrap(), b.measure.unwrap(), plain.measure.unwrap());
    // one rim rounded: one new face, the same volume taken off either copy
    assert_eq!(ma.faces, m0.faces + 1);
    assert!((ma.volume - mb.volume).abs() < 1e-6 * ma.volume);
    assert!(ma.volume < m0.volume);
    // the rounded copy is the far one: the part's centroid moves less toward it
    assert!(
        ma.centroid[0] < mb.centroid[0],
        "{:?} {:?}",
        ma.centroid,
        mb.centroid
    );
    // the edge where a copy meets the plate, by the copy and the plate's top
    let seat = format!("{PEGS}round = fillet(edges(between: (pegs[1], base.end)), 1)\n");
    let s = run(&seat, &[]);
    assert!(s.ok(), "{:?}", s.errors);
    assert_eq!(s.measure.unwrap().faces, m0.faces + 1);
}

#[test]
fn a_holes_places_by_index_and_circular_and_mirrored_copies() {
    let holes = "sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (60, 20))
}
base = extrude(plan, 8)
sketch pts(on: base.end) {
  a = pt(10, 10)
  b = pt(30, 10)
  c = pt(50, 10)
}
param bolt.d = 4
bolt = hole(at: [pts.a, pts.b, pts.c], d: d, depth: thru)
";
    let r = run(
        holes,
        &[
            "bolt[0]",
            "bolt[1]",
            "bolt[2]",
            "edges(of: bolt[1], crease: \"convex\")",
        ],
    );
    assert!(r.ok(), "{:?}", r.errors);
    for k in 0..3 {
        let fs = faces(&r, k);
        assert_eq!(fs.len(), 1, "a place's wall: {fs:?}");
        assert!((mid_x(&r, &fs) - (10.0 + 20.0 * k as f64)).abs() < 1e-6);
    }
    assert_eq!(edges(&r, 3).len(), 2, "a place's two rims");
    let one = format!("{holes}edge = chamfer(edges(of: bolt[2], crease: \"convex\"), 0.5)\n");
    let c = run(&one, &[]);
    assert!(c.ok(), "{:?}", c.errors);
    assert_eq!(c.measure.unwrap().faces, r.measure.as_ref().unwrap().faces + 2);
    // past the last place, and a feature that has no copies
    let out = run(&format!("{holes}x = faces(of: bolt[3])\n"), &[]);
    assert_eq!(
        out.errors.first().map(|e| e.kind()),
        Some("IndexOut"),
        "{:?}",
        out.errors
    );
    let not = run(&format!("{holes}x = faces(of: base[1])\n"), &[]);
    assert_eq!(
        not.errors.first().map(|e| e.kind()),
        Some("WrongType"),
        "{:?}",
        not.errors
    );

    let ring = "sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (100, 100))
}
base = extrude(plan, 5)
param top.r = 6
sketch top(on: base.end) {
  c = circle((50, 80), r)
  hub = circle((50, 50), r)
}
param peg.h = 8
peg = extrude(top.c, h)
param ring.n = count(4)
ring = pattern(of: peg, layout: circ(about: top.hub.axis, n: n))
";
    let r = run(ring, &["ring[1]", "ring[2]", "ring[3]"]);
    assert!(r.ok(), "{:?}", r.errors);
    // the peg at (50, 80) turned about (50, 50): to x = 20, 50 and 80, one copy each
    let mut xs: Vec<i64> = (0..3).map(|k| mid_x(&r, &faces(&r, k)).round() as i64).collect();
    xs.sort();
    assert_eq!(xs, [20, 50, 80]);
    assert!(faces(&r, 0).iter().all(|f| !faces(&r, 1).contains(f)));
}

#[test]
fn copies_of_the_whole_part() {
    let src = "sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (10, 10))
}
block = extrude(plan, 4)
param row.n = count(3)
param row.s = 20
row = pattern(whole: true, layout: lin(along: block.side(plan.outline[1]).normal, n: n, step: s))
";
    let r = run(src, &["row[0]", "row[1]", "row[2]"]);
    assert!(r.ok(), "{:?}", r.errors);
    for k in 0..3 {
        let fs = faces(&r, k);
        assert_eq!(fs.len(), 6, "a block's faces: {fs:?}");
        assert!((mid_x(&r, &fs) - (5.0 + 20.0 * k as f64)).abs() < 1e-6);
    }
}

#[test]
fn an_anchor_on_a_copy_is_the_copied_features_anchor_carried_there() {
    let src = "sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (90, 30))
}
base = extrude(plan, 5)
sketch top(on: base.end) {
  sq = rect(corner: (10, 10), size: (10, 10))
}
param blk.h = 5
blk = extrude(top.sq, h)
param blks.n = count(3)
param blks.s = 25
blks = pattern(of: blk, layout: lin(along: base.side(plan.outline[1]).normal, n: n, step: s))
";
    let r = run(
        src,
        &[
            "blks[2].end",
            "blks[1].side(top.sq[0])",
            "edges(between: (blks[2].end, blks[2].side(top.sq[0])))",
            "blks[0].end",
            "blks[2].end.edges",
        ],
    );
    assert!(r.ok(), "{:?}", r.errors);
    let top2 = faces(&r, 0);
    assert_eq!(top2.len(), 1);
    assert!((mid_x(&r, &top2) - 65.0).abs() < 1e-6);
    let wall1 = faces(&r, 1);
    assert_eq!(wall1.len(), 1);
    assert!((mid_x(&r, &wall1) - 40.0).abs() < 1e-6);
    // the front top edge of the far copy: one of twelve alike
    assert_eq!(edges(&r, 2).len(), 1);
    assert!((mid_x(&r, &faces(&r, 3)) - 15.0).abs() < 1e-6);
    assert_eq!(edges(&r, 4).len(), 4);
    let round = format!("{src}round = fillet(edges(between: (blks[2].end, blks[2].side(top.sq[0]))), 1)\n");
    let a = run(&round, &[]);
    assert!(a.ok(), "{:?}", a.errors);
    assert_eq!(a.measure.unwrap().faces, r.measure.as_ref().unwrap().faces + 1);
    // a hole's anchor at one place
    let holes = "sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (60, 20))
}
base = extrude(plan, 8)
sketch pts(on: base.end) {
  a = pt(10, 10)
  b = pt(30, 10)
}
param bolt.d = 4
bolt = hole(at: [pts.a, pts.b], d: d, depth: thru)
";
    let h = run(
        holes,
        &["bolt[0].rim", "bolt[1].rim", "bolt[1].wall", "bolt[1].edges"],
    );
    assert!(h.ok(), "{:?}", h.errors);
    let (first, second) = (edges(&h, 0), edges(&h, 1));
    assert!(
        !second.is_empty() && first.len() == second.len(),
        "{first:?} {second:?}"
    );
    assert!(second.iter().all(|e| !first.contains(e)));
    assert!(
        second.iter().all(|e| edges(&h, 3).contains(e)),
        "the place's own mouth"
    );
    assert!((mid_x(&h, &faces(&h, 2)) - 30.0).abs() < 1e-6);
}
