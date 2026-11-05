//! Key and mouse bindings as data: the shortcut list (`data/keymap.csv`) and the CAD tools' mouse
//! presets (`data/mouse_presets.csv`, each row with its source), read into tables the shell and
//! the navigation look things up in. Also the small TOML the settings file is written in.

use winit::event::MouseButton;

pub const KEYMAP_CSV: &str = include_str!("../data/keymap.csv");
pub const MOUSE_CSV: &str = include_str!("../data/mouse_presets.csv");

/// The rows of a CSV with a header line; a field may be quoted to hold commas.
pub fn csv_rows(text: &str) -> Vec<Vec<String>> {
    text.lines()
        .skip(1)
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| {
            let (mut out, mut cur, mut quoted) = (Vec::new(), String::new(), false);
            for ch in l.chars() {
                match ch {
                    '"' => quoted = !quoted,
                    ',' if !quoted => out.push(std::mem::take(&mut cur)),
                    c => cur.push(c),
                }
            }
            out.push(cur);
            out.into_iter().map(|s| s.trim().to_string()).collect()
        })
        .collect()
}

/// A keyboard shortcut: the key by egui's name for it, and what's held with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Chord {
    pub key: egui::Key,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Chord {
    pub fn parse(s: &str) -> Option<Chord> {
        let (mut ctrl, mut shift, mut alt, mut key) = (false, false, false, None);
        for w in s.split('+').map(str::trim) {
            match w {
                "Ctrl" => ctrl = true,
                "Shift" => shift = true,
                "Alt" => alt = true,
                k => key = Some(egui::Key::from_name(k)?),
            }
        }
        Some(Chord {
            key: key?,
            ctrl,
            shift,
            alt,
        })
    }

    pub fn matches(&self, key: egui::Key, m: egui::Modifiers) -> bool {
        self.key == key && self.ctrl == (m.ctrl || m.command) && self.shift == m.shift && self.alt == m.alt
    }

    /// Modifier keys alone are no shortcut.
    pub fn from_event(key: egui::Key, m: egui::Modifiers) -> Chord {
        Chord {
            key,
            ctrl: m.ctrl || m.command,
            shift: m.shift,
            alt: m.alt,
        }
    }
}

impl std::fmt::Display for Chord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (on, w) in [(self.ctrl, "Ctrl+"), (self.shift, "Shift+"), (self.alt, "Alt+")] {
            if on {
                f.write_str(w)?;
            }
        }
        f.write_str(self.key.name())
    }
}

/// One shortcut of the list: its id (as the settings file names it), words, group, the key it
/// has by default, and whether it can be changed (keys a dialog or the sketcher reads itself).
#[derive(Clone, Debug)]
pub struct KeyCommand {
    pub id: String,
    pub name: String,
    pub group: String,
    pub default: Option<Chord>,
    /// Escape and Enter: always theirs.
    pub fixed: bool,
    /// Read only where it applies (in a sketch, in a drawing): it takes no key from the others.
    pub context: bool,
}

pub fn commands() -> Vec<KeyCommand> {
    csv_rows(KEYMAP_CSV)
        .into_iter()
        .filter(|r| r.len() >= 4)
        .map(|r| KeyCommand {
            id: r[0].clone(),
            name: r[1].clone(),
            group: r[2].clone(),
            default: Chord::parse(&r[3]),
            fixed: r.get(4).is_some_and(|f| f == "fixed" || f == "context"),
            context: r.get(4).is_some_and(|f| f == "context"),
        })
        .collect()
}

/// The keys in force: each command's default unless the person set another (or none).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KeyMap {
    pub custom: Vec<(String, Option<Chord>)>,
}

impl KeyMap {
    pub fn get(&self, c: &KeyCommand) -> Option<Chord> {
        match self.custom.iter().find(|(id, _)| *id == c.id) {
            Some((_, k)) if !c.fixed => *k,
            _ => c.default,
        }
    }

    pub fn set(&mut self, id: &str, k: Option<Chord>, default: Option<Chord>) {
        self.custom.retain(|(i, _)| i != id);
        if k != default {
            self.custom.push((id.to_string(), k));
        }
    }

    /// The command this key runs, of those that can be run from anywhere.
    pub fn command_for(&self, cmds: &[KeyCommand], key: egui::Key, m: egui::Modifiers) -> Option<String> {
        cmds.iter()
            .filter(|c| !c.fixed)
            .find(|c| self.get(c).is_some_and(|k| k.matches(key, m)))
            .map(|c| c.id.clone())
    }

