//! Settings: Inventor's Application Options as one floating window from Manage > Settings. Its
//! tabs hold what was about the app (the theme, part colours, the view's background and
//! defaults, units, every key, the mouse with the CAD tools' presets); what's set takes effect at
//! once and is kept in `settings.toml` under `$XDG_CONFIG_HOME/forgecad`.

use super::fgasm::{dialog_frame, dialog_head};
use super::theme::{Theme, medium, sans};
use super::{ASM_OVERLAYS, Action, OVERLAY_SWITCHES, Overlays, PART_OVERLAYS, Shell, theme};
use crate::bindings::{
    Chord, Gesture, KeyCommand, KeyMap, MouseMap, NAV_ACTIONS, NavAction, Preset, Toml, gesture_choices,
    preset_map, presets,
};
use crate::render::{Backdrop, LineMode};
use egui::{Rect, Stroke, StrokeKind, Ui, pos2, vec2};

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
];

/// The tables the built-in sections own; any other in settings.toml is a registered section's.
const OWN_TABLES: [&str; 6] = ["appearance", "display", "units", "keys", "mouse", "mouse.custom"];
const BACKDROPS: [(Backdrop, &str, &str); 3] = [
    (Backdrop::Floor, "floor", "Inventor floor"),
    (Backdrop::Gradient, "gradient", "Gradient"),
    (Backdrop::Solid, "solid", "Solid colour"),
];
const LINES: [(LineMode, &str, &str); 3] = [
    (LineMode::Near, "near", "Near parts"),
    (LineMode::Far, "far", "All parts"),
    (LineMode::None, "none", "None"),
];

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub dark: bool,
    pub pastel: bool,
    pub backdrop: Backdrop,
    pub lines: LineMode,
    pub parallel: bool,
    /// What a part's and an assembly's view show when opened.
    pub overlays: [Overlays; 2],
    pub length: &'static str,
    pub angle: &'static str,
    pub decimals: usize,
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
            lines: LineMode::Near,
            parallel: true,
            overlays: [PART_OVERLAYS, ASM_OVERLAYS],
            length: "mm",
            angle: "deg",
            decimals: 3,
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
        t.set("display", "edges", word(&LINES, self.lines));
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
        s.lines = by_word(&LINES, &get("display", "edges")).unwrap_or(s.lines);
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
        }
    }
}

/// A choice among a few, as a pill; true when clicked.
fn pill(
    ui: &mut Ui,
    t: &Theme,
    on: bool,
    text: &str,
    name: String,
    places: &mut Vec<(String, Rect)>,
) -> bool {
    let g = ui.fonts_mut(|f| f.layout_no_wrap(text.to_string(), sans(12.0), t.ink));
    let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 20.0, 24.0), egui::Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::RadioButton, true, on, text));
    let p = ui.painter();
    let fill = if on {
        t.on
    } else if resp.hovered() {
        t.hover
    } else {
        t.field
    };
    p.rect_filled(r, 4.0, fill);
    p.rect_stroke(
        r,
        4.0,
        Stroke::new(1.0, if on { t.on_ink.gamma_multiply(0.6) } else { t.line }),
        StrokeKind::Inside,
    );
    p.text(
        r.center(),
        egui::Align2::CENTER_CENTER,
        text,
        sans(12.0),
        if on { t.on_ink } else { t.ink },
    );
    places.push((name, r));
    resp.clicked()
}

/// A titled box of settings, as Inventor's option groups.
fn group(ui: &mut Ui, t: &Theme, title: &str, body: impl FnOnce(&mut Ui)) {
    ui.label(egui::RichText::new(title).font(medium(12.0)).color(t.ink));
    egui::Frame::new()
        .stroke(Stroke::new(1.0, t.line))
        .corner_radius(4)
        .inner_margin(10)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            body(ui);
        });
    ui.add_space(8.0);
}

/// Words on the left of a fixed width, as a form's.
fn left_label(ui: &mut Ui, w: f32, text: egui::RichText) {
    ui.allocate_ui_with_layout(
        vec2(w, 22.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_width(w);
            ui.add(egui::Label::new(text).truncate());
        },
    );
}

