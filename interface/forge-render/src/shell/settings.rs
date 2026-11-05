//! Settings: Inventor's Application Options as one floating window from Manage > Settings. Its
//! tabs hold what was about the app (the theme, part colours, the view's background and
//! defaults, units, every key, the mouse with the CAD tools' presets); what's set takes effect at
//! once and is kept in `settings.toml` under `$XDG_CONFIG_HOME/forgecad`.

use super::fgasm::{dialog_frame, dialog_head};
use super::materials::{button_at, section};
use super::theme::{Theme, medium, sans, semi};
use super::{ASM_OVERLAYS, Action, OVERLAY_SWITCHES, Overlays, PART_OVERLAYS, Shell, theme};
use crate::bindings::{
    Chord, Gesture, KeyCommand, KeyMap, MouseMap, NAV_ACTIONS, NavAction, Preset, Toml, gesture_choices,
    preset_map, presets,
};
use crate::render::{Backdrop, VisualStyle};
use egui::{Color32, Rect, Stroke, StrokeKind, Ui, pos2, vec2};

pub const ANGLES: [&str; 2] = ["deg", "rad"];

/// What a section draws: the settings being changed, the window's own state (the key list, the
/// presets, a key being set), and the names of what it drew, for tests and scripts.
pub type SectionUi = fn(&mut Ui, &Theme, &mut Settings, &mut Window, &mut Vec<(String, Rect)>);

/// A tab of the Settings window. A new one is a module with its `SectionUi` that keeps its own
/// `[table]` in `Settings::extra` (`extra.get(table, key)`, `extra.set(table, key, value)`: it's
/// written to settings.toml and read back as it is), and one line here.
pub const SECTIONS: &[(&str, SectionUi)] = &[
    ("Appearance", appearance),
    ("Display", display),
    ("Units", units),
    ("Keyboard", keyboard),
    ("Mouse", mouse),
    ("Agent", super::agent_settings::settings_section),
];

/// The tables the built-in sections own; any other in settings.toml is a registered section's.
const OWN_TABLES: [&str; 6] = ["appearance", "display", "units", "keys", "mouse", "mouse.custom"];
const BACKDROPS: [(Backdrop, &str, &str); 3] = [
    (Backdrop::Floor, "floor", "Inventor floor"),
    (Backdrop::Gradient, "gradient", "Gradient"),
    (Backdrop::Solid, "solid", "Solid colour"),
];

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub dark: bool,
    pub pastel: bool,
    pub backdrop: Backdrop,
    /// The view's visual style (the View tab's dropdown sets it too).
    pub style: VisualStyle,
    pub parallel: bool,
    /// What a part's and an assembly's view show when opened.
    pub overlays: [Overlays; 2],
    pub length: &'static str,
    pub angle: &'static str,
    pub decimals: usize,
    /// Seconds the pointer rests on a tool before its tooltip, and before the longer one.
    pub tip_delay: f32,
    pub tip_long: f32,
    pub keys: KeyMap,
    pub mouse_preset: String,
    /// Actions whose gestures the person set, in place of the preset's.
    pub mouse_custom: Vec<(NavAction, Vec<Gesture>)>,
    pub invert_zoom: bool,
    /// The registered sections' tables, kept as they are.
    pub extra: Toml,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            dark: true,
            pastel: false,
            backdrop: Backdrop::Floor,
            style: VisualStyle::ShadedEdges,
            parallel: true,
            overlays: [PART_OVERLAYS, ASM_OVERLAYS],
            length: "mm",
            angle: "deg",
            decimals: 3,
            tip_delay: 0.6,
            tip_long: 2.0,
            keys: KeyMap::default(),
            mouse_preset: presets()[0].id.clone(),
            mouse_custom: Vec::new(),
            invert_zoom: false,
            extra: Toml::default(),
        }
    }
}

fn word<T: Copy + PartialEq>(table: &[(T, &'static str, &'static str)], v: T) -> &'static str {
    table.iter().find(|r| r.0 == v).map_or("", |r| r.1)
}

fn by_word<T: Copy>(table: &[(T, &'static str, &'static str)], w: &str) -> Option<T> {
    table.iter().find(|r| r.1 == w).map(|r| r.0)
}

