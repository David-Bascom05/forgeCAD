//! The Versions tab's host: fills its graph from the document's history, does what it asks
//! (create, restore, compare, open, release), builds both sides of a Compare and the graph's
//! thumbnails on threads of their own, and while comparing shows the two sides in the view:
//! the one the slider leans to solid, coloured by Compare's classes, the other as a ghost.

use super::Gui;
use crate::mesh::PartMesh;
use crate::render::{FaceColour, Renderer};
use crate::scene::{Placement, Scene};
use crate::shell::versions::{self, DiffRow, Filter, Node, SaveRow, VersionsAct};
use forge_history::History;
use forge_lang::compare::{self, Change};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

/// A Compare (or an Open) built, ready to show.
pub struct Ready {
    pub base: String,
    pub target: String,
    pub meshes: Vec<PartMesh>,
    pub base_cls: Vec<Change>,
    pub target_cls: Vec<Change>,
    pub rows: Vec<DiffRow>,
    pub counts: String,
    pub volume: Option<(f64, f64)>,
    /// Only one side, shown as it is (Open).
    pub single: bool,
}

/// What the view held before a Compare took it, put back after.
struct Taken {
    renderer: Renderer,
    scene: Scene,
}

#[derive(Default)]
pub struct VersState {
    ready: Option<Receiver<Result<Ready, String>>>,

    shown: Option<Ready>,
    taken: Option<Taken>,
    /// The slider when the colours were last made, and those colours.
    made_at: Option<(f32, Option<usize>)>,
    /// The file whose released revision the browser shows.
    rev_of: Option<String>,
    pub colours: Vec<FaceColour>,
    pub ghosts: Vec<FaceColour>,
}

fn rgba(c: egui::Color32, a: u8) -> [u8; 4] {
    [c.r(), c.g(), c.b(), a]
}

fn rows_of(c: &compare::Comparison) -> Vec<DiffRow> {
    let mut out = vec![];
    for (group, items) in [
        ("Features", &c.features),
        ("Parameters", &c.params),
        ("Occurrences", &c.occurrences),
    ] {
        for x in items.iter().filter(|x| x.change != Change::Same) {
            let summary = match group {
                "Occurrences" => match x.change {
                    Change::Changed => "placed differently".to_string(),
                    c => c.word().to_string(),
                },
                _ => x.summary(),
            };
            out.push(DiffRow {
                group,
                name: x.name.clone(),
                change: x.change.word(),
                kind: x.kind.clone(),
                summary,
                faces: x.faces.clone(),
            });
        }
    }
    out
}

fn mesh(b: &forge_topo::Body) -> PartMesh {
    crate::tess::build_part(b).unwrap_or_else(|_| PartMesh::empty())
}

/// Both sides built, their faces classed, their meshes made: on a thread.
fn build(file: PathBuf, a: String, b: String, single: bool) -> Result<Ready, String> {
    let h = History::open(&file).ok_or("no history here")?;
    let rel = h.rel(&file).map_err(|e| e.to_string())?;
    if single {
        let at = h.resolve(&rel, &a).map_err(|e| e.to_string())?;
        let side = compare::build_at(&h, &rel, at.as_ref(), &a)?;
        let body = side.run.body.as_ref().ok_or("that save does not build a part")?;
        return Ok(Ready {
            base: a.clone(),
            target: a,
            meshes: vec![mesh(body)],
            base_cls: vec![],
            target_cls: vec![],
            rows: vec![],
            counts: String::new(),
            volume: None,
            single: true,
        });
    }
    let c = compare::between(&h, &rel, &a, &b)?;
    let rows = rows_of(&c);
    let mut volume = None;
    let (meshes, base_cls, target_cls, counts) = match (&c.base.run.body, &c.target.run.body, &c.faces) {
        (Some(x), Some(y), Some(f)) => {
            volume = compare::volume(x, y).ok().map(|v| (v.added, v.removed));
            (
                vec![mesh(y), mesh(x)],
                f.base.clone(),
                f.target.clone(),
                format!(
                    "{} changed · {} added · {} removed faces",
                    f.changed, f.added, f.removed
                ),
            )
        }
        _ => (vec![], vec![], vec![], String::new()),
    };
    Ok(Ready {
        base: a,
        target: b,
        meshes,
        base_cls,
        target_cls,
        rows,
        counts,
        volume,
        single: false,
    })
}

