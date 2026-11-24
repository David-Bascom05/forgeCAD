//! The library builder's workholding tab (research/cam/library-builder.md): vises, clamps,
//! plates, zero-point systems, tombstones, vacuum, custom fixtures and lathe chucks, collets,
//! drivers, centres and rests, from forge-workholding through forge-workholding-builder. The
//! records by level and kind with a search, a new one derived from the chosen one
//! (forge_workholding::schema::new_from), each field of its kind's form with its provenance (an
//! edit makes it the user's; conflicting and unpublished values say so), the checks (a field not
//! published is "not checked", not a failure), save to the user's or the project's folder, and
//! the record placed and drawn in the view.
use super::theme::{Theme, sans, semi};
use egui::{RichText, Ui};
use forge_library::schema::Ty;
use forge_library::{Catalog, Entry, Level};
use forge_workholding::Record;
use forge_workholding_builder::{Finding, Workholding};
use std::path::Path;

pub struct WorkholdingUi {
    pub entries: Vec<Entry<Record>>,
    pub errors: Vec<(Level, String, String)>,
    gen_seen: u64,
    pub cur: Option<Record>,
    pub origin: Option<(Level, String)>,
    pub changed: bool,
    pub findings: Vec<Finding>,
    pub new_id: String,
    pub filter: String,
    pub message: String,
}

fn user_dir() -> Option<std::path::PathBuf> {
    forge_library::user_dir::<Workholding>()
}

impl WorkholdingUi {
    pub fn new(project: Option<&Path>) -> WorkholdingUi {
        let mut w = WorkholdingUi {
            entries: Vec::new(),
            errors: Vec::new(),
            gen_seen: u64::MAX,
            cur: None,
            origin: None,
            changed: false,
            findings: Vec::new(),
            new_id: "my-fixture".into(),
            filter: String::new(),
            message: String::new(),
        };
        w.reload(project);
        if let Some(e) = w.entries.first().cloned() {
            w.pick(e);
        }
        w
    }

    pub fn reload(&mut self, project: Option<&Path>) {
        let c = Catalog::<Workholding>::load(project, user_dir().as_deref());
        self.entries = c.entries;
        self.errors = c.errors;
        self.gen_seen = forge_library::generation();
    }

    pub fn pick(&mut self, e: Entry<Record>) {
        self.cur = Some(e.def);
        self.origin = Some((e.level, e.file));
        self.changed = false;
        self.findings.clear();
    }

    pub fn derive(&mut self) {
        let Some(from) = self.cur.as_ref().map(|r| r.name.clone()) else {
            return;
        };
        if let Err(e) = forge_library::check_id(&self.new_id) {
            self.message = e;
            return;
        }
        let lib = forge_workholding::Library {
            records: self
                .entries
                .iter()
                .map(|e| (e.def.clone(), e.file.clone()))
                .collect(),
        };
        match forge_workholding::schema::new_from(&lib, &self.new_id, &from, &[]) {
            Ok(r) => {
                self.message = format!(
                    "New `{}` from `{from}`: its values keep their cites (a stand-in's say so)",
                    r.name
                );
                self.cur = Some(r);
                self.origin = None;
                self.changed = true;
                self.findings.clear();
            }
            Err(e) => self.message = e.to_string(),
        }
    }

    /// Field `k` set to `v` as the user's value.
    pub fn set(&mut self, k: &str, v: f64) {
        let Some(r) = &self.cur else { return };
        let got = r.with(k, v);
        self.took(got);
    }

    /// A list field (a parallel set's heights) set as the user's.
    pub fn set_list(&mut self, k: &str, v: Vec<f64>) {
        let Some(r) = &self.cur else { return };
        let got = r.with_list(k, v);
        self.took(got);
    }

    /// A word field (screw, system, mount, ...) set as the user's; empty removes it.
    pub fn set_word(&mut self, k: &str, v: &str) {
        let Some(r) = &self.cur else { return };
        let got = r.with_word(k, v);
        self.took(got);
    }

    fn took(&mut self, got: Result<Record, forge_workholding::WhError>) {
        match got {
            Ok(mut r) => {
                for c in &mut r.cites {
                    if c.source == "given in the job" {
                        c.source = "user".into();
                    }
                }
                self.cur = Some(r);
                self.changed = true;
                self.findings.clear();
            }
            Err(e) => self.message = e.to_string(),
        }
    }

    pub fn check(&mut self) {
        let Some(r) = &self.cur else { return };
        self.findings = forge_workholding_builder::check(r);
        let bad = self.findings.iter().filter(|f| !f.not_checked).count();
        let nc = self.findings.len() - bad;
        self.message = format!("`{}`: {bad} problems, {nc} not checked (unpublished)", r.name);
    }

