//! Every machine the search order sees, with its level and what it hides.
//!
//! Search order (docs/cam-machine.md): the project's `cam/machines/*.toml`, then the user's
//! `<config>/forgecad/cam/machines/*.toml`, then the built-in set; a name found earlier hides a
//! later one. Unlike `forge_machine::library::Library::add_level`, a file that does not parse
//! does not hide the rest: it is listed with its error, and the machines of other files load.
use forge_machine::def::MachineDef;
use forge_machine::library;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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

/// The user's machines folder: `<config>/forgecad/cam/machines`.
pub fn user_dir() -> Option<PathBuf> {
    Some(
        forge_dirs::config()?
            .join("forgecad")
            .join("cam")
            .join("machines"),
    )
}

/// A project's machines folder: `cam/machines` in the project.
pub fn project_dir(project: &Path) -> PathBuf {
    project.join("cam").join("machines")
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub def: MachineDef,
    pub level: Level,
    /// The file it is in (`builtin/<vendor>.toml` for the built-in set).
    pub file: String,
    /// Records of the same name at later levels, which this one hides: (level, file).
    pub hides: Vec<(Level, String)>,
}

#[derive(Clone, Debug, Default)]
pub struct Catalog {
    /// In search order; one per name (the one a lookup finds).
    pub entries: Vec<Entry>,
    /// Files that did not load: (level, file, why).
    pub errors: Vec<(Level, String, String)>,
}

/// (file, text) pairs, or (file, why it could not be read).
pub type Files = Vec<(String, String)>;

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

impl Catalog {
    /// The catalog from (file, text) per level (empty: nothing at that level).
    pub fn from_sources(project: &[(String, String)], user: &[(String, String)]) -> Catalog {
        let mut cat = Catalog::default();
        let mut all: Vec<(Level, MachineDef, String)> = Vec::new();
        for (level, files) in [(Level::Project, project), (Level::User, user)] {
            for (file, src) in files {
                match MachineDef::parse_all(src) {
                    Ok(ms) => {
                        for m in ms {
                            if all.iter().any(|(l, x, _)| *l == level && x.name == m.name) {
                                cat.errors.push((
                                    level,
                                    file.clone(),
                                    format!("machine `{}` is twice in the {} level", m.name, level.name()),
                                ));
                                continue;
                            }
                            all.push((level, m, file.clone()));
                        }
                    }
                    Err(e) => cat.errors.push((level, file.clone(), e.to_string())),
                }
            }
        }
        for (m, f) in library::builtin_files() {
            all.push((Level::Builtin, m, f));
        }
        for (level, def, file) in all {
            if let Some(e) = cat.entries.iter_mut().find(|e| e.def.name == def.name) {
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

    /// The catalog a project folder sees (`None`: no project level) with the user's folder.
    pub fn load(project: Option<&Path>, user: Option<&Path>) -> Catalog {
        let (p, pbad) = project.map(|d| read_level(&project_dir(d))).unwrap_or_default();
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

    pub fn get(&self, name: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.def.name == name)
    }

    /// The names, for a "did you mean" list.
    pub fn names(&self) -> Vec<String> {
        self.entries.iter().map(|e| e.def.name.clone()).collect()
    }

    /// As forge-machine's Library (what a setup's lookup reads), in the same order.
    pub fn library(&self) -> library::Library {
        library::Library {
            machines: self
                .entries
                .iter()
                .map(|e| (e.def.clone(), e.file.clone()))
                .collect(),
        }
    }
}

static GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Bumped by every save through `make::save`: a picker that loaded the catalog at generation g
/// reloads when this is no longer g.
pub fn generation() -> u64 {
    GENERATION.load(std::sync::atomic::Ordering::Relaxed)
}

pub(crate) fn bump() {
    GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}
