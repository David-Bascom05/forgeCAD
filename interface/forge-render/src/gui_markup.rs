//! The markup's host side (`shell/markup.rs`): once the frame with the marks is drawn, the window
//! is drawn again into a picture of its own, what's under each mark is named through the view's
//! ids (the window select's pass), and the message goes with both. A sent message's marks shown
//! again put their camera back.

use super::{Gui, posed_name};
use crate::render::{Entity, ViewIds};
use crate::shell::agent::Entry;
use crate::shell::markup::{Mark, Sent, Thumb, Under, describe};
use crate::viewer::now_ms;
use egui::{Pos2, Rect, pos2, vec2};
use std::collections::HashMap;
use std::sync::Arc;

/// The thumbnail's largest side, in pixels.
const THUMB_PX: usize = 200;

fn kind(e: Entity) -> &'static str {
    match e {
        Entity::Face(_) => "face",
        Entity::Edge(_) => "edge",
        Entity::Vertex(_) => "vertex",
    }
}

/// What's under the marks, named as the app tools name it.
struct Names<'a> {
    g: &'a Gui,
    ids: Option<&'a ViewIds>,
    /// The view's rect in window points, and pixels per point.
    view: Rect,
    ppp: f32,
    /// Each placement's pixels, and each entity's, in the whole view.
    wholes: HashMap<u32, u32>,
    parts: HashMap<(u32, Entity), u32>,
    /// An assembly's occurrences named as their own models name them.
    occ: Option<&'a OccNames>,
}

