//! Open documents, Inventor's way: one active, each with its own browser, view, selection and
//! undo. The active one's state lives in `Shell`'s (and `Gui`'s) own fields, so nothing that reads
//! them changes; an idle one's is swapped out into a `ShellDoc`.

use super::Shell;

/// Splits a struct's fields into the document's and the app's. The document's go into `$doc`,
/// swapped in and out whole; the app's stay. Every field must be named in one list or the other,
/// so a field added later fails to build until it's placed.
macro_rules! per_doc {
    ($(#[$m:meta])* $host:ident, $doc:ident, [$($f:ident: $t:ty),* $(,)?], app [$($a:ident),* $(,)?]) => {
        $(#[$m])*
        pub struct $doc {
            $(pub(crate) $f: $t,)*
        }

        impl $host {
            /// This document's state in, the one that was in out to `d`.
            pub fn swap_doc(&mut self, d: &mut $doc) {
                $(std::mem::swap(&mut self.$f, &mut d.$f);)*
            }

            #[allow(dead_code)]
            fn every_field_placed(&self) {
                let $host { $($f: _,)* $($a: _,)* } = self;
            }
        }
    };
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use per_doc;

per_doc!(
    Shell,
    ShellDoc,
    [
        tab: usize,
        name: String,
        doc: Option<forge_doc::Document>,
        rows: Vec<super::Row>,
        failed: Option<(forge_doc::ItemId, String)>,
        selected: Option<forge_doc::ItemId>,
        version: u64,
        status: String,
        view_tex: Option<egui::TextureId>,
        cube_labels: Vec<crate::viewcube::Label>,
        cube_place: Option<crate::viewcube::Place>,
        camera: Option<(crate::camera::Camera, (u32, u32))>,
        origin_open: bool,
        origin_shown: [bool; 7],
        origin_hover: Option<crate::origin::Item>,
        origin_tree_hover: Option<crate::origin::Item>,
        origin_selected: Option<crate::origin::Item>,
        tree_hover: Option<forge_doc::ItemId>,
        end_at: Option<forge_doc::ItemId>,
        renaming: Option<(forge_doc::ItemId, String)>,
        renamed: Option<(forge_doc::ItemId, String)>,
        sketch_shapes: Vec<forge_agent::handles::SketchShape>,
        sketch_eye: std::collections::HashMap<String, bool>,
        sketch_hover: Option<forge_doc::ItemId>,
        unshown: bool,
        flags: Vec<forge_agent::live::Flag>,
        checking_since: Option<f64>,
        units: &'static str,
        part_middle: [f32; 3],
        sketch: Option<super::sketching::SketchMode>,
        sketch_pick: bool,
        open_rows: std::collections::HashSet<forge_doc::ItemId>,
        then_feature: Option<&'static str>,
        dialog: Option<super::dialog::Dialog>,
        dim: Option<super::dim::DimEdit>,
        content: super::content::ContentCenter,
        dim_typed: Option<String>,
        fg: Option<super::fg::FgPanel>,
        sheet: super::sheet::SheetUi,
        inspect: super::inspect::Inspect,
        drawing: Option<super::drawing::DrawingEnv>,
        new_drawing: Option<super::drawing::NewDrawing>,
    ],
    app [
        theme, view, out, view_rect, places, cube_menu, cube_menu_fresh, cube_region, marking,
        marking_row, window_box, view_blocks, side_px, overlays, file_menu, style_set, settings,
        prefs, doc_tabs, active_doc, close_ask, open_paths, doc_scroll, doc_drag, agent,
    ]
);

impl Default for ShellDoc {
    /// A document with nothing in it: as a new shell has it.
    fn default() -> ShellDoc {
        let mut d = ShellDoc::blank();
        Shell::default().swap_doc(&mut d);
        d
    }
}

impl ShellDoc {
    /// Placeholders, swapped out at once.
    fn blank() -> ShellDoc {
        ShellDoc {
            tab: 0,
            name: String::new(),
            doc: None,
            rows: vec![],
            failed: None,
            selected: None,
            version: 0,
            status: String::new(),
            view_tex: None,
            cube_labels: vec![],
            cube_place: None,
            camera: None,
            origin_open: false,
            origin_shown: [false; 7],
            origin_hover: None,
            origin_tree_hover: None,
            origin_selected: None,
            tree_hover: None,
            end_at: None,
            renaming: None,
            renamed: None,
            sketch_shapes: vec![],
            sketch_eye: Default::default(),
            sketch_hover: None,
            unshown: false,
            flags: vec![],
            checking_since: None,
            units: "mm",
            part_middle: [0.0; 3],
            sketch: None,
            sketch_pick: false,
            open_rows: Default::default(),
            then_feature: None,
            dialog: None,
            dim: None,
            content: Default::default(),
            dim_typed: None,
            fg: None,
            sheet: super::sheet::SheetUi::new(),
            inspect: Default::default(),
            drawing: None,
            new_drawing: None,
        }
    }
}

/// What a document is, for its tab's icon.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DocKind {
    #[default]
    Part,
    Assembly,
    Drawing,
}

/// A tab in the strip along the bottom.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocTab {
    pub title: String,
    /// Changes not saved.
    pub dirty: bool,
    pub kind: DocKind,
}

/// What the tabs and their keys ask of the host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocCmd {
    Switch(usize),
    /// Ctrl+Tab, Ctrl+Shift+Tab.
    Next,
    Prev,
    /// Its ×, a middle click: asked first if it has changes.
    Close(usize),
    /// Ctrl+W.
    CloseActive,
    /// Closed now, saved first or not: the unsaved prompt's answer.
    CloseNow(usize, bool),
    /// Dragged from one place in the strip to another.
    Move(usize, usize),
}

