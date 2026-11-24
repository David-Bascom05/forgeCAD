//! The shared UX toolkit (docs/ux/STANDARD.md §12): what every environment's panels use so they
//! read the same. A value is shown with a person's label, its unit in the user's system and one
//! provenance badge; the raw key and the citation are on hover. Lists that can be empty say so
//! and offer the next step; anything that plays has the one transport.
//!
//! Immediate mode, like the rest of the shell: call the functions each frame.

use super::icons;
use super::theme::{self, Theme};
use egui::{Align, Color32, CornerRadius, Layout, Response, RichText, Sense, Stroke, Ui, vec2};

// ----- units -----------------------------------------------------------------------------------

/// The user's unit system: the status bar's mm / in switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum System {
    #[default]
    Metric,
    Inch,
}

impl System {
    /// From a part's length unit (`mm`, `cm`, `m`, `in`, `ft`).
    pub fn of_doc(unit: &str) -> System {
        match unit {
            "in" | "ft" => System::Inch,
            _ => System::Metric,
        }
    }
}

/// What a number measures, in the base unit it is stored in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Qty {
    /// mm
    Length,
    /// degrees
    Angle,
    /// kg
    Mass,
    /// kN (press force, clamp force)
    Force,
    /// N·m
    Torque,
    /// m/min (cutting speed, press approach / work / return)
    Speed,
    /// mm/min (feed, rapid)
    Feed,
    /// mm/s² (axis acceleration)
    Accel,
    /// rev/min
    Rpm,
    /// kW
    Power,
    /// kN/m (a die's load per length)
    LoadPerLength,
    /// a count: flutes, teeth, stations
    Count,
    /// a plain number with no unit
    Plain,
}

/// A value as a person reads it: number and unit, in their system, to a sensible precision.
pub fn show(v: f64, q: Qty, sys: System) -> String {
    let (x, unit) = convert(v, q, sys);
    let n = match q {
        Qty::Count => format!("{}", x.round() as i64),
        Qty::Angle => trim_to(x, 2),
        _ => forge_lang::units::trim(x),
    };
    if unit.is_empty() {
        n
    } else if unit == "°" {
        format!("{n}°")
    } else {
        format!("{n} {unit}")
    }
}

/// The value in the displayed unit and that unit's symbol.
pub fn convert(v: f64, q: Qty, sys: System) -> (f64, &'static str) {
    use System::*;
    match (q, sys) {
        (Qty::Length, Metric) => (v, "mm"),
        (Qty::Length, Inch) => (v / 25.4, "in"),
        (Qty::Angle, _) => (v, "°"),
        (Qty::Mass, Metric) => (v, "kg"),
        (Qty::Mass, Inch) => (v / 0.453_592_37, "lb"),
        (Qty::Force, Metric) => (v, "kN"),
        (Qty::Force, Inch) => (v / 8.896_443_230_521, "US tonf"),
        (Qty::Torque, Metric) => (v, "N·m"),
        (Qty::Torque, Inch) => (v / 1.355_817_948, "lbf·ft"),
        (Qty::Speed, Metric) => (v, "m/min"),
        (Qty::Speed, Inch) => (v / 0.3048, "ft/min"),
        (Qty::Feed, Metric) => (v, "mm/min"),
        (Qty::Feed, Inch) => (v / 25.4, "in/min"),
        (Qty::Accel, Metric) => (v, "mm/s²"),
        (Qty::Accel, Inch) => (v / 25.4, "in/s²"),
        (Qty::Rpm, _) => (v, "rpm"),
        (Qty::Power, Metric) => (v, "kW"),
        (Qty::Power, Inch) => (v / 0.745_699_872, "hp"),
        (Qty::LoadPerLength, Metric) => (v, "kN/m"),
        (Qty::LoadPerLength, Inch) => (v / 8.896_443_230_521 * 0.3048, "US tonf/ft"),
        (Qty::Count | Qty::Plain, _) => (v, ""),
    }
}

