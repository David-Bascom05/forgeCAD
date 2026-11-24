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
    /// over the column's tabs: the simulation (CAM-SIM's panel), NC Programs, Quality. None
    /// while there is none of them: the view keeps the width.
    fn properties_width(&self, s: &Shell) -> Option<f32> {
        let w = s.cam.as_ref()?;
        let sheet = w.op_dlg.is_some() || w.setup_dlg.is_some();
        (sheet || s.camsim.is_some() || w.programs_open || w.quality_open).then_some(super::super::cam::dock_w(w.screen_w))
    }

    /// Wide (1280 px and over): the sheet on top (80 % of the height while one is open), the
    /// tabs under it (Simulation, NC Programs, Quality). Narrow: the sheet is one of the tabs,
    /// chosen while it is open.
    fn properties(&self, s: &mut Shell, ui: &mut Ui, r: Rect) {
        let Some(w) = s.cam.as_ref() else { return };
        let sheet = w.op_dlg.is_some() || w.setup_dlg.is_some();
        let sim = s.camsim.is_some();
        let narrow = super::super::cam::narrow(w.screen_w);
        let t = s.theme;
        let lower = if w.programs_open {
            Some(Col::Programs)
        } else if w.quality_open {
            Some(Col::Quality)
        } else if sim {
            Some(Col::Sim)
        } else {
            None
        };
        // a sheet with nothing under it (turning, nothing else asked): the whole column
        let Some(lower) = lower else {
            if let Some(w) = &mut s.cam {
                w.dock = Some(r);
                w.sheet_hidden = false;
                w.dock_aux = None;
                w.sim_tab = false;
            }
            return;
        };
        let sheet_tab = sheet && narrow;
        let now = if sheet_tab && !w.sim_tab { Col::Sheet } else { lower };
        let mut tabs: Vec<(Col, String, &str)> = vec![];
        if sheet_tab {
            let title = w
                .op_dlg
                .as_ref()
                .map(|d| format!("{} \u{b7} {}", d.name, super::super::cam::schema::spec(&d.func).map_or("", |sp| sp.label)))
                .or_else(|| w.setup_dlg.as_ref().map(|d| format!("{} \u{b7} Setup", d.name)))
                .unwrap_or_default();
            tabs.push((Col::Sheet, title, "cam:dock:sheet"));
        }
        if sim {
            tabs.push((Col::Sim, "Simulation".into(), "cam:dock:sim"));
        }
        tabs.push((Col::Programs, "NC Programs".into(), "cam:dock:programs"));
        // the toolpath quality is measured on milling only
        if !w.turning() {
            tabs.push((Col::Quality, "Quality".into(), "cam:dock:quality"));
        }
        // wide with a sheet: the sheet on top, the tabs in what is left
        let (dock, area) = if sheet && !narrow {
            let split = r.top() + r.height() * 0.8;
            ui.painter().hline(r.x_range(), split, egui::Stroke::new(1.0, t.line));
            (
                Some(Rect::from_min_max(r.min, egui::pos2(r.right(), split))),
                Rect::from_min_max(egui::pos2(r.left(), split + 1.0), r.max),
            )
        } else {
            (None, r)
        };
        let bar = Rect::from_min_max(area.min, egui::pos2(area.right(), area.top() + 32.0));
        let mut pick = None;
        ui.scope_builder(egui::UiBuilder::new().max_rect(bar.shrink2(egui::vec2(8.0, 4.0))), |ui| {
            ui.horizontal(|ui| {
                for (tab, words, key) in &tabs {
                    let r = ui.selectable_label(*tab == now, words.as_str());
                    s.places.push(((*key).into(), r.rect));
                    if r.clicked() {
                        pick = Some(*tab);
                    }
                }
            });
        });
        ui.painter().hline(area.x_range(), bar.bottom(), egui::Stroke::new(1.0, t.line));
        let rest = Rect::from_min_max(egui::pos2(area.left(), bar.bottom() + 1.0), area.max);
        let now = pick.unwrap_or(now);
        if let Some(w) = &mut s.cam {
            // the tab chosen is what is open (the sheet's tab leaves the lower one as it was)
            if now != Col::Sheet {
                w.programs_open = now == Col::Programs;
                w.quality_open = now == Col::Quality;
            }
            w.sim_tab = now != Col::Sheet;
            if !sheet {
                // a new sheet opens on its own tab
                w.sim_tab = false;
            }
            w.dock = if now == Col::Sheet { Some(rest) } else { dock };
            w.sheet_hidden = sheet_tab && now != Col::Sheet;
            w.dock_aux = matches!(now, Col::Programs | Col::Quality).then_some(rest);
        }
        if now == Col::Sim {
            s.camsim_properties(ui, rest.shrink2(egui::vec2(8.0, 4.0)));
        } else if let Some(st) = s.camsim.as_deref_mut() {
            // behind its tab: hosted all the same (it does not float over the view meanwhile)
            st.hosted = true;
        }
    }
}

/// The Manufacture column's tabs.
#[derive(Clone, Copy, PartialEq)]
enum Col {
    Sheet,
    Sim,
    Programs,
    Quality,
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

    /// Library (new, derive, save), Check and, for machines, Motion; then Finish.
    fn groups(&self, s: &Shell, _tab: usize) -> Vec<Group> {
        super::super::machine::groups(s)
    }

    /// The library (kind switcher, search, records by level and maker) is its browser.
    fn browser(&self, s: &mut Shell, ui: &mut Ui, r: Rect) -> bool {
        s.machine_side(ui, r);
        true
    }

    /// The chosen record's sheet, docked right at full height.
    fn properties_width(&self, s: &Shell) -> Option<f32> {
        s.machine.as_ref().map(|_| 360.0)
    }

    fn properties(&self, s: &mut Shell, ui: &mut Ui, r: Rect) {
        s.machine_props(ui, r);
    }

    fn status(&self, s: &Shell) -> Option<String> {
        s.machine.as_ref().map(super::super::machine::status_line)
    }

    fn still(&self, s: &Shell) -> bool {
        s.machine.is_some()
    }
}
