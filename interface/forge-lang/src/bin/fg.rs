//! `fg`: run, check, edit and explain Forge (.fg) parts.

use forge_agent::json::Json;
use forge_lang::build::Cache;
use forge_lang::error::Op;
use forge_lang::{Options, edit, report};
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::process::ExitCode;

const USAGE: &str = "fg run FILE [--json] [--render OUT.png] [--view iso|front|top|right|..] [--px N] [--dxf OUT.dxf] [--step OUT.step]
fg check FILE [--json]          parse, static rules, types and dimensions; no kernel
fg edit FILE 'OP' [--json]      apply a typed op (JSON, as errors' fixes give them), then check
fg handles FILE                 the editable handles: every param, its value and dimension
fg serve                        JSON lines on stdin: {\"cmd\":\"run\"|\"check\",\"file\":F}; stages kept per file
fg reference                    the language reference";

fn read(file: &str) -> Result<String, String> {
    std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))
}

/// The directory a part file is in: where the files it reads are found.
fn dir_of(file: &str) -> Option<std::path::PathBuf> {
    std::fs::canonicalize(file)
        .ok()?
        .parent()
        .map(|d| d.to_path_buf())
}

fn flag(args: &[String], f: &str) -> bool {
    args.iter().any(|a| a == f)
}

fn opt<'a>(args: &'a [String], f: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == f)
        .and_then(|k| args.get(k + 1))
        .map(String::as_str)
}

fn print_json(j: &Json) {
    let mut s = String::new();
    j.write(&mut s);
    println!("{s}");
}

/// Each sheet's flat pattern as DXF: OUT itself for one sheet, OUT with `-<id>` before its
/// extension for each of several.
fn write_dxf(run: &forge_lang::Run, out: &str) {
    let many = run.sheets.len() > 1;
    if run.sheets.is_empty() {
        eprintln!("--dxf: the part has no sheet");
    }
    for s in &run.sheets {
        let path = if many {
            match out.rsplit_once('.') {
                Some((stem, ext)) => format!("{stem}-{}.{ext}", s.id),
                None => format!("{out}-{}", s.id),
            }
        } else {
            out.to_string()
        };
        match s.dxf() {
            Ok(text) => {
                if let Err(e) = std::fs::write(&path, text) {
                    eprintln!("{path}: {e}");
                }
            }
            Err(e) => eprintln!("--dxf {}: the sheet is refused: {e:?}", s.id),
        }
    }
}

/// The file's name without its directory and extension: the product's name.
fn stem(file: &str) -> String {
    std::path::Path::new(file)
        .file_stem()
        .map_or_else(|| "part".into(), |n| n.to_string_lossy().into_owned())
}

/// The part's STEP (folded) in OUT and each flat pattern beside it (`OUT-<unfold>.step`); an
/// assembly's structure in OUT. False when nothing could be written.
fn write_step(run: &forge_lang::Run, out: &str, name: &str) -> bool {
    match forge_lang::export::step(run, name) {
        Ok(files) => {
            for f in files {
                let path = match (f.suffix.is_empty(), out.rsplit_once('.')) {
                    (true, _) => out.to_string(),
                    (false, Some((stem, ext))) => format!("{stem}{}.{ext}", f.suffix),
                    (false, None) => format!("{out}{}", f.suffix),
                };
                if let Err(e) = std::fs::write(&path, f.text) {
                    eprintln!("{path}: {e}");
                    return false;
                }
                eprintln!("--step: {path}, {} solids", f.solids);
            }
            true
        }
        Err(e) => {
            eprintln!("--step: {e}");
            false
        }
    }
}

