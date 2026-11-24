//! New machines: from a template (a kinematic layout with placeholder numbers) or a clone of any
//! machine the catalog sees, written to the user's or the project's folder.
//!
//! Nothing a new machine holds is silently the user's value. A value copied from a template or
//! from another machine is cited `status = "unknown"` with a basis saying where it came from and
//! that it is not this machine's value, unless the source already cites it (a clone keeps its
//! source's cites). The user's own values (entered in the builder) carry no cite: in a user or
//! project file an uncited value is the user's (CheckReport shows it as status "user").
use crate::catalog::Level;
use forge_machine::def::{Cite, CiteStatus, MachineDef};
use std::path::{Path, PathBuf};

include!(concat!(env!("OUT_DIR"), "/parts.rs"));

const TEMPLATES: &str = include_str!("../data/templates.toml");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Template {
    Vmc3,
    Trunnion,
    HeadTable,
    Lathe,
    MillTurn,
    Gantry,
}

impl Template {
    pub const ALL: [Template; 6] = [
        Template::Vmc3,
        Template::Trunnion,
        Template::HeadTable,
        Template::Lathe,
        Template::MillTurn,
        Template::Gantry,
    ];
    /// The command-line word.
    pub fn word(self) -> &'static str {
        match self {
            Template::Vmc3 => "vmc3",
            Template::Trunnion => "trunnion",
            Template::HeadTable => "head-table",
            Template::Lathe => "lathe",
            Template::MillTurn => "mill-turn",
            Template::Gantry => "gantry",
        }
    }
    pub fn title(self) -> &'static str {
        match self {
            Template::Vmc3 => "VMC, 3-axis",
            Template::Trunnion => "5-axis trunnion (table-table)",
            Template::HeadTable => "5-axis head-table",
            Template::Lathe => "Lathe, 2-axis",
            Template::MillTurn => "Mill-turn",
            Template::Gantry => "Gantry router",
        }
    }
    pub fn from_word(w: &str) -> Option<Template> {
        Template::ALL.into_iter().find(|t| t.word() == w)
    }
    /// The machine it starts from (a built-in generic one, or the builder's own).
    pub fn source(self) -> MachineDef {
        let name = match self {
            Template::Vmc3 => "generic-vmc3",
            Template::Trunnion => "generic-trunnion-bc",
            Template::Lathe => "generic-lathe",
            Template::MillTurn => "generic-mill-turn",
            Template::HeadTable => "template-head-table",
            Template::Gantry => "template-gantry",
        };
        if let Some(m) = forge_machine::library::builtin_named(name) {
            return m;
        }
        // ours, tested to parse (tests/make.rs)
        MachineDef::parse_all(TEMPLATES)
            .expect("the builder's templates parse")
            .into_iter()
            .find(|m| m.name == name)
            .expect("every template is in templates.toml")
    }
}

/// A built-in body part's text by its `part` path (`parts/<vendor>/<machine>/<body>.fg`).
pub fn builtin_part(path: &str) -> Option<&'static str> {
    PARTS.iter().find(|(p, _)| *p == path).map(|(_, s)| *s)
}

/// A machine name the builder will write: it names the file, so `[A-Za-z0-9_.-]`, not starting
/// with a dot, 1..=64 characters.
pub fn check_name(name: &str) -> Result<(), String> {
    let ok = !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c));
    if ok {
        Ok(())
    } else {
        Err(format!(
            "`{name}`: a machine name is 1 to 64 of A-Z a-z 0-9 _ . - and does not start with a dot"
        ))
    }
}

/// What to do with values copied from the source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Copied {
    /// Keep the source's cites; every value it does not cite is cited "unknown" (copied, not
    /// this machine's).
    Mark,
    /// Keep the source's cites only (values it leaves uncited then read as the user's own).
    KeepCites,
    /// Drop every cite: the user takes every value as their own.
    Own,
}

/// A machine ready to save: its record and the part files it brings (path relative to the
/// machine file's folder, text).
#[derive(Clone, Debug)]
pub struct Made {
    pub def: MachineDef,
    pub parts: Vec<(String, String)>,
}

/// The basis of a copied value's cite.
pub fn copied_basis(from: &str) -> String {
    format!(
        "copied from `{from}`: not measured on this machine and not known to be conservative for it; enter this machine's value"
    )
}

