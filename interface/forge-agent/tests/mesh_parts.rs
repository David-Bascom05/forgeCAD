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

/// A hole tangent inside a rim with no vertex there, and rims tangent at a shared vertex: the chords
/// cross at any fineness unless each edge has a sample at the point, or the tighter one stays coarse
/// (WHUCAD 100002382, 100002741).
#[test]
fn circles_touching_inside_a_face() {
    for src in [
        "extrude 9 z 0 0 1 x 1 0 0 at -1 0 0 circle 0.95 0 0.95\n\
revolve add  z 1 0 0 x 0 1 0 at 0 0 9 axis 0 1 0 -1 loop line 0 0 1 0 line 1 0 0 1 line 0 1 0 0\n",
        "extrude 7 z 0 0 1 x 1 0 0 at -2 0 0 circle 2 0 2\n\
fillet 0.5 (edges top)\n\
extrude add 2 z 0 0 1 x 1 0 0 at -1.5 0 7 circle 1.45 0 1.45\n",
    ] {
        let m = Model::new(src);
        assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
        let mesh = tessellate(&m.body().expect("a body").compact(), &Quality::default()).expect("meshes");
        assert!(mesh.open_edges().is_empty());
    }
}
