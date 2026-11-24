//! The edit test on the bracket: change a size, move a hole, insert a feature above others, move
//! and delete one, split one loop instance off, and edit a shared param. Every edit is a typed
//! op; after each the part builds and every feature the edit did not add or delete keeps its id.

use forge_lang::edit::apply;
use forge_lang::error::{Literal, Op, ParamKey, parse_path};
use forge_lang::values::Dim;
use forge_lang::{Options, Run};
use std::collections::BTreeSet;

fn bracket() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/parts/bracket.fg")).expect("part")
}

fn build(src: &str) -> Run {
    let opts = Options {
        volumes: true,
        ..Options::default()
    };
    let r = forge_lang::run(src, opts, Default::default()).0;
    // an edit that changes the volume fails the part's expect line, as it should: that typed
    // error is the only one allowed here
    assert!(
        r.errors.iter().all(|e| e.kind() == "ExpectFailed"),
        "{}\n{}",
        forge_lang::report::text(&r, src),
        src
    );
    assert!(r.measure.is_some());
    r
}

fn ids(r: &Run) -> Vec<String> {
    r.feats.iter().map(|f| f.id.clone()).collect()
}

fn set(param: &str, value: f64) -> Op {
    Op::SetParam {
        param: ParamKey::parse(param).expect("a param"),
        value: Literal {
            value,
            dim: Dim::Length,
        },
    }
}

/// Every byte outside one span (the edit's) is the same.
fn local(a: &str, b: &str) -> (usize, usize) {
    let pre = a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count();
    let suf = a
        .bytes()
        .rev()
        .zip(b.bytes().rev())
        .take_while(|(x, y)| x == y)
        .count();
    (pre, suf.min(a.len() - pre).min(b.len() - pre))
}

/// The features whose own definition changed: the kernel feature they became (a sketch's
/// includes its plane). The part after them changes for everything downstream; that is not this.
fn moved(a: &Run, b: &Run) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for f in &a.feats {
        let Some(g) = b.feats.iter().find(|g| g.id == f.id) else {
            continue;
        };
        if f.fingerprint != g.fingerprint {
            out.insert(f.id.clone());
        }
    }
    out
}

#[test]
fn edits_keep_ids_and_build() {
    let src = bracket();
    let r0 = build(&src);
    let ids0 = ids(&r0);
    let vol0 = r0.measure.as_ref().expect("built").volume;

    // a size: the lug centre's height
    let s1 = apply(&src, &set("websk.H", 100.0)).expect("applies");
    let (pre, suf) = local(&src, &s1);
    assert_eq!(&src[pre..src.len() - suf], "95");
    let r1 = build(&s1);
    assert_eq!(ids(&r1), ids0);
    assert!(r1.measure.as_ref().expect("built").volume > vol0);

    // move the bolt holes in
    let s2 = apply(&src, &set("boltsk.x", 50.0)).expect("applies");
    let r2 = build(&s2);
    assert_eq!(ids(&r2), ids0);
    assert_eq!(
        moved(&r0, &r2).into_iter().next().as_deref(),
        Some("bolts"),
        "only the holes and what follows them change"
    );

    // a feature inserted above others: a drilled dimple in the cross rib, above the bolts
    let add = Op::AddFeature {
        before: Some("boltsk".into()),
        text: "param dimple_sk.x = 25\nparam dimple_sk.y = 35\nsketch dimple_sk(on: base.end) {\n  p = pt(x, y)\n}\nparam dimple.d = 3\ndimple = hole(at: dimple_sk.p, d: d, depth: thru)".into(),
    };
    let s3 = apply(&src, &add).expect("applies");
    let r3 = build(&s3);
    let ids3: Vec<String> = ids(&r3)
        .into_iter()
        .filter(|i| !i.starts_with("dimple"))
        .collect();
    assert_eq!(ids3, ids0);
    assert!(ids(&r3).contains(&"dimple".to_string()));

    // moved and deleted: the last fillet above the chamfer, then out
    let s4 = apply(
        &src,
        &Op::MoveFeature {
            id: "hub_edge".into(),
            before: Some("bore_break".into()),
        },
    )
    .expect("applies");
    let r4 = build(&s4);
    let mut a = ids(&r4);
    let mut b = ids0.clone();
    a.sort();
    b.sort();
    assert_eq!(a, b);
    let s5 = apply(
        &src,
        &Op::DeleteFeature {
            id: "hub_edge".into(),
        },
    )
    .expect("applies");
    let r5 = build(&s5);
    assert_eq!(
        ids(&r5),
        ids0.iter()
            .filter(|i| *i != "hub_edge")
            .cloned()
            .collect::<Vec<_>>()
    );
    // the web is read below it (the pocket sits on its end): deleting it is refused, naming
    // the first reader
    let e = apply(&src, &Op::DeleteFeature { id: "web".into() }).unwrap_err();
    assert!(
        matches!(&e, forge_lang::error::Error::StillRead { name, .. } if name == "web"),
        "{e:?}"
    );
    // an instance is no statement: deleting one is a typed error
    assert_eq!(
        apply(
            &src,
            &Op::DeleteFeature {
                id: "bpocket[1,1]".into()
            }
        )
        .unwrap_err()
        .kind(),
        "NotAStatement"
    );

    // one pocket of four made narrower: split its instance off, then set its own param
    let split = Op::SplitInstance {
        id: parse_path("bpsk[1,1]").expect("path"),
        param: ParamKey::parse("bpsk.w").expect("param"),
    };
    let s6 = apply(&src, &split).expect("applies");
    assert_eq!(
        build(&s6).measure.as_ref().expect("built").volume,
        vol0,
        "a split changes no number"
    );
    let s7 = apply(&s6, &set("bpsk[1,1].w", 30.0)).expect("applies");
    let r7 = build(&s7);
    assert_eq!(ids(&r7), ids0);
    let m = moved(&r0, &r7);
    assert!(
        m.contains("bpsk[1,1]") && !m.contains("bpsk[0,0]") && !m.contains("bpsk[1,0]"),
        "{m:?}"
    );

    // a value with the wrong dimension is refused, typed
    let bad = Op::SetParam {
        param: ParamKey::parse("websk.H").expect("param"),
        value: Literal {
            value: 30.0,
            dim: Dim::Angle,
        },
    };
    assert_eq!(apply(&src, &bad).unwrap_err().kind(), "EditDim");
    assert_eq!(
        apply(&src, &set("websk.nope", 1.0)).unwrap_err().kind(),
        "UnknownParam"
    );
}

