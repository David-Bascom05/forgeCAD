//! Corner patches mesh like any face: vertices on the exact surface, chords within quality,
//! triangles facing out.

use forge_geom::{Surface, SurfaceGeom};
use forge_math::{Point3, Vec3};
use forge_mesh::{Quality, tessellate};
use forge_ops::corner::patch;
use forge_ops::corner::synthetic::{corner, planned_hole};
use forge_ops::fillet::CornerInput;
use forge_topo::{Body, Name};

fn mesh_corner(c: &CornerInput, what: &str) {
    let hole = planned_hole(c);
    let p = patch(c, &hole).unwrap().unwrap();
    let face = &p.faces[0];
    let mut body = Body::new();
    let shell = body.add_shell();
    let vs: Vec<_> = hole
        .iter()
        .map(|h| body.add_vertex(h.start, Name::NONE))
        .collect();
    let n = hole.len();
    let edges: Vec<_> = (0..n)
        .map(|i| {
            let h = &hole[i];
            body.add_edge(h.curve.clone(), h.range, vs[i], vs[(i + 1) % n], Name::NONE)
        })
        .collect();
    let f = body.add_face(shell, face.surface.clone(), face.same_sense, Name::NONE);
    body.add_loop(f, &edges.iter().map(|&e| (e, true)).collect::<Vec<_>>());
    let q = Quality::default();
    let m = tessellate(&body, &q).unwrap_or_else(|e| panic!("{what}: {e:?}"));
    let s: &Surface = &face.surface;
    assert!(m.triangles.len() >= 8, "{what}: {} triangles", m.triangles.len());
    for &v in &m.positions {
        assert!(
            s.distance(v) <= 1e-9,
            "{what}: vertex {v:?} off the surface by {}",
            s.distance(v)
        );
    }
    for &t in &m.triangles {
        let [a, b, c] = t.map(|i| m.positions[i as usize]);
        let mid = Point3::new(
            (a.x + b.x + c.x) / 3.0,
            (a.y + b.y + c.y) / 3.0,
            (a.z + b.z + c.z) / 3.0,
        );
        let uv = s.project(mid);
        assert!(
            s.eval(uv).distance(mid) <= 2.0 * q.chord,
            "{what}: chord {}",
            s.eval(uv).distance(mid)
        );
        let tn: Vec3 = (b - a).cross(c - a);
        let sn = s.normal(uv).unwrap().get() * if face.same_sense { 1.0 } else { -1.0 };
        assert!(tn.dot(sn) > 0.0, "{what}: triangle faces in");
    }
}

fn box_corner(r: [Option<(f64, bool)>; 3]) -> CornerInput {
    corner(
        Point3::new(1.0, 2.0, 3.0),
        &[-Vec3::X, -Vec3::Y, -Vec3::Z],
        &[Vec3::Y, Vec3::Z, Vec3::X],
        &r,
    )
}

#[test]
fn corner_patches_mesh() {
    mesh_corner(&box_corner([Some((2.0, true)); 3]), "sphere");
    mesh_corner(
        &box_corner([Some((1.0, true)), Some((2.0, true)), Some((3.0, true))]),
        "1 2 3",
    );
    mesh_corner(
        &box_corner([Some((0.5, true)), Some((4.0, true)), Some((1.0, true))]),
        "0.5 4 1",
    );
    mesh_corner(&box_corner([Some((1.0, true)), None, None]), "one fillet");
    let step = corner(
        Point3::ORIGIN,
        &[Vec3::X, -Vec3::Z, Vec3::Y],
        &[Vec3::Z, Vec3::Y, Vec3::X],
        &[Some((1.0, true)), Some((2.0, false)), Some((1.5, true))],
    );
    mesh_corner(&step, "step");
    // a four-edged apex, and a saddle of two ridges and two valleys
    let apex = [
        Vec3::new(1.0, 0.1, -0.8),
        Vec3::new(0.0, 1.0, -0.6),
        Vec3::new(-1.0, 0.0, -1.0),
        Vec3::new(0.1, -1.0, -0.7),
    ];
    mesh_corner(
        &cone(&apex, &[(1.0, true), (1.5, true), (0.8, true), (1.2, true)]),
        "apex",
    );
    let saddle = [
        Vec3::new(1.0, 0.0, -0.9),
        Vec3::new(0.0, 1.0, 0.5),
        Vec3::new(-1.0, 0.1, -0.8),
        Vec3::new(0.0, -1.0, 0.4),
    ];
    mesh_corner(
        &cone(&saddle, &[(1.0, false), (1.5, true), (0.8, false), (1.2, true)]),
        "saddle",
    );
}

/// A vertex with material below faces between the given edge directions.
fn cone(dirs: &[Vec3], fillets: &[(f64, bool)]) -> CornerInput {
    let n = dirs.len();
    let normals: Vec<Vec3> = (0..n)
        .map(|i| {
            let m = dirs[(i + n - 1) % n].cross(dirs[i]).normalize().unwrap().get();
            if m.z < 0.0 { -m } else { m }
        })
        .collect();
    let f: Vec<_> = fillets.iter().map(|&f| Some(f)).collect();
    corner(Point3::new(0.5, -1.0, 2.0), dirs, &normals, &f)
}
