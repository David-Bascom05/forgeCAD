//! `forgecad web DIR [--port P] [--bind ADDR] [--token T] [--app WEBDIR] [--read-only]`: the
//! browser app (the static files `interface/forge-render/web/build.sh` makes) and a small HTTP API
//! over the files of DIR, so the editor in a browser works on files kept on this machine.
//!
//! The API (JSON; every file has an ETag, the hash `forgecad edit --base` takes as a version):
//!   GET    /api/info                  {"server":"forgecad web","root":NAME,"writable":B}
//!   GET    /api/files                 {"files":[{"path","size","etag"}]} (no dot files)
//!   GET    /api/files/PATH            the bytes, ETag
//!   PUT    /api/files/PATH            the bytes; If-Match: ETAG (the version it was made from)
//!                                     or If-None-Match: * (a new file); {"etag"}
//!   DELETE /api/files/PATH            If-Match: ETAG
//!   POST   /api/rename                {"from","to"}; refused if `to` is there; {"etag"}
//! A write against an ETag that is no longer the file's is refused with 412
//! {"error":"StaleBase","etag":CURRENT}: two editors never overwrite each other unseen; one
//! without a precondition is refused with 428.
//!
//! Safety: it listens on 127.0.0.1 unless `--bind` says otherwise, and on any other address it
//! requires a token (`--token`, or one it makes and prints), sent as `Authorization: Bearer T` or
//! `?token=T`. On loopback it answers only requests whose Host is a loopback name (no DNS
//! rebinding), and the API refuses a request from another origin. No path leaves DIR: `..`,
//! absolute paths and links pointing out are refused; dot files and folders are not served.

use forge_agent::json::Json;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

/// The most a request's body may hold.
const MAX_BODY: usize = 64 << 20;
/// The most a request's head may hold.
const MAX_HEAD: usize = 64 << 10;

pub struct Config {
    pub root: PathBuf,
    pub app: Option<PathBuf>,
    pub bind: IpAddr,
    pub port: u16,
    pub token: Option<String>,
    pub writable: bool,
}

const USAGE: &str = "forgecad web DIR [--port P] [--bind ADDR] [--token T] [--app WEBDIR] [--read-only]";

/// The command line's words after `web`.
pub fn main(args: &[String]) -> ExitCode {
    match parse(args) {
        Ok(c) => match serve(c, |a, url| {
            println!("forgecad web: serving {} at {url}", a.display());
        }) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("forgecad web: {e}");
                ExitCode::FAILURE
            }
        },
        Err(e) => {
            eprintln!("forgecad web: {e}\nusage: {USAGE}");
            ExitCode::from(2)
        }
    }
}

fn parse(args: &[String]) -> Result<Config, String> {
    let (mut root, mut app, mut bind, mut port, mut token, mut writable) =
        (None, None, IpAddr::from([127, 0, 0, 1]), 8080u16, None, true);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut val = |name: &str| it.next().cloned().ok_or(format!("{name} takes a value"));
        match a.as_str() {
            "--port" => port = val("--port")?.parse().map_err(|_| "--port takes a number")?,
            "--bind" => bind = val("--bind")?.parse().map_err(|_| "--bind takes an IP address")?,
            "--token" => token = Some(val("--token")?),
            "--app" => app = Some(PathBuf::from(val("--app")?)),
            "--read-only" => writable = false,
            s if s.starts_with("--") => return Err(format!("{s}: not an option")),
            s if root.is_none() => root = Some(PathBuf::from(s)),
            s => return Err(format!("{s}: one folder only")),
        }
    }
    let root = root.ok_or("which folder?")?;
    let root = std::fs::canonicalize(&root).map_err(|e| format!("{}: {e}", root.display()))?;
    if !root.is_dir() {
        return Err(format!("{}: not a folder", root.display()));
    }
    let app = app.or_else(default_app);
    if token.as_deref().is_some_and(|t| t.len() < 16) {
        return Err("--token: at least 16 characters".into());
    }
    if !bind.is_loopback() && token.is_none() {
        token = Some(random_token());
    }
    Ok(Config {
        root,
        app,
        bind,
        port,
        token,
        writable,
    })
}

