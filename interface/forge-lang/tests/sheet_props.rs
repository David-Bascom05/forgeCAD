//! Property tests over the sheet features: random sheet parts (a base, flanges off its edges, a
//! lip off a flange's tip, a range loop of cutouts sketched on the base, an unfold) and random
//! typed edits. As for ordinary parts: the edited text parses to the tree the op makes, SetParam
//! changes one span, ids of features the op did not add or delete are unchanged, and a document
//! that checked still checks (the one order rule of sheets aside: nothing after an unfold).
//! Built: every part builds valid, its folded and flat volumes differ by the closed form, and a
//! SetParam renames no face.

use forge_lang::edit::{apply, apply_ast, strip};
use forge_lang::error::{Literal, Op, ParamKey, parse_path};
use forge_lang::sheet::SheetErr;
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
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        let u = (self.next() >> 11) as f64 / (1u64 << 53) as f64;
        // to a tenth, as people write them
        ((lo + (hi - lo) * u) * 10.0).round() / 10.0
    }
}

/// A sheet part. Sizes keep every flange clear of its setback and the cutouts on the base, clear
/// of its edges and bends.
fn document(rng: &mut Rng) -> (String, Vec<(f64, f64)>, f64, f64) {
    let t = rng.range(0.8, 3.0);
    let r = rng.range(0.5, 3.0);
    let k = rng.range(0.25, 0.5);
    let (l, w) = (rng.range(60.0, 120.0), rng.range(40.0, 80.0));
    let mut s = format!(
        "shared t = {t}\nshared r = {r}\nshared k = ratio({k})\nshared L = {l}\nshared W = {w}\n\
         sketch plan(on: xy) {{\n  outline = rect(corner: (0, 0), size: (L, W))\n}}\n\
         base = sheet(plan, t: t, r: r, bend: kfactor(k))\n"
    );
    // flanges on the right and left edges (1, 3): no corners to meet
    let mut bends = vec![];
    for (e, name) in [(1, "east"), (3, "west")] {
        if rng.below(3) == 0 {
            continue;
        }
        let h = rng.range(15.0, 40.0);
        let a = rng.range(30.0, 90.0);
        let side = if rng.below(2) == 0 { "up" } else { "down" };
        s += &format!(
            "param {name}.h = {h}\nparam {name}.a = {a}deg\n{name} = flange(base.edge(plan.outline[{e}]), height: h, angle: a, side: \"{side}\")\n"
        );
        bends.push((a.to_radians(), w));
        if name == "east" && rng.below(2) == 0 {
            let lh = rng.range(10.0, 25.0);
            s += &format!("param lip.h = {lh}\nlip = flange(east.tip, height: h, side: \"down\")\n");
            bends.push((std::f64::consts::FRAC_PI_2, w));
        }
    }
    if rng.below(2) == 0 {
        let m = 1 + rng.below(3);
        let hr = rng.range(1.0, 4.0);
        s += &format!(
            "param hk.hr = {hr}\nparam hk.dx = {}\nfor i in 0..{m} {{\n  sketch hk(on: base.top) {{\n    c = circle((L / 2 + dx * i - dx, W / 2), hr)\n  }}\n  vent = cutout(hk.c, wall: base)\n}}\n",
            rng.range(10.0, 15.0)
        );
    }
    s += &format!(
        "param blank.gap = {}\nblank = unfold(base, on: offset(base.bottom, gap))\n",
        rng.range(50.0, 90.0)
    );
    (s, bends, t, k)
}

