//! The host's side of the assembly tools (shell/asmtools.rs): their ops through the Forge
//! session (one undo step each, previews in the file's place), the snap points under the pointer
//! while an origin is wanted, the picks, Free Move's drags, placing and patterning components.

use super::Gui;
use crate::render::Entity;
use crate::shell::asmtools::{AsmTool, Hover, pose_text};
use crate::shell::fg::FgAct;
use crate::snaps::PartSnaps;
use forge_lang::error::Op;
use forge_math::{Frame, Point3, Vec3};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

type Found = (String, Result<Arc<PartSnaps>, String>);

/// What the host keeps for the tools between frames.
pub struct AsmHost {
    snaps: (Sender<Found>, Receiver<Found>),
    browsed: Option<Receiver<Option<String>>>,
    /// A Free Move drag: the instance, its move when it began, where in the world it was taken.
    drag: Option<(String, Frame, [f32; 3])>,
    /// A component was placed: the view fits the assembly once it's drawn.
    fit: bool,
}

impl Default for AsmHost {
    fn default() -> AsmHost {
        AsmHost {
            snaps: channel(),
            browsed: None,
            drag: None,
            fit: false,
        }
    }
}

/// A placement's rows as the frame they are.
fn frame_of(r: &crate::scene::Rows) -> Frame {
    let col = |k: usize| Vec3::new(r[0][k] as f64, r[1][k] as f64, r[2][k] as f64);
    Frame::from_axes(
        Point3::new(r[0][3] as f64, r[1][3] as f64, r[2][3] as f64),
        col(0),
        col(1),
        col(2),
    )
    .unwrap_or(Frame::WORLD)
}

fn f3(v: [f32; 3]) -> Vec3 {
    Vec3::new(v[0] as f64, v[1] as f64, v[2] as f64)
}

/// A turn by `angle` about an axis through `at`.
fn turn(at: Point3, axis: Vec3, angle: f64) -> Frame {
    let Some(z) = axis.normalize() else {
        return Frame::WORLD;
    };
    let about = Frame::from_z(at, z);
    about.rotated_about_z(angle).compose(&about.inverse())
}

impl Gui {
    pub(super) fn asm_act(&mut self, act: FgAct) {
        match act {
            FgAct::AsmOps(ops) => {
                let Some(s) = &mut self.fg else { return };
                let n = ops.len();
                match s.apply_all(ops) {
                    Ok(()) => {
                        self.shell.status =
                            format!("{n} edit{} to the assembly", if n == 1 { "" } else { "s" })
                    }
                    Err(e) => self.shell.status = format!("{}: {}", e.kind(), e.text()),
                }
            }
            FgAct::AsmPreview(ops) => {
                let Some(s) = &mut self.fg else { return };
                if ops.is_some() || s.previewing {
                    s.preview(ops.as_deref());
                }
            }
            FgAct::AsmPlace { file, like } => self.asm_place(&file, like.as_deref()),
            FgAct::AsmBrowse => {
                let (tx, rx) = channel();
                self.asm.browsed = Some(rx);
                let waker = self.waker.clone();
                std::thread::spawn(move || {
                    let got = rfd::FileDialog::new()
                        .add_filter("Forge parts and assemblies", &["fg"])
                        .pick_file()
                        .map(|p| p.to_string_lossy().into_owned());
                    let _ = tx.send(got);
                    waker();
                });
            }
            FgAct::AsmPattern { inst, count, step } => self.asm_pattern(&inst, count, step),
            FgAct::AsmApplyMoves => {
                let Some(a) = self.shell.fg.as_mut().and_then(|p| p.asm.as_mut()) else {
                    return;
                };
                let ops: Vec<Op> = a
                    .moved
                    .iter()
                    .filter_map(|(id, m)| {
                        let pose = a.poses.iter().find(|p| p.0 == *id)?.1;
                        Some(Op::SetArg {
                            id: id.clone(),
                            arg: "at".into(),
                            text: pose_text(&m.compose(&pose)),
                        })
                    })
                    .collect();
                a.moved.clear();
                self.asm_act(FgAct::AsmOps(ops));
                self.asm_show();
            }
            FgAct::AsmSnaps(file) => self.asm_find_snaps(file),
            _ => {}
        }
    }

