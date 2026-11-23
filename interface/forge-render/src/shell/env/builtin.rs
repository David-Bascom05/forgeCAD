//! The environments: those on main (Thermoform, Flat Pattern, Bend Simulation, Render Studio,
//! Drawing) and the places kept for those still on their branches (Manufacture, Machine Builder,
//! Robot) or not begun (Simulation), drawn off with the reason.

use super::super::ribbon::{Group, Tool};
use super::super::sheet::SheetCmd;
use super::super::{Action, FLAT_TAB, Shell, THERMO_TAB};
use super::{BENDSIM_TAB, Entered, EnvId, Environment, RENDER_TAB};
use egui::{Rect, Ui};

pub(super) fn of(id: EnvId) -> &'static dyn Environment {
    match id {
        EnvId::Thermoform => &Thermoform,
        EnvId::FlatPattern => &FlatPattern,
        EnvId::BendSim => &BendSim,
        EnvId::RenderStudio => &RenderStudio,
        EnvId::Drawing => &Drawing,
        EnvId::Manufacture => &Pending(EnvId::Manufacture, MANUFACTURE),
        EnvId::MachineBuilder => &Pending(EnvId::MachineBuilder, MACHINE),
        EnvId::Robot => &Pending(EnvId::Robot, ROBOT),
        EnvId::Simulation => &Pending(EnvId::Simulation, SIMULATION),
    }
}

const MANUFACTURE: &str =
    "coming: the Manufacture workspace (CAM setups, operations, posts, simulation) is still on its branch";
const MACHINE: &str = "coming: the Machine Builder is still on its branch";
const ROBOT: &str = "coming: the robot (URDF) editor is still on its branch";
const SIMULATION: &str = "not yet: the kernel has no structural or motion solver";

/// The ribbon's tuples (thermo_ui, harness_ui give theirs so) as groups.
type Button = (&'static str, &'static str, Option<Action>, bool);
fn as_groups(gs: Vec<(&'static str, Vec<Button>, Vec<Button>)>) -> Vec<Group> {
    let tool = |(name, icon, action, on): Button| Tool {
        name,
        icon,
        action,
        on,
    };
    gs.into_iter()
        .map(|(name, big, small)| Group {
            name,
            big: big.into_iter().map(tool).collect(),
            small: small.into_iter().map(tool).collect(),
            icons: vec![],
        })
        .collect()
}

fn group(name: &'static str, big: Vec<Tool>, small: Vec<Tool>) -> Group {
    Group {
        name,
        big,
        small,
        icons: vec![],
    }
}

fn tool(name: &'static str, icon: &'static str, action: Option<Action>, on: bool) -> Tool {
    Tool {
        name,
        icon,
        action,
        on,
    }
}

/// A Forge part (not an assembly) is open.
fn part(s: &Shell) -> Result<&super::super::fg::FgPanel, String> {
    match &s.fg {
        None => Err("open or create a Forge part first (File > New, or Open)".into()),
        Some(f) if f.assembly => Err("a part's, not an assembly's: open the part".into()),
        Some(f) => Ok(f),
    }
}

/// A sheet metal part with a sheet to flatten or bend.
fn sheet_part(s: &Shell) -> Result<&super::super::fg::FgPanel, String> {
    let f = part(s)?;
    if f.sheets.is_empty() {
        return Err("make the part sheet metal first (3D Model > Convert to Sheet Metal, then Face)".into());
    }
    Ok(f)
}

struct Thermoform;

impl Environment for Thermoform {
    fn id(&self) -> EnvId {
        EnvId::Thermoform
    }

    fn available(&self, s: &Shell) -> Result<(), String> {
        match &s.fg {
            None => Err("open or create a Forge part first (File > New, or Open)".into()),
            Some(f) if f.assembly && f.thermo.asm_files.is_empty() => {
                Err("this assembly has no thermoformed parts".into())
            }
            Some(_) => Ok(()),
        }
    }

    fn enter(&self, _s: &mut Shell) -> Result<Entered, String> {
        Ok(Entered::Mode)
    }

    fn tabs(&self) -> &'static [(usize, &'static str)] {
        &[(THERMO_TAB, "Thermoform")]
    }

    fn groups(&self, s: &Shell, _tab: usize) -> Vec<Group> {
        as_groups(super::super::thermo_ui::groups(s))
    }
}

struct FlatPattern;

impl Environment for FlatPattern {
    fn id(&self) -> EnvId {
        EnvId::FlatPattern
    }

    fn available(&self, s: &Shell) -> Result<(), String> {
        let f = sheet_part(s)?;
        match f.sheets.iter().find_map(|x| x.refused.clone()) {
            Some(why) => Err(format!("no flat pattern: {why}")),
            None => Ok(()),
        }
    }

