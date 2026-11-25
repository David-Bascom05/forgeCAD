//! An assembly's overlap check and expects run after its shape: once asked for, each build
//! first sends the posed parts (`checked: false`), then the same build with its checks
//! (`checked: true`), so an edit is on screen before the check, however long the check takes.
use std::sync::Arc;
use std::time::{Duration, Instant};

fn builds(s: &mut forge_render::fg::Session, seq: u64, until_checked: bool) -> Vec<(bool, Instant)> {
    let deadline = Instant::now() + Duration::from_secs(120);
    let mut got = vec![];
    while Instant::now() < deadline {
        for m in s.poll() {
            if let forge_render::fg::Msg::Built(b) = m
                && b.seq == seq
            {
                got.push((b.checked, Instant::now()));
            }
        }
        if got.iter().any(|g| g.0) || (!until_checked && !got.is_empty()) {
            return got;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("seq {seq}: no checked build in 120 s: {got:?}");
}

#[test]
fn an_assemblys_shape_comes_before_its_checks() {
    let f = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/rack18u/hw/tests/hinge_pin.fg");
    let src = std::fs::read_to_string(&f).unwrap();
    let waker: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let mut s = forge_render::fg::Session::new(Some(f.to_str().unwrap().to_string()), src, None, waker);
    // no checks asked for: one build, done
    let seq0 = s.seq;
    let first = builds(&mut s, seq0, true);
    assert_eq!(first.iter().map(|g| g.0).collect::<Vec<_>>(), [true], "{first:?}");
    // asked for: the shape, then the checks
    let seq = s.check_overlap();
    let got = builds(&mut s, seq, true);
    assert_eq!(got.iter().map(|g| g.0).collect::<Vec<_>>(), [false, true], "{got:?}");
    assert!(got[0].1 <= got[1].1);
    // and from then on, each edit too
    let seq = s.rebuild();
    let got = builds(&mut s, seq, true);
    assert_eq!(got.iter().map(|g| g.0).collect::<Vec<_>>(), [false, true], "{got:?}");
}

/// The rows the last checks found (an interference) stay listed, marked stale, while the next
/// build's checks run, and the checked build replaces them: a clash never seems to vanish.
#[test]
fn an_interference_stays_listed_stale_while_the_checks_run_again() {
    let f = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sheetmetal/probes/tray_clash.fg");
    let src = std::fs::read_to_string(&f).unwrap();
    let waker: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let mut s = forge_render::fg::Session::new(Some(f.to_str().unwrap().to_string()), src, None, waker);
    let seq0 = s.seq;
    builds(&mut s, seq0, true);
    let interference = |b: &forge_render::fg::Built| -> Vec<bool> {
        b.issues.iter().filter(|i| i.kind == "Interference").map(|i| i.stale).collect()
    };
    let collect = |s: &mut forge_render::fg::Session, seq: u64| -> Vec<(bool, Vec<bool>)> {
        let deadline = Instant::now() + Duration::from_secs(120);
        let mut got = vec![];
        while Instant::now() < deadline && !got.iter().any(|g: &(bool, Vec<bool>)| g.0) {
            for m in s.poll() {
                if let forge_render::fg::Msg::Built(b) = m
                    && b.seq == seq
                {
                    got.push((b.checked, interference(&b)));
                }
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        got
    };
    // the first checks: nothing found before, so the shape step lists no interference yet
    let seq = s.check_overlap();
    assert_eq!(collect(&mut s, seq), [(false, vec![]), (true, vec![false])]);
    // the next build: the interference stays, stale, until its checks are done
    let seq = s.rebuild();
    assert_eq!(collect(&mut s, seq), [(false, vec![true]), (true, vec![false])]);
}
