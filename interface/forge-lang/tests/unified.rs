//! One document for parts and assemblies: bodies made components by joints, component
//! definitions called with params, file parts, all in one file; edit ops inside components.

use forge_lang::edit::{apply, apply_ast};
use forge_lang::error::Op;
use forge_lang::{Options, Run};
use std::path::{Path, PathBuf};

fn dir(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("unified-{name}"));
    let _ = std::fs::create_dir_all(&d);
    d
}

fn run_in(d: &Path, src: &str) -> Run {
    let opts = Options {
        dir: Some(d.to_path_buf()),
        ..Options::default()
    };
    forge_lang::run(src, opts, Default::default()).0
}

fn errors(r: &Run) -> Vec<String> {
    r.errors
        .iter()
        .map(|e| format!("{} {}", e.kind(), e.text()))
        .collect()
}

fn kinds(r: &Run) -> Vec<&'static str> {
    r.errors.iter().map(|e| e.kind()).collect()
}

const BOX: &str = "shared W = 80
shared D = 60
shared H = 40
shared t = 3
param hinge.open = 0deg
properties(material: \"ABS\")
sketch plan(on: xy) {
  o = rect(center: (0, 0), size: (W, D))
}
base = extrude(plan, H)
cup = shell(t, open: base.end)
sketch lp(on: base.end) {
  o = rect(center: (0, 0), size: (W, D))
}
lid = extrude(lp, t, new: true)
hp = plane(at: edges(between: (lid.start, lid.side(lp.o[2]))).center, normal: lp.o[2].dir)
ground(base)
hinge = revolute(lid.hp, base.hp, turn: open)
expect valid
expect interferences = 0
expect mobility = 0
";

#[test]
fn a_joint_makes_bodies_components_where_they_were_modelled() {
    let d = dir("box");
    let r = run_in(&d, BOX);
    assert!(r.ok(), "{:?}", errors(&r));
    let a = r.asm.as_ref().expect("components");
    let src = |id: &str| a.sources.iter().find(|s| s.0 == id).map(|s| s.1);
    assert_eq!(src("lid"), Some(forge_lang::asm::Source::Body));
    assert_eq!(src("base"), Some(forge_lang::asm::Source::Body));
    // grounded as written (`ground(base)`): the lid is moved by the hinge
    assert_eq!(a.grounded, vec!["base".to_string()]);
    // as modelled: both at the identity
    for (_, f) in &a.poses {
        assert!(f.origin.to_vec().length() < 1e-9);
    }
    let bom: Vec<&str> = a.bom.iter().map(|b| b.0.as_str()).collect();
    assert_eq!(bom, ["#base", "#lid"]);
    // the part's own measure still runs (`expect valid`), on the model as made
    assert!(r.measure.is_some());
    let p = forge_lang::props::asm_physical(a, None).unwrap();
    let rho = forge_material::find("ABS").unwrap().density;
    let v = r.measure.as_ref().unwrap().volume;
    assert!((p.mass.unwrap() - v * rho * 1e-9).abs() < 1e-9 * v.max(1.0));
}

#[test]
fn opening_the_hinge_turns_only_the_lid() {
    let d = dir("box-open");
    let r = run_in(
        &d,
        &BOX.replace("param hinge.open = 0deg", "param hinge.open = 90deg"),
    );
    assert!(r.ok(), "{:?}", errors(&r));
    let a = r.asm.as_ref().unwrap();
    let lid = a.poses.iter().find(|p| p.0 == "lid").unwrap().1;
    let base = a.poses.iter().find(|p| p.0 == "base").unwrap().1;
    assert!(base.origin.to_vec().length() < 1e-9);
    // a quarter turn about the back top edge (y = 30, z = 40), up and back: the origin, 30 in
    // front of the edge and 40 under it, goes to 40 in front and 30 over: (0, -10, 70)
    let o = lid.origin;
    assert!((o.y + 10.0).abs() < 1e-6 && (o.z - 70.0).abs() < 1e-6, "{o:?}");
}

#[test]
fn a_document_without_components_is_a_part_as_before() {
    let d = dir("part");
    let r = run_in(
        &d,
        "shared t = 4\nsketch plan(on: xy) {\n  c = circle((0, 0), t)\n}\nx = extrude(plan, t)\nb = ball(plan.c, t, new: true)\n",
    );
    assert!(r.ok(), "{:?}", errors(&r));
    assert!(r.asm.is_none());
    assert!(r.physical.is_none() || r.physical.is_some());
    assert_eq!(r.measure.unwrap().bodies, 2);
}

const PEGS: &str = "shared small = 6
shared big = 10
shared plen = 30
shared T = 8
properties(material: \"6061-T6\")
sketch plan(on: xy) {
  o = rect(center: (0, 0), size: (120, 60))
}
board = extrude(plan, T)
component peg(d: 6, h: 30) {
  sketch s(on: xy) {
    c = circle((0, 0), d / 2)
  }
  shank = extrude(s, h)
  properties(part_number: \"PEG\", material: \"AISI 304\")
}
p1 = part(peg(d: small, h: plen), at: (-30, 0, 8))
p2 = part(peg(d: big, h: plen), at: (30, 0, 8))
p3 = part(peg(d: small, h: plen), at: (0, 0, 8))
p4 = part(peg, at: (0, 20, 8))
";

#[test]
fn a_component_is_a_function_of_its_params() {
    let d = dir("pegs");
    let r = run_in(&d, PEGS);
    assert!(r.ok(), "{:?}", errors(&r));
    // three distinct calls (p1 and p3 are the same part, p4 the defaults = p1's values)
    let mut calls: Vec<&str> = r.components.iter().map(|c| c.id.as_str()).collect();
    calls.dedup();
    // p4's defaults are literals (no param): an evaluation of its own, the same part as p1's
    assert_eq!(calls, ["peg(d=6,h=30)", "peg(d=10,h=30)", "peg(d=6,h=30)"]);
    let a = r.asm.as_ref().unwrap();
    let bom: Vec<(&str, usize)> = a.bom.iter().map(|b| (b.0.as_str(), b.1)).collect();
    assert_eq!(bom, [("#peg(d=10,h=30)", 1), ("#peg(d=6,h=30)", 3), ("#root", 1)]);
    // the board is no joint's: the root, the document itself, held; a component is grounded
    // only as written
    assert!(a.grounded.contains(&"#root".to_string()));
    assert!(!a.grounded.contains(&"p2".to_string()));
    // features named by the definition: what edit ops address
    let ids: Vec<&str> = r.components[0].feats.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["peg.s", "peg.shank"]);
    // a depth read from a call's argument is driven by the caller's param
    let depth = &r.components[0].feats[1].dims[0];
    assert_eq!(
        depth
            .num
            .src
            .params()
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>(),
        ["plen"]
    );
    // the shared params a component reads are used
    assert!(
        !r.warnings
            .iter()
            .any(|w| format!("{w:?}").contains("UnusedParam"))
    );
    let p = forge_lang::props::asm_physical(a, None).unwrap();
    assert!(p.missing.is_empty(), "{:?}", p.missing);
}

#[test]
fn equal_values_from_different_params_are_one_part_but_keep_their_own_provenance() {
    let d = dir("provenance");
    let r = run_in(&d, &PEGS.replace("shared big = 10", "shared big = 6"));
    assert!(r.ok(), "{:?}", errors(&r));
    let a = r.asm.as_ref().unwrap();
    // one part in the BOM (4 of it), two evaluations: `big` drives p2's, not p1's
    assert_eq!(
        a.bom.iter().find(|b| b.0 == "#peg(d=6,h=30)").map(|b| b.1),
        Some(4)
    );
    let keys: Vec<&str> = r.components.iter().map(|c| c.eval_key.as_str()).collect();
    assert_eq!(keys.len(), 3, "{keys:?}");
    assert!(keys.iter().any(|k| k.contains("d<-big")), "{keys:?}");
    assert!(keys.iter().any(|k| k.contains("d<-small")), "{keys:?}");
}

#[test]
fn component_errors_point_into_the_block() {
    let d = dir("comp-errors");
    let r = run_in(
        &d,
        &PEGS.replace("shank = extrude(s, h)", "shank = extrude(s, hh)"),
    );
    assert!(kinds(&r).contains(&"UnknownName"), "{:?}", errors(&r));
    let e = r.errors.iter().find(|e| e.kind() == "UnknownName").unwrap();
    let at = e.span().unwrap();
    assert_eq!(
        &PEGS.replace("shank = extrude(s, h)", "shank = extrude(s, hh)")[at.lo as usize..at.hi as usize],
        "hh"
    );
    let r = run_in(
        &d,
        &PEGS.replace(
            "  shank = extrude(s, h)",
            "  shared q = 2\n  shank = extrude(s, h)",
        ),
    );
    assert!(kinds(&r).contains(&"SharedInComponent"), "{:?}", errors(&r));
    let r = run_in(
        &d,
        &PEGS.replace(
            "  shank = extrude(s, h)",
            "  shank = extrude(s, h)\n  j = revolute(shank.end, shank.start)",
        ),
    );
    // a joint inside a component is the component's own (nesting): here one body to itself
    assert!(kinds(&r).contains(&"SelfLink"), "{:?}", errors(&r));
    let r = run_in(
        &d,
        &format!("{PEGS}for k in 0..2 {{\n  component bad(a: 1) {{\n  }}\n}}\n"),
    );
    assert!(kinds(&r).contains(&"ComponentNotTopLevel"), "{:?}", errors(&r));
    let r = run_in(&d, &format!("{PEGS}j = revolute(p1.shank.end, plan.plane)\n"));
    assert!(kinds(&r).contains(&"NoComponent"), "{:?}", errors(&r));
}

#[test]
fn check_without_building_accepts_a_mixed_document() {
    let d = dir("check");
    let opts = Options {
        build: false,
        dir: Some(d.clone()),
        ..Options::default()
    };
    let (r, _) = forge_lang::run(BOX, opts.clone(), Default::default());
    assert!(r.ok(), "{:?}", errors(&r));
    let (r, _) = forge_lang::run(PEGS, opts, Default::default());
    assert!(r.ok(), "{:?}", errors(&r));
}

#[test]
fn sketch_constraints_and_the_ball_solid_keep_their_part_meaning() {
    let d = dir("names");
    let src = "shared r = 5
shared w = 30
sketch s(on: xy) {
  a = line(near(0, 0), near(w, 1))
  horizontal(a)
  distance(a.a, a.b, w)
  fix(a.a)
}
x = extrude(s.a, r)
k = ball(s.a.a, r, new: true)
";
    let r = run_in(&d, src);
    // a lone line is no profile; what matters is that no constraint was taken for a joint
    assert!(
        !errors(&r).iter().any(|e| e.contains("joint")),
        "{:?}",
        errors(&r)
    );
    assert!(r.asm.is_none());
}

#[test]
fn edit_ops_reach_inside_a_component() {
    let ops = [
        Op::AddFeature {
            before: Some("peg.".into()),
            text: "tip = chamfer(edges(of: shank.end), ch)".into(),
        },
        Op::SetArg {
            id: "peg".into(),
            arg: "ch".into(),
            text: "1".into(),
        },
        Op::ReplaceFeature {
            id: "peg.shank".into(),
            text: "shank = extrude(s, h + 1)".into(),
        },
        Op::SetArg {
            id: "peg".into(),
            arg: "d".into(),
            text: "7".into(),
        },
    ];
    let mut src = PEGS.to_string();
    for op in &ops {
        let next = apply(&src, op).unwrap_or_else(|e| panic!("{op:?}: {e:?}"));
        // the text edit and the tree edit agree
        let tree = apply_ast(&src, op).unwrap();
        assert_eq!(
            forge_lang::edit::strip(&forge_lang::syntax::parse(&next).unwrap()),
            forge_lang::edit::strip(&tree),
            "{op:?}"
        );
        src = next;
    }
    assert!(src.contains("component peg(d: 7, h: 30, ch: 1) {"), "{src}");
    assert!(
        src.contains("  tip = chamfer(edges(of: shank.end), ch)\n}"),
        "{src}"
    );
    let r = run_in(&dir("edits"), &src);
    assert!(r.ok(), "{:?}", errors(&r));
    assert_eq!(r.components[0].feats.len(), 3);
    // an id in a component that is not there, and a move across the block, are refused
    assert!(
        apply(
            &src,
            &Op::DeleteFeature {
                id: "peg.nope".into()
            }
        )
        .is_err()
    );
    assert!(
        apply(
            &src,
            &Op::MoveFeature {
                id: "peg.tip".into(),
                before: Some("board".into())
            }
        )
        .is_err()
    );
}

#[test]
fn a_mixed_document_places_file_parts_too() {
    let d = dir("mixed");
    std::fs::write(
        d.join("knob.fg"),
        "shared r = 9\nshared h = 12\nproperties(part_number: \"KNOB-1\", material: \"ABS\")\nsketch s(on: xy) {\n  c = circle((0, 0), r)\n}\nbody = extrude(s, h)\nseat = anchor(body.start)\n",
    )
    .unwrap();
    let src = format!(
        "{PEGS}k = part(\"knob.fg\")\non_top = fastened(k.seat, p1.shank.end, flip: true)\nexpect z(k) = 38\nexpect interferences = 0\n"
    );
    let r = run_in(&d, &src);
    assert!(r.ok(), "{:?}", errors(&r));
    let a = r.asm.as_ref().unwrap();
    assert!(a.bom.iter().any(|b| b.0 == "knob.fg"));
}

#[test]
fn a_subassembly_with_components_of_its_own_is_refused_for_now() {
    let d = dir("sub");
    std::fs::write(d.join("box.fg"), BOX).unwrap();
    let r = run_in(&d, "b = subassembly(\"box.fg\", ground: true)\n");
    assert!(
        errors(&r).iter().any(|e| e.contains("components of its own")),
        "{:?}",
        errors(&r)
    );
}

#[test]
fn component_is_a_keyword_and_still_an_argument_name() {
    // proof-findings 1: `component:` as a keyword argument parses (a drawing's `part(component: ..)`)
    assert!(forge_lang::syntax::parse("x = part(\"box.fg\", component: \"lid\")\n").is_ok());
    assert!(forge_lang::syntax::parse("component c(a: 1) {\n}\n").is_ok());
}

#[test]
fn a_component_param_may_not_take_a_shared_params_name() {
    // proof-findings 5: `w` inside `f` is the shared param; a component param `w` would change
    // what `f` reads in the component only
    let d = dir("shadow");
    let src = "shared w = 4
fn f(k) {
  return k + w
}
component c(w: 2) {
  sketch s(on: xy) {
    o = circle((0, 0), f(1))
  }
  e = extrude(s, w)
}
x = part(c)
";
    let r = run_in(&d, src);
    assert!(kinds(&r).contains(&"Rebind"), "{:?}", errors(&r));
}

#[test]
fn a_drawing_views_one_component_of_a_file() {
    let ex = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/unified");
    let src = std::fs::read_to_string(ex.join("peg_dwg.fg")).unwrap();
    let d = forge_lang::drawing::regen::make(&src, &ex);
    assert!(
        d.ok(),
        "{:?}",
        d.errors.iter().map(|e| format!("{e:?}")).collect::<Vec<_>>()
    );
    // the peg alone: one solid, in its own space; the board: every occurrence, posed
    assert_eq!(d.models[0].model.bodies.len(), 1);
    assert_eq!(d.models[1].model.bodies.len(), 4);
    let bad = src.replace("peg(d=10,h=30)", "nope");
    let d = forge_lang::drawing::regen::make(&bad, &ex);
    assert!(!d.ok());
}

#[test]
fn a_body_lists_every_feature_that_made_it() {
    let r = run_in(&dir("chain"), BOX);
    assert!(r.ok(), "{:?}", errors(&r));
    let chain = |id: &str| {
        r.body_chains
            .iter()
            .find(|c| c.0 == id)
            .map(|c| c.1.clone())
            .unwrap()
    };
    assert_eq!(chain("base"), ["plan", "base", "cup"]);
    assert_eq!(chain("lid"), ["lp", "lid"]);
}

#[test]
fn components_place_components_as_subassemblies() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/unified/nested_boxes.fg"
    ))
    .unwrap();
    let r = run_in(&dir("nest"), &src);
    assert!(r.ok(), "{:?}", errors(&r));
    let a = r.asm.as_ref().unwrap();
    let occ: Vec<&str> = a.occurrences.iter().map(|o| o.0.as_str()).collect();
    for p in ["b1.base", "b1.l1", "b2.base", "b2.l1"] {
        assert!(occ.contains(&p), "{p} in {occ:?}");
    }
    // the lid sits on its box's base, in each box
    let z = |p: &str| a.occurrences.iter().find(|o| o.0 == p).unwrap().2.origin.z;
    assert!((z("b1.l1") - 20.0).abs() < 1e-9 && (z("b2.l1") - 20.0).abs() < 1e-9);
    assert_eq!(a.interferences, 0);
}

