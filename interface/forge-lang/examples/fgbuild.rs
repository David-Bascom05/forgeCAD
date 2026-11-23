//! One .fg file built: its errors, and its part regenerated with each face's name and plane
//! (normal, offset) printed, sorted. `fgbuild FILE.fg`. For comparing builds (forge-bool's
//! decide_once switch on and off, say) on a repro.
use forge_geom::Surface;

fn main() {
    let path = std::env::args().nth(1).expect("fgbuild FILE.fg");
    let src = std::fs::read_to_string(&path).expect("a .fg file");
    let dir = std::path::Path::new(&path).parent().map(|p| p.to_path_buf());
    let opts = forge_lang::Options {
        dir,
        ..forge_lang::Options::default()
    };
    let (run, cache) = forge_lang::run(&src, opts, Default::default());
    for e in &run.errors {
        println!("error: {e:?}");
    }
    println!("{} errors, {} features", run.errors.len(), run.feats.len());
    let (f, _) = cache.history();
    let part = forge_model::Part { features: f.to_vec() };
    let solids = match part.regenerate_solids() {
        Err(e) => {
            println!("regen: {e:?}");
            return;
        }
        Ok(s) => s,
    };
    for (si, sol) in solids.iter().enumerate() {
        let b = &sol.body;
        println!("solid {si}");
        {
            let mut lines: Vec<String> = b
                .face_ids()
                .map(|fi| {
                    let face = b.face(fi);
                    let s = match &face.surface {
                        Surface::Plane(p) => {
                            let n = p.frame.z().get();
                            let o = p.frame.origin;
                            let s = if face.same_sense { 1.0 } else { -1.0 };
                            format!(
                                "plane n=({:.3},{:.3},{:.3}) d={:.4}",
                                s * n.x,
                                s * n.y,
                                s * n.z,
                                n.x * o.x + n.y * o.y + n.z * o.z
                            )
                        }
                        _ => "curved".to_string(),
                    };
                    format!("{} {:?}", s, face.name)
                })
                .collect();
            lines.sort();
            for l in lines {
                println!("  {l}");
            }
        }
    }
}
