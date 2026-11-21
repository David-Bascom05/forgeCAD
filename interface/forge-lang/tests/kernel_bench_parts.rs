//! The real-part benchmarks' Forge ports (parts/bench) build, then regenerate from their
//! features seq and n-ary to valid, closed bodies, n-ary naming everything by itself.
mod common;

use forge_model::Part;

/// Bench part `name`'s Forge port, as the kernel takes it: its features and their ids.
fn bench(name: &str) -> (Part, Vec<u64>) {
    let path = format!("{}/parts/bench/{name}.fg", env!("CARGO_MANIFEST_DIR"));
    let src = std::fs::read_to_string(&path).expect("part file");
    let (r, cache) = common::run(&src);
    assert!(
        r.errors.is_empty(),
        "{name}: {}",
        forge_lang::report::text(&r, &src)
    );
    let (f, ids) = cache.history();
    (Part { features: f.to_vec() }, ids.to_vec())
}

fn builds(name: &str) {
    forge_topo::check::check_against_both_ways(true);
    let (part, ids) = bench(name);
    let seq = part
        .regenerate_solids_ids(Some(&ids))
        .unwrap_or_else(|e| panic!("{name} seq: {e:?}"));
    let before = forge_model::nary_name_fallbacks();
    let nary = part
        .regenerate_nary_solids_ids(4, Some(&ids))
        .unwrap_or_else(|e| panic!("{name} nary: {e:?}"));
    assert_eq!(
        forge_model::nary_name_fallbacks(),
        before,
        "{name} nary relies on the sequential regen for a name"
    );
    for (path, b) in seq
        .iter()
        .map(|s| ("seq", &s.body))
        .chain(nary.iter().map(|s| ("nary", &s.body)))
    {
        let problems = forge_topo::check::check(b);
        assert!(
            problems.is_empty(),
            "{name} {path}: {:?}",
            &problems[..problems.len().min(3)]
        );
        let mesh = forge_mesh::tessellate(b, &forge_mesh::Quality::default()).expect("meshes");
        assert!(mesh.open_edges().is_empty(), "{name} {path}: mesh not closed");
    }
}

#[test]
fn gearbox() {
    builds("gearbox");
}

#[test]
fn volute() {
    builds("volute");
}

#[test]
fn crank() {
    builds("crank");
}

#[test]
fn valve() {
    builds("valve");
}

#[test]
fn heatsink() {
    builds("heatsink");
}

#[test]
fn quadarm() {
    builds("quadarm");
}

#[test]
fn conrod() {
    builds("conrod");
}

#[test]
fn jack() {
    builds("jack");
}

#[test]
fn frame() {
    builds("frame");
}

#[test]
fn manifold() {
    builds("manifold");
}

#[test]
fn bracket() {
    builds("bracket");
}

#[test]
fn insert() {
    builds("insert");
}

#[test]
fn clamp() {
    builds("clamp");
}

#[test]
fn hub() {
    builds("hub");
}

#[test]
fn guard() {
    builds("guard");
}

/// crank's varying fillet replaces a planar face's outer loop: the face must still list it first
/// (the booleans read a face from its first loop), its holes after.
#[test]
fn fillet_keeps_outer_loop_first() {
    let (mut part, ids) = bench("crank");
    // up to the spindle boss's varying round
    let at = part
        .features
        .iter()
        .position(|f| match f {
            forge_model::Feature::Fillet { edges, .. } => edges
                .iter()
                .any(|(_, r)| !matches!(r, forge_ops::Radius::Constant(_))),
            _ => false,
        })
        .expect("a varying fillet");
    part.features.truncate(at + 1);
    let b = part.regenerate_ids(Some(&ids[..=at])).expect("builds");
    for f in b.face_ids() {
        let face = b.face(f);
        if face.loops.len() > 1 && matches!(face.surface, forge_geom::Surface::Plane(_)) {
            let first = &forge_topo::uv::face_uv_loops(&b, f)[0];
            assert_eq!(
                forge_topo::uv::signed_area(&first.points) > 0.0,
                face.same_sense,
                "face {f:?} lists a hole first"
            );
        }
    }
}
