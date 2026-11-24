mod fg;
mod serve;
mod web;

use fg::{Args, COMMANDS};
use forge_geom::{CurveGeom, SurfaceGeom};
use forge_math::{Frame, Point2, Vec2, dmath};
use forge_mesh::{Quality, TriMesh, tessellate, to_binary_stl};
use forge_ops::{Axis2, Profile, ProfileLoop, Segment, extrude, revolve};
use forge_topo::{Body, FaceId, check, genus};
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

/// Today's date in UTC from the system clock.
fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    // civil from days (H. Hinnant's algorithm)
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

fn usage() -> ExitCode {
    eprintln!(
        "usage: forgecad [open [FILE | APP_ARGS...]]   (open the native app)\n       \
         forgecad agent list | send WINDOW MESSAGE | status WINDOW REQUEST\n       \
         forgecad update [INSTALL_ROOT]   (install latest main)\n       \
         forgecad photo F [--env E] [--out F.png] [--size WxH]   (the app's view as a PNG, no window)\n       \
         forgecad record F [--env E] [--out F.mp4|DIR] [--fps N] [--size WxH] [--seconds S] [--frames]\n       \
           (the environment's timeline, or a turntable, to MP4 by ffmpeg or PNG frames; no window)\n       \
         forgecad build|check|query|measure|render|export FILE.fg [ARGS] [--json] [--labels SEL] [--px N] [--at FEAT] [-o OUT]\n       \
         forgecad edit FILE.fg 'OP'.. [--base VERSION]   (typed ops, as a part's fixes give them; export F.fg stl|3mf|obj|urdf|dxf [SHEET])\n       \
         forgecad cam FILE.fg [sheet] [--json] [-o OUT]   (a machining document: certified toolpaths, the program or setup sheet)\n       \
         forgecad bend FILE.fg [sheet|program] [--json] [-o OUT]   (a bending document: simulated; its setup sheet, or its press-brake program where the format is openly specified)\n       \
         forgecad diff A.fg B.fg [volume]   (what B changed from A: features, params, faces, volume)\n       \
         forgecad help [forge | WORD]   (the .fg language, or its sections on WORD)\n       \
         forgecad versions F | history F [--all] | create_version F NAME [NOTES] | restore F v3|SAVE | compare F A B [--volume]\n       \
         forgecad release F REASON [--approver NAME] [--numeric] | releases F | regenerate_release F REV   (JSON; FORGECAD_AGENT names an agent)\n       \
         forgecad serve | mcp [--root DIR] [--writable]   (JSON-RPC / MCP on stdio, parts under DIR, default .; edits only if --writable)\n       \
         forgecad web DIR [--port P] [--bind ADDR] [--token T] [--app WEBDIR] [--read-only]   (the browser app on DIR's files)\n       \
         forgecad import FILE.FCStd [-o DIR] [--json]   (each body a part file DIR/<file>.<body>.fg)\n       \
         forgecad tool import FILE.json|FILE.tools [--project DIR | --user] [--json]   (a Fusion tool library into your own cam/ tools, cited to the file)\n       \
         forgecad machine SETUP.toml [--json]   (posted G-code on a machine: timeline, collisions, removal)\n       \
         forgecad machine list [--json] | new NAME (--from M | --template T) [--project] | check FILE|NAME [--json]   (your own machines)\n       \
         forgecad mesh FILE.stl|obj|3mf [--json]        (a mesh read and measured: not a B-rep)\n       \
         forgecad tool log add|list|observe|worn|chipped|broken|regrind|predict|run|calibrate ..   (physical tools: wear and life)\n       \
         forgecad tool|holder|collet|collet_nut|pull_stud|insert|turn_holder list | new ID --from ID [FIELD=VALUE..] [--project] | check ID|FILE   (your own tooling)\n       \
         forgecad vendor list | show ID | export ID [-o F] | import F | new ID [--from ID] | check F [--json]   (sheet-metal vendor profiles)\n       \
         forgecad workholding list [KIND] | new ID --from X [field=value ..] [--user] | check [ID ..] | show ID | schema [--json]   (vises, clamps, plates, chucks, collets)\n       \
         forgecad demo <out.stl> [chord-mm] | preview <out.json>\nthe language: AGENTS.md, forgecad help forge"
    );
    ExitCode::from(2)
}

/// A mounting plate: rounded outline, a slot, and two bolt holes.
fn plate() -> Profile {
    Profile {
        plane: Frame::WORLD,
        loops: vec![
            Profile::rounded_rectangle(Point2::new(-40.0, -25.0), 80.0, 50.0, 8.0, 0),
            Profile::rounded_rectangle(Point2::new(-15.0, -4.0), 30.0, 8.0, 3.999, 100),
            Profile::circle(Point2::new(-30.0, 15.0), 3.25, 200),
            Profile::circle(Point2::new(30.0, 15.0), 3.25, 201),
        ],
    }
}

const Y: Axis2 = Axis2 {
    origin: Point2::ORIGIN,
    dir: Vec2::Y,
};

/// A pin: cylinder shank, hemispherical head.
fn pin() -> Profile {
    let q = dmath::FRAC_PI_2;
    Profile {
        plane: Frame::WORLD,
        loops: vec![ProfileLoop::new(vec![
            (
                Segment::Line {
                    a: Point2::ORIGIN,
                    b: Point2::new(5.0, 0.0),
                },
                0,
            ),
            (
                Segment::Line {
                    a: Point2::new(5.0, 0.0),
                    b: Point2::new(5.0, 20.0),
                },
                1,
            ),
            (
                Segment::Arc {
                    center: Point2::new(0.0, 20.0),
                    radius: 5.0,
                    start: 0.0,
                    sweep: q,
                },
                2,
            ),
            (
                Segment::Line {
                    a: Point2::new(0.0, 25.0),
                    b: Point2::ORIGIN,
                },
                3,
            ),
        ])],
    }
}

