//! Environments' host side (shell/env): into and out of one with the view's selection dropped,
//! and an environment asked by name (a deep link, `--env`, the agent) entered once the document
//! it is for has built.

use super::Gui;
use crate::shell::env::EnvId;
use crate::shell::sheet::SheetCmd;

impl Gui {
    /// Into an environment: the view's selection dropped, as a dialog drops it.
    pub fn env_enter(&mut self, id: EnvId) {
        if id == EnvId::FlatPattern {
            return self.sheet_cmd(SheetCmd::FlatPattern);
        }
        // the Machine Builder takes the view for its machine's scene: the host opens it
        if id == EnvId::MachineBuilder && self.shell.machine.is_none() {
            if self.shell.env.is_some() {
                self.env_exit();
            }
            self.machine_toggle();
            self.shell.env_sync();
            return;
        }
        if self.shell.enter_env(id).is_ok()
            && let Some(v) = &mut self.view
        {
            v.nav.selected.clear();
            v.clicked.clear();
            v.dirty = true;
            self.env_frame_view();
        }
    }

    /// The subject framed from the 3/4 view as an environment opens (docs/ux/STANDARD.md §2):
    /// centred in the view, the projection kept as it was.
    fn env_frame_view(&mut self) {
        let Some(v) = &mut self.view else { return };
        // the box the scene shows (as Fit All frames it), else its bounding sphere's
        let subject = match v.scene.shown_box() {
            Some((lo, hi)) => crate::frame::Bounds::new(lo, hi),
            None => {
                let (c, r) = v.scene.bounds();
                if r.is_nan() || r <= 0.0 {
                    return;
                }
                crate::frame::Bounds::new([c[0] - r, c[1] - r, c[2] - r], [c[0] + r, c[1] + r, c[2] + r])
            }
        };
        let (w, h) = v.nav.size;
        let view = crate::frame::ViewRect::full(w as f32, h as f32);
        let mut cam = crate::frame::frame(subject, view, crate::frame::Framing::default(), &[]).camera;
        cam.parallel = v.nav.camera.parallel;
        v.nav.go_to(cam);
        v.dirty = true;
    }

    /// Out of the environment in.
    pub fn env_exit(&mut self) {
        if self.shell.env == Some(EnvId::FlatPattern) {
            return self.sheet_cmd(SheetCmd::Folded);
        }
        if self.shell.env == Some(EnvId::MachineBuilder) {
            self.machine_toggle();
            self.shell.env_sync();
            return;
        }
        self.shell.exit_env();
        if let Some(v) = &mut self.view {
            v.dirty = true;
        }
    }

    /// An environment by its key or name ("exit" leaves the one in): what is in after, or why
    /// not. The agent's `environment` tool and `env_asked` come here.
    pub fn env_named(&mut self, name: &str) -> Result<Option<EnvId>, String> {
        match name.trim().to_lowercase().as_str() {
            "exit" | "none" | "design" | "" => {
                self.env_exit();
                return Ok(None);
            }
            _ => {}
        }
        let id = EnvId::parse(name).ok_or_else(|| {
            format!(
                "no environment `{name}`; there are: {}",
                EnvId::ALL.map(EnvId::key).join(", ")
            )
        })?;
        id.available(&self.shell)?;
        self.env_enter(id);
        match self.shell.env {
            Some(e) if e == id => Ok(Some(id)),
            // a drawing: a document of its own
            _ if id == EnvId::Drawing => Ok(None),
            _ => Err(self.shell.status.clone()),
        }
    }

    /// Each frame: the environment asked for, once the document has built (an environment
    /// reads the built part: its sheets, its thermoforms).
    pub(super) fn env_frame(&mut self) {
        let Some(name) = self.env_asked.clone() else {
            return;
        };
        // a document open and built (a file that failed to open leaves none: still waiting)
        if self.loading() || self.shell.fg.is_none() || self.fg.as_ref().is_none_or(|s| s.done != s.seq) {
            return;
        }
        self.env_asked = None;
        if let Err(why) = self.env_named(&name) {
            self.shell.status = format!("--env {name}: {why}");
        }
    }
}
