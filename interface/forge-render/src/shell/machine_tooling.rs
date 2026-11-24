//! The library builder's tooling tabs (research/cam/library-builder.md): cutting tools,
//! holders, collets, collet nuts, pull studs, inserts and turning holders, from forge-tooling
//! through forge-tooling-builder. The same shape as the machine tabs: the records by level and
//! file with a search, a new one derived from the chosen one, every field of its table edited
//! with its provenance (an edit makes the value the user's), the record read again after every
//! edit (forge-tooling's own rules), the checks, save to the user's or the project's folder,
//! and its turned outline revolved in the view.
use super::theme::{Theme, sans, semi};
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
    thumbs: std::collections::HashMap<String, Vec<Vec<[f64; 2]>>>,
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
            thumbs: Default::default(),
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

    /// The preview from the record's dimensions: a cutting tool's fluted length (at its helix
    /// angle, straight when the record gives none), neck and shank; a turned record's profile.
    pub fn library_preview(&self) -> Option<crate::library_mesh::Preview> {
        let r = self.cur.as_ref()?;
        let o = dynamic::outline(r)?;
        Some(match &r.typed {
            forge_tooling::build::Record::Tool(tl) => {
                let helix = forge_tooling::toml::num(&r.table, "helix").unwrap_or(0.0);
                crate::library_mesh::cutter(&o, tl.lcf, tl.nof, helix)
            }
            _ => crate::library_mesh::turned(&o),
        })
    }

    /// The preview: the record's turned outline, revolved; a cutting tool with its flutes drawn
    /// over the fluted length (at its helix angle when the record gives one, else straight).
    pub fn preview(&self) -> Option<crate::machine_mesh::Tris> {
        let r = self.cur.as_ref()?;
        let o = dynamic::outline(r)?;
        let mut m = crate::machine_mesh::revolve(&o);
        if let forge_tooling::build::Record::Tool(tl) = &r.typed
            && tl.nof > 0
            && tl.lcf > 0.0
        {
            let helix = forge_tooling::toml::num(&r.table, "helix").unwrap_or(0.0);
            m.edges.extend(flutes(tl.dc / 2.0, tl.lcf, tl.nof, helix));
        }
        Some(m)
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
    // the thumbnails: each record's turned silhouette, worked out once
    for e in &st.entries {
        if !st.thumbs.contains_key(&e.def.id) {
            let v = dynamic::outline(&e.def)
                .map(|o| vec![super::lib_form::silhouette(&o)])
                .unwrap_or_default();
            st.thumbs.insert(e.def.id.clone(), v);
        }
    }
    let thumbs = st.thumbs.clone();
    egui::ScrollArea::vertical()
        .id_salt(("tooling:list", st.word))
        .max_height((ui.available_height() - 200.0).max(80.0))
        .show(ui, |ui| {
            super::lib_form::empty_levels(ui, t, st.entries.iter().any(|e| e.level != Level::Builtin));
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
                if here.is_empty() {
                    continue;
                }
                super::lib_form::level_head(ui, t, lv.name(), here.len());
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
                            let polys = thumbs.get(&e.def.id).cloned().unwrap_or_default();
                            let r = ui
                                .horizontal(|ui| {
                                    super::lib_form::thumb(ui, t, &polys, 26.0);
                                    ui.selectable_label(on, RichText::new(txt).font(sans(12.0)))
                                })
                                .inner;
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

/// Tool types, as forge-tooling reads `type`.
const TOOL_TYPES: &[&str] = &[
    "flat",
    "ball",
    "bull",
    "chamfer",
    "spot",
    "drill",
    "tap",
    "thread_mill",
    "reamer",
    "bore",
    "counterbore",
    "countersink",
    "slot",
    "engrave",
];

/// Keys that say where a record came from, not what it is: no cite of their own.
fn bookkeeping(k: &str) -> bool {
    matches!(
        k,
        "source" | "taken" | "note" | "vendor" | "product_id" | "maker" | "code"
    )
}

fn unit_of(k: &str) -> &'static str {
    match k {
        "angle" | "sig" | "helix" | "clearance" | "kr" => " °",
        "nof" => "",
        "max_rpm" => " rpm",
        _ => " mm",
    }
}

fn cite_detail(table: &Table, k: &str) -> String {
    match table.get("cite") {
        Some(Val::Tables(c)) => c
            .iter()
            .filter(|c| forge_tooling::toml::text(c, "path") == Some(k))
            .map(|c| {
                ["value", "source", "taken", "basis", "note"]
                    .iter()
                    .filter_map(|f| forge_tooling::toml::text(c, f))
                    .collect::<Vec<_>>()
                    .join(" · ")
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

/// The editor: every field of the record's table in its section, with its provenance.
pub fn editor(ui: &mut Ui, t: &Theme, st: &mut ToolingUi) {
    use super::lib_form::{self as lf, Prov};
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
        ui.add_space(8.0);
        ui.label(
            RichText::new(st.word.replace('_', " "))
                .font(sans(11.0))
                .color(t.ink2),
        );
    });
    let from = match &st.origin {
        Some((l, f)) => format!("{} level · {f}", l.name()),
        None => "not saved yet".into(),
    };
    ui.label(RichText::new(from).font(sans(10.5)).color(t.ink2));
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
    lf::legend(ui, t);
    ui.separator();
    // the table's keys, and the values it cites as unknown without holding them
    let mut keys: Vec<String> = st
        .table
        .keys()
        .filter(|k| !matches!(k.as_str(), "id" | "cite" | "real"))
        .cloned()
        .collect();
    if let Some(Val::Tables(c)) = st.table.get("cite") {
        for c in c {
            if let Some(p) = forge_tooling::toml::text(c, "path")
                && !p.contains('.')
                && !keys.iter().any(|k| k == p)
                && p != "record"
            {
                keys.push(p.to_string());
            }
        }
    }
    let mut edit: Option<(String, Val)> = None;
    let mut unknown: Option<String> = None;
    egui::ScrollArea::vertical()
        .id_salt("tooling:form")
        .show(ui, |ui| {
            for sec in ["Geometry", "Cutting", "Holder interface", "Source", "Other"] {
                let here: Vec<(&String, String)> = keys
                    .iter()
                    .filter_map(|k| {
                        let (l, g) = lf::tooling_label(k);
                        (g == sec).then_some((k, l))
                    })
                    .collect();
                if here.is_empty() {
                    continue;
                }
                lf::section(ui, t, sec);
                egui::Grid::new(("tooling:grid", sec))
                    .num_columns(3)
                    .spacing(egui::vec2(8.0, 4.0))
                    .striped(true)
                    .show(ui, |ui| {
                        for (k, label) in here {
                            lf::name(ui, t, &label, k, "", 118.0);
                            match st.table.get(k).cloned() {
                                None => {
                                    if lf::unknown_slot(ui, t, 140.0) {
                                        edit = Some((k.clone(), Val::Num(1.0)));
                                    }
                                }
                                Some(Val::Num(mut x)) => {
                                    let ch = if k == "nof" {
                                        lf::int_edit(ui, &mut x)
                                    } else {
                                        lf::num_edit(ui, &mut x, unit_of(k).trim())
                                    };
                                    if ch {
                                        edit = Some((k.clone(), Val::Num(x)));
                                    }
                                }
                                Some(Val::Str(s0)) if k == "type" && st.word == "tool" => {
                                    let mut s = s0.clone();
                                    egui::ComboBox::from_id_salt("tooling:type")
                                        .width(140.0)
                                        .selected_text(s.replace('_', " "))
                                        .show_ui(ui, |ui| {
                                            for w in TOOL_TYPES {
                                                ui.selectable_value(
                                                    &mut s,
                                                    w.to_string(),
                                                    w.replace('_', " "),
                                                );
                                            }
                                        });
                                    if s != s0 {
                                        edit = Some((k.clone(), Val::Str(s)));
                                    }
                                }
                                Some(Val::Str(mut s)) => {
                                    if ui
                                        .add_sized(
                                            egui::vec2(140.0, 20.0),
                                            egui::TextEdit::singleline(&mut s),
                                        )
                                        .on_hover_text(s.clone())
                                        .changed()
                                    {
                                        edit = Some((k.clone(), Val::Str(s)));
                                    }
                                }
                                Some(Val::Bool(mut b)) => {
                                    if ui.checkbox(&mut b, "").changed() {
                                        edit = Some((k.clone(), Val::Bool(b)));
                                    }
                                }
                                Some(Val::Arr(a)) => {
                                    let mut a2 = a.clone();
                                    let mut ch = false;
                                    egui::CollapsingHeader::new(
                                        RichText::new(format!("{} rows", a2.len())).font(sans(11.0)),
                                    )
                                    .id_salt(("tooling:rows", k))
                                    .show(ui, |ui| {
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
                                                        .add(
                                                            egui::DragValue::new(x)
                                                                .speed(0.05)
                                                                .max_decimals(4),
                                                        )
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
                                Some(Val::Table(_)) | Some(Val::Tables(_)) => {
                                    ui.label(RichText::new("kept as read").font(sans(10.5)).color(t.ink2));
                                }
                            }
                            if bookkeeping(k) {
                                ui.label("");
                            } else {
                                let held = st.table.contains_key(k);
                                let p = match dynamic::status_of(&st.table, k) {
                                    None if !held => Prov::Unknown,
                                    s => Prov::of(s.as_deref()),
                                };
                                if lf::chip(ui, t, p, &cite_detail(&st.table, k), held) {
                                    unknown = Some(k.clone());
                                }
                            }
                            ui.end_row();
                        }
                    });
            }
            if !st.findings.is_empty() {
                ui.separator();
                ui.label(RichText::new("Checks").font(semi(12.0)).color(t.ink));
                for f in &st.findings {
                    ui.label(
                        RichText::new(format!(
                            "PROBLEM {}: {} (limit {}): {}",
                            f.path, f.measured, f.limit, f.fix
                        ))
                        .font(sans(11.0))
                        .color(t.error),
                    );
                }
            }
        });
    if let Some((k, v)) = edit {
        if bookkeeping(&k) {
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

/// Flute lines on a cutter of radius `rc` over its fluted length: `n` of them, wound at the
/// helix angle (degrees; 0 = straight).
pub fn flutes(rc: f64, lcf: f64, n: u32, helix: f64) -> Vec<Vec<[f64; 3]>> {
    let (hs, hc) = forge_math::dmath::sin_cos(helix.to_radians());
    let tan = hs / hc;
    let r = rc * 1.004;
    (0..n)
        .map(|k| {
            let th0 = std::f64::consts::TAU * k as f64 / n as f64;
            (0..=40)
                .map(|i| {
                    let z = lcf * i as f64 / 40.0;
                    let th = th0 + z * tan / rc.max(1e-9);
                    let (sn, cs) = forge_math::dmath::sin_cos(th);
                    [r * cs, r * sn, z]
                })
                .collect()
        })
        .collect()
}
