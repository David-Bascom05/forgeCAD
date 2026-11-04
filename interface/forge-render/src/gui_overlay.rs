//! The view's switches made into world triangles (grid, planes, anchors), and the anchors they
//! need: each part file's found once on a thread and kept.

use super::Gui;
use crate::origin::WorldVert;
use crate::shell::Overlays;
use forge_math::{Frame, Point3, Vec3};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, Sender, channel};

pub type Frames = Vec<(String, Frame)>;

/// The own part's anchors are kept under this, not a file's name: its text is the session's.
pub const SELF: &str = "";

pub struct Extras {
    pub frames: HashMap<String, Frames>,
    pub asked: HashSet<String>,
    send: Sender<(String, Frames)>,
    recv: Receiver<(String, Frames)>,
    /// What the extras were drawn for: a change draws the view again.
    pub was: Option<(Overlays, Option<String>, usize, u64)>,
}

impl Default for Extras {
    fn default() -> Extras {
        let (send, recv) = channel();
        Extras {
            frames: HashMap::new(),
            asked: HashSet::new(),
            send,
            recv,
            was: None,
        }
    }
}

/// A placement's rows applied to a point and a direction.
fn place(rows: &crate::scene::Rows, p: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|r| rows[r][0] * p[0] + rows[r][1] * p[1] + rows[r][2] * p[2] + rows[r][3])
}

fn turn(rows: &crate::scene::Rows, d: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|r| rows[r][0] * d[0] + rows[r][1] * d[1] + rows[r][2] * d[2])
}

fn f3(p: Point3) -> [f32; 3] {
    [p.x as f32, p.y as f32, p.z as f32]
}

fn v3(v: Vec3) -> [f32; 3] {
    [v.x as f32, v.y as f32, v.z as f32]
}

impl Gui {
    /// A Forge part's solid hidden or shown by its eye in the tree: kept so through rebuilds.
    pub(super) fn body_eye(&mut self) {
        const NOWHERE: crate::scene::Rows = [[0.0; 4]; 3];
        let hide = self
            .shell
            .fg
            .as_ref()
            .is_some_and(|p| p.body_hidden && p.asm.is_none() && !p.assembly);
        let Some(v) = &mut self.view else { return };
        if self.fg.is_none() || v.scene.placements.is_empty() {
            return;
        }
        let now = v.scene.rows_of(0);
        if hide && now != NOWHERE {
            self.body_rows = Some(now);
            v.scene.set_rows(0, &NOWHERE);
            v.dirty = true;
        } else if !hide
            && now == NOWHERE
            && let Some(r) = self.body_rows.take()
        {
            v.scene.set_rows(0, &r);
            v.dirty = true;
        }
    }

    /// Anchors found since the last frame; the own part's dropped when it's built again.
    pub(super) fn take_extras(&mut self, rebuilt: bool) {
        if rebuilt {
            self.extras.frames.remove(SELF);
            self.extras.asked.remove(SELF);
        }
        let mut got = false;
        while let Ok((file, f)) = self.extras.recv.try_recv() {
            self.extras.frames.insert(file, f);
            got = true;
        }
        if got && let Some(v) = &mut self.view {
            v.dirty = true;
        }
    }

    /// The occurrences whose anchors are shown: what's chosen, at most a few dozen.
    fn anchored(&self) -> Vec<usize> {
        let Some(a) = self.shell.fg.as_ref().and_then(|p| p.asm.as_ref()) else {
            return vec![];
        };
        a.picked_mask()
            .iter()
            .enumerate()
            .filter(|x| *x.1)
            .map(|x| x.0)
            .take(48)
            .collect()
    }

    /// With Anchors on: the anchors wanted and not yet found, asked for.
    pub(super) fn ask_extras(&mut self) {
        if !self.shell.overlays().anchors || self.shell.fg.is_none() {
            return;
        }
        let Some(s) = &self.fg else { return };
        let dir = s
            .path
            .as_deref()
            .and_then(|p| std::path::Path::new(p).parent().map(|d| d.to_path_buf()));
        let mut want: Vec<(String, Option<String>)> = vec![];
        if self.shell.is_asm() {
            if let Some(a) = self.shell.fg.as_ref().and_then(|p| p.asm.as_ref()) {
                for k in self.anchored() {
                    want.push((a.occ[k].file.clone(), None));
                }
            }
        } else {
            want.push((SELF.to_string(), Some(s.src.clone())));
        }
        for (file, text) in want {
            if !self.extras.asked.insert(file.clone()) {
                continue;
            }
            let (send, waker, dir) = (self.extras.send.clone(), self.waker.clone(), dir.clone());
            std::thread::spawn(move || {
                let path = dir
                    .as_ref()
                    .map(|d| d.join(&file))
                    .filter(|p| p.exists())
                    .unwrap_or_else(|| std::path::PathBuf::from(&file));
                let text = text
                    .or_else(|| forge_lang::stdparts::text_of(&file))
                    .or_else(|| std::fs::read_to_string(&path).ok());
                let frames = text.map_or(vec![], |t| {
                    let at = if file == SELF {
                        dir.clone()
                    } else {
                        path.parent().map(|d| d.to_path_buf())
                    };
                    forge_lang::asm::part_anchors(&t, at)
                });
                let _ = send.send((file, frames));
                waker();
            });
        }
    }

