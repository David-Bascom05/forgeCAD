//! The browser's agent: the person's own Claude Code, run by `forgecad web DIR --agent` on the
//! machine serving the page (interface/forge-cli/src/agent_bridge.rs, research/agent-tab/
//! bridge-threat-model.md). The page starts it, sends messages, answers and stop, and reads its
//! stream-json lines, which the desktop's `claude::Parser` turns into the same transcript, cards
//! and diffs. The bridge's own `forge_file` lines are the project files Claude Code changed: put
//! in the page's filesystem quietly (they're on the machine already) and told as `FileChanged`,
//! so a turn's edits are one undo step as on the desktop.

use super::claude::Parser;
use super::Event;
use forge_agent::json::Json;

/// Where the page keeps the project (web/app.js `MOUNT`).
pub const MOUNT: &str = "/project";

/// A file the bridge sent: its path in the page, its text and version. What it held before, as
/// the page had it, comes back.
pub type Load<'a> = dyn FnMut(&str, &str, &str) -> Option<String> + 'a;

/// One line of the bridge's stream, as the transcript's events.
pub fn take_line(parser: &mut Parser, line: &str, load: &mut Load) -> Vec<Event> {
    let Ok(j) = Json::parse(line.trim()) else {
        return vec![];
    };
    let s = |k: &str| j.get(k).and_then(Json::str).unwrap_or("");
    match s("type") {
        "forge_file" => {
            let (rel, text, etag) = (s("path"), s("text"), s("etag"));
            if rel.is_empty() {
                return vec![];
            }
            let path = format!("{MOUNT}/{rel}");
            let before = load(&path, text, etag);
            vec![Event::FileChanged {
                path,
                before,
                text: text.to_string(),
            }]
        }
        // the session ended badly (a bypass refused, the stream lost): the turn ends with why
        "forge_bridge" => vec![Event::Done {
            error: Some(s("error").to_string()),
            stopped: false,
        }],
        _ => parser.line(line),
    }
}

/// A call's answer as the page's `forge_agent_call` gives it: its status and JSON (an error's
/// `{"error"}`).
pub fn reply(text: &str) -> (u16, Json) {
    let Ok(j) = Json::parse(text) else {
        return (0, Json::Null);
    };
    let status = j.get("status").and_then(Json::num).unwrap_or(0.0) as u16;
    let body = j
        .get("body")
        .and_then(Json::str)
        .and_then(|b| Json::parse(b).ok())
        .unwrap_or(Json::Null);
    (status, body)
}

/// Why a call failed, in words: the server's, else its status.
pub fn why(status: u16, body: &Json) -> String {
    match body.get("error").and_then(Json::str) {
        Some(e) => e.to_string(),
        None if status == 0 => "forgecad web isn't reachable: is it still running?".into(),
        None => format!("forgecad web said HTTP {status}"),
    }
}

/// What the tab says when there's no bridge to drive: how to start one.
pub const NO_BRIDGE: &str = "In the browser, the agent is your own Claude Code on your machine: run forgecad web on the project folder with --agent, and open the link it prints.";
pub const NO_BRIDGE_FIX: &str = "forgecad web . --agent";

#[cfg(target_arch = "wasm32")]
pub use page::{Bridge, Unbridged, device, ready, remember, take_said};

