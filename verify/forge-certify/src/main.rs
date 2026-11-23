//! `cargo run -p forge-certify --release --bin certify [-- SOURCE..] [--seeds N] [--suite N]
//! [--lang N] [--frames x|xyz] [--time] [--mutate]`
//!
//! Every boolean the kernel makes while regenerating the cases of the chosen sources (the
//! forge-exact fuzz corpus: `families`, `more`, `plane_plane`, `notched` with its 400 ulp stages,
//! `suite` (forge_model::suite::fuzz, `--suite N` seeds, default FUZZ_SEEDS or 150), `lang`)
//! certified and handed to the proven checker: totals accepted / rejected / out of scope, each
//! rejection with its case and reason, the out-of-scope reasons, and with `--time` the checker's
//! time against the kernel's per boolean. `repros` (not in the default set) is every part and
//! pair of verify/forge-exact/fuzz/tests/repros.rs and the drafted lid (coverage G5) at 0.05°,
//! 0.1° and 0.2°. `--mutate`: each accepted result is also sent wrong on purpose (its first and
//! last face dropped, and claimed for each other op), and each mutant's verdict is counted: a
//! mutant accepted is a checker bug unless the mutant is right too.

use forge_bool::Op;
use forge_certify::{Checker, Mutation, Verdict, dump, dump_mutated, kernel_time};
use forge_exact_fuzz::pairs::{Boxes, Pattern, RELS, block, kernel_pair};
use forge_exact_fuzz::{Case, guarded};
use forge_model::{History, Part};
use std::collections::BTreeMap;

const FAMILIES: [&str; 6] = [
    "near_tolerance",
    "coincident_walls",
    "coplanar_stack",
    "through_vertex",
    "exact_patterns",
    "remove_all",
];

fn flat_kinds() -> u64 {
    let names = ["extrude", "pattern", "workplane", "param", "sketch"];
    forge_lang::fuzz::MAKERS
        .iter()
        .enumerate()
        .filter(|(_, m)| names.contains(&m.name))
        .fold(0u64, |k, (i, _)| k | 1 << i)
}

const CURVED: [&str; 12] = [
    "circle(", "arc(", "rounded", "revolve(", "ball(", "hole(", "fillet(", "chamfer(", "slot(", "spline(",
    "ellipse(", "coil(",
];

fn flat_src(src: &str) -> String {
    let kept: Vec<String> = forge_lang::fuzz::statements(src)
        .into_iter()
        .filter(|s| !CURVED.iter().any(|c| s.contains(c)))
        .collect();
    kept.join("\n") + "\n"
}

fn case(name: String, part: Part) -> Case {
    let part = forge_model::adversarial::canonical(&part);
    Case {
        source: String::new(),
        name,
        part,
    }
}

/// The drafted lid (verify/forge-coverage/parts/drafted_lid.fg on the coverage branch), drafted `deg`.
fn drafted_lid(deg: &str) -> String {
    format!(
        "shared W = 10\nshared H = 10\nshared T = 5\nparam bottom.a = {deg}deg\nparam ends.a = -{deg}deg\n\
         sketch lidp(on: offset(xy, H)) {{\n  r = rect(corner: (0, 0), size: (W, W))\n  m = pt(0, W / 2)\n}}\n\
         lid = extrude(lidp, T)\nhalf = plane(at: lidp.m, normal: lidp.r[1].dir, x: lidp.r[0].dir)\n\
         bottom = draft(lid.start, angle: a, neutral: half)\n\
         ends = draft(lid.side(lidp.r[0]), lid.side(lidp.r[2]), angle: a, neutral: lidp.plane)\n\
         sketch foot(on: offset(lidp.plane, 0 - H)) {{\n  s = rect(corner: (0, 0), size: (W, W))\n}}\n\
         block = extrude(foot, H)\n"
    )
}

