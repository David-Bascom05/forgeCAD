//! The tessellator's guarantee on every valid body (audit 2 §4): its mesh is made, first time
//! (no retry is left to make it), is watertight and consistently oriented (each directed edge
//! once, its reverse once: `open_edges` empty), faces out (positive volume), and encloses the
//! body's exact volume to within the chord: every point of it within the chord of its surface, so
//! the two differ by at most the chord times the area of the curved faces (a plane's triangles lie
//! on it).
//!
//! Over the suite, fuzz histories, every adversarial family and degenerate configuration, and the
//! fillet cases, at a coarse and a fine chord.
use forge_geom::Surface;
use forge_mesh::{Quality, tessellate};
use forge_model::adversarial::{canonical, degenerate_parts, families};
use forge_model::{Part, fillets, suite};

fn corpus() -> Vec<(String, Part)> {
    let mut parts: Vec<(String, Part)> = suite::parts()
        .into_iter()
        .map(|(n, p)| (n.to_string(), p))
        .collect();
    for s in 1..=12u64 {
        parts.push((format!("fuzz {s}"), suite::fuzz(s, 80)));
    }
    for (fam, g) in families() {
        for s in 0..4 {
            parts.push((format!("{fam}/{s}"), canonical(&g(s))));
        }
    }
    parts.extend(degenerate_parts().into_iter().map(|(n, p)| (n, canonical(&p))));
    for (n, p, _) in fillets::cases().into_iter().chain(fillets::variable_cases()) {
        if !parts.iter().any(|(m, _)| m == n) {
            parts.push((n.to_string(), p));
        }
    }
    parts
}

#[test]
fn every_valid_body_meshes_closed_and_within_its_volume() {
    let parts = corpus();
    let (bodies, bad): (Vec<usize>, Vec<Vec<String>>) = suite::each(&parts, |(name, part)| {
        let mut bad = Vec::new();
        let Ok(b) = part.regenerate() else {
            return (0, bad);
        };
        if !forge_topo::check::check(&b).is_empty() {
            return (0, bad);
        }
        let Ok(m) = forge_topo::mass::mass(&b) else {
            return (0, bad);
        };
        let curved: f64 = m
            .faces
            .iter()
            .filter(|(f, _)| !matches!(b.face(*f).surface, Surface::Plane(_)))
            .map(|x| x.1)
            .sum();
        for chord in [0.25, 0.01] {
            let q = Quality {
                chord,
                ..Quality::default()
            };
            let mesh = match tessellate(&b, &q) {
                Ok(mesh) => mesh,
                Err(e) => {
                    bad.push(format!("{name} at {chord}: {e:?}"));
                    continue;
                }
            };
            let open = mesh.open_edges().len();
            let v = mesh.volume();
            let bound = chord * curved + 1e-9 * m.volume.abs();
            if open > 0 || v <= 0.0 || (v - m.volume).abs() > bound {
                bad.push(format!(
                    "{name} at {chord}: {open} open edges, volume {v} against {} (bound {bound})",
                    m.volume
                ));
            }
        }
        (1, bad)
    })
    .into_iter()
    .unzip();
    let bad: Vec<String> = bad.concat();
    assert!(
        bodies.iter().sum::<usize>() > 400,
        "{} valid bodies",
        bodies.iter().sum::<usize>()
    );
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
