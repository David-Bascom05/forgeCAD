//! The Machine Builder workspace (research/cam/machine-builder.md §4): the machines the search
//! order sees, a machine's definition edited field by field, its joints jogged through travel
//! with self-collision shown live, the check, and saving to the user's or the project's folder.
//!
//! The panels edit a `MachineDef` here; the host (gui_machine.rs) shows its bodies in the view,
//! rebuilding the meshes when `geom` moves and placing them when `pose` moves.
//!
//! Provenance: a value copied from a template or another machine is cited unknown ("copied");
//! editing it drops its cites, making it the user's own. "?" marks a value unknown (a cite with
//! the number kept as a stand-in until the format can hold UNKNOWN: request F1).
use super::Shell;
use super::theme::{Theme, mono, sans, semi};
use egui::{Color32, RichText, Ui, pos2, vec2};
use forge_machine::collide::{self, Verdict};
use forge_machine::def::{
    Body, Changer, Cite, CiteStatus, JointType, Kind, MachineDef, MountKind, Prim, Reset, Side,
};
use forge_machine_builder::catalog::{self, Catalog, Level};
use forge_machine_builder::check::{self, CheckReport, EnvelopeCheck, Travel};
use forge_machine_builder::make::{self, Copied, Template};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Machine,
    Joints,
    Spindles,
    Changer,
    Mounts,
    Bodies,
    Check,
}

impl Tab {
    pub const ALL: [Tab; 7] = [
        Tab::Machine,
        Tab::Joints,
        Tab::Spindles,
        Tab::Changer,
        Tab::Mounts,
        Tab::Bodies,
        Tab::Check,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Tab::Machine => "Machine",
            Tab::Joints => "Joints",
            Tab::Spindles => "Spindles",
            Tab::Changer => "Changer",
            Tab::Mounts => "Mounts",
            Tab::Bodies => "Bodies",
            Tab::Check => "Check",
        }
    }
}

/// The basis of a value the user marks unknown.
pub const USER_UNKNOWN: &str = "not known: the number is a stand-in entered in the machine builder";

pub struct MachineUi {
    /// The project's folder (the open document's), if any: `cam/machines` under it.
    pub project: Option<PathBuf>,
    pub user: Option<PathBuf>,
    pub cat: Catalog,
    cat_gen: u64,
    /// The machine being edited.
    pub def: Option<MachineDef>,
    /// Where it was loaded from (None: made here, not saved yet).
    pub origin: Option<(Level, String)>,
    /// Part files a made machine brings: written with it.
    pub parts: Vec<(String, String)>,
    /// Unsaved changes.
    pub changed: bool,
    pub tab: Tab,
    /// Joint values (def.joints order).
    pub q: Vec<f64>,
    /// Moves when the bodies or the tree change: the host rebuilds the meshes.
    pub geom: u64,
    /// Moves when q or the live verdicts change: the host places and tints the bodies.
    pub pose: u64,
    /// Moves when another machine is loaded: the host fits the view to it.
    pub fit: u64,
    /// Pairs not clear at q: (a, b, verdict).
    pub live: Vec<(String, String, bool)>,
    /// The definition's refusal (validate, validate_cites), if any.
    pub problem: Option<String>,
    pub report: Option<CheckReport>,
    pub new_name: String,
    pub template: Template,
    pub level: Level,
    pub copied: Copied,
    /// What the last action said.
    pub message: String,
    /// The joint / body / spindle / mount chosen in its tab.
    pub pick: usize,
    /// YYYY-MM-DD, for `taken`.
    pub today: String,
    /// The Bodies tab's file to attach (a path).
    pub file_in: String,
    /// The library list's search.
    pub filter: String,
    /// Hosted by the Machine Builder environment: the library is drawn in its browser
    /// (`Shell::machine_side`), not in a floating panel.
    pub in_env: bool,
    /// Which library the workspace edits: "machine", or a tooling kind ("tool", "holder", ...).
    pub lib: &'static str,
    /// The tooling tab's state, while a tooling kind is chosen.
    pub tooling: Option<super::machine_tooling::ToolingUi>,
    /// The workholding tab's state, while it is chosen.
    pub workholding: Option<super::machine_workholding::WorkholdingUi>,
    /// The tooling record last shown (its text): the host is told when it changes.
    tooling_seen: String,
}

fn today() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        forge_machine_builder::cli::today_utc()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let ms = js_sys::Date::now();
        forge_machine_builder::cli::date_of_days((ms / 86_400_000.0).floor() as i64)
    }
}

impl MachineUi {
    pub fn new(project: Option<PathBuf>) -> MachineUi {
        let user = catalog::user_dir();
        let cat = Catalog::load(project.as_deref(), user.as_deref());
        let mut m = MachineUi {
            level: if project.is_some() {
                Level::Project
            } else {
                Level::User
            },
            project,
            user,
            cat,
            cat_gen: catalog::generation(),
            def: None,
            origin: None,
            parts: Vec::new(),
            changed: false,
            tab: Tab::Joints,
            q: Vec::new(),
            geom: 1,
            pose: 1,
            fit: 1,
            live: Vec::new(),
            problem: None,
            report: None,
            new_name: "my-machine".into(),
            template: Template::Vmc3,
            copied: Copied::Mark,
            message: String::new(),
            pick: 0,
            today: today(),
            file_in: String::new(),
            filter: String::new(),
            in_env: false,
            lib: "machine",
            tooling: None,
            workholding: None,
            tooling_seen: String::new(),
        };
        if let Some(e) = m.cat.entries.first() {
            let (d, l, f) = (e.def.clone(), e.level, e.file.clone());
            m.load(d, Some((l, f)), Vec::new());
        }
        m
    }

    pub fn reload_catalog(&mut self) {
        self.cat = Catalog::load(self.project.as_deref(), self.user.as_deref());
        self.cat_gen = catalog::generation();
    }

    /// Puts a machine in the editor, at home.
    pub fn load(&mut self, def: MachineDef, origin: Option<(Level, String)>, parts: Vec<(String, String)>) {
        self.q = def.joints.iter().map(|j| j.home).collect();
        self.def = Some(def);
        self.origin = origin;
        self.parts = parts;
        self.changed = false;
        self.report = None;
        self.pick = 0;
        self.fit += 1;
        self.edited(true);
    }

    /// After any edit: the refusal, the live verdicts, and what the host must redo.
    pub fn edited(&mut self, geometry: bool) {
        let Some(d) = &mut self.def else { return };
        canon(d);
        self.problem = d
            .validate()
            .and_then(|_| d.validate_cites())
            .err()
            .map(|e| e.to_string());
        if self.q.len() != d.joints.len() {
            self.q = d.joints.iter().map(|j| j.home).collect();
        }
        for (q, j) in self.q.iter_mut().zip(&d.joints) {
            *q = q.clamp(j.range[0].min(j.range[1]), j.range[1].max(j.range[0]));
        }
        if geometry {
            self.geom += 1;
        }
        self.jogged();
    }

    /// The live verdicts at q (only when the definition is valid: the kinematics need its tree).
    pub fn jogged(&mut self) {
        self.live.clear();
        if let (Some(d), None) = (&self.def, &self.problem) {
            for (a, b, v) in collide::check_pose(d, &self.q) {
                self.live.push((a, b, matches!(v, Verdict::Hit { .. })));
            }
        }
        self.pose += 1;
    }

    /// A new machine from the chosen template.
    pub fn make_template(&mut self) {
        match make::from_template(self.template, &self.new_name, &self.today) {
            Ok(m) => {
                self.message = format!(
                    "New `{}` from the {} template: every value copied, cited unknown",
                    m.def.name,
                    self.template.title()
                );
                self.load(m.def, None, m.parts);
                self.changed = true;
            }
            Err(e) => self.message = e,
        }
    }

    /// A clone of the machine in the editor under the new name.
    pub fn make_clone(&mut self) {
        let Some(src) = self.def.clone() else { return };
        let (from, builtin, dir) = match &self.origin {
            Some((l, f)) => (
                format!("{} ({}, {f})", src.name, l.name()),
                *l == Level::Builtin,
                std::path::Path::new(f).parent().map(|p| p.to_path_buf()),
            ),
            None => (src.name.clone(), false, None),
        };
        let parts = self.parts.clone();
        let got = make::clone_machine(&src, &from, &self.new_name, self.copied, &self.today, &|p| {
            if builtin {
                make::builtin_part(p).map(str::to_string)
            } else if let Some((_, t)) = parts.iter().find(|(rel, _)| rel == p) {
                Some(t.clone())
            } else {
                dir.as_ref()
                    .and_then(|d| forge_dirs::fs::read_to_string(d.join(p)).ok())
            }
        });
        match got {
            Ok(m) => {
                self.message = format!("New `{}`, a clone of `{}`", m.def.name, src.name);
                self.load(m.def, None, m.parts);
                self.changed = true;
            }
            Err(e) => self.message = e,
        }
    }

