//! Open documents, Inventor's way: one active, each with its own browser, view, selection and
//! undo. The active one's state lives in `Shell`'s (and `Gui`'s) own fields, so nothing that reads
//! them changes; an idle one's is swapped out into a `ShellDoc`.

use super::Shell;
use crate::shell::placed::Placed;

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
pub(crate) use per_doc;

per_doc!(
    Shell,
    ShellDoc,
    [
        tab: usize,
        name: String,
        versions: super::versions::Versions,
        status: String,
        view_tex: Option<egui::TextureId>,
        cube_labels: Vec<crate::viewcube::Label>,
        cube_place: Option<crate::viewcube::Place>,
        camera: Option<(crate::camera::Camera, (u32, u32))>,
        origin_shown: [bool; 7],
        origin_hover: Option<crate::origin::Item>,
        origin_selected: Option<crate::origin::Item>,
        unshown: bool,
        units: &'static str,
        part_middle: [f32; 3],
        sketch: Option<super::sketching::SketchMode>,
        sketch_pick: bool,
        content: super::content::ContentCenter,
        fg: Option<super::fg::FgPanel>,
        fg_dim: Option<super::fgdim::FgDim>,
        sheet: super::sheet::SheetUi,
        env: Option<super::env::EnvId>,
        studio: super::studio_env::StudioUi,
        inspect: super::inspect::Inspect,
        drawing: Option<super::drawing::DrawingEnv>,
        new_drawing: Option<super::drawing::NewDrawing>,
        dwgdoc: Option<super::dwgdoc::DwgDoc>,
        camsim: Option<Box<crate::gui_camsim_panel::CamSimUi>>,
        camsim_job: crate::gui_camsim_panel::SimJob,
        cam: Option<super::cam::CamWs>,
    ],
    app [
        theme, view, out, view_rect, places, cube_menu, cube_menu_fresh, cube_region, marking, window_box,
        view_blocks, view_owned, side_px, overlays, file_menu, style_set, settings,
        prefs, doc_tabs, active_doc, close_ask, open_paths, open_blocks, doc_scroll, doc_drag, agent,
        machine,
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
            versions: Default::default(),
            status: String::new(),
            view_tex: None,
            cube_labels: vec![],
            cube_place: None,
            camera: None,
            origin_shown: [false; 7],
            origin_hover: None,
            origin_selected: None,
            unshown: false,
            units: "mm",
            part_middle: [0.0; 3],
            sketch: None,
            sketch_pick: false,
            content: Default::default(),
            fg: None,
            fg_dim: None,
            sheet: super::sheet::SheetUi::new(),
            env: None,
            studio: Default::default(),
            inspect: Default::default(),
            drawing: None,
            new_drawing: None,
            dwgdoc: None,
            camsim: None,
            camsim_job: Default::default(),
            cam: None,
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
    /// A DWG file.
    Dwg,
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
        let font = super::theme::small();
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
            let r1 = super::theme::R1;
            p.rect_filled(tab, r1, fill);
            p.rect_stroke(tab, r1, Stroke::new(1.0, t.line), StrokeKind::Inside);
            // the one up: the ribbon's chosen-tab rule along its top
            if active {
                let top = egui::Rect::from_min_size(tab.min, vec2(tab.width(), 2.0));
                p.rect_filled(
                    top,
                    egui::CornerRadius {
                        nw: r1,
                        ne: r1,
                        sw: 0,
                        se: 0,
                    },
                    t.accent,
                );
            }
            let icon = match d.kind {
                DocKind::Part => "part",
                DocKind::Assembly => "subasm",
                DocKind::Drawing => "dwgbase",
                DocKind::Dwg => "polyline",
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
        let placed_before = self.places.len();
        let (_, sizing) = egui::Window::new("Save changes?")
            .id(egui::Id::unique("doc-close-ask"))
            .frame(super::fgasm::dialog_frame(&self.theme))
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .default_pos(area.center() - egui::vec2(170.0, 50.0))
            .constrain_to(area)
            .show_sizing(ctx, |ui| {
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
        // what it named unseen, sizing itself, is off
        if sizing {
            self.places.truncate(placed_before);
        }
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
