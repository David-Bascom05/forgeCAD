//! The evaluator against the Lean model (impl-proof-findings.md E1-E10): each counterexample
//! the differential harness found, as it was found, and what the model says it must give.

use forge_lang::edit::apply_checked;
use forge_lang::error::{Literal, Op, ParamKey};
use forge_lang::values::{Dim, Src};
use forge_lang::{Options, Run};
use std::path::{Path, PathBuf};

fn dir(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("proof-{name}"));
    let _ = std::fs::create_dir_all(&d);
    d
}

fn run_in(d: &Path, src: &str, build: bool) -> Run {
    let opts = Options {
        build,
        dir: Some(d.to_path_buf()),
        ..Options::default()
    };
    forge_lang::run(src, opts, Default::default()).0
}

fn run(src: &str) -> Run {
    run_in(&dir("plain"), src, true)
}

fn structural(r: &Run, p: &str) -> bool {
    r.observed.contains(&ParamKey::shared(p))
}

fn set(p: &str, v: f64, dim: Dim) -> Op {
    Op::SetParam {
        param: ParamKey::shared(p),
        value: Literal { value: v, dim },
    }
}

fn kinds(r: &Run) -> Vec<&'static str> {
    r.errors.iter().map(|e| e.kind()).collect()
}

const CUP: &str = "shared a = 80
component cup(d: 50) {
  sketch s(on: xy) {
    c = circle((0, 0), 5)
  }
  b = extrude(s, d)
}
c1 = part(cup(d: a))
c2 = part(cup(d: 160 - a))
";

#[test]
fn e1_equal_values_from_different_traces_are_separate_evaluations() {
    let r = run(CUP);
    assert!(r.ok(), "{:?}", kinds(&r));
    assert_eq!(
        r.components.len(),
        2,
        "{:?}",
        r.components.iter().map(|c| &c.eval_key).collect::<Vec<_>>()
    );
    // c2's depth is driven (160 - a): no handle; c1's is `a`'s
    let depth = |key: &str| {
        let c = r.components.iter().find(|c| c.eval_key.contains(key)).unwrap();
        c.feats.iter().find(|f| f.id == "cup.b").unwrap().dims[0]
            .num
            .src
            .clone()
    };
    assert_eq!(depth("d<-a="), Src::Driving(ParamKey::shared("a")));
    assert!(matches!(depth("160 - a"), Src::Driven(_)));
}

#[test]
fn e2_a_params_reach_counts_features_inside_components() {
    let src = CUP.replace(
        "c2 = part(cup(d: 160 - a))",
        "shared big = 80\nc2 = part(cup(d: big))",
    );
    let (_, reach) = apply_checked(&src, &set("big", 90.0, Dim::Length)).unwrap();
    let reach = reach.unwrap();
    assert!(
        reach.values >= 1 && reach.features.contains(&"cup.b".to_string()),
        "{reach:?}"
    );
    let r = run(&src);
    let uses = forge_lang::report::uses(&r);
    assert_eq!(
        uses.get(&ParamKey::shared("big")),
        Some(&vec!["cup.b".to_string()])
    );
}

#[test]
fn e3_each_evaluation_lists_only_its_own_occurrences() {
    let src = "shared small = 80
shared big = 80
component cup(d: 50) {
  sketch s(on: xy) {
    c = circle((0, 0), 5)
  }
  b = extrude(s, d)
}
c1 = part(cup(d: small))
c2 = part(cup(d: big))
";
    let r = run(src);
    let a = r.asm.as_ref().unwrap();
    let of = |occ: &str| a.evals.iter().find(|e| e.0 == occ).unwrap().1.clone();
    assert!(
        of("c1").contains("small") && !of("c1").contains("big"),
        "{:?}",
        a.evals
    );
    assert!(of("c2").contains("big"), "{:?}", a.evals);
    let j = forge_lang::report::json(&r, src).to_string();
    assert!(
        j.contains("\"occurrences\":[\"c1\"]") && j.contains("\"occurrences\":[\"c2\"]"),
        "{j}"
    );
}