    /// What the extras depend on besides the camera: a change marks the view to draw again.
    pub(super) fn extras_changed(&mut self) -> bool {
        let sel = self
            .shell
            .fg
            .as_ref()
            .and_then(|p| p.asm.as_ref())
            .and_then(|a| a.selected.clone());
        let feats = self.shell.fg.as_ref().map_or(0, |p| {
            p.feats.len() * 7
                + p.eyes.values().filter(|v| **v).count() * 131
                + p.eyes.len()
                + usize::from(p.thermo.mould.is_some()) * 31
                + match p.thermo.analysis {
                    crate::shell::thermo_ui::Analysis::None => 0,
                    crate::shell::thermo_ui::Analysis::Draft => {
                        7919 + (p.thermo.draft_min.unwrap_or(0.0) * 100.0) as usize * 3
                    }
                    crate::shell::thermo_ui::Analysis::Thickness => {
                        104729 + p.thermo.step.unwrap_or(usize::MAX / 1024) * 11
                    }
                }
        });
        let key = (
            self.shell.overlays(),
            sel,
            self.extras.frames.len() + feats,
            self.explode_key(),
        );
        if self.extras.was.as_ref() != Some(&key) {
            self.extras.was = Some(key);
            return true;
        }
        false
    }

    /// The grid, the planes and the anchors the switches ask for, for this view.
    /// And what shows through the model, dimmed: the anchors.
    pub(super) fn overlay_world(&self) -> (Vec<WorldVert>, Vec<WorldVert>) {
        let mut out = vec![];
        let mut behind = vec![];
        let (Some(v), Some(p)) = (&self.view, &self.shell.fg) else {
            return (out, behind);
        };
        let o = self.shell.overlays();
        let (c, r) = v.scene.bounds();
        let (cam, size, px) = (&v.nav.camera, v.size, self.pixels_per_point);
        let ink = {
            let i = self.shell.theme.ink2;
            crate::viewcube::rgb((i.r() as u32) << 16 | (i.g() as u32) << 8 | i.b() as u32)
        };
        if o.grid {
            crate::overlay::grid(c, r, cam, size, px, ink, &mut out);
        }
        {
            let blue = crate::viewcube::rgb(0x5A9BE0);
            let shown = |id: &str| p.eyes.get(id).copied().unwrap_or(o.planes);
            for f in p.feats.iter().filter(|f| shown(&f.id)).filter_map(|f| f.frame) {
                crate::overlay::plane(
                    f3(f.origin),
                    v3(f.x().get()),
                    v3(f.y().get()),
                    r * 0.35,
                    blue,
                    cam,
                    size,
                    px,
                    &mut out,
                );
            }
        }
        if o.anchors {
            let len = r * 0.1;
            let triads = |frames: &Frames, rows: Option<&crate::scene::Rows>, out: &mut Vec<WorldVert>| {
                for (_, f) in frames {
                    let (mut at, mut axes) =
                        (f3(f.origin), [v3(f.x().get()), v3(f.y().get()), v3(f.z().get())]);
                    if let Some(m) = rows {
                        at = place(m, at);
                        axes = axes.map(|d| turn(m, d));
                    }
                    crate::overlay::triad(at, axes, len, cam, size, px, out);
                }
            };
            let mut through = vec![];
            if self.shell.is_asm() {
                if let (Some(a), Some((occ, placed))) = (p.asm.as_ref(), &self.fg_asm) {
                    for k in self.anchored() {
                        let Some(frames) = self.extras.frames.get(&a.occ[k].file) else {
                            continue;
                        };
                        let Some(pl) = occ.get(k).and_then(|o| o.placement) else {
                            continue;
                        };
                        let rows = v.scene.rows_of(pl);
                        let rows = if rows == [[0.0; 4]; 3] { placed[pl].1 } else { rows };
                        triads(frames, Some(&rows), &mut through);
                    }
                }
            } else if let Some(frames) = self.extras.frames.get(SELF) {
                triads(frames, None, &mut through);
            }
            for w in &through {
                behind.push(WorldVert {
                    at: w.at,
                    colour: [w.colour[0], w.colour[1], w.colour[2], w.colour[3] * 0.45],
                });
            }
            out.extend(through);
        }
        // a thermoform's analysis: the wall thickness at the step watched, or the drafts
        if let Some(g) = self.fg_walls.first() {
            use crate::shell::thermo_ui::Analysis;
            match p.thermo.analysis {
                Analysis::Thickness => {
                    let k = p
                        .thermo
                        .step
                        .unwrap_or(usize::MAX)
                        .min(g.steps.len().saturating_sub(1));
                    if let Some(s) = g.steps.get(k) {
                        out.extend_from_slice(s);
                    }
                }
                Analysis::Draft => {
                    let min = p
                        .thermo
                        .draft_min
                        .or(p.thermos.first().map(|r| r.draft_min))
                        .unwrap_or(3.0);
                    out.extend(g.draft_tris(min));
                }
                Analysis::None => {}
            }
            // the mould's vents while its dialog is up
            if p.thermo.mould.is_some() {
                // seen through the part too: they lie under the sheet
                for v in &g.vents {
                    for (list, a) in [(&mut out, 1.0), (&mut behind, 0.9)] {
                        crate::origin::disc(*v, px * 6.0, [0.05, 0.06, 0.08, a], cam, size, list);
                        crate::origin::disc(*v, px * 3.5, [0.98, 0.72, 0.22, a], cam, size, list);
                    }
                }
            }
        }
        self.explode_trails(&mut out);
        (out, behind)
    }
}
