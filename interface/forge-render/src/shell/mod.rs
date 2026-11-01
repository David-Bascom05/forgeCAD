//! The app's chrome: title bar, ribbon, tree, panel, tabs, status bar. It knows nothing of the
//! window or the kernel's worker: it reads a document and says what was asked in `Action`s, so the
//! desktop app and the browser share it. Buttons never change the document themselves: the host
//! turns an action into a tool call.

pub mod icons;
pub mod panel;
mod ribbon;
pub mod theme;

pub use crate::nav::NavTool;
use crate::render::LineMode;
use egui::{Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use forge_doc::{Body, Document, ItemId};
use theme::{Theme, mono, sans, semi};

pub const TABS: [&str; 5] = ["3D Model", "Sketch", "Inspect", "View", "Manage"];

const TITLE_H: f32 = 36.0;
const TABS_H: f32 = 34.0;
const RIBBON_H: f32 = 104.0;
const SIDE_W: f32 = 270.0;
const HEAD_H: f32 = 30.0;
const ROW_H: f32 = 26.0;
const DOCS_H: f32 = 28.0;
const STATUS_H: f32 = 24.0;
const NAVBAR_W: f32 = 32.0;
/// Below the view cube's widget.
const NAVBAR_TOP: f32 = 176.0;
const RESIZE_EDGE: f32 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    N,
    S,
    E,
    W,
    NE,
    NW,
    SE,
    SW,
}

/// What the chrome was asked for. The host does it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    New,
    Open,
    Undo,
    Redo,
    FitAll,
    Home,
    SetHome,
    Lines(LineMode),
    Section,
    Tool(NavTool),
    Parallel(bool),
    Minimize,
    Maximize,
    Close,
    DragWindow,
    ResizeWindow(Edge),
}

/// One row of the tree.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub id: ItemId,
    pub name: String,
    pub kind: String,
    pub icon: &'static str,
    /// As the tree reads: the kind's word and its count, "Extrude2".
    pub title: String,
    /// Its line in the part file, label and all.
    pub line: String,
}

/// What the view shows, for the buttons that reflect it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewState {
    pub lines: LineMode,
    pub section: bool,
    pub parallel: bool,
    pub tool: NavTool,
    pub maximized: bool,
}

impl Default for ViewState {
    fn default() -> ViewState {
        ViewState {
            lines: LineMode::Near,
            section: false,
            parallel: true,
            tool: NavTool::Orbit,
            maximized: false,
        }
    }
}

pub struct Shell {
    pub theme: Theme,
    pub tab: usize,
    pub name: String,
    pub doc: Option<Document>,
    pub rows: Vec<Row>,
    pub failed: Option<(ItemId, String)>,
    pub selected: Option<ItemId>,
    pub version: u64,
    pub status: String,
    pub view: ViewState,
    pub out: Vec<Action>,
    /// Where the viewport is, in logical pixels, snapped to device pixels.
    pub view_rect: Rect,
    /// Where each thing was drawn, last frame, by name: tests and scripts click by it.
    pub places: Vec<(String, Rect)>,
    /// The view cube's right-click menu, at this point.
    pub cube_menu: Option<egui::Pos2>,
    /// The part's picture, drawn by the renderer this same frame.
    pub view_tex: Option<egui::TextureId>,
    /// The view cube's words, in the view's device pixels: real type over its picture.
    pub cube_labels: Vec<crate::viewcube::Label>,
    file_menu: bool,
    style_set: Option<bool>,
}

impl Default for Shell {
    fn default() -> Shell {
        Shell {
            theme: theme::DARK,
            tab: 0,
            name: String::new(),
            doc: None,
            rows: Vec::new(),
            failed: None,
            selected: None,
            version: 0,
            status: String::new(),
            view: ViewState::default(),
            out: Vec::new(),
            view_rect: Rect::ZERO,
            places: Vec::new(),
            cube_menu: None,
            view_tex: None,
            cube_labels: Vec::new(),
            file_menu: false,
            style_set: None,
        }
    }
}

