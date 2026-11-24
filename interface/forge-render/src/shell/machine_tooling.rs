//! The library builder's tooling tabs (research/cam/library-builder.md): cutting tools,
//! holders, collets, collet nuts, pull studs, inserts and turning holders, from forge-tooling
//! through forge-tooling-builder. The same shape as the machine tabs: the records by level and
//! file with a search, a new one derived from the chosen one, every field of its table edited
//! with its provenance (an edit makes the value the user's), the record read again after every
//! edit (forge-tooling's own rules), the checks, save to the user's or the project's folder,
//! and its turned outline revolved in the view.
use super::theme::{Theme, mono, sans, semi};
use egui::{RichText, Ui};
use forge_library::{Entry, Level};
use forge_tooling::build::Finding;
use forge_tooling::toml::{Table, Val};
use forge_tooling_builder::Rec;
use forge_tooling_builder::dynamic;
use std::path::PathBuf;

pub struct ToolingUi {
    /// The kind's TOML key: "tool", "holder", ...
    pub word: &'static str,
    pub entries: Vec<Entry<Rec>>,
    pub errors: Vec<(Level, String, String)>,
    gen_seen: u64,
    /// The record in the editor as last read, and its table as edited (may not read).
    pub cur: Option<Rec>,
    pub table: Table,
    pub origin: Option<(Level, String)>,
    pub changed: bool,
    /// The edited table's refusal by forge-tooling, if any.
    pub problem: Option<String>,
    pub findings: Vec<Finding>,
    pub new_id: String,
    pub filter: String,
    pub message: String,
}

fn user_cam() -> Option<PathBuf> {
    forge_tooling_builder::user_cam()
}

impl ToolingUi {
    pub fn new(word: &'static str, project: Option<&std::path::Path>) -> ToolingUi {
        let mut t = ToolingUi {
            word,
            entries: Vec::new(),
            errors: Vec::new(),
            gen_seen: u64::MAX,
            cur: None,
            table: Table::new(),
            origin: None,
            changed: false,
            problem: None,
            findings: Vec::new(),
            new_id: format!("my-{}", word.replace('_', "-")),
            filter: String::new(),
            message: String::new(),
        };
        t.reload(project);
        if let Some(e) = t.entries.first().cloned() {
            t.pick(e);
        }
        t
    }

    pub fn reload(&mut self, project: Option<&std::path::Path>) {
        let (e, errs) = dynamic::catalog(self.word, project, user_cam().as_deref());
        self.entries = e;
        self.errors = errs;
        self.gen_seen = forge_library::generation();
    }

    pub fn pick(&mut self, e: Entry<Rec>) {
        self.table = e.def.table.clone();
        self.cur = Some(e.def);
        self.origin = Some((e.level, e.file));
        self.changed = false;
        self.problem = None;
        self.findings.clear();
    }

    pub fn derive(&mut self, project: Option<&std::path::Path>) {
        let Some(from) = self.cur.as_ref().map(|r| r.id.clone()) else {
            return;
        };
        match dynamic::derive(self.word, &from, &self.new_id, project, user_cam().as_deref()) {
            Ok(r) => {
                self.message = format!(
                    "New {} `{}` from `{from}`: copied values cited unknown",
                    self.word, r.id
                );
                self.table = r.table.clone();
                self.cur = Some(r);
                self.origin = None;
                self.changed = true;
                self.problem = None;
                self.findings.clear();
            }
            Err(e) => self.message = e,
        }
    }

    /// After an edit of `table`: read it again with forge-tooling's rules.
    pub fn reread(&mut self) {
        match dynamic::reread(self.word, &self.table) {
            Ok(r) => {
                self.cur = Some(r);
                self.problem = None;
            }
            Err(e) => self.problem = Some(e),
        }
        self.changed = true;
        self.findings.clear();
    }

    pub fn save(&mut self, level: Level, project: Option<&std::path::Path>) {
        if let Some(p) = &self.problem {
            self.message = format!("Not saved: {p}");
            return;
        }
        let Some(r) = self.cur.clone() else { return };
        let Some(dir) = dynamic::level_dir(self.word, level, project, user_cam().as_deref()) else {
            self.message = "Not saved: no folder for that level (open a part for its project)".into();
            return;
        };
        let same = matches!(&self.origin, Some((l, f)) if *l == level
            && std::path::Path::new(f) == forge_library::file_for(&dir, &r.id));
        match dynamic::save(self.word, &r, &dir, level, same || self.origin.is_none()) {
            Ok(f) => {
                self.message = format!("Saved {} ({} level)", f.display(), level.name());
                self.origin = Some((level, f.display().to_string()));
                self.changed = false;
                self.reload(project);
            }
            Err(e) => self.message = format!("Not saved: {e}"),
        }
    }

