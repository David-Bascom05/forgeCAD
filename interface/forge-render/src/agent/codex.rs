//! Codex CLI JSONL, one process per turn, resumed by thread id. Authentication stays in Codex.
use super::{Backend, Event, Out, Watch, claude::Config, clean_env};
use forge_agent::json::Json;
use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

fn text<'a>(j: &'a Json, key: &str) -> &'a str {
    j.get(key).and_then(Json::str).unwrap_or("")
}

#[derive(Default)]
pub struct Parser {
    pub session: Option<String>,
    pub error: Option<String>,
    pub completed: bool,
    tools: HashSet<String>,
}

impl Parser {
    pub fn line(&mut self, line: &str) -> Vec<Event> {
        let Ok(j) = Json::parse(line) else { return vec![] };
        match text(&j, "type") {
            "thread.started" => self.session = Some(text(&j, "thread_id").into()),
            "turn.completed" => {
                self.completed = true;
                self.error = None;
            }
            "turn.failed" => {
                self.error = Some(
                    j.get("error")
                        .map(|e| text(e, "message"))
                        .unwrap_or("Codex turn failed")
                        .into(),
                );
            }
            "error" => self.error = Some(text(&j, "message").into()),
            kind @ ("item.started" | "item.completed") => {
                let Some(item) = j.get("item") else { return vec![] };
                let id = text(item, "id").to_string();
                let done = kind == "item.completed";
                let name = match text(item, "type") {
                    "agent_message" if done => {
                        return vec![Event::Text(format!("{}\n\n", text(item, "text")))];
                    }
                    "mcp_tool_call" => format!("mcp__{}__{}", text(item, "server"), text(item, "tool")),
                    "command_execution" => "shell".into(),
                    "file_change" => "file changes".into(),
                    _ => return vec![],
                };
                let mut events = Vec::new();
                if self.tools.insert(id.clone()) {
                    let input = item
                        .get("arguments")
                        .cloned()
                        .unwrap_or_else(|| text(item, "command").into());
                    let (summary, input) = crate::shell::agent::tool_summary(&name, &input);
                    events.extend([
                        Event::ToolStart { id: id.clone(), name },
                        Event::ToolInput {
                            id: id.clone(),
                            summary,
                            input,
                        },
                    ]);
                }
                if !done {
                    return events;
                }
                let output = item
                    .get("result")
                    .filter(|v| **v != Json::Null)
                    .or_else(|| item.get("error").filter(|v| **v != Json::Null))
                    .map_or_else(
                        || {
                            item.get("changes").map_or_else(
                                || text(item, "aggregated_output").to_string(),
                                ToString::to_string,
                            )
                        },
                        ToString::to_string,
                    );
                events.push(Event::ToolDone {
                    id,
                    output,
                    error: text(item, "status") == "failed"
                        || item.get("error").is_some_and(|e| *e != Json::Null)
                        || item.get("result").and_then(|r| r.get("isError")) == Some(&Json::Bool(true)),
                });
                return events;
            }
            _ => {}
        }
        vec![]
    }
}

pub struct Codex {
    cfg: Config,
    root: std::path::PathBuf,
    out: Out,
    pub session: Arc<Mutex<Option<String>>>,
    child: Arc<Mutex<Option<Child>>>,
    stopped: Arc<AtomicBool>,
}

