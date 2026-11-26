//! One long-lived process on stdio: plain JSON-RPC (`serve`) or an MCP server (`mcp`), both
//! over a session that keeps each part's build stages and rebuilds only from the first feature
//! that changed.

use crate::b64;
use crate::fg::{self, Args, Reply};
use forge_agent::json::Json;
use std::io::{BufRead, Write};
use std::path::Path;
use std::process::ExitCode;

/// Each tool: name, description, how many of its parameters are required, its parameters,
/// and kept short: a client sends these with every request.
type Tool = (
    &'static str,
    &'static str,
    usize,
    &'static [(&'static str, &'static str)],
);

const TOOLS: [Tool; 22] = [
    (
        "plan_fulfillment",
        "Confirm a typed candidate on an existing hand-drawn circuit. Returns one checked edit and the source version.",
        5,
        &[
            ("file", "schematic .fg"),
            ("want", "qualified consumer pin"),
            ("give", "qualified provider pin"),
            ("role", "declared role ID"),
            ("id", "new relationship identifier"),
            ("return_from", "optional provider return pin"),
            ("return_to", "optional consumer return pin"),
        ],
    ),
    (
        "new_schematic",
        "Create a new empty native schematic or symbol library under the writable root.",
        2,
        &[("file", "new .fg path"), ("kind", "schematic or symbol_library")],
    ),
    (
        "plan_bypass",
        "From a selected consumer pin, propose a checked symbol placement, supply wire, return wire and typed fulfillment. Send the returned ops and version to edit to commit once.",
        10,
        &[
            ("file", "schematic .fg"),
            ("want", "qualified consumer pin"),
            ("return_pin", "qualified return terminal"),
            ("symbol", "library-qualified capacitor symbol"),
            ("give", "provider pin key on that symbol"),
            ("return_key", "return pin key on that symbol"),
            ("role", "declared bypass role ID"),
            ("id", "new .fg occurrence name"),
            ("x_mm", "sheet X in mm"),
            ("y_mm", "sheet Y in mm"),
        ],
    ),
    (
        "schematic",
        "Inspect a native schematic or symbol library. Typed roles, pin candidates, confirmed links, net partitions, sheet geometry, ERC, rooted wiring and the fitted BOM. Use edit for checked source changes.",
        1,
        &[
            ("file", "path to .fg"),
            (
                "sel",
                "summary, symbols, symbol ID, sheet PATH, erc, nets, links, pin PATH, tree PATH, search TERMS, variants, bom [VARIANT], bom-csv [VARIANT], board [VARIANT], erc [VARIANT], plan-reannotate [SHEET], plan-block copy|move SCOPE DX_MM DY_MM OBJECT... or pdf",
            ),
        ],
    ),
    (
        "import",
        "A model file (STEP, IGES, .brep, Parasolid .x_t/.x_b) under the root as a part: the file kept in `out` beside a new `<stem>.fg` that imports it (writable sessions). Replies with its solids and what was refused",
        1,
        &[
            ("file", "path to the model under the root"),
            (
                "out",
                "the directory under the root for the part (default: the model's own)",
            ),
        ],
    ),
    (
        "bend",
        "Run a bending document (.fg starting with `bending`): the press-brake program simulated as it states (certified tool clearance through each bend, handling between steps); reply forgecad.bending/1. what=sheet: the setup sheet",
        1,
        &[
            ("file", "path to a bending .fg"),
            ("what", "sheet: the setup sheet"),
        ],
    ),
    (
        "cam",
        "Run a machining document (.fg starting with `machining`): every op's path certified against the exact part (gouge, holder, rapids) or refused with the check and a fix; reply forgecad.cam/1. what=sheet: the setup sheet",
        1,
        &[
            ("file", "path to a machining .fg"),
            ("what", "sheet: the setup sheet instead of the program"),
        ],
    ),
    (
        "query",
        "selector -> @handles; or stat, file, check, params, features; holes, section z=V, gap A B, ..",
        2,
        &[
            ("file", "path to a .fg part"),
            ("sel", "what to ask"),
            ("at", "a feature id: the part as it stood after it"),
            (
                "tooling",
                "bendsim: a tooling set (std-eu, std-us) or a .json file",
            ),
            (
                "vendor",
                "bendsim: a vendor file whose `tooling` names the press tools",
            ),
            ("sequence", "bendsim: bends in order, e.g. 3,1,2 or right,left"),
        ],
    ),
    (
        "measure",
        "mass, wall, or one or two refs",
        2,
        &[
            ("file", "path"),
            ("what", "mass|wall|ref"),
            ("to", "second ref"),
            ("at", "a feature id: the part as it stood after it"),
        ],
    ),
    (
        "render",
        "PNG of the part",
        1,
        &[
            ("file", "path"),
            ("view", "iso front top right back left bottom"),
            ("labels", "selector to label"),
            ("px", "long side"),
            ("at", "a feature id: the part as it stood after it"),
        ],
    ),
    (
        "export",
        "The part as a file: stl 3mf obj (meshes; tol MM, angle DEG, ascii, normals, faces, flat), step ap214 ap242 (tol MM), dxf dwg [SHEET]",
        2,
        &[
            ("file", "path to a .fg part"),
            ("format", "stl|3mf|obj|step|ap214|ap242|dxf|dwg"),
            (
                "options",
                "e.g. \"tol 0.05 angle 10\", \"ascii\" (STL), \"normals faces\" (OBJ), \"flat\" (an assembly's 3MF)",
            ),
            (
                "out",
                "a path under the root to write (writable sessions); else the file comes back inline",
            ),
        ],
    ),
    (
        "edit",
        "Change a .fg part by typed ops (as errors' fixes give them), all applied or none; replies with the build's typed report",
        2,
        &[
            ("file", "path to a .fg part"),
            (
                "op",
                "one op: {\"op\":\"SetParam\",\"param\":\"t\",\"value\":{\"value\":3,\"dim\":\"Length\"}}",
            ),
            ("ops", "several ops, in order"),
            (
                "base",
                "the version a reply gave; refused (StaleBase) if the file has moved on",
            ),
        ],
    ),
    (
        "vendors",
        "Sheet-metal vendor profiles (stocks, radii, K, rules, limits, sources): none, the list; an id, its profile. Check a part with query sel dfm",
        0,
        &[("id", "a vendor's id")],
    ),
    (
        "help",
        "The .fg language: forge, all of it; a word, its sections (holes, sheet, queries); none: the sections",
        0,
        &[("topic", "a feature or topic")],
    ),
    (
        "versions",
        "A file's named versions as JSON: n, name, notes, save, author, agent.",
        1,
        &[("file", "path")],
    ),
    (
        "history",
        "A file's saves, newest first (all: the whole project's), as JSON; versions marked.",
        1,
        &[
            ("file", "path"),
            ("all", "\"true\" for every file's saves"),
            ("limit", "at most this many"),
        ],
    ),
    (
        "create_version",
        "Save the file and mark it as its next named version (writable sessions only).",
        2,
        &[
            ("file", "path"),
            ("name", "the version's name"),
            ("notes", "what changed"),
        ],
    ),
    (
        "restore",
        "Restore the file (an assembly: and the parts it follows) as a version or save was, as a new save.",
        2,
        &[("file", "path"), ("to", "v3, or a save id")],
    ),
    (
        "compare",
        "What changed between two states of a file: features, params, faces by name, volume.",
        3,
        &[
            ("file", "path"),
            ("a", "base: v3, a save id, or current"),
            ("b", "target: v3, a save id, or current"),
            ("volume", "\"true\" for the exact added and removed volume"),
        ],
    ),
    (
        "release",
        "Release the file as its next revision: a named version, exports outside history, a release record.",
        2,
        &[
            ("file", "path"),
            ("reason", "why"),
            ("approver", "who approved it"),
            ("numeric", "\"true\" for a numeric pre-release revision"),
        ],
    ),
    (
        "releases",
        "A file's releases: part number, revision, reason, approver, kernel, export hashes.",
        1,
        &[("file", "path")],
    ),
    (
        "regenerate_release",
        "Rebuild a release's files from its record and check every hash (drift on another kernel).",
        2,
        &[("file", "path"), ("rev", "the revision, e.g. B")],
    ),
];

