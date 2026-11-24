//! The Render Studio in the window: while its environment is active, the view shows the path
//! tracer's picture of the view's parts, from the view's camera, kept improving a few samples a
//! frame and started again on any change; the Render Image and Turntable commands queue final
//! renders written to files (the browser keeps them as the project's files). What the user chose
//! is `shell.studio` (`shell/studio_env.rs`).

use super::{Gui, now_ms};
use crate::shell::env::EnvId;
use crate::shell::studio_env::Ask;
use crate::studio::appearance::Appearance;
use crate::studio::bsdf::{Bsdf, Lut};
use crate::studio::env::{self, Env};
use crate::studio::live::{Job, Live, Output, Progress, Queue};
use crate::studio::scene::{Built, Instance};
use crate::studio::{Backdrop, Settings, View};

pub struct StudioHost {
    live: Live,
    queue: Queue,
    tex: egui::TextureId,
    tex_size: (u32, u32),
    env: Env,
    env_key: Option<(usize, String)>,
    scene_key: Option<(u64, u64, u64)>,
    scene: Option<(Built, Vec<Bsdf>)>,
    /// When the last frame's samples went in, ms.
    last: f64,
}

/// A hash of the placements' poses and what's shown: the scene changed when it does.
fn poses_key(scene: &crate::scene::Scene) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    scene.rows_bytes.hash(&mut h);
    for i in 0..scene.placements.len() {
        scene.is_gone(i).hash(&mut h);
        scene.offset_of(i).map(f32::to_bits).hash(&mut h);
    }
    h.finish()
}

fn lighting(light: usize, hdri: &str) -> Result<Env, String> {
    if light == 1 && !hdri.trim().is_empty() {
        let bytes = forge_dirs::fs::read(hdri.trim()).map_err(|e| format!("{hdri}: {e}"))?;
        let (w, h, rgb) = env::read_hdr(&bytes).map_err(|e| format!("{hdri}: {e}"))?;
        return Ok(Env::new(w, h, rgb));
    }
    Ok(Env::new(512, 256, env::studio(512, 256, 1.0)))
}

/// sRGB bytes of a placement's tint (the app's part colour).
fn tint_bytes(t: [f32; 4]) -> [u8; 3] {
    let enc = |c: f32| {
        let c = c.clamp(0.0, 1.0);
        let s = if c <= 0.0031308 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 };
        (s * 255.0).round() as u8
    };
    [enc(t[0]), enc(t[1]), enc(t[2])]
}