impl Gui {
    /// The document the tab shows: the one picked, else the open one.
    fn vers_doc(&self) -> Option<PathBuf> {
        self.shell
            .versions
            .doc
            .as_ref()
            .map(|d| d.1.clone())
            .or_else(|| self.fg.as_ref().and_then(|s| s.path.clone()).map(PathBuf::from))
    }

    /// The open document and the part files it places, for the tab's list.
    fn vers_docs(&self) -> Vec<(String, PathBuf)> {
        let Some(s) = &self.fg else { return vec![] };
        let Some(p) = s.path.as_ref().map(PathBuf::from) else {
            return vec![];
        };
        let dir = p.parent().map(Path::to_path_buf).unwrap_or_default();
        let mut out = vec![(name(&p), p.clone())];
        let mut rest = s.src.as_str();
        while let Some(i) = rest.find("part(\"") {
            rest = &rest[i + 6..];
            if let Some(j) = rest.find('"') {
                let f = &rest[..j];
                let q = dir.join(f);
                if f.ends_with(".fg") && !out.iter().any(|o| o.1 == q) {
                    out.push((f.to_string(), q));
                }
            }
        }
        out
    }

    fn vers_load(&mut self) {
        let docs = self.vers_docs();
        let doc = self.vers_doc();
        let v = &mut self.shell.versions;
        v.docs = docs;
        let Some(file) = doc else {
            v.nodes.clear();
            v.off = false;
            return;
        };
        if v.doc.as_ref().is_none_or(|d| d.1 != file) {
            v.doc = Some((name(&file), file.clone()));
        }
        let Some(h) = History::open(&file) else {
            v.off = true;
            v.nodes.clear();
            return;
        };
        v.off = false;
        let Ok(rel) = h.rel(&file) else {
            v.nodes.clear();
            return;
        };
        let path = match v.filter {
            Filter::Part => Some(rel.as_str()),
            _ => None,
        };
        let saves = h.saves(path).unwrap_or_default();
        let mut vs = h.versions(path).unwrap_or_default();
        let releases = forge_lang::release::records(&h, path).unwrap_or_default();
        v.revision = self
            .fg
            .as_ref()
            .and_then(|s| s.path.as_deref())
            .and_then(|p| forge_lang::release::revision_of(Path::new(p)))
            .unwrap_or_default();
        if v.filter == Filter::Releases {
            vs.retain(|x| releases.iter().any(|r| r.path == x.path && r.version == x.n));
        }
        v.nodes = graph(&saves, &vs, &releases, v.filter == Filter::Releases);
        v.pins = self
            .fg
            .as_ref()
            .and_then(|s| s.path.as_deref().map(|p| (s.src.as_str(), p)))
            .map(|(src, p)| pinned_rows(src, Path::new(p)))
            .unwrap_or_default();
        v.note.clear();
    }

    /// Each frame: what the tab asked, and what the threads have finished.
    pub fn versions_frame(&mut self) {
        // the open document's released revision, read once a document
        let path = self.fg.as_ref().and_then(|s| s.path.clone());
        if path != self.vers.rev_of {
            self.shell.versions.revision = path
                .as_deref()
                .and_then(|p| forge_lang::release::revision_of(Path::new(p)))
                .unwrap_or_default();
            self.vers.rev_of = path;
        }
        for a in std::mem::take(&mut self.shell.versions.out) {
            self.vers_act(a);
        }

        if let Some(rx) = &self.vers.ready
            && let Ok(got) = rx.try_recv()
        {
            self.vers.ready = None;
            match got {
                Ok(r) => self.vers_show(r),
                Err(e) => {
                    self.shell.versions.note = e.clone();
                    self.shell.status = e;
                }
            }
        }
        let s = (self.shell.versions.slider, self.shell.versions.focus);
        if self.vers.shown.is_some() && self.vers.made_at != Some(s) {
            self.vers_colours();
        }
    }

