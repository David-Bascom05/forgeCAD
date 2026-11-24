//! The environments: those on main (Thermoform, Flat Pattern, Bend Simulation, Render Studio,
//! Drawing) and the places kept for those still on their branches (Manufacture, Machine Builder,
//! Robot) or not begun (Simulation), drawn off with the reason.

use super::super::ribbon::{Group, Tool};
use super::super::sheet::SheetCmd;
use super::super::{Action, FLAT_TAB, Shell, THERMO_TAB};
use super::{Entered, EnvId, Environment, RENDER_TAB};

pub(super) fn of(id: EnvId) -> &'static dyn Environment {
    match id {
        EnvId::Thermoform => &Thermoform,
        EnvId::FlatPattern => &FlatPattern,
        EnvId::BendSim => &super::bendsim::BendSim,
        EnvId::RenderStudio => &RenderStudio,
        EnvId::Drawing => &Drawing,
        EnvId::Manufacture => &super::workspaces::Manufacture,
        EnvId::MachineBuilder => &super::workspaces::MachineBuilder,
        EnvId::Robot => &Pending(EnvId::Robot, ROBOT),
        EnvId::Simulation => &Pending(EnvId::Simulation, SIMULATION),
    }
}

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
pub(super) fn sheet_part(s: &Shell) -> Result<&super::super::fg::FgPanel, String> {
    let f = part(s)?;
    if f.sheets.is_empty() {
        return Err("make the part sheet metal first (3D Model > Convert to Sheet Metal, then Face)".into());
    }
    Ok(f)
}

struct Thermoform;

impl Environment for Thermoform {
    /// It works on the part: the part's tree stays.
    fn keeps_tree(&self) -> bool {
        true
    }

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

pub(super) struct FlatPattern;

impl Environment for FlatPattern {
    /// It works on the part: the part's tree stays.
    fn keeps_tree(&self) -> bool {
        true
    }

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

/// The look of the picture: style, ground, shadows, camera. Its content (materials, lights,
/// environments, render output) is the Render Studio worker's.
struct RenderStudio;

impl Environment for RenderStudio {
    /// It works on the part: the part's tree stays.
    fn keeps_tree(&self) -> bool {
        true
    }

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
            EnvId::Robot => &[(13, "Robot")],
            _ => &[(14, "Simulation")],
        }
    }

    fn groups(&self, _s: &Shell, _tab: usize) -> Vec<Group> {
        vec![]
    }
}