    pub fn save(&mut self, level: Level, project: Option<&Path>) {
        let Some(r) = self.cur.clone() else { return };
        let dir = match level {
            Level::Project => project.map(forge_library::project_dir::<Workholding>),
            Level::User => user_dir(),
            Level::Builtin => None,
        };
        let Some(dir) = dir else {
            self.message = "Not saved: no folder for that level (open a part for its project)".into();
            return;
        };
        let same = matches!(&self.origin, Some((l, f)) if *l == level
            && Path::new(f) == forge_library::file_for(&dir, &r.name));
        match forge_library::save::<Workholding>(&r, &[], &dir, level, same || self.origin.is_none()) {
            Ok(f) => {
                self.message = format!("Saved {} ({} level)", f.display(), level.name());
                self.origin = Some((level, f.display().to_string()));
                self.changed = false;
                self.reload(project);
            }
            Err(e) => self.message = format!("Not saved: {e}"),
        }
    }

    /// The preview from the record's dimensions: a vise or a chuck drawn as one (its parts at
    /// the record's sizes, what it does not give as ghosts); other kinds as forge-workholding
    /// places their bodies.
    pub fn library_preview(&self) -> Option<crate::library_mesh::Preview> {
        let r = self.cur.as_ref()?;
        match r.kind.name() {
            "vise" | "self_centring" => Some(crate::library_mesh::vise(r)),
            "chuck" => Some(crate::library_mesh::chuck(r)),
            _ => {
                let mut p = crate::library_mesh::Preview::default();
                for t in self.preview()? {
                    p.parts.push((t, crate::library_mesh::Look::ToolSteel));
                }
                Some(p)
            }
        }
    }

    /// The record placed as the check places it, as triangles.
    pub fn preview(&self) -> Option<Vec<crate::machine_mesh::Tris>> {
        let r = self.cur.as_ref()?;
        let bodies = forge_workholding_builder::preview(r).ok()?;
        Some(
            bodies
                .iter()
                .map(|(_, prims)| {
                    let mut t = crate::machine_mesh::Tris::default();
                    for p in prims {
                        match p {
                            forge_workholding::Prim::Block { o, axes, size } => {
                                crate::machine_mesh::obox_tris(*o, *axes, *size, &mut t)
                            }
                            forge_workholding::Prim::Cyl { base, axis, r, h } => {
                                crate::machine_mesh::prim_tris(
                                    &forge_machine::def::Prim::Cyl {
                                        base: *base,
                                        axis: *axis,
                                        r: *r,
                                        h: *h,
                                    },
                                    &mut t,
                                )
                            }
                        }
                    }
                    t
                })
                .collect(),
        )
    }
}

/// A field's provenance: (word, which colour).
fn status(t: &Theme, r: &Record, k: &str) -> (String, egui::Color32) {
    if r.conflict.iter().any(|x| x == k) {
        return ("conflict".into(), t.error);
    }
    if r.unknown.iter().any(|x| x == k) {
        return ("unknown".into(), t.warn);
    }
    if r.standin {
        return ("estimate (stand-in)".into(), t.warn);
    }
    match r.cites.iter().find(|c| c.fields.iter().any(|f| f == k)) {
        None => ("uncited".into(), t.ink2),
        Some(c) if c.source == "user" || c.source == "given in the job" => ("user".into(), t.success),
        Some(c) if c.source.starts_with("stand-in") => ("copied (stand-in)".into(), t.warn),
        Some(c) if c.source.starts_with("http") => ("published".into(), t.accent),
        Some(_) => ("cited".into(), t.ink2),
    }
}