    fn vers_act(&mut self, a: VersionsAct) {
        let file = self.vers_doc();
        match a {
            VersionsAct::Load => self.vers_load(),
            VersionsAct::Show(p) => {
                self.shell.versions.doc = Some((name(&p), p));
                self.shell.versions.sel = None;
                self.vers_load();
            }
            VersionsAct::TurnOn => {
                let Some(f) = file else { return };
                let dir = f.parent().map(Path::to_path_buf).unwrap_or_default();
                match History::turn_on(&dir) {
                    Ok(_) => {
                        forge_lang::pins::forget();
                        let _ = forge_lang::vtools::save(&f, "saved", None);
                        self.shell.status = "History is on: every save is kept".into();
                    }
                    Err(e) => self.shell.versions.note = e.to_string(),
                }
                self.vers_load();
            }
            VersionsAct::Create { name: n, notes } => {
                let Some(f) = file else { return };
                if self.vers_is_open(&f) && self.fg.as_ref().is_some_and(|s| s.dirty()) {
                    self.fg_save();
                }
                match forge_lang::vtools::create_version(&f, &n, &notes, None) {
                    Ok(j) => {
                        let n = j.get("n").and_then(|x| x.num()).unwrap_or(0.0);
                        self.shell.status = format!("Version {n} of {} created", name(&f));
                    }
                    Err(e) => self.shell.versions.note = e,
                }
                forge_lang::pins::forget();
                self.vers_load();
            }
            VersionsAct::Restore(save) => {
                let Some(f) = file else { return };
                if self.vers_is_open(&f) && self.fg.as_ref().is_some_and(|s| s.dirty()) {
                    self.fg_save();
                }
                match forge_lang::vtools::restore(&f, &save, None) {
                    Ok(j) => {
                        self.shell.status = format!(
                            "Restored {} {}",
                            name(&f),
                            j.get("restored").and_then(|x| x.str()).unwrap_or("")
                        );
                        if self.vers_is_open(&f)
                            && let Ok(t) = std::fs::read_to_string(&f)
                            && let Some(s) = &mut self.fg
                        {
                            s.reload(t);
                        }
                        self.check_files();
                    }
                    Err(e) => self.shell.versions.note = e,
                }
                self.vers_load();
            }
            VersionsAct::Compare(a, b) => {
                let Some(f) = file else { return };
                self.vers_spawn(f, a, b, false);
            }
            VersionsAct::Open(save) => {
                let Some(f) = file else { return };
                self.vers_spawn(f, save.clone(), save, true);
            }
            VersionsAct::EndCompare => self.vers_end(),
            VersionsAct::Update(which) => {
                let Some(s) = &mut self.fg else { return };
                let mut text = s.src.clone();
                let mut done = vec![];
                for (inst, _, n, m) in self.shell.versions.pins.clone() {
                    if which.as_ref().is_some_and(|w| *w != inst) {
                        continue;
                    }
                    if let Some(t) = repin(&text, &inst, n, m) {
                        text = t;
                        done.push(format!("{inst} to v{m}"));
                    }
                }
                if !done.is_empty() {
                    s.external(text, &format!("Update {}", done.join(", ")), 0x7665_7273, false);
                    self.fg_text();
                    self.shell.status = format!("Updated {}", done.join(", "));
                }
                self.vers_load();
            }
            VersionsAct::CopyId(save) => {
                let label = self
                    .shell
                    .versions
                    .nodes
                    .iter()
                    .find(|n| n.save == save && n.n.is_some())
                    .map(|n| format!("{}@v{}", n.path, n.n.unwrap_or(0)))
                    .unwrap_or_else(|| save.clone());
                self.ctx.copy_text(label.clone());
                self.shell.status = format!("Copied {label}");
            }
            VersionsAct::Release {
                reason,
                approver,
                numeric,
            } => {
                let Some(f) = file else { return };
                if self.vers_is_open(&f) && self.fg.as_ref().is_some_and(|s| s.dirty()) {
                    self.fg_save();
                }
                match forge_lang::release::release(&f, &reason, &approver, numeric, None) {
                    Ok(r) => {
                        self.shell.status = format!("Released {} revision {}", name(&f), r.revision);
                    }
                    Err(e) => self.shell.versions.note = e,
                }
                self.vers_load();
            }
        }
    }

    fn vers_is_open(&self, f: &Path) -> bool {
        self.fg
            .as_ref()
            .and_then(|s| s.path.as_deref())
            .is_some_and(|p| Path::new(p) == f)
    }

    fn vers_spawn(&mut self, f: PathBuf, a: String, b: String, single: bool) {
        let (tx, rx) = channel();
        self.vers.ready = Some(rx);
        let waker = self.waker.clone();
        self.shell.status = if single {
            "Opening that version".into()
        } else {
            "Comparing".into()
        };
        std::thread::spawn(move || {
            let _ = tx.send(build(f, a, b, single));
            waker();
        });
    }

