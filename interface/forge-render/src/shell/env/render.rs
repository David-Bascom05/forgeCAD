//! The Render Studio environment's wrapper: when it may be entered, into and out of it, its tab.
//! Its content — ribbon, browser, what it writes over the view, commands — is
//! `shell/studio_env.rs` (RENDER-STUDIO's); the picture is the GUI's (`gui_studio.rs`).

use super::super::ribbon::Group;
use super::super::{Shell, studio_env as content};
use super::{Entered, EnvId, Environment, RENDER_TAB};
use egui::{Rect, Ui};

pub(super) struct RenderStudio;

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

    fn enter(&self, s: &mut Shell) -> Result<Entered, String> {
        s.studio.samples = 0;
        Ok(Entered::Mode)
    }

    fn tabs(&self) -> &'static [(usize, &'static str)] {
        &[(RENDER_TAB, "Render Studio")]
    }

    fn groups(&self, s: &Shell, _tab: usize) -> Vec<Group> {
        content::groups(s)
    }

    fn browser(&self, s: &mut Shell, ui: &mut Ui, r: Rect) -> bool {
        content::browser(s, ui, r)
    }

    fn draw(&self, s: &mut Shell, ui: &mut Ui, view: Rect) -> bool {
        content::draw(s, ui, view)
    }

    fn command(&self, s: &mut Shell, c: u16) {
        content::command(s, c);
    }
}
