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
const OVERVIEW: u16 = 105;
const MACHINE: u16 = 107;
const SUGGEST: u16 = 108;
const WRITE: u16 = 109;
const LAST: u16 = 110;
const FOLLOW: u16 = 111;
const SEE: u16 = 112;
const PLAN: u16 = 113;

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
    // the world's axes cross the whole view at the press's scale (STANDARD.md §2): off while in
    let axes = s.overlays().axes;
    if let Some(b) = &mut s.sheet.bend_sim
        && b.axes_was.is_none()
    {
        b.axes_was = Some(axes);
        s.overlays_mut().axes = false;
    }
    Ok(())
}

/// Out of it: play stops, the rest kept.
pub(super) fn exit(s: &mut Shell) {
    let mut axes = None;
    if let Some(b) = &mut s.sheet.bend_sim {
        b.playing = false;
        axes = b.axes_was.take();
    }
    if let Some(a) = axes {
        s.overlays_mut().axes = a;
    }
}

/// The status bar's line: the machine, how many bends, whether the program is feasible.
pub(super) fn status(s: &Shell) -> Option<String> {
    let b = s.sheet.bend_sim.as_deref()?;
    let sim = b.sim.as_ref().ok()?;
    let machine = match &b.choice.press {
        Some(id) => b
            .catalog
            .as_ref()
            .and_then(|c| c.as_ref().ok())
            .and_then(|c| c.press(id))
            .map_or(id.clone(), |r| format!("{} {}", r.prov.maker, r.model)),
        None => "stand-in tooling".into(),
    };
    let n = sim.plan.steps.len();
    // the environment owns the status bar: no feasible order is said there with its first
    // reason (BUTTON-AUDITOR: Suggest Order and Simulate Again never silent)
    Some(format!(
        "Bend Simulation · {machine} · {n} bend{} · {}",
        if n == 1 { "" } else { "s" },
        infeasible(b).unwrap_or_else(|| "feasible".into())
    ))
}

/// Why a Bend Simulation tool is off, in words (its tooltip; BUTTON-AUDITOR: no silent button).
pub(super) fn why_off(s: &Shell, name: &str) -> Option<String> {
    let b = s.sheet.bend_sim.as_deref()?;
    let sim = match &b.sim {
        Ok(x) => x,
        Err(e) => return Some(format!("nothing simulated: {e}")),
    };
    let n = sim.plan.steps.len();
    match name {
        "First Bend" | "Previous Bend" if b.step == 0 => Some("already at bend 1".into()),
        "Next Bend" | "Last Bend" if b.step + 1 >= n => Some(format!("already at the last bend ({n})")),
        "Write Program" if b.doc.is_some() => Some("the program is written: its document is open in the browser".into()),
        _ => None,
    }
}