    pub fn level_dir(&self) -> Option<PathBuf> {
        match self.level {
            Level::Project => self.project.as_ref().map(|p| catalog::project_dir(p)),
            Level::User => self.user.clone(),
            Level::Builtin => None,
        }
    }

    /// Saves to the chosen level: a machine loaded from there is overwritten in place.
    pub fn save(&mut self) {
        let Some(d) = &self.def else { return };
        if let Some(p) = &self.problem {
            self.message = format!("Not saved: {p}");
            return;
        }
        let Some(dir) = self.level_dir() else {
            self.message = "Not saved: no folder for that level (open a part for its project)".into();
            return;
        };
        // a machine loaded from this level's own file: its parts are beside it already
        let same = matches!(&self.origin, Some((l, f)) if *l == self.level
            && std::path::Path::new(f) == make::file_for(&dir, &d.name));
        let mut parts = self.parts.clone();
        if !same && let Some((Level::User | Level::Project, f)) = &self.origin {
            // a user or project machine saved to another level: bring its parts
            let src = std::path::Path::new(f)
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_default();
            for b in &d.bodies {
                if let Some(p) = &b.part
                    && !parts.iter().any(|(r, _)| r == p)
                    && let Ok(t) = forge_dirs::fs::read_to_string(src.join(p))
                {
                    parts.push((p.clone(), t));
                }
            }
        }
        let made = make::Made {
            def: d.clone(),
            parts,
        };
        match make::save(
            &made,
            &dir,
            self.level,
            same || self.origin.is_none() && self.changed,
        ) {
            Ok(f) => {
                self.message = format!("Saved {} ({} level)", f.display(), self.level.name());
                self.origin = Some((self.level, f.display().to_string()));
                self.parts.clear();
                self.changed = false;
                self.reload_catalog();
            }
            Err(e) => self.message = format!("Not saved: {e}"),
        }
    }

    /// The check (structure, home, travel, provenance; parts in envelopes where they build).
    pub fn run_check(&mut self) {
        let Some(d) = &self.def else { return };
        let (level, file) = match &self.origin {
            Some((l, f)) => (Some(*l), f.clone()),
            None => (Some(self.level), "(not saved)".to_string()),
        };
        let mut r = check::check(d, level, &file);
        if let Some((Level::User | Level::Project, f)) = &self.origin {
            let dir = std::path::Path::new(f)
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_default();
            for (b, e) in forge_machine_lang::envelope_checks(d, &dir) {
                r.envelopes.retain(|(x, _)| *x != b);
                r.envelopes.push((b, e));
            }
        }
        self.report = Some(r);
        self.tab = Tab::Check;
    }
}

/// Attaches a file (.fg, .step, .stl) to body `b` of a machine saved in `dir` and fits the
/// envelope to its part (the Bodies tab's Attach).
pub fn attach_and_fit(
    d: &mut MachineDef,
    b: usize,
    dir: &std::path::Path,
    file: &std::path::Path,
) -> Result<String, String> {
    let m = super::machine_parts::attach(d, b, dir, file)?;
    // the envelope is the user's now
    let path = format!("body.{}", d.bodies[b].name);
    d.cites.retain(|c| c.path != path);
    if d.bodies[b].part.is_some() {
        let m2 = super::machine_parts::fit(d, b, dir)?;
        Ok(format!("{m}; {m2}"))
    } else {
        Ok(m)
    }
}

/// Unit axes and directions (the reader normalises them; the writer must write what it reads).
fn canon(d: &mut MachineDef) {
    let unit = |a: &mut [f64; 3]| {
        let n = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
        if n > 0.0 && n.is_finite() && (n - 1.0).abs() > 0.0 {
            for x in a.iter_mut() {
                *x /= n;
            }
        }
    };
    for j in &mut d.joints {
        unit(&mut j.axis);
    }
    for s in &mut d.spindles {
        unit(&mut s.axis);
    }
    for m in &mut d.mounts {
        unit(&mut m.dir);
        unit(&mut m.xdir);
    }
    for b in &mut d.bodies {
        for p in &mut b.prims {
            if let Prim::Cyl { axis, .. } = p {
                unit(axis);
            }
        }
    }
}

/// Where each value stands, by cite path: (status word, colour).
fn badge(t: &Theme, d: &MachineDef, path: &str, own: bool) -> (String, Color32) {
    let cs: Vec<&Cite> = d.cites.iter().filter(|c| c.path == path).collect();
    match cs.first() {
        None => {
            if own {
                ("user".into(), t.success)
            } else {
                ("uncited".into(), t.ink2)
            }
        }
        Some(c) => match c.status {
            CiteStatus::Unknown if c.basis.starts_with("copied from") => ("copied".into(), t.warn),
            CiteStatus::Unknown => ("UNKNOWN".into(), t.error),
            CiteStatus::Published => ("published".into(), t.accent),
            CiteStatus::Estimate => ("estimate".into(), t.warn),
            CiteStatus::Conflict => ("conflict".into(), t.error),
        },
    }
}

/// What a frame's widgets asked to change in the definition's cites.
#[derive(Default)]
struct Touch {
    /// Values the user changed: their cites go (the value is the user's now).
    edited: Vec<String>,
    /// Values the user marked unknown.
    unknown: Vec<String>,
    geometry: bool,
    any: bool,
    /// Bodies tab: attach this file to the chosen body / fit its envelope to its part / drop it.
    attach: Option<String>,
    fit: bool,
    detach: bool,
}

struct Ctx<'a> {
    t: &'a Theme,
    cites: &'a MachineDef,
    own: bool,
}

fn small(t: &Theme, s: impl Into<String>) -> RichText {
    RichText::new(s.into()).font(sans(11.0)).color(t.ink2)
}

fn label_w(ui: &mut Ui, t: &Theme, s: &str) {
    ui.add_sized(
        vec2(84.0, 18.0),
        egui::Label::new(RichText::new(s).font(sans(12.0)).color(t.ink2)),
    );
}

/// The provenance chip and the "?" (mark unknown) of a value.
fn chip(ui: &mut Ui, c: &Ctx, path: &str, touch: &mut Touch) {
    let (w, col) = badge(c.t, c.cites, path, c.own);
    ui.label(RichText::new(w).font(sans(10.0)).color(col))
        .on_hover_text(path);
    if ui
        .small_button("?")
        .on_hover_text("I don't know this value: mark it UNKNOWN")
        .clicked()
    {
        touch.unknown.push(path.to_string());
        touch.any = true;
    }
}

fn num_row(ui: &mut Ui, c: &Ctx, name: &str, path: &str, v: &mut f64, speed: f64, touch: &mut Touch) {
    ui.horizontal(|ui| {
        label_w(ui, c.t, name);
        if ui
            .add(egui::DragValue::new(v).speed(speed).max_decimals(4))
            .changed()
        {
            touch.edited.push(path.to_string());
            touch.any = true;
        }
        chip(ui, c, path, touch);
    });
}

fn vec_row<const N: usize>(
    ui: &mut Ui,
    c: &Ctx,
    name: &str,
    path: &str,
    v: &mut [f64; N],
    speed: f64,
    touch: &mut Touch,
) {
    ui.horizontal(|ui| {
        label_w(ui, c.t, name);
        let mut ch = false;
        for x in v.iter_mut() {
            ch |= ui
                .add(egui::DragValue::new(x).speed(speed).max_decimals(4))
                .changed();
        }
        if ch {
            touch.edited.push(path.to_string());
            touch.any = true;
        }
        if !path.is_empty() {
            chip(ui, c, path, touch);
        }
    });
}

fn text_row(ui: &mut Ui, t: &Theme, name: &str, v: &mut String, touch: &mut Touch) {
    ui.horizontal(|ui| {
        label_w(ui, t, name);
        if ui
            .add(egui::TextEdit::singleline(v).desired_width(180.0))
            .changed()
        {
            touch.any = true;
        }
    });
}

fn pick_row<T: Copy + PartialEq>(
    ui: &mut Ui,
    t: &Theme,
    name: &str,
    id: &str,
    v: &mut T,
    all: &[(&str, T)],
    touch: &mut Touch,
) {
    ui.horizontal(|ui| {
        label_w(ui, t, name);
        let cur = all.iter().find(|(_, x)| x == v).map(|(n, _)| *n).unwrap_or("?");
        egui::ComboBox::from_id_salt(id)
            .selected_text(cur)
            .show_ui(ui, |ui| {
                for (n, x) in all {
                    if ui.selectable_label(x == v, *n).clicked() && x != v {
                        *v = *x;
                        touch.any = true;
                        touch.geometry = true;
                    }
                }
            });
    });
}

