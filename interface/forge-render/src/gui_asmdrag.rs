//! The host's side of dragging in an assembly (shell/asmdrag.rs): a selecting press on a
//! component taken from the window, the joint values the pointer asks for set straight in the
//! kept solve each frame (`Session::drive`), the whole pose built once the button is let go; and
//! the transient pose's API for the app's agent tools.
//!
//! The agent's tools (one call each, all transient until `keep_position`):
//! - `Gui::drive_joint(joint, value)`: `joint` is a joint's id (`f_door_f`) or a freedom of it
//!   (`f_door_f.turn`); `value` in degrees for a turn, mm for a length; held within the joint's
//!   limits. Only a freedom its own param drives can be driven. Ok: the status line
//!   (`f_door_f.turn = 63°`); Err: why not.
//! - `Gui::return_to_design()`: the file's own pose back.
//! - `Gui::keep_position()`: the pose's values written to the file as SetParam (and moved
//!   instances' `at:`), one undo step.

use super::Gui;
use crate::shell::asmdrag::{self, Grab, Hold, Plan};
use forge_math::{Frame, Point3, Vec3};

fn p3(v: [f32; 3]) -> Point3 {
    Point3::new(v[0] as f64, v[1] as f64, v[2] as f64)
}

fn v3(v: [f32; 3]) -> Vec3 {
    Vec3::new(v[0] as f64, v[1] as f64, v[2] as f64)
}

impl Gui {
    /// Each frame of an assembly: the pose dropped when the file changed under it, a press
    /// on a component taken for a drag, the drag followed, its pose built when let go.
    pub(super) fn asm_drag_frame(&mut self) {
        let Some(s) = &mut self.fg else { return };
        let Some(a) = self.shell.fg.as_mut().and_then(|p| p.asm.as_mut()) else {
            return;
        };
        let p = &mut a.posing;
        // the Drive dialog's slider, moved: its value at once
        if !p.live.is_empty() {
            s.drive(std::mem::take(&mut p.live));
        }
        if !p.is_empty() && p.over.is_none() {
            p.over = Some(s.src.clone());
        }
        if p.over.as_ref().is_some_and(|o| *o != s.src) {
            // an edit, an undo: what's built is the file's
            p.clear();
            s.previewing = false;
            for t in std::mem::take(&mut a.posing.settling) {
                a.moved.remove(&t);
            }
        }
        if a.posing.say && s.done == s.seq {
            a.posing.say = false;
            if !a.posing.is_empty() {
                self.shell.status = format!(
                    "{} · Keep Position writes it, Esc or Return to Design goes back",
                    a.posing.words()
                );
            }
        }
        // a free body's view-only motion stays till its pose is built
        if !a.posing.settling.is_empty() && s.done == s.seq && a.posing.grab.is_none() {
            for t in std::mem::take(&mut a.posing.settling) {
                a.moved.remove(&t);
            }
            self.asm_show();
        }
        let tool = self
            .shell
            .fg
            .as_ref()
            .and_then(|p| p.asm.as_ref())
            .is_some_and(|a| a.tool.is_some());
        let posed = self
            .shell
            .fg
            .as_ref()
            .and_then(|p| p.asm.as_ref())
            .is_some_and(|a| !a.posing.is_empty());
        // Esc is the shell's (one level back at a time): Return to Design there
        if tool {
            if posed {
                let _ = self.return_to_design();
            }
            if let Some(a) = self.shell.fg.as_mut().and_then(|p| p.asm.as_mut()) {
                a.posing.grab = None;
            }
            return;
        }
        if self.asm_grab() {
            self.asm_show();
        }
        // a component held: no origin plane lit under the pointer
        if self
            .shell
            .fg
            .as_ref()
            .and_then(|p| p.asm.as_ref())
            .is_some_and(|a| a.posing.grab.is_some())
        {
            self.shell.origin_hover = None;
        }
    }

