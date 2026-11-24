//! The shared library builder (research/cam/library-builder.md): what machines, cutting tools,
//! holders, collets and workholding have in common as libraries a user extends.
//!
//! - `Kind`: a kind of record, supplied by its data owner (reader, canonical writer, id, the
//!   built-in files, its form's schema).
//! - `Catalog<K>`: every record the search order sees: the project's `cam/<folder>/*.toml`, the
//!   user's `<config>/forgecad/cam/<folder>/`, then the built-in set; a record found earlier
//!   hides a later one of the same id; a file that does not load is listed with its error and
//!   hides nothing.
//! - `save`: a record (and the part files it brings) into a level's folder, read back before
//!   anything is written; `generation` moves on every save so pickers reload.
//! - `schema`: fields for the builder's forms (type, unit, need, provenance).
pub mod schema;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    Project,
    User,
    Builtin,
}

impl Level {
    pub fn name(self) -> &'static str {
        match self {
            Level::Project => "project",
            Level::User => "user",
            Level::Builtin => "built-in",
        }
    }
}

/// A kind of library record, as its data owner reads and writes it.
pub trait Kind: 'static {
    type Record: Clone + PartialEq + std::fmt::Debug;
    /// The command word and the record's name: "machine", "tool", ...
    const WORD: &'static str;
    /// Its folder under `cam/`: "machines", "tools", ...
    const FOLDER: &'static str;
    /// The built-in files: (file name as listed, text).
    fn builtin() -> Vec<(String, String)>;
    /// Every record in a file's text (the owner's reader: refuses with where and why).
    fn parse(text: &str) -> Result<Vec<Self::Record>, String>;
    /// A file of records, canonical: `write(parse(write(rs))) == write(rs)` byte for byte.
    fn write(header: &str, records: &[Self::Record]) -> Result<String, String>;
    fn id(r: &Self::Record) -> &str;
    /// The comment a saved file starts with.
    fn header(id: &str) -> String {
        format!("{id} — written by forgecad's library builder (research/cam/library-builder.md).")
    }
}

/// (file, text) pairs, or (file, why it could not be read).
pub type Files = Vec<(String, String)>;

/// The user's folder of a kind: `<config>/forgecad/cam/<folder>`.
pub fn user_dir<K: Kind>() -> Option<PathBuf> {
    Some(forge_dirs::config()?.join("forgecad").join("cam").join(K::FOLDER))
}

/// A project's folder of a kind: `cam/<folder>` in the project.
pub fn project_dir<K: Kind>(project: &Path) -> PathBuf {
    project.join("cam").join(K::FOLDER)
}

/// The `.toml` files of a folder, sorted by name, with their text; and those it could not read.
pub fn read_level(dir: &Path) -> (Files, Files) {
    let mut ok = Vec::new();
    let mut bad = Vec::new();
    let Ok(rd) = forge_dirs::fs::read_dir(dir) else {
        return (ok, bad);
    };
    let mut paths: Vec<PathBuf> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    paths.sort();
    for p in paths {
        match forge_dirs::fs::read_to_string(&p) {
            Ok(s) => ok.push((p.display().to_string(), s)),
            Err(e) => bad.push((p.display().to_string(), e.to_string())),
        }
    }
    (ok, bad)
}

#[derive(Clone, Debug)]
pub struct Entry<R> {
    pub def: R,
    pub level: Level,
    /// The file it is in (`builtin/<file>` for the built-in set).
    pub file: String,
    /// Records of the same id at later levels, which this one hides: (level, file).
    pub hides: Vec<(Level, String)>,
}

#[derive(Clone, Debug)]
pub struct Catalog<K: Kind> {
    /// In search order; one per id (the one a lookup finds).
    pub entries: Vec<Entry<K::Record>>,
    /// Files that did not load: (level, file, why).
    pub errors: Vec<(Level, String, String)>,
}

impl<K: Kind> Default for Catalog<K> {
    fn default() -> Self {
        Catalog {
            entries: Vec::new(),
            errors: Vec::new(),
        }
    }
}