#[cfg(target_arch = "wasm32")]
mod page {
    use super::*;
    use crate::agent::{Backend, Out};
    use crate::shell::agent::{Answer, Mode};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = window, js_name = forge_agent_ready)]
        fn page_ready() -> bool;
        #[wasm_bindgen(js_namespace = window, js_name = forge_agent_call)]
        fn page_call(name: &str, body: &str) -> js_sys::Promise;
        #[wasm_bindgen(js_namespace = window, js_name = forge_agent_events)]
        fn page_events(from: f64, on_line: &Closure<dyn FnMut(String)>);
        #[wasm_bindgen(js_namespace = window, js_name = forge_agent_adopt)]
        fn page_adopt(rel: &str, etag: &str);
        #[wasm_bindgen(js_namespace = window, js_name = forge_agent_device)]
        fn page_device() -> f64;
        #[wasm_bindgen(js_namespace = window, js_name = forge_agent_remember)]
        fn page_remember(on: bool) -> js_sys::Promise;
    }

    /// This page's pairing: None with no bridge, Some(0) by the link, else Some(when the
    /// remembered device lapses, in seconds since the epoch).
    pub fn device() -> Option<u64> {
        let d = page_device();
        (d >= 0.0).then_some(d as u64)
    }

    /// What remembering or forgetting this device came to, said once.
    static SAID: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

    /// Remember this device (7 days, a device token kept in the page) or forget it.
    pub fn remember(on: bool) {
        wasm_bindgen_futures::spawn_local(async move {
            let said = wasm_bindgen_futures::JsFuture::from(page_remember(on))
                .await
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_else(|| "Couldn't reach the page's pairing".into());
            *crate::plat::lock(&SAID) = Some(said);
        });
    }

    pub fn take_said() -> Option<String> {
        crate::plat::lock(&SAID).take()
    }

    /// The page was opened from `forgecad web --agent`'s link (it holds the agent's token).
    pub fn ready() -> bool {
        page_ready()
    }

    async fn call(name: &str, body: Json) -> (u16, Json) {
        let p = page_call(name, &body.to_string());
        match wasm_bindgen_futures::JsFuture::from(p).await {
            Ok(v) => reply(&v.as_string().unwrap_or_default()),
            Err(_) => (0, Json::Null),
        }
    }

    /// The bridge's session, driven from the tab.
    pub struct Bridge {
        out: Out,
        parser: Arc<Mutex<Parser>>,
        /// The open file, its path in the project.
        file: Arc<Mutex<Option<String>>>,
        started: Arc<AtomicBool>,
        mode: Arc<Mutex<Option<Mode>>>,
    }

    impl Bridge {
        pub fn new(out: Out, file: &std::path::Path) -> Bridge {
            let b = Bridge {
                out,
                parser: Default::default(),
                file: Default::default(),
                started: Default::default(),
                mode: Default::default(),
            };
            b.set_rel(file);
            b
        }

        fn set_rel(&self, file: &std::path::Path) {
            let rel = file
                .to_str()
                .and_then(|f| f.strip_prefix(MOUNT))
                .map(|f| f.trim_start_matches('/').to_string())
                .filter(|f| !f.is_empty());
            *crate::plat::lock(&self.file) = rel;
        }

        /// Reads the session's lines from `from`, as they come, into the transcript.
        fn listen(out: Out, parser: Arc<Mutex<Parser>>, from: f64) {
            let on_line = Closure::<dyn FnMut(String)>::new(move |line: String| {
                let mut load = |path: &str, text: &str, etag: &str| {
                    let before = forge_dirs::fs::read_to_string(path).ok();
                    crate::webapp::fs_load(path, text.as_bytes());
                    page_adopt(path.strip_prefix(MOUNT).unwrap_or(path).trim_start_matches('/'), etag);
                    before
                };
                let said = take_line(&mut crate::plat::lock(&parser), &line, &mut load);
                for e in said {
                    out.send(e);
                }
            });
            page_events(from, &on_line);
            // the page holds it for the stream's life (one at a time: a new stream ends the last)
            on_line.forget();
        }
    }

    impl Backend for Bridge {
        fn send(&mut self, prompt: String, pictures: Vec<crate::agent::Picture>) {
            let (out, parser, file, started, mode) = (
                self.out.clone(),
                self.parser.clone(),
                self.file.clone(),
                self.started.clone(),
                self.mode.clone(),
            );
            wasm_bindgen_futures::spawn_local(async move {
                if !started.load(Ordering::SeqCst) {
                    let mut body = vec![];
                    if let Some(f) = crate::plat::lock(&file).clone() {
                        body.push(("file", f.into()));
                    }
                    if let Some(m) = *crate::plat::lock(&mode) {
                        body.push(("mode", m.flag().into()));
                    }
                    match call("start", Json::obj(body)).await {
                        (200, j) => {
                            started.store(true, Ordering::SeqCst);
                            let from = j.get("from").and_then(Json::num).unwrap_or(0.0);
                            Self::listen(out.clone(), parser, from);
                        }
                        (503, j) => {
                            out.send(Event::Problem {
                                text: why(503, &j),
                                fix: j.get("fix").and_then(Json::str).map(str::to_string),
                            });
                            return;
                        }
                        (status, j) => {
                            out.send(Event::Done {
                                error: Some(why(status, &j)),
                                stopped: false,
                            });
                            return;
                        }
                    }
                }
                let mut body = vec![("text", prompt.into())];
                if !pictures.is_empty() {
                    let all = pictures.iter().map(|p| {
                        Json::obj(vec![
                            ("media", p.media.into()),
                            ("data", crate::agent::apptools::base64(&p.bytes).into()),
                        ])
                    });
                    body.push(("pictures", Json::Arr(all.collect())));
                }
                let (status, j) = call("send", Json::obj(body)).await;
                if status != 200 {
                    // the session went (it ended, another page has it): start again next time
                    started.store(false, Ordering::SeqCst);
                    out.send(Event::Done {
                        error: Some(why(status, &j)),
                        stopped: false,
                    });
                }
            });
        }

        fn stop(&mut self) {
            let out = self.out.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if call("stop", Json::obj(vec![])).await.0 != 200 {
                    out.send(Event::Done {
                        error: None,
                        stopped: true,
                    });
                }
            });
        }

        fn set_file(&mut self, file: &std::path::Path) {
            self.set_rel(file);
        }

        fn answer(&mut self, id: &str, a: Answer) {
            let (id, word) = (
                id.to_string(),
                match a {
                    Answer::Allow => "allow",
                    Answer::Always => "always",
                    Answer::Deny => "deny",
                },
            );
            let out = self.out.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let (status, j) = call("answer", Json::obj(vec![("id", id.into()), ("answer", word.into())])).await;
                if status != 200 {
                    out.send(Event::Note {
                        text: format!("The answer didn't reach Claude Code: {}", why(status, &j)),
                        error: true,
                    });
                }
            });
        }

        fn set_mode(&mut self, m: Mode) {
            *crate::plat::lock(&self.mode) = Some(m);
            if self.started.load(Ordering::SeqCst) {
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = call("mode", Json::obj(vec![("mode", m.flag().into())])).await;
                });
            }
        }
    }

    /// No bridge behind the page: the tab says how to start one, at the first message.
    pub struct Unbridged {
        pub out: Out,
    }

    impl Backend for Unbridged {
        fn send(&mut self, _prompt: String, _pictures: Vec<crate::agent::Picture>) {
            self.out.send(Event::Problem {
                text: NO_BRIDGE.into(),
                fix: Some(NO_BRIDGE_FIX.into()),
            });
        }
        fn stop(&mut self) {
            self.out.send(Event::Done {
                error: None,
                stopped: true,
            });
        }
    }
}
