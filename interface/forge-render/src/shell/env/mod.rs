//! Environments (research/ui/environments.md): a mode of the window over the active document for
//! a job that is not modelling it — manufacture, forming, simulation, rendering. One at a time;
//! entered from the Environments tab (or a command, a deep link, an agent tool) with the document
//! as its context; it replaces the ribbon's tabs with its own (Inspect, View and Manage stay), may
//! own the browser and draw over the view, and is left by its Finish. Which one is active is the
//! document's (`docs.rs`), so a document tab comes back in the environment it was left in.
//!
//! Each environment is a zero-sized `impl Environment` in `ALL`; its state lives where it already
//! did (`fg.thermo`, `sheet`, ...), so the shell borrows whole and nothing aliases. Nothing else in
//! the shell names an environment: `tab_order`, the ribbon and the browser ask this module.

mod bendsim;
mod builtin;
mod workspaces;

use super::ribbon::{Group, Tool};
use super::{Action, Shell};
use egui::{Rect, Ui};

/// The Environments tab: after Manage, as Inventor has it.
pub const ENV_TAB: usize = 20;
/// The tabs of environments with no design tab of their own (11..19 kept for the in-flight ones).
pub const BENDSIM_TAB: usize = 21;
/// The Machine Builder's (Manufacture's is `CAM_TAB`, 11).
pub const MACHINE_TAB: usize = 12;
pub const RENDER_TAB: usize = 22;

/// Every environment, known or still to land.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EnvId {
    Manufacture,
    MachineBuilder,
    Thermoform,
    FlatPattern,
    BendSim,
    Robot,
    RenderStudio,
    Drawing,
    Simulation,
}

impl EnvId {
    /// In the Environments tab's order.
    pub const ALL: [EnvId; 9] = [
        EnvId::Manufacture,
        EnvId::MachineBuilder,
        EnvId::Thermoform,
        EnvId::FlatPattern,
        EnvId::BendSim,
        EnvId::Robot,
        EnvId::RenderStudio,
        EnvId::Drawing,
        EnvId::Simulation,
    ];

    /// Its name, as its tile and its Finish say it.
    pub fn name(self) -> &'static str {
        match self {
            EnvId::Manufacture => "Manufacture",
            EnvId::MachineBuilder => "Machine Builder",
            EnvId::Thermoform => "Thermoform",
            EnvId::FlatPattern => "Flat Pattern",
            EnvId::BendSim => "Bend Simulation",
            EnvId::Robot => "Robot",
            EnvId::RenderStudio => "Render Studio",
            EnvId::Drawing => "Drawing",
            EnvId::Simulation => "Simulation",
        }
    }

    /// Its key in deep links (`?env=`), `--env` and the agent's `environment` tool.
    pub fn key(self) -> &'static str {
        match self {
            EnvId::Manufacture => "manufacture",
            EnvId::MachineBuilder => "machine-builder",
            EnvId::Thermoform => "thermoform",
            EnvId::FlatPattern => "flat-pattern",
            EnvId::BendSim => "bendsim",
            EnvId::Robot => "robot",
            EnvId::RenderStudio => "render",
            EnvId::Drawing => "drawing",
            EnvId::Simulation => "simulation",
        }
    }

    /// From a key or a name, any case, `_`, `-` or a space alike.
    pub fn parse(s: &str) -> Option<EnvId> {
        let norm = |x: &str| {
            x.chars()
                .filter(|c| !matches!(c, ' ' | '-' | '_'))
                .flat_map(char::to_lowercase)
                .collect::<String>()
        };
        let want = norm(s);
        let alias = match want.as_str() {
            "cam" => Some(EnvId::Manufacture),
            "urdf" => Some(EnvId::Robot),
            "thermoforming" => Some(EnvId::Thermoform),
            "flat" => Some(EnvId::FlatPattern),
            "bendsimulation" => Some(EnvId::BendSim),
            "renderstudio" => Some(EnvId::RenderStudio),
            _ => None,
        };
        alias.or_else(|| {
            EnvId::ALL
                .into_iter()
                .find(|e| norm(e.key()) == want || norm(e.name()) == want)
        })
    }

    pub fn icon(self) -> &'static str {
        match self {
            EnvId::Manufacture => "hole",
            EnvId::MachineBuilder => "gear",
            EnvId::Thermoform => "thicken",
            EnvId::FlatPattern => "flatpattern",
            EnvId::BendSim => "bendtable",
            EnvId::Robot => "jrevolute",
            EnvId::RenderStudio => "sun",
            EnvId::Drawing => "file",
            EnvId::Simulation => "measure",
        }
    }

    /// The Environments tab's group it sits in.
    fn group(self) -> &'static str {
        match self {
            EnvId::Manufacture | EnvId::MachineBuilder => "Manufacture",
            EnvId::Thermoform | EnvId::FlatPattern | EnvId::BendSim => "Forming",
            EnvId::Robot => "Robotics",
            EnvId::RenderStudio => "Visualize",
            EnvId::Drawing => "Document",
            EnvId::Simulation => "Analysis",
        }
    }

    /// May it be entered for what is open? Err: why not.
    pub fn available(self, s: &Shell) -> Result<(), String> {
        self.imp().available(s)
    }

    pub(in crate::shell) fn imp(self) -> &'static dyn Environment {
        builtin::of(self)
    }
}

