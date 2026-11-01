//! Chrome and viewport in one frame: the shell over the part's picture, the live worker behind
//! them. The host (a window, or `Headless` here) gives input and a target. Nothing here runs
//! unless the host asks for a frame, and the host asks only when something changed.

use crate::camera::Camera;
use crate::live::{self, Out};
use crate::mesh::PartMesh;
use crate::nav::Nav;
use crate::render::{self, Offscreen, Overlay, Renderer, part_info};
use crate::scene::{Motion, Placement, Scene};
use crate::shell::{Action, Shell};
use crate::viewcube;
use crate::viewer::now_ms;
use forge_agent::live::{Event, Live};
use forge_agent::tools::{Call, Part};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, WindowEvent};

pub type Waker = Arc<dyn Fn() + Send + Sync>;

/// The formats a target can be: blending in sRGB, as a browser's. The viewport's picture is sampled
/// as stored.
pub const HEADLESS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

struct Loaded {
    name: String,
    mesh: PartMesh,
    part: Part,
}

/// The part in view.
pub struct View {
    pub renderer: Renderer,
    pub scene: Scene,
    pub nav: Nav,
    target: Offscreen,
    tex: egui::TextureId,
    size: (u32, u32),
    /// Its picture is out of date.
    dirty: bool,
    pub live: Live<Out>,
    pub motion: Option<Motion>,
}

/// A frame's times, in milliseconds.
#[derive(Clone, Copy, Debug, Default)]
pub struct Times {
    pub ui: f64,
    pub tess: f64,
    pub view: f64,
    pub draw: f64,
    pub view_drawn: bool,
}

pub struct FrameOut {
    pub platform: egui::PlatformOutput,
    /// What the host must do, the window's things.
    pub host: Vec<Action>,
    /// Draw again after this long; `None`: only when something happens.
    pub again_in: Option<std::time::Duration>,
    pub times: Times,
}

pub struct Gui {
    pub ctx: egui::Context,
    egui: egui_wgpu::Renderer,
    pub shell: Shell,
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter: wgpu::AdapterInfo,
    pub view: Option<View>,
    loading: Option<Receiver<Result<Loaded, String>>>,
    waker: Waker,
    /// The view holds the pointer: a press began in it.
    captured: bool,
    over_view: bool,
    /// Tool calls made, for replay.
    pub calls: Vec<Call>,
    pixels_per_point: f32,
    /// The window, in device pixels.
    target_px: (u32, u32),
    /// The GPU refused something: said once, and no more views are drawn.
    gpu_failed: Arc<std::sync::Mutex<Option<String>>>,
}

fn view_camera(center: [f32; 3], radius: f32, parallel: bool, size: (u32, u32)) -> Camera {
    let mut c = Camera::fit(center, radius);
    c.yaw = -35f32.to_radians();
    if parallel {
        c.set_parallel(true, radius);
    }
    c.fit_to(radius * 1.1, size.0, size.1);
    c
}

fn name_of(path: &str) -> String {
    let n = path.rsplit('/').next().unwrap_or(path);
    if n.ends_with(".fc") {
        n.to_string()
    } else {
        format!("{n}.fc")
    }
}

impl View {
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

impl Gui {
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        adapter: wgpu::AdapterInfo,
        format: wgpu::TextureFormat,
        waker: Waker,
    ) -> Gui {
        // never panic from a thread the GPU reports on: say it, and the window stays up
        let gpu_failed: Arc<std::sync::Mutex<Option<String>>> = Arc::default();
        let failed = gpu_failed.clone();
        device.on_uncaptured_error(Arc::new(move |e| {
            eprintln!("forge-app: GPU error: {e}");
            let mut f = failed.lock().expect("no panics while held");
            f.get_or_insert_with(|| e.to_string());
        }));
        let ctx = egui::Context::default();
        ctx.set_fonts(crate::shell::theme::fonts());
        let egui = egui_wgpu::Renderer::new(&device, format, egui_wgpu::RendererOptions::default());
        Gui {
            ctx,
            egui,
            shell: Shell::default(),
            device,
            queue,
            adapter,
            view: None,
            loading: None,
            waker,
            captured: false,
            over_view: false,
            calls: Vec::new(),
            pixels_per_point: 1.0,
            target_px: (1, 1),
            gpu_failed,
        }
    }