    fn enter(&self, s: &mut Shell) -> Result<Entered, String> {
        let fg = s.fg.as_ref().ok_or("no part")?;
        s.sheet.command(SheetCmd::FlatPattern, fg)?;
        Ok(Entered::Mode)
    }

    fn exit(&self, s: &mut Shell) {
        if let Some(fg) = s.fg.as_ref() {
            let _ = s.sheet.command(SheetCmd::Folded, fg);
        }
    }

    fn tabs(&self) -> &'static [(usize, &'static str)] {
        &[(FLAT_TAB, "Flat Pattern")]
    }

    fn groups(&self, s: &Shell, _tab: usize) -> Vec<Group> {
        super::super::sheetui::flat_groups(s)
    }

    /// Go to Folded Part is its way out, as Inventor has it.
    fn finish_button(&self) -> bool {
        false
    }

    fn still(&self, s: &Shell) -> bool {
        s.sheet.flat_env && s.fg.is_some()
    }
}

/// The press brake simulation (kernel/forge-brake): the section drawn in the view, the bend
/// order and each step's checks in the browser, play and the tooling on the ribbon.
struct BendSim;

/// Its commands: a tooling set by its index in `forge_brake::library::IDS`, then these.
const FIRST: u16 = 100;
const BACK: u16 = 101;
const PLAY: u16 = 102;
const NEXT: u16 = 103;
const AGAIN: u16 = 104;

impl Environment for BendSim {
    fn id(&self) -> EnvId {
        EnvId::BendSim
    }

    fn available(&self, s: &Shell) -> Result<(), String> {
        sheet_part(s).map(|_| ())
    }

    /// Simulated afresh if the part's text changed since; its step kept where it still is one.
    fn enter(&self, s: &mut Shell) -> Result<Entered, String> {
        if s.sheet.flat_env {
            FlatPattern.exit(s);
        }
        let fg = s.fg.as_ref().ok_or("no part")?;
        let fresh = s
            .sheet
            .bend_sim
            .as_ref()
            .is_some_and(|b| b.source == fg.source && b.path == fg.path);
        if !fresh {
            let was = s.sheet.bend_sim.take();
            let tooling = was.as_ref().map_or("std-eu", |b| b.tooling.as_str());
            let mut b = super::super::bendsim::BendSimUi::open(&fg.source, &fg.path, tooling);
            if let Some(w) = was {
                b.resume(&w);
            }
            s.sheet.bend_sim = Some(Box::new(b));
        }
        Ok(Entered::Mode)
    }

    fn exit(&self, s: &mut Shell) {
        if let Some(b) = &mut s.sheet.bend_sim {
            b.playing = false;
        }
    }

    fn tabs(&self) -> &'static [(usize, &'static str)] {
        &[(BENDSIM_TAB, "Bend Simulation")]
    }

    fn groups(&self, s: &Shell, _tab: usize) -> Vec<Group> {
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
                    b.is_some_and(|b| b.tooling == *id),
                )
            })
            .collect();
        vec![
            group("Tooling", vec![], tooling),
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

    fn browser(&self, s: &mut Shell, ui: &mut Ui, r: Rect) -> bool {
        let Some(mut b) = s.sheet.bend_sim.take() else {
            return false;
        };
        let t = s.theme;
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r.shrink2(egui::vec2(10.0, 8.0))));
        child.set_clip_rect(r);
        egui::ScrollArea::both()
            .id_salt("env-bendsim-browser")
            .show(&mut child, |ui| {
                ui.label(
                    egui::RichText::new("Bend order")
                        .font(super::super::theme::semi(13.0))
                        .color(t.ink),
                );
                super::super::bendsim::body(ui, &mut b, false);
            });
        s.places.push(("env:bendsim:browser".into(), r));
        s.sheet.bend_sim = Some(b);
        true
    }

    fn panels(&self, s: &mut Shell, ctx: &egui::Context) {
        if let Some(b) = &mut s.sheet.bend_sim {
            b.tick(f64::from(ctx.input(|i| i.stable_dt)).min(0.1));
            if b.playing {
                ctx.request_repaint();
            }
        }
    }

    /// The press section fills the view: the sim is the picture here.
    fn draw(&self, s: &mut Shell, ui: &mut Ui, view: Rect) -> bool {
        let Some(b) = s.sheet.bend_sim.as_deref() else {
            return false;
        };
        super::super::bendsim::paint(ui, view, b);
        s.places.push(("env:bendsim:view".into(), view));
        true
    }

    fn command(&self, s: &mut Shell, c: u16) {
        let Some(b) = s.sheet.bend_sim.as_deref_mut() else {
            return;
        };
        let n = b.sim.as_ref().map_or(0, |x| x.plan.steps.len());
        match c {
            FIRST => b.go(0),
            BACK => b.go(b.step.saturating_sub(1)),
            NEXT => b.go((b.step + 1).min(n.saturating_sub(1))),
            PLAY => b.play(),
            AGAIN => {
                if let Some(fg) = s.fg.as_ref() {
                    let mut fresh = super::super::bendsim::BendSimUi::open(&fg.source, &fg.path, &b.tooling);
                    fresh.resume(b);
                    *b = fresh;
                }
            }
            k => {
                if let (Some(id), Some(fg)) = (forge_brake::library::IDS.get(usize::from(k)), s.fg.as_ref()) {
                    let mut fresh = super::super::bendsim::BendSimUi::open(&fg.source, &fg.path, id);
                    fresh.resume(b);
                    *b = fresh;
                }
            }
        }
    }

    fn still(&self, s: &Shell) -> bool {
        sheet_part(s).is_ok()
    }
}

