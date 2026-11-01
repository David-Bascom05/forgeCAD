//! Per-feature cost, for instruction counting by prefix difference (research/feature-costs):
//!   featcost sources                  every source: suite, fillet corpus, finishing, edits, bodies
//!   featcost resolve NAME OUT         a bench part with its selectors resolved, as a part file
//!   featcost list SRC                 index, kind label, size of each feature of SRC
//!   featcost run SRC N MODE           regen of SRC's first N features: seq | nary1 (counted)
//!   featcost post SRC WHAT            regen, then none | mesh | mass | check on each body
//!   featcost info SRC MODE            bodies, faces, edges, problems, open mesh edges, volume, area
//! SRC is a resolved .fc file or a source name.
use forge_agent::{bench, text};
use forge_model::{BodyFeature, Edit, End, Extent, Feature, Head, Layout, Part, Source, Turn};
use forge_model::{edits, fillets, finishing, names, suite, torture};
use std::sync::Arc;

fn sources() -> Vec<(String, Part)> {
    let mut v: Vec<(String, Part)> = vec![("torture".into(), torture::part(221))];
    v.push(("torture-2".into(), suite::torture2()));
    let three = |p: &str, l: Vec<(&'static str, Part, Option<f64>)>| -> Vec<(String, Part)> {
        l.into_iter().map(|c| (format!("{p}/{}", c.0), c.1)).collect()
    };
    v.extend(suite::parts().into_iter().map(|(n, p)| (format!("suite/{n}"), p)));
    v.extend(three("fillet", fillets::cases()));
    v.extend(three("variant", fillets::variant_cases()));
    v.extend(three("variable", fillets::variable_cases()));
    v.extend(three("finishing", finishing::parts()));
    v.extend(three("edits", edits::parts()));
    v.extend(names::body_parts().into_iter().map(|(n, p)| (format!("bodies/{n}"), p)));
    v
}

fn load(src: &str) -> Part {
    if src.ends_with(".fc") {
        let s = std::fs::read_to_string(src).expect("read");
        let mut part = text::parse(&s).unwrap_or_else(|(l, e)| panic!("line {l}: {e}"));
        let font = Arc::new(forge_font::Font::parse(bench::font_bytes()).expect("font"));
        for f in &mut part.features {
            if let Feature::Text(t) = f {
                t.font.font = Some(font.clone());
            }
        }
        return part;
    }
    // only this part is built: the count is its regen's
    match src {
        "torture" => return torture::part(221),
        "torture-2" => return suite::torture2(),
        _ => {}
    }
    if let Some(p) = src.strip_prefix("suite/").and_then(suite::part) {
        return p;
    }
    sources().into_iter().find(|s| s.0 == src).expect("a source").1
}

fn op_name(f: impl std::fmt::Debug) -> String {
    format!("{f:?}").to_lowercase()
}

fn first_word(f: impl std::fmt::Debug) -> String {
    let s = format!("{f:?}");
    let n = s.find(|c: char| !c.is_alphanumeric()).unwrap_or(s.len());
    s[..n].to_lowercase()
}

