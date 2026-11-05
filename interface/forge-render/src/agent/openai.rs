//! Any OpenAI-compatible chat completions endpoint (OpenAI, OpenRouter, llama.cpp, vLLM,
//! Ollama...) driven by ForgeCAD's own agent loop: the reply streamed, its tool calls run on the
//! open file (`tools.rs`), their answers sent back, until it answers without one.

use super::{Backend, Event, Out, Watch, tools};
use crate::shell::agent::tool_summary;
use forge_agent::json::Json;
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const MAX_STEPS: usize = 30;

#[derive(Clone)]
pub struct Config {
    /// Up to and including `/v1`.
    pub base_url: String,
    pub model: String,
    pub file: PathBuf,
    pub system: String,
    /// A key given outright (tests); else the keyring's for the URL.
    pub key: Option<String>,
}

pub struct OpenAi {
    cfg: Arc<Config>,
    out: Out,
    history: Arc<Mutex<Vec<Json>>>,
    stop: Arc<AtomicBool>,
    busy: Arc<AtomicBool>,
}

/// One streamed reply, put together.
#[derive(Default, Debug, PartialEq)]
pub struct Acc {
    pub text: String,
    /// Each call's id, function name and arguments (JSON text).
    pub calls: Vec<(String, String, String)>,
    pub finish: Option<String>,
}

fn arr(j: Option<&Json>) -> &[Json] {
    match j {
        Some(Json::Arr(a)) => a,
        _ => &[],
    }
}

impl Acc {
    /// A chunk (`data:` of the stream) or a whole reply's `message`: its text said as it comes.
    pub fn take(&mut self, j: &Json, said: &mut Vec<Event>) {
        let Some(c) = arr(j.get("choices")).first() else {
            return;
        };
        let d = c.get("delta").or_else(|| c.get("message"));
        if let Some(d) = d {
            if let Some(t) = d.get("content").and_then(Json::str)
                && !t.is_empty()
            {
                self.text.push_str(t);
                said.push(Event::Text(t.to_string()));
            }
            for (k, tc) in arr(d.get("tool_calls")).iter().enumerate() {
                let n = tc.get("index").and_then(Json::num).map_or(k, |n| n as usize);
                while self.calls.len() <= n {
                    self.calls.push(Default::default());
                }
                let call = &mut self.calls[n];
                if let Some(id) = tc.get("id").and_then(Json::str) {
                    call.0 = id.to_string();
                }
                if let Some(f) = tc.get("function") {
                    if let Some(name) = f.get("name").and_then(Json::str) {
                        call.1.push_str(name);
                    }
                    match f.get("arguments") {
                        Some(Json::Str(a)) => call.2.push_str(a),
                        Some(o @ Json::Obj(_)) => call.2 = o.to_string(),
                        _ => {}
                    }
                }
            }
        }
        if let Some(f) = c.get("finish_reason").and_then(Json::str) {
            self.finish = Some(f.to_string());
        }
    }
}

/// An HTTP error's words: the endpoint's own message when it gives one.
fn http_error(status: u16, body: &str) -> String {
    let said = Json::parse(body).ok().and_then(|j| {
        let e = j.get("error")?;
        e.get("message")
            .and_then(Json::str)
            .or_else(|| e.str())
            .map(str::to_string)
    });
    let why = said.unwrap_or_else(|| body.trim().chars().take(300).collect());
    match status {
        401 | 403 => format!("The endpoint refused the key (HTTP {status}): {why}"),
        404 => format!("Not found (HTTP 404): {why}. Check the base URL and model id"),
        _ => format!("HTTP {status}: {why}"),
    }
}