const LINKS: &str = "shared n = count(2)
shared m = n + 1
shared t = 6
shared u = 7
shared r = t + 1
shared k = count(1)
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (10, 10))
}
for i in 0..m {
  e = extrude(plan, [t, u, t][i], new: true)
}
g = extrude(plan, t if r == 7 else u, new: true)
c = extrude(plan, [t, u][k], new: true)
";

#[test]
fn e4_an_observed_link_observes_what_it_is_made_of() {
    let r = run_in(&dir("e4"), LINKS, false);
    assert!(r.ok(), "{:?}", kinds(&r));
    for p in ["m", "n", "r", "t"] {
        assert!(structural(&r, p), "{p} is structural");
    }
    let (_, reach) = apply_checked(LINKS, &set("n", 1.0, Dim::Count)).unwrap();
    assert!(reach.unwrap().structural);
}

#[test]
fn e5_an_index_or_a_loop_count_is_control_not_data() {
    let r = run_in(&dir("e5"), LINKS, false);
    let src_of = |id: &str| {
        r.feats.iter().find(|f| f.id == id).unwrap().dims[0]
            .num
            .src
            .clone()
    };
    assert_eq!(src_of("c"), Src::Driving(ParamKey::shared("u")));
    assert_eq!(src_of("e[1]"), Src::Driving(ParamKey::shared("u")));
    assert!(structural(&r, "k"));
}

#[test]
fn e6_a_literal_default_is_its_own_handle() {
    let src = "component cup(d: 50, e: 4) {
  sketch s(on: xy) {
    c = circle((0, 0), e)
  }
  b = extrude(s, d)
}
c1 = part(cup)
";
    let r = run(src);
    assert!(r.ok(), "{:?}", kinds(&r));
    let b = r.components[0].feats.iter().find(|f| f.id == "cup.b").unwrap();
    let key = ParamKey {
        owner: vec![forge_lang::edit::seg("cup")],
        name: "d".into(),
    };
    assert_eq!(b.dims[0].num.src, Src::Driving(key.clone()));
    let (text, _) = apply_checked(
        src,
        &Op::SetParam {
            param: key,
            value: Literal {
                value: 60.0,
                dim: Dim::Length,
            },
        },
    )
    .unwrap();
    assert!(text.contains("component cup(d: 60, e: 4) {"), "{text}");
}

#[test]
fn e7_a_component_no_one_places_observes_nothing() {
    let src = "shared w = 4
shared v = 5
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (10, 10))
}
component cp(d: 3) {
  sketch s(on: xy) {
    o = circle((0, 0), 5)
  }
  b = extrude(s, d if w == 4 else v)
}
e = extrude(plan, v)
";
    let r = run_in(&dir("e7"), src, false);
    assert!(r.ok(), "{:?}", kinds(&r));
    assert!(!structural(&r, "w"));
}

#[test]
fn e8_a_loop_that_runs_no_times_still_observes_its_count() {
    let src = "shared n = count(0)
shared w = 4
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (10, 10))
}
for i in 0..n {
  e = extrude(plan, w)
}
";
    let r = run_in(&dir("e8"), src, false);
    assert!(structural(&r, "n"));
}