/// The feature's kind with its options, and its size: edges, copies, profile segments, letters.
fn label(f: &Feature) -> (String, usize) {
    let segs = |p: &forge_ops::Profile| p.loops.iter().map(|l| l.segments.len()).sum::<usize>();
    match f {
        Feature::Extrude { profile, extent, op } => {
            let e = match extent {
                Extent::Distance(_) => "dist",
                Extent::Tapered { .. } => "taper",
                Extent::Symmetric(_) => "sym",
                Extent::TwoSided { .. } => "two",
                Extent::Through(_) => "through",
                Extent::To { .. } => "to",
                Extent::ToNext { .. } => "next",
                _ => "other",
            };
            (format!("extrude.{e}.{}", op_name(op)), segs(profile))
        }
        Feature::Revolve { profile, extent, op, .. } => {
            let e = match extent {
                Turn::Angle(_) => "angle",
                Turn::Symmetric(_) => "sym",
                Turn::TwoSided { .. } => "two",
                Turn::To { .. } => "to",
            };
            (format!("revolve.{e}.{}", op_name(op)), segs(profile))
        }
        Feature::Ball { op, .. } => (format!("ball.{}", op_name(op)), 1),
        Feature::Fillet { edges, setbacks, .. } => {
            let var = edges.iter().any(|e| !matches!(e.1, forge_ops::Radius::Constant(_)));
            let k = if var { "variable" } else { "constant" };
            let s = if setbacks.is_empty() { "" } else { ".setback" };
            (format!("fillet.{k}{s}"), edges.len())
        }
        Feature::FaceFillet { first, second, .. } => ("fillet.face".into(), first.len() + second.len()),
        Feature::FullRound { .. } => ("fillet.fullround".into(), 1),
        Feature::Chamfer { edges, size, .. } => (format!("chamfer.{}", first_word(size)), edges.len()),
        Feature::Shell { outward, open, .. } => {
            (format!("shell.{}", if *outward { "out" } else { "in" }), open.len())
        }
        Feature::Draft { faces, .. } => ("draft".into(), faces.len()),
        Feature::Hole(h) => {
            let head = match h.head {
                Head::Plain => "plain",
                Head::Counterbore { .. } => "cbore",
                Head::Countersink { .. } => "csink",
            };
            let end = match h.end {
                End::Through => "through",
                End::Depth(_) => "depth",
                End::To { .. } => "to",
                End::ToNext => "next",
            };
            let n = match &h.places {
                Some(forge_model::At::Sketch { at, .. }) => at.len(),
                _ => 1,
            };
            (format!("hole.{head}.{end}"), n)
        }
        Feature::Pattern { of, layout } => {
            let s = match of {
                Source::Features(_) => "features",
                Source::Body => "body",
            };
            let (l, n) = match layout {
                Layout::Linear { first, second } => {
                    ("linear", first.count * second.map_or(1, |r| r.count))
                }
                Layout::Circular { count, .. } => ("circular", *count),
                Layout::Mirror { .. } => ("mirror", 2),
                Layout::Path { count, .. } => ("path", *count),
                Layout::Points(p) => ("points", p.len()),
            };
            (format!("pattern.{s}.{l}"), n)
        }
        Feature::Plane(_) | Feature::Work(_) | Feature::Param(_) | Feature::Sketch(_) => ("none".into(), 0),
        Feature::Sweep(s) => (
            format!("sweep.{}.{}", first_word(&s.ends), op_name(s.op)),
            s.path.len(),
        ),
        Feature::Coil(c) => (format!("coil.{}", op_name(c.op)), segs(&c.profile)),
        Feature::Loft(l) => (
            format!("loft.{}.{}", first_word(&l.kind), op_name(l.op)),
            l.profiles.iter().map(&segs).sum(),
        ),
        Feature::Thread(t) => (format!("thread.{}", first_word(t.on)), 1),
        Feature::Text(t) => (
            format!("text.{}.{}", first_word(t.on), op_name(t.op)),
            t.text.len(),
        ),
        Feature::Edit(e) => match e {
            Edit::Rib(r) => ("edit.rib".into(), r.path.len()),
            Edit::Offset { faces, .. } => ("edit.offset".into(), faces.len()),
            Edit::Thicken { faces, op, .. } => (format!("edit.thicken.{}", op_name(op)), faces.len()),
            Edit::Delete { faces } => ("edit.delete".into(), faces.len()),
            Edit::Replace { .. } => ("edit.replace".into(), 1),
            Edit::Move { faces, .. } => ("edit.move".into(), faces.len()),
        },
        Feature::Body(b) => match b.as_ref() {
            BodyFeature::In { feature, .. } => {
                let (l, n) = label(feature);
                (format!("in.{l}"), n)
            }
            BodyFeature::Combine { op, tools, .. } => (format!("body.combine.{}", op_name(op)), tools.len()),
            BodyFeature::Split { keep, .. } => (format!("body.split.{}", op_name(keep)), 1),
            BodyFeature::SplitFace { faces, .. } => ("body.splitface".into(), faces.len()),
            BodyFeature::Move { steps, copy } => {
                (format!("body.move{}", if *copy { ".copy" } else { "" }), steps.len())
            }
        },
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |k: usize| args.get(k).map(String::as_str).expect("see the usage above");
    match arg(0) {
        "sources" => {
            for (n, _) in sources() {
                println!("{n}");
            }
        }
        "resolve" => std::fs::write(arg(2), text::print(&bench::part(arg(1)))).expect("write"),
        // a part file with selectors, resolved (a generator's output); prints why not where not
        "resolve-file" => {
            let m = bench::model(&std::fs::read_to_string(arg(1)).expect("read"));
            match &m.error {
                Some(e) => println!("err {}", e.msg),
                None => {
                    let part = Part {
                        features: m.features.clone(),
                    };
                    std::fs::write(arg(2), text::print(&part)).expect("write")
                }
            }
        }
        "list" => {
            for (i, f) in load(arg(1)).features.iter().enumerate() {
                let (l, n) = label(f);
                println!("{i}\t{l}\t{n}");
            }
        }
        "run" => {
            let mut part = load(arg(1));
            part.features.truncate(arg(2).parse().expect("N"));
            let r = match arg(3) {
                "seq" => part.regenerate_solids(),
                m => part.regenerate_nary_solids(m[4..].parse().expect("naryN")),
            };
            match r {
                Ok(s) => println!("ok {}", s.iter().map(|s| s.body.faces.len()).sum::<usize>()),
                Err(e) => println!("err {}", first_word(e)),
            }
        }
        "post" => {
            let part = load(arg(1));
            let solids = part.regenerate_solids().expect("regen");
            let q = forge_mesh::Quality::default();
            for s in &solids {
                match arg(2) {
                    "mesh" => {
                        std::hint::black_box(forge_mesh::tessellate(&s.body, &q).expect("mesh"));
                    }
                    "mass" => {
                        std::hint::black_box(forge_topo::mass::mass(&s.body).expect("mass"));
                    }
                    "check" => {
                        std::hint::black_box(forge_topo::check(&s.body));
                    }
                    _ => {}
                }
            }
        }
        "info" => {
            let part = load(arg(1));
            let r = match arg(2) {
                "seq" => part.regenerate_solids(),
                m => part.regenerate_nary_solids(m[4..].parse().expect("naryN")),
            };
            let solids = match r {
                Ok(s) => s,
                Err(e) => {
                    println!("err {e:?}");
                    return;
                }
            };
            let q = forge_mesh::Quality::default();
            let (mut faces, mut edges, mut bad, mut open, mut vol, mut area) = (0, 0, 0, 0, 0.0, 0.0);
            for s in &solids {
                let b = &s.body;
                faces += b.faces.len();
                edges += b.edges.len();
                bad += forge_topo::check(b).len();
                open += forge_mesh::tessellate(b, &q).map_or(usize::MAX / 2, |m| m.open_edges().len());
                if let Ok(m) = forge_topo::mass::mass(b) {
                    vol += m.volume;
                    area += m.area;
                }
            }
            println!(
                "ok bodies {} faces {faces} edges {edges} problems {bad} open {open} volume {vol:.6} area {area:.6}",
                solids.len()
            );
        }
        a => panic!("unknown command {a}"),
    }
}