fn args(p: &Json) -> Args {
    let s = |k: &str| p.get(k).and_then(Json::str).map(str::to_string);
    let mut a = Args {
        file: s("file").unwrap_or_default(),
        labels: s("labels"),
        at: s("at"),
        base: s("base"),
        px: p.get("px").and_then(Json::num).map(|n| n as usize),
        ..Args::default()
    };
    // a question sent as one string, its words apart (`section z=67`, `gap A B`): a selector is
    // read joined again. A ref to measure stays whole: `(edges circle r=3)` is one
    a.words = s("sel")
        .map(|w| w.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default();
    a.words
        .extend(["what", "to", "view", "other"].iter().filter_map(|k| s(k)));
    // bendsim's options, as its words
    for k in ["tooling", "vendor", "sequence"] {
        if let Some(v) = s(k) {
            a.words.push(k.into());
            a.words.push(v);
        }
    }
    // export's format, then its options as words; plain STEP is no word
    if let Some(f) = s("format").filter(|f| f != "step") {
        a.words.push(f);
    }
    if let Some(o) = s("options") {
        a.words.extend(o.split_whitespace().map(str::to_string));
    }
    if let Some(t) = s("topic") {
        a.file = t;
    }
    a
}

/// Serves files under `root` only, writing them (edit, the version tools) only if `writable`.
pub fn run(mcp: bool, root: &str, writable: bool) -> ExitCode {
    let root = match std::fs::canonicalize(root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{root}: {e}");
            return ExitCode::from(2);
        }
    };
    let mut forge = fg::Session::new(Some(root.clone()), writable);
    let _ = FENCE.set((root.clone(), writable));
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(r) = respond(&mut forge, &line, mcp, (&root, writable))
            && writeln!(out, "{r}").and_then(|_| out.flush()).is_err()
        {
            break;
        }
    }
    ExitCode::SUCCESS
}

