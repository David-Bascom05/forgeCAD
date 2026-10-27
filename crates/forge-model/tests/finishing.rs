//! Chamfer, shell and draft through the part history: valid solids, watertight meshes, the
//! volumes worked out by hand, and the same result from the sequential and n-ary regens.

use forge_mesh::{Quality, tessellate};
use forge_model::finishing;
use forge_topo::{Body, check};

fn volume(b: &Body) -> f64 {
    let q = Quality {
        chord: 1e-4,
        ..Quality::default()
    };
    let m = tessellate(b, &q).expect("tessellates");
    assert!(m.open_edges().is_empty(), "mesh has open edges");
    m.volume()
}

#[test]
fn finishing_parts() {
    for (name, part, want) in finishing::parts() {
        let seq = part.regenerate().unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let problems = check(&seq);
        assert!(problems.is_empty(), "{name}: {problems:?}");
        let v = volume(&seq);
        if let Some(w) = want {
            assert!((v - w).abs() <= 1e-4 * w, "{name}: volume {v} want {w}");
        }
        for threads in [1, 4] {
            let nary = part
                .regenerate_nary(threads)
                .unwrap_or_else(|e| panic!("{name} nary: {e:?}"));
            assert!(check(&nary).is_empty(), "{name} nary");
            let vn = volume(&nary);
            assert!((vn - v).abs() <= 1e-6 * v, "{name}: nary volume {vn} vs {v}");
        }
        eprintln!("{name}: {} faces, volume {v:.4}", seq.faces.len());
    }
}