/// The n-ary lost-face repro (repros.rs `nary_keeps_every_face_of_a_patterned_prism_cut`).
const NARY_PRISM: &str = r#"sketch S0(on: xy) {
  o = rect(corner: (0, 0), size: (74.39, 86.74))
}
L0 = extrude(S0, 34.72)
sketch H0(on: offset(xy, 17.36)) {
  p0 = pt(20.67, 0.83)
}
sketch S1(on: plane(at: H0.p0, normal: -Y)) {
  o = poly([(3.86, 1.37), (-0.78, 4.48), (-4.8, -1.37), (-0.35, -3.93)])
}
L1 = extrude(S1, next, flip: true)
sketch H2(on: offset(xy, 0)) {
  p0 = pt(0, 0)
  p1 = pt(-100, -5)
}
L3 = pattern(of: L1, layout: lin(along: axis(H2.p0, H2.p1), n: 3, step: 24.43))
sketch H4(on: offset(xy, 15.15)) {
  p0 = pt(-25.99, 84.3)
}
sketch S5(on: plane(at: H4.p0, normal: Y)) {
  o = poly([(-14.56, -9.7), near(14.56, -9.7), near(14.56, 9.7), near(-14.56, 9.7)])
  horizontal(o[0])
  vertical(o[1])
  horizontal(o[2])
  vertical(o[3])
  distance(o[1], 19.4)
  distance(o[0], 29.11)
}
L5 = cut(S5, 5.71)
"#;

fn family(name: &str, seed: u64) -> Part {
    let g = forge_model::adversarial::families()
        .into_iter()
        .find(|(f, _)| *f == name)
        .expect("the family")
        .1;
    forge_exact_fuzz::flat_part(&forge_model::adversarial::canonical(&g(seed)))
}

fn shrunk(name: &str, seed: u64, keep: &[usize]) -> Part {
    let mut p = family(name, seed);
    p.features = keep.iter().map(|&i| p.features[i].clone()).collect();
    p
}

/// The repros' box pairs: (name, A, B, op).
fn repro_pairs() -> Vec<(String, Boxes, Boxes, Op)> {
    let band = |rels: &str, op: Op, axes: &[usize], delta: f64| {
        let r: Vec<_> = rels
            .split('/')
            .map(|n| RELS.into_iter().find(|r| r.name == n).expect("a relation"))
            .collect();
        let p = Pattern {
            rel: [r[0], r[1], r[2]],
            op,
        };
        let (a, b) = p.banded(10.0, axes, delta).expect("a coincident end");
        (format!("band {rels} {axes:?} {delta}"), a, b, op)
    };
    vec![
        (
            "touching_subtract".into(),
            [[0.0, 10.0], [0.0, 20.0], [0.0, 20.0]],
            [[10.0, 20.0], [10.0, 30.0], [10.0, 30.0]],
            Op::Subtract,
        ),
        band("meets/meets/overlaps", Op::Union, &[0, 1], 0.9),
        band("meets/meets/meets", Op::Union, &[0, 1, 2], 0.9),
        band("equal/meets/equal", Op::Union, &[0], 1.5),
        band("meets/meets/overlaps", Op::Union, &[0], 1.5),
    ]
}

fn degenerate(name: &str) -> Part {
    forge_model::adversarial::degenerate_parts()
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no part {name}"))
        .1
}

