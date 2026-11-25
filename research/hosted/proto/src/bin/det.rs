//! For each .fg file named on stdin-list (argv[1]: a file of paths): a cold build, its exact
//! body hash and mesh hash; then up to N SetParam edits (each handle x1.05) rebuilt warm (the
//! Cache of the last run) and cold, timed. One line per file:
//! file ok body_hash mesh_hash faces tris mesh_bytes cold_ms mesh_ms | edits: warm_ms.. ; cold2_ms..
use forge_lang::edit;
use forge_lang::error::Op;
use hosted_proto::*;

fn main() {
    let list = std::env::args().nth(1).expect("list file");
    let edits: usize = std::env::args().nth(2).map_or(3, |s| s.parse().unwrap());
    let text = std::fs::read_to_string(&list).expect("list");
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let path = std::path::PathBuf::from(line.trim());
        let Ok(src) = std::fs::read_to_string(&path) else {
            println!("{} UNREADABLE", path.display());
            continue;
        };
        let t0 = now_ms();
        let (run, mut cache) = forge_lang::run(&src, opts(&path), Default::default());
        let cold = now_ms() - t0;
        let b = hash_run(&run, cold);
        if std::env::args().nth(3).as_deref() == Some("errs") {
            for e in &run.errors {
                let t = e.text();
                println!("  ERR {} {}", e.kind(), t.lines().next().unwrap_or(""));
            }
        }
        let mut warm = vec![];
        let mut cold2 = vec![];
        let mut text = src.clone();
        for (key, lit) in edit::handles(&src).into_iter().take(edits) {
            let mut v = lit;
            v.value = if v.value == 0.0 { 0.5 } else { v.value * 1.05 };
            let op = Op::SetParam { param: key, value: v };
            let Ok(next) = edit::apply(&text, &op) else { continue };
            let t = now_ms();
            let (r2, c2) = forge_lang::run(&next, opts(&path), cache);
            warm.push(now_ms() - t);
            cache = c2;
            let t = now_ms();
            let (r3, _) = forge_lang::run(&next, opts(&path), Default::default());
            cold2.push(now_ms() - t);
            // warm and cold must give the same bits
            let hw = hash_run(&r2, 0.0);
            let hc = hash_run(&r3, 0.0);
            if std::env::var("DET_WHY").is_ok() {
                let ek = |r: &forge_lang::Run| r.errors.iter().map(|e| e.kind()).collect::<Vec<_>>().join("+");
                eprintln!("edit {} -> warm {:.1} ms errs [{}] rebuilt {} reused {} | cold {:.1} ms errs [{}] rebuilt {} reused {}",
                    op.json(), warm.last().unwrap(), ek(&r2), r2.rebuilt, r2.reused, cold2.last().unwrap(), ek(&r3), r3.rebuilt, r3.reused);
            }
            if hw.body_hash != hc.body_hash {
                println!("WARM_COLD_DIFFER {}", path.display());
            }
            text = next;
        }
        let f = |v: &[f64]| v.iter().map(|x| format!("{x:.2}")).collect::<Vec<_>>().join(",");
        println!(
            "{} {} {:016x} {:016x} faces={} tris={} meshbytes={} errors={} cold_ms={:.2} mesh_ms={:.2} warm_ms={} cold2_ms={}",
            path.display(),
            if b.ok { "ok" } else { "err" },
            b.body_hash,
            b.mesh_hash,
            b.faces,
            b.tris,
            b.mesh_bytes,
            b.errors,
            b.build_ms,
            b.mesh_ms,
            f(&warm),
            f(&cold2)
        );
    }
}
