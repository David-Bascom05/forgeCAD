//! The Versions tab, beside Model and Agent: Onshape's version graph. Named versions down one
//! lane, newest first, each with its saves folded under it; the selected row's Open, Restore,
//! Compare and Copy ID; Create Version and Release at the top; and, while comparing, what
//! changed in features and parameters. The host fills it and does what `VersionsAct`s ask.

use super::{Shell, icons};
use super::theme::{Theme, mono, sans, semi};
use egui::{Align2, Color32, Rect, Sense, Stroke, Ui, pos2, vec2};
use std::collections::HashSet;
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
    /// The feature's kind, for its icon (None: a parameter or occurrence).
    pub kind: Option<String>,
    /// What a person reads: `Hole Ø8 added`, `40 to 50 mm`.
    pub summary: String,
    /// The faces it made, by place in the base's and the target's bodies.
    pub faces: (Vec<usize>, Vec<usize>),
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
    /// Material added and removed, mm³ (None: no volume).
    pub volume: Option<(f64, f64)>,
    /// The Compare row whose faces are lit in the view.
    pub focus: Option<usize>,
    /// The saves between versions are listed too.
    pub show_saves: bool,
    pub slider: f32,
    /// Open: one save shown as it is, not a Compare.
    pub single: bool,
    /// The open document's newest released revision, for the browser's top row.
    pub revision: String,
    /// The open assembly's pinned parts with a newer version: instance, file, pinned, newest.
    pub pins: Vec<(String, String, u32, u32)>,
    pub note: String,
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

const ROW: f32 = 44.0;
const SAVE_ROW: f32 = 30.0;
/// The selected row's context bar, under its text.
const BAR: f32 = 30.0;
const LANE_X: f32 = 14.0;
const TEXT_X: f32 = 30.0;
const NODE_R: f32 = 5.0;

/// Colours of Compare's classes, in either theme.
pub const ADDED: Color32 = Color32::from_rgb(0x3F, 0xB9, 0x6B);
pub const REMOVED: Color32 = Color32::from_rgb(0xE0, 0x4F, 0x4F);
pub const CHANGED: Color32 = Color32::from_rgb(0xE8, 0xA3, 0x2E);

/// The feature kinds with an icon of their own.
const KIND_ICONS: &[&str] = &[
    "sketch", "extrude", "revolve", "sweep", "loft", "coil", "rib", "hole", "fillet", "chamfer", "shell",
    "draft", "thread", "plane", "axis", "point", "mirror", "thicken", "split", "move", "text",
];

fn class_colour(change: &str, t: &Theme) -> Color32 {
    match change {
        "added" => ADDED,
        "removed" => REMOVED,
        "changed" => CHANGED,
        _ => t.ink2,
    }
}