/// The built app beside the installed binary (`share/forgecad/web`), or `FORGECAD_WEB`.
fn default_app() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("FORGECAD_WEB") {
        return Some(PathBuf::from(d));
    }
    let exe = std::env::current_exe().ok()?;
    let d = exe.parent()?.parent()?.join("share/forgecad/web");
    d.join("index.html").is_file().then_some(d)
}

/// 128 bits from the OS's randomness (std seeds each `RandomState` from it), as hex.
fn random_token() -> String {
    let mut s = String::new();
    for i in 0..2u64 {
        let mut h = RandomState::new().build_hasher();
        h.write_u64(i);
        s.push_str(&format!("{:016x}", h.finish()));
    }
    s
}

/// Serves until the listener fails; `ready` is told the folder and the URL to open.
pub fn serve(c: Config, ready: impl FnOnce(&Path, &str)) -> Result<(), String> {
    let l = TcpListener::bind(SocketAddr::new(c.bind, c.port))
        .map_err(|e| format!("{}:{}: {e}", c.bind, c.port))?;
    let at = l.local_addr().map_err(|e| e.to_string())?;
    let host = if c.bind.is_unspecified() || c.bind.is_loopback() {
        "localhost".to_string()
    } else {
        at.ip().to_string()
    };
    let url = match &c.token {
        Some(t) => format!("http://{host}:{}/?token={t}", at.port()),
        None => format!("http://{host}:{}/", at.port()),
    };
    if c.app.is_none() {
        eprintln!("forgecad web: no built app (FORGECAD_WEB, --app): only the API is served");
    }
    ready(&c.root, &url);
    run(l, Arc::new(c))
}

/// Serves on a listener already bound (the tests' port 0).
pub fn run(l: TcpListener, c: Arc<Config>) -> Result<(), String> {
    // one writer at a time: a check-then-write is whole
    let lock = Arc::new(Mutex::new(()));
    for s in l.incoming() {
        let Ok(s) = s else { continue };
        let (c, lock) = (c.clone(), lock.clone());
        std::thread::spawn(move || {
            let _ = connection(s, &c, &lock);
        });
    }
    Ok(())
}

struct Request {
    method: String,
    path: String,
    query: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Request {
    fn header(&self, k: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(k))
            .map(|(_, v)| v.as_str())
    }
    fn query(&self, k: &str) -> Option<String> {
        self.query.split('&').find_map(|kv| {
            let (a, b) = kv.split_once('=')?;
            (a == k).then(|| unescape(b).unwrap_or_default())
        })
    }
}

struct Response {
    status: u16,
    kind: &'static str,
    headers: Vec<(&'static str, String)>,
    body: Vec<u8>,
}

fn json(status: u16, j: Json) -> Response {
    Response {
        status,
        kind: "application/json",
        headers: vec![],
        body: j.to_string().into_bytes(),
    }
}

fn error(status: u16, e: &str) -> Response {
    json(status, Json::obj(vec![("error", e.into())]))
}

fn reason(s: u16) -> &'static str {
    match s {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        412 => "Precondition Failed",
        413 => "Payload Too Large",
        421 => "Misdirected Request",
        428 => "Precondition Required",
        _ => "Error",
    }
}

fn read_request(r: &mut BufReader<TcpStream>) -> Result<Option<Request>, Response> {
    let mut head = Vec::new();
    loop {
        let mut line = Vec::new();
        let n = r
            .read_until(b'\n', &mut line)
            .map_err(|_| error(400, "unreadable"))?;
        if n == 0 {
            return if head.is_empty() {
                Ok(None)
            } else {
                Err(error(400, "cut short"))
            };
        }
        if line == b"\r\n" || line == b"\n" {
            break;
        }
        head.extend_from_slice(&line);
        if head.len() > MAX_HEAD {
            return Err(error(413, "the request's head is too large"));
        }
    }
    let head = String::from_utf8(head).map_err(|_| error(400, "not UTF-8"))?;
    let mut lines = head.lines();
    let first = lines.next().unwrap_or_default();
    let mut w = first.split(' ');
    let (method, target) = (w.next().unwrap_or_default(), w.next().unwrap_or_default());
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect();
    let len: usize = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .map_or(Ok(0), |(_, v)| v.parse())
        .map_err(|_| error(400, "a bad Content-Length"))?;
    if len > MAX_BODY {
        return Err(error(413, "the file is too large"));
    }
    let mut body = vec![0; len];
    r.read_exact(&mut body)
        .map_err(|_| error(400, "the body cut short"))?;
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    Ok(Some(Request {
        method: method.to_string(),
        path: path.to_string(),
        query: query.to_string(),
        headers,
        body,
    }))
}