pub fn library(
    ui: &mut Ui,
    t: &Theme,
    st: &mut WorkholdingUi,
    project: Option<&Path>,
    places: &mut Vec<(String, egui::Rect)>,
) {
    if st.gen_seen != forge_library::generation() {
        st.reload(project);
    }
    let r = ui.add(
        egui::TextEdit::singleline(&mut st.filter)
            .hint_text("search: name, kind, vendor")
            .desired_width(ui.available_width()),
    );
    places.push(("workholding:search".into(), r.rect));
    let want = st.filter.trim().to_lowercase();
    let mut chosen = None;
    egui::ScrollArea::vertical()
        .id_salt("workholding:list")
        .max_height((ui.available_height() - 200.0).max(80.0))
        .show(ui, |ui| {
            super::lib_form::empty_levels(ui, t, st.entries.iter().any(|e| e.level != Level::Builtin));
            for lv in [Level::Project, Level::User, Level::Builtin] {
                let here: Vec<&Entry<Record>> = st
                    .entries
                    .iter()
                    .filter(|e| e.level == lv)
                    .filter(|e| {
                        want.is_empty()
                            || [e.def.name.as_str(), e.def.kind.name(), e.def.vendor.as_str()]
                                .iter()
                                .any(|x| x.to_lowercase().contains(&want))
                    })
                    .collect();
                if here.is_empty() {
                    continue;
                }
                super::lib_form::level_head(ui, t, lv.name(), here.len());
                let mut kinds: Vec<&str> = here.iter().map(|e| e.def.kind.name()).collect();
                kinds.sort();
                kinds.dedup();
                for k in kinds {
                    let inner: Vec<&&Entry<Record>> =
                        here.iter().filter(|e| e.def.kind.name() == k).collect();
                    egui::CollapsingHeader::new(
                        RichText::new(format!("{k} ({})", inner.len())).font(sans(11.0)),
                    )
                    .id_salt(("workholding:group", lv.name(), k))
                    .default_open(!want.is_empty() || inner.len() <= 4)
                    .show(ui, |ui| {
                        for e in inner {
                            let on = st.cur.as_ref().is_some_and(|c| c.name == e.def.name)
                                && st.origin.as_ref().is_some_and(|(l, _)| *l == e.level);
                            let mut txt = e.def.name.clone();
                            if e.def.standin {
                                txt.push_str("  · stand-in");
                            }
                            let r = ui
                                .horizontal(|ui| {
                                    super::lib_form::thumb(ui, t, &thumb(&e.def), 26.0);
                                    ui.selectable_label(on, RichText::new(txt).font(sans(12.0)))
                                })
                                .inner;
                            places.push((format!("workholding:pick:{}", e.def.name), r.rect));
                            if r.clicked() {
                                chosen = Some((*e).clone());
                            }
                        }
                    });
                }
            }
            for (l, f, e) in &st.errors {
                ui.label(
                    RichText::new(format!("not loaded ({}): {f}: {e}", l.name()))
                        .font(sans(10.0))
                        .color(t.error),
                );
            }
        });
    if let Some(e) = chosen {
        st.pick(e);
    }
    ui.separator();
    ui.label(RichText::new("New workholding").font(semi(12.0)).color(t.ink));
    ui.horizontal(|ui| {
        ui.label(RichText::new("Name").font(sans(12.0)).color(t.ink2));
        let r = ui.add(egui::TextEdit::singleline(&mut st.new_id).desired_width(150.0));
        places.push(("workholding:id".into(), r.rect));
    });
    let r = ui.add_enabled(st.cur.is_some(), egui::Button::new("Derive from this one"));
    places.push(("workholding:derive".into(), r.rect));
    if r.clicked() {
        st.derive();
    }
}

