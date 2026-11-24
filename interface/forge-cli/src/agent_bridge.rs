//! `forgecad web DIR --agent`: the page's agent is the person's own Claude Code, run here in DIR
//! with their own login, settings, CLAUDE.md and permissions, as `claude` runs in a terminal
//! (research/agent-tab/bridge-threat-model.md). The page drives it:
//!
//!   POST /api/agent/start   {"file"?: REL, "mode"?: M}   starts the session (or finds it running)
//!   GET  /api/agent/events?from=N                        its stream-json lines from the Nth, as
//!                                                       they come (a streamed response, a line each)
//!   POST /api/agent/send    {"text", "pictures"?: [{"media", "data": B64}]}
//!                                                       a message, and its pictures (PNG, JPEG,
//!                                                       GIF or WebP, base64)
//!   POST /api/agent/answer  {"id", "answer"}             a permission: allow | always | deny
//!   POST /api/agent/mode    {"mode"}                     default | acceptEdits | plan
//!   POST /api/agent/stop    {}                           interrupt the turn
//!
//! Every call needs the agent's token (`Authorization: Bearer T`), made at start and printed in
//! the URL once; it's never on disk and never a cookie. Writes are JSON only, and fields other than
//! these are refused: the page never names a program, an argument, a folder, an environment
//! variable, an MCP server or a system prompt. Nothing here reads Claude Code's login.
//!
//! Besides Claude Code's own lines the stream carries the bridge's, typed `forge_*`:
//! `{"type":"forge_file","path":REL,"text":T,"etag":E}` for a project file Claude Code changed or
//! made (read after each tool and at a turn's end, so the page's copy and its undo follow), and
//! `{"type":"forge_bridge","error":E}` when the session ends badly.

use forge_agent::json::Json;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime};

/// The most lines kept for a page that reconnects; older ones are let go.
const KEEP_LINES: usize = 20_000;
/// A session with no page reading it this long is ended.
const IDLE: Duration = Duration::from_secs(600);
/// Another page may take the session over once its owner's stream has been closed this long.
const OWNER_GONE: Duration = Duration::from_secs(60);
/// A project file larger than this isn't sent to the page as it changes.
const FILE_BYTES: u64 = 4 << 20;
/// The modes the page may choose; anything else (a bypass) is the person's own settings' to set.
pub const MODES: [&str; 3] = ["default", "acceptEdits", "plan"];

pub const INSTALL: &str = "curl -fsSL https://claude.ai/install.sh | bash";

pub struct Agent {
    pub token: String,
    pub root: PathBuf,
    /// The program: `claude` on PATH, or `FORGECAD_CLAUDE` (the tests' stand-in).
    pub program: String,
    /// This binary, for the session's MCP server (`forgecad mcp --root DIR --writable`).
    pub forgecad: PathBuf,
    /// A page on another origin (case 2) may run a session in a bypass mode only with this.
    pub allow_bypass: bool,
    pub cross_origin: bool,
    s: Mutex<Session>,
    cv: Condvar,
}

#[derive(Default)]
struct Session {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    /// The lines said, `base` the number of the first one kept.
    lines: Vec<String>,
    base: usize,
    /// Permissions waiting: the request's tool input and suggestions, by request id.
    asked: HashMap<String, (Json, Json)>,
    /// A turn's running: the process ending now ends it with an error.
    busy: bool,
    owner: Option<String>,
    readers: usize,
    last_read: Option<Instant>,
    mode: Option<String>,
    file: Option<String>,
    /// Bumped at each start: a reader of an ended session's stream stops.
    run: u64,
    stops: u64,
    seen: Seen,
}

/// A call's answer: its status and JSON.
pub type Answer = (u16, Json);

fn err(status: u16, e: &str) -> Answer {
    (status, Json::obj(vec![("error", e.into())]))
}

fn ok() -> Answer {
    (200, Json::obj(vec![("ok", true.into())]))
}

