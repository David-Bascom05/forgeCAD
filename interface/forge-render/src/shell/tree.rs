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
        let (r, resp) = ui.allocate_exact_size(vec2(w, ROW_H), Sense::click());
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

    /// An eye at a row's right: shown or hidden, kept apart from the row's own click.
    fn eye(&self, ui: &mut Ui, row: Rect, key: usize, shown: bool) -> bool {
        let t = self.theme;
        let r = Rect::from_min_size(pos2(row.right() - 28.0, row.top() + 3.0), vec2(20.0, 20.0));
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

                    for (k, row) in rows.iter().enumerate() {
                        let why = (Some(k) == failed_at)
                            .then(|| failed.as_ref().map(|f| f.1.as_str()))
                            .flatten();
                        let (r, resp) = self.tree_row(
                            ui,
                            w,
                            RowLook {
                                label: &row.title,
                                icon: row.icon,
                                tint: None,
                                on: self.selected == Some(row.id),
                                dim: failed_at.is_some_and(|f| k > f),
                                depth: 0.5,
                            },
                        );
                        if why.is_some() {
                            icons::draw(
                                ui.painter(),
                                pos2(r.right() - 26.0, r.top() + 5.0),
                                16.0,
                                "warning",
                                1.5,
                                t.error,
                            );
                        }
                        let tip = match why {
                            Some(w) => format!("{}\n{w}", row.line),
                            None => row.line.clone(),
                        };
                        let resp = resp.on_hover_text(tip);
                        if resp.double_clicked() {
                            opened = Some(row.id);
                        } else if resp.clicked() {
                            chosen = Some(Some(row.id));
                        }
                        places.push((format!("tree:{}", row.name), r));
                    }

                    let (r, _) = ui.allocate_exact_size(vec2(w, 28.0), Sense::hover());
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
            self.dialog = Some(id);
        }
    }

    /// What of the origin's drawn, each and how strongly.
    pub fn origin_drawn(&self) -> Vec<(Item, bool)> {
        origin::ITEMS
            .into_iter()
            .enumerate()
            .filter_map(|(k, i)| {
                let strong = self.origin_hover == Some(i)
                    || self.origin_tree_hover == Some(i)
                    || self.origin_selected == Some(i);
                (self.origin_shown[k] || strong).then_some((i, strong))
            })
            .collect()
    }
}
