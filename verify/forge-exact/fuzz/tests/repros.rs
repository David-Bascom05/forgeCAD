//! Disagreements between the kernel and the exact reference, each a minimal repro, kept as
//! failing tests until the kernel is fixed (`cargo test -p forge-exact-fuzz -- --ignored`, after
//! `lake build` in verify/forge-exact). Each names the reference's answer and the kernel's.
//!
//! The first three were found under the strict rule and fail under either; the rest were found
//! under §6.1's snap rule (research/kernel-audit/exact-comparison.md) and are judged under it.

use forge_bool::Op;
use forge_exact_fuzz::pairs::{Boxes, Pattern, RELS, block, kernel_pair, pair_input_rule};
use forge_exact_fuzz::{
    Class, Reference, classify, judge_kernel, judge_kernel_rule, nary, nudged_last, reference_input,
    reference_input_rule, sequential,
};
use forge_model::Part;
use forge_topo::Body;

/// The family's flat part with only the features at `keep` (a shrunk repro).
fn shrunk(name: &str, seed: u64, keep: &[usize]) -> Part {
    let mut p = family(name, seed);
    p.features = keep.iter().map(|&i| p.features[i].clone()).collect();
    p
}

/// The part's sequential and n-ary regens judged under §6.1's snap rule: every one must agree.
fn agrees_snap(part: &Part) {
    let mut reference = Reference::start();
    let ans = reference.ask(&reference_input_rule(part, "snap").expect("a flat-face part"));
    let mut bad = Vec::new();
    for (path, k) in [("sequential", sequential(part)), ("nary", nary(part))] {
        let c = judge_kernel_rule(&mut reference, part, &ans, &k, "snap");
        if !matches!(c, Class::Agree) {
            bad.push(format!("{path}: {} {}", c.key(), c.detail()));
        }
    }
    assert!(bad.is_empty(), "reference: {ans:?}\n{}", bad.join("\n"));
}

/// The part's last feature applied to its stage moved by ulps (`forge_proof::props::nudged`),
/// judged under the snap rule against the unmoved part's answer (the moves are far inside LINEAR,
/// so §6.1 decides both alike).
fn nudged_agrees_snap(part: &Part, seed: u64) {
    let mut reference = Reference::start();
    let ans = reference.ask(&reference_input_rule(part, "snap").expect("a flat-face part"));
    let c = judge_kernel_rule(&mut reference, part, &ans, &nudged_last(part, seed), "snap");
    assert!(
        matches!(c, Class::Agree),
        "reference: {ans:?}\nulp {seed}: {} {}",
        c.key(),
        c.detail()
    );
}

/// A box-box pattern ("x/y/z", op) in the tolerance band: B's coincident ends on `axes` moved
/// outward by `delta` LINEARs (pairs.rs), judged under the snap rule.
fn band_agrees_snap(rels: &str, op: Op, axes: &[usize], delta: f64) {
    let r: Vec<_> = rels
        .split('/')
        .map(|n| RELS.into_iter().find(|r| r.name == n).expect("a relation"))
        .collect();
    let p = Pattern {
        rel: [r[0], r[1], r[2]],
        op,
    };
    let (a, b) = p.banded(10.0, axes, delta).expect("a coincident end");
    pair_agrees_snap(a, b, op);
}

/// A op B of two axis boxes, judged under the snap rule.
fn pair_agrees_snap(a: Boxes, b: Boxes, op: Op) {
    let (ba, bb) = (block(a, 1), block(b, 2));
    let mut reference = Reference::start();
    let ans = reference.ask(&pair_input_rule(&ba, &bb, op, "snap").expect("boxes are flat"));
    let c = classify(&ans, &kernel_pair(&ba, &bb, op));
    assert!(
        matches!(c, Class::Agree),
        "reference: {ans:?}\n{} {}",
        c.key(),
        c.detail()
    );
}

fn degenerate(name: &str) -> Part {
    forge_model::adversarial::degenerate_parts()
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no part {name}"))
        .1
}

fn family(name: &str, seed: u64) -> Part {
    let g = forge_model::adversarial::families()
        .into_iter()
        .find(|(f, _)| *f == name)
        .expect("the family")
        .1;
    forge_exact_fuzz::flat_part(&forge_model::adversarial::canonical(&g(seed)))
}