    /// The press, the drag and the letting go. True: the placements' view-only motions changed.
    fn asm_grab(&mut self) -> bool {
        let Some(v) = &mut self.view else { return false };
        let Some((occ, _)) = &self.fg_asm else {
            return false;
        };
        let Some(s) = &mut self.fg else { return false };
        let Some(p) = self.shell.fg.as_mut() else {
            return false;
        };
        let (feats, params) = (&p.feats, &p.params);
        let Some(a) = p.asm.as_mut() else { return false };
        let held = v.nav.held();
        // let go
        if !held {
            let Some(g) = a.posing.grab.take() else {
                return false;
            };
            if self.shell.status.starts_with("Locked: ") {
                self.shell.status.clear();
            }
            let moved = match &g.plan {
                Plan::Joint(j) => j
                    .dofs
                    .iter()
                    .zip(&g.values)
                    .any(|(d, x)| (d.value - x).abs() > 1e-9),
                Plan::Free { .. } => g.motion != Frame::WORLD,
                Plan::Locked(_) => false,
            };
            if moved && !a.posing.is_empty() {
                let ops = a.posing.ops();
                s.preview(Some(&ops));
                a.posing.say = true;
                self.shell.status = format!(
                    "{} · Keep Position writes it, Esc or Return to Design goes back",
                    a.posing.words()
                );
            }
            return false;
        }
        // a press on a component: taken from the window select (a click still selects)
        if a.posing.grab.is_none() {
            let Some(hit) = v.nav.press_hit() else {
                return false;
            };
            let Some(o) = occ.iter().find(|o| o.placement == Some(hit.placement as usize)) else {
                return false;
            };
            if !v.nav.grab() {
                return false;
            }
            a.posing.grab = Some(Grab {
                occ: o.path.clone(),
                // planned once what's built is up to date: its frames are where the drag starts
                plan: Plan::Locked(String::new()),
                at: p3(hit.point),
                values: vec![],
                motion: Frame::WORLD,
                at_limit: false,
            });
            a.posing.planned = false;
        }
        if !a.posing.planned && s.done == s.seq {
            let Some(occ) = a.posing.grab.as_ref().map(|g| g.occ.clone()) else {
                return false;
            };
            let mut plan = asmdrag::plan(a, feats, params, &occ);
            // values the pose has set and the last build may not show yet
            if let Plan::Joint(j) = &mut plan {
                for d in &mut j.dofs {
                    if let Hold::Param(k) = &d.hold
                        && let Some(l) = a.posing.params.iter().find(|x| x.0 == *k)
                    {
                        d.value = if d.angle {
                            l.1.value.to_radians()
                        } else {
                            l.1.value
                        };
                    }
                }
            }
            let Some(g) = &mut a.posing.grab else { return false };
            g.values = match &plan {
                Plan::Joint(j) => j.dofs.iter().map(|d| d.value).collect(),
                _ => vec![],
            };
            g.plan = plan;
            a.posing.planned = true;
        }
        let Some((from, to)) = v.nav.grabbed() else {
            return false;
        };
        if !a.posing.planned {
            self.shell.status = "Solving: the drag starts when it's done".into();
            return false;
        }
        let Some(g) = &mut a.posing.grab else { return false };
        let cam = v.nav.camera;
        let (w, h) = v.size;
        let rotate = v.nav.shift_held();
        let pixel = |q: Point3| {
            cam.pixel_of([q.x as f32, q.y as f32, q.z as f32], w, h)
                .map(|(x, y)| (x as f64, y as f64))
        };
        match g.plan.clone() {
            Plan::Locked(why) => {
                self.shell.status = format!("Locked: {why}");
                return false;
            }
            Plan::Joint(j) => {
                let moving = j.moving(rotate);
                if moving.is_empty() {
                    self.shell.status = format!("Locked: {}", j.held().join("; "));
                    return false;
                }
                let (vals, hit) = asmdrag::follow(
                    &j,
                    &moving,
                    g.at,
                    &g.values,
                    (to.0 as f64, to.1 as f64),
                    &pixel,
                    cam.dist as f64,
                );
                g.at_limit = hit;
                let changed = vals.iter().zip(&g.values).any(|(x, y)| (x - y).abs() > 1e-9);
                g.values = vals;
                self.shell.status = g.words();
                if !changed {
                    return false;
                }
                let g = g.clone();
                g.pose_into(&mut a.posing);
                let free = moving.iter().any(|&i| j.dofs[i].hold == Hold::Free);
                if free {
                    let m = j.world_motion(&g.values);
                    for (t, _) in &j.body {
                        a.moved.insert(t.clone(), m);
                        a.posing.settling.push(t.clone());
                    }
                    v.dirty = true;
                    return true;
                } else {
                    let drives = moving
                        .iter()
                        .filter(|&&i| matches!(j.dofs[i].hold, Hold::Param(_)))
                        .map(|&i| (j.joint.clone(), j.dofs[i].word.to_string(), g.values[i]))
                        .collect();
                    s.drive(drives);
                }
            }
            Plan::Free { body } => {
                let (_, right, up) = cam.axes();
                let eye = cam.eye();
                let fwd = cam.axes().0;
                let at = [g.at.x as f32, g.at.y as f32, g.at.z as f32];
                let depth = (0..3).map(|i| (at[i] - eye[i]) * fwd[i]).sum::<f32>();
                let point_at = |px: (f64, f64)| p3(cam.point_at((px.0 as f32, px.1 as f32), depth, w, h));
                let f = (from.0 as f64, from.1 as f64);
                let t = (to.0 as f64, to.1 as f64);
                g.motion = asmdrag::free_motion(g.at, f, t, rotate, &point_at, v3(up), v3(right));
                self.shell.status = g.words();
                let g = g.clone();
                g.pose_into(&mut a.posing);
                for (t, _) in &body {
                    a.moved.insert(t.clone(), g.motion);
                    if !a.posing.settling.contains(t) {
                        a.posing.settling.push(t.clone());
                    }
                }
                v.dirty = true;
                return true;
            }
        }
        false
    }

