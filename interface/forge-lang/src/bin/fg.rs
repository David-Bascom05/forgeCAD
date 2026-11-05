//! `fg`: run, check, edit and explain Forge (.fg) parts.

use forge_agent::json::Json;
use forge_lang::build::Cache;
use forge_lang::error::Op;
use forge_lang::{Options, edit, report};
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::process::ExitCode;

const USAGE: &str = "fg run FILE [--json] [--render OUT.png] [--view iso|front|top|right|..] [--px N] [--dxf OUT.dxf] [--step OUT.step] [--field OUT.csv|inline]
                                --field: each thermoform's wall at each point, as CSV (the JSON
                                names the file) or inline in the JSON; left out, only its count
fg run DRAWING.fg [--pdf OUT] [--dxf OUT] [--svg OUT] [--refs VIEW]   a drawing: its sheets, report or a view's references
fg check FILE [--json]          parse, static rules, types and dimensions; no kernel
fg edit FILE 'OP' [--json]      apply a typed op (JSON, as errors' fixes give them), then check
fg handles FILE                 the editable handles: every param, its value and dimension
fg serve                        JSON lines on stdin: {\"cmd\":\"run\"|\"check\",\"file\":F[,\"checks\":false]}; stages kept per file
fg reference                    the language reference
fg view FILE [--view front,top,..] [--tangent] [--chord MM] [--pdf OUT] [--svg OUT] [--points]
                                a part's or assembly's exact hidden-line views: lengths by class (JSON), pages
fg std [FAMILY [SIZE [LENGTH]]] [--thread cosmetic] [--grade G] [--sheet T] [-o OUT.fg]
                                the standard parts: families, a family's sizes and lengths, or a
                                part's file (FILE may also be its key, `fg run 'std:ISO 4762|M6|20'`)";

fn read(file: &str) -> Result<String, String> {
    if file.starts_with(forge_lang::stdparts::KEY) {
        return forge_lang::stdparts::text_of(file).ok_or(format!("{file}: no such standard part"));
    }
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

/// What a run prints and writes: made whole first, so a run kept on disk (`store`) gives back
/// the same, byte for byte.
#[derive(Default)]
struct Output {
    stdout: String,
    stderr: String,
    /// Each file, and whether failing to write it fails the run (a STEP's).
    files: Vec<(String, Vec<u8>, bool)>,
    code: u8,
}

impl Output {
    fn err(&mut self, line: impl std::fmt::Display) {
        self.stderr.push_str(&format!("{line}\n"));
    }

    fn json(&mut self, j: &Json) {
        j.write(&mut self.stdout);
        self.stdout.push('\n');
    }

    fn write(&mut self, path: String, bytes: impl Into<Vec<u8>>) {
        self.files.push((path, bytes.into(), false));
    }

    /// Writes its files and prints what it printed, as the run would have.
    fn emit(self) -> ExitCode {
        let mut stderr = self.stderr;
        let mut code = self.code;
        for (path, bytes, must) in &self.files {
            if let Err(e) = std::fs::write(path, bytes) {
                stderr.push_str(&format!("{path}: {e}\n"));
                if *must {
                    code = code.max(1);
                }
            }
        }
        eprint!("{stderr}");
        print!("{}", self.stdout);
        ExitCode::from(code)
    }

    fn save(&self, o: &mut forge_lang::store::Out) {
        o.str(&self.stdout);
        o.str(&self.stderr);
        o.u64(self.files.len() as u64);
        for (p, b, must) in &self.files {
            o.str(p);
            o.bytes(b);
            o.bool(*must);
        }
        o.u64(self.code as u64);
    }

    fn load(i: &mut forge_lang::store::In) -> Option<Output> {
        let stdout = i.str()?.to_string();
        let stderr = i.str()?.to_string();
        let n = i.count(16)?;
        let mut files = Vec::with_capacity(n);
        for _ in 0..n {
            files.push((i.str()?.to_string(), i.bytes()?.to_vec(), i.bool()?));
        }
        let code = u8::try_from(i.u64()?).ok()?;
        i.done().then_some(Output {
            stdout,
            stderr,
            files,
            code,
        })
    }
}

/// Each sheet's flat pattern as DXF: OUT itself for one sheet, OUT with `-<id>` before its
/// extension for each of several.
fn write_dxf(run: &forge_lang::Run, out: &str, o: &mut Output) {
    let many = run.sheets.len() > 1;
    if run.sheets.is_empty() {
        o.err("--dxf: the part has no sheet");
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
            Ok(text) => o.write(path, text),
            Err(e) => o.err(format!("--dxf {}: the sheet is refused: {e:?}", s.id)),
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
/// assembly's structure in OUT. False when nothing could be made.
fn write_step(run: &forge_lang::Run, out: &str, name: &str, o: &mut Output) -> bool {
    match forge_lang::export::step(run, name) {
        Ok(files) => {
            for f in files {
                let path = match (f.suffix.is_empty(), out.rsplit_once('.')) {
                    (true, _) => out.to_string(),
                    (false, Some((stem, ext))) => format!("{stem}{}.{ext}", f.suffix),
                    (false, None) => format!("{out}{}", f.suffix),
                };
                o.err(format!("--step: {path}, {} solids", f.solids));
                o.files.push((path, f.text.into(), true));
            }
            true
        }
        Err(e) => {
            o.err(format!("--step: {e}"));
            false
        }
    }
}

/// A drawing: made, its sheets written as PDF (all sheets), SVG and DXF (one per sheet: OUT with
/// `-<n>` before the extension when there are several), its report printed.
fn run_drawing(file: &str, src: &str, args: &[String], o: &mut Output) {
    let dir = dir_of(file).unwrap_or_default();
    // one of a .fg's drawing blocks, drawing the document it is in
    let d = match opt(args, "--drawing") {
        Some(id) => {
            let own = std::path::Path::new(file)
                .file_name()
                .map_or_else(|| file.to_string(), |n| n.to_string_lossy().to_string());
            match forge_lang::drawing::block::make(src, &dir, None, &own, id) {
                Some(d) => d,
                None => {
                    o.err(format!("no drawing block {id}"));
                    o.code = 2;
                    return;
                }
            }
        }
        None => forge_lang::drawing::regen::make(src, &dir),
    };
    if let Some(out) = opt(args, "--pdf") {
        o.write(out.to_string(), forge_draw::pdf::pdf(&d.pages));
    }
    let per_sheet = |out: &str, k: usize| match (d.pages.len() > 1, out.rsplit_once('.')) {
        (false, _) => out.to_string(),
        (true, Some((stem, ext))) => format!("{stem}-{}.{ext}", k + 1),
        (true, None) => format!("{out}-{}", k + 1),
    };
    for (k, p) in d.pages.iter().enumerate() {
        if let Some(out) = opt(args, "--svg") {
            o.write(per_sheet(out, k), forge_draw::svg::svg(p));
        }
        if let Some(out) = opt(args, "--dxf") {
            o.write(per_sheet(out, k), forge_draw::dxf::dxf(p));
        }
    }
    if let Some(view) = opt(args, "--refs") {
        match forge_lang::drawing::regen::refs(&d, view) {
            Some(j) => o.json(&j),
            None => o.err(format!("no view {view}")),
        }
    } else {
        o.json(&forge_lang::drawing::regen::json(&d, src));
    }
    o.code = if d.ok() { 0 } else { 1 };
}

fn run_file(file: &str, args: &[String], build: bool) -> ExitCode {
    let src = match read(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    if !build {
        let mut o = Output::default();
        run_src(file, &src, args, false, &mut o);
        return o.emit();
    }
    // the same file, text, arguments and everything it read: the last run's output
    let place = std::fs::canonicalize(file).map_or_else(|_| file.to_string(), |p| p.to_string_lossy().into());
    let key = forge_lang::kept::run_key(&place, &src, args);
    if let Some(o) =
        forge_lang::kept::fetch("fg-run", key).and_then(|b| Output::load(&mut forge_lang::store::In(&b)))
    {
        return o.emit();
    }
    let (o, from) = forge_lang::kept::watched(std::path::Path::new(&place), &src, || {
        let mut o = Output::default();
        run_src(file, &src, args, true, &mut o);
        o
    });
    let mut e = forge_lang::store::Out::default();
    o.save(&mut e);
    forge_lang::kept::keep("fg-run", key, &from, &e.0);
    o.emit()
}

fn run_src(file: &str, src: &str, args: &[String], build: bool, o: &mut Output) {
    if forge_lang::drawing::is_drawing(src) || opt(args, "--drawing").is_some() {
        return run_drawing(file, src, args, o);
    }
    let opts = Options {
        build,
        dir: dir_of(file),
        render: opt(args, "--render").is_some(),
        step: opt(args, "--step").map(|_| stem(file)),
        ..Options::default()
    };
    let (run, _) = forge_lang::run(src, opts, Cache::default());
    let posed = run.asm.as_ref().and_then(forge_lang::asm::compound);
    if let (Some(out), Some(body)) = (opt(args, "--render"), posed.as_ref().or(run.body.as_ref())) {
        let view = opt(args, "--view").unwrap_or("iso");
        let px = opt(args, "--px").and_then(|p| p.parse().ok()).unwrap_or(600);
        match forge_lang::render(body, view, px) {
            Ok(png) => o.write(out.to_string(), png),
            Err(e) => o.err(format!("render: {e}")),
        }
    }
    if let Some(out) = opt(args, "--dxf") {
        write_dxf(&run, out, o);
    }
    let mut wrote = true;
    if let Some(out) = opt(args, "--step") {
        wrote = write_step(&run, out, &stem(file), o);
    }
    if flag(args, "--json") {
        let mut j = report::json(&run, src);
        if let Some(out) = opt(args, "--field") {
            put_fields(&mut j, &run, out);
        }
        if let (Some(art), Some(out)) = (opt(args, "--print-art"), opt(args, "--print")) {
            put_prints(&mut j, &run, art, out);
        }
        o.json(&j);
    } else {
        o.stdout.push_str(&report::text(&run, src));
    }
    o.code = if run.ok() && wrote { 0 } else { 1 };
}

/// Each thermoform's distortion print of the artwork `art` (a PPM spanning the trim's box, seen
/// along the pull) at 0.5 mm a pixel, written to `out` (or `out` less .ppm then -ID.ppm when there
/// are several) and named in its report's `print`.
fn put_prints(j: &mut Json, run: &forge_lang::Run, art: &str, out: &str) {
    let Json::Obj(top) = j else { return };
    let Some((_, Json::Arr(ts))) = top.iter_mut().find(|f| f.0 == "thermoforms") else {
        return;
    };
    // an image's top row is the trim's far edge, seen from above
    let image = std::fs::read(art)
        .ok()
        .and_then(|b| forge_lang::thermo::Image::from_ppm(&b))
        .map(|i| i.flipped());
    let many = run.thermos.len() > 1;
    for (t, o) in ts.iter_mut().zip(&run.thermos) {
        let Json::Obj(f) = t else { continue };
        let Some(image) = &image else {
            f.push((
                "print".into(),
                Json::obj(vec![("error", format!("{art}: not a P6 PPM").into())]),
            ));
            continue;
        };
        let file = if many {
            format!("{}-{}.ppm", out.trim_end_matches(".ppm"), o.id)
        } else {
            out.to_string()
        };
        let Some(p) = o.print(image, 0.5) else {
            f.push((
                "print".into(),
                Json::obj(vec![("error", "no forming (walls: false)".into())]),
            ));
            continue;
        };
        let img = forge_lang::thermo::Image {
            width: p.width,
            height: p.height,
            rgb: p.rgb.clone(),
        }
        .flipped();
        let done = std::fs::write(&file, img.to_ppm());
        f.push((
            "print".into(),
            Json::obj(vec![
                (
                    "file",
                    match &done {
                        Ok(()) => file.as_str().into(),
                        Err(e) => format!("{file}: {e}").into(),
                    },
                ),
                ("size_px", Json::Arr(vec![p.width.into(), p.height.into()])),
                ("mm_per_px", p.pixel.into()),
                ("corner", Json::Arr(vec![p.corner.0.into(), p.corner.1.into()])),
                ("bridged_px", p.bridged.iter().filter(|b| **b).count().into()),
            ]),
        ));
    }
}

/// Each thermoform's wall at each point, in its report's `field`: inline, or as CSV at `out` (or
/// `out` less .csv then -ID.csv when there are several), the file named there.
fn put_fields(j: &mut Json, run: &forge_lang::Run, out: &str) {
    let Json::Obj(top) = j else { return };
    let Some((_, Json::Arr(ts))) = top.iter_mut().find(|f| f.0 == "thermoforms") else {
        return;
    };
    let many = run.thermos.len() > 1;
    for (t, o) in ts.iter_mut().zip(&run.thermos) {
        let Json::Obj(f) = t else { continue };
        let Some((_, Json::Obj(field))) = f.iter_mut().find(|x| x.0 == "field") else {
            continue;
        };
        field.retain(|x| x.0 != "get");
        if out == "inline" {
            field.push(("points".into(), o.field_json()));
            continue;
        }
        let file = if many {
            format!("{}-{}.csv", out.trim_end_matches(".csv"), o.id)
        } else {
            out.to_string()
        };
        match std::fs::write(&file, o.field_csv()) {
            Ok(()) => field.push(("csv".into(), file.as_str().into())),
            Err(e) => field.push(("error".into(), e.to_string().into())),
        }
    }
}

fn view_file(file: &str, args: &[String]) -> ExitCode {
    let src = match read(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let opts = Options {
        build: true,
        dir: dir_of(file),
        ..Options::default()
    };
    let (run, _) = forge_lang::run(&src, opts, Cache::default());
    let Some(model) = forge_lang::drawing::Model::of(&run) else {
        print!("{}", report::text(&run, &src));
        return ExitCode::from(1);
    };
    let names: Vec<&str> = opt(args, "--view")
        .unwrap_or("front,top,right,iso")
        .split(',')
        .collect();
    let hopts = forge_draw::Options {
        tangent: flag(args, "--tangent"),
        chord: opt(args, "--chord").and_then(|c| c.parse().ok()).unwrap_or(0.01),
    };
    let looks = match forge_lang::drawing::look::views(&model, &names, &hopts) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    print_json(&forge_lang::drawing::look::json(
        &looks,
        flag(args, "--points").then_some(hopts.chord),
    ));
    let pages = forge_lang::drawing::look::pages(&looks);
    if let Some(out) = opt(args, "--pdf")
        && let Err(e) = std::fs::write(out, forge_draw::pdf::pdf(&pages))
    {
        eprintln!("{out}: {e}");
    }
    if let Some(out) = opt(args, "--svg")
        && let Some(p) = pages.first()
        && let Err(e) = std::fs::write(out, forge_draw::svg::svg(p))
    {
        eprintln!("{out}: {e}");
    }
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("run") if args.len() >= 2 => run_file(&args[1], &args[2..], true),
        Some("check") if args.len() >= 2 => run_file(&args[1], &args[2..], false),
        Some("view") if args.len() >= 2 => view_file(&args[1], &args[2..]),
        Some("std") => std_cmd(&args[1..]),
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
            // a drawing op: on a drawing file, or inside a .fg's drawing block (`"drawing": ID`)
            let block = Json::parse(&args[2])
                .ok()
                .and_then(|j| j.get("drawing").and_then(|d| d.str().map(str::to_string)));
            if forge_lang::drawing::is_drawing(&src) || block.is_some() {
                let Some(op) = Json::parse(&args[2])
                    .ok()
                    .and_then(|j| forge_lang::drawing::ops::DrawOp::from_json(&j))
                else {
                    eprintln!("not a drawing op: {}", args[2]);
                    return ExitCode::from(2);
                };
                let done = match &block {
                    Some(id) => forge_lang::drawing::block::apply_checked(&src, id, &op),
                    None => forge_lang::drawing::ops::apply_checked(&src, &op),
                };
                return match done {
                    Ok(new) => {
                        if let Err(e) = std::fs::write(file, &new) {
                            eprintln!("{file}: {e}");
                            return ExitCode::from(2);
                        }
                        print_json(&Json::obj(vec![("ok", true.into()), ("op", op.json())]));
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        print_json(&Json::obj(vec![("ok", false.into()), ("error", e.text().into())]));
                        ExitCode::from(1)
                    }
                };
            }
            // a refactor between this file and others: the document and the files it makes
            if let Some(r) = Json::parse(&args[2])
                .ok()
                .and_then(|j| forge_lang::refactor::Refactor::from_json(&j))
            {
                let doc = std::path::Path::new(file);
                return match forge_lang::refactor::run_on_disk(Some(doc), None, &src, &r) {
                    Ok(w) => {
                        if let Err(e) = std::fs::write(file, &w.doc) {
                            eprintln!("{file}: {e}");
                            return ExitCode::from(2);
                        }
                        let files = Json::Arr(w.files.iter().map(|f| Json::Str(f.0.clone())).collect());
                        print_json(&Json::obj(vec![
                            ("ok", true.into()),
                            ("op", r.json()),
                            ("files", files),
                        ]));
                        run_file(file, &args[3..], false)
                    }
                    Err(e) => {
                        print_json(&Json::obj(vec![("ok", false.into()), ("error", e.json(&src))]));
                        ExitCode::from(1)
                    }
                };
            }
            let Some(op) = Json::parse(&args[2]).ok().and_then(|j| Op::from_json(&j)) else {
                eprintln!("not an op: {}", args[2]);
                return ExitCode::from(2);
            };
            match edit::apply_checked_in(&src, &op, dir_of(file)) {
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
                // `"checks": false`: the shape alone, as a UI shows it before the checks
                let checks = !matches!(req.get("checks"), Some(Json::Bool(false)));
                let opts = Options {
                    build: cmd != "check",
                    dir: dir_of(&file),
                    checks,
                    overlap: checks,
                    expects: checks,
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

fn std_cmd(args: &[String]) -> ExitCode {
    use forge_lang::stdparts::{Spans, Spec, check, families, family, generate};
    let fmt = forge_lang::values::fmt;
    let mut pos = vec![];
    let mut k = 0;
    while k < args.len() {
        if args[k].starts_with('-') {
            k += 2;
        } else {
            pos.push(args[k].as_str());
            k += 1;
        }
    }
    match pos[..] {
        [] => {
            for f in families() {
                println!(
                    "{:<16} {:<5} {:<56} {} sizes",
                    f.name,
                    f.kind,
                    f.title,
                    f.rows.len()
                );
            }
            ExitCode::SUCCESS
        }
        [name] => match family(name) {
            None => {
                eprintln!("no family {name:?}");
                ExitCode::from(1)
            }
            Some(f) => {
                println!(
                    "{} ({}), {}",
                    f.title,
                    f.standard,
                    if f.inch { "inches" } else { "mm" }
                );
                for r in &f.rows {
                    let ls: Vec<String> = f.lengths_of(r).iter().map(|l| fmt(*l)).collect();
                    println!("{:<10} {}", r[0], ls.join(" "));
                }
                ExitCode::SUCCESS
            }
        },
        [name, size, ..] => {
            let mut options = std::collections::BTreeMap::new();
            for o in ["thread", "grade", "sheet"] {
                if let Some(v) = opt(args, &format!("--{o}")) {
                    options.insert(o.to_string(), v.to_string());
                }
            }
            let spec = Spec {
                family: name.into(),
                size: size.into(),
                length: pos.get(2).and_then(|l| l.parse().ok()),
                options,
            };
            if let Err(e) = check(&spec, Spans::default()) {
                let e = forge_lang::stdparts::Located { err: e, id: None };
                eprintln!("{}: {}", e.kind(), e.text());
                return ExitCode::from(1);
            }
            let g = generate(&spec).expect("checked");
            match opt(args, "-o") {
                Some(out) => {
                    if let Err(e) = std::fs::write(out, &g.text) {
                        eprintln!("{out}: {e}");
                        return ExitCode::from(2);
                    }
                    println!("{} -> {out} ({})", g.designation, g.key);
                }
                None => print!("{}", g.text),
            }
            ExitCode::SUCCESS
        }
    }
}
