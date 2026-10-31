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
    js_sys::Date::now()
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
    #[cfg(not(target_arch = "wasm32"))]
    measure_of: Vec<(u32, crate::render::Entity)>,
    #[cfg(not(target_arch = "wasm32"))]
    measured: String,
    last_frame: f64,
    frame_ms: f64,
    parts: u32,
    tris: u64,
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
        let measured = self.measured_text();
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

    /// Between batches of events: asks what's under the cursor, and takes in the answer.
    pub fn about_to_wait(&mut self, on_loop: &ActiveEventLoop) {
        if !self.renderer.pick_busy()
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
        } else {
            ControlFlow::Wait
        });
    }
}