fn link_row(
    ui: &mut Ui,
    t: &Theme,
    name: &str,
    id: &str,
    v: &mut String,
    links: &[String],
    touch: &mut Touch,
) {
    ui.horizontal(|ui| {
        label_w(ui, t, name);
        egui::ComboBox::from_id_salt(id)
            .selected_text(v.as_str())
            .show_ui(ui, |ui| {
                for l in links {
                    if ui.selectable_label(l == v, l).clicked() && l != v {
                        *v = l.clone();
                        touch.any = true;
                        touch.geometry = true;
                    }
                }
            });
    });
}

/// The users of a link (by name), for refusing to delete a joint something rides on.
fn link_users(d: &MachineDef, name: &str) -> Vec<String> {
    let mut u = Vec::new();
    for j in &d.joints {
        if j.parent == name {
            u.push(format!("joint {}", j.name));
        }
    }
    for s in &d.spindles {
        if s.link == name {
            u.push(format!("spindle {}", s.name));
        }
    }
    for s in &d.turrets {
        if s.link == name {
            u.push(format!("turret {}", s.name));
        }
    }
    for m in &d.mounts {
        if m.link == name {
            u.push(format!("mount {}", m.name));
        }
    }
    for b in &d.bodies {
        if b.link == name {
            u.push(format!("body {}", b.name));
        }
    }
    if d.length_joint.as_deref() == Some(name) {
        u.push("length_joint".into());
    }
    u
}

/// A joint renamed: every reference to it follows.
fn rename_joint(d: &mut MachineDef, from: &str, to: &str) {
    for j in &mut d.joints {
        if j.parent == from {
            j.parent = to.into();
        }
        if j.name == from {
            if j.letter == from {
                j.letter = to.into();
            }
            j.name = to.into();
        }
    }
    for s in &mut d.spindles {
        if s.link == from {
            s.link = to.into();
        }
        if s.c_joint.as_deref() == Some(from) {
            s.c_joint = Some(to.into());
        }
    }
    for s in &mut d.turrets {
        if s.link == from {
            s.link = to.into();
        }
    }
    for m in &mut d.mounts {
        if m.link == from {
            m.link = to.into();
        }
    }
    for b in &mut d.bodies {
        if b.link == from {
            b.link = to.into();
        }
    }
    if d.length_joint.as_deref() == Some(from) {
        d.length_joint = Some(to.into());
    }
    let pre = format!("joint.{from}.");
    for c in &mut d.cites {
        if let Some(rest) = c.path.strip_prefix(&pre) {
            c.path = format!("joint.{to}.{rest}");
        }
    }
}

/// The Machine Builder's commands, as its ribbon groups and the environment send them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MachineCmd {
    /// A new machine from the chosen template, under the name in the panel.
    FromTemplate,
    /// A clone of the machine in the editor, under the name in the panel.
    Clone,
    Save,
    /// The travel check.
    Check,
    /// Every joint at home.
    Home,
    Close,
}

/// The Machine Builder's ribbon groups (for the environment's tabs; research/ui/environments.md §5).
pub(in crate::shell) fn groups(s: &Shell) -> Vec<super::ribbon::Group> {
    use super::ribbon::{Group, Tool};
    let m = s.machine.as_ref();
    let has = m.is_some_and(|m| m.def.is_some());
    let valid = m.is_some_and(|m| m.def.is_some() && m.problem.is_none());
    let tool = |name, icon, cmd, on: bool| Tool {
        name,
        icon,
        action: on.then_some(super::Action::Machine(cmd)),
        on: false,
    };
    vec![
        Group {
            name: "Machine",
            big: vec![
                tool("From Template", "new", MachineCmd::FromTemplate, m.is_some()),
                tool("Clone Machine", "copy", MachineCmd::Clone, has),
                tool("Save Machine", "save", MachineCmd::Save, valid),
            ],
            small: vec![],
            icons: vec![],
        },
        Group {
            name: "Motion",
            big: vec![
                tool("Home Axes", "home", MachineCmd::Home, valid),
                tool("Check Travel", "check", MachineCmd::Check, valid),
            ],
            small: vec![],
            icons: vec![],
        },
        Group {
            name: "Exit",
            big: vec![tool("Close Builder", "close", MachineCmd::Close, m.is_some())],
            small: vec![],
            icons: vec![],
        },
    ]
}

impl Shell {
    /// The environment's browser: the machine library (search, by level and vendor), new and save.
    pub fn machine_side(&mut self, ui: &mut Ui, r: egui::Rect) {
        let Some(mut st) = self.machine.take() else { return };
        let t = self.theme;
        let mut places = Vec::new();
        ui.scope_builder(egui::UiBuilder::new().max_rect(r), |ui| {
            library(ui, &t, &mut st, &mut places);
        });
        self.places.extend(places);
        self.machine = Some(st);
    }

    /// The workspace's panels, while it is open.
    pub fn machine_ui(&mut self, ctx: &egui::Context) {
        let Some(mut st) = self.machine.take() else { return };
        // the tooling record changed: the host draws it again
        let shown = match (&st.tooling, &st.workholding) {
            (Some(tu), _) => Some(format!("{:?}", tu.cur.as_ref().map(|r| &r.typed))),
            (None, Some(w)) => Some(format!("{:?}", w.cur.as_ref().map(|r| (&r.name, &r.vals)))),
            _ => None,
        };
        if let Some(k) = shown
            && k != st.tooling_seen
        {
            // another record (not an edit of this one): the view fits it
            let id = |x: &str| {
                x.split("id: \"")
                    .nth(1)
                    .or_else(|| x.split("((\"").nth(1))
                    .and_then(|r| r.split('"').next())
                    .map(str::to_string)
            };
            if id(&k) != id(&st.tooling_seen) {
                st.fit += 1;
            }
            st.tooling_seen = k;
            st.geom += 1;
            st.pose += 1;
        }
        if st.cat_gen != catalog::generation() {
            st.reload_catalog();
        }
        let t = self.theme;
        let v = self.view_rect;
        let mut close = false;
        let mut places: Vec<(String, egui::Rect)> = Vec::new();
        // the library (in the environment's browser instead when it hosts the builder)
        if !st.in_env {
            egui::Window::new("Machines")
                .id(egui::Id::unique("machine:library"))
                .title_bar(false)
                .resizable(false)
                .collapsible(false)
                .fixed_pos(pos2(v.left() + 8.0, v.top() + 8.0))
                .fixed_size(vec2(250.0, (v.height() - 140.0).max(200.0)))
                .frame(super::theme::panel_frame(&t))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Machine Builder").font(semi(14.0)).color(t.ink));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let r = ui.button("Close");
                            places.push(("machine:close".into(), r.rect));
                            if r.clicked() {
                                close = true;
                            }
                        });
                    });
                    ui.add_space(4.0);
                    lib_choice(ui, &mut st, &mut places);
                    if st.lib == "machine" {
                        library(ui, &t, &mut st, &mut places);
                    } else if st.lib == "workholding" {
                        workholding_library(ui, &t, &mut st, &mut places);
                    } else {
                        tooling_library(ui, &t, &mut st, &mut places);
                    }
                });
        }
        // the editor
        egui::Window::new("Machine editor")
            .id(egui::Id::unique("machine:editor"))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .fixed_pos(pos2(v.right() - 438.0, v.top() + 8.0))
            .fixed_size(vec2(430.0, (v.height() - 140.0).max(200.0)))
            .frame(super::theme::panel_frame(&t))
            .show(ctx, |ui| {
                if st.lib == "machine" {
                    editor(ui, &t, &mut st, &mut places)
                } else if let Some(tu) = &mut st.tooling {
                    super::machine_tooling::editor(ui, &t, tu)
                } else if let Some(w) = &mut st.workholding {
                    super::machine_workholding::editor(ui, &t, w)
                }
            });
        // the jog bar
        egui::Window::new("Jog")
            .id(egui::Id::unique("machine:jog"))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .fixed_pos(pos2(v.left() + 8.0, v.bottom() - 124.0))
            .fixed_size(vec2((v.width() - 16.0).max(200.0), 108.0))
            .frame(super::theme::panel_frame(&t))
            .show(ctx, |ui| {
                if st.lib == "machine" {
                    jog(ui, &t, &mut st, &mut places)
                } else if st.lib == "workholding" {
                    workholding_bar(ui, &t, &mut st, &mut places)
                } else {
                    tooling_bar(ui, &t, &mut st, &mut places)
                }
            });
        self.places.extend(places);
        if close {
            self.status = "Machine Builder closed".into();
        } else {
            self.machine = Some(st);
        }
    }
}