/// A catalogue key in words: "trumpf/EV004_30deg" reads "EV004 30°" (UX-LEAD; the key itself
/// on hover where it is shown).
pub fn tool_words(id: &str) -> String {
    let w = id.rsplit('/').next().unwrap_or(id).replace('_', " ");
    w.split(' ')
        .map(|p| {
            // "30deg", "85deg" read as degrees; a part such as "V16-85deg" too
            match p.rsplit_once('-') {
                Some((a, b)) if b.ends_with("deg") => format!("{a}-{}", degrees(b)),
                _ => degrees(p),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn degrees(p: &str) -> String {
    match p.strip_suffix("deg") {
        Some(n) if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit() || c == '.') => format!("{n}°"),
        _ => p.to_string(),
    }
}

/// The planner found no feasible order: the first reason, in words.
pub(super) fn infeasible(b: &BendSimUi) -> Option<String> {
    let sim = b.sim.as_ref().ok().filter(|x| !x.plan.feasible)?;
    let first = sim.plan.blocking.first().map(|k| {
        let rules: Vec<String> = k
            .checks
            .iter()
            .filter(|c| !c.pass)
            .map(|c| super::bendsim_cards::rule_failed(c.rule).to_string())
            .collect();
        format!(
            "bend {} ({}) fails: {} (best tried: {} punch on {} die)",
            k.bend + 1,
            sim.names[k.bend],
            if rules.is_empty() { "its checks".to_string() } else { rules.join("; ") },
            tool_words(&k.punch),
            tool_words(&k.die)
        )
    });
    Some(format!(
        "No feasible order: {}",
        first.unwrap_or_else(|| "the planner found none with these tools".into())
    ))
}

pub(super) fn groups(s: &Shell) -> Vec<Group> {
    let b = s.sheet.bend_sim.as_deref();
    let n = b.and_then(|b| b.sim.as_ref().ok()).map_or(0, |x| x.plan.steps.len());
    let at_first = b.is_some_and(|b| b.step == 0);
    let at_last = b.is_some_and(|b| b.step + 1 >= n);
    let ran = b.is_some_and(|b| b.sim.is_ok());
    let cmd = |c: u16| ran.then_some(Action::Env(EnvId::BendSim, c));
    let any = b.is_some().then_some(());
    let playing = b.is_some_and(|b| b.playing);
    let setup = b.is_some_and(|b| b.pane == super::bendsim::Pane::Setup);
    let written = b.is_some_and(|b| b.doc.is_some());
    vec![
        group(
            "Machine",
            vec![tool(
                "Choose Press",
                "bendtable",
                any.map(|_| Action::Env(EnvId::BendSim, MACHINE)),
                setup,
            )],
            vec![tool(
                "Tooling",
                "bendtable",
                any.map(|_| Action::Env(EnvId::BendSim, MACHINE)),
                false,
            )],
        ),
        group(
            "Program",
            vec![
                tool("Suggest Order", "redo", cmd(SUGGEST), false),
                tool("Plan Stations", "bendtable", cmd(PLAN), false),
            ],
            vec![
                tool(
                    "Write Program",
                    "save",
                    if written { None } else { cmd(WRITE) },
                    false,
                ),
                tool(
                    "Simulate Again",
                    "redo",
                    any.map(|_| Action::Env(EnvId::BendSim, AGAIN)),
                    false,
                ),
            ],
        ),
        group(
            "Play",
            vec![tool(
                if playing { "Pause" } else { "Play" },
                if playing { "pause" } else { "play" },
                cmd(PLAY),
                playing,
            )],
            vec![
                tool("First Bend", "first", cmd(FIRST).filter(|_| !at_first), false),
                tool("Previous Bend", "stepback", cmd(BACK).filter(|_| !at_first), false),
                tool("Next Bend", "stepfwd", cmd(NEXT).filter(|_| !at_last), false),
                tool("Last Bend", "last", cmd(LAST).filter(|_| !at_last), false),
            ],
        ),
        group(
            "View",
            vec![tool("Press Overview", "zoom", cmd(OVERVIEW), false)],
            vec![
                tool("Follow Bend", "eye", cmd(FOLLOW), b.is_some_and(|b| b.follow)),
                tool("See Through", "hide", cmd(SEE), b.is_some_and(|b| b.see_through)),
            ],
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
            use super::bendsim::Pane;
            // the panel's width: long lines wrap within it
            ui.set_max_width((r.width() - 24.0).max(0.0));
            ui.spacing_mut().item_spacing.x = ui.spacing().item_spacing.x.max(6.0);
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                for (pane, word) in [(Pane::Program, "Program"), (Pane::Setup, "Machine & Tools")] {
                    if ui.selectable_label(b.pane == pane, word).clicked() {
                        b.pane = pane;
                    }
                }
            });
            match b.pane {
                Pane::Setup => super::bendsim::setup_ui(ui, &t, &mut b),
                Pane::Program => super::bendsim::sequence_ui(ui, &t, &mut b),
            }
        });
    s.places.push(("env:bendsim:browser".into(), r));
    s.sheet.bend_sim = Some(b);
    true
}

/// The docked properties panel's width: the selected bend's sheet.
pub(super) fn dock_width(s: &Shell) -> Option<f32> {
    s.sheet.bend_sim.is_some().then_some(340.0)
}

/// The selected bend's properties, docked at the right.
pub(super) fn dock(s: &mut Shell, ui: &mut Ui, r: Rect) {
    let Some(mut b) = s.sheet.bend_sim.take() else {
        return;
    };
    let t = s.theme;
    b.inch = s.units == "in";
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r.shrink2(egui::vec2(12.0, 10.0))));
    child.set_clip_rect(r);
    let mut flat = None;
    egui::ScrollArea::vertical()
        .id_salt("env-bendsim-dock")
        .show(&mut child, |ui| {
            ui.set_max_width((r.width() - 40.0).max(0.0));
            flat = super::bendsim::bend_sheet(ui, &t, &mut b);
        });
    s.sheet.bend_sim = Some(b);
    // the flat pattern, its bend lines shown, the bend named (picking one line out is a gap)
    if let Some(bend) = flat {
        s.sheet.bend_lines = true;
        s.out.push(Action::EnterEnv(EnvId::FlatPattern));
        s.status = format!("Flat Pattern: bend {bend}");
    }
}

