//! Claude Code, headless: `claude -p` reading and writing stream-json, one process per chat kept
//! open across turns (resumed by its session id if it ends), David's own login and settings.
// desktop only: a page starts no CLI (std's threads and sleeps never run there)
#![cfg_attr(target_arch = "wasm32", allow(clippy::disallowed_methods))]

use super::{Backend, Event, Out, Watch, clean_env};
use crate::shell::agent::{Answer, Mode, present, tool_summary};
use forge_agent::json::Json;
use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub const INSTALL: &str = "curl -fsSL https://claude.ai/install.sh | bash";
pub const LOGIN: &str = "claude auth login";

/// The stream-json lines a turn writes, as the transcript's events.
#[derive(Default)]
pub struct Parser {
    pub session: Option<String>,
    pub model: Option<String>,
    /// Messages whose text came a piece at a time: their whole is said already.
    streamed: HashSet<String>,
    tools: HashSet<String>,
    /// This turn already said why it failed.
    failed: bool,
    /// Permissions asked since last taken: the request's id, the tool's input, and the
    /// suggestions "Always" would apply.
    pub asked: Vec<(String, Json, Json)>,
}

/// What "Always" would allow, from the request's `permission_suggestions`, in words.
pub fn always_words(suggestions: &Json) -> Option<String> {
    let place = |d: &str| match d {
        "session" => "for this chat",
        "localSettings" => "in this project, on this computer",
        "projectSettings" => "in this project",
        "userSettings" => "in every project",
        _ => "",
    };
    let said: Vec<String> = arr(Some(suggestions))
        .iter()
        .filter_map(|s| match str_of(s, "type") {
            "setMode" => Some(match str_of(s, "mode") {
                "acceptEdits" => format!("Accept edits {}", place(str_of(s, "destination"))),
                m => format!("Switch to {m} {}", place(str_of(s, "destination"))),
            }),
            "addRules" if str_of(s, "behavior") == "allow" => {
                let rules: Vec<String> = arr(s.get("rules"))
                    .iter()
                    .map(|r| match str_of(r, "ruleContent") {
                        "" => str_of(r, "toolName").to_string(),
                        c => format!("{}({c})", str_of(r, "toolName")),
                    })
                    .collect();
                (!rules.is_empty())
                    .then(|| format!("Allow {} {}", rules.join(", "), place(str_of(s, "destination"))))
            }
            "addDirectories" => Some(format!(
                "Allow {} {}",
                arr(s.get("directories"))
                    .iter()
                    .filter_map(Json::str)
                    .collect::<Vec<_>>()
                    .join(", "),
                place(str_of(s, "destination"))
            )),
            _ => None,
        })
        .map(|w| w.trim().to_string())
        .collect();
    (!said.is_empty()).then(|| said.join("; "))
}

fn get<'a>(j: &'a Json, k: &str) -> Option<&'a Json> {
    j.get(k)
}

fn str_of<'a>(j: &'a Json, k: &str) -> &'a str {
    j.get(k).and_then(Json::str).unwrap_or("")
}

fn arr(j: Option<&Json>) -> &[Json] {
    match j {
        Some(Json::Arr(a)) => a,
        _ => &[],
    }
}