/// The response; to a HEAD its head only (its Content-Length still the body's): a body sent
/// would be read as the next response on a kept connection.
fn write_response(s: &mut TcpStream, r: &Response, keep: bool, head: bool) -> std::io::Result<()> {
    let mut text = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nX-Content-Type-Options: nosniff\r\n\
         Cache-Control: no-cache\r\nCross-Origin-Opener-Policy: same-origin\r\n\
         Cross-Origin-Embedder-Policy: require-corp\r\nConnection: {}\r\n",
        r.status,
        reason(r.status),
        r.kind,
        r.body.len(),
        if keep { "keep-alive" } else { "close" }
    );
    for (k, v) in &r.headers {
        text.push_str(&format!("{k}: {v}\r\n"));
    }
    text.push_str("\r\n");
    s.write_all(text.as_bytes())?;
    if !head {
        s.write_all(&r.body)?;
    }
    s.flush()
}

fn connection(s: TcpStream, c: &Config, lock: &Mutex<()>) -> std::io::Result<()> {
    let mut out = s.try_clone()?;
    let mut r = BufReader::new(s);
    loop {
        let (resp, keep, head) = match read_request(&mut r) {
            Ok(None) => return Ok(()),
            Ok(Some(q)) => {
                let keep = !q
                    .header("connection")
                    .is_some_and(|v| v.eq_ignore_ascii_case("close"));
                (answer(&q, c, lock), keep, q.method == "HEAD")
            }
            Err(e) => (e, false, false),
        };
        write_response(&mut out, &resp, keep, head)?;
        if !keep {
            return Ok(());
        }
    }
}

/// Equal without saying where they differ.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

fn host_name(h: &str) -> &str {
    if let Some(rest) = h.strip_prefix('[') {
        return rest.split(']').next().unwrap_or_default();
    }
    h.rsplit_once(':').map_or(h, |(n, p)| {
        if p.bytes().all(|b| b.is_ascii_digit()) {
            n
        } else {
            h
        }
    })
}

fn is_loopback_name(h: &str) -> bool {
    let n = host_name(h);
    n.eq_ignore_ascii_case("localhost") || n.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

fn answer(q: &Request, c: &Config, lock: &Mutex<()>) -> Response {
    // on loopback, only a loopback Host: a page elsewhere can't reach this through DNS
    if c.bind.is_loopback() && !q.header("host").is_some_and(is_loopback_name) {
        return error(421, "this server answers to localhost only");
    }
    let api = q.path == "/api" || q.path.starts_with("/api/");
    if !api {
        return match q.method.as_str() {
            "GET" | "HEAD" => static_file(c, &q.path),
            _ => error(405, "the app's files are read only"),
        };
    }
    if let Some(t) = &c.token {
        let given = q
            .header("authorization")
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(str::to_string)
            .or_else(|| q.query("token"));
        if !given.is_some_and(|g| same(&g, t)) {
            return error(401, "a token is needed (Authorization: Bearer T, or ?token=T)");
        }
    }
    // a page from another origin may not drive the API with this browser's access
    if let (Some(o), Some(h)) = (q.header("origin"), q.header("host")) {
        let o = o.split_once("://").map_or(o, |x| x.1);
        if !o.eq_ignore_ascii_case(h) {
            return error(403, "a request from another origin");
        }
    }
    let rest = q.path.strip_prefix("/api").unwrap_or_default();
    match (q.method.as_str(), rest) {
        ("GET", "/info") => json(
            200,
            Json::obj(vec![
                ("server", "forgecad web".into()),
                (
                    "root",
                    c.root
                        .file_name()
                        .map_or("/".into(), |n| n.to_string_lossy().into_owned())
                        .into(),
                ),
                ("writable", c.writable.into()),
            ]),
        ),
        ("GET", "/files") => list(c),
        ("POST", "/rename") => {
            if !c.writable {
                return error(403, "read only");
            }
            let _held = lock.lock().unwrap_or_else(|e| e.into_inner());
            rename(c, &q.body)
        }
        (m, f) if f.starts_with("/files/") => {
            let rel = match unescape(&f["/files/".len()..]).and_then(|p| clean(&p)) {
                Ok(p) => p,
                Err(e) => return error(400, &e),
            };
            match m {
                "GET" | "HEAD" => read(c, &rel),
                "PUT" | "DELETE" if !c.writable => error(403, "read only"),
                "PUT" => {
                    let _held = lock.lock().unwrap_or_else(|e| e.into_inner());
                    write(c, &rel, q)
                }
                "DELETE" => {
                    let _held = lock.lock().unwrap_or_else(|e| e.into_inner());
                    delete(c, &rel, q)
                }
                _ => error(405, "GET, PUT or DELETE"),
            }
        }
        _ => error(404, "no such call"),
    }
}

fn unescape(s: &str) -> Result<String, String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let h = s.get(i + 1..i + 3).ok_or("a bad %-escape")?;
            out.push(u8::from_str_radix(h, 16).map_err(|_| "a bad %-escape")?);
            i += 3;
        } else {
            out.push(if b[i] == b'+' { b' ' } else { b[i] });
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| "not UTF-8".into())
}