#[test]
fn e9_a_used_files_components_and_with_place_parts() {
    let d = dir("e9");
    std::fs::write(
        d.join("lib.fg"),
        "shared W = 7\ncomponent cup(d: 9) {\n  sketch s(on: xy) {\n    o = circle((0, 0), 5)\n  }\n  b = extrude(s, d + W)\n}\nsketch p(on: xy) {\n  o = rect(center: (0, 0), size: (W, W))\n}\nplate = extrude(p, W)\n",
    )
    .unwrap();
    let r = run_in(
        &d,
        "t = use(\"lib.fg\")\nshared d = 3\nc1 = part(t.cup(d: d), ground: true)\nc2 = part(t.cup, at: (50, 0, 0))\nc3 = part(\"lib.fg\", with: {W: 12}, at: (100, 0, 0))\n",
        true,
    );
    assert!(
        r.ok(),
        "{:?}",
        r.errors.iter().map(|e| e.text()).collect::<Vec<_>>()
    );
    let a = r.asm.as_ref().unwrap();
    let files: Vec<&str> = a.bom.iter().map(|b| b.0.as_str()).collect();
    assert_eq!(files, ["lib.fg|cup()", "lib.fg|cup(d=3)", "lib.fg|with:W=12"]);
    // each its own part: the call's depth d + W, the file's plate with W set
    let vol = |f: &str| {
        let s = &a.part_docs[f];
        s.masses().unwrap().iter().map(|m| m.volume).sum::<f64>()
    };
    let disc = std::f64::consts::PI * 25.0;
    assert!((vol("lib.fg|cup(d=3)") - disc * 10.0).abs() < 1e-6);
    assert!((vol("lib.fg|cup()") - disc * 16.0).abs() < 1e-6);
    assert!((vol("lib.fg|with:W=12") - 12.0f64.powi(3)).abs() < 1e-6);
    // a param the file has not, or a component it has not, is refused typed
    let r = run_in(&d, "c = part(\"lib.fg\", with: {Q: 1})\n", true);
    assert!(
        r.errors.iter().any(|e| e.text().contains("no shared param Q")),
        "{:?}",
        kinds(&r)
    );
    let r = run_in(&d, "t = use(\"lib.fg\")\nc = part(t.mug)\n", true);
    assert!(!r.ok());
}

#[test]
fn e10_a_default_sees_only_the_shared_params() {
    let src = "shared w = 4
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (10, 10))
}
d = w + 100
component cup(d: 80, h: d * 2) {
  sketch s(on: xy) {
    o = circle((0, 0), 5)
  }
  b = extrude(s, h)
}
c = part(cup(d: 50))
c0 = part(cup)
";
    let r = run(src);
    assert!(kinds(&r).contains(&"UnknownName"), "{:?}", kinds(&r));
    // a default reading a shared param is fine
    let ok = run(&src.replace("h: d * 2", "h: w * 2"));
    assert!(ok.ok(), "{:?}", kinds(&ok));
}

/// A component's call and its definition's check keep the session's fence: a font outside the
/// root is refused inside a component as it is at the document's top level.
#[test]
fn a_component_default_keeps_the_sessions_fence() {
    let d = dir("fence");
    let inside = d.join("session");
    std::fs::create_dir_all(&inside).unwrap();
    // a default reads no file (`use` must be bound, features named), so the reads are the
    // component's own statements: fenced as the document's are
    std::fs::write(d.join("outside.ttf"), forge_agent::bench::font_bytes()).unwrap();
    for path in ["/etc/passwd", "../outside.ttf"] {
        let src = format!(
            "component c(w: 4) {{\n  sketch s(on: xy) {{\n    o = circle((0, 0), w)\n  }}\n  b = extrude(s, w)\n  t = text(\"A\", font: \"{path}\", size: w, depth: w, on: b.end)\n}}\nx = part(c)\n"
        );
        let r = forge_lang::run(
            &src,
            Options {
                dir: Some(inside.clone()),
                root: Some(inside.clone()),
                ..Options::default()
            },
            Default::default(),
        )
        .0;
        assert!(kinds(&r).contains(&"File"), "{path}: {:?}", kinds(&r));
        // the fence is what refuses it: unfenced, the font beside the session reads
        if path.starts_with("..") {
            let free = forge_lang::run(
                &src,
                Options {
                    dir: Some(inside.clone()),
                    ..Options::default()
                },
                Default::default(),
            )
            .0;
            assert!(!kinds(&free).contains(&"File"), "{:?}", kinds(&free));
        }
    }
}

