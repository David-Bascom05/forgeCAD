//! One long-lived process on stdio: plain JSON-RPC (`serve`) or an MCP server (`mcp`), both
//! over a session that keeps each file's build and rebuilds only from the first changed line.

use crate::{b64, fg};
use forge_agent::cmd::{Args, Reply, Session};
use forge_agent::json::Json;
use std::io::{BufRead, Write};
use std::process::ExitCode;

/// Each tool: name, description, how many of its parameters are required, its parameters. Six,
/// and kept short: a client sends these with every request.
type Tool = (
    &'static str,
    &'static str,
    usize,
    &'static [(&'static str, &'static str)],
);

const TOOLS: [Tool; 14] = [
    (
        "act",
        "Change a .fc part: newline-separated lines, all applied or none. A .fg part: edit.",
        2,
        &[
            ("file", "path"),
            ("lines", "the lines"),
            ("base", "refuse if not this version"),
        ],
    ),
    (
        "query",
        "selector -> @handles; or stat, file, check, dims, since N; .fg also params, features",
        2,
        &[
            ("file", "path"),
            ("sel", "what to ask"),
            ("at", "after this feature"),
        ],
    ),
    (
        "measure",
        "mass, wall, or one or two refs",
        2,
        &[("file", "path"), ("what", "mass|wall|ref"), ("to", "second ref")],
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
        ],
    ),
    (
        "help",
        "The grammar of a feature or topic; none: the topics; forge: the .fg language",
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
        at: s("at"),
        labels: s("labels"),
        px: p.get("px").and_then(Json::num).map(|n| n as usize),
        base: p.get("base").and_then(Json::num).map(|n| n as u64),
        ..Args::default()
    };
    a.words = ["sel", "what", "to", "view", "other", "lines"]
        .iter()
        .filter_map(|k| s(k))
        .collect();
    // a question's words apart: `since 4`
    if let [one] = &a.words[..]
        && (one == "since" || one.starts_with("since "))
    {
        a.words = one.split_whitespace().map(str::to_string).collect();
    }
    if let Some(t) = s("topic") {
        a.file = t;
    }
    a
}

/// Serves files under `root` only, writing them (act) only if `writable`.
pub fn run(mcp: bool, root: &str, writable: bool) -> ExitCode {
    let root = match std::fs::canonicalize(root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{root}: {e}");
            return ExitCode::from(2);
        }
    };
    let mut session = Session::rooted(root.clone()).writable(writable);
    let mut forge = fg::Session::new(Some(root.clone()), writable);
    let _ = FENCE.set((root.clone(), writable));
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match Json::parse(&line) {
            // a request that panics answers with an error, and the session starts afresh
            Ok(req) => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                answer(&mut session, &mut forge, &req, mcp)
            }))
            .unwrap_or_else(|_| {
                session = Session::rooted(root.clone()).writable(writable);
                forge = fg::Session::new(Some(root.clone()), writable);
                let id = req.get("id").cloned().unwrap_or(Json::Null);
                Some(error(
                    id,
                    -32603,
                    "internal error: the request failed; the session was reset",
                ))
            }),
            Err(e) => Some(error(Json::Null, -32700, &e)),
        };
        if let Some(r) = reply
            && writeln!(out, "{r}").and_then(|_| out.flush()).is_err()
        {
            break;
        }
    }
    ExitCode::SUCCESS
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

/// A tool call, to the .fc or the .fg session by the file's kind.
fn call(session: &mut Session, forge: &mut fg::Session, name: &str, p: &Json) -> Reply {
    if VERSION_TOOLS.contains(&name) {
        return version_tool(name, p);
    }
    let a = args(p);
    if name == "help" && matches!(a.file.as_str(), "forge" | "fg" | ".fg") {
        return Reply {
            ok: true,
            text: forge_lang::REFERENCE.to_string(),
            json: Json::obj(vec![("ok", true.into()), ("text", forge_lang::REFERENCE.into())]),
            png: None,
        };
    }
    if name != "help" && fg::is_fg(&a.file) {
        return forge.run(name, &a, p);
    }
    if name == "edit" {
        let msg = format!("{}: edit takes .fg parts; a .fc part changes by act", a.file);
        return Reply {
            ok: false,
            text: format!("error: {msg}"),
            json: Json::obj(vec![("ok", false.into()), ("error", msg.into())]),
            png: None,
        };
    }
    session.run(name, &a)
}

fn answer(session: &mut Session, forge: &mut fg::Session, req: &Json, mcp: bool) -> Option<Json> {
    let method = req.get("method").and_then(Json::str).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(Json::Obj(vec![]));
    if !mcp {
        // every serve method answers, id or not
        let r = call(session, forge, method, &params);
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
            let fg_file = arguments.get("file").and_then(Json::str).is_some_and(fg::is_fg);
            let r = call(session, forge, name, &arguments);
            let mut content = Vec::new();
            if let Some(png) = &r.png {
                content.push(Json::obj(vec![
                    ("type", "image".into()),
                    ("data", b64(png).into()),
                    ("mimeType", "image/png".into()),
                ]));
            }
            // a .fg reply is its typed record: errors, warnings and fixes as data
            let text = if fg_file { r.json.to_string() } else { r.text };
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
        f.push(("png".into(), b64(&png).into()));
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
                "px" => f.insert(0, ("type", "number".into())),
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
