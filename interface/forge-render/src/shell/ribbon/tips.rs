//! Every ribbon tool's tooltip: its name, a line on what it does and its shortcut
//! (`data/tool_tips.csv`, keys as the settings have them); after a longer rest, its icon large
//! and, for a sketch tool, the clicks it takes. A tool that's off says why.

use super::super::sketching::SkTool;
use super::super::theme::{mono, sans, semi};
use super::super::{Action, FLAT_TAB, SHEET_TAB, Shell, THERMO_TAB, icons};
use super::{Tool, why_off};
use egui::{Response, RichText, Ui, vec2};
use std::sync::OnceLock;

const TIPS_CSV: &str = include_str!("../../../data/tool_tips.csv");
/// How long the pointer rests before the extended tooltip, in seconds.
const LONG_REST: f64 = 1.4;

/// The rows of a CSV with a header line; a field may be quoted to hold commas.
pub(in crate::shell) fn csv_rows(text: &str) -> Vec<Vec<String>> {
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

/// (scope, name) to (what it does, its key's id).
fn table() -> &'static Vec<(String, String, String, String)> {
    static T: OnceLock<Vec<(String, String, String, String)>> = OnceLock::new();
    T.get_or_init(|| {
        csv_rows(TIPS_CSV)
            .into_iter()
            .filter(|r| r.len() >= 3)
            .map(|r| {
                (
                    r[0].clone(),
                    r[1].clone(),
                    r[2].clone(),
                    r.get(3).cloned().unwrap_or_default(),
                )
            })
            .collect()
    })
}

/// Which tab's tools these are, as the tips file names them.
pub(super) fn scope(s: &Shell) -> &'static str {
    if s.drawing.is_some() {
        "drawing"
    } else if s.is_asm() && s.tab == 0 {
        "asm"
    } else {
        match s.tab {
            0 => "model",
            1 => "sketch",
            2 => "inspect",
            3 => "view",
            SHEET_TAB => "sheet",
            FLAT_TAB => "flat",
            THERMO_TAB => "thermo",
            _ => "tools",
        }
    }
}

/// A tool's line and its key's id: this tab's, else any tab's of that name.
pub(super) fn tip(scope: &str, name: &str) -> Option<(&'static str, &'static str)> {
    let t = table();
    t.iter()
        .find(|r| r.0 == scope && r.1 == name)
        .or_else(|| t.iter().find(|r| r.1 == name))
        .map(|r| (r.2.as_str(), r.3.as_str()))
}

/// The key a shortcut id is on now: the person's, else the table's default (`data/keymap.csv`).
pub(in crate::shell) fn key_of(s: &Shell, id: &str) -> Option<String> {
    let c = s.prefs.cmds.iter().find(|c| c.id == id)?;
    s.settings.keys.get(c).map(|k| k.to_string())
}

/// Why a tool is off, in words.
fn why(s: &Shell, scope: &str, name: &str) -> String {
    if let Some(w) = why_off(name) {
        return w.to_string();
    }
    match scope {
        "sketch" if s.sketch.is_none() => {
            "start a sketch first: 3D Model > Start Sketch, or double-click a sketch in the browser".into()
        }
        "thermo" if !matches!(name, "Materials" | "Cutout") && s.fg.is_some() => {
            "convert the solid to a thermoformed part first (Thermoform > Convert)".into()
        }
        _ if s.fg.is_none() => "only in a Forge part (.fg): open one to use it".into(),
        _ => "not built yet".into(),
    }
}

/// The clicks a sketch tool takes, in order.
fn steps(t: SkTool) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = vec![];
    let tries: Vec<(usize, usize, bool)> = if t.modifies() {
        vec![
            (0, 0, false),
            (0, 1, false),
            (0, 1, true),
            (1, 1, true),
            (2, 1, true),
        ]
    } else {
        (0..4).map(|k| (k, 0, false)).collect()
    };
    for (k, c, p) in tries {
        if let Some(w) = t.prompt(k, c, p)
            && !out.contains(&w)
        {
            out.push(w);
        }
    }
    out
}