/// The body's fields, all of them among `allowed`, else why not.
fn fields(body: &[u8], allowed: &[&str]) -> Result<Json, Answer> {
    let text = std::str::from_utf8(body).map_err(|_| err(400, "not UTF-8"))?;
    let j = Json::parse(if text.trim().is_empty() { "{}" } else { text })
        .map_err(|e| err(400, &format!("not JSON: {e}")))?;
    let Json::Obj(kv) = &j else {
        return Err(err(400, "a JSON object"));
    };
    if let Some((k, _)) = kv.iter().find(|(k, _)| !allowed.contains(&k.as_str())) {
        return Err(err(400, &format!("`{k}`: not a field of this call")));
    }
    Ok(j)
}

/// A message's pictures, checked: each a type the models read and base64 data, nothing else.
fn pictures(j: Option<&Json>) -> Result<Vec<(&'static str, String)>, String> {
    const MEDIA: [&str; 4] = ["image/png", "image/jpeg", "image/gif", "image/webp"];
    let Some(j) = j else { return Ok(vec![]) };
    let Json::Arr(all) = j else {
        return Err("pictures: a list".into());
    };
    all.iter()
        .map(|p| {
            let Json::Obj(kv) = p else {
                return Err("pictures: each {\"media\", \"data\"}".to_string());
            };
            if kv.iter().any(|(k, _)| k != "media" && k != "data") {
                return Err("pictures: each {\"media\", \"data\"} only".into());
            }
            let media = s(p, "media").unwrap_or("");
            let Some(media) = MEDIA.iter().find(|m| **m == media) else {
                return Err(format!("pictures: {media:?} isn't PNG, JPEG, GIF or WebP"));
            };
            let data = s(p, "data").unwrap_or("");
            if data.is_empty()
                || !data
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"+/=".contains(&b))
            {
                return Err("pictures: data must be base64".into());
            }
            Ok((*media, data.to_string()))
        })
        .collect()
}

fn s<'a>(j: &'a Json, k: &str) -> Option<&'a str> {
    j.get(k).and_then(Json::str)
}

