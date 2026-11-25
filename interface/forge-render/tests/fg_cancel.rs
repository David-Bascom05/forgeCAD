//! A build the app is waiting on can be stopped: a newer edit stops it (its op refuses within a
//! slice of work, `forge_geom::budget::Watch`) and the newer text is built at once; Cancel stops
//! it, the last build staying; and while it runs its progress is there to show.

use forge_render::fg::{Msg, Session};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// UMC-400 #63060 (73 faces, from the Haas machine models): a bore through it runs some 90 s
/// before its boolean refuses past its budget.
fn board() -> PathBuf {
    let d = std::env::temp_dir().join(format!("forge-fg-cancel-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../kernel/forge-step/tests/data/haas/umc400_bore_runaway_73faces.step"),
        d.join("umc.step"),
    )
    .unwrap();
    d
}

/// The part bored down through its middle (radius `r`, a share of its least width: forge-stress's
/// bore is 0.13).
fn bored(d: &Path, r: f64) -> String {
    let opts = forge_lang::Options {
        dir: Some(d.to_path_buf()),
        ..forge_lang::Options::default()
    };
    let (run, _) = forge_lang::run("i = import(\"umc.step\")\n", opts, Default::default());
    let bx = run.body.as_ref().expect("the solid").faces_box();
    let c = bx.center();
    let small = bx.x.width().min(bx.y.width()).min(bx.z.width());
    format!(
        "i = import(\"umc.step\")\nsketch s(on: offset(xy, {})) {{\n  o = circle(({}, {}), {})\n}}\nb = cut(s, thru)\n",
        bx.z.hi,
        c.x,
        c.y,
        r * small
    )
}

/// Polls till `want` comes or `within` passes: what came, in order.
fn wait(s: &mut Session, within: Duration, want: impl Fn(&Msg) -> bool) -> Option<Vec<String>> {
    let t = Instant::now();
    let mut seen = vec![];
    while t.elapsed() < within {
        for m in s.poll() {
            seen.push(match &m {
                Msg::Built(b) => format!("built {} checked {}", b.seq, b.checked),
                Msg::Cancelled { seq } => format!("cancelled {seq}"),
                Msg::Final { seq, .. } => format!("final {seq}"),
                Msg::Posed { seq, .. } => format!("posed {seq}"),
                Msg::Warm(_) => "warm".into(),
            });
            if want(&m) {
                return Some(seen);
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    None
}

/// Till the running build has run a second and is in its op (its budgeted work under way).
fn running_a_while(s: &mut Session) {
    let t = Instant::now();
    let mut got = vec![];
    while s.progress().is_none_or(|(secs, part)| secs < 1.0 || part == 0.0) {
        got.extend(s.poll().into_iter().map(|m| match m {
            Msg::Built(b) => format!("built {} ok {}", b.seq, b.ok),
            Msg::Cancelled { seq } => format!("cancelled {seq}"),
            _ => "other".into(),
        }));
        assert!(
            t.elapsed() < Duration::from_secs(60),
            "no build running: {:?} seq {} done {} {got:?}",
            s.progress(),
            s.seq,
            s.done
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn a_newer_edit_or_cancel_stops_a_long_build_at_once() {
    let d = board();
    let f = d.join("board.fg");
    std::fs::write(&f, bored(&d, 0.13)).unwrap();
    let waker: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let mut s = Session::open(f.to_str().unwrap(), None, waker).unwrap();
    running_a_while(&mut s);
    // its progress: some of its budget spent, not all
    let (_, part) = s.progress().expect("running");
    assert!(part > 0.0 && part < 1.0, "{part}");

    // a newer edit: the long build is stopped and the newer one comes back, well before the
    // 90 s the bore would take
    let t = Instant::now();
    s.external("i = import(\"umc.step\")\n".into(), "test", 1, false);
    let newer = s.seq;
    let seen = wait(
        &mut s,
        Duration::from_secs(30),
        |m| matches!(m, Msg::Built(b) if b.seq == newer && b.checked),
    )
    .expect("the newer build");
    assert!(seen.iter().any(|m| m.starts_with("cancelled")), "{seen:?}");
    assert!(
        t.elapsed() < Duration::from_secs(20),
        "{:?}: {seen:?}",
        t.elapsed()
    );

    // the stopped text again: built afresh (nothing of the stopped build was kept), so it runs
    // long again; Cancel stops it, the last build staying, nothing left running
    s.external(bored(&d, 0.13), "test", 2, false);
    let long = s.seq;
    running_a_while(&mut s);
    let t = Instant::now();
    s.cancel();
    let seen = wait(
        &mut s,
        Duration::from_secs(20),
        |m| matches!(m, Msg::Cancelled { seq } if *seq == long),
    )
    .expect("cancelled");
    // (a slice of the bore's budgeted work is some 3 s here: the cut's other work is unbudgeted)
    assert!(
        t.elapsed() < Duration::from_secs(10),
        "{:?}: {seen:?}",
        t.elapsed()
    );
    assert_eq!(s.cancelled, Some(long));
    assert_eq!(s.done, long);
    assert!(s.progress().is_none());
}
