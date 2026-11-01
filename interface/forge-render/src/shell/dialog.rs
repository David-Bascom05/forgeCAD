//! A feature's dialog: floats over the view, drags by its title, remembers where it was left,
//! scrolls, and groups its fields in sections, in tabs where a kind has many. It says what the fields
//! are; the host turns them into a tool call, previews it live and commits it on OK.

use super::theme::{mono, sans, semi};
use super::{Action, Shell, panel};
use egui::{Align2, Color32, Rect, Sense, Stroke, pos2, vec2};
use forge_doc::ItemId;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DialogTarget {
    New(&'static str),
    Edit(ItemId),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dialog {
    pub target: DialogTarget,
    pub kind: String,
    pub title: String,
    /// Label and text, in the line's order.
    pub fields: Vec<(String, String)>,
    /// The field that takes what's clicked in the view, and what to say till it has some.
    pub pick: Option<(String, &'static str)>,
    /// The line last previewed, and that preview's number.
    pub sent: Option<String>,
    pub seq: u64,
    pub error: Option<String>,
    /// OK was pressed: the answer's on its way.
    pub awaiting: bool,
}

impl Dialog {
    pub fn new_feature(kind: &'static str) -> Dialog {
        let (line, pick) = panel::template(kind);
        let words: Vec<&str> = line.split_whitespace().collect();
        let mut fields = panel::fields(kind, &words);
        if let Some((label, _)) = pick
            && !fields.iter().any(|f| f.0 == label)
        {
            fields.push((label.to_string(), String::new()));
        }
        Dialog {
            target: DialogTarget::New(kind),
            kind: kind.to_string(),
            title: panel::kind_word(kind),
            fields,
            pick: pick.map(|(l, why)| (l.to_string(), why)),
            sent: None,
            seq: 0,
            error: None,
            awaiting: false,
        }
    }

    pub fn edit(id: ItemId, kind: &str, title: String, fields: Vec<(String, String)>) -> Dialog {
        Dialog {
            target: DialogTarget::Edit(id),
            kind: kind.to_string(),
            title,
            fields,
            pick: None,
            sent: None,
            seq: 0,
            error: None,
            awaiting: false,
        }
    }

    /// The feature's line, from its fields.
    pub fn line(&self) -> String {
        let mut out = self.kind.clone();
        for (_, text) in &self.fields {
            let t = text.trim();
            if !t.is_empty() {
                out.push(' ');
                out.push_str(t);
            }
        }
        out
    }

    /// A field that must come from the view is still empty.
    pub fn waiting_for_pick(&self) -> bool {
        self.pick
            .as_ref()
            .is_some_and(|(l, _)| self.fields.iter().any(|f| f.0 == *l && f.1.trim().is_empty()))
    }

    /// What's selected in the view, as the part file names it, goes to the field taking picks.
    pub fn picked(&mut self, refs: &[String]) {
        let Some((label, _)) = &self.pick else { return };
        if let Some(f) = self.fields.iter_mut().find(|f| f.0 == *label) {
            f.1 = refs.join(" ");
        }
    }
}

impl Shell {
    pub(super) fn dialog_ui(&mut self, ctx: &egui::Context) {
        let t = self.theme;
        let Some(mut d) = self.dialog.take() else { return };
        let frame = egui::Frame::new()
            .fill(t.surface)
            .stroke(Stroke::new(1.0, t.line))
            .corner_radius(4)
            .inner_margin(10);
        let v = self.view_rect;
        let mut out = None;
        let shown = egui::Window::new(egui::RichText::new(d.title.clone()).font(semi(13.0)))
            .id(egui::Id::unique("feature-dialog"))
            .collapsible(false)
            .resizable(true)
            .default_pos(pos2(v.left() + 16.0, v.top() + 16.0))
            .default_size(vec2(340.0, 420.0))
            .min_size(vec2(280.0, 180.0))
            .frame(frame)
            .show(ctx, |ui| {
                let footer = if d.error.is_some() { 76.0 } else { 44.0 };
                let body = (ui.available_height() - footer).max(60.0);
                egui::ScrollArea::vertical()
                    .id_salt("dialog-body")
                    .max_height(body)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // sections in the order their fields come; the first always open
                        let mut sections: Vec<&str> = Vec::new();
                        for (label, _) in &d.fields {
                            let s = panel::section(label);
                            if !sections.contains(&s) {
                                sections.push(s);
                            }
                        }
                        let pick = d.pick.clone();
                        for (k, s) in sections.into_iter().enumerate() {
                            egui::CollapsingHeader::new(
                                egui::RichText::new(s).font(semi(12.0)).color(t.ink2),
                            )
                            .id_salt(s)
                            .default_open(k < 2)
                            .show_unindented(ui, |ui| {
                                for (label, text) in d.fields.iter_mut().filter(|f| panel::section(&f.0) == s)
                                {
                                    let w = ui.available_width();
                                    let (row, _) = ui.allocate_exact_size(vec2(w, 32.0), Sense::hover());
                                    ui.painter().text(
                                        pos2(row.left(), row.center().y),
                                        Align2::LEFT_CENTER,
                                        label.as_str(),
                                        sans(12.0),
                                        t.ink2,
                                    );
                                    let field = Rect::from_min_size(
                                        pos2(row.left() + 88.0, row.top() + 3.0),
                                        vec2((w - 92.0).max(60.0), 26.0),
                                    );
                                    let hint = pick
                                        .as_ref()
                                        .filter(|(l, _)| l == label)
                                        .map_or("", |(_, why)| *why);
                                    ui.put(
                                        field,
                                        egui::TextEdit::singleline(text)
                                            .font(mono(12.0))
                                            .hint_text(hint)
                                            .desired_width(field.width() - 14.0)
                                            .margin(vec2(7.0, 6.0)),
                                    );
                                }
                            });
                        }
                    });
                if let Some(why) = &d.error {
                    ui.add_space(4.0);
                    let g = ui.fonts_mut(|f| {
                        f.layout(
                            why.clone(),
                            sans(12.0),
                            Color32::WHITE,
                            ui.available_width() - 14.0,
                        )
                    });
                    let (r, _) =
                        ui.allocate_exact_size(vec2(ui.available_width(), g.size().y + 8.0), Sense::hover());
                    ui.painter().rect_filled(r, 4.0, t.error);
                    ui.painter().galley(r.min + vec2(7.0, 4.0), g, Color32::WHITE);
                }
                ui.add_space(6.0);
                let can_ok = d.error.is_none() && !d.waiting_for_pick() && !d.awaiting;
                ui.horizontal(|ui| {
                    ui.add_space((ui.available_width() - 126.0).max(0.0));
                    if ui
                        .add_sized(vec2(64.0, 28.0), egui::Button::new("Cancel"))
                        .clicked()
                    {
                        out = Some(Action::DialogCancel);
                    }
                    ui.add_space(6.0);
                    let label = egui::RichText::new("OK").font(semi(12.0)).color(Color32::WHITE);
                    let ok = egui::Button::new(label).fill(t.accent);
                    if ui.add_enabled(can_ok, ok).clicked() {
                        out = Some(Action::DialogOk);
                    }
                });
                if can_ok && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                    out = Some(Action::DialogOk);
                }
            });
        if let Some(s) = shown {
            self.note("dialog", s.response.rect);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            out = Some(Action::DialogCancel);
        }
        self.dialog = Some(d);
        self.out.extend(out);
    }
}