impl<'a> Names<'a> {
    fn new(
        g: &'a Gui,
        ids: Option<&'a ViewIds>,
        ppp: f32,
        regions: bool,
        occ: Option<&'a OccNames>,
    ) -> Names<'a> {
        let (mut wholes, mut parts) = (HashMap::new(), HashMap::new());
        if let (Some(ids), true) = (ids, regions) {
            for y in 0..ids.h {
                for x in 0..ids.w {
                    if let Some((pl, e)) = ids.at(x, y) {
                        *wholes.entry(pl).or_insert(0) += 1;
                        *parts.entry((pl, e)).or_insert(0) += 1;
                    }
                }
            }
        }
        Names {
            g,
            ids,
            view: g.shell.view_rect,
            ppp,
            wholes,
            parts,
            occ,
        }
    }

    /// The view's pixel under a window point.
    fn px(&self, p: Pos2) -> (i64, i64) {
        let q = (p - self.view.min) * self.ppp;
        (q.x.floor() as i64, q.y.floor() as i64)
    }

    /// What's drawn at a window point, or nearest it within `reach` points.
    fn hit(&self, p: Pos2, reach: f32) -> Option<(u32, Entity)> {
        let ids = self.ids?;
        let (x, y) = self.px(p);
        let r = (reach * self.ppp).round() as i64;
        let mut best: Option<(i64, (u32, Entity))> = None;
        for dy in -r..=r {
            for dx in -r..=r {
                let d = dx * dx + dy * dy;
                if d > r * r || best.is_some_and(|b| b.0 <= d) || x + dx < 0 || y + dy < 0 {
                    continue;
                }
                if let Some(h) = ids.at((x + dx) as usize, (y + dy) as usize) {
                    best = Some((d, h));
                }
            }
        }
        best.map(|b| b.1)
    }

    /// A placement as a whole: an assembly's occurrence (or its part's thermoform mould), a
    /// part's solid. A harness's wires and segments are named here once they're drawn.
    fn whole(&self, pl: u32) -> Option<String> {
        match &self.g.fg_asm {
            Some((occ, _)) => occ.iter().find_map(|o| {
                if o.placement == Some(pl as usize) {
                    Some(format!("occurrence `{}`", o.path))
                } else if o.tooling == Some(pl as usize) {
                    Some(format!("the thermoform mould of occurrence `{}`", o.path))
                } else {
                    None
                }
            }),
            None => Some("the solid (`solid`)".into()),
        }
    }

    /// A face or edge by its stable name. In an assembly, as its occurrence's own model names it
    /// (the part file's anchors, or the in-file component's); on a part, by the part's anchors,
    /// a thermoform's bodies said as such. One without a name is said so.
    fn part(&self, pl: u32, e: Entity) -> Option<String> {
        let g = self.g;
        let (list, k) = match e {
            Entity::Face(k) => (g.fg_anchors.as_deref().map(|a| &a.faces), k),
            Entity::Edge(k) => (g.fg_anchors.as_deref().map(|a| &a.edges), k),
            Entity::Vertex(_) => return None,
        };
        if let Some((occ, _)) = &g.fg_asm {
            let o = occ.iter().find(|o| o.placement == Some(pl as usize))?;
            if let Ok(n) = posed_name(
                occ,
                g.inspector.data.as_deref(),
                g.fg_anchors.as_deref(),
                pl as usize,
                e,
            ) {
                return Some(format!("{} `{n}`", kind(e)));
            }
            let (body, an) = self.occ.and_then(|m| m.get(&o.path))?;
            // the body drawn, if the inspector has it; else the one the names were found on
            let drawn = g
                .inspector
                .data
                .as_ref()
                .and_then(|d| d.bodies.get(d.placed.get(pl as usize)?.0).cloned());
            let b = drawn.as_ref().unwrap_or(body);
            let name = match e {
                Entity::Face(k) if (k as usize) < b.faces.len() => {
                    an.faces.get(&b.face(forge_topo::FaceId(k)).name)
                }
                Entity::Edge(k) if (k as usize) < b.edges.len() => {
                    an.edges.get(&b.edge(forge_topo::EdgeId(k)).name)
                }
                _ => None,
            };
            return Some(match name {
                Some(n) => format!("{} `{n}`", kind(e)),
                None => format!("{} #{k} (no stable name)", kind(e)),
            });
        }
        let mut of = "";
        if let (Some(w), Entity::Face(f)) = (g.fg_walls.first(), e) {
            use crate::thermo_view::UNPICKED;
            if (UNPICKED..UNPICKED + 3).contains(&f) {
                return Some("the formed part, as simulated".into());
            }
            if (UNPICKED + 8..UNPICKED + 14).contains(&f) {
                return Some("the thermoform blank".into());
            }
            if w.mould_span.as_ref().is_some_and(|s| s.faces.contains(&f)) {
                of = " of the thermoform mould";
            } else if w.part_span.as_ref().is_some_and(|s| s.faces.contains(&f)) {
                of = " of the formed part";
            }
        }
        Some(match list.and_then(|l| l.get(k as usize).cloned().flatten()) {
            Some(n) => format!("{} `{n}`{of}", kind(e)),
            None => format!("{} #{k}{of} (no stable name)", kind(e)),
        })
    }

    fn thing(&self, pl: u32, e: Entity) -> Option<String> {
        match (self.part(pl, e), self.g.fg_asm.is_some()) {
            (Some(p), true) => Some(match self.whole(pl) {
                Some(w) => format!("{w} › {p}"),
                None => p,
            }),
            (Some(p), false) => Some(p),
            (None, _) => self.whole(pl),
        }
    }

    /// A browser row, or another named place of the app, under a point.
    fn place(&self, p: Pos2) -> Option<String> {
        let (name, _) = self
            .g
            .shell
            .places
            .iter()
            .filter(|(n, r)| r.contains(p) && !n.starts_with("markup:") && !n.starts_with("side:"))
            .min_by(|a, b| a.1.area().total_cmp(&b.1.area()))?;
        Some(match name.split_once(":row:") {
            Some((_, key)) => format!("browser row `{key}`"),
            None => format!("the app's `{name}`"),
        })
    }
}

