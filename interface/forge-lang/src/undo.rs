//! Undo and redo of a Forge file's edits: each edit written through
//! `write` is logged beside the file (`F.fg.log`, one JSON object a line: the text before and
//! after), and `step` takes the last one back or puts the last taken back in again. A step is
//! refused, nothing written, when the file is no longer the text the log ends on (a hand edit
//! since): taking it back would lose that edit.

use forge_agent::json::Json;
use std::path::{Path, PathBuf};

/// The log of the edits of `file`.
pub fn log_path(file: &Path) -> PathBuf {
    let mut p = file.as_os_str().to_os_string();
    p.push(".log");
    PathBuf::from(p)
}

/// One logged edit: the text before and after.
struct Edit {
    before: String,
    after: String,
}

/// The log's edits, and the stacks undo and redo take from (indices into the edits).
struct Log {
    edits: Vec<Edit>,
    undos: Vec<usize>,
    redos: Vec<usize>,
}

fn read(file: &Path) -> Log {
    let mut log = Log {
        edits: vec![],
        undos: vec![],
        redos: vec![],
    };
    let Ok(text) = forge_dirs::fs::read_to_string(log_path(file)) else {
        return log;
    };
    for line in text.lines() {
        let Ok(j) = Json::parse(line) else { continue };
        match j.get("kind").and_then(Json::str) {
            Some("do") => {
                let s = |k: &str| j.get(k).and_then(Json::str).unwrap_or("").to_string();
                log.undos.push(log.edits.len());
                log.edits.push(Edit {
                    before: s("before"),
                    after: s("after"),
                });
                log.redos.clear();
            }
            Some("undo") => {
                if let Some(i) = log.undos.pop() {
                    log.redos.push(i);
                }
            }
            Some("redo") => {
                if let Some(i) = log.redos.pop() {
                    log.undos.push(i);
                }
            }
            _ => {}
        }
    }
    log
}

fn append(file: &Path, j: Json) -> std::io::Result<()> {
    use std::io::Write;
    let mut line = String::new();
    j.write(&mut line);
    line.push('\n');
    forge_dirs::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path(file))?
        .write_all(line.as_bytes())
}