/// Play's clock, and the keys (docs/ux/STANDARD.md §9): Space plays or pauses, the left and
/// right arrows step back and on, F frames the part and tools; not while a field has the keys.
pub(super) fn panels(s: &mut Shell, ctx: &egui::Context) {
    if let Some(b) = &mut s.sheet.bend_sim
        && b.sim.is_ok()
        && !ctx.egui_wants_keyboard_input()
    {
        let n = b.steps();
        let (space, left, right, f, home, end) = ctx.input(|i| {
            let k = |key| i.key_pressed(key) && i.modifiers.is_none();
            (
                k(egui::Key::Space),
                k(egui::Key::ArrowLeft),
                k(egui::Key::ArrowRight),
                k(egui::Key::F),
                k(egui::Key::Home),
                k(egui::Key::End),
            )
        });
        if home {
            b.go(0);
        }
        if end {
            b.go(n.saturating_sub(1));
        }
        if space {
            b.play();
        }
        if left {
            b.go(b.step.saturating_sub(1));
        }
        if right {
            b.go((b.step + 1).min(n.saturating_sub(1)));
        }
        if f {
            b.frame = Some(super::bendsim::Framing::Work);
        }
    }
    if let Some(b) = &mut s.sheet.bend_sim {
        b.tick(f64::from(ctx.input(|i| i.stable_dt)).min(0.1));
        // playing, or moved from outside (the page, the agent): the view has frames to draw
        let now = (b.step, b.t.to_bits());
        if b.painted != Some(now) {
            b.painted = Some(now);
            // the new instant's bodies go up this frame; the view's shadows and the panels
            // settle over the next few
            b.settle = 6;
        }
        if b.playing || b.settle > 0 {
            b.settle = b.settle.saturating_sub(1);
            ctx.request_repaint();
        }
    }
}

/// The view is the renderer's (the press in 3D, gui_bendsim.rs): over it, the machine, step and
/// failures, and the timeline along its foot. The view keeps its pointer (orbit, zoom).
pub(super) fn draw(s: &mut Shell, ui: &mut Ui, view: Rect) -> bool {
    let camera = s.camera;
    let theme = s.theme;
    let Some(b) = s.sheet.bend_sim.as_deref_mut() else {
        return false;
    };
    super::bendsim::overlay(ui, view, b);
    if b.overview {
        for (k, r) in station_chips(ui, view, camera, &theme, b) {
            s.places.push((format!("env:bendsim:station:{}", k + 1), r));
        }
    }
    // a narrow view: the view's settings on a row of their own, the bar taller for it
    let bar = super::bendsim::Bar::for_width(view.width());
    let narrow = bar == super::bendsim::Bar::Narrow;
    let band = Rect::from_min_max(
        egui::pos2(
            view.left() + 12.0,
            view.bottom() - if narrow { 104.0 } else { 74.0 },
        ),
        egui::pos2(view.right() - 12.0, view.bottom() - 8.0),
    );
    // a view too narrow for the bar (BUTTON-AUDITOR's 480 px window: a view ~200 px, or none):
    // no bar; the ribbon's Play group still steps through the program
    s.places.push(("env:bendsim:view".into(), view));
    if band.width() < MIN_BAR {
        return false;
    }
    egui::Area::new(egui::Id::unique("env-bendsim-timeline"))
        .fixed_pos(band.min)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_black_alpha(170))
                .corner_radius(6.0)
                .inner_margin(6.0)
                .show(ui, |ui| {
                    ui.set_width((band.width() - 12.0).max(0.0));
                    super::bendsim::timeline_ui(ui, b, bar);
                });
        });
    s.places.push(("env:bendsim:timeline".into(), band));
    false
}