/// Edges with one face on both sides: slits left inside a face.
fn slits(b: &Body) -> Vec<String> {
    let b = b.compact();
    b.edge_ids()
        .filter(|&e| {
            let fs: Vec<_> = b.edge(e).coedges.iter().map(|&c| b.coedge_face(c)).collect();
            fs.len() == 2 && fs[0] == fs[1]
        })
        .map(|e| {
            let ed = b.edge(e);
            format!("{:?} – {:?}", b.vertex(ed.start).point, b.vertex(ed.end).point)
        })
        .collect()
}

/// The part's sequential and n-ary regens judged against the reference: every one must agree.
fn agrees(part: &Part) {
    let mut reference = Reference::start();
    let ans = reference.ask(&reference_input(part).expect("a flat-face part"));
    let mut bad = Vec::new();
    for (path, k) in [("sequential", sequential(part)), ("nary", nary(part))] {
        let c = judge_kernel(&mut reference, part, &ans, &k);
        if !matches!(c, Class::Agree) {
            bad.push(format!("{path}: {} {}", c.key(), c.detail()));
        }
    }
    if let Ok(b) = part.regenerate() {
        let s = slits(&b);
        if !s.is_empty() {
            bad.push(format!("slit edges: {s:?}"));
        }
    }
    assert!(
        bad.is_empty(),
        "reference: {} V{} E{} F{} S{} volume {} {:?}\n{}",
        ans.verdict,
        ans.vertices,
        ans.edges,
        ans.faces,
        ans.shells,
        ans.volume,
        ans.notes,
        bad.join("\n")
    );
}

/// A 10 mm cube added on the base's corner, touching it at that one vertex
/// (plane_plane/vertex_on_vertex/Add). The union is not a manifold there: the reference says
/// non-manifold at (20, 20, 0) and the kernel must refuse with ZeroThickness. It builds a body of
/// 33000 mm³ that passes `forge_topo::check`.
#[test]
#[ignore = "exact reference: the kernel builds a solid where the union is non-manifold at a vertex"]
fn cube_on_a_corner_vertex_is_non_manifold() {
    agrees(&degenerate("plane_plane/vertex_on_vertex/Add"));
}

/// A tilted plate cut from the base, touching its top face only along one edge
/// (plane_plane/tilted_edge_on_top/Cut). Nothing is removed: the reference gives the base box,
/// V8 E12 F6. The kernel gives V10 E13 F6 at the right volume: the touching edge left as a slit
/// inside the top face (both sides of the edge on one face).
#[test]
#[ignore = "exact reference: a cut touching along an edge leaves a slit edge in the top face"]
fn cut_touching_along_an_edge_leaves_no_slit() {
    agrees(&degenerate("plane_plane/tilted_edge_on_top/Cut"));
}

/// coincident_walls 1, its blocks: the kernel refuses ZeroThickness at (29.78, 23.23, 7) where
/// the reference, with no decision within tolerance, gives a solid.
#[test]
#[ignore = "exact reference: ZeroThickness refused where the exact result is a valid solid"]
fn coincident_walls_1_blocks_build() {
    agrees(&family("coincident_walls", 1));
}

/// The near-coincident part (near-coincident.md: a box whose bottom is 4e-16 inside LINEAR below
/// a notched plate's top), the plate moved by ulps (seed 0). Under §6.1 the box bottom and the
/// plate top are one carrier and the reference gives a solid; the kernel refuses SplitCrossed on
/// the plate's top. 301 of the 400 ulp seeds refuse so, 32 build an invalid body (next test), 67
/// agree. Likely site: imprint's `on_edge` (forge-bool boolean.rs:283) missing a box edge that
/// crosses a notch edge ~1e-5 out of plane, so `split_face`'s cycle walk (boolean.rs:5154)
/// finds the top's split crossing itself: the gap re-measured after the planes were decided
/// coincident.
#[test]
#[ignore = "exact reference (snap): the near-coincident notched part refuses SplitCrossed on 301 of 400 ulp nudges"]
fn notched_part_nudged_builds() {
    nudged_agrees_snap(&forge_exact_fuzz::notched(), 0);
}