/// Back from the displayed unit to the stored one (what an edited field writes).
pub fn unconvert(shown: f64, q: Qty, sys: System) -> f64 {
    let (one, _) = convert(1.0, q, sys);
    if one == 0.0 { shown } else { shown / one }
}

fn trim_to(v: f64, places: usize) -> String {
    let s = format!("{v:.places$}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() {
        "0".into()
    } else {
        s.into()
    }
}

// ----- labels ----------------------------------------------------------------------------------

/// A person's label for a record key: the common ISO 13399 / library keys by name, anything else
/// by its words (`max_opening` → "Max opening", `speeds.approach` → "Approach speed").
pub fn label(key: &str) -> String {
    let known = match key {
        "dc" => "Cutting diameter",
        "dcon" => "Shank diameter",
        "lcf" => "Flute length",
        "apmx" => "Max depth of cut",
        "oal" | "lf" => "Overall length",
        "lu" => "Usable length",
        "nof" | "z" => "Flutes",
        "re" => "Corner radius",
        "sig" => "Point angle",
        "kapr" => "Lead angle",
        "material" => "Material",
        "type" => "Type",
        "note" => "Note",
        "source" => "Source",
        "taken" => "Read on",
        "maker" | "vendor" => "Maker",
        "model" => "Model",
        "real" => "Real product",
        "force" => "Press force",
        "throat" => "Throat depth",
        "table_height" => "Table height",
        "bed_length" => "Bed length",
        "housing_gap" => "Distance between housings",
        "open_height" => "Open height",
        "stroke" => "Stroke",
        "max_load" => "Max load",
        "v" => "V opening",
        "angle" => "Angle",
        "shoulder_radius" => "Shoulder radius",
        "height" => "Height",
        "width" => "Width",
        "floor" => "Floor",
        "outline" => "Outline",
        "segments" => "Segments",
        "clamp" => "Clamping",
        "clamps_included" => "Clamps included",
        "clamp_force" => "Clamping force",
        "jaw_width" => "Jaw width",
        "jaw_height" => "Jaw height",
        "jaw_thickness" => "Jaw thickness",
        "max_opening" => "Max opening",
        "min_opening" => "Min opening",
        "body_length" => "Body length",
        "body_width" => "Body width",
        "weight" | "mass" => "Weight",
        "rapid" => "Rapid",
        "max_feed" => "Max feed",
        "accel" => "Acceleration",
        "range" => "Travel",
        "home" => "Home",
        _ => "",
    };
    if !known.is_empty() {
        return known.to_string();
    }
    // `speeds.approach` → "Approach speed"; `frame.thickness` → "Frame thickness"
    let (group, last) = key.rsplit_once('.').unwrap_or(("", key));
    let words = |s: &str| s.replace(['_', '-'], " ");
    let text = match group {
        "speeds" => format!("{} speed", words(last)),
        "" => words(last),
        g => format!("{} {}", words(g.rsplit('.').next().unwrap_or(g)), words(last)),
    };
    let mut c = text.chars();
    c.next().map_or(String::new(), |f| {
        f.to_uppercase().chain(c.flat_map(char::to_lowercase)).collect()
    })
}

// ----- provenance ------------------------------------------------------------------------------

/// Where a value came from (§6). The strings are what the badge's hover says.
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    /// A published, cited value: the citation (document, page, URL, date).
    Published(String),
    /// Derived or estimated: how.
    Estimate(String),
    /// Sources disagree: which, with their values.
    Conflict(String),
    /// Not published anywhere we read.
    Unknown,
    /// The user's own value: when, and what it replaced.
    User(String),
    /// Given without a source.
    Uncited,
}

