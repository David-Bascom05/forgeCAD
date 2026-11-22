//! Undo and redo of a Forge file's edits (`.fc`'s `act undo|redo`): each edit written through
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
    let Ok(text) = std::fs::read_to_string(log_path(file)) else {
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
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path(file))?
        .write_all(line.as_bytes())
}

/// Writes `after` to `file` (which held `before`) and logs the edit.
pub fn write(file: &Path, before: &str, after: &str) -> std::io::Result<()> {
    std::fs::write(file, after)?;
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
    let now = std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
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
    std::fs::write(file, to).map_err(|e| format!("{}: {e}", file.display()))?;
    append(file, Json::obj(vec![("kind", word.into())])).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(to.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_and_redo_walk_the_log() {
        let dir = std::env::temp_dir().join(format!("forge-undo-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let f = dir.join("p.fg");
        let _ = std::fs::remove_file(log_path(&f));
        std::fs::write(&f, "a").expect("write");
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
        std::fs::write(&f, "hand").expect("write");
        assert!(step(&f, Step::Undo).is_err());
        assert_eq!(std::fs::read_to_string(&f).expect("read"), "hand");
    }
}