/// A path inside the folder, as its parts: relative, no `.`/`..`, no dot files, no separators
/// but `/`, nothing a filesystem reads specially.
pub fn clean(p: &str) -> Result<PathBuf, String> {
    if p.contains('\\')
        || p.contains('\0')
        || p.contains(':')
        || p.split('/').any(|s| s.is_empty() || s == "." || s == "..")
    {
        return Err(format!("{p}: not a project path"));
    }
    let mut out = PathBuf::new();
    for c in Path::new(p).components() {
        match c {
            Component::Normal(n) if !n.to_string_lossy().starts_with('.') => out.push(n),
            _ => return Err(format!("{p}: not a project path")),
        }
    }
    if out.as_os_str().is_empty() {
        return Err("an empty path".into());
    }
    Ok(out)
}

/// The file's place on disk, refused if it (or the folder it would be made in) leads out of the
/// root through a link.
fn place(c: &Config, rel: &Path) -> Result<PathBuf, Response> {
    let p = c.root.join(rel);
    // the nearest part of the path that exists, resolved, must be inside
    let mut there = p.as_path();
    while !there.exists() {
        there = there.parent().unwrap_or(&c.root);
    }
    let real = std::fs::canonicalize(there).map_err(|_| error(404, "not found"))?;
    if !real.starts_with(&c.root) {
        return Err(error(403, "outside the folder"));
    }
    if std::fs::symlink_metadata(&p).is_ok_and(|m| m.file_type().is_symlink())
        && std::fs::canonicalize(&p).is_ok_and(|r| !r.starts_with(&c.root))
    {
        return Err(error(403, "outside the folder"));
    }
    Ok(p)
}

/// A file's version: the hash `edit --base` takes, quoted as an ETag.
pub fn etag(bytes: &[u8]) -> String {
    format!("\"{:016x}\"", forge_math::hash::hash128(bytes) as u64)
}

fn list(c: &Config) -> Response {
    let mut files = vec![];
    let mut dirs = vec![c.root.clone()];
    while let Some(d) = dirs.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let Ok(ft) = e.file_type() else { continue };
            let p = e.path();
            if ft.is_dir() {
                dirs.push(p);
            } else if ft.is_file()
                && let Ok(bytes) = std::fs::read(&p)
            {
                let rel = p.strip_prefix(&c.root).unwrap_or(&p);
                let rel: Vec<String> = rel
                    .components()
                    .map(|x| x.as_os_str().to_string_lossy().into_owned())
                    .collect();
                files.push((rel.join("/"), bytes.len(), etag(&bytes)));
            }
        }
    }
    files.sort();
    json(
        200,
        Json::obj(vec![(
            "files",
            Json::Arr(
                files
                    .into_iter()
                    .map(|(p, n, e)| {
                        Json::obj(vec![
                            ("path", p.into()),
                            ("size", Json::Num(n as f64)),
                            ("etag", e.into()),
                        ])
                    })
                    .collect(),
            ),
        )]),
    )
}