fn row(ui: &mut Ui, t: &Theme, label: &str, body: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        left_label(
            ui,
            130.0,
            egui::RichText::new(label).font(sans(12.0)).color(t.ink2),
        );
        ui.spacing_mut().item_spacing.x = 4.0;
        body(ui);
    });
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
        let w = 640.0;
        let at = pos2(area.center().x - w * 0.5, area.top() + 60.0);
        let r = egui::Window::new("Settings")
            .id(egui::Id::unique("settings-window"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .fade_in(false)
            .default_pos(at)
            .min_width(w)
            .max_width(w)
            .constrain_to(area)
            // over the view's own drawings (Measure's marks), which are a layer of their own
            .order(egui::Order::Foreground)
            .frame(dialog_frame(&t))
            .show(ctx, |ui| {
                // a checkbox's square in the fields' colour with an edge that shows, in either theme
                let w = &mut ui.visuals_mut().widgets;
                w.inactive.bg_fill = t.field;
                w.inactive.bg_stroke = Stroke::new(1.0, t.ink2.gamma_multiply(0.8));
                w.hovered.bg_stroke = Stroke::new(1.0, t.ink2);
                close = dialog_head(ui, &t, "options", "Settings", "settings:close", &mut places);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for (k, (name, _)) in SECTIONS.iter().enumerate() {
                        let on = k == prefs.tab;
                        let g = ui.fonts_mut(|f| f.layout_no_wrap(name.to_string(), sans(13.0), t.ink));
                        let (r, resp) =
                            ui.allocate_exact_size(vec2(g.size().x + 26.0, 28.0), egui::Sense::click());
                        resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, on, *name));
                        let p = ui.painter();
                        if on {
                            p.rect_filled(r, 0.0, t.raised);
                            p.hline(r.x_range(), r.bottom() - 1.0, Stroke::new(2.0, t.focus));
                        } else if resp.hovered() {
                            p.rect_filled(r, 0.0, t.hover);
                        }
                        p.text(
                            r.center(),
                            egui::Align2::CENTER_CENTER,
                            *name,
                            if on { medium(13.0) } else { sans(13.0) },
                            if on { t.ink } else { t.ink2 },
                        );
                        places.push((format!("settings:tab:{name}"), r));
                        if resp.clicked() {
                            prefs.tab = k;
                            prefs.capture = None;
                        }
                    }
                });
                ui.add(egui::Separator::default().spacing(0.0));
                ui.add_space(10.0);
                egui::ScrollArea::vertical()
                    .max_height(430.0)
                    .min_scrolled_height(430.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let (_, draw) = SECTIONS[prefs.tab.min(SECTIONS.len() - 1)];
                        draw(ui, &t, &mut s, prefs, &mut places);
                    });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let r = ui.button(egui::RichText::new("Reset All").font(sans(12.0)));
                    places.push(("settings:reset-all".into(), r.rect));
                    if r.clicked() {
                        s = Settings::default();
                        prefs.capture = None;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let r = ui.button(egui::RichText::new("Close").font(sans(12.0)));
                        places.push(("settings:done".into(), r.rect));
                        if r.clicked() {
                            close = true;
                        }
                    });
                });
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
        self.view.lines = s.lines;
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
        if s.lines != was.lines {
            self.out.push(Action::Lines(s.lines));
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
        for (key, m) in keys {
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

fn appearance(ui: &mut Ui, t: &Theme, s: &mut Settings, _: &mut Window, places: &mut Vec<(String, Rect)>) {
    group(ui, t, "UI Theme", |ui| {
        row(ui, t, "Theme", |ui| {
            for (dark, word) in [(false, "Light"), (true, "Dark")] {
                if pill(
                    ui,
                    t,
                    s.dark == dark,
                    word,
                    format!("settings:theme:{word}"),
                    places,
                ) {
                    s.dark = dark;
                }
            }
        });
    });
    group(ui, t, "Colours", |ui| {
        row(ui, t, "Parts", |ui| {
            for (pastel, word) in [(false, "Inventor grey"), (true, "A pastel per part")] {
                if pill(
                    ui,
                    t,
                    s.pastel == pastel,
                    word,
                    format!("settings:parts:{pastel}"),
                    places,
                ) {
                    s.pastel = pastel;
                }
            }
        });
    });
    group(ui, t, "Background", |ui| {
        row(ui, t, "Viewport", |ui| {
            for (b, name, word) in BACKDROPS {
                if pill(
                    ui,
                    t,
                    s.backdrop == b,
                    word,
                    format!("settings:background:{name}"),
                    places,
                ) {
                    s.backdrop = b;
                }
            }
        });
    });
}

fn display(ui: &mut Ui, t: &Theme, s: &mut Settings, _: &mut Window, places: &mut Vec<(String, Rect)>) {
    group(ui, t, "View", |ui| {
        row(ui, t, "Edges", |ui| {
            for (m, name, word) in LINES {
                if pill(
                    ui,
                    t,
                    s.lines == m,
                    word,
                    format!("settings:edges:{name}"),
                    places,
                ) {
                    s.lines = m;
                }
            }
        });
        row(ui, t, "Projection", |ui| {
            for (p, word) in [(true, "Orthographic"), (false, "Perspective")] {
                if pill(
                    ui,
                    t,
                    s.parallel == p,
                    word,
                    format!("settings:projection:{word}"),
                    places,
                ) {
                    s.parallel = p;
                }
            }
        });
    });
    for (k, kind) in ["Parts open with", "Assemblies open with"]
        .into_iter()
        .enumerate()
    {
        group(ui, t, kind, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 18.0;
                for (name, word) in OVERLAY_SWITCHES {
                    let mut on = s.overlays[k].get(name);
                    let r = ui.checkbox(&mut on, egui::RichText::new(word).font(sans(12.0)));
                    places.push((format!("settings:show:{k}:{name}"), r.rect));
                    if on != s.overlays[k].get(name) {
                        s.overlays[k].flip(name);
                    }
                }
            });
        });
    }
}

