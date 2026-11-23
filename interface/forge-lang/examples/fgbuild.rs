//! One .fg file built: its errors, and its part regenerated with each face's name and plane
//! (normal, offset) printed, sorted. `fgbuild FILE.fg [fuzz]`: `fuzz` builds with the fuzz
//! judge's options (forge_lang::fuzz::options: lints off). For comparing builds (forge-bool's
//! decide_once switch on and off, say) on a repro.
use forge_geom::Surface;

fn main() {
    let path = std::env::args().nth(1).expect("fgbuild FILE.fg");
    let src = std::fs::read_to_string(&path).expect("a .fg file");
    let dir = std::path::Path::new(&path).parent().map(|p| p.to_path_buf());
    let fuzz = std::env::args().nth(2).as_deref() == Some("fuzz");
    let opts = forge_lang::Options {
        dir,
        ..forge_lang::Options::default()
    };
    let built;
    let regen;
    let solids: &[forge_model::Solid] = if fuzz {
        // the fuzz judge's build, its last stage's solids as they stand
        built = forge_lang::fuzz::build(&src);
        for e in &built.run.errors {
            println!("error: {e:?}");
        }
        println!("{} errors, {} stages", built.run.errors.len(), built.built());
        built.solids_after(built.built())
    } else {
        let (run, cache) = forge_lang::run(&src, opts, Default::default());
        for e in &run.errors {
            println!("error: {e:?}");
        }
        println!("{} errors, {} features", run.errors.len(), run.feats.len());
        let (f, _) = cache.history();
        let part = forge_model::Part { features: f.to_vec() };
        regen = match part.regenerate_solids() {
            Err(e) => {
                println!("regen: {e:?}");
                return;
            }
            Ok(s) => s,
        };
        &regen
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