/// A tool result's text: a string, or text blocks.
fn content_text(c: Option<&Json>) -> String {
    match c {
        Some(Json::Str(s)) => s.clone(),
        Some(Json::Arr(a)) => a
            .iter()
            .filter_map(|b| b.get("text").and_then(Json::str))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

impl Parser {
    pub fn line(&mut self, line: &str) -> Vec<Event> {
        let Ok(d) = Json::parse(line.trim()) else {
            return vec![];
        };
        let mut out = Vec::new();
        // a subagent's own work stays inside its tool row
        if get(&d, "parent_tool_use_id").is_some_and(|p| *p != Json::Null) {
            return out;
        }
        match str_of(&d, "type") {
            "system" if str_of(&d, "subtype") == "init" => {
                self.session = Some(str_of(&d, "session_id").to_string());
                self.model = Some(str_of(&d, "model").to_string());
                if let Some(m) = d.get("permissionMode").and_then(Json::str) {
                    out.push(Event::Mode(m.to_string()));
                }
                // the commands it takes in a message: the person's skills, compact, context...
                let commands: Vec<String> = arr(get(&d, "slash_commands"))
                    .iter()
                    .filter_map(Json::str)
                    .map(str::to_string)
                    .collect();
                if !commands.is_empty() {
                    out.push(Event::Commands(commands));
                }
            }
            "system" if str_of(&d, "subtype") == "status" => {
                if let Some(m) = d.get("permissionMode").and_then(Json::str) {
                    out.push(Event::Mode(m.to_string()));
                }
            }
            // refused without asking (a rule, or the mode): said, not just a red dot
            "system" if str_of(&d, "subtype") == "permission_denied" => out.push(Event::Note {
                text: format!("Not allowed: {}", str_of(&d, "message")),
                error: true,
            }),
            "control_request" => {
                let Some(r) = get(&d, "request") else { return out };
                if str_of(r, "subtype") == "can_use_tool" {
                    let id = str_of(&d, "request_id").to_string();
                    let tool = str_of(r, "tool_name").to_string();
                    let input = r.get("input").cloned().unwrap_or(Json::Obj(vec![]));
                    let suggestions = r
                        .get("permission_suggestions")
                        .cloned()
                        .unwrap_or(Json::Arr(vec![]));
                    let (summary, whole) = tool_summary(&tool, &input);
                    let summary = present(&summary);
                    out.push(Event::Permission {
                        id: id.clone(),
                        tool,
                        tool_use: str_of(r, "tool_use_id").to_string(),
                        summary,
                        input: whole,
                        always: always_words(&suggestions),
                    });
                    self.asked.push((id, input, suggestions));
                }
            }
            "stream_event" => {
                let Some(e) = get(&d, "event") else { return out };
                match str_of(e, "type") {
                    "content_block_start" => {
                        let b = get(e, "content_block");
                        if let Some(b) = b
                            && str_of(b, "type") == "tool_use"
                        {
                            let id = str_of(b, "id").to_string();
                            self.tools.insert(id.clone());
                            out.push(Event::ToolStart {
                                id,
                                name: str_of(b, "name").to_string(),
                            });
                        }
                    }
                    "content_block_delta" => {
                        if let Some(delta) = get(e, "delta")
                            && str_of(delta, "type") == "text_delta"
                        {
                            self.streamed.insert(str_of(&d, "api_message_id").to_string());
                            out.push(Event::Text(str_of(delta, "text").to_string()));
                        }
                    }
                    _ => {}
                }
            }
            "assistant" => {
                let Some(m) = get(&d, "message") else { return out };
                let text = arr(get(m, "content"))
                    .iter()
                    .filter_map(|b| b.get("text").and_then(Json::str))
                    .collect::<Vec<_>>()
                    .join("\n");
                match str_of(&d, "error") {
                    "" => {}
                    "authentication_failed" => {
                        self.failed = true;
                        out.push(Event::Problem {
                            text: "Claude Code isn't signed in.".into(),
                            fix: Some(LOGIN.into()),
                        });
                        return out;
                    }
                    _ => {
                        self.failed = true;
                        out.push(Event::Note { text, error: true });
                        return out;
                    }
                }
                let streamed = self.streamed.contains(str_of(m, "id"));
                for b in arr(get(m, "content")) {
                    match str_of(b, "type") {
                        "text" if !streamed => out.push(Event::Text(str_of(b, "text").to_string())),
                        "tool_use" => {
                            let id = str_of(b, "id").to_string();
                            let name = str_of(b, "name").to_string();
                            if self.tools.insert(id.clone()) {
                                out.push(Event::ToolStart {
                                    id: id.clone(),
                                    name: name.clone(),
                                });
                            }
                            let (summary, input) = tool_summary(&name, b.get("input").unwrap_or(&Json::Null));
                            out.push(Event::ToolInput { id, summary, input });
                        }
                        _ => {}
                    }
                }
            }
            "user" => {
                let Some(m) = get(&d, "message") else { return out };
                for b in arr(get(m, "content")) {
                    if str_of(b, "type") == "tool_result" {
                        out.push(Event::ToolDone {
                            id: str_of(b, "tool_use_id").to_string(),
                            output: content_text(b.get("content")),
                            error: b.get("is_error") == Some(&Json::Bool(true)),
                        });
                    }
                }
            }
            "result" => {
                let stopped = str_of(&d, "terminal_reason") == "aborted_streaming";
                let is_error = d.get("is_error") == Some(&Json::Bool(true));
                let error = (is_error && !stopped && !self.failed).then(|| {
                    let r = str_of(&d, "result");
                    if r.is_empty() {
                        arr(get(&d, "errors"))
                            .iter()
                            .filter_map(Json::str)
                            .collect::<Vec<_>>()
                            .join("; ")
                    } else {
                        r.to_string()
                    }
                });
                if let Some(s) = d.get("session_id").and_then(Json::str) {
                    self.session = Some(s.to_string());
                }
                self.failed = false;
                out.push(Event::Done { error, stopped });
            }
            _ => {}
        }
        out
    }
}

pub struct Config {
    pub program: String,
    /// `--model`; empty for Claude Code's default.
    pub model: String,
    /// Open document, or a project directory before the first document is created.
    pub file: PathBuf,
    pub system: String,
    /// Project working directory; tests may provide an isolated directory.
    pub cwd: PathBuf,
    /// Additional MCP configuration for the app tools.
    pub mcp: Option<String>,
}

/// Legacy working directory, retained for importing conversations from older installs.
pub fn clean_cwd() -> PathBuf {
    let base = forge_dirs::state().unwrap_or_else(forge_dirs::temp_dir);
    base.join("forgecad").join("agent")
}

pub struct ClaudeCode {
    cfg: Config,
    root: PathBuf,
    out: Out,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    /// The chat's session, for `--resume` when the process has to start again.
    pub session: Arc<Mutex<Option<String>>>,
    alive: Arc<Mutex<bool>>,
    /// A turn's running: the process ending now ends it with an error.
    busy: Arc<AtomicBool>,
    stops: u64,
    watch: Arc<Mutex<Watch>>,
    /// Permissions waiting on the person, by request id: the tool's input and the suggestions.
    asked: Arc<Mutex<std::collections::HashMap<String, (Json, Json)>>>,
    /// What may run without asking, when the person chose it in the tab: `--permission-mode`
    /// when it starts, and sent when it changes. None: their own settings' `defaultMode`.
    mode: Option<Mode>,
}

impl ClaudeCode {
    pub fn new(cfg: Config, out: Out, session: Option<String>) -> ClaudeCode {
        let root = super::project::root(&cfg.file);
        let watch = Arc::new(Mutex::new(Watch::new(&root)));
        ClaudeCode {
            cfg,
            root,
            watch,
            out,
            child: None,
            stdin: None,
            session: Arc::new(Mutex::new(session)),
            alive: Arc::new(Mutex::new(false)),
            busy: Arc::new(AtomicBool::new(false)),
            stops: 0,
            asked: Default::default(),
            mode: None,
        }
    }

    /// The command line, as run. Only David's user settings load (no project or local
    /// settings, so no project hooks), only ForgeCAD's MCP server, no Bash, and in `dontAsk`
    /// mode only what's allowed here runs: the folder's files, and the app's tools (fg among them).
    pub fn args(&self) -> Vec<String> {
        let mut a: Vec<String> = [
            "-p",
            "--output-format",
            "stream-json",
            "--input-format",
            "stream-json",
            "--verbose",
            "--include-partial-messages",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        // What would prompt in a terminal asks the app (a `can_use_tool` control request) instead
        // of being refused unseen; the person's own allow and deny rules still apply first.
        a.extend(["--permission-prompt-tool".into(), "stdio".into()]);
        if let Some(m) = self.mode {
            a.extend(["--permission-mode".into(), m.flag().into()]);
        }
        // Add the app connection alongside the user's configured tools and permissions.
        if let Some(mcp) = &self.cfg.mcp {
            a.extend(["--mcp-config".into(), mcp.clone()]);
        }
        a.push("--append-system-prompt".into());
        a.push(self.cfg.system.clone());
        if !self.cfg.model.trim().is_empty() {
            a.push("--model".into());
            a.push(self.cfg.model.trim().into());
        }
        if let Some(s) = self.session.lock().expect("no panics while held").clone() {
            a.push("--resume".into());
            a.push(s);
        }
        a
    }

    // the error is the transcript's event saying why (sent once, at most once a turn): its size
    // is the event's, not worth a box
    #[allow(clippy::result_large_err)]
    fn spawn(&mut self) -> Result<(), Event> {
        let dir = self.cfg.cwd.clone();
        if let Err(e) = forge_dirs::fs::create_dir_all(&dir) {
            return Err(Event::Problem {
                text: format!("No folder for the agent to run in: {}: {e}", dir.display()),
                fix: None,
            });
        }
        let mut c = Command::new(&self.cfg.program);
        c.args(self.args())
            .current_dir(&dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        clean_env(&mut c);
        // the folder as it is before the agent can touch it: a write in its first moment is a change
        let watch = self.watch.clone();
        let mut child = c.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Event::Problem {
                    text: "Claude Code isn't installed.".into(),
                    fix: Some(INSTALL.into()),
                }
            } else {
                Event::Problem {
                    text: format!("Claude Code didn't start: {e}"),
                    fix: None,
                }
            }
        })?;
        let stdout = child.stdout.take().expect("piped");
        let stderr = child.stderr.take().expect("piped");
        self.stdin = child.stdin.take();
        self.child = Some(child);
        *self.alive.lock().expect("no panics while held") = true;
        let said = Arc::new(Mutex::new(String::new()));
        let said2 = said.clone();
        std::thread::Builder::new()
            .name("agent-claude-err".into())
            .spawn(move || {
                for l in BufReader::new(stderr).lines().map_while(Result::ok) {
                    let mut s = said2.lock().expect("no panics while held");
                    s.push_str(&l);
                    s.push('\n');
                    if s.len() > 4000 {
                        let cut = s.len() - 4000;
                        let cut = (cut..s.len()).find(|k| s.is_char_boundary(*k)).unwrap_or(0);
                        s.drain(..cut);
                    }
                }
            })
            .expect("a thread");
        let (out, session, alive, busy, asked) = (
            self.out.clone(),
            self.session.clone(),
            self.alive.clone(),
            self.busy.clone(),
            self.asked.clone(),
        );
        std::thread::Builder::new()
            .name("agent-claude".into())
            .spawn(move || {
                let mut parser = Parser::default();
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    let said = parser.line(&line);
                    if !parser.asked.is_empty() {
                        let mut a = asked.lock().expect("no panics while held");
                        for (id, input, sugg) in parser.asked.drain(..) {
                            a.insert(id, (input, sugg));
                        }
                    }
                    for e in said {
                        let after = matches!(e, Event::ToolDone { .. } | Event::Done { .. });
                        if after {
                            for c in watch.lock().expect("watch").check() {
                                out.send(c);
                            }
                        }
                        if matches!(e, Event::Done { .. } | Event::Problem { .. }) {
                            busy.store(false, Ordering::SeqCst);
                            // a turn's over: nothing it asked is waiting any more
                            asked.lock().expect("no panics while held").clear();
                        }
                        out.send(e);
                    }
                    if let Some(s) = &parser.session {
                        *session.lock().expect("no panics while held") = Some(s.clone());
                    }
                }
                *alive.lock().expect("no panics while held") = false;
                for c in watch.lock().expect("watch").check() {
                    out.send(c);
                }
                if !busy.swap(false, Ordering::SeqCst) {
                    return;
                }
                // ended mid-turn: say why, from what it wrote on stderr
                std::thread::sleep(std::time::Duration::from_millis(50));
                let err = said.lock().expect("no panics while held").trim().to_string();
                let last = err.lines().last().unwrap_or("").to_string();
                out.send(Event::Done {
                    error: Some(if last.is_empty() {
                        "Claude Code stopped".into()
                    } else {
                        format!("Claude Code stopped: {last}")
                    }),
                    stopped: false,
                });
            })
            .expect("a thread");
        Ok(())
    }

    fn write(&mut self, j: &Json) -> bool {
        let Some(w) = &mut self.stdin else { return false };
        let mut s = String::new();
        j.write(&mut s);
        s.push('\n');
        w.write_all(s.as_bytes()).and_then(|_| w.flush()).is_ok()
    }
}

