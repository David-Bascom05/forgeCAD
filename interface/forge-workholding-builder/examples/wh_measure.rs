//! A STEP's solids measured on their mesh (every vertex on the body within the chord), so a
//! size cited from a model is the body's, not a face box's pad: the hull per solid and the whole.
fn main() {
    let mut a = std::env::args().skip(1);
    let p = a.next().expect("STEP");
    let chord: f64 = a.next().map(|x| x.parse().unwrap()).unwrap_or(0.01);
    let im = forge_step::import(&std::fs::read(&p).expect("read"), &forge_step::Limits::default())
        .expect("import");
    let q = forge_mesh::Quality {
        chord,
        ..Default::default()
    };
    let (mut alo, mut ahi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
    for (i, s) in im.solids.iter().enumerate() {
        let m = forge_mesh::tessellate_tolerant(&s.body, &q).expect("mesh");
        let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
        for v in &m.positions {
            for (k, x) in [v.x, v.y, v.z].into_iter().enumerate() {
                lo[k] = lo[k].min(x);
                hi[k] = hi[k].max(x);
            }
        }
        for k in 0..3 {
            alo[k] = alo[k].min(lo[k]);
            ahi[k] = ahi[k].max(hi[k]);
        }
        println!(
            "{i:3} x {:?} {:?}  y {:?} {:?}  z {:?} {:?}  {}",
            lo[0], hi[0], lo[1], hi[1], lo[2], hi[2], s.name
        );
    }
    println!(
        "all x {:?} {:?}  y {:?} {:?}  z {:?} {:?}  (± {chord} the chord)",
        alo[0], ahi[0], alo[1], ahi[1], alo[2], ahi[2]
    );
}