impl Gui {
    /// One frame of the Studio; true while it has more to do (the window should draw again).
    pub(super) fn studio_frame(&mut self) -> bool {
        let active = self.shell.env == Some(EnvId::RenderStudio);
        if !active {
            if let Some(h) = self.studio.take() {
                self.egui.free_texture(&h.tex);
                if let Some(v) = &self.view {
                    self.shell.view_tex = Some(v.tex);
                }
            }
            return false;
        }
        let Some(v) = &self.view else {
            return false;
        };
        let size = v.size;
        if self.studio.is_none() {
            let live = Live::new(&self.device, &self.queue, size);
            let tex = self
                .egui
                .register_native_texture(&self.device, &live.tracer.picture_view, wgpu::FilterMode::Linear);
            self.studio = Some(StudioHost {
                live,
                queue: Queue::default(),
                tex,
                tex_size: size,
                env: Env::new(8, 4, vec![[0.0; 3]; 32]),
                env_key: None,
                scene_key: None,
                scene: None,
                last: now_ms(),
            });
        }
        let mut host = self.studio.take().expect("made above");
        let mut st = std::mem::take(&mut self.shell.studio);
        let st = &mut st;
        let mut status = None;
        let lib = st.library();

        // the light
        let env_key = (st.light, st.hdri.clone());
        if host.env_key.as_ref() != Some(&env_key) {
            match lighting(st.light, &st.hdri) {
                Ok(e) => {
                    host.live.set_env(&e);
                    host.env = e;
                }
                Err(why) => {
                    status = Some(format!("Render Studio: {why}"));
                    if host.env_key.is_none() {
                        let e = lighting(0, "").expect("the drawn studio");
                        host.live.set_env(&e);
                        host.env = e;
                    }
                }
            }
            host.env_key = Some(env_key);
        }

        // the scene: the view's parts where they stand, each its appearance
        let name = self.shell.name.clone();
        let parts = v.renderer.cpu_parts.clone();
        // a part's material (its iProperties): its appearance's default
        let material_name = (!self.shell.inspect.assembly)
            .then(|| self.shell.inspect.doc.material.clone())
            .flatten();
        let material = material_name.as_deref().and_then(|n| {
            forge_material::Libraries::cached(None)
                .find(n)
                .map(|m| m.1.clone())
        });
        let key = (
            v.renderer.geometry(),
            poses_key(&v.scene) ^ material_name.as_deref().map_or(0, |n| n.len() as u64 * 0x9E37_79B9),
            st.version,
        );
        if host.scene_key != Some(key) {
            st.names = (0..parts.len())
                .map(|k| match (parts.len(), &name) {
                    (1, n) if !n.is_empty() => n.clone(),
                    _ => format!("Part {}", k + 1),
                })
                .collect();
            st.looks.resize(parts.len(), None);
            let mut mats: Vec<Bsdf> = Vec::new();
            let mut inst = Vec::new();
            for i in 0..v.scene.placements.len() {
                if v.scene.is_gone(i) {
                    continue;
                }
                let pl = &v.scene.placements[i];
                let mut rows = v.scene.rows_of(i);
                let off = v.scene.offset_of(i);
                for k in 0..3 {
                    rows[k][3] += off[k];
                }
                let look = st.looks.get(pl.part as usize).cloned().flatten();
                let colour = if material.is_some() { None } else { Some(tint_bytes(pl.tint)) };
                let b = lib.resolve(look.as_deref(), material.as_ref(), colour);
                let m = mats.iter().position(|x| *x == b).unwrap_or_else(|| {
                    mats.push(b);
                    mats.len() - 1
                });
                inst.push(Instance {
                    part: pl.part,
                    rows,
                    material: m as u32,
                });
            }
            mats.push(lib.find("Studio floor").map_or_else(Bsdf::default, |a: &Appearance| a.bsdf));
            let built = Built::new(&parts, &inst);
            if let Err(why) = host.live.set_built(&built, &mats) {
                status = Some(format!("Render Studio: {why}"));
            }
            host.scene = Some((built, mats));
            host.scene_key = Some(key);
            host.live.restart();
        }
        let Some((built, mats)) = host.scene.clone() else {
            self.shell.studio = std::mem::take(st);
            self.studio = Some(host);
            return false;
        };

        // the camera, the settings
        let cam = &v.nav.camera;
        let mut view = View::of(cam);
        if st.depth_of_field && !cam.parallel {
            view = view.with_lens(view.focal_mm(), Some(st.f_number), cam.dist);
        }
        let ground = (st.floor)
            .then(|| built.bounds.map(|b| (b.min.z, (mats.len() - 1) as u32)))
            .flatten();
        let settings = Settings {
            exposure: st.exposure,
            env_turn: st.turn_deg.to_radians(),
            env_strength: st.strength,
            clamp: 16.0,
            ground,
            backdrop: Backdrop::Environment,
            ..Settings::default()
        };
        let now = now_ms();
        host.live.pace.after(now - host.last);
        host.last = now;
        let more = host.live.frame(view, settings, size);
        if !st.denoise && more {
            host.live.tracer.show(settings.exposure, false);
        }
        if host.live.tracer.size != host.tex_size {
            self.egui.update_egui_texture_from_wgpu_texture(
                &self.device,
                &host.live.tracer.picture_view,
                wgpu::FilterMode::Linear,
                host.tex,
            );
            host.tex_size = host.live.tracer.size;
        }
        st.samples = host.live.samples();
        let tex = host.tex;

        // final renders
        for ask in std::mem::take(&mut st.asks) {
            match ask {
                Ask::Cancel => host.queue.cancel(),
                Ask::Image | Ask::Turntable => {
                    let (w, h) = st.size;
                    // the view's field, kept for the render's shape along its height
                    let mut fview = view;
                    if st.depth_of_field && !cam.parallel {
                        fview = fview.with_lens(view.focal_mm(), Some(st.f_number), cam.dist);
                    }
                    let output = if ask == Ask::Image {
                        Output::Image(st.out.clone())
                    } else {
                        Output::Turntable {
                            stem: st.out.clone(),
                            frames: st.frames,
                        }
                    };
                    let pivot = built.bounds.map_or(cam.target, |b| ((b.min + b.max) * 0.5).arr());
                    host.queue.push(Job {
                        output,
                        size: (w.max(16), h.max(16)),
                        spp: st.spp.max(1),
                        denoise: st.denoise,
                        view: fview,
                        settings,
                        pivot,
                    });
                }
            }
        }
        host.queue.tick(
            &self.device,
            &self.queue,
            || Some((built, mats, Lut::new(512))),
            &host.env,
            |path, size, rgba| crate::studio::save_png(path, size, &rgba),
        );
        st.queue = match host.queue.progress() {
            Progress::Idle => match host.queue.log.last() {
                Some(Ok(p)) => format!("Wrote {p}"),
                Some(Err(e)) => format!("Render failed: {e}"),
                None => String::new(),
            },
            Progress::Running { job, frame, done, of } => {
                let frames = host.queue.jobs.get(job).map_or(1, Job::frames);
                if frames > 1 {
                    format!("Rendering frame {} of {frames}: {done} of {of} samples", frame + 1)
                } else {
                    format!("Rendering: {done} of {of} samples")
                }
            }
        };
        let busy = more || host.queue.progress() != Progress::Idle;
        if host.queue.progress() == Progress::Idle {
            // the queue's log has said its last; only its newest line stays
            let keep = host.queue.log.pop();
            host.queue.log.clear();
            host.queue.log.extend(keep);
        }
        self.studio = Some(host);
        self.shell.studio = std::mem::take(st);
        self.shell.view_tex = Some(tex);
        if let Some(s) = status {
            self.shell.status = s;
        }
        busy
    }
}
