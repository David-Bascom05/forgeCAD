//! The Versions tab, beside Model and Agent: Onshape's version graph. Named versions down one
//! lane, newest first, each with its saves folded under it; the selected row's Open, Restore,
//! Compare and Copy ID; Create Version and Release at the top; and, while comparing, what
//! changed in features and parameters. The host fills it and does what `VersionsAct`s ask.

use super::Shell;
use super::theme::{Theme, mono, sans, semi};
use egui::{Align2, Color32, Rect, Sense, Stroke, Ui, pos2, vec2};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq)]
pub struct SaveRow {
    pub save: String,
    pub time: i64,
    pub author: String,
    pub summary: String,
    pub agent: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// The save it marks (a version's), or "" for the work since the newest version.
    pub save: String,
    pub path: String,
    pub n: Option<u32>,
    pub title: String,
    pub notes: String,
    pub author: String,
    pub time: i64,
    pub agent: bool,
    /// A release's revision letter, if this version is released.
    pub revision: Option<String>,
    /// The saves that led to it from the version before (its own first), newest first.
    pub saves: Vec<SaveRow>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DiffRow {
    pub group: &'static str,
    pub name: String,
    pub change: &'static str,
    pub old: String,
    pub new: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Filter {
    #[default]
    Part,
    Project,
    Releases,
}

/// The ribbon's way into the Versions tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VersionsCmd {
    Show,
    Create,
    CompareCurrent,
    Release,
    UpdateAll,
}

#[derive(Clone, Debug, PartialEq)]
pub enum VersionsAct {
    /// Read the graph again.
    Load,
    /// Start keeping history in the document's folder.
    TurnOn,
    /// The document's versions, or a part's it places.
    Show(PathBuf),
    Create {
        name: String,
        notes: String,
    },
    Open(String),
    Restore(String),
    /// Base, target (a save id, or "current").
    Compare(String, String),
    /// Back to the model, comparing no more.
    EndCompare,
    CopyId(String),
    /// A pinned part (None: every one) to its newest version.
    Update(Option<String>),
    Release {
        reason: String,
        approver: String,
        numeric: bool,
    },
}

#[derive(Default)]
pub struct Versions {
    /// Whose versions are shown, as a label and a file.
    pub doc: Option<(String, PathBuf)>,
    /// The open document and the parts it places: what the list beside the filter picks from.
    pub docs: Vec<(String, PathBuf)>,
    pub filter: Filter,
    pub nodes: Vec<Node>,
    /// The folder keeps no history: Turn on history instead of the graph.
    pub off: bool,
    /// Versions whose saves are unfolded.
    pub open: HashSet<String>,
    pub sel: Option<String>,
    /// Compare with..: the first side chosen, waiting for the second.
    pub first: Option<String>,
    pub dialog: Option<(String, String)>,
    pub release: Option<(String, String, bool)>,
    /// While comparing: base, target, and what changed.
    pub comparing: Option<(String, String)>,
    pub diff: Vec<DiffRow>,
    /// Compare's counts for the status bar, and its Base to Target slider (0 base, 1 target).
    pub counts: String,
    pub slider: f32,
    /// Open: one save shown as it is, not a Compare.
    pub single: bool,
    /// The open document's newest released revision, for the browser's top row.
    pub revision: String,
    /// The open assembly's pinned parts with a newer version: instance, file, pinned, newest.
    pub pins: Vec<(String, String, u32, u32)>,
    pub note: String,
    pub thumbs: HashMap<String, egui::TextureHandle>,
    pub out: Vec<VersionsAct>,
}

impl Versions {
    /// A save as the graph names it: its version's name, else its short id.
    pub fn label_of(&self, key: &str) -> String {
        if key == "current" {
            return "current".into();
        }
        match self.nodes.iter().find(|n| n.save == key && n.n.is_some()) {
            Some(n) => n.title.clone(),
            None => short(key),
        }
    }
}

/// `2026-11-06 14:02` (UTC) from seconds since 1970.
pub fn date(t: i64) -> String {
    let days = t.div_euclid(86_400);
    let secs = t.rem_euclid(86_400);
    // civil from days (Howard Hinnant's algorithm)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}", secs / 3600, secs / 60 % 60)
}

const ROW: f32 = 54.0;
const SAVE_ROW: f32 = 34.0;
const LANE_X: f32 = 18.0;
const THUMB: f32 = 40.0;

/// Colours of Compare's classes, in either theme.
pub const ADDED: Color32 = Color32::from_rgb(0x3F, 0xB9, 0x6B);
pub const REMOVED: Color32 = Color32::from_rgb(0xE0, 0x4F, 0x4F);
pub const CHANGED: Color32 = Color32::from_rgb(0xE8, 0xA3, 0x2E);

impl Shell {
    pub(super) fn versions_ui(&mut self, ui: &mut Ui, r: Rect) {
        let t = self.theme;
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r.shrink2(vec2(8.0, 6.0))));
        let ui = &mut child;
        if self.versions.off {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("This folder keeps no history.")
                    .font(sans(13.0))
                    .color(t.ink2),
            );
            ui.add_space(8.0);
            let b = ui.add(primary(&t, "Turn on history"));
            self.places.push(("versions:turn-on".into(), b.rect));
            if b.clicked() {
                self.versions.out.push(VersionsAct::TurnOn);
            }
            return;
        }
        self.versions_top(ui);
        ui.add_space(4.0);
        let list_h = if self.versions.comparing.is_some() {
            (ui.available_height() * 0.55).max(120.0)
        } else {
            ui.available_height() - 40.0
        };
        egui::ScrollArea::vertical()
            .id_salt("versions-graph")
            .max_height(list_h)
            .auto_shrink([false, false])
            .show(ui, |ui| self.versions_graph(ui));
        self.versions_actions(ui);
        if self.versions.comparing.is_some() {
            self.versions_diff(ui);
        }
        if !self.versions.note.is_empty() {
            ui.label(
                egui::RichText::new(&self.versions.note)
                    .font(sans(12.0))
                    .color(t.error),
            );
        }
    }

    fn versions_top(&mut self, ui: &mut Ui) {
        let t = self.theme;
        ui.horizontal(|ui| {
            let b = ui.add(primary(&t, "Create Version"));
            self.places.push(("versions:create".into(), b.rect));
            if b.clicked() {
                self.versions.dialog = Some((String::new(), String::new()));
            }
            let b = ui.button(egui::RichText::new("Release").font(sans(12.0)));
            self.places.push(("versions:release".into(), b.rect));
            if b.clicked() {
                self.versions.release = Some((String::new(), String::new(), false));
            }
        });
        let pins = self.versions.pins.clone();
        for (inst, file, n, m) in &pins {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("{inst}  {file} v{n} · v{m} available"))
                        .font(sans(12.0))
                        .color(t.ink),
                );
                let b = ui.button(egui::RichText::new("Update").font(sans(12.0)));
                self.places.push((format!("versions:update:{inst}"), b.rect));
                if b.clicked() {
                    self.versions.out.push(VersionsAct::Update(Some(inst.clone())));
                }
            });
        }
        if pins.len() > 1 {
            let b = ui.button(egui::RichText::new("Update All").font(sans(12.0)));
            self.places.push(("versions:update-all".into(), b.rect));
            if b.clicked() {
                self.versions.out.push(VersionsAct::Update(None));
            }
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            for (f, word) in [
                (Filter::Part, "This part"),
                (Filter::Project, "Whole project"),
                (Filter::Releases, "Releases"),
            ] {
                let on = self.versions.filter == f;
                let b = ui.selectable_label(on, egui::RichText::new(word).font(sans(12.0)));
                self.places.push((format!("versions:filter:{word}"), b.rect));
                if b.clicked() && !on {
                    self.versions.filter = f;
                    self.versions.out.push(VersionsAct::Load);
                }
            }
        });
        if self.versions.docs.len() > 1 {
            let shown = self.versions.doc.as_ref().map_or(String::new(), |d| d.0.clone());
            let mut pick = None;
            egui::ComboBox::from_id_salt("versions-doc")
                .selected_text(egui::RichText::new(&shown).font(mono(12.0)))
                .width(ui.available_width() - 4.0)
                .show_ui(ui, |ui| {
                    for (label, path) in &self.versions.docs {
                        if ui
                            .selectable_label(*label == shown, egui::RichText::new(label).font(mono(12.0)))
                            .clicked()
                        {
                            pick = Some(path.clone());
                        }
                    }
                });
            if let Some(p) = pick {
                self.versions.out.push(VersionsAct::Show(p));
            }
        }
    }

    fn versions_graph(&mut self, ui: &mut Ui) {
        let t = self.theme;
        let nodes = self.versions.nodes.clone();
        let w = ui.available_width();
        let p = ui.painter().clone();
        for (k, n) in nodes.iter().enumerate() {
            let key = if n.save.is_empty() {
                "current".to_string()
            } else {
                n.save.clone()
            };
            let (r, resp) = ui.allocate_exact_size(vec2(w, ROW), Sense::click());
            let sel = self.versions.sel.as_deref() == Some(key.as_str());
            if sel {
                p.rect_filled(r, 3.0, t.selection);
            } else if resp.hovered() {
                p.rect_filled(r, 3.0, t.hover);
            }
            // the lane: through every node, a dot for a version, a ring for the work since
            let cx = r.left() + LANE_X;
            let top = if k == 0 { r.center().y } else { r.top() };
            p.vline(cx, top..=r.bottom(), Stroke::new(2.0, t.line));
            if n.n.is_some() {
                p.circle_filled(
                    pos2(cx, r.center().y),
                    6.0,
                    if n.revision.is_some() { t.focus } else { t.accent },
                );
            } else {
                p.circle_stroke(pos2(cx, r.center().y), 5.0, Stroke::new(2.0, t.ink2));
            }
            // its thumbnail (the work since the newest version has none of its own)
            let th = Rect::from_min_size(
                pos2(r.left() + 34.0, r.center().y - THUMB / 2.0),
                vec2(THUMB, THUMB),
            );
            if !n.save.is_empty() {
                p.rect_filled(th, 3.0, t.raised);
            }
            if let Some(tex) = self.versions.thumbs.get(&key) {
                p.image(
                    tex.id(),
                    th,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            let x = th.right() + 8.0;
            let mut title = n.title.clone();
            if self.versions.filter == Filter::Project && !n.path.is_empty() && n.n.is_some() {
                title = format!("{title}  {}", n.path);
            }
            p.text(
                pos2(x, r.top() + 6.0),
                Align2::LEFT_TOP,
                &title,
                semi(13.0),
                t.ink,
            );
            if let Some(rev) = &n.revision {
                let g = ui.fonts_mut(|f| f.layout_no_wrap(title.clone(), semi(13.0), t.ink));
                let b = Rect::from_min_size(pos2(x + g.size().x + 6.0, r.top() + 6.0), vec2(18.0, 16.0));
                p.rect_filled(b, 3.0, t.focus);
                p.text(
                    b.center(),
                    Align2::CENTER_CENTER,
                    rev,
                    semi(11.0),
                    Color32::from_gray(20),
                );
            }
            let who = if n.agent {
                format!("{} (agent)", n.author)
            } else {
                n.author.clone()
            };
            let meta = if n.time > 0 {
                format!("{who} · {}", date(n.time))
            } else {
                who
            };
            p.text(
                pos2(x, r.top() + 24.0),
                Align2::LEFT_TOP,
                meta,
                sans(11.0),
                t.ink2,
            );
            if !n.notes.is_empty() {
                let line = n.notes.lines().next().unwrap_or("");
                p.text(
                    pos2(x, r.top() + 38.0),
                    Align2::LEFT_TOP,
                    line,
                    sans(11.0),
                    t.ink2,
                );
            }
            self.places.push((format!("versions:row:{}", n.title), r));
            if resp.clicked() {
                self.versions_pick(&key);
            }
            if resp.double_clicked() && !n.save.is_empty() {
                self.versions.out.push(VersionsAct::Open(n.save.clone()));
            }
            // its saves, folded
            if n.saves.is_empty() {
                continue;
            }
            let opened = self.versions.open.contains(&key);
            let (fr, fresp) = ui.allocate_exact_size(vec2(w, 20.0), Sense::click());
            p.vline(cx, fr.y_range(), Stroke::new(2.0, t.line));
            let word = format!(
                "{} save{}",
                n.saves.len(),
                if n.saves.len() == 1 { "" } else { "s" }
            );
            let ink = if fresp.hovered() { t.ink } else { t.ink2 };
            // a small triangle: right while folded, down when open
            let c = pos2(r.left() + 38.0, fr.center().y);
            let tri = if opened {
                vec![c + vec2(-3.5, -2.0), c + vec2(3.5, -2.0), c + vec2(0.0, 2.5)]
            } else {
                vec![c + vec2(-2.0, -3.5), c + vec2(-2.0, 3.5), c + vec2(2.5, 0.0)]
            };
            p.add(egui::Shape::convex_polygon(tri, ink, Stroke::NONE));
            p.text(
                pos2(r.left() + 46.0, fr.center().y),
                Align2::LEFT_CENTER,
                word,
                sans(11.0),
                ink,
            );
            self.places.push((format!("versions:saves:{}", n.title), fr));
            if fresp.clicked() {
                if opened {
                    self.versions.open.remove(&key);
                } else {
                    self.versions.open.insert(key.clone());
                }
            }
            if !opened {
                continue;
            }
            for s in &n.saves {
                let (sr, sresp) = ui.allocate_exact_size(vec2(w, SAVE_ROW), Sense::click());
                let ssel = self.versions.sel.as_deref() == Some(s.save.as_str());
                if ssel {
                    p.rect_filled(sr, 3.0, t.selection);
                } else if sresp.hovered() {
                    p.rect_filled(sr, 3.0, t.hover);
                }
                p.vline(cx, sr.y_range(), Stroke::new(2.0, t.line));
                p.circle_filled(pos2(cx, sr.center().y), 2.5, t.ink2);
                let x = r.left() + 34.0;
                p.text(
                    pos2(x, sr.top() + 3.0),
                    Align2::LEFT_TOP,
                    &s.summary,
                    sans(12.0),
                    t.ink,
                );
                let who = if s.agent {
                    format!("{} (agent)", s.author)
                } else {
                    s.author.clone()
                };
                p.text(
                    pos2(x, sr.top() + 18.0),
                    Align2::LEFT_TOP,
                    format!("{who} · {} · {}", date(s.time), &s.save[..s.save.len().min(7)]),
                    sans(11.0),
                    t.ink2,
                );
                self.places
                    .push((format!("versions:save:{}", &s.save[..s.save.len().min(8)]), sr));
                if sresp.clicked() {
                    self.versions_pick(&s.save);
                }
            }
        }
        if nodes.is_empty() {
            ui.label(
                egui::RichText::new("No saves yet.")
                    .font(sans(12.0))
                    .color(t.ink2),
            );
        }
    }

    fn versions_pick(&mut self, key: &str) {
        if let Some(first) = self.versions.first.take() {
            if first != key {
                self.versions
                    .out
                    .push(VersionsAct::Compare(first, key.to_string()));
            }
            self.versions.sel = Some(key.to_string());
            return;
        }
        self.versions.sel = Some(key.to_string());
    }

    fn versions_actions(&mut self, ui: &mut Ui) {
        let t = self.theme;
        let Some(sel) = self.versions.sel.clone() else {
            return;
        };
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            let mut b = |ui: &mut Ui, word: &str, name: &str, act: Option<VersionsAct>| {
                let enabled = act.is_some();
                let r = ui.add_enabled(
                    enabled,
                    egui::Button::new(egui::RichText::new(word).font(sans(12.0))),
                );
                self.places.push((format!("versions:{name}"), r.rect));
                if r.clicked()
                    && let Some(a) = act
                {
                    self.versions.out.push(a);
                }
            };
            let saved = sel != "current";
            b(ui, "Open", "open", saved.then(|| VersionsAct::Open(sel.clone())));
            b(
                ui,
                "Restore",
                "restore",
                saved.then(|| VersionsAct::Restore(sel.clone())),
            );
            b(
                ui,
                "Compare with current",
                "compare-current",
                saved.then(|| VersionsAct::Compare(sel.clone(), "current".into())),
            );
            b(
                ui,
                "Copy ID",
                "copy-id",
                saved.then(|| VersionsAct::CopyId(sel.clone())),
            );
            let r = ui.add(
                egui::Button::new(egui::RichText::new("Compare with…").font(sans(12.0)))
                    .selected(self.versions.first.is_some()),
            );
            self.places.push(("versions:compare-with".into(), r.rect));
            if r.clicked() {
                self.versions.first = if self.versions.first.is_some() {
                    None
                } else {
                    Some(sel.clone())
                };
            }
        });
        if self.versions.first.is_some() {
            ui.label(
                egui::RichText::new("Pick the other side")
                    .font(sans(11.0))
                    .color(t.ink2),
            );
        }
    }

    fn versions_diff(&mut self, ui: &mut Ui) {
        let t = self.theme;
        let Some((a, b)) = self.versions.comparing.clone() else {
            return;
        };
        ui.separator();
        let (la, lb) = (self.versions.label_of(&a), self.versions.label_of(&b));
        ui.label(
            egui::RichText::new(format!("Compare {la} to {lb}"))
                .font(semi(12.0))
                .color(t.ink),
        );
        let r = ui.button(egui::RichText::new("End Compare").font(sans(12.0)));
        self.places.push(("versions:end-compare".into(), r.rect));
        if r.clicked() {
            self.versions.out.push(VersionsAct::EndCompare);
        }
        egui::ScrollArea::vertical()
            .id_salt("versions-diff")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut group = "";
                for d in &self.versions.diff {
                    if d.group != group {
                        group = d.group;
                        ui.label(egui::RichText::new(group).font(semi(11.0)).color(t.ink2));
                    }
                    let (mark, ink) = match d.change {
                        "added" => ("+", ADDED),
                        "removed" => ("-", REMOVED),
                        "moved" => ("~", t.ink2),
                        _ => ("≠", CHANGED),
                    };
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        ui.label(egui::RichText::new(mark).font(mono(12.0)).color(ink));
                        ui.label(egui::RichText::new(&d.name).font(mono(12.0)).color(t.ink));
                        let what = match (d.old.is_empty(), d.new.is_empty()) {
                            (false, false) if d.change == "changed" => format!("{} to {}", d.old, d.new),
                            (true, false) => d.new.clone(),
                            (false, _) => d.old.clone(),
                            _ => String::new(),
                        };
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(one_line(&what, 60))
                                    .font(mono(11.0))
                                    .color(t.ink2),
                            )
                            .truncate(),
                        );
                    });
                }
                if self.versions.diff.is_empty() {
                    ui.label(
                        egui::RichText::new("No feature or parameter changed.")
                            .font(sans(12.0))
                            .color(t.ink2),
                    );
                }
            });
    }

    /// Create Version's and Release's windows.
    pub(super) fn versions_dialogs(&mut self, ctx: &egui::Context) {
        let area = super::dialog_area(ctx);
        if let Some((mut name, mut notes)) = self.versions.dialog.clone() {
            let mut done: Option<bool> = None;
            egui::Window::new("Create Version")
                .id(egui::Id::unique("versions-create"))
                .frame(super::fgasm::dialog_frame(&self.theme))
                .collapsible(false)
                .resizable(false)
                .order(egui::Order::Foreground)
                .default_pos(area.center() - vec2(180.0, 90.0))
                .constrain_to(area)
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("Name").font(sans(12.0)));
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut name)
                            .hint_text("v1")
                            .desired_width(320.0),
                    );
                    self.places.push(("versions:dialog:name".into(), r.rect));
                    ui.label(egui::RichText::new("Notes").font(sans(12.0)));
                    let r = ui.add(
                        egui::TextEdit::multiline(&mut notes)
                            .desired_rows(3)
                            .desired_width(320.0),
                    );
                    self.places.push(("versions:dialog:notes".into(), r.rect));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let r = ui.add(primary(&self.theme, "Create"));
                        self.places.push(("versions:dialog:ok".into(), r.rect));
                        if r.clicked() {
                            done = Some(true);
                        }
                        let r = ui.button("Cancel");
                        self.places.push(("versions:dialog:cancel".into(), r.rect));
                        if r.clicked() {
                            done = Some(false);
                        }
                    });
                });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                done = Some(false);
            }
            match done {
                Some(true) => {
                    self.versions.out.push(VersionsAct::Create { name, notes });
                    self.versions.dialog = None;
                }
                Some(false) => self.versions.dialog = None,
                None => self.versions.dialog = Some((name, notes)),
            }
        }
        if let Some((mut reason, mut approver, mut numeric)) = self.versions.release.clone() {
            let mut done: Option<bool> = None;
            egui::Window::new("Release")
                .id(egui::Id::unique("versions-release"))
                .frame(super::fgasm::dialog_frame(&self.theme))
                .collapsible(false)
                .resizable(false)
                .order(egui::Order::Foreground)
                .default_pos(area.center() - vec2(180.0, 100.0))
                .constrain_to(area)
                .show(ctx, |ui| {
                    let what = self.versions.doc.as_ref().map_or(String::new(), |d| d.0.clone());
                    ui.label(egui::RichText::new(what).font(mono(12.0)));
                    ui.label(egui::RichText::new("Reason").font(sans(12.0)));
                    let r = ui.add(
                        egui::TextEdit::multiline(&mut reason)
                            .desired_rows(2)
                            .desired_width(320.0),
                    );
                    self.places.push(("versions:release:reason".into(), r.rect));
                    ui.label(egui::RichText::new("Approver").font(sans(12.0)));
                    let r = ui.add(egui::TextEdit::singleline(&mut approver).desired_width(320.0));
                    self.places.push(("versions:release:approver".into(), r.rect));
                    ui.checkbox(
                        &mut numeric,
                        egui::RichText::new("Numeric pre-release revision").font(sans(12.0)),
                    );
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let r = ui.add_enabled(!reason.trim().is_empty(), primary(&self.theme, "Release"));
                        self.places.push(("versions:release:ok".into(), r.rect));
                        if r.clicked() {
                            done = Some(true);
                        }
                        if ui.button("Cancel").clicked() {
                            done = Some(false);
                        }
                    });
                });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                done = Some(false);
            }
            match done {
                Some(true) => {
                    self.versions.out.push(VersionsAct::Release {
                        reason,
                        approver,
                        numeric,
                    });
                    self.versions.release = None;
                }
                Some(false) => self.versions.release = None,
                None => self.versions.release = Some((reason, approver, numeric)),
            }
        }
    }
}