    /// A drive's poses from the kept solve, onto the placements. True: the view moved.
    pub(super) fn asm_posed(&mut self, poses: &[Frame]) -> bool {
        let Some((occ, placed)) = &mut self.fg_asm else {
            return false;
        };
        if poses.len() != occ.len() {
            return false;
        }
        for (o, f) in occ.iter().zip(poses) {
            if let Some(k) = o.placement
                && let Some(p) = placed.get_mut(k)
            {
                p.1 = crate::fg::rows_of(f);
            }
        }
        true
    }

    /// The pointer while a component is held: grabbing, or not allowed when it can't move.
    pub(super) fn asm_drag_cursor(&self, out: &mut egui::PlatformOutput) {
        let Some(a) = self.shell.fg.as_ref().and_then(|p| p.asm.as_ref()) else {
            return;
        };
        let Some(g) = &a.posing.grab else { return };
        let dragging = self.view.as_ref().is_some_and(|v| v.nav.grabbed().is_some());
        if !dragging || !a.posing.planned {
            return;
        }
        let stuck = match &g.plan {
            Plan::Locked(_) => true,
            Plan::Joint(j) => j.moving(false).is_empty() && j.moving(true).is_empty(),
            Plan::Free { .. } => false,
        };
        out.cursor_icon = if stuck {
            egui::CursorIcon::NotAllowed
        } else {
            egui::CursorIcon::Grabbing
        };
    }