pub fn editor(ui: &mut Ui, t: &Theme, st: &mut WorkholdingUi) {
    let Some(r) = st.cur.clone() else {
        ui.label(
            RichText::new("No record: pick one")
                .font(sans(11.0))
                .color(t.ink2),
        );
        return;
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(&r.name).font(semi(14.0)).color(t.ink));
        ui.add_space(6.0);
        ui.label(RichText::new(r.kind.name()).font(sans(11.0)).color(t.ink2));
    });
    let from = match &st.origin {
        Some((l, f)) => format!("{} level · {f}", l.name()),
        None => "not saved yet".into(),
    };
    ui.label(RichText::new(from).font(sans(10.5)).color(t.ink2));
    if !r.vendor.is_empty() || !r.product.is_empty() {
        ui.label(
            RichText::new(format!("{} {}", r.vendor, r.product))
                .font(sans(11.0))
                .color(t.ink2),
        );
    }
    super::lib_form::legend(ui, t);
    ui.separator();
    let mut set: Option<(String, f64)> = None;
    let mut set_list: Option<(String, Vec<f64>)> = None;
    let mut set_word: Option<(String, String)> = None;
    egui::ScrollArea::vertical()
        .id_salt("workholding:form")
        .show(ui, |ui| {
            let fields = forge_workholding_builder::fields(r.kind);
            for (group, title) in [("size", "Geometry"), ("rated", "Rating")] {
                super::lib_form::section(ui, t, title);
                egui::Grid::new(("workholding:grid", group))
                    .num_columns(3)
                    .spacing(egui::vec2(8.0, 4.0))
                    .striped(true)
                    .show(ui, |ui| {
                        for f in fields.iter().filter(|f| f.group == group) {
                            super::lib_form::name(
                                ui,
                                t,
                                &super::lib_form::workholding_label(&f.path),
                                &f.path,
                                f.help,
                                118.0,
                            );
                            match &f.ty {
                                Ty::Num | Ty::Int => match r.vals.get(&f.path) {
                                    Some(v) => {
                                        let mut x = *v;
                                        let ch = if f.ty == Ty::Int {
                                            super::lib_form::int_edit(ui, &mut x)
                                        } else {
                                            super::lib_form::num_edit(ui, &mut x, f.unit.suffix())
                                        };
                                        if ch {
                                            set = Some((f.path.clone(), x));
                                        }
                                    }
                                    None => {
                                        if super::lib_form::unknown_slot(ui, t, 140.0) {
                                            set = Some((f.path.clone(), 1.0));
                                        }
                                    }
                                },
                                Ty::Points(_) => {
                                    let mut v = r.lists.get(&f.path).cloned().unwrap_or_default();
                                    let mut ch = false;
                                    ui.horizontal_wrapped(|ui| {
                                        let mut del = None;
                                        for (i, x) in v.iter_mut().enumerate() {
                                            ch |= ui
                                                .add(egui::DragValue::new(x).speed(0.1).max_decimals(3))
                                                .changed();
                                            if ui.small_button("x").clicked() {
                                                del = Some(i);
                                            }
                                        }
                                        if let Some(i) = del {
                                            v.remove(i);
                                            ch = true;
                                        }
                                        if ui.small_button("+").clicked() {
                                            v.push(v.last().copied().unwrap_or(10.0));
                                            ch = true;
                                        }
                                    });
                                    if ch {
                                        set_list = Some((f.path.clone(), v));
                                    }
                                }
                                _ => {
                                    let mut w = r.words.get(&f.path).cloned().unwrap_or_default();
                                    if ui
                                        .add_sized(
                                            egui::vec2(140.0, 20.0),
                                            egui::TextEdit::singleline(&mut w),
                                        )
                                        .on_hover_text(w.clone())
                                        .lost_focus()
                                    {
                                        set_word = Some((f.path.clone(), w));
                                    }
                                }
                            }
                            let (w, _) = status(t, &r, &f.path);
                            let not_given = matches!(&f.need, forge_library::schema::Need::Optional(_))
                                && !r.vals.contains_key(&f.path)
                                && !r.unknown.contains(&f.path);
                            if not_given {
                                ui.label(RichText::new("not given").font(sans(10.5)).color(t.ink2));
                            } else {
                                let detail = r
                                    .cites
                                    .iter()
                                    .filter(|c| c.fields.contains(&f.path))
                                    .map(|c| c.source.clone())
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                super::lib_form::chip(
                                    ui,
                                    t,
                                    super::lib_form::Prov::of(Some(&w)),
                                    &detail,
                                    false,
                                );
                            }
                            ui.end_row();
                        }
                    });
            }
            if !st.findings.is_empty() {
                ui.separator();
                ui.label(RichText::new("Checks").font(semi(12.0)).color(t.ink));
                for f in &st.findings {
                    let col = if f.not_checked { t.warn } else { t.error };
                    ui.label(
                        RichText::new(format!("{}: {} — {}", f.path, f.what, f.fix))
                            .font(sans(11.0))
                            .color(col),
                    );
                }
            }
        });
    if let Some((k, v)) = set {
        st.set(&k, v);
    }
    if let Some((k, v)) = set_list {
        st.set_list(&k, v);
    }
    if let Some((k, v)) = set_word
        && st
            .cur
            .as_ref()
            .and_then(|r| r.words.get(&k))
            .map(String::as_str)
            .unwrap_or("")
            != v
    {
        st.set_word(&k, &v);
    }
}

/// A record's thumbnail: its placed bodies seen from the front.
fn thumb(r: &Record) -> Vec<Vec<[f64; 2]>> {
    use super::lib_form::{cyl_pts, rect_xz};
    let Ok(bodies) = forge_workholding_builder::preview(r) else {
        return Vec::new();
    };
    bodies
        .iter()
        .flat_map(|(_, prims)| prims.iter())
        .map(|p| match p {
            forge_workholding::Prim::Block { o, axes, size } => {
                let mut pts = Vec::new();
                for i in 0..8 {
                    let mut q = *o;
                    for (k, ax) in axes.iter().enumerate() {
                        if i >> k & 1 == 1 {
                            for d in 0..3 {
                                q[d] += ax[d] * size[k];
                            }
                        }
                    }
                    pts.push(q);
                }
                rect_xz(&pts)
            }
            forge_workholding::Prim::Cyl { base, axis, r, h } => rect_xz(&cyl_pts(*base, *axis, *r, *h)),
        })
        .collect()
}