impl Source {
    pub fn word(&self) -> &'static str {
        match self {
            Source::Published(_) => "published",
            Source::Estimate(_) => "estimate",
            Source::Conflict(_) => "conflict",
            Source::Unknown => "unknown",
            Source::User(_) => "yours",
            Source::Uncited => "uncited",
        }
    }

    pub fn colour(&self, t: &Theme) -> Color32 {
        match self {
            Source::Published(_) => t.on_ink,
            Source::Estimate(_) => t.warn,
            Source::Conflict(_) => t.error,
            Source::Unknown | Source::Uncited => t.ink2,
            Source::User(_) => Color32::from_rgb(0xB3, 0x8C, 0xF0),
        }
    }

    pub fn detail(&self) -> &str {
        match self {
            Source::Published(s) | Source::Estimate(s) | Source::Conflict(s) | Source::User(s) => s,
            Source::Unknown => "Not published in any source we read.",
            Source::Uncited => "Given without a source.",
        }
    }
}

/// Badge column width, so every row's badge lines up.
pub const BADGE_W: f32 = 64.0;

/// The one provenance badge: a small pill, its hover the citation.
pub fn badge(ui: &mut Ui, t: &Theme, s: &Source) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(BADGE_W, 16.0), Sense::hover());
    let col = s.colour(t);
    let font = theme::caption();
    let galley = ui.painter().layout_no_wrap(s.word().into(), font, col);
    let w = (galley.size().x + 10.0).min(BADGE_W);
    let pill = egui::Rect::from_min_size(rect.min + vec2(0.0, 0.5), vec2(w, 15.0));
    ui.painter().rect(
        pill,
        CornerRadius::same(7),
        col.gamma_multiply(0.12),
        Stroke::new(1.0, col.gamma_multiply(0.55)),
        egui::StrokeKind::Inside,
    );
    ui.painter()
        .galley(pill.center() - galley.size() * 0.5, galley, col);
    let detail = s.detail().to_string();
    resp.on_hover_text(detail)
}

// ----- property sheet --------------------------------------------------------------------------

/// What a sheet row did this frame.
#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    /// A number committed, in the stored unit.
    Num(f64),
    Text(String),
    Bool(bool),
    /// "Set…" pressed on an unknown value.
    Set,
}

/// A sectioned property sheet: label column, value column, badge column (§5, §6). The raw key and
/// file show on the label's hover.
pub struct Sheet<'a> {
    pub t: &'a Theme,
    pub sys: System,
    /// Shown on hover with each key (`builtin/tools.toml`).
    pub file: &'a str,
    pub label_w: f32,
    pub value_w: f32,
}

