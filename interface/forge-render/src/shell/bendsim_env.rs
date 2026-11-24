//! The Bend Simulation environment's content (owned by BEND-SIM): its ribbon, its browser, what
//! it draws in the view and its commands. The framework's wrapper (`env/bendsim.rs`) enters and
//! leaves it and calls these; nothing else in the shell does. Replace freely behind these
//! signatures.

use super::bendsim::BendSimUi;
use super::env::EnvId;
use super::ribbon::{Group, Tool};
use super::{Action, Shell};
use egui::{Rect, Ui};

/// Its commands: a tooling set by its index in `forge_brake::library::IDS`, then these.
const FIRST: u16 = 100;
const BACK: u16 = 101;
const PLAY: u16 = 102;
const NEXT: u16 = 103;
const AGAIN: u16 = 104;

fn tool(name: &'static str, icon: &'static str, action: Option<Action>, on: bool) -> Tool {
    Tool {
        name,
        icon,
        action,
        on,
    }
}

fn group(name: &'static str, big: Vec<Tool>, small: Vec<Tool>) -> Group {
    Group {
        name,
        big,
        small,
        icons: vec![],
    }
}

/// Into it: simulated afresh if the part's text changed since; its step kept where it still
/// is one.
pub(super) fn enter(s: &mut Shell) -> Result<(), String> {
    let fg = s.fg.as_ref().ok_or("no part")?;
    let fresh = s
        .sheet
        .bend_sim
        .as_ref()
        .is_some_and(|b| b.source == fg.source && b.path == fg.path);
    if !fresh {
        let was = s.sheet.bend_sim.take();
        let tooling = was.as_ref().map_or("std-eu", |b| b.tooling.as_str());
        // first time in: a real press from the library (the first whose maker's own tools the
        // library holds), not a stand-in
        let choice = was.as_ref().map(|b| b.choice.clone()).unwrap_or_else(|| {
            let dir = std::path::Path::new(&fg.path)
                .parent()
                .map(std::path::Path::to_path_buf);
            let press = forge_lang::bendsim::catalog(dir.as_deref()).ok().and_then(|c| {
                c.presses
                    .iter()
                    .find(|p| {
                        p.press.is_ok()
                            && !c.default_tools(p, true).is_empty()
                            && !c.default_tools(p, false).is_empty()
                    })
                    .map(|p| p.id.clone())
            });
            super::bendsim::Choice {
                press,
                ..Default::default()
            }
        });
        let mut b = BendSimUi::open_with(&fg.source, &fg.path, tooling, choice);
        if let Some(w) = was {
            b.resume(&w);
        }
        s.sheet.bend_sim = Some(Box::new(b));
    }
    Ok(())
}

/// Out of it: play stops, the rest kept.
pub(super) fn exit(s: &mut Shell) {
    if let Some(b) = &mut s.sheet.bend_sim {
        b.playing = false;
    }
}

pub(super) fn groups(s: &Shell) -> Vec<Group> {
    let b = s.sheet.bend_sim.as_deref();
    let ran = b.is_some_and(|b| b.sim.is_ok());
    let cmd = |c: u16| ran.then_some(Action::Env(EnvId::BendSim, c));
    let playing = b.is_some_and(|b| b.playing);
    let tooling = forge_brake::library::IDS
        .iter()
        .enumerate()
        .map(|(k, id)| {
            tool(
                tooling_word(id),
                "bendtable",
                Some(Action::Env(EnvId::BendSim, k as u16)),
                b.is_some_and(|b| b.tooling == *id && b.choice.press.is_none()),
            )
        })
        .collect();
    vec![
        group("Stand-in Tooling", vec![], tooling),
        group(
            "Play",
            vec![tool(
                if playing { "Pause" } else { "Play" },
                "forward",
                cmd(PLAY),
                playing,
            )],
            vec![
                tool("First Bend", "home", cmd(FIRST), false),
                tool("Previous Bend", "backward", cmd(BACK), false),
                tool("Next Bend", "forward", cmd(NEXT), false),
            ],
        ),
        group(
            "Simulate",
            vec![tool(
                "Simulate Again",
                "redo",
                Some(Action::Env(EnvId::BendSim, AGAIN)),
                false,
            )],
            vec![],
        ),
    ]
}

