//! Diagnostic: one face's display-mesh triangles against its surface: corners (on it by
//! construction) and centroids, by `distance`; and the face's edge kinds. `facedev FILE FACE`
use forge_geom::SurfaceGeom;
fn main() {
    let mut a = std::env::args().skip(1);
    let p = a.next().unwrap();
    let face: u32 = a.next().unwrap().parse().unwrap();
    let imp = forge_step::import(&std::fs::read(&p).unwrap(), &forge_step::Limits::default()).unwrap();
    let s = &imp.solids[0];
    let m = s.display_mesh().unwrap();
    let f = s.body.face(forge_topo::FaceId(face));
    let (mut corner, mut mid, mut n) = (0.0f64, 0.0f64, 0);
    let mut worst = None;
    for (t, tri) in m.triangles.iter().enumerate() {
        if m.face_of[t] != face { continue; }
        n += 1;
        let [x, y, z] = tri.map(|i| m.positions[i as usize]);
        for c in [x, y, z] { corner = corner.max(f.surface.distance(c)); }
        let g = x.lerp(y, 0.5).lerp(z, 1.0 / 3.0);
        let d = f.surface.distance(g);
        if d > mid { mid = d; worst = Some((x, y, z, g)); }
    }
    println!("face {face} tris {n} max corner dist {corner:.4} max centroid dist {mid:.4}");
    if let Some((x, y, z, g)) = worst {
        println!("  worst tri {x:?} {y:?} {z:?}\n  centroid {g:?} side lengths {:.3} {:.3} {:.3}", x.distance(y), y.distance(z), z.distance(x));
        // the projection's own answer, from the surface's nearest search
        let uv = f.surface.project(g);
        println!("  projected point {:?} dist {:.4}", f.surface.eval(uv), f.surface.eval(uv).distance(g));
    }
    // the worst edge midpoint, and whether the edge is on the boundary (one of the body's edges)
    let on_b: std::collections::HashSet<(u32, u32)> = m.edges.iter().flat_map(|e| e.windows(2).map(|w| (w[0].min(w[1]), w[0].max(w[1])))).collect();
    let mut wm = (0.0f64, false, 0.0f64, Default::default(), Default::default());
    for (t, tri) in m.triangles.iter().enumerate() {
        if m.face_of[t] != face { continue; }
        for k in 0..3 {
            let (i, j) = (tri[k], tri[(k + 1) % 3]);
            let (a, b) = (m.positions[i as usize], m.positions[j as usize]);
            let d = f.surface.distance(a.lerp(b, 0.5));
            if d > wm.0 { wm = (d, on_b.contains(&(i.min(j), i.max(j))), a.distance(b), a, b); }
        }
    }
    println!("  worst edge midpoint dist {:.4} on boundary {} length {:.3} from {:?} to {:?}", wm.0, wm.1, wm.2, wm.3, wm.4);
    for &l in &f.loops {
        for c in s.body.lp(l).coedges() {
            let e = s.body.edge(s.body.coedge(c).edge);
            println!("  edge {} {}", s.body.coedge(c).edge.0, format!("{:?}", e.curve).split(['(', ' ']).next().unwrap());
        }
    }
}
