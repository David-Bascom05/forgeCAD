//! Each solid of a STEP file as the render draws it (`tess::build_part`, its finest level): how
//! many triangles, how many whose corner normals point against the triangle (a normal facing in),
//! how many corner normals are not finite, and whether every triangle's face id names a face of
//! the body that the level lists (what picking reads). One line per solid. `bodycheck FILE...`
use forge_render::mesh::VERTEX;
use forge_render::tess;

fn f32_at(b: &[u8], at: usize) -> f32 {
    f32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn main() {
    for p in std::env::args().skip(1) {
        let Ok(text) = std::fs::read(&p) else { continue };
        let Ok(imp) = forge_step::import(&text, &forge_step::Limits::default()) else {
            println!("{p}\terr");
            continue;
        };
        for s in &imp.solids {
            let part = match tess::build_part(&s.body) {
                Ok(x) => x,
                Err(e) => {
                    println!("{p}\t#{}\tnopart\t{e:?}", s.entity);
                    continue;
                }
            };
            let l = &part.lods[0];
            let v = &l.vertices;
            let pos = |i: u32| {
                let at = i as usize * VERTEX;
                [f32_at(v, at), f32_at(v, at + 4), f32_at(v, at + 8)]
            };
            let nrm = |i: u32| {
                let at = i as usize * VERTEX;
                let x = i16::from_le_bytes([v[at + 12], v[at + 13]]) as f64 / 32767.0;
                let y = i16::from_le_bytes([v[at + 14], v[at + 15]]) as f64 / 32767.0;
                let z = 1.0 - x.abs() - y.abs();
                let t = (-z).max(0.0);
                [x - t * x.signum(), y - t * y.signum(), z]
            };
            let idx = |t: u32, k: usize| {
                let at = (t as usize * 3 + k) * 4;
                u32::from_le_bytes([l.indices[at], l.indices[at + 1], l.indices[at + 2], l.indices[at + 3]])
            };
            let (mut against, mut nonfinite, mut badface) = (0usize, 0usize, 0usize);
            let nf = s.body.faces.len() as u32;
            for t in 0..l.tris() {
                let [a, b, c] = [0, 1, 2].map(|k| pos(idx(t, k)));
                let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
                let g = [
                    (e1[1] * e2[2] - e1[2] * e2[1]) as f64,
                    (e1[2] * e2[0] - e1[0] * e2[2]) as f64,
                    (e1[0] * e2[1] - e1[1] * e2[0]) as f64,
                ];
                for k in 0..3 {
                    let n = nrm(idx(t, k));
                    if !n.iter().all(|x| x.is_finite()) {
                        nonfinite += 1;
                    } else if n[0] * g[0] + n[1] * g[1] + n[2] * g[2] < 0.0 {
                        against += 1;
                    }
                }
                let f = l.face_of(t);
                if f >= nf || l.face(f).is_none() {
                    badface += 1;
                }
            }
            println!(
                "{p}\t#{}\ttris {}\tagainst {}\tnonfinite {}\tbadface {}\tfaces {}/{}",
                s.entity,
                l.tris(),
                against,
                nonfinite,
                badface,
                l.faces.len(),
                nf
            );
        }
    }
}
