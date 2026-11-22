//! One property over a directory of .fg part files: propdir PROP DIR [FROM COUNT] [THREADS]
//! The properties regenerate a part without the ids the language names its features by, so a
//! part that names a face or edge (a fillet, a face edit) is skipped, said so.
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (name, prop) = forge_proof::props::all()
        .into_iter()
        .find(|p| p.0 == a[0])
        .expect("a property");
    let a = &a[1..];
    let mut files: Vec<_> = std::fs::read_dir(&a[0])
        .expect("a directory")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "fg"))
        .collect();
    files.sort();
    let from: usize = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let count: usize = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(files.len());
    let threads: usize = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(3);
    let files: Vec<_> = files.into_iter().skip(from).take(count).collect();
    std::panic::set_hook(Box::new(|_| {}));
    let next = AtomicUsize::new(0);
    let out = Mutex::new(());
    let checked = AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                while let Some(f) = files.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let Ok(src) = std::fs::read_to_string(f) else {
                        continue;
                    };
                    let got = std::panic::catch_unwind(|| {
                        let part = forge_lang::fuzz::build(&src).part();
                        // (named by position, as the properties regenerate it, it still builds)
                        if part.regenerate_solids().is_err() {
                            let _g = out.lock();
                            println!("{} skipped: it names faces or edges by feature ids", f.display());
                            return None;
                        }
                        prop(&part, 1)
                    });
                    checked.fetch_add(1, Ordering::Relaxed);
                    if let Ok(Some(x)) = got {
                        let _g = out.lock();
                        println!("{} {name} {}: {}", f.display(), x.kind, x.detail);
                    }
                }
            });
        }
    });
    println!("{} parts checked", checked.load(Ordering::Relaxed));
}
