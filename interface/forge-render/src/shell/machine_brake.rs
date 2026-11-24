//! The library builder's press-brake tabs (research/cam/library-builder.md): punches, dies and
//! presses, from forge-brake through forge-brake-builder, kept in a project's or the user's
//! `bend/tooling`. The same shape as the other tabs: the records by level with a search, a new
//! one from the template or derived from the chosen one, each field of its kind's form
//! (forge_brake::schema) with its provenance, the record read again by forge-brake after every
//! edit, the checks (an unpublished press value is "not checked", not a failure), save, and the
//! record's exact sections drawn along the bed.
//!
//! forge-brake's cites know published, estimate, user, unknown and conflict: a value the user enters
//! is cited `user` with the basis [`USER_VALUE`] (and the record is no longer `real`); "?" makes a value
//! unknown (removed, cited unknown with the basis [`USER_UNKNOWN`]).
use super::theme::{Theme, sans, semi};
use egui::{RichText, Ui, vec2};
use forge_brake_builder::{Dies, Finding, Presses, Punches, Rec};
use forge_library::schema::{Field, Ty};
use forge_library::{Catalog, Entry, Kind, Level};
use forge_vendor::toml::{Item, Table};
use std::path::{Path, PathBuf};

/// The basis of a value the user entered.
pub const USER_VALUE: &str =
    "the user's own value, entered in forgecad's library builder (not a maker's published number)";
/// The basis of a value the user marked unknown.
pub const USER_UNKNOWN: &str = "not known: marked unknown in forgecad's library builder";

/// Is `word` one of the press-brake kinds?
pub fn is_brake(word: &str) -> bool {
    forge_brake_builder::KINDS.iter().any(|(w, _)| *w == word)
}

type Errors = Vec<(Level, String, String)>;

fn load_k<K: Kind<Record = Rec>>(project: Option<&Path>) -> (Vec<Entry<Rec>>, Errors) {
    let user = forge_library::user_dir::<K>();
    let c = Catalog::<K>::load(project, user.as_deref());
    (c.entries, c.errors)
}

/// The catalog of a kind: the project's, the user's and the built-in records.
pub fn load(word: &str, project: Option<&Path>) -> (Vec<Entry<Rec>>, Errors) {
    match word {
        "punch" => load_k::<Punches>(project),
        "die" => load_k::<Dies>(project),
        _ => load_k::<Presses>(project),
    }
}

fn dir_k<K: Kind>(level: Level, project: Option<&Path>) -> Option<PathBuf> {
    match level {
        Level::Project => project.map(forge_library::project_dir::<K>),
        Level::User => forge_library::user_dir::<K>(),
        Level::Builtin => None,
    }
}

/// The folder a level keeps a kind in.
pub fn level_dir(word: &str, level: Level, project: Option<&Path>) -> Option<PathBuf> {
    match word {
        "punch" => dir_k::<Punches>(level, project),
        "die" => dir_k::<Dies>(level, project),
        _ => dir_k::<Presses>(level, project),
    }
}

fn save_k<K: Kind<Record = Rec>>(
    r: &Rec,
    dir: &Path,
    level: Level,
    overwrite: bool,
) -> Result<PathBuf, String> {
    forge_library::save::<K>(r, &[], dir, level, overwrite)
}

/// The value at a dotted path (`relief.depth`: the inline table `relief`, its `depth`).
pub fn get<'a>(t: &'a Table, path: &str) -> Option<&'a Item> {
    match path.split_once('.') {
        None => t.get(path),
        Some((head, rest)) => match t.get(head) {
            Some(Item::Table(inner)) => get(inner, rest),
            _ => None,
        },
    }
}

/// Sets (or with None removes) the value at a dotted path; an inline table left empty goes.
pub fn put(t: &mut Table, path: &str, v: Option<Item>) {
    match path.split_once('.') {
        None => match v {
            Some(v) => t.set(path, v),
            None => t.0.retain(|(k, _)| k != path),
        },
        Some((head, rest)) => {
            let mut inner = match t.get(head) {
                Some(Item::Table(x)) => x.clone(),
                _ => Table::default(),
            };
            put(&mut inner, rest, v);
            if inner.0.is_empty() {
                t.0.retain(|(k, _)| k != head);
            } else {
                t.set(head, Item::Table(inner));
            }
        }
    }
}