    /// Each command whose key another also has: it, and the other's name.
    pub fn conflicts(&self, cmds: &[KeyCommand]) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for a in cmds {
            let Some(ka) = self.get(a) else { continue };
            for b in cmds {
                if a.id != b.id
                    && !a.context
                    && !b.context
                    && !(a.fixed && b.fixed)
                    && self.get(b) == Some(ka)
                {
                    out.push((a.id.clone(), b.name.clone()));
                }
            }
        }
        out
    }
}

/// What a drag in the view does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavAction {
    Select,
    Orbit,
    Pan,
    Zoom,
}

pub const NAV_ACTIONS: [(NavAction, &str, &str); 4] = [
    (NavAction::Select, "select", "Select"),
    (NavAction::Orbit, "orbit", "Orbit"),
    (NavAction::Pan, "pan", "Pan"),
    (NavAction::Zoom, "zoom", "Zoom"),
];

/// A mouse gesture: a button (and a second held with it), the modifiers, a key held down.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gesture {
    pub button: MouseButton,
    pub with: Option<MouseButton>,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub key: Option<String>,
}

fn button(w: &str) -> Option<MouseButton> {
    Some(match w {
        "Left" => MouseButton::Left,
        "Middle" => MouseButton::Middle,
        "Right" => MouseButton::Right,
        _ => return None,
    })
}

fn button_name(b: MouseButton) -> &'static str {
    match b {
        MouseButton::Left => "Left",
        MouseButton::Middle => "Middle",
        _ => "Right",
    }
}

impl Gesture {
    pub fn parse(s: &str) -> Option<Gesture> {
        let mut g = Gesture {
            button: MouseButton::Left,
            with: None,
            shift: false,
            ctrl: false,
            alt: false,
            key: None,
        };
        let mut buttons = Vec::new();
        for w in s.split('+').map(str::trim) {
            match w {
                "Shift" => g.shift = true,
                "Ctrl" => g.ctrl = true,
                "Alt" => g.alt = true,
                w if button(w).is_some() => buttons.push(button(w)?),
                // a key held down while the button drags: F-keys, as Inventor's F2 to F4
                w if w.len() <= 3 && w.starts_with('F') && w[1..].parse::<u8>().is_ok() => {
                    g.key = Some(w.into())
                }
                _ => return None,
            }
        }
        g.button = *buttons.first()?;
        g.with = buttons.get(1).copied();
        (buttons.len() <= 2).then_some(g)
    }
}

impl std::fmt::Display for Gesture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts: Vec<&str> = Vec::new();
        if let Some(k) = &self.key {
            parts.push(k);
        }
        for (on, w) in [(self.ctrl, "Ctrl"), (self.shift, "Shift"), (self.alt, "Alt")] {
            if on {
                parts.push(w);
            }
        }
        parts.push(button_name(self.button));
        if let Some(b) = self.with {
            parts.push(button_name(b));
        }
        f.write_str(&parts.join("+"))
    }
}

/// The gestures a binding can be set to in the settings, in the order offered.
pub fn gesture_choices() -> Vec<Gesture> {
    let mut out = Vec::new();
    for b in ["Left", "Middle", "Right"] {
        for m in ["", "Shift+", "Ctrl+", "Alt+"] {
            out.extend(Gesture::parse(&format!("{m}{b}")));
        }
    }
    for s in [
        "Middle+Left",
        "Middle+Right",
        "Left+Middle",
        "Left+Right",
        "F3+Left",
        "F4+Left",
    ] {
        out.extend(Gesture::parse(s));
    }
    out
}

/// What the mouse does in the view: each action's gestures, and which way the wheel zooms.
#[derive(Clone, Debug, PartialEq)]
pub struct MouseMap {
    pub binds: Vec<(Gesture, NavAction)>,
    /// Rolling the wheel away from you zooms in.
    pub forward_zooms_in: bool,
    /// The wheel zooms at all (CATIA V5's doesn't).
    pub wheel_zooms: bool,
}

impl Default for MouseMap {
    fn default() -> MouseMap {
        preset_map(&presets()[0])
    }
}

impl MouseMap {
    /// What this press does: the gesture exactly, or the button with no modifier's.
    pub fn action(
        &self,
        button: MouseButton,
        with: Option<MouseButton>,
        shift: bool,
        ctrl: bool,
        alt: bool,
        keys: &[String],
    ) -> Option<NavAction> {
        let exact = |g: &Gesture| {
            g.button == button
                && g.with == with
                && g.shift == shift
                && g.ctrl == ctrl
                && g.alt == alt
                && g.key.as_ref().is_none_or(|k| keys.contains(k))
        };
        // a held key's gesture before the plain one
        let mut found: Vec<&(Gesture, NavAction)> = self.binds.iter().filter(|(g, _)| exact(g)).collect();
        found.sort_by_key(|(g, _)| g.key.is_none());
        if let Some((_, a)) = found.first() {
            return Some(*a);
        }
        // shift and ctrl with a selecting button add to and take from the selection
        self.binds
            .iter()
            .filter(|(g, _)| {
                g.button == button && g.with == with && g.key.is_none() && !g.shift && !g.ctrl && !g.alt
            })
            .map(|(_, a)| *a)
            .next()
    }

