//! Ribs, offset and thickened faces and direct edits through the part history: valid solids,
//! watertight meshes, the volumes worked out by hand, and the same result from the sequential and
//! n-ary regens.

use forge_mesh::{Quality, tessellate};
use forge_model::edits;
use forge_topo::{Body, check};

/// The body the last `new` made (earlier ones are bodies of their own).
fn last(
    r: Result<Vec<forge_model::Solid>, forge_model::RegenError>,
) -> Result<Body, forge_model::RegenError> {
    r.map(|mut s| s.pop().expect("a body").body)
}

fn volume(b: &Body) -> f64 {
    mesh_volume(b).unwrap()
}

fn mesh_volume(b: &Body) -> Result<f64, String> {
    let q = Quality {
        chord: 1e-4,
        ..Quality::default()
    };
    let m = tessellate(b, &q).map_err(|e| format!("{e:?}"))?;
    if !m.open_edges().is_empty() {
        return Err("mesh has open edges".into());
    }
    Ok(m.volume())
}

/// Random edits either work and give a valid, watertight solid, or say why they cannot.
#[test]
fn fuzz_edits() {
    let (mut ok, mut refused) = (0, Vec::new());
    let n = 300;
    for seed in 0..n {
        let part = edits::fuzz(seed);
        match last(part.regenerate_solids()) {
            Ok(b) => {
                let problems = check(&b);
                assert!(problems.is_empty(), "seed {seed}: {problems:?}\n{part:?}");
                let v = mesh_volume(&b);
                assert!(v.as_ref().is_ok_and(|&v| v > 0.0), "seed {seed}: {v:?}\n{part:?}");
                ok += 1;
            }
            Err(e) => {
                let s = format!("{e:?}");
                assert!(!s.contains("Invalid"), "seed {seed}: {s}\n{part:?}");
                let what: String = s.split("error: ").last().unwrap_or(&s).chars().take(40).collect();
                refused.push(what);
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
    eprintln!("{ok} of {n} built; refusals: {counts:?}");
    assert!(ok > n / 2, "{ok}");
}

#[test]
fn edit_parts() {
    for (name, part, want) in edits::parts() {
        let seq = last(part.regenerate_solids()).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let problems = check(&seq);
        assert!(problems.is_empty(), "{name}: {problems:?}");
        let v = volume(&seq);
        if let Some(w) = want {
            assert!((v - w).abs() <= 1e-4 * w, "{name}: volume {v} want {w}");
        }
        for threads in [1, 4] {
            let nary =
                last(part.regenerate_nary_solids(threads)).unwrap_or_else(|e| panic!("{name} nary: {e:?}"));
            assert!(check(&nary).is_empty(), "{name} nary");
            let vn = volume(&nary);
            assert!((vn - v).abs() <= 1e-6 * v, "{name}: nary volume {vn} vs {v}");
        }
        eprintln!("{name}: {} faces, volume {v:.4}", seq.faces.len());
    }
}