fn cites(t: &Table) -> Vec<Table> {
    match t.get("cite") {
        Some(Item::Tables(c)) => c.clone(),
        _ => Vec::new(),
    }
}

fn text<'a>(t: &'a Table, k: &str) -> Option<&'a str> {
    match t.get(k) {
        Some(Item::Str(s)) => Some(s),
        _ => None,
    }
}

/// The path's cites replaced by one (status, basis); the cites kept last, as written.
pub fn recite(t: &mut Table, path: &str, status: &str, basis: &str) {
    let mut c = cites(t);
    c.retain(|r| text(r, "path") != Some(path));
    let mut row = Table::default();
    row.set("path", Item::Str(path.into()));
    row.set("status", Item::Str(status.into()));
    row.set("basis", Item::Str(basis.into()));
    c.push(row);
    t.0.retain(|(k, _)| k != "cite");
    t.set("cite", Item::Tables(c));
}

/// A value set as the user's own.
pub fn set_user(t: &mut Table, f: &Field, v: Item) {
    put(t, &f.path, Some(v));
    if f.cited {
        recite(t, &f.path, "user", USER_VALUE);
        // a record holding a user's value is not a maker's published record (forge-brake)
        put(t, "real", Some(Item::Bool(false)));
        if let Some(i) = t.0.iter().position(|(k, _)| k == "cite") {
            let c = t.0.remove(i);
            t.0.push(c);
        }
    } else {
        // keep the cites last
        if let Some(i) = t.0.iter().position(|(k, _)| k == "cite") {
            let c = t.0.remove(i);
            t.0.push(c);
        }
    }
}

/// A value the user does not know: removed, cited unknown.
pub fn set_unknown(t: &mut Table, path: &str) {
    put(t, path, None);
    recite(t, path, "unknown", USER_UNKNOWN);
}

/// A field's provenance: (word, is it the user's).
pub fn status_of(t: &Table, path: &str) -> Option<String> {
    let c = cites(t);
    let mine: Vec<&Table> = c.iter().filter(|r| text(r, "path") == Some(path)).collect();
    if mine.is_empty() {
        return None;
    }
    if mine.iter().any(|r| text(r, "basis") == Some(USER_VALUE)) {
        return Some("user".into());
    }
    // the weakest, as forge-brake reads it
    for s in ["conflict", "unknown", "estimate", "published"] {
        if mine.iter().any(|r| text(r, "status") == Some(s)) {
            return Some(s.into());
        }
    }
    None
}

pub struct BrakeUi {
    /// "punch", "die" or "press".
    pub word: &'static str,
    pub entries: Vec<Entry<Rec>>,
    pub errors: Errors,
    gen_seen: u64,
    /// The record as last read by forge-brake, and its table as edited (may not read).
    pub cur: Option<Rec>,
    pub table: Table,
    pub origin: Option<(Level, String)>,
    pub changed: bool,
    /// forge-brake's refusal of the edited table, if any.
    pub problem: Option<String>,
    pub findings: Vec<Finding>,
    pub checked: bool,
    thumbs: std::collections::HashMap<String, Vec<Vec<[f64; 2]>>>,
    pub new_id: String,
    pub filter: String,
    pub message: String,
}

impl BrakeUi {
    pub fn new(word: &'static str, project: Option<&Path>) -> BrakeUi {
        let mut b = BrakeUi {
            word,
            entries: Vec::new(),
            errors: Vec::new(),
            gen_seen: u64::MAX,
            cur: None,
            table: Table::default(),
            origin: None,
            changed: false,
            problem: None,
            findings: Vec::new(),
            checked: false,
            thumbs: Default::default(),
            new_id: format!("my-{word}"),
            filter: String::new(),
            message: String::new(),
        };
        b.reload(project);
        if let Some(e) = b.entries.first().cloned() {
            b.pick(e);
        }
        b
    }

    pub fn reload(&mut self, project: Option<&Path>) {
        let (e, errs) = load(self.word, project);
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
        self.checked = false;
    }