fn known(list: &[&'static str], w: &str) -> Option<&'static str> {
    list.iter().find(|u| **u == w).copied()
}

impl Settings {
    pub fn preset(&self) -> Preset {
        let all = presets();
        all.iter()
            .find(|p| p.id == self.mouse_preset)
            .unwrap_or(&all[0])
            .clone()
    }

    /// The mouse table in force: the preset's, the person's own gestures, the wheel turned round.
    pub fn mouse_map(&self) -> MouseMap {
        let mut m = preset_map(&self.preset());
        for (a, gs) in &self.mouse_custom {
            m.binds.retain(|b| b.1 != *a);
            m.binds.extend(gs.iter().map(|g| (g.clone(), *a)));
        }
        if self.invert_zoom {
            m.forward_zooms_in = !m.forward_zooms_in;
        }
        m
    }

    pub fn to_toml(&self) -> String {
        let mut t = Toml::default();
        let b = |v: bool| if v { "true" } else { "false" };
        t.set("appearance", "theme", if self.dark { "dark" } else { "light" });
        t.set(
            "appearance",
            "part_colours",
            if self.pastel { "pastel" } else { "grey" },
        );
        t.set("appearance", "background", word(&BACKDROPS, self.backdrop));
        t.set("display", "visual_style", self.style.key());
        t.set(
            "display",
            "projection",
            if self.parallel {
                "orthographic"
            } else {
                "perspective"
            },
        );
        for (k, kind) in ["part", "assembly"].into_iter().enumerate() {
            for (name, _) in OVERLAY_SWITCHES {
                t.set(
                    "display",
                    &format!("{kind}_{name}"),
                    b(self.overlays[k].get(name)),
                );
            }
        }
        t.set("units", "length", self.length);
        t.set("units", "angle", self.angle);
        t.set("units", "decimals", self.decimals.to_string());
        t.set("display", "tooltip_delay", self.tip_delay.to_string());
        t.set("display", "tooltip_long", self.tip_long.to_string());
        for (id, k) in &self.keys.custom {
            t.set("keys", id, k.map_or(String::new(), |k| k.to_string()));
        }
        t.set("mouse", "preset", self.mouse_preset.clone());
        t.set("mouse", "invert_zoom", b(self.invert_zoom));
        for (a, gs) in &self.mouse_custom {
            let name = NAV_ACTIONS.iter().find(|n| n.0 == *a).map_or("", |n| n.1);
            let list: Vec<String> = gs.iter().map(|g| g.to_string()).collect();
            t.set("mouse.custom", name, list.join(", "));
        }
        t.entries.extend(self.extra.entries.iter().cloned());
        t.write()
    }

    pub fn from_toml(text: &str) -> Settings {
        let t = Toml::parse(text);
        let mut s = Settings::default();
        s.extra.entries = t
            .entries
            .iter()
            .filter(|e| !OWN_TABLES.contains(&e.0.as_str()))
            .cloned()
            .collect();
        let get = |sec: &str, k: &str| t.get(sec, k).unwrap_or("").to_string();
        s.dark = get("appearance", "theme") != "light";
        s.pastel = get("appearance", "part_colours") == "pastel";
        s.backdrop = by_word(&BACKDROPS, &get("appearance", "background")).unwrap_or(s.backdrop);
        // the edges setting it replaces: no lines was Shaded, the rest Shaded with Edges
        s.style = VisualStyle::from_key(&get("display", "visual_style")).unwrap_or(
            if get("display", "edges") == "none" {
                VisualStyle::Shaded
            } else {
                s.style
            },
        );
        s.parallel = get("display", "projection") != "perspective";
        for (k, kind) in ["part", "assembly"].into_iter().enumerate() {
            for (name, _) in OVERLAY_SWITCHES {
                match get("display", &format!("{kind}_{name}")).as_str() {
                    "true" if !s.overlays[k].get(name) => s.overlays[k].flip(name),
                    "false" if s.overlays[k].get(name) => s.overlays[k].flip(name),
                    _ => {}
                }
            }
        }
        s.length = known(&forge_doc::units::DOC_LENGTHS, &get("units", "length")).unwrap_or("mm");
        s.angle = known(&ANGLES, &get("units", "angle")).unwrap_or("deg");
        s.decimals = get("units", "decimals").parse().unwrap_or(3).min(6);
        let secs = |k: &str, d: f32| {
            get("display", k)
                .parse::<f32>()
                .ok()
                .filter(|v| v.is_finite())
                .map_or(d, |v| v.clamp(0.0, 10.0))
        };
        s.tip_delay = secs("tooltip_delay", s.tip_delay);
        s.tip_long = secs("tooltip_long", s.tip_long);
        let cmds = crate::bindings::commands();
        for (id, k) in t.section("keys") {
            if let Some(c) = cmds.iter().find(|c| c.id == id && !c.fixed) {
                s.keys.set(id, Chord::parse(k), c.default);
            }
        }
        if let Some(p) = t
            .get("mouse", "preset")
            .filter(|p| presets().iter().any(|q| q.id == *p))
        {
            s.mouse_preset = p.to_string();
        }
        s.invert_zoom = get("mouse", "invert_zoom") == "true";
        for (name, list) in t.section("mouse.custom") {
            if let Some((a, _, _)) = NAV_ACTIONS.iter().find(|n| n.1 == name) {
                let gs = list.split(',').filter_map(|g| Gesture::parse(g.trim())).collect();
                s.mouse_custom.push((*a, gs));
            }
        }
        s
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn config_dir() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))?;
    Some(base.join("forgecad"))
}

/// What's kept, or the old `settings` file's theme and part colours, or the defaults.
#[cfg(not(target_arch = "wasm32"))]
pub fn load() -> Settings {
    let Some(dir) = config_dir() else {
        return Settings::default();
    };
    if let Ok(text) = std::fs::read_to_string(dir.join("settings.toml")) {
        return Settings::from_toml(&text);
    }
    let old = std::fs::read_to_string(dir.join("settings")).unwrap_or_default();
    let mut s = Settings::default();
    for l in old.lines() {
        match l.split_once('=').map(|(k, v)| (k.trim(), v.trim())) {
            Some(("theme", "light")) => s.dark = false,
            Some(("part_colours", "pastel")) => s.pastel = true,
            _ => {}
        }
    }
    s
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save(s: &Settings) {
    let Some(dir) = config_dir() else { return };
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join("settings.toml"), s.to_toml());
}

/// The window's own state.
pub struct Window {
    pub open: bool,
    pub tab: usize,
    /// The command waiting for its new key.
    pub capture: Option<String>,
    pub cmds: Vec<KeyCommand>,
    pub presets: Vec<Preset>,
    /// The mouse table the settings make, for the view.
    pub mouse: MouseMap,
    /// What the key list is narrowed to.
    pub search: String,
}

impl Default for Window {
    fn default() -> Window {
        Window {
            open: false,
            tab: 0,
            capture: None,
            cmds: crate::bindings::commands(),
            presets: presets(),
            mouse: MouseMap::default(),
            search: String::new(),
        }
    }
}

const WIN_W: f32 = 900.0;
const WIN_H: f32 = 680.0;
const HEAD_H: f32 = 48.0;
const SIDE_W: f32 = 184.0;
const FOOT_H: f32 = 56.0;
/// A form row: its words' column and height; a control's usual width.
pub const LABEL_W: f32 = 170.0;
pub const ROW_H: f32 = 36.0;

/// A row of a form: the words at the left, the rest of the row for `body`.
pub fn row(ui: &mut Ui, t: &Theme, label: &str, body: impl FnOnce(&mut Ui)) {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, ROW_H), egui::Sense::hover());
    ui.painter().text(
        pos2(r.left(), r.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        sans(12.5),
        t.ink2,
    );
    let at = Rect::from_min_max(
        pos2(r.left() + LABEL_W, r.top() + 5.0),
        pos2(r.right(), r.bottom() - 5.0),
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(at)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            body(ui)
        },
    );
}

/// A few choices side by side as one control, the chosen one lit; which was clicked.
pub fn segmented(
    ui: &mut Ui,
    t: &Theme,
    items: &[(&str, bool)],
    name: &dyn Fn(usize) -> String,
    places: &mut Vec<(String, Rect)>,
) -> Option<usize> {
    let ws: Vec<f32> = items
        .iter()
        .map(|(s, _)| {
            ui.fonts_mut(|f| f.layout_no_wrap(s.to_string(), sans(12.5), t.ink))
                .size()
                .x
                + 24.0
        })
        .map(|w| w.max(44.0))
        .collect();
    let (all, _) = ui.allocate_exact_size(vec2(ws.iter().sum(), 26.0), egui::Sense::hover());
    let p = ui.painter().clone();
    p.rect_filled(all, 4.0, t.field);
    let mut x = all.left();
    let mut out = None;
    for (k, ((text, on), w)) in items.iter().zip(&ws).enumerate() {
        let r = Rect::from_min_size(pos2(x, all.top()), vec2(*w, all.height()));
        x += w;
        let resp = ui.interact(r, ui.scope_id().with(("seg", name(k))), egui::Sense::click());
        resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::RadioButton, true, *on, *text));
        if *on {
            p.rect_filled(r, 4.0, t.on);
            p.rect_stroke(
                r,
                4.0,
                Stroke::new(1.0, t.on_ink.gamma_multiply(0.7)),
                StrokeKind::Inside,
            );
        } else {
            if resp.hovered() {
                p.rect_filled(r.shrink(1.0), 3.0, t.hover);
            }
            if k > 0 && !items[k - 1].1 {
                p.vline(r.left(), r.y_range().shrink(5.0), Stroke::new(1.0, t.line));
            }
        }
        p.text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            *text,
            if *on { medium(12.5) } else { sans(12.5) },
            if *on { t.on_ink } else { t.ink },
        );
        places.push((name(k), r));
        if resp.clicked() {
            out = Some(k);
        }
    }
    p.rect_stroke(all, 4.0, Stroke::new(1.0, theme::edge(t)), StrokeKind::Inside);
    out
}