    /// Opens a part in the background: the window never waits on the kernel.
    pub fn open(&mut self, path: &str) {
        let (send, recv) = channel();
        self.loading = Some(recv);
        let (path, waker) = (path.to_string(), self.waker.clone());
        self.shell.status = format!("Opening {}", name_of(&path));
        std::thread::spawn(move || {
            let got = (|| {
                let (mesh, _) = crate::load::open(&path, true)?;
                let part = crate::load::tool_part(&path)?;
                Ok(Loaded {
                    name: name_of(&path),
                    mesh,
                    part,
                })
            })();
            let _ = send.send(got);
            waker();
        });
    }

    fn take_loaded(&mut self) {
        // the view takes its size from the layout: nothing to allocate until there is one
        if self.shell.view_rect.area() <= 0.0 {
            return;
        }
        let Some(r) = &self.loading else { return };
        let Ok(got) = r.try_recv() else { return };
        self.loading = None;
        let l = match got {
            Ok(l) => l,
            Err(why) => {
                self.shell.status = why;
                return;
            }
        };
        let parts = [l.mesh];
        let mut scene = Scene::new(&part_info(&parts), vec![Placement::at(0, [0.0; 3])]);
        let (center, radius) = scene.bounds();
        scene.set_explode(0.0);
        let size = self.view_px();
        eprintln!(
            "forge-app: opening {} with a view of {}x{}",
            l.name, size.0, size.1
        );
        let camera = view_camera(center, radius, self.shell.view.parallel, size);
        let mut renderer = Renderer::new(
            self.device.clone(),
            self.queue.clone(),
            self.adapter.clone(),
            render::OFFSCREEN,
            size,
            &parts,
            &scene,
        );
        self.theme_view(&mut renderer);
        let target = Offscreen::new(&self.device, size);
        let raw = raw_view(&target);
        let tex = self
            .egui
            .register_native_texture(&self.device, &raw, wgpu::FilterMode::Nearest);
        let mut live = live::start(l.part);
        let w = self.waker.clone();
        live.on_ready(move || w());
        let mut nav = Nav::new(camera, size, self.pixels_per_point);
        nav.tool = self.shell.view.tool;
        self.view = Some(View {
            renderer,
            scene,
            nav,
            target,
            tex,
            size,
            dirty: true,
            live,
            motion: None,
        });
        self.shell.view_tex = Some(tex);
        self.shell.name = l.name;
        self.shell.status.clear();
    }

    fn theme_view(&self, r: &mut Renderer) {
        let t = self.shell.theme;
        r.back_colour = viewcube::rgb(t.view);
        r.cube_palette = if t.dark { viewcube::DARK } else { viewcube::LIGHT };
    }

    /// The viewport's size in device pixels, at least one.
    fn view_px(&self) -> (u32, u32) {
        let v = self.shell.view_rect;
        let p = self.pixels_per_point;
        (
            ((v.width() * p).round() as u32).max(1),
            ((v.height() * p).round() as u32).max(1),
        )
    }

    /// What the worker has finished.
    fn take_live(&mut self) {
        let Some(v) = &mut self.view else { return };
        let events = v.live.poll();
        let part = v.scene.placements[0].part;
        for e in events {
            match e {
                Event::Doc(d) => self.shell.set_doc(&d.src, d.version, d.failed),
                Event::Done(d) => {
                    // the reply's first line says it
                    self.shell.status = d.text.lines().next().unwrap_or("").to_string();
                }
                Event::Frame(f) => {
                    if let Some(Out::Delta(d)) = f.out {
                        v.renderer.live_apply(part, &d);
                    }
                }
                Event::Final(out) => {
                    if let Out::Whole(m) = out {
                        v.renderer.set_parts(std::slice::from_ref(&*m));
                        let chords = m.lods.iter().map(|x| x.chord).collect();
                        v.scene.set_part(part, m.center, m.radius, chords);
                    }
                    v.renderer.live_show(false);
                    v.dirty = true;
                }
            }
        }
    }

