//! The Bend Simulation environment's wrapper: when it may be entered, into and out of it, its
//! tab. Its content — ribbon, browser, view, commands — is `shell/bendsim_env.rs` (BEND-SIM's);
//! this file only delegates.

use super::super::ribbon::Group;
use super::super::{Shell, bendsim_env as content};
use super::builtin::{FlatPattern, sheet_part};
use super::{BENDSIM_TAB, Entered, EnvId, Environment, Timeline};
use egui::{Rect, Ui};

pub(super) struct BendSim;

/// The bend program through time, for the recorder: every step's motion in order.
struct BendTimeline;

impl Timeline for BendTimeline {
    fn duration(&self, s: &Shell) -> f64 {
        content::duration(s)
    }

    fn label_at(&self, s: &Shell, t: f64) -> Option<String> {
        content::label_at(s, t)
    }

    fn seek(&self, s: &mut Shell, t: f64) {
        content::seek(s, t);
    }
}

impl Environment for BendSim {
    fn id(&self) -> EnvId {
        EnvId::BendSim
    }

    fn available(&self, s: &Shell) -> Result<(), String> {
        sheet_part(s).map(|_| ())
    }

    fn enter(&self, s: &mut Shell) -> Result<Entered, String> {
        // one environment at a time: the flat folded first
        if s.sheet.flat_env {
            FlatPattern.exit(s);
        }
        content::enter(s)?;
        Ok(Entered::Mode)
    }

    fn exit(&self, s: &mut Shell) {
        content::exit(s);
    }

    fn tabs(&self) -> &'static [(usize, &'static str)] {
        &[(BENDSIM_TAB, "Bend Simulation")]
    }

    fn groups(&self, s: &Shell, _tab: usize) -> Vec<Group> {
        content::groups(s)
    }

    fn browser(&self, s: &mut Shell, ui: &mut Ui, r: Rect) -> bool {
        content::browser(s, ui, r)
    }

    fn panels(&self, s: &mut Shell, ctx: &egui::Context) {
        content::panels(s, ctx);
    }

    fn status(&self, s: &Shell) -> Option<String> {
        content::status(s)
    }

    fn timeline(&self) -> Option<&dyn Timeline> {
        Some(&BendTimeline)
    }

    fn properties_width(&self, s: &Shell) -> Option<f32> {
        content::dock_width(s)
    }

    fn properties(&self, s: &mut Shell, ui: &mut Ui, r: Rect) {
        content::dock(s, ui, r);
    }

    fn draw(&self, s: &mut Shell, ui: &mut Ui, view: Rect) -> bool {
        content::draw(s, ui, view)
    }

    fn command(&self, s: &mut Shell, c: u16) {
        content::command(s, c);
    }
}
