//! Property tests: random documents and random typed edits. For every op, the edited text parses
//! to the same tree as the op applied to the tree; SetParam changes one span; a document that
//! evaluated still evaluates after any op that applied; and ids of features the op did not add
//! or delete are unchanged.

use forge_lang::edit::{apply, apply_ast, strip};
use forge_lang::error::{Literal, Op, ParamKey, parse_path};
use forge_lang::values::Dim;
use forge_lang::{Options, Run, syntax};
use std::collections::BTreeSet;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn len(&mut self) -> f64 {
        (5 + self.below(60)) as f64 / 2.0
    }
}

/// A document: shared sizes, a stack of extrudes each on the one before, some with their own
/// params, and a range loop of holes with a param of their own.
fn document(rng: &mut Rng) -> String {
    let mut s = String::new();
    for i in 0..3 {
        s += &format!("shared s{i} = {}\n", rng.len());
    }
    s += "sketch k0(on: xy) {\n  o = rect(corner: (0, 0), size: (s0, s1))\n}\nf0 = extrude(k0, s2)\n";
    let n = 2 + rng.below(4);
    for j in 1..n {
        if rng.below(2) == 0 {
            s += &format!("param k{j}.w = {}\n", rng.len());
            s += &format!(
                "sketch k{j}(on: f{}.end) {{\n  o = rect(corner: (0, 0), size: (w, s1))\n}}\n",
                j - 1
            );
        } else {
            s += &format!(
                "sketch k{j}(on: f{}.end) {{\n  o = rect(corner: (0, 0), size: (s0, s1))\n}}\n",
                j - 1
            );
        }
        if rng.below(2) == 0 {
            s += &format!("param f{j}.d = {}\nf{j} = extrude(k{j}, d)\n", rng.len());
        } else {
            s += &format!("f{j} = extrude(k{j}, s2)\n");
        }
    }
    // shape features (sweep, coil, loft, ball, shell), a few at random
    if rng.below(2) == 0 {
        s += "sketch lk(on: f0.start) {\n  c = circle((s0, s1), s2)\n}\n";
        s += "sketch lk2(on: offset(lk.plane, s2)) {\n  q = rect(center: (s0, s1), size: (s2, s2))\n}\n";
        s += "lo = loft([lk.c, lk2.q], smooth: true)\n";
    }
    if rng.below(2) == 0 {
        s += &format!(
            "param co.pp = {}\nsketch ck(on: rotate(k0.plane, about: k0.o[0], by: 90deg)) {{\n  c = circle((s0, s1), s2)\n  ax = line((0, 0), (0, s1), construction: true)\n}}\nco = coil(ck.c, axis: ck.ax, pitch: pp, turns: count(3))\n",
            rng.len()
        );
    }
    if rng.below(2) == 0 {
        s += "sketch sk(on: rotate(k0.plane, about: k0.o[0], by: 90deg)) {\n  path rail(start: (0, 0)) {\n    line(to: (0, s1))\n    tarc(to: (s2, s1 + s2))\n  }\n}\n";
        s += "sketch sp(on: k0.plane) {\n  c = circle((0, 0), s0)\n}\nsw = sweep(sp.c, along: sk.rail, extend: (s0, s1))\n";
    }
    if rng.below(2) == 0 {
        s += &format!(
            "param bb.r = {}\nfor i in 0..2 {{\n  sketch bk(on: f0.end) {{\n    p = pt(s0 * i, s1)\n  }}\n  bb = ball(bk.p, r)\n}}\n",
            rng.len()
        );
    }
    if rng.below(3) == 0 {
        s += "sh = shell(s2, open: f0.start, out: true)\n";
    }
    if rng.below(2) == 0 {
        let m = 2 + rng.below(3);
        s += &format!(
            "param g.dd = {}\nfor i in 0..{m} {{\n  sketch gk(on: f0.end) {{\n    p = pt(s0 * i, s1)\n  }}\n  g = hole(at: gk.p, d: dd, depth: thru)\n}}\n",
            rng.len()
        );
    }
    // the wired features: a pattern, a work plane, a threaded hole and its thread, a body of its
    // own joined in
    if rng.below(2) == 0 {
        s += "param pat.n = count(2)\npat = pattern(of: f0, layout: lin(along: k0.o[0].dir, n: n, step: s0 + s0))\n";
    }
    if rng.below(2) == 0 {
        s += &format!(
            "param wp.d = {}\nwp = workplane(offset: f0.end, d: d)\n",
            rng.len()
        );
    }
    if rng.below(2) == 0 {
        s += &format!(
            "sketch tk(on: f0.end) {{\n  p = pt(s0 / 2, s1 / 2)\n}}\nparam th.d = {}\nth = hole(at: tk.p, d: d, depth: s2, thread: \"M8\")\ntt = thread(hole: th, length: full)\n",
            rng.len()
        );
    }
    if rng.below(2) == 0 {
        s += "sketch nk(on: f0.start) {\n  o = rect(corner: (0, 0), size: (s1, s1))\n}\nnb = extrude(nk, s2, new: true)\nj = union(f0, tools: nb)\n";
    }
    s
}

fn check(src: &str) -> Run {
    let opts = Options {
        build: false,
        ..Options::default()
    };
    forge_lang::run(src, opts, Default::default()).0
}

fn ids(r: &Run) -> Vec<String> {
    r.feats.iter().map(|f| f.id.clone()).collect()
}