/// Cards in rows, each a picture over its words; which was clicked. `paint` draws card k's
/// picture in its rect.
#[allow(clippy::too_many_arguments)]
pub fn cards(
    ui: &mut Ui,
    t: &Theme,
    items: &[(&str, bool)],
    size: egui::Vec2,
    name: &dyn Fn(usize) -> String,
    places: &mut Vec<(String, Rect)>,
    paint: &dyn Fn(&egui::Painter, Rect, usize),
) -> Option<usize> {
    let w = ui.available_width();
    let gap = 10.0;
    let per = (((w + gap) / (size.x + gap)).floor() as usize).max(1);
    let rows = items.len().div_ceil(per);
    let (all, _) = ui.allocate_exact_size(vec2(w, rows as f32 * (size.y + gap) - gap), egui::Sense::hover());
    let p = ui.painter().clone();
    let mut out = None;
    for (k, (text, on)) in items.iter().enumerate() {
        let r = Rect::from_min_size(
            all.min
                + vec2(
                    (k % per) as f32 * (size.x + gap),
                    (k / per) as f32 * (size.y + gap),
                ),
            size,
        );
        let resp = ui.interact(r, ui.scope_id().with(("card", name(k))), egui::Sense::click());
        resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::RadioButton, true, *on, *text));
        p.rect_filled(
            r,
            6.0,
            if *on {
                t.on
            } else if resp.hovered() {
                t.hover
            } else {
                t.raised
            },
        );
        let pic = Rect::from_min_max(r.min + vec2(8.0, 8.0), pos2(r.right() - 8.0, r.bottom() - 30.0));
        paint(&p.with_clip_rect(pic), pic, k);
        p.rect_stroke(pic, 3.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
        let words = Rect::from_min_max(
            pos2(r.left() + 10.0, r.bottom() - 25.0),
            pos2(r.right() - 8.0, r.bottom() - 5.0),
        );
        super::materials::clipped(
            ui,
            words,
            text,
            if *on { medium(12.5) } else { sans(12.5) },
            if *on { t.on_ink } else { t.ink },
            &name(k),
        );
        let edge = if *on {
            Stroke::new(1.5, t.on_ink)
        } else {
            Stroke::new(1.0, t.line)
        };
        p.rect_stroke(r, 6.0, edge, StrokeKind::Inside);
        places.push((name(k), r));
        if resp.clicked() {
            out = Some(k);
        }
    }
    out
}