/// An edit that adds an error is refused even when the document already has one of that kind.
#[test]
fn an_edit_adding_a_second_error_of_a_kind_is_refused() {
    let src = "shared w = 11
shared n = count(1)
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (10, 10))
}
for k in {lf: w} {
  e = extrude(plan, [[w][n], w][n])
}
for i in 0..n {
  g = extrude(plan, [w, w][n])
}
";
    let before = run(src);
    assert!(kinds(&before).contains(&"IndexOut"), "{:?}", kinds(&before));
    let got = apply_checked(src, &set("n", 3.0, Dim::Count));
    assert!(
        matches!(&got, Err(e) if e.kind() == "IndexOut"),
        "{:?}",
        got.map(|x| x.0)
    );
}

/// A component's defaults are params of the document's run: listed, with their handle and
/// whether evaluation observed them.
#[test]
fn component_defaults_are_listed_with_their_handle_and_structure() {
    let src = "component c(k: 2, d: 50) {
  sketch s(on: xy) {
    o = circle((0, 0), 5)
  }
  for i in 0..k {
    b = extrude(s, d, new: true)
  }
}
x = part(c)
";
    let r = run(src);
    assert!(r.ok(), "{:?}", kinds(&r));
    let j = forge_lang::report::json(&r, src).to_string();
    let entry = |name: &str| {
        let at = j
            .find(&format!("\"param\":\"{name}\""))
            .unwrap_or_else(|| panic!("{name} listed: {j}"));
        j[at..at + 200].to_string()
    };
    assert!(entry("c.k").contains("\"structural\":true"), "{}", entry("c.k"));
    assert!(entry("c.d").contains("\"structural\":false"), "{}", entry("c.d"));
    let used = forge_lang::report::uses(&r);
    let d = ParamKey {
        owner: vec![forge_lang::edit::seg("c")],
        name: "d".into(),
    };
    assert!(
        used.get(&d)
            .is_some_and(|f| f.iter().any(|x| x.starts_with("c.b"))),
        "{used:?}"
    );
}

const XCUP: &str = "shared W = 4
shared H = 3
component cup(d: 5) {
  sketch s(on: xy) {
    o = circle((0, 0), 5)
  }
  b = extrude(s, d if W == 4 else H)
}
c1 = part(cup(d: H))
";

fn extracted(name: &str, src: &str) -> (PathBuf, String) {
    use forge_lang::refactor::{Refactor, apply_checked};
    let d = dir(name);
    let _ = std::fs::remove_file(d.join("cup.fg"));
    let read = |f: &str| {
        std::fs::read_to_string(d.join(f)).map_or(
            forge_lang::refactor::Found::Missing,
            forge_lang::refactor::Found::Text,
        )
    };
    let w = apply_checked(
        src,
        &Refactor::ExtractComponent {
            component: "cup".into(),
            file: "cup.fg".into(),
        },
        &read,
    )
    .unwrap();
    for (f, t) in &w.files {
        std::fs::write(d.join(f), t).unwrap();
    }
    (d, w.doc)
}

/// X1: provenance goes through `with:`: what the part reads is a param's use, its reach.
#[test]
fn x1_provenance_goes_through_with() {
    let before = run_in(&dir("x1-in"), XCUP, true);
    let uses = forge_lang::report::uses(&before);
    assert_eq!(uses.get(&ParamKey::shared("H")), Some(&vec!["cup.b".to_string()]));
    let (d, doc) = extracted("x1", XCUP);
    let after = run_in(&d, &doc, true);
    assert!(after.ok(), "{:?}", kinds(&after));
    let uses = forge_lang::report::uses(&after);
    assert_eq!(
        uses.get(&ParamKey::shared("H")),
        Some(&vec!["cup.b".to_string()]),
        "{doc}"
    );
}

/// X2: what the part observes stays structural after it moves to its own file.
#[test]
fn x2_what_the_part_observes_stays_structural() {
    assert!(structural(&run_in(&dir("x2-in"), XCUP, false), "W"));
    let (d, doc) = extracted("x2", XCUP);
    let after = run_in(&d, &doc, true);
    assert!(structural(&after, "W"), "{doc}");
}