/// The library the workspace edits: machines, or one of forge-tooling's kinds.
fn lib_choice(ui: &mut Ui, st: &mut MachineUi, places: &mut Vec<(String, egui::Rect)>) {
    let title = |w: &str| {
        if w == "machine" {
            "Machines"
        } else if w == "workholding" {
            "Workholding"
        } else {
            forge_tooling_builder::KINDS
                .iter()
                .find(|(k, _, _)| *k == w)
                .map_or("?", |(_, _, t)| *t)
        }
    };
    let mut want = st.lib;
    let r = egui::ComboBox::from_id_salt("machine:lib")
        .selected_text(title(st.lib))
        .width(ui.available_width() - 8.0)
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut want, "machine", "Machines");
            for (w, _, t) in forge_tooling_builder::KINDS {
                ui.selectable_value(&mut want, *w, *t);
            }
            ui.selectable_value(&mut want, "workholding", "Workholding");
        });
    places.push(("machine:lib".into(), r.response.rect));
    if want != st.lib {
        st.set_lib(want);
    }
}

impl MachineUi {
    /// Switches the library edited: "machine" or a tooling kind.
    pub fn set_lib(&mut self, word: &'static str) {
        self.lib = word;
        self.tooling = (word != "machine" && word != "workholding")
            .then(|| super::machine_tooling::ToolingUi::new(word, self.project.as_deref()));
        self.workholding = (word == "workholding")
            .then(|| super::machine_workholding::WorkholdingUi::new(self.project.as_deref()));
        self.tooling_seen.clear();
        self.geom += 1;
        self.pose += 1;
        self.fit += 1;
    }
}

fn tooling_library(ui: &mut Ui, t: &Theme, st: &mut MachineUi, places: &mut Vec<(String, egui::Rect)>) {
    let project = st.project.clone();
    let Some(tu) = &mut st.tooling else { return };
    super::machine_tooling::library(ui, t, tu, project.as_deref(), places);
    ui.separator();
    ui.horizontal(|ui| {
        label_w(ui, t, "Save to");
        ui.selectable_value(&mut st.level, Level::User, "User");
        ui.add_enabled_ui(st.project.is_some(), |ui| {
            ui.selectable_value(&mut st.level, Level::Project, "Project");
        });
    });
    let Some(tu) = &mut st.tooling else { return };
    let r = ui.add_enabled(
        tu.cur.is_some(),
        egui::Button::new(if tu.changed { "Save *" } else { "Save" }),
    );
    places.push(("tooling:save".into(), r.rect));
    if r.clicked() {
        tu.save(st.level, project.as_deref());
    }
    if !tu.message.is_empty() {
        ui.label(RichText::new(&tu.message).font(sans(11.0)).color(t.ink2));
    }
}

impl MachineUi {
    /// The scene for a library other than machines: the chosen record's bodies (None: machines;
    /// Some(empty): nothing to draw).
    pub fn preview_bodies(&self) -> Option<Vec<crate::machine_mesh::Tris>> {
        match self.lib {
            "machine" => None,
            "workholding" => Some(
                self.workholding
                    .as_ref()
                    .and_then(|w| w.preview())
                    .unwrap_or_default(),
            ),
            _ => Some(
                self.tooling
                    .as_ref()
                    .and_then(|t| t.preview())
                    .into_iter()
                    .collect(),
            ),
        }
    }
}

fn workholding_library(ui: &mut Ui, t: &Theme, st: &mut MachineUi, places: &mut Vec<(String, egui::Rect)>) {
    let project = st.project.clone();
    let Some(w) = &mut st.workholding else { return };
    super::machine_workholding::library(ui, t, w, project.as_deref(), places);
    ui.separator();
    ui.horizontal(|ui| {
        label_w(ui, t, "Save to");
        ui.selectable_value(&mut st.level, Level::User, "User");
        ui.add_enabled_ui(st.project.is_some(), |ui| {
            ui.selectable_value(&mut st.level, Level::Project, "Project");
        });
    });
    let Some(w) = &mut st.workholding else { return };
    let r = ui.add_enabled(
        w.cur.is_some(),
        egui::Button::new(if w.changed { "Save *" } else { "Save" }),
    );
    places.push(("workholding:save".into(), r.rect));
    if r.clicked() {
        w.save(st.level, project.as_deref());
    }
    if !w.message.is_empty() {
        ui.label(RichText::new(&w.message).font(sans(11.0)).color(t.ink2));
    }
}

fn workholding_bar(ui: &mut Ui, t: &Theme, st: &mut MachineUi, places: &mut Vec<(String, egui::Rect)>) {
    let Some(w) = &mut st.workholding else { return };
    ui.horizontal(|ui| {
        ui.label(RichText::new("Checks").font(semi(12.0)).color(t.ink));
        let r = ui.add_enabled(w.cur.is_some(), egui::Button::new("Check"));
        places.push(("workholding:check".into(), r.rect));
        if r.clicked() {
            w.check();
        }
        ui.label(RichText::new(&w.message).font(sans(11.0)).color(t.ink2));
    });
    ui.label(small(
        t,
        "The preview places the record as its check does (a mill device on a reference stock, a lathe device about +z from the chuck face).",
    ));
}

fn tooling_bar(ui: &mut Ui, t: &Theme, st: &mut MachineUi, places: &mut Vec<(String, egui::Rect)>) {
    let project = st.project.clone();
    let Some(tu) = &mut st.tooling else { return };
    ui.horizontal(|ui| {
        ui.label(RichText::new("Checks").font(semi(12.0)).color(t.ink));
        let r = ui.add_enabled(
            tu.cur.is_some() && tu.problem.is_none(),
            egui::Button::new("Check"),
        );
        places.push(("tooling:check".into(), r.rect));
        if r.clicked() {
            tu.check(project.as_deref());
        }
        let (txt, col) = match (&tu.problem, tu.findings.is_empty()) {
            (Some(p), _) => (format!("refused: {p}"), t.error),
            (None, false) => (
                format!("{} problems (see the editor)", tu.findings.len()),
                t.error,
            ),
            (None, true) => (tu.message.clone(), t.ink2),
        };
        ui.label(RichText::new(txt).font(sans(11.0)).color(col));
    });
    ui.label(small(
        t,
        "The preview is the record's turned outline about its axis (z from the tip or nose); inserts and turning holders have none yet.",
    ));
}

