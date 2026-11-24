//! The library builder's press-brake tabs (research/cam/library-builder.md): punches, dies and
//! presses, from forge-brake through forge-brake-builder, kept in a project's or the user's
//! `bend/tooling`. The same shape as the other tabs: the records by level with a search, a new
//! one from the template or derived from the chosen one, each field of its kind's form
//! (forge_brake::schema) with its provenance, the record read again by forge-brake after every
//! edit, the checks (an unpublished press value is "not checked", not a failure), save, and the
//! record's exact sections drawn along the bed.
//!
//! forge-brake's cites know published, estimate, unknown and conflict: a value the user enters
//! is cited `estimate` with the basis [`USER_VALUE`] (shown as the user's); "?" makes a value
//! unknown (removed, cited unknown with the basis [`USER_UNKNOWN`]).
use super::theme::{Theme, mono, sans, semi};
use egui::{RichText, Ui};
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
        recite(t, &f.path, "estimate", USER_VALUE);
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
        if let Ok(old) = std::fs::read_to_string(&file) {
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
        Some(
            secs.iter()
                .map(|(_, outline, len)| crate::machine_mesh::prism(outline, *len))
                .collect(),
        )
    }
}

fn kind_static(w: &str) -> &'static str {
    match w {
        "punch" => "punch",
        "die" => "die",
        _ => "press",
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
    egui::ScrollArea::vertical()
        .id_salt(("brake:list", st.word))
        .max_height((ui.available_height() - 220.0).max(80.0))
        .show(ui, |ui| {
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
                ui.label(
                    RichText::new(lv.name().to_uppercase())
                        .font(semi(10.0))
                        .color(t.ink2),
                );
                if here.is_empty() {
                    ui.label(RichText::new("  none").font(sans(11.0)).color(t.ink2));
                    continue;
                }
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
                            let r = ui.selectable_label(on, RichText::new(txt).font(sans(12.0)));
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

fn num_cell(ui: &mut Ui, x: &mut f64, suffix: &str) -> bool {
    ui.add(egui::DragValue::new(x).speed(0.1).max_decimals(4).suffix(suffix))
        .changed()
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
            let mut s2 = s.clone();
            let r = ui.add(egui::TextEdit::singleline(&mut s2).desired_width(170.0));
            (r.lost_focus() && s2 != *s).then_some(Item::Str(s2))
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
            let r = ui.add(egui::TextEdit::singleline(&mut s2).desired_width(170.0));
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
            ui.vertical(|ui| {
                let mut del = None;
                for (i, row) in rows.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        match row {
                            // segments: { length, count }
                            Item::Table(tb) => {
                                for (c, _) in cols.iter() {
                                    if let Some(Item::Num(x)) = tb.get(c) {
                                        let mut x = *x;
                                        if num_cell(ui, &mut x, "") {
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
                                        ch |= num_cell(ui, x, "");
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
        (_, None) => {
            let w = if f.cited { "UNKNOWN" } else { "not given" };
            ui.label(RichText::new(w).font(sans(11.0)).color(t.warn));
            None
        }
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

/// The editor: every field of the kind's form, with its provenance.
pub fn editor(ui: &mut Ui, t: &Theme, st: &mut BrakeUi) {
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
        ui.label(RichText::new(st.word).font(sans(11.0)).color(t.ink2));
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
            RichText::new("valid record (read by forge-brake)")
                .font(sans(11.0))
                .color(t.success),
        ),
    };
    if st.problem.is_none()
        && let Err(f) = forge_brake_builder::preview(&cur)
    {
        ui.label(
            RichText::new(format!("no section drawn: {}", f.what))
                .font(sans(11.0))
                .color(t.warn),
        );
    }
    ui.separator();
    let fields = forge_brake_builder::fields(st.word);
    let mut groups: Vec<&'static str> = Vec::new();
    for f in &fields {
        if f.path != "id" && !groups.contains(&f.group) {
            groups.push(f.group);
        }
    }
    let mut change: Option<Change> = None;
    egui::ScrollArea::vertical().id_salt("brake:form").show(ui, |ui| {
        for g in &groups {
            let g = if g.is_empty() { "Other" } else { g };
            ui.label(RichText::new(g.to_uppercase()).font(semi(10.0)).color(t.ink2));
            egui::Grid::new(("brake:grid", g))
                .num_columns(3)
                .striped(true)
                .show(ui, |ui| {
                    for f in fields
                        .iter()
                        .filter(|f| f.path != "id" && (f.group == g || (f.group.is_empty() && g == "Other")))
                    {
                        ui.label(RichText::new(&f.path).font(mono(11.5)).color(t.ink2))
                            .on_hover_text(f.help);
                        let v = get(&st.table, &f.path);
                        if let Some(nv) = cell(ui, t, f, v) {
                            change = Some(Change::Set(f.clone(), nv));
                        }
                        ui.horizontal(|ui| {
                            if v.is_none()
                                && ui
                                    .small_button("set")
                                    .on_hover_text("enter this value (yours)")
                                    .clicked()
                            {
                                change = Some(Change::Set(f.clone(), first_value(st.word, f)));
                            }
                            if f.cited {
                                let (w, col) = badge(t, status_of(&st.table, &f.path).as_deref());
                                let w = if v.is_none() && w == "uncited" {
                                    "not given".to_string()
                                } else {
                                    w
                                };
                                ui.label(RichText::new(w).font(sans(10.0)).color(col));
                                if v.is_some()
                                    && ui
                                        .small_button("?")
                                        .on_hover_text("I don't know this value: mark it unknown")
                                        .clicked()
                                {
                                    change = Some(Change::Unknown(f.path.clone()));
                                }
                            }
                        });
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
