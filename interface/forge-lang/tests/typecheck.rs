//! Type errors and lints: each a typed record with a span and typed fixes, found without the
//! kernel (`check`).

use forge_lang::error::{Error, Op, Warning};
use forge_lang::{Options, Run};

fn check(src: &str) -> Run {
    let opts = Options {
        build: false,
        ..Options::default()
    };
    forge_lang::run(src, opts, Default::default()).0
}

fn errors(src: &str) -> Vec<&'static str> {
    check(src).errors.iter().map(Error::kind).collect()
}

fn lints(r: &Run) -> Vec<&Error> {
    r.warnings
        .iter()
        .filter_map(|w| match w {
            Warning::Lint(e) => Some(e),
            _ => None,
        })
        .collect()
}

const BASE: &str = "shared L = 40\nshared t = 5\nsketch s(on: xy) {\n  o = rect(corner: (0, 0), size: (L, L))\n}\nb = extrude(s, t)\n";

#[test]
fn a_clean_part_checks() {
    let r = check(BASE);
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert!(lints(&r).is_empty());
    let ids: Vec<&str> = r.feats.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["s", "b"]);
}

#[test]
fn mixing_dimensions_is_a_type_error() {
    let src = "shared a = 10\nshared b = 30deg\nx = a + b\n";
    assert_eq!(errors(src), ["DimMismatch"]);
    // a size is never a count
    let src = "shared a = 10\nfor i in 0..a {\n  y = i\n}\n";
    assert_eq!(errors(src), ["WrongDim"]);
    // a count scales a size
    let src = "shared a = 10\nshared n = count(3)\nfor i in 0..n {\n  y = a * i\n}\n";
    assert!(errors(src).is_empty());
    // trig takes angles
    let src = "shared a = 10\nx = sin(a)\n";
    assert_eq!(errors(src), ["WrongDim"]);
}

#[test]
fn a_loose_number_is_a_lint_with_lift_fixes() {
    let src = format!("{BASE}param c.k = 3\nsketch c(on: b.end) {{\n  p = pt(7, k)\n}}\n");
    let r = check(&src);
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let l = lints(&r);
    assert_eq!(l.len(), 1);
    let Error::LooseLiteral { value, span, .. } = l[0] else {
        panic!("{:?}", l[0])
    };
    assert_eq!(*value, 7.0);
    assert_eq!(&src[span.lo as usize..span.hi as usize], "7");
    let fixes = l[0].fixes();
    assert!(
        matches!(&fixes[0], Op::LiftLiteral { scope: forge_lang::error::Scope::Local(o), .. } if o[0].name == "c")
    );
    assert!(matches!(
        &fixes[1],
        Op::LiftLiteral {
            scope: forge_lang::error::Scope::Shared,
            ..
        }
    ));
    // the fix applies and the lint is gone
    let fixed = forge_lang::edit::apply(&src, &fixes[0]).expect("applies");
    assert!(lints(&check(&fixed)).is_empty(), "{fixed}");
}

#[test]
fn another_features_param_is_a_lint_with_both_fixes() {
    let src = format!(
        "{BASE}param c.r = 3\nsketch c(on: b.end) {{\n  k = circle((r, r), r)\n}}\nd = extrude(c, r)\n"
    );
    let r = check(&src);
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let l = lints(&r);
    assert_eq!(l.len(), 1, "{l:?}");
    assert_eq!(l[0].kind(), "ParamOutOfScope");
    let fixes = l[0].fixes();
    assert!(
        matches!(&fixes[0], Op::AddLocalParam { feature, name, .. } if feature[0].name == "d" && name == "r")
    );
    assert!(matches!(&fixes[1], Op::MakeShared { param } if param.to_string() == "c.r"));
    for f in &fixes {
        let fixed = forge_lang::edit::apply(&src, f).expect("applies");
        let r2 = check(&fixed);
        assert!(
            r2.errors.is_empty() && lints(&r2).is_empty(),
            "{fixed}\n{:?}",
            r2.errors
        );
    }
}

#[test]
fn world_placement_after_the_first_feature_is_a_lint() {
    let src = format!("{BASE}sketch c(on: xy) {{\n  k = circle((L, L), t)\n}}\n");
    let r = check(&src);
    assert!(r.errors.is_empty());
    assert_eq!(lints(&r)[0].kind(), "WorldAfterFirst");
}

#[test]
fn names_are_bound_once() {
    assert_eq!(errors("shared a = 1\nx = a\nx = a\n"), ["Rebind"]);
    // no shadowing: a loop variable may not hide a top-level name (found before evaluating)
    assert_eq!(
        errors("shared a = 1\ni = a\nfor i in 0..2 {\n  y = a\n}\n")[0],
        "ShadowsTopLevel"
    );
    // nor a function's local, even one bound before the top-level name
    assert_eq!(
        errors("shared a = 1\nfn f(x) {\n  k = x\n  return k\n}\nk = f(a)\n"),
        ["ShadowsTopLevel"]
    );
    // a function body reads top-level names
    assert!(errors("shared a = 1\nb = a\nfn f(x) {\n  return x + b\n}\nc = f(a)\n").is_empty());
    // nor a param
    assert_eq!(errors("shared a = 1\nfor k in 0..2 {\n  a = k\n}\n"), ["Rebind"]);
    assert_eq!(errors("shared sqrt = 1\n"), ["ShadowsBuiltin"]);
}

