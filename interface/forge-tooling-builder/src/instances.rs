//! Physical tool instances as a library kind of their own (forge_tooling::instances): one
//! record per tool on the shelf, id = its label, in `cam/tool-instances/<label>.toml` at the
//! project or user level (no built-in level). The user level is the default: a shop's tool crib
//! outlives its projects. An instance names its library tool by id and is resolved through the
//! tools catalog the project sees; its values are never copied in.
use forge_library::{Catalog, Kind, Level};
use forge_tooling::instances::{self, Instance, Store};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The `tool_instance` kind.
pub struct ToolInstances;

impl Kind for ToolInstances {
    type Record = Instance;
    const WORD: &'static str = "tool_instance";
    const FOLDER: &'static str = "tool-instances";
    fn builtin() -> Vec<(String, String)> {
        Vec::new()
    }
    fn parse(text: &str) -> Result<Vec<Instance>, String> {
        instances::read(text, "tool instance").map_err(|e| e.to_string())
    }
    fn write(header: &str, rs: &[Instance]) -> Result<String, String> {
        Ok(instances::write_with(header, rs))
    }
    fn id(r: &Instance) -> &str {
        &r.label
    }
}

/// Instances on disk: the project's and the user's `cam/tool-instances/` folders.
pub struct LevelStore {
    pub project: Option<PathBuf>,
    /// The user's `cam` folder (`<config>/forgecad/cam`).
    pub user_cam: Option<PathBuf>,
    /// Where a new instance is saved.
    pub default: Level,
    /// The level each loaded instance came from.
    seen: BTreeMap<String, Level>,
    /// Files that did not load.
    pub errors: Vec<(Level, String, String)>,
}

impl LevelStore {
    pub fn new(project: Option<&Path>, user_cam: Option<&Path>, default: Level) -> LevelStore {
        LevelStore {
            project: project.map(Path::to_path_buf),
            user_cam: user_cam.map(Path::to_path_buf),
            default,
            seen: BTreeMap::new(),
            errors: Vec::new(),
        }
    }

    fn dir(&self, level: Level) -> Result<PathBuf, String> {
        match level {
            Level::Project => self
                .project
                .as_ref()
                .map(|p| forge_library::project_dir::<ToolInstances>(p))
                .ok_or("no project folder".into()),
            Level::User => self
                .user_cam
                .as_ref()
                .map(|u| u.join(ToolInstances::FOLDER))
                .ok_or("no user folder (no config directory): use the project level".into()),
            Level::Builtin => Err("tool instances have no built-in level".into()),
        }
    }
}

impl Store for LevelStore {
    fn load(&mut self) -> Result<Vec<Instance>, String> {
        let user = self.user_cam.as_ref().map(|u| u.join(ToolInstances::FOLDER));
        let cat = Catalog::<ToolInstances>::load(self.project.as_deref(), user.as_deref());
        self.errors = cat.errors.clone();
        self.seen = cat
            .entries
            .iter()
            .map(|e| (e.def.label.clone(), e.level))
            .collect();
        Ok(cat.entries.into_iter().map(|e| e.def).collect())
    }

    fn save(&mut self, i: &Instance) -> Result<(), String> {
        let level = self.seen.get(&i.label).copied().unwrap_or(self.default);
        let dir = self.dir(level)?;
        forge_dirs::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        forge_library::save::<ToolInstances>(i, &[], &dir, level, true)?;
        self.seen.insert(i.label.clone(), level);
        Ok(())
    }
}

/// The store `forgecad tool log` uses: new instances at the user level, or the project's with
/// `project_level`.
pub fn store(project: &Path, user_cam: Option<&Path>, project_level: bool) -> LevelStore {
    LevelStore::new(
        Some(project),
        user_cam,
        if project_level || user_cam.is_none() { Level::Project } else { Level::User },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_tooling::instances::Event;

    #[test]
    fn one_file_per_instance_at_its_level() {
        let tmp = forge_dirs::temp_dir().join(format!("forge-instances-{}", forge_dirs::process_id()));
        let _ = forge_dirs::fs::remove_dir_all(&tmp);
        let (proj, user) = (tmp.join("p"), tmp.join("u"));
        let mut s = LevelStore::new(Some(&proj), Some(&user), Level::User);
        let mut a = Instance::new("EM6-a", "flat-6");
        a.event(Event::Observe, "2026-11-24", Some(0.05), "");
        s.save(&a).unwrap();
        assert!(user.join("tool-instances/EM6-a.toml").exists());
        // a project-level instance stays at the project level when saved again
        let mut s2 = LevelStore::new(Some(&proj), Some(&user), Level::Project);
        s2.save(&Instance::new("job-T3", "flat-10")).unwrap();
        let mut s3 = LevelStore::new(Some(&proj), Some(&user), Level::User);
        let all = s3.load().unwrap();
        assert_eq!(all.len(), 2);
        let mut j = all.iter().find(|i| i.label == "job-T3").unwrap().clone();
        j.retire();
        s3.save(&j).unwrap();
        assert!(proj.join("cam/tool-instances/job-T3.toml").exists());
        assert!(!user.join("tool-instances/job-T3.toml").exists());
        assert!(s3.save(&Instance::new("bad label", "flat-6")).is_err());
        let _ = forge_dirs::fs::remove_dir_all(&tmp);
    }
}