impl<'a> Sheet<'a> {
    pub fn new(t: &'a Theme, sys: System, file: &'a str) -> Sheet<'a> {
        Sheet {
            t,
            sys,
            file,
            label_w: 140.0,
            value_w: 120.0,
        }
    }

    /// A section heading.
    pub fn section(&self, ui: &mut Ui, title: &str) {
        ui.add_space(theme::S2);
        ui.label(RichText::new(title).font(theme::semi(12.0)).color(self.t.ink2));
        ui.add(egui::Separator::default().spacing(theme::S1));
    }

    fn label_cell(&self, ui: &mut Ui, key: &str, text: &str) {
        let (rect, resp) = ui.allocate_exact_size(vec2(self.label_w, theme::CONTROL_H), Sense::hover());
        let galley = ui
            .painter()
            .layout(text.into(), theme::body(), self.t.ink2, self.label_w - theme::S2);
        let y = rect.center().y - galley.size().y * 0.5;
        ui.painter()
            .galley(egui::pos2(rect.left(), y), galley, self.t.ink2);
        let file = self.file;
        resp.on_hover_ui(|ui| {
            ui.label(RichText::new(key).font(theme::code()));
            if !file.is_empty() {
                ui.label(
                    RichText::new(file)
                        .font(theme::caption())
                        .color(ui.visuals().weak_text_color()),
                );
            }
        });
    }

    /// A number: shown in the user's unit; editable unless `src` is unknown (then "Set…").
    /// `buf` keeps the field's text between frames.
    pub fn num(
        &self,
        ui: &mut Ui,
        key: &str,
        v: Option<f64>,
        q: Qty,
        src: &Source,
        buf: &mut String,
    ) -> Option<Edit> {
        let mut out = None;
        ui.horizontal(|ui| {
            self.label_cell(ui, key, &label(key));
            match v {
                Some(x) if *src != Source::Unknown => {
                    let (shown, unit) = convert(x, q, self.sys);
                    let resp = ui.add_sized(
                        vec2(self.value_w - 44.0, theme::CONTROL_H),
                        egui::TextEdit::singleline(buf).align(egui::Align2::RIGHT_CENTER),
                    );
                    if !resp.has_focus() && !resp.lost_focus() {
                        *buf = match q {
                            Qty::Count => format!("{}", shown.round() as i64),
                            _ => forge_lang::units::trim(shown),
                        };
                    }
                    if resp.lost_focus()
                        && let Ok(n) = buf.trim().parse::<f64>()
                        && (n - shown).abs() > 1e-12
                    {
                        out = Some(Edit::Num(unconvert(n, q, self.sys)));
                    }
                    let (ur, _) = ui.allocate_exact_size(vec2(40.0, theme::CONTROL_H), Sense::hover());
                    ui.painter().text(
                        ur.left_center(),
                        egui::Align2::LEFT_CENTER,
                        unit,
                        theme::small(),
                        self.t.ink2,
                    );
                }
                _ => {
                    let (r, _) =
                        ui.allocate_exact_size(vec2(self.value_w - 56.0, theme::CONTROL_H), Sense::hover());
                    ui.painter().text(
                        r.right_center() - vec2(4.0, 0.0),
                        egui::Align2::RIGHT_CENTER,
                        "unknown",
                        egui::FontId::new(13.0, egui::FontFamily::Proportional),
                        self.t.ink2.gamma_multiply(0.8),
                    );
                    if ui
                        .add_sized(vec2(52.0, 22.0), egui::Button::new("Set…"))
                        .clicked()
                    {
                        out = Some(Edit::Set);
                    }
                }
            }
            if *src != Source::Unknown {
                badge(ui, self.t, src);
            }
        });
        out
    }

    /// Read-only text (a name, a maker); wraps rather than clips, full text on hover.
    pub fn text(&self, ui: &mut Ui, key: &str, value: &str, src: Option<&Source>) {
        ui.horizontal(|ui| {
            self.label_cell(ui, key, &label(key));
            let w = self.value_w;
            ui.allocate_ui_with_layout(
                vec2(w, theme::CONTROL_H),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.set_max_width(w);
                    ui.add(
                        egui::Label::new(RichText::new(value).font(theme::body()).color(self.t.ink))
                            .truncate(),
                    )
                    .on_hover_text(value);
                },
            );
            if let Some(s) = src {
                badge(ui, self.t, s);
            }
        });
    }

    /// A yes/no value, as a labelled checkbox.
    pub fn flag(&self, ui: &mut Ui, key: &str, mut on: bool) -> Option<Edit> {
        let mut out = None;
        ui.horizontal(|ui| {
            self.label_cell(ui, key, &label(key));
            if ui.checkbox(&mut on, "").changed() {
                out = Some(Edit::Bool(on));
            }
        });
        out
    }
}

// ----- empty and loading states ----------------------------------------------------------------

/// An empty list or view (§7): what it is, and the next step as buttons. Returns the index of the
/// button pressed.
pub fn empty_state(ui: &mut Ui, t: &Theme, what: &str, hint: &str, actions: &[&str]) -> Option<usize> {
    let mut hit = None;
    ui.vertical_centered(|ui| {
        ui.add_space(theme::S3);
        ui.label(RichText::new(what).font(theme::title()).color(t.ink));
        if !hint.is_empty() {
            ui.label(RichText::new(hint).font(theme::small()).color(t.ink2));
        }
        ui.add_space(theme::S2);
        ui.horizontal_wrapped(|ui| {
            for (k, a) in actions.iter().enumerate() {
                if ui.button(*a).clicked() {
                    hit = Some(k);
                }
            }
        });
        ui.add_space(theme::S3);
    });
    hit
}

