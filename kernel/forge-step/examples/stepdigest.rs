//! What a read of each STEP file gives, as hashes: every solid's body and integrals, every
//! refusal, the read's statistics, and each solid's mesh as the app draws it (`tessellate_tolerant`
//! at 5e-4 of the body's diagonal). For checking that a change to the reader's speed changes no
//! bit of what it reads: run on a corpus before and after, and diff the lines.
//!
//! `stepdigest [--solids] FILE..`: a line per file (`--solids`: and one per solid), with the
//! wall time of the read and of the meshes. Lazily filled caches inside a surface (`OnceLock`)
//! are left out of a body's hash: whether one is filled yet is not what was read.
use std::hash::{Hash, Hasher};
use std::time::Instant;

/// Debug text with every `OnceLock(..)`'s contents taken out.
fn without_caches(s: &str) -> String {
    const KEY: &str = "OnceLock(";
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find(KEY) {
        out.push_str(&rest[..at + KEY.len()]);
        let mut depth = 1;
        let mut end = rest.len();
        for (i, c) in rest[at + KEY.len()..].char_indices() {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = at + KEY.len() + i;
                        break;
                    }
                }
                _ => {}
            }
        }
        out.push_str("..");
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

fn hash(s: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

fn main() {
    let mut per_solid = false;
    for path in std::env::args().skip(1) {
        if path == "--solids" {
            per_solid = true;
            continue;
        }
        let text = std::fs::read(&path).expect("read");
        let t = Instant::now();
        // `STEPDIGEST_DRAWN`: read as a viewer reads it (`import_drawn`), its drawings kept
        let imp = if std::env::var_os("STEPDIGEST_DRAWN").is_some() {
            forge_step::import_drawn(&text, &forge_step::Limits::default())
        } else {
            forge_step::import(&text, &forge_step::Limits::default())
        };
        let read_s = t.elapsed().as_secs_f64();
        let imp = match imp {
            Ok(i) => i,
            Err(e) => {
                println!("{path} err {:016x} read {read_s:.3}s", hash(&format!("{e:?}")));
                continue;
            }
        };
        let t = Instant::now();
        let mut all = std::collections::hash_map::DefaultHasher::new();
        let mut lines = Vec::new();
        for s in &imp.solids {
            // as the app draws it: the read's own drawing where it kept one (`STEPDIGEST_REDRAW`:
            // drawn again regardless, to check the kept one is the same)
            let mesh = if std::env::var_os("STEPDIGEST_REDRAW").is_some() {
                forge_mesh::tessellate::tessellate_tolerant(
                    &s.body,
                    &forge_step::Solid::display_quality(&s.body),
                )
                .map(std::borrow::Cow::Owned)
            } else {
                s.display_mesh()
            };
            let b = hash(&without_caches(&format!("{:?}", s.body)));
            let m = hash(&format!("{:?}", s.mass));
            let me = hash(&format!("{mesh:?}"));
            (s.entity, &s.name, b, m, me).hash(&mut all);
            lines.push(format!(
                "  solid {} faces {} body {b:016x} mass {m:016x} mesh {me:016x}",
                s.entity,
                s.body.faces.len()
            ));
        }
        let mesh_s = t.elapsed().as_secs_f64();
        let r = hash(&format!(
            "{:?}",
            imp.refusals
                .iter()
                .map(|r| (r.entity, &r.kind, &r.why))
                .collect::<Vec<_>>()
        ));
        let st = hash(&format!("{:?}", imp.stats));
        (r, st, &imp.schema, &imp.system).hash(&mut all);
        println!(
            "{path} solids {} refused {} all {:016x} refusals {r:016x} stats {st:016x} read {read_s:.3}s mesh {mesh_s:.3}s",
            imp.solids.len(),
            imp.refusals.len(),
            all.finish()
        );
        if per_solid {
            for l in lines {
                println!("{l}");
            }
        }
    }
}