fn cases(sources: &[String], seeds: u64, suite: u64, lang: u64) -> Vec<(Case, Vec<u64>)> {
    let on = |s: &str| {
        if sources.is_empty() {
            ["families", "plane_plane", "notched", "suite", "lang"].contains(&s)
        } else {
            sources.iter().any(|x| x == s)
        }
    };
    let mut out = Vec::new();
    let fams = forge_model::adversarial::families();
    if on("families") || on("more") {
        for (f, g) in &fams {
            let core = FAMILIES.contains(f);
            if (core && !on("families")) || (!core && !on("more")) {
                continue;
            }
            for s in 0..seeds {
                let Ok(raw) = guarded(|| Ok(forge_model::adversarial::canonical(&g(s)))) else {
                    continue;
                };
                out.push((
                    case(format!("{f}/{s}"), forge_exact_fuzz::flat_part(&raw)),
                    vec![],
                ));
            }
        }
    }
    if on("plane_plane") {
        for (name, part) in forge_model::adversarial::degenerate_parts() {
            if name.starts_with("plane_plane/") {
                out.push((case(name, part), vec![]));
            }
        }
    }
    if on("notched") {
        out.push((
            case("notched".into(), forge_exact_fuzz::notched()),
            (0..400).collect(),
        ));
    }
    if on("suite") {
        for s in 0..suite {
            out.push((
                case(
                    format!("suite/{s}"),
                    forge_exact_fuzz::flat_part(&forge_model::suite::fuzz(s, 8)),
                ),
                vec![],
            ));
        }
    }
    if on("repros") {
        out.push((
            case(
                "repro/corner_touch_pinch".into(),
                degenerate("plane_plane/vertex_on_vertex/Add"),
            ),
            vec![],
        ));
        out.push((
            case(
                "repro/slit_edge".into(),
                degenerate("plane_plane/tilted_edge_on_top/Cut"),
            ),
            vec![],
        ));
        out.push((
            case("repro/notched_ulp".into(), forge_exact_fuzz::notched()),
            vec![0, 13],
        ));
        out.push((
            case(
                "repro/near_tolerance_24".into(),
                shrunk("near_tolerance", 24, &[0, 1, 2]),
            ),
            vec![],
        ));
        out.push((
            case(
                "repro/near_tolerance_53".into(),
                shrunk("near_tolerance", 53, &[0, 2, 3]),
            ),
            vec![],
        ));
        out.push((
            case(
                "repro/near_tolerance_45".into(),
                shrunk("near_tolerance", 45, &[0, 1, 2, 3, 4, 5]),
            ),
            vec![],
        ));
        let mut cw = family("coincident_walls", 1);
        cw.features.truncate(11);
        out.push((case("repro/coincident_walls_1".into(), cw), vec![]));
    }
    if on("lang") {
        for s in 1..=lang {
            let Ok(run) = guarded(|| Ok(forge_lang::fuzz::run_kinds(s, 8, flat_kinds()))) else {
                continue;
            };
            let src = flat_src(&run.src);
            if let Ok(b) = guarded(|| Ok(forge_lang::fuzz::build(&src))) {
                out.push((case(format!("lang/{s}"), b.part()), vec![]));
            }
        }
    }
    out
}

/// The repros given as source, certified as the language builds them (its own regen).
fn repro_sources() -> Vec<(String, String)> {
    let mut srcs = vec![("repro/nary_lost_face".to_string(), NARY_PRISM.to_string())];
    for d in ["0.05", "0.1", "0.2"] {
        srcs.push((format!("repro/drafted_lid {d}deg"), drafted_lid(d)));
    }
    srcs
}

enum Item {
    Part(Case, Vec<u64>),
    Pair(String, Boxes, Boxes, Op),
    Lang(String, String),
}

/// The pair's one boolean.
fn pair_records(a: Boxes, b: Boxes, op: Op) -> Vec<(String, forge_bool::certify::Record)> {
    let (ba, bb) = (block(a, 1), block(b, 2));
    forge_bool::certify::record(true);
    let _ = guarded(|| {
        kernel_pair(&ba, &bb, op);
        Ok(())
    });
    forge_bool::certify::record(false);
    forge_bool::certify::take()
        .into_iter()
        .map(|r| ("pair".to_string(), r))
        .collect()
}

/// Every boolean of the case's regen (and of the last stage moved by each ulp seed).
fn records(c: &Case, ulps: &[u64]) -> Vec<(String, forge_bool::certify::Record)> {
    let mut out = Vec::new();
    forge_bool::certify::record(true);
    if let Err(e) = guarded(|| c.part.regenerate().map_err(|e| format!("{e:?}"))) {
        eprintln!("{}: the regen fails: {e}", c.name);
    }
    out.extend(
        forge_bool::certify::take()
            .into_iter()
            .map(|r| ("seq".to_string(), r)),
    );
    let h = History::new(&c.part.features, None);
    let k = c.part.features.len().saturating_sub(1);
    for &s in ulps {
        forge_bool::certify::record(false);
        let stage = guarded(|| {
            let mut body = None;
            for i in 0..k {
                body = forge_model::step(h, i, body).map_err(|e| format!("{e:?}"))?;
            }
            body.ok_or_else(|| "no stage".to_string())
        });
        let Ok(stage) = stage else { continue };
        let moved = forge_proof::props::nudged(&stage, s);
        forge_bool::certify::record(true);
        let _ = guarded(|| {
            forge_model::step(h, k, Some(moved))
                .map_err(|e| format!("{e:?}"))
                .map(|_| ())
        });
        out.extend(
            forge_bool::certify::take()
                .into_iter()
                .map(|r| (format!("ulp {s}"), r)),
        );
    }
    forge_bool::certify::record(false);
    out
}

