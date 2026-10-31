//! One long-lived process on stdio: plain JSON-RPC (`serve`) or an MCP server (`mcp`), both
//! over a session that keeps each file's build and rebuilds only from the first changed line.

use crate::b64;
use forge_agent::cmd::{Args, Reply, Session};
use forge_agent::json::Json;
use std::io::{BufRead, Write};
use std::process::ExitCode;

/// Each tool: name, description, how many of its parameters are required, its parameters.
type Tool = (
    &'static str,
    &'static str,
    usize,
    &'static [(&'static str, &'static str)],
);

const TOOLS: [Tool; 6] = [
    (
        "build",
        "Regenerate a .fc part file. Returns ok or the failing line with a fix hint, then volume, area, bbox, counts.",
        1,
        &[("file", "path to the .fc file")],
    ),
    (
        "query",
        "Find faces/edges/verts; returns @handles usable in the file. e.g. `edges of hole1`, `faces plane normal +z top`, `edges circle r=3`.",
        2,
        &[
            ("file", "path"),
            ("sel", "selector"),
            ("at", "ask about the part after this feature"),
        ],
    ),
    (
        "measure",
        "`mass`, `wall` (min thickness), one @handle/(selector) (area|length, radius, centre) or two (distance, angle).",
        2,
        &[
            ("file", "path"),
            ("what", "mass | wall | @handle | (selector)"),
            ("to", "second @handle or (selector)"),
        ],
    ),
    (
        "render",
        "PNG of the part. Labels show handles (without @).",
        1,
        &[
            ("file", "path"),
            ("view", "iso front top right back left bottom"),
            ("labels", "selector of entities to label"),
            ("px", "long side in pixels, default 400"),
        ],
    ),
    (
        "check",
        "Validity of the part after every line.",
        1,
        &[("file", "path")],
    ),
    (
        "diff",
        "What changed between two versions of a part: first changed line, volume, faces added/removed.",
        2,
        &[("file", "path of version a"), ("other", "path of version b")],
    ),
];

fn args(p: &Json) -> Args {
    let s = |k: &str| p.get(k).and_then(Json::str).map(str::to_string);
    let mut a = Args {
        file: s("file").unwrap_or_default(),
        at: s("at"),
        labels: s("labels"),
        px: p.get("px").and_then(Json::num).map(|n| n as usize),
        ..Args::default()
    };
    a.words = ["sel", "what", "to", "view", "other"]
        .iter()
        .filter_map(|k| s(k))
        .collect();
    a
}

/// Serves files under `root` only.
pub fn run(mcp: bool, root: &str) -> ExitCode {
    let root = match std::fs::canonicalize(root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{root}: {e}");
            return ExitCode::from(2);
        }
    };
    let mut session = Session::rooted(root.clone());
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match Json::parse(&line) {
            // a request that panics answers with an error, and the session starts afresh
            Ok(req) => {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| answer(&mut session, &req, mcp)))
                    .unwrap_or_else(|_| {
                        session = Session::rooted(root.clone());
                        let id = req.get("id").cloned().unwrap_or(Json::Null);
                        Some(error(
                            id,
                            -32603,
                            "internal error: the request failed; the session was reset",
                        ))
                    })
            }
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

fn answer(session: &mut Session, req: &Json, mcp: bool) -> Option<Json> {
    let method = req.get("method").and_then(Json::str).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(Json::Obj(vec![]));
    if !mcp {
        // every serve method answers, id or not
        let r = session.run(method, &args(&params));
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
            let r = session.run(name, &args(params.get("arguments").unwrap_or(&Json::Null)));
            let mut content = Vec::new();
            if let Some(png) = &r.png {
                content.push(Json::obj(vec![
                    ("type", "image".into()),
                    ("data", b64(png).into()),
                    ("mimeType", "image/png".into()),
                ]));
            }
            content.push(Json::obj(vec![("type", "text".into()), ("text", r.text.into())]));
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
            let ty = if *p == "px" { "number" } else { "string" };
            (
                p.to_string(),
                Json::obj(vec![("type", ty.into()), ("description", (*d).into())]),
            )
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