/// Entering: the window is now in the environment, or it opened a document of its own instead
/// (Drawing: a drawing is a document, with its own tab in the strip).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entered {
    Mode,
    Opened,
}

/// What each environment does. Zero-sized: what it holds is the shell's.
pub(in crate::shell) trait Environment: Sync {
    fn id(&self) -> EnvId;
    /// May it be entered for what is open? Err: why not, in words (the tile's tooltip, the
    /// agent's error).
    fn available(&self, s: &Shell) -> Result<(), String>;
    /// Into it, the active document and its selection as context. Its state, if it was left
    /// before, is taken up where it was.
    fn enter(&self, s: &mut Shell) -> Result<Entered, String>;
    /// Out of it (its Finish): its state is kept for coming back.
    fn exit(&self, _s: &mut Shell) {}
    /// Its ribbon tabs: (tab index, the tab's word). Inspect, View and Manage follow them.
    fn tabs(&self) -> &'static [(usize, &'static str)];
    /// The groups of one of its tabs.
    fn groups(&self, s: &Shell, tab: usize) -> Vec<Group>;
    /// A Finish button of its own at the ribbon's end (false: one of its tools is its way out).
    fn finish_button(&self) -> bool {
        true
    }
    /// Its browser in the left panel; false: none of its own (see `keeps_tree`).
    fn browser(&self, _s: &mut Shell, _ui: &mut Ui, _r: Rect) -> bool {
        false
    }
    /// With no browser of its own, the document's feature tree stays in the left panel. Only an
    /// environment that works on the part itself opts in (Thermoform, Flat Pattern, Render
    /// Studio); by default an environment owns the window and the part's tree goes.
    fn keeps_tree(&self) -> bool {
        false
    }
    /// Its line in the status bar. None: the shell's own line if it keeps the tree, else its
    /// name (or what it last said, a line starting with its name).
    fn status(&self, _s: &Shell) -> Option<String> {
        None
    }
    /// Its properties panel, docked at the window's right at full height (the view gives up the
    /// width): the width, or None for none. Use this, not floating windows.
    fn properties_width(&self, _s: &Shell) -> Option<f32> {
        None
    }
    fn properties(&self, _s: &mut Shell, _ui: &mut Ui, _r: Rect) {}
    /// Its windows and panels, once a frame.
    fn panels(&self, _s: &mut Shell, _ctx: &egui::Context) {}
    /// What it draws over the view; true: it takes the view's pointer too (hit-testing is its).
    fn draw(&self, _s: &mut Shell, _ui: &mut Ui, _view: Rect) -> bool {
        false
    }
    /// One of its own ribbon commands (`Action::Env`).
    fn command(&self, _s: &mut Shell, _c: u16) {}
    /// Kept up with the document each frame: false if it can no longer be in it (left then).
    fn still(&self, s: &Shell) -> bool {
        self.available(s).is_ok()
    }
}

impl Shell {
    /// Into environment `id`, out of any other first. Err: why it can't be entered (the
    /// status says so too).
    pub fn enter_env(&mut self, id: EnvId) -> Result<Entered, String> {
        let e = id.imp();
        debug_assert_eq!(e.id(), id);
        if let Err(why) = e.available(self) {
            self.status = format!("{}: {why}", id.name());
            return Err(why);
        }
        if self.env == Some(id) {
            return Ok(Entered::Mode);
        }
        if let Some(was) = self.env {
            self.exit_env();
            if self.env == Some(was) {
                return Err(format!("{} could not be left", was.name()));
            }
        }
        match e.enter(self) {
            Ok(Entered::Mode) => {
                self.env = Some(id);
                self.tab = e.tabs()[0].0;
                // the status bar keeps the document's line (or the environment's own: env_status)
                Ok(Entered::Mode)
            }
            Ok(Entered::Opened) => Ok(Entered::Opened),
            Err(why) => {
                self.status = format!("{}: {why}", id.name());
                Err(why)
            }
        }
    }

