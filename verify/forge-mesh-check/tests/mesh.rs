//! `tessellate` on the benchmark and fuzz parts and on closed-form solids, at several chords.
//!
//! - `meshes_are_closed_oriented_manifolds`: `open_edges` empty (lean `closed_of_open_edges`:
//!   every directed edge once, its reverse there), and one fan at every vertex, which `open_edges`
//!   does not see (lean `pinch_passes_open_edges`).
//! - `volume_within_chord_bound`: the mesh's volume against the body's exact `mass` volume:
//!   |V_mesh − V| ≤ chord · area, the bound a mesh within `chord` of the surface everywhere obeys
//!   to first order. The worst ratio is printed.
//! - `volume_is_translation_invariant`: a closed oriented mesh's `volume()` does not depend on the
//!   origin (lean `volume_translate`).

use forge_math::rng::Rng;
use forge_math::{Point3, Vec3};
use forge_mesh::{Quality, TriMesh, tessellate};
use forge_mesh_check::vertex_fans;
use forge_model::suite;
use forge_topo::Body;

fn bodies() -> Vec<(String, Body)> {
    let mut parts: Vec<(String, forge_model::Part)> = suite::parts()
        .into_iter()
        .filter(|(n, _)| !matches!(*n, "torture-2" | "text" | "thread"))
        .map(|(n, p)| (n.to_string(), p))
        .collect();
    parts.extend((0..8u64).map(|s| (format!("fuzz {s}"), suite::fuzz(s, 4))));
    suite::each(&parts, |(n, p)| (n.clone(), p.regenerate().ok()))
        .into_iter()
        .filter_map(|(n, b)| b.map(|b| (n, b)))
        .collect()
}

fn mesh(b: &Body, chord: f64) -> TriMesh {
    tessellate(
        b,
        &Quality {
            chord,
            ..Quality::default()
        },
    )
    .expect("a mesh")
}

#[test]
fn meshes_are_closed_oriented_manifolds() {
    let mut n = 0;
    for (name, b) in bodies() {
        for chord in [0.1, 0.01] {
            let m = mesh(&b, chord);
            assert_eq!(m.open_edges(), vec![], "{name} at {chord}");
            let fans = vertex_fans(&m);
            let used: Vec<bool> = {
                let mut u = vec![false; m.positions.len()];
                m.triangles.iter().flatten().for_each(|&v| u[v as usize] = true);
                u
            };
            for (v, &k) in fans.iter().enumerate() {
                assert!(!used[v] || k == 1, "{name} at {chord}: vertex {v} in {k} fans");
            }
            n += 1;
        }
    }
    assert!(n >= 30, "{n} meshes");
}

#[test]
fn volume_within_chord_bound() {
    let mut worst = (0.0f64, String::new());
    for (name, b) in bodies() {
        let Ok(mp) = forge_topo::mass::mass(&b) else {
            continue;
        };
        for chord in [0.5, 0.1, 0.02] {
            let m = mesh(&b, chord);
            let ratio = (m.volume() - mp.volume).abs() / (chord * mp.area);
            if ratio > worst.0 {
                worst = (ratio, format!("{name} at {chord}"));
            }
            assert!(ratio <= 1.0, "{name} at {chord}: |dV| = {ratio} chord·area");
        }
    }
    eprintln!(
        "worst |V_mesh - V| / (chord · area): {:.4} ({})",
        worst.0, worst.1
    );
}

#[test]
fn volume_is_translation_invariant() {
    let mut r = Rng::new(1);
    for (name, b) in bodies().into_iter().take(8) {
        let m = mesh(&b, 0.1);
        let v = m.volume();
        let t = Vec3::new(r.range(-1e3, 1e3), r.range(-1e3, 1e3), r.range(-1e3, 1e3));
        let mut moved = m.clone();
        for p in &mut moved.positions {
            *p += t;
        }
        let scale = m
            .positions
            .iter()
            .map(|p| (*p - Point3::ORIGIN).length() + t.length())
            .fold(0.0, f64::max);
        // rounding: each triple product carries a few ulps of scale³
        let tol = 1e-12 * scale.powi(3) * (m.triangles.len() as f64).sqrt();
        assert!(
            (moved.volume() - v).abs() <= tol,
            "{name}: {} vs {v}",
            moved.volume()
        );
    }
}

/// Every chord the mesh draws along a circular edge sags at most `chord` (lean `segments_sag`).
#[test]
fn circle_chords_within_chord() {
    let mut worst = 0.0f64;
    let mut n = 0;
    for (name, b) in bodies() {
        let b = if b.dead > 0 { b.compact() } else { b };
        for chord in [0.1, 0.01] {
            let m = mesh(&b, chord);
            for (e, cuts) in m.edges.iter().enumerate() {
                let forge_geom::Curve::Circle(c) = &b.edges[e].curve else {
                    continue;
                };
                for w in cuts.windows(2) {
                    let (p, q) = (m.positions[w[0] as usize], m.positions[w[1] as usize]);
                    let mid = Point3::ORIGIN + ((p - Point3::ORIGIN) + (q - Point3::ORIGIN)) * 0.5;
                    let sag = c.radius - mid.distance(c.frame.origin);
                    worst = worst.max(sag / chord);
                    assert!(
                        sag <= chord * (1.0 + 1e-9),
                        "{name} at {chord}: edge {e} sags {sag}"
                    );
                    n += 1;
                }
            }
        }
    }
    assert!(n > 1000, "{n} chords");
    eprintln!("{n} circle chords, worst sag / chord {worst:.4}");
}

/// The √2-shortened step (`edge_segments` with `fine`) sags a little more than half
/// (lean `sqrt2_over_half`): by how much, up to `max_angle`.
#[test]
fn sqrt2_step_excess() {
    let sag = |r: f64, t: f64| r * (1.0 - forge_math::dmath::cos(t / 2.0));
    let worst = (1..=1000)
        .map(|k| forge_math::dmath::PI / 12.0 * k as f64 / 1000.0)
        .map(|t| 2.0 * sag(1.0, t / core::f64::consts::SQRT_2) / sag(1.0, t))
        .fold(0.0f64, f64::max);
    eprintln!("two √2-steps sag up to {worst:.6} of one full step's");
    assert!(worst > 1.0 && worst < 1.0008);
}
