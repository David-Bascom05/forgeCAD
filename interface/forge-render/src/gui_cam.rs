//! The host's side of the Manufacture workspace (shell/cam): a machining document opened with its
//! part in the view, the document generated off the window's thread, the files it asks for
//! written, and in the view what the workspace has in focus: the setup's stock and work
//! offset and a located check's witness (the toolpath and its simulation are CAM-SIM's view,
//! hosted on the same tab, which reads `CamWs::focus()`).
use super::Gui;
use crate::origin::WorldVert;
use crate::shell::cam::{CamWs, run};

impl Gui {
    /// Opens a machining document: the workspace, over its part (opened as the view's model).
    pub(super) fn open_cam(&mut self, path: &str, text: &str) -> bool {
        let ws = CamWs::open(path, text);
        let part = ws.part_path();
        let shown = match &part {
            Some(p) if forge_dirs::fs::is_file(p) => self.open_fg(&p.display().to_string()),
            _ => false,
        };
        if !shown {
            // no part to show: the workspace alone, over an empty view
            self.fg = None;
            self.shell.fg = None;
        }
        let name = std::path::Path::new(path)
            .file_name()
            .map_or_else(|| path.to_string(), |n| n.to_string_lossy().into_owned());
        self.shell.cam = Some(ws);
        self.shell.tab = crate::shell::CAM_TAB;
        self.shell.name = name.clone();
        self.shell.status = match part {
            Some(p) if !shown => format!("{name}: its part {} could not be opened", p.display()),
            _ => format!("Opening {name}"),
        };
        true
    }

    /// Each frame: a run asked for is started, one finished is taken in, files are written.
    pub(super) fn cam_host(&mut self) {
        let Some(w) = &mut self.shell.cam else { return };
        // a finished run
        let done = w.job.lock().unwrap_or_else(|e| e.into_inner()).take();
        let mut changed = false;
        if let Some(r) = done {
            w.ran(r);
            changed = true;
            self.shell.status = crate::shell::cam::status_line(w);
        }
        // a run asked for, none going
        if w.want_run && w.running.is_none() {
            w.want_run = false;
            let text = w.text.clone();
            let dir = w.dir.clone();
            let slot = w.job.clone();
            let waker = self.waker.clone();
            w.running = Some(text.clone());
            self.shell.status = "Generating: every op posted, read back and certified".into();
            crate::plat::spawn("forge-cam", move || {
                let r = run::run(&text, &dir);
                *slot.lock().unwrap_or_else(|e| e.into_inner()) = Some(r);
                waker();
            });
        }
        // a check asked, none going: the run checked again off this thread
        let Some(w) = &mut self.shell.cam else { return };
        if w.want_check && w.running.is_none() {
            w.want_check = false;
            if let Some(r) = w.run.take() {
                let slot = w.job.clone();
                let waker = self.waker.clone();
                w.running = Some(r.text.clone());
                self.shell.status = "Checking: the posted paths certified again".into();
                crate::plat::spawn("forge-cam-check", move || {
                    let r = run::recheck(r);
                    *slot.lock().unwrap_or_else(|e| e.into_inner()) = Some(r);
                    waker();
                });
            }
        }
        let Some(w) = &mut self.shell.cam else { return };
        for (path, text) in std::mem::take(&mut w.want_write) {
            match forge_dirs::fs::write(&path, text.as_bytes()) {
                Ok(()) => {
                    if path == w.path {
                        w.saved = text;
                    }
                    w.said = Some(format!("Wrote {path}"));
                    self.shell.status = format!("Wrote {path}");
                }
                Err(e) => {
                    w.said = Some(format!("{path}: {e}"));
                    self.shell.status = format!("{path}: {e}");
                }
            }
        }
        if changed && let Some(v) = &mut self.view {
            v.dirty = true;
        }
    }

    /// What the view's extras depend on from the workspace.
    pub(super) fn cam_key(&self) -> usize {
        let Some(w) = &self.shell.cam else { return 0 };
        let mut h = std::hash::DefaultHasher::new();
        use std::hash::{Hash, Hasher};
        format!("{:?}", w.focus).hash(&mut h);
        w.run.as_ref().map(|r| r.text.len()).hash(&mut h);
        w.running.is_some().hash(&mut h);
        h.finish() as usize
    }

