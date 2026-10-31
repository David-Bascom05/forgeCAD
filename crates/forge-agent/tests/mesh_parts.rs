//! Real parts mesh closed, and within the chord at triangles' centroids and edge midpoints, at
//! chords from coarse to fine. A crank's and a frame's cone faces once
//! failed: unrolled flat, two coarse chords of curves meeting at a sharp corner crossed.
use forge_agent::model::Model;
use forge_geom::SurfaceGeom;
use forge_mesh::{Quality, tessellate};

fn meshes(src: &str) {
    let mut m = Model::new(src);
    let _ = m.faces();
    // the mesh's face indices are the compacted body's
    let b = m.body().expect("a body").compact();
    let bb = forge_math::Box3::from_points(b.vertices.iter().map(|v| v.point));
    let diag = (bb.x.width().powi(2) + bb.y.width().powi(2) + bb.z.width().powi(2)).sqrt();
    for chord in [0.25, 4e-4 * diag, 0.05] {
        let q = Quality {
            chord,
            ..Quality::default()
        };
        let mesh = tessellate(&b, &q).unwrap_or_else(|e| panic!("chord {chord}: {e:?}"));
        assert!(mesh.open_edges().is_empty(), "chord {chord}: open");
        assert_eq!(mesh.degenerate_triangles(1e-12), 0, "chord {chord}");
        for (t, &f) in mesh.triangles.iter().zip(&mesh.face_of) {
            let s = &b.face(forge_topo::FaceId(f)).surface;
            let p = t.map(|i| mesh.positions[i as usize]);
            for x in [
                p[0].lerp(p[1], 0.5).lerp(p[2], 1.0 / 3.0),
                p[0].lerp(p[1], 0.5),
                p[1].lerp(p[2], 0.5),
                p[2].lerp(p[0], 0.5),
            ] {
                let d = s.distance(x);
                assert!(d <= chord * (1.0 + 1e-6), "chord {chord}: face {f} {d} off");
            }
        }
    }
}

#[test]
fn crank() {
    meshes(include_str!("parts/crank.fc"));
}

#[test]
fn frame() {
    meshes(include_str!("parts/frame.fc"));
}

#[test]
fn balls_at_pole() {
    meshes(include_str!("parts/balls-at-pole.fc"));
}