fn ring() -> Profile {
    Profile {
        plane: Frame::WORLD,
        loops: vec![Profile::rounded_rectangle(
            Point2::new(10.0, 0.0),
            8.0,
            12.0,
            2.0,
            0,
        )],
    }
}

fn holed_ring() -> Profile {
    Profile {
        plane: Frame::WORLD,
        loops: vec![
            Profile::rounded_rectangle(Point2::new(8.0, 0.0), 14.0, 10.0, 3.0, 0),
            Profile::circle(Point2::new(15.0, 5.0), 2.5, 20),
        ],
    }
}

fn open_app(args: &[String]) -> ExitCode {
    run_installed("forge-app", args)
}

/// `forgecad photo|record F ..`: forge-app's headless Capture (`--photo`, `--record`), the output
/// `<stem>-<env or view>.png|mp4` here unless `--out` names it (research/render/record.md).
fn capture(cmd: &str, args: &[String]) -> ExitCode {
    let mut file = None;
    let (mut env, mut out, mut pass) = (None, None, Vec::new());
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--frames" if cmd == "record" => pass.push(a.clone()),
            f if !f.starts_with("--") && file.is_none() => file = Some(f.to_string()),
            "--env" | "--out" | "--size" | "--fps" | "--seconds" => {
                let Some(v) = it.next().cloned() else {
                    return usage();
                };
                match a.as_str() {
                    "--env" => env = Some(v),
                    "--out" => out = Some(v),
                    "--size" => pass.extend(["--out-size".to_string(), v]),
                    _ if cmd == "record" => pass.extend([a.clone(), v]),
                    _ => return usage(),
                }
            }
            _ => return usage(),
        }
    }
    let Some(file) = file else { return usage() };
    let stem = std::path::Path::new(&file)
        .file_stem()
        .map_or("forgecad".into(), |s| s.to_string_lossy().into_owned());
    let ext = if cmd == "photo" { "png" } else { "mp4" };
    let out = out.unwrap_or_else(|| format!("{stem}-{}.{ext}", env.as_deref().unwrap_or("view")));
    let mut app = vec![file];
    if let Some(e) = env {
        app.extend(["--env".into(), e]);
    }
    app.extend([format!("--{cmd}"), out]);
    app.extend(pass);
    run_installed("forge-app", &app)
}

/// `forgecad update`: the updater installed beside this program (a shell script, or on
/// Windows a PowerShell one).
fn update(args: &[String]) -> ExitCode {
    if !cfg!(windows) {
        return run_installed("forgecad-update", args);
    }
    let Ok(exe) = std::env::current_exe() else {
        eprintln!("forgecad: cannot locate the installed updater");
        return ExitCode::FAILURE;
    };
    let script = exe.with_file_name("forgecad-update.ps1");
    let status = std::process::Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script)
        .args(args)
        .status();
    match status {
        Ok(s) => ExitCode::from(s.code().unwrap_or(1) as u8),
        Err(e) => {
            eprintln!(
                "forgecad: cannot run {}: {e}\nInstall with tools\\install.ps1.",
                script.display()
            );
            ExitCode::FAILURE
        }
    }
}

