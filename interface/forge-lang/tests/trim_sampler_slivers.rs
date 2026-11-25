//! trim_in's sampler (forge-bool boolean.rs, `inside`): a stretch of an intersection curve between
//! two cuts is tried at its middle, then at its golden sections, but a golden section within
//! 4 LINEAR of an end is skipped. Whether the samples kept decide the stretch rightly is an open
//! G11 site (research/decide-once/flat-g11.md, class U); the corpus never takes the skip. Here
//! parts that do: a strip 1.05e-5 to 1e-4 mm wide standing on a block, bored through across its
//! width, so the bore's circle crosses the strip's top in a stretch shorter than the skip's reach
//! (each part skips; with the skip off the answers are the same). Each built part's strip holds
//! exactly its material outside the bore.

use forge_lang::{Options, Run};

fn run(src: &str) -> Run {
    forge_lang::run(src, Options::default(), Default::default()).0
}

/// A 20 x 20 x 10 block, a strip `w` wide and 2 tall on it (when `strip`), and a bore of radius
/// `r` at `(cx, 10)` through all.
fn part(w: f64, cx: f64, r: f64, strip: bool) -> String {
    let (st, top) = if strip {
        (
            format!(
                "sketch s(on: base.end) {{\n  st = rect(corner: (10, 0), size: ({w}, 20))\n}}\n\
                 strip = extrude(s.st, 2)\n"
            ),
            "strip.end",
        )
    } else {
        (String::new(), "base.end")
    };
    format!(
        "sketch plan(on: xy) {{\n  o = rect(corner: (0, 0), size: (20, 20))\n}}\n\
         base = extrude(plan, 10)\n{st}sketch h(on: {top}) {{\n  c = circle(({cx}, 10), {r})\n}}\n\
         bore = cut(h.c, thru)\n"
    )
}

fn volume(src: &str) -> f64 {
    let r = run(src);
    assert!(r.ok(), "{:?}", r.errors);
    let b = r.body.as_ref().unwrap();
    assert!(forge_topo::check(b).is_empty());
    forge_topo::mass::mass(b).unwrap().volume
}

#[test]
fn a_bore_across_a_hair_wide_strip_leaves_its_material_exactly() {
    let (cx, r) = (12.0, 2.5);
    for w in [1.05e-5, 1.2e-5, 3e-5, 5e-5, 1e-4] {
        let got = volume(&part(w, cx, r, true)) - volume(&part(w, cx, r, false));
        // w x 20 x 2, less the bore's part of it (the chord's length across x, integrated)
        let n = 4000;
        let chord = |x: f64| {
            let dx = cx - x;
            2.0 * (r * r - dx * dx).max(0.0).sqrt()
        };
        let cut: f64 = (0..n)
            .map(|i| chord(10.0 + w * (i as f64 + 0.5) / n as f64))
            .sum::<f64>()
            * w
            / n as f64;
        let want = (w * 20.0 - cut) * 2.0;
        assert!(
            ((got - want) / want).abs() < 1e-8,
            "w {w:e}: strip holds {got:e}, want {want:e}"
        );
    }
}
