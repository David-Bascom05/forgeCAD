//! Local project conversations. Agent credentials and project files are never stored here.
use super::Event;
use crate::shell::agent::{Entry, ToolState};
use crate::shell::agent_settings::AgentSettings;
use forge_agent::json::Json;
use std::path::{Path, PathBuf};

const MAX_BYTES: u64 = 32 << 20;

#[derive(Clone, Debug)]
pub struct Summary {
    pub id: String,
    pub title: String,
    pub backend: String,
    pub updated: u64,
    pub session: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Conversation {
    pub id: String,
    pub project: String,
    pub title: String,
    pub updated: u64,
    pub generation: u64,
    pub settings: AgentSettings,
    pub session: Option<String>,
    pub entries: Vec<Entry>,
    pub history: Vec<Json>,
    pub interrupted: bool,
}

pub fn default_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))?;
    Some(base.join("forgecad/conversations"))
}

fn canonical(project: &Path) -> Result<String, String> {
    project
        .canonicalize()
        .map(|p| p.display().to_string())
        .map_err(|e| e.to_string())
}
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 96 && id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
}
fn folder(base: &Path, project: &Path) -> Result<PathBuf, String> {
    let project = canonical(project)?;
    Ok(base.join(forge_git::HashKind::Sha256.of(project.as_bytes()).hex()))
}
fn location(base: &Path, project: &Path, id: &str) -> Result<PathBuf, String> {
    if !valid_id(id) {
        return Err("Invalid conversation ID".into());
    }
    Ok(folder(base, project)?.join(id))
}
fn string(j: &Json, key: &str) -> String {
    j.get(key).and_then(Json::str).unwrap_or("").to_string()
}
fn array(j: &Json, key: &str) -> Vec<Json> {
    match j.get(key) {
        Some(Json::Arr(a)) => a.clone(),
        _ => vec![],
    }
}
fn number(j: &Json, key: &str) -> u64 {
    j.get(key).and_then(Json::num).unwrap_or(0.0) as u64
}
fn read(path: &Path, max: u64) -> Result<Json, String> {
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > max {
        return Err("Conversation is too large".into());
    }
    Json::parse(&std::fs::read_to_string(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
fn private_dir(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())
}
fn write(path: &Path, value: &Json) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let text = value.to_string();
    let mut lock = forge_git::fsutil::Lock::take(path).map_err(|e| e.to_string())?;
    lock.write(text.as_bytes())
        .and_then(|_| lock.commit())
        .map_err(|e| e.to_string())?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())
}

impl Conversation {
    pub fn new(project: &Path, settings: &AgentSettings, title: &str) -> Result<Self, String> {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let mut settings = settings.clone();
        settings.trusted.clear();
        Ok(Self {
            id: format!(
                "{stamp:x}-{:x}-{:x}",
                std::process::id(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ),
            project: canonical(project)?,
            title: title
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("Conversation")
                .chars()
                .take(80)
                .collect(),
            updated: forge_history::now() as u64,
            generation: 0,
            settings,
            session: None,
            entries: vec![],
            history: vec![],
            interrupted: false,
        })
    }
    pub fn summary(&self) -> Summary {
        Summary {
            id: self.id.clone(),
            title: self.title.clone(),
            backend: self.settings.label(),
            updated: self.updated,
            session: self.session.clone(),
        }
    }
    pub fn context(&self) -> String {
        self.entries
            .iter()
            .filter_map(|e| match e {
                Entry::User { text, .. } => Some(format!("User: {text}\n")),
                Entry::Agent { text } => Some(format!("Agent: {text}\n")),
                _ => None,
            })
            .collect::<String>()
    }
    pub fn save(&mut self, base: &Path) -> Result<(), String> {
        let dir = location(base, Path::new(&self.project), &self.id)?;
        private_dir(dir.parent().expect("project folder"))?;
        private_dir(&dir)?;
        let path = dir.join("chat.json");
        let mut lock = forge_git::fsutil::Lock::take(&path).map_err(|e| e.to_string())?;
        if path.exists() && number(&read(&path, MAX_BYTES)?, "generation") != self.generation {
            return Err(
                "This conversation was updated in another window; resume it again before continuing".into(),
            );
        }
        let json = Json::obj(vec![
            ("format", 1.0.into()),
            ("id", self.id.clone().into()),
            ("project", self.project.clone().into()),
            ("title", self.title.clone().into()),
            ("updated", (self.updated as f64).into()),
            ("generation", ((self.generation + 1) as f64).into()),
            ("settings", self.settings.to_toml().into()),
            ("session", self.session.clone().map_or(Json::Null, Json::from)),
            ("interrupted", self.interrupted.into()),
            (
                "entries",
                Json::Arr(self.entries.iter().map(entry_json).collect()),
            ),
            ("history", Json::Arr(self.history.clone())),
        ]);
        let text = json.to_string();
        if text.len() as u64 > MAX_BYTES {
            return Err("Conversation exceeds the 32 MiB storage limit".into());
        }
        lock.write(text.as_bytes())
            .and_then(|_| lock.commit())
            .map_err(|e| e.to_string())?;
        self.generation += 1;
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
        write(
            &dir.join("summary.json"),
            &Json::obj(vec![
                ("id", self.id.clone().into()),
                ("project", self.project.clone().into()),
                ("title", self.title.clone().into()),
                ("backend", self.settings.label().into()),
                ("session", self.session.clone().map_or(Json::Null, Json::from)),
                ("updated", (self.updated as f64).into()),
            ]),
        )
    }
}

pub fn list(base: &Path, project: &Path) -> Result<Vec<Summary>, String> {
    let dir = folder(base, project)?;
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e.to_string()),
    };
    let expected = canonical(project)?;
    let mut out = vec![];
    for entry in entries.filter_map(Result::ok) {
        let id = entry.file_name().to_string_lossy().to_string();
        if !valid_id(&id) {
            continue;
        }
        let Ok(j) = read(&entry.path().join("summary.json"), 16384) else {
            continue;
        };
        if string(&j, "project") != expected || string(&j, "id") != id {
            continue;
        }
        out.push(Summary {
            id,
            title: string(&j, "title"),
            backend: string(&j, "backend"),
            updated: number(&j, "updated"),
            session: j.get("session").and_then(Json::str).map(str::to_string),
        });
    }
    out.sort_by(|a, b| b.updated.cmp(&a.updated).then_with(|| b.id.cmp(&a.id)));
    Ok(out)
}