    /// The workspace's extras in the view, in the part's frame.
    pub(super) fn cam_overlay(&self, out: &mut Vec<WorldVert>, behind: &mut Vec<WorldVert>) {
        let (Some(v), Some(w)) = (&self.view, &self.shell.cam) else {
            return;
        };
        let Some(run) = &w.run else { return };
        let (cam, size, px) = (&v.nav.camera, v.size, self.pixels_per_point);
        let Some(su) = w.focus.setup.as_ref().and_then(|s| run.setup(s)) else {
            return;
        };
        // a setup that turns the part over, or machines another part (soft jaws), is not in the
        // frame the view shows
        // a lathe setup: in the part's own frame (z the spindle axis), a witness at (ρ, 0, z)
        if run.process == "turning" {
            if let Some(l) = &w.focus.locate {
                let q = [l.at[0] as f32, l.at[1] as f32, l.at[2] as f32];
                for (list, a) in [(&mut *out, 1.0), (&mut *behind, 0.9)] {
                    crate::origin::disc(q, px * 9.0, [0.05, 0.06, 0.08, a], cam, size, list);
                    crate::origin::disc(q, px * 6.0, [0.93, 0.27, 0.22, a], cam, size, list);
                }
            }
            return;
        }
        let turned = w.stmt(&su.name).is_some_and(|s| s.arg("from").is_some());
        let main = w.parts.first().map(|p| p.file.as_str());
        if turned || main != Some(su.part.as_str()) {
            return;
        }
        let o = su.origin;
        let p3 = |q: [f64; 3]| [(q[0] + o[0]) as f32, (q[1] + o[1]) as f32, (q[2] + o[2]) as f32];
        // the stock's box
        let (lo, hi) = (su.stock_min, su.stock_max);
        let c = |k: usize| {
            [
                if k & 1 == 0 { lo[0] } else { hi[0] } as f32,
                if k & 2 == 0 { lo[1] } else { hi[1] } as f32,
                if k & 4 == 0 { lo[2] } else { hi[2] } as f32,
            ]
        };
        let stock = [0.55, 0.62, 0.72, 0.9];
        for (a, b) in [
            (0, 1),
            (2, 3),
            (4, 5),
            (6, 7),
            (0, 2),
            (1, 3),
            (4, 6),
            (5, 7),
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7),
        ] {
            crate::origin::polyline(&[c(a), c(b)], px * 1.2, stock, cam, size, out);
            crate::origin::polyline(
                &[c(a), c(b)],
                px * 1.0,
                [stock[0], stock[1], stock[2], 0.35],
                cam,
                size,
                behind,
            );
        }
        // the work offset: its axes at the origin
        let len = ((hi[0] - lo[0]).max(hi[1] - lo[1]) * 0.18).max(5.0);
        for (k, col) in [
            [0.9, 0.3, 0.25, 1.0],
            [0.3, 0.75, 0.35, 1.0],
            [0.3, 0.5, 0.95, 1.0],
        ]
        .into_iter()
        .enumerate()
        {
            let mut e = [0.0; 3];
            e[k] = len;
            crate::origin::polyline(&[p3([0.0; 3]), p3(e)], px * 2.5, col, cam, size, out);
            crate::origin::polyline(
                &[p3([0.0; 3]), p3(e)],
                px * 2.0,
                [col[0], col[1], col[2], 0.5],
                cam,
                size,
                behind,
            );
        }
        // a located check's witness, seen through the part
        if let Some(l) = &w.focus.locate {
            let q = p3(l.at);
            for (list, a) in [(&mut *out, 1.0), (&mut *behind, 0.9)] {
                crate::origin::disc(q, px * 9.0, [0.05, 0.06, 0.08, a], cam, size, list);
                crate::origin::disc(q, px * 6.0, [0.93, 0.27, 0.22, a], cam, size, list);
            }
        }
    }
}