    /// Out of the active environment, back to the design's first tab.
    pub fn exit_env(&mut self) {
        let Some(id) = self.env else { return };
        id.imp().exit(self);
        self.env = None;
        self.tab = self.tab_order()[0];
    }

    pub fn env_command(&mut self, id: EnvId, c: u16) {
        if self.env == Some(id) {
            id.imp().command(self, c);
        }
    }

    /// Each frame, first: the environment kept up with the document (another way into or out
    /// of it — the Flat Pattern's own buttons — or the document no longer fit for it).
    pub(crate) fn env_sync(&mut self) {
        // the workspaces with state of their own: open is in, closed is out
        for (open, id) in [
            (self.machine.is_some(), EnvId::MachineBuilder),
            (self.cam.is_some(), EnvId::Manufacture),
        ] {
            if open && self.env != Some(id) && (self.env.is_none() || id == EnvId::MachineBuilder) {
                self.env = Some(id);
                self.tab = id.imp().tabs()[0].0;
            } else if !open && self.env == Some(id) {
                self.env = None;
                self.tab = self.tab_order()[0];
            }
        }
        let flat = self.fg.is_some() && self.sheet.flat_env;
        if flat && self.env != Some(EnvId::FlatPattern) {
            self.env = Some(EnvId::FlatPattern);
        } else if !flat && self.env == Some(EnvId::FlatPattern) {
            self.env = None;
            if !self.tab_order().contains(&self.tab) {
                self.tab = self.tab_order()[0];
            }
        }
        if let Some(id) = self.env
            && !id.imp().still(self)
        {
            self.exit_env();
        }
        if !self.tab_order().contains(&self.tab) && self.tab != 1 {
            self.tab = self.tab_order()[0];
        }
    }

    /// The tab's word, if an environment (or the Environments tab) has it.
    pub(super) fn env_tab_name(&self, tab: usize) -> Option<&'static str> {
        if tab == ENV_TAB {
            return Some("Environments");
        }
        EnvId::ALL
            .into_iter()
            .flat_map(|e| e.imp().tabs())
            .find(|t| t.0 == tab)
            .map(|t| t.1)
    }

    /// The tabs in the environment: its own, then Inspect, View, Manage.
    pub(super) fn env_tabs(&self) -> Option<Vec<usize>> {
        let id = self.env?;
        let mut v: Vec<usize> = id.imp().tabs().iter().map(|t| t.0).collect();
        v.extend([2, 3, 4]);
        Some(v)
    }

    /// The browser, if the environment has its own.
    /// The left panel in an environment: its browser, else the part's tree if it keeps it, else
    /// the environment's name and the way out (the part's tree is not its).
    pub(super) fn env_browser(&mut self, ui: &mut Ui, r: Rect) -> bool {
        let Some(id) = self.env else { return false };
        let e = id.imp();
        if e.browser(self, ui, r) {
            return true;
        }
        if e.keeps_tree() {
            return false;
        }
        let t = self.theme;
        let p = ui.painter();
        p.text(
            egui::pos2(r.left() + super::theme::S3, r.top() + super::theme::S3),
            egui::Align2::LEFT_TOP,
            id.name(),
            super::theme::semi(13.0),
            t.ink,
        );
        let why = if e.finish_button() {
            format!(
                "Its tools are on the ribbon. {} goes back to the design.",
                finish_word(id)
            )
        } else {
            "Its tools are on the ribbon.".to_string()
        };
        let g = p.layout(
            why,
            super::theme::small(),
            t.ink2,
            r.width() - 2.0 * super::theme::S3,
        );
        p.galley(
            egui::pos2(r.left() + super::theme::S3, r.top() + super::theme::S3 + 22.0),
            g,
            t.ink2,
        );
        self.places.push((format!("env:{}:browser", id.key()), r));
        true
    }

    /// The status bar's line in an environment (None: the shell's own).
    pub(super) fn env_status(&self) -> Option<String> {
        let id = self.env?;
        let e = id.imp();
        if let Some(s) = e.status(self) {
            return Some(s);
        }
        if e.keeps_tree() {
            return None;
        }
        Some(if self.status.starts_with(id.name()) {
            self.status.clone()
        } else {
            id.name().to_string()
        })
    }

    /// The width the environment's docked properties take at the right.
    pub(super) fn env_properties_width(&self) -> f32 {
        self.env
            .and_then(|id| id.imp().properties_width(self))
            .unwrap_or(0.0)
    }

    /// The environment's properties, docked at the right of the view at full height.
    pub(super) fn env_properties(&mut self, ui: &mut Ui, r: Rect) {
        let Some(id) = self.env else { return };
        if r.width() <= 0.0 {
            return;
        }
        let t = self.theme;
        ui.painter().rect_filled(r, 0.0, t.surface);
        ui.painter()
            .vline(r.left(), r.y_range(), egui::Stroke::new(1.0, t.line));
        self.places.push((format!("env:{}:properties", id.key()), r));
        id.imp().properties(self, ui, r);
    }

    pub(super) fn env_panels(&mut self, ctx: &egui::Context) {
        if let Some(id) = self.env {
            id.imp().panels(self, ctx);
        }
    }

    /// Over the view, before the chrome's windows.
    pub(super) fn env_draw(&mut self, ui: &mut Ui) {
        let Some(id) = self.env else { return };
        let v = self.view_rect;
        if id.imp().draw(self, ui, v) {
            self.view_blocks.push(v);
            self.view_owned = true;
        }
    }

    /// Why an environment's tile is off, for its tooltip.
    pub(super) fn env_why_off(&self, name: &str) -> Option<String> {
        let id = EnvId::ALL.into_iter().find(|e| e.name() == name)?;
        id.imp().available(self).err()
    }
}

