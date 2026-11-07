//! The version tools, one path for the app and for agents: `versions`, `history`,
//! `create_version`, `restore` and `compare`, each answering JSON.

use crate::compare;
use forge_agent::json::Json;
use forge_git::Oid;
use forge_history::{History, Paused, Rec, Saved, Version};
use std::path::Path;

fn open(file: &Path) -> Result<(History, String), String> {
    let h = History::open(file).ok_or_else(|| format!("{} keeps no history", file.display()))?;
    let rel = h.rel(file).map_err(|e| e.to_string())?;
    Ok((h, rel))
}

fn short(id: &Oid) -> String {
    id.hex()[..10].to_string()
}

pub fn version_json(v: &Version) -> Json {
    let mut f = vec![
        ("file", Json::Str(v.path.clone())),
        ("n", Json::Num(v.n as f64)),
        ("name", Json::Str(v.name.clone())),
        ("notes", Json::Str(v.notes.clone())),
        ("save", Json::Str(short(&v.save))),
        ("time", Json::Num(v.time as f64)),
        ("author", Json::Str(v.author.clone())),
    ];
    if let Some(a) = &v.agent {
        f.push(("agent", Json::Str(a.clone())));
    }
    Json::obj(f)
}

fn rec_json(r: &Rec, versions: &[Version]) -> Json {
    let mut f = vec![
        ("save", Json::Str(short(&r.id))),
        ("time", Json::Num(r.time as f64)),
        ("author", Json::Str(r.author.clone())),
        ("summary", Json::Str(r.summary.clone())),
        (
            "files",
            Json::Arr(r.paths.iter().map(|p| Json::Str(p.clone())).collect()),
        ),
    ];
    if let Some(a) = &r.agent {
        f.push(("agent", Json::Str(a.clone())));
    }
    let marks: Vec<Json> = versions
        .iter()
        .filter(|v| v.save == r.id)
        .map(|v| Json::Str(format!("{}@v{}", v.path, v.n)))
        .collect();
    if !marks.is_empty() {
        f.push(("versions", Json::Arr(marks)));
    }
    Json::obj(f)
}

pub fn versions(file: &Path) -> Result<Json, String> {
    let (h, rel) = open(file)?;
    let vs = h.versions(Some(&rel)).map_err(|e| e.to_string())?;
    Ok(Json::obj(vec![
        ("file", Json::Str(rel)),
        ("versions", Json::Arr(vs.iter().map(version_json).collect())),
        (
            "newest",
            vs.iter()
                .map(|v| v.n)
                .max()
                .map_or(Json::Null, |n| Json::Num(n as f64)),
        ),
    ]))
}

/// The file's saves (or with `all`, the whole project's), newest first.
pub fn history(file: &Path, all: bool, limit: usize) -> Result<Json, String> {
    let (h, rel) = open(file)?;
    let saves = h
        .saves(if all { None } else { Some(&rel) })
        .map_err(|e| e.to_string())?;
    let vs = h.versions(None).map_err(|e| e.to_string())?;
    Ok(Json::obj(vec![
        ("file", Json::Str(rel)),
        ("count", Json::Num(saves.len() as f64)),
        (
            "saves",
            Json::Arr(saves.iter().take(limit).map(|r| rec_json(r, &vs)).collect()),
        ),
    ]))
}

fn paused(p: &Paused) -> String {
    match p {
        Paused::NeedsIdentity => "no name and email to sign saves with (set user.name and user.email)".into(),
        Paused::Busy(w) => format!("history is paused while another tool is mid-change here ({w})"),
        Paused::Locked => "another tool is writing this history now; try again".into(),
        Paused::Ignored => "the project ignores this file".into(),
    }
}

/// Saves the file as it is on disk (a save only if it changed); its id, or None.
pub fn save(file: &Path, summary: &str, agent: Option<&str>) -> Result<Option<Oid>, String> {
    let (h, rel) = open(file)?;
    match h
        .save(file, &[], &format!("{rel}: {summary}"), agent)
        .map_err(|e| e.to_string())?
    {
        Saved::New(id) => Ok(Some(id)),
        Saved::Unchanged => Ok(None),
        Saved::Paused(p) => Err(paused(&p)),
    }
}

pub fn create_version(file: &Path, name: &str, notes: &str, agent: Option<&str>) -> Result<Json, String> {
    let (h, rel) = open(file)?;
    match h
        .save(file, &[], &format!("{rel}: saved"), agent)
        .map_err(|e| e.to_string())?
    {
        Saved::Paused(p) => return Err(paused(&p)),
        Saved::New(_) | Saved::Unchanged => {}
    }
    let v = h
        .create_version(&rel, name, notes, agent)
        .map_err(|e| e.to_string())?;
    Ok(version_json(&v))
}

/// The files a document follows as latest (its unpinned part files), as project paths, at a save.
fn followed(h: &History, rel: &str, at: &Oid) -> Vec<String> {
    let Ok(side) = compare::build_at(h, rel, Some(at), "restore") else {
        return vec![];
    };
    let Some(a) = side.run.asm.as_ref() else {
        return vec![];
    };
    let base: Vec<&str> = rel.split('/').collect();
    let mut out = vec![];
    for (_, file, _) in &a.occurrences {
        if crate::pins::is_pinned(file) || file.starts_with('#') || file.starts_with('?') {
            continue;
        }
        let file = file.split('|').next().unwrap_or(file);
        let f = std::path::Path::new(file);
        let p = if f.is_absolute() {
            match f.strip_prefix(dir_of_pin(h, rel, at)) {
                Ok(r) => join(&base, &r.to_string_lossy()),
                Err(_) => continue,
            }
        } else {
            join(&base, file)
        };
        if let Some(p) = p
            && p.ends_with(".fg")
            && !out.contains(&p)
            && p != rel
        {
            out.push(p);
        }
    }
    out
}