impl Shell {
    /// Manage > Settings: the window, and what changing it does at once.
    pub(super) fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.prefs.open {
            return;
        }
        let t = self.theme;
        let was = self.settings.clone();
        let mut s = self.settings.clone();
        let mut places = Vec::new();
        let mut close = false;
        let prefs = &mut self.prefs;
        let area = super::dialog_area(ctx);
        let h = WIN_H.min(area.height() - 16.0).max(400.0);
        let at = pos2(
            area.center().x - WIN_W * 0.5,
            (area.top() + 60.0)
                .min(area.bottom() - h - 8.0)
                .max(area.top() + 4.0),
        );
        let r = egui::Window::new("Settings")
            .id(egui::Id::unique("settings-window"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .fade_in(false)
            .default_pos(at)
            .fixed_size(vec2(WIN_W, h))
            .constrain_to(area)
            // over the view's own drawings (Measure's marks), which are a layer of their own
            .order(egui::Order::Foreground)
            .frame(dialog_frame(&t).inner_margin(0))
            .show(ctx, |ui| {
                let all = Rect::from_min_size(ui.max_rect().min, vec2(WIN_W, h));
                ui.allocate_rect(all, egui::Sense::hover());
                let head = Rect::from_min_size(all.min, vec2(WIN_W, HEAD_H));
                let side = Rect::from_min_max(
                    pos2(all.left(), head.bottom()),
                    pos2(all.left() + SIDE_W, all.bottom()),
                );
                let main = Rect::from_min_max(pos2(side.right(), head.bottom()), all.max);
                let foot = Rect::from_min_max(pos2(main.left(), main.bottom() - FOOT_H), main.max);
                let p = ui.painter().clone();
                p.rect_filled(
                    side,
                    egui::CornerRadius {
                        nw: 0,
                        ne: 0,
                        sw: 6,
                        se: 0,
                    },
                    t.raised,
                );
                p.hline(all.x_range(), head.bottom() - 0.5, Stroke::new(1.0, t.line));
                p.vline(side.right() - 0.5, side.y_range(), Stroke::new(1.0, t.line));
                p.hline(foot.x_range(), foot.top() + 0.5, Stroke::new(1.0, t.line));
                ui.scope_builder(
                    egui::UiBuilder::new().max_rect(head.shrink2(vec2(16.0, 11.0))),
                    |ui| {
                        close = dialog_head(ui, &t, "options", "Settings", "settings:close", &mut places);
                    },
                );
                // the sections, Inventor's Application Options' list
                let mut y = side.top() + 10.0;
                for (k, (name, _)) in SECTIONS.iter().enumerate() {
                    let on = k == prefs.tab;
                    let r = Rect::from_min_size(pos2(side.left() + 8.0, y), vec2(SIDE_W - 16.0, 34.0));
                    y += 36.0;
                    let resp = ui.interact(r, ui.scope_id().with(("settings-tab", k)), egui::Sense::click());
                    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, on, *name));
                    if on {
                        p.rect_filled(r, 4.0, t.on);
                        p.vline(
                            r.left() + 1.0,
                            r.y_range().shrink(7.0),
                            Stroke::new(2.5, t.on_ink),
                        );
                    } else if resp.hovered() {
                        p.rect_filled(r, 4.0, t.hover);
                    }
                    p.text(
                        pos2(r.left() + 14.0, r.center().y),
                        egui::Align2::LEFT_CENTER,
                        *name,
                        medium(13.0),
                        if on { t.on_ink } else { t.ink },
                    );
                    places.push((format!("settings:tab:{name}"), r));
                    if resp.clicked() {
                        prefs.tab = k;
                        prefs.capture = None;
                    }
                }
                let tab = prefs.tab.min(SECTIONS.len() - 1);
                let (title, draw) = SECTIONS[tab];
                let body = Rect::from_min_max(
                    pos2(main.left() + 24.0, main.top() + 14.0),
                    pos2(main.right() - 8.0, foot.top() - 4.0),
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
                    ui.painter().text(
                        pos2(body.left(), body.top() + 12.0),
                        egui::Align2::LEFT_CENTER,
                        title,
                        semi(17.0),
                        t.ink,
                    );
                    ui.add_space(28.0);
                    ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
                    egui::ScrollArea::vertical()
                        .id_salt(("settings-body", tab))
                        .auto_shrink([false, false])
                        .max_height(body.height() - 28.0)
                        .show(ui, |ui| {
                            ui.set_width(body.width() - 20.0);
                            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                            draw(ui, &t, &mut s, prefs, &mut places);
                            ui.add_space(12.0);
                        });
                });
                let y = foot.center().y;
                let b = Rect::from_min_size(pos2(foot.left() + 24.0, y - 15.0), vec2(96.0, 30.0));
                let defaults = Settings {
                    extra: s.extra.clone(),
                    ..Settings::default()
                };
                let r = button_at(ui, &t, b, "Reset All", false, s != defaults);
                places.push(("settings:reset-all".into(), r.rect));
                if r.clicked() {
                    // the built-in sections' defaults back; a registered section's own table
                    // (the Agent's backend, model, trusted folders) is its own, kept
                    s = Settings {
                        extra: std::mem::take(&mut s.extra),
                        ..Settings::default()
                    };
                    prefs.capture = None;
                }
                let b = Rect::from_min_size(pos2(foot.right() - 20.0 - 84.0, y - 15.0), vec2(84.0, 30.0));
                let r = button_at(ui, &t, b, "Close", true, true);
                places.push(("settings:done".into(), r.rect));
                close |= r.clicked();
            });
        if let Some(r) = r {
            self.view_blocks.push(r.response.rect);
        }
        self.places.extend(places);
        if close {
            self.prefs.capture = None;
        }
        self.prefs.open = !close;
        if s != was {
            self.settings = s;
            self.settings_changed(&was);
        }
    }

    /// Settings as loaded: everything they decide, without the actions a change makes.
    pub fn use_settings(&mut self, s: Settings) {
        self.theme = if s.dark { theme::DARK } else { theme::LIGHT };
        self.view.pastel = s.pastel;
        self.view.style = s.style;
        self.view.parallel = s.parallel;
        self.overlays = s.overlays;
        self.units = s.length;
        self.prefs.mouse = s.mouse_map();
        self.settings = s;
    }

    /// What a change does now: the rest is read where it's used (the mouse, the background).
    pub fn settings_changed(&mut self, was: &Settings) {
        let s = self.settings.clone();
        self.prefs.mouse = s.mouse_map();
        if s.dark != was.dark {
            self.theme = if s.dark { theme::DARK } else { theme::LIGHT };
        }
        if s.pastel != was.pastel {
            self.out.push(Action::PartColours(s.pastel));
        }
        if s.length != was.length {
            self.out.push(Action::SetUnits(s.length));
        }
        if s.style != was.style {
            self.out.push(Action::VisualStyle(s.style));
        }
        if s.parallel != was.parallel {
            self.out.push(Action::Parallel(s.parallel));
        }
        if s.overlays != was.overlays {
            self.overlays = s.overlays;
        }
        if let Some(m) = &mut self.inspect.measure {
            m.precision = s.decimals;
            m.angle = s.angle;
        }
    }

    /// The keys in force, when no field has the keyboard: each runs its command; while a key
    /// is being set in Settings the next one is it.
    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) {
        let keys: Vec<(egui::Key, egui::Modifiers)> = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        repeat: false,
                        modifiers,
                        ..
                    } => Some((*key, *modifiers)),
                    _ => None,
                })
                .collect()
        });
        if let Some(id) = self.prefs.capture.clone() {
            let Some(&(key, m)) = keys.first() else { return };
            self.prefs.capture = None;
            if key == egui::Key::Escape {
                ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
                return;
            }
            let default = self
                .prefs
                .cmds
                .iter()
                .find(|c| c.id == id)
                .and_then(|c| c.default);
            let was = self.settings.clone();
            self.settings
                .keys
                .set(&id, Some(Chord::from_event(key, m)), default);
            self.settings_changed(&was);
            return;
        }
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        // in a sketch its tools' keys are its own
        let sketch_keys = if self.sketch.is_some() {
            super::sketching::sketch_keys(self)
        } else {
            vec![]
        };
        for (key, m) in keys {
            if sketch_keys.iter().any(|k| k.1.matches(key, m)) {
                continue;
            }
            let Some(id) = self.settings.keys.command_for(&self.prefs.cmds, key, m) else {
                continue;
            };
            let Some(cmd) = KeyCmd::from_id(&id) else { continue };
            match cmd {
                KeyCmd::Rename | KeyCmd::Delete => {
                    let Some(sel) = self.selected else { continue };
                    self.out.push(if cmd == KeyCmd::Rename {
                        Action::StartRename(sel)
                    } else {
                        Action::Delete(sel)
                    });
                }
                c => self.out.push(Action::Key(c)),
            }
        }
    }
}

/// What a shortcut runs; the key it's on is data (`data/keymap.csv`, the person's settings).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyCmd {
    Undo,
    Redo,
    Open,
    Save,
    Measure,
    Rename,
    Delete,
    Home,
    FitAll,
    FitSelected,
    Section,
    Explode,
    Lines,
    Ao,
}

impl KeyCmd {
    pub fn from_id(id: &str) -> Option<KeyCmd> {
        Some(match id {
            "undo" => KeyCmd::Undo,
            "redo" | "redo_alt" => KeyCmd::Redo,
            "open" => KeyCmd::Open,
            "save" => KeyCmd::Save,
            "measure" => KeyCmd::Measure,
            "rename" => KeyCmd::Rename,
            "delete" => KeyCmd::Delete,
            "home" | "home_alt" => KeyCmd::Home,
            "fit_all" => KeyCmd::FitAll,
            "fit_selected" => KeyCmd::FitSelected,
            "section" => KeyCmd::Section,
            "explode" => KeyCmd::Explode,
            "lines" => KeyCmd::Lines,
            "ao" => KeyCmd::Ao,
            _ => return None,
        })
    }
}

fn rgb(c: [f32; 3]) -> Color32 {
    let s = |x: f32| (x.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0).round() as u8;
    Color32::from_rgb(s(c[0]), s(c[1]), s(c[2]))
}