fn run(src: &str, build: bool) -> Run {
    let opts = Options {
        build,
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
    let length = |rng: &mut Rng| Literal {
        value: rng.range(15.0, 40.0),
        dim: Dim::Length,
    };
    match rng.below(7) {
        0 | 1 => {
            // a length param to a length
            let lengths: Vec<&ParamKey> = ps
                .iter()
                .filter(|p| {
                    r.shared.iter().chain(&r.locals).any(|q| {
                        &q.key == *p
                            && matches!(&q.value, forge_lang::eval::Value::Num(n) if n.dim == Dim::Length)
                    })
                })
                .collect();
            Op::SetParam {
                param: lengths[rng.below(lengths.len())].clone(),
                value: length(rng),
            }
        }
        2 => Op::AddLocalParam {
            feature: parse_path(&pick(rng, &st)).expect("a path"),
            name: format!("x{k}"),
            value: length(rng),
        },
        3 => match ps.iter().filter(|p| !p.is_shared()).collect::<Vec<_>>() {
            locals if !locals.is_empty() && rng.below(2) == 0 => Op::MakeShared {
                param: locals[rng.below(locals.len())].clone(),
            },
            _ if ps.iter().any(|p| p.to_string() == "hk.hr") => Op::SplitInstance {
                id: parse_path(&format!("hk[{}]", rng.below(2))).expect("a path"),
                param: ParamKey::parse("hk.hr").expect("a param"),
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
                "param zk{k}.c = {}\nsketch zk{k}(on: base.top) {{\n  o = circle((c, c), t)\n}}\nz{k} = cutout(zk{k}, wall: base)",
                rng.range(6.0, 12.0)
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
fn sheet_edits_keep_text_tree_and_ids() {
    let mut rng = Rng(0x5eed_5ee7_0000_0001);
    let (mut applied, mut refused, mut ordered, mut geometric) = (0, 0, 0, 0);
    for case in 0..300 {
        let (mut src, ..) = document(&mut rng);
        for step in 0..4 {
            let r0 = run(&src, false);
            assert!(
                r0.errors.is_empty(),
                "case {case}: must check\n{src}\n{:?}",
                r0.errors
            );
            let o = op(&mut rng, &src, &r0, case * 10 + step);
            let (text, tree) = match (apply(&src, &o), apply_ast(&src, &o)) {
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
            let r1 = run(&text, false);
            if r1.errors.iter().any(|e| e.kind() == "AfterUnfold") {
                // a sheet feature moved or added after the unfold: the one order rule
                assert!(
                    matches!(o, Op::MoveFeature { .. } | Op::AddFeature { .. }),
                    "case {case}: {o:?}"
                );
                assert!(
                    r1.errors.iter().all(|e| e.kind() == "AfterUnfold"),
                    "case {case}: {:?}",
                    r1.errors
                );
                ordered += 1;
                break;
            }
            let kernel = |e: &forge_lang::error::Error| matches!(e, forge_lang::error::Error::Sheet(b) if matches!(**b, SheetErr::Kernel { .. }));
            if matches!(o, Op::SetParam { .. }) && !r1.errors.is_empty() && r1.errors.iter().all(kernel) {
                // a size the sheet cannot take: a typed geometric refusal, outside the language's
                // guarantees (a flange shorter than its setback, say)
                geometric += 1;
                break;
            }
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
        applied > 400 && refused > 20 && ordered > 0 && geometric < applied / 10,
        "applied {applied}, refused {refused}, ordered {ordered}, geometric {geometric}"
    );
    println!("applied {applied}, refused {refused}, ordered {ordered}, geometric {geometric}");
}

#[test]
fn sheet_parts_build_and_hold_their_names() {
    let mut rng = Rng(0x5eed_5ee7_0000_0002);
    let mut built = 0;
    for case in 0..120 {
        let (src, bends, t, k) = document(&mut rng);
        let r = run(&src, true);
        assert!(
            r.ok(),
            "case {case}\n{src}\n{}",
            forge_lang::report::text(&r, &src)
        );
        let m = r.measure.as_ref().expect("measured");
        assert_eq!((m.problems, m.bodies), (0, 2), "case {case}");
        let sh = &r.sheets[0];
        let fold = forge_topo::mass::mass(sh.folded.as_ref().expect("built")).expect("mass");
        let flat = forge_topo::mass::mass(sh.unfolds[0].1.as_ref().expect("unfolded")).expect("mass");
        // cutouts stay on the base, clear of the bends: the identity holds as for a bare part
        let want: f64 = bends.iter().map(|(a, len)| a * len * t * t * (0.5 - k)).sum();
        assert!(
            ((fold.volume - flat.volume) - want).abs() <= fold.err[0] + flat.err[0] + 1e-8,
            "case {case}: {} against {want}",
            fold.volume - flat.volume
        );
        // a length edited: every face keeps its name
        let r0 = run(&src, false);
        let lengths: Vec<ParamKey> = r0
            .locals
            .iter()
            .filter(|p| p.key.name == "h")
            .map(|p| p.key.clone())
            .collect();
        if let Some(p) = lengths.first() {
            let edited = apply(
                &src,
                &Op::SetParam {
                    param: p.clone(),
                    value: Literal {
                        value: rng.range(18.0, 38.0),
                        dim: Dim::Length,
                    },
                },
            )
            .expect("applies");
            let r1 = run(&edited, true);
            assert!(r1.ok(), "case {case}");
            let names = |r: &Run| {
                let b = r.sheets[0].folded.as_ref().expect("built");
                let mut v: Vec<_> = b.face_ids().map(|f| b.face(f).name).collect();
                v.sort();
                v
            };
            assert_eq!(names(&r), names(&r1), "case {case}");
        }
        built += 1;
    }
    assert_eq!(built, 120);
}