fn units(ui: &mut Ui, t: &Theme, s: &mut Settings, _: &mut Window, places: &mut Vec<(String, Rect)>) {
    group(ui, t, "Units", |ui| {
        row(ui, t, "Length", |ui| {
            for u in forge_doc::units::DOC_LENGTHS {
                if pill(ui, t, s.length == u, u, format!("settings:length:{u}"), places) {
                    s.length = u;
                }
            }
        });
        row(ui, t, "Angle", |ui| {
            for (u, word) in ANGLES.into_iter().zip(["degrees", "radians"]) {
                if pill(ui, t, s.angle == u, word, format!("settings:angle:{u}"), places) {
                    s.angle = u;
                }
            }
        });
        row(ui, t, "Decimal places", |ui| {
            for n in 0..=6usize {
                if pill(
                    ui,
                    t,
                    s.decimals == n,
                    &n.to_string(),
                    format!("settings:decimals:{n}"),
                    places,
                ) {
                    s.decimals = n;
                }
            }
        });
    });
}

fn keyboard(ui: &mut Ui, t: &Theme, s: &mut Settings, w: &mut Window, places: &mut Vec<(String, Rect)>) {
    let (cmds, capture) = (&w.cmds, &mut w.capture);
    let conflicts = s.keys.conflicts(cmds);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Click a key to set it, then press the new one (Esc keeps it)")
                .font(sans(11.0))
                .color(t.ink2),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let r = ui.button(egui::RichText::new("Reset Keys").font(sans(12.0)));
            places.push(("settings:keys:reset".into(), r.rect));
            if r.clicked() {
                s.keys = KeyMap::default();
                *capture = None;
            }
        });
    });
    ui.add_space(4.0);
    let mut last = "";
    for c in cmds {
        if c.group != last {
            last = &c.group;
            ui.add_space(4.0);
            ui.label(egui::RichText::new(&c.group).font(medium(12.0)).color(t.ink));
        }
        let now = s.keys.get(c);
        ui.horizontal(|ui| {
            left_label(
                ui,
                250.0,
                egui::RichText::new(&c.name).font(sans(12.0)).color(t.ink2),
            );
            let waiting = capture.as_deref() == Some(c.id.as_str());
            let text = if waiting {
                "Press a key...".to_string()
            } else {
                now.map_or("(none)".into(), |k| k.to_string())
            };
            if c.fixed {
                let r = ui.add_sized(
                    vec2(130.0, 22.0),
                    egui::Label::new(
                        egui::RichText::new(text)
                            .font(super::theme::mono(12.0))
                            .color(t.ink2),
                    ),
                );
                places.push((format!("settings:key:{}", c.id), r.rect));
                return;
            }
            let r = ui.add_sized(
                vec2(130.0, 22.0),
                egui::Button::new(egui::RichText::new(text).font(super::theme::mono(12.0))).selected(waiting),
            );
            places.push((format!("settings:key:{}", c.id), r.rect));
            if r.clicked() {
                *capture = if waiting { None } else { Some(c.id.clone()) };
            }
            let changed = now != c.default;
            let r = ui.add_enabled(
                changed,
                egui::Button::new(egui::RichText::new("Reset").font(sans(11.0))).small(),
            );
            places.push((format!("settings:key-reset:{}", c.id), r.rect));
            if r.clicked() {
                s.keys.set(&c.id, c.default, c.default);
            }
            let r = ui.add_enabled(
                now.is_some(),
                egui::Button::new(egui::RichText::new("Clear").font(sans(11.0))).small(),
            );
            places.push((format!("settings:key-clear:{}", c.id), r.rect));
            if r.clicked() {
                s.keys.set(&c.id, None, c.default);
            }
            if let Some((_, other)) = conflicts.iter().find(|(id, _)| *id == c.id) {
                ui.label(
                    egui::RichText::new(format!("also {other}"))
                        .font(sans(11.0))
                        .color(t.error),
                );
            }
        });
    }
}

