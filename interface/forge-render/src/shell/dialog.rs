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
    /// The unit a bare number's in, and the part's parameter names.
    pub units: &'static str,
    pub params: Vec<String>,
    /// Each number field as first shown and as the file had it: what's untouched goes back as it was.
    pub first: Vec<(String, String, String)>,
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
        // the line reads in their order: size, where, how far
        match kind {
            "hole" => {
                fields = vec![
                    ("Diameter".to_string(), "5".to_string()),
                    ("Points".to_string(), String::new()),
                    ("Depth".to_string(), "thru".to_string()),
                ];
            }
            "thread" => {
                fields = vec![
                    ("Size".to_string(), String::new()),
                    ("Cylinder".to_string(), String::new()),
                    ("Length".to_string(), "full".to_string()),
                ];
            }
            _ => {}
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
            units: "mm",
            params: Vec::new(),
            first: Vec::new(),
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
            units: "mm",
            params: Vec::new(),
            first: Vec::new(),
        }
    }

    /// Its numbers as a person reads them in the document's unit.
    pub fn in_units(mut self, units: &'static str, params: Vec<String>) -> Dialog {
        self.units = units;
        self.params = params;
        for (label, text) in &mut self.fields {
            if let Some(kind) = panel::field_kind(label) {
                let file = std::mem::take(text);
                *text = forge_doc::units::field_show(&file, kind, units);
                self.first.push((label.clone(), text.clone(), file));
            }
        }
        self
    }

    /// A field's text, as the file would have it.
    fn file_text(&self, label: &str, text: &str) -> String {
        let Some(kind) = panel::field_kind(label) else {
            return text.to_string();
        };
        if let Some(f) = self.first.iter().find(|f| f.0 == label && f.1 == text) {
            return f.2.clone();
        }
        let is_param = |w: &str| self.params.iter().any(|p| p == w);
        forge_doc::units::field_file(text, kind, self.units, &is_param)
    }

    /// It reads this sketch, with no plane or profile of its own.
    pub fn on_sketch(&mut self, name: &str) {
        self.fields
            .retain(|f| f.0 != "Plane" && f.0 != "Profile" && f.0 != "Axis");
        self.fields.push(("Profile".to_string(), name.to_string()));
    }

    /// The feature's line, from its fields.
    pub fn line(&self) -> String {
        let mut out = self.kind.clone();
        for (label, text) in &self.fields {
            let file = self.file_text(label, text);
            let t = file.trim();
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

    /// The feature's profile as a person reads it: its sketch's dimensions, if it's written inline.
    pub fn sketch_dims(&self) -> Vec<panel::Shown> {
        let words: Vec<String> = self
            .fields
            .iter()
            .flat_map(|f| f.1.split_whitespace().map(str::to_string))
            .collect();
        let words: Vec<&str> = words.iter().map(String::as_str).collect();
        panel::inline_profile(&self.kind, &words).map_or(Vec::new(), |p| p.shown)
    }

    /// What's selected in the view, as the part file names it, goes to the field taking picks.
    /// A hole's point: on or off the list; another sketch's starts the list again. `on` says which
    /// way the sketch faces into the part.
    pub fn toggle_point(&mut self, sketch: &str, point: &str, on: &str) {
        let Some(f) = self.fields.iter_mut().find(|f| f.0 == "Points") else {
            return;
        };
        let head = format!("{on} {sketch} pts");
        let mut points: Vec<String> = match f.1.strip_prefix(&head) {
            Some(rest) => rest.split_whitespace().map(str::to_string).collect(),
            None => Vec::new(),
        };
        if let Some(k) = points.iter().position(|p| p == point) {
            points.remove(k);
        } else {
            points.push(point.to_string());
        }
        f.1 = if points.is_empty() {
            String::new()
        } else {
            format!("{head} {}", points.join(" "))
        };
    }

    /// A thread's cylinder, and the size that fits it till one's typed.
    pub fn thread_on(&mut self, on: String, size: Option<String>) {
        for f in &mut self.fields {
            if f.0 == "Cylinder" {
                f.1.clone_from(&on);
            }
            if f.0 == "Size"
                && f.1.is_empty()
                && let Some(s) = &size
            {
                f.1.clone_from(s);
            }
        }
    }

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
        let mut edit_sketch = false;
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
                        let dims = d.sketch_dims();
                        let units = d.units;
                        for (k, s) in sections.into_iter().enumerate() {
                            egui::CollapsingHeader::new(
                                egui::RichText::new(s).font(semi(12.0)).color(t.ink2),
                            )
                            .id_salt(s)
                            .default_open(k < 2)
                            .show_unindented(ui, |ui| {
                                for (label, text) in d.fields.iter_mut().filter(|f| panel::section(&f.0) == s)
                                {
                                    // a profile shows as its sketch, references as a count: never as text
                                    if label == "Profile" && !dims.is_empty() {
                                        for (name, value) in &dims {
                                            let w = ui.available_width();
                                            let (row, _) =
                                                ui.allocate_exact_size(vec2(w, 24.0), Sense::hover());
                                            let (mid, left) = (row.center().y, row.left());
                                            let p = ui.painter();
                                            p.text(
                                                pos2(left, mid),
                                                Align2::LEFT_CENTER,
                                                name,
                                                sans(12.0),
                                                t.ink2,
                                            );
                                            p.text(
                                                pos2(left + 88.0, mid),
                                                Align2::LEFT_CENTER,
                                                value,
                                                mono(12.0),
                                                t.ink,
                                            );
                                        }
                                        if ui.button("Edit Sketch").clicked() {
                                            edit_sketch = true;
                                        }
                                        continue;
                                    }
                                    if matches!(
                                        label.as_str(),
                                        "Edges" | "Faces" | "Reference" | "Features" | "Bodies"
                                    ) && !text.trim().is_empty()
                                    {
                                        let n = forge_doc::text::tokens(text).len();
                                        let w = ui.available_width();
                                        let (row, resp) =
                                            ui.allocate_exact_size(vec2(w, 28.0), Sense::hover());
                                        let _ = resp.on_hover_text(text.as_str());
                                        let (mid, left) = (row.center().y, row.left());
                                        let p = ui.painter();
                                        p.text(
                                            pos2(left, mid),
                                            Align2::LEFT_CENTER,
                                            label.as_str(),
                                            sans(12.0),
                                            t.ink2,
                                        );
                                        p.text(
                                            pos2(left + 88.0, mid),
                                            Align2::LEFT_CENTER,
                                            format!("{n} selected"),
                                            sans(12.0),
                                            t.ink,
                                        );
                                        continue;
                                    }
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
                                    if let Some(kind) = panel::field_kind(label)
                                        && forge_doc::units::is_bare(text)
                                    {
                                        ui.painter().text(
                                            pos2(field.right() - 7.0, field.center().y),
                                            Align2::RIGHT_CENTER,
                                            forge_doc::units::unit_of(kind, units),
                                            mono(11.0),
                                            t.ink2,
                                        );
                                    }
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
        if edit_sketch && let DialogTarget::Edit(id) = d.target {
            out = Some(Action::DialogCancel);
            self.out.push(Action::EditInline(id));
        }
        self.dialog = Some(d);
        self.out.extend(out);
    }
}