/// The same part, ulp seed 13: the kernel builds a body with a zero-length edge
/// (EdgeDegenerate, 32 of 400 seeds). Likely site: `place_cuts` / `end_vertex`
/// (boolean.rs:4561) making a crossing ~1e-5 from a corner its own vertex instead of the corner.
#[test]
#[ignore = "exact reference (snap): the notched part nudged builds an invalid body (EdgeDegenerate)"]
fn notched_part_nudged_has_no_degenerate_edge() {
    nudged_agrees_snap(&forge_exact_fuzz::notched(), 13);
}

/// near_tolerance 24, features 0 1 2: a 60×40×10 plate and two 10×8 blocks added near its top,
/// one 1e-5 above it (just past LINEAR), one 1e-6 into it, overlapping each other. The reference
/// gives a solid; the kernel refuses InconsistentClassification at feature 2 (also on seed 85).
/// Likely site: `classify_specs`' flood (boolean.rs:5861, the refusal at 6000): one piece reached
/// from two specs classified differently, the in/out of a face 1e-5 off another decided twice.
#[test]
#[ignore = "exact reference (snap): InconsistentClassification refused where the exact result is a solid"]
fn near_tolerance_24_blocks_build() {
    agrees_snap(&shrunk("near_tolerance", 24, &[0, 1, 2]));
}

/// near_tolerance 53, features 0 2 3: the plate and two blocks within LINEAR of its top. The
/// reference gives a solid; the kernel refuses SplitCrossed at feature 2, with no ulp move (also
/// seed 103). Likely site as `notched_part_nudged_builds`: `on_edge` / `split_face`.
#[test]
#[ignore = "exact reference (snap): SplitCrossed refused where the exact result is a solid"]
fn near_tolerance_53_blocks_build() {
    agrees_snap(&shrunk("near_tolerance", 53, &[0, 2, 3]));
}

/// coincident_walls 42, features 0 4 9 10 11: the sequential regen refuses ZeroThickness at
/// (12.81, 16.41, 7) where the reference gives a solid; the n-ary regen builds it right (as on
/// seed 46; seed 1, `coincident_walls_1_blocks_build`, refuses on both). Likely site: the
/// sequential path's `undecided_touch` (boolean.rs:592), a touch judged on the in-place target.
#[test]
#[ignore = "exact reference (snap): the sequential regen refuses ZeroThickness where the n-ary one builds the exact solid"]
fn coincident_walls_42_blocks_build() {
    agrees_snap(&shrunk("coincident_walls", 42, &[0, 4, 9, 10, 11]));
}

/// near_tolerance 45, features 0–5: blocks cut from the plate's x = 60 end, their inner walls at
/// x = 49.99998, 50.000005, 50.00001, 50.00002. Under §6.1 the last three are one class (chained
/// within LINEAR, spread 1.5e-5 under the 2·LINEAR cap) and the reference gives V32 E48 F18; the
/// kernel's valid body keeps the 1.5e-5 step between the x = 50.000005 and x = 50.00002 walls as
/// a face (V38 E57 F21, volume within 1e-8). Not an invalid body: the kernel measures each pair
/// against LINEAR and has no transitive class (the "2× chain"; also seed 111). Site: the
/// coincidence tests (`planes_coincident`, forge-geom intersect.rs:845, and `ssi::nearly_flush`, forge-bool
/// ssi.rs:65) decided per pair.
#[test]
#[ignore = "exact reference (snap): walls chained within LINEAR are not one surface (a §6.1 class kept as steps)"]
fn near_tolerance_45_chained_walls_are_one() {
    agrees_snap(&shrunk("near_tolerance", 45, &[0, 1, 2, 3, 4, 5]));
}

/// boxes meets/overlaps/overlaps subtract: B touches A's x = 10 face from outside, so A is left
/// as it was (V8 E12 F6). The kernel's body, valid and of the right volume, keeps the contact
/// rectangle imprinted as a seventh face (V10 E15 F7). 1444 of the 6591 box patterns, and the
/// near_tolerance unmerged cases, are this: one surface left as several faces. Site:
/// `general` (forge-bool boolean.rs:557): no pass dissolves an edge between two faces of one
/// carrier after `apply`.
#[test]
#[ignore = "exact reference (snap): a touching boolean leaves its contact imprinted as an extra face"]
fn touching_subtract_leaves_no_imprint() {
    pair_agrees_snap(
        [[0.0, 10.0], [0.0, 20.0], [0.0, 20.0]],
        [[10.0, 20.0], [10.0, 30.0], [10.0, 30.0]],
        Op::Subtract,
    );
}