    fn start(&mut self, mut table: Table, from: &str) {
        if let Err(e) = forge_library::check_id(&self.new_id) {
            self.message = e;
            return;
        }
        if self.entries.iter().any(|e| e.def.id == self.new_id) {
            self.message = format!("`{}` is taken: choose another id", self.new_id);
            return;
        }
        table.set("id", Item::Str(self.new_id.clone()));
        self.table = table;
        self.origin = None;
        self.cur = None;
        self.reread();
        self.message = match &self.problem {
            None => format!(
                "New {} `{}` from {from}: its values keep their cites",
                self.word, self.new_id
            ),
            Some(p) => format!("New {} `{}` from {from}: {p}", self.word, self.new_id),
        };
    }

    /// A new record from the chosen one (its values and their cites copied).
    pub fn derive(&mut self) {
        let Some(c) = self.cur.clone() else { return };
        self.start(c.table, &format!("`{}`", c.id));
    }

    /// A new record from the kind's template (forgecad's stand-in numbers, `real = false`).
    pub fn from_template(&mut self) {
        let Some(t) = forge_brake_builder::templates()
            .into_iter()
            .find(|r| r.kind == self.word)
        else {
            self.message = format!("no {} template", self.word);
            return;
        };
        self.start(
            t.table,
            "the template (stand-in numbers: replace each with the maker's)",
        );
    }

    /// After an edit of `table`: read it again with forge-brake's rules.
    pub fn reread(&mut self) {
        let id = text(&self.table, "id").unwrap_or_default().to_string();
        let r = Rec {
            kind: kind_static(self.word),
            id,
            table: self.table.clone(),
        };
        match forge_brake_builder::write_kind(self.word, "", std::slice::from_ref(&r)) {
            Ok(_) => {
                self.cur = Some(r);
                self.problem = None;
            }
            Err(e) => self.problem = Some(e),
        }
        self.changed = true;
        self.findings.clear();
        self.checked = false;
    }

    pub fn check(&mut self) {
        let Some(r) = &self.cur else { return };
        self.findings = forge_brake_builder::check(r);
        self.checked = true;
        let bad = self.findings.iter().filter(|f| !f.not_checked).count();
        let nc = self.findings.len() - bad;
        self.message = format!(
            "{} `{}`: {bad} problems, {nc} not checked (unpublished)",
            self.word, r.id
        );
    }

    pub fn save(&mut self, level: Level, project: Option<&Path>) {
        if let Some(p) = &self.problem {
            self.message = format!("Not saved: {p}");
            return;
        }
        let Some(r) = self.cur.clone() else { return };
        let Some(dir) = level_dir(self.word, level, project) else {
            self.message = "Not saved: no folder for that level (open a part for its project)".into();
            return;
        };
        let file = forge_library::file_for(&dir, &r.id);
        // punches, dies and presses share the folder: never write over another kind's records
        if let Ok(old) = forge_dirs::fs::read_to_string(&file) {
            let others = forge_brake_builder::KINDS
                .iter()
                .filter(|(w, _)| *w != self.word)
                .any(|(w, _)| {
                    forge_brake_builder::parse_kind(kind_static(w), &old).is_ok_and(|v| !v.is_empty())
                });
            if others {
                self.message = format!(
                    "Not saved: {} also holds other press-brake records; save under another id",
                    file.display()
                );
                return;
            }
        }
        let same = matches!(&self.origin, Some((l, f)) if *l == level && Path::new(f) == file);
        let got = match self.word {
            "punch" => save_k::<Punches>(&r, &dir, level, same),
            "die" => save_k::<Dies>(&r, &dir, level, same),
            _ => save_k::<Presses>(&r, &dir, level, same),
        };
        match got {
            Ok(f) => {
                self.message = format!("Saved {} ({} level)", f.display(), level.name());
                self.origin = Some((level, f.display().to_string()));
                self.changed = false;
                self.reload(project);
            }
            Err(e) => self.message = format!("Not saved: {e}"),
        }
    }

    /// The preview: each exact section drawn along the bed.
    pub fn preview(&self) -> Option<Vec<crate::machine_mesh::Tris>> {
        let secs = forge_brake_builder::preview(self.cur.as_ref()?).ok()?;
        Some(prisms(&secs))
    }