/// Work in progress where its result will appear: a spinner and what it is doing.
pub fn loading(ui: &mut Ui, t: &Theme, doing: &str) {
    ui.horizontal(|ui| {
        ui.add(egui::Spinner::new().size(14.0).color(t.ink2));
        ui.label(RichText::new(doing).font(theme::small()).color(t.ink2));
    });
}

// ----- thumbnails ------------------------------------------------------------------------------

/// A record's thumbnail (§8), drawn from its real geometry by `crate::thumb` the first time `key`
/// is seen and kept as a texture after that. `key` must change when the geometry does (hash the
/// record). `mesh` is only called on a miss. None geometry draws the kind's ghost icon.
pub fn thumbnail(
    ui: &mut Ui,
    t: &Theme,
    key: u64,
    side: f32,
    mesh: impl FnOnce() -> Option<crate::thumb::Image>,
) -> Response {
    let id = egui::Id::unique(("ux-thumb", key));
    let tex: Option<Option<egui::TextureHandle>> = ui.ctx().data(|d| d.get_temp(id));
    let tex = match tex {
        Some(t) => t,
        None => {
            let made = mesh().map(|img| {
                let ci = egui::ColorImage::from_rgba_unmultiplied(
                    [img.side as usize, img.side as usize],
                    &img.rgba,
                );
                ui.ctx()
                    .load_texture(format!("ux-thumb-{key:x}"), ci, egui::TextureOptions::LINEAR)
            });
            ui.ctx().data_mut(|d| d.insert_temp(id, made.clone()));
            made
        }
    };
    let (rect, resp) = ui.allocate_exact_size(vec2(side, side), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(theme::R1), t.raised);
    match tex {
        Some(tex) => {
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            ui.painter().image(tex.id(), rect.shrink(2.0), uv, Color32::WHITE);
        }
        None => {
            // no geometry published: a dashed square, the ghost's look
            let r = rect.shrink(side * 0.22);
            let ink = t.ink2.gamma_multiply(0.7);
            for (a, b) in [
                (r.left_top(), r.right_top()),
                (r.right_top(), r.right_bottom()),
                (r.right_bottom(), r.left_bottom()),
                (r.left_bottom(), r.left_top()),
            ] {
                ui.painter()
                    .add(egui::Shape::dashed_line(&[a, b], Stroke::new(1.0, ink), 3.0, 2.0));
            }
        }
    }
    resp
}

// ----- ghost labels ----------------------------------------------------------------------------

/// Labels for ghosts (`crate::ghost`), drawn over the view: a short leader up and right from
/// the anchor to a pill saying what is unknown. `view` is the view's rect on screen; anchors
/// behind the eye are skipped.
pub fn ghost_labels(
    painter: &egui::Painter,
    t: &Theme,
    camera: &crate::camera::Camera,
    view: egui::Rect,
    labels: &[crate::ghost::Label],
) {
    let size = (view.width().max(1.0) as u32, view.height().max(1.0) as u32);
    for l in labels {
        let Some((x, y)) = camera.pixel_of(l.at, size.0, size.1) else {
            continue;
        };
        let a = view.min + vec2(x, y);
        let b = a + vec2(18.0, -18.0);
        let ink = t.ink;
        painter.line_segment([a, b], Stroke::new(1.0, ink.gamma_multiply(0.7)));
        painter.circle_filled(a, 2.5, ink.gamma_multiply(0.8));
        let g = painter.layout_no_wrap(l.text.clone(), theme::small(), ink);
        let r = egui::Rect::from_min_size(b + vec2(0.0, -g.size().y * 0.5 - 3.0), g.size() + vec2(12.0, 6.0));
        painter.rect(
            r,
            CornerRadius::same(theme::R1),
            t.surface.gamma_multiply(0.92),
            Stroke::new(1.0, t.line),
            egui::StrokeKind::Inside,
        );
        painter.galley(r.min + vec2(6.0, 3.0), g, ink);
    }
}

// ----- transport -------------------------------------------------------------------------------

/// What the transport asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transport {
    First,
    Back,
    PlayPause,
    Forward,
    Last,
}