#[test]
fn ids_come_from_names_ranges_and_labels() {
    let src = format!(
        "{BASE}param h.d = 2\nsketch c(on: b.end) {{\n  for k, p in {{left: (t, t), right: (L - t, t)}} {{\n    q = pt(p.x, p.y)\n  }}\n}}\nfor k, q in c.q {{\n  h = hole(at: q, d: d, depth: thru)\n}}\nfor i in 0..2 {{\n  g = hole(at: c.q.left, d: d, depth: thru)\n}}\n"
    );
    let r = check(&src);
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let ids: Vec<&str> = r.feats.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["s", "b", "c", "h[left]", "h[right]", "g[0]", "g[1]"]);
}

#[test]
fn a_feature_in_a_plain_list_loop_has_no_id() {
    let src = format!(
        "{BASE}param h.d = 2\nsketch c(on: b.end) {{\n  p = pt(t, t)\n}}\nfor q in [c.p] {{\n  h = hole(at: q, d: d, depth: thru)\n}}\n"
    );
    assert_eq!(errors(&src), ["PositionalId"]);
}

#[test]
fn features_need_names_and_ids_are_unique() {
    assert_eq!(errors(&format!("{BASE}extrude(s, t)\n")), ["UnnamedFeature"]);
    assert_eq!(
        errors(&format!(
            "{BASE}fn f(p) {{\n  e = extrude(p, t)\n  return e\n}}\nx = f(s)\ny = f(s)\n"
        )),
        Vec::<&str>::new()
    );
    // a call's name is part of its features' ids
    let r = check(&format!(
        "{BASE}fn f(p) {{\n  e = extrude(p, t)\n  return e\n}}\nx = f(s)\ny = f(s)\n"
    ));
    let ids: Vec<&str> = r.feats.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["s", "b", "x.e", "y.e"]);
}

#[test]
fn references_are_absolute() {
    let src = "shared a = 1\nxs = [a, a]\ny = xs[2]\n";
    assert_eq!(errors(src), ["IndexOut"]);
    let src = "shared a = 1\nxs = [a, a]\ny = xs[-1]\n";
    assert_eq!(errors(src), ["IndexOut"]);
}

#[test]
fn selection_by_position_is_an_error() {
    let src = format!("{BASE}f = fillet(edges(of: b, extreme: Z), t)\n");
    assert_eq!(errors(&src), ["PositionalSelection"]);
}

#[test]
fn recursion_runs_on_a_budget() {
    let src = "shared a = 1\nfn f(x) {\n  return f(x)\n}\ny = f(a)\n";
    assert_eq!(errors(src), ["Fuel"]);
    let src = "shared n = count(5)\nfn down(k) {\n  return 0 if k == 0 else down(k - 1)\n}\ny = down(n)\n";
    assert!(errors(src).is_empty());
}

#[test]
fn unknown_names_wrong_types_and_syntax() {
    assert_eq!(errors("y = nope\n"), ["UnknownName"]);
    assert_eq!(errors(&format!("{BASE}e = extrude(b, t)\n")), ["WrongType"]);
    assert_eq!(
        errors(&format!("{BASE}e = extrude(s, t, depth: t)\n")),
        ["UnknownArg"]
    );
    let r = check("shared a = (1, 2\n");
    assert_eq!(r.errors[0].kind(), "Syntax");
    assert!(r.errors[0].span().is_some());
}

#[test]
fn provenance_drives_and_derives() {
    use forge_lang::values::Src;
    let src = format!(
        "{BASE}param c.r = 3\nsketch c(on: b.end) {{\n  k = circle((L / 2, L / 2), r)\n}}\nd = extrude(c, t * 2)\ne = extrude(c, t)\n"
    );
    let r = check(&src);
    let d = r.feats.iter().find(|f| f.id == "d").expect("d");
    let e = r.feats.iter().find(|f| f.id == "e").expect("e");
    assert!(matches!(&d.dims[0].num.src, Src::Driven(ps) if ps.len() == 1));
    assert!(matches!(&e.dims[0].num.src, Src::Driving(p) if p.to_string() == "t"));
}

#[test]
fn observed_params_are_structural() {
    // a loop count and a comparison are observed; a plain size is not
    let src = format!(
        "{BASE}shared n = count(2)\nshared lim = 3\nfor i in 0..n {{\n  y = i\n}}\nz = 1 if t > lim else 2\n"
    );
    let r = check(&src);
    let names: Vec<String> = r.observed.iter().map(|p| p.to_string()).collect();
    assert!(
        names.contains(&"n".to_string())
            && names.contains(&"lim".to_string())
            && names.contains(&"t".to_string())
    );
    assert!(!names.contains(&"L".to_string()));
}

#[test]
fn errors_and_lints_serialize_with_fixes() {
    let r = check(&format!("{BASE}sketch c(on: b.end) {{\n  p = pt(7, t)\n}}\n"));
    let mut s = String::new();
    r.warnings[0].json(BASE).write(&mut s);
    assert!(
        s.contains("\"severity\":\"lint\"") && s.contains("\"fixes\":[{\"op\":\"LiftLiteral\""),
        "{s}"
    );
}
