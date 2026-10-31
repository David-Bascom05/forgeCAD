//! The window: one renderer, one camera, mouse to move it. The same on both targets.

use crate::camera::Camera;
use crate::nav::Nav;
use crate::render::{Overlay, Renderer};
use crate::scene::Scene;
use std::sync::Arc;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::Window;

/// Milliseconds on a clock that only moves forward.
#[cfg(not(target_arch = "wasm32"))]
pub fn now_ms() -> f64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START.get_or_init(std::time::Instant::now).elapsed().as_secs_f64() * 1e3
}

#[cfg(target_arch = "wasm32")]
pub fn now_ms() -> f64 {
    performance_now()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn say(s: &str) {
    println!("{s}");
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    pub fn say(s: &str);
    #[wasm_bindgen(js_namespace = performance, js_name = now)]
    fn performance_now() -> f64;
}

pub struct Viewer {
    pub window: Arc<Window>,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub renderer: Renderer,
    pub scene: Scene,
    pub nav: Nav,
    /// Runs on its own until this many frames of a fixed path are done, then reports (--bench).
    pub bench: Option<Bench>,
    /// Placements moving on their own: every frame draws them where they've got to.
    pub motion: Option<crate::scene::Motion>,
    motion_frame: usize,
    pick_ms: f64,
    #[cfg(not(target_arch = "wasm32"))]
    pub measure: crate::measure::Measurer,
    /// A value being driven by the mouse (--drag).
    #[cfg(not(target_arch = "wasm32"))]
    pub drag: Option<LiveDrag>,
    #[cfg(not(target_arch = "wasm32"))]
    measure_of: Vec<(u32, crate::render::Entity)>,
    #[cfg(not(target_arch = "wasm32"))]
    measured: String,
    last_frame: f64,
    frame_ms: f64,
    parts: u32,
    tris: u64,
}

/// What `--drag` adds to the window.
#[cfg(not(target_arch = "wasm32"))]
pub struct LiveDrag {
    pub live: forge_agent::live::Live<crate::live::Out>,
    pub drag: crate::live::Drag,
    /// The left button drags the value; off, it orbits as usual.
    pub on: bool,
    pub cursor_x: f64,
    pub from_x: f64,
    /// Frames asked for and not yet back.
    pub waiting: u64,
    pub last_seq: u64,
    /// `--drag-demo`: this many values to go, driven as a mouse would, then it reports and exits.
    pub demo: Option<usize>,
    /// The commit's full mesh has come.
    pub demo_done: bool,
}

pub struct Bench {
    pub path: crate::bench::Path,
    pub frame: usize,
    pub times: Vec<f64>,
    pub gpu: Vec<f64>,
    pub motion_ms: Vec<f64>,
    pub drawn: Vec<crate::render::Drawn>,
    /// Asks what's under a wandering pixel every frame, as hovering does.
    pub hover: bool,
    pub pick_ms: Vec<f64>,
}

impl Viewer {
    pub fn new(
        window: Arc<Window>,
        surface: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
        renderer: Renderer,
        scene: Scene,
        camera: Camera,
    ) -> Viewer {
        let size = (config.width, config.height);
        let nav = Nav::new(camera, size, window.scale_factor() as f32);
        Viewer {
            window,
            surface,
            config,
            renderer,
            scene,
            nav,
            bench: None,
            motion: None,
            motion_frame: 0,
            pick_ms: 0.0,
            #[cfg(not(target_arch = "wasm32"))]
            measure: crate::measure::Measurer::default(),
            #[cfg(not(target_arch = "wasm32"))]
            drag: None,
            #[cfg(not(target_arch = "wasm32"))]
            measure_of: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            measured: String::new(),
            last_frame: now_ms(),
            frame_ms: 0.0,
            parts: 0,
            tris: 0,
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.renderer.device, &self.config);
        self.renderer.resize((size.width, size.height));
    }

    fn draw(&mut self) {
        use wgpu::CurrentSurfaceTexture as T;
        let frame = match self.surface.get_current_texture() {
            T::Success(f) | T::Suboptimal(f) => f,
            T::Outdated | T::Lost => {
                self.surface.configure(&self.renderer.device, &self.config);
                return;
            }
            _ => return,
        };
        if let Some(b) = &self.bench {
            self.nav.camera = b.path.at(b.frame);
        }
        if self.nav.tick(now_ms()) {
            self.window.request_redraw();
        }
        if let Some(t) = self.nav.explode_tick(now_ms()) {
            self.scene.set_explode(t);
            self.window.request_redraw();
        }
        self.renderer.section = self.nav.section_plane();
        if let Some(b) = &mut self.bench
            && b.hover
            && !self.renderer.pick_busy()
        {
            let at = crate::bench::hover_at(b.frame, (self.config.width, self.config.height));
            self.renderer.pick_start(&mut self.scene, &self.nav.camera, at);
        }
        if let Some(m) = &self.motion {
            let t = now_ms();
            m.apply(self.motion_frame, &mut self.scene);
            self.motion_frame += 1;
            if let Some(b) = &mut self.bench {
                b.motion_ms.push(now_ms() - t);
            }
        }
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.config.format.add_srgb_suffix()),
            ..Default::default()
        });
        let now = now_ms();
        // a gap between frames (nothing moved) is no frame time
        if now - self.last_frame < 250.0 {
            self.frame_ms = now - self.last_frame;
        }
        self.last_frame = now;
        let gpu = self.renderer.gpu_ms.unwrap_or(0.0);
        let measured = self.drag_text().unwrap_or_else(|| self.measured_text());
        let hud = format!(
            "frame {:.1} ms gpu {gpu:.1} ms pick {:.1} ms {} parts {:.1}m tris lines {} {}\n{}",
            self.frame_ms,
            self.pick_ms,
            self.parts,
            self.tris as f64 / 1e6,
            self.renderer.line_mode.name(),
            self.nav.marked_text(),
            measured
        );
        let (at, side) = self.nav.cube_box();
        let marks = self.nav.marks();
        let overlay = Overlay {
            hud: Some(&hud),
            cube: Some((at, side, self.nav.cube_hover)),
            marks: &marks,
        };
        let drawn = self
            .renderer
            .frame(&view, &mut self.scene, &self.nav.camera, overlay);
        self.tris = drawn.tris;
        self.parts = drawn.placements;
        self.window.pre_present_notify();
        self.renderer.queue.present(frame);
        let _ = self.renderer.device.poll(wgpu::PollType::Poll);
        if let Some(b) = &mut self.bench {
            b.times.push(self.frame_ms);
            b.gpu.push(self.renderer.gpu_ms.unwrap_or(0.0) as f64);
            b.drawn.push(drawn);
            b.frame += 1;
        }
    }

    fn drag_demo_running(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        return self.drag.as_ref().is_some_and(|l| l.demo.is_some());
        #[cfg(target_arch = "wasm32")]
        false
    }

    /// After a commit, until the full mesh has come.
    fn drag_waiting_final(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        return self.renderer.live_shown();
        #[cfg(target_arch = "wasm32")]
        false
    }

    #[cfg(target_arch = "wasm32")]
    fn drag_text(&self) -> Option<String> {
        None
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn drag_text(&self) -> Option<String> {
        let l = self.drag.as_ref()?;
        let d = &l.drag;
        let mut t = format!(
            "{} = {} rebuild {:.1} mesh {:.1} upload {:.1} ms dropped {}",
            d.name,
            forge_agent::short(d.value),
            d.rebuild_ms,
            d.mesh_ms,
            d.upload_ms,
            l.live.dropped(),
        );
        if let Some(why) = &d.failure {
            t.push_str(" this value fails: ");
            t.push_str(why);
        }
        if !l.on {
            t.push_str(" (d: back to dragging)");
        }
        Some(t)
    }

    /// The left button goes down on a dragged value.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn drag_press(&mut self) {
        let Some(l) = &mut self.drag else { return };
        l.from_x = l.cursor_x;
        l.drag.start = l.drag.value;
        l.drag.dragging = true;
        self.nav.selected.clear();
    }

    /// The cursor moves: while dragging, a new value goes to the worker. Never waits.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn drag_move(&mut self, x: f64) {
        let Some(l) = &mut self.drag else { return };
        l.cursor_x = x;
        if l.drag.dragging {
            let v = l.drag.at_pixels(l.cursor_x - l.from_x);
            if v != l.drag.value {
                l.drag.value = v;
                l.last_seq = l.live.drag(l.drag.at.op(forge_agent::short(v)));
                l.waiting += 1;
                self.window.request_redraw();
            }
        }
    }

    /// The button comes up: the value is committed (the worker falls back on the last that
    /// built, if this one doesn't).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn drag_release(&mut self) {
        let Some(l) = &mut self.drag else { return };
        if l.drag.dragging {
            l.drag.dragging = false;
            l.drag.failure = None;
            l.live.commit(l.drag.at.op(forge_agent::short(l.drag.value)));
            l.waiting += 1;
        }
    }

    /// The mouse on a dragged value. True if the event was its.
    #[cfg(not(target_arch = "wasm32"))]
    fn drag_event(&mut self, event: &WindowEvent) -> bool {
        let Some(l) = &mut self.drag else {
            return false;
        };
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let dragging = l.drag.dragging;
                self.drag_move(position.x);
                dragging
            }
            WindowEvent::MouseInput {
                state,
                button: winit::event::MouseButton::Left,
                ..
            } if l.on => {
                if *state == ElementState::Pressed {
                    self.drag_press();
                } else {
                    self.drag_release();
                }
                true
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::KeyD) => {
                        l.on = !l.on;
                        self.window.request_redraw();
                        true
                    }
                    PhysicalKey::Code(KeyCode::Escape) if l.drag.dragging => {
                        l.drag.dragging = false;
                        l.drag.value = l.drag.start;
                        l.drag.failure = None;
                        l.live.cancel();
                        l.waiting += 1;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// What the worker has made since last time: changed faces go to the GPU, nothing else moves.
    #[cfg(not(target_arch = "wasm32"))]
    fn drag_poll(&mut self) {
        use crate::live::Out;
        use forge_agent::live::Event;
        let Some(l) = &mut self.drag else { return };
        let events = l.live.poll();
        if events.is_empty() {
            return;
        }
        let part = self.scene.placements[0].part;
        for e in events {
            match e {
                Event::Frame(f) => {
                    if f.seq > 0 {
                        l.waiting = l.waiting.saturating_sub(1);
                        l.drag.rebuild_ms = f.rebuild_ms + f.finish_ms;
                        l.drag.mesh_ms = f.mesh_ms;
                        l.drag.failure = f.failure.clone();
                    }
                    if let Some(Out::Delta(d)) = f.out {
                        let t = now_ms();
                        self.renderer.live_apply(part, &d);
                        l.drag.upload_ms = now_ms() - t;
                        if f.seq > 0 {
                            self.renderer.live_show(true);
                            self.scene.set_part(part, d.center, d.radius, vec![d.chord]);
                        }
                    }
                }
                Event::Done(d) => {
                    l.waiting = l.waiting.saturating_sub(1);
                    for line in &d.lines {
                        say(line);
                    }
                    say(&d.text);
                    // what it settled on: the committed line's value
                    if let Some(v) = d.lines.last().and_then(|x| x.rsplit(' ').next()?.parse().ok()) {
                        l.drag.value = v;
                    }
                }
                Event::Final(out) => {
                    if let Out::Whole(m) = out {
                        self.renderer.set_parts(std::slice::from_ref(&*m));
                        let chords = m.lods.iter().map(|x| x.chord).collect();
                        self.scene.set_part(part, m.center, m.radius, chords);
                    }
                    self.renderer.live_show(false);
                    l.demo_done = true;
                }
            }
        }
        self.window.request_redraw();
    }

    /// What the kernel says of the selection, as soon as it has said it.
    #[cfg(not(target_arch = "wasm32"))]
    fn measured_text(&mut self) -> String {
        let want = self.nav.selected.clone();
        if want != self.measure_of {
            self.measure_of = want.clone();
            self.measured.clear();
            self.measure.ask(want, &self.scene);
        }
        if let Some(text) = self.measure.answer() {
            self.measured = text;
        }
        if self.measure.waiting() {
            self.window.request_redraw();
        }
        self.measured.clone()
    }

    #[cfg(target_arch = "wasm32")]
    fn measured_text(&mut self) -> String {
        String::new()
    }

    /// What one window event does.
    pub fn event(&mut self, event: WindowEvent, on_loop: &ActiveEventLoop) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.drag_event(&event) {
            return;
        }
        if self.bench.is_none() && self.nav.event(&event) {
            self.window.request_redraw();
        }
        match event {
            WindowEvent::CloseRequested => on_loop.exit(),
            WindowEvent::Resized(size) => {
                self.resize(size);
                self.window.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    let changed = match code {
                        KeyCode::KeyL => {
                            self.renderer.line_mode = self.renderer.line_mode.next();
                            true
                        }
                        KeyCode::KeyO => {
                            self.renderer.ao = !self.renderer.ao;
                            true
                        }
                        _ => self.nav.key(code, &self.scene),
                    };
                    if changed {
                        self.window.request_redraw();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.draw();
                if let Some(b) = &self.bench {
                    if b.frame >= b.path.frames {
                        let mode = format!("window {}x{}", self.config.width, self.config.height);
                        let out = crate::bench::report(&mode, &b.times, &b.gpu, &self.renderer.adapter);
                        say(&out);
                        let moved = self.motion.as_ref().map_or(0, |m| m.len());
                        let times = if b.motion_ms.is_empty() {
                            vec![0.0]
                        } else {
                            b.motion_ms.clone()
                        };
                        say(&crate::bench::report_cpu(moved, &times, &b.drawn));
                        if !b.pick_ms.is_empty() {
                            say(&crate::bench::report_pick(&b.pick_ms));
                        }
                        self.bench = None;
                        on_loop.exit();
                        return;
                    }
                    self.window.request_redraw();
                }
                if self.motion.is_some() {
                    self.window.request_redraw();
                }
            }
            _ => {}
        }
    }

    /// `--drag-demo`: one more value, a frame apart, as a mouse would give them.
    #[cfg(not(target_arch = "wasm32"))]
    fn drag_demo(&mut self, on_loop: &ActiveEventLoop) {
        let Some(l) = &mut self.drag else { return };
        let Some(left) = l.demo else { return };
        let total = 120.0;
        if left > 0 {
            if now_ms() - self.last_frame < 16.0 {
                return;
            }
            let x = 40.0 * forge_math::dmath::sin((total - left as f64) / total * core::f64::consts::TAU);
            l.demo = Some(left - 1);
            if !l.drag.dragging {
                self.drag_press();
            }
            self.drag_move(x);
            self.window.request_redraw();
        } else if l.drag.dragging {
            self.drag_release();
        } else if l.demo_done {
            say(&format!(
                "demo done: {} values, {} dropped, last frame rebuild {:.1} mesh {:.1} upload {:.1} ms, {:.0} ms in all",
                l.last_seq,
                l.live.dropped(),
                l.drag.rebuild_ms,
                l.drag.mesh_ms,
                l.drag.upload_ms,
                now_ms()
            ));
            on_loop.exit();
        }
    }

    /// Between batches of events: asks what's under the cursor, and takes in the answer.
    pub fn about_to_wait(&mut self, on_loop: &ActiveEventLoop) {
        #[cfg(not(target_arch = "wasm32"))]
        self.drag_demo(on_loop);
        #[cfg(not(target_arch = "wasm32"))]
        self.drag_poll();
        // what a pick would read is the mesh from before the drag
        #[cfg(not(target_arch = "wasm32"))]
        let dragged = self
            .drag
            .as_ref()
            .is_some_and(|l| l.drag.dragging || l.waiting > 0);
        #[cfg(target_arch = "wasm32")]
        let dragged = false;
        if dragged {
            self.nav.take_pick();
        } else if !self.renderer.pick_busy()
            && let Some(px) = self.nav.take_pick()
        {
            self.renderer.pick_start(&mut self.scene, &self.nav.camera, px);
        }
        if let Some((px, hit, ms)) = self.renderer.pick_poll(&self.scene) {
            self.pick_ms = ms;
            if self.nav.on_pick(px, hit) {
                self.window.request_redraw();
            }
            if let Some(b) = &mut self.bench {
                b.pick_ms.push(ms);
            }
        }
        on_loop.set_control_flow(if self.renderer.pick_busy() {
            ControlFlow::Poll
        } else if dragged || self.drag_waiting_final() || self.drag_demo_running() {
            // the worker has no way to wake the loop: look again very soon
            ControlFlow::wait_duration(std::time::Duration::from_millis(1))
        } else {
            ControlFlow::Wait
        });
    }
}