    /// The preview from the record: a punch or die's exact section swept along each of its
    /// segments, with its section and dimensions for the inset; a press's bed, ram and
    /// housings; what the record does not give as ghosts.
    pub fn library_preview(&self) -> Option<crate::library_mesh::Preview> {
        use crate::library_mesh as lm;
        let r = self.cur.as_ref()?;
        if r.kind == "press" {
            return Some(lm::press(&r.table));
        }
        let num = |k: &str| match get(&r.table, k) {
            Some(Item::Num(x)) => Some(*x),
            _ => None,
        };
        match forge_brake_builder::preview(r) {
            Ok(secs) => {
                let segs: Vec<(f64, u32)> = match get(&r.table, "segments") {
                    Some(Item::Arr(rows)) => rows
                        .iter()
                        .filter_map(|x| match x {
                            Item::Table(t) => match (t.get("length"), t.get("count")) {
                                (Some(Item::Num(l)), Some(Item::Num(c))) => Some((*l, *c as u32)),
                                _ => None,
                            },
                            _ => None,
                        })
                        .collect(),
                    _ => Vec::new(),
                };
                let mut p = lm::brake_tool(&secs, &segs);
                let outline: Vec<[f64; 2]> = secs.iter().flat_map(|s| s.1.iter().copied()).collect();
                let keys: &[(&str, &str, &str)] = if r.kind == "die" {
                    &[
                        ("V", "v", "mm"),
                        ("angle", "angle", "°"),
                        ("radius", "shoulder_radius", "mm"),
                        ("height", "height", "mm"),
                    ]
                } else {
                    &[
                        ("height", "height", "mm"),
                        ("tip radius", "tip_radius", "mm"),
                        ("tip angle", "tip_angle", "°"),
                    ]
                };
                let dims = keys
                    .iter()
                    .map(|(l, k, u)| {
                        (
                            l.to_string(),
                            num(k).map_or("unknown".into(), |x| {
                                format!("{} {u}", forge_vendor::toml::num(x))
                            }),
                        )
                    })
                    .collect();
                p.section = Some((outline, dims));
                Some(p)
            }
            Err(f) => {
                // no exact section: the envelope the record does give, as a ghost
                let h = num("height").unwrap_or(100.0);
                let w = num("width").unwrap_or(h * 0.3);
                let mut p = lm::Preview::default();
                p.ghosts.push(lm::Ghost {
                    bounds: Some(([-w / 2.0, 0.0, 0.0], [w / 2.0, h * 2.0, h])),
                    edges: Vec::new(),
                    at: [w / 2.0, h, h],
                    label: format!(
                        "no section: {}",
                        f.what.trim_start_matches("unknown: ").to_string() + " · unknown"
                    ),
                });
                Some(p)
            }
        }
    }

    /// When the record's working geometry is not all known: its section drawn from the values
    /// it holds and the template's stand-ins for the rest (to be shown as a ghost), and the
    /// values stood in.
    pub fn preview_ghost(&self) -> Option<(Vec<crate::machine_mesh::Tris>, Vec<String>)> {
        let r = self.cur.as_ref()?;
        let filled = stand_in_for_unknown(r);
        let secs = forge_brake_builder::preview(&filled.0).ok()?;
        Some((prisms(&secs), filled.1))
    }

    /// The thumbnail outlines of a record (cached by id).
    pub fn thumb(&mut self, r: &Rec) -> Vec<Vec<[f64; 2]>> {
        if let Some(v) = self.thumbs.get(&r.id) {
            return v.clone();
        }
        let secs = forge_brake_builder::preview(r)
            .or_else(|_| forge_brake_builder::preview(&stand_in_for_unknown(r).0))
            .unwrap_or_default();
        // a press's blocks are far larger than its tools: its housing and beams as they are
        let v: Vec<Vec<[f64; 2]>> = secs.into_iter().map(|(_, o, _)| o).collect();
        self.thumbs.insert(r.id.clone(), v.clone());
        v
    }
}

/// The sections drawn along the bed: over their length, but no longer than four times the
/// largest section's size, so a long tool's section still reads in the view.
fn prisms(secs: &[forge_brake_builder::Section]) -> Vec<crate::machine_mesh::Tris> {
    let size = secs.iter().flat_map(|(_, o, _)| o.iter()).fold(
        (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY),
        |a, p| (a.0.min(p[0]), a.1.max(p[0]), a.2.min(p[1]), a.3.max(p[1])),
    );
    let span = (size.1 - size.0).max(size.3 - size.2);
    let cap = if span.is_finite() {
        4.0 * span
    } else {
        f64::INFINITY
    };
    secs.iter()
        .map(|(_, outline, len)| crate::machine_mesh::prism(outline, len.min(cap)))
        .collect()
}