fn read(c: &Config, rel: &Path) -> Response {
    let p = match place(c, rel) {
        Ok(p) => p,
        Err(r) => return r,
    };
    match std::fs::read(&p) {
        Ok(bytes) if p.is_file() => Response {
            status: 200,
            kind: "application/octet-stream",
            headers: vec![("ETag", etag(&bytes))],
            body: bytes,
        },
        _ => error(404, "no such file"),
    }
}

fn current(p: &Path) -> Option<String> {
    std::fs::read(p).ok().filter(|_| p.is_file()).map(|b| etag(&b))
}

fn stale(now: Option<String>) -> Response {
    json(
        412,
        Json::obj(vec![
            ("error", "StaleBase".into()),
            ("etag", now.map_or(Json::Null, Json::from)),
        ]),
    )
}

/// Whether the request's precondition holds for the file as it is.
fn precondition(q: &Request, now: &Option<String>) -> Result<(), Response> {
    match (q.header("if-match"), q.header("if-none-match")) {
        (Some(want), _) if now.as_deref() == Some(want) => Ok(()),
        (Some(_), _) => Err(stale(now.clone())),
        (None, Some("*")) if now.is_none() => Ok(()),
        (None, Some("*")) => Err(stale(now.clone())),
        _ => Err(error(
            428,
            "If-Match (the version it was made from) or If-None-Match: *",
        )),
    }
}

/// Written whole or not at all: a file beside it, then renamed over it.
fn put(p: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d)?;
    }
    let tmp = p.with_file_name(format!(
        ".{}.forgecad-web-{}",
        p.file_name()
            .map_or_else(Default::default, |n| n.to_string_lossy()),
        std::process::id()
    ));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, p).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

fn write(c: &Config, rel: &Path, q: &Request) -> Response {
    let p = match place(c, rel) {
        Ok(p) => p,
        Err(r) => return r,
    };
    if p.is_dir() {
        return error(409, "a folder is there");
    }
    if let Err(r) = precondition(q, &current(&p)) {
        return r;
    }
    match put(&p, &q.body) {
        Ok(()) => json(200, Json::obj(vec![("etag", etag(&q.body).into())])),
        Err(e) => error(500, &e.to_string()),
    }
}

fn delete(c: &Config, rel: &Path, q: &Request) -> Response {
    let p = match place(c, rel) {
        Ok(p) => p,
        Err(r) => return r,
    };
    let now = current(&p);
    if now.is_none() {
        return error(404, "no such file");
    }
    if q.header("if-match").is_none() {
        return error(428, "If-Match: the version it was read at");
    }
    if let Err(r) = precondition(q, &now) {
        return r;
    }
    match std::fs::remove_file(&p) {
        Ok(()) => Response {
            status: 204,
            kind: "application/json",
            headers: vec![],
            body: vec![],
        },
        Err(e) => error(500, &e.to_string()),
    }
}

fn rename(c: &Config, body: &[u8]) -> Response {
    let Ok(j) = Json::parse(&String::from_utf8_lossy(body)) else {
        return error(400, "not JSON");
    };
    let get = |k| {
        j.get(k)
            .and_then(Json::str)
            .ok_or(format!("{k}?"))
            .and_then(clean)
    };
    let (from, to) = match (get("from"), get("to")) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) | (_, Err(e)) => return error(400, &e),
    };
    let (a, b) = match (place(c, &from), place(c, &to)) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(r), _) | (_, Err(r)) => return r,
    };
    let Some(now) = current(&a) else {
        return error(404, "no such file");
    };
    if b.exists() {
        return error(409, "a file is there already");
    }
    if let Some(base) = j.get("base").and_then(Json::str)
        && base != now
    {
        return stale(Some(now));
    }
    if let Some(d) = b.parent()
        && let Err(e) = std::fs::create_dir_all(d)
    {
        return error(500, &e.to_string());
    }
    match std::fs::rename(&a, &b) {
        Ok(()) => json(200, Json::obj(vec![("etag", now.into())])),
        Err(e) => error(500, &e.to_string()),
    }
}

