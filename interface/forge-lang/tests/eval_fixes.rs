//! Evaluator fixes found by the complex sheet parts: a warm rerun reuses what did not change,
//! loops in a sketch read the sketch's own params, params read only by `expect` are used, and an
//! invalid body says what is wrong with it.

use forge_lang::{Options, Run, report};

fn part(name: &str) -> String {
    std::fs::read_to_string(format!("{}/parts/{name}.fg", env!("CARGO_MANIFEST_DIR"))).expect("part")
}

fn run(src: &str) -> Run {
    forge_lang::run(src, Options::default(), Default::default()).0
}

/// The report less its timing line, which is all a warm run may change.
fn steady(r: &Run, src: &str) -> String {
    report::text(r, src).lines().filter(|l| !l.contains(" ms (parse ")).collect::<Vec<_>>().join("\n")
}

/// A warm rerun of an unchanged sheet part rebuilds nothing and works nothing out again, and
/// reports what the cold run did.
#[test]
fn a_warm_rerun_reuses_every_stage() {
    for name in ["sheet/box", "sheet/holes", "sheet/l", "sheet/u", "sheet/z"] {
        let src = part(name);
        let (cold, cache) = forge_lang::run(&src, Options::default(), Default::default());
        assert!(cold.errors.is_empty(), "{name}: {}", report::text(&cold, &src));
        assert!(cold.remade > 0, "{name}: the cold run works the sheet out");
        let (warm, _) = forge_lang::run(&src, Options::default(), cache);
        assert_eq!((warm.rebuilt, warm.remade), (0, 0), "{name}");
        assert_eq!(steady(&warm, &src), steady(&cold, &src), "{name}");
    }
    // an edit remakes the sheet it touches
    let src = part("sheet/l");
    let (_, cache) = forge_lang::run(&src, Options::default(), Default::default());
    let edited = src.replacen("= 2", "= 2.5", 1);
    assert_ne!(edited, src);
    let (r, _) = forge_lang::run(&edited, Options::default(), cache);
    assert!(r.remade > 0, "{}", report::text(&r, &edited));
}