    /// What one window event means for the view, if it's the view's.
    pub fn view_event(&mut self, event: &WindowEvent) {
        let Some(v) = &mut self.view else { return };
        let p = self.pixels_per_point;
        let origin = self.shell.view_rect.min;
        let to_view =
            |x: f64, y: f64| PhysicalPosition::new(x - (origin.x * p) as f64, y - (origin.y * p) as f64);
        let forward = match event {
            WindowEvent::ModifiersChanged(_) => {
                v.nav.event(event);
                return;
            }
            WindowEvent::CursorMoved { device_id, position } => {
                let at = egui::pos2(position.x as f32 / p, position.y as f32 / p);
                let over = self.shell.view_rect.contains(at)
                    && !self.shell.menu_open()
                    && self
                        .ctx
                        .layer_id_at(at)
                        .is_none_or(|l| l.order == egui::Order::Background);
                if self.captured || over {
                    self.over_view = over;
                    Some(WindowEvent::CursorMoved {
                        device_id: *device_id,
                        position: to_view(position.x, position.y),
                    })
                } else if std::mem::take(&mut self.over_view) {
                    Some(WindowEvent::CursorLeft {
                        device_id: *device_id,
                    })
                } else {
                    None
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.over_view = false;
                self.captured = false;
                Some(event.clone())
            }
            WindowEvent::MouseInput { state, .. } => match state {
                ElementState::Pressed if self.over_view => {
                    self.captured = true;
                    Some(event.clone())
                }
                ElementState::Released if self.captured => {
                    self.captured = false;
                    Some(event.clone())
                }
                _ => None,
            },
            WindowEvent::MouseWheel { .. } if self.over_view => Some(event.clone()),
            _ => None,
        };
        if let Some(e) = forward {
            if v.nav.event(&e) {
                v.dirty = true;
            }
            if v.nav.home_if_asked(&v.scene) {
                v.dirty = true;
            }
            if let Some(at) = v.nav.cube_menu.take() {
                self.shell.cube_menu = Some(egui::pos2(origin.x + at.0 / p, origin.y + at.1 / p));
            }
        }
    }

    /// A key, if no field has the keyboard.
    pub fn view_key(&mut self, code: winit::keyboard::KeyCode) -> bool {
        if self.ctx.egui_wants_keyboard_input() {
            return false;
        }
        let Some(v) = &mut self.view else { return false };
        use winit::keyboard::KeyCode as K;
        let changed = match code {
            K::KeyL => {
                v.renderer.line_mode = v.renderer.line_mode.next();
                self.shell.view.lines = v.renderer.line_mode;
                true
            }
            K::KeyO => {
                v.renderer.ao = !v.renderer.ao;
                true
            }
            _ => v.nav.key(code, &v.scene),
        };
        self.shell.view.section = v.nav.section.is_some();
        v.dirty |= changed;
        changed
    }

    /// Asks for what's under the cursor, and takes in the answer. True if that changed anything.
    pub fn picks(&mut self) -> bool {
        let Some(v) = &mut self.view else { return false };
        if !v.renderer.pick_busy()
            && let Some(px) = v.nav.take_pick()
        {
            v.renderer.pick_start(&mut v.scene, &v.nav.camera, px);
        }
        if let Some((px, hit, _)) = v.renderer.pick_poll(&v.scene)
            && v.nav.on_pick(px, hit)
        {
            v.dirty = true;
            return true;
        }
        false
    }

    pub fn pick_busy(&self) -> bool {
        self.view.as_ref().is_some_and(|v| v.renderer.pick_busy())
    }

    /// The tool calls and view changes the chrome asked for; the window's things go to the host.
    fn apply(&mut self) -> Vec<Action> {
        let mut host = Vec::new();
        for a in std::mem::take(&mut self.shell.out) {
            match a {
                Action::Undo | Action::Redo => {
                    let call = if a == Action::Undo { Call::Undo } else { Call::Redo };
                    if let Some(v) = &mut self.view {
                        self.calls.push(call.clone());
                        v.live.call(vec![call]);
                    }
                }
                Action::FitAll
                | Action::Home
                | Action::SetHome
                | Action::Parallel(_)
                | Action::Lines(_)
                | Action::Section
                | Action::Tool(_) => {
                    self.apply_view(a);
                }
                other => host.push(other),
            }
        }
        host
    }

    fn apply_view(&mut self, a: Action) {
        if let Action::Tool(t) = a {
            self.shell.view.tool = t;
        }
        if let Action::Parallel(on) = a {
            self.shell.view.parallel = on;
        }
        let Some(v) = &mut self.view else { return };
        match a {
            Action::FitAll => v.nav.fit_all(&v.scene),
            Action::Home => v.nav.home(&v.scene),
            Action::SetHome => v.nav.home_view = Some(v.nav.camera),
            Action::Parallel(on) => v.nav.set_parallel(on, &v.scene),
            Action::Lines(mode) => {
                v.renderer.line_mode = mode;
                self.shell.view.lines = mode;
            }
            Action::Section => {
                v.nav.key(winit::keyboard::KeyCode::KeyS, &v.scene);
                self.shell.view.section = v.nav.section.is_some();
            }
            Action::Tool(t) => v.nav.tool = t,
            _ => {}
        }
        v.dirty = true;
    }

    /// One whole frame into `target`, `size` device pixels.
    pub fn frame(&mut self, raw: egui::RawInput, target: &wgpu::TextureView, size: (u32, u32)) -> FrameOut {
        let t0 = now_ms();
        let mut times = Times::default();
        self.target_px = size;
        if let Some(why) = self.gpu_failed.lock().expect("no panics while held").as_ref() {
            self.shell.status = format!("GPU error: {}", why.lines().next().unwrap_or(""));
            self.view = None;
            self.shell.view_tex = None;
        }
        self.take_loaded();
        self.take_live();
        let mut animating = false;
        if let Some(v) = &mut self.view {
            if v.nav.tick(now_ms()) {
                v.dirty = true;
                animating = true;
            }
            if let Some(t) = v.nav.explode_tick(now_ms()) {
                v.scene.set_explode(t);
                v.dirty = true;
                animating = true;
            }
            self.shell.cube_labels = viewcube::labels(&v.nav.camera, &v.nav.cube_place(), v.nav.cube_hover);
        }
        let shell = &mut self.shell;
        let output = self.ctx.run_ui(raw, |ui| shell.ui(ui));
        self.pixels_per_point = output.pixels_per_point;
        let host = self.apply();
        times.ui = now_ms() - t0;

        let want_view = self.view_px();
        if let Some(v) = &mut self.view {
            if want_view != v.size {
                eprintln!(
                    "forge-app: view {}x{} in a window of {}x{}",
                    want_view.0, want_view.1, size.0, size.1
                );
                v.size = want_view;
                v.renderer.resize(want_view);
                v.target = Offscreen::new(&self.device, want_view);
                let raw = raw_view(&v.target);
                self.egui.update_egui_texture_from_wgpu_texture(
                    &self.device,
                    &raw,
                    wgpu::FilterMode::Nearest,
                    v.tex,
                );
                v.nav.size = want_view;
                v.dirty = true;
            }
            v.nav.scale = self.pixels_per_point;
        }
        if let Some(mut v) = self.view.take() {
            self.theme_view(&mut v.renderer);
            self.view = Some(v);
        }
        let t1 = now_ms();
        if let Some(v) = &mut self.view
            && v.dirty
        {
            v.renderer.section = v.nav.section_plane();
            let marks = v.nav.marks();
            let overlay = Overlay {
                hud: None,
                cube: Some((v.nav.cube_place(), v.nav.cube_hover)),
                marks: &marks,
            };
            v.renderer
                .frame(&v.target.view, &mut v.scene, &v.nav.camera, overlay);
            v.dirty = false;
            times.view_drawn = true;
        }
        times.view = now_ms() - t1;

        let t2 = now_ms();
        let prims = self.ctx.tessellate(output.shapes, output.pixels_per_point);
        times.tess = now_ms() - t2;
        let t3 = now_ms();
        for (id, deltas) in &output.textures_delta.set {
            for d in deltas.iter() {
                self.egui.update_texture(&self.device, &self.queue, *id, d);
            }
        }
        let sd = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [size.0, size.1],
            pixels_per_point: output.pixels_per_point,
        };
        let mut enc = self.device.create_command_encoder(&Default::default());
        let more = self
            .egui
            .update_buffers(&self.device, &self.queue, &mut enc, &prims, &sd);
        {
            let mut pass = enc
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("window"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target,
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
            self.egui.render(&mut pass, &prims, &sd);
        }
        self.queue.submit(more.into_iter().chain([enc.finish()]));
        for id in &output.textures_delta.free {
            self.egui.free_texture(id);
        }
        times.draw = now_ms() - t3;

        let delay = output
            .viewport_output
            .values()
            .next()
            .map(|v| v.repaint_delay)
            .unwrap_or(std::time::Duration::MAX);
        let again_in = if animating || self.view.as_ref().is_some_and(|v| v.dirty) {
            Some(std::time::Duration::ZERO)
        } else if delay == std::time::Duration::MAX {
            None
        } else {
            Some(delay)
        };
        FrameOut {
            platform: output.platform_output,
            host,
            again_in,
            times,
        }
    }

    pub fn device(&self) -> wgpu::Device {
        self.device.clone()
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn view_dirty(&self) -> bool {
        self.view.as_ref().is_some_and(|v| v.dirty)
    }

    /// Is a load or a worker job still out? Only tests and shots need to ask.
    pub fn loading(&self) -> bool {
        self.loading.is_some()
    }
}

fn raw_view(t: &Offscreen) -> wgpu::TextureView {
    t.texture.create_view(&wgpu::TextureViewDescriptor {
        format: Some(wgpu::TextureFormat::Rgba8Unorm),
        ..Default::default()
    })
}

/// The app with no window: input by hand, a texture for a target. What `--shot`, `--bench` and the
/// tests run. Clicks go by the shell's own names for what it drew.
pub struct Headless {
    pub gui: Gui,
    size: (f32, f32),
    scale: f32,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    device: wgpu::Device,
    queue: wgpu::Queue,
    time: f64,
    pending: Vec<egui::Event>,
    pointer: egui::Pos2,
}

impl Headless {
    pub fn new(size: (f32, f32), scale: f32) -> Result<Headless, String> {
        let instance = render::native_instance();
        let (device, queue, adapter) = render::block_on(render::open_device(&instance, None))?;
        let px = ((size.0 * scale).round() as u32, (size.1 * scale).round() as u32);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("window"),
            size: wgpu::Extent3d {
                width: px.0,
                height: px.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: HEADLESS_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let gui = Gui::new(
            device.clone(),
            queue.clone(),
            adapter.get_info(),
            HEADLESS_FORMAT,
            Arc::new(|| {}),
        );
        Ok(Headless {
            gui,
            size,
            scale,
            texture,
            view,
            device,
            queue,
            time: 0.0,
            pending: Vec::new(),
            pointer: egui::Pos2::ZERO,
        })
    }

    fn px(&self) -> (u32, u32) {
        (
            (self.size.0 * self.scale).round() as u32,
            (self.size.1 * self.scale).round() as u32,
        )
    }

    /// One frame with what's been queued; waits for the GPU, so a time is a real one.
    pub fn step(&mut self) -> FrameOut {
        self.gui.ctx.set_pixels_per_point(self.scale);
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(self.size.0, self.size.1),
            )),
            time: Some(self.time),
            focused: true,
            events: std::mem::take(&mut self.pending),
            ..Default::default()
        };
        self.time += 1.0 / 60.0;
        let view = self.view.clone();
        let px = self.px();
        let out = self.gui.frame(raw, &view, px);
        self.gui.picks();
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        self.gui.picks();
        out
    }