impl Under for Names<'_> {
    fn at(&self, p: Pos2, reach: f32) -> Option<String> {
        if !self.view.contains(p) {
            return self.place(p);
        }
        if let Some(d) = &self.g.shell.drawing {
            return d.markup_at(p, reach);
        }
        let (pl, e) = self.hit(p, reach)?;
        self.thing(pl, e)
    }

    fn sheet(&self, p: Pos2) -> Option<(f64, f64)> {
        let d = self.g.shell.drawing.as_ref()?;
        self.view.contains(p).then(|| d.sheet_mm(p))
    }

    fn within(&self, m: &Mark) -> Vec<(String, u32, u32, bool)> {
        let b = m.bounds();
        let mut out: Vec<(String, u32, u32, bool)> = Vec::new();
        // the browser's rows whose middle it holds
        for (n, r) in &self.g.shell.places {
            if let Some((_, key)) = n.split_once(":row:")
                && !self.view.contains(r.center())
                && m.inside(r.center())
            {
                out.push((format!("browser row `{key}`"), 1, 1, true));
            }
        }
        let over = b.intersect(self.view);
        if over.is_negative() || over.area() <= 0.0 {
            return out;
        }
        if let Some(d) = &self.g.shell.drawing {
            let mut seen: HashMap<String, u32> = HashMap::new();
            let (mut y, step) = (over.top(), 6.0);
            while y <= over.bottom() {
                let mut x = over.left();
                while x <= over.right() {
                    if m.inside(pos2(x, y))
                        && let Some(n) = d.markup_at(pos2(x, y), 0.0)
                    {
                        *seen.entry(n).or_insert(0) += 1;
                    }
                    x += step;
                }
                y += step;
            }
            out.extend(seen.into_iter().map(|(n, c)| (n, c, c, true)));
            return out;
        }
        let Some(ids) = self.ids else { return out };
        let (lo, hi) = (self.px(over.min), self.px(over.max));
        let (mut wholes, mut parts): (HashMap<u32, u32>, HashMap<(u32, Entity), u32>) = Default::default();
        for y in lo.1.max(0)..=hi.1.min(ids.h as i64 - 1) {
            for x in lo.0.max(0)..=hi.0.min(ids.w as i64 - 1) {
                let at = self.view.min + vec2(x as f32 + 0.5, y as f32 + 0.5) / self.ppp;
                if !m.inside(at) {
                    continue;
                }
                if let Some((pl, e)) = ids.at(x as usize, y as usize) {
                    *wholes.entry(pl).or_insert(0) += 1;
                    *parts.entry((pl, e)).or_insert(0) += 1;
                }
            }
        }
        for (pl, n) in wholes {
            if let Some(w) = self.whole(pl) {
                out.push((w, n, self.wholes.get(&pl).copied().unwrap_or(n), true));
            }
        }
        for ((pl, e), n) in parts {
            let p = match self.g.fg_asm.is_some() {
                true => self.thing(pl, e).filter(|_| self.part(pl, e).is_some()),
                false => self.part(pl, e),
            };
            if let Some(p) = p {
                out.push((p, n, self.parts.get(&(pl, e)).copied().unwrap_or(n), false));
            }
        }
        out
    }
}

/// A picture made smaller, each pixel the mean of those it covers; RGB in, RGBA out.
fn shrink(rgb: &[u8], w: usize, h: usize, most: usize) -> Thumb {
    let s = (most as f32 / w.max(h) as f32).min(1.0);
    let (tw, th) = (((w as f32 * s) as usize).max(1), ((h as f32 * s) as usize).max(1));
    let mut rgba = Vec::with_capacity(tw * th * 4);
    for ty in 0..th {
        let (y0, y1) = (ty * h / th, ((ty + 1) * h / th).max(ty * h / th + 1));
        for tx in 0..tw {
            let (x0, x1) = (tx * w / tw, ((tx + 1) * w / tw).max(tx * w / tw + 1));
            let mut sum = [0u32; 3];
            for y in y0..y1 {
                for x in x0..x1 {
                    let p = &rgb[(y * w + x) * 3..][..3];
                    for c in 0..3 {
                        sum[c] += p[c] as u32;
                    }
                }
            }
            let n = ((y1 - y0) * (x1 - x0)) as u32;
            rgba.extend(sum.map(|c| (c / n) as u8));
            rgba.push(255);
        }
    }
    Thumb { w: tw, h: th, rgba }
}