/// A built-in tooling set's word on the ribbon.
fn tooling_word(id: &str) -> &'static str {
    match id {
        "std-eu" => "European Tooling",
        "std-us" => "American Tooling",
        _ => "Tooling",
    }
}

/// The look of the picture: style, ground, shadows, camera. Its content (materials, lights,
/// environments, render output) is the Render Studio worker's.
struct RenderStudio;

impl Environment for RenderStudio {
    fn id(&self) -> EnvId {
        EnvId::RenderStudio
    }

    fn available(&self, s: &Shell) -> Result<(), String> {
        match s.fg {
            Some(_) => Ok(()),
            None => Err("open a part or an assembly first".into()),
        }
    }

    fn enter(&self, _s: &mut Shell) -> Result<Entered, String> {
        Ok(Entered::Mode)
    }

    fn tabs(&self) -> &'static [(usize, &'static str)] {
        &[(RENDER_TAB, "Render Studio")]
    }

    fn groups(&self, s: &Shell, _tab: usize) -> Vec<Group> {
        let v = s.view;
        vec![
            group(
                "Scene",
                vec![tool(
                    super::super::visual_style::BUTTON,
                    "lines",
                    Some(Action::VisualStyle(v.style)),
                    false,
                )],
                super::super::visual_style::ground_tools(&s.settings),
            ),
            group(
                "Camera",
                vec![tool(
                    "Perspective",
                    "eye",
                    Some(Action::Parallel(!v.parallel)),
                    !v.parallel,
                )],
                vec![
                    tool("Fit All", "fit", Some(Action::FitAll), false),
                    tool("Home", "home", Some(Action::Home), false),
                ],
            ),
        ]
    }
}

/// A drawing is a document of its own: its tile makes one of the open model (File > New
/// Drawing), in a tab of the strip.
struct Drawing;

impl Environment for Drawing {
    fn id(&self) -> EnvId {
        EnvId::Drawing
    }

    fn available(&self, s: &Shell) -> Result<(), String> {
        match &s.fg {
            None => Err("open a part or an assembly first".into()),
            Some(f) if f.path.is_empty() => Err("save the part first: a drawing names its file".into()),
            Some(_) => Ok(()),
        }
    }

    fn enter(&self, s: &mut Shell) -> Result<Entered, String> {
        s.out.push(Action::NewDrawing);
        Ok(Entered::Opened)
    }

    fn tabs(&self) -> &'static [(usize, &'static str)] {
        &[(0, "Drawing")]
    }

    fn groups(&self, _s: &Shell, _tab: usize) -> Vec<Group> {
        vec![]
    }
}

/// An environment whose work is not on main yet: its tile drawn off, saying why. Its adapter
/// replaces this when its branch lands (research/ui/environments.md §5).
struct Pending(EnvId, &'static str);

impl Environment for Pending {
    fn id(&self) -> EnvId {
        self.0
    }

    fn available(&self, _s: &Shell) -> Result<(), String> {
        Err(self.1.into())
    }

    fn enter(&self, _s: &mut Shell) -> Result<Entered, String> {
        Err(self.1.into())
    }

    fn tabs(&self) -> &'static [(usize, &'static str)] {
        match self.0 {
            EnvId::Manufacture => &[(11, "Manufacture")],
            EnvId::MachineBuilder => &[(12, "Machine Builder")],
            EnvId::Robot => &[(13, "Robot")],
            _ => &[(14, "Simulation")],
        }
    }

    fn groups(&self, _s: &Shell, _tab: usize) -> Vec<Group> {
        vec![]
    }
}
