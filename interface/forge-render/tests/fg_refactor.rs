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
    assert!(s.src.contains("part(\"peg.fg\", with: {d: 8, plen: plen}, ground: true)"), "{}", s.src);
    s.refactor(&Refactor::InlineComponent {
        file: "peg.fg".into(),
        name: "peg".into(),
    })
    .unwrap();
    assert_eq!(s.src, src);
    assert!(s.undo());
    assert!(s.src.contains("peg.fg"));
}