impl Backend for ClaudeCode {
    fn send(&mut self, prompt: String, picture: Option<Vec<u8>>) {
        *self.watch.lock().expect("watch") = Watch::new(&self.root);
        // a picture goes first, as an image block the message reads after
        let content = match picture {
            None => prompt.into(),
            Some(png) => Json::Arr(vec![
                Json::obj(vec![
                    ("type", "image".into()),
                    (
                        "source",
                        Json::obj(vec![
                            ("type", "base64".into()),
                            ("media_type", "image/png".into()),
                            ("data", super::apptools::base64(&png).into()),
                        ]),
                    ),
                ]),
                Json::obj(vec![("type", "text".into()), ("text", prompt.into())]),
            ]),
        };
        let msg = Json::obj(vec![
            ("type", "user".into()),
            (
                "message",
                Json::obj(vec![("role", "user".into()), ("content", content)]),
            ),
        ]);
        self.busy.store(true, Ordering::SeqCst);
        let alive = *self.alive.lock().expect("no panics while held");
        if (!alive || !self.write(&msg))
            && let Err(e) = self.spawn().map(|_| self.write(&msg))
        {
            self.busy.store(false, Ordering::SeqCst);
            self.out.send(e);
        }
    }

    fn answer(&mut self, id: &str, a: Answer) {
        let Some((input, suggestions)) = self.asked.lock().expect("no panics while held").remove(id) else {
            return;
        };
        let response = match a {
            Answer::Allow => Json::obj(vec![("behavior", "allow".into()), ("updatedInput", input)]),
            Answer::Always => Json::obj(vec![
                ("behavior", "allow".into()),
                ("updatedInput", input),
                ("updatedPermissions", suggestions),
            ]),
            Answer::Deny => Json::obj(vec![
                ("behavior", "deny".into()),
                ("message", "The person declined this in the app.".into()),
            ]),
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
        self.write(&msg);
    }

    fn set_mode(&mut self, m: Mode) {
        self.mode = Some(m);
        self.stops += 1;
        let req = Json::obj(vec![
            ("type", "control_request".into()),
            ("request_id", format!("mode-{}", self.stops).into()),
            (
                "request",
                Json::obj(vec![
                    ("subtype", "set_permission_mode".into()),
                    ("mode", m.flag().into()),
                ]),
            ),
        ]);
        // not running yet: `--permission-mode` says it when it starts
        if *self.alive.lock().expect("no panics while held") {
            self.write(&req);
        }
    }

    fn stop(&mut self) {
        self.stops += 1;
        let req = Json::obj(vec![
            ("type", "control_request".into()),
            ("request_id", format!("stop-{}", self.stops).into()),
            ("request", Json::obj(vec![("subtype", "interrupt".into())])),
        ]);
        if !self.write(&req) {
            self.out.send(Event::Done {
                error: None,
                stopped: true,
            });
        }
    }
}

impl Drop for ClaudeCode {
    fn drop(&mut self) {
        self.stdin = None;
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            std::thread::spawn(move || c.wait());
        }
    }
}