fn dir_of_pin(h: &History, rel: &str, at: &Oid) -> std::path::PathBuf {
    crate::pins::path_of(&crate::pins::Pin {
        work: h.work().to_path_buf(),
        n: 0,
        save: at.hex(),
        rel: rel.to_string(),
    })
    .parent()
    .map(|p| p.to_path_buf())
    .unwrap_or_default()
}

fn join(base: &[&str], file: &str) -> Option<String> {
    let mut parts: Vec<&str> = base[..base.len() - 1].to_vec();
    for c in file.split('/') {
        match c {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            x => parts.push(x),
        }
    }
    Some(parts.join("/"))
}

/// Restore: the file (and, for an assembly, the parts it follows as latest) as they were at
/// `to`, saved anew.
pub fn restore(file: &Path, to: &str, agent: Option<&str>) -> Result<Json, String> {
    let (h, rel) = open(file)?;
    let at = h
        .resolve(&rel, to)
        .map_err(|e| e.to_string())?
        .ok_or("restore needs a version or a save, not current")?;
    let mut paths = vec![rel.clone()];
    paths.extend(followed(&h, &rel, &at));
    let label = match h
        .versions(Some(&rel))
        .map_err(|e| e.to_string())?
        .iter()
        .find(|v| v.save == at)
    {
        Some(v) => format!("v{} ({})", v.n, v.name),
        None => format!("save {}", &at.hex()[..10]),
    };
    let r = h
        .restore(&paths, &at, &format!("{rel}: restored {label}"), agent)
        .map_err(|e| e.to_string())?;
    let save = match r {
        Saved::New(id) => Json::Str(short(&id)),
        Saved::Unchanged => Json::Null,
        Saved::Paused(p) => return Err(paused(&p)),
    };
    Ok(Json::obj(vec![
        ("file", Json::Str(rel)),
        ("restored", Json::Str(label)),
        ("from", Json::Str(short(&at))),
        ("save", save),
        ("files", Json::Arr(paths.into_iter().map(Json::Str).collect())),
    ]))
}

pub fn compare(file: &Path, a: &str, b: &str, volume: bool) -> Result<Json, String> {
    let (h, rel) = open(file)?;
    let c = compare::between(&h, &rel, a, b)?;
    let vol = match (volume, &c.base.run.body, &c.target.run.body) {
        (true, Some(x), Some(y)) => Some(compare::volume(x, y)),
        _ => None,
    };
    Ok(c.json(vol.as_ref()))
}

/// The tools' names and arguments, as `forgecad TOOL ..` takes them.
pub const USAGE: &str = "versions F | history F [--all] [--limit N] | create_version F NAME [NOTES] | \
restore F v3|SAVE | compare F A B [--volume]   (A, B: v3, a save id, or current) | \
release F REASON [--approver NAME] [--numeric] | releases F | regenerate_release F REV";

/// A command line's tool: (`versions`, [F]) and so on. `agent` marks what it writes.
pub fn call(tool: &str, args: &[String], agent: Option<&str>) -> Option<Result<Json, String>> {
    let flag = |f: &str| args.iter().any(|a| a == f);
    let pos: Vec<&str> = {
        let mut v = vec![];
        let mut skip = false;
        for a in args {
            if skip {
                skip = false;
                continue;
            }
            if a == "--limit" || a == "--approver" {
                skip = true;
                continue;
            }
            if !a.starts_with("--") {
                v.push(a.as_str());
            }
        }
        v
    };
    let limit = args
        .iter()
        .position(|a| a == "--limit")
        .and_then(|i| args.get(i + 1))
        .and_then(|n| n.parse().ok())
        .unwrap_or(200);
    let f = |i: usize| pos.get(i).map(Path::new);
    let need = |n: usize| -> Result<(), String> {
        if pos.len() < n {
            Err(format!("usage: {USAGE}"))
        } else {
            Ok(())
        }
    };
    Some(match tool {
        "versions" => need(1).and_then(|_| versions(f(0).expect("one"))),
        "history" => need(1).and_then(|_| history(f(0).expect("one"), flag("--all"), limit)),
        "create_version" => need(2).and_then(|_| {
            create_version(
                f(0).expect("one"),
                pos[1],
                pos.get(2).copied().unwrap_or(""),
                agent,
            )
        }),
        "restore" => need(2).and_then(|_| restore(f(0).expect("one"), pos[1], agent)),
        "compare" => need(3).and_then(|_| compare(f(0).expect("one"), pos[1], pos[2], flag("--volume"))),
        "release" => need(2).and_then(|_| {
            let approver = args
                .iter()
                .position(|a| a == "--approver")
                .and_then(|i| args.get(i + 1))
                .map_or("", String::as_str);
            crate::release::release(f(0).expect("one"), pos[1], approver, flag("--numeric"), agent)
                .map(|r| crate::release::record_json(&r))
        }),
        "releases" => need(1).and_then(|_| {
            let (h, rel) = open(f(0).expect("one"))?;
            let rs = crate::release::records(&h, Some(&rel))?;
            Ok(Json::obj(vec![
                ("file", Json::Str(rel)),
                (
                    "releases",
                    Json::Arr(rs.iter().map(crate::release::record_json).collect()),
                ),
            ]))
        }),
        "regenerate_release" => need(2).and_then(|_| {
            crate::release::regenerate(f(0).expect("one"), pos[1])
                .map(|g| crate::release::regenerated_json(&g))
        }),
        _ => return None,
    })
}