/// An entry's group in the list: its vendor (`builtin/<vendor>.toml`) or its file's name.
fn group_of(e: &catalog::Entry) -> String {
    std::path::Path::new(&e.file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("?")
        .to_string()
}

fn library(ui: &mut Ui, t: &Theme, st: &mut MachineUi, places: &mut Vec<(String, egui::Rect)>) {
    ui.horizontal(|ui| {
        let r = ui.add(
            egui::TextEdit::singleline(&mut st.filter)
                .hint_text("search: name, kind, control, vendor")
                .desired_width(ui.available_width()),
        );
        places.push(("machine:search".into(), r.rect));
    });
    let mut chosen = None;
    egui::ScrollArea::vertical()
        .id_salt("machine:list")
        .max_height((ui.available_height() - 230.0).max(80.0))
        .show(ui, |ui| {
            let want = st.filter.trim().to_lowercase();
            let matches = |e: &catalog::Entry| {
                want.is_empty()
                    || [
                        e.def.name.as_str(),
                        e.def.kind.name(),
                        e.def.control.as_str(),
                        group_of(e).as_str(),
                    ]
                    .iter()
                    .any(|x| x.to_lowercase().contains(&want))
            };
            for lv in [Level::Project, Level::User, Level::Builtin] {
                let here: Vec<_> = st
                    .cat
                    .entries
                    .iter()
                    .filter(|e| e.level == lv && matches(e))
                    .collect();
                ui.label(
                    RichText::new(lv.name().to_uppercase())
                        .font(semi(10.0))
                        .color(t.ink2),
                );
                if here.is_empty() {
                    ui.label(small(
                        t,
                        match lv {
                            Level::Project if st.project.is_none() => "  (no part open: no project folder)",
                            _ if !want.is_empty() => "  none match",
                            _ => "  none",
                        },
                    ));
                }
                // by vendor (a built-in file) or by file (the user's, the project's)
                let mut groups: Vec<String> = here.iter().map(|e| group_of(e)).collect();
                groups.dedup();
                for g in groups {
                    let inner: Vec<_> = here.iter().filter(|e| group_of(e) == g).collect();
                    let picked = inner.iter().any(|e| {
                        st.def.as_ref().is_some_and(|d| d.name == e.def.name)
                            && st.origin.as_ref().is_some_and(|(l, _)| *l == e.level)
                    });
                    egui::CollapsingHeader::new(
                        RichText::new(format!("{g} ({})", inner.len())).font(sans(11.0)),
                    )
                    .id_salt(("machine:group", lv.name(), g.as_str()))
                    .default_open(picked || !want.is_empty() || inner.len() <= 6)
                    .show(ui, |ui| {
                        for e in inner {
                            let on = st.def.as_ref().is_some_and(|d| d.name == e.def.name)
                                && st.origin.as_ref().is_some_and(|(l, _)| *l == e.level);
                            let mut txt = format!("{}  ·  {}", e.def.name, e.def.kind.name());
                            if e.def.real {
                                txt.push_str("  · real");
                            }
                            if !e.hides.is_empty() {
                                txt.push_str("  (hides ");
                                txt.push_str(
                                    &e.hides
                                        .iter()
                                        .map(|(l, _)| l.name())
                                        .collect::<Vec<_>>()
                                        .join(", "),
                                );
                                txt.push(')');
                            }
                            let r = ui.selectable_label(on, RichText::new(txt).font(sans(12.0)));
                            places.push((format!("machine:pick:{}", e.def.name), r.rect));
                            if r.clicked() {
                                chosen = Some((e.def.clone(), e.level, e.file.clone()));
                            }
                        }
                    });
                }
            }
            for (l, f, e) in &st.cat.errors {
                ui.label(
                    RichText::new(format!("not loaded ({}): {f}: {e}", l.name()))
                        .font(sans(10.0))
                        .color(t.error),
                );
            }
        });
    if let Some((d, l, f)) = chosen {
        if st.changed {
            st.message = format!(
                "Unsaved changes to `{}` dropped",
                st.def.as_ref().map_or("", |d| d.name.as_str())
            );
        }
        st.load(d, Some((l, f)), Vec::new());
    }
    ui.separator();
    ui.label(RichText::new("New machine").font(semi(12.0)).color(t.ink));
    ui.horizontal(|ui| {
        label_w(ui, t, "Name");
        let r = ui.add(egui::TextEdit::singleline(&mut st.new_name).desired_width(140.0));
        places.push(("machine:name".into(), r.rect));
    });
    ui.horizontal(|ui| {
        label_w(ui, t, "Template");
        egui::ComboBox::from_id_salt("machine:template")
            .selected_text(st.template.title())
            .show_ui(ui, |ui| {
                for tp in Template::ALL {
                    ui.selectable_value(&mut st.template, tp, tp.title());
                }
            });
    });
    ui.horizontal(|ui| {
        let r = ui.button("From template");
        places.push(("machine:from-template".into(), r.rect));
        if r.clicked() {
            st.make_template();
        }
        let r = ui
            .add_enabled(st.def.is_some(), egui::Button::new("Clone this one"))
            .on_hover_text("a copy of the machine in the editor under the new name");
        places.push(("machine:clone".into(), r.rect));
        if r.clicked() {
            st.make_clone();
        }
    });
    ui.horizontal(|ui| {
        label_w(ui, t, "Copied values");
        egui::ComboBox::from_id_salt("machine:copied")
            .selected_text(match st.copied {
                Copied::Mark => "cite as copied",
                Copied::KeepCites => "keep source cites",
                Copied::Own => "take as mine",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut st.copied, Copied::Mark, "cite as copied");
                ui.selectable_value(&mut st.copied, Copied::KeepCites, "keep source cites");
                ui.selectable_value(&mut st.copied, Copied::Own, "take as mine");
            });
    });
    ui.separator();
    ui.horizontal(|ui| {
        label_w(ui, t, "Save to");
        ui.selectable_value(&mut st.level, Level::User, "User");
        ui.add_enabled_ui(st.project.is_some(), |ui| {
            ui.selectable_value(&mut st.level, Level::Project, "Project");
        });
    });
    ui.horizontal(|ui| {
        let r = ui.add_enabled(
            st.def.is_some(),
            egui::Button::new(if st.changed { "Save *" } else { "Save" }),
        );
        places.push(("machine:save".into(), r.rect));
        if r.clicked() {
            st.save();
        }
    });
    if !st.message.is_empty() {
        ui.label(RichText::new(&st.message).font(sans(11.0)).color(t.ink2));
    }
}

fn editor(ui: &mut Ui, t: &Theme, st: &mut MachineUi, places: &mut Vec<(String, egui::Rect)>) {
    let Some(d) = st.def.clone() else {
        ui.label(small(t, "No machine: pick one, or make one from a template"));
        return;
    };
    // the head: name, where it is from, its refusal
    ui.horizontal(|ui| {
        ui.label(RichText::new(&d.name).font(semi(14.0)).color(t.ink));
        ui.add_space(6.0);
        let from = match &st.origin {
            Some((l, f)) => format!("{} · {f}", l.name()),
            None => "not saved".into(),
        };
        ui.label(small(t, from));
    });
    match &st.problem {
        Some(p) => {
            ui.label(
                RichText::new(format!("refused: {p}"))
                    .font(sans(11.0))
                    .color(t.error),
            );
        }
        None => {
            ui.label(
                RichText::new("valid definition")
                    .font(sans(11.0))
                    .color(t.success),
            );
        }
    }
    ui.horizontal_wrapped(|ui| {
        for tab in Tab::ALL {
            let r = ui.selectable_label(st.tab == tab, RichText::new(tab.name()).font(sans(12.0)));
            places.push((format!("machine:tab:{}", tab.name()), r.rect));
            if r.clicked() {
                st.tab = tab;
                st.pick = 0;
            }
        }
    });
    ui.separator();
    let own = matches!(st.origin, Some((Level::User | Level::Project, _)) | None);
    let mut d2 = d.clone();
    let mut touch = Touch::default();
    let c = Ctx { t, cites: &d, own };
    egui::ScrollArea::vertical()
        .id_salt("machine:edit-scroll")
        .show(ui, |ui| match st.tab {
            Tab::Machine => tab_machine(ui, &c, &mut d2, &mut touch),
            Tab::Joints => tab_joints(ui, &c, &mut d2, &mut st.pick, &mut touch),
            Tab::Spindles => tab_spindles(ui, &c, &mut d2, &mut st.pick, &mut touch),
            Tab::Changer => tab_changer(ui, &c, &mut d2, &mut touch),
            Tab::Mounts => tab_mounts(ui, &c, &mut d2, &mut st.pick, &mut touch),
            Tab::Bodies => tab_bodies(ui, &c, &mut d2, &mut st.pick, &mut st.file_in, &mut touch),
            Tab::Check => tab_check(ui, t, st.report.as_ref(), &d, &st.q),
        });
    // a body's geometry from a file: beside the saved machine
    if touch.attach.is_some() || touch.fit || touch.detach {
        let dir = match &st.origin {
            Some((Level::User | Level::Project, f)) => {
                std::path::Path::new(f).parent().map(|p| p.to_path_buf())
            }
            _ => None,
        };
        let b = st.pick;
        let got = if b >= d2.bodies.len() {
            Err("no body chosen".to_string())
        } else if touch.detach {
            d2.bodies[b].part = None;
            Ok(format!(
                "`{}`: no part (its envelope is the body)",
                d2.bodies[b].name
            ))
        } else if let Some(dir) = dir {
            match &touch.attach {
                Some(f) => super::machine_parts::attach(&mut d2, b, &dir, std::path::Path::new(f.trim()))
                    .and_then(|m| {
                        if d2.bodies[b].part.is_some() {
                            super::machine_parts::fit(&mut d2, b, &dir).map(|m2| format!("{m}; {m2}"))
                        } else {
                            Ok(m)
                        }
                    }),
                None => super::machine_parts::fit(&mut d2, b, &dir),
            }
        } else {
            Err("save the machine to the user or project level first: its part files go beside it".into())
        };
        match got {
            Ok(m) => {
                st.message = m;
                touch.edited.push(format!("body.{}", d2.bodies[b].name));
                touch.any = true;
                touch.geometry = true;
            }
            Err(e) => st.message = e,
        }
    }
    if touch.any {
        for p in &touch.edited {
            d2.cites.retain(|c| &c.path != p);
        }
        for p in &touch.unknown {
            d2.cites.retain(|c| &c.path != p);
            d2.cites.push(Cite {
                path: p.clone(),
                status: CiteStatus::Unknown,
                source: String::new(),
                taken: String::new(),
                value: String::new(),
                basis: USER_UNKNOWN.into(),
                note: String::new(),
            });
        }
        st.def = Some(d2);
        st.changed = true;
        st.report = None;
        st.edited(touch.geometry || !touch.edited.is_empty());
    }
}

const KINDS: [(&str, Kind); 7] = Kind::ALL;