    /// A built Compare in the view: the two sides as two parts at one place.
    fn vers_show(&mut self, r: Ready) {
        self.shell.versions.comparing = Some((r.base.clone(), r.target.clone()));
        self.shell.versions.diff = r.rows.clone();
        self.shell.versions.counts = r.counts.clone();
        self.shell.versions.volume = r.volume;
        self.shell.versions.focus = None;
        self.shell.versions.single = r.single;
        self.shell.versions.slider = 1.0;
        self.shell.status = if r.single {
            format!("Viewing {}", self.shell.versions.label_of(&r.base))
        } else {
            format!(
                "Comparing {} to {}",
                self.shell.versions.label_of(&r.base),
                self.shell.versions.label_of(&r.target)
            )
        };
        if r.meshes.is_empty() {
            self.vers.shown = Some(r);
            return;
        }
        let Some(v) = &mut self.view else {
            self.vers.shown = Some(r);
            return;
        };
        let info: Vec<([f32; 3], f32, Vec<f32>)> = r
            .meshes
            .iter()
            .map(|m| (m.center, m.radius, m.lods.iter().map(|x| x.chord).collect()))
            .collect();
        let placements: Vec<Placement> = (0..r.meshes.len())
            .map(|k| Placement {
                tint: crate::palette::GREY,
                ..Placement::at(k as u32, [0.0; 3])
            })
            .collect();
        let mut scene = Scene::new(&info, placements);
        scene.set_explode(0.0);
        let mut renderer = Renderer::new(
            self.device.clone(),
            self.queue.clone(),
            self.adapter.clone(),
            crate::render::OFFSCREEN,
            (16, 16),
            &r.meshes,
            &scene,
        );
        renderer.resize(v.size);
        renderer.style = v.renderer.style;
        renderer.look = v.renderer.look.clone();
        renderer.back_colour = v.renderer.back_colour;
        renderer.set_tints(&scene);
        let old_r = std::mem::replace(&mut v.renderer, renderer);
        let old_s = std::mem::replace(&mut v.scene, scene);
        if self.vers.taken.is_none() {
            self.vers.taken = Some(Taken {
                renderer: old_r,
                scene: old_s,
            });
        }
        v.dirty = true;
        self.vers.shown = Some(r);
        self.vers.made_at = None;
    }

    /// The faces' colours for the slider where it is: the side it leans to solid and coloured,
    /// the other a ghost of its changed faces, strongest midway.
    fn vers_colours(&mut self) {
        let focus = self.shell.versions.focus;
        self.vers.made_at = Some((self.shell.versions.slider, focus));
        let s = self.shell.versions.slider.clamp(0.0, 1.0);
        let lit: Vec<usize> = focus
            .and_then(|i| self.shell.versions.diff.get(i))
            .map(|d| {
                if s >= 0.5 {
                    d.faces.1.clone()
                } else {
                    d.faces.0.clone()
                }
            })
            .unwrap_or_default();
        self.vers.colours.clear();
        self.vers.ghosts.clear();
        let Some(r) = &self.vers.shown else { return };
        let Some(v) = &mut self.view else { return };
        if r.single || r.meshes.len() < 2 {
            v.dirty = true;
            return;
        }
        let target_solid = s >= 0.5;
        v.scene.set_ghost(0, !target_solid);
        v.scene.set_ghost(1, target_solid);
        let mid = 1.0 - (2.0 * s - 1.0).abs();
        let ghost_a = (40.0 + 150.0 * mid) as u8;
        let class = |c: Change| match c {
            Change::Added => Some(versions::ADDED),
            Change::Removed => Some(versions::REMOVED),
            Change::Changed => Some(versions::CHANGED),
            _ => None,
        };
        let (solid, ghost, sc, gc) = if target_solid {
            (0u32, 1u32, &r.target_cls, &r.base_cls)
        } else {
            (1, 0, &r.base_cls, &r.target_cls)
        };
        for &i in &lit {
            self.vers.colours.push(FaceColour {
                placement: solid,
                face: i as u32,
                rgba: [0x4C, 0x9A, 0xFF, 235],
            });
        }
        for (i, c) in sc.iter().enumerate() {
            if lit.contains(&i) {
                continue;
            }
            if let Some(col) = class(*c) {
                self.vers.colours.push(FaceColour {
                    placement: solid,
                    face: i as u32,
                    rgba: rgba(col, 210),
                });
            }
        }
        for (i, c) in gc.iter().enumerate() {
            let (col, a) = match class(*c) {
                Some(col) => (col, ghost_a),
                None => (egui::Color32::from_gray(150), (30.0 * mid) as u8),
            };
            if a > 0 {
                self.vers.ghosts.push(FaceColour {
                    placement: ghost,
                    face: i as u32,
                    rgba: rgba(col, a),
                });
            }
        }
        v.dirty = true;
    }