impl Shell {
    pub(super) fn versions_ui(&mut self, ui: &mut Ui, r: Rect) {
        let t = self.theme;
        let inner = r.shrink2(vec2(8.0, 6.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
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
        let comparing = self.versions.comparing.is_some();
        // the graph takes what the Compare panel, docked below it, leaves
        let panel = if comparing {
            (ui.available_height() * 0.5).max(180.0)
        } else {
            0.0
        };
        egui::ScrollArea::vertical()
            .id_salt("versions-graph")
            .max_height((ui.available_height() - panel).max(80.0))
            .auto_shrink([false, false])
            .show(ui, |ui| self.versions_graph(ui));
        if comparing {
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

    /// The header bar: Create Version and Release, the filter, Show saves, the document picked.
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
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            for (f, word) in [
                (Filter::Part, "This part"),
                (Filter::Project, "Project"),
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
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut on = self.versions.show_saves;
                let b = ui.checkbox(&mut on, egui::RichText::new("Show saves").font(sans(12.0)));
                self.places.push(("versions:show-saves".into(), b.rect));
                self.versions.show_saves = on;
            });
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
        let pins = self.versions.pins.clone();
        for (inst, file, n, m) in &pins {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("{inst}: {file} v{n}, v{m} available"))
                        .font(sans(12.0))
                        .color(t.ink),
                );
                let b = ui.small_button(egui::RichText::new("Update").font(sans(11.0)));
                self.places.push((format!("versions:update:{inst}"), b.rect));
                if b.clicked() {
                    self.versions.out.push(VersionsAct::Update(Some(inst.clone())));
                }
            });
        }
        ui.add_space(4.0);
        let y = ui.cursor().top();
        ui.painter()
            .hline(ui.max_rect().x_range(), y, Stroke::new(1.0, t.line));
        ui.add_space(2.0);
    }

    fn versions_graph(&mut self, ui: &mut Ui) {
        let t = self.theme;
        let nodes = self.versions.nodes.clone();
        let show_saves = self.versions.show_saves;
        let w = ui.available_width();
        let p = ui.painter().clone();
        let lane = Stroke::new(1.5, t.line);
        let mut rows = 0;
        for (k, n) in nodes.iter().enumerate() {
            // only the work since the newest version, when there is some and saves are shown
            if n.n.is_none() && !show_saves {
                continue;
            }
            let key = if n.save.is_empty() {
                "current".to_string()
            } else {
                n.save.clone()
            };
            let sel = self.versions.sel.as_deref() == Some(key.as_str());
            let h = if sel && n.n.is_some() { ROW + BAR } else { ROW };
            let (r, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
            if sel {
                p.rect_filled(r, 4.0, t.selection);
            } else if resp.hovered() {
                p.rect_filled(r, 4.0, t.hover);
            }
            let cx = r.left() + LANE_X;
            let cy = r.top() + ROW / 2.0;
            let first = rows == 0;
            let last = k + 1 == nodes.len() && !(show_saves && !n.saves.is_empty());
            p.vline(
                cx,
                (if first { cy } else { r.top() })..=(if last { cy } else { r.bottom() }),
                lane,
            );
            match (&n.revision, n.n) {
                (Some(_), _) => {
                    p.circle_filled(pos2(cx, cy), NODE_R + 1.0, t.surface);
                    p.circle_filled(pos2(cx, cy), NODE_R, t.focus);
                }
                (None, Some(_)) => {
                    p.circle_filled(pos2(cx, cy), NODE_R + 1.0, t.surface);
                    p.circle_filled(pos2(cx, cy), NODE_R, t.accent);
                }
                _ => {
                    p.circle_filled(pos2(cx, cy), NODE_R, t.surface);
                    p.circle_stroke(pos2(cx, cy), NODE_R - 0.75, Stroke::new(1.5, t.ink2));
                }
            }
            let x = r.left() + TEXT_X;
            // the revision badge in one column at the right
            let right = r.right() - 8.0;
            if let Some(rev) = &n.revision {
                let g = ui.fonts_mut(|f| f.layout_no_wrap(rev.clone(), semi(11.0), Color32::from_gray(20)));
                let bw = g.size().x + 10.0;
                let b = Rect::from_min_size(pos2(right - bw, r.top() + 7.0), vec2(bw, 17.0));
                p.rect_filled(b, 3.0, t.focus);
                p.galley(
                    pos2(b.left() + 5.0, b.center().y - g.size().y / 2.0),
                    g,
                    Color32::from_gray(20),
                );
            }
            let mut title = n.title.clone();
            if self.versions.filter == Filter::Project && n.n.is_some() && !n.path.is_empty() {
                title = format!("{title}  ·  {}", n.path);
            }
            p.text(
                pos2(x, r.top() + 7.0),
                Align2::LEFT_TOP,
                &title,
                semi(13.0),
                t.ink,
            );
            let who = if n.agent {
                format!("{} (agent)", n.author)
            } else {
                n.author.clone()
            };
            let mut meta = if n.time > 0 {
                format!("{who} · {}", date(n.time))
            } else {
                who
            };
            if !n.notes.is_empty() {
                meta = format!("{meta} · {}", one_line(&n.notes, 40));
            }
            p.text(
                pos2(x, r.top() + 25.0),
                Align2::LEFT_TOP,
                meta,
                sans(11.0),
                t.ink2,
            );
            self.places.push((format!("versions:row:{}", n.title), r));
            if resp.clicked() {
                self.versions_pick(&key);
            }
            if sel && n.n.is_some() {
                let bar = Rect::from_min_max(
                    pos2(x - 4.0, r.top() + ROW),
                    pos2(r.right() - 6.0, r.bottom() - 4.0),
                );
                self.versions_bar(ui, bar, &key);
            }
            rows += 1;
            if !show_saves {
                continue;
            }
            for (j, s) in n.saves.iter().enumerate() {
                let (sr, sresp) = ui.allocate_exact_size(vec2(w, SAVE_ROW), Sense::click());
                let ssel = self.versions.sel.as_deref() == Some(s.save.as_str());
                if ssel {
                    p.rect_filled(sr, 4.0, t.selection);
                } else if sresp.hovered() {
                    p.rect_filled(sr, 4.0, t.hover);
                }
                let end = k + 1 == nodes.len() && j + 1 == n.saves.len();
                p.vline(
                    cx,
                    sr.top()..=(if end { sr.center().y } else { sr.bottom() }),
                    lane,
                );
                p.circle_filled(pos2(cx, sr.center().y), 2.5, t.ink2);
                p.text(
                    pos2(x, sr.top() + 3.0),
                    Align2::LEFT_TOP,
                    one_line(&s.summary, 44),
                    sans(12.0),
                    t.ink,
                );
                let who = if s.agent {
                    format!("{} (agent)", s.author)
                } else {
                    s.author.clone()
                };
                p.text(
                    pos2(x, sr.top() + 17.0),
                    Align2::LEFT_TOP,
                    format!("{who} · {}", date(s.time)),
                    sans(10.5),
                    t.ink2,
                );
                self.places
                    .push((format!("versions:save:{}", &s.save[..s.save.len().min(8)]), sr));
                if sresp.clicked() {
                    self.versions_pick(&s.save);
                }
            }
        }
        if rows == 0 {
            ui.add_space(6.0);
            let word = if nodes.is_empty() {
                "No saves yet."
            } else {
                "No named versions yet."
            };
            ui.label(egui::RichText::new(word).font(sans(12.0)).color(t.ink2));
        }
    }

    /// The selected version's actions: icon buttons with tooltips, the rest in a menu.
    fn versions_bar(&mut self, ui: &mut Ui, bar: Rect, key: &str) {
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(bar)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        let ui = &mut child;
        let k = key.to_string();
        for (icon, tip, name, act) in [
            (
                "open",
                "Open this version, read-only",
                "open",
                VersionsAct::Open(k.clone()),
            ),
            (
                "undo",
                "Restore: save this version again as the newest",
                "restore",
                VersionsAct::Restore(k.clone()),
            ),
            (
                "measure",
                "Compare with the part as it is now",
                "compare-current",
                VersionsAct::Compare(k.clone(), "current".into()),
            ),
        ] {
            let (r, resp) = ui.allocate_exact_size(vec2(26.0, 24.0), Sense::click());
            let resp = resp.on_hover_text(tip);
            if resp.hovered() {
                ui.painter().rect_filled(r, 4.0, self.theme.hover);
            }
            icons::draw(
                ui.painter(),
                r.center() - vec2(8.0, 8.0),
                16.0,
                icon,
                1.5,
                self.theme.ink,
            );
            self.places.push((format!("versions:{name}"), r));
            if resp.clicked() {
                self.versions.out.push(act);
            }
        }
        let m = ui.menu_button(egui::RichText::new("···").font(semi(13.0)), |ui| {
            let r = ui.button("Compare with…");
            self.places.push(("versions:compare-with".into(), r.rect));
            if r.clicked() {
                self.versions.first = Some(k.clone());
                ui.close();
            }
            let r = ui.button("Copy ID");
            self.places.push(("versions:copy-id".into(), r.rect));
            if r.clicked() {
                self.versions.out.push(VersionsAct::CopyId(k.clone()));
                ui.close();
            }
        });
        self.places.push(("versions:more".into(), m.response.rect));
        if self.versions.first.is_some() {
            ui.label(
                egui::RichText::new("pick the other version")
                    .font(sans(11.0))
                    .color(self.theme.ink2),
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
        }
        self.versions.sel = Some(key.to_string());
    }

    /// The Compare panel, docked under the graph: what was compared, the material added and
    /// removed, and each change as a person says it; a row lights its faces in the view.
    fn versions_diff(&mut self, ui: &mut Ui) {
        let t = self.theme;
        let Some((a, b)) = self.versions.comparing.clone() else {
            return;
        };
        ui.add_space(2.0);
        let y = ui.cursor().top();
        ui.painter()
            .hline(ui.max_rect().x_range(), y, Stroke::new(1.0, t.line));
        ui.add_space(6.0);
        let (la, lb) = (self.versions.label_of(&a), self.versions.label_of(&b));
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{la}  to  {lb}"))
                    .font(semi(12.0))
                    .color(t.ink),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let r = ui.small_button(egui::RichText::new("End").font(sans(11.0)));
                self.places.push(("versions:end-compare".into(), r.rect));
                if r.on_hover_text("Back to the model").clicked() {
                    self.versions.out.push(VersionsAct::EndCompare);
                }
            });
        });
        if let Some((add, rem)) = self.versions.volume {
            let w = ui.available_width();
            let most = add.max(rem).max(1e-9);
            for (v, c, sign, name) in [(add, ADDED, "+", "added"), (rem, REMOVED, "−", "removed")] {
                let (r, _) = ui.allocate_exact_size(vec2(w, 18.0), Sense::hover());
                let p = ui.painter();
                let track =
                    Rect::from_min_size(pos2(r.left() + 64.0, r.center().y - 4.0), vec2(w - 150.0, 8.0));
                p.rect_filled(track, 4.0, t.raised);
                let fill =
                    Rect::from_min_size(track.min, vec2((track.width() * (v / most) as f32).max(2.0), 8.0));
                p.rect_filled(fill, 4.0, c);
                p.text(
                    pos2(r.left(), r.center().y),
                    Align2::LEFT_CENTER,
                    name,
                    sans(11.0),
                    t.ink2,
                );
                p.text(
                    pos2(r.right(), r.center().y),
                    Align2::RIGHT_CENTER,
                    format!("{sign}{v:.1} mm³"),
                    mono(11.0),
                    t.ink,
                );
            }
            ui.add_space(4.0);
        }
        let rows = self.versions.diff.clone();
        egui::ScrollArea::vertical()
            .id_salt("versions-diff")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let w = ui.available_width();
                let mut group = "";
                for (i, d) in rows.iter().enumerate() {
                    if d.group != group {
                        group = d.group;
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(group).font(semi(11.0)).color(t.ink2));
                    }
                    let (r, resp) = ui.allocate_exact_size(vec2(w, 28.0), Sense::click());
                    let on = self.versions.focus == Some(i);
                    let p = ui.painter();
                    if on {
                        p.rect_filled(r, 4.0, t.selection);
                    } else if resp.hovered() {
                        p.rect_filled(r, 4.0, t.hover);
                    }
                    let icon = match (&d.kind, d.group) {
                        (Some(k), _) if KIND_ICONS.contains(&k.as_str()) => k.as_str(),
                        (Some(_), _) => "sketchnode",
                        (None, "Parameters") => "param",
                        _ => "file",
                    };
                    icons::draw(
                        p,
                        pos2(r.left() + 4.0, r.center().y - 8.0),
                        16.0,
                        icon,
                        1.4,
                        t.ink2,
                    );
                    let g = ui.fonts_mut(|f| f.layout_no_wrap(d.name.clone(), semi(12.0), t.ink));
                    let nx = r.left() + 26.0;
                    let name_w = g.size().x;
                    p.galley(pos2(nx, r.center().y - g.size().y / 2.0), g, t.ink);
                    // the change, as a coloured chip at the right
                    let c = class_colour(d.change, &t);
                    let cg = ui.fonts_mut(|f| f.layout_no_wrap(d.change.to_string(), sans(10.5), c));
                    let cw = cg.size().x + 12.0;
                    let chip =
                        Rect::from_min_size(pos2(r.right() - cw - 4.0, r.center().y - 9.0), vec2(cw, 18.0));
                    p.rect_filled(chip, 9.0, c.gamma_multiply(0.18));
                    p.galley(
                        pos2(chip.left() + 6.0, chip.center().y - cg.size().y / 2.0),
                        cg,
                        c,
                    );
                    let room = (chip.left() - nx - name_w - 18.0).max(0.0);
                    let sg = ui.fonts_mut(|f| {
                        let mut job =
                            egui::text::LayoutJob::simple_singleline(d.summary.clone(), sans(11.5), t.ink2);
                        job.wrap.max_width = room;
                        job.wrap.max_rows = 1;
                        job.wrap.break_anywhere = true;
                        f.layout_job(job)
                    });
                    p.galley(
                        pos2(nx + name_w + 10.0, r.center().y - sg.size().y / 2.0),
                        sg,
                        t.ink2,
                    );
                    self.places.push((format!("versions:diff:{}", d.name), r));
                    if resp.on_hover_text(&d.summary).clicked() {
                        self.versions.focus = if on { None } else { Some(i) };
                    }
                }
                if rows.is_empty() {
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