fn mouse(ui: &mut Ui, t: &Theme, s: &mut Settings, w: &mut Window, places: &mut Vec<(String, Rect)>) {
    let all = &w.presets;
    group(ui, t, "Preset", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            for p in all {
                let on = s.mouse_preset == p.id && s.mouse_custom.is_empty();
                if pill(ui, t, on, &p.name, format!("settings:preset:{}", p.id), places) {
                    s.mouse_preset = p.id.clone();
                    s.mouse_custom.clear();
                }
            }
        });
        // its page; every row's source and caveat stays in data/mouse_presets.csv
        if let Some(src) = s.preset().sources().first() {
            ui.add_space(4.0);
            let r = ui.hyperlink_to(egui::RichText::new("Source").font(sans(11.0)), src);
            places.push(("settings:preset-source".into(), r.rect));
        }
    });
    let map = s.mouse_map();
    let choices = gesture_choices();
    group(ui, t, "Buttons", |ui| {
        for (a, name, word) in NAV_ACTIONS {
            let mut gs = map.for_action(a);
            let before = gs.clone();
            ui.horizontal(|ui| {
                left_label(ui, 70.0, egui::RichText::new(word).font(sans(12.0)).color(t.ink2));
                let mut gone = None;
                for (k, g) in gs.iter_mut().enumerate() {
                    let r = egui::ComboBox::from_id_salt(("mouse-gesture", name, k))
                        .selected_text(egui::RichText::new(g.to_string()).font(super::theme::mono(12.0)))
                        .width(128.0)
                        .show_ui(ui, |ui| {
                            for c in &choices {
                                ui.selectable_value(g, c.clone(), c.to_string());
                            }
                        });
                    places.push((format!("settings:mouse:{name}:{k}"), r.response.rect));
                    let x = ui.small_button("×").on_hover_text("Take this gesture off");
                    places.push((format!("settings:mouse-off:{name}:{k}"), x.rect));
                    if x.clicked() {
                        gone = Some(k);
                    }
                }
                if let Some(k) = gone {
                    gs.remove(k);
                }
                let add = ui.small_button("+").on_hover_text("Another gesture for this");
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
                    .font(sans(11.0))
                    .color(t.error),
            );
        }
    });
    group(ui, t, "Wheel", |ui| {
        let words = if !map.wheel_zooms {
            "The wheel does not zoom in this preset"
        } else if map.forward_zooms_in {
            "Rolling the wheel away from you zooms in"
        } else {
            "Rolling the wheel away from you zooms out"
        };
        ui.label(egui::RichText::new(words).font(sans(12.0)).color(t.ink));
        let r = ui.checkbox(
            &mut s.invert_zoom,
            egui::RichText::new("Reverse the zoom direction").font(sans(12.0)),
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
            lines: LineMode::Far,
            parallel: false,
            length: "in",
            angle: "rad",
            decimals: 5,
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