fn tab_machine(ui: &mut Ui, c: &Ctx, d: &mut MachineDef, touch: &mut Touch) {
    let t = c.t;
    text_row(ui, t, "Name", &mut d.name, touch);
    pick_row(ui, t, "Kind", "machine:kind", &mut d.kind, &KINDS, touch);
    ui.horizontal(|ui| {
        label_w(ui, t, "Control");
        if ui
            .add(egui::TextEdit::singleline(&mut d.control).desired_width(120.0))
            .changed()
        {
            touch.any = true;
        }
        egui::ComboBox::from_id_salt("machine:dialect")
            .selected_text("dialects")
            .show_ui(ui, |ui| {
                for x in ["fanuc", "haas", "linuxcnc", "grbl", "fanuc-lathe"] {
                    if ui.selectable_label(d.control == x, x).clicked() {
                        d.control = x.into();
                        touch.any = true;
                    }
                }
            });
    });
    let joints: Vec<String> = d
        .joints
        .iter()
        .filter(|j| j.ty == JointType::Linear)
        .map(|j| j.name.clone())
        .collect();
    ui.horizontal(|ui| {
        label_w(ui, t, "Length joint");
        let cur = d.length_joint.clone().unwrap_or_else(|| "(none: lathe)".into());
        egui::ComboBox::from_id_salt("machine:lj")
            .selected_text(cur)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(d.length_joint.is_none(), "(none: lathe)")
                    .clicked()
                {
                    d.length_joint = None;
                    touch.any = true;
                }
                for j in &joints {
                    if ui
                        .selectable_label(d.length_joint.as_ref() == Some(j), j)
                        .clicked()
                    {
                        d.length_joint = Some(j.clone());
                        touch.any = true;
                    }
                }
            });
    });
    ui.horizontal(|ui| {
        label_w(ui, t, "Coolants");
        for k in ["flood", "mist", "through", "air"] {
            let mut on = d.coolants.iter().any(|x| x == k);
            if ui.checkbox(&mut on, k).changed() {
                if on {
                    d.coolants.push(k.into());
                } else {
                    d.coolants.retain(|x| x != k);
                }
                touch.any = true;
            }
        }
    });
    ui.horizontal(|ui| {
        label_w(ui, t, "Clearance");
        if ui
            .add(
                egui::DragValue::new(&mut d.clearance)
                    .speed(0.1)
                    .range(0.0..=100.0)
                    .suffix(" mm"),
            )
            .changed()
        {
            touch.any = true;
        }
    });
    text_row(ui, t, "Source", &mut d.source, touch);
    text_row(ui, t, "Taken", &mut d.taken, touch);
    text_row(ui, t, "Note", &mut d.note, touch);
    ui.label(small(
        t,
        "real = false: a machine you made (a real maker's model cites every value)",
    ));
}

fn tab_joints(ui: &mut Ui, c: &Ctx, d: &mut MachineDef, pick: &mut usize, touch: &mut Touch) {
    let t = c.t;
    // the tree: each joint under its parent
    fn rows(ui: &mut Ui, t: &Theme, d: &MachineDef, parent: &str, depth: usize, pick: &mut usize) {
        for (i, j) in d.joints.iter().enumerate() {
            if j.parent == parent {
                let txt = format!(
                    "{}{} {} ({}, {})",
                    "    ".repeat(depth),
                    "└",
                    j.name,
                    if j.ty == JointType::Linear {
                        "linear"
                    } else {
                        "rotary"
                    },
                    if j.side == Side::Head { "head" } else { "table" }
                );
                if ui
                    .selectable_label(*pick == i, RichText::new(txt).font(mono(12.0)).color(t.ink))
                    .clicked()
                {
                    *pick = i;
                }
                if depth < 16 {
                    rows(ui, t, d, &j.name, depth + 1, pick);
                }
            }
        }
    }
    ui.label(RichText::new("base").font(mono(12.0)).color(t.ink2));
    rows(ui, t, d, "base", 0, pick);
    ui.horizontal(|ui| {
        if ui.button("+ child joint").clicked() {
            let parent = d.joints.get(*pick).map_or("base".to_string(), |j| j.name.clone());
            let side = d.joints.get(*pick).map_or(Side::Head, |j| j.side);
            let used: Vec<&str> = d.joints.iter().map(|j| j.letter.as_str()).collect();
            let name = ["U", "V", "W", "X", "Y", "Z"]
                .into_iter()
                .find(|l| !used.contains(l))
                .unwrap_or("W")
                .to_string();
            d.joints.push(forge_machine::def::Joint {
                letter: name.clone(),
                name: name.clone(),
                parent,
                side,
                ty: JointType::Linear,
                axis: [1.0, 0.0, 0.0],
                pivot: [0.0; 3],
                range: [-100.0, 100.0],
                home: 0.0,
                rapid: 1000.0,
                max_feed: 1000.0,
                accel: None,
                cyclic: false,
                tcp: false,
                preference: 0,
                reset: Reset::Never,
                diameter: false,
            });
            for f in ["range", "rapid", "max_feed"] {
                touch.unknown.push(format!("joint.{name}.{f}"));
            }
            *pick = d.joints.len() - 1;
            touch.any = true;
            touch.geometry = true;
        }
        if let Some(j) = d.joints.get(*pick) {
            let users = link_users(d, &j.name);
            let r = ui.add_enabled(users.is_empty(), egui::Button::new("Delete"));
            let r = if users.is_empty() {
                r
            } else {
                r.on_disabled_hover_text(format!("used by {}", users.join(", ")))
            };
            if r.clicked() {
                let n = j.name.clone();
                d.joints.remove(*pick);
                d.cites.retain(|c| !c.path.starts_with(&format!("joint.{n}.")));
                *pick = 0;
                touch.any = true;
                touch.geometry = true;
            }
        }
    });
    let Some(j) = d.joints.get(*pick).cloned() else {
        return;
    };
    ui.separator();
    let mut jj = j.clone();
    let p = |f: &str| format!("joint.{}.{f}", j.name);
    let mut name = jj.name.clone();
    ui.horizontal(|ui| {
        label_w(ui, t, "Name");
        if ui
            .add(egui::TextEdit::singleline(&mut name).desired_width(60.0))
            .changed()
        {
            touch.any = true;
        }
        label_w(ui, t, "Letter");
        if ui
            .add(egui::TextEdit::singleline(&mut jj.letter).desired_width(40.0))
            .changed()
        {
            touch.any = true;
        }
    });
    let mut links: Vec<String> = vec!["base".into()];
    links.extend(
        d.joints
            .iter()
            .filter(|x| x.name != j.name)
            .map(|x| x.name.clone()),
    );
    link_row(ui, t, "Parent", "machine:j-parent", &mut jj.parent, &links, touch);
    pick_row(
        ui,
        t,
        "Type",
        "machine:j-type",
        &mut jj.ty,
        &[("linear", JointType::Linear), ("rotary", JointType::Rotary)],
        touch,
    );
    pick_row(
        ui,
        t,
        "Side",
        "machine:j-side",
        &mut jj.side,
        &[("head", Side::Head), ("table", Side::Table)],
        touch,
    );
    vec_row(ui, c, "Axis", &p("axis"), &mut jj.axis, 0.01, touch);
    if jj.ty == JointType::Rotary {
        vec_row(ui, c, "Pivot", &p("pivot"), &mut jj.pivot, 1.0, touch);
    }
    vec_row(ui, c, "Range", &p("range"), &mut jj.range, 1.0, touch);
    num_row(ui, c, "Home", &p("home"), &mut jj.home, 1.0, touch);
    let u = if jj.ty == JointType::Linear {
        "mm/min"
    } else {
        "deg/min"
    };
    num_row(
        ui,
        c,
        &format!("Rapid {u}"),
        &p("rapid"),
        &mut jj.rapid,
        10.0,
        touch,
    );
    num_row(ui, c, "Max feed", &p("max_feed"), &mut jj.max_feed, 10.0, touch);
    ui.horizontal(|ui| {
        label_w(ui, t, "Accel");
        let mut has = jj.accel.is_some();
        if ui.checkbox(&mut has, "").changed() {
            jj.accel = if has { Some(1000.0) } else { None };
            touch.edited.push(p("accel"));
            touch.any = true;
        }
        if let Some(a) = &mut jj.accel {
            if ui.add(egui::DragValue::new(a).speed(10.0)).changed() {
                touch.edited.push(p("accel"));
                touch.any = true;
            }
            chip(ui, c, &p("accel"), touch);
        } else {
            ui.label(small(t, "absent: times assume infinite acceleration"));
        }
    });
    ui.horizontal(|ui| {
        if jj.ty == JointType::Rotary {
            touch.any |= ui.checkbox(&mut jj.cyclic, "cyclic").changed();
            touch.any |= ui.checkbox(&mut jj.tcp, "tcp").changed();
        } else {
            touch.any |= ui.checkbox(&mut jj.diameter, "diameter (lathe X)").changed();
        }
    });
    if jj != j {
        if jj.ty == JointType::Linear {
            jj.cyclic = false;
            jj.tcp = false;
        } else {
            jj.diameter = false;
        }
        d.joints[*pick] = jj;
    }
    if name != j.name && !name.is_empty() && name != "base" && !d.joints.iter().any(|x| x.name == name) {
        rename_joint(d, &j.name, &name);
        touch.geometry = true;
    }
}