    pub fn for_action(&self, a: NavAction) -> Vec<Gesture> {
        self.binds
            .iter()
            .filter(|b| b.1 == a)
            .map(|b| b.0.clone())
            .collect()
    }

    /// The keys a gesture holds down: the view watches only these.
    pub fn held_keys(&self) -> Vec<String> {
        self.binds.iter().filter_map(|(g, _)| g.key.clone()).collect()
    }
}

/// A CAD tool's mouse defaults, as its own documentation gives them.
#[derive(Clone, Debug)]
pub struct Preset {
    pub id: String,
    pub name: String,
    /// Action, gesture, source page.
    pub rows: Vec<(String, String, String)>,
    /// What a row's source does not say outright, by row.
    pub notes: Vec<String>,
}

impl Preset {
    /// Where the rows say they're from, once each.
    pub fn sources(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for (_, _, s) in &self.rows {
            if !out.contains(s) {
                out.push(s.clone());
            }
        }
        out
    }
}

/// `preset,name,action,gesture,source`; the first preset is the app's own default.
pub fn presets() -> Vec<Preset> {
    let mut out: Vec<Preset> = Vec::new();
    for r in csv_rows(MOUSE_CSV) {
        if r.len() < 5 {
            continue;
        }
        let row = (r[2].clone(), r[3].clone(), r[4].clone());
        let note = r.get(5).cloned().unwrap_or_default();
        match out.iter_mut().find(|p| p.id == r[0]) {
            Some(p) => {
                p.rows.push(row);
                p.notes.push(note);
            }
            None => out.push(Preset {
                id: r[0].clone(),
                name: r[1].clone(),
                rows: vec![row],
                notes: vec![note],
            }),
        }
    }
    out
}

pub fn preset_map(p: &Preset) -> MouseMap {
    let mut m = MouseMap {
        binds: Vec::new(),
        forward_zooms_in: true,
        wheel_zooms: true,
    };
    for (action, gesture, _) in &p.rows {
        if action == "wheel" {
            m.forward_zooms_in = gesture == "forward-in";
            m.wheel_zooms = gesture != "none";
            continue;
        }
        let Some((a, _, _)) = NAV_ACTIONS.iter().find(|n| n.1 == action) else {
            continue;
        };
        if let Some(g) = Gesture::parse(gesture) {
            m.binds.push((g, *a));
        }
    }
    m
}

/// A TOML table's values, as the settings file holds them: strings, numbers and booleans in
/// `[section]`s. Nothing else is needed, so nothing else is read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Toml {
    pub entries: Vec<(String, String, String)>,
}

