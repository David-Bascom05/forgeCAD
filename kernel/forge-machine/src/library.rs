//! The built-in machines (data/machines.toml) and lookup with the project/user search order of
//! forge-tooling: a project's `cam/machines/*.toml`, then the user's, then these.
use crate::def::{DefError, MachineDef};

pub const BUILTIN: &str = include_str!("../data/machines.toml");

/// The built-in generic machines; the file is ours and is tested to parse, so a failure here is a
/// bug in this crate.
pub fn builtin() -> Vec<MachineDef> {
    MachineDef::parse_all(BUILTIN).expect("built-in machines.toml must parse")
}

#[derive(Clone, Debug, PartialEq)]
pub enum LookupError {
    Def { file: String, err: DefError },
    Unknown { name: String, near: Vec<String> },
    Duplicate { name: String, files: [String; 2] },
}

impl std::fmt::Display for LookupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LookupError::Def { file, err } => write!(f, "{file}: {err}"),
            LookupError::Unknown { name, near } => {
                write!(f, "no machine `{name}`; known: {}", near.join(", "))
            }
            LookupError::Duplicate { name, files } => {
                write!(f, "machine `{name}` is in both {} and {}", files[0], files[1])
            }
        }
    }
}

/// Machines from (file name, text) sources in search order: a name found earlier hides a later
/// one (project over user over built-in); within one level a name twice is an error.
#[derive(Clone, Debug, Default)]
pub struct Library {
    pub machines: Vec<(MachineDef, String)>,
}

impl Library {
    pub fn with_builtin() -> Library {
        Library {
            machines: builtin()
                .into_iter()
                .map(|m| (m, "builtin/machines.toml".to_string()))
                .collect(),
        }
    }

    /// Adds a level above what is loaded (call project last to put it first).
    pub fn add_level(&mut self, files: &[(String, String)]) -> Result<(), LookupError> {
        let mut level: Vec<(MachineDef, String)> = Vec::new();
        for (file, src) in files {
            for m in MachineDef::parse_all(src).map_err(|err| LookupError::Def {
                file: file.clone(),
                err,
            })? {
                if let Some((_, f)) = level.iter().find(|(x, _)| x.name == m.name) {
                    return Err(LookupError::Duplicate {
                        name: m.name,
                        files: [f.clone(), file.clone()],
                    });
                }
                level.push((m, file.clone()));
            }
        }
        self.machines
            .retain(|(m, _)| !level.iter().any(|(x, _)| x.name == m.name));
        level.append(&mut self.machines);
        self.machines = level;
        Ok(())
    }

    pub fn get(&self, name: &str) -> Result<&MachineDef, LookupError> {
        self.machines
            .iter()
            .map(|(m, _)| m)
            .find(|m| m.name == name)
            .ok_or_else(|| LookupError::Unknown {
                name: name.into(),
                near: self.machines.iter().map(|(m, _)| m.name.clone()).collect(),
            })
    }
}