/// A view's background as a card shows it: the renderer's colours for the theme.
fn backdrop_preview(p: &egui::Painter, r: Rect, b: Backdrop, t: &Theme) {
    let v = t.view;
    let flat = [(v >> 16) as u8, (v >> 8) as u8, v as u8].map(|x| (x as f32 / 255.0).powf(2.2));
    let [top, bottom, floor] = crate::render::backdrop_colours(flat).map(rgb);
    match b {
        Backdrop::Solid => {
            p.rect_filled(r, 3.0, rgb(flat));
        }
        _ => {
            // a gradient, top to bottom, in bands
            let n = 24;
            for k in 0..n {
                let f = k as f32 / (n - 1) as f32;
                let c = lerp(top, bottom, f);
                let y0 = r.top() + r.height() * k as f32 / n as f32;
                let band = Rect::from_min_max(
                    pos2(r.left(), y0),
                    pos2(r.right(), y0 + r.height() / n as f32 + 1.0),
                );
                p.rect_filled(band, 0.0, c);
            }
            if b == Backdrop::Floor {
                // the floor's grid going away from the eye
                let horizon = r.top() + r.height() * 0.48;
                let ground = Rect::from_min_max(pos2(r.left(), horizon), r.max);
                p.rect_filled(ground, 0.0, floor.gamma_multiply(0.85));
                let ink = lerp(floor, Color32::WHITE, if t.dark { 0.12 } else { 0.45 });
                for k in 1..6 {
                    let f = (k as f32 / 6.0).powf(1.6);
                    p.hline(
                        r.x_range(),
                        horizon + (r.bottom() - horizon) * f,
                        Stroke::new(1.0, ink),
                    );
                }
                for k in -6..=6 {
                    let x = r.center().x + k as f32 * r.width() / 10.0;
                    p.line_segment(
                        [
                            pos2(r.center().x + k as f32 * 4.0, horizon),
                            pos2(x * 1.0 + (x - r.center().x) * 0.8, r.bottom()),
                        ],
                        Stroke::new(1.0, ink),
                    );
                }
            }
        }
    }
    // a part on it, for scale
    super::materials::ball(
        p,
        pos2(r.center().x, r.top() + r.height() * 0.5),
        r.height() * 0.2,
        Color32::from_rgb(0xC8, 0xCC, 0xD2),
    );
}

fn lerp(a: Color32, b: Color32, f: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * f).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

/// The app in a theme, small: its bar, browser, view and a part.
fn theme_preview(p: &egui::Painter, r: Rect, th: &Theme) {
    p.rect_filled(r, 3.0, th.surface);
    let bar = Rect::from_min_max(r.min, pos2(r.right(), r.top() + 9.0));
    p.rect_filled(bar, 0.0, th.bar);
    let side = Rect::from_min_max(
        pos2(r.left(), bar.bottom()),
        pos2(r.left() + r.width() * 0.28, r.bottom()),
    );
    p.rect_filled(side, 0.0, th.surface);
    for k in 0..4 {
        let y = side.top() + 8.0 + k as f32 * 8.0;
        p.hline(
            side.left() + 5.0..=side.right() - 6.0 - (k % 2) as f32 * 8.0,
            y,
            Stroke::new(2.0, th.ink2.gamma_multiply(0.6)),
        );
    }
    let v = th.view;
    let view = Rect::from_min_max(pos2(side.right(), bar.bottom()), r.max);
    p.rect_filled(
        view,
        0.0,
        Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8),
    );
    super::materials::ball(
        p,
        view.center(),
        view.height() * 0.26,
        Color32::from_rgb(0xC8, 0xCC, 0xD2),
    );
}

fn appearance(ui: &mut Ui, t: &Theme, s: &mut Settings, _: &mut Window, places: &mut Vec<(String, Rect)>) {
    let card = vec2(170.0, 124.0);
    section(ui, t, "Theme");
    let items = [("Light", !s.dark), ("Dark", s.dark)];
    let names = ["Light", "Dark"];
    if let Some(k) = cards(
        ui,
        t,
        &items,
        card,
        &|k| format!("settings:theme:{}", names[k]),
        places,
        &|p, r, k| theme_preview(p, r, if k == 0 { &theme::LIGHT } else { &theme::DARK }),
    ) {
        s.dark = k == 1;
    }
    section(ui, t, "Part colours");
    let items = [("Inventor grey", !s.pastel), ("A pastel per part", s.pastel)];
    if let Some(k) = cards(
        ui,
        t,
        &items,
        card,
        &|k| format!("settings:parts:{}", k == 1),
        places,
        &|p, r, k| {
            let v = t.view;
            p.rect_filled(
                r,
                3.0,
                Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8),
            );
            let cols = if k == 0 {
                [0xC8CCD2, 0xBFC3C9, 0xD2D5DA]
            } else {
                [0xE8A7A0, 0xA6C8E8, 0xB8DDB0]
            };
            for (j, c) in cols.iter().enumerate() {
                let c = Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, *c as u8);
                let at = pos2(
                    r.left() + r.width() * (0.25 + 0.25 * j as f32),
                    r.center().y + if j == 1 { -6.0 } else { 6.0 },
                );
                super::materials::ball(p, at, r.height() * 0.2, c);
            }
        },
    ) {
        s.pastel = k == 1;
    }
    section(ui, t, "Background");
    let items: Vec<(&str, bool)> = BACKDROPS.iter().map(|b| (b.2, s.backdrop == b.0)).collect();
    if let Some(k) = cards(
        ui,
        t,
        &items,
        card,
        &|k| format!("settings:background:{}", BACKDROPS[k].1),
        places,
        &|p, r, k| backdrop_preview(p, r, BACKDROPS[k].0, t),
    ) {
        s.backdrop = BACKDROPS[k].0;
    }
}