const TAB_H: f32 = 22.0;
const TAB_GAP: f32 = 4.0;
const TAB_MAX_W: f32 = 220.0;

impl Shell {
    /// Ctrl+Tab and Ctrl+Shift+Tab go round the documents, Ctrl+W closes the one up.
    pub(super) fn doc_keys(&mut self, ui: &egui::Ui) {
        use egui::{Key, Modifiers};
        let ctrl_shift = Modifiers::CTRL | Modifiers::SHIFT;
        let (prev, next, close) = ui.input_mut(|i| {
            let prev = i.consume_key(ctrl_shift, Key::Tab);
            let next = i.consume_key(Modifiers::CTRL, Key::Tab);
            let close = i.consume_key(Modifiers::CTRL, Key::W);
            (prev, next, close)
        });
        if prev {
            self.out.push(super::Action::Doc(DocCmd::Prev));
        }
        if next {
            self.out.push(super::Action::Doc(DocCmd::Next));
        }
        if close && !self.doc_tabs.is_empty() {
            self.out.push(super::Action::Doc(DocCmd::CloseActive));
        }
    }

    /// One tab per open document in the strip `r`: click for it, × or a middle click to close it,
    /// drag to put it elsewhere; the wheel scrolls a strip too full to show them all.
    pub(super) fn doc_strip(&mut self, ui: &mut egui::Ui, r: egui::Rect) {
        use egui::{Align2, Sense, Stroke, StrokeKind, pos2, vec2};
        let t = self.theme;
        let p = ui.painter().with_clip_rect(r);
        let font = super::theme::mono(12.0);
        let widths: Vec<f32> = self
            .doc_tabs
            .iter()
            .map(|d| {
                let w = ui.fonts_mut(|f| f.layout_no_wrap(d.title.clone(), font.clone(), t.ink).size().x);
                (w + 64.0).min(TAB_MAX_W)
            })
            .collect();
        let total: f32 = widths.iter().map(|w| w + TAB_GAP).sum::<f32>() + 8.0;
        let room = r.width();
        let strip = ui.interact(r, ui.scope_id().with("doc-strip"), Sense::hover());
        if strip.hovered() {
            let d = ui.input(|i| i.smooth_scroll_delta);
            self.doc_scroll -= d.x + d.y;
        }
        // the active tab always in sight
        let mut x0 = 8.0;
        for (k, w) in widths.iter().enumerate() {
            if k == self.active_doc {
                if x0 - self.doc_scroll < 0.0 {
                    self.doc_scroll = x0 - 8.0;
                } else if x0 + w - self.doc_scroll > room {
                    self.doc_scroll = x0 + w - room + 8.0;
                }
            }
            x0 += w + TAB_GAP;
        }
        self.doc_scroll = self.doc_scroll.clamp(0.0, (total - room).max(0.0));
        let mut x = r.left() + 8.0 - self.doc_scroll;
        let mut asks = vec![];
        let mut rects = vec![];
        for (k, d) in self.doc_tabs.iter().enumerate() {
            let w = widths[k];
            let tab = egui::Rect::from_min_size(pos2(x, r.center().y - TAB_H * 0.5), vec2(w, TAB_H));
            rects.push(tab);
            x += w + TAB_GAP;
            let active = k == self.active_doc;
            let id = ui.scope_id().with(("doc-tab", k));
            let resp = ui.interact(tab.intersect(r), id, Sense::click_and_drag());
            let close_r =
                egui::Rect::from_center_size(pos2(tab.right() - 12.0, tab.center().y), vec2(16.0, 16.0));
            let close = ui.interact(close_r.intersect(r), id.with("x"), Sense::click());
            let fill = if active {
                t.raised
            } else if resp.hovered() {
                t.hover
            } else {
                t.surface
            };
            p.rect_filled(tab, 3.0, fill);
            p.rect_stroke(
                tab,
                3.0,
                Stroke::new(1.0, if active { t.accent } else { t.line }),
                StrokeKind::Inside,
            );
            let icon = match d.kind {
                DocKind::Part => "part",
                DocKind::Assembly => "subasm",
                DocKind::Drawing => "dwgbase",
            };
            let ink = if active { t.ink } else { t.ink2 };
            super::icons::draw(
                &p,
                pos2(tab.left() + 6.0, tab.center().y - 7.0),
                14.0,
                icon,
                1.25,
                ink,
            );
            let text_r = egui::Rect::from_min_max(
                pos2(tab.left() + 24.0, tab.top()),
                pos2(close_r.left() - if d.dirty { 12.0 } else { 2.0 }, tab.bottom()),
            );
            let g = ui.fonts_mut(|f| {
                let mut job = egui::text::LayoutJob::simple_singleline(d.title.clone(), font.clone(), ink);
                job.wrap = egui::text::TextWrapping::truncate_at_width(text_r.width());
                f.layout_job(job)
            });
            p.with_clip_rect(text_r.intersect(r)).galley(
                pos2(text_r.left(), tab.center().y - g.size().y * 0.5),
                g,
                ink,
            );
            if d.dirty {
                p.circle_filled(pos2(close_r.left() - 6.0, tab.center().y), 3.0, t.accent);
            }
            if close.hovered() {
                p.rect_filled(close_r, 3.0, t.hover);
            }
            if active || resp.hovered() || close.hovered() {
                super::icons::draw(&p, close_r.min, 16.0, "close", 1.5, ink);
            }
            self.places.push((format!("doc:tab:{}", d.title), tab));
            self.places.push((format!("doc:close:{}", d.title), close_r));
            if close.clicked() || resp.middle_clicked() {
                asks.push(DocCmd::Close(k));
            } else if resp.clicked() && !active {
                asks.push(DocCmd::Switch(k));
            }
            if resp.drag_started() {
                self.doc_drag = Some(k);
                if !active {
                    asks.push(DocCmd::Switch(k));
                }
            }
            resp.on_hover_text(&d.title);
        }
        // a dragged tab takes the place of the one it's over
        if let Some(from) = self.doc_drag {
            if !ui.input(|i| i.pointer.any_down()) {
                self.doc_drag = None;
            } else if let Some(at) = ui.input(|i| i.pointer.interact_pos())
                && let Some(to) = rects.iter().position(|q| q.x_range().contains(at.x))
                && to != from
            {
                asks.push(DocCmd::Move(from, to));
                self.doc_drag = Some(to);
            }
        }
        if total > room {
            // a fade at each end there's more past
            let fade = |x: f32, a: Align2| {
                p.text(pos2(x, r.center().y), a, "…", font.clone(), t.ink2);
            };
            if self.doc_scroll > 0.0 {
                fade(r.left() + 1.0, Align2::LEFT_CENTER);
            }
            if self.doc_scroll < total - room {
                fade(r.right() - 1.0, Align2::RIGHT_CENTER);
            }
        }
        self.out.extend(asks.into_iter().map(super::Action::Doc));
    }