fn run_installed(name: &str, args: &[String]) -> ExitCode {
    // Installed together, so opening the app never depends on the working directory.
    let app = match std::env::current_exe() {
        Ok(exe) => exe.with_file_name(format!("{name}{}", std::env::consts::EXE_SUFFIX)),
        Err(e) => {
            eprintln!("forgecad: cannot locate the installed app: {e}");
            return ExitCode::FAILURE;
        }
    };
    match std::process::Command::new(&app).args(args).status() {
        Ok(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(e) => {
            eprintln!(
                "forgecad: cannot open {}: {e}\nInstall both binaries with tools/install.sh.",
                app.display()
            );
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        None => open_app(&[]),
        Some("open") => open_app(&args[2..]),
        Some("agent") => agent_bridge(&args[2..]),
        Some("update") => update(&args[2..]),
        Some(c @ ("photo" | "record")) => capture(c, &args[2..]),
        Some(
            t @ ("versions" | "history" | "create_version" | "restore" | "compare" | "release" | "releases"
            | "regenerate_release"),
        ) => {
            let agent = std::env::var("FORGECAD_AGENT").ok();
            match forge_lang::vtools::call(t, &args[2..], agent.as_deref()) {
                Some(Ok(j)) => {
                    println!("{j}");
                    ExitCode::SUCCESS
                }
                Some(Err(e)) => {
                    let j = forge_agent::json::Json::obj(vec![
                        ("ok", false.into()),
                        ("error", forge_agent::json::Json::Str(e)),
                    ]);
                    println!("{j}");
                    ExitCode::from(1)
                }
                None => usage(),
            }
        }
        Some("demo") if args.len() >= 3 => demo(&args),
        Some("preview") if args.len() >= 3 => preview(&args[2]),
        Some(m @ ("serve" | "mcp")) => {
            let writable = args[2..].iter().any(|a| a == "--writable");
            let rest: Vec<&str> = args[2..]
                .iter()
                .map(String::as_str)
                .filter(|a| *a != "--writable")
                .collect();
            match rest[..] {
                [] => serve::run(m == "mcp", ".", writable),
                ["--root", root] => serve::run(m == "mcp", root, writable),
                _ => usage(),
            }
        }
        Some("web") => web::main(&args[2..]),
        Some("import") if args.len() >= 3 => import(&args[2..]),
        Some("machine") if args.len() >= 3 => machine(&args[2..]),
        Some("tool") if args.len() >= 4 && args[2] == "import" => tool_import(&args[3..]),
        Some("mesh") if args.len() >= 3 => mesh(&args[2..]),
        Some("tool") if args.get(2).is_some_and(|w| w == "log") => {
            let env = forge_cam_lang::toollog::Env {
                project: std::path::PathBuf::from("."),
                today: today(),
            };
            let user = forge_dirs::config().map(|c| c.join("forgecad").join("cam"));
            let mut store = forge_cam_lang::toolstore::store(
                &env.project,
                user.as_deref(),
                args.iter().any(|a| a == "--project"),
            );
            let o = forge_cam_lang::toollog::run(&env, &mut store, &args[3..]);
            let _ = write!(std::io::stdout(), "{}", o.out);
            if !o.err.is_empty() {
                eprintln!("{}", o.err.trim_end());
            }
            ExitCode::from(o.code)
        }
        Some(w @ ("tool" | "holder" | "collet" | "collet_nut" | "pull_stud" | "insert" | "turn_holder"))
            if args.len() >= 3 =>
        {
            let env = forge_tooling_builder::cli::Env {
                project: std::path::PathBuf::from("."),
                user_cam: forge_tooling_builder::user_cam(),
            };
            let o = forge_tooling_builder::cli::run(&env, w, &args[2..]).expect("a tooling kind");
            let _ = write!(std::io::stdout(), "{}", o.out);
            if !o.err.is_empty() {
                eprintln!("{}", o.err.trim_end());
            }
            ExitCode::from(o.code)
        }
        Some("vendor") => {
            let json = args.iter().any(|a| a == "--json");
            let rest: Vec<String> = args[2..].iter().filter(|a| *a != "--json").cloned().collect();
            let here = std::env::current_dir().ok();
            match forge_lang::vendor::tool(&rest, here.as_deref()) {
                Ok((j, text)) => {
                    if json {
                        println!("{j}");
                    } else {
                        print!("{text}");
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::from(1)
                }
            }
        }
        Some("workholding") => {
            let here = std::env::current_dir().unwrap_or_else(|_| ".".into());
            let o = forge_workholding_builder::cli::run(&args[2..], &here, None);
            print!("{}", o.text);
            if !o.text.ends_with('\n') {
                println!();
            }
            ExitCode::from(o.code)
        }
        Some("help" | "--help" | "-h") => {
            println!("{}", forge_lang::help(&args[2..].join(" ")));
            ExitCode::SUCCESS
        }
        Some(c) if COMMANDS.contains(&c) && args.len() >= 3 => command(c, &args[2..]),
        _ => usage(),
    }
}

fn quoted(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o += "\\\"",
            '\\' => o += "\\\\",
            c if (c as u32) < 32 => o += &format!("\\u{:04x}", c as u32),
            c => o.push(c),
        }
    }
    o + "\""
}

/// A mesh file read and measured: triangles, whether it is closed, its volume, area and box. A
/// mesh is for reference: nothing is built on it.
fn mesh(argv: &[String]) -> ExitCode {
    let json = argv.iter().any(|a| a == "--json");
    let Some(file) = argv.iter().find(|a| !a.starts_with("--")) else {
        return usage();
    };
    let ext = std::path::Path::new(file)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let read = std::fs::read(file).map_err(|e| e.to_string()).and_then(|d| {
        match ext.as_str() {
            "stl" => forge_mesh::read::stl(&d),
            "obj" => forge_mesh::read::obj(&d),
            "3mf" => forge_import::threemf::read(&d),
            _ => return Err(format!("{ext:?}: a mesh is .stl, .obj or .3mf")),
        }
        .map_err(|e| e.to_string())
    });
    let m = match read {
        Ok(m) => m,
        Err(e) => {
            if json {
                println!("{{\"file\":{},\"error\":{}}}", quoted(file), quoted(&e));
            } else {
                eprintln!("{file}: {e}");
            }
            return ExitCode::FAILURE;
        }
    };
    let s = forge_mesh::read::summary(&m);
    let num = |x: f64| {
        if x.is_finite() {
            format!("{x}")
        } else {
            "null".into()
        }
    };
    if json {
        println!(
            "{{\"file\":{},\"kind\":\"mesh\",\"brep\":false,\"triangles\":{},\"vertices\":{},\"closed\":{},\"open_edges\":{},\"volume\":{},\"area\":{},\"box\":[{},{},{},{},{},{}]}}",
            quoted(file),
            s.triangles,
            s.vertices,
            s.closed,
            s.open_edges,
            s.volume.map_or("null".into(), num),
            num(s.area),
            num(s.bbox.0[0]),
            num(s.bbox.0[1]),
            num(s.bbox.0[2]),
            num(s.bbox.1[0]),
            num(s.bbox.1[1]),
            num(s.bbox.1[2])
        );
    } else {
        println!(
            "{file}: a mesh (triangles, not a B-rep: for reference and measurement), {} triangles, {} vertices",
            s.triangles, s.vertices
        );
        match s.volume {
            Some(v) => println!("  closed; volume {v:.6} mm3"),
            None => println!(
                "  open ({} edges not shared by two triangles): no volume",
                s.open_edges
            ),
        }
        println!("  area {:.6} mm2", s.area);
        println!(
            "  box ({:.4}, {:.4}, {:.4}) to ({:.4}, {:.4}, {:.4}) mm",
            s.bbox.0[0], s.bbox.0[1], s.bbox.0[2], s.bbox.1[0], s.bbox.1[1], s.bbox.1[2]
        );
    }
    ExitCode::SUCCESS
}

/// `forgecad tool import FILE`: a Fusion tool library (the JSON, or a `.tools` archive holding
/// it) imported into the user's own library: the project's `cam/` (default: the current
/// folder's) or the user level (`--user`: `<config>/forgecad/cam/tools`). Every value is cited to
/// the file and its library version (`user` cites), presets kept where the file names a material.
/// Never written into forgecad's own data.
fn tool_import(argv: &[String]) -> ExitCode {
    let (mut project, mut user, mut json, mut file) = (".".to_string(), false, false, None);
    let mut it = argv.iter();
    while let Some(w) = it.next() {
        match w.as_str() {
            "--project" => project = it.next().cloned().unwrap_or(project),
            "--user" => user = true,
            "--json" => json = true,
            _ => file = Some(w.clone()),
        }
    }
    let Some(file) = file else { return usage() };
    let path = std::path::Path::new(&file);
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("tools")
        .to_string();
    let bytes = match forge_dirs::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{file}: {e}");
            return ExitCode::FAILURE;
        }
    };
    // a `.tools` file is a zip holding tools.json
    let text = if bytes.starts_with(b"PK") {
        match forge_import::zip::Zip::new(&bytes)
            .map_err(|e| e.to_string())
            .and_then(|z| {
                z.read("tools.json")
                    .ok_or_else(|| "no tools.json in the archive".to_string())
            }) {
            Ok(b) => String::from_utf8_lossy(&b).into_owned(),
            Err(e) => {
                eprintln!("{file}: {e}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        String::from_utf8_lossy(&bytes).into_owned()
    };
    let today = forge_machine_builder::cli::today_utc();
    let imp = match forge_cam_import::fusion_with(
        &text,
        &name,
        &today,
        forge_cam_import::Opts {
            cite: true,
            presets: true,
            assemblies: false,
        },
    ) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("{file}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let dir = if user {
        match forge_dirs::config() {
            Some(c) => c.join("forgecad").join("cam").join("tools"),
            None => {
                eprintln!("no user configuration folder on this system: use --project DIR");
                return ExitCode::FAILURE;
            }
        }
    } else {
        std::path::Path::new(&project).join("cam")
    };
    if dir.components().any(|c| c.as_os_str() == "forge-tooling") {
        eprintln!("refused: {} is forgecad's own library, not yours", dir.display());
        return ExitCode::FAILURE;
    }
    let stem: String = name
        .split('.')
        .next()
        .unwrap_or("tools")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let out = dir.join(format!("{stem}.tools.toml"));
    if let Err(e) = forge_dirs::fs::create_dir_all(&dir).and_then(|_| forge_dirs::fs::write(&out, &imp.toml))
    {
        eprintln!("{}: {e}", out.display());
        return ExitCode::FAILURE;
    }
    if json {
        let j = forge_agent::json::Json::obj(vec![
            ("ok", true.into()),
            ("file", forge_agent::json::Json::Str(out.display().to_string())),
            ("tools", forge_agent::json::Json::Num(imp.tools as f64)),
            (
                "warnings",
                forge_agent::json::Json::Num(imp.warnings.len() as f64),
            ),
        ]);
        println!("{j}");
    } else {
        println!("{} tools from {name} into {}", imp.tools, out.display());
        for w in imp.warnings.iter().take(20) {
            println!("  warning: {w}");
        }
        if imp.warnings.len() > 20 {
            println!("  ... {} more warnings", imp.warnings.len() - 20);
        }
    }
    ExitCode::SUCCESS
}

/// Posted G-code simulated on a machine (forge-machine): `list` the machines, or run a setup file.
fn machine(argv: &[String]) -> ExitCode {
    let envelopes = |def: &forge_machine_lang::MachineDefRef, dir: &std::path::Path| {
        forge_machine_lang::envelope_checks(def, dir)
    };
    let env = forge_machine_builder::cli::Env {
        project: std::path::PathBuf::from("."),
        user: forge_machine_builder::catalog::user_dir(),
        today: forge_machine_builder::cli::today_utc(),
        envelopes: &envelopes,
    };
    if let Some(o) = forge_machine_builder::cli::run(&env, argv) {
        let _ = write!(std::io::stdout(), "{}", o.out);
        if !o.err.is_empty() {
            eprintln!("{}", o.err.trim_end());
        }
        return ExitCode::from(o.code);
    }
    if argv[0] == "list" {
        let lib = match forge_machine_lang::library(std::path::Path::new(".")) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::FAILURE;
            }
        };
        for (m, file) in &lib.machines {
            println!("{} {} ({}) {}", m.name, m.kind.name(), m.control, file);
        }
        return ExitCode::SUCCESS;
    }
    let json = argv.iter().any(|a| a == "--json");
    let Some(file) = argv.iter().find(|a| !a.starts_with("--")) else {
        return usage();
    };
    match forge_machine_lang::run_setup(std::path::Path::new(file)) {
        Ok(o) => {
            let _ = writeln!(std::io::stdout(), "{}", if json { o.json } else { o.text });
            if o.ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

/// Another system's file as part files, one a body: a line each says what became of it.
fn import(argv: &[String]) -> ExitCode {
    let (mut dir, mut json, mut file) = (".".to_string(), false, None);
    let mut it = argv.iter();
    while let Some(w) = it.next() {
        match w.as_str() {
            "-o" => dir = it.next().cloned().unwrap_or(dir),
            "--json" => json = true,
            _ => file = Some(w.clone()),
        }
    }
    let Some(file) = file else { return usage() };
    let path = std::path::Path::new(&file);
    if path.extension().is_some_and(|x| x.eq_ignore_ascii_case("urdf")) {
        return import_urdf(path, Path::new(&dir), json);
    }
    let stem = path
        .file_name()
        .and_then(|s| s.to_str())
        .and_then(|s| s.split('.').next())
        .unwrap_or("part")
        .to_string();
    let bodies = match std::fs::read(path)
        .map_err(|e| e.to_string())
        .and_then(|d| forge_import::fcstd::translate(&d))
    {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{file}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let names = forge_import::fcstd::part_files(&stem, &bodies);
    for (b, name) in bodies.iter().zip(&names) {
        // names are [A-Za-z0-9_-] and dots: nothing a file holds can write outside DIR
        let out = std::path::Path::new(&dir).join(name);
        if out.parent() != Some(std::path::Path::new(&dir)) {
            eprintln!("{}: not in {dir}", out.display());
            return ExitCode::FAILURE;
        }
        if let Some(fg) = &b.fg
            && let Err(e) = std::fs::write(&out, fg)
        {
            eprintln!("{}: {e}", out.display());
            return ExitCode::FAILURE;
        }
        let why = b.fail.as_ref().map_or("", |f| f.why());
        if json {
            println!(
                "{{\"file\":{},\"body\":{},\"status\":\"{}\",\"why\":{},\"approx\":{}}}",
                quoted(&stem),
                quoted(&b.name),
                b.status(),
                quoted(why),
                b.approx
            );
        } else if b.fg.is_some() {
            println!(
                "{}: {} {}{}",
                b.name,
                b.status(),
                out.display(),
                if why.is_empty() {
                    String::new()
                } else {
                    format!(" ({why})")
                }
            );
        } else {
            println!("{}: {} ({why})", b.name, b.status());
        }
    }
    ExitCode::SUCCESS
}

fn command(cmd: &str, argv: &[String]) -> ExitCode {
    let mut a = Args {
        file: argv[0].clone(),
        ..Args::default()
    };
    let (mut json, mut out, mut allow_uncertified) = (false, None, false);
    let mut it = argv[1..].iter();
    while let Some(w) = it.next() {
        match w.as_str() {
            "--json" => json = true,
            "--labels" => a.labels = it.next().cloned(),
            "--px" => a.px = it.next().and_then(|p| p.parse().ok()),
            "--at" => a.at = it.next().cloned(),
            "--base" => a.base = it.next().cloned(),
            // `query F bendsim`'s: carried as its words
            flag @ ("--vendor" | "--tooling" | "--sequence" | "--sheet" | "--uts") => {
                a.words.push(flag.trim_start_matches("--").to_string());
                if let Some(v) = it.next() {
                    a.words.push(v.clone());
                }
            }
            // `export F urdf`'s opt-in: 0 for unknown effort and velocity, marked as placeholders
            "--placeholder-limits" => a.words.push("placeholder-limits".into()),
            "-o" => out = it.next().cloned(),
            "--allow-uncertified" => allow_uncertified = true,
            w if w.starts_with("--") => {
                eprintln!(
                    "unknown option {w} (--json --labels --px --at --base -o; bendsim: --vendor --tooling --sequence --sheet --uts; urdf: --placeholder-limits; cam post: --allow-uncertified)"
                );
                return ExitCode::from(2);
            }
            _ => a.words.push(w.clone()),
        }
    }
    // `cam F post [PROGRAM] [-o FILE|DIR] [--allow-uncertified]`: the NC files, as the app exports them
    if cmd == "cam" && a.words.first().is_some_and(|w| w == "post") {
        return cam_post(&a, out.as_deref(), allow_uncertified);
    }
    let r = fg::Session::new(None, true).run(cmd, &a, &forge_agent::json::Json::Null);
    let mut text = if json { r.json.to_string() } else { r.text };
    if let Some(png) = &r.png {
        let ext = match cmd {
            "export" if a.words.first().is_some_and(|w| w == "dxf") => "dxf",
            "export" if a.words.first().is_some_and(|w| w == "dwg") => "dwg",
            "export" if a.words.first().is_some_and(|w| w == "stl") => "stl",
            "export" if a.words.first().is_some_and(|w| w == "3mf") => "3mf",
            "export" if a.words.first().is_some_and(|w| w == "obj") => "obj",
            "export" if a.words.first().is_some_and(|w| w == "urdf") => "zip",
            "export" => "step",
            "cam" if a.words.first().is_some_and(|w| w == "sheet") => "html",
            "cam" => "nc",
            "bend" if a.words.first().is_some_and(|w| w == "program") => "pgm",
            "bend" => "html",
            _ => "png",
        };
        let stem = a.file.trim_end_matches(".fg");
        // a URDF is a directory (the robot and its meshes), or the ZIP as it is with `-o X.zip`
        if cmd == "export"
            && a.words.first().is_some_and(|w| w == "urdf")
            && !out.as_deref().is_some_and(|o| o.ends_with(".zip"))
        {
            let dir = out.unwrap_or_else(|| format!("{stem}_urdf"));
            if let Err(e) = unpack(png, Path::new(&dir)) {
                eprintln!("writing {dir}: {e}");
                return ExitCode::FAILURE;
            }
            if !json {
                text = format!("wrote {dir}/ {text}");
            }
            let _ = writeln!(std::io::stdout(), "{text}");
            return ExitCode::SUCCESS;
        }
        let path = out.unwrap_or_else(|| format!("{stem}.{ext}"));
        if let Err(e) = std::fs::write(&path, png) {
            eprintln!("writing {path}: {e}");
            return ExitCode::FAILURE;
        }
        if !json {
            text = format!("wrote {path} {text}");
        }
    }
    // a closed pipe (`| head`) is not an error worth a panic
    let _ = writeln!(std::io::stdout(), "{text}");
    // cam: 0 fully certified, 3 released but certified except a check (G5: a setup not placed;
    // G4: the leftover not computed), 1 refused
    if cmd == "cam"
        && r.ok
        && let forge_agent::json::Json::Obj(fs) = &r.json
        && fs.iter().any(|(k, v)| {
            k == "certification" && matches!(v, forge_agent::json::Json::Str(c) if c.contains("except"))
        })
    {
        return ExitCode::from(3);
    }
    if r.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// `import ROBOT.urdf [-o DIR]`: the URDF as an unlinked robot document, DIR/ROBOT_robot.fg, its
/// meshes referenced where they are.
fn import_urdf(path: &Path, dir: &Path, json: bool) -> ExitCode {
    let stem = path
        .file_stem()
        .map_or_else(|| "robot".into(), |s| s.to_string_lossy().into_owned());
    let out = dir.join(format!("{stem}_robot.fg"));
    let urdf_dir = path
        .parent()
        .map(|p| {
            std::fs::canonicalize(if p.as_os_str().is_empty() {
                Path::new(".")
            } else {
                p
            })
        })
        .and_then(Result::ok)
        .unwrap_or_default();
    let _ = std::fs::create_dir_all(dir);
    let doc_dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let r = std::fs::read_to_string(path)
        .map_err(|e| e.to_string())
        .and_then(|x| forge_urdf::import::import(&x, &urdf_dir, &doc_dir));
    let i = match r {
        Ok(i) => i,
        Err(e) => {
            eprintln!("{}: {e}", path.display());
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = std::fs::write(&out, &i.text) {
        eprintln!("{}: {e}", out.display());
        return ExitCode::FAILURE;
    }
    if json {
        println!(
            "{}",
            forge_agent::json::Json::obj(vec![
                ("ok", true.into()),
                ("file", out.to_string_lossy().as_ref().into()),
                ("links", i.links.into()),
                ("joints", i.joints.into()),
                (
                    "dropped",
                    forge_agent::json::Json::Arr(i.dropped.iter().map(|d| d.as_str().into()).collect()),
                ),
            ])
        );
    } else {
        println!("wrote {}: {} links, {} joints", out.display(), i.links, i.joints);
        for d in &i.dropped {
            println!("not imported: {d}");
        }
    }
    ExitCode::SUCCESS
}

/// `cam F post`: each program's file written where its statement says (beside the document), or
/// under `-o DIR`, or to `-o FILE` when there is one; refused, nothing written, if any is refused.
fn cam_post(a: &fg::Args, out: Option<&str>, allow_uncertified: bool) -> ExitCode {
    let doc = std::path::Path::new(&a.file);
    let files = match fg::Session::cam_post(doc, a.words.get(1).map(String::as_str), allow_uncertified) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("refused: {e}");
            return ExitCode::FAILURE;
        }
    };
    let base = doc.parent().map(std::path::Path::to_path_buf).unwrap_or_default();
    for f in &files {
        let path = match out {
            // one file and a name that is not a folder: that name
            Some(o) if files.len() == 1 && !o.ends_with('/') && !std::path::Path::new(o).is_dir() => {
                std::path::PathBuf::from(o)
            }
            Some(o) => std::path::Path::new(o).join(f.path.rsplit('/').next().unwrap_or(&f.path)),
            None => base.join(&f.path),
        };
        if let Some(d) = path.parent()
            && !d.as_os_str().is_empty()
            && let Err(e) = std::fs::create_dir_all(d)
        {
            eprintln!("{}: {e}", d.display());
            return ExitCode::FAILURE;
        }
        if let Err(e) = std::fs::write(&path, &f.text) {
            eprintln!("writing {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
        println!("wrote {} ({} lines)", path.display(), f.text.lines().count());
    }
    ExitCode::SUCCESS
}

/// A URDF export's ZIP written out under `dir`.
fn unpack(zip: &[u8], dir: &Path) -> Result<(), String> {
    let members = forge_mesh::zip::read(zip, 1 << 31).map_err(|e| format!("{e:?}"))?;
    for (name, bytes) in members {
        // only names the exporter writes: no absolute path, no `..`
        let rel = Path::new(&name);
        if rel.is_absolute()
            || rel
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err(format!("refusing member {name}"));
        }
        let p = dir.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&p, bytes).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn demo(args: &[String]) -> ExitCode {
    let chord = match args.get(3).map(|s| s.parse::<f64>()) {
        None => Quality::default().chord,
        Some(Ok(c)) if c > 0.0 => c,
        Some(_) => return usage(),
    };
    let body = extrude(&plate(), 6.0, 1).expect("demo profile is valid");
    let problems = check(&body);
    let mesh = match tessellate(
        &body,
        &Quality {
            chord,
            ..Quality::default()
        },
    ) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("tessellation failed: {e:?}");
            return ExitCode::FAILURE;
        }
    };
    println!(
        "faces {}  edges {}  vertices {}  genus {}  problems {}",
        body.faces.len(),
        body.edges.len(),
        body.vertices.len(),
        genus(&body),
        problems.len()
    );
    println!(
        "triangles {}  volume {:.4} mm^3  area {:.4} mm^2  open edges {}",
        mesh.triangles.len(),
        mesh.volume(),
        mesh.area(),
        mesh.open_edges().len()
    );
    if let Err(e) = std::fs::write(&args[2], to_binary_stl(&mesh, "ForgeCAD demo")) {
        eprintln!("writing {}: {e}", args[2]);
        return ExitCode::FAILURE;
    }
    println!("wrote {}", args[2]);
    if problems.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

pub(crate) fn b64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for k in 0..4 {
            if k <= c.len() {
                out.push(T[(n >> (18 - 6 * k) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Coordinates as 16-bit steps across `bbox` (lo xyz, hi xyz), little endian.
fn q16(v: &[f32], bbox: &[f32; 6]) -> String {
    let bytes: Vec<u8> = v
        .iter()
        .enumerate()
        .flat_map(|(i, &x)| {
            let (lo, hi) = (bbox[i % 3], bbox[3 + i % 3]);
            let t = if hi > lo { (x - lo) / (hi - lo) } else { 0.0 };
            ((t.clamp(0.0, 1.0) * 65535.0).round() as u16).to_le_bytes()
        })
        .collect();
    b64(&bytes)
}

/// Unit vectors as signed bytes.
fn q8(v: &[f32]) -> String {
    b64(&v
        .iter()
        .map(|&x| (x.clamp(-1.0, 1.0) * 127.0).round() as i8 as u8)
        .collect::<Vec<u8>>())
}

fn bbox(sets: &[&[f32]]) -> [f32; 6] {
    let mut b = [
        f32::INFINITY,
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    ];
    for v in sets {
        for (i, &x) in v.iter().enumerate() {
            b[i % 3] = b[i % 3].min(x);
            b[3 + i % 3] = b[3 + i % 3].max(x);
        }
    }
    b
}

/// Triangles split per face so each face carries its exact surface normals.
fn shaded(body: &Body, m: &TriMesh) -> (Vec<f32>, Vec<f32>) {
    let (mut pos, mut nrm) = (Vec::new(), Vec::new());
    for (t, &f) in m.triangles.iter().zip(&m.face_of) {
        let face = body.face(FaceId(f));
        let flat = m.triangle_normal(*t).normalize().map(|u| u.get());
        for &i in t {
            let p = m.positions[i as usize];
            let n = face
                .surface
                .normal(face.surface.project(p))
                .map(|n| if face.same_sense { n.get() } else { -n.get() })
                .or(flat)
                .unwrap_or_default();
            pos.extend([p.x as f32, p.y as f32, p.z as f32]);
            nrm.extend([n.x as f32, n.y as f32, n.z as f32]);
        }
    }
    (pos, nrm)
}

fn edges(body: &Body) -> Vec<f32> {
    let mut out = Vec::new();
    for e in body.edge_ids() {
        let edge = body.edge(e);
        let n = if matches!(edge.curve, forge_geom::Curve::Line(_)) {
            1
        } else {
            12
        };
        for i in 0..n {
            for k in [i, i + 1] {
                let t = edge.range.lo + edge.range.width() * (k as f64 / n as f64);
                let p = edge.curve.eval(t.min(edge.range.hi));
                out.extend([p.x as f32, p.y as f32, p.z as f32]);
            }
        }
    }
    out
}

/// A part for the preview page: the solid, how long building it took, and
/// whether its history regenerates.
struct Shown {
    name: String,
    how: String,
    body: Body,
    build_us: f64,
    features: usize,
    status: String,
}

fn simple(name: &str, how: &str, build: impl Fn() -> Body) -> Shown {
    let reps = 200;
    let t0 = Instant::now();
    for _ in 0..reps {
        std::hint::black_box(build());
    }
    let build_us = t0.elapsed().as_secs_f64() * 1e6 / reps as f64;
    Shown {
        name: name.into(),
        how: how.into(),
        body: build(),
        build_us,
        features: 1,
        status: "ok".into(),
    }
}

/// Regenerates a history from scratch (best of three). If it fails, shows the
/// solid up to the last feature that worked and says where it stops.
fn history(name: &str, how: &str, part: &forge_model::Part) -> Shown {
    let n = part.features.len();
    let prefix = |k: usize| forge_model::Part {
        features: part.features[..k].to_vec(),
    };
    let (k, status) = match part.regenerate() {
        Ok(_) => (n, "ok".to_string()),
        Err(e) => {
            let (mut lo, mut hi) = (1, n);
            while lo < hi {
                let mid = (lo + hi) / 2;
                if prefix(mid).regenerate().is_ok() {
                    lo = mid + 1
                } else {
                    hi = mid
                }
            }
            (lo - 1, format!("stops at feature {}: {e:?}", lo - 1))
        }
    };
    let shown = prefix(k.max(1));
    let mut best = f64::INFINITY;
    let mut body = None;
    for _ in 0..3 {
        let t = Instant::now();
        let b = shown.regenerate().expect("prefix regenerates");
        best = best.min(t.elapsed().as_secs_f64() * 1e6);
        body = Some(b);
    }
    Shown {
        name: name.into(),
        how: how.into(),
        body: body.expect("ran"),
        build_us: best,
        features: k,
        status,
    }
}

fn preview(path: &str) -> ExitCode {
    let mut parts: Vec<Shown> = vec![
        simple(
            "Mounting plate",
            "Extrude 6 mm: rounded outline, slot, two bolt holes",
            || extrude(&plate(), 6.0, 1).unwrap(),
        ),
        simple("Pin", "Revolve 360°: cylinder shank, hemisphere head", || {
            revolve(&pin(), Y, dmath::TAU, 1).unwrap()
        }),
        simple("Ring", "Revolve 360°: rounded rectangle, four tori", || {
            revolve(&ring(), Y, dmath::TAU, 1).unwrap()
        }),
        simple(
            "Partial ring",
            "Revolve 270°: rounded section with a hole, planar end caps",
            || revolve(&holed_ring(), Y, 1.5 * dmath::PI, 1).unwrap(),
        ),
    ];
    let blurb = |n: &str| match n {
        "plate" => "Torture plate: holes, pockets, bosses on one plane, rotated slots, dimples, counterbores",
        "shaft" => "Revolved stepped shaft: grooves, cross holes, flats, keyways, radial pin holes",
        "housing" => "Hollow box: inside bosses, screw holes, ribs, wall windows",
        "coplanar" => "Features that share planes and faces on purpose",
        "patterns" => "Arrays of holes, bosses and rotated slots that overlap",
        "mixed" => "Domes, spherical cuts, revolved rings and cones with extrudes",
        _ => "",
    };
    for (name, part) in forge_model::suite::parts() {
        parts.push(history(&format!("Suite: {name}"), blurb(name), &part));
    }
    for seed in [1u64, 12, 15, 16, 2, 19] {
        parts.push(history(
            &format!("Fuzz seed {seed}"),
            "Random history: blocks, rods, rounded slots, balls, rings, tilted planes",
            &forge_model::suite::fuzz(seed, 80),
        ));
    }
    let mut json = String::from("{\"parts\":[");
    for (k, p) in parts.iter().enumerate() {
        let big = p.body.faces.len() > 50;
        let q = if big {
            Quality {
                chord: 0.25,
                max_angle: 0.7,
            }
        } else {
            Quality::default()
        };
        let body = p.body.compact();
        let mut status = p.status.replace('"', "'");
        let (mesh, mesh_us) = match tessellate(&body, &q) {
            Ok(m) => {
                let reps = if big { 3 } else { 20 };
                let t1 = Instant::now();
                for _ in 0..reps {
                    std::hint::black_box(tessellate(&body, &q).ok());
                }
                (m, t1.elapsed().as_secs_f64() * 1e6 / reps as f64)
            }
            Err(e) => {
                status = format!("{status}; mesh failed: {e:?}").replace('"', "'");
                (forge_mesh::TriMesh::default(), 0.0)
            }
        };
        let (pos, nrm) = shaded(&body, &mesh);
        let lines = edges(&body);
        let bb = bbox(&[&pos, &lines]);
        if k > 0 {
            json.push(',');
        }
        json.push_str(&format!(
            "{{\"name\":\"{}\",\"how\":\"{}\",\"status\":\"{}\",\"features\":{},\"faces\":{},\"edges\":{},\"vertices\":{},\"genus\":{},\"problems\":{},\
             \"triangles\":{},\"open\":{},\"volume\":{:.3},\"build_us\":{:.1},\"mesh_us\":{:.1},\
             \"bbox\":{:?},\"pos\":\"{}\",\"nrm\":\"{}\",\"lines\":\"{}\"}}",
            p.name,
            p.how,
            status,
            p.features,
            body.faces.len(),
            body.edges.len(),
            body.vertices.len(),
            genus(&body),
            check(&body).len(),
            mesh.triangles.len(),
            mesh.open_edges().len(),
            mesh.volume(),
            p.build_us,
            mesh_us,
            bb,
            q16(&pos, &bb),
            q8(&nrm),
            q16(&lines, &bb),
        ));
    }
    json.push_str("]}");
    if let Err(e) = std::fs::write(path, json) {
        eprintln!("writing {path}: {e}");
        return ExitCode::FAILURE;
    }
    println!("wrote {path}");
    ExitCode::SUCCESS
}

fn agent_bridge(args: &[String]) -> ExitCode {
    use forge_agent::{chat_bridge, json::Json};
    let result = match args.first().map(String::as_str) {
        Some("list") if args.len() == 1 => chat_bridge::list(),
        Some("send") if args.len() == 3 => chat_bridge::call(
            &args[1],
            Json::obj(vec![
                ("method", "send".into()),
                ("message", args[2].clone().into()),
            ]),
        ),
        Some("status") if args.len() == 3 => chat_bridge::call(
            &args[1],
            Json::obj(vec![
                ("method", "status".into()),
                ("request", args[2].clone().into()),
            ]),
        ),
        _ => Err("usage: forgecad agent list | send WINDOW MESSAGE | status WINDOW REQUEST".into()),
    };
    match result {
        Ok(j) => {
            println!("{j}");
            if j.get("error").is_some() {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}
