//! Part files the random history generator (forge_agent::history) shrank from its failures:
//! each builds valid or is refused with a clear error, without a panic or a hang.
use forge_agent::model::Model;

/// The first line that fails and why, or None when every line builds valid.
fn outcome(src: &str) -> Option<(usize, String)> {
    let m = Model::new(src);
    if let Some(e) = &m.error {
        return Some((e.at, e.msg.clone()));
    }
    let b = m.body().expect("a body");
    assert!(forge_topo::check(b).is_empty(), "{:?}", forge_topo::check(b));
    None
}

/// A revolve keep whose tool misses the part leaves nothing: refused there, and a split of
/// what would be left no longer panics on an empty box.
#[test]
fn keep_that_leaves_nothing() {
    let src = "extrude 14.83 rect 0 0 117.88 47.1
fillet 0:4.2,0.72:2.6,1:2.18 (edges near 58.94 0 0) (edges near 117.88 47.1 7.42) (edges near 58.94 47.1 14.83)
draft -7.36 xy at 0 0 11.59 (faces plane near 0 23.55 7.42)
revolve keep sym 140.83 xz at 111.27 40.53 26.03 axis 0 0 0 -1 poly 14.6 5.61 14.6 -5.61 25.83 0
";
    let (at, msg) = outcome(src).expect("refused");
    assert_eq!(at, 3);
    assert!(msg.contains("leaves nothing"), "{msg}");
    let (at, _) = outcome(&format!("{src}split plane z 1 0 0 at 0 0 0 +\n")).expect("refused");
    assert_eq!(at, 3);
}
