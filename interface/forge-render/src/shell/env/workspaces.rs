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

    /// The properties column: the op or setup being edited (its sheet, CAM-UI's dialogs docked)
    /// over the simulation (CAM-SIM's panel, its Simulation section). None while there is
    /// neither: the view keeps the width.
    fn properties_width(&self, s: &Shell) -> Option<f32> {
        let w = s.cam.as_ref()?;
        let sheet = w.op_dlg.is_some() || w.setup_dlg.is_some();
        (sheet || s.camsim.is_some()).then_some(super::super::cam::dock_w(w.screen_w))
    }

    /// Wide (1280 px and over): the sheet on top (80 % of the height while one is open), the
    /// simulation under it. Narrow: one or the other, as tabs at the column's top, the sheet's
    /// chosen while it is open.
    fn properties(&self, s: &mut Shell, ui: &mut Ui, r: Rect) {
        let Some(w) = s.cam.as_ref() else { return };
        let sheet = w.op_dlg.is_some() || w.setup_dlg.is_some();
        let sim = s.camsim.is_some();
        let narrow = super::super::cam::narrow(w.screen_w);
        let t = s.theme;
        let mut dock = None;
        let mut sim_rect = None;
        let mut hidden = false;
        if sheet && sim && narrow {
            // the tabs: the sheet's name, Simulation
            let title = w
                .op_dlg
                .as_ref()
                .map(|d| format!("{} · {}", d.name, super::super::cam::schema::spec(&d.func).map_or("", |sp| sp.label)))
                .or_else(|| w.setup_dlg.as_ref().map(|d| format!("{} · Setup", d.name)))
                .unwrap_or_default();
            let mut sim_tab = w.sim_tab;
            let bar = Rect::from_min_max(r.min, egui::pos2(r.right(), r.top() + 32.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(bar.shrink2(egui::vec2(8.0, 4.0))), |ui| {
                ui.horizontal(|ui| {
                    let a = ui.selectable_label(!sim_tab, title);
                    let b = ui.selectable_label(sim_tab, "Simulation");
                    s.places.push(("cam:dock:sheet".into(), a.rect));
                    s.places.push(("cam:dock:sim".into(), b.rect));
                    if a.clicked() {
                        sim_tab = false;
                    }
                    if b.clicked() {
                        sim_tab = true;
                    }
                });
            });
            ui.painter().hline(r.x_range(), bar.bottom(), egui::Stroke::new(1.0, t.line));
            let rest = Rect::from_min_max(egui::pos2(r.left(), bar.bottom() + 1.0), r.max);
            if sim_tab {
                sim_rect = Some(rest.shrink2(egui::vec2(8.0, 4.0)));
                hidden = true;
            } else {
                dock = Some(rest);
            }
            if let Some(w) = &mut s.cam {
                w.sim_tab = sim_tab;
            }
        } else {
            let split = if sheet && sim { r.top() + r.height() * 0.8 } else if sheet { r.bottom() } else { r.top() };
            if sheet {
                dock = Some(Rect::from_min_max(r.min, egui::pos2(r.right(), split)));
            }
            if sim {
                if sheet {
                    ui.painter().hline(r.x_range(), split, egui::Stroke::new(1.0, t.line));
                }
                sim_rect = Some(Rect::from_min_max(egui::pos2(r.left() + 8.0, split + 6.0), egui::pos2(r.right() - 8.0, r.bottom())));
            }
        }
        if let Some(w) = &mut s.cam {
            w.dock = dock;
            w.sheet_hidden = hidden;
            if !sheet {
                // a new sheet opens on its own tab
                w.sim_tab = false;
            }
        }
        if let Some(rest) = sim_rect {
            s.camsim_properties(ui, rest);
        } else if hidden_sim(sheet, sim, narrow) && let Some(st) = s.camsim.as_deref_mut() {
            // behind its tab: hosted all the same (it does not float over the view meanwhile)
            st.hosted = true;
        }
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

/// A narrow column showing the sheet's tab: the simulation is behind its own tab.
fn hidden_sim(sheet: bool, sim: bool, narrow: bool) -> bool {
    sheet && sim && narrow
}
