//! The real-part benchmarks (tools/bench-parts), for the speed and bit gate:
//!   benchparts resolve NAME OUT    the part with its selectors resolved, as a part file
//!   benchparts run FILE MODE       one regen of a resolved file: seq | nary1 | naryN (counted)
//!   benchparts bits                each part's exact result hashed, seq and nary 4
//!   benchparts font DIR            the block font the parts' text uses, as DIR/block.ttf
//!   benchparts check [NAME]        each part's lines, faces, validity, closed mesh and times
//!   benchparts nary FILE           a part file's n-ary regen, ok or its error (for shrinking)
use forge_agent::{bench, text};
use forge_math::hash::BitHasher;
use forge_model::{Feature, Part};
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |k: usize| args.get(k).map(String::as_str).expect("see the usage above");
    match arg(0) {
        "resolve" => std::fs::write(arg(2), text::print(&bench::part(arg(1)))).expect("write"),
        "run" => {
            let src = std::fs::read_to_string(arg(1)).expect("read");
            let mut part = text::parse(&src).unwrap_or_else(|(l, e)| panic!("line {l}: {e}"));
            let font = Arc::new(forge_font::Font::parse(bench::font_bytes()).expect("font"));
            for f in &mut part.features {
                if let Feature::Text(t) = f {
                    t.font.font = Some(font.clone());
                }
            }
            let ok = match arg(2) {
                "seq" => part.regenerate_solids().is_ok(),
                m => part
                    .regenerate_nary_solids(m[4..].parse().expect("naryN"))
                    .is_ok(),
            };
            assert!(ok, "{} {} failed", arg(1), arg(2));
        }
        "bits" => {
            for name in bench::NAMES {
                let part: Part = bench::part(name);
                for (path, r) in [
                    ("seq", part.regenerate_solids()),
                    ("nary", part.regenerate_nary_solids(4)),
                ] {
                    let mut h = BitHasher::new();
                    match r {
                        Ok(solids) => {
                            for b in solids.iter().map(|s| &s.body) {
                                for v in &b.vertices {
                                    h.p3(v.point);
                                }
                                for e in &b.edges {
                                    h.f64(e.range.lo);
                                    h.f64(e.range.hi);
                                }
                                h.u64(b.faces.len() as u64);
                            }
                        }
                        Err(_) => h.u64(u64::MAX),
                    }
                    println!("{name} {path} {:016x}", h.finish());
                }
            }
        }
        "check" => {
            for name in bench::NAMES
                .iter()
                .filter(|n| args.get(1).is_none_or(|a| a == *n))
            {
                let src = bench::src(name).expect("a part");
                let t = std::time::Instant::now();
                let part = bench::part(name);
                let resolve = t.elapsed().as_secs_f64() * 1e3;
                let feats = part
                    .features
                    .iter()
                    .filter(|f| !matches!(f, Feature::Param(_) | Feature::Sketch(_)))
                    .count();
                print!(
                    "{name}: {} lines, {feats} features, resolve {resolve:.0} ms",
                    src.lines().count()
                );
                for (path, threads) in [("seq", 0), ("nary1", 1), ("nary4", 4)] {
                    let t = std::time::Instant::now();
                    let r = if threads == 0 {
                        part.regenerate_solids()
                    } else {
                        part.regenerate_nary_solids(threads)
                    };
                    let ms = t.elapsed().as_secs_f64() * 1e3;
                    match r {
                        Ok(solids) => {
                            let bodies = solids.iter().map(|s| &s.body);
                            let (mut faces, mut bad, mut open) = (0, 0, 0);
                            for b in bodies {
                                faces += b.faces.len();
                                bad += forge_topo::check::check(b).len();
                                open += forge_mesh::tessellate(b, &forge_mesh::Quality::default())
                                    .map_or(usize::MAX, |m| m.open_edges().len());
                            }
                            print!(
                                " | {path} {ms:.0} ms {} bodies {faces} faces, {bad} problems, {open} open edges",
                                solids.len()
                            );
                        }
                        Err(e) => print!(" | {path} FAILED {e:?}"),
                    }
                }
                println!();
            }
        }
        "nary" => {
            let m = bench::model(&std::fs::read_to_string(arg(1)).expect("read"));
            let part = Part {
                features: m.features.clone(),
            };
            let faces = |s: &[forge_model::Solid]| s.iter().map(|s| s.body.faces.len()).sum::<usize>();
            match (&m.error, part.regenerate_nary_solids(1)) {
                (Some(e), _) => println!("model error line {}: {}", m.lines[e.at].no, e.msg),
                (None, Ok(s)) => {
                    let bad: usize = s.iter().map(|s| forge_topo::check::check(&s.body).len()).sum();
                    let seq = part.regenerate_solids().map_or(0, |s| faces(&s));
                    println!("ok, {bad} problems, {} faces (seq {seq})", faces(&s))
                }
                (None, Err(e)) => println!("nary error {e:?}"),
            }
        }
        "font" => std::fs::write(
            std::path::Path::new(arg(1)).join(bench::FONT),
            bench::font_bytes(),
        )
        .expect("write"),
        a => panic!("unknown command {a}"),
    }
}