    /// Agent tool: a joint's value set in the transient pose. `joint`: `id` or `id.word`;
    /// `value`: degrees for a turn, mm for a length; held within its limits.
    pub fn drive_joint(&mut self, joint: &str, value: f64) -> Result<String, String> {
        let p = self.shell.fg.as_mut().ok_or("no Forge file open")?;
        let (feats, params) = (&p.feats, &p.params);
        let a = p.asm.as_mut().ok_or("not an assembly, or still solving")?;
        let (id, word) = match joint.split_once('.') {
            Some((i, w)) => (i, Some(w)),
            None => (joint, None),
        };
        let dofs = asmdrag::joint_dofs(a, feats, params, id);
        if dofs.is_empty() {
            return Err(format!("{id}: no joint of that id with a value to drive"));
        }
        let d = match word {
            Some(w) => dofs
                .iter()
                .find(|d| d.word == w)
                .ok_or(format!("{id} has no {w}: it has {}", words(&dofs)))?,
            None => dofs
                .iter()
                .find(|d| matches!(d.hold, Hold::Param(_)))
                .unwrap_or(&dofs[0]),
        };
        let key = match &d.hold {
            Hold::Param(k) => k.clone(),
            Hold::Free => {
                return Err(format!(
                    "{id}.{}: nothing drives it; give it a param (`{}: {}`) to drive it",
                    d.word, d.word, d.word
                ));
            }
            Hold::Held(why) => return Err(format!("{id}.{}: {why}", d.word)),
        };
        let mut x = if d.angle { value.to_radians() } else { value };
        let mut note = "";
        if let Some((lo, hi)) = d.limits
            && (x < lo || x > hi)
        {
            x = x.clamp(lo, hi);
            note = " (held at its limit)";
        }
        let lit = asmdrag::literal(d, x);
        a.posing.set_param(key, lit);
        let ops = a.posing.ops();
        let words = format!("{id}.{} = {}{note}", d.word, asmdrag::literal_text(&lit));
        let s = self.fg.as_mut().ok_or("no Forge file open")?;
        if a.posing.over.is_none() {
            a.posing.over = Some(s.src.clone());
        }
        s.preview(Some(&ops));
        a.posing.say = true;
        self.shell.status = format!("Driven: {words}");
        Ok(words)
    }

    /// A transient pose put back as it was (the agent's undo_view).
    pub(crate) fn pose_again(
        &mut self,
        params: Vec<(forge_lang::error::ParamKey, forge_lang::error::Literal)>,
        poses: Vec<(String, forge_math::Frame)>,
    ) {
        let Some(a) = self.shell.fg.as_mut().and_then(|p| p.asm.as_mut()) else {
            return;
        };
        a.posing.params = params;
        a.posing.poses = poses;
        let ops = a.posing.ops();
        let Some(s) = self.fg.as_mut() else { return };
        if a.posing.over.is_none() {
            a.posing.over = Some(s.src.clone());
        }
        s.preview(Some(&ops));
    }

    /// Agent tool: the transient pose dropped, the file's own built again.
    pub fn return_to_design(&mut self) -> Result<String, String> {
        let a = self
            .shell
            .fg
            .as_mut()
            .and_then(|p| p.asm.as_mut())
            .ok_or("not an assembly")?;
        let was = !a.posing.is_empty();
        for t in std::mem::take(&mut a.posing.settling) {
            a.moved.remove(&t);
        }
        a.posing.clear();
        let s = self.fg.as_mut().ok_or("no Forge file open")?;
        if was || s.previewing {
            s.preview(None);
        }
        self.asm_show();
        self.shell.status = if was {
            "Returned to the design".into()
        } else {
            "Nothing dragged: the assembly is as designed".into()
        };
        Ok(self.shell.status.clone())
    }

    /// Agent tool: the transient pose written to the file, one undo step.
    pub fn keep_position(&mut self) -> Result<String, String> {
        let a = self
            .shell
            .fg
            .as_mut()
            .and_then(|p| p.asm.as_mut())
            .ok_or("not an assembly")?;
        if a.posing.is_empty() {
            return Err("Nothing dragged to keep".into());
        }
        let (ops, words) = (a.posing.ops(), a.posing.words());
        for t in std::mem::take(&mut a.posing.settling) {
            a.moved.remove(&t);
        }
        a.posing.clear();
        let s = self.fg.as_mut().ok_or("no Forge file open")?;
        s.apply_all(ops)
            .map_err(|e| format!("{}: {}", e.kind(), e.text()))?;
        self.asm_show();
        self.shell.status = format!("Kept: {words}");
        Ok(self.shell.status.clone())
    }
}

fn words(d: &[asmdrag::Dof]) -> String {
    d.iter().map(|d| d.word).collect::<Vec<_>>().join(", ")
}
