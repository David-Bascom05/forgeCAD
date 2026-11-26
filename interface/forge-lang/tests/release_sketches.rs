//! A run keeps its sketches on its thread (a sketch with a named entity is an `Rc` cycle, which
//! a run never frees: `forge_keep`); `forge_lang::release_sketches` frees them between runs, and
//! `ReleaseOnDrop` at a thread's end. Without either a host leaked every run's sketches for good
//! (the hosted server: 38 KB an op).
#![cfg(not(target_os = "wasi"))] // threads

const PART: &str = "shared w = 40
sketch plan(on: xy) {
  o = rect(center: (0, 0), size: (w, 30))
  c = circle((0, 0), 4)
}
base = extrude(plan, 10)
";

fn run(k: usize) {
    let text = PART.replace("40", &format!("{}", 40 + k % 7));
    let o = forge_lang::Options {
        build: false,
        ..forge_lang::Options::default()
    };
    let (r, _) = forge_lang::run(&text, o, Default::default());
    assert!(r.ok(), "{:?}", r.errors);
}

#[test]
fn a_release_after_runs_returns_to_none_kept() {
    std::thread::spawn(|| {
        for k in 0..50 {
            run(k);
        }
        assert!(
            forge_keep::kept() >= 50,
            "each run kept its sketch: {}",
            forge_keep::kept()
        );
        forge_lang::release_sketches();
        assert_eq!(forge_keep::kept(), 0);
        // and again: what is kept after a release is only the runs since
        run(0);
        assert_eq!(forge_keep::kept(), 1);
        forge_lang::release_sketches();
        assert_eq!(forge_keep::kept(), 0);
    })
    .join()
    .unwrap();
}

#[test]
fn a_thread_holding_release_on_drop_ends_with_none_kept() {
    let left = std::thread::spawn(|| {
        let n = {
            let _release = forge_lang::ReleaseOnDrop;
            for k in 0..1000 {
                run(k);
            }
            forge_keep::kept()
        };
        (n, forge_keep::kept())
    })
    .join()
    .unwrap();
    assert!(left.0 >= 1000, "kept while the thread ran: {}", left.0);
    assert_eq!(left.1, 0, "none kept once the guard dropped, at the thread's end");
}