/// The ribbon's groups when an environment or the Environments tab has them.
pub(super) fn groups(s: &Shell) -> Option<Vec<Group>> {
    if s.tab == ENV_TAB && s.env.is_none() {
        return Some(env_tab(s));
    }
    let id = s.env?;
    let e = id.imp();
    if !e.tabs().iter().any(|t| t.0 == s.tab) {
        return None;
    }
    let mut gs = e.groups(s, s.tab);
    if e.finish_button() {
        gs.push(Group {
            name: "Exit",
            big: vec![Tool {
                name: finish_word(id),
                icon: "check",
                action: Some(Action::ExitEnv),
                on: false,
            }],
            small: vec![],
            icons: vec![],
        });
    }
    Some(gs)
}

/// "Finish <name>", a 'static word per environment.
fn finish_word(id: EnvId) -> &'static str {
    match id {
        EnvId::Manufacture => "Finish Manufacture",
        EnvId::MachineBuilder => "Finish Machine Builder",
        EnvId::Thermoform => "Finish Thermoform",
        EnvId::FlatPattern => "Finish Flat Pattern",
        EnvId::BendSim => "Finish Bend Simulation",
        EnvId::Robot => "Finish Robot",
        EnvId::RenderStudio => "Finish Render Studio",
        EnvId::Drawing => "Finish Drawing",
        EnvId::Simulation => "Finish Simulation",
    }
}

/// The Environments tab: a tile per environment, in groups; one that can't be entered now is
/// drawn off, its tooltip saying why.
fn env_tab(s: &Shell) -> Vec<Group> {
    let mut out: Vec<Group> = vec![];
    for id in EnvId::ALL {
        let tool = Tool {
            name: id.name(),
            icon: id.icon(),
            action: id.imp().available(s).is_ok().then_some(Action::EnterEnv(id)),
            on: s.env == Some(id),
        };
        match out.last_mut() {
            Some(g) if g.name == id.group() => g.big.push(tool),
            _ => out.push(Group {
                name: id.group(),
                big: vec![tool],
                small: vec![],
                icons: vec![],
            }),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_and_names_parse_back() {
        for id in EnvId::ALL {
            assert_eq!(EnvId::parse(id.key()), Some(id));
            assert_eq!(EnvId::parse(id.name()), Some(id));
            assert_eq!(id.imp().id(), id);
            assert!(!id.imp().tabs().is_empty());
        }
        assert_eq!(EnvId::parse("CAM"), Some(EnvId::Manufacture));
        assert_eq!(EnvId::parse("bend_sim"), Some(EnvId::BendSim));
        assert_eq!(EnvId::parse("nope"), None);
    }

    #[test]
    fn nothing_open_every_tile_is_off_and_says_why() {
        let s = Shell::default();
        assert!(s.tab_order().contains(&ENV_TAB));
        let t = Shell {
            tab: ENV_TAB,
            ..Default::default()
        };
        let gs = groups(&t).unwrap();
        let tiles: Vec<_> = gs.iter().flat_map(|g| &g.big).collect();
        assert_eq!(tiles.len(), EnvId::ALL.len());
        // the Machine Builder edits machine files, not the document: on with nothing open
        for tile in tiles.into_iter().filter(|t| t.name != "Machine Builder") {
            assert!(tile.action.is_none(), "{} is on with nothing open", tile.name);
            assert!(t.env_why_off(tile.name).is_some());
        }
        let mut u = Shell::default();
        assert!(u.enter_env(EnvId::Thermoform).is_err());
        assert_eq!(u.env, None);
    }
}