    /// A part file the solve names, as a path to read.
    fn asm_path(&self, file: &str) -> std::path::PathBuf {
        let p = std::path::PathBuf::from(file);
        if p.is_absolute() || p.exists() {
            return p;
        }
        let dir = self
            .fg
            .as_ref()
            .and_then(|s| s.path.as_deref())
            .and_then(|p| std::path::Path::new(p).parent().map(|d| d.to_path_buf()));
        dir.map_or(p.clone(), |d| d.join(&p))
    }

    fn asm_find_snaps(&mut self, file: String) {
        let Some(a) = self.shell.fg.as_mut().and_then(|p| p.asm.as_mut()) else {
            return;
        };
        if a.snaps.contains_key(&file) {
            return;
        }
        a.snaps.insert(file.clone(), Err("Finding snap points…".into()));
        let path = self.asm_path(&file);
        let tx = self.asm.snaps.0.clone();
        let waker = self.waker.clone();
        std::thread::spawn(move || {
            let got = std::fs::read_to_string(&path)
                .map_err(|e| format!("{}: {e}", path.display()))
                .and_then(|text| crate::snaps::part_snaps(&text, path.parent().map(|d| d.to_path_buf())))
                .map(Arc::new);
            let _ = tx.send((file, got));
            waker();
        });
    }

