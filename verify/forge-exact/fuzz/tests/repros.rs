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
/// 33000 mm³ that passes `forge_topo::check`.
#[test]
#[ignore = "exact reference: the kernel builds a solid where the union is non-manifold at a vertex"]
fn cube_on_a_corner_vertex_is_non_manifold() {
    agrees(&degenerate("plane_plane/vertex_on_vertex/Add"));
}

/// A tilted plate cut from the base, touching its top face only along one edge
/// (plane_plane/tilted_edge_on_top/Cut). Nothing is removed: the reference gives the base box,
/// V8 E12 F6. The kernel gave V10 E13 F6 at the right volume: the touching edge imprinted into the
/// top face as a slit (both sides of the edge on one face). Now an edge lying in a face is
/// imprinted only where its body crosses the face, and unify drops any edge with one face on both
/// sides that is not a seam the face wraps round.
#[test]
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
