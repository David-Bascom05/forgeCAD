//! The host's side of the Robot workspace (shell/robot_ui.rs): a robot document opens its
//! source assembly as the document (all of its views and tools as they are) with the robot's
//! panel beside it; the robot's export runs on each solve of the source; and each frame the
//! panel's joint values are put on the parts through the robot's own forward kinematics
//! (`RobotView::place`), so what a robot document overrides (a freeze, a re-root, a joint's
//! axis) shows, which the assembly's solver knows nothing of.

use super::Gui;
use crate::shell::robot_ui::{RobotAct, RobotPanel};
use forge_lang::urdf::{self, Exported, UrdfError};
use std::path::Path;

/// The robot document's export, as the panel shows it: the source built with its structure, the
/// overrides on top, unknown effort and velocity left as placeholders (a view, not a file).
pub fn robot_export(file: &str) -> Result<Result<Exported, UrdfError>, String> {
    let text = forge_dirs::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
    let dir = Path::new(file).parent().map(Path::to_path_buf);
    let r = forge_lang::robot::read(&text, dir.as_deref())
        .map_err(|es| es.iter().map(ToString::to_string).collect::<Vec<_>>().join("; "))?;
    let name = Path::new(file)
        .file_stem()
        .map_or_else(|| "robot".into(), |s| s.to_string_lossy().into_owned());
    let o = urdf::Options {
        placeholder: true,
        ..urdf::Options::default()
    };
    let e = match &r.source_path {
        None => urdf::export_unlinked(&name, &o, &r),
        Some(sp) => {
            let stext = forge_dirs::fs::read_to_string(sp).map_err(|e| format!("{}: {e}", sp.display()))?;
            let sdir = sp.parent().map(Path::to_path_buf);
            let opts = forge_lang::Options {
                dir: sdir.clone(),
                render: true,
                kin: true,
                overlap: false,
                ..forge_lang::Options::default()
            };
            let run = forge_lang::run(&stext, opts, Default::default()).0;
            urdf::export_robot(&run, &name, sdir.as_deref(), &o, Some(&r))
        }
    };
    Ok(e)
}

impl Gui {
    /// Opens a robot document: its source as the document, its panel beside it.
    pub(super) fn open_robot(&mut self, path: &str) -> bool {
        let text = match forge_dirs::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                self.shell.status = format!("{path}: {e}");
                return false;
            }
        };
        let dir = Path::new(path).parent().map(Path::to_path_buf);
        let r = match forge_lang::robot::read(&text, dir.as_deref()) {
            Ok(r) => r,
            Err(es) => {
                self.shell.status = es.iter().map(ToString::to_string).collect::<Vec<_>>().join("; ");
                return false;
            }
        };
        let mut panel = RobotPanel::new(path);
        panel.source = r.source.clone();
        if let Some(sp) = &r.source_path
            && !self.open_fg(&sp.to_string_lossy())
        {
            return false;
        }
        self.robot_reload(&mut panel);
        self.shell.robot = Some(panel);
        true
    }

    fn robot_reload(&mut self, panel: &mut RobotPanel) {
        let t = crate::viewer::now_ms();
        match robot_export(&panel.file) {
            Ok(r) => panel.load(r),
            Err(why) => panel.error = Some(("Robot".into(), why, vec![])),
        }
        eprintln!("forge-app: robot export in {:.0} ms", crate::viewer::now_ms() - t);
    }

    /// The source solved again: the robot's export again (next frame), the parts placed again.
    pub(super) fn robot_solved(robot: &mut Option<RobotPanel>) {
        if let Some(p) = robot {
            p.stale = true;
            p.changed = true;
        }
    }

    /// The panel's asks: edits of the robot document (each undoable), a limit written in the
    /// source assembly, the URDF written out.
    fn robot_acts(&mut self) {
        let Some(mut p) = self.shell.robot.take() else {
            return;
        };
        let acts = std::mem::take(&mut p.out);
        let mut reload = false;
        for a in acts {
            match a {
                RobotAct::Edit(e) => match robot_edit(&p.file, &e) {
                    Ok(before) => {
                        p.undo.push(before);
                        p.said = "edited".into();
                        reload = true;
                    }
                    Err(why) => p.said = why,
                },
                RobotAct::Undo => {
                    if let Some(before) = p.undo.pop() {
                        match forge_dirs::fs::write(&p.file, before) {
                            Ok(()) => {
                                p.said = "undone".into();
                                reload = true;
                            }
                            Err(e) => p.said = e.to_string(),
                        }
                    }
                }
                RobotAct::Assembly {
                    joint,
                    arg,
                    text,
                    then,
                } => {
                    let op = forge_lang::error::Op::SetArg {
                        id: joint.clone(),
                        arg,
                        text,
                    };
                    let done = match &mut self.fg {
                        Some(s) => s
                            .apply(op)
                            .map(|_| ())
                            .map_err(|e| format!("{}: {}", e.kind(), e.text())),
                        None => Err("the source is not open".into()),
                    };
                    match done.and_then(|()| self.save_source()) {
                        Ok(()) => match robot_edit(&p.file, &then) {
                            Ok(before) => {
                                p.undo.push(before);
                                p.said = format!("{joint}'s limits written in the assembly");
                                reload = true;
                            }
                            Err(why) => p.said = why,
                        },
                        Err(why) => p.said = why,
                    }
                }
                RobotAct::Export => p.said = robot_write(&p.file),
            }
        }
        if reload {
            self.robot_reload(&mut p);
        }
        self.shell.robot = Some(p);
    }

    /// The source assembly saved, so the robot's export (which reads the file) sees the edit.
    fn save_source(&mut self) -> Result<(), String> {
        match &mut self.fg {
            Some(s) => s.save(),
            None => Ok(()),
        }
    }

    /// Each frame: the parts where the panel's joint values put them. True when the view moved.
    pub(super) fn robot_tick(&mut self) -> bool {
        if self.shell.robot.as_ref().is_some_and(|p| !p.out.is_empty()) {
            self.robot_acts();
        }
        if self.shell.robot.as_ref().is_some_and(|p| p.stale)
            && let Some(mut p) = self.shell.robot.take()
        {
            p.stale = false;
            self.robot_reload(&mut p);
            self.shell.robot = Some(p);
        }
        let Some(p) = &mut self.shell.robot else {
            return false;
        };
        if !p.changed {
            return false;
        }
        let (Some(view), Some((occ, _)), Some(v)) = (&p.view, &self.fg_asm, &mut self.view) else {
            return false;
        };
        let frames = view.place(&p.q);
        let mut moved = 0;
        for (path, f) in view.occurrences(&frames) {
            if let Some(pl) = occ.iter().find(|o| o.path == path).and_then(|o| o.placement)
                && pl < v.scene.placements.len()
            {
                v.scene.set_rows(pl, &crate::fg::rows_of(&f));
                moved += 1;
            }
        }
        if moved > 0 {
            v.dirty = true;
            p.changed = false;
        }
        moved > 0
    }
}