fn main() {
    let mut sources = Vec::new();
    let mut seeds = 24;
    let mut suite: u64 = std::env::var("FUZZ_SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(150);
    let mut lang = 0;
    let mut frames = "xyz".to_string();
    let mut time = false;
    let mut mutate = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--seeds" => seeds = args.next().and_then(|s| s.parse().ok()).expect("--seeds N"),
            "--suite" => suite = args.next().and_then(|s| s.parse().ok()).expect("--suite N"),
            "--lang" => lang = args.next().and_then(|s| s.parse().ok()).expect("--lang N"),
            "--frames" => frames = args.next().expect("--frames x|xyz"),
            "--time" => time = true,
            "--mutate" => mutate = true,
            s => sources.push(s.to_string()),
        }
    }
    if lang == 0 {
        lang = seeds * 8;
    }
    let mut checker = Checker::start();
    let mut totals: BTreeMap<&str, usize> = BTreeMap::new();
    let mut rejected: Vec<(String, String)> = Vec::new();
    let mut scope: BTreeMap<String, usize> = BTreeMap::new();
    let mut ratios: Vec<(f64, f64, f64)> = Vec::new();
    let mut mutants: BTreeMap<String, usize> = BTreeMap::new();
    let mut mutants_accepted: Vec<String> = Vec::new();
    let mut items: Vec<Item> = cases(&sources, seeds, suite, lang)
        .into_iter()
        .map(|(c, u)| Item::Part(c, u))
        .collect();
    if sources.iter().any(|s| s == "repros") {
        items.extend(
            repro_pairs()
                .into_iter()
                .map(|(n, a, b, o)| Item::Pair(format!("repro/{n}"), a, b, o)),
        );
        items.extend(repro_sources().into_iter().map(|(n, s)| Item::Lang(n, s)));
    }
    for item in items {
        let t = std::time::Instant::now();
        let (name, recs) = match &item {
            Item::Part(c, ulps) => (c.name.clone(), records(c, ulps)),
            Item::Pair(n, a, b, o) => (n.clone(), pair_records(*a, *b, *o)),
            Item::Lang(n, src) => {
                forge_bool::certify::record(true);
                if let Err(e) = guarded(|| Ok(forge_lang::fuzz::build(src))) {
                    eprintln!("{n}: does not build: {e}");
                }
                forge_bool::certify::record(false);
                let recs = forge_bool::certify::take()
                    .into_iter()
                    .map(|r| ("lang".to_string(), r))
                    .collect();
                (n.clone(), recs)
            }
        };
        let skipped = forge_bool::certify::take_skipped();
        for path in &skipped {
            *totals.entry("out of scope").or_default() += 1;
            *scope.entry(format!("no certificate: {path}")).or_default() += 1;
        }
        for (i, (path, r)) in recs.iter().enumerate() {
            let what = format!("{name} [{path}] boolean {i} {}", forge_certify::op_name(r.op));
            let verbose = std::env::var_os("FORGE_CERTIFY_VERBOSE").is_some();
            match dump(r, &frames) {
                Err(e) if verbose => {
                    eprintln!("  {what}: out of scope: {e}");
                    *totals.entry("out of scope").or_default() += 1;
                    *scope.entry(e).or_default() += 1;
                }
                Err(e) => {
                    *totals.entry("out of scope").or_default() += 1;
                    *scope.entry(e).or_default() += 1;
                }
                Ok(line) => {
                    if let Ok(dir) = std::env::var("FORGE_CERTIFY_DUMP_ALL") {
                        let name: String = what
                            .chars()
                            .map(|c| if c.is_alphanumeric() { c } else { '_' })
                            .collect();
                        let _ = std::fs::write(format!("{dir}/{name}.json"), &line);
                    }
                    let t0 = std::time::Instant::now();
                    let v = checker.ask(&line);
                    let tc = t0.elapsed().as_secs_f64();
                    if time {
                        let tk = kernel_time(r, 3).as_secs_f64();
                        ratios.push((tc, tk, line.len() as f64));
                    }
                    if mutate && v == Verdict::Accepted {
                        let nf = r.result.as_ref().map_or(0, |b| b.face_ids().count());
                        let mut ms = vec![Mutation::DropFace(0)];
                        if nf > 1 {
                            ms.push(Mutation::DropFace(nf - 1));
                        }
                        for o in [Op::Union, Op::Subtract, Op::Intersect] {
                            if o != r.op {
                                ms.push(Mutation::Op(o));
                            }
                        }
                        for m in ms {
                            let kind = match m {
                                Mutation::DropFace(_) => "face dropped",
                                _ => "op changed",
                            };
                            let mv = match dump_mutated(r, &frames, &m) {
                                Ok(l) => checker.ask(&l),
                                Err(e) => Verdict::Scope(e),
                            };
                            let out = match &mv {
                                Verdict::Accepted => {
                                    mutants_accepted.push(format!("{what}: {m:?}"));
                                    "accepted"
                                }
                                Verdict::Rejected(_) => "rejected",
                                Verdict::Scope(_) => "out of scope",
                            };
                            *mutants.entry(format!("{kind}: {out}")).or_default() += 1;
                        }
                    }
                    if verbose {
                        eprintln!("  {what}: {v:?}");
                    }
                    match v {
                        Verdict::Accepted => *totals.entry("accepted").or_default() += 1,
                        Verdict::Rejected(why) => {
                            if let Ok(dir) = std::env::var("FORGE_CERTIFY_DUMP") {
                                let name: String = what
                                    .chars()
                                    .map(|c| if c.is_alphanumeric() { c } else { '_' })
                                    .collect();
                                let _ = std::fs::write(format!("{dir}/{name}.json"), &line);
                            }
                            *totals.entry("rejected").or_default() += 1;
                            rejected.push((what, why));
                        }
                        Verdict::Scope(why) => {
                            *totals.entry("out of scope").or_default() += 1;
                            *scope.entry(why).or_default() += 1;
                        }
                    }
                }
            }
        }
        eprintln!(
            "{} ({} booleans, {:.1}s)",
            name,
            recs.len() + skipped.len(),
            t.elapsed().as_secs_f64()
        );
    }
    println!("== totals");
    for (k, n) in &totals {
        println!("{k:14} {n}");
    }
    println!("\n== rejected");
    for (w, why) in &rejected {
        println!("{w}: {why}");
    }
    println!("\n== out of scope");
    for (s, n) in &scope {
        println!("{n:6} {s}");
    }
    if mutate {
        println!("\n== mutants (wrong on purpose)");
        for (k, n) in &mutants {
            println!("{n:6} {k}");
        }
        for m in &mutants_accepted {
            println!("accepted mutant: {m}");
        }
    }
    if time && !ratios.is_empty() {
        let mut r: Vec<f64> = ratios.iter().map(|(c, k, _)| c / k.max(1e-9)).collect();
        r.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let tc: f64 = ratios.iter().map(|x| x.0).sum();
        let tk: f64 = ratios.iter().map(|x| x.1).sum();
        let q = |p: f64| r[((r.len() - 1) as f64 * p) as usize];
        println!("\n== cost ({} booleans checked)", ratios.len());
        println!(
            "checker {:.3}s, kernel {:.3}s, total ratio {:.0}x",
            tc,
            tk,
            tc / tk.max(1e-9)
        );
        println!(
            "per boolean checker/kernel: median {:.0}x, p90 {:.0}x, max {:.0}x",
            q(0.5),
            q(0.9),
            q(1.0)
        );
        let mc: f64 = ratios.iter().map(|x| x.0).fold(0.0, f64::max);
        println!("slowest check {:.3}s", mc);
    }
}