fn params(r: &Run) -> Vec<ParamKey> {
    r.shared.iter().chain(&r.locals).map(|p| p.key.clone()).collect()
}

/// Top-level feature statements' names.
fn statements(src: &str) -> Vec<String> {
    syntax::parse(src)
        .expect("parses")
        .iter()
        .filter_map(|s| match &s.kind {
            syntax::StmtKind::Assign { name, .. } | syntax::StmtKind::Block { name, .. } => {
                Some(name.clone())
            }
            _ => None,
        })
        .collect()
}

fn op(rng: &mut Rng, src: &str, r: &Run, k: usize) -> Op {
    let ps = params(r);
    let st = statements(src);
    let pick = |rng: &mut Rng, v: &[String]| v[rng.below(v.len())].clone();
    let lit = |v: f64| Literal {
        value: v,
        dim: Dim::Length,
    };
    match rng.below(7) {
        0 | 1 => Op::SetParam {
            param: ps[rng.below(ps.len())].clone(),
            value: lit(rng.len()),
        },
        2 => Op::AddLocalParam {
            feature: parse_path(&pick(rng, &st)).expect("a path"),
            name: format!("x{k}"),
            value: lit(rng.len()),
        },
        3 => match ps.iter().filter(|p| !p.is_shared()).collect::<Vec<_>>() {
            locals if !locals.is_empty() && rng.below(2) == 0 => Op::MakeShared {
                param: locals[rng.below(locals.len())].clone(),
            },
            _ if ps.iter().any(|p| p.to_string() == "bb.r") && rng.below(2) == 0 => Op::SplitInstance {
                id: parse_path(&format!("bb[{}]", rng.below(2))).expect("a path"),
                param: ParamKey::parse("bb.r").expect("a param"),
            },
            _ if ps.iter().any(|p| p.to_string() == "g.dd") => Op::SplitInstance {
                id: parse_path(&format!("g[{}]", rng.below(2))).expect("a path"),
                param: ParamKey::parse("g.dd").expect("a param"),
            },
            _ => Op::DeleteFeature { id: pick(rng, &st) },
        },
        4 => Op::AddFeature {
            before: if rng.below(3) == 0 {
                None
            } else {
                Some(pick(rng, &st))
            },
            text: format!(
                "param zk{k}.w = {}\nsketch zk{k}(on: f0.start) {{\n  o = rect(corner: (0, 0), size: (w, s1))\n}}\nz{k} = extrude(zk{k}, s2)",
                rng.len()
            ),
        },
        5 => Op::DeleteFeature { id: pick(rng, &st) },
        _ => Op::MoveFeature {
            id: pick(rng, &st),
            before: if rng.below(3) == 0 {
                None
            } else {
                Some(pick(rng, &st))
            },
        },
    }
}

#[test]
fn text_and_tree_edits_agree_and_ids_hold() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let (mut applied, mut refused) = (0, 0);
    for case in 0..400 {
        let mut src = document(&mut rng);
        // a chain of edits on one document
        for step in 0..4 {
            let r0 = check(&src);
            assert!(
                r0.errors.is_empty(),
                "case {case}: the document must check\n{src}\n{:?}",
                r0.errors
            );
            let o = op(&mut rng, &src, &r0, case * 10 + step);
            let text = apply(&src, &o);
            let tree = apply_ast(&src, &o);
            let (text, tree) = match (text, tree) {
                (Ok(t), Ok(a)) => (t, a),
                (Err(x), Err(y)) => {
                    assert_eq!(x.kind(), y.kind(), "case {case}: {o:?}");
                    refused += 1;
                    break;
                }
                (t, a) => panic!(
                    "case {case}: text and tree disagree on {o:?}: {:?} vs {:?}",
                    t.err(),
                    a.err()
                ),
            };
            applied += 1;
            let reparsed = syntax::parse(&text).expect("an applied edit parses");
            assert_eq!(strip(&reparsed), strip(&tree), "case {case}: {o:?}\n{text}");
            if let Op::SetParam { param, value } = &o {
                // one span changes: the param's right side
                let pre = src.bytes().zip(text.bytes()).take_while(|(a, b)| a == b).count();
                let suf = src
                    .bytes()
                    .rev()
                    .zip(text.bytes().rev())
                    .take_while(|(a, b)| a == b)
                    .count();
                let new = &text[pre..text.len() - suf.min(text.len() - pre)];
                assert!(value.text().contains(new), "case {case}: {param} -> {new:?}");
            }
            let r1 = check(&text);
            assert!(
                r1.errors.is_empty(),
                "case {case}: {o:?} applied, but\n{text}\n{:?}",
                r1.errors
            );
            let (a, b): (BTreeSet<String>, BTreeSet<String>) =
                (ids(&r0).into_iter().collect(), ids(&r1).into_iter().collect());
            match &o {
                Op::AddFeature { .. } => assert!(a.is_subset(&b), "case {case}"),
                Op::DeleteFeature { id } => {
                    let gone: BTreeSet<String> = a.difference(&b).cloned().collect();
                    assert!(
                        gone.iter().all(|g| g == id || g.starts_with(&format!("{id}["))),
                        "case {case}: {gone:?}"
                    );
                    assert!(b.is_subset(&a));
                }
                Op::MoveFeature { .. } => assert_eq!(a, b, "case {case}"),
                _ => assert_eq!(ids(&r0), ids(&r1), "case {case}: {o:?}"),
            }
            src = text;
        }
    }
    assert!(
        applied > 500 && refused > 20,
        "applied {applied}, refused {refused}"
    );
}
