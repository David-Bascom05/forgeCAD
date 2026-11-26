//! Each solid's display mesh checked against the exact body: every triangle's centroid and edge
//! midpoints (each distance the surface's own, or no more than a corner's way plus its distance)
//! within the chord (plus the face's largest edge tolerance, for triangles on a file's
//! tolerant boundary) of its face's surface; open edges and pinched vertices (a closed manifold has
//! neither); the triangle count; and the solid's verdict. One line per solid, `meshcheck FILE...`.
use forge_geom::SurfaceGeom;

fn main() {
    for p in std::env::args().skip(1) {
        let Ok(text) = std::fs::read(&p) else { continue };
        let imp = match forge_step::import(&text, &forge_step::Limits::default()) {
            Ok(i) => i,
            Err(e) => {
                println!("{p}\terr\t{e:?}");
                continue;
            }
        };
        for r in &imp.refusals {
            println!("{p}\t#{}\trefused\t{}", r.entity, r.why);
        }
        for s in &imp.solids {
            let b = &s.body;
            let q = forge_step::Solid::display_quality(b);
            let Ok(m) = s.display_mesh() else {
                println!("{p}\t#{}\tnomesh\t{:?}", s.entity, b.unverified);
                continue;
            };
            let mut face_tol = vec![0.0_f64; b.faces.len()];
            for f in b.face_ids() {
                for &l in &b.face(f).loops {
                    for c in b.lp(l).coedges() {
                        let e = b.edge(b.coedge(c).edge);
                        face_tol[f.index()] = face_tol[f.index()].max(e.tol).max(b.vertex(e.start).tol).max(b.vertex(e.end).tol);
                    }
                }
            }
            let mut worst = 0.0_f64; // the largest deviation over (chord + face's tolerance)
            for (t, tri) in m.triangles.iter().enumerate() {
                let f = forge_topo::FaceId(m.face_of[t]);
                let surf = &b.face(f).surface;
                let [a, bb, c] = tri.map(|i| m.positions[i as usize]);
                let pts = [
                    a.lerp(bb, 0.5).lerp(c, 1.0 / 3.0),
                    a.lerp(bb, 0.5),
                    bb.lerp(c, 0.5),
                    c.lerp(a, 0.5),
                ];
                let allow = q.chord + face_tol[f.index()];
                // the corners' own distances: a point's distance is at most its way to a corner
                // and that corner's, so where a projection finds a far sheet of the surface
                // (a NURBS search's local answer) the bound stands: never less than the truth
                let corners = [a, bb, c].map(|k| (k, surf.distance(k), surf.project(k)));
                for x in pts {
                    let bound = corners
                        .iter()
                        .map(|&(k, dk, uv)| {
                            // a surface point near this corner's: its distance is an upper bound too
                            let near = surf.eval(surf.project_near(x, uv)).distance(x);
                            near.min(x.distance(k) + dk)
                        })
                        .fold(f64::INFINITY, f64::min);
                    let d = surf.distance(x).min(bound);
                    if d / allow > worst && std::env::var_os("MESHCHECK_WHERE").is_some() {
                        eprintln!("  worse: face {} ratio {:.3} d {:.4} chord {:.4} tol {:.2e} surface {}", f.0, d / allow, d, q.chord, face_tol[f.index()], format!("{:?}", surf).chars().take(120).collect::<String>());
                    }
                    worst = worst.max(d / allow);
                }
            }
            // each face's triangle count, for counting the faces whose mesh changed
            let mut per = vec![0u32; b.faces.len()];
            for &f in &m.face_of {
                per[f as usize] += 1;
            }
            println!(
                "{p}\t#{}\tfaces\t{}",
                s.entity,
                per.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",")
            );
            println!(
                "{p}\t#{}\ttris {}\tworst {:.3}\topen {}\tpinched {}\tunverified {}",
                s.entity,
                m.triangles.len(),
                worst,
                m.open_edges().len(),
                m.pinched_vertices().len(),
                b.unverified.is_some()
            );
        }
    }
}