impl Shell {
    /// The document is what this text says now.
    pub fn set_doc(&mut self, src: &str, version: u64, failed: Option<(u64, String)>) {
        let doc = Document::parse(src);
        let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        self.rows = doc
            .items
            .iter()
            .filter_map(|it| {
                let kind = match &it.body {
                    Body::Feature { kind, .. } => kind.clone(),
                    Body::Sketch { .. } => "sketch".to_string(),
                    _ => return None,
                };
                let word = panel::kind_word(&kind);
                let count = counts.entry(word.clone()).or_default();
                *count += 1;
                let text = doc.text_of(it.id).unwrap_or_default();
                Some(Row {
                    id: it.id,
                    title: format!("{word}{count}"),
                    line: text.lines().next().unwrap_or("").to_string(),
                    name: doc.name(it.id).unwrap_or_else(|| kind.clone()),
                    icon: panel::icon(&kind),
                    kind,
                })
            })
            .collect();
        self.failed = failed.map(|(id, why)| (ItemId(id), why));
        if self
            .selected
            .is_some_and(|s| !self.rows.iter().any(|r| r.id == s))
        {
            self.selected = None;
        }
        self.doc = Some(doc);
        self.version = version;
    }

    pub fn place(&self, name: &str) -> Option<Rect> {
        self.places.iter().find(|p| p.0 == name).map(|p| p.1)
    }

    fn note(&mut self, name: impl Into<String>, r: Rect) {
        self.places.push((name.into(), r));
    }

    /// A menu's up: a press anywhere else only closes it.
    pub fn menu_open(&self) -> bool {
        self.file_menu || self.cube_menu.is_some()
    }

    /// The whole window.
    pub fn ui(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let t = self.theme;
        if self.style_set != Some(t.dark) {
            theme::apply(&ctx, &t);
            self.style_set = Some(t.dark);
        }
        self.places.clear();
        let s = ctx.content_rect();
        let p = ui.painter().clone();
        p.rect_filled(s, 0.0, t.bg);

        let top = TITLE_H + TABS_H + RIBBON_H;
        let bottom = s.height() - DOCS_H - STATUS_H;
        let side = Rect::from_min_max(pos2(0.0, top), pos2(SIDE_W, bottom));
        let ppp = ctx.pixels_per_point();
        let snap = |v: f32| (v * ppp).round() / ppp;
        self.view_rect = Rect::from_min_max(
            pos2(snap(SIDE_W + 1.0), snap(top)),
            pos2(snap(s.width()), snap(bottom)),
        );

        p.rect_filled(
            self.view_rect,
            0.0,
            egui::Color32::from_rgb((t.view >> 16) as u8, (t.view >> 8) as u8, t.view as u8),
        );
        if let Some(tex) = self.view_tex {
            let whole = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
            p.image(tex, self.view_rect, whole, Color32::WHITE);
        }
        self.cube_words(ui, ppp);
        self.title_bar(ui, s.width());
        self.tabs_row(ui, s.width());
        self.ribbon(ui, s.width());
        self.side(ui, side);
        self.docs_and_status(ui, s, bottom);
        self.navbar(&ctx);
        self.resize_edges(ui, s);
        self.menus(&ctx);
    }

    fn cube_words(&self, ui: &mut Ui, ppp: f32) {
        let t = self.theme;
        for l in &self.cube_labels {
            if l.alpha <= 0.0 {
                continue;
            }
            let ink = if l.hovered { t.on_ink } else { t.ink };
            let g = ui.fonts_mut(|f| f.layout_no_wrap(l.text.to_string(), semi(l.size), ink));
            // the text's top left turns about itself: its middle lands on the point
            let (half, turn) = (g.size() * 0.5, egui::emath::Rot2::from_angle(l.angle));
            let at = self.view_rect.min + vec2(l.at[0], l.at[1]) / ppp - turn * half;
            let shape = egui::epaint::TextShape::new(at, g, ink)
                .with_angle(l.angle)
                .with_opacity_factor(l.alpha);
            ui.painter().add(shape);
        }
    }