/// Writes `after` to `file` (which held `before`) and logs the edit.
pub fn write(file: &Path, before: &str, after: &str) -> std::io::Result<()> {
    forge_dirs::fs::write(file, after)?;
    if before != after {
        append(
            file,
            Json::obj(vec![
                ("kind", "do".into()),
                ("before", before.into()),
                ("after", after.into()),
            ]),
        )?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Undo,
    Redo,
}

/// Undoes the last edit, or redoes the last undone: the file's new text, written and logged.
pub fn step(file: &Path, step: Step) -> Result<String, String> {
    let log = read(file);
    let now = forge_dirs::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let (stack, word) = match step {
        Step::Undo => (&log.undos, "undo"),
        Step::Redo => (&log.redos, "redo"),
    };
    let Some(&i) = stack.last() else {
        return Err(format!("nothing to {word}"));
    };
    let e = &log.edits[i];
    let (from, to) = match step {
        Step::Undo => (&e.after, &e.before),
        Step::Redo => (&e.before, &e.after),
    };
    if &now != from {
        return Err(format!(
            "{} changed since its last logged edit: {word} would lose that change",
            file.display()
        ));
    }
    forge_dirs::fs::write(file, to).map_err(|e| format!("{}: {e}", file.display()))?;
    append(file, Json::obj(vec![("kind", word.into())])).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(to.clone())
}

/// The undo and redo stacks the log holds for `file` as it reads `now`: each step's text, the
/// oldest first (undo takes the last of `undos`, redo the last of `redos`). None when the log
/// does not end on `now` (no log, or a hand edit since): there is nothing to step from.
pub fn history(file: &Path, now: &str) -> Option<(Vec<String>, Vec<String>)> {
    let log = read(file);
    let at = match (log.undos.last(), log.redos.last()) {
        (Some(&i), _) => &log.edits[i].after,
        (None, Some(&i)) => &log.edits[i].before,
        (None, None) => return None,
    };
    if at != now {
        return None;
    }
    let undos = log.undos.iter().map(|&i| log.edits[i].before.clone()).collect();
    let redos = log.redos.iter().map(|&i| log.edits[i].after.clone()).collect();
    Some((undos, redos))
}

/// Writes the log of `file` anew to hold these stacks (as `history` gives them) about `now`, the
/// text the file holds: an app's undo depth kept with the file, for `fg edit F Undo` and the
/// next time it opens. No steps: no log.
pub fn keep(file: &Path, undos: &[String], now: &str, redos: &[String]) -> std::io::Result<()> {
    let path = log_path(file);
    if undos.is_empty() && redos.is_empty() {
        return match forge_dirs::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        };
    }
    let mut out = String::new();
    let mut line = |j: Json| {
        j.write(&mut out);
        out.push('\n');
    };
    let doit = |before: &str, after: &str| {
        Json::obj(vec![
            ("kind", "do".into()),
            ("before", before.into()),
            ("after", after.into()),
        ])
    };
    // the texts in order: the undone ones, now, then what redo puts back (the last redone first)
    let mut texts: Vec<&str> = undos.iter().map(String::as_str).collect();
    texts.push(now);
    texts.extend(redos.iter().rev().map(String::as_str));
    for w in texts.windows(2) {
        line(doit(w[0], w[1]));
    }
    for _ in redos {
        line(Json::obj(vec![("kind", "undo".into())]));
    }
    forge_dirs::fs::write(path, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_and_redo_walk_the_log() {
        let dir = forge_dirs::temp_dir().join(format!("forge-undo-{}", forge_dirs::process_id()));
        forge_dirs::fs::create_dir_all(&dir).expect("dir");
        let f = dir.join("p.fg");
        let _ = forge_dirs::fs::remove_file(log_path(&f));
        forge_dirs::fs::write(&f, "a").expect("write");
        write(&f, "a", "b").expect("edit");
        write(&f, "b", "c").expect("edit");
        assert_eq!(step(&f, Step::Undo).as_deref(), Ok("b"));
        assert_eq!(step(&f, Step::Undo).as_deref(), Ok("a"));
        assert!(step(&f, Step::Undo).is_err());
        assert_eq!(step(&f, Step::Redo).as_deref(), Ok("b"));
        // a new edit drops what was left to redo
        write(&f, "b", "d").expect("edit");
        assert!(step(&f, Step::Redo).is_err());
        assert_eq!(step(&f, Step::Undo).as_deref(), Ok("b"));
        // a hand edit since: refused, the file as it is
        forge_dirs::fs::write(&f, "hand").expect("write");
        assert!(step(&f, Step::Undo).is_err());
        assert_eq!(forge_dirs::fs::read_to_string(&f).expect("read"), "hand");
    }

    #[test]
    fn an_apps_stacks_kept_are_the_stacks_read_back_and_the_clis_to_step() {
        let dir = forge_dirs::temp_dir().join(format!("forge-undo-keep-{}", forge_dirs::process_id()));
        forge_dirs::fs::create_dir_all(&dir).expect("dir");
        let f = dir.join("p.fg");
        forge_dirs::fs::write(&f, "c").expect("write");
        let (undos, redos) = (
            vec!["a".to_string(), "b".to_string()],
            vec!["e".to_string(), "d".to_string()],
        );
        keep(&f, &undos, "c", &redos).expect("keep");
        assert_eq!(history(&f, "c"), Some((undos, redos)));
        // a hand edit since: nothing to step from
        assert_eq!(history(&f, "x"), None);
        assert_eq!(step(&f, Step::Redo).as_deref(), Ok("d"));
        assert_eq!(step(&f, Step::Undo).as_deref(), Ok("c"));
        assert_eq!(step(&f, Step::Undo).as_deref(), Ok("b"));
        keep(&f, &[], "b", &[]).expect("keep");
        assert!(!log_path(&f).exists());
    }
}