    /// Closing a tab with changes not saved: Save, Don't Save, Cancel, in the window.
    pub(super) fn close_prompt(&mut self, ctx: &egui::Context) {
        let Some(k) = self.close_ask else { return };
        let Some(d) = self.doc_tabs.get(k) else {
            self.close_ask = None;
            return;
        };
        let title = d.title.clone();
        let mut answer: Option<Option<bool>> = None;
        let area = super::dialog_area(ctx);
        egui::Window::new("Save changes?")
            .id(egui::Id::unique("doc-close-ask"))
            .frame(super::fgasm::dialog_frame(&self.theme))
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .default_pos(area.center() - egui::vec2(170.0, 50.0))
            .constrain_to(area)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(format!("Save changes to {title} before closing?"))
                        .font(super::theme::sans(13.0)),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let mut b = |ui: &mut egui::Ui, label: &str, name: &str, a: Option<bool>| {
                        let r = ui.button(label);
                        self.places.push((format!("doc:ask:{name}"), r.rect));
                        if r.clicked() {
                            answer = Some(a);
                        }
                    };
                    b(ui, "  Save  ", "save", Some(true));
                    b(ui, "Don't Save", "discard", Some(false));
                    b(ui, "Cancel", "cancel", None);
                });
            });
        if ui_escape(ctx) {
            answer = Some(None);
        }
        match answer {
            Some(Some(save)) => {
                self.close_ask = None;
                self.out.push(super::Action::Doc(DocCmd::CloseNow(k, save)));
            }
            Some(None) => self.close_ask = None,
            None => {}
        }
    }
}

fn ui_escape(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.key_pressed(egui::Key::Escape))
}