fn display(ui: &mut Ui, t: &Theme, s: &mut Settings, _: &mut Window, places: &mut Vec<(String, Rect)>) {
    section(ui, t, "View");
    row(ui, t, "Visual style", |ui| {
        let r = egui::ComboBox::from_id_salt("settings visual style")
            .selected_text(egui::RichText::new(s.style.name()).font(sans(12.5)))
            .width(260.0)
            .show_ui(ui, |ui| {
                for v in VisualStyle::ALL {
                    let r =
                        ui.selectable_value(&mut s.style, v, egui::RichText::new(v.name()).font(sans(12.5)));
                    places.push((format!("settings:style:{}", v.key()), r.rect));
                }
            });
        places.push(("settings:visual_style".into(), r.response.rect));
    });
    row(ui, t, "Projection", |ui| {
        let words = ["Orthographic", "Perspective"];
        let items = [(words[0], s.parallel), (words[1], !s.parallel)];
        if let Some(k) = segmented(
            ui,
            t,
            &items,
            &|k| format!("settings:projection:{}", words[k]),
            places,
        ) {
            s.parallel = k == 0;
        }
    });
    section(ui, t, "Shown when a file opens");
    // a table: what's shown down the side, parts and assemblies across
    let w = ui.available_width();
    let col = |j: usize| LABEL_W + 50.0 + 110.0 * j as f32;
    let (r, _) = ui.allocate_exact_size(vec2(w, 26.0), egui::Sense::hover());
    for (j, word) in ["Parts", "Assemblies"].iter().enumerate() {
        ui.painter().text(
            pos2(r.left() + col(j), r.center().y),
            egui::Align2::CENTER_CENTER,
            *word,
            medium(12.0),
            t.ink2,
        );
    }
    for (name, word) in OVERLAY_SWITCHES {
        let (r, _) = ui.allocate_exact_size(vec2(w, 32.0), egui::Sense::hover());
        ui.painter().text(
            pos2(r.left(), r.center().y),
            egui::Align2::LEFT_CENTER,
            word,
            sans(12.5),
            t.ink,
        );
        for k in 0..2 {
            let mut on = s.overlays[k].get(name);
            let at = Rect::from_center_size(pos2(r.left() + col(k) + 4.0, r.center().y), vec2(22.0, 22.0));
            let resp = ui.put(at, egui::Checkbox::without_text(&mut on));
            places.push((format!("settings:show:{k}:{name}"), resp.rect));
            if on != s.overlays[k].get(name) {
                s.overlays[k].flip(name);
            }
        }
    }
    tooltips(ui, t, s, places);
}

/// The tooltips' delays: the short tip, then the longer one with its picture.
fn tooltips(ui: &mut Ui, t: &Theme, s: &mut Settings, places: &mut Vec<(String, Rect)>) {
    section(ui, t, "Tooltips");
    for (words, key, v) in [
        ("Tooltip after", "tip_delay", &mut s.tip_delay),
        ("Longer tooltip after", "tip_long", &mut s.tip_long),
    ] {
        row(ui, t, words, |ui| {
            let r = ui.add_sized(
                vec2(84.0, 26.0),
                egui::DragValue::new(v)
                    .range(0.0..=10.0)
                    .speed(0.05)
                    .fixed_decimals(1)
                    .suffix(" s"),
            );
            places.push((format!("settings:{key}"), r.rect));
        });
    }
}

fn units(ui: &mut Ui, t: &Theme, s: &mut Settings, _: &mut Window, places: &mut Vec<(String, Rect)>) {
    section(ui, t, "Units");
    row(ui, t, "Length", |ui| {
        let items: Vec<(&str, bool)> = forge_doc::units::DOC_LENGTHS
            .iter()
            .map(|u| (*u, s.length == *u))
            .collect();
        if let Some(k) = segmented(
            ui,
            t,
            &items,
            &|k| format!("settings:length:{}", forge_doc::units::DOC_LENGTHS[k]),
            places,
        ) {
            s.length = forge_doc::units::DOC_LENGTHS[k];
        }
    });
    row(ui, t, "Angle", |ui| {
        let items: Vec<(&str, bool)> = ANGLES
            .iter()
            .zip(["Degrees", "Radians"])
            .map(|(u, w)| (w, s.angle == *u))
            .collect();
        if let Some(k) = segmented(
            ui,
            t,
            &items,
            &|k| format!("settings:angle:{}", ANGLES[k]),
            places,
        ) {
            s.angle = ANGLES[k];
        }
    });
    section(ui, t, "Precision");
    row(ui, t, "Decimal places", |ui| {
        let words: Vec<String> = (0..=6).map(|n| n.to_string()).collect();
        let items: Vec<(&str, bool)> = words
            .iter()
            .enumerate()
            .map(|(n, w)| (w.as_str(), s.decimals == n))
            .collect();
        if let Some(k) = segmented(ui, t, &items, &|k| format!("settings:decimals:{k}"), places) {
            s.decimals = k;
        }
    });
    // what a measurement reads as with these
    let d = s.decimals;
    let len = 25.4 / forge_doc::units::compound(s.length).map_or(1.0, |(_, n, m)| m / n);
    let ang = if s.angle == "rad" {
        format!("{:.d$} rad", std::f64::consts::FRAC_PI_4)
    } else {
        format!("{:.d$}°", 45.0)
    };
    row(ui, t, "Reads as", |ui| {
        ui.label(
            egui::RichText::new(format!("{len:.d$} {}   ·   {ang}", s.length))
                .font(super::theme::mono(12.5))
                .color(t.ink),
        );
    });
}

/// A key as keycaps, one a part of the chord; their width.
fn keycaps(p: &egui::Painter, t: &Theme, at: egui::Pos2, chord: &str, ink: Color32, edge: Color32) -> f32 {
    let mut x = at.x;
    for (k, part) in chord.split('+').filter(|s| !s.is_empty()).enumerate() {
        if k > 0 {
            p.text(
                pos2(x + 1.0, at.y),
                egui::Align2::LEFT_CENTER,
                "+",
                sans(11.0),
                t.ink2,
            );
            x += 10.0;
        }
        let g = p.layout_no_wrap(part.to_string(), super::theme::mono(11.5), ink);
        let r = Rect::from_min_size(pos2(x, at.y - 10.0), vec2(g.size().x + 12.0, 20.0));
        p.rect_filled(r.translate(vec2(0.0, 1.5)), 4.0, edge.gamma_multiply(0.5));
        p.rect_filled(r, 4.0, t.raised);
        p.rect_stroke(r, 4.0, Stroke::new(1.0, edge), StrokeKind::Inside);
        p.galley(pos2(r.left() + 6.0, r.center().y - g.size().y / 2.0), g, ink);
        x = r.right() + 3.0;
    }
    x - at.x
}