/// One request's answer (none for an MCP notification), and the sketches its runs kept freed.
fn respond(
    forge: &mut fg::Session,
    line: &str,
    mcp: bool,
    (root, writable): (&std::path::Path, bool),
) -> Option<Json> {
    let reply = match Json::parse(line) {
        // a request that panics answers with an error, and the session starts afresh
        Ok(req) => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| answer(forge, &req, mcp)))
            .unwrap_or_else(|_| {
                *forge = fg::Session::new(Some(root.to_path_buf()), writable);
                let id = req.get("id").cloned().unwrap_or(Json::Null);
                Some(error(
                    id,
                    -32603,
                    "internal error: the request failed; the session was reset",
                ))
            }),
        Err(e) => Some(error(Json::Null, -32700, &e)),
    };
    // the sketches the request's runs made: freed between requests (a run lets none go)
    release_sketches();
    reply
}

fn error(id: Json, code: i32, msg: &str) -> Json {
    Json::obj(vec![
        ("jsonrpc", "2.0".into()),
        ("id", id),
        (
            "error",
            Json::obj(vec![("code", Json::Num(code as f64)), ("message", msg.into())]),
        ),
    ])
}

fn result(id: Json, r: Json) -> Json {
    Json::obj(vec![("jsonrpc", "2.0".into()), ("id", id), ("result", r)])
}

/// The session's root and whether it may write, for the version tools.
static FENCE: std::sync::OnceLock<(std::path::PathBuf, bool)> = std::sync::OnceLock::new();

const VERSION_TOOLS: &[&str] = &[
    "versions",
    "history",
    "create_version",
    "restore",
    "compare",
    "release",
    "releases",
    "regenerate_release",
];

