//! The Robot environment's wrapper: when it may be entered, into and out of it, its tab. Its
//! content — ribbon, browser, sheet, what it draws over the view, commands — is `shell/robot/`
//! (research/export/urdf-editor.md §6); this file only delegates.

use super::super::ribbon::Group;
use super::super::{Action, Shell, robot as content};
use super::{Entered, EnvId, Environment, ROBOT_TAB};
use egui::{Rect, Ui};

pub(super) struct Robot;

impl Environment for Robot {
    fn id(&self) -> EnvId {
        EnvId::Robot
    }

    fn available(&self, s: &Shell) -> Result<(), String> {
        content::available(s)
    }

    fn enter(&self, s: &mut Shell) -> Result<Entered, String> {
        content::enter(s)?;
        Ok(Entered::Mode)
    }

    fn exit(&self, s: &mut Shell) {
        content::exit(s);
    }

    fn tabs(&self) -> &'static [(usize, &'static str)] {
        &[(ROBOT_TAB, "Robot")]
    }

    fn groups(&self, s: &Shell, _tab: usize) -> Vec<Group> {
        content::groups(s, &|c| Action::Env(EnvId::Robot, c))
    }

    fn browser(&self, s: &mut Shell, ui: &mut Ui, r: Rect) -> bool {
        s.robot_browser(ui, r)
    }

    fn status(&self, s: &Shell) -> Option<String> {
        s.robot_status()
    }

    fn properties_width(&self, s: &Shell) -> Option<f32> {
        s.robot_properties_width()
    }

    fn properties(&self, s: &mut Shell, ui: &mut Ui, r: Rect) {
        s.robot_properties(ui, r);
    }

    fn draw(&self, s: &mut Shell, ui: &mut Ui, view: Rect) -> bool {
        s.robot_draw(ui, view)
    }

    fn command(&self, s: &mut Shell, c: u16) {
        content::command(s, c);
    }

    fn still(&self, s: &Shell) -> bool {
        s.robot.is_some()
    }
}