impl OpenAi {
    pub fn new(cfg: Config, out: Out) -> OpenAi {
        OpenAi {
            cfg: Arc::new(cfg),
            out,
            history: Arc::new(Mutex::new(Vec::new())),
            stop: Arc::new(AtomicBool::new(false)),
            busy: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Backend for OpenAi {
    fn send(&mut self, prompt: String) {
        if self.busy.swap(true, Ordering::SeqCst) {
            return;
        }
        self.stop.store(false, Ordering::SeqCst);
        self.history
            .lock()
            .expect("no panics while held")
            .push(Json::obj(vec![
                ("role", "user".into()),
                ("content", prompt.into()),
            ]));
        let (cfg, out, history, stop, busy) = (
            self.cfg.clone(),
            self.out.clone(),
            self.history.clone(),
            self.stop.clone(),
            self.busy.clone(),
        );
        std::thread::Builder::new()
            .name("agent-openai".into())
            .spawn(move || {
                let done = turn(&cfg, &out, &history, &stop);
                busy.store(false, Ordering::SeqCst);
                out.send(done);
            })
            .expect("a thread");
    }

    fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    fn set_file(&mut self, file: &std::path::Path) {
        let mut c = (*self.cfg).clone();
        c.file = file.to_path_buf();
        self.cfg = Arc::new(c);
    }
}

/// A turn: replies and tool calls until a reply calls none. Its end, as the event to send.
fn turn(cfg: &Config, out: &Out, history: &Mutex<Vec<Json>>, stop: &AtomicBool) -> Event {
    let key = match &cfg.key {
        Some(k) => Some(k.clone()),
        None => match super::keys::get(&cfg.base_url) {
            Ok(k) => k,
            Err(e) => {
                out.send(Event::Note {
                    text: format!("{e}: going on without a key"),
                    error: true,
                });
                None
            }
        },
    };
    let agent = ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_connect(Some(std::time::Duration::from_secs(10)))
            .build(),
    );
    let url = format!("{}/chat/completions", cfg.base_url.trim().trim_end_matches('/'));
    let mut watch = Watch::new(cfg.file.parent().unwrap_or(std::path::Path::new(".")));
    let stopped = Event::Done {
        error: None,
        stopped: true,
    };
    for _ in 0..MAX_STEPS {
        let mut messages = vec![Json::obj(vec![
            ("role", "system".into()),
            ("content", cfg.system.clone().into()),
        ])];
        messages.extend(history.lock().expect("no panics while held").iter().cloned());
        let body = Json::obj(vec![
            ("model", cfg.model.trim().into()),
            ("messages", Json::Arr(messages)),
            ("tools", tools::specs()),
            ("stream", true.into()),
        ])
        .to_string();
        let mut req = agent.post(&url).header("Content-Type", "application/json");
        if let Some(k) = key.as_deref().filter(|k| !k.is_empty()) {
            req = req.header("Authorization", format!("Bearer {k}"));
        }
        let resp = match req.send(body.as_str()) {
            Ok(r) => r,
            Err(e) => {
                return Event::Done {
                    error: Some(format!("Can't reach {url}: {e}")),
                    stopped: false,
                };
            }
        };
        let status = resp.status().as_u16();
        let stream = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.contains("event-stream"));
        let mut reader = resp.into_body().into_reader();
        if status >= 400 {
            let mut b = String::new();
            let _ = reader.read_to_string(&mut b);
            return Event::Done {
                error: Some(http_error(status, &b)),
                stopped: false,
            };
        }
        let mut acc = Acc::default();
        if stream {
            for line in BufReader::new(reader).lines() {
                if stop.load(Ordering::SeqCst) {
                    return stopped;
                }
                let Ok(line) = line else { break };
                let Some(data) = line.strip_prefix("data:").map(str::trim) else {
                    continue;
                };
                if data == "[DONE]" {
                    break;
                }
                let Ok(j) = Json::parse(data) else { continue };
                if let Some(e) = j.get("error") {
                    let msg = e
                        .get("message")
                        .and_then(Json::str)
                        .unwrap_or("the endpoint failed");
                    return Event::Done {
                        error: Some(msg.to_string()),
                        stopped: false,
                    };
                }
                let mut said = Vec::new();
                acc.take(&j, &mut said);
                for e in said {
                    out.send(e);
                }
            }
        } else {
            let mut b = String::new();
            let _ = reader.read_to_string(&mut b);
            match Json::parse(&b) {
                Ok(j) => {
                    let mut said = Vec::new();
                    acc.take(&j, &mut said);
                    for e in said {
                        out.send(e);
                    }
                }
                Err(e) => {
                    return Event::Done {
                        error: Some(format!(
                            "Not a chat completions reply ({e}): {}",
                            b.chars().take(200).collect::<String>()
                        )),
                        stopped: false,
                    };
                }
            }
        }
        if stop.load(Ordering::SeqCst) {
            return stopped;
        }
        let calls: Vec<Json> = acc
            .calls
            .iter()
            .enumerate()
            .map(|(k, (id, name, args))| {
                let id = if id.is_empty() {
                    format!("call_{k}")
                } else {
                    id.clone()
                };
                Json::obj(vec![
                    ("id", id.into()),
                    ("type", "function".into()),
                    (
                        "function",
                        Json::obj(vec![
                            ("name", name.as_str().into()),
                            ("arguments", args.as_str().into()),
                        ]),
                    ),
                ])
            })
            .collect();
        let mut msg = vec![
            ("role", "assistant".into()),
            (
                "content",
                if acc.text.is_empty() {
                    Json::Null
                } else {
                    acc.text.clone().into()
                },
            ),
        ];
        if !calls.is_empty() {
            msg.push(("tool_calls", Json::Arr(calls.clone())));
        }
        history.lock().expect("no panics while held").push(Json::obj(msg));
        if calls.is_empty() {
            return Event::Done {
                error: None,
                stopped: false,
            };
        }
        for c in &calls {
            let id = c.get("id").and_then(Json::str).unwrap_or("").to_string();
            let f = c.get("function").cloned().unwrap_or(Json::Null);
            let name = f.get("name").and_then(Json::str).unwrap_or("").to_string();
            let args_text = f.get("arguments").and_then(Json::str).unwrap_or("");
            let args = if args_text.trim().is_empty() {
                Json::Obj(vec![])
            } else {
                Json::parse(args_text).unwrap_or(Json::Null)
            };
            out.send(Event::ToolStart {
                id: id.clone(),
                name: name.clone(),
            });
            let (summary, input) = tool_summary(&name, &args);
            out.send(Event::ToolInput {
                id: id.clone(),
                summary,
                input,
            });
            let (text, error) = if args == Json::Null {
                (format!("the arguments aren't JSON: {args_text}"), true)
            } else {
                tools::call(&name, &args, &cfg.file)
            };
            for c in watch.check() {
                out.send(c);
            }
            out.send(Event::ToolDone {
                id: id.clone(),
                output: text.clone(),
                error,
            });
            history.lock().expect("no panics while held").push(Json::obj(vec![
                ("role", "tool".into()),
                ("tool_call_id", id.into()),
                ("content", text.into()),
            ]));
            if stop.load(Ordering::SeqCst) {
                return stopped;
            }
        }
    }
    Event::Done {
        error: Some(format!("Stopped after {MAX_STEPS} steps without a final answer")),
        stopped: false,
    }
}
