//! The real-part benchmarks (parts/bench/*.fg), for the speed and bit gate:
//!   benchparts resolve NAME        the part built from its .fg (evaluation, selections and the
//!                                  kernel's own build) as the app builds it, without checks: the
//!                                  cost `run` subtracts, and the cold build the gate holds
//!   benchparts cli NAME            the same with the CLI's options (measures, checks)
//!   benchparts edit NAME           resolve, then the part rebuilt from that run's cache after its
//!                                  first param scaled by 1.01, as an edit in the app (the gate
//!                                  holds both builds together)
//!   benchparts run NAME MODE       resolve, then one more regen of its kernel features by the
//!                                  kernel alone: seq | nary1 | naryN (the counted part)
//!   benchparts bits                each part's exact result hashed, seq and nary 4
//!   benchparts font DIR            the block font the parts' text uses, as DIR/block.ttf
//!   benchparts check [NAME]        each part's features, faces, validity, closed mesh and times
//!   benchparts nary FILE.fg        a part file's n-ary regen, ok or its error (for shrinking)
//! (`run`'s instructions less `resolve`'s are the kernel's regen alone: one regen of a part whose
//! selectors were resolved beforehand.)
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
    // (one name for every run, not one by process id: a count must not depend on the id's digits)
    let dir = forge_dirs::temp_dir().join("forge-benchparts");
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(dir.join(forge_lang::bench::FONT), forge_lang::bench::font_bytes()).expect("font");
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
    let (p, ids, errors) = build(&source(name), font_dir());
    assert!(errors.is_empty(), "{name}: {errors:?}");
    (p, ids)
}

fn source(name: &str) -> String {
    // (FORGE_BENCH_PARTS: the folder the speed gate copies them to, a path of one length from
    // any checkout, so a count does not depend on where the checkout is)
    let path = match std::env::var("FORGE_BENCH_PARTS") {
        Ok(dir) => format!("{dir}/{name}.fg"),
        Err(_) => format!("{}/parts/bench/{name}.fg", env!("CARGO_MANIFEST_DIR")),
    };
    std::fs::read_to_string(&path).expect("a bench part")
}

/// The text with its first param's value scaled by 1.01: an edit of one dimension.
fn edited(src: &str) -> String {
    let mut out = String::with_capacity(src.len() + 8);
    let mut done = false;
    for line in src.split_inclusive('\n') {
        let body = line.trim_end();
        let head = body
            .strip_prefix("shared ")
            .or_else(|| body.strip_prefix("param "));
        if !done
            && let Some((_, v)) = head.and_then(|h| h.split_once(" = "))
            && let Ok(x) = v.split('#').next().unwrap_or("").trim().parse::<f64>()
        {
            let prefix = &line[..line.find(" = ").expect("found") + 3];
            out.push_str(&format!("{prefix}{}\n", x * 1.01));
            done = true;
            continue;
        }
        out.push_str(line);
    }
    assert!(done, "a param to edit");
    out
}

/// A regen's exact result hashed, as the gate keeps it.
fn bits_of(r: &Result<Vec<Solid>, forge_model::RegenError>) -> u64 {
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
    h.finish()
}

/// (never inlined: the speed gate counts this call alone, `--toggle-collect=*benchparts::regen`)
#[inline(never)]
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
        "cli" => {
            let opts = forge_lang::Options {
                dir: Some(font_dir()),
                ..forge_lang::Options::default()
            };
            let (run, _) = forge_lang::run(&source(arg(1)), opts, Default::default());
            assert!(run.errors.is_empty(), "{}: {:?}", arg(1), run.errors);
            std::hint::black_box(run.measure.map(|m| m.volume));
        }
        "edit" => {
            let src = source(arg(1));
            let opts = forge_lang::Options {
                dir: Some(font_dir()),
                checks: false,
                ..forge_lang::Options::default()
            };
            let (run, cache) = forge_lang::run(&src, opts.clone(), Default::default());
            assert!(run.errors.is_empty(), "{}: {:?}", arg(1), run.errors);
            let (run, _) = forge_lang::run(&edited(&src), opts, cache);
            assert!(run.errors.is_empty(), "{} edited: {:?}", arg(1), run.errors);
            std::hint::black_box(run.body.map(|b| b.faces.len()));
        }
        "run" => {
            let (part, ids) = bench(arg(1));
            assert!(regen(&part, &ids, arg(2)).is_ok(), "{} {} failed", arg(1), arg(2));
        }
        "measures" => {
            // `measures [part] [chord]`
            let chord = args.get(2).and_then(|c| c.parse().ok()).unwrap_or(0.02);
            let q = forge_mesh::Quality {
                chord,
                max_angle: 0.5,
            };
            for name in names()
                .iter()
                .filter(|n| args.get(1).is_none_or(|a| a == *n || a == "all"))
            {
                let (part, ids) = bench(name);
                for (path, mode) in [("seq", "seq"), ("nary", "nary4")] {
                    let line = match regen(&part, &ids, mode) {
                        Ok(solids) => {
                            let (mut valid, mut vol, mut area, mut faces) = (true, 0.0, 0.0, 0);
                            for b in solids.iter().map(|s| &s.body) {
                                valid &= forge_topo::check(b).is_empty();
                                faces += b.faces.len();
                                match forge_mesh::tessellate(b, &q) {
                                    Ok(m) => {
                                        vol += m.volume();
                                        area += m.area();
                                    }
                                    Err(_) => valid = false,
                                }
                            }
                            format!(
                                "{} {vol:.6} {area:.6} {faces}",
                                if valid { "valid" } else { "INVALID" }
                            )
                        }
                        Err(_) => "error".into(),
                    };
                    println!("{name} {path} {line}");
                }
            }
        }
        "bits" => {
            for name in names() {
                let (part, ids) = bench(&name);
                for (path, r) in [
                    ("seq", regen(&part, &ids, "seq")),
                    ("nary", regen(&part, &ids, "nary4")),
                ] {
                    println!("{name} {path} {:016x}", bits_of(&r));
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
            std::path::Path::new(arg(1)).join(forge_lang::bench::FONT),
            forge_lang::bench::font_bytes(),
        )
        .expect("write"),
        a => panic!("unknown command {a}"),
    }
}