/// Band, two axes: meets/meets/overlaps union, B's ends on x and y moved 0.9·LINEAR outward
/// (into A). Each plane pair is within LINEAR, so §6.1 makes them one carrier and the exact
/// result is the coincident pattern's: two boxes touching along the edge x = y = 10,
/// non-manifold (ZeroThickness expected). The kernel refuses SplitCrossed: 2778 of 5943
/// patterns at every δ in (LINEAR/√2, LINEAR] on three axes, 1323 of 5187 on two. Likely
/// site: every plane reads coincident (|d| ≤ LINEAR, `planes_coincident`, forge-geom
/// intersect.rs:845) while B's corner, moved on two axes, is √2·δ > LINEAR from the edge it
/// should be on: `on_edge` (forge-bool boolean.rs:283) and `place_cuts`' `end_vertex`
/// (boolean.rs:4561) miss it, and `split_face` (5154) finds a crossed split.
#[test]
#[ignore = "exact reference (snap): the two-axis tolerance band refuses SplitCrossed"]
fn band_two_axes_split_does_not_cross() {
    band_agrees_snap("meets/meets/overlaps", Op::Union, &[0, 1], 0.9);
}

/// Band, three axes: meets/meets/meets union, B 0.9·LINEAR into A at the corner. Under §6.1
/// the boxes touch at the vertex (10, 10, 10) only: non-manifold. The kernel builds a body of
/// volume 2000.0027 (a corner overlap 0.9·LINEAR across, sub-LINEAR, kept). With B 0.9·LINEAR
/// short of A (δ = −0.9), edge touches are built the same way. The banded form of
/// `cube_on_a_corner_vertex_is_non_manifold`, whose site this shares: the touch test
/// (boolean.rs:1108) and the four-use count (1746) only see a touch the cuts made exact.
#[test]
#[ignore = "exact reference (snap): a corner overlap under LINEAR is built where §6.1 makes it a vertex touch"]
fn band_three_axes_corner_overlap_is_a_touch() {
    band_agrees_snap("meets/meets/meets", Op::Union, &[0, 1, 2], 0.9);
}

/// Band, one axis: equal/meets/equal union with B's x ends 1.5·LINEAR wider than A's (the
/// coverage worker's smallest band case). The planes are past LINEAR, so §6.1 merges nothing
/// and the exact result is a solid with two 1.5e-5 ledges. The kernel builds an invalid body
/// (EdgeUses: an edge on one face; 1717 runs on x, 1460 on z for δ in 1.5..4.8). Likely site:
/// the coplanar imprint puts A's corner on B's end vertex 1.5e-5 away (`place_cuts`,
/// boolean.rs:4561, merging with the heads' reach, a 2× allowance) while the strip between
/// keeps both edges; the `4.0 * LINEAR` sample break (boolean.rs:3022) then skips it.
#[test]
#[ignore = "exact reference (snap): a one-axis 1.5·LINEAR offset builds an invalid body (EdgeUses)"]
fn band_one_axis_offset_builds_valid() {
    band_agrees_snap("equal/meets/equal", Op::Union, &[0], 1.5);
}

/// Band, one axis: meets/meets/overlaps union, B's low x end 1.5·LINEAR into A. B touches A's
/// y = 10 face along a strip 1.5e-5 wide: the exact union is a solid (the strip is wider than
/// LINEAR). The kernel refuses ZeroThickness at (9.9999925, 10, 10), the strip's middle: a
/// feature over LINEAR thick read as zero thickness (δ in 1.5..4.8, on x and z). Likely site:
/// the four-use count (boolean.rs:1746) after `place_cuts` merged the strip's two edges, 1.5e-5
/// apart, within its 2× reach.
#[test]
#[ignore = "exact reference (snap): ZeroThickness refused for a contact strip 1.5·LINEAR wide"]
fn band_one_axis_strip_builds() {
    band_agrees_snap("meets/meets/overlaps", Op::Union, &[0], 1.5);
}