/// The narrowest band the timeline bar is drawn in (its transport, the stage strip).
pub const MIN_BAR: f32 = 240.0;

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
        LAST => b.go(n.saturating_sub(1)),
        FOLLOW => {
            b.follow = !b.follow;
            if b.follow {
                b.frame = Some(super::bendsim::Framing::Work);
                b.overview = false;
            }
        }
        SEE => b.see_through = !b.see_through,
        MACHINE => b.pane = super::bendsim::Pane::Setup,
        SUGGEST => {
            b.pane = super::bendsim::Pane::Program;
            b.suggest = true;
            if let Some(w) = infeasible(b) {
                s.status = w;
            }
        }
        WRITE => {
            if let Err(e) = b.write_doc() {
                s.status = format!("Bend Simulation: {e}");
            }
        }
        OVERVIEW => {
            b.frame = Some(super::bendsim::Framing::Overview);
            b.overview = true;
        }
        PLAN => {
            // the program's tools laid out as stations along the bed (BENDSIM-TOOLS' planner);
            // over a document's own stations only when the user says so
            let mut c = b.choice.clone();
            c.stations_auto = true;
            let tooling = b.tooling.clone();
            b.redo(c, &tooling);
        }
        AGAIN => {
            if let Some(fg) = s.fg.as_ref() {
                let tooling = b.tooling.clone();
                again(b, fg, &tooling);
            }
            // never silent: what the planner found, or why there is no order
            let b = s.sheet.bend_sim.as_deref().unwrap();
            s.status = infeasible(b).unwrap_or_else(|| match &b.sim {
                Ok(x) => format!("Simulated again: {} bends, feasible", x.plan.steps.len()),
                Err(e) => format!("Bend Simulation: {e}"),
            });
            if let Some(n) = b.note.clone() {
                s.status = format!("{} · {n}", s.status);
            }
        }
        k => {
            if let (Some(id), Some(fg)) = (forge_brake::library::IDS.get(usize::from(k)), s.fg.as_ref()) {
                again(b, fg, id);
            }
        }
    }
}

/// The program's length in seconds as it plays (`STEP_SECONDS` a step); 0 with nothing simulated.
pub(super) fn duration(s: &Shell) -> f64 {
    s.sheet
        .bend_sim
        .as_deref()
        .filter(|b| b.sim.is_ok())
        .map_or(0.0, |b| b.steps() as f64 * super::bendsim::STEP_SECONDS)
}

/// The simulation put at `t` seconds into the program: its step and instant, play stopped, Follow
/// bend on (a recording frames each bend and keeps the handling in view). The instant's pose
/// (ram, sheet, backgauge) is a function of these alone: no clock is read.
pub(super) fn seek(s: &mut Shell, t: f64) {
    if let Some(b) = s.sheet.bend_sim.as_deref_mut() {
        b.playing = false;
        // a recording follows the bends: each framed on its station, the handling kept in view
        b.follow = true;
        b.set_position(t.max(0.0) / super::bendsim::STEP_SECONDS);
    }
}

