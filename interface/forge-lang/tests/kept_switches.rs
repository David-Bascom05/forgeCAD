//! A kernel switch a step's result depends on (the imprint's mode) is in the key of a kept stage:
//! the same text built after the switch changed is built again, as fresh. Its own binary: the
//! switch is the whole process's.

use forge_lang::build::Cache;
use forge_lang::{Options, report};

const PLATE: &str = "shared w = 40
sketch s(on: xy) {
  r = rect(corner: (0, 0), size: (w, 30))
}
base = extrude(s, 6)
sketch c(on: base.end) {
  k = rect(corner: (5, 5), size: (10, 10))
}
pocket = cut(c, 2)
";

#[test]
fn a_kept_stage_is_keyed_with_the_imprints_mode() {
    use forge_bool::imprint::{Mode, mode, set_mode};
    let was = mode();
    let (first, cache) = forge_lang::run(PLATE, Options::default(), Cache::default());
    set_mode(if was == Mode::Off { Mode::On } else { Mode::Off });
    let (again, _) = forge_lang::run(PLATE, Options::default(), cache);
    let (fresh, _) = forge_lang::run(PLATE, Options::default(), Cache::default());
    set_mode(was);
    assert!(first.errors.is_empty());
    assert_eq!(again.reused, 0, "another mode: nothing kept is served");
    let text = |r: &forge_lang::Run| {
        let mut s = String::new();
        report::json(r, PLATE).write(&mut s);
        let at = s.find("\"timings_ms\":{").unwrap();
        let end = at + s[at..].find('}').unwrap() + 2;
        format!("{}{}", &s[..at], &s[end..])
    };
    assert_eq!(text(&again), text(&fresh));
}
