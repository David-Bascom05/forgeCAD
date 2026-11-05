//! ForgeCAD's MCP server for Claude Code: the app tools (`apptools.rs`) over JSON-RPC lines on
//! a unix socket of this app's own (in `$XDG_RUNTIME_DIR`, the person's alone). Claude Code
//! starts `forge-app --mcp-bridge SOCKET`, which carries its stdio to the socket.

use super::apptools::{AppTools, base64, schemas};
use forge_agent::json::Json;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

pub const SERVER: &str = "forgecad";

/// Where this app instance's socket goes.
pub fn socket_path() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    dir.join(format!("forgecad-agent-{}.sock", std::process::id()))
}

/// Serves the tools on `path` from a thread, a connection at a time each on its own.
pub fn serve(path: &Path, tools: AppTools) -> std::io::Result<()> {
    if path.exists() {
        // a socket left by a process of the same id that's gone: moved aside, never deleted
        let aside = path.with_extension(format!(
            "sock.{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::rename(path, aside)?;
    }
    let l = UnixListener::bind(path)?;
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    std::thread::Builder::new()
        .name("agent-mcp".into())
        .spawn(move || {
            for s in l.incoming().map_while(Result::ok) {
                let tools = tools.clone();
                let _ = std::thread::Builder::new()
                    .name("agent-mcp-conn".into())
                    .spawn(move || connection(s, &tools));
            }
        })?;
    Ok(())
}

fn connection(s: UnixStream, tools: &AppTools) {
    let Ok(mut w) = s.try_clone() else { return };
    for line in BufReader::new(s).lines().map_while(Result::ok) {
        if line.trim().is_empty() {
            continue;
        }
        if let Some(reply) = answer(&line, tools) {
            let mut t = String::new();
            reply.write(&mut t);
            t.push('\n');
            if w.write_all(t.as_bytes()).and_then(|_| w.flush()).is_err() {
                return;
            }
        }
    }
}

/// A JSON-RPC request's reply; none for a notification.
pub fn answer(line: &str, tools: &AppTools) -> Option<Json> {
    let req = match Json::parse(line) {
        Ok(r) => r,
        Err(e) => return Some(error(Json::Null, -32700, &format!("parse error: {e}"))),
    };
    let id = req.get("id").cloned()?;
    let method = req.get("method").and_then(Json::str).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(Json::Obj(vec![]));
    let result = match method {
        "initialize" => Json::obj(vec![
            (
                "protocolVersion",
                params
                    .get("protocolVersion")
                    .cloned()
                    .unwrap_or_else(|| "2025-06-18".into()),
            ),
            ("capabilities", Json::obj(vec![("tools", Json::obj(vec![]))])),
            (
                "serverInfo",
                Json::obj(vec![
                    ("name", SERVER.into()),
                    ("version", env!("CARGO_PKG_VERSION").into()),
                ]),
            ),
        ]),
        "ping" => Json::obj(vec![]),
        "tools/list" => Json::obj(vec![(
            "tools",
            Json::Arr(
                schemas()
                    .into_iter()
                    .map(|(name, what, schema)| {
                        Json::obj(vec![
                            ("name", name.into()),
                            ("description", what.into()),
                            ("inputSchema", schema),
                        ])
                    })
                    .collect(),
            ),
        )]),
        "tools/call" => {
            let name = params.get("name").and_then(Json::str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(Json::Obj(vec![]));
            let r = tools.call(name, args);
            let mut content = vec![Json::obj(vec![("type", "text".into()), ("text", r.text.into())])];
            if let Some(png) = &r.png {
                content.push(Json::obj(vec![
                    ("type", "image".into()),
                    ("data", base64(png).into()),
                    ("mimeType", "image/png".into()),
                ]));
            }
            Json::obj(vec![("content", Json::Arr(content)), ("isError", r.error.into())])
        }
        m => return Some(error(id, -32601, &format!("no method {m}"))),
    };
    Some(Json::obj(vec![
        ("jsonrpc", "2.0".into()),
        ("id", id),
        ("result", result),
    ]))
}

fn error(id: Json, code: i32, msg: &str) -> Json {
    Json::obj(vec![
        ("jsonrpc", "2.0".into()),
        ("id", id),
        (
            "error",
            Json::obj(vec![("code", (code as f64).into()), ("message", msg.into())]),
        ),
    ])
}

/// `forge-app --mcp-bridge SOCKET`: stdin to the socket, the socket to stdout, until either ends.
pub fn bridge(socket: &str) -> std::io::Result<()> {
    let s = UnixStream::connect(socket)?;
    let mut back = s.try_clone()?;
    let mut to = s;
    std::thread::spawn(move || {
        let _ = std::io::copy(&mut back, &mut std::io::stdout());
        std::process::exit(0);
    });
    std::io::copy(&mut std::io::stdin(), &mut to)?;
    Ok(())
}

/// The `--mcp-config` that gives Claude Code these tools and nothing else.
pub fn config(exe: &Path, socket: &Path) -> String {
    let c = Json::obj(vec![(
        "mcpServers",
        Json::obj(vec![(
            SERVER,
            Json::obj(vec![
                ("type", "stdio".into()),
                ("command", exe.display().to_string().into()),
                (
                    "args",
                    Json::Arr(vec!["--mcp-bridge".into(), socket.display().to_string().into()]),
                ),
            ]),
        )]),
    )]);
    c.to_string()
}