/// An icon button with a tooltip (no ASCII stand-ins: §5).
pub fn icon_button(ui: &mut Ui, t: &Theme, icon: &str, tip: &str, on: bool) -> Response {
    let side = theme::CONTROL_H;
    let (rect, resp) = ui.allocate_exact_size(vec2(side, side), Sense::click());
    let fill = if on {
        t.on
    } else if resp.hovered() {
        t.hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(theme::R1), fill);
    let ink = if on { t.on_ink } else { t.ink };
    icons::draw(
        ui.painter(),
        rect.min + vec2(5.0, 5.0),
        side - 10.0,
        icon,
        1.5,
        ink,
    );
    resp.on_hover_text(tip)
}

/// The one transport row for anything that plays (CAM sim, bend sim): first, back, play/pause,
/// forward, last, then `status` ("Bend 1 of 2 · bending · 51.4°"). Space, ←/→ and Home/End do
/// the same when the view has focus; the caller routes those keys to `transport_key`.
pub fn transport(ui: &mut Ui, t: &Theme, playing: bool, status: &str) -> Option<Transport> {
    let mut out = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        if icon_button(ui, t, "first", "First (Home)", false).clicked() {
            out = Some(Transport::First);
        }
        if icon_button(ui, t, "stepback", "Previous step (Left arrow)", false).clicked() {
            out = Some(Transport::Back);
        }
        let (icon, tip) = if playing {
            ("pause", "Pause (Space)")
        } else {
            ("play", "Play (Space)")
        };
        if icon_button(ui, t, icon, tip, playing).clicked() {
            out = Some(Transport::PlayPause);
        }
        if icon_button(ui, t, "stepfwd", "Next step (Right arrow)", false).clicked() {
            out = Some(Transport::Forward);
        }
        if icon_button(ui, t, "last", "Last (End)", false).clicked() {
            out = Some(Transport::Last);
        }
        ui.add_space(theme::S2);
        ui.label(RichText::new(status).font(theme::small()).color(t.ink));
    });
    out
}

/// The transport's keys, for a view that has focus and no text field active.
pub fn transport_key(ctx: &egui::Context) -> Option<Transport> {
    if ctx.egui_wants_keyboard_input() {
        return None;
    }
    ctx.input(|i| {
        use egui::Key;
        if i.key_pressed(Key::Space) {
            Some(Transport::PlayPause)
        } else if i.key_pressed(Key::ArrowLeft) {
            Some(Transport::Back)
        } else if i.key_pressed(Key::ArrowRight) {
            Some(Transport::Forward)
        } else if i.key_pressed(Key::Home) {
            Some(Transport::First)
        } else if i.key_pressed(Key::End) {
            Some(Transport::Last)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_follow_the_system() {
        assert_eq!(show(850.0, Qty::Force, System::Metric), "850 kN");
        assert_eq!(show(10.0, Qty::Length, System::Metric), "10 mm");
        assert_eq!(show(25.4, Qty::Length, System::Inch), "1 in");
        assert_eq!(show(30.0, Qty::Angle, System::Metric), "30°");
        assert_eq!(show(3.0, Qty::Count, System::Inch), "3");
        assert_eq!(show(25.0, Qty::Speed, System::Metric), "25 m/min");
        let back = unconvert(
            convert(150.0, Qty::Length, System::Inch).0,
            Qty::Length,
            System::Inch,
        );
        assert!((back - 150.0).abs() < 1e-9);
    }

    #[test]
    fn labels_read_as_words() {
        assert_eq!(label("dc"), "Cutting diameter");
        assert_eq!(label("lcf"), "Flute length");
        assert_eq!(label("jaw_width"), "Jaw width");
        assert_eq!(label("speeds.approach"), "Approach speed");
        assert_eq!(label("frame.thickness"), "Frame thickness");
        assert_eq!(label("ram.front"), "Ram front");
        assert_eq!(label("backgauge_reach"), "Backgauge reach");
    }
}