/// An edit of the robot document on disk: the text before it, for Undo.
pub fn robot_edit(file: &str, e: &forge_lang::robot::Edit) -> Result<String, String> {
    let before = forge_dirs::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
    let after = forge_lang::robot::edit(&before, e)?;
    forge_dirs::fs::write(file, &after).map_err(|e| format!("{file}: {e}"))?;
    Ok(before)
}

/// The robot's URDF written beside its document, in NAME_urdf/ (the export's refusal said
/// instead: unknown effort and velocity are not written as placeholders here).
pub fn robot_write(file: &str) -> String {
    let text = match forge_dirs::fs::read_to_string(file) {
        Ok(t) => t,
        Err(e) => return e.to_string(),
    };
    let dir = Path::new(file).parent().map(Path::to_path_buf);
    let r = match forge_lang::robot::read(&text, dir.as_deref()) {
        Ok(r) => r,
        Err(es) => return es.iter().map(ToString::to_string).collect::<Vec<_>>().join("; "),
    };
    let name = Path::new(file)
        .file_stem()
        .map_or_else(|| "robot".into(), |s| s.to_string_lossy().into_owned());
    let o = urdf::Options::default();
    let e = match &r.source_path {
        None => urdf::export_unlinked(&name, &o, &r),
        Some(sp) => {
            let Ok(stext) = forge_dirs::fs::read_to_string(sp) else {
                return format!("{}: cannot read", sp.display());
            };
            let sdir = sp.parent().map(Path::to_path_buf);
            let opts = forge_lang::Options {
                dir: sdir.clone(),
                render: true,
                kin: true,
                overlap: false,
                ..forge_lang::Options::default()
            };
            let run = forge_lang::run(&stext, opts, Default::default()).0;
            urdf::export_robot(&run, &name, sdir.as_deref(), &o, Some(&r))
        }
    };
    let e = match e {
        Ok(e) => e,
        Err(e) => return format!("not exported: {}: {}", e.kind(), e.text()),
    };
    let out = dir.unwrap_or_default().join(format!("{name}_urdf"));
    let mut files = vec![(out.join(format!("{}.urdf", e.name)), e.urdf.into_bytes())];
    files.extend(e.meshes.into_iter().map(|(n, b)| (out.join(n), b)));
    for (p, b) in files {
        if let Some(d) = p.parent()
            && let Err(e) = forge_dirs::fs::create_dir_all(d)
        {
            return e.to_string();
        }
        if let Err(e) = forge_dirs::fs::write(&p, b) {
            return e.to_string();
        }
    }
    format!("wrote {}", out.display())
}