/// A version tool: the file inside the root, writes only where the session writes.
fn version_tool(name: &str, p: &Json) -> Reply {
    let s = |k: &str| p.get(k).and_then(Json::str).map(str::to_string);
    let fail = |msg: String| Reply {
        ok: false,
        text: format!("error: {msg}"),
        json: Json::obj(vec![("ok", false.into()), ("error", msg.into())]),
        png: None,
    };
    let Some((root, writable)) = FENCE.get() else {
        return fail("no session root".into());
    };
    let file = s("file").unwrap_or_default();
    let path = root.join(&file);
    let inside = path
        .parent()
        .and_then(|d| std::fs::canonicalize(d).ok())
        .is_some_and(|d| d.starts_with(root));
    if !inside {
        return fail(format!("{file} is outside the session's root"));
    }
    if !writable
        && matches!(
            name,
            "create_version" | "restore" | "release" | "regenerate_release"
        )
    {
        return fail(format!(
            "{name} writes: the session is read-only (serve --writable)"
        ));
    }
    let mut a = vec![path.to_string_lossy().to_string()];
    for k in ["name", "notes", "to", "a", "b", "reason", "rev"] {
        if let Some(v) = s(k) {
            a.push(v);
        }
    }
    for k in ["all", "volume", "numeric"] {
        if s(k).as_deref() == Some("true") || p.get(k).is_some_and(|v| matches!(v, Json::Bool(true))) {
            a.push(format!("--{k}"));
        }
    }
    for k in ["approver", "limit"] {
        if let Some(v) = s(k).or_else(|| p.get(k).and_then(Json::num).map(|n| n.to_string())) {
            a.push(format!("--{k}"));
            a.push(v);
        }
    }
    let agent = std::env::var("FORGECAD_AGENT").unwrap_or_else(|_| "agent".into());
    match forge_lang::vtools::call(name, &a, Some(&agent)) {
        Some(Ok(j)) => Reply {
            ok: true,
            text: j.to_string(),
            json: j,
            png: None,
        },
        Some(Err(e)) => fail(e),
        None => fail(format!("no tool {name}")),
    }
}

/// A tool call: the version tools, help, or the part session.
fn call(forge: &mut fg::Session, name: &str, p: &Json) -> Reply {
    if VERSION_TOOLS.contains(&name) {
        return version_tool(name, p);
    }
    let mut a = args(p);
    if name == "new_schematic" {
        let result = (|| -> Result<Reply, String> {
            let (root, writable) = FENCE.get().ok_or("no session root")?;
            if !writable {
                return Err("the session is read-only".into());
            }
            let src = match p.get("kind").and_then(Json::str) {
                Some("schematic") => forge_lang::schematic::NEW_SCHEMATIC,
                Some("symbol_library") => "symbol_library\n",
                _ => return Err("kind must be schematic or symbol_library".into()),
            };
            let path = out_place(root, &a.file)?;
            if path.extension().and_then(|x| x.to_str()) != Some("fg") {
                return Err("new document needs a .fg extension".into());
            }
            let mut file = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&path)
                .map_err(|e| e.to_string())?;
            if let Err(e) = file.write_all(src.as_bytes()) {
                drop(file);
                let _ = std::fs::remove_file(&path);
                return Err(e.to_string());
            }
            a.file = path.to_str().ok_or("expected UTF-8 path")?.into();
            Ok(forge.run("schematic", &a, p))
        })();
        return result.unwrap_or_else(|msg| Reply {
            text: msg.clone(),
            json: Json::obj(vec![("ok", false.into()), ("error", msg.into())]),
            ok: false,
            png: None,
        });
    }
    if name == "plan_fulfillment" {
        a.words = vec!["plan-fulfillment".into()];
        for key in ["want", "give", "role", "id", "return_from", "return_to"] {
            if let Some(value) = p.get(key).and_then(Json::str) {
                a.words.push(value.into());
            } else if !key.starts_with("return_") {
                return fg::fail(format!("missing {key}"));
            }
        }
        return forge.run("schematic", &a, p);
    }
    if name == "plan_bypass" {
        a.words = vec!["plan-bypass".into()];
        for key in [
            "want",
            "return_pin",
            "symbol",
            "give",
            "return_key",
            "role",
            "id",
            "x_mm",
            "y_mm",
        ] {
            let value = match p.get(key) {
                Some(Json::Str(s)) => s.clone(),
                Some(Json::Num(n)) if matches!(key, "x_mm" | "y_mm") => n.to_string(),
                _ => String::new(),
            };
            a.words.push(value);
        }
        return forge.run("schematic", &a, p);
    }
    if name == "vendors" {
        let mut v = vec![];
        match p.get("id").and_then(Json::str) {
            Some(id) => v.extend(["show".to_string(), id.to_string()]),
            None => v.push("list".to_string()),
        }
        return match forge_lang::vendor::tool(&v, Some(std::path::Path::new("."))) {
            Ok((json, text)) => Reply {
                ok: true,
                json,
                text,
                png: None,
            },
            Err(e) => Reply {
                ok: false,
                json: Json::obj(vec![("ok", false.into()), ("error", e.as_str().into())]),
                text: e,
                png: None,
            },
        };
    }
    if name == "import" {
        return model_import(p);
    }
    if name == "help" {
        let text = forge_lang::help(&a.file);
        return Reply {
            ok: true,
            json: Json::obj(vec![("ok", true.into()), ("text", text.as_str().into())]),
            text,
            png: None,
        };
    }
    let mut r = forge.run(name, &a, p);
    if (name == "cam" || name == "bend")
        && let Some(b) = r.png.take()
    {
        // the program or the sheet is text: it goes as text, never as an image
        r.text.push('\n');
        r.text.push_str(&String::from_utf8_lossy(&b));
    }
    if name == "export"
        && r.ok
        && let Some(out) = p.get("out").and_then(Json::str)
    {
        write_out(&mut r, out);
    }
    r
}

