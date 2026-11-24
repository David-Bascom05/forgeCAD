//! NC export from the workspace, the files `forgecad cam F post` writes (forge_cam_lang::export):
//!
//! - **Post**: a program's files into the document's folder, named and placed by its statement
//!   (`file:`, `ext:`, `out:`), in the project in the browser;
//! - **Export…**: the host's Save As. Natively a save dialog (one file) or a folder (several,
//!   replacing files only when confirmed), starting at the folder last exported to; in the browser
//!   the page's save picker where there is one, else a download, several files in one zip;
//! - each program's state against its file: not posted, posted (the file is what it would
//!   post now), stale (the ops changed since: re-post), or refused (an op of it is).
use super::CamWs;
use forge_cam_lang::export::{self, NcFile, ProgIn};
use std::path::{Path, PathBuf};

/// Files the host is asked to save with its Save As.
#[derive(Clone, Debug, PartialEq)]
pub struct ExportAsk {
    pub files: Vec<NcFile>,
    /// Where the dialog starts (the document's folder, or the program's `out:` in it).
    pub dir: PathBuf,
    /// The zip's name in the browser when there are several files.
    pub zip: String,
}

/// A program's posted file against what it would post now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Posted {
    /// No file at its path.
    No,
    /// The file is what it would post now.
    Current,
    /// The file differs: the ops changed since it was posted.
    Stale,
    /// An op it posts is refused: no text to post.
    Refused,
}

/// A program's name, its file's state and its path (relative to the document).
pub type PostedRow = (String, Posted, String);

impl Posted {
    pub fn words(self) -> &'static str {
        match self {
            Posted::No => "not posted",
            Posted::Current => "posted",
            Posted::Stale => "stale: the ops changed since it was posted",
            Posted::Refused => "not posted: an op is refused",
        }
    }
}

/// The key the posted states are kept under: the document, its run and the files written since.
fn key(w: &CamWs) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::hash::DefaultHasher::new();
    w.text.hash(&mut h);
    w.run.as_ref().map(|r| &r.text).hash(&mut h);
    w.allow_uncertified.hash(&mut h);
    w.write_gen.hash(&mut h);
    h.finish()
}

impl CamWs {
    /// The run's programs, as forge_cam_lang::export takes them.
    pub fn prog_ins(&self) -> Vec<ProgIn> {
        let Some(r) = &self.run else { return vec![] };
        r.programs
            .iter()
            .map(|p| ProgIn {
                name: p.name.clone(),
                post: p.post.clone(),
                text: p.text.clone(),
                error: p.error.clone(),
                certified: p.certified.clone(),
            })
            .collect()
    }

    /// The files of program `which` (or all), refused as the CLI refuses them; the words say
    /// the workspace's way past an uncertified program.
    pub fn nc_files(&self, which: Option<&str>) -> Result<Vec<NcFile>, String> {
        if self.run.is_none() {
            return Err("generate first".into());
        }
        export::files(&self.text, &self.prog_ins(), which, self.allow_uncertified).map_err(|e| {
            e.text()
                .replace("export it anyway with --allow-uncertified", "tick Allow uncertified to post it anyway")
        })
    }

    /// Post: the files written into the document's folder.
    pub fn post_programs(&mut self, which: Option<&str>) {
        match self.nc_files(which) {
            Ok(files) => {
                for f in files {
                    let p = self.dir.join(&f.path).display().to_string();
                    self.want_write.push((p, f.text));
                }
            }
            Err(e) => self.said = Some(e),
        }
    }

    /// Re-post every program whose file is stale.
    pub fn repost_stale(&mut self) {
        let stale: Vec<String> = self
            .posted()
            .into_iter()
            .filter(|p| p.1 == Posted::Stale)
            .map(|p| p.0)
            .collect();
        if stale.is_empty() {
            self.said = Some("every posted program is current".into());
        }
        for n in stale {
            self.post_programs(Some(&n));
        }
    }

    /// Export…: the host's Save As for program `which` (or all).
    pub fn export_programs(&mut self, which: Option<&str>) {
        match self.nc_files(which) {
            Ok(files) => {
                let first = files[0].path.rsplit_once('/').map(|(d, _)| self.dir.join(d));
                let stem = self.stem();
                self.export = Some(ExportAsk {
                    files,
                    dir: first.filter(|d| d.is_dir()).unwrap_or_else(|| self.dir.clone()),
                    zip: format!("{stem}-nc.zip"),
                });
            }
            Err(e) => self.said = Some(e),
        }
    }

    /// The setup sheet (HTML) and the tool list (text), with the host's Save As.
    pub fn export_sheet(&mut self) {
        let Some(r) = &self.run else {
            self.said = Some("generate first".into());
            return;
        };
        let stem = self.stem();
        let tools = export::tool_list(&self.text).join("\n") + "\n";
        self.export = Some(ExportAsk {
            files: vec![
                NcFile {
                    path: format!("{stem}-sheet.html"),
                    text: r.sheet.clone(),
                },
                NcFile {
                    path: format!("{stem}-tools.txt"),
                    text: tools,
                },
            ],
            dir: self.dir.clone(),
            zip: format!("{stem}-sheet.zip"),
        });
    }

    /// The document's file name without `.fg`.
    fn stem(&self) -> String {
        Path::new(&self.path)
            .file_stem()
            .map_or_else(|| "cam".into(), |s| s.to_string_lossy().into_owned())
    }

