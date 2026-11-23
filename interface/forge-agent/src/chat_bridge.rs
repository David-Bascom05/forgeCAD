//! Local terminal-to-UI agent requests. One private socket per window; no model credentials.
use crate::ipc;
use crate::json::Json;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU32, Ordering},
};
use std::time::Duration;

#[derive(Clone)]
pub struct Request {
    pub id: String,
    pub message: String,
    pub project: String,
    pub model: String,
}
struct Record {
    request: Request,
    state: String,
    reply: String,
}
#[derive(Default)]
struct State {
    project: String,
    model: String,
    busy: bool,
    next: u64,
    records: VecDeque<Record>,
}
pub struct Bridge {
    pub id: String,
    state: Arc<Mutex<State>>,
    stop: Arc<AtomicBool>,
    path: PathBuf,
}
fn directory() -> Result<PathBuf, String> {
    let base = forge_dirs::runtime().ok_or("No user runtime directory")?;
    Ok(base.join("forgecad-chat"))
}
fn err(message: impl Into<String>) -> Json {
    Json::obj(vec![("error", message.into().into())])
}
fn string<'a>(j: &'a Json, k: &str) -> &'a str {
    j.get(k).and_then(Json::str).unwrap_or("")
}
fn record(r: &Record) -> Json {
    Json::obj(vec![
        ("request", r.request.id.clone().into()),
        ("state", r.state.clone().into()),
        ("reply", r.reply.clone().into()),
        ("project", r.request.project.clone().into()),
        ("model", r.request.model.clone().into()),
    ])
}
impl Bridge {
    pub fn start(wake: Arc<dyn Fn() + Send + Sync>) -> Result<Self, String> {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let dir = directory()?;
        ipc::private_dir(&dir).map_err(|e| e.to_string())?;
        let id = format!(
            "{}-{}",
            forge_dirs::process_id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let path = dir.join(format!("{id}.sock"));
        // a stale socket is replaced only after proving no listener owns it
        let listener = ipc::Listener::bind(&path).map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let state = Arc::new(Mutex::new(State::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let (shared, done, window) = (state.clone(), stop.clone(), id.clone());
        std::thread::spawn(move || {
            while !done.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok(mut stream) => {
                        let (shared, wake, window) = (shared.clone(), wake.clone(), window.clone());
                        std::thread::spawn(move || {
                            let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                            let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
                            let mut line = String::new();
                            if BufReader::new((&stream).take(70_000))
                                .read_line(&mut line)
                                .is_err()
                            {
                                return;
                            }
                            let answer = match Json::parse(&line) {
                                Ok(j) => answer(&window, &shared, &j),
                                Err(e) => err(e),
                            };
                            wake();
                            let _ = writeln!(stream, "{answer}");
                        });
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(30))
                    }
                    // a connection without the token (Windows): dropped, the next one heard
                    Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {}
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            id,
            state,
            stop,
            path,
        })
    }
    pub fn update(&self, project: String, model: String, busy: bool) {
        let mut s = self.state.lock().expect("bridge");
        s.project = project;
        s.model = model;
        s.busy = busy;
    }
    pub fn next(&self) -> Option<Request> {
        let mut s = self.state.lock().expect("bridge");
        let r = s.records.iter_mut().find(|r| r.state == "queued")?;
        r.state = "running".into();
        Some(r.request.clone())
    }
    pub fn finish(&self, id: &str, reply: String, failed: bool) {
        let mut s = self.state.lock().expect("bridge");
        if let Some(r) = s.records.iter_mut().find(|r| r.request.id == id) {
            r.state = if failed { "failed" } else { "done" }.into();
            r.reply = reply;
        }
    }
}
impl Drop for Bridge {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = forge_dirs::fs::remove_file(&self.path);
    }
}
fn answer(window: &str, shared: &Mutex<State>, j: &Json) -> Json {
    let mut s = shared.lock().expect("bridge");
    match string(j, "method") {
        "info" => Json::obj(vec![
            ("window", window.into()),
            ("project", s.project.clone().into()),
            ("model", s.model.clone().into()),
            ("busy", s.busy.into()),
        ]),
        "send" => {
            let message = string(j, "message").trim();
            if message.is_empty() || message.len() > 64_000 {
                return err("Message must contain 1–64000 bytes");
            }
            if s.project.is_empty() {
                return err("Window project is not ready");
            }
            if s.records
                .iter()
                .filter(|r| r.state == "queued" || r.state == "running")
                .count()
                >= 32
            {
                return err("Window request queue is full");
            }
            while s.records.len() >= 128 {
                let Some(i) = s
                    .records
                    .iter()
                    .position(|r| r.state == "done" || r.state == "failed")
                else {
                    break;
                };
                s.records.remove(i);
            }
            s.next += 1;
            let r = Record {
                request: Request {
                    id: s.next.to_string(),
                    message: message.into(),
                    project: s.project.clone(),
                    model: s.model.clone(),
                },
                state: "queued".into(),
                reply: String::new(),
            };
            let result = record(&r);
            s.records.push_back(r);
            result
        }
        "status" => s
            .records
            .iter()
            .find(|r| r.request.id == string(j, "request"))
            .map(record)
            .unwrap_or_else(|| err("Unknown or expired request")),
        _ => err("Unknown bridge method"),
    }
}
pub fn call(window: &str, request: Json) -> Result<Json, String> {
    if window.is_empty() || !window.bytes().all(|b| b.is_ascii_digit() || b == b'-') {
        return Err("Invalid window ID".into());
    }
    let mut s = ipc::connect(&directory()?.join(format!("{window}.sock"))).map_err(|e| e.to_string())?;
    s.set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    s.set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    writeln!(s, "{request}").map_err(|e| e.to_string())?;
    let mut line = String::new();
    BufReader::new(s.take(2_000_000))
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    Json::parse(&line)
}
pub fn list() -> Result<Json, String> {
    let dir = directory()?;
    if !dir.exists() {
        return Ok(Json::Arr(vec![]));
    }
    let mut windows = Vec::new();
    for entry in forge_dirs::fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .flatten()
    {
        let path = entry.path();
        if path.extension().is_none_or(|x| x != "sock") {
            continue;
        }
        let id = path.file_stem().and_then(|x| x.to_str()).unwrap_or("");
        if let Ok(info) = call(id, Json::obj(vec![("method", "info".into())])) {
            windows.push(info);
        }
    }
    Ok(Json::Arr(windows))
}