    fn vers_end(&mut self) {
        self.shell.versions.comparing = None;
        self.shell.versions.diff.clear();
        self.shell.versions.counts.clear();
        self.shell.versions.volume = None;
        self.shell.versions.focus = None;
        self.vers.shown = None;
        self.vers.colours.clear();
        self.vers.ghosts.clear();
        self.vers.made_at = None;
        if let (Some(t), Some(v)) = (self.vers.taken.take(), &mut self.view) {
            v.renderer = t.renderer;
            v.scene = t.scene;
            v.renderer.resize(v.size);
            v.dirty = true;
        }
        self.shell.status.clear();
    }

    /// A save made from the app: its history's save too (the file is on disk already).
    /// Agent tools can change tags without changing document text. Refresh the graph, pins,
    /// and revision badge after such a turn as well as after file edits.
    pub fn versions_changed(&mut self) {
        self.vers.rev_of = None;
        forge_lang::pins::forget();
        if self.shell.agent.tab == crate::shell::agent::SideTab::Versions {
            self.vers_load();
        }
    }

    pub fn versions_saved(&mut self, file: &str, agent: Option<&str>) {
        let f = Path::new(file);
        if History::open(f).is_none() {
            return;
        }
        match forge_lang::vtools::save(f, "saved", agent) {
            Ok(_) => {}
            Err(e) => self.shell.status = format!("Saved {} · {e}", name(f)),
        }
        forge_lang::pins::forget();
        if self.shell.agent.tab == crate::shell::agent::SideTab::Versions {
            self.vers_load();
        }
    }
}

/// The pinned `part(..)` lines of a document whose part has a newer named version.
fn pinned_rows(src: &str, doc: &Path) -> Vec<(String, String, u32, u32)> {
    let dir = doc.parent().unwrap_or(Path::new(""));
    let mut out = vec![];
    for line in src.lines() {
        let Some((inst, rest)) = line.split_once('=') else {
            continue;
        };
        let Some(i) = rest.find("part(\"") else { continue };
        let rest = &rest[i + 6..];
        let Some(j) = rest.find('"') else { continue };
        let file = &rest[..j];
        let Some(k) = rest.find("version:") else { continue };
        let digits: String = rest[k + 8..]
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        let Ok(n) = digits.parse::<u32>() else { continue };
        if let Some(m) = forge_lang::pins::newest(dir, file)
            && m > n
        {
            out.push((inst.trim().to_string(), file.to_string(), n, m));
        }
    }
    out
}

/// The document with `inst`'s pin moved from version n to m (its own line only).
fn repin(src: &str, inst: &str, n: u32, m: u32) -> Option<String> {
    let mut out = String::with_capacity(src.len());
    let mut changed = false;
    for line in src.split_inclusive('\n') {
        let is_it = line.split_once('=').is_some_and(|(a, _)| a.trim() == inst);
        if is_it && !changed {
            for (from, to) in [
                (format!("version: {n}"), format!("version: {m}")),
                (format!("version:{n}"), format!("version: {m}")),
            ] {
                if line.contains(&from) {
                    out.push_str(&line.replacen(&from, &to, 1));
                    changed = true;
                    break;
                }
            }
            if changed {
                continue;
            }
        }
        out.push_str(line);
    }
    changed.then_some(out)
}

