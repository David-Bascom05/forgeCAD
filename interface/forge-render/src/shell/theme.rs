//! Commonty's colours and type, as egui's style.

use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId, Stroke};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub dark: bool,
    pub bar: Color32,
    pub bg: Color32,
    pub surface: Color32,
    pub raised: Color32,
    pub ink: Color32,
    pub ink2: Color32,
    pub line: Color32,
    pub hover: Color32,
    /// The one accent: primary buttons, the File button, the chosen tab's rule.
    pub accent: Color32,
    pub field: Color32,
    pub view: u32,
    pub on: Color32,
    pub on_ink: Color32,
    pub focus: Color32,
    pub error: Color32,
    pub selection: Color32,
    /// Sketches in the view: apart from the part, the view and what's lit, in either theme.
    pub sketch: Color32,
    /// Sketch geometry projected from the part.
    pub projected: Color32,
    /// A warning: an issue that doesn't stop the build.
    pub warn: Color32,
    /// Done and valid; never a call to action.
    pub success: Color32,
    /// Over the app behind a modal window.
    pub scrim: Color32,
    /// A floating panel's shadow.
    pub shadow: Color32,
}

const fn c(x: u32) -> Color32 {
    Color32::from_rgb((x >> 16) as u8, (x >> 8) as u8, x as u8)
}

pub const DARK: Theme = Theme {
    dark: true,
    bar: c(0x16304F),
    bg: c(0x0F1318),
    surface: c(0x171C22),
    raised: c(0x1F252D),
    ink: c(0xE5E9EE),
    ink2: c(0xA1A9B3),
    line: c(0x2A313B),
    hover: c(0x232A33),
    accent: c(0x2F6FB3),
    field: c(0x0F1318),
    view: 0x12171D,
    on: c(0x1B2838),
    on_ink: c(0x9CC3EA),
    sketch: c(0xFF2E9A),
    focus: c(0xE0A21B),
    error: c(0xE0554A),
    selection: c(0x2C4A6B),
    projected: c(0xD9C24A),
    warn: c(0xE0A030),
    success: c(0x2F7D59),
    scrim: Color32::from_black_alpha(140),
    shadow: Color32::from_black_alpha(90),
};

pub const LIGHT: Theme = Theme {
    dark: false,
    bar: c(0x1D4470),
    bg: c(0xEEF1F5),
    surface: c(0xFFFFFF),
    raised: c(0xF7F8FA),
    ink: c(0x1A1F26),
    ink2: c(0x545C66),
    line: c(0xD5DAE1),
    hover: c(0xEEF1F5),
    accent: c(0x1D5FA8),
    field: c(0xFFFFFF),
    view: 0xDDE2E8,
    on: c(0xE3ECF5),
    on_ink: c(0x1D4470),
    sketch: c(0xC4006E),
    focus: c(0xE0A21B),
    error: c(0xB83A30),
    selection: c(0xB9D2EC),
    projected: c(0x9A7A00),
    warn: c(0xB87400),
    success: c(0x1D5C42),
    scrim: Color32::from_black_alpha(100),
    shadow: Color32::from_black_alpha(36),
};

// The design system's numbers (research/ui/polish.md): every panel takes its spacing, sizes and
// type from here.

/// Spacing on a 4-px grid: icon to label, tight rows.
pub const S1: f32 = 4.0;
/// Between controls in a row; a button's side padding.
pub const S2: f32 = 8.0;
/// A panel's padding; between groups.
pub const S3: f32 = 12.0;
/// Between sections.
pub const S4: f32 = 16.0;
/// Between an empty state's blocks.
pub const S5: f32 = 24.0;
/// A control's corners: buttons, fields, chips.
pub const R1: u8 = 4;
/// A panel's or dialog's corners.
pub const R2: u8 = 6;
/// A button's, field's or combo's height.
pub const CONTROL_H: f32 = 26.0;
/// A floating panel's title row.
pub const TITLE_ROW_H: f32 = 28.0;

/// Group labels, hints, secondary meta.
pub fn caption() -> FontId {
    sans(11.0)
}

/// Dense UI: tree details, the status bar, tables.
pub fn small() -> FontId {
    sans(12.0)
}

/// Labels, menus, dialog fields.
pub fn body() -> FontId {
    sans(13.0)
}

/// A panel's or dialog's title.
pub fn title() -> FontId {
    semi(14.0)
}

/// An empty state's or a settings page's heading.
pub fn heading() -> FontId {
    semi(18.0)
}