    pub fn check(&mut self, project: Option<&std::path::Path>) {
        let Some(r) = &self.cur else { return };
        let (lib, _) = forge_tooling_builder::library(project, user_cam().as_deref());
        self.findings = forge_tooling::build::check(&lib, &r.typed);
        self.message = if self.findings.is_empty() {
            format!("{} `{}`: no problem found", self.word, r.id)
        } else {
            format!("{} `{}`: {} problems", self.word, r.id, self.findings.len())
        };
    }

    /// The preview: the record's turned outline, revolved.
    pub fn preview(&self) -> Option<crate::machine_mesh::Tris> {
        let o = dynamic::outline(self.cur.as_ref()?)?;
        Some(crate::machine_mesh::revolve(&o))
    }
}

fn badge(t: &Theme, s: Option<&str>) -> (String, egui::Color32) {
    match s {
        None => ("uncited".into(), t.ink2),
        Some("user") => ("user".into(), t.success),
        Some("unknown") => ("unknown".into(), t.warn),
        Some("published") => ("published".into(), t.accent),
        Some("estimate") => ("estimate".into(), t.warn),
        Some("conflict") => ("conflict".into(), t.error),
        Some(x) => (x.to_string(), t.ink2),
    }
}

/// The library list of a tooling kind (the browser's or the floating panel's).
pub fn library(
    ui: &mut Ui,
    t: &Theme,
    st: &mut ToolingUi,
    project: Option<&std::path::Path>,
    places: &mut Vec<(String, egui::Rect)>,
) {
    if st.gen_seen != forge_library::generation() {
        st.reload(project);
    }
    let r = ui.add(
        egui::TextEdit::singleline(&mut st.filter)
            .hint_text("search: id, file")
            .desired_width(ui.available_width()),
    );
    places.push(("tooling:search".into(), r.rect));
    let want = st.filter.trim().to_lowercase();
    let mut chosen = None;
    egui::ScrollArea::vertical()
        .id_salt(("tooling:list", st.word))
        .max_height((ui.available_height() - 200.0).max(80.0))
        .show(ui, |ui| {
            for lv in [Level::Project, Level::User, Level::Builtin] {
                let here: Vec<&Entry<Rec>> = st
                    .entries
                    .iter()
                    .filter(|e| {
                        e.level == lv
                            && (want.is_empty()
                                || e.def.id.to_lowercase().contains(&want)
                                || e.file.to_lowercase().contains(&want))
                    })
                    .collect();
                ui.label(
                    RichText::new(lv.name().to_uppercase())
                        .font(semi(10.0))
                        .color(t.ink2),
                );
                if here.is_empty() {
                    ui.label(RichText::new("  none").font(sans(11.0)).color(t.ink2));
                    continue;
                }
                let n = here.len();
                egui::CollapsingHeader::new(RichText::new(format!("{} ({n})", st.word)).font(sans(11.0)))
                    .id_salt(("tooling:group", st.word, lv.name()))
                    .default_open(n <= 40 || !want.is_empty())
                    .show(ui, |ui| {
                        for e in here.into_iter().take(400) {
                            let on = st.cur.as_ref().is_some_and(|c| c.id == e.def.id)
                                && st.origin.as_ref().is_some_and(|(l, _)| *l == e.level);
                            let mut txt = e.def.id.clone();
                            if !e.hides.is_empty() {
                                txt.push_str("  (hides)");
                            }
                            let r = ui.selectable_label(on, RichText::new(txt).font(sans(12.0)));
                            places.push((format!("tooling:pick:{}", e.def.id), r.rect));
                            if r.clicked() {
                                chosen = Some(e.clone());
                            }
                        }
                    });
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
    ui.label(
        RichText::new(format!("New {}", st.word.replace('_', " ")))
            .font(semi(12.0))
            .color(t.ink),
    );
    ui.horizontal(|ui| {
        ui.label(RichText::new("Id").font(sans(12.0)).color(t.ink2));
        let r = ui.add(egui::TextEdit::singleline(&mut st.new_id).desired_width(150.0));
        places.push(("tooling:id".into(), r.rect));
    });
    let r = ui.add_enabled(st.cur.is_some(), egui::Button::new("Derive from this one"));
    places.push(("tooling:derive".into(), r.rect));
    if r.clicked() {
        st.derive(project);
    }
}

/// The editor: every field of the record's table, with its provenance.
pub fn editor(ui: &mut Ui, t: &Theme, st: &mut ToolingUi) {
    let Some(cur) = st.cur.clone() else {
        ui.label(
            RichText::new("No record: pick one")
                .font(sans(11.0))
                .color(t.ink2),
        );
        return;
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(&cur.id).font(semi(14.0)).color(t.ink));
        ui.add_space(6.0);
        let from = match &st.origin {
            Some((l, f)) => format!("{} · {f}", l.name()),
            None => "not saved".into(),
        };
        ui.label(RichText::new(from).font(sans(11.0)).color(t.ink2));
    });
    match &st.problem {
        Some(p) => ui.label(
            RichText::new(format!("refused: {p}"))
                .font(sans(11.0))
                .color(t.error),
        ),
        None => ui.label(
            RichText::new("valid record (read by forge-tooling)")
                .font(sans(11.0))
                .color(t.success),
        ),
    };
    ui.separator();
    let keys: Vec<String> = st
        .table
        .keys()
        .filter(|k| !matches!(k.as_str(), "id" | "cite" | "real"))
        .cloned()
        .collect();
    let mut edit: Option<(String, Val)> = None;
    let mut unknown: Option<String> = None;
    egui::ScrollArea::vertical()
        .id_salt("tooling:form")
        .show(ui, |ui| {
            egui::Grid::new("tooling:grid")
                .num_columns(3)
                .striped(true)
                .show(ui, |ui| {
                    for k in &keys {
                        let v = st.table.get(k).cloned().unwrap_or(Val::Bool(false));
                        ui.label(RichText::new(k).font(mono(11.5)).color(t.ink2));
                        let bookkeeping = matches!(
                            k.as_str(),
                            "source" | "taken" | "note" | "vendor" | "product_id" | "maker"
                        );
                        match v {
                            Val::Num(mut x) => {
                                if ui
                                    .add(egui::DragValue::new(&mut x).speed(0.05).max_decimals(4))
                                    .changed()
                                {
                                    edit = Some((k.clone(), Val::Num(x)));
                                }
                            }
                            Val::Str(mut s) => {
                                if ui
                                    .add(egui::TextEdit::singleline(&mut s).desired_width(180.0))
                                    .changed()
                                {
                                    edit = Some((k.clone(), Val::Str(s)));
                                }
                            }
                            Val::Bool(mut b) => {
                                if ui.checkbox(&mut b, "").changed() {
                                    edit = Some((k.clone(), Val::Bool(b)));
                                }
                            }
                            Val::Arr(a) => {
                                // rows of numbers (a profile's rings) or a list
                                let mut a2 = a.clone();
                                let mut ch = false;
                                ui.vertical(|ui| {
                                    for row in a2.iter_mut() {
                                        ui.horizontal(|ui| match row {
                                            Val::Arr(cols) => {
                                                for c in cols.iter_mut() {
                                                    if let Val::Num(x) = c {
                                                        ch |= ui
                                                            .add(
                                                                egui::DragValue::new(x)
                                                                    .speed(0.05)
                                                                    .max_decimals(4),
                                                            )
                                                            .changed();
                                                    }
                                                }
                                            }
                                            Val::Num(x) => {
                                                ch |= ui
                                                    .add(egui::DragValue::new(x).speed(0.05).max_decimals(4))
                                                    .changed()
                                            }
                                            Val::Str(s) => {
                                                ui.label(RichText::new(s.as_str()).font(sans(11.0)));
                                            }
                                            _ => {}
                                        });
                                    }
                                });
                                if ch {
                                    edit = Some((k.clone(), Val::Arr(a2)));
                                }
                            }
                            Val::Table(_) | Val::Tables(_) => {
                                ui.label(
                                    RichText::new("(nested: kept as read)")
                                        .font(sans(10.0))
                                        .color(t.ink2),
                                );
                            }
                        }
                        if bookkeeping {
                            ui.label("");
                        } else {
                            ui.horizontal(|ui| {
                                let (w, col) = badge(t, dynamic::status_of(&st.table, k).as_deref());
                                ui.label(RichText::new(w).font(sans(10.0)).color(col));
                                if ui
                                    .small_button("?")
                                    .on_hover_text("I don't know this value: mark it unknown")
                                    .clicked()
                                {
                                    unknown = Some(k.clone());
                                }
                            });
                        }
                        ui.end_row();
                    }
                });
            if !st.findings.is_empty() {
                ui.separator();
                ui.label(RichText::new("Checks").font(semi(12.0)).color(t.ink));
                for f in &st.findings {
                    ui.label(
                        RichText::new(format!(
                            "{}: {} (limit {}): {}",
                            f.path, f.measured, f.limit, f.fix
                        ))
                        .font(sans(11.0))
                        .color(t.error),
                    );
                }
            }
        });
    if let Some((k, v)) = edit {
        let bookkeeping = matches!(
            k.as_str(),
            "source" | "taken" | "note" | "vendor" | "product_id" | "maker"
        );
        if bookkeeping {
            st.table.insert(k, v);
        } else {
            dynamic::set_user(&mut st.table, &k, v);
        }
        st.reread();
    }
    if let Some(k) = unknown {
        let mut cites: Vec<Table> = match st.table.remove("cite") {
            Some(Val::Tables(c)) => c,
            _ => Vec::new(),
        };
        cites.retain(|c| forge_tooling::toml::text(c, "path") != Some(k.as_str()));
        let mut c = Table::new();
        c.insert("path".into(), Val::Str(k.clone()));
        c.insert("status".into(), Val::Str("unknown".into()));
        c.insert("basis".into(), Val::Str(super::machine::USER_UNKNOWN.into()));
        cites.push(c);
        st.table.insert("cite".into(), Val::Tables(cites));
        st.reread();
    }
}