fn keyboard(ui: &mut Ui, t: &Theme, s: &mut Settings, w: &mut Window, places: &mut Vec<(String, Rect)>) {
    let (cmds, capture, search) = (&w.cmds, &mut w.capture, &mut w.search);
    let conflicts = s.keys.conflicts(cmds);
    let width = ui.available_width();
    ui.add_space(4.0);
    let (r, _) = ui.allocate_exact_size(vec2(width, 30.0), egui::Sense::hover());
    let f = ui.put(
        Rect::from_min_size(r.min, vec2(280.0, 28.0)),
        egui::TextEdit::singleline(search)
            .font(sans(12.5))
            .margin(vec2(8.0, 6.0))
            .hint_text("Search commands and keys"),
    );
    places.push(("settings:keys:search".into(), f.rect));
    if !conflicts.is_empty() {
        let n = conflicts.len() / 2;
        ui.painter().text(
            pos2(r.left() + 296.0, r.center().y),
            egui::Align2::LEFT_CENTER,
            format!("{} key{} on two commands", n.max(1), if n > 1 { "s" } else { "" }),
            medium(12.0),
            t.error,
        );
    }
    let b = Rect::from_min_size(pos2(r.right() - 100.0, r.top()), vec2(100.0, 28.0));
    let resp = button_at(ui, t, b, "Reset Keys", false, s.keys != KeyMap::default());
    places.push(("settings:keys:reset".into(), resp.rect));
    if resp.clicked() {
        s.keys = KeyMap::default();
        *capture = None;
    }
    ui.add_space(10.0);
    let key_x = 300.0;
    let (h, _) = ui.allocate_exact_size(vec2(width, 24.0), egui::Sense::hover());
    for (x, word) in [(10.0, "COMMAND"), (key_x, "KEY")] {
        ui.painter().text(
            pos2(h.left() + x, h.center().y),
            egui::Align2::LEFT_CENTER,
            word,
            semi(10.5),
            t.ink2,
        );
    }
    ui.painter()
        .hline(h.x_range(), h.bottom() - 0.5, Stroke::new(1.0, t.line));
    let q = search.trim().to_lowercase();
    let mut last = "";
    let mut odd = false;
    for c in cmds {
        let now = s.keys.get(c);
        let text = now.map(|k| k.to_string()).unwrap_or_default();
        if !q.is_empty()
            && ![&c.name, &c.group, &text]
                .iter()
                .any(|x| x.to_lowercase().contains(&q))
        {
            continue;
        }
        if c.group != last {
            last = &c.group;
            let (g, _) = ui.allocate_exact_size(vec2(width, 30.0), egui::Sense::hover());
            ui.painter().text(
                pos2(g.left() + 10.0, g.bottom() - 9.0),
                egui::Align2::LEFT_CENTER,
                &c.group,
                medium(12.0),
                t.ink2,
            );
            odd = false;
        }
        let (r, _) = ui.allocate_exact_size(vec2(width, 32.0), egui::Sense::hover());
        let p = ui.painter().clone();
        let clash = conflicts.iter().find(|(id, _)| *id == c.id).map(|x| x.1.clone());
        if clash.is_some() {
            p.rect_filled(r, 0.0, t.error.gamma_multiply(0.10));
            p.vline(r.left() + 1.0, r.y_range(), Stroke::new(2.0, t.error));
        } else if odd {
            p.rect_filled(r, 0.0, t.raised);
        }
        odd = !odd;
        p.text(
            pos2(r.left() + 10.0, r.center().y),
            egui::Align2::LEFT_CENTER,
            &c.name,
            sans(12.5),
            t.ink,
        );
        let waiting = capture.as_deref() == Some(c.id.as_str());
        let cell = Rect::from_min_size(pos2(r.left() + key_x - 6.0, r.top() + 3.0), vec2(170.0, 26.0));
        let resp = ui.interact(cell, ui.scope_id().with(("key", &c.id)), egui::Sense::click());
        places.push((format!("settings:key:{}", c.id), cell));
        let edge = if clash.is_some() {
            t.error
        } else {
            super::theme::edge(t)
        };
        if waiting {
            p.rect_stroke(cell, 4.0, Stroke::new(1.5, t.focus), StrokeKind::Inside);
            p.text(
                pos2(cell.left() + 8.0, cell.center().y),
                egui::Align2::LEFT_CENTER,
                "Press a key…",
                sans(12.0),
                t.focus,
            );
        } else {
            if resp.hovered() && !c.fixed {
                p.rect_filled(cell, 4.0, t.hover);
            }
            if text.is_empty() {
                p.text(
                    pos2(cell.left() + 8.0, cell.center().y),
                    egui::Align2::LEFT_CENTER,
                    "—",
                    sans(12.5),
                    t.ink2,
                );
            } else {
                let ink = if c.fixed { t.ink2 } else { t.ink };
                keycaps(&p, t, pos2(cell.left() + 6.0, cell.center().y), &text, ink, edge);
            }
        }
        if c.fixed {
            continue;
        }
        if resp.clicked() {
            *capture = if waiting { None } else { Some(c.id.clone()) };
        }
        if let Some(other) = &clash {
            let room = Rect::from_min_max(
                pos2(cell.right() + 10.0, r.top()),
                pos2(r.right() - 64.0, r.bottom()),
            );
            super::materials::clipped(ui, room, &format!("also {other}"), sans(11.5), t.error, &c.id);
        }
        // reset to its default, take it off
        for (k, (icon, what, enabled)) in [
            ("undo", "Back to its default", now != c.default),
            ("close", "No key", now.is_some()),
        ]
        .into_iter()
        .enumerate()
        {
            let b = Rect::from_center_size(
                pos2(r.right() - 48.0 + 26.0 * k as f32, r.center().y),
                vec2(22.0, 22.0),
            );
            let resp = ui
                .interact(
                    b,
                    ui.scope_id().with((icon, &c.id)),
                    if enabled {
                        egui::Sense::click()
                    } else {
                        egui::Sense::hover()
                    },
                )
                .on_hover_text(what);
            if enabled && resp.hovered() {
                p.rect_filled(b, 4.0, t.hover);
            }
            let ink = if enabled {
                t.ink2
            } else {
                t.ink2.gamma_multiply(0.3)
            };
            super::icons::draw(&p, b.min + vec2(3.0, 3.0), 16.0, icon, 1.5, ink);
            places.push((
                format!(
                    "settings:{}:{}",
                    if k == 0 { "key-reset" } else { "key-clear" },
                    c.id
                ),
                b,
            ));
            if enabled && resp.clicked() {
                s.keys
                    .set(&c.id, if k == 0 { c.default } else { None }, c.default);
            }
        }
    }
}

