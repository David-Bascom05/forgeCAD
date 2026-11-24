//! Every machine the search order sees, with its level and what it hides: forge-library's
//! catalog for the machine kind (research/cam/library-builder.md).
//!
//! Search order (docs/cam-machine.md): the project's `cam/machines/*.toml`, then the user's
//! `<config>/forgecad/cam/machines/*.toml`, then the built-in set; a name found earlier hides a
//! later one. Unlike `forge_machine::library::Library::add_level`, a file that does not parse
//! does not hide the rest: it is listed with its error, and the machines of other files load.
use forge_machine::def::MachineDef;
use forge_machine::library;
use std::path::{Path, PathBuf};

pub use forge_library::{Files, Level, generation, read_level};

/// The machine kind of library record.
pub struct Machine;

impl forge_library::Kind for Machine {
    type Record = MachineDef;
    const WORD: &'static str = "machine";
    const FOLDER: &'static str = "machines";
    fn builtin() -> Vec<(String, String)> {
        // the built-in files, as forge-machine embeds them, one text per file
        let mut files: Vec<(String, Vec<MachineDef>)> = Vec::new();
        for (m, f) in library::builtin_files() {
            let f = f.strip_prefix("builtin/").unwrap_or(&f).to_string();
            match files.iter_mut().find(|(x, _)| *x == f) {
                Some((_, ms)) => ms.push(m),
                None => files.push((f, vec![m])),
            }
        }
        files
            .into_iter()
            .map(|(f, ms)| {
                let text = crate::write::file("", &ms).expect("the built-in machines write");
                (f, text)
            })
            .collect()
    }
    fn parse(text: &str) -> Result<Vec<MachineDef>, String> {
        MachineDef::parse_all(text).map_err(|e| e.to_string())
    }
    fn write(header: &str, records: &[MachineDef]) -> Result<String, String> {
        crate::write::file(header, records)
    }
    fn id(r: &MachineDef) -> &str {
        &r.name
    }
    fn header(id: &str) -> String {
        format!("{id} — written by forgecad's machine builder (docs/cam-machine.md).")
    }
}

pub type Catalog = forge_library::Catalog<Machine>;
pub type Entry = forge_library::Entry<MachineDef>;

/// The user's machines folder: `<config>/forgecad/cam/machines`.
pub fn user_dir() -> Option<PathBuf> {
    forge_library::user_dir::<Machine>()
}

/// A project's machines folder: `cam/machines` in the project.
pub fn project_dir(project: &Path) -> PathBuf {
    forge_library::project_dir::<Machine>(project)
}

/// As forge-machine's Library (what a setup's lookup reads), in the same order.
pub fn library(cat: &Catalog) -> library::Library {
    library::Library {
        machines: cat
            .entries
            .iter()
            .map(|e| (e.def.clone(), e.file.clone()))
            .collect(),
    }
}
