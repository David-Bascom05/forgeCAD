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

/// Random finishing either works and gives a valid, watertight solid, or says why it cannot.
#[test]
fn fuzz_finishing() {
    let (mut ok, mut refused) = (0, Vec::new());
    for seed in 0..300 {
        let part = finishing::fuzz(seed);
        match part.regenerate() {
            Ok(b) => {
                let problems = check(&b);
                assert!(problems.is_empty(), "seed {seed}: {problems:?}\n{part:?}");
                assert!(volume(&b) > 0.0, "seed {seed}");
                ok += 1;
            }
            Err(e) => {
                let s = format!("{e:?}");
                assert!(!s.contains("Invalid"), "seed {seed}: {s}\n{part:?}");
                let what = s.split("error: ").nth(1).unwrap_or(&s);
                let what: String = what.chars().take_while(|c| c.is_alphanumeric()).collect();
                let inner = s
                    .split("what: ")
                    .nth(1)
                    .map_or(String::new(), |w| w.chars().take(40).collect());
                refused.push(format!("{} {what} {inner}", s.split(' ').next().unwrap_or("")));
            }
        }
    }
    refused.sort();
    let mut counts: Vec<(usize, String)> = Vec::new();
    for r in refused {
        match counts.last_mut() {
            Some(c) if c.1 == r => c.0 += 1,
            _ => counts.push((1, r)),
        }
    }
    eprintln!("{ok} of 300 built; refusals: {counts:?}");
    assert!(ok > 150, "{ok}");
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