fn tab_spindles(ui: &mut Ui, c: &Ctx, d: &mut MachineDef, pick: &mut usize, touch: &mut Touch) {
    let t = c.t;
    ui.horizontal_wrapped(|ui| {
        for (i, s) in d.spindles.iter().enumerate() {
            if ui.selectable_label(*pick == i, &s.name).clicked() {
                *pick = i;
            }
        }
    });
    let links: Vec<String> = std::iter::once("base".to_string())
        .chain(d.joints.iter().map(|j| j.name.clone()))
        .collect();
    let Some(s) = d.spindles.get_mut(*pick) else {
        ui.label(small(t, "No spindle"));
        return;
    };
    let p = |f: &str| format!("spindle.{}.{f}", s.name);
    let (pr, pn, pm, pa, pc) = (p("rpm"), p("nose"), p("max"), p("axis"), p("curve"));
    link_row(ui, t, "Link", "machine:s-link", &mut s.link, &links, touch);
    pick_row(
        ui,
        t,
        "Kind",
        "machine:s-kind",
        &mut s.kind,
        &[
            ("milling", forge_machine::def::SpindleKind::Milling),
            ("turning", forge_machine::def::SpindleKind::Turning),
        ],
        touch,
    );
    vec_row(ui, c, "Axis", &pa, &mut s.axis, 0.01, touch);
    vec_row(ui, c, "Nose", &pn, &mut s.nose, 1.0, touch);
    vec_row(ui, c, "rpm", &pr, &mut s.rpm, 10.0, touch);
    vec_row(ui, c, "Max", &pm, &mut s.max, 1.0, touch);
    ui.label(small(
        t,
        "Max: [tool length, diameter] (milling) or [swing, part length] (turning)",
    ));
    text_row(ui, t, "Interface", &mut s.interface, touch);
    ui.label(
        RichText::new("Power curve (rpm, kW)")
            .font(semi(12.0))
            .color(t.ink),
    );
    let mut del = None;
    let mut ch = false;
    for (i, pt) in s.power.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ch |= ui
                .add(egui::DragValue::new(&mut pt[0]).speed(10.0).suffix(" rpm"))
                .changed();
            ch |= ui
                .add(egui::DragValue::new(&mut pt[1]).speed(0.1).suffix(" kW"))
                .changed();
            if ui.small_button("x").clicked() {
                del = Some(i);
            }
        });
    }
    ui.horizontal(|ui| {
        if ui.button("+ point").clicked() {
            let last = s.power.last().copied().unwrap_or([0.0, 0.0]);
            s.power.push([last[0] + 1000.0, last[1]]);
            ch = true;
        }
        if s.power.is_empty() {
            ui.label(small(t, "none: no power check"));
        } else {
            chip(ui, c, &pc, touch);
        }
    });
    if let Some(i) = del {
        s.power.remove(i);
        ch = true;
    }
    if ch {
        touch.edited.push(pc);
        touch.any = true;
    }
}

fn tab_changer(ui: &mut Ui, c: &Ctx, d: &mut MachineDef, touch: &mut Touch) {
    let t = c.t;
    let mut has = d.changer.is_some();
    if ui.checkbox(&mut has, "Tool changer").changed() {
        d.changer = has.then_some(Changer {
            pockets: 1,
            change_s: 1.0,
            max_tool_kg: 1.0,
        });
        if has {
            for f in ["pockets", "change_s", "max_tool_kg"] {
                touch.unknown.push(format!("changer.{f}"));
            }
        } else {
            d.cites.retain(|c| !c.path.starts_with("changer."));
        }
        touch.any = true;
    }
    if let Some(ch) = &mut d.changer {
        let mut p = ch.pockets as f64;
        num_row(ui, c, "Pockets", "changer.pockets", &mut p, 1.0, touch);
        ch.pockets = p.max(1.0).round() as u32;
        num_row(
            ui,
            c,
            "Change s",
            "changer.change_s",
            &mut ch.change_s,
            0.1,
            touch,
        );
        num_row(
            ui,
            c,
            "Max tool kg",
            "changer.max_tool_kg",
            &mut ch.max_tool_kg,
            0.1,
            touch,
        );
        ui.label(small(
            t,
            "Tool-change (M06) position: not in the format yet (request F3)",
        ));
    } else {
        ui.label(small(t, "None: tools are changed by hand"));
    }
    for tu in &mut d.turrets {
        ui.separator();
        ui.label(
            RichText::new(format!("Turret {}", tu.name))
                .font(semi(12.0))
                .color(t.ink),
        );
        let p = |f: &str| format!("turret.{}.{f}", tu.name);
        let mut s = tu.stations as f64;
        num_row(ui, c, "Stations", &p("stations"), &mut s, 1.0, touch);
        tu.stations = s.max(1.0).round() as u32;
        num_row(ui, c, "Index s", &p("index_s"), &mut tu.index_s, 0.05, touch);
        vec_row(ui, c, "Center", &p("center"), &mut tu.center, 1.0, touch);
    }
}

fn tab_mounts(ui: &mut Ui, c: &Ctx, d: &mut MachineDef, pick: &mut usize, touch: &mut Touch) {
    let t = c.t;
    ui.horizontal_wrapped(|ui| {
        for (i, m) in d.mounts.iter().enumerate() {
            let k = if m.kind == MountKind::Tool { "tool" } else { "part" };
            if ui
                .selectable_label(*pick == i, format!("{} ({k})", m.name))
                .clicked()
            {
                *pick = i;
            }
        }
    });
    let links: Vec<String> = std::iter::once("base".to_string())
        .chain(d.joints.iter().map(|j| j.name.clone()))
        .collect();
    let Some(m) = d.mounts.get_mut(*pick) else { return };
    let pa = format!("mount.{}.at", m.name);
    link_row(ui, t, "Link", "machine:m-link", &mut m.link, &links, touch);
    vec_row(ui, c, "At", &pa, &mut m.at, 1.0, touch);
    vec_row(ui, c, "Dir", "", &mut m.dir, 0.01, touch);
    if m.kind == MountKind::Part {
        vec_row(ui, c, "X dir", "", &mut m.xdir, 0.01, touch);
    }
    ui.label(small(
        t,
        if m.kind == MountKind::Tool {
            "at: the gauge point; dir: from it to the tool tip"
        } else {
            "at: the fixture datum; dir: the part's +Z; X dir: its +X"
        },
    ));
}

