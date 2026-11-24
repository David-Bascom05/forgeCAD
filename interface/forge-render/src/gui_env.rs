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
        if self.shell.enter_env(id).is_ok()
            && let Some(v) = &mut self.view
        {
            v.nav.selected.clear();
            v.clicked.clear();
            v.dirty = true;
        }
    }

    /// Out of the environment in.
    pub fn env_exit(&mut self) {
        if self.shell.env == Some(EnvId::FlatPattern) {
            return self.sheet_cmd(SheetCmd::Folded);
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
        if self.loading() || self.fg.as_ref().is_none_or(|s| s.done != s.seq) {
            return;
        }
        self.env_asked = None;
        if let Err(why) = self.env_named(&name) {
            self.shell.status = format!("--env {name}: {why}");
        }
    }
}