/// "Bend 2 of 5 · bend" at `t` seconds.
pub(super) fn label_at(s: &Shell, t: f64) -> Option<String> {
    let b = s.sheet.bend_sim.as_deref()?;
    let sim = b.sim.as_ref().ok()?;
    let n = b.steps();
    if n == 0 {
        return None;
    }
    let x = (t.max(0.0) / super::bendsim::STEP_SECONDS).min(n as f64);
    let k = (x.floor() as usize).min(n - 1);
    let (stage, _) = forge_brake::scene::stage_at((x - k as f64).clamp(0.0, 1.0));
    Some(format!(
        "Bend {} of {n} · {} · {}",
        k + 1,
        sim.names[sim.plan.steps[k].bend],
        stage.word()
    ))
}

/// A station's chip in the press overview: its name, tools and bends, over its place on the bed.
pub struct Chip {
    pub station: usize,
    pub text: String,
    /// The tools' catalogue keys (on hover).
    pub keys: String,
    pub rect: Rect,
    /// Its station's place on the screen (the chip's leader runs to it).
    pub at: egui::Pos2,
    /// The station the shown bend works at.
    pub here: bool,
}

/// The stations' chips, where they stand on the screen (UX-LEAD: the overview labelled, a click
/// selects): each over its die's middle at the bed, the working station's in the accent.
pub fn chips(ui: &Ui, view: Rect, camera: Option<(crate::camera::Camera, (u32, u32))>, b: &BendSimUi) -> Vec<Chip> {
    let (Some((cam, size)), Ok(sim)) = (camera, b.sim.as_ref()) else {
        return vec![];
    };
    let ppp = ui.ctx().pixels_per_point();
    let here = sim.plan.steps.get(b.step).and_then(|s| s.station);
    let short = tool_words;
    let mut out = Vec::new();
    for (k, st) in sim.tooling.stations.iter().enumerate() {
        let bends: Vec<String> = sim
            .plan
            .steps
            .iter()
            .enumerate()
            .filter(|(_, s)| s.station == Some(k))
            .map(|(i, _)| (i + 1).to_string())
            .collect();
        let keys = format!(
            "punch {} · die {}",
            sim.tooling.punches[st.punch].id, sim.tooling.dies[st.die].id
        );
        let text = format!(
            "S{} · {} punch / {} die · {}",
            k + 1,
            short(&sim.tooling.punches[st.punch].id),
            short(&sim.tooling.dies[st.die].id),
            if bends.is_empty() {
                "no bends".to_string()
            } else {
                format!("bend{} {}", if bends.len() == 1 { "" } else { "s" }, bends.join(", "))
            }
        );
        // the die's foot on the bed, at the station's middle
        let at = [0.0f32, st.y as f32, -(sim.tooling.dies[st.die].height as f32) - 10.0];
        let Some((x, y)) = cam.pixel_of(at, size.0, size.1) else { continue };
        let q = egui::pos2(view.left() + x / ppp, view.top() + y / ppp);
        if !view.contains(q) {
            continue;
        }
        let g = ui.painter().layout_no_wrap(text.clone(), egui::FontId::proportional(12.0), egui::Color32::WHITE);
        // hung under its station's place, clear of the tools
        let sz = g.size() + egui::vec2(12.0, 6.0);
        let rect = Rect::from_min_size(egui::pos2(q.x - 0.5 * sz.x, q.y + 14.0), sz);
        out.push(Chip {
            station: k,
            text,
            keys,
            rect,
            at: q,
            here: here == Some(k),
        });
    }
    // stations close on the screen (seen along the bed): each chip moved down clear of the ones
    // already placed, a leader back to its station
    let mut placed: Vec<Rect> = Vec::new();
    for c in &mut out {
        while placed.iter().any(|r| r.expand(2.0).intersects(c.rect)) {
            c.rect = c.rect.translate(egui::vec2(0.0, c.rect.height() + 4.0));
        }
        placed.push(c.rect);
    }
    out
}