impl Codex {
    pub fn new(cfg: Config, out: Out, session: Option<String>) -> Self {
        let root = super::project::root(&cfg.file);
        Self {
            cfg,
            root,
            out,
            session: Arc::new(Mutex::new(session)),
            child: Arc::new(Mutex::new(None)),
            stopped: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn args(&self, picture: Option<&Path>) -> Vec<String> {
        let mut args: Vec<String> = ["exec", "--json", "--skip-git-repo-check", "--color", "never"]
            .into_iter()
            .map(str::to_string)
            .collect();
        // Add ForgeCAD's tools to the user's normal CLI configuration.
        if let Some(mcp) = self.cfg.mcp.as_ref().and_then(|m| Json::parse(m).ok())
            && let Some(server) = mcp.get("mcpServers").and_then(|s| s.get(super::mcp::SERVER))
            && let (Some(command), Some(server_args)) = (server.get("command"), server.get("args"))
        {
            args.extend([
                "-c".into(),
                format!("mcp_servers.forgecad={{command={command},args={server_args},required=true}}"),
            ]);
        }
        if !self.cfg.model.trim().is_empty() {
            args.extend(["--model".into(), self.cfg.model.trim().into()]);
        }
        if let Some(session) = self.session.lock().expect("session").clone() {
            args.extend(["resume".into(), session]);
        }
        // Resume has its own image option; keep it after the subcommand.
        if let Some(path) = picture {
            args.extend(["--image".into(), path.display().to_string()]);
        }
        args.push("-".into());
        args
    }

    fn start(&mut self, prompt: &str, picture: Option<Vec<u8>>) -> Result<(), String> {
        std::fs::create_dir_all(&self.cfg.cwd).map_err(|e| e.to_string())?;
        let image = if let Some(png) = picture {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let p = self.cfg.cwd.join(format!(
                "codex-{}-{}.png",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::write(&p, png).map_err(|e| e.to_string())?;
            Some(p)
        } else {
            None
        };
        let mut command = Command::new(&self.cfg.program);
        command
            .args(self.args(image.as_deref()))
            .current_dir(&self.cfg.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        clean_env(&mut command);
        // An app-launched Codex is its own session, not a child turn of this coding agent.
        command.env_remove("CODEX_THREAD_ID");
        let mut watch = Watch::new(&self.root);
        let child = command.spawn();
        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                if let Some(p) = image {
                    let _ = std::fs::remove_file(p);
                }
                return Err(format!(
                    "Codex didn't start: {e}. Install the Codex CLI and run codex login."
                ));
            }
        };
        let mut stdin = child.stdin.take().expect("piped");
        // Supply app context without replacing the user's configured developer instructions.
        let prompt = format!("{}\n\nUser request:\n{prompt}", self.cfg.system);
        if let Err(e) = stdin.write_all(prompt.as_bytes()) {
            let _ = child.kill();
            let _ = child.wait();
            if let Some(p) = image {
                let _ = std::fs::remove_file(p);
            }
            return Err(format!("Couldn't send to Codex: {e}"));
        }
        drop(stdin);
        let stdout = child.stdout.take().expect("piped");
        let stderr = child.stderr.take().expect("piped");
        *self.child.lock().expect("child") = Some(child);
        self.stopped.store(false, Ordering::SeqCst);
        let (out, session, child, stopped) = (
            self.out.clone(),
            self.session.clone(),
            self.child.clone(),
            self.stopped.clone(),
        );
        std::thread::spawn(move || {
            let errors = std::thread::spawn(move || {
                let mut tail = String::new();
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    tail.push_str(&line);
                    tail.push('\n');
                    if tail.len() > 4000 {
                        let cut = (tail.len() - 4000..tail.len())
                            .find(|i| tail.is_char_boundary(*i))
                            .unwrap_or(0);
                        tail.drain(..cut);
                    }
                }
                tail
            });
            let mut parser = Parser::default();
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                for event in parser.line(&line) {
                    if matches!(event, Event::ToolDone { .. }) {
                        for change in watch.check() {
                            out.send(change);
                        }
                    }
                    out.send(event);
                }
                if let Some(id) = &parser.session {
                    *session.lock().expect("session") = Some(id.clone());
                }
            }
            // stdout is closed; polling leaves the mutex free for Stop while exiting.
            let status = loop {
                let mut slot = child.lock().expect("child");
                let result = slot.as_mut().map(|c| c.try_wait());
                match result {
                    Some(Ok(None)) => {}
                    Some(Ok(Some(status))) => {
                        slot.take();
                        break Some(status);
                    }
                    _ => {
                        slot.take();
                        break None;
                    }
                }
                drop(slot);
                std::thread::sleep(std::time::Duration::from_millis(10));
            };
            let stderr = errors.join().unwrap_or_default();
            if let Some(p) = image {
                let _ = std::fs::remove_file(p);
            }
            for change in watch.check() {
                out.send(change);
            }
            let stopped = stopped.load(Ordering::SeqCst);
            let error = if stopped {
                None
            } else {
                parser.error.or_else(|| {
                    (!parser.completed || !status.is_some_and(|s| s.success())).then(|| {
                        let last = stderr
                            .lines()
                            .rev()
                            .find(|l| !l.trim().is_empty())
                            .unwrap_or("No completed turn received");
                        format!("Codex stopped: {last}. Check codex login status.")
                    })
                })
            };
            out.send(Event::Done { error, stopped });
        });
        Ok(())
    }
}

impl Backend for Codex {
    fn send(&mut self, prompt: String, picture: Option<Vec<u8>>) {
        if self.child.lock().expect("child").is_some() {
            return;
        }
        if let Err(text) = self.start(&prompt, picture) {
            self.out.send(Event::Problem {
                text,
                fix: Some("codex login".into()),
            });
        }
    }
    fn stop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        if let Some(child) = self.child.lock().expect("child").as_mut() {
            let _ = child.kill();
        }
    }
    fn set_file(&mut self, file: &Path) {
        self.cfg.file = file.to_path_buf();
    }
}

impl Drop for Codex {
    fn drop(&mut self) {
        self.stop();
    }
}