pub fn load(base: &Path, project: &Path, id: &str) -> Result<Conversation, String> {
    let j = read(&location(base, project, id)?.join("chat.json"), MAX_BYTES)?;
    if number(&j, "format") != 1 || string(&j, "id") != id || string(&j, "project") != canonical(project)? {
        return Err("Conversation belongs to another project or has an unsupported format".into());
    }
    Ok(Conversation {
        id: id.into(),
        project: canonical(project)?,
        title: string(&j, "title"),
        updated: number(&j, "updated"),
        generation: number(&j, "generation"),
        settings: AgentSettings::from_toml(&string(&j, "settings")),
        session: j.get("session").and_then(Json::str).map(str::to_string),
        entries: array(&j, "entries").iter().filter_map(entry_from_json).collect(),
        history: complete_history(array(&j, "history")),
        interrupted: j.get("interrupted") == Some(&Json::Bool(true)),
    })
}

fn entry_json(entry: &Entry) -> Json {
    match entry {
        Entry::User { text, refs, markup } => Json::obj(vec![
            ("type", "user".into()),
            ("text", text.clone().into()),
            ("refs", Json::Arr(refs.iter().cloned().map(Json::from).collect())),
            (
                "markup",
                markup.as_ref().map_or(Json::Null, |m| m.said.clone().into()),
            ),
        ]),
        Entry::Agent { text } => Json::obj(vec![("type", "agent".into()), ("text", text.clone().into())]),
        Entry::Note { text, error } => Json::obj(vec![
            ("type", "note".into()),
            ("text", text.clone().into()),
            ("error", (*error).into()),
        ]),
        Entry::Tool {
            id,
            name,
            summary,
            input,
            output,
            state,
        } => Json::obj(vec![
            ("type", "tool".into()),
            ("id", id.clone().into()),
            ("name", name.clone().into()),
            ("summary", summary.clone().into()),
            ("input", input.clone().into()),
            ("output", output.clone().into()),
            (
                "state",
                match state {
                    ToolState::Running => "interrupted",
                    ToolState::Done => "done",
                    ToolState::Failed => "failed",
                }
                .into(),
            ),
        ]),
        Entry::Diff {
            file,
            created,
            added,
            removed,
            undone,
            ..
        } => Json::obj(vec![
            ("type", "note".into()),
            (
                "text",
                format!(
                    "{} {file} (+{added}/−{removed}){}",
                    if *created { "Created" } else { "Changed" },
                    if *undone { " · undone" } else { "" }
                )
                .into(),
            ),
            ("error", false.into()),
        ]),
    }
}
fn entry_from_json(j: &Json) -> Option<Entry> {
    Some(match string(j, "type").as_str() {
        "user" => {
            let mut text = string(j, "text");
            if let Some(markup) = j.get("markup").and_then(Json::str) {
                text.push_str(&format!("\n[Markup attached: {markup}]"));
            }
            Entry::User {
                text,
                refs: array(j, "refs")
                    .iter()
                    .filter_map(Json::str)
                    .map(str::to_string)
                    .collect(),
                markup: None,
            }
        }
        "agent" => Entry::Agent {
            text: string(j, "text"),
        },
        "note" => Entry::Note {
            text: string(j, "text"),
            error: j.get("error") == Some(&Json::Bool(true)),
        },
        "tool" => Entry::Tool {
            id: string(j, "id"),
            name: string(j, "name"),
            summary: string(j, "summary"),
            input: string(j, "input"),
            output: string(j, "output"),
            state: if string(j, "state") == "done" {
                ToolState::Done
            } else {
                ToolState::Failed
            },
        },
        _ => return None,
    })
}