/// A shared param edit: what it drives by provenance, and what else moves through anchors.
#[test]
fn a_shared_param_moves_what_reads_it_and_what_sits_on_it() {
    let src = bracket();
    let r0 = build(&src);
    let s = apply(&src, &set("tw", 12.0)).expect("applies");
    let r1 = build(&s);
    assert_eq!(ids(&r1), ids(&r0));
    let tw = ParamKey::parse("tw").expect("param");
    // read directly, or through a param linked to it
    let linked: BTreeSet<ParamKey> = r0
        .shared
        .iter()
        .chain(&r0.locals)
        .filter(|p| p.links.contains(&tw))
        .map(|p| p.key.clone())
        .chain([tw.clone()])
        .collect();
    let reads: BTreeSet<String> = r0
        .feats
        .iter()
        .filter(|f| f.reads.iter().any(|p| linked.contains(&**p)))
        .map(|f| f.id.clone())
        .collect();
    let moved = moved(&r0, &r1);
    // what changed without reading tw: placed on an anchor of a reader, or selecting edges a
    // reader made
    let through_anchors: Vec<&String> = moved.iter().filter(|m| !reads.contains(*m)).collect();
    eprintln!("tw is read by {reads:?}; also moved: {through_anchors:?}");
    assert!(reads.contains("web") && reads.contains("hubsk") && reads.contains("boresk"));
    // the web pocket sits on the web's end face: it moves with tw though it never reads it
    assert!(moved.contains("wpsk") && !reads.contains("wpsk"));
}

#[test]
fn set_arg_names_a_positional_argument_by_its_place() {
    let src = "shared t = 2\nshared k = ratio(0.4)\nparam plan.L = 40\nparam plan.W = 30\nparam up.h = 20\nsketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (L, W))\n}\nbase = sheet(plan, t: t, r: t, bend: kfactor(k))\nup = flange(base.edge(plan.outline[1]), height: h)\n";
    let op = Op::SetArg {
        id: "up".into(),
        arg: "#0".into(),
        text: "base.edge(plan.outline[2])".into(),
    };
    let out = apply(src, &op).unwrap();
    assert!(
        out.contains("up = flange(base.edge(plan.outline[2]), height: h)"),
        "{out}"
    );
    assert!(build(&out).ok());
    // no second positional argument: refused, typed
    let op = Op::SetArg {
        id: "up".into(),
        arg: "#1".into(),
        text: "3".into(),
    };
    assert_eq!(apply(src, &op).unwrap_err().kind(), "UnknownArg");
}