fn run_file(file: &str, args: &[String], build: bool) -> ExitCode {
    let src = match read(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let opts = Options {
        build,
        dir: dir_of(file),
        render: opt(args, "--render").is_some(),
        step: opt(args, "--step").map(|_| stem(file)),
        ..Options::default()
    };
    let (run, _) = forge_lang::run(&src, opts, Cache::default());
    let posed = run.asm.as_ref().and_then(forge_lang::asm::compound);
    if let (Some(out), Some(body)) = (opt(args, "--render"), run.body.as_ref().or(posed.as_ref())) {
        let view = opt(args, "--view").unwrap_or("iso");
        let px = opt(args, "--px").and_then(|p| p.parse().ok()).unwrap_or(600);
        match forge_lang::render(body, view, px) {
            Ok(png) => {
                if let Err(e) = std::fs::write(out, png) {
                    eprintln!("{out}: {e}");
                }
            }
            Err(e) => eprintln!("render: {e}"),
        }
    }
    if let Some(out) = opt(args, "--dxf") {
        write_dxf(&run, out);
    }
    let mut wrote = true;
    if let Some(out) = opt(args, "--step") {
        wrote = write_step(&run, out, &stem(file));
    }
    if flag(args, "--json") {
        print_json(&report::json(&run, &src));
    } else {
        print!("{}", report::text(&run, &src));
    }
    if run.ok() && wrote {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("run") if args.len() >= 2 => run_file(&args[1], &args[2..], true),
        Some("check") if args.len() >= 2 => run_file(&args[1], &args[2..], false),
        Some("reference") => {
            print!("{}", forge_lang::REFERENCE);
            ExitCode::SUCCESS
        }
        Some("handles") if args.len() >= 2 => {
            let src = match read(&args[1]) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(2);
                }
            };
            for (k, l) in edit::handles(&src) {
                println!("{k} = {}  ({})", l.text(), l.dim.name());
            }
            ExitCode::SUCCESS
        }
        Some("edit") if args.len() >= 3 => {
            let file = &args[1];
            let src = match read(file) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(2);
                }
            };
            let Some(op) = Json::parse(&args[2]).ok().and_then(|j| Op::from_json(&j)) else {
                eprintln!("not an op: {}", args[2]);
                return ExitCode::from(2);
            };
            match edit::apply_checked(&src, &op) {
                Ok((new, reach)) => {
                    if let Err(e) = std::fs::write(file, &new) {
                        eprintln!("{file}: {e}");
                        return ExitCode::from(2);
                    }
                    let mut f = vec![("ok", true.into()), ("op", op.json())];
                    if let Some(r) = reach {
                        f.push(("drives_values", r.values.into()));
                        f.push((
                            "features",
                            Json::Arr(r.features.iter().map(|x| Json::Str(x.clone())).collect()),
                        ));
                        f.push(("structural", r.structural.into()));
                    }
                    print_json(&Json::obj(f));
                    run_file(file, &args[3..], false)
                }
                Err(e) => {
                    print_json(&Json::obj(vec![("ok", false.into()), ("error", e.json(&src))]));
                    ExitCode::from(1)
                }
            }
        }
        Some("serve") => {
            let mut caches: HashMap<String, Cache> = HashMap::new();
            let stdin = std::io::stdin();
            let mut out = std::io::stdout();
            for line in stdin.lock().lines() {
                let Ok(line) = line else { break };
                let Ok(req) = Json::parse(&line) else {
                    let _ = writeln!(out, "{{\"ok\":false,\"error\":\"not JSON\"}}");
                    continue;
                };
                let file = req.get("file").and_then(Json::str).unwrap_or("").to_string();
                let cmd = req.get("cmd").and_then(Json::str).unwrap_or("run");
                let src = match read(&file) {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = writeln!(out, "{{\"ok\":false,\"error\":{:?}}}", e);
                        continue;
                    }
                };
                let cache = caches.remove(&file).unwrap_or_default();
                let opts = Options {
                    build: cmd != "check",
                    dir: dir_of(&file),
                    ..Options::default()
                };
                let (run, cache) = forge_lang::run(&src, opts, cache);
                caches.insert(file, cache);
                let mut s = String::new();
                report::json(&run, &src).write(&mut s);
                let _ = writeln!(out, "{s}");
                let _ = out.flush();
            }
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