/// `import`: a model under the root brought in as a part, in a writable session only.
fn model_import(p: &Json) -> Reply {
    let fail = |msg: String| Reply {
        ok: false,
        text: format!("error: {msg}"),
        json: Json::obj(vec![("ok", false.into()), ("error", msg.as_str().into())]),
        png: None,
    };
    let Some((root, writable)) = FENCE.get() else {
        return fail("no session root".into());
    };
    if !writable {
        return fail("import writes: the session is read-only (serve --writable)".into());
    }
    let Some(file) = p.get("file").and_then(Json::str) else {
        return fail("import needs a file".into());
    };
    let src = root.join(file);
    let dir = match p.get("out").and_then(Json::str) {
        Some(o) => root.join(o),
        None => src.parent().map_or_else(|| root.clone(), Path::to_path_buf),
    };
    let under = |q: &Path| std::fs::canonicalize(q).is_ok_and(|c| c.starts_with(root));
    if !under(&src) {
        return fail(format!("{file} is not a file under the session's root"));
    }
    let git = |q: &Path| {
        q.strip_prefix(root)
            .unwrap_or(q)
            .components()
            .any(|c| c.as_os_str().to_string_lossy().eq_ignore_ascii_case(".git"))
    };
    if git(&dir) {
        return fail("out: not into the repository's .git".into());
    }
    let _ = std::fs::create_dir_all(&dir);
    if !under(&dir) || std::fs::canonicalize(&dir).is_ok_and(|d| git(&d)) {
        return fail("out is outside the session's root".into());
    }
    if !crate::model::is_model_name(&src) {
        return fail(format!(
            "{file}: not a model file (STEP, IGES, .brep, .x_t, .x_b)"
        ));
    }
    match crate::model::import(&src, &dir) {
        Ok((json, text)) => Reply {
            ok: true,
            json,
            text,
            png: None,
        },
        Err(e) => fail(e),
    }
}

/// Where an export's `out` really goes: under the root, its folder there already, not in the
/// repository's .git (git would run what's written there), and not through a link out of the
/// root or into .git (`model.stl -> ~/.bashrc`, `g -> .git`).
fn out_place(root: &std::path::Path, out: &str) -> Result<std::path::PathBuf, String> {
    let git = |p: &std::path::Path| {
        p.components()
            .any(|c| c.as_os_str().to_string_lossy().eq_ignore_ascii_case(".git"))
    };
    let rel = std::path::Path::new(out);
    if git(rel) {
        return Err(format!("{out}: not into the repository's .git"));
    }
    let path = root.join(rel);
    let (Some(dir), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(format!("{out} is outside the session's root"));
    };
    let dir = std::fs::canonicalize(dir).map_err(|_| format!("{out} is outside the session's root"))?;
    if !dir.starts_with(root) {
        return Err(format!("{out} is outside the session's root"));
    }
    let real = dir.join(name);
    // a link there already: where it leads is where the old code would have written
    if std::fs::symlink_metadata(&real).is_ok_and(|m| m.file_type().is_symlink()) {
        let to = std::fs::canonicalize(&real).unwrap_or_else(|_| real.clone());
        if !to.starts_with(root) || git(to.strip_prefix(root).unwrap_or(&to)) {
            return Err(format!("{out} is a link out of the session's root"));
        }
    }
    if git(real.strip_prefix(root).unwrap_or(&real)) {
        return Err(format!("{out}: not into the repository's .git"));
    }
    Ok(real)
}

