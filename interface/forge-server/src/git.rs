//! The git writer (DESIGN.md §5): the op seq is the live history; git gets a commit only for a
//! **named version** (a forge-history save + version tag) or an **idle checkpoint** (a save),
//! never per op. The project folder stays a plain folder with git, clonable and openable offline
//! with today's code.

use forge_git::object::Sig;
use forge_history::{History, Saved};
use std::path::Path;

fn sig(author: &str) -> Sig {
    Sig {
        name: author.into(),
        // no one's real address: the account name at a reserved domain
        email: format!("{author}@forgecad.invalid"),
        time: forge_history::now(),
        tz: 0,
    }
}

/// Writes `text` to the document's file (whole: a fresh temporary file renamed over it).
fn write_doc(root: &Path, doc: &str, text: &str) -> Result<std::path::PathBuf, String> {
    let path = root.join(doc);
    let tmp = path.with_extension(format!("fg.{}.tmp", std::process::id()));
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })?;
    Ok(path)
}

fn history(root: &Path) -> Result<History, String> {
    match History::open(&root.join(".")) {
        Some(h) if h.work().canonicalize().ok() == root.canonicalize().ok() => Ok(h),
        _ => History::turn_on(root).map_err(|e| format!("{e:?}")),
    }
}

fn save(h: &History, path: &Path, summary: &str, who: &Sig) -> Result<(), String> {
    match h
        .save_as(path, &[], summary, None, who)
        .map_err(|e| format!("{e:?}"))?
    {
        Saved::New(_) | Saved::Unchanged => Ok(()),
        Saved::Paused(p) => Err(format!("paused: {p:?}")),
    }
}

/// The head as named version `name`: a save, then forge-history's version tag. Returns the
/// commit's id.
pub fn version(
    root: &Path,
    doc: &str,
    text: &str,
    name: &str,
    author: &str,
    seq: u64,
) -> Result<String, String> {
    let path = write_doc(root, doc, text)?;
    let h = history(root)?;
    let who = sig(author);
    save(&h, &path, &format!("{name} (seq {seq})"), &who)?;
    let v = h
        .create_version_as(doc, name, &format!("ForgeCAD-Seq: {seq}"), None, &who)
        .map_err(|e| format!("{e:?}"))?;
    Ok(v.save.hex())
}

/// The head saved after an idle period (no version tag).
pub fn checkpoint(root: &Path, doc: &str, text: &str, seq: u64) -> Result<(), String> {
    let path = write_doc(root, doc, text)?;
    let h = history(root)?;
    save(
        &h,
        &path,
        &format!("checkpoint at seq {seq}"),
        &sig("forge-server"),
    )
}
