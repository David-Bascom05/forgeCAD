//! The Render Studio in the window: while its environment is active, the view shows the path
//! tracer's picture of the view's parts, from the view's camera, kept improving a few samples a
//! frame and started again on any change; the Render Image and Turntable commands queue final
//! renders written to files (the browser keeps them as the project's files). What the user chose
//! is `shell.studio` (`shell/studio_env.rs`).

use super::{Gui, now_ms};
use crate::shell::env::EnvId;
use crate::shell::studio_env::{Ask, READABLE, RenderResult, Rendering};
use crate::studio::appearance::Appearance;
use crate::studio::bsdf::{Bsdf, Lut};
use crate::studio::env::{self, Env};
use crate::studio::live::{Job, Live, Output, Progress, Queue};
use crate::studio::scene::{Built, Instance};
use crate::studio::{Backdrop, Settings, View};

pub struct StudioHost {
    /// The view cube alone, over transparent, laid on the traced picture.
    cube: crate::render::Offscreen,
    cube_tex: egui::TextureId,
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
    /// The render under way began (ms); the picture it last wrote, to show; its texture.
    render_started: Option<f64>,
    written: Option<((u32, u32), Vec<u8>)>,
    thumb: Option<egui::TextureHandle>,
    /// The placements' tints before the Studio gave them its looks (given back on leaving), for
    /// the geometry they were taken from.
    saved_tints: Option<(u64, Vec<[f32; 4]>)>,
    /// The view's last pick, to take a new one once.
    last_pick: Option<(u32, crate::render::Entity)>,
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

/// The light chosen (`studio_env::lights`): the drawn studio, a manifest HDRI, or a file.
fn lighting(light: usize, hdri: &str) -> Result<Env, String> {
    let list = crate::studio::hdri::list();
    if (1..=list.len()).contains(&light) {
        return crate::studio::hdri::load(&list[light - 1].key);
    }
    if light == list.len() + 1 && !hdri.trim().is_empty() {
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
        let s = if c <= 0.0031308 {
            c * 12.92
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        };
        (s * 255.0).round() as u8
    };
    [enc(t[0]), enc(t[1]), enc(t[2])]
}

/// The faces to light for the Model tab: every face of each placement of a chosen part, and
/// fainter of the part whose row is under the pointer.
pub(super) fn lit(
    st: &crate::shell::studio_env::StudioUi,
    scene: &crate::scene::Scene,
    parts: &[crate::mesh::PartMesh],
    colour: egui::Color32,
) -> Vec<crate::render::FaceColour> {
    let mut out = Vec::new();
    if st.selected.is_empty() && st.hover_row.is_empty() {
        return out;
    }
    for (i, p) in scene.placements.iter().enumerate() {
        let k = p.part as usize;
        // faint enough that the look under it still shows (the look is what's being changed); the
        // ghost pass draws the far faces too, so layers add up: low per layer
        let a = if st.selected.contains(&k) {
            45
        } else if st.hover_row.contains(&k) {
            22
        } else {
            continue;
        };
        let Some(lod) = parts.get(k).and_then(|m| m.lods.first()) else {
            continue;
        };
        for f in &lod.faces {
            if out.len() >= crate::render::FACE_COLOURS {
                return out;
            }
            out.push(crate::render::FaceColour {
                placement: i as u32,
                face: f.face,
                rgba: [colour.r(), colour.g(), colour.b(), a],
            });
        }
    }
    out
}

impl Gui {
    /// One frame of the Studio; true while it has more to do (the window should draw again).
    pub(super) fn studio_frame(&mut self) -> bool {
        let active = self.shell.env == Some(EnvId::RenderStudio);
        if !active {
            if let Some(h) = self.studio.take() {
                self.egui.free_texture(&h.tex);
                self.egui.free_texture(&h.cube_tex);
                self.shell.view_over_tex = None;
                self.shell.view_over_alpha = 1.0;
                if let Some(v) = &mut self.view {
                    self.shell.view_tex = Some(v.tex);
                    // the design's colours again, where the Studio's looks were
                    if let Some((geo, tints)) = h.saved_tints
                        && geo == v.renderer.geometry()
                        && tints.len() == v.scene.placements.len()
                    {
                        for (p, t) in v.scene.placements.iter_mut().zip(tints) {
                            p.tint = t;
                        }
                        v.renderer.set_tints(&v.scene);
                        v.dirty = true;
                    }
                }
            }
            return false;
        }
        // where Render Image and Turntable write when no file is named
        let (render_dir, render_name) = self.capture_place_for("render");
        let Some(v) = &mut self.view else {
            return false;
        };
        let size = v.size;
        if self.studio.is_none() {
            let live = Live::new(&self.device, &self.queue, size);
            let tex = self.egui.register_native_texture(
                &self.device,
                &live.tracer.picture_view,
                wgpu::FilterMode::Linear,
            );
            self.studio = Some(StudioHost {
                live,
                queue: Queue::default(),
                tex,
                tex_size: size,
                cube: crate::render::Offscreen::new(&self.device, size),
                cube_tex: egui::TextureId::default(),
                env: Env::new(8, 4, vec![[0.0; 3]; 32]),
                env_key: None,
                scene_key: None,
                scene: None,
                last: now_ms(),
                render_started: None,
                written: None,
                thumb: None,
                saved_tints: None,
                last_pick: None,
            });
        }
        let mut host = self.studio.take().expect("made above");
        let mut st = std::mem::take(&mut self.shell.studio);
        let st = &mut st;
        let mut status = None;
        let lib = st.library();

        // the part under the pointer lights its row; a click (not painting) chooses its row,
        // its group opened and the row shown
        st.hover_view = v
            .nav
            .hovered()
            .and_then(|h| v.scene.placements.get(h.placement as usize))
            .map(|p| p.part as usize);
        let pick = v.nav.selected.last().copied();
        if !st.painting
            && pick != host.last_pick
            && let Some((pl, _)) = pick
            && let Some(p) = v.scene.placements.get(pl as usize)
        {
            let k = p.part as usize;
            st.selected = vec![k];
            st.anchor = Some(k);
            if let Some(g) = st.groups.get(k).filter(|g| !g.is_empty()) {
                st.open.insert(g.clone());
            }
            st.reveal = Some(k);
        }
        host.last_pick = pick;

        // the brush: what's clicked in the view while painting takes its appearance
        if st.painting
            && let Some(brush) = st.brush.clone()
            && !v.nav.selected.is_empty()
        {
            st.history.push(st.looks.clone());
            for (pl, _) in std::mem::take(&mut v.nav.selected) {
                if let Some(p) = v.scene.placements.get(pl as usize) {
                    let k = p.part as usize;
                    if st.looks.len() <= k {
                        st.looks.resize(k + 1, None);
                    }
                    st.looks[k] = Some(brush.clone());
                    st.said = format!("{}: {brush}", st.names.get(k).map_or("part", String::as_str));
                }
            }
            st.version += 1;
            v.dirty = true;
        }

        // the light
        let env_key = (st.light, st.hdri.clone());
        if host.env_key.as_ref() != Some(&env_key) {
            match lighting(st.light, &st.hdri) {
                Ok(e) => {
                    host.live.set_env(&e);
                    host.env = e;
                }
                Err(why) => {
                    status = Some(format!("Render Studio: {why}; the drawn studio meanwhile"));
                    let e = lighting(0, "").expect("the drawn studio");
                    host.live.set_env(&e);
                    host.env = e;
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
            poses_key(&v.scene)
                ^ material_name
                    .as_deref()
                    .map_or(0, |n| n.len() as u64 * 0x9E37_79B9),
            // the looks themselves, wherever they were changed from
            st.version ^ {
                use std::hash::{Hash, Hasher};
                let mut h = std::collections::hash_map::DefaultHasher::new();
                st.looks.hash(&mut h);
                h.finish()
            },
        );
        if host.scene_key != Some(key) {
            // an assembly's part by the file of an occurrence of it, its folder its group, its
            // occurrences counted, its own material; one part by the document
            let mut names: Vec<Option<String>> = vec![None; parts.len()];
            let mut groups: Vec<String> = vec![String::new(); parts.len()];
            let mut counts: Vec<usize> = vec![0; parts.len()];
            let mut own: Vec<Option<String>> = vec![None; parts.len()];
            let mut files: Vec<String> = vec![String::new(); parts.len()];
            if let Some((occ, _)) = &self.fg_asm {
                for o in occ {
                    let Some(pl) = o.placement else { continue };
                    let Some(p) = v.scene.placements.get(pl) else {
                        continue;
                    };
                    let k = p.part as usize;
                    if k >= parts.len() {
                        continue;
                    }
                    let path = std::path::Path::new(&o.file);
                    let stem = path
                        .file_stem()
                        .map_or_else(|| o.file.clone(), |s| s.to_string_lossy().into_owned());
                    if names[k].is_none() {
                        names[k] = Some(stem);
                        groups[k] = path
                            .parent()
                            .and_then(|d| d.file_name())
                            .map(|d| d.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        own[k] = o.material.clone();
                        files[k] = o.file.clone();
                    }
                    counts[k] += 1;
                }
            }
            // one folder for all: no groups
            if groups.iter().all(|g| *g == groups[0]) {
                groups.iter_mut().for_each(String::clear);
            }
            st.groups = groups;
            st.files = files;
            st.counts = counts;
            if own.iter().all(Option::is_none) {
                own = vec![material_name.clone(); parts.len()];
            }
            let mats_of: Vec<Option<forge_material::Material>> = own
                .iter()
                .map(|n| {
                    n.as_deref().and_then(|n| {
                        forge_material::Libraries::cached(None)
                            .find(n)
                            .map(|m| m.1.clone())
                    })
                })
                .collect();
            st.own = own;
            st.names = names
                .into_iter()
                .enumerate()
                .map(|(k, n)| {
                    n.unwrap_or_else(|| match (parts.len(), &name) {
                        (1, n) if !n.is_empty() => n.strip_suffix(".fg").unwrap_or(n).to_string(),
                        _ => format!("Part {}", k + 1),
                    })
                })
                .collect();
            st.looks.resize(parts.len(), None);
            let mut mats: Vec<Bsdf> = Vec::new();
            let mut inst = Vec::new();
            let mut raster: Vec<(usize, [f32; 4])> = Vec::new();
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
                let colour = if material.is_some() {
                    None
                } else {
                    Some(tint_bytes(pl.tint))
                };
                // the part's own material, else the document's
                let mine = mats_of.get(pl.part as usize).cloned().flatten();
                let material = mine.as_ref().or(material.as_ref());
                let colour = if material.is_some() { None } else { colour };
                let b = lib.resolve(look.as_deref(), material, colour);
                let m = mats.iter().position(|x| *x == b).unwrap_or_else(|| {
                    mats.push(b);
                    mats.len() - 1
                });
                inst.push(Instance {
                    part: pl.part,
                    rows,
                    material: m as u32,
                });
                raster.push((
                    i,
                    [
                        b.base.x,
                        b.base.y,
                        b.base.z,
                        crate::studio::appearance::pack_tint(b.metal, b.rough, b.coat),
                    ],
                ));
            }
            mats.push(
                lib.find("Studio floor")
                    .map_or_else(Bsdf::default, |a: &Appearance| a.bsdf),
            );
            let built = Built::new(&parts, &inst);
            if let Err(why) = host.live.set_built(&built, &mats) {
                status = Some(format!("Render Studio: {why}"));
            }
            host.scene = Some((built, mats));
            host.scene_key = Some(key);
            host.live.restart();
            // the raster view in the same looks, to stand in while the trace starts again
            let geo = v.renderer.geometry();
            if host.saved_tints.as_ref().is_none_or(|(g, _)| *g != geo) {
                host.saved_tints = Some((geo, v.scene.placements.iter().map(|p| p.tint).collect()));
            }
            for (i, t) in raster {
                v.scene.placements[i].tint = t;
            }
            v.renderer.set_tints(&v.scene);
            v.dirty = true;
        }
        let Some((built, mats)) = host.scene.clone() else {
            self.shell.studio = std::mem::take(st);
            self.studio = Some(host);
            return false;
        };

        // the camera, the settings
        let cam = &v.nav.camera;
        let mut view = View::of(cam);
        st.view_focal_mm = if cam.parallel { 0.0 } else { view.focal_mm() };
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
        // a final render asked or under way has the GPU: the live picture waits, as it is (on a
        // software GPU a live sample is seconds, and the click must answer at once)
        let rendering = host.queue.progress() != Progress::Idle
            || st.asks.iter().any(|a| matches!(a, Ask::Image | Ask::Turntable));
        let more = !rendering && host.live.frame(view, settings, size);
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
        // the view cube over the picture, as the shaded view has it
        if host.cube_tex == egui::TextureId::default()
            || host.cube.texture.size().width != size.0
            || host.cube.texture.size().height != size.1
        {
            if host.cube_tex != egui::TextureId::default() {
                self.egui.free_texture(&host.cube_tex);
            }
            host.cube = crate::render::Offscreen::new(&self.device, size);
            host.cube_tex = self.egui.register_native_texture(
                &self.device,
                &super::raw_view(&host.cube),
                wgpu::FilterMode::Nearest,
            );
        }
        let overlay = crate::render::Overlay {
            cube: Some((v.nav.cube_place(), v.nav.cube_hover)),
            ..crate::render::Overlay::default()
        };
        // what's chosen in the Model tab, lit over the trace as over the raster view
        let lit_faces = lit(st, &v.scene, &v.renderer.cpu_parts, self.shell.theme.selection);
        v.renderer
            .cube_only(&host.cube.view, &v.nav.camera, &overlay, &v.scene, &lit_faces);
        let cube_tex = host.cube_tex;
        let tex = host.tex;
        let raster_tex = v.tex;

        // final renders
        let mut written: Option<((u32, u32), Vec<u8>)> = None;
        // a job asked this frame starts on the next: this one shows its progress, no more
        let mut asked_now = false;
        for ask in std::mem::take(&mut st.asks) {
            match ask {
                Ask::Cancel => {
                    if host.queue.progress() != Progress::Idle {
                        host.queue.cancel();
                        status = Some("Render cancelled".into());
                    }
                    host.render_started = None;
                }
                Ask::Reveal(path) => super::gui_record::reveal(&path),
                Ask::Image | Ask::Turntable => {
                    let (w, h) = st.size;
                    // the view's field, kept for the render's shape along its height
                    let mut fview = view;
                    if st.depth_of_field && !cam.parallel {
                        fview = fview.with_lens(view.focal_mm(), Some(st.f_number), cam.dist);
                    }
                    // where captures go (the Capture folder, else the document's): a fresh
                    // `<document>-render-<time>.png` there, or the file named (a name alone there)
                    let out = {
                        let p = std::path::Path::new(st.out.trim());
                        if st.out.trim().is_empty() {
                            let ext = if ask == Ask::Image { ".png" } else { "" };
                            Self::fresh_name(&render_dir, &render_name, ext)
                        } else if p.is_relative() && !render_dir.as_os_str().is_empty() {
                            render_dir.join(p).to_string_lossy().into_owned()
                        } else {
                            st.out.trim().to_string()
                        }
                    };
                    let output = if ask == Ask::Image {
                        Output::Image(out)
                    } else {
                        Output::Turntable {
                            stem: out,
                            frames: st.frames,
                        }
                    };
                    st.result = None;
                    asked_now = true;
                    if host.render_started.is_none() {
                        host.render_started = Some(now_ms());
                    }
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
        if !asked_now {
            host.queue.tick(
                &self.device,
                &self.queue,
                || Some((built, mats, Lut::new(512))),
                &host.env,
                |path, size, rgba| {
                    let r = crate::studio::save_png(path, size, &rgba);
                    written = Some((size, rgba));
                    r
                },
            );
        }
        if let Some(w) = written.take() {
            host.written = Some(w);
        }
        // the cards over the view: the progress, from the frame it was asked; then the result
        let was_running = st.rendering.is_some();
        st.rendering = match host.queue.progress() {
            Progress::Running { job, frame, done, of } => {
                let frames = host.queue.jobs.get(job).map_or(1, Job::frames);
                let what = if frames > 1 {
                    format!("Rendering turntable · frame {} of {frames}", frame + 1)
                } else {
                    "Rendering image".to_string()
                };
                let per = of as u64;
                let started = *host.render_started.get_or_insert(now_ms());
                Some(Rendering {
                    what,
                    done: frame as u64 * per + done as u64,
                    of: frames as u64 * per,
                    started_ms: started,
                    now_ms: now_ms(),
                })
            }
            Progress::Idle => None,
        };
        if was_running
            && st.rendering.is_none()
            && let Some(last) = host.queue.log.last().cloned()
        {
            host.render_started = None;
            let thumb = host.written.take().map(|((w, h), rgba)| {
                let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                let t = self
                    .ctx
                    .load_texture("studio-render", img, egui::TextureOptions::LINEAR);
                let id = t.id();
                host.thumb = Some(t);
                (id, (w, h))
            });
            status = Some(match &last {
                Ok(p) => format!("Wrote {p}"),
                Err(e) => format!("Render failed: {e}"),
            });
            st.result = Some(RenderResult {
                said: last,
                thumb: thumb.filter(|_| host.queue.log.last().is_some_and(Result::is_ok)),
            });
        }
        st.queue = match host.queue.progress() {
            Progress::Idle => match host.queue.log.last() {
                Some(Ok(p)) => format!("Wrote {p}"),
                Some(Err(e)) => format!("Render failed: {e}"),
                None => String::new(),
            },
            Progress::Running { job, frame, done, of } => {
                let frames = host.queue.jobs.get(job).map_or(1, Job::frames);
                if frames > 1 {
                    format!(
                        "Rendering frame {} of {frames}: {done} of {of} samples",
                        frame + 1
                    )
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
        let s = host.live.samples();
        self.studio = Some(host);
        self.shell.studio = std::mem::take(st);
        // until the trace is readable the raster view (the same looks, its own cube) stands in;
        // then the trace fades in over it
        if s < READABLE {
            self.shell.view_tex = Some(raster_tex);
            self.shell.view_over_tex = None;
            self.shell.view_over_alpha = 1.0;
        } else if s < 2 * READABLE {
            self.shell.view_tex = Some(raster_tex);
            self.shell.view_over_tex = Some(tex);
            self.shell.view_over_alpha = (s - READABLE) as f32 / READABLE as f32;
        } else {
            self.shell.view_tex = Some(tex);
            self.shell.view_over_tex = Some(cube_tex);
            self.shell.view_over_alpha = 1.0;
        }
        if let Some(s) = status {
            self.shell.status = s;
        }
        busy
    }
}