/// Setup, sequence and the step's results, in the left panel.
pub(super) fn browser(s: &mut Shell, ui: &mut Ui, r: Rect) -> bool {
    let Some(mut b) = s.sheet.bend_sim.take() else {
        return false;
    };
    let t = s.theme;
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r.shrink2(egui::vec2(10.0, 8.0))));
    child.set_clip_rect(r);
    egui::ScrollArea::vertical()
        .id_salt("env-bendsim-browser")
        .show(&mut child, |ui| {
            ui.label(
                egui::RichText::new("Bend Simulation")
                    .font(super::theme::semi(13.0))
                    .color(t.ink),
            );
            super::bendsim::setup_ui(ui, &mut b);
            ui.separator();
            super::bendsim::sequence_ui(ui, &mut b);
            ui.separator();
            super::bendsim::results_ui(ui, &b);
        });
    s.places.push(("env:bendsim:browser".into(), r));
    s.sheet.bend_sim = Some(b);
    true
}

/// Play's clock.
pub(super) fn panels(s: &mut Shell, ctx: &egui::Context) {
    if let Some(b) = &mut s.sheet.bend_sim {
        b.tick(f64::from(ctx.input(|i| i.stable_dt)).min(0.1));
        if b.playing {
            ctx.request_repaint();
        }
    }
}

/// The view is the renderer's (the press in 3D, gui_bendsim.rs): over it, the machine, step and
/// failures, and the timeline along its foot. The view keeps its pointer (orbit, zoom).
pub(super) fn draw(s: &mut Shell, ui: &mut Ui, view: Rect) -> bool {
    let Some(b) = s.sheet.bend_sim.as_deref_mut() else {
        return false;
    };
    super::bendsim::overlay(ui, view, b);
    let band = Rect::from_min_max(
        egui::pos2(view.left() + 12.0, view.bottom() - 74.0),
        egui::pos2(view.right() - 12.0, view.bottom() - 8.0),
    );
    egui::Area::new(egui::Id::unique("env-bendsim-timeline"))
        .fixed_pos(band.min)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_black_alpha(170))
                .corner_radius(6.0)
                .inner_margin(6.0)
                .show(ui, |ui| {
                    ui.set_width(band.width() - 12.0);
                    super::bendsim::timeline_ui(ui, b);
                });
        });
    s.places.push(("env:bendsim:view".into(), view));
    s.places.push(("env:bendsim:timeline".into(), band));
    false
}

pub(super) fn command(s: &mut Shell, c: u16) {
    let Some(b) = s.sheet.bend_sim.as_deref_mut() else {
        return;
    };
    let n = b.sim.as_ref().map_or(0, |x| x.plan.steps.len());
    let again = |b: &mut BendSimUi, fg: &super::fg::FgPanel, tooling: &str| {
        b.source = fg.source.clone();
        b.path = fg.path.clone();
        let mut c = b.choice.clone();
        if tooling != b.tooling {
            // a stand-in set asked for: off the library's press
            c = Default::default();
        }
        b.redo(c, tooling);
    };
    match c {
        FIRST => b.go(0),
        BACK => b.go(b.step.saturating_sub(1)),
        NEXT => b.go((b.step + 1).min(n.saturating_sub(1))),
        PLAY => b.play(),
        AGAIN => {
            if let Some(fg) = s.fg.as_ref() {
                let tooling = b.tooling.clone();
                again(b, fg, &tooling);
            }
        }
        k => {
            if let (Some(id), Some(fg)) = (forge_brake::library::IDS.get(usize::from(k)), s.fg.as_ref()) {
                again(b, fg, id);
            }
        }
    }
}

/// A built-in tooling set's word on the ribbon.
fn tooling_word(id: &str) -> &'static str {
    match id {
        "std-eu" => "Stand-in European",
        "std-us" => "Stand-in American",
        _ => "Tooling",
    }
}
