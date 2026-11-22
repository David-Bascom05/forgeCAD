//! The real-part benchmarks (parts/bench/*.fg), for the speed and bit gate:
//!   benchparts resolve NAME        the part built from its .fg (evaluation, selections and the
//!                                  kernel's own build): the cost `run` subtracts
//!   benchparts run NAME MODE       resolve, then one more regen of its kernel features by the
//!                                  kernel alone: seq | nary1 | naryN (the counted part)
//!   benchparts bits                each part's exact result hashed, seq and nary 4
//!   benchparts font DIR            the block font the parts' text uses, as DIR/block.ttf
//!   benchparts check [NAME]        each part's features, faces, validity, closed mesh and times
//!   benchparts nary FILE.fg        a part file's n-ary regen, ok or its error (for shrinking)
//! (`run`'s instructions less `resolve`'s are the kernel's regen alone, as the .fc gate counted
//! one regen of a part whose selectors were resolved beforehand.)
use forge_math::hash::BitHasher;
use forge_model::{Feature, Part, Solid};

/// The bench parts, by name.
fn names() -> Vec<String> {
    let dir = format!("{}/parts/bench", env!("CARGO_MANIFEST_DIR"));
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .expect("the bench parts")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "fg"))
        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .collect();
    v.sort();
    v
}

/// A folder with the block font in it, for the parts' text.
fn font_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("forge-benchparts-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(
        dir.join(forge_agent::bench::FONT),
        forge_agent::bench::font_bytes(),
    )
    .expect("font");
    dir
}

/// Part `src` built: its kernel features, their ids, and the run's errors.
fn build(src: &str, dir: std::path::PathBuf) -> (Part, Vec<u64>, Vec<forge_lang::error::Error>) {
    let opts = forge_lang::Options {
        dir: Some(dir),
        checks: false,
        ..forge_lang::Options::default()
    };
    let (run, cache) = forge_lang::run(src, opts, Default::default());
    let (f, ids) = cache.history();
    (Part { features: f.to_vec() }, ids.to_vec(), run.errors)
}

fn bench(name: &str) -> (Part, Vec<u64>) {
    let path = format!("{}/parts/bench/{name}.fg", env!("CARGO_MANIFEST_DIR"));
    let src = std::fs::read_to_string(&path).expect("a bench part");
    let (p, ids, errors) = build(&src, font_dir());
    assert!(errors.is_empty(), "{name}: {errors:?}");
    (p, ids)
}

fn regen(part: &Part, ids: &[u64], mode: &str) -> Result<Vec<Solid>, forge_model::RegenError> {
    match mode {
        "seq" => part.regenerate_solids_ids(Some(ids)),
        m => part.regenerate_nary_solids_ids(m[4..].parse().expect("naryN"), Some(ids)),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |k: usize| args.get(k).map(String::as_str).expect("see the usage above");
    match arg(0) {
        "resolve" => {
            std::hint::black_box(bench(arg(1)));
        }
        "run" => {
            let (part, ids) = bench(arg(1));
            assert!(regen(&part, &ids, arg(2)).is_ok(), "{} {} failed", arg(1), arg(2));
        }
        "bits" => {
            for name in names() {
                let (part, ids) = bench(&name);
                for (path, r) in [
                    ("seq", regen(&part, &ids, "seq")),
                    ("nary", regen(&part, &ids, "nary4")),
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
            for name in names().iter().filter(|n| args.get(1).is_none_or(|a| a == *n)) {
                let t = std::time::Instant::now();
                let (part, ids) = bench(name);
                let resolve = t.elapsed().as_secs_f64() * 1e3;
                let feats = part
                    .features
                    .iter()
                    .filter(|f| !matches!(f, Feature::Param(_) | Feature::Sketch(_)))
                    .count();
                print!("{name}: {feats} features, built from .fg {resolve:.0} ms");
                for (path, mode) in [("seq", "seq"), ("nary1", "nary1"), ("nary4", "nary4")] {
                    let t = std::time::Instant::now();
                    let r = regen(&part, &ids, mode);
                    let ms = t.elapsed().as_secs_f64() * 1e3;
                    match r {
                        Ok(solids) => {
                            let (mut faces, mut bad, mut open) = (0, 0, 0);
                            for b in solids.iter().map(|s| &s.body) {
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
            let file = std::path::Path::new(arg(1));
            let src = std::fs::read_to_string(file).expect("read");
            let dir = file.parent().map(|d| d.to_path_buf()).unwrap_or_else(font_dir);
            let (part, ids, errors) = build(&src, dir);
            let faces = |s: &[Solid]| s.iter().map(|s| s.body.faces.len()).sum::<usize>();
            match (errors.first(), regen(&part, &ids, "nary1")) {
                (Some(e), _) => println!("model error: {e:?}"),
                (None, Ok(s)) => {
                    let bad: usize = s.iter().map(|s| forge_topo::check::check(&s.body).len()).sum();
                    let seq = regen(&part, &ids, "seq").map_or(0, |s| faces(&s));
                    println!("ok, {bad} problems, {} faces (seq {seq})", faces(&s))
                }
                (None, Err(e)) => println!("nary error {e:?}"),
            }
        }
        "font" => std::fs::write(
            std::path::Path::new(arg(1)).join(forge_agent::bench::FONT),
            forge_agent::bench::font_bytes(),
        )
        .expect("write"),
        a => panic!("unknown command {a}"),
    }
}