fn name(p: &Path) -> String {
    p.file_name()
        .map(|x| x.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Versions newest first, each with the saves that led to it from the one before; the saves since the
/// newest version first, as Current.
fn graph(
    saves: &[forge_history::Rec],
    versions: &[forge_history::Version],
    releases: &[forge_lang::release::Record],
    only_versions: bool,
) -> Vec<Node> {
    let row = |r: &forge_history::Rec| SaveRow {
        save: r.id.hex(),
        time: r.time,
        author: r.author.clone(),
        summary: r.summary.clone(),
        agent: r.agent.is_some(),
    };
    let mut out: Vec<Node> = vec![];
    let mut current = Node {
        save: String::new(),
        path: String::new(),
        n: None,
        title: "Current".into(),
        notes: String::new(),
        author: String::new(),
        time: 0,
        agent: false,
        revision: None,
        saves: vec![],
    };
    let mut at: Option<usize> = None;
    let mut seen_saves = HashSet::new();
    for r in saves {
        seen_saves.insert(r.id);
        let marks: Vec<&forge_history::Version> = versions.iter().filter(|v| v.save == r.id).collect();
        if marks.is_empty() {
            match at {
                None => current.saves.push(row(r)),
                Some(k) => out[k].saves.push(row(r)),
            }
            continue;
        }
        for v in marks.iter().rev() {
            let revision = releases
                .iter()
                .find(|x| x.path == v.path && x.version == v.n)
                .map(|x| x.revision.clone());
            out.push(Node {
                save: v.save.hex(),
                path: v.path.clone(),
                n: Some(v.n),
                title: v.name.clone(),
                notes: v.notes.clone(),
                author: v.author.clone(),
                time: v.time,
                agent: v.agent.is_some(),
                revision,
                saves: vec![],
            });
        }
        at = Some(out.len() - 1);
        // the version's own save is listed under it too
        out[at.expect("one")].saves.push(row(r));
    }
    // versions of saves no longer in the list (the history moved): listed at the end
    for v in versions.iter().filter(|v| !seen_saves.contains(&v.save)) {
        out.push(Node {
            save: v.save.hex(),
            path: v.path.clone(),
            n: Some(v.n),
            title: v.name.clone(),
            notes: v.notes.clone(),
            author: v.author.clone(),
            time: v.time,
            agent: v.agent.is_some(),
            revision: None,
            saves: vec![],
        });
    }
    if only_versions {
        for n in &mut out {
            n.saves.clear();
        }
        return out;
    }
    if !current.saves.is_empty() {
        current.time = current.saves[0].time;
        current.author = current.saves[0].author.clone();
        out.insert(0, current);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_git::{HashKind, Oid};

    fn id(n: u8) -> Oid {
        Oid::from_bytes(HashKind::Sha1, &[n; 20]).expect("20")
    }

    fn rec(n: u8, t: i64) -> forge_history::Rec {
        forge_history::Rec {
            id: id(n),
            parents: vec![],
            time: t,
            author: "D".into(),
            agent: None,
            summary: format!("s{n}"),
            paths: vec!["a.fg".into()],
        }
    }

    fn ver(n: u32, save: u8) -> forge_history::Version {
        forge_history::Version {
            path: "a.fg".into(),
            n,
            name: format!("v{n}"),
            notes: String::new(),
            save: id(save),
            time: save as i64,
            author: "D".into(),
            agent: None,
        }
    }

    #[test]
    fn saves_fold_under_the_version_they_led_to() {
        // saves 5 4 3 2 1, newest first; v1 marks 2, v2 marks 4
        let saves: Vec<_> = (1..=5).rev().map(|n| rec(n, n as i64)).collect();
        let g = graph(&saves, &[ver(1, 2), ver(2, 4)], &[], false);
        let titles: Vec<&str> = g.iter().map(|n| n.title.as_str()).collect();
        assert_eq!(titles, ["Current", "v2", "v1"]);
        let count = |k: usize| g[k].saves.iter().map(|s| s.summary.clone()).collect::<Vec<_>>();
        assert_eq!(count(0), ["s5"]);
        assert_eq!(count(1), ["s4", "s3"]);
        assert_eq!(count(2), ["s2", "s1"]);
        assert!(
            graph(&saves, &[ver(1, 2)], &[], true)
                .iter()
                .all(|n| n.saves.is_empty())
        );
    }

    #[test]
    fn a_pin_moves_on_its_own_line_only() {
        let src = "assembly\nold = part(\"b.fg\", version: 3)\nnew = part(\"b.fg\", version: 3)\n";
        let out = repin(src, "old", 3, 5).expect("changed");
        assert_eq!(
            out,
            "assembly\nold = part(\"b.fg\", version: 5)\nnew = part(\"b.fg\", version: 3)\n"
        );
        assert!(repin(src, "nope", 3, 5).is_none());
    }
}