/// An export written to `out` under the session's root, not sent back: refused in a read-only
/// session or outside the root.
fn write_out(r: &mut Reply, out: &str) {
    let refuse = |r: &mut Reply, msg: String| {
        r.ok = false;
        r.png = None;
        r.text = format!("error: {msg}");
        r.json = Json::obj(vec![("ok", false.into()), ("error", msg.into())]);
    };
    let Some((root, writable)) = FENCE.get() else {
        return refuse(r, "no session root".into());
    };
    if !writable {
        return refuse(
            r,
            "export out writes: the session is read-only (serve --writable)".into(),
        );
    }
    let path = match out_place(root, out) {
        Ok(p) => p,
        Err(e) => return refuse(r, e),
    };
    let Some(bytes) = r.png.take() else { return };
    // as forgecad web writes: a file made new under a random name beside it, renamed over it,
    // never written through a link
    if let Err(e) = crate::web::put(&path, &bytes) {
        return refuse(r, format!("{out}: {e}"));
    }
    if let Json::Obj(f) = &mut r.json {
        f.push(("wrote".into(), out.into()));
    }
    r.text = format!("wrote {out} {}", r.text);
}

/// What an export's bytes are, for an MCP client.
fn mime(r: &Reply) -> String {
    r.json
        .get("mime")
        .and_then(Json::str)
        .map(str::to_string)
        .unwrap_or_else(|| {
            if r.json.get("sheet").is_some() {
                "application/octet-stream".into()
            } else {
                "model/step".into()
            }
        })
}

fn answer(forge: &mut fg::Session, req: &Json, mcp: bool) -> Option<Json> {
    let method = req.get("method").and_then(Json::str).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(Json::Obj(vec![]));
    if !mcp {
        // every serve method answers, id or not
        let r = call(forge, method, &params);
        return Some(result(req.get("id").cloned().unwrap_or(Json::Null), served(r)));
    }
    let id = req.get("id").cloned()?; // MCP notifications get no answer
    Some(match method {
        "initialize" => {
            let version = params
                .get("protocolVersion")
                .and_then(Json::str)
                .unwrap_or("2025-06-18");
            result(
                id,
                Json::obj(vec![
                    ("protocolVersion", version.into()),
                    ("capabilities", Json::obj(vec![("tools", Json::obj(vec![]))])),
                    (
                        "serverInfo",
                        Json::obj(vec![
                            ("name", "forgecad".into()),
                            ("version", env!("CARGO_PKG_VERSION").into()),
                        ]),
                    ),
                    ("instructions", include_str!("../../../AGENTS.md").into()),
                ]),
            )
        }
        "ping" => result(id, Json::obj(vec![])),
        "tools/list" => result(
            id,
            Json::obj(vec![("tools", Json::Arr(TOOLS.iter().map(tool).collect()))]),
        ),
        "tools/call" => {
            let name = params.get("name").and_then(Json::str).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(Json::Null);
            let r = call(forge, name, &arguments);
            let mut content = Vec::new();
            if let Some(png) = &r.png {
                if name == "export" || r.json.get("mime").and_then(Json::str) == Some("application/pdf") {
                    // a file, not a picture: an embedded resource
                    let ext = r
                        .json
                        .get("format")
                        .or_else(|| arguments.get("format"))
                        .and_then(Json::str)
                        .filter(|f| !matches!(*f, "ap214" | "ap242"))
                        .unwrap_or("step");
                    let file = arguments.get("file").and_then(Json::str).unwrap_or("part");
                    let stem = std::path::Path::new(file)
                        .file_stem()
                        .map_or_else(|| "part".into(), |s| s.to_string_lossy().into_owned());
                    content.push(Json::obj(vec![
                        ("type", "resource".into()),
                        (
                            "resource",
                            Json::obj(vec![
                                ("uri", format!("forgecad:///{stem}.{ext}").into()),
                                ("mimeType", mime(&r).into()),
                                ("blob", b64(png).into()),
                            ]),
                        ),
                    ]));
                } else {
                    content.push(Json::obj(vec![
                        ("type", "image".into()),
                        ("data", b64(png).into()),
                        ("mimeType", "image/png".into()),
                    ]));
                }
            }
            // a reply is its typed record: errors, warnings and fixes as data; help is prose
            let text = if name == "help" {
                r.text
            } else {
                r.json.to_string()
            };
            content.push(Json::obj(vec![("type", "text".into()), ("text", text.into())]));
            result(
                id,
                Json::obj(vec![("content", Json::Arr(content)), ("isError", (!r.ok).into())]),
            )
        }
        _ => error(id, -32601, &format!("no method {method}")),
    })
}