    /// Frames until the view's part is in and the worker has said what it is.
    pub fn settle(&mut self) {
        for _ in 0..20_000 {
            self.step();
            if !self.gui.loading() && self.gui.shell.doc.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    pub fn move_to(&mut self, x: f32, y: f32) {
        let p = egui::pos2(x, y);
        self.pointer = p;
        self.pending.push(egui::Event::PointerMoved(p));
        let px = PhysicalPosition::new((x * self.scale) as f64, (y * self.scale) as f64);
        self.gui.view_event(&WindowEvent::CursorMoved {
            device_id: winit::event::DeviceId::dummy(),
            position: px,
        });
    }

    pub fn button(&mut self, down: bool) {
        self.pending.push(egui::Event::PointerButton {
            pos: self.pointer,
            button: egui::PointerButton::Primary,
            pressed: down,
            modifiers: Default::default(),
        });
        self.gui.view_event(&WindowEvent::MouseInput {
            device_id: winit::event::DeviceId::dummy(),
            state: if down {
                ElementState::Pressed
            } else {
                ElementState::Released
            },
            button: MouseButton::Left,
        });
    }

    /// Queues a click at the middle of what the shell last drew under this name.
    pub fn click(&mut self, name: &str) -> bool {
        let Some(r) = self.gui.shell.place(name) else {
            return false;
        };
        self.move_to(r.center().x, r.center().y);
        // egui takes hover first, as a real pointer gives it
        self.step();
        self.button(true);
        self.step();
        self.button(false);
        true
    }

    /// The frame as PPM bytes.
    pub fn ppm(&self) -> Vec<u8> {
        let (w, h) = self.px();
        let row = (w * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (row * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut e = self.device.create_command_encoder(&Default::default());
        e.copy_texture_to_buffer(
            self.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([e.finish()]);
        buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        let v = buffer.slice(..).get_mapped_range().expect("mapped");
        let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
        for y in 0..h as usize {
            for x in 0..w as usize {
                let k = y * row as usize + x * 4;
                out.extend_from_slice(&v[k..k + 3]);
            }
        }
        out
    }
}