/// X3, X4: Extract then Inline gives the text back, an unplaced component too.
#[test]
fn x3_x4_extract_then_inline_is_the_same_text() {
    use forge_lang::refactor::{Refactor, apply_checked};
    for src in [
        XCUP.to_string(),
        XCUP.replace("c1 = part(cup(d: H))\n", "e = 1\n"),
    ] {
        let (d, doc) = extracted("x34", &src);
        let read = |f: &str| {
            std::fs::read_to_string(d.join(f)).map_or(
                forge_lang::refactor::Found::Missing,
                forge_lang::refactor::Found::Text,
            )
        };
        let back = apply_checked(
            &doc,
            &Refactor::InlineComponent {
                file: "cup.fg".into(),
                name: "cup".into(),
            },
            &read,
        )
        .unwrap();
        assert_eq!(back.doc, src);
    }
}

/// A component key is never a file: `#` names are refused where files are named, never read.
#[test]
fn a_component_key_never_reaches_a_file() {
    let d = dir("hashkey");
    let inside = d.join("in");
    std::fs::create_dir_all(&inside).unwrap();
    std::fs::write(d.join("outside.fg"), "shared W = 5\n").unwrap();
    let opts = Options {
        dir: Some(inside.clone()),
        root: Some(inside.clone()),
        ..Options::default()
    };
    for src in [
        "t = use(\"#../outside.fg\")\n",
        "t = use(\"#/etc/passwd\")\n",
        "p = part(\"#../outside.fg\")\n",
        "p = part(\"#/etc/passwd\", ground: true)\n",
    ] {
        let r = forge_lang::run(src, opts.clone(), Default::default()).0;
        assert!(!r.ok(), "{src}");
        let said: String = r.errors.iter().map(|e| e.text()).collect();
        assert!(!said.contains("root:"), "{said}");
    }
}

/// One budget for the whole run: a tree of components each placing ten distinct calls of the
/// next stops with Fuel, quickly.
#[test]
fn a_fan_out_of_components_runs_out_of_fuel() {
    let mut src = String::from("component l7(k: 0) {\n}\n");
    for l in (0..7).rev() {
        src.push_str(&format!(
            "component l{l}(k: 0) {{\n  for i in 0..10 {{\n    x = part(l{}(k: i))\n  }}\n}}\n",
            l + 1
        ));
    }
    src.push_str("top = part(l0)\n");
    let t = std::time::Instant::now();
    let r = forge_lang::run(&src, Options::default(), Default::default()).0;
    assert!(r.errors.iter().any(|e| e.kind() == "Fuel"), "{:?}", kinds(&r));
    assert!(t.elapsed().as_secs() < 60, "{:?}", t.elapsed());
}

/// X1 (reach): after Extract, a SetParam on a param the file part reads reaches its values.
#[test]
fn x1_reach_goes_through_with() {
    let (d, doc) = extracted("x1-reach", XCUP);
    let (_, reach) = forge_lang::edit::apply_checked_in(&doc, &set("H", 4.0, Dim::Length), Some(d)).unwrap();
    let reach = reach.unwrap();
    assert!(
        reach.values >= 1 && reach.features.contains(&"cup.b".to_string()),
        "{reach:?}"
    );
}

/// A sketch on an occurrence's face: where that occurrence is written, made in context.
#[test]
fn a_sketch_sits_on_an_occurrences_face() {
    let src = "shared bw = 40
shared h = 20
shared r = 3
component box(w: 40) {
  sketch s(on: xy) {
    o = rect(center: (0, 0), size: (w, w))
  }
  wall = extrude(s, h)
}
b1 = part(box(w: bw), ground: true, at: (0, 0, 5))
sketch t(on: b1.wall.end) {
  c = circle((0, 0), r)
}
knob = extrude(t, r)
";
    let r = run(src);
    assert!(
        r.ok(),
        "{:?}",
        r.errors.iter().map(|e| e.text()).collect::<Vec<_>>()
    );
    let f = r.feats.iter().find(|f| f.id == "t").unwrap().frame.unwrap();
    assert!((f.origin.z - 25.0).abs() < 1e-9, "{:?}", f.origin);
}