    /// Each frame of an assembly with a tool up: snaps found, a browsed file placed, the
    /// pointer's snaps, a click taken as an origin, Free Move's drag. True: a click was the tool's.
    pub(super) fn asm_tools_frame(&mut self) -> bool {
        if self.asm.fit
            && self.fg.as_ref().is_some_and(|s| s.done == s.seq)
            && self.fg_view.is_none()
            && let Some(v) = &mut self.view
        {
            self.asm.fit = false;
            v.nav.home(&v.scene);
            v.dirty = true;
        }
        while let Ok((file, got)) = self.asm.snaps.1.try_recv() {
            if let Some(a) = self.shell.fg.as_mut().and_then(|p| p.asm.as_mut()) {
                a.snaps.insert(file, got);
            }
        }
        if let Some(r) = &self.asm.browsed
            && let Ok(got) = r.try_recv()
        {
            self.asm.browsed = None;
            if let Some(f) = got {
                let rel = self.asm_relative(&f);
                self.asm_place(&rel, None);
            }
        }
        let tool = self
            .shell
            .fg
            .as_ref()
            .and_then(|p| p.asm.as_ref())
            .and_then(|a| a.tool.clone());
        let Some(tool) = tool else {
            self.asm.drag = None;
            return false;
        };
        if let AsmTool::Free(rotate) = tool {
            self.asm_free(rotate);
            return false;
        }
        let Some(v) = &mut self.view else { return false };
        let clicked = v.nav.clicks != v.clicks;
        // a click: the snap the dialog lit last frame is the origin
        let mut took = false;
        if clicked && let Some(a) = self.shell.fg.as_mut().and_then(|p| p.asm.as_mut()) {
            let near = a.hover.as_ref().and_then(|h| h.near.clone());
            if let Some(AsmTool::Link(f)) = &mut a.tool
                && let Some(w) = f.waiting
            {
                v.clicks = v.nav.clicks;
                v.nav.selected.clear();
                v.clicked.clear();
                took = true;
                if let Some(p) = near {
                    f.picks[w] = Some(p);
                    f.waiting = match w {
                        0 if f.picks[1].is_none() => Some(1),
                        _ => None,
                    };
                    // two faces' origins face each other, as Inventor aligns them
                    let face = |k: usize| {
                        f.picks[k].as_ref().and_then(|p| p.kind).is_some_and(|k| {
                            !matches!(k, crate::snaps::SnapKind::Anchor | crate::snaps::SnapKind::Axis)
                        })
                    };
                    if f.joint && f.picks.iter().all(Option::is_some) {
                        f.flip = face(0) && face(1);
                    }
                    v.dirty = true;
                }
            }
        }
        // the picks follow their occurrences as the preview moves them
        if let (Some(a), Some((occ, placed))) =
            (self.shell.fg.as_mut().and_then(|p| p.asm.as_mut()), &self.fg_asm)
            && let Some(AsmTool::Link(f)) = &mut a.tool
        {
            for p in f.picks.iter_mut().flatten() {
                let pose = occ
                    .iter()
                    .find(|o| o.path == p.occ)
                    .and_then(|o| placed.get(o.placement?))
                    .map(|x| frame_of(&x.1));
                if let (Some(pose), Some(l)) = (pose, p.local) {
                    p.world = Some(pose.compose(&l));
                }
            }
        }
        if !tool.waiting() {
            if let Some(a) = self.shell.fg.as_mut().and_then(|p| p.asm.as_mut()) {
                a.hover = None;
            }
            return took;
        }
        // what's under the pointer, and its snaps in the world (a press hides the mark: kept)
        if v.nav.held() {
            return took;
        }
        let mark = v.nav.marks().into_iter().find(|m| !m.selected);
        let hit = v.nav.hit_point();
        let cam = v.nav.camera;
        let Some((occ, placed)) = &self.fg_asm else {
            return took;
        };
        let under = mark.and_then(|m| {
            let k = occ
                .iter()
                .position(|o| o.placement == Some(m.placement as usize))?;
            Some((
                occ[k].path.clone(),
                occ[k].file.clone(),
                frame_of(&placed[m.placement as usize].1),
                m.entity,
            ))
        });
        let mut ask = None;
        if let Some(a) = self.shell.fg.as_mut().and_then(|p| p.asm.as_mut()) {
            // the snap the dialog lit stays lit till it looks again
            let near = a.hover.take().and_then(|h| h.near);
            a.hover = under.map(|(path, file, pose, ent)| {
                let mut h = Hover {
                    near: near.filter(|n| n.occ == path),
                    occ: path,
                    ..Hover::default()
                };
                match a.snaps.get(&file) {
                    None => {
                        h.why = Some("Finding snap points…".into());
                        ask = Some(file);
                    }
                    Some(Err(e)) => h.why = Some(e.clone()),
                    Some(Ok(ps)) => {
                        let mut list: Vec<usize> = match ent {
                            Entity::Face(f) => ps.faces.get(f as usize).cloned().unwrap_or_default(),
                            Entity::Edge(e) => ps.edges.get(&e).cloned().unwrap_or_default(),
                            Entity::Vertex(_) => vec![],
                        };
                        // the part's own anchors near the pointer
                        let reach = (cam.dist * 0.08) as f64;
                        if let Some(p) = hit {
                            let p = Point3::new(p[0] as f64, p[1] as f64, p[2] as f64);
                            list.extend(ps.anchors.iter().filter(|&&k| {
                                (pose.point_to_world(ps.snaps[k].frame.origin) - p).length() < reach
                            }));
                        }
                        if list.is_empty() {
                            h.why =
                                Some("No Forge name here: pick a face an anchor names, or an anchor".into());
                        }
                        h.snaps = list
                            .into_iter()
                            .map(|k| {
                                let s = &ps.snaps[k];
                                (s.kind, s.text.clone(), pose.compose(&s.frame), s.frame)
                            })
                            .collect();
                    }
                }
                h
            });
        }
        if let Some(f) = ask {
            self.asm_find_snaps(f);
        }
        took
    }

