//! The window: one renderer, one camera, mouse to move it. The same on both targets.

use crate::camera::Camera;
use crate::render::Renderer;
use crate::scene::Scene;
use std::sync::Arc;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
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
    pub camera: Camera,
    /// Runs on its own until this many frames of a fixed path are done, then reports (--bench).
    pub bench: Option<Bench>,
    held: Option<MouseButton>,
    last_mouse: Option<(f32, f32)>,
    last_frame: f64,
    frame_ms: f64,
    tris: u64,
}

pub struct Bench {
    pub path: crate::bench::Path,
    pub frame: usize,
    pub times: Vec<f64>,
    pub gpu: Vec<f64>,
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
        Viewer {
            window,
            surface,
            config,
            renderer,
            scene,
            camera,
            bench: None,
            held: None,
            last_mouse: None,
            last_frame: now_ms(),
            frame_ms: 0.0,
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
        if let Some(b) = &mut self.bench {
            self.camera = b.path.at(b.frame);
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
        let hud = format!(
            "frame {:.1} ms gpu {gpu:.1} ms {} parts {:.1}m tris",
            self.frame_ms,
            self.scene.visible.len(),
            self.tris as f64 / 1e6
        );
        self.tris = self
            .renderer
            .frame(&view, &mut self.scene, &self.camera, Some(&hud))
            .tris;
        self.window.pre_present_notify();
        self.renderer.queue.present(frame);
        let _ = self.renderer.device.poll(wgpu::PollType::Poll);
        if let Some(b) = &mut self.bench {
            b.times.push(self.frame_ms);
            b.gpu.push(self.renderer.gpu_ms.unwrap_or(0.0) as f64);
            b.frame += 1;
        }
    }

    /// What one window event does.
    pub fn event(&mut self, event: WindowEvent, on_loop: &ActiveEventLoop) {
        match event {
            WindowEvent::CloseRequested => on_loop.exit(),
            WindowEvent::Resized(size) => {
                self.resize(size);
                self.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                self.draw();
                if let Some(b) = &self.bench {
                    if b.frame >= b.path.frames {
                        let mode = format!("window {}x{}", self.config.width, self.config.height);
                        let out = crate::bench::report(&mode, &b.times, &b.gpu, &self.renderer.adapter);
                        say(&out);
                        self.bench = None;
                        on_loop.exit();
                        return;
                    }
                    self.window.request_redraw();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.held = (state == ElementState::Pressed).then_some(button);
                self.last_mouse = None;
            }
            WindowEvent::CursorMoved { position, .. } => {
                let at = (position.x as f32, position.y as f32);
                if let (Some(button), Some(last)) = (self.held, self.last_mouse) {
                    let (dx, dy) = (at.0 - last.0, at.1 - last.1);
                    match button {
                        MouseButton::Left => self.camera.orbit(dx, dy),
                        _ => self.camera.pan(dx, dy, self.config.height),
                    }
                    self.window.request_redraw();
                }
                self.last_mouse = Some(at);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let steps = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                self.camera.zoom(steps);
                self.window.request_redraw();
            }
            _ => {}
        }
    }
}