/// `src` under a new name. `from` says where it came from (shown in `source` and cites);
/// `part_text` gives a body part's text by the source's `part` path (None: the body keeps its
/// envelope and loses its part, said in the result's note).
pub fn clone_machine(
    src: &MachineDef,
    from: &str,
    name: &str,
    copied: Copied,
    today: &str,
    part_text: &dyn Fn(&str) -> Option<String>,
) -> Result<Made, String> {
    check_name(name)?;
    let mut d = src.clone();
    d.name = name.to_string();
    d.real = false;
    d.source = format!("made from `{from}` ({})", src.source);
    d.taken = today.to_string();
    let mut lost = Vec::new();
    let mut parts = Vec::new();
    for b in &mut d.bodies {
        if let Some(p) = b.part.take() {
            match part_text(&p) {
                Some(t) => {
                    let rel = format!("parts/{name}/{}.fg", b.name);
                    check_name(&b.name).map_err(|e| format!("body {e}"))?;
                    parts.push((rel.clone(), t));
                    b.part = Some(rel);
                }
                None => lost.push(b.name.clone()),
            }
        }
    }
    d.note = if lost.is_empty() {
        src.note.clone()
    } else {
        format!(
            "{}{}bodies without their part (not found): {}",
            src.note,
            if src.note.is_empty() { "" } else { "; " },
            lost.join(", ")
        )
    };
    match copied {
        Copied::Own => d.cites.clear(),
        Copied::KeepCites => {}
        Copied::Mark => {
            for p in d.cite_paths() {
                if !d.cites.iter().any(|c| c.path == p) {
                    d.cites.push(Cite {
                        path: p,
                        status: CiteStatus::Unknown,
                        source: String::new(),
                        taken: String::new(),
                        value: String::new(),
                        basis: copied_basis(from),
                        note: String::new(),
                    });
                }
            }
        }
    }
    d.validate().map_err(|e| e.to_string())?;
    d.validate_cites().map_err(|e| e.to_string())?;
    Ok(Made { def: d, parts })
}

/// A new machine from a template: every value cited as copied from it.
pub fn from_template(t: Template, name: &str, today: &str) -> Result<Made, String> {
    let src = t.source();
    clone_machine(
        &src,
        &format!("template {}", t.word()),
        name,
        Copied::Mark,
        today,
        &|p| builtin_part(p).map(str::to_string),
    )
}

/// Where a machine of `level` is saved: `<dir>/<name>.toml`.
pub fn file_for(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}.toml"))
}

/// Writes a made machine (its file and parts) into a level's folder. Refuses to overwrite a
/// different machine's file or to save a name that is already at this level in another file.
pub fn save(made: &Made, dir: &Path, level: Level, overwrite: bool) -> Result<PathBuf, String> {
    if level == Level::Builtin {
        return Err("the built-in machines are part of forgecad; save to the user or project level".into());
    }
    let file = file_for(dir, &made.def.name);
    if forge_dirs::fs::exists(&file) && !overwrite {
        return Err(format!("{} exists (overwrite it explicitly)", file.display()));
    }
    let (others, _) = crate::catalog::read_level(dir);
    for (f, src) in others {
        if Path::new(&f) == file {
            continue;
        }
        if let Ok(ms) = MachineDef::parse_all(&src)
            && ms.iter().any(|m| m.name == made.def.name)
        {
            return Err(format!("machine `{}` is already in {f}", made.def.name));
        }
    }
    let text = crate::write::file(
        &format!(
            "{} — written by forgecad's machine builder (docs/cam-machine.md).",
            made.def.name
        ),
        std::slice::from_ref(&made.def),
    )?;
    // read back before writing: what is saved must load as what was made
    let back = MachineDef::parse_all(&text).map_err(|e| format!("the written text does not load: {e}"))?;
    if back.len() != 1 || back[0] != made.def {
        return Err("the written text loads as a different machine (a writer bug)".into());
    }
    forge_dirs::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for (rel, t) in &made.parts {
        let p = dir.join(rel);
        if let Some(parent) = p.parent() {
            forge_dirs::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        forge_dirs::fs::write(&p, t).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    forge_dirs::fs::write(&file, text).map_err(|e| format!("{}: {e}", file.display()))?;
    crate::catalog::bump();
    Ok(file)
}