fn mouse(ui: &mut Ui, t: &Theme, s: &mut Settings, w: &mut Window, places: &mut Vec<(String, Rect)>) {
    let all = &w.presets;
    section(ui, t, "Preset");
    let items: Vec<(&str, bool)> = all
        .iter()
        .map(|p| {
            (
                p.name.as_str(),
                s.mouse_preset == p.id && s.mouse_custom.is_empty(),
            )
        })
        .collect();
    // each card: the CAD tool, then what its buttons do
    let maps: Vec<MouseMap> = all.iter().map(preset_map).collect();
    let width = ui.available_width();
    let cw = ((width - 3.0 * 10.0) / 4.0).floor();
    let picked = cards(
        ui,
        t,
        &items,
        vec2(cw, 104.0),
        &|k| format!("settings:preset:{}", all[k].id),
        places,
        &|p, r, k| {
            p.rect_filled(r, 3.0, t.surface);
            for (j, (a, _, word)) in NAV_ACTIONS.iter().skip(1).enumerate() {
                let y = r.top() + 12.0 + 18.0 * j as f32;
                p.text(
                    pos2(r.left() + 6.0, y),
                    egui::Align2::LEFT_CENTER,
                    *word,
                    sans(11.0),
                    t.ink2,
                );
                let g = maps[k]
                    .for_action(*a)
                    .first()
                    .map_or("—".to_string(), |g| g.to_string());
                p.text(
                    pos2(r.left() + 48.0, y),
                    egui::Align2::LEFT_CENTER,
                    g,
                    super::theme::mono(11.0),
                    t.ink,
                );
            }
        },
    );
    if let Some(k) = picked {
        s.mouse_preset = all[k].id.clone();
        s.mouse_custom.clear();
    }
    // its page; every row's source and caveat stays in data/mouse_presets.csv
    if let Some(src) = s.preset().sources().first() {
        ui.add_space(8.0);
        let host = src
            .split("//")
            .nth(1)
            .and_then(|x| x.split('/').next())
            .unwrap_or(src)
            .to_string();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Source  ").font(sans(12.0)).color(t.ink2));
            ui.visuals_mut().hyperlink_color = t.on_ink;
            let r = ui
                .hyperlink_to(egui::RichText::new(host).font(sans(12.0)), src)
                .on_hover_text(src);
            places.push(("settings:preset-source".into(), r.rect));
        });
    }
    let map = s.mouse_map();
    let choices = gesture_choices();
    section(ui, t, "Buttons");
    for (a, name, word) in NAV_ACTIONS {
        let mut gs = map.for_action(a);
        let before = gs.clone();
        row(ui, t, word, |ui| {
            let mut gone = None;
            for (k, g) in gs.iter_mut().enumerate() {
                let r = egui::ComboBox::from_id_salt(("mouse-gesture", name, k))
                    .selected_text(egui::RichText::new(g.to_string()).font(super::theme::mono(12.0)))
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        for c in &choices {
                            ui.selectable_value(g, c.clone(), c.to_string());
                        }
                    });
                places.push((format!("settings:mouse:{name}:{k}"), r.response.rect));
                let x = ui
                    .add_sized(
                        vec2(26.0, 26.0),
                        egui::Button::new(egui::RichText::new("×").font(sans(14.0))),
                    )
                    .on_hover_text("Take this gesture off");
                places.push((format!("settings:mouse-off:{name}:{k}"), x.rect));
                if x.clicked() {
                    gone = Some(k);
                }
            }
            if let Some(k) = gone {
                gs.remove(k);
            }
            let add = ui
                .add_sized(
                    vec2(26.0, 26.0),
                    egui::Button::new(egui::RichText::new("+").font(sans(14.0))),
                )
                .on_hover_text("Another gesture for this");
            places.push((format!("settings:mouse-add:{name}"), add.rect));
            if add.clicked() {
                gs.push(choices[0].clone());
            }
        });
        if gs != before {
            s.mouse_custom.retain(|c| c.0 != a);
            s.mouse_custom.push((a, gs));
        }
    }
    let taken: Vec<String> = map
        .binds
        .iter()
        .enumerate()
        .filter(|(k, (g, _))| map.binds.iter().take(*k).any(|(h, _)| h == g))
        .map(|(_, (g, _))| g.to_string())
        .collect();
    for g in taken {
        ui.label(
            egui::RichText::new(format!("{g} is set for two actions: the first listed wins"))
                .font(sans(12.0))
                .color(t.error),
        );
    }
    section(ui, t, "Wheel");
    row(ui, t, "Rolled away from you", |ui| {
        let words = if !map.wheel_zooms {
            "Doesn't zoom"
        } else if map.forward_zooms_in {
            "Zooms in"
        } else {
            "Zooms out"
        };
        ui.add_sized(
            vec2(110.0, 24.0),
            egui::Label::new(egui::RichText::new(words).font(sans(12.5)).color(t.ink)),
        );
        let r = ui.checkbox(
            &mut s.invert_zoom,
            egui::RichText::new("Reverse").font(sans(12.5)),
        );
        places.push(("settings:invert-zoom".into(), r.rect));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_go_to_toml_and_back() {
        let mut s = Settings {
            dark: false,
            pastel: true,
            backdrop: Backdrop::Gradient,
            style: VisualStyle::WireframeHidden,
            parallel: false,
            length: "in",
            angle: "rad",
            decimals: 5,
            tip_delay: 1.5,
            tip_long: 3.0,
            mouse_preset: "onshape".into(),
            invert_zoom: true,
            ..Settings::default()
        };
        s.overlays[0].flip("grid");
        s.overlays[1].flip("anchors");
        s.keys.set("measure", Chord::parse("Shift+M"), Chord::parse("M"));
        s.keys.set("lines", None, Chord::parse("L"));
        s.mouse_custom
            .push((NavAction::Zoom, vec![Gesture::parse("Ctrl+Right").unwrap()]));
        // a registered section's own table goes through untouched
        s.extra.set("agent", "model", "some-model");
        s.extra.set("agent", "base_url", "http://localhost:8080/v1");
        let text = s.to_toml();
        assert_eq!(Settings::from_toml(&text), s, "{text}");
        assert!(text.contains("[agent]"), "{text}");
        assert_eq!(Settings::from_toml(""), Settings::default());
    }

    #[test]
    fn the_old_edges_setting_becomes_a_visual_style() {
        let style = |t: &str| Settings::from_toml(t).style;
        assert_eq!(style("[display]\nedges = none\n"), VisualStyle::Shaded);
        assert_eq!(style("[display]\nedges = far\n"), VisualStyle::ShadedEdges);
        let both = "[display]\nedges = none\nvisual_style = wireframe-hidden\n";
        assert_eq!(style(both), VisualStyle::WireframeHidden);
    }

    #[test]
    fn a_preset_and_the_persons_own_gestures_make_the_table() {
        let mut s = Settings {
            mouse_preset: "solidworks".into(),
            ..Settings::default()
        };
        let m = s.mouse_map();
        assert_eq!(
            m.for_action(NavAction::Orbit),
            vec![Gesture::parse("Middle").unwrap()]
        );
        assert!(!m.forward_zooms_in);
        s.invert_zoom = true;
        assert!(s.mouse_map().forward_zooms_in);
        s.mouse_custom
            .push((NavAction::Orbit, vec![Gesture::parse("Right").unwrap()]));
        assert_eq!(
            s.mouse_map().for_action(NavAction::Orbit),
            vec![Gesture::parse("Right").unwrap()]
        );
    }
}