impl Shell {
    /// Puts a tool's tooltip on its button.
    pub(super) fn tool_tip(&self, resp: Response, tool: &Tool) -> Response {
        let ctx = resp.ctx.clone();
        let now = ctx.input(|i| i.time);
        let key = resp.id.with("tip-since");
        let since = if resp.hovered() {
            ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(key, || now))
        } else {
            ctx.data_mut(|d| d.remove::<f64>(key));
            now
        };
        let long = now - since >= LONG_REST;
        if resp.hovered() && !long {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(LONG_REST - (now - since)));
        }
        let scope = scope(self);
        resp.on_hover_ui(|ui| self.tip_body(ui, scope, tool, long))
    }

    fn tip_body(&self, ui: &mut Ui, scope: &str, tool: &Tool, long: bool) {
        let t = self.theme;
        ui.set_max_width(300.0);
        let (line, key) = tip(scope, tool.name).unwrap_or(("", ""));
        let key = key_of(self, key);
        ui.horizontal(|ui| {
            ui.label(RichText::new(tool.name).font(semi(13.0)).color(t.ink));
            if let Some(k) = &key {
                ui.add_space(8.0);
                ui.label(RichText::new(k).font(mono(11.0)).color(t.ink2));
            }
        });
        if !line.is_empty() {
            ui.label(RichText::new(line).font(sans(12.0)).color(t.ink));
        }
        if tool.action.is_none() {
            ui.label(
                RichText::new(format!("Off: {}", why(self, scope, tool.name)))
                    .font(sans(12.0))
                    .color(t.error),
            );
        }
        if !long {
            return;
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(64.0, 64.0), egui::Sense::hover());
            ui.painter().rect_filled(r, 4.0, t.raised);
            icons::draw(ui.painter(), r.min + vec2(8.0, 8.0), 48.0, tool.icon, 2.0, t.ink);
            if let Some(Action::SketchTool(st)) = tool.action {
                ui.vertical(|ui| {
                    for (k, w) in steps(st).into_iter().enumerate() {
                        ui.label(
                            RichText::new(format!("{}. {w}", k + 1))
                                .font(sans(11.0))
                                .color(t.ink2),
                        );
                    }
                });
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tip_row_is_whole_and_every_key_it_names_is_known() {
        let s = Shell::default();
        for r in table() {
            assert!(!r.2.is_empty(), "{r:?} says nothing");
            assert!(
                r.3.is_empty() || key_of(&s, &r.3).is_some(),
                "{r:?}: no key {}",
                r.3
            );
        }
        assert_eq!(key_of(&s, "sketch_line").as_deref(), Some("L"));
        assert_eq!(tip("sketch", "Two Point Center Rectangle").map(|t| t.1), Some(""));
        // a name on two tabs says what it does on each
        assert_ne!(tip("sketch", "Mirror"), tip("model", "Mirror"));
    }

    #[test]
    fn every_ribbon_tool_on_every_tab_has_its_own_tip() {
        let mut s = Shell::default();
        let mut missing = vec![];
        for tab in [0, 1, 2, 3, 4, SHEET_TAB, FLAT_TAB, THERMO_TAB] {
            s.tab = tab;
            let scope = scope(&s);
            let names: Vec<&str> = if tab == 1 {
                super::super::sketch_tab::groups(&s)
                    .iter()
                    .flat_map(|g| g.big.iter().chain(&g.small))
                    .flat_map(|sp| sp.variants.iter().chain(&sp.toggles))
                    .map(|t| t.name)
                    .collect()
            } else {
                super::super::groups(&s)
                    .iter()
                    .flat_map(|g| g.big.iter().chain(&g.small).chain(&g.icons))
                    .map(|t| t.name)
                    .collect()
            };
            assert!(!names.is_empty(), "tab {tab}");
            for n in names {
                if !table().iter().any(|r| r.0 == scope && r.1 == n) {
                    missing.push(format!("{scope},{n}"));
                }
            }
        }
        for tab in 0..4 {
            s.tab = tab;
            for (_, big, small) in s.drawing_groups() {
                for (c, _) in big.into_iter().chain(small) {
                    if !table().iter().any(|r| r.0 == "drawing" && r.1 == c.name()) {
                        missing.push(format!("drawing,{}", c.name()));
                    }
                }
            }
        }
        assert!(missing.is_empty(), "no tip for:\n{}", missing.join("\n"));
    }
}