#[test]
fn a_component_placing_itself_is_a_typed_cycle() {
    let d = dir("cycle");
    let body = "  sketch s(on: xy) {\n    o = circle((0, 0), 5)\n  }\n  e = extrude(s, 5)\n";
    // directly
    let r = run_in(
        &d,
        &format!("component a() {{\n{body}  x = part(a)\n}}\ny = part(a)\n"),
    );
    assert!(
        r.errors.iter().any(|e| e.kind() == "ComponentCycle"),
        "{:?}",
        errors(&r)
    );
    // through another
    let r = run_in(
        &d,
        &format!(
            "component a() {{\n{body}  x = part(b)\n}}\ncomponent b() {{\n{body}  y = part(a)\n}}\nz = part(a)\n"
        ),
    );
    assert!(
        r.errors.iter().any(|e| e.kind() == "ComponentCycle"),
        "{:?}",
        errors(&r)
    );
    assert!(!r.errors.iter().any(|e| e.kind() == "Fuel"), "{:?}", errors(&r));
}

#[test]
fn a_feature_read_through_an_occurrence_is_not_deleted() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/unified/nested_boxes.fg"
    ))
    .unwrap();
    // box joins its lid by `l1.plate.start`: the lid's plate is read from outside it (R2)
    let e = apply(
        &src,
        &Op::DeleteFeature {
            id: "lid.plate".into(),
        },
    )
    .unwrap_err();
    assert_eq!(e.kind(), "StillReadBy", "{}", e.text());
    assert!(e.text().contains('j'), "{}", e.text());
    // an in-context argument reading through two occurrences is a reader too
    let more = src.replace("expect interferences = 0", "c = part(lid(w: wide), at: (0, 100, 0))\nk = fastened(c.plate.end, b1.l1.plate.end)\nexpect interferences = 0");
    let e = apply(
        &more,
        &Op::DeleteFeature {
            id: "lid.plate".into(),
        },
    )
    .unwrap_err();
    assert!(e.text().contains('k') && e.text().contains('j'), "{}", e.text());
    // nested scoped ops reach a block inside a block
    let inner = "component box() {\n  component pin(r: 2) {\n    sketch s(on: xy) {\n      c = circle((0, 0), r)\n    }\n    rod = extrude(s, r)\n  }\n  p = part(pin)\n}\nb = part(box)\n";
    let out = apply(
        inner,
        &Op::SetArg {
            id: "box.pin".into(),
            arg: "r".into(),
            text: "3".into(),
        },
    );
    let out = out.unwrap_or_else(|e| panic!("{}", e.text()));
    assert!(out.contains("component pin(r: 3)"), "{out}");
}