    /// Free Move and Free Rotate: a drag in the view moves (turns) the component it began on, in
    /// the screen's plane at the depth it was taken; nothing is written till Apply.
    fn asm_free(&mut self, rotate: bool) {
        let Some(v) = &mut self.view else { return };
        v.nav.window = None;
        let Some((occ, _)) = &self.fg_asm else { return };
        let Some(a) = self.shell.fg.as_mut().and_then(|p| p.asm.as_mut()) else {
            return;
        };
        // what's under the pointer, for its DOF
        let mark = v.nav.marks().into_iter().find(|m| !m.selected);
        let under = mark.and_then(|m| occ.iter().find(|o| o.placement == Some(m.placement as usize)));
        a.hover = under.map(|o| Hover {
            occ: o.path.clone(),
            ..Hover::default()
        });
        let Some((from, to)) = v.nav.window_box() else {
            self.asm.drag = None;
            return;
        };
        if self.asm.drag.is_none() {
            let (Some(o), Some(p)) = (under, v.nav.hit_point()) else {
                return;
            };
            let top = o.path.split('.').next().unwrap_or(&o.path).to_string();
            let start = a.moved.get(&top).copied().unwrap_or(Frame::WORLD);
            self.asm.drag = Some((top, start, p));
        }
        let Some((top, start, at)) = self.asm.drag.clone() else {
            return;
        };
        let cam = v.nav.camera;
        let (fwd, right, up) = cam.axes();
        let eye = cam.eye();
        let depth = (0..3).map(|i| (at[i] - eye[i]) * fwd[i]).sum::<f32>();
        let step = if rotate {
            let (dx, dy) = ((to.0 - from.0) as f64, (to.1 - from.1) as f64);
            let c = Point3::new(at[0] as f64, at[1] as f64, at[2] as f64);
            turn(c, f3(up), dx * 0.01).compose(&turn(c, f3(right), dy * 0.01))
        } else {
            let p = cam.point_at(from, depth, v.size.0, v.size.1);
            let q = cam.point_at(to, depth, v.size.0, v.size.1);
            Frame::WORLD.with_origin(Point3::ORIGIN + (f3(q) - f3(p)))
        };
        a.moved.insert(top, step.compose(&start));
        v.dirty = true;
        self.asm_show();
    }

    /// A file the system's dialog gave, as the assembly writes it: from its folder.
    fn asm_relative(&self, f: &str) -> String {
        let dir = self
            .fg
            .as_ref()
            .and_then(|s| s.path.as_deref())
            .and_then(|p| std::fs::canonicalize(p).ok())
            .and_then(|p| p.parent().map(|d| d.to_path_buf()));
        let Some(dir) = dir else { return f.to_string() };
        let f = std::fs::canonicalize(f).unwrap_or_else(|_| f.into());
        // up from the assembly's folder as far as needed
        let mut up = String::new();
        let mut d = dir.as_path();
        loop {
            if let Ok(rest) = f.strip_prefix(d) {
                return format!("{up}{}", rest.to_string_lossy());
            }
            match d.parent() {
                Some(p) => {
                    d = p;
                    up.push_str("../");
                }
                None => return f.to_string_lossy().into_owned(),
            }
        }
    }

    /// The occurrences' box in the world.
    fn asm_world_box(&self) -> Option<([f32; 3], [f32; 3])> {
        let (_, placed) = self.fg_asm.as_ref()?;
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for (m, r) in placed {
            let Some(b) = self.fg_asm_boxes.get(*m) else {
                continue;
            };
            for c in 0..8 {
                let p = [
                    if c & 1 == 0 { b.0[0] } else { b.1[0] },
                    if c & 2 == 0 { b.0[1] } else { b.1[1] },
                    if c & 4 == 0 { b.0[2] } else { b.1[2] },
                ];
                for i in 0..3 {
                    let w = r[i][0] * p[0] + r[i][1] * p[1] + r[i][2] * p[2] + r[i][3];
                    lo[i] = lo[i].min(w);
                    hi[i] = hi[i].max(w);
                }
            }
        }
        (lo[0] <= hi[0]).then_some((lo, hi))
    }

