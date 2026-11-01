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
    /// How an extrude's distance is taken, which the file writes with it.
    pub extent: Extent,
    pub toward: Toward,
    /// + was pressed: OK, then another of these.
    pub again: bool,
    /// The selections still to come after the one taking clicks.
    pub queue: Vec<(String, &'static str)>,
}

/// How far an extrude goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extent {
    Distance,
    Through,
    Next,
}

/// Which way it goes from its sketch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toward {
    Forward,
    Back,
    Both,
}

/// Does this kind's line start with how it combines with the part?
fn has_operation(kind: &str) -> bool {
    matches!(kind, "extrude" | "revolve" | "sweep" | "coil" | "loft")
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
        let mut queue: Vec<(String, &'static str)> = Vec::new();
        // the line reads in their order: size, where, how far
        match kind {
            "sweep" => {
                fields = vec![("Profile".into(), String::new()), ("Path".into(), String::new())];
                queue.push(("Path".to_string(), "Pick the sketch to sweep along"));
            }
            "loft" => fields = vec![("Profiles".into(), String::new())],
            "coil" => {
                fields = vec![
                    ("Profile".into(), String::new()),
                    ("Axis".into(), String::new()),
                    ("Pitch".into(), "8".into()),
                    ("Turns".into(), "4".into()),
                ];
                queue.push(("Axis".to_string(), "Pick a line of its sketch to turn about"));
            }
            "pattern" => {
                fields = vec![
                    ("Features".into(), String::new()),
                    ("Direction".into(), "lin 1 0 0".into()),
                    ("Count".into(), "3".into()),
                    ("Spacing".into(), "20".into()),
                ];
            }
            "combine" => {
                fields = vec![
                    ("Operation".into(), "add".into()),
                    ("Target".into(), String::new()),
                    ("Tools".into(), String::new()),
                ];
                queue.push(("Tools".to_string(), "Pick the bodies to combine with it"));
            }
            "plane" => {
                fields = vec![
                    ("Reference".into(), String::new()),
                    ("Offset".into(), "10".into()),
                ];
            }
            "axis" | "point" => fields = vec![("Reference".into(), String::new())],
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
            queue,
            sent: None,
            seq: 0,
            error: None,
            awaiting: false,
            units: "mm",
            params: Vec::new(),
            first: Vec::new(),
            extent: Extent::Distance,
            toward: Toward::Forward,
            again: false,
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
            extent: Extent::Distance,
            toward: Toward::Forward,
            again: false,
            queue: Vec::new(),
        }
    }

    /// Its numbers as a person reads them in the document's unit.
    pub fn in_units(mut self, units: &'static str, params: Vec<String>) -> Dialog {
        self.units = units;
        self.params = params;
        if has_operation(&self.kind) && !self.fields.iter().any(|f| f.0 == "Operation") {
            self.fields.insert(0, ("Operation".to_string(), String::new()));
        }
        self.read_extent();
        for (label, text) in &mut self.fields {
            if let Some(kind) = panel::field_kind(label) {
                let file = std::mem::take(text);
                *text = forge_doc::units::field_show(&file, kind, units);
                self.first.push((label.clone(), text.clone(), file));
            }
        }
        self
    }

    /// An extrude read from its file: how its distance is taken, apart from the number.
    fn read_extent(&mut self) {
        if self.kind != "extrude" {
            return;
        }
        let Some(f) = self.fields.iter_mut().find(|f| f.0 == "Distance") else {
            return;
        };
        let w: Vec<&str> = f.1.split_whitespace().collect();
        let (extent, toward, rest) = match w[..] {
            ["sym", n] => (Extent::Distance, Toward::Both, n.to_string()),
            ["thru"] => (Extent::Through, Toward::Forward, "10".to_string()),
            ["thru", "back"] => (Extent::Through, Toward::Back, "10".to_string()),
            ["thru", "both"] => (Extent::Through, Toward::Both, "10".to_string()),
            ["next"] => (Extent::Next, Toward::Forward, "10".to_string()),
            ["next", "back"] => (Extent::Next, Toward::Back, "10".to_string()),
            [n] if n.starts_with('-') => (Extent::Distance, Toward::Back, n[1..].to_string()),
            _ => return,
        };
        f.1 = rest;
        self.extent = extent;
        self.toward = toward;
    }

    /// Sets what a selection has, and goes on to the next one wanted.
    pub fn take_pick(&mut self, label: &str, text: String) {
        if let Some(f) = self.fields.iter_mut().find(|f| f.0 == label) {
            f.1 = text;
        }
        self.next_pick();
    }

    fn next_pick(&mut self) {
        let done = self
            .pick
            .as_ref()
            .is_some_and(|(l, _)| self.fields.iter().any(|f| f.0 == *l && !f.1.trim().is_empty()));
        if done && !self.queue.is_empty() {
            self.pick = Some(self.queue.remove(0));
        }
    }

    /// The selection a row of it says should take clicks now.
    pub fn activate(&mut self, label: &str) {
        let active = self.pick.as_ref().is_some_and(|(l, _)| l == label);
        if let (false, Some(k)) = (active, self.queue.iter().position(|q| q.0 == label)) {
            let next = self.queue.remove(k);
            if let Some(was) = self.pick.replace(next) {
                self.queue.push(was);
            }
        }
    }

    /// A field's text, as the file would have it.
    fn file_text(&self, label: &str, text: &str) -> String {
        match (self.kind.as_str(), label) {
            ("plane", "Reference") if !text.trim().is_empty() => return format!("off {}", text.trim()),
            ("coil", "Pitch") if !text.trim_start().starts_with("pitch") => {
                return format!("pitch {}", self.number_text(label, text));
            }
            ("coil", "Turns") if !text.trim_start().starts_with("turns") => {
                return format!("turns {}", text.trim());
            }
            _ => {}
        }
        if label == "Distance" && self.kind == "extrude" {
            let number = self.number_text(label, text);
            return match (self.extent, self.toward) {
                (Extent::Distance, Toward::Forward) => number,
                (Extent::Distance, Toward::Back) => format!("-{number}"),
                (Extent::Distance, Toward::Both) => format!("sym {number}"),
                (Extent::Through, Toward::Forward) => "thru".to_string(),
                (Extent::Through, Toward::Back) => "thru back".to_string(),
                (Extent::Through, Toward::Both) => "thru both".to_string(),
                (Extent::Next, Toward::Back) => "next back".to_string(),
                (Extent::Next, _) => "next".to_string(),
            };
        }
        self.number_text(label, text)
    }

    fn number_text(&self, label: &str, text: &str) -> String {
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
            || self
                .queue
                .iter()
                .any(|(l, _)| self.fields.iter().any(|f| f.0 == *l && f.1.trim().is_empty()))
            || (self.kind == "loft"
                && self
                    .fields
                    .iter()
                    .any(|f| f.0 == "Profiles" && f.1.matches("profile").count() < 2))
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
        let label = label.clone();
        if let Some(f) = self.fields.iter_mut().find(|f| f.0 == label) {
            f.1 = refs.join(" ");
        }
    }
}