/// Each occurrence's part as its own model names it, by the occurrence's path: the body the
/// names were found on, and its anchors.
pub(super) type OccNames = HashMap<String, (Arc<forge_topo::Body>, Arc<forge_lang::anchors::Anchors>)>;

/// A message with marks, taken, waiting on its names.
pub(super) struct Waiting {
    text: String,
    refs: Vec<String>,
    sent: Sent,
    /// The picture's top left in window points, and pixels per point.
    at: (Pos2, f32),
    png: Option<Vec<u8>>,
    ids: Option<ViewIds>,
    since: f64,
}

fn text_key(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

/// An assembly's occurrences named: the document built once more, as the view builds it (its
/// folder, the same fences), with each part's anchors found as it is loaded: no file is read here
/// but by the language's own loaders.
fn occ_names(src: &str, dir: Option<std::path::PathBuf>) -> OccNames {
    let opts = forge_lang::Options {
        dir,
        render: true,
        overlap: false,
        expects: false,
        checks: false,
        meshes: false,
        comp_anchors: true,
        ..Default::default()
    };
    let (run, _) = forge_lang::run(src, opts, Default::default());
    let mut out = OccNames::new();
    let Some(a) = &run.asm else { return out };
    let comps: HashMap<&str, Arc<forge_lang::anchors::Anchors>> = run
        .comp_anchors
        .iter()
        .map(|(f, a)| (f.as_str(), Arc::new(a.clone())))
        .collect();
    for (i, (path, file, _)) in a.occurrences.iter().enumerate() {
        let Some(body) = a.occ_part.get(i).copied().flatten().and_then(|k| a.parts.get(k)) else {
            continue;
        };
        let anchors = if file.starts_with('#') {
            // a body of a call (`#box(w=40)/base`) is named as its call names it
            comps
                .get(file.as_str())
                .or_else(|| file.rsplit_once('/').and_then(|(p, _)| comps.get(p)))
                .cloned()
        } else {
            a.occ_anchors.get(i).cloned().flatten()
        };
        if let Some(an) = anchors {
            out.insert(path.clone(), (body.clone(), an));
        }
    }
    out
}

impl Gui {
    /// The window as this frame drew it, again into a texture of its own: RGB rows from the top.
    fn window_rgb(&mut self, prims: &[egui::ClippedPrimitive], sd: &egui_wgpu::ScreenDescriptor) -> Vec<u8> {
        let [w, h] = sd.size_in_pixels;
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("markup picture"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = tex.create_view(&Default::default());
        let row = (w * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let read = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("markup read"),
            size: (row * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = enc
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("markup picture"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();
            self.egui.render(&mut pass, prims, sd);
        }
        enc.copy_texture_to_buffer(
            tex.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &read,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: None,
                },
            },
            tex.size(),
        );
        self.queue.submit([enc.finish()]);
        read.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        let bgra = matches!(
            self.format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );
        let map = read.slice(..).get_mapped_range().expect("it was waited for");
        let mut out = Vec::with_capacity((w * h * 3) as usize);
        for y in 0..h as usize {
            for px in map[y * row as usize..][..w as usize * 4].as_chunks::<4>().0 {
                if bgra {
                    out.extend_from_slice(&[px[2], px[1], px[0]]);
                } else {
                    out.extend_from_slice(&px[..3]);
                }
            }
        }
        drop(map);
        read.unmap();
        out
    }

    /// A message with marks, after the frame that shows them: the picture cropped to the view and
    /// the marks, and the view's ids, taken now; it goes once what's under the marks can be named
    /// (an assembly's occurrences' own names are found off the frame's thread, once a text).
    pub(super) fn markup_send(
        &mut self,
        (text, refs): (String, Vec<String>),
        prims: &[egui::ClippedPrimitive],
        sd: &egui_wgpu::ScreenDescriptor,
    ) {
        let t0 = now_ms();
        let marks = std::mem::take(&mut self.shell.agent.markup.marks);
        let ppp = sd.pixels_per_point;
        let [w, h] = sd.size_in_pixels.map(|v| v as usize);
        let rgb = self.window_rgb(prims, sd);
        let window = Rect::from_min_size(Pos2::ZERO, vec2(w as f32 / ppp, h as f32 / ppp));
        let crop = marks
            .iter()
            .fold(self.shell.view_rect, |r, m| r.union(m.bounds().expand(8.0)))
            .intersect(window);
        let (x0, y0) = ((crop.min.x * ppp) as usize, (crop.min.y * ppp) as usize);
        let (x1, y1) = (
            ((crop.max.x * ppp) as usize).min(w),
            ((crop.max.y * ppp) as usize).min(h),
        );
        let (cw, ch) = (x1.saturating_sub(x0).max(1), y1.saturating_sub(y0).max(1));
        let mut cut = Vec::with_capacity(cw * ch * 3);
        for y in y0..y0 + ch {
            cut.extend_from_slice(&rgb[(y * w + x0) * 3..][..cw * 3]);
        }
        let png = crate::agent::apptools::png(cw as u32, ch as u32, &cut).ok();
        let ids = match (&self.shell.drawing, &mut self.view) {
            (None, Some(v)) => {
                let cam = v.nav.camera;
                let ids = v.renderer.view_ids(&mut v.scene, &cam);
                v.dirty = true;
                Some(ids)
            }
            _ => None,
        };
        let sent = Sent {
            marks,
            camera: self
                .view
                .as_ref()
                .filter(|_| self.shell.drawing.is_none())
                .map(|v| v.nav.camera),
            thumb: Arc::new(shrink(&cut, cw, ch, THUMB_PX)),
            said: String::new(),
        };
        let at = (crop.min, ppp);
        eprintln!("forge-app: markup taken in {:.1} ms, {cw}×{ch} px", now_ms() - t0);
        self.markup_wait = Some(Box::new(Waiting {
            text,
            refs,
            sent,
            at,
            png,
            ids,
            since: now_ms(),
        }));
        self.markup_names();
        self.markup_ready();
    }

    /// The message waiting on its names, sent once they're found (or a while has gone by: then
    /// its occurrences go named as wholes).
    pub(super) fn markup_ready(&mut self) {
        let Some(w) = &self.markup_wait else { return };
        let asm = self.fg_asm.is_some() && self.shell.drawing.is_none();
        let key = self.fg.as_ref().map(|s| text_key(&s.src));
        let ready = !asm
            || self.markup_occ.as_ref().is_some_and(|o| Some(o.0) == key)
            || self.markup_occ_rx.is_none()
            || now_ms() - w.since > 20_000.0;
        if !ready {
            return;
        }
        let t0 = now_ms();
        let mut w = *self.markup_wait.take().expect("checked");
        self.shell.agent.markup.sending = false;
        let occ = self
            .markup_occ
            .as_ref()
            .filter(|o| Some(o.0) == key)
            .map(|o| o.1.clone());
        let (min, ppp) = w.at;
        let names = Names::new(
            self,
            w.ids.as_ref(),
            ppp,
            w.sent.marks.iter().any(Mark::region),
            occ.as_deref(),
        );
        let said = describe(&w.sent.marks, &names, &|p| {
            (
                ((p.x - min.x) * ppp).round() as i32,
                ((p.y - min.y) * ppp).round() as i32,
            )
        });
        eprintln!(
            "forge-app: markup named in {:.1} ms, {:.0} ms after it was taken",
            now_ms() - t0,
            now_ms() - w.since
        );
        w.sent.said = said.clone();
        if let Some(Entry::User { markup, .. }) = self
            .shell
            .agent
            .entries
            .iter_mut()
            .rev()
            .find(|e| matches!(e, Entry::User { .. }))
        {
            *markup = Some(w.sent);
        }
        let full = if w.text.is_empty() {
            said
        } else {
            format!("{}\n\n{said}", w.text)
        };
        self.agent_send(&full, &w.refs, w.png);
    }

    /// An assembly's occurrences named as their own models name them, found on a thread for the
    /// text open, kept till it changes: asked for when markup starts, so a send seldom waits.
    pub(super) fn markup_names(&mut self) {
        if let Some((key, rx)) = &self.markup_occ_rx {
            match rx.try_recv() {
                Ok(got) => {
                    self.markup_occ = Some((*key, Arc::new(got)));
                    self.markup_occ_rx = None;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => self.markup_occ_rx = None,
                Err(_) => {}
            }
        }
        let wanted = self.shell.agent.markup.on || self.markup_wait.is_some();
        if !wanted || self.fg_asm.is_none() || self.shell.drawing.is_some() {
            return;
        }
        let Some(s) = &self.fg else { return };
        let key = text_key(&s.src);
        if self.markup_occ.as_ref().is_some_and(|o| o.0 == key)
            || self.markup_occ_rx.as_ref().is_some_and(|r| r.0 == key)
        {
            return;
        }
        let (src, dir) = (
            s.src.clone(),
            s.path
                .as_deref()
                .and_then(|p| std::path::Path::new(p).parent().map(|d| d.to_path_buf())),
        );
        let (tx, rx) = std::sync::mpsc::channel();
        let waker = self.waker.clone();
        std::thread::Builder::new()
            .name("markup-names".into())
            .spawn(move || {
                let _ = tx.send(occ_names(&src, dir));
                waker();
            })
            .expect("a thread");
        self.markup_occ_rx = Some((key, rx));
    }

    /// The agent's screenshot of a drawing: the window's view area as this frame drew it.
    pub(super) fn window_shot(&mut self, prims: &[egui::ClippedPrimitive], sd: &egui_wgpu::ScreenDescriptor) {
        let ppp = sd.pixels_per_point;
        let [w, h] = sd.size_in_pixels.map(|v| v as usize);
        let rgb = self.window_rgb(prims, sd);
        let r = self.shell.view_rect;
        let (x0, y0) = (
            ((r.min.x * ppp) as usize).min(w),
            ((r.min.y * ppp) as usize).min(h),
        );
        let (cw, ch) = (
            (((r.max.x * ppp) as usize).min(w)).saturating_sub(x0).max(1),
            (((r.max.y * ppp) as usize).min(h)).saturating_sub(y0).max(1),
        );
        let mut cut = Vec::with_capacity(cw * ch * 3);
        for y in y0..(y0 + ch).min(h) {
            cut.extend_from_slice(&rgb[(y * w + x0) * 3..][..cw * 3]);
        }
        let reply = match crate::agent::apptools::png(cw as u32, (cut.len() / (cw * 3)) as u32, &cut) {
            Ok(png) => crate::agent::apptools::Reply {
                text: format!("The sheet as the person sees it, {cw}×{ch} px"),
                png: Some(png),
                error: false,
            },
            Err(e) => crate::agent::apptools::Reply::err(format!("couldn't make the picture: {e}")),
        };
        for r in std::mem::take(&mut self.apps.window_shots) {
            let _ = r.send(reply.clone());
        }
    }

    /// A sent message's marks shown again: the camera they were drawn over, back.
    pub(super) fn markup_show(&mut self, k: usize) {
        let cam = match self.shell.agent.entries.get(k) {
            Some(Entry::User { markup: Some(s), .. }) => s.camera,
            _ => None,
        };
        if let (Some(c), Some(v)) = (cam, &mut self.view)
            && self.shell.drawing.is_none()
        {
            v.nav.camera = c;
            v.dirty = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_thumbnail_is_the_mean_of_what_it_covers() {
        let rgb: Vec<u8> = (0..4 * 2)
            .flat_map(|k| if k % 2 == 0 { [0, 0, 0] } else { [200, 100, 50] })
            .collect();
        let t = shrink(&rgb, 4, 2, 2);
        assert_eq!((t.w, t.h), (2, 1));
        assert_eq!(&t.rgba[..4], &[100, 50, 25, 255]);
    }
}