    /// Place: a part (or subassembly) statement at the end, beside what's there; the first
    /// grounded. Another like `like`: its orientation, beside it.
    fn asm_place(&mut self, file: &str, like: Option<&str>) {
        let path = self.asm_path(file);
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                self.shell.status = format!("{}: {e}", path.display());
                return;
            }
        };
        let sub = crate::fg::is_assembly(&text);
        let Some(p) = self.shell.fg.as_ref() else { return };
        let src = p.source.clone();
        let first = p.asm.as_ref().is_none_or(|a| a.insts.is_empty());
        let stem: String = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "part".into())
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect();
        let stem = if stem.starts_with(|c: char| c.is_ascii_digit()) {
            format!("p{stem}")
        } else {
            stem
        };
        let name = crate::shell::asmtools::free_name(&src, &stem);
        let bx = self.asm_world_box();
        let size = bx.map_or(50.0, |(lo, hi)| {
            (hi[0] - lo[0]).max(hi[1] - lo[1]).max(hi[2] - lo[2]).max(20.0)
        });
        let pose = match like.and_then(|l| p.asm.as_ref()?.poses.iter().find(|x| x.0 == l).map(|x| x.1)) {
            Some(f) => f.with_origin(f.origin + Vec3::new((size * 0.3) as f64, 0.0, 0.0)),
            None if first => Frame::WORLD,
            None => {
                let x = bx.map_or(0.0, |(_, hi)| hi[0] + size * 0.6);
                Frame::WORLD.with_origin(Point3::new(x as f64, 0.0, 0.0))
            }
        };
        let call = if sub { "subassembly" } else { "part" };
        let ground = if first { ", ground: true" } else { "" };
        let line = format!("{name} = {call}(\"{file}\", at: {}{ground})", pose_text(&pose));
        self.asm_act(FgAct::AsmOps(vec![Op::AddFeature {
            before: None,
            text: line,
        }]));
        self.shell.status = format!("Placed {name}{}", if first { " (grounded)" } else { "" });
        self.asm.fit = true;
    }

    /// Pattern: `count - 1` more of an instance, each `step` further on, in a loop.
    fn asm_pattern(&mut self, inst: &str, count: usize, step: [f64; 3]) {
        let Some(a) = self.shell.fg.as_ref().and_then(|p| p.asm.as_ref()) else {
            return;
        };
        let Some(i) = a.insts.iter().find(|i| i.path == inst) else {
            return;
        };
        let Some(pose) = a.poses.iter().find(|p| p.0 == inst).map(|p| p.1) else {
            return;
        };
        let src = self
            .shell
            .fg
            .as_ref()
            .map(|p| p.source.clone())
            .unwrap_or_default();
        let name = crate::shell::asmtools::free_name(&src, &format!("{inst}_pat"));
        let v = |x: f64| crate::shell::fg::short((x * 1e6).round() / 1e6);
        let (o, z, x) = (pose.origin, pose.z().get(), pose.x().get());
        let at = |o: f64, s: f64| {
            if s == 0.0 {
                v(o)
            } else {
                format!("{} + {} * k", v(o), v(s))
            }
        };
        let text = format!(
            "for k in 1..{count} {{\n  {name} = part(\"{}\", at: pose(at: ({}, {}, {}), z: ({}, {}, {}), x: ({}, {}, {})))\n}}",
            i.file,
            at(o.x, step[0]),
            at(o.y, step[1]),
            at(o.z, step[2]),
            v(z.x),
            v(z.y),
            v(z.z),
            v(x.x),
            v(x.y),
            v(x.z),
        );
        self.asm_act(FgAct::AsmOps(vec![Op::AddFeature { before: None, text }]));
    }
}

/// Free Move's motion of an occurrence's rows (the view only).
pub fn moved_rows(m: &Frame, rows: &crate::scene::Rows) -> crate::scene::Rows {
    crate::fg::rows_of(&m.compose(&frame_of(rows)))
}