impl<K: Kind> Catalog<K> {
    /// The catalog from (file, text) per level (empty: nothing at that level).
    pub fn from_sources(project: &[(String, String)], user: &[(String, String)]) -> Catalog<K> {
        let mut cat = Catalog::<K>::default();
        let builtin: Vec<(String, String)> = K::builtin()
            .into_iter()
            .map(|(f, s)| (format!("builtin/{f}"), s))
            .collect();
        let mut all: Vec<(Level, K::Record, String)> = Vec::new();
        for (level, files) in [
            (Level::Project, project),
            (Level::User, user),
            (Level::Builtin, &builtin[..]),
        ] {
            for (file, src) in files {
                match K::parse(src) {
                    Ok(rs) => {
                        for r in rs {
                            if all.iter().any(|(l, x, _)| *l == level && K::id(x) == K::id(&r)) {
                                cat.errors.push((
                                    level,
                                    file.clone(),
                                    format!(
                                        "{} `{}` is twice in the {} level",
                                        K::WORD,
                                        K::id(&r),
                                        level.name()
                                    ),
                                ));
                                continue;
                            }
                            all.push((level, r, file.clone()));
                        }
                    }
                    Err(e) => cat.errors.push((level, file.clone(), e)),
                }
            }
        }
        for (level, def, file) in all {
            if let Some(e) = cat.entries.iter_mut().find(|e| K::id(&e.def) == K::id(&def)) {
                e.hides.push((level, file));
            } else {
                cat.entries.push(Entry {
                    def,
                    level,
                    file,
                    hides: Vec::new(),
                });
            }
        }
        cat
    }

    /// The catalog a project folder (None: no project level) and the user's folder see.
    pub fn load(project: Option<&Path>, user: Option<&Path>) -> Catalog<K> {
        let (p, pbad) = project
            .map(|d| read_level(&project_dir::<K>(d)))
            .unwrap_or_default();
        let (u, ubad) = user.map(read_level).unwrap_or_default();
        let mut cat = Catalog::from_sources(&p, &u);
        for (f, e) in pbad {
            cat.errors.push((Level::Project, f, e));
        }
        for (f, e) in ubad {
            cat.errors.push((Level::User, f, e));
        }
        cat
    }

    pub fn get(&self, id: &str) -> Option<&Entry<K::Record>> {
        self.entries.iter().find(|e| K::id(&e.def) == id)
    }

    pub fn names(&self) -> Vec<String> {
        self.entries.iter().map(|e| K::id(&e.def).to_string()).collect()
    }
}

static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Moves on every save through `save`: a picker that loaded at generation g reloads when this is
/// no longer g.
pub fn generation() -> u64 {
    GENERATION.load(Ordering::Relaxed)
}

/// A record id the builder will write: it names the file, so `[A-Za-z0-9_.-]`, not starting
/// with a dot, 1..=64 characters.
pub fn check_id(id: &str) -> Result<(), String> {
    let ok = !id.is_empty()
        && id.len() <= 64
        && !id.starts_with('.')
        && id.chars().all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c));
    if ok {
        Ok(())
    } else {
        Err(format!(
            "`{id}`: a name is 1 to 64 of A-Z a-z 0-9 _ . - and does not start with a dot"
        ))
    }
}

/// Where a record is saved: `<dir>/<id>.toml`.
pub fn file_for(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.toml"))
}

/// Writes `record` (and the part files it brings: path relative to `dir`, text) into a level's
/// folder. Refuses the built-in level, an existing file unless `overwrite`, an id already at
/// this level in another file, and a text that does not read back as `record`.
pub fn save<K: Kind>(
    record: &K::Record,
    parts: &[(String, String)],
    dir: &Path,
    level: Level,
    overwrite: bool,
) -> Result<PathBuf, String> {
    if level == Level::Builtin {
        return Err(format!(
            "the built-in {}s are part of forgecad; save to the user or project level",
            K::WORD
        ));
    }
    let id = K::id(record);
    check_id(id)?;
    let file = file_for(dir, id);
    if forge_dirs::fs::exists(&file) && !overwrite {
        return Err(format!("{} exists (overwrite it explicitly)", file.display()));
    }
    let (others, _) = read_level(dir);
    for (f, src) in others {
        if Path::new(&f) == file {
            continue;
        }
        if let Ok(rs) = K::parse(&src)
            && rs.iter().any(|r| K::id(r) == id)
        {
            return Err(format!("{} `{id}` is already in {f}", K::WORD));
        }
    }
    let text = K::write(&K::header(id), std::slice::from_ref(record))?;
    let back = K::parse(&text).map_err(|e| format!("the written text does not load: {e}"))?;
    if back.len() != 1 || back[0] != *record {
        return Err("the written text loads as a different record (a writer bug)".into());
    }
    forge_dirs::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for (rel, t) in parts {
        let p = dir.join(rel);
        if let Some(parent) = p.parent() {
            forge_dirs::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        forge_dirs::fs::write(&p, t).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    forge_dirs::fs::write(&file, text).map_err(|e| format!("{}: {e}", file.display()))?;
    GENERATION.fetch_add(1, Ordering::Relaxed);
    Ok(file)
}