/// The record with its unknown cited fields given the template's values (the record's cites
/// left as they are, `real = false`), and the fields stood in.
pub fn stand_in_for_unknown(r: &Rec) -> (Rec, Vec<String>) {
    let mut table = r.table.clone();
    let mut stood = Vec::new();
    let tpl = forge_brake_builder::templates()
        .into_iter()
        .find(|x| x.kind == r.kind);
    for f in forge_brake_builder::fields(r.kind) {
        if f.cited
            && get(&table, &f.path).is_none()
            && let Some(v) = tpl.as_ref().and_then(|t| get(&t.table, &f.path).cloned())
        {
            put(&mut table, &f.path, Some(v));
            stood.push(f.path.clone());
        }
    }
    // stand-ins are not a maker's values: the record is read as forgecad's own
    table.set("real", Item::Bool(false));
    let mut c = cites(&table);
    c.retain(|row| !stood.iter().any(|p| text(row, "path") == Some(p.as_str())));
    table.0.retain(|(k, _)| k != "cite");
    if !c.is_empty() {
        table.set("cite", Item::Tables(c));
    }
    (
        Rec {
            kind: r.kind,
            id: r.id.clone(),
            table,
        },
        stood,
    )
}

fn kind_static(w: &str) -> &'static str {
    match w {
        "punch" => "punch",
        "die" => "die",
        _ => "press",
    }
}

