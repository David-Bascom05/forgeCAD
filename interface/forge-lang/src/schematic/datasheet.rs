//! Store an explicitly imported PDF beside its owning .fg. A content address makes shared
//! datasheets portable and a second import idempotent on native and browser filesystems.
use forge_dirs::fs;
use std::{
    io::{self, Write},
    path::Path,
};
pub const MAX_PDF_BYTES: usize = 32 * 1024 * 1024;
pub fn store_pdf(owner: &Path, bytes: &[u8]) -> io::Result<String> {
    if !bytes.starts_with(b"%PDF-") || bytes.len() > MAX_PDF_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Choose a PDF of at most 32 MiB",
        ));
    }
    let parent = owner
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let root = fs::canonicalize(parent)?;
    let folder = root.join("datasheets");
    match fs::create_dir(&folder) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e),
    }
    if fs::canonicalize(&folder)? != folder {
        return Err(io::Error::other(
            "The datasheets folder must be inside the document folder",
        ));
    }
    let id = forge_git::HashKind::Sha256.of(bytes);
    let name = format!("{id}.pdf");
    let dest = folder.join(&name);
    match fs::OpenOptions::new().write(true).create_new(true).open(&dest) {
        Ok(mut f) => {
            if let Err(e) = f.write_all(bytes) {
                drop(f);
                let _ = fs::remove_file(&dest);
                return Err(e);
            }
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            if fs::canonicalize(&dest)? != dest
                || !fs::symlink_metadata(&dest)?.is_file()
                || fs::metadata(&dest)?.len() != bytes.len() as u64
                || fs::read(&dest)? != bytes
            {
                return Err(io::Error::other(
                    "Different content already occupies the datasheet path",
                ));
            }
        }
        Err(e) => return Err(e),
    }
    Ok(format!("datasheets/{name}"))
}

/// Preserve locally imported attachments when a document is saved in another folder.
pub fn copy_managed(old: &Path, new: &Path, link: &str) -> io::Result<()> {
    let Some(hash) = link
        .strip_prefix("datasheets/")
        .and_then(|name| name.strip_suffix(".pdf"))
    else {
        return Ok(());
    };
    if old.parent() == new.parent() || hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Ok(());
    }
    let base = old
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let path = base.join(link);
    if fs::canonicalize(&path)? != fs::canonicalize(base)?.join(link) {
        return Err(io::Error::other(
            "The datasheet must be inside the document folder",
        ));
    }
    let bytes = crate::deps::read_plain(&path, MAX_PDF_BYTES as u64)
        .map_err(|_| io::Error::other("Cannot read the original datasheet"))?;
    if forge_git::HashKind::Sha256.of(&bytes).to_string() != hash {
        return Err(io::Error::other("The original datasheet has changed"));
    }
    store_pdf(new, &bytes).map(|_| ())
}