fn tab_bodies(
    ui: &mut Ui,
    c: &Ctx,
    d: &mut MachineDef,
    pick: &mut usize,
    file_in: &mut String,
    touch: &mut Touch,
) {
    let t = c.t;
    ui.horizontal_wrapped(|ui| {
        for (i, b) in d.bodies.iter().enumerate() {
            if ui.selectable_label(*pick == i, &b.name).clicked() {
                *pick = i;
            }
        }
        if ui.button("+ body").clicked() {
            let n = (1..)
                .map(|k| format!("body{k}"))
                .find(|n| !d.bodies.iter().any(|b| &b.name == n))
                .unwrap();
            d.bodies.push(Body {
                name: n.clone(),
                link: "base".into(),
                prims: vec![Prim::Box {
                    min: [-50.0; 3],
                    max: [50.0; 3],
                }],
                part: None,
                ignore: Vec::new(),
            });
            touch.unknown.push(format!("body.{n}"));
            *pick = d.bodies.len() - 1;
            touch.any = true;
            touch.geometry = true;
        }
    });
    let links: Vec<String> = std::iter::once("base".to_string())
        .chain(d.joints.iter().map(|j| j.name.clone()))
        .collect();
    let names: Vec<String> = d.bodies.iter().map(|b| b.name.clone()).collect();
    if *pick >= d.bodies.len() {
        return;
    }
    let path = format!("body.{}", d.bodies[*pick].name);
    let b = &mut d.bodies[*pick];
    link_row(ui, t, "Link", "machine:b-link", &mut b.link, &links, touch);
    ui.horizontal(|ui| {
        label_w(ui, t, "Envelope");
        chip(ui, c, &path, touch);
    });
    ui.horizontal(|ui| {
        label_w(ui, t, "Part");
        ui.label(small(
            t,
            b.part
                .clone()
                .unwrap_or_else(|| "none (the envelope is the body)".into()),
        ));
    });
    ui.horizontal(|ui| {
        label_w(ui, t, "Geometry");
        ui.add(
            egui::TextEdit::singleline(file_in)
                .hint_text(".fg, .step or .stl path")
                .desired_width(170.0),
        );
        if ui.button("Browse").clicked()
            && let Some(f) = crate::plat::pick_file(
                Some("A body's geometry"),
                "Part, STEP or mesh",
                &["fg", "step", "stp", "stl"],
            )
        {
            *file_in = f;
        }
    });
    ui.horizontal(|ui| {
        if ui
            .add_enabled(!file_in.trim().is_empty(), egui::Button::new("Attach"))
            .clicked()
        {
            touch.attach = Some(file_in.clone());
        }
        if ui
            .add_enabled(b.part.is_some(), egui::Button::new("Fit envelope to part"))
            .clicked()
        {
            touch.fit = true;
        }
        if ui
            .add_enabled(b.part.is_some(), egui::Button::new("Detach"))
            .clicked()
        {
            touch.detach = true;
        }
    });
    let mut del = None;
    let mut ch = false;
    for (i, p) in b.prims.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            match p {
                Prim::Box { min, max } => {
                    ui.label(small(t, "box min"));
                    for x in min.iter_mut() {
                        ch |= ui.add(egui::DragValue::new(x).speed(1.0)).changed();
                    }
                    ui.label(small(t, "max"));
                    for x in max.iter_mut() {
                        ch |= ui.add(egui::DragValue::new(x).speed(1.0)).changed();
                    }
                }
                Prim::Cyl { base, axis, r, h } => {
                    ui.label(small(t, "cyl base"));
                    for x in base.iter_mut() {
                        ch |= ui.add(egui::DragValue::new(x).speed(1.0)).changed();
                    }
                    ui.label(small(t, "axis"));
                    for x in axis.iter_mut() {
                        ch |= ui.add(egui::DragValue::new(x).speed(0.01)).changed();
                    }
                    ui.label(small(t, "r h"));
                    ch |= ui.add(egui::DragValue::new(r).speed(1.0)).changed();
                    ch |= ui.add(egui::DragValue::new(h).speed(1.0)).changed();
                }
            }
            if ui.small_button("x").clicked() {
                del = Some(i);
            }
        });
    }
    if let Some(i) = del {
        b.prims.remove(i);
        ch = true;
    }
    ui.horizontal(|ui| {
        if ui.button("+ box").clicked() {
            b.prims.push(Prim::Box {
                min: [-50.0; 3],
                max: [50.0; 3],
            });
            ch = true;
        }
        if ui.button("+ cylinder").clicked() {
            b.prims.push(Prim::Cyl {
                base: [0.0; 3],
                axis: [0.0, 0.0, 1.0],
                r: 50.0,
                h: 100.0,
            });
            ch = true;
        }
    });
    // boxes before cylinders: the reader's order
    if ch {
        b.prims.sort_by_key(|p| matches!(p, Prim::Cyl { .. }));
        touch.edited.push(path);
        touch.any = true;
        touch.geometry = true;
    }
    ui.label(
        RichText::new("Touches by construction (not checked)")
            .font(semi(12.0))
            .color(t.ink),
    );
    ui.horizontal_wrapped(|ui| {
        for n in &names {
            if *n == b.name {
                continue;
            }
            let mut on = b.ignore.contains(n);
            if ui.checkbox(&mut on, n).changed() {
                if on {
                    b.ignore.push(n.clone());
                } else {
                    b.ignore.retain(|x| x != n);
                }
                touch.any = true;
            }
        }
    });
}

fn tab_check(ui: &mut Ui, t: &Theme, r: Option<&CheckReport>, d: &MachineDef, _q: &[f64]) {
    let Some(r) = r else {
        ui.label(small(
            t,
            "Not checked yet: \"Check travel\" in the jog bar runs it",
        ));
        return;
    };
    let good = |ok: bool, s: &str| {
        RichText::new(s)
            .font(sans(12.0))
            .color(if ok { t.success } else { t.error })
    };
    ui.label(good(
        r.ok(),
        if r.ok() {
            "no problem found"
        } else {
            "problems found"
        },
    ));
    if let Some(e) = &r.structure {
        ui.label(good(false, &format!("structure: {e}")));
        return;
    }
    ui.label(good(
        r.home.is_empty(),
        if r.home.is_empty() {
            "home pose: every pair clear"
        } else {
            "home pose: contact"
        },
    ));
    for (a, b, v) in &r.home {
        ui.label(good(false, &format!("  {a} / {b}: {v}")));
    }
    ui.label(
        RichText::new("Self-collision through travel")
            .font(semi(12.0))
            .color(t.ink),
    );
    let names: Vec<&str> = d.joints.iter().map(|j| j.name.as_str()).collect();
    for p in &r.pairs {
        let (txt, col) = match &p.travel {
            Travel::Proven => ("proven clear".to_string(), t.success),
            Travel::Hit { q, .. } => (
                format!(
                    "HIT at {}",
                    q.iter()
                        .zip(&names)
                        .map(|(x, n)| format!("{n}={x:.1}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                ),
                t.error,
            ),
            Travel::Near { gap_lo, .. } => (format!("NEAR (bound {gap_lo:.2} mm)"), t.error),
            Travel::Sampled { lines } => (format!("{lines} lines clear, between them not proven"), t.warn),
            Travel::Skipped(w) => (format!("not checked ({w})"), t.ink2),
        };
        let unk = if p.unknown_envelope.is_empty() {
            ""
        } else {
            "  [envelope UNKNOWN]"
        };
        ui.label(
            RichText::new(format!("{} / {}: {txt}{unk}", p.a, p.b))
                .font(sans(11.0))
                .color(col),
        );
    }
    if !r.envelopes.is_empty() {
        ui.label(RichText::new("Parts in envelopes").font(semi(12.0)).color(t.ink));
        for (b, e) in &r.envelopes {
            let (s, col) = match e {
                EnvelopeCheck::Inside { .. } => ("inside".to_string(), t.success),
                EnvelopeCheck::Outside { outside, worst, .. } => {
                    (format!("OUTSIDE ({outside} vertices, {worst:.2} mm)"), t.error)
                }
                EnvelopeCheck::NotChecked(w) => (format!("not checked ({w})"), t.ink2),
            };
            ui.label(RichText::new(format!("{b}: {s}")).font(sans(11.0)).color(col));
        }
    }
    ui.label(RichText::new("Provenance").font(semi(12.0)).color(t.ink));
    for v in &r.values {
        let s = v.status.join(", ");
        let col = if s.contains("unknown") { t.warn } else { t.ink2 };
        ui.label(
            RichText::new(format!("{}: {s}", v.path))
                .font(mono(10.5))
                .color(col),
        );
    }
    for n in &r.notes {
        ui.label(small(t, format!("note: {n}")));
    }
}

fn jog(ui: &mut Ui, t: &Theme, st: &mut MachineUi, places: &mut Vec<(String, egui::Rect)>) {
    let Some(d) = st.def.clone() else { return };
    let mut moved = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Jog").font(semi(12.0)).color(t.ink));
        let r = ui.button("Home");
        places.push(("machine:home".into(), r.rect));
        if r.clicked() {
            st.q = d.joints.iter().map(|j| j.home).collect();
            moved = true;
        }
        let r = ui.add_enabled(st.problem.is_none(), egui::Button::new("Check travel"));
        places.push(("machine:check".into(), r.rect));
        if r.clicked() {
            st.run_check();
        }
        if st.problem.is_some() {
            ui.label(
                RichText::new("fix the definition to jog and check")
                    .font(sans(11.0))
                    .color(t.error),
            );
        } else if st.live.is_empty() {
            ui.label(
                RichText::new("no contact at this pose")
                    .font(sans(11.0))
                    .color(t.success),
            );
        } else {
            let s: Vec<String> = st
                .live
                .iter()
                .map(|(a, b, hit)| format!("{a} / {b} {}", if *hit { "HIT" } else { "near" }))
                .collect();
            ui.label(
                RichText::new(format!("contact: {}", s.join(", ")))
                    .font(sans(11.0))
                    .color(t.error),
            );
        }
    });
    ui.spacing_mut().slider_width = 150.0;
    ui.horizontal_wrapped(|ui| {
        for (i, j) in d.joints.iter().enumerate() {
            if i >= st.q.len() {
                break;
            }
            let (lo, hi) = (j.range[0].min(j.range[1]), j.range[0].max(j.range[1]));
            ui.label(RichText::new(&j.name).font(semi(12.0)).color(t.ink));
            let u = if j.ty == JointType::Linear { " mm" } else { "°" };
            // a copy: the slider rounds what it shows, and only a move by the user may change q
            let mut x = st.q[i];
            let r = ui.add(egui::Slider::new(&mut x, lo..=hi).suffix(u).max_decimals(3));
            places.push((format!("machine:jog:{}", j.name), r.rect));
            if r.changed() && (r.dragged() || r.has_focus() || r.clicked()) {
                st.q[i] = x;
                moved = true;
            }
            ui.add_space(8.0);
        }
    });
    if moved {
        st.jogged();
    }
}
