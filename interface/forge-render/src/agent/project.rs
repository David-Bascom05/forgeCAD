//! Project paths and agent adapters for the same version tools used by the CLI and UI.
use forge_agent::json::Json;
use std::path::{Component, Path, PathBuf};

pub fn root(file: &Path) -> PathBuf {
    forge_history::History::open(file)
        .map(|h| h.work().to_path_buf())
        .unwrap_or_else(|| {
            if file.is_dir() {
                file.to_path_buf()
            } else {
                file.parent().unwrap_or(Path::new(".")).to_path_buf()
            }
        })
}

fn permitted(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('\\')
        && !name.chars().any(char::is_control)
        && Path::new(name).components().all(|c| match c {
            Component::Normal(s) => {
                let s = s.to_string_lossy();
                !s.starts_with('.')
                    && !matches!(s.as_ref(), "target" | "node_modules" | "releases" | "exports")
            }
            _ => false,
        })
}

pub fn text_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("fg" | "md" | "txt" | "csv")
    )
}

/// Walk existing parents without following a link, for reads and for paths not created yet.
pub fn path(root: &Path, name: &str, file: bool) -> Result<PathBuf, String> {
    if !permitted(name) {
        return Err("Use a project-relative path, without hidden, generated, or parent directories".into());
    }
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let full = root.join(name);
    if file && !text_file(&full) {
        return Err("Project files must be .fg, .md, .txt, or .csv".into());
    }
    let mut p = root;
    for part in Path::new(name).components() {
        p.push(part);
        match std::fs::symlink_metadata(&p) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err("Project paths cannot follow symbolic links".into());
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(full)
}

/// Deterministic bounded traversal. Git metadata, links and generated directories stay out.
pub fn files(root: &Path) -> Vec<PathBuf> {
    fn visit(
        root: &Path,
        dir: &Path,
        out: &mut Vec<PathBuf>,
        count: &mut usize,
        depth: usize,
        ignore: &mut Option<forge_git::ignore::Ignore>,
    ) {
        if depth > 32 || *count >= 10000 || out.len() >= 500 {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            if *count >= 10000 || out.len() >= 500 {
                break;
            }
            *count += 1;
            let p = entry.path();
            let Some(rel) = p.strip_prefix(root).ok().and_then(|p| p.to_str()) else {
                continue;
            };
            if !permitted(rel) {
                continue;
            }
            let Ok(kind) = entry.file_type() else { continue };
            if ignore.as_mut().is_some_and(|i| i.ignored(rel, kind.is_dir())) {
                continue;
            }
            if kind.is_dir() {
                visit(root, &p, out, count, depth + 1, ignore);
            } else if kind.is_file() && text_file(&p) {
                out.push(p);
            }
        }
    }
    let mut out = vec![];
    let mut ignore = forge_history::History::open(root).map(|h| {
        forge_git::ignore::Ignore::new(
            root,
            &h.repo.git_dir,
            h.repo
                .config
                .get("core.excludesfile")
                .map(|p| forge_git::config::expand(p, None)),
        )
    });
    visit(root, root, &mut out, &mut 0, 0, &mut ignore);
    out
}

pub fn is_version(name: &str) -> bool {
    matches!(
        name,
        "fg_versions"
            | "fg_history"
            | "fg_create_version"
            | "fg_restore"
            | "fg_compare"
            | "fg_release"
            | "fg_releases"
            | "fg_regenerate_release"
    )
}

pub fn call(name: &str, args: &Json, root: &Path) -> Result<Json, String> {
    let required = |key: &str| {
        args.get(key)
            .and_then(Json::str)
            .ok_or_else(|| format!("{name} needs `{key}`"))
    };
    if name == "project_info" {
        let h = forge_history::History::open(root);
        let entries = files(root);
        return Ok(Json::obj(vec![
            ("root", root.display().to_string().into()),
            ("history_enabled", h.is_some().into()),
            (
                "files",
                Json::Arr(
                    entries
                        .iter()
                        .filter_map(|p| p.strip_prefix(root).ok())
                        .map(|p| p.display().to_string().into())
                        .collect(),
                ),
            ),
            ("limit", 500.0.into()),
            ("possibly_truncated", (entries.len() == 500).into()),
        ]));
    }
    if name == "project_enable_history" {
        if args.get("confirmed") != Some(&Json::Bool(true)) {
            return Err("Enabling history requires an explicit user request and confirmed=true".into());
        }
        let identity = match (
            args.get("name").and_then(Json::str),
            args.get("email").and_then(Json::str),
        ) {
            (Some(name), Some(email))
                if !name.trim().is_empty()
                    && !email.trim().is_empty()
                    && !name.chars().any(char::is_control)
                    && !email.chars().any(char::is_control) =>
            {
                Some((name, email))
            }
            (None, None) => None,
            _ => {
                return Err(
                    "Supply both a nonempty name and email without control characters, or neither".into(),
                );
            }
        };
        let mut history = match forge_history::History::open(root) {
            Some(h) => h,
            None => forge_history::History::turn_on(root).map_err(|e| e.to_string())?,
        };
        if let Some((name, email)) = identity {
            history.set_identity(name, email).map_err(|e| e.to_string())?;
        }
        return Ok(Json::obj(vec![
            ("root", history.work().display().to_string().into()),
            ("history_enabled", true.into()),
            ("identity_configured", history.who().is_some().into()),
        ]));
    }
    if name == "project_mkdir" {
        let p = path(root, required("path")?, false)?;
        std::fs::create_dir_all(&p).map_err(|e| e.to_string())?;
        return Ok(Json::obj(vec![("created", required("path")?.into())]));
    }
    if matches!(name, "project_read" | "project_write") {
        let p = path(root, required("file")?, true)?;
        if name == "project_read" {
            if std::fs::metadata(&p).map_err(|e| e.to_string())?.len() > 4 << 20 {
                return Err("File is too large (4 MiB maximum)".into());
            }
            let text = std::fs::read_to_string(p).map_err(|e| e.to_string())?;
            return Ok(Json::obj(vec![
                ("file", required("file")?.into()),
                ("text", text.into()),
            ]));
        }
        let text = required("text")?;
        if text.len() > 4 << 20 {
            return Err("File is too large (4 MiB maximum)".into());
        }
        if p.exists() {
            let old = required("old")?;
            if std::fs::read_to_string(&p).map_err(|e| e.to_string())? != old {
                return Err("File changed: read it again before writing".into());
            }
            let mut lock = forge_git::fsutil::Lock::take(&p).map_err(|e| e.to_string())?;
            // Check again under our write lock.
            if std::fs::read_to_string(&p).map_err(|e| e.to_string())? != old {
                return Err("File changed: read it again before writing".into());
            }
            lock.write(text.as_bytes())
                .and_then(|_| lock.commit())
                .map_err(|e| e.to_string())?;
        } else {
            if args.get("old").is_some() {
                return Err("File no longer exists: read the project again".into());
            }
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            forge_git::fsutil::write_new(&p, text.as_bytes()).map_err(|e| e.to_string())?;
        }
        return Ok(Json::obj(vec![("written", required("file")?.into())]));
    }
    let f = super::tools::in_folder(root, required("file")?)?;
    let mut values = vec![f.display().to_string()];
    match name {
        "fg_versions" | "fg_releases" => {}
        "fg_history" => {
            if args.get("all") == Some(&Json::Bool(true)) {
                values.push("--all".into());
            }
            if let Some(n) = args.get("limit").and_then(Json::num) {
                if !n.is_finite() || n.fract() != 0.0 || !(1.0..=1000.0).contains(&n) {
                    return Err("limit must be an integer from 1 to 1000".into());
                }
                values.extend(["--limit".into(), (n as usize).to_string()]);
            }
        }
        "fg_create_version" => values.extend([
            required("name")?.into(),
            args.get("notes").and_then(Json::str).unwrap_or("").into(),
        ]),
        "fg_restore" => values.push(required("to")?.into()),
        "fg_compare" => {
            values.extend([required("base")?.into(), required("target")?.into()]);
            if args.get("volume") == Some(&Json::Bool(true)) {
                values.push("--volume".into());
            }
        }
        "fg_release" => {
            if args.get("confirmed") != Some(&Json::Bool(true)) {
                return Err(
                    "Release requires confirmed=true after the user explicitly requests this release".into(),
                );
            }
            values.push(required("reason")?.into());
            if let Some(who) = args.get("approver").and_then(Json::str) {
                values.extend(["--approver".into(), who.into()]);
            }
            if args.get("numeric") == Some(&Json::Bool(true)) {
                values.push("--numeric".into());
            }
        }
        "fg_regenerate_release" => values.push(required("revision")?.into()),
        _ => return Err(format!("Unknown project tool: {name}")),
    }
    // vtools::call accepts CLI arguments; reject flag-looking user values instead of letting
    // a name/reason become a CLI option. No shell is involved.
    for key in [
        "name", "notes", "reason", "approver", "to", "base", "target", "revision",
    ] {
        if args
            .get(key)
            .and_then(Json::str)
            .is_some_and(|v| v.starts_with("--"))
        {
            return Err(format!("`{key}` cannot start with --"));
        }
    }
    if matches!(name, "fg_create_version" | "fg_release") {
        if name == "fg_release" && required("reason")?.trim().is_empty() {
            return Err("A release needs a reason".into());
        }
        // A new assembly can read inputs that already existed before this turn. Include
        // those untracked inputs, as a normal document save does, so its version can rebuild.
        let history =
            forge_history::History::open(&f).ok_or("This project keeps no history; turn history on first")?;
        let source = std::fs::read_to_string(&f).map_err(|e| e.to_string())?;
        let watching = forge_lang::deps::watch();
        if forge_lang::drawing::is_drawing(&source) {
            let _ = forge_lang::drawing::regen::make_in(&source, f.parent().unwrap_or(root), Some(root));
        } else {
            let _ = forge_lang::run(
                &source,
                forge_lang::Options {
                    build: true,
                    dir: f.parent().map(Path::to_path_buf),
                    root: Some(root.to_path_buf()),
                    ..Default::default()
                },
                Default::default(),
            );
        }
        let inputs: Vec<_> = watching
            .done()
            .0
            .into_iter()
            .filter_map(|(p, seen)| matches!(seen, forge_lang::deps::Seen::Bytes(_)).then_some(p))
            .collect();
        match history
            .save(&f, &inputs, "Agent version checkpoint", Some("agent"))
            .map_err(|e| e.to_string())?
        {
            forge_history::Saved::Paused(why) => return Err(format!("History is paused: {why:?}")),
            _ => {}
        }
    }
    forge_lang::vtools::call(name.strip_prefix("fg_").unwrap_or(name), &values, Some("agent"))
        .ok_or_else(|| format!("Unknown version tool: {name}"))?
}
