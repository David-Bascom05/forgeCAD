//! The environments over workspaces that live in their own modules: Manufacture (`shell/cam`,
//! CAM-UI's, with CAM-SIM's simulation) and the Machine Builder (`shell/machine.rs`). Their state
//! is theirs (`Shell::cam`, `Shell::machine`); the environment follows it (`Shell::env_sync`),
//! so opening a machining document is entering Manufacture and the Manage button still opens the
//! Machine Builder. Here: the tab, the ribbon and the browser, delegated.

use super::super::ribbon::{Group, Tool};
use super::super::{CAM_TAB, Shell};
use super::{Entered, EnvId, Environment, MACHINE_TAB};
use egui::{Rect, Ui};

pub(super) struct Manufacture;

impl Environment for Manufacture {
    fn id(&self) -> EnvId {
        EnvId::Manufacture
    }

    /// A machining document is a document of its own (its part in the view): open one and the
    /// window is in Manufacture.
    fn available(&self, s: &Shell) -> Result<(), String> {
        if s.cam.is_some() {
            Ok(())
        } else {
            Err("open the part's machining document (File > Open, a .fg that starts `machining`)".into())
        }
    }

    fn enter(&self, _s: &mut Shell) -> Result<Entered, String> {
        Ok(Entered::Mode)
    }

    fn tabs(&self) -> &'static [(usize, &'static str)] {
        &[(CAM_TAB, "Manufacture")]
    }

    fn groups(&self, s: &Shell, _tab: usize) -> Vec<Group> {
        super::super::cam::groups(s)
            .into_iter()
            .map(|(name, big, small)| {
                let tool = |(name, icon, action, on)| Tool {
                    name,
                    icon,
                    action,
                    on,
                };
                Group {
                    name,
                    big: big.into_iter().map(tool).collect(),
                    small: small.into_iter().map(tool).collect(),
                    icons: vec![],
                }
            })
            .collect()
    }

    /// The machining document is the window's document: it is left by its tab's ×, or another
    /// document's tab, not by a Finish that would drop it.
    fn finish_button(&self) -> bool {
        false
    }

    fn browser(&self, s: &mut Shell, ui: &mut Ui, r: Rect) -> bool {
        s.cam_side(ui, r);
        true
    }

    fn status(&self, s: &Shell) -> Option<String> {
        s.cam.as_ref().map(super::super::cam::status_line)
    }
}

pub(super) struct MachineBuilder;

impl Environment for MachineBuilder {
    fn id(&self) -> EnvId {
        EnvId::MachineBuilder
    }

    fn available(&self, _s: &Shell) -> Result<(), String> {
        Ok(())
    }

    /// Opened by the host (`Gui::env_enter`: it takes the view for the machine's scene).
    fn enter(&self, s: &mut Shell) -> Result<Entered, String> {
        if s.machine.is_some() {
            Ok(Entered::Mode)
        } else {
            Err("the Machine Builder opens from the host".into())
        }
    }

    fn tabs(&self) -> &'static [(usize, &'static str)] {
        &[(MACHINE_TAB, "Machine Builder")]
    }

    /// The builder's own groups (`machine::groups`: library, check, motion), then Exit.
    fn groups(&self, s: &Shell, _tab: usize) -> Vec<Group> {
        super::super::machine::groups(s)
    }

    fn still(&self, s: &Shell) -> bool {
        s.machine.is_some()
    }
}