impl Toml {
    pub fn parse(text: &str) -> Toml {
        let (mut section, mut entries) = (String::new(), Vec::new());
        for line in text.lines() {
            let l = line.trim();
            if l.is_empty() || l.starts_with('#') {
                continue;
            }
            if let Some(s) = l.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                section = s.trim().to_string();
                continue;
            }
            let Some((k, v)) = l.split_once('=') else { continue };
            let k = k.trim().trim_matches('"').to_string();
            let v = v.trim();
            let v = match v.strip_prefix('"') {
                Some(s) => {
                    let end = s.rfind('"').unwrap_or(s.len());
                    s[..end].replace("\\\"", "\"").replace("\\\\", "\\")
                }
                None => v.split('#').next().unwrap_or("").trim().to_string(),
            };
            entries.push((section.clone(), k, v));
        }
        Toml { entries }
    }

    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .rev()
            .find(|e| e.0 == section && e.1 == key)
            .map(|e| e.2.as_str())
    }

    pub fn section(&self, section: &str) -> Vec<(&str, &str)> {
        self.entries
            .iter()
            .filter(|e| e.0 == section)
            .map(|e| (e.1.as_str(), e.2.as_str()))
            .collect()
    }

    pub fn set(&mut self, section: &str, key: &str, value: impl Into<String>) {
        self.entries.push((section.into(), key.into(), value.into()));
    }

    /// Strings quoted, `true`/`false` and numbers bare, a section's keys under its header.
    pub fn write(&self) -> String {
        let mut out = String::new();
        let mut sections: Vec<&str> = Vec::new();
        for e in &self.entries {
            if !sections.contains(&e.0.as_str()) {
                sections.push(&e.0);
            }
        }
        for s in sections {
            if !s.is_empty() {
                out.push_str(&format!("\n[{s}]\n"));
            }
            let mut seen: Vec<&str> = Vec::new();
            for e in self.entries.iter().rev().filter(|e| e.0 == s) {
                if seen.contains(&e.1.as_str()) {
                    continue;
                }
                seen.push(&e.1);
            }
            seen.reverse();
            for k in seen {
                let v = self.get(s, k).unwrap_or("");
                let bare = v == "true" || v == "false" || (!v.is_empty() && v.parse::<f64>().is_ok());
                let key = if k
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                {
                    k.to_string()
                } else {
                    format!("\"{k}\"")
                };
                if bare {
                    out.push_str(&format!("{key} = {v}\n"));
                } else {
                    out.push_str(&format!(
                        "{key} = \"{}\"\n",
                        v.replace('\\', "\\\\").replace('"', "\\\"")
                    ));
                }
            }
        }
        out.trim_start().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_data_row_reads() {
        let cmds = commands();
        assert!(cmds.len() >= 15);
        for c in &cmds {
            assert!(c.default.is_some(), "{} has a key that doesn't read", c.id);
        }
        for p in presets() {
            assert!(!p.sources().is_empty());
            for (action, gesture, source) in &p.rows {
                assert!(source.starts_with("https://"), "{}: {action} cites no page", p.id);
                if action == "wheel" {
                    assert!(["forward-in", "forward-out", "none"].contains(&gesture.as_str()));
                } else {
                    assert!(Gesture::parse(gesture).is_some(), "{}: {gesture}", p.id);
                }
            }
        }
    }

    #[test]
    fn the_default_keys_have_no_conflicts_and_m_measures() {
        let cmds = commands();
        let k = KeyMap::default();
        assert_eq!(k.conflicts(&cmds), vec![]);
        let none = egui::Modifiers::NONE;
        assert_eq!(
            k.command_for(&cmds, egui::Key::M, none).as_deref(),
            Some("measure")
        );
        assert_eq!(
            k.command_for(&cmds, egui::Key::Z, egui::Modifiers::CTRL)
                .as_deref(),
            Some("undo")
        );
        // a key moved: the old one does nothing, the new one runs it, and taking another's shows
        let mut k = k;
        let measure = cmds.iter().find(|c| c.id == "measure").unwrap();
        k.set("measure", Chord::parse("Shift+M"), measure.default);
        assert_eq!(k.command_for(&cmds, egui::Key::M, none), None);
        assert_eq!(
            k.command_for(&cmds, egui::Key::M, egui::Modifiers::SHIFT)
                .as_deref(),
            Some("measure")
        );
        k.set("measure", Chord::parse("A"), measure.default);
        assert!(k.conflicts(&cmds).iter().any(|(id, _)| id == "measure"));
        k.set("measure", measure.default, measure.default);
        assert!(k.custom.is_empty(), "back to the default keeps nothing");
    }

    #[test]
    fn inventor_is_the_default_and_its_gestures_are_the_apps() {
        let m = MouseMap::default();
        let none: &[String] = &[];
        assert_eq!(
            m.action(MouseButton::Left, None, false, false, false, none),
            Some(NavAction::Select)
        );
        assert_eq!(
            m.action(MouseButton::Left, None, true, false, false, none),
            Some(NavAction::Select)
        );
        assert_eq!(
            m.action(MouseButton::Middle, None, true, false, false, none),
            Some(NavAction::Orbit)
        );
        assert_eq!(
            m.action(MouseButton::Middle, None, false, false, false, none),
            Some(NavAction::Pan)
        );
        assert_eq!(
            m.action(MouseButton::Left, None, false, false, false, &["F4".into()]),
            Some(NavAction::Orbit)
        );
        assert!(m.forward_zooms_in);
    }

    #[test]
    fn toml_goes_there_and_back() {
        let mut t = Toml::default();
        t.set("appearance", "theme", "dark");
        t.set("units", "decimals", "3");
        t.set("keys", "measure", "Shift+M");
        t.set("mouse", "invert_zoom", "true");
        t.set("keys", "measure", "M");
        let text = t.write();
        assert!(text.contains("decimals = 3"), "{text}");
        assert!(text.contains("measure = \"M\""), "{text}");
        let back = Toml::parse(&text);
        assert_eq!(back.get("keys", "measure"), Some("M"));
        assert_eq!(back.get("mouse", "invert_zoom"), Some("true"));
        assert_eq!(back.get("appearance", "theme"), Some("dark"));
    }
}
