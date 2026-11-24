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
    Some(format!(
        "Bend Simulation · {machine} · {n} bend{} · {}",
        if n == 1 { "" } else { "s" },
        if sim.plan.feasible {
            "feasible"
        } else {
            "no feasible order"
        }
    ))
}

pub(super) fn groups(s: &Shell) -> Vec<Group> {
    let b = s.sheet.bend_sim.as_deref();
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
            vec![tool("Suggest Order", "redo", cmd(SUGGEST), false)],
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
                tool("First Bend", "first", cmd(FIRST), false),
                tool("Previous Bend", "stepback", cmd(BACK), false),
                tool("Next Bend", "stepfwd", cmd(NEXT), false),
                tool("Last Bend", "last", cmd(LAST), false),
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
            ui.set_max_width(r.width() - 24.0);
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
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r.shrink2(egui::vec2(12.0, 10.0))));
    child.set_clip_rect(r);
    let mut flat = None;
    egui::ScrollArea::vertical()
        .id_salt("env-bendsim-dock")
        .show(&mut child, |ui| {
            ui.set_max_width(r.width() - 40.0);
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
    let Some(b) = s.sheet.bend_sim.as_deref_mut() else {
        return false;
    };
    super::bendsim::overlay(ui, view, b);
    // what is seen through: its outline, so the ghost reads
    if let Some((cam, size)) = s.camera
        && b.see_through
        && !b.outlines.is_empty()
    {
        let m = cam.view(size.0.max(1), size.1.max(1)).matrix;
        let sx = view.width() / size.0.max(1) as f32;
        let sy = view.height() / size.1.max(1) as f32;
        let to = |p: &[f32; 3]| -> Option<egui::Pos2> {
            let c = |r: usize| m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r];
            let w = c(3);
            (w > 1e-6).then(|| {
                let (x, y) = (c(0) / w, c(1) / w);
                egui::pos2(
                    view.left() + (x + 1.0) * 0.5 * size.0 as f32 * sx,
                    view.top() + (1.0 - y) * 0.5 * size.1 as f32 * sy,
                )
            })
        };
        let p = ui.painter_at(view);
        for o in &b.outlines {
            let pts: Vec<egui::Pos2> = o.iter().filter_map(to).collect();
            let hull = hull(pts);
            if hull.len() > 2 {
                p.add(egui::Shape::closed_line(
                    hull,
                    egui::Stroke::new(1.5, egui::Color32::from_rgba_unmultiplied(235, 240, 250, 210)),
                ));
            }
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
                    super::bendsim::timeline_ui(ui, b, bar);
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
        LAST => b.go(n.saturating_sub(1)),
        FOLLOW => {
            b.follow = !b.follow;
            if b.follow {
                b.frame = Some(super::bendsim::Framing::Work);
            }
        }
        SEE => b.see_through = !b.see_through,
        MACHINE => b.pane = super::bendsim::Pane::Setup,
        SUGGEST => {
            b.pane = super::bendsim::Pane::Program;
            b.suggest = true;
        }
        WRITE => {
            if let Err(e) = b.write_doc() {
                s.status = format!("Bend Simulation: {e}");
            }
        }
        OVERVIEW => b.frame = Some(super::bendsim::Framing::Overview),
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

/// The convex hull of points on the screen (Andrew's monotone chain), counter-clockwise.
fn hull(mut p: Vec<egui::Pos2>) -> Vec<egui::Pos2> {
    p.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    p.dedup();
    if p.len() < 3 {
        return p;
    }
    let cross =
        |o: egui::Pos2, a: egui::Pos2, b: egui::Pos2| (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
    let mut lower: Vec<egui::Pos2> = Vec::new();
    for &q in &p {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], q) <= 0.0 {
            lower.pop();
        }
        lower.push(q);
    }
    let mut upper: Vec<egui::Pos2> = Vec::new();
    for &q in p.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], q) <= 0.0 {
            upper.pop();
        }
        upper.push(q);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
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
