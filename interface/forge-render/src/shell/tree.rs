//! The feature tree: Origin, which opens to its planes, axes and point, then a row per feature, then
//! the end of the part. A click selects, a double click opens the feature's dialog.

use super::theme::{medium, sans};
use super::{ROW_H, Shell, icons};
use crate::origin::{self, Item};
use egui::{Align2, Color32, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use forge_doc::ItemId;

const INDENT: f32 = 18.0;

struct RowLook<'a> {
    label: &'a str,
    icon: &'a str,
    tint: Option<Color32>,
    on: bool,
    dim: bool,
    depth: f32,
}

impl Shell {
    fn tree_row(&self, ui: &mut Ui, w: f32, look: RowLook) -> (Rect, egui::Response) {
        let t = self.theme;
        let (r, resp) = ui.allocate_exact_size(vec2(w, ROW_H), Sense::click_and_drag());
        resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::TreeItem, true, look.on, look.label));
        let p = ui.painter();
        if look.on {
            p.rect_filled(r, 0.0, t.on);
            p.rect_filled(Rect::from_min_size(r.min, vec2(2.0, ROW_H)), 0.0, t.focus);
        } else if resp.hovered() {
            p.rect_filled(r, 0.0, t.hover);
        }
        if resp.has_focus() {
            p.rect_stroke(r, 0.0, Stroke::new(2.0, t.focus), StrokeKind::Inside);
        }
        let ink = if look.on {
            t.on_ink
        } else if look.dim {
            t.ink2
        } else {
            t.ink
        };
        let x = r.left() + 8.0 + look.depth * INDENT;
        icons::draw(
            p,
            pos2(x, r.top() + 5.0),
            16.0,
            look.icon,
            1.5,
            look.tint.unwrap_or(ink),
        );
        let font = if look.on { medium(13.0) } else { sans(13.0) };
        p.text(
            pos2(x + 23.0, r.center().y),
            Align2::LEFT_CENTER,
            look.label,
            font,
            ink,
        );
        (r, resp)
    }

    fn eye_rect(row: Rect) -> Rect {
        Rect::from_min_size(pos2(row.right() - 28.0, row.top() + 3.0), vec2(20.0, 20.0))
    }

    /// An eye at a row's right: shown or hidden, kept apart from the row's own click.
    fn eye(&self, ui: &mut Ui, row: Rect, key: usize, shown: bool) -> bool {
        let t = self.theme;
        let r = Self::eye_rect(row);
        let resp = ui.interact(r, ui.scope_id().with(("eye", key)), Sense::click());
        let tip = if shown { "Hide" } else { "Show" };
        let resp = resp.on_hover_text(tip);
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tip));
        let ink = if shown || resp.hovered() {
            t.ink
        } else {
            t.ink2.gamma_multiply(0.6)
        };
        icons::draw(
            ui.painter(),
            pos2(r.left() + 2.0, r.top() + 2.0),
            16.0,
            if shown { "eye" } else { "hide" },
            1.25,
            ink,
        );
        resp.clicked()
    }

    pub(super) fn tree(&mut self, ui: &mut Ui, tree: Rect) {
        let t = self.theme;
        let rows = std::mem::take(&mut self.rows);
        let failed = self.failed.clone();
        let w = tree.width();
        let (mut chosen, mut opened): (Option<Option<ItemId>>, Option<ItemId>) = (None, None);
        let mut chose_origin = None;
        let mut hover_origin = None;
        let mut toggles = Vec::new();
        let mut all_shown = None;
        let mut places: Vec<(String, Rect)> = Vec::new();
        let mut toggle_open = false;
        let mut toggle_row: Option<ItemId> = None;
        let mut child_edit: Option<super::Action> = None;
        let mut tree_hover = None;
        let mut menu = None;
        let mut moved = None;
        let mut end_to = None;
        let (mut rename_to, mut stop_rename, mut typed_name) = (None, false, None);
        ui.scope_builder(egui::UiBuilder::new().max_rect(tree), |ui| {
            egui::ScrollArea::vertical()
                .id_salt("tree")
                .auto_shrink(false)
                .show(ui, |ui| {
                    ui.allocate_exact_size(vec2(w, 4.0), Sense::hover());
                    let failed_at = failed
                        .as_ref()
                        .and_then(|(id, _)| rows.iter().position(|r| r.id == *id));

                    let shown_any = self.origin_shown.iter().any(|s| *s);
                    let (r, resp) = self.tree_row(
                        ui,
                        w,
                        RowLook {
                            label: "Origin",
                            icon: "folder",
                            tint: None,
                            on: false,
                            dim: false,
                            depth: 0.5,
                        },
                    );
                    places.push(("tree:Origin".into(), r));
                    icons::draw(
                        ui.painter(),
                        pos2(r.left() + 2.0, r.top() + 7.0),
                        12.0,
                        if self.origin_open { "chevron" } else { "right" },
                        1.5,
                        t.ink2,
                    );
                    if resp.clicked() {
                        toggle_open = true;
                    }
                    if self.eye(ui, r, 100, shown_any) {
                        all_shown = Some(!shown_any);
                    }
                    places.push((
                        "tree:Origin:eye".into(),
                        Rect::from_min_size(pos2(r.right() - 28.0, r.top() + 3.0), vec2(20.0, 20.0)),
                    ));
                    if self.origin_open {
                        for (k, item) in origin::ITEMS.into_iter().enumerate() {
                            let rgb = item.colour();
                            let (r, resp) = self.tree_row(
                                ui,
                                w,
                                RowLook {
                                    label: item.name(),
                                    icon: item.icon(),
                                    tint: Some(Color32::from_rgb(
                                        (rgb >> 16) as u8,
                                        (rgb >> 8) as u8,
                                        rgb as u8,
                                    )),
                                    on: self.origin_selected == Some(item),
                                    dim: false,
                                    depth: 1.0,
                                },
                            );
                            places.push((format!("tree:{}", item.name()), r));
                            let resp = resp.on_hover_text(item.reference());
                            if resp.hovered() {
                                hover_origin = Some(item);
                            }
                            if resp.clicked() {
                                chose_origin = Some(item);
                            }
                            if self.eye(ui, r, k, self.origin_shown[k]) {
                                toggles.push(k);
                            }
                            places.push((
                                format!("tree:{}:eye", item.name()),
                                Rect::from_min_size(pos2(r.right() - 28.0, r.top() + 3.0), vec2(20.0, 20.0)),
                            ));
                        }
                    }

                    let end_row = self.end_at.and_then(|e| rows.iter().position(|r| r.id == e));
                    let mut tops: Vec<f32> = Vec::with_capacity(rows.len());
                    let end_mark = |ui: &mut Ui, places: &mut Vec<(String, Rect)>| {
                        let (r, resp) = ui.allocate_exact_size(vec2(w, 28.0), Sense::click_and_drag());
                        resp.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::Role::Button, true, "End of part")
                        });
                        let resp = resp.on_hover_cursor(egui::CursorIcon::Grab);
                        let p = ui.painter();
                        let ink = if resp.hovered() || resp.dragged() {
                            t.focus
                        } else {
                            t.line
                        };
                        let mut x = r.left() + 8.0;
                        while x < r.right() - 8.0 {
                            p.hline(x..=x + 3.0, r.top() + 3.0, Stroke::new(1.0, ink));
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
                        places.push(("tree:End of part".into(), r));
                        resp
                    };
                    let mut end_resp = None;
                    let mut row_drag: Option<(usize, egui::Response)> = None;
                    for (k, row) in rows.iter().enumerate() {
                        if row.tucked {
                            tops.push(ui.cursor().top());
                            continue;
                        }
                        let why = (Some(k) == failed_at)
                            .then(|| failed.as_ref().map(|f| f.1.as_str()))
                            .flatten();
                        tops.push(ui.cursor().top());
                        let editing = self
                            .renaming
                            .as_ref()
                            .filter(|n| n.0 == row.id)
                            .map(|n| n.1.clone());
                        let (r, resp) = self.tree_row(
                            ui,
                            w,
                            RowLook {
                                label: if editing.is_some() { "" } else { &row.title },
                                icon: row.icon,
                                tint: None,
                                on: self.selected == Some(row.id),
                                dim: row.suppressed
                                    || failed_at.is_some_and(|f| k > f)
                                    || end_row.is_some_and(|e| k > e),
                                depth: 0.5,
                            },
                        );
                        if row.kind == "sketch" {
                            let shown = self.sketch_shown(row.id);
                            if self.eye(ui, r, 1000 + k, shown) {
                                self.out.push(super::Action::SketchVisibility(row.id));
                            }
                            places.push((format!("tree:{}:eye", row.name), Self::eye_rect(r)));
                        }
                        if row.suppressed {
                            let y = r.center().y;
                            let g = ui
                                .fonts_mut(|f| f.layout_no_wrap(row.title.clone(), sans(13.0), t.ink2))
                                .size()
                                .x;
                            ui.painter().hline(
                                r.left() + 40.0..=r.left() + 40.0 + g,
                                y,
                                Stroke::new(1.0, t.ink2),
                            );
                        }
                        if let Some(mut text) = editing {
                            let field = Rect::from_min_size(
                                pos2(r.left() + 38.0, r.top() + 2.0),
                                vec2(w - 48.0, ROW_H - 4.0),
                            );
                            let f = ui.put(
                                field,
                                egui::TextEdit::singleline(&mut text)
                                    .font(sans(13.0))
                                    .margin(vec2(4.0, 3.0))
                                    .desired_width(field.width() - 8.0),
                            );
                            if !f.has_focus() && !f.lost_focus() {
                                f.request_focus();
                            }
                            if f.lost_focus() {
                                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                    rename_to = Some((row.id, text.clone()));
                                }
                                stop_rename = true;
                            }
                            typed_name = Some(text);
                        }
                        if row.sketch.is_some() {
                            let open = self.open_rows.contains(&row.id);
                            let chev = Rect::from_min_size(pos2(r.left(), r.top() + 4.0), vec2(16.0, 18.0));
                            let c = ui.interact(chev, ui.scope_id().with(("chev", k)), Sense::click());
                            icons::draw(
                                ui.painter(),
                                pos2(r.left() + 2.0, r.top() + 7.0),
                                12.0,
                                if open { "chevron" } else { "right" },
                                1.5,
                                t.ink2,
                            );
                            places.push((format!("tree:{}:open", row.name), chev));
                            if c.clicked() {
                                toggle_row = Some(row.id);
                            }
                        }
                        // a failed build is a red mark, a validity warning an amber triangle
                        let flags: Vec<&forge_agent::live::Flag> =
                            self.flags.iter().filter(|f| f.item == row.id).collect();
                        let beside = if row.kind == "sketch" { 50.0 } else { 26.0 };
                        let mark =
                            Rect::from_min_size(pos2(r.right() - beside, r.top() + 5.0), vec2(16.0, 16.0));
                        if why.is_some() {
                            ui.painter().circle_filled(mark.center(), 7.0, t.error);
                            icons::draw(
                                ui.painter(),
                                mark.min + vec2(2.0, 2.0),
                                12.0,
                                "close",
                                2.0,
                                Color32::WHITE,
                            );
                            places.push((format!("tree:{}:failed", row.name), mark));
                        } else if !flags.is_empty() {
                            icons::draw(ui.painter(), mark.min, 16.0, "warning", 1.5, t.focus);
                            places.push((format!("tree:{}:warning", row.name), mark));
                        }
                        let tip = match why {
                            Some(w) => format!("{}\n{w}", row.line),
                            None if !flags.is_empty() => {
                                let said: Vec<String> = flags
                                    .iter()
                                    .map(|f| format!("{} ({})", f.reason, f.rule))
                                    .collect();
                                format!("{}\n{}", row.line, said.join("\n"))
                            }
                            None => row.line.clone(),
                        };
                        let resp = resp.on_hover_text(tip);
                        if resp.hovered() {
                            tree_hover = Some(row.id);
                        }
                        if resp.double_clicked() && !row.suppressed {
                            opened = Some(row.id);
                        } else if resp.clicked() {
                            chosen = Some(Some(row.id));
                        }
                        if resp.secondary_clicked() {
                            menu = resp.interact_pointer_pos().map(|p| (row.id, p));
                        }
                        if resp.dragged() || resp.drag_stopped() {
                            row_drag = Some((k, resp.clone()));
                        }
                        places.push((format!("tree:{}", row.name), r));
                        if self.open_rows.contains(&row.id)
                            && let Some(child) = &row.sketch
                        {
                            let (title, on) = match child {
                                super::SketchChild::Line(id, title) => {
                                    (title.as_str(), self.selected == Some(*id))
                                }
                                super::SketchChild::Inline(_, title) => (title.as_str(), false),
                            };
                            let (r, resp) = self.tree_row(
                                ui,
                                w,
                                RowLook {
                                    label: title,
                                    icon: "sketch",
                                    tint: None,
                                    on,
                                    dim: false,
                                    depth: 1.5,
                                },
                            );
                            places.push((format!("tree:{}:sketch", row.name), r));
                            if let super::SketchChild::Line(id, _) = child {
                                let shown = self.sketch_shown(*id);
                                if self.eye(ui, r, 2000 + k, shown) {
                                    self.out.push(super::Action::SketchVisibility(*id));
                                }
                                places.push((format!("tree:{}:sketch:eye", row.name), Self::eye_rect(r)));
                                if resp.hovered() {
                                    tree_hover = Some(*id);
                                }
                                if resp.secondary_clicked() {
                                    menu = resp.interact_pointer_pos().map(|p| (*id, p));
                                }
                            }
                            let edit = match child {
                                super::SketchChild::Line(id, _) => super::Action::Edit(*id),
                                super::SketchChild::Inline(..) => super::Action::EditInline(row.id),
                            };
                            let inline = matches!(child, super::SketchChild::Inline(..));
                            if resp.double_clicked() || (inline && resp.secondary_clicked()) {
                                child_edit = Some(edit);
                            } else if resp.clicked()
                                && let super::SketchChild::Line(id, _) = child
                            {
                                chosen = Some(Some(*id));
                            }
                        }
                        if end_row == Some(k) {
                            end_resp = Some(end_mark(ui, &mut places));
                        }
                    }
                    tops.push(ui.cursor().top());
                    if end_row.is_none() {
                        end_resp = Some(end_mark(ui, &mut places));
                    }
                    // where a drag would drop: the gap between rows nearest the pointer
                    let gap_at = |y: f32| {
                        tops.iter()
                            .enumerate()
                            .min_by(|a, b| (a.1 - y).abs().total_cmp(&(b.1 - y).abs()))
                            .map_or(0, |g| g.0)
                    };
                    let pointer = ui.input(|i| i.pointer.interact_pos()).map(|p| p.y);
                    if let (Some((k, resp)), Some(y)) = (&row_drag, pointer) {
                        let gap = gap_at(y);
                        ui.painter()
                            .hline(tree.x_range(), tops[gap], Stroke::new(2.0, t.focus));
                        if resp.drag_stopped() && gap != *k && gap != *k + 1 {
                            moved = Some((rows[*k].id, rows.get(gap).map(|r| r.id)));
                        }
                    }
                    if let (Some(resp), Some(y)) = (&end_resp, pointer)
                        && (resp.dragged() || resp.drag_stopped())
                    {
                        let gap = gap_at(y);
                        ui.painter()
                            .hline(tree.x_range(), tops[gap], Stroke::new(2.0, t.focus));
                        if resp.drag_stopped() {
                            // at least the first feature stays
                            end_to = Some(if gap >= rows.len() {
                                None
                            } else {
                                rows.get(gap.max(1) - 1).map(|r| r.id)
                            });
                        }
                    }
                });
        });
        self.rows = rows;
        self.places.extend(places);
        self.tree_hover = tree_hover;
        if let Some(id) = toggle_row
            && !self.open_rows.remove(&id)
        {
            self.open_rows.insert(id);
        }
        self.out.extend(child_edit);
        if let Some(m) = menu {
            self.marking = Some(m.1);
            self.marking_row = Some(m.0);
        }
        if let Some((id, before)) = moved {
            self.out.push(super::Action::Reorder(id, before));
        }
        if let Some(to) = end_to {
            self.out.push(super::Action::EndAt(to));
        }
        if let (Some(text), Some(r)) = (typed_name, &mut self.renaming) {
            r.1 = text;
        }
        if stop_rename {
            self.renaming = None;
        }
        if rename_to.is_some() {
            self.renamed = rename_to;
        }
        if toggle_open {
            self.origin_open = !self.origin_open;
        }
        for k in toggles {
            self.origin_shown[k] = !self.origin_shown[k];
        }
        if let Some(on) = all_shown {
            self.origin_shown = [on; 7];
        }
        self.origin_tree_hover = hover_origin;
        if let Some(i) = chose_origin {
            self.origin_selected = (self.origin_selected != Some(i)).then_some(i);
            self.selected = None;
        }
        if let Some(c) = chosen {
            self.selected = c;
            self.origin_selected = None;
        }
        if let Some(id) = opened {
            self.selected = Some(id);
            self.origin_selected = None;
            self.out.push(super::Action::Edit(id));
        }
    }

    /// What of the origin's drawn, each and how strongly.
    pub fn origin_drawn(&self) -> Vec<(Item, bool)> {
        // a Forge file's origin is the view's switches' (Axes, Planes), and what's picked of it
        if self.fg.is_some() {
            let o = self.overlays();
            return origin::ITEMS
                .into_iter()
                .enumerate()
                .filter_map(|(k, i)| {
                    let strong = self.origin_hover == Some(i) || self.origin_selected == Some(i);
                    let on = if k < 3 { o.planes } else { o.axes };
                    (on || strong).then_some((i, strong))
                })
                .collect();
        }
        origin::ITEMS
            .into_iter()
            .enumerate()
            .filter_map(|(k, i)| {
                let strong = self.origin_hover == Some(i)
                    || self.origin_tree_hover == Some(i)
                    || self.origin_selected == Some(i);
                // an empty part shows its planes, to start on; after that, only what's asked for
                (self.origin_shown[k] || (self.rows.is_empty() && k < 3) || strong).then_some((i, strong))
            })
            .collect()
    }
}