impl Shell {
    /// The status bar while comparing: Base, the slider, Target, the counts, Back to Model.
    pub(super) fn compare_status(&mut self, ui: &mut Ui, st: Rect) {
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(
            pos2(st.right() - 640.0, st.top() + 1.0),
            pos2(st.right() - 8.0, st.bottom() - 1.0),
        )));
        let ui = &mut child;
        ui.horizontal_centered(|ui| {
            let w = |s: &str| {
                egui::RichText::new(s.to_string())
                    .font(sans(12.0))
                    .color(Color32::WHITE)
            };
            if !self.versions.single {
                ui.label(w(&self.versions.counts));
                ui.add_space(8.0);
                ui.label(w("Base"));
                let r = ui.add(egui::Slider::new(&mut self.versions.slider, 0.0..=1.0).show_value(false));
                self.places.push(("compare:slider".into(), r.rect));
                ui.add_space(4.0);
                ui.label(w("Target"));
                ui.add_space(8.0);
            }
            let r = ui.button(egui::RichText::new("Back to Model").font(sans(12.0)));
            self.places.push(("compare:back".into(), r.rect));
            if r.clicked() {
                self.versions.out.push(VersionsAct::EndCompare);
            }
        });
    }
}

fn primary(t: &Theme, word: &str) -> egui::Button<'static> {
    egui::Button::new(
        egui::RichText::new(word.to_string())
            .font(semi(12.0))
            .color(t.on_ink),
    )
    .fill(t.on)
}

fn short(s: &str) -> String {
    if s == "current" {
        "current".into()
    } else {
        s[..s.len().min(8)].to_string()
    }
}

fn one_line(s: &str, n: usize) -> String {
    let l = s.lines().next().unwrap_or("");
    if l.chars().count() > n {
        format!("{}…", l.chars().take(n).collect::<String>())
    } else {
        l.to_string()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn dates() {
        assert_eq!(super::date(0), "1970-01-01 00:00");
        assert_eq!(super::date(1_762_430_520), "2025-11-06 12:02");
        assert_eq!(super::date(951_782_400), "2000-02-29 00:00");
    }
}