/// A plain JSON-RPC result: the reply's JSON, the picture inline as base64.
fn served(r: Reply) -> Json {
    let Json::Obj(mut f) = r.json else { return r.json };
    if let Some(png) = r.png {
        let key = if f
            .iter()
            .any(|(k, v)| k == "media_type" && v.str() == Some("application/pdf"))
        {
            "data"
        } else {
            "png"
        };
        f.push((key.into(), b64(&png).into()));
    }
    Json::Obj(f)
}

fn tool(t: &Tool) -> Json {
    let (name, desc, required, params) = *t;
    let props = params
        .iter()
        .map(|(p, d)| {
            // an op is an object (or its JSON as text), ops a list of them
            let mut f = vec![("description", (*d).into())];
            match *p {
                "px" | "x_mm" | "y_mm" => f.insert(0, ("type", "number".into())),
                "op" | "ops" => {}
                _ => f.insert(0, ("type", "string".into())),
            }
            (p.to_string(), Json::obj(f))
        })
        .collect();
    let required: Vec<Json> = params.iter().take(required).map(|p| p.0.into()).collect();
    Json::obj(vec![
        ("name", name.into()),
        ("description", desc.into()),
        (
            "inputSchema",
            Json::obj(vec![
                ("type", "object".into()),
                ("properties", Json::Obj(props)),
                ("required", Json::Arr(required)),
            ]),
        ),
    ])
}

/// Frees the sketches forge-lang's runs on this thread kept (`forge_keep`): each lets go of its
/// names, the cycle through them broken (a sketch with a named entity is an `Rc` cycle: its
/// `named` holds `Value::Ent`s of itself). A run frees none itself, so the speed gate's builds do
/// what they did; between builds, never during one, on the thread that ran them. Without it every
/// run kept its sketches (5.5 KB a run of a small part natively, ~30 KB an edit in the browser).
/// A process that never calls it (the speed gate's benchparts, a one-shot CLI command) keeps them
/// till it exits, as every process did before. Its own cost: the freeing a run paid before, about
/// 5,500 instructions a run of a small one-sketch part.
fn release_sketches() {
    forge_lang::release_sketches();
}

#[cfg(test)]
mod bounded {
    use super::*;

    /// A long session keeps no more sketches than one request's runs made: each request's are
    /// freed as it is answered (`release_sketches`), in `serve` and in `mcp` alike. Without it
    /// every build of a part kept its sketches (a sketch with a named entity is an `Rc` cycle).
    #[test]
    fn a_session_frees_each_requests_sketches() {
        let dir = forge_dirs::temp_dir().join(format!("forge-serve-bounded-{}", forge_dirs::process_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let part = dir.join("plate.fg");
        std::fs::write(
            &part,
            "shared t = 6\nsketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (60, 30))\n}\nbase = extrude(plan, t)\n",
        )
        .unwrap();
        let root = std::fs::canonicalize(&dir).unwrap();
        let file = part.to_str().unwrap().to_string();
        for mcp in [false, true] {
            let mut forge = fg::Session::new(Some(root.clone()), true);
            for k in 0..20 {
                let req = if mcp {
                    format!(
                        r#"{{"jsonrpc":"2.0","id":{k},"method":"tools/call","params":{{"name":"build","arguments":{{"file":{file:?}}}}}}}"#
                    )
                } else {
                    format!(r#"{{"jsonrpc":"2.0","id":{k},"method":"build","params":{{"file":{file:?}}}}}"#)
                };
                let r = respond(&mut forge, &req, mcp, (&root, true)).expect("an answer");
                assert!(r.get("error").is_none(), "{r}");
                assert_eq!(
                    forge_keep::kept(),
                    0,
                    "mcp {mcp}, request {k}: sketches kept after it"
                );
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
