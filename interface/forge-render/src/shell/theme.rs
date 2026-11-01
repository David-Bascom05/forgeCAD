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
    pub accent: Color32,
    pub field: Color32,
    pub view: u32,
    pub on: Color32,
    pub on_ink: Color32,
    pub focus: Color32,
    pub error: Color32,
    pub selection: Color32,
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
    accent: c(0x2F7D59),
    field: c(0x0F1318),
    view: 0x12171D,
    on: c(0x1B2838),
    on_ink: c(0x9CC3EA),
    focus: c(0xE0A21B),
    error: c(0xE0554A),
    selection: c(0x2C4A6B),
};

pub const LIGHT: Theme = Theme {
    dark: false,
    bar: c(0x1D4470),
    bg: c(0xEEF1F5),
    surface: c(0xFFFFFF),
    raised: c(0xF7F8FA),
    ink: c(0x1A1F1C),
    ink2: c(0x525A55),
    line: c(0xD5D9D3),
    hover: c(0xEEF1F5),
    accent: c(0x1D5C42),
    field: c(0xFFFFFF),
    view: 0xDDE2E8,
    on: c(0xE3ECF5),
    on_ink: c(0x1D4470),
    focus: c(0xE0A21B),
    error: c(0xB83A30),
    selection: c(0xB9D2EC),
};

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
