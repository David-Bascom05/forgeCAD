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
    report::text(r, src)
        .lines()
        .filter(|l| !l.contains(" ms (parse "))
        .collect::<Vec<_>>()
        .join("\n")
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

fn kinds(r: &Run) -> Vec<String> {
    let w = r.warnings.iter().map(|w| format!("{w:?}"));
    r.errors.iter().map(|e| format!("{e:?}")).chain(w).collect()
}

/// A loop's range and keyed map inside a sketch read the sketch's own params: a local count
/// wins over a shared one of the same name, and a local read in the header is no lint.
#[test]
fn a_loop_in_a_sketch_reads_the_sketchs_params() {
    let src = "shared n = count(2)
shared r = 2
shared d = 5
param holes.n = count(4)
param holes.step = 10
param holes.w = 60
param holes.h = 20
param holes.y = 10
param holes.ya = 5
sketch holes(on: xy) {
  o = rect(corner: (0, 0), size: (w, h))
  for i in 0..n {
    c = circle((step * (i + 1), y), r)
  }
  for k, p in {a: (step, ya), b: (w - step, ya)} {
    e = circle(p, r)
  }
}
f = extrude(holes, d)
";
    let r = run(src);
    assert!(r.errors.is_empty(), "{}", report::text(&r, src));
    let lints: Vec<String> = kinds(&r)
        .into_iter()
        .filter(|k| k.contains("OutOfScope"))
        .collect();
    assert!(lints.is_empty(), "{lints:?}");
    // the shared n is the one nothing reads
    assert!(
        r.warnings.iter().any(
            |w| matches!(w, forge_lang::error::Warning::UnusedParam { param, .. } if param.to_string() == "n")
        ),
        "{}",
        report::text(&r, src)
    );
    // four holes of the local count and two of the map: 60 x 20 less six discs of radius 2, 5 deep
    let v = r.measure.as_ref().expect("measured").volume;
    let want = (60.0 * 20.0 - 6.0 * std::f64::consts::PI * 4.0) * 5.0;
    assert!((v - want).abs() < 1e-6 * want, "{v} vs {want}");
}

/// Another feature's param read in a sketch loop is one lint per place it is read, not one per
/// loop instance, and its AddLocalParam fix names the sketch.
#[test]
fn an_out_of_scope_read_in_a_sketch_loop_names_the_sketch() {
    let src = "shared r = 2
param f.n = count(3)
param f.d = 5
param holes.step = 10
param holes.w = 60
param holes.h = 20
param holes.y = 10
sketch holes(on: xy) {
  o = rect(corner: (0, 0), size: (w, h))
  for i in 0..n {
    c = circle((step * (i + 1), y), r)
  }
}
f = extrude(holes, d)
";
    let r = run(src);
    let lints: Vec<&forge_lang::error::Error> = r
        .warnings
        .iter()
        .filter_map(|w| match w {
            forge_lang::error::Warning::Lint(e @ forge_lang::error::Error::ParamOutOfScope { .. }) => Some(e),
            _ => None,
        })
        .collect();
    assert_eq!(lints.len(), 1, "{}", report::text(&r, src));
    let forge_lang::error::Error::ParamOutOfScope { used_in, .. } = lints[0] else {
        unreachable!()
    };
    assert_eq!(forge_lang::error::path_text(used_in), "holes");
    let fixes = lints[0].fixes();
    assert!(
        fixes.iter().any(|f| matches!(f, forge_lang::error::Op::AddLocalParam { feature, .. } if forge_lang::error::path_text(feature) == "holes")),
        "{fixes:?}"
    );
}

/// A param only an expect line reads is the design's intent, not an unused param.
#[test]
fn a_param_only_expect_reads_is_used() {
    let src = "shared w = 20
shared d = 5
shared box_volume = 2000
shared spare = 3
sketch k(on: xy) {
  o = rect(corner: (0, 0), size: (w, w))
}
f = extrude(k, d)
expect volume = box_volume +- 0.1%
";
    let r = run(src);
    assert!(r.errors.is_empty(), "{}", report::text(&r, src));
    let unused: Vec<String> = r
        .warnings
        .iter()
        .filter_map(|w| match w {
            forge_lang::error::Warning::UnusedParam { param, .. } => Some(param.to_string()),
            _ => None,
        })
        .collect();
    assert_eq!(unused, vec!["spare".to_string()]);
    // still so when the part is only checked, and the expect line never evaluates
    let opts = Options {
        build: false,
        ..Options::default()
    };
    let c = forge_lang::run(src, opts, Default::default()).0;
    assert!(
        c.warnings
            .iter()
            .all(|w| !format!("{w:?}").contains("box_volume"))
    );
}

/// An invalid body says what is wrong and where: here (a flat whose ear strip touches the
/// unfolded side wall along an edge, which forge-sheet builds as an invalid flat today) the
/// flat's skin, by its wall.
#[test]
fn an_invalid_body_names_its_problems() {
    let src = "shared t = 1.5
shared r = 1.5
shared k = ratio(0.42)
shared W = 440
shared D = 300
shared E = 21.3
shared a = 20
shared H = 42
shared Hf = 43.6
shared lip = 10
param rel.w = 1.5
param rel.x0 = 5.5
param rel.x1 = 2
param flat.gap = 80
sketch plan(on: xy) {
  outline = poly([(-E, 0), (W + E, 0), (W + E, a), (W, a), (W, D), (0, D), (0, a), (-E, a)])
}
base = sheet(plan, t: t, r: r, bend: kfactor(k))
front = flange(base.edge(plan.outline[0]), height: Hf)
sketch rel(on: base.top) {
  rr = rect(corner: (W - x0, a - w), size: (x0 + x1, w))
}
reliefs = cutout(rel, wall: base)
right = flange(base.edge(plan.outline[3]), height: H)
front_lip = flange(front.tip, height: lip)
flat = unfold(base, on: offset(base.bottom, gap))
";
    let r = run(src);
    let m = r.measure.as_ref().expect("measured");
    assert!(
        m.problems > 0,
        "this flat now builds valid: give the test another invalid body\n{}",
        report::text(&r, src)
    );
    assert_eq!(m.invalid.len(), m.problems);
    for p in &m.invalid {
        assert_eq!(p.body, "flat", "{p:?}");
        assert!(!p.at.is_empty(), "{p:?}");
    }
    assert!(
        m.invalid
            .iter()
            .any(|p| p.kind == "LoopsCross" && p.at.iter().any(|a| a.contains("skin of wall base"))),
        "{:?}",
        m.invalid
    );
    let text = report::text(&r, src);
    assert!(text.contains("invalid flat LoopsCross"), "{text}");
    let json = report::json(&r, src).to_string();
    assert!(
        json.contains("\"problems\"") && json.contains("LoopsCross"),
        "{json}"
    );
}
