//! The host's side of the Robot workspace (shell/robot_ui.rs): a robot document opens its
//! source assembly as the document (all of its views and tools as they are) with the robot's
//! panel beside it; the robot's export runs on each solve of the source; and each frame the
//! panel's joint values are put on the parts through the robot's own forward kinematics
//! (`RobotView::place`), so what a robot document overrides (a freeze, a re-root, a joint's
//! axis) shows, which the assembly's solver knows nothing of.

use super::Gui;
use crate::origin::WorldVert;
use crate::shell::robot::{RobotAct, RobotPanel};
use forge_urdf::{self as urdf, Exported, UrdfError};
use std::path::Path;

/// The robot document's export, as the panel shows it: the source built with its structure, the
/// overrides on top, unknown effort and velocity left as placeholders (a view, not a file).
pub fn robot_export(file: &str) -> Result<Result<Exported, UrdfError>, String> {
    let text = forge_dirs::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
    let dir = Path::new(file).parent().map(Path::to_path_buf);
    let r = forge_urdf::robot::read(&text, dir.as_deref())
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

fn f3(p: forge_math::Point3) -> [f32; 3] {
    [p.x as f32, p.y as f32, p.z as f32]
}

fn v3(v: forge_math::Vec3) -> [f32; 3] {
    [v.x as f32, v.y as f32, v.z as f32]
}

impl Gui {
    /// What the robot's overlays depend on: its joint values, selection and switches.
    pub(super) fn robot_key(&self) -> u64 {
        let Some(p) = &self.shell.robot else { return 0 };
        let mut h: u64 = 0x9E37_79B9_7F4A_7C15;
        for (k, v) in &p.q {
            for b in k.bytes() {
                h = (h ^ b as u64).wrapping_mul(0x100_0000_01B3);
            }
            h = (h ^ v.to_bits()).wrapping_mul(0x100_0000_01B3);
        }
        let sel = format!("{:?}{}{}", p.sel, p.show_axes, p.show_com);
        for b in sel.bytes() {
            h = (h ^ b as u64).wrapping_mul(0x100_0000_01B3);
        }
        h
    }

    /// Each joint's frame (a triad) and axis (a line, the selected joint's longer and in the
    /// accent), each link's centre of mass (a dot, seen through the parts).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn robot_overlay(
        &self,
        r: f32,
        cam: &crate::camera::Camera,
        size: (u32, u32),
        px: f32,
        out: &mut Vec<WorldVert>,
        behind: &mut Vec<WorldVert>,
    ) {
        let Some(p) = &self.shell.robot else { return };
        let Some(view) = &p.view else { return };
        if !p.open || (!p.show_axes && !p.show_com) {
            return;
        }
        let frames = view.place(&p.q);
        let mut through = vec![];
        if p.show_axes {
            let accent = self.shell.theme.accent;
            let acc = [
                accent.r() as f32 / 255.0,
                accent.g() as f32 / 255.0,
                accent.b() as f32 / 255.0,
            ];
            for j in &view.joints {
                let Some(pf) = frames.get(&j.parent) else { continue };
                let jf = pf.compose(&j.origin);
                let at = f3(jf.origin);
                let axes = [v3(jf.x().get()), v3(jf.y().get()), v3(jf.z().get())];
                crate::overlay::triad(at, axes, r * 0.05, cam, size, px, &mut through);
                if let Some(a) = j.axis {
                    let d = jf.vec_to_world(forge_math::Vec3::new(a[0], a[1], a[2]));
                    let chosen = p.sel == Some(crate::shell::robot::Sel::Joint(j.name.clone()));
                    let len = if chosen { r * 0.3 } else { r * 0.15 };
                    let (w, c) = if chosen {
                        (px * 4.0, [acc[0], acc[1], acc[2], 1.0])
                    } else {
                        (px * 2.5, [0.98, 0.72, 0.22, 1.0])
                    };
                    // an arrow along the axis, from the joint's origin: the way positive turns
                    // or slides (its name at the tip, `robot_labels`)
                    let (tip, head) = arrow(at, d, len);
                    crate::origin::polyline(&[at, tip], w, c, cam, size, &mut through);
                    crate::origin::polyline(&[head[0], tip, head[1]], w, c, cam, size, &mut through);
                }
            }
        }
        if p.show_com {
            for l in view.links.iter().filter(|l| l.mass.is_some()) {
                let Some(f) = frames.get(&l.name) else { continue };
                let c = f3(f.point_to_world(forge_math::Point3::new(l.com[0], l.com[1], l.com[2])));
                crate::origin::disc(c, px * 6.0, [0.05, 0.06, 0.08, 1.0], cam, size, &mut through);
                crate::origin::disc(c, px * 4.0, [0.95, 0.95, 0.95, 1.0], cam, size, &mut through);
            }
        }
        for w in &through {
            behind.push(WorldVert {
                at: w.at,
                colour: [w.colour[0], w.colour[1], w.colour[2], w.colour[3] * 0.6],
            });
        }
        out.extend(through);
    }

    /// Where each joint's name goes over the view this frame: at its axis arrow's tip, in points
    /// from the view's corner (the shell paints them, `Shell::robot_labels`).
    fn robot_labels(&mut self) {
        let ppp = self.pixels_per_point;
        let (Some(p), Some(v)) = (&mut self.shell.robot, &self.view) else {
            return;
        };
        p.labels.clear();
        let Some(view) = &p.view else { return };
        if !p.open || !p.show_axes {
            return;
        }
        let r = v.scene.bounds().1;
        let frames = view.place(&p.q);
        for j in &view.joints {
            let (Some(pf), Some(a)) = (frames.get(&j.parent), j.axis) else {
                continue;
            };
            let jf = pf.compose(&j.origin);
            let d = jf.vec_to_world(forge_math::Vec3::new(a[0], a[1], a[2]));
            let chosen = p.sel == Some(crate::shell::robot::Sel::Joint(j.name.clone()));
            let len = if chosen { r * 0.3 } else { r * 0.15 };
            let (tip, _) = arrow(f3(jf.origin), d, len);
            if let Some((x, y)) = v.nav.camera.pixel_of(tip, v.size.0, v.size.1) {
                p.labels.push((j.name.clone(), [x / ppp, y / ppp], chosen));
            }
        }
    }

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
        let r = match forge_urdf::robot::read(&text, dir.as_deref()) {
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
        // into the environment: its tab, browser and sheet; the view framed once the parts are placed
        self.shell.tab = crate::shell::ROBOT_TAB;
        true
    }

    fn robot_reload(&mut self, panel: &mut RobotPanel) {
        panel.stale = false;
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
        self.robot_labels();
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
        let want = self.view_px();
        let Some(p) = &mut self.shell.robot else {
            return false;
        };
        // framed on entering (docs/ux/STANDARD.md §2): the robot at its drive pose, in the view
        // its browser and sheet leave, once that view has its size and the parts their places
        if p.open
            && !p.framed
            && !p.changed
            && p.view.is_some()
            && let Some(v) = &mut self.view
            && v.nav.size == want
        {
            p.framed = true;
            // the robot at its drive pose framed by the shared helper (docs/ux/STANDARD.md §2):
            // the view is already what the browser and the sheet leave, so nothing is covered
            match v.scene.shown_box() {
                Some((lo, hi)) => {
                    let (w, h) = (v.nav.size.0 as f32, v.nav.size.1 as f32);
                    let b = crate::frame::Bounds::new(lo, hi);
                    let mut c = crate::frame::frame(
                        b,
                        crate::frame::ViewRect::full(w, h),
                        crate::frame::Framing::default(),
                        &[],
                    )
                    .camera;
                    // the same picture by the user's parallel lens, if theirs is one
                    if v.nav.camera.parallel {
                        c.set_parallel(true, b.radius());
                    }
                    v.nav.cut_to(c);
                }
                None => v.nav.fit_all(&v.scene),
            }
            v.dirty = true;
            return true;
        }
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
pub fn robot_edit(file: &str, e: &forge_urdf::robot::Edit) -> Result<String, String> {
    let before = forge_dirs::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
    let after = forge_urdf::robot::edit(&before, e)?;
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
    let r = match forge_urdf::robot::read(&text, dir.as_deref()) {
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

/// An axis arrow from `at` along unit `d`, `len` long: its tip, and its head's two barbs.
fn arrow(at: [f32; 3], d: forge_math::Vec3, len: f32) -> ([f32; 3], [[f32; 3]; 2]) {
    let d = [d.x as f32, d.y as f32, d.z as f32];
    let tip = [0, 1, 2].map(|i| at[i] + d[i] * len);
    // a direction across the axis: world x, or y where the axis is near x
    let side = if d[0].abs() < 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let k = side[0] * d[0] + side[1] * d[1] + side[2] * d[2];
    let mut n = [0, 1, 2].map(|i| side[i] - d[i] * k);
    let m = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-6);
    n = n.map(|x| x / m);
    let (back, wide) = (len * 0.22, len * 0.1);
    let barb = |s: f32| [0, 1, 2].map(|i| tip[i] - d[i] * back + n[i] * wide * s);
    (tip, [barb(1.0), barb(-1.0)])
}
