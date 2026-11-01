//! The `volume` property over a directory of part files: toolvolume DIR [FROM COUNT] [THREADS]
use forge_agent::model::Model;
use forge_model::Part;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mut files: Vec<_> = std::fs::read_dir(&a[0])
        .expect("a directory")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "fc"))
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
                        let m = Model::new(&src);
                        let part = Part {
                            features: m.features.clone(),
                        };
                        forge_proof::props::tool_volume(&part, 1)
                    });
                    checked.fetch_add(1, Ordering::Relaxed);
                    if let Ok(Some(x)) = got {
                        let _g = out.lock();
                        println!("{} {}: {}", f.display(), x.kind, x.detail);
                    }
                }
            });
        }
    });
    println!("{} parts checked", checked.load(Ordering::Relaxed));
}