/// How wide a dialog's body is.
const DIALOG_W: f32 = 284.0;

fn heading(ui: &mut egui::Ui, text: &str, t: &super::theme::Theme) {
    let (r, _) = ui.allocate_exact_size(vec2(DIALOG_W, 18.0), Sense::hover());
    let g = ui.fonts_mut(|f| f.layout_no_wrap(text.to_string(), semi(12.0), t.ink2));
    let right = r.left() + g.size().x + 8.0;
    ui.painter()
        .galley(pos2(r.left(), r.center().y - g.size().y * 0.5), g, t.ink2);
    ui.painter()
        .hline(right..=r.right(), r.center().y, Stroke::new(1.0, t.line));
}

/// A row of big toggles with their words under them; which was clicked.
fn toggle_row(
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

/// A row of small icon buttons after its word: for options that don't earn a big toggle.
fn small_row(
    ui: &mut egui::Ui,
    t: &super::theme::Theme,
    label: &str,
    items: &[(&str, &str, bool)],
    name: &str,
    places: &mut Vec<(String, Rect)>,
) -> Option<usize> {
    let (row, _) = ui.allocate_exact_size(vec2(DIALOG_W, 28.0), Sense::hover());
    ui.painter().text(
        pos2(row.left(), row.center().y),
        Align2::LEFT_CENTER,
        label,
        sans(13.0),
        t.ink,
    );
    let mut chose = None;
    for (k, (icon, word, on)) in items.iter().enumerate() {
        let r = Rect::from_min_size(
            pos2(row.left() + FIELD_AT + k as f32 * 30.0, row.top()),
            vec2(28.0, 28.0),
        );
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
        let edge = if *on { t.on_ink } else { t.line };
        p.rect_stroke(r, 3.0, Stroke::new(1.0, edge), egui::StrokeKind::Inside);
        let ink = if *on { t.on_ink } else { t.ink };
        super::icons::draw(p, r.min + vec2(5.0, 5.0), 18.0, icon, 1.5, ink);
        places.push((format!("{name}:{word}"), r));
        if resp.on_hover_text(*word).clicked() {
            chose = Some(k);
        }
    }
    chose
}

/// A selection: what it wants, how many it has, and whether it's what the view's clicks go to.
fn pick_row(
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

fn value_row(ui: &mut egui::Ui, name: &str, value: &str, t: &super::theme::Theme) {
    let (r, _) = ui.allocate_exact_size(vec2(DIALOG_W, 22.0), Sense::hover());
    let p = ui.painter();
    p.text(
        pos2(r.left(), r.center().y),
        Align2::LEFT_CENTER,
        name,
        sans(12.0),
        t.ink2,
    );
    p.text(
        pos2(r.left() + FIELD_AT, r.center().y),
        Align2::LEFT_CENTER,
        value,
        mono(12.0),
        t.ink,
    );
}

const FIELD_AT: f32 = 92.0;

/// A named value: its word, its field, its unit when what's typed has none.
fn field_row(
    ui: &mut egui::Ui,
    label: &str,
    text: &mut String,
    units: &str,
    t: &super::theme::Theme,
    places: &mut Vec<(String, Rect)>,
) {
    let (row, _) = ui.allocate_exact_size(vec2(DIALOG_W, 30.0), Sense::hover());
    ui.painter().text(
        pos2(row.left(), row.center().y),
        Align2::LEFT_CENTER,
        label,
        sans(13.0),
        t.ink,
    );
    let field = Rect::from_min_size(
        pos2(row.left() + FIELD_AT, row.top() + 2.0),
        vec2(DIALOG_W - FIELD_AT, 26.0),
    );
    ui.put(
        field,
        egui::TextEdit::singleline(text)
            .font(mono(13.0))
            .desired_width(field.width() - 14.0)
            .margin(vec2(7.0, 5.0)),
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
    places.push((format!("dialog:field:{label}"), field));
}

impl Shell {
    pub(super) fn dialog_ui(&mut self, ctx: &egui::Context) {
        let t = self.theme;
        let Some(mut d) = self.dialog.take() else { return };
        let frame = egui::Frame::new()
            .fill(t.surface)
            .stroke(Stroke::new(1.0, t.line))
            .corner_radius(6)
            .shadow(egui::Shadow {
                offset: [0, 4],
                blur: 16,
                spread: 0,
                color: Color32::from_black_alpha(90),
            })
            .inner_margin(12);
        let v = self.view_rect;
        let mut out = None;
        let mut edit_sketch = false;
        let mut places: Vec<(String, Rect)> = Vec::new();
        let shown = egui::Window::new(egui::RichText::new(d.title.clone()).font(semi(14.0)))
            .id(egui::Id::unique("feature-dialog"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .default_pos(pos2(v.left() + 16.0, v.top() + 16.0))
            .fixed_size(vec2(DIALOG_W, 0.0))
            .frame(frame)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                // the feature, by its icon and name
                let (head, _) = ui.allocate_exact_size(vec2(DIALOG_W, 30.0), Sense::hover());
                super::icons::draw(
                    ui.painter(),
                    head.left_top() + vec2(0.0, 3.0),
                    24.0,
                    panel::icon(&d.kind),
                    1.5,
                    t.ink,
                );
                ui.painter().text(
                    pos2(head.left() + 32.0, head.center().y),
                    Align2::LEFT_CENTER,
                    panel::kind_word(&d.kind),
                    semi(15.0),
                    t.ink,
                );
                let pick = d.pick.clone();
                let dims = d.sketch_dims();
                let units = d.units;
                let waiting = d.waiting_for_pick();
                let extrude = d.kind == "extrude";
                let combine = d.kind == "combine";
                let mut activate = None;
                let d_kind = d.kind.clone();
                let mut extent = d.extent;
                let mut toward = d.toward;
                for section in ["Input Geometry", "Behavior", "Output"] {
                    let there = d.fields.iter().any(|f| panel::section(&f.0) == section);
                    if !there {
                        continue;
                    }
                    ui.add_space(4.0);
                    heading(ui, section, &t);
                    for (label, text) in d.fields.iter_mut().filter(|f| panel::section(&f.0) == section) {
                        if label == "Operation" {
                            let now = text.clone();
                            let three = [
                                ("join", "Join", now == "add"),
                                ("opcut", "Cut", now == "cut"),
                                ("intersect", "Intersect", now == "keep"),
                            ];
                            let four = [
                                ("join", "Join", now.is_empty() || now == "add"),
                                ("opcut", "Cut", now == "cut"),
                                ("intersect", "Intersect", now == "keep"),
                                ("newsolid", "New Solid", now == "new"),
                            ];
                            let row = toggle_row(
                                ui,
                                &t,
                                if combine { &three[..] } else { &four[..] },
                                "dialog:op",
                                &mut places,
                            );
                            if let Some(k) = row {
                                *text = if combine {
                                    ["add", "cut", "keep"][k].to_string()
                                } else {
                                    ["", "cut", "keep", "new"][k].to_string()
                                };
                            }
                            continue;
                        }
                        if label == "Direction" {
                            let now = text.clone();
                            let row = small_row(
                                ui,
                                &t,
                                "Direction",
                                &[
                                    ("axisx", "X", now == "lin 1 0 0"),
                                    ("axisy", "Y", now == "lin 0 1 0"),
                                    ("axisz", "Z", now == "lin 0 0 1"),
                                ],
                                "dialog:way",
                                &mut places,
                            );
                            if let Some(k) = row {
                                *text = ["lin 1 0 0", "lin 0 1 0", "lin 0 0 1"][k].to_string();
                            }
                            continue;
                        }
                        if label == "Profile" && !dims.is_empty() {
                            for (name, value) in &dims {
                                value_row(ui, name, value, &t);
                            }
                            if ui.button("Edit Sketch").clicked() {
                                edit_sketch = true;
                            }
                            continue;
                        }
                        let is_pick = (matches!(
                            label.as_str(),
                            "Edges"
                                | "Faces"
                                | "Reference"
                                | "Features"
                                | "Bodies"
                                | "Points"
                                | "Cylinder"
                                | "Profile"
                                | "Profiles"
                                | "Path"
                                | "Target"
                                | "Tools"
                        )) || (label == "Axis" && d_kind == "coil");
                        if is_pick {
                            let wanted = pick.as_ref().is_some_and(|(l, _)| l == label);
                            let n = match label.as_str() {
                                "Points" => text.split_whitespace().count().saturating_sub(3),
                                "Cylinder" => (!text.trim().is_empty()) as usize,
                                "Profile" | "Path" | "Axis" | "Reference" | "Target" => {
                                    (!text.trim().is_empty()) as usize
                                }
                                "Profiles" => text.matches("profile").count(),
                                _ => forge_doc::text::tokens(text).len(),
                            };
                            let hint = pick.as_ref().filter(|(l, _)| l == label).map_or("", |(_, w)| *w);
                            if pick_row(ui, label, n, wanted && waiting, hint, &t, &mut places) {
                                activate = Some(label.clone());
                            }
                            continue;
                        }
                        // how far: the kind of extent and the way it goes, then the number if it has one
                        if extrude && label == "Distance" {
                            if let Some(k) = small_row(
                                ui,
                                &t,
                                "Extent",
                                &[
                                    ("distance", "Distance", extent == Extent::Distance),
                                    ("thruall", "Through All", extent == Extent::Through),
                                    ("tonext", "To Next", extent == Extent::Next),
                                ],
                                "dialog:extent",
                                &mut places,
                            ) {
                                extent = [Extent::Distance, Extent::Through, Extent::Next][k];
                            }
                            if let Some(k) = small_row(
                                ui,
                                &t,
                                "Direction",
                                &[
                                    ("forward", "Forward", toward == Toward::Forward),
                                    ("backward", "Back", toward == Toward::Back),
                                    ("symmetric", "Symmetric", toward == Toward::Both),
                                ],
                                "dialog:dir",
                                &mut places,
                            ) {
                                toward = [Toward::Forward, Toward::Back, Toward::Both][k];
                            }
                            if extent != Extent::Distance {
                                continue;
                            }
                        }
                        field_row(ui, label, text, units, &t, &mut places);
                    }
                }
                d.extent = extent;
                d.toward = toward;
                if let Some(l) = activate {
                    d.activate(&l);
                }
                if let Some(why) = &d.error {
                    ui.add_space(2.0);
                    let g =
                        ui.fonts_mut(|f| f.layout(why.clone(), sans(12.0), Color32::WHITE, DIALOG_W - 14.0));
                    let (r, _) = ui.allocate_exact_size(vec2(DIALOG_W, g.size().y + 8.0), Sense::hover());
                    ui.painter().rect_filled(r, 4.0, t.error);
                    ui.painter().galley(r.min + vec2(7.0, 4.0), g, Color32::WHITE);
                }
                ui.add_space(4.0);
                let can_ok = d.error.is_none() && !d.waiting_for_pick() && !d.awaiting;
                ui.horizontal(|ui| {
                    let new = matches!(d.target, DialogTarget::New(_));
                    if new {
                        let r = ui.add_enabled(
                            can_ok,
                            egui::Button::new(egui::RichText::new("+").font(semi(15.0)))
                                .min_size(vec2(30.0, 28.0)),
                        );
                        places.push(("dialog:apply".to_string(), r.rect));
                        if r.on_hover_text("Apply and make another").clicked() {
                            d.again = true;
                            out = Some(Action::DialogOk);
                        }
                    }
                    let gap = DIALOG_W - if new { 30.0 + 8.0 } else { 0.0 } - 64.0 - 8.0 - 64.0;
                    ui.add_space(gap.max(0.0));
                    let c = ui.add_sized(vec2(64.0, 28.0), egui::Button::new("Cancel"));
                    places.push(("dialog:cancel".to_string(), c.rect));
                    if c.clicked() {
                        out = Some(Action::DialogCancel);
                    }
                    let label = egui::RichText::new("OK").font(semi(13.0)).color(Color32::WHITE);
                    let ok = ui.add_enabled(
                        can_ok,
                        egui::Button::new(label).fill(t.accent).min_size(vec2(64.0, 28.0)),
                    );
                    places.push(("dialog:ok".to_string(), ok.rect));
                    if ok.clicked() {
                        out = Some(Action::DialogOk);
                    }
                });
                if can_ok && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                    out = Some(Action::DialogOk);
                }
            });
        self.places.extend(places);
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