    /// Each program: its name, its state, and the path its file has (relative to the document).
    pub fn posted(&mut self) -> Vec<PostedRow> {
        let k = key(self);
        if let Some((at, v)) = &self.posted_cache
            && *at == k
        {
            return v.clone();
        }
        let progs = self.prog_ins();
        let v: Vec<PostedRow> = progs
            .iter()
            .map(|p| {
                // what it would post now (stamped when not certified in full)
                match export::files(&self.text, &progs, Some(&p.name), true) {
                    Ok(f) => {
                        let path = f[0].path.clone();
                        let state = match forge_dirs::fs::read_to_string(self.dir.join(&path)) {
                            Err(_) => Posted::No,
                            Ok(t) if t == f[0].text => Posted::Current,
                            Ok(_) => Posted::Stale,
                        };
                        (p.name.clone(), state, path)
                    }
                    Err(_) => (p.name.clone(), Posted::Refused, String::new()),
                }
            })
            .collect();
        self.posted_cache = Some((k, v.clone()));
        v
    }
}

/// `files` written into `dir` by their file names (a program's `out:` is the document's
/// folder's, not the chosen one's): the paths written.
pub fn write_to(files: &[NcFile], dir: &Path) -> Result<Vec<PathBuf>, String> {
    forge_dirs::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut out = vec![];
    for f in files {
        let p = dir.join(base(&f.path));
        forge_dirs::fs::write(&p, f.text.as_bytes()).map_err(|e| format!("{}: {e}", p.display()))?;
        out.push(p);
    }
    Ok(out)
}

/// A path's file name.
pub fn base(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Natively: the system's save dialog (one file) or a folder (several), replacing existing
/// files only when the user confirms; the folder is remembered. What happened, in words.
#[cfg(not(target_arch = "wasm32"))]
pub fn save_dialog(x: ExportAsk) -> String {
    let start = crate::plat::kv_get("cam-export-dir")
        .map(PathBuf::from)
        .filter(|d| d.is_dir())
        .unwrap_or_else(|| x.dir.clone());
    if let [f] = x.files.as_slice() {
        let name = base(&f.path);
        let ext = name.rsplit_once('.').map_or("nc", |e| e.1);
        // the system's dialog asks before it replaces a file
        let Some(p) = rfd::FileDialog::new()
            .set_title("Export NC program")
            .set_directory(&start)
            .set_file_name(name)
            .add_filter("NC program", &[ext])
            .save_file()
        else {
            return "Export cancelled".into();
        };
        match forge_dirs::fs::write(&p, f.text.as_bytes()) {
            Ok(()) => {
                remember(p.parent());
                format!("Exported {}", p.display())
            }
            Err(e) => format!("Not exported: {}: {e}", p.display()),
        }
    } else {
        let Some(d) = rfd::FileDialog::new()
            .set_title("Export NC programs to a folder")
            .set_directory(&start)
            .pick_folder()
        else {
            return "Export cancelled".into();
        };
        let there: Vec<&str> = x.files.iter().map(|f| base(&f.path)).filter(|n| d.join(n).exists()).collect();
        if !there.is_empty() {
            let yes = rfd::MessageDialog::new()
                .set_title("Replace files?")
                .set_description(format!("{} already in {}: replace them?", there.join(", "), d.display()))
                .set_buttons(rfd::MessageButtons::YesNo)
                .show();
            if yes != rfd::MessageDialogResult::Yes {
                return format!("Not exported: {} kept", there.join(", "));
            }
        }
        match write_to(&x.files, &d) {
            Ok(ps) => {
                remember(Some(&d));
                format!("Exported {} files to {}", ps.len(), d.display())
            }
            Err(e) => format!("Not exported: {e}"),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn remember(d: Option<&Path>) {
    if let Some(d) = d {
        crate::plat::kv_set("cam-export-dir", &d.display().to_string());
    }
}

/// In the browser: what the page's `forge_export` takes (a name for the zip, the files).
pub fn page_json(x: &ExportAsk) -> String {
    use forge_agent::json::Json;
    Json::obj(vec![
        ("name", x.zip.clone().into()),
        (
            "files",
            Json::Arr(
                x.files
                    .iter()
                    .map(|f| Json::obj(vec![("path", base(&f.path).to_string().into()), ("text", f.text.clone().into())]))
                    .collect(),
            ),
        ),
    ])
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn several_files_go_into_a_folder_by_their_names() {
        let d = forge_dirs::temp_dir().join(format!("fg-cam-nc-{}", forge_dirs::process_id()));
        let files = vec![
            NcFile { path: "nc/a.nc".into(), text: "%\nG0\n%\n".into() },
            NcFile { path: "b.ngc".into(), text: "G1\n".into() },
        ];
        let ps = write_to(&files, &d).unwrap();
        assert_eq!(ps, vec![d.join("a.nc"), d.join("b.ngc")]);
        assert_eq!(forge_dirs::fs::read_to_string(d.join("a.nc")).unwrap(), "%\nG0\n%\n");
        let j = page_json(&ExportAsk { files, dir: d.clone(), zip: "x-nc.zip".into() });
        assert!(j.contains("\"path\":\"a.nc\"") && j.contains("x-nc.zip"), "{j}");
    }
}