/// In a machining document a call names a built-in, never a binding (forge-cam-lang's and
/// forge-turn-lang's evaluators), so `drill = drill(..)` is legal there; Rename takes the
/// binding and its reads, never the call's head (it once wrote `holes = holes(..)`).
#[test]
fn rename_in_a_machining_document_leaves_call_heads() {
    use forge_lang::syntax::{ExprKind, StmtKind};
    let src = "machining\nf = part(\"flange.fg\")\nop1 = setup(f)\ndrill = drill(op1, f.bolts)\nprog = program(op1, after: drill)\n";
    let op = Op::Rename {
        from: "drill".into(),
        to: "holes".into(),
    };
    let out = forge_lang::edit::apply(src, &op).unwrap();
    assert_eq!(
        out,
        "machining\nf = part(\"flange.fg\")\nop1 = setup(f)\nholes = drill(op1, f.bolts)\nprog = program(op1, after: holes)\n"
    );
    let prog = forge_lang::edit::apply_ast(src, &op).unwrap();
    let StmtKind::Assign { name, value } = &prog[3].kind else {
        panic!("an assignment")
    };
    assert_eq!(name, "holes");
    let ExprKind::Call(f, _) = &value.kind else {
        panic!("a call")
    };
    assert!(matches!(&f.kind, ExprKind::Name(n) if n == "drill"));
    // a part's calls are reads: a function's name is renamed where it is called
    let part = "fn sq(x) {\n  return x * x\n}\na = sq(2)\n";
    let out = forge_lang::edit::apply(
        part,
        &Op::Rename {
            from: "sq".into(),
            to: "sq2".into(),
        },
    )
    .unwrap();
    assert_eq!(out, "fn sq2(x) {\n  return x * x\n}\na = sq2(2)\n");
}

/// In a machining document a call's head is not a read: deleting, moving or suppressing
/// `drill = drill(..)` is not held up by, and does not take out, a later statement that only
/// calls the built-in `drill` (it once refused with StillRead, and Suppress took `d2` too).
#[test]
fn a_call_head_in_a_machining_document_is_not_a_read() {
    let src = "machining\nf = part(\"flange.fg\")\nop1 = setup(f)\ndrill = drill(op1, f.bolts)\nd2 = drill(op1, f.other)\nprog = program(op1, after: drill)\n";
    let del = apply(
        "machining\nf = part(\"flange.fg\")\nop1 = setup(f)\ndrill = drill(op1, f.bolts)\nd2 = drill(op1, f.other)\n",
        &Op::DeleteFeature { id: "drill".into() },
    )
    .unwrap();
    assert_eq!(
        del,
        "machining\nf = part(\"flange.fg\")\nop1 = setup(f)\nd2 = drill(op1, f.other)\n"
    );
    // a real read still holds it: `prog` reads `drill`
    assert!(apply(src, &Op::DeleteFeature { id: "drill".into() }).is_err());
    let sup = apply(src, &Op::Suppress { id: "drill".into() }).unwrap();
    assert!(sup.contains("\nd2 = drill(op1, f.other)\n"), "{sup}");
    assert!(sup.contains("#~drill drill = drill(op1, f.bolts)"), "{sup}");
    assert!(sup.contains("#~drill prog = program(op1, after: drill)"), "{sup}");
    let mv = apply(
        src,
        &Op::MoveFeature {
            id: "d2".into(),
            before: Some("drill".into()),
        },
    )
    .unwrap();
    assert!(
        mv.contains("d2 = drill(op1, f.other)\ndrill = drill(op1, f.bolts)\n"),
        "{mv}"
    );
    let add = apply(
        src,
        &Op::AddFeature {
            before: Some("drill".into()),
            text: "d3 = drill(op1, f.third)".into(),
        },
    )
    .unwrap();
    assert!(
        add.contains("d3 = drill(op1, f.third)\ndrill = drill(op1, f.bolts)\n"),
        "{add}"
    );
    // a part's call of a function is a read of it
    let part = "fn sq(x) {\n  return x * x\n}\na = sq(2)\n";
    assert!(apply(part, &Op::DeleteFeature { id: "sq".into() }).is_err());
}

/// A robot document's calls are built-ins too (robot.rs): `frame = frame(..)` renamed keeps the
/// call, and deleting it is not held up by a later call of `frame`.
#[test]
fn a_robot_documents_calls_are_built_ins() {
    let src = "robot\nsrc = source(\"arm.fg\")\nframe = frame(on: src.base)\ntool0 = frame(on: src.tip)\n";
    let out = apply(
        src,
        &Op::Rename {
            from: "frame".into(),
            to: "base0".into(),
        },
    )
    .unwrap();
    assert_eq!(
        out,
        "robot\nsrc = source(\"arm.fg\")\nbase0 = frame(on: src.base)\ntool0 = frame(on: src.tip)\n"
    );
    let del = apply(src, &Op::DeleteFeature { id: "frame".into() }).unwrap();
    assert_eq!(
        del,
        "robot\nsrc = source(\"arm.fg\")\ntool0 = frame(on: src.tip)\n"
    );
}