fn mime(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()).unwrap_or_default() {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "wasm" => "application/wasm",
        "css" => "text/css",
        "json" => "application/json",
        "txt" | "fg" | "csv" => "text/plain; charset=utf-8",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

fn static_file(c: &Config, path: &str) -> Response {
    let Some(app) = &c.app else {
        return Response {
            status: 404,
            kind: "text/plain; charset=utf-8",
            headers: vec![],
            body: b"forgecad web: no built app here. Build it (sh web/build.sh OUT in interface/forge-render) and give --app OUT.\n".to_vec(),
        };
    };
    let rel = path.trim_start_matches('/');
    let rel = if rel.is_empty() { "index.html" } else { rel };
    let Ok(rel) = unescape(rel).and_then(|r| clean(&r)) else {
        return error(404, "no such file");
    };
    let p = app.join(&rel);
    match std::fs::read(&p) {
        Ok(b) if p.is_file() => Response {
            status: 200,
            kind: mime(&p),
            headers: vec![],
            body: b,
        },
        _ => error(404, "no such file"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_never_leaves_the_folder() {
        for bad in [
            "../x",
            "/etc/passwd",
            "a/../../b",
            "a\\b",
            ".git/config",
            "a/.hidden",
            "",
            "c:x",
            "a/./b",
        ] {
            assert!(clean(bad).is_err(), "{bad}");
        }
        assert_eq!(clean("a/b.fg").unwrap(), PathBuf::from("a/b.fg"));
    }

    #[test]
    fn hosts_are_loopback_names_only_when_they_are() {
        for ok in ["localhost:8080", "127.0.0.1:1", "[::1]:80", "LOCALHOST"] {
            assert!(is_loopback_name(ok), "{ok}");
        }
        for bad in ["evil.com:8080", "10.0.0.1", "localhost.evil.com"] {
            assert!(!is_loopback_name(bad), "{bad}");
        }
    }

    #[test]
    fn a_made_token_is_long_and_different_each_time() {
        let (a, b) = (random_token(), random_token());
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
    }

    /// One request over a fresh connection: the status, the headers' text and the body.
    fn call(
        at: SocketAddr,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> (u16, String, Vec<u8>) {
        let mut s = TcpStream::connect(at).unwrap();
        let mut req = format!(
            "{method} {path} HTTP/1.1\r\nContent-Length: {}\r\nConnection: close\r\n",
            body.len()
        );
        if !headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("host")) {
            req.push_str(&format!("Host: localhost:{}\r\n", at.port()));
        }
        for (k, v) in headers {
            req.push_str(&format!("{k}: {v}\r\n"));
        }
        req.push_str("\r\n");
        s.write_all(req.as_bytes()).unwrap();
        s.write_all(body).unwrap();
        let mut got = vec![];
        s.read_to_end(&mut got).unwrap();
        let split = got.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let head = String::from_utf8(got[..split].to_vec()).unwrap();
        let status = head[9..12].parse().unwrap();
        (status, head, got[split + 4..].to_vec())
    }

    fn header(head: &str, k: &str) -> Option<String> {
        head.lines()
            .filter_map(|l| l.split_once(':'))
            .find(|(n, _)| n.eq_ignore_ascii_case(k))
            .map(|(_, v)| v.trim().to_string())
    }

    fn started(token: Option<&str>, writable: bool) -> (SocketAddr, PathBuf) {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let d = std::env::temp_dir().join(format!("forgecad-web-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("sub")).unwrap();
        std::fs::write(d.join("part.fg"), "shared t = 2\n").unwrap();
        std::fs::write(d.join("sub/b.fg"), "shared u = 3\n").unwrap();
        std::fs::create_dir_all(d.join(".git")).unwrap();
        std::fs::write(d.join(".git/config"), "secret").unwrap();
        let root = std::fs::canonicalize(&d).unwrap();
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let at = l.local_addr().unwrap();
        let c = Arc::new(Config {
            root: root.clone(),
            app: None,
            bind: IpAddr::from([127, 0, 0, 1]),
            port: at.port(),
            token: token.map(str::to_string),
            writable,
        });
        std::thread::spawn(move || run(l, c));
        (at, root)
    }

    #[test]
    fn files_are_listed_read_and_written_against_their_versions() {
        let (at, root) = started(None, true);
        let (s, _, b) = call(at, "GET", "/api/info", &[], b"");
        assert_eq!(s, 200);
        assert!(String::from_utf8_lossy(&b).contains("\"server\":\"forgecad web\""));
        let (s, _, b) = call(at, "GET", "/api/files", &[], b"");
        let listed = String::from_utf8(b).unwrap();
        assert_eq!(s, 200);
        assert!(
            listed.contains("\"part.fg\"") && listed.contains("\"sub/b.fg\""),
            "{listed}"
        );
        assert!(!listed.contains(".git"), "dot files are not served: {listed}");
        // read: the bytes and the version `edit --base` takes
        let (s, head, b) = call(at, "GET", "/api/files/part.fg", &[], b"");
        assert_eq!((s, b.as_slice()), (200, &b"shared t = 2\n"[..]));
        let v0 = header(&head, "etag").unwrap();
        assert_eq!(v0, format!("\"{}\"", crate::fg::version("shared t = 2\n")));
        // a write made from that version is taken; one made from it again is stale
        let (s, _, b) = call(
            at,
            "PUT",
            "/api/files/part.fg",
            &[("If-Match", &v0)],
            b"shared t = 3\n",
        );
        assert_eq!(s, 200, "{}", String::from_utf8_lossy(&b));
        assert_eq!(
            std::fs::read_to_string(root.join("part.fg")).unwrap(),
            "shared t = 3\n"
        );
        let (s, _, b) = call(
            at,
            "PUT",
            "/api/files/part.fg",
            &[("If-Match", &v0)],
            b"shared t = 4\n",
        );
        let said = String::from_utf8(b).unwrap();
        assert_eq!(s, 412);
        assert!(
            said.contains("StaleBase") && said.contains(&crate::fg::version("shared t = 3\n")),
            "{said}"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("part.fg")).unwrap(),
            "shared t = 3\n",
            "nothing lost"
        );
        // no precondition: refused, not a blind overwrite
        let (s, _, _) = call(at, "PUT", "/api/files/part.fg", &[], b"x");
        assert_eq!(s, 428);
        // a new file, in a new folder; then not new again
        let (s, _, _) = call(at, "PUT", "/api/files/new/c.fg", &[("If-None-Match", "*")], b"c");
        assert_eq!(s, 200);
        let (s, _, _) = call(at, "PUT", "/api/files/new/c.fg", &[("If-None-Match", "*")], b"c2");
        assert_eq!(s, 412);
        // renamed, then removed against its version
        let (s, _, b) = call(
            at,
            "POST",
            "/api/rename",
            &[],
            br#"{"from":"new/c.fg","to":"new/d.fg"}"#,
        );
        assert_eq!(s, 200, "{}", String::from_utf8_lossy(&b));
        assert!(!root.join("new/c.fg").exists() && root.join("new/d.fg").is_file());
        let (s, _, _) = call(
            at,
            "POST",
            "/api/rename",
            &[],
            br#"{"from":"new/d.fg","to":"part.fg"}"#,
        );
        assert_eq!(s, 409, "a rename never replaces a file");
        let vd = etag(b"c");
        let (s, _, _) = call(at, "DELETE", "/api/files/new/d.fg", &[("If-Match", "\"0\"")], b"");
        assert_eq!(s, 412);
        let (s, _, _) = call(at, "DELETE", "/api/files/new/d.fg", &[("If-Match", &vd)], b"");
        assert_eq!(s, 204);
        assert!(!root.join("new/d.fg").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn nothing_outside_the_folder_is_reached() {
        let (at, root) = started(None, true);
        let outside = root
            .parent()
            .unwrap()
            .join(format!("forgecad-web-outside-{}", std::process::id()));
        std::fs::write(&outside, "private").unwrap();
        for p in [
            "/api/files/..%2Fsecret",
            "/api/files/%2E%2E/x",
            "/api/files/.git/config",
            "/api/files/sub%5C..%5C..%5Cx",
        ] {
            let (s, _, b) = call(at, "GET", p, &[], b"");
            assert!(s == 400 || s == 404, "{p}: {s} {}", String::from_utf8_lossy(&b));
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, root.join("link.fg")).unwrap();
            std::os::unix::fs::symlink(root.parent().unwrap(), root.join("up")).unwrap();
            let (s, _, _) = call(at, "GET", "/api/files/link.fg", &[], b"");
            assert_eq!(s, 403, "a link out of the folder is not followed");
            let name = outside.file_name().unwrap().to_string_lossy().into_owned();
            let (s, _, _) = call(at, "GET", &format!("/api/files/up/{name}"), &[], b"");
            assert_eq!(s, 403);
            let (s, _, _) = call(
                at,
                "PUT",
                "/api/files/up/planted.fg",
                &[("If-None-Match", "*")],
                b"x",
            );
            assert_eq!(s, 403);
            assert!(!root.parent().unwrap().join("planted.fg").exists());
        }
        let _ = std::fs::remove_file(&outside);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_token_another_host_or_another_origin_is_refused() {
        let token = "0123456789abcdef0123";
        let (at, root) = started(Some(token), true);
        let (s, _, _) = call(at, "GET", "/api/files", &[], b"");
        assert_eq!(s, 401);
        let (s, _, _) = call(
            at,
            "GET",
            "/api/files",
            &[("Authorization", "Bearer wrong-wrong-wrong-wrong")],
            b"",
        );
        assert_eq!(s, 401);
        let bearer = format!("Bearer {token}");
        let (s, _, _) = call(at, "GET", "/api/files", &[("Authorization", &bearer)], b"");
        assert_eq!(s, 200);
        let (s, _, _) = call(at, "GET", &format!("/api/files?token={token}"), &[], b"");
        assert_eq!(s, 200);
        // DNS rebinding: a loopback server answers loopback names only
        let (s, _, _) = call(
            at,
            "GET",
            "/api/info",
            &[("Host", "evil.example:80"), ("Authorization", &bearer)],
            b"",
        );
        assert_eq!(s, 421);
        let (s, _, _) = call(
            at,
            "PUT",
            "/api/files/x.fg",
            &[
                ("Origin", "http://evil.example"),
                ("Authorization", &bearer),
                ("If-None-Match", "*"),
            ],
            b"x",
        );
        assert_eq!(s, 403);
        assert!(!root.join("x.fg").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_head_has_no_body_and_the_connection_goes_on() {
        let (at, root) = started(None, true);
        let mut s = TcpStream::connect(at).unwrap();
        let host = format!("Host: localhost:{}\r\n", at.port());
        let req = format!(
            "HEAD /api/files/part.fg HTTP/1.1\r\n{host}\r\nGET /api/files/part.fg HTTP/1.1\r\n{host}Connection: close\r\n\r\n"
        );
        s.write_all(req.as_bytes()).unwrap();
        let mut got = String::new();
        s.read_to_string(&mut got).unwrap();
        // two heads, one body: the GET's, after its own head
        assert_eq!(got.matches("HTTP/1.1 200").count(), 2, "{got}");
        assert_eq!(got.matches("shared t = 2").count(), 1, "{got}");
        assert!(got.trim_end().ends_with("shared t = 2"), "{got}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_read_only_server_writes_nothing() {
        let (at, root) = started(None, false);
        let (s, _, _) = call(at, "PUT", "/api/files/part.fg", &[("If-None-Match", "*")], b"x");
        assert_eq!(s, 403);
        let (s, _, _) = call(at, "GET", "/api/files/part.fg", &[], b"");
        assert_eq!(s, 200);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_non_loopback_bind_makes_a_token() {
        let d = std::env::temp_dir();
        let c = parse(&[
            d.to_string_lossy().into_owned(),
            "--bind".into(),
            "0.0.0.0".into(),
        ])
        .unwrap();
        assert!(c.token.as_ref().is_some_and(|t| t.len() == 32));
        let c = parse(&[d.to_string_lossy().into_owned()]).unwrap();
        assert!(c.token.is_none() && c.bind.is_loopback());
        assert!(parse(&[d.to_string_lossy().into_owned(), "--token".into(), "short".into()]).is_err());
    }
}