    /// Draws a flat clickable square with an icon, in the title bar's or the bar's colours.
    fn icon_button(
        &mut self,
        ui: &mut Ui,
        name: &str,
        r: Rect,
        icon: &str,
        tip: &str,
        ink: Color32,
    ) -> egui::Response {
        let resp = ui
            .interact(r, ui.scope_id().with(name), Sense::click())
            .on_hover_text(tip);
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tip));
        if resp.hovered() || resp.has_focus() {
            ui.painter().rect_filled(r, 4.0, ink.gamma_multiply(0.14));
        }
        if resp.has_focus() {
            ui.painter()
                .rect_stroke(r, 4.0, Stroke::new(2.0, self.theme.focus), StrokeKind::Inside);
        }
        icons::draw(
            ui.painter(),
            pos2(r.center().x - 8.0, r.center().y - 8.0),
            16.0,
            icon,
            1.5,
            ink,
        );
        self.note(name, r);
        resp
    }

    fn title_bar(&mut self, ui: &mut Ui, w: f32) {
        let t = self.theme;
        let bar = Rect::from_min_size(pos2(0.0, 0.0), vec2(w, TITLE_H));
        let p = ui.painter().clone();
        p.rect_filled(bar, 0.0, t.bar);

        // the bar itself moves the window; a double click maximizes
        let drag = ui.interact(bar, ui.scope_id().with("title-drag"), Sense::click_and_drag());
        if drag.double_clicked() {
            self.out.push(Action::Maximize);
        } else if drag.drag_started() {
            self.out.push(Action::DragWindow);
        }

        p.text(
            pos2(12.0, TITLE_H * 0.5),
            Align2::LEFT_CENTER,
            "ForgeCAD",
            semi(13.0),
            Color32::WHITE,
        );
        let quick = [
            ("new", "new", "New", Action::New),
            ("open", "open", "Open", Action::Open),
            ("undo", "undo", "Undo", Action::Undo),
            ("redo", "redo", "Redo", Action::Redo),
        ];
        for (k, (name, icon, tip, action)) in quick.into_iter().enumerate() {
            let r = Rect::from_min_size(pos2(84.0 + k as f32 * 30.0, 3.0), vec2(30.0, 30.0));
            if self.icon_button(ui, name, r, icon, tip, Color32::WHITE).clicked() {
                self.out.push(action);
            }
        }
        p.text(
            pos2(w * 0.5, TITLE_H * 0.5),
            Align2::CENTER_CENTER,
            &self.name,
            mono(12.0),
            Color32::WHITE,
        );

        let wins = [
            ("minimize", "minimize", "Minimize", Action::Minimize),
            ("maximize", "maximize", "Maximize", Action::Maximize),
            ("close", "close", "Close", Action::Close),
        ];
        for (k, (name, icon, tip, action)) in wins.into_iter().enumerate() {
            let r = Rect::from_min_size(pos2(w - 110.0 + k as f32 * 36.0, 3.0), vec2(34.0, 30.0));
            if self.icon_button(ui, name, r, icon, tip, Color32::WHITE).clicked() {
                self.out.push(action);
            }
        }
        let label = if t.dark { "Light" } else { "Dark" };
        let r = Rect::from_min_size(pos2(w - 170.0, 5.0), vec2(52.0, 26.0));
        let resp = ui.interact(r, ui.scope_id().with("theme"), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
        let edge = Color32::from_white_alpha(if resp.hovered() { 150 } else { 77 });
        p.rect_stroke(r, 4.0, Stroke::new(1.0, edge), StrokeKind::Inside);
        p.text(
            r.center(),
            Align2::CENTER_CENTER,
            label,
            sans(12.0),
            Color32::WHITE,
        );
        self.note("theme", r);
        if resp.clicked() {
            self.theme = if t.dark { theme::LIGHT } else { theme::DARK };
        }
    }

    fn tabs_row(&mut self, ui: &mut Ui, w: f32) {
        let t = self.theme;
        let p = ui.painter().clone();
        let y = TITLE_H;
        p.rect_filled(Rect::from_min_size(pos2(0.0, y), vec2(w, TABS_H)), 0.0, t.surface);
        let up = CornerRadius {
            nw: 4,
            ne: 4,
            sw: 0,
            se: 0,
        };
        let file = Rect::from_min_size(pos2(8.0, y + 4.0), vec2(52.0, 30.0));
        let resp = ui.interact(file, ui.scope_id().with("file"), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "File"));
        p.rect_filled(
            file,
            up,
            if resp.hovered() {
                t.accent.gamma_multiply(1.15)
            } else {
                t.accent
            },
        );
        p.text(
            file.center(),
            Align2::CENTER_CENTER,
            "File",
            semi(13.0),
            Color32::WHITE,
        );
        self.note("file", file);
        if resp.clicked() {
            self.file_menu = !self.file_menu;
        }
        let mut x = 66.0;
        for (k, name) in TABS.iter().enumerate() {
            let on = k == self.tab;
            let w = ui
                .fonts_mut(|f| f.layout_no_wrap(name.to_string(), sans(13.0), t.ink))
                .size()
                .x
                + 28.0;
            let r = Rect::from_min_size(pos2(x, y + 4.0), vec2(w, 30.0));
            let resp = ui.interact(r, ui.scope_id().with(("tab", k)), Sense::click());
            resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, on, *name));
            if resp.clicked() {
                self.tab = k;
            }
            if on {
                p.rect_filled(r, up, t.raised);
                p.rect_stroke(r, up, Stroke::new(1.0, t.line), StrokeKind::Inside);
                p.rect_filled(Rect::from_min_size(r.min, vec2(w, 2.0)), 0.0, t.focus);
            } else if resp.hovered() {
                p.rect_filled(r, up, t.hover);
            }
            let (font, ink) = if on {
                (semi(13.0), t.ink)
            } else {
                (sans(13.0), if resp.hovered() { t.ink } else { t.ink2 })
            };
            p.text(r.center(), Align2::CENTER_CENTER, *name, font, ink);
            self.note(format!("tab:{name}"), r);
            x += w + 2.0;
        }
    }

    fn side(&mut self, ui: &mut Ui, side: Rect) {
        let t = self.theme;
        let p = ui.painter().clone();
        p.rect_filled(side, 0.0, t.surface);
        p.vline(side.right(), side.y_range(), Stroke::new(1.0, t.line));
        let panel_h = self.panel_height();
        let tree_bottom = side.bottom() - panel_h;
        self.head(&p, side.top(), "Model", self.name.trim_end_matches(".fc"));
        let tree = Rect::from_min_max(pos2(0.0, side.top() + HEAD_H), pos2(SIDE_W, tree_bottom));
        self.tree(ui, tree);
        if panel_h > 0.0 {
            self.panel(
                ui,
                Rect::from_min_max(pos2(0.0, tree_bottom), pos2(SIDE_W, side.bottom())),
            );
        }
    }

    fn head(&self, p: &egui::Painter, y: f32, left: &str, right: &str) {
        let t = self.theme;
        let h = Rect::from_min_size(pos2(0.0, y), vec2(SIDE_W, HEAD_H));
        p.rect_filled(h, 0.0, t.raised);
        p.hline(0.0..=SIDE_W, y + HEAD_H - 0.5, Stroke::new(1.0, t.line));
        p.text(
            pos2(10.0, y + HEAD_H * 0.5),
            Align2::LEFT_CENTER,
            left,
            semi(12.0),
            t.ink,
        );
        p.text(
            pos2(SIDE_W - 10.0, y + HEAD_H * 0.5),
            Align2::RIGHT_CENTER,
            right,
            sans(12.0),
            t.ink2,
        );
    }

    fn tree(&mut self, ui: &mut Ui, tree: Rect) {
        let t = self.theme;
        let rows = std::mem::take(&mut self.rows);
        let selected = self.selected;
        let failed = self.failed.clone();
        let mut chosen = None;
        let mut places = Vec::new();
        ui.scope_builder(egui::UiBuilder::new().max_rect(tree), |ui| {
            egui::ScrollArea::vertical()
                .id_salt("tree")
                .auto_shrink(false)
                .show(ui, |ui| {
                    let (top, _) = ui.allocate_exact_size(vec2(tree.width(), 4.0), Sense::hover());
                    let _ = top;
                    let failed_at = failed
                        .as_ref()
                        .and_then(|(id, _)| rows.iter().position(|r| r.id == *id));
                    let mut origin = |ui: &mut Ui,
                                      label: &str,
                                      icon: &str,
                                      id: Option<ItemId>,
                                      dim: bool,
                                      why: Option<&str>,
                                      line: &str| {
                        let (r, resp) = ui.allocate_exact_size(vec2(tree.width(), ROW_H), Sense::click());
                        resp.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::Role::TreeItem,
                                true,
                                id.is_some() && id == selected,
                                label,
                            )
                        });
                        let on = id.is_some() && id == selected;
                        let p = ui.painter();
                        if on {
                            p.rect_filled(r, 0.0, t.on);
                            p.rect_filled(Rect::from_min_size(r.min, vec2(2.0, ROW_H)), 0.0, t.focus);
                        } else if resp.hovered() {
                            p.rect_filled(r, 0.0, t.hover);
                        }
                        if resp.has_focus() {
                            p.rect_stroke(r, 0.0, Stroke::new(2.0, t.focus), StrokeKind::Inside);
                        }
                        let ink = if on {
                            t.on_ink
                        } else if dim {
                            t.ink2
                        } else {
                            t.ink
                        };
                        icons::draw(p, pos2(r.left() + 8.0, r.top() + 5.0), 16.0, icon, 1.5, ink);
                        let font = if on { theme::medium(13.0) } else { sans(13.0) };
                        p.text(
                            pos2(r.left() + 31.0, r.center().y),
                            Align2::LEFT_CENTER,
                            label,
                            font,
                            ink,
                        );
                        if why.is_some() {
                            icons::draw(
                                p,
                                pos2(r.right() - 26.0, r.top() + 5.0),
                                16.0,
                                "warning",
                                1.5,
                                t.error,
                            );
                        }
                        let resp = match why {
                            Some(w) => resp.on_hover_text(format!("{line}\n{w}")),
                            None if !line.is_empty() => resp.on_hover_text(line),
                            None => resp,
                        };
                        if resp.clicked() {
                            chosen = Some(id);
                        }
                        r
                    };
                    origin(ui, "Origin", "folder", None, false, None, "");
                    for (k, row) in rows.iter().enumerate() {
                        let why = (Some(k) == failed_at)
                            .then(|| failed.as_ref().map(|f| f.1.as_str()))
                            .flatten();
                        let dim = failed_at.is_some_and(|f| k > f);
                        let r = origin(ui, &row.title, row.icon, Some(row.id), dim, why, &row.line);
                        places.push((format!("tree:{}", row.name), r));
                    }
                    let (r, _) = ui.allocate_exact_size(vec2(tree.width(), 28.0), Sense::hover());
                    let p = ui.painter();
                    let mut x = r.left() + 8.0;
                    while x < r.right() - 8.0 {
                        p.hline(x..=x + 3.0, r.top() + 3.0, Stroke::new(1.0, t.line));
                        x += 6.0;
                    }
                    icons::draw(p, pos2(r.left() + 8.0, r.top() + 9.0), 16.0, "end", 1.5, t.ink2);
                    p.text(
                        pos2(r.left() + 31.0, r.top() + 17.0),
                        Align2::LEFT_CENTER,
                        "End of part",
                        sans(12.0),
                        t.ink2,
                    );
                });
        });
        self.rows = rows;
        self.places.extend(places);
        if let Some(c) = chosen {
            self.selected = c;
        }
    }

    fn panel_rows(&self) -> Vec<(String, String)> {
        let Some(id) = self.selected else {
            return Vec::new();
        };
        let Some(doc) = &self.doc else { return Vec::new() };
        let Some(item) = doc.items.iter().find(|i| i.id == id) else {
            return Vec::new();
        };
        match &item.body {
            Body::Feature { kind, args } => {
                let words: Vec<&str> = args.iter().map(|a| a.text.as_str()).collect();
                panel::fields(kind, &words)
            }
            Body::Sketch { rows, .. } => rows.iter().filter_map(|r| panel::sketch_row(&r.args)).collect(),
            _ => Vec::new(),
        }
    }

    fn panel_height(&self) -> f32 {
        if self.selected.is_none() {
            return 0.0;
        }
        (HEAD_H + 8.0 + self.panel_rows().len().min(9) as f32 * 34.0 + 8.0).min(340.0)
    }

    fn panel(&mut self, ui: &mut Ui, r: Rect) {
        let t = self.theme;
        let p = ui.painter().clone();
        p.hline(r.x_range(), r.top(), Stroke::new(1.0, t.line));
        let title = self
            .rows
            .iter()
            .find(|w| Some(w.id) == self.selected)
            .map_or(String::new(), |w| w.title.clone());
        self.head(&p, r.top(), &title, "");
        let rows = self.panel_rows();
        let body = Rect::from_min_max(pos2(r.left(), r.top() + HEAD_H), r.max);
        ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
            egui::ScrollArea::vertical()
                .id_salt("panel")
                .auto_shrink(false)
                .show(ui, |ui| {
                    ui.add_space(8.0);
                    for (label, value) in &rows {
                        let (row, _) = ui.allocate_exact_size(vec2(SIDE_W, 34.0), Sense::hover());
                        ui.painter().text(
                            pos2(row.left() + 10.0, row.center().y),
                            Align2::LEFT_CENTER,
                            label,
                            sans(12.0),
                            t.ink2,
                        );
                        let field =
                            Rect::from_min_size(pos2(row.left() + 96.0, row.top() + 4.0), vec2(164.0, 26.0));
                        let mut shown: &str = value;
                        ui.put(
                            field,
                            egui::TextEdit::singleline(&mut shown)
                                .font(mono(12.0))
                                .desired_width(148.0)
                                .margin(egui::vec2(7.0, 6.0)),
                        );
                    }
                });
        });
    }

    fn docs_and_status(&mut self, ui: &mut Ui, s: Rect, bottom: f32) {
        let t = self.theme;
        let p = ui.painter().clone();
        p.rect_filled(
            Rect::from_min_size(pos2(0.0, bottom), vec2(s.width(), DOCS_H)),
            0.0,
            t.surface,
        );
        p.hline(0.0..=s.width(), bottom, Stroke::new(1.0, t.line));
        if !self.name.is_empty() {
            let g = ui.fonts_mut(|f| f.layout_no_wrap(self.name.clone(), mono(12.0), t.ink));
            let d = Rect::from_min_size(pos2(8.0, bottom + 3.0), vec2(g.size().x + 24.0, 22.0));
            p.rect_filled(d, 3.0, t.raised);
            p.rect_stroke(d, 3.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
            p.galley(pos2(d.left() + 12.0, d.center().y - g.size().y * 0.5), g, t.ink);
        }
        let st = Rect::from_min_size(pos2(0.0, bottom + DOCS_H), vec2(s.width(), STATUS_H));
        p.rect_filled(st, 0.0, t.bar);
        p.text(
            pos2(10.0, st.center().y),
            Align2::LEFT_CENTER,
            &self.status,
            sans(12.0),
            Color32::WHITE,
        );
        let right = if self.failed.is_some() {
            "error | mm"
        } else {
            "valid | mm"
        };
        p.text(
            pos2(s.width() - 10.0, st.center().y),
            Align2::RIGHT_CENTER,
            right,
            mono(12.0),
            Color32::WHITE,
        );
    }

    fn navbar(&mut self, ctx: &egui::Context) {
        let t = self.theme;
        let v = self.view_rect;
        let at = pos2(v.right() - NAVBAR_W - 12.0, v.top() + NAVBAR_TOP);
        let items = [
            (
                "nav:orbit",
                "orbit",
                "Orbit",
                Action::Tool(NavTool::Orbit),
                Some(NavTool::Orbit),
            ),
            (
                "nav:pan",
                "pan",
                "Pan",
                Action::Tool(NavTool::Pan),
                Some(NavTool::Pan),
            ),
            (
                "nav:zoom",
                "zoom",
                "Zoom",
                Action::Tool(NavTool::Zoom),
                Some(NavTool::Zoom),
            ),
            ("nav:fit", "fit", "Fit All", Action::FitAll, None),
        ];
        let tool = self.view.tool;
        egui::Area::new(egui::Id::unique("navbar"))
            .fixed_pos(at)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                let (bar, _) = ui.allocate_exact_size(vec2(NAVBAR_W, 4.0 * 30.0 + 2.0), Sense::hover());
                ui.painter().rect_filled(bar, 4.0, t.surface);
                ui.painter()
                    .rect_stroke(bar, 4.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
                for (k, (name, icon, tip, action, is)) in items.into_iter().enumerate() {
                    let r = Rect::from_min_size(
                        pos2(bar.left() + 1.0, bar.top() + 1.0 + k as f32 * 30.0),
                        vec2(30.0, 30.0),
                    );
                    let on = is == Some(tool);
                    if on {
                        ui.painter().rect_filled(r.shrink(2.0), 3.0, t.on);
                    }
                    let ink = if on { t.on_ink } else { t.ink };
                    let resp = self.icon_button(ui, name, r, icon, tip, ink);
                    if resp.clicked() {
                        self.out.push(action);
                    }
                }
            });
    }

    /// With no title bar from the system, the window's edges resize it.
    fn resize_edges(&mut self, ui: &mut Ui, s: Rect) {
        if self.view.maximized {
            return;
        }
        let Some(at) = ui.ctx().input(|i| i.pointer.hover_pos()) else {
            return;
        };
        let (l, r) = (at.x < RESIZE_EDGE, at.x > s.width() - RESIZE_EDGE);
        let (top, b) = (at.y < RESIZE_EDGE, at.y > s.height() - RESIZE_EDGE);
        let edge = match (top, b, l, r) {
            (true, _, true, _) => Edge::NW,
            (true, _, _, true) => Edge::NE,
            (_, true, true, _) => Edge::SW,
            (_, true, _, true) => Edge::SE,
            (true, ..) => Edge::N,
            (_, true, ..) => Edge::S,
            (_, _, true, _) => Edge::W,
            (_, _, _, true) => Edge::E,
            _ => return,
        };
        let icon = match edge {
            Edge::N | Edge::S => egui::CursorIcon::ResizeVertical,
            Edge::E | Edge::W => egui::CursorIcon::ResizeHorizontal,
            Edge::NE | Edge::SW => egui::CursorIcon::ResizeNeSw,
            Edge::NW | Edge::SE => egui::CursorIcon::ResizeNwSe,
        };
        ui.ctx().set_cursor_icon(icon);
        if ui.ctx().input(|i| i.pointer.primary_pressed()) {
            self.out.push(Action::ResizeWindow(edge));
        }
    }

    fn menus(&mut self, ctx: &egui::Context) {
        let t = self.theme;
        if self.file_menu {
            let at = pos2(8.0, TITLE_H + TABS_H);
            let items = [("New", Some(Action::New)), ("Open...", Some(Action::Open))];
            match menu_at(ctx, "file-menu", at, &t, &items) {
                MenuResult::Chose(a) => {
                    self.out.push(a);
                    self.file_menu = false;
                }
                MenuResult::Close => self.file_menu = false,
                MenuResult::Open => {}
            }
        }
        if let Some(at) = self.cube_menu {
            let parallel = self.view.parallel;
            let items = [
                ("Home", Some(Action::Home)),
                ("Set Current View as Home", Some(Action::SetHome)),
                (
                    if parallel {
                        "\u{2713} Orthographic"
                    } else {
                        "Orthographic"
                    },
                    Some(Action::Parallel(true)),
                ),
                (
                    if parallel {
                        "Perspective"
                    } else {
                        "\u{2713} Perspective"
                    },
                    Some(Action::Parallel(false)),
                ),
            ];
            match menu_at(ctx, "cube-menu", at, &t, &items) {
                MenuResult::Chose(a) => {
                    self.out.push(a);
                    self.cube_menu = None;
                }
                MenuResult::Close => self.cube_menu = None,
                MenuResult::Open => {}
            }
        }
    }
}