/// The library list of a press-brake kind.
pub fn library(
    ui: &mut Ui,
    t: &Theme,
    st: &mut BrakeUi,
    project: Option<&Path>,
    places: &mut Vec<(String, egui::Rect)>,
) {
    if st.gen_seen != forge_library::generation() {
        st.reload(project);
    }
    let r = ui.add(
        egui::TextEdit::singleline(&mut st.filter)
            .hint_text("search: id, maker, file")
            .desired_width(ui.available_width()),
    );
    places.push(("brake:search".into(), r.rect));
    let want = st.filter.trim().to_lowercase();
    let mut chosen = None;
    // the thumbnails, worked out once per record
    let all: Vec<Rec> = st.entries.iter().map(|e| e.def.clone()).collect();
    let thumbs: std::collections::HashMap<String, Vec<Vec<[f64; 2]>>> =
        all.iter().map(|r| (r.id.clone(), st.thumb(r))).collect();
    egui::ScrollArea::vertical()
        .id_salt(("brake:list", st.word))
        .max_height((ui.available_height() - 220.0).max(80.0))
        .show(ui, |ui| {
            super::lib_form::empty_levels(ui, t, st.entries.iter().any(|e| e.level != Level::Builtin));
            for lv in [Level::Project, Level::User, Level::Builtin] {
                let here: Vec<&Entry<Rec>> = st
                    .entries
                    .iter()
                    .filter(|e| e.level == lv)
                    .filter(|e| {
                        want.is_empty()
                            || [
                                e.def.id.as_str(),
                                text(&e.def.table, "maker").unwrap_or(""),
                                e.file.as_str(),
                            ]
                            .iter()
                            .any(|x| x.to_lowercase().contains(&want))
                    })
                    .collect();
                if here.is_empty() {
                    continue;
                }
                super::lib_form::level_head(ui, t, lv.name(), here.len());
                let mut makers: Vec<&str> = here
                    .iter()
                    .map(|e| text(&e.def.table, "maker").unwrap_or("?"))
                    .collect();
                makers.sort();
                makers.dedup();
                for mk in makers {
                    let inner: Vec<&&Entry<Rec>> = here
                        .iter()
                        .filter(|e| text(&e.def.table, "maker").unwrap_or("?") == mk)
                        .collect();
                    egui::CollapsingHeader::new(
                        RichText::new(format!("{mk} ({})", inner.len())).font(sans(11.0)),
                    )
                    .id_salt(("brake:group", st.word, lv.name(), mk))
                    .default_open(!want.is_empty() || inner.len() <= 6)
                    .show(ui, |ui| {
                        for e in inner {
                            let on = st.cur.as_ref().is_some_and(|c| c.id == e.def.id)
                                && st.origin.as_ref().is_some_and(|(l, _)| *l == e.level);
                            let mut txt = e.def.id.clone();
                            if !matches!(e.def.table.get("real"), Some(Item::Bool(true))) {
                                txt.push_str("  · stand-in");
                            }
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
                            places.push((format!("brake:pick:{}", e.def.id), r.rect));
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
    ui.label(
        RichText::new(format!("New {}", st.word))
            .font(semi(12.0))
            .color(t.ink),
    );
    ui.horizontal(|ui| {
        ui.label(RichText::new("Id").font(sans(12.0)).color(t.ink2));
        let r = ui.add(egui::TextEdit::singleline(&mut st.new_id).desired_width(150.0));
        places.push(("brake:id".into(), r.rect));
    });
    ui.horizontal(|ui| {
        let r = ui.button("From the template");
        places.push(("brake:template".into(), r.rect));
        if r.clicked() {
            st.from_template();
        }
        let r = ui.add_enabled(st.cur.is_some(), egui::Button::new("Derive from this one"));
        places.push(("brake:derive".into(), r.rect));
        if r.clicked() {
            st.derive();
        }
    });
}

/// What the form changed.
enum Change {
    Set(Field, Item),
    Unknown(String),
}

fn num_cell(ui: &mut Ui, x: &mut f64, unit: &str) -> bool {
    super::lib_form::num_edit(ui, x, unit.trim())
}

/// One field's value cell; Some(new value) when edited.
fn cell(ui: &mut Ui, t: &Theme, f: &Field, v: Option<&Item>) -> Option<Item> {
    let sfx = match f.unit.suffix() {
        "" => String::new(),
        s => format!(" {s}"),
    };
    match (&f.ty, v) {
        (Ty::Num | Ty::Int, Some(Item::Num(x))) => {
            let mut x = *x;
            num_cell(ui, &mut x, &sfx).then_some(Item::Num(x))
        }
        (Ty::Text, Some(Item::Str(s))) => {
            super::lib_form::text_cell(ui, t, ("brake:text", &f.path), s, 140.0).map(Item::Str)
        }
        (Ty::Text, Some(Item::Arr(a))) => {
            // a list of words, comma separated
            let joined: Vec<String> = a
                .iter()
                .filter_map(|x| match x {
                    Item::Str(s) => Some(s.clone()),
                    _ => None,
                })
                .collect();
            let was = joined.join(", ");
            let mut s2 = was.clone();
            let r = ui.add(egui::TextEdit::singleline(&mut s2).desired_width(160.0));
            (r.lost_focus() && s2 != was).then(|| {
                Item::Arr(
                    s2.split(',')
                        .map(str::trim)
                        .filter(|w| !w.is_empty())
                        .map(|w| Item::Str(w.into()))
                        .collect(),
                )
            })
        }
        (Ty::Choice(ch), Some(Item::Str(s))) => {
            let mut pick = s.clone();
            egui::ComboBox::from_id_salt(("brake:choice", &f.path))
                .width(140.0)
                .selected_text(pick.clone())
                .show_ui(ui, |ui| {
                    for c in *ch {
                        ui.selectable_value(&mut pick, c.to_string(), *c);
                    }
                });
            (pick != *s).then_some(Item::Str(pick))
        }
        (Ty::Bool, Some(Item::Bool(b))) => {
            let mut b2 = *b;
            ui.checkbox(&mut b2, "").changed().then_some(Item::Bool(b2))
        }
        (Ty::Range, Some(Item::Arr(a))) => {
            let mut a2 = a.clone();
            let mut ch = false;
            ui.horizontal(|ui| {
                for x in a2.iter_mut() {
                    if let Item::Num(x) = x {
                        ch |= num_cell(ui, x, &sfx);
                    }
                }
            });
            ch.then_some(Item::Arr(a2))
        }
        (Ty::Points(cols), Some(Item::Arr(rows))) => {
            let mut rows = rows.clone();
            let mut ch = false;
            let head = format!(
                "{} rows: {}",
                rows.len(),
                rows.iter()
                    .take(4)
                    .map(|r| match r {
                        Item::Table(tb) => cols
                            .iter()
                            .filter_map(|(c, _)| match tb.get(c) {
                                Some(Item::Num(x)) => Some(forge_vendor::toml::num(*x)),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join("×"),
                        Item::Arr(xs) => xs
                            .iter()
                            .filter_map(|x| match x {
                                Item::Num(x) => Some(forge_vendor::toml::num(*x)),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join(", "),
                        _ => String::new(),
                    })
                    .collect::<Vec<_>>()
                    .join("; ")
                    + if rows.len() > 4 { "; …" } else { "" }
            );
            egui::CollapsingHeader::new(RichText::new(head).font(sans(11.0)))
                .id_salt(("brake:rows", &f.path))
                .show(ui, |ui| {
                    let mut del = None;
                    for (i, row) in rows.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            match row {
                                // segments: { length, count }
                                Item::Table(tb) => {
                                    for (c, u) in cols.iter() {
                                        if let Some(Item::Num(x)) = tb.get(c) {
                                            let mut x = *x;
                                            let edited = if *u == forge_library::schema::Unit::None {
                                                super::lib_form::int_edit(ui, &mut x)
                                            } else {
                                                num_cell(ui, &mut x, u.suffix())
                                            };
                                            if edited {
                                                tb.set(c, Item::Num(x));
                                                ch = true;
                                            }
                                        }
                                    }
                                }
                                // an outline's [x, z]
                                Item::Arr(xs) => {
                                    for x in xs.iter_mut() {
                                        if let Item::Num(x) = x {
                                            ch |= num_cell(ui, x, "mm");
                                        }
                                    }
                                }
                                _ => {}
                            }
                            if ui.small_button("x").clicked() {
                                del = Some(i);
                            }
                        });
                    }
                    if let Some(i) = del {
                        rows.remove(i);
                        ch = true;
                    }
                    if let Some(last) = rows.last().cloned()
                        && ui.small_button("+").clicked()
                    {
                        rows.push(last);
                        ch = true;
                    }
                });
            ch.then_some(Item::Arr(rows))
        }
        (_, None) => None,
        (_, Some(_)) => {
            ui.label(RichText::new("(kept as read)").font(sans(10.0)).color(t.ink2));
            None
        }
    }
}

/// A first value for a field the record does not hold: the template's, else a plain one.
fn first_value(word: &str, f: &Field) -> Item {
    if let Some(v) = forge_brake_builder::templates()
        .iter()
        .find(|r| r.kind == word)
        .and_then(|r| get(&r.table, &f.path).cloned())
    {
        return v;
    }
    match &f.ty {
        Ty::Text => Item::Str(String::new()),
        Ty::Choice(c) => Item::Str(c.first().copied().unwrap_or("").into()),
        Ty::Bool => Item::Bool(false),
        Ty::Range => Item::Arr(vec![Item::Num(0.0), Item::Num(1.0)]),
        Ty::Points(cols) if cols.first().is_some_and(|c| c.0 == "length") => {
            let mut tb = Table::default();
            tb.set("length", Item::Num(100.0));
            tb.set("count", Item::Num(1.0));
            Item::Arr(vec![Item::Table(tb)])
        }
        Ty::Points(_) => Item::Arr(vec![
            Item::Arr(vec![Item::Num(0.0), Item::Num(0.0)]),
            Item::Arr(vec![Item::Num(10.0), Item::Num(0.0)]),
            Item::Arr(vec![Item::Num(0.0), Item::Num(10.0)]),
        ]),
        _ => Item::Num(1.0),
    }
}

/// A cite's words for the chip's menu: its source and date, or its basis.
fn cite_detail(t: &Table, path: &str) -> String {
    cites(t)
        .iter()
        .filter(|r| text(r, "path") == Some(path))
        .map(|r| {
            let mut v: Vec<&str> = Vec::new();
            for k in ["value", "source", "taken", "basis", "note"] {
                if let Some(x) = text(r, k) {
                    v.push(x);
                }
            }
            v.join(" · ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The sections of the form, in order.
const SECTIONS: &[&str] = &[
    "Working geometry",
    "Tooling interface",
    "Rating and lengths",
    "Frame",
    "Speeds",
    "Backgauge",
    "Source",
];

/// The editor: every field of the kind's form, with its provenance.
pub fn editor(ui: &mut Ui, t: &Theme, st: &mut BrakeUi) {
    use super::lib_form::{self as lf, Prov};
    let Some(cur) = st.cur.clone() else {
        ui.label(
            RichText::new("No record: pick one")
                .font(sans(11.0))
                .color(t.ink2),
        );
        return;
    };
    let title = forge_brake_builder::KINDS
        .iter()
        .find(|(w, _)| *w == st.word)
        .map_or(st.word, |(_, t)| *t);
    ui.horizontal(|ui| {
        ui.label(RichText::new(&cur.id).font(semi(14.0)).color(t.ink));
        ui.add_space(8.0);
        ui.label(RichText::new(title).font(sans(11.0)).color(t.ink2));
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
            RichText::new("valid record (read by forge-brake)")
                .font(sans(11.0))
                .color(t.success),
        ),
    };
    if st.problem.is_none() && forge_brake_builder::preview(&cur).is_err() {
        let msg = match st.preview_ghost() {
            Some((_, stood)) => format!(
                "drawn in amber: the template's stand-in values for the unknown {}",
                stood.join(", ")
            ),
            None => "no section: its working geometry is not known".into(),
        };
        ui.label(RichText::new(msg).font(sans(11.0)).color(t.warn));
    }
    lf::legend(ui, t);
    ui.separator();
    let real = matches!(st.table.get("real"), Some(Item::Bool(true)));
    let fields = forge_brake_builder::fields(st.word);
    let mut change: Option<Change> = None;
    egui::ScrollArea::vertical().id_salt("brake:form").show(ui, |ui| {
        for sec in SECTIONS {
            let here: Vec<(&Field, String)> = fields
                .iter()
                .filter(|f| f.path != "id")
                .filter_map(|f| {
                    let (l, s) = lf::brake_label(&f.path, f.group);
                    (s == *sec).then_some((f, l))
                })
                .collect();
            if here.is_empty() {
                continue;
            }
            lf::section(ui, t, sec);
            egui::Grid::new(("brake:grid", *sec))
                .num_columns(3)
                .spacing(vec2(8.0, 4.0))
                .striped(true)
                .show(ui, |ui| {
                    for (f, label) in here {
                        lf::name(ui, t, &label, &f.path, f.help, 118.0);
                        let v = get(&st.table, &f.path);
                        match v {
                            Some(v) => {
                                if let Some(nv) = cell(ui, t, f, Some(v)) {
                                    change = Some(Change::Set(f.clone(), nv));
                                }
                            }
                            None if f.cited => {
                                if lf::unknown_slot(ui, t, 140.0) {
                                    change = Some(Change::Set(f.clone(), first_value(st.word, f)));
                                }
                            }
                            None => {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("not given · add").font(sans(11.0)).color(t.ink2),
                                        )
                                        .min_size(vec2(140.0, 18.0)),
                                    )
                                    .clicked()
                                {
                                    change = Some(Change::Set(f.clone(), first_value(st.word, f)));
                                }
                            }
                        }
                        if f.cited {
                            let p = match status_of(&st.table, &f.path) {
                                None if v.is_none() => Prov::Unknown,
                                None if !real => Prov::StandIn,
                                s => Prov::of(s.as_deref()),
                            };
                            if lf::chip(ui, t, p, &cite_detail(&st.table, &f.path), v.is_some()) {
                                change = Some(Change::Unknown(f.path.clone()));
                            }
                        } else {
                            ui.label("");
                        }
                        ui.end_row();
                    }
                });
        }
        if st.checked {
            ui.separator();
            ui.label(RichText::new("Checks").font(semi(12.0)).color(t.ink));
            if st.findings.is_empty() {
                ui.label(
                    RichText::new("no problem found")
                        .font(sans(11.0))
                        .color(t.success),
                );
            }
            for f in &st.findings {
                let (tag, col) = if f.not_checked {
                    ("NOT CHECKED", t.warn)
                } else {
                    ("PROBLEM", t.error)
                };
                ui.label(
                    RichText::new(format!("{tag} {}: {} — {}", f.path, f.what, f.fix))
                        .font(sans(11.0))
                        .color(col),
                );
            }
        }
    });
    match change {
        Some(Change::Set(f, v)) => {
            set_user(&mut st.table, &f, v);
            st.reread();
        }
        Some(Change::Unknown(p)) => {
            set_unknown(&mut st.table, &p);
            st.reread();
        }
        None => {}
    }
}