fn station_chips(
    ui: &mut Ui,
    view: Rect,
    camera: Option<(crate::camera::Camera, (u32, u32))>,
    t: &super::theme::Theme,
    b: &mut BendSimUi,
) -> Vec<(usize, Rect)> {
    let list = chips(ui, view, camera, b);
    let p = ui.painter_at(view);
    let mut go = None;
    for c in &list {
        let r = ui.interact(c.rect, egui::Id::unique("bendsim-station-chip").with(c.station), egui::Sense::click());
        let fill = if c.here {
            t.accent
        } else if r.hovered() {
            egui::Color32::from_black_alpha(220)
        } else {
            egui::Color32::from_black_alpha(170)
        };
        if !c.rect.contains(c.at) {
            let from = egui::pos2(c.rect.center().x, c.rect.top());
            p.line_segment([from, c.at], egui::Stroke::new(1.0, egui::Color32::from_black_alpha(170)));
            p.circle_filled(c.at, 3.0, fill);
        }
        p.rect_filled(c.rect, 9.0, fill);
        p.text(
            c.rect.center(),
            egui::Align2::CENTER_CENTER,
            &c.text,
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
        let r = r.on_hover_text(format!("{}\nclick: go to this station's first bend", c.keys));
        if r.clicked() {
            go = Some(c.station);
        }
    }
    if let Some(k) = go {
        select_station(b, k);
    }
    list.iter().map(|c| (c.station, c.rect)).collect()
}

/// A station chosen (its chip clicked): its first bend shown, framed at its tools.
pub fn select_station(b: &mut BendSimUi, k: usize) {
    let first = b
        .sim
        .as_ref()
        .ok()
        .and_then(|s| s.plan.steps.iter().position(|st| st.station == Some(k)));
    if let Some(i) = first {
        b.go(i);
        b.playing = false;
        b.overview = false;
        b.frame = Some(super::bendsim::Framing::Work);
    }
}

#[cfg(test)]
mod words_tests {
    #[test]
    fn a_catalogue_key_reads_in_words() {
        assert_eq!(super::tool_words("trumpf/EV004_30deg"), "EV004 30°");
        assert_eq!(super::tool_words("bystronic/V16-85deg_H55"), "V16-85° H55");
        assert_eq!(super::tool_words("trumpf/OW202-S"), "OW202-S");
    }
}

#[cfg(test)]
mod timeline_tests {
    use super::*;

    #[test]
    fn the_recorder_steps_the_program_by_its_clock() {
        let src = "shared t = 2\nshared r = 2\nshared k = ratio(0.4)\n\
            sketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (100, 200))\n}\n\
            base = sheet(plan, t: t, r: r, bend: kfactor(k))\n\
            right = flange(base.edge(plan.outline[1]), height: 30)\n\
            left = flange(base.edge(plan.outline[3]), height: 30)\n";
        let mut s = Shell::default();
        s.sheet.bend_sim = Some(Box::new(BendSimUi::open(src, "c.fg", "std-eu")));
        let n = s.sheet.bend_sim.as_ref().unwrap().steps();
        assert_eq!(n, 2);
        assert_eq!(duration(&s), 2.0 * super::super::bendsim::STEP_SECONDS);
        // seconds in: the second bend, 60 % through its motion, the same every time
        let t = super::super::bendsim::STEP_SECONDS * 1.6;
        seek(&mut s, t);
        let b = s.sheet.bend_sim.as_ref().unwrap();
        assert_eq!(b.step, 1);
        assert!((b.t - 0.6).abs() < 1e-9 && !b.playing);
        assert!(b.follow, "a recording follows the bends");
        let pose = b.pose().unwrap();
        seek(&mut s, 0.0);
        seek(&mut s, t);
        assert_eq!(s.sheet.bend_sim.as_ref().unwrap().pose().unwrap(), pose);
        assert!(
            label_at(&s, t).unwrap().starts_with("Bend 2 of 2"),
            "{:?}",
            label_at(&s, t)
        );
        // the end is the last step's end
        let end = duration(&s) + 5.0;
        seek(&mut s, end);
        let b = s.sheet.bend_sim.as_ref().unwrap();
        assert_eq!((b.step, b.t), (1, 1.0));
    }
}