/// Names and values in code: feature names, file names, numbers.
pub fn code() -> FontId {
    mono(12.0)
}

/// A floating panel's frame: the tool dialog, Measure, Issues, Settings.
pub fn panel_frame(t: &Theme) -> egui::Frame {
    egui::Frame::new()
        .fill(t.surface)
        .stroke(Stroke::new(1.0, t.line))
        .corner_radius(R2)
        .shadow(egui::Shadow {
            offset: [0, 4],
            blur: 16,
            spread: 0,
            color: t.shadow,
        })
        .inner_margin(S3 as i8)
}

/// A control's edge: a box, a button, a checkbox. Firmer than a rule (`line`), so a checkbox or a
/// field reads on its window in either theme.
pub fn edge(t: &Theme) -> Color32 {
    let m = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * 0.38).round() as u8;
    let (a, b) = (t.line, t.ink2);
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

pub fn sans(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("medium".into()))
}

pub fn semi(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semi".into()))
}

pub fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

/// Public Sans and IBM Plex Mono, bundled.
pub fn fonts() -> FontDefinitions {
    let mut f = FontDefinitions::empty();
    let faces: [(&str, &'static [u8]); 4] = [
        ("sans", include_bytes!("../../fonts/PublicSans-Regular.ttf")),
        ("medium", include_bytes!("../../fonts/PublicSans-Medium.ttf")),
        ("semi", include_bytes!("../../fonts/PublicSans-SemiBold.ttf")),
        ("mono", include_bytes!("../../fonts/IBMPlexMono-Regular.ttf")),
    ];
    for (name, bytes) in faces {
        f.font_data
            .insert(name.into(), FontData::from_static(bytes).into());
    }
    f.families.insert(FontFamily::Proportional, vec!["sans".into()]);
    f.families
        .insert(FontFamily::Monospace, vec!["mono".into(), "sans".into()]);
    f.families.insert(
        FontFamily::Name("medium".into()),
        vec!["medium".into(), "sans".into()],
    );
    f.families.insert(
        FontFamily::Name("semi".into()),
        vec!["semi".into(), "sans".into()],
    );
    f
}

/// egui's own widgets (fields, menus, scrollbars) in these colours: flat, 4 px radius, no shadow.
pub fn apply(ctx: &egui::Context, t: &Theme) {
    let t = *t;
    ctx.all_styles_mut(move |s| {
        s.visuals = if t.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        let v = &mut s.visuals;
        v.window_shadow = egui::Shadow::NONE;
        v.popup_shadow = egui::Shadow::NONE;
        v.window_fill = t.surface;
        v.panel_fill = t.surface;
        v.window_stroke = Stroke::new(1.0, t.line);
        v.window_corner_radius = 4.into();
        v.menu_corner_radius = 4.into();
        v.extreme_bg_color = t.field;
        v.text_edit_bg_color = Some(t.field);
        v.override_text_color = Some(t.ink);
        v.selection.bg_fill = t.selection;
        v.selection.stroke = Stroke::new(1.0, t.focus);
        v.text_cursor.stroke = Stroke::new(1.0, t.ink);
        let w = &mut v.widgets;
        for x in [
            &mut w.noninteractive,
            &mut w.inactive,
            &mut w.hovered,
            &mut w.active,
            &mut w.open,
        ] {
            x.corner_radius = 4.into();
            x.expansion = 0.0;
            x.fg_stroke = Stroke::new(1.0, t.ink);
            x.bg_stroke = Stroke::new(1.0, t.line);
            x.bg_fill = t.surface;
            x.weak_bg_fill = t.surface;
        }
        w.noninteractive.bg_stroke = Stroke::new(1.0, t.line);
        // a checkbox's square and a radio's ring: in the fields' colour, an edge that shows
        w.inactive.bg_fill = t.field;
        w.inactive.bg_stroke = Stroke::new(1.0, edge(&t));
        w.hovered.bg_stroke = Stroke::new(1.0, t.ink2);
        w.hovered.bg_fill = t.hover;
        w.hovered.weak_bg_fill = t.hover;
        w.active.bg_fill = t.on;
        w.active.weak_bg_fill = t.on;
        w.active.bg_stroke = Stroke::new(1.0, t.focus);
        s.spacing.item_spacing = egui::vec2(0.0, 0.0);
        s.spacing.button_padding = egui::vec2(8.0, 5.0);
        s.interaction.tooltip_delay = 0.5;
    });
}
