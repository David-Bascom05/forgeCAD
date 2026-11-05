//! Move to Own File and Bring Into This File from the app's session: the file written beside,
//! the document one undo step, and back.

use forge_lang::refactor::Refactor;
use std::sync::Arc;

#[test]
fn a_component_moves_out_and_back_in_one_undo_step_each() {
    let d = std::env::temp_dir().join(format!("forge-fg-refactor-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let _ = std::fs::remove_file(d.join("peg.fg"));
    let src = "shared plen = 30
component peg(d: 6) {
  sketch s(on: xy) {
    c = circle((0, 0), d / 2)
  }
  shank = extrude(s, plen)
}
p1 = part(peg(d: 8), ground: true)
";
    let f = d.join("board.fg");
    std::fs::write(&f, src).unwrap();
    let waker: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let mut s = forge_render::fg::Session::open(f.to_str().unwrap(), None, waker).unwrap();
    let wrote = s
        .refactor(&Refactor::ExtractComponent {
            component: "peg".into(),
            file: "peg.fg".into(),
        })
        .unwrap();
    assert_eq!(wrote, ["peg.fg"]);
    assert!(d.join("peg.fg").exists());
    assert!(
        s.src
            .contains("part(\"peg.fg\", with: {d: 8, plen: plen}, ground: true)"),
        "{}",
        s.src
    );
    s.refactor(&Refactor::InlineComponent {
        file: "peg.fg".into(),
        name: "peg".into(),
    })
    .unwrap();
    assert_eq!(s.src, src);
    assert!(s.undo());
    assert!(s.src.contains("peg.fg"));
}

#[test]
fn a_refactor_needs_a_saved_file_and_never_follows_a_link() {
    let d = std::env::temp_dir().join(format!("forge-fg-refactor-link-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let src = "component peg(d: 6) {\n  sketch s(on: xy) {\n    c = circle((0, 0), d)\n  }\n  e = extrude(s, d)\n}\np1 = part(peg, ground: true)\n";
    let f = d.join("board.fg");
    std::fs::write(&f, src).unwrap();
    let victim = d.join("victim.txt");
    std::fs::write(&victim, "keep").unwrap();
    let _ = std::fs::remove_file(d.join("peg.fg"));
    #[cfg(unix)]
    std::os::unix::fs::symlink(&victim, d.join("peg.fg")).unwrap();
    let waker: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let mut s = forge_render::fg::Session::open(f.to_str().unwrap(), None, waker).unwrap();
    let r = Refactor::ExtractComponent {
        component: "peg".into(),
        file: "peg.fg".into(),
    };
    assert!(s.refactor(&r).is_err());
    assert_eq!(std::fs::read_to_string(&victim).unwrap(), "keep");
    assert_eq!(s.src, src);
    // an untitled document has nowhere to write
    s.path = None;
    let r2 = Refactor::ExtractComponent {
        component: "peg".into(),
        file: "peg2.fg".into(),
    };
    assert!(s.refactor(&r2).is_err());
    assert!(!std::path::Path::new("peg2.fg").exists());
}
