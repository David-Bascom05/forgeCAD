//! Meshes of parts with large analytic faces (where the interior is gridded straight off and
//! only the band by the boundary goes through the Delaunay): closed, and every analytic face's
//! triangles within the chord of their surface at the centroid and the edge midpoints.
use forge_geom::{Surface, SurfaceGeom};
use forge_mesh::{Quality, tessellate};
use forge_model::{fillets, suite};
use forge_topo::FaceId;

#[test]
fn analytic_faces_closed_and_within_chord() {
    let mut parts: Vec<(&str, forge_model::Part, f64)> = vec![
        ("tour", suite::tour(), 0.01),
        ("upto", suite::upto(false), 0.01),
        ("housing", suite::part("housing").expect("housing"), 0.05),
    ];
    let pipe = fillets::cases()
        .into_iter()
        .find(|c| c.0 == "ball-rod-pipe")
        .expect("case");
    parts.push((pipe.0, pipe.1, 0.002));
    for (name, part, chord) in parts {
        let b = part.regenerate().expect("regen");
        let q = Quality {
            chord,
            ..Quality::default()
        };
        let m = tessellate(&b, &q).expect("mesh");
        assert!(m.open_edges().is_empty(), "{name}: open");
        assert_eq!(m.degenerate_triangles(1e-12), 0, "{name}");
        for (t, &f) in m.triangles.iter().zip(&m.face_of) {
            let s = &b.face(FaceId(f)).surface;
            if matches!(s, Surface::Procedural(_)) {
                continue;
            }
            let p = t.map(|i| m.positions[i as usize]);
            let c = p[0].lerp(p[1], 0.5).lerp(p[2], 1.0 / 3.0);
            for x in [
                c,
                p[0].lerp(p[1], 0.5),
                p[1].lerp(p[2], 0.5),
                p[2].lerp(p[0], 0.5),
            ] {
                let d = s.distance(x);
                assert!(d <= chord * 1.000001, "{name} face {f}: {d} off");
            }
        }
    }
}
