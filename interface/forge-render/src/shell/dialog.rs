//! The pieces a dialog over the view is made of, Inventor's way: section headings, rows of big
//! and worded toggles, dropdowns, and the selection rows that take the view's clicks.

use super::theme::{sans, semi};
use egui::{Align2, Rect, Sense, Stroke, pos2, vec2};

/// How wide a dialog's body is.
pub(super) const DIALOG_W: f32 = 284.0;

pub(super) fn heading(ui: &mut egui::Ui, text: &str, t: &super::theme::Theme) {
    let (r, _) = ui.allocate_exact_size(vec2(DIALOG_W, 18.0), Sense::hover());
    let g = ui.fonts_mut(|f| f.layout_no_wrap(text.to_string(), semi(12.0), t.ink2));
    let right = r.left() + g.size().x + 8.0;
    ui.painter()
        .galley(pos2(r.left(), r.center().y - g.size().y * 0.5), g, t.ink2);
    ui.painter()
        .hline(right..=r.right(), r.center().y, Stroke::new(1.0, t.line));
}

/// A row of big toggles with their words under them; which was clicked.
pub(super) fn toggle_row(
    ui: &mut egui::Ui,
    t: &super::theme::Theme,
    items: &[(&str, &str, bool)],
    name: &str,
    places: &mut Vec<(String, Rect)>,
) -> Option<usize> {
    let (row, _) = ui.allocate_exact_size(vec2(DIALOG_W, 52.0), Sense::hover());
    let w = (DIALOG_W - 4.0 * (items.len() as f32 - 1.0)) / items.len() as f32;
    let mut chose = None;
    for (k, (icon, word, on)) in items.iter().enumerate() {
        let r = Rect::from_min_size(row.left_top() + vec2(k as f32 * (w + 4.0), 0.0), vec2(w, 52.0));
        let resp = ui.interact(r, ui.scope_id().with((name, k)), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::RadioButton, true, *on, *word));
        let fill = if *on {
            t.on
        } else if resp.hovered() {
            t.hover
        } else {
            t.raised
        };
        let p = ui.painter();
        p.rect_filled(r, 4.0, fill);
        let edge = if *on { t.on_ink } else { t.line };
        p.rect_stroke(r, 4.0, Stroke::new(1.0, edge), egui::StrokeKind::Inside);
        let ink = if *on { t.on_ink } else { t.ink };
        super::icons::draw(p, pos2(r.center().x - 11.0, r.top() + 6.0), 22.0, icon, 1.5, ink);
        p.text(
            pos2(r.center().x, r.bottom() - 11.0),
            Align2::CENTER_CENTER,
            *word,
            sans(11.0),
            ink,
        );
        places.push((format!("{name}:{word}"), r));
        if resp.clicked() {
            chose = Some(k);
        }
    }
    chose
}

/// One of a thread's dropdowns.
pub(super) fn dropdown(
    ui: &mut egui::Ui,
    t: &super::theme::Theme,
    label: &str,
    now: &str,
    items: &[(String, String, bool)],
) -> Option<String> {
    let (row, _) = ui.allocate_exact_size(vec2(DIALOG_W, 28.0), Sense::hover());
    ui.painter().text(
        pos2(row.left(), row.center().y),
        Align2::LEFT_CENTER,
        label,
        sans(13.0),
        t.ink,
    );
    let at = Rect::from_min_size(
        pos2(row.left() + FIELD_AT, row.top() + 1.0),
        vec2(DIALOG_W - FIELD_AT, 26.0),
    );
    let mut chose = None;
    ui.scope_builder(egui::UiBuilder::new().max_rect(at), |ui| {
        egui::ComboBox::from_id_salt(("thread", label))
            .selected_text(now)
            .width(DIALOG_W - FIELD_AT)
            .height(320.0)
            .show_ui(ui, |ui| {
                for (value, words, greyed) in items {
                    let text = egui::RichText::new(words);
                    let text = if *greyed { text.color(t.ink2) } else { text };
                    if ui.selectable_label(value == now, text).clicked() {
                        chose = Some(value.clone());
                    }
                }
            });
    });
    chose
}

/// A row of worded toggles, side by side.
pub(super) fn text_toggles(
    ui: &mut egui::Ui,
    t: &super::theme::Theme,
    items: &[(&str, bool)],
    name: &str,
    places: &mut Vec<(String, Rect)>,
) -> Option<usize> {
    let (row, _) = ui.allocate_exact_size(vec2(DIALOG_W, 26.0), Sense::hover());
    let w = (DIALOG_W - 3.0 * (items.len() as f32 - 1.0)) / items.len() as f32;
    let mut chose = None;
    for (k, (word, on)) in items.iter().enumerate() {
        let r = Rect::from_min_size(row.left_top() + vec2(k as f32 * (w + 3.0), 0.0), vec2(w, 26.0));
        let resp = ui.interact(r, ui.scope_id().with((name, k)), Sense::click());
        let fill = if *on {
            t.on
        } else if resp.hovered() {
            t.hover
        } else {
            t.raised
        };
        let p = ui.painter();
        p.rect_filled(r, 3.0, fill);
        p.rect_stroke(
            r,
            3.0,
            Stroke::new(1.0, if *on { t.on_ink } else { t.line }),
            egui::StrokeKind::Inside,
        );
        p.text(
            r.center(),
            Align2::CENTER_CENTER,
            *word,
            sans(11.0),
            if *on { t.on_ink } else { t.ink },
        );
        places.push((format!("{name}:{word}"), r));
        if resp.clicked() {
            chose = Some(k);
        }
    }
    chose
}

/// A selection: what it wants, how many it has, and whether it's what the view's clicks go to.
pub(super) fn pick_row(
    ui: &mut egui::Ui,
    label: &str,
    n: usize,
    waiting: bool,
    hint: &str,
    t: &super::theme::Theme,
    places: &mut Vec<(String, Rect)>,
) -> bool {
    let (r, resp) = ui.allocate_exact_size(vec2(DIALOG_W, 34.0), Sense::click());
    let p = ui.painter();
    let (edge, fill) = if waiting {
        (t.focus, t.raised)
    } else if n > 0 {
        (t.accent, t.raised)
    } else {
        (t.line, t.raised)
    };
    p.rect_filled(r, 4.0, fill);
    p.rect_stroke(
        r,
        4.0,
        Stroke::new(if waiting { 2.0 } else { 1.0 }, edge),
        egui::StrokeKind::Inside,
    );
    super::icons::draw(p, r.left_top() + vec2(8.0, 9.0), 16.0, "select", 1.5, edge);
    p.text(
        pos2(r.left() + 32.0, r.center().y),
        Align2::LEFT_CENTER,
        label,
        sans(13.0),
        t.ink,
    );
    let state = if n == 0 {
        "None selected".to_string()
    } else {
        format!("{n} selected")
    };
    p.text(
        pos2(r.right() - 10.0, r.center().y),
        Align2::RIGHT_CENTER,
        state,
        sans(12.0),
        if n > 0 { t.ink } else { t.ink2 },
    );
    let clicked = resp.clicked();
    if !hint.is_empty() {
        let _ = resp.on_hover_text(hint);
    }
    places.push((format!("dialog:pick:{label}"), r));
    clicked
}

/// Where a row's field starts, after its word.
pub(super) const FIELD_AT: f32 = 92.0;