/// Drop an interrupted assistant tool-call group rather than send an invalid API transcript.
pub fn complete_history(mut history: Vec<Json>) -> Vec<Json> {
    for (index, message) in history.iter().enumerate() {
        if message.get("role").and_then(Json::str) != Some("assistant") {
            continue;
        }
        let calls = array(message, "tool_calls");
        if calls.is_empty() {
            continue;
        }
        let mut pending: std::collections::HashSet<_> = calls.iter().map(|c| string(c, "id")).collect();
        for next in &history[index + 1..] {
            if next.get("role").and_then(Json::str) != Some("tool") {
                break;
            }
            pending.remove(&string(next, "tool_call_id"));
        }
        if !pending.is_empty() {
            history.truncate(index);
            break;
        }
    }
    history
}

/// Whether an event adds content that should be checkpointed.
pub fn terminal(event: &Event) -> bool {
    matches!(event, Event::Done { .. } | Event::Problem { .. })
}

/// Best-effort import of older ForgeCAD Codex CLI sessions. This never writes Codex's logs.
/// The explicit ForgeCAD project marker and clean agent cwd keep unrelated coding chats out.
pub fn import_codex(base: &Path, project: &Path) -> Result<usize, String> {
    let codex_home = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".codex")));
    let Some(home) = codex_home else { return Ok(0) };
    import_codex_from(base, project, &home.join("sessions"), &super::claude::clean_cwd())
}

pub fn import_codex_from(
    base: &Path,
    project: &Path,
    sessions: &Path,
    agent_cwd: &Path,
) -> Result<usize, String> {
    use std::io::{BufRead, BufReader, Read};
    fn scan(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
        if depth > 5 {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_dir() {
                scan(&entry.path(), out, depth + 1);
            } else if kind.is_file() && entry.path().extension().is_some_and(|x| x == "jsonl") {
                out.push(entry.path());
            }
        }
    }
    let expected = canonical(project)?;
    let marker = format!("The project root is {expected}. All app file paths");
    let mut known: std::collections::HashSet<String> = list(base, project)?
        .into_iter()
        .filter_map(|s| s.session)
        .collect();
    let mut paths = Vec::new();
    scan(sessions, &mut paths, 0);
    let mut imported = 0;
    for path in paths {
        let Ok(file) = std::fs::File::open(&path) else {
            continue;
        };
        let mut lines = BufReader::new(file.take(128 << 20)).lines();
        let Some(Ok(first)) = lines.next() else { continue };
        let Ok(meta) = Json::parse(&first) else { continue };
        let Some(payload) = meta.get("payload") else {
            continue;
        };
        if string(&meta, "type") != "session_meta" || Path::new(&string(payload, "cwd")) != agent_cwd {
            continue;
        }
        let id = string(payload, "id");
        if !valid_id(&id) || known.contains(&id) {
            continue;
        }
        let mut matched = false;
        let mut foreign = false;
        let mut entries = Vec::new();
        let mut interrupted = false;
        let mut model = String::new();
        for line in lines.map_while(Result::ok) {
            let Ok(record) = Json::parse(&line) else { continue };
            let Some(payload) = record.get("payload") else {
                continue;
            };
            match string(&record, "type").as_str() {
                "turn_context" => model = string(payload, "model"),
                "event_msg" => match string(payload, "type").as_str() {
                    "task_started" => interrupted = true,
                    "task_complete" | "turn_aborted" => interrupted = false,
                    _ => {}
                },
                "response_item" => {
                    let role = string(payload, "role");
                    let text = array(payload, "content")
                        .iter()
                        .filter_map(|c| c.get("text").and_then(Json::str))
                        .collect::<Vec<_>>()
                        .join("\n");
                    if role == "developer" && text.contains("You are ForgeCAD's in-app agent") {
                        if text.contains(&marker) {
                            matched = true
                        } else {
                            foreign = true;
                        }
                    }
                    if !matched || foreign || text.trim().is_empty() {
                        continue;
                    }
                    if role == "user" && !text.trim_start().starts_with('<') {
                        entries.push(Entry::User {
                            text,
                            refs: vec![],
                            markup: None,
                        });
                    } else if role == "assistant" {
                        entries.push(Entry::Agent { text });
                    }
                }
                _ => {}
            }
        }
        if !matched || foreign || entries.is_empty() {
            continue;
        }
        let title = entries
            .iter()
            .find_map(|e| match e {
                Entry::User { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .unwrap_or("Imported Codex conversation");
        let settings = AgentSettings {
            kind: crate::shell::agent_settings::Kind::Codex,
            codex_model: model,
            ..Default::default()
        };
        let mut chat = Conversation::new(project, &settings, title)?;
        chat.id = format!("c-{id}");
        chat.session = Some(id.clone());
        chat.entries = entries;
        chat.interrupted = interrupted;
        chat.updated = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(chat.updated, |d| d.as_secs());
        // Never overwrite a previous import, even when its native log has grown since then.
        if location(base, project, &chat.id)?.join("chat.json").exists() {
            continue;
        }
        chat.save(base)?;
        known.insert(id);
        imported += 1;
    }
    Ok(imported)
}
