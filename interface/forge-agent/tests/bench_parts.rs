//! The real-part benchmarks build as the agent builds them, then regenerate from their
//! resolved features seq and n-ary to valid, closed bodies; the resolved features
//! print and parse back unchanged.
use forge_agent::{bench, text};
use forge_model::Feature;

fn builds(name: &str) {
    let part = bench::part(name);
    let mut back = text::parse(&text::print(&part)).unwrap_or_else(|(l, e)| panic!("{name}: line {l}: {e}"));
    // text keeps only its font's name in the file
    for (f, g) in back.features.iter_mut().zip(&part.features) {
        if let (Feature::Text(t), Feature::Text(u)) = (f, g) {
            t.font.font = u.font.font.clone();
        }
    }
    assert!(back == part, "{name}: printed and parsed back differently");
    let seq = part
        .regenerate_solids()
        .unwrap_or_else(|e| panic!("{name} seq: {e:?}"));
    let before = forge_model::nary_name_fallbacks();
    let nary = part
        .regenerate_nary_solids(4)
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
    let mut part = bench::part("crank");
    part.features.truncate(16);
    let b = part.regenerate().expect("builds");
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