impl Agent {
    pub fn new(root: PathBuf, token: String, cross_origin: bool, allow_bypass: bool) -> Agent {
        Agent {
            token,
            root,
            program: std::env::var("FORGECAD_CLAUDE").unwrap_or_else(|_| "claude".into()),
            forgecad: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("forgecad")),
            allow_bypass,
            cross_origin,
            s: Mutex::new(Session::default()),
            cv: Condvar::new(),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Session> {
        self.s.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The command line, whole: nothing in it comes from the page but a mode among `MODES`.
    pub fn args(&self, mode: Option<&str>) -> Vec<String> {
        let mcp = Json::obj(vec![(
            "mcpServers",
            Json::obj(vec![(
                "forgecad",
                Json::obj(vec![
                    ("type", "stdio".into()),
                    ("command", self.forgecad.display().to_string().into()),
                    (
                        "args",
                        Json::Arr(vec![
                            "mcp".into(),
                            "--root".into(),
                            self.root.display().to_string().into(),
                            "--writable".into(),
                        ]),
                    ),
                ]),
            )]),
        )])
        .to_string();
        let mut a: Vec<String> = [
            "-p",
            "--output-format",
            "stream-json",
            "--input-format",
            "stream-json",
            "--verbose",
            "--include-partial-messages",
            "--permission-prompt-tool",
            "stdio",
            "--mcp-config",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        a.push(mcp);
        a.push("--append-system-prompt".into());
        a.push(system_prompt(&self.root));
        if let Some(m) = mode {
            a.extend(["--permission-mode".into(), m.to_string()]);
        }
        a
    }

    /// A call to `/api/agent/NAME` with its body; `client` is the page's own id
    /// (`X-Forge-Client`), for who owns the session.
    pub fn call(self: &Arc<Self>, name: &str, body: &[u8], client: &str) -> Answer {
        match name {
            "start" => self.start(body, client),
            "send" => self.send(body, client),
            "answer" => self.answer(body, client),
            "mode" => self.mode(body, client),
            "stop" => self.stop(body, client),
            _ => err(404, "no such call"),
        }
    }

    /// The session's owner is `client`, or none is and it takes it, or its owner's been gone a
    /// while.
    fn owns(&self, st: &mut Session, client: &str) -> bool {
        match &st.owner {
            Some(o) if o == client => true,
            Some(_) if st.readers > 0 || st.last_read.is_some_and(|t| t.elapsed() < OWNER_GONE) => false,
            _ => {
                st.owner = Some(client.to_string());
                true
            }
        }
    }

    fn start(self: &Arc<Self>, body: &[u8], client: &str) -> Answer {
        let j = match fields(body, &["file", "mode"]) {
            Ok(j) => j,
            Err(e) => return e,
        };
        let mode = s(&j, "mode");
        if let Some(m) = mode
            && !MODES.contains(&m)
        {
            return err(
                400,
                "mode: default, acceptEdits or plan (a bypass is the person's own settings' to set)",
            );
        }
        let file = match s(&j, "file") {
            Some(f) => match crate::web::clean_project(f) {
                Ok(p) => Some(p.display().to_string()),
                Err(e) => return err(400, &format!("file: {e}")),
            },
            None => None,
        };
        let mut st = self.lock();
        if !self.owns(&mut st, client) {
            return err(
                409,
                "another page has this agent: close it there, or wait a minute after it closes",
            );
        }
        st.file = file;
        let running = st
            .child
            .as_mut()
            .is_some_and(|c| c.try_wait().ok().flatten().is_none());
        if running {
            return (
                200,
                Json::obj(vec![
                    ("from", (st.base + st.lines.len()).into()),
                    ("running", true.into()),
                ]),
            );
        }
        st.mode = mode.map(str::to_string);
        let mut c = Command::new(&self.program);
        c.args(self.args(mode))
            .current_dir(&self.root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // only the markers of a Claude Code session above this one: the person's own settings,
        // tools and login are Claude Code's to load, untouched
        for k in [
            "CLAUDECODE",
            "CLAUDE_CODE_ENTRYPOINT",
            "CLAUDE_CODE_SESSION_ID",
            "AI_AGENT",
        ] {
            c.env_remove(k);
        }
        let mut child = match c.spawn() {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return (
                    503,
                    Json::obj(vec![
                        ("error", "Claude Code isn't installed on this machine.".into()),
                        ("fix", INSTALL.into()),
                    ]),
                );
            }
            Err(e) => return err(503, &format!("Claude Code didn't start: {e}")),
        };
        let out = child.stdout.take().expect("piped");
        let errs = child.stderr.take().expect("piped");
        st.stdin = child.stdin.take();
        st.child = Some(child);
        st.run += 1;
        st.busy = false;
        st.asked.clear();
        let run = st.run;
        let from = st.base + st.lines.len();
        st.seen = scan(&self.root, &HashMap::new(), true).1;
        drop(st);
        let said = Arc::new(Mutex::new(String::new()));
        let said2 = said.clone();
        let errs_read = std::thread::spawn(move || {
            for l in BufReader::new(errs).lines().map_while(Result::ok) {
                let mut s = said2.lock().unwrap_or_else(|e| e.into_inner());
                *s = l;
            }
        });
        let me = self.clone();
        std::thread::spawn(move || me.read(out, run, said, errs_read));
        (
            200,
            Json::obj(vec![("from", from.into()), ("running", false.into())]),
        )
    }

    /// Claude Code's lines, kept and told; the project's changed files after each tool.
    fn read(
        &self,
        out: std::process::ChildStdout,
        run: u64,
        said: Arc<Mutex<String>>,
        errs_read: std::thread::JoinHandle<()>,
    ) {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            let j = Json::parse(line.trim()).ok();
            let kind = j.as_ref().and_then(|j| s(j, "type")).unwrap_or("").to_string();
            let mut st = self.lock();
            if st.run != run {
                return;
            }
            if let Some(j) = &j {
                // a bypass on another origin's page, unless the bridge was started to allow it
                let bypass = j.get("permissionMode").and_then(Json::str) == Some("bypassPermissions");
                if bypass && self.cross_origin && !self.allow_bypass {
                    self.push(&mut st, line.clone());
                    self.push(
                        &mut st,
                        bridge_error(
                            "Your Claude settings bypass permissions, and this page is on another origin: start forgecad web with --allow-bypass to allow that, or change your settings",
                        ),
                    );
                    if let Some(c) = &mut st.child {
                        let _ = c.kill();
                    }
                    st.child = None;
                    st.stdin = None;
                    st.busy = false;
                    self.cv.notify_all();
                    return;
                }
                if kind == "control_request"
                    && let Some(r) = j.get("request")
                    && s(r, "subtype") == Some("can_use_tool")
                {
                    let id = s(j, "request_id").unwrap_or("").to_string();
                    let input = r.get("input").cloned().unwrap_or(Json::Obj(vec![]));
                    let sugg = r
                        .get("permission_suggestions")
                        .cloned()
                        .unwrap_or(Json::Arr(vec![]));
                    st.asked.insert(id, (input, sugg));
                }
            }
            self.push(&mut st, line);
            // after a tool, and at a turn's end: what it changed in the project
            if kind == "user" || kind == "result" {
                let (changed, seen) = scan(&self.root, &st.seen, false);
                st.seen = seen;
                for (path, text, etag) in changed {
                    let l = Json::obj(vec![
                        ("type", "forge_file".into()),
                        ("path", path.into()),
                        ("text", text.into()),
                        ("etag", etag.into()),
                    ])
                    .to_string();
                    self.push(&mut st, l);
                }
            }
            if kind == "result" {
                st.busy = false;
                st.asked.clear();
            }
            self.cv.notify_all();
        }
        // ended: its last word on stderr read first (a second at most: a process it started may
        // hold stderr open), then, mid-turn, said as the turn's failure
        let t = Instant::now();
        while !errs_read.is_finished() && t.elapsed() < Duration::from_secs(1) {
            std::thread::sleep(Duration::from_millis(10));
        }
        let mut st = self.lock();
        if st.run != run {
            return;
        }
        st.child = None;
        st.stdin = None;
        st.asked.clear();
        if std::mem::take(&mut st.busy) {
            let last = said.lock().unwrap_or_else(|e| e.into_inner()).clone();
            let why = if last.is_empty() {
                "Claude Code stopped".to_string()
            } else {
                format!("Claude Code stopped: {last}")
            };
            let l = Json::obj(vec![
                ("type", "result".into()),
                ("is_error", true.into()),
                ("result", why.into()),
            ])
            .to_string();
            self.push(&mut st, l);
        }
        self.cv.notify_all();
    }

    fn push(&self, st: &mut Session, line: String) {
        st.lines.push(line);
        if st.lines.len() > KEEP_LINES {
            let cut = st.lines.len() - KEEP_LINES;
            st.lines.drain(..cut);
            st.base += cut;
        }
    }

    fn write(st: &mut Session, j: &Json) -> bool {
        let Some(w) = &mut st.stdin else { return false };
        let mut line = j.to_string();
        line.push('\n');
        w.write_all(line.as_bytes()).and_then(|_| w.flush()).is_ok()
    }

    fn send(&self, body: &[u8], client: &str) -> Answer {
        let j = match fields(body, &["text", "pictures"]) {
            Ok(j) => j,
            Err(e) => return e,
        };
        let Some(text) = s(&j, "text") else {
            return err(400, "text: the message");
        };
        let pictures = match pictures(j.get("pictures")) {
            Ok(p) => p,
            Err(e) => return err(400, &e),
        };
        let mut st = self.lock();
        if !self.owns(&mut st, client) {
            return err(409, "another page has this agent");
        }
        // the pictures first, as image blocks the text reads after
        let content = if pictures.is_empty() {
            text.into()
        } else {
            let mut blocks: Vec<Json> = pictures
                .into_iter()
                .map(|(media, data)| {
                    Json::obj(vec![
                        ("type", "image".into()),
                        (
                            "source",
                            Json::obj(vec![
                                ("type", "base64".into()),
                                ("media_type", media.into()),
                                ("data", data.into()),
                            ]),
                        ),
                    ])
                })
                .collect();
            blocks.push(Json::obj(vec![("type", "text".into()), ("text", text.into())]));
            Json::Arr(blocks)
        };
        let msg = Json::obj(vec![
            ("type", "user".into()),
            (
                "message",
                Json::obj(vec![("role", "user".into()), ("content", content)]),
            ),
        ]);
        if !Self::write(&mut st, &msg) {
            return err(409, "no session running: start one");
        }
        st.busy = true;
        ok()
    }

    fn answer(&self, body: &[u8], client: &str) -> Answer {
        let j = match fields(body, &["id", "answer"]) {
            Ok(j) => j,
            Err(e) => return e,
        };
        let (Some(id), Some(a)) = (s(&j, "id"), s(&j, "answer")) else {
            return err(400, "id and answer");
        };
        let mut st = self.lock();
        if !self.owns(&mut st, client) {
            return err(409, "another page has this agent");
        }
        let Some((input, sugg)) = st.asked.remove(id) else {
            return err(404, "no such question waiting");
        };
        let response = match a {
            "allow" => Json::obj(vec![("behavior", "allow".into()), ("updatedInput", input)]),
            "always" => Json::obj(vec![
                ("behavior", "allow".into()),
                ("updatedInput", input),
                ("updatedPermissions", sugg),
            ]),
            "deny" => Json::obj(vec![
                ("behavior", "deny".into()),
                ("message", "The person declined this in the app.".into()),
            ]),
            _ => {
                st.asked.insert(id.to_string(), (input, sugg));
                return err(400, "answer: allow, always or deny");
            }
        };
        let msg = Json::obj(vec![
            ("type", "control_response".into()),
            (
                "response",
                Json::obj(vec![
                    ("subtype", "success".into()),
                    ("request_id", id.into()),
                    ("response", response),
                ]),
            ),
        ]);
        if Self::write(&mut st, &msg) {
            ok()
        } else {
            err(409, "no session running")
        }
    }

    fn control(st: &mut Session, subtype: &str, more: Vec<(&str, Json)>) -> bool {
        st.stops += 1;
        let mut r = vec![("subtype", subtype.into())];
        r.extend(more);
        let msg = Json::obj(vec![
            ("type", "control_request".into()),
            ("request_id", format!("bridge-{}", st.stops).into()),
            ("request", Json::obj(r)),
        ]);
        Self::write(st, &msg)
    }

    fn mode(&self, body: &[u8], client: &str) -> Answer {
        let j = match fields(body, &["mode"]) {
            Ok(j) => j,
            Err(e) => return e,
        };
        let Some(m) = s(&j, "mode").filter(|m| MODES.contains(m)) else {
            return err(400, "mode: default, acceptEdits or plan");
        };
        let mut st = self.lock();
        if !self.owns(&mut st, client) {
            return err(409, "another page has this agent");
        }
        st.mode = Some(m.to_string());
        Self::control(&mut st, "set_permission_mode", vec![("mode", m.into())]);
        ok()
    }

    fn stop(&self, body: &[u8], client: &str) -> Answer {
        if let Err(e) = fields(body, &[]) {
            return e;
        }
        let mut st = self.lock();
        if !self.owns(&mut st, client) {
            return err(409, "another page has this agent");
        }
        if !Self::control(&mut st, "interrupt", vec![]) {
            return err(409, "no session running");
        }
        ok()
    }

    /// The stream: each line from `from` on, then each as it comes, to `w` (a chunk a line),
    /// until the page goes or the session's restarted. An empty chunk-line every 15 s finds a
    /// page that's gone.
    pub fn stream(&self, from: usize, client: &str, mut w: impl FnMut(&[u8]) -> std::io::Result<()>) {
        let mut next = from;
        let run = {
            let mut st = self.lock();
            if st.owner.as_deref().is_some_and(|o| o != client) && !self.owns(&mut st, client) {
                return;
            }
            st.readers += 1;
            st.run
        };
        loop {
            let (lines, ended) = {
                let mut st = self.lock();
                let mut waited = Duration::ZERO;
                while st.base + st.lines.len() <= next && st.run == run && waited < Duration::from_secs(15) {
                    let t = Instant::now();
                    st = self
                        .cv
                        .wait_timeout(st, Duration::from_secs(15) - waited)
                        .unwrap_or_else(|e| e.into_inner())
                        .0;
                    waited += t.elapsed();
                }
                let start = next.max(st.base) - st.base;
                let lines: Vec<String> = st.lines[start.min(st.lines.len())..].to_vec();
                // a session started again has a stream of its own (`start` says where it begins)
                (lines, st.run != run)
            };
            let mut out = Vec::new();
            for l in &lines {
                out.extend_from_slice(l.as_bytes());
                out.push(b'\n');
            }
            if out.is_empty() {
                out.push(b'\n');
            }
            next += lines.len();
            if w(&out).is_err() || ended {
                break;
            }
        }
        let mut st = self.lock();
        st.readers = st.readers.saturating_sub(1);
        st.last_read = Some(Instant::now());
    }

    /// Ends a session no page has read for `IDLE`. Called now and then.
    pub fn reap(&self) {
        let mut st = self.lock();
        let idle = st.readers == 0 && st.last_read.is_some_and(|t| t.elapsed() > IDLE);
        if idle && let Some(mut c) = st.child.take() {
            let _ = c.kill();
            st.stdin = None;
            eprintln!("forgecad web: ended the agent's session: no page had read it for 10 minutes");
        }
    }
}

impl Drop for Agent {
    fn drop(&mut self) {
        if let Some(mut c) = self.lock().child.take() {
            let _ = c.kill();
        }
    }
}

fn bridge_error(e: &str) -> String {
    Json::obj(vec![("type", "forge_bridge".into()), ("error", e.into())]).to_string()
}

/// The project's files (as the file API serves them: no dot files) changed since `seen`: each
/// one's path, text and ETag; and what's seen now (`first`: only that). A file whose size and
/// time are as seen isn't read again (one this fresh is: coarse times). Binary and large files
/// aren't sent.
type Seen = HashMap<String, (u64, Option<SystemTime>, String)>;

fn scan(root: &Path, seen: &Seen, first: bool) -> (Vec<(String, String, String)>, Seen) {
    let now = SystemTime::now();
    let mut out = vec![];
    let mut next = HashMap::new();
    let mut dirs = vec![root.to_path_buf()];
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
                continue;
            }
            if !ft.is_file() {
                continue;
            }
            let Ok(m) = e.metadata() else { continue };
            if m.len() > FILE_BYTES {
                continue;
            }
            let rel: Vec<String> = p
                .strip_prefix(root)
                .unwrap_or(&p)
                .components()
                .map(|x| x.as_os_str().to_string_lossy().into_owned())
                .collect();
            let rel = rel.join("/");
            let modified = m.modified().ok();
            let fresh = modified
                .and_then(|t| now.duration_since(t).ok())
                .is_none_or(|age| age < Duration::from_secs(3));
            if let Some(was) = seen.get(&rel)
                && was.0 == m.len()
                && was.1 == modified
                && modified.is_some()
                && !fresh
            {
                next.insert(rel, was.clone());
                continue;
            }
            let Ok(bytes) = std::fs::read(&p) else { continue };
            let etag = crate::web::etag(&bytes);
            let same = seen.get(&rel).is_some_and(|w| w.2 == etag);
            next.insert(rel.clone(), (m.len(), modified, etag.clone()));
            if !same
                && !first
                && let Ok(text) = String::from_utf8(bytes)
            {
                out.push((rel, text, etag));
            }
        }
    }
    out.sort();
    (out, next)
}

/// What the agent is told it is, appended to Claude Code's own system prompt.
fn system_prompt(root: &Path) -> String {
    format!(
        "You are ForgeCAD's agent, driven from the ForgeCAD app in a web browser. The project folder is {}; \
         the person's message says which .fg file is open in the app. The app names the project's files \
         /project/REL: that is REL in the project folder. Make the change asked by editing the \
         files in place: the page shows each change as a diff with Undo. A .fg file is a Forge part, \
         assembly or drawing (`forgecad help forge`, or the forgecad MCP server's tools: build, check, \
         edit with typed ops, query, measure). Check your work by building the file. Reply in one to three \
         short sentences unless asked for more: what you changed, and any new error or warning.",
        root.display()
    )
}