enum MenuResult {
    Open,
    Close,
    Chose(Action),
}

/// A flat menu at a point; closes on Esc or a press outside it.
fn menu_at(
    ctx: &egui::Context,
    id: &str,
    at: egui::Pos2,
    t: &Theme,
    items: &[(&str, Option<Action>)],
) -> MenuResult {
    let mut out = MenuResult::Open;
    let w = 220.0;
    let inner = egui::Area::new(egui::Id::unique(id))
        .fixed_pos(at)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let (panel, _) =
                ui.allocate_exact_size(vec2(w, 8.0 + items.len() as f32 * ROW_H), Sense::click());
            ui.painter().rect_filled(panel, 4.0, t.surface);
            ui.painter()
                .rect_stroke(panel, 4.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
            for (k, (label, action)) in items.iter().enumerate() {
                let r = Rect::from_min_size(
                    pos2(panel.left() + 1.0, panel.top() + 4.0 + k as f32 * ROW_H),
                    vec2(w - 2.0, ROW_H),
                );
                let sense = if action.is_some() {
                    Sense::click()
                } else {
                    Sense::hover()
                };
                let resp = ui.interact(r, ui.scope_id().with(k), sense);
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, action.is_some(), *label));
                if (resp.hovered() || resp.has_focus()) && action.is_some() {
                    ui.painter().rect_filled(r, 0.0, t.on);
                }
                let ink = if action.is_none() {
                    t.ink2
                } else if resp.hovered() || resp.has_focus() {
                    t.on_ink
                } else {
                    t.ink
                };
                ui.painter().text(
                    pos2(r.left() + 12.0, r.center().y),
                    Align2::LEFT_CENTER,
                    *label,
                    sans(13.0),
                    ink,
                );
                if resp.clicked()
                    && let Some(a) = action
                {
                    out = MenuResult::Chose(*a);
                }
            }
        });
    let outside = ctx.input(|i| {
        i.pointer.any_pressed()
            && i.pointer
                .interact_pos()
                .is_some_and(|p| !inner.response.rect.contains(p))
    });
    if matches!(out, MenuResult::Open) && (outside || ctx.input(|i| i.key_pressed(egui::Key::Escape))) {
        out = MenuResult::Close;
    }
    out
}
