//! Disagreements between the kernel and the exact reference, each a minimal repro, kept as
//! failing tests until the kernel is fixed (`cargo test -p forge-exact-fuzz -- --ignored`, after
//! `lake build` in verify/forge-exact). Each names the reference's answer and the kernel's.

use forge_exact_fuzz::{Class, Reference, judge_kernel, nary, reference_input, sequential};
use forge_model::Part;
use forge_topo::Body;

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
/// 33000 mm³ that passes `forge_topo::check`: it gave each fan of faces round the corner a vertex
/// of its own. Now a kept vertex counts its sides as a kept edge does (more than one fan of kept
/// faces, as more than two kept faces at an edge, is material meeting itself) and refuses.
#[test]
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
/// the reference gives a solid. The reference decides the whole history as one boolean; the
/// regen builds it a feature at a time, and after feature 10 (an Add) the part is not a manifold:
/// the reference itself says so of that stage (`coincident_walls_1_stage_is_non_manifold`), a
/// non-manifold edge of 4 facets from (29.70, 23.23, 7) to (29.86, 23.23, 7), where the added
/// block and the material under a pocket's floor meet along a line, diagonally. Feature 11 cuts
/// the block away there. The refusal is right for the stage; agreeing needs a regen that carries a
/// non-manifold stage on to the next feature, which no B-rep body here can hold.
#[test]
#[ignore = "whole-history reference: the stage after feature 10 is non-manifold (the reference agrees), cut away by feature 11"]
fn coincident_walls_1_blocks_build() {
    agrees(&family("coincident_walls", 1));
}

/// lang/28, shrunk: a box, a slanted 4-gon extruded to next and patterned 3 times, then a pocket
/// cut. The pattern's third copy ends 4.8e-5 mm past the pocket's face (y 84.3), beyond the
/// tolerance, so the cut leaves a sliver of it. The reference and the sequential regen agree:
/// 230176.32 mm³, V36 E54 F24 S3. The n-ary regen took the pocket's face for one bounding nothing
/// (its outer side tested only 10 LINEAR out, past the sliver), never crossed the copy's sides
/// with it and refused NotClosed.
#[test]
fn nary_keeps_a_sliver_past_a_cuts_face() {
    let src = "\
sketch S0(on: xy) {
  o = rect(corner: (0, 0), size: (74.39, 86.74))
}
L0 = extrude(S0, 34.72)
sketch H0(on: offset(xy, 17.36)) {
  p0 = pt(20.67, 0.83)
}
sketch S1(on: plane(at: H0.p0, normal: -Y)) {
  o = poly([(3.86, 1.37), (-0.78, 4.48), (-4.8, -1.37), (-0.35, -3.93)])
}
L1 = extrude(S1, next, flip: true)
sketch H2(on: offset(xy, 0)) {
  p0 = pt(0, 0)
  p1 = pt(-100, -5)
}
L3 = pattern(of: L1, layout: lin(along: axis(H2.p0, H2.p1), n: 3, step: 24.43))
sketch H4(on: offset(xy, 15.15)) {
  p0 = pt(-25.99, 84.3)
}
sketch S5(on: plane(at: H4.p0, normal: Y)) {
  o = poly([(-14.56, -9.7), near(14.56, -9.7), near(14.56, 9.7), near(-14.56, 9.7)])
  horizontal(o[0])
  vertical(o[1])
  horizontal(o[2])
  vertical(o[3])
  distance(o[1], 19.4)
  distance(o[0], 29.11)
}
L5 = cut(S5, 5.71)
";
    agrees(&forge_lang::fuzz::build(src).part());
}

/// The stage coincident_walls 1 is refused at, judged alone: the reference says non-manifold along
/// the line the kernel refuses at, and the kernel's refusal agrees.
#[test]
fn coincident_walls_1_stage_is_non_manifold() {
    let mut part = family("coincident_walls", 1);
    part.features.truncate(11);
    agrees(&part);
}
