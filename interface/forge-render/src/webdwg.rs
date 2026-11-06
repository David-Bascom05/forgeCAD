//! The browser's DWG document: the shell's AutoCAD layout (`shell/dwgdoc_ui.rs`) on a canvas,
//! egui fed from winit's events here (egui-winit is native only). The page hands over a file's
//! bytes (web/index.html: a file picker, a dropped file, or `?dwg=URL`); Open asks the page for
//! another, Save hands the page the bytes to download. `dwg_offscreen` draws one frame to a
//! texture, for where the browser can't show a WebGPU canvas (see research/render).

use crate::dwgview::gpu::Target;
use crate::render::open_device;
use crate::shell::docs::{DocKind, DocTab};
use crate::shell::dwgdoc::DwgDoc;
use crate::shell::{Action, Shell};
use crate::viewer::{now_ms, say};
use js_sys::{Promise, Uint8Array};
use std::cell::RefCell;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, NamedKey};
use winit::platform::web::{EventLoopExtWebSys, WindowAttributesExtWebSys};
use winit::window::{Window, WindowId};

#[wasm_bindgen]
extern "C" {
    /// The page's: a download of these bytes under this name.
    #[wasm_bindgen(js_namespace = window, js_name = forge_save)]
    fn page_save(name: &str, bytes: &[u8]);
    /// The page's: its file picker, which calls `open_dwg` with what's picked.
    #[wasm_bindgen(js_namespace = window, js_name = forge_pick)]
    fn page_pick();
}

thread_local! {
    /// Files the page handed over, for the loop to open.
    static INBOX: RefCell<Vec<(String, Vec<u8>)>> = const { RefCell::new(Vec::new()) };
    static WAKE: RefCell<Option<EventLoopProxy<Msg>>> = const { RefCell::new(None) };
}

/// The shell, egui and what draws them: shared by the canvas and the offscreen frame.
struct Ui {
    ctx: egui::Context,
    egui: egui_wgpu::Renderer,
    shell: Shell,
    device: wgpu::Device,
    queue: wgpu::Queue,
    events: Vec<egui::Event>,
    modifiers: egui::Modifiers,
    pointer: egui::Pos2,
    /// The last frame's shell, in ms.
    ui_ms: f64,
}

impl Ui {
    fn new(device: wgpu::Device, queue: wgpu::Queue, format: wgpu::TextureFormat) -> Ui {
        let ctx = egui::Context::default();
        ctx.set_fonts(crate::shell::theme::fonts());
        let mut egui = egui_wgpu::Renderer::new(&device, format, egui_wgpu::RendererOptions::default());
        egui.callback_resources.insert(Target(format));
        Ui {
            ctx,
            egui,
            shell: Shell::default(),
            device,
            queue,
            events: vec![],
            modifiers: Default::default(),
            pointer: egui::Pos2::ZERO,
            ui_ms: 0.0,
        }
    }

    /// A file's bytes as the document shown; why not, in the status bar.
    fn open(&mut self, name: &str, bytes: &[u8]) {
        match DwgDoc::from_bytes(None, name, bytes) {
            Ok(d) => {
                say(&format!(
                    "forge-render: opened {name}, {} items",
                    d.geom.n_items()
                ));
                self.shell.open_dwg(d);
                self.shell.doc_tabs = vec![DocTab {
                    title: name.to_string(),
                    dirty: false,
                    kind: DocKind::Dwg,
                }];
                self.shell.active_doc = 0;
            }
            Err(e) => {
                say(&format!("forge-render: {e}"));
                self.shell.status = e;
            }
        }
    }

    /// One frame of the shell into `target` (size in device pixels); whether to draw again now.
    fn frame(&mut self, target: &wgpu::TextureView, size: (u32, u32), ppp: f32) -> (bool, egui::CursorIcon) {
        self.ctx.set_pixels_per_point(ppp);
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(size.0 as f32 / ppp, size.1 as f32 / ppp),
            )),
            time: Some(now_ms() / 1000.0),
            focused: true,
            events: std::mem::take(&mut self.events),
            ..Default::default()
        };
        let shell = &mut self.shell;
        let t = now_ms();
        let mut out = self.ctx.run_ui(raw, |ui| shell.ui(ui));
        self.ui_ms = now_ms() - t;
        self.asks();
        let prims = self
            .ctx
            .tessellate(std::mem::take(&mut out.shapes), out.pixels_per_point);
        for (id, deltas) in &out.textures_delta.set {
            for d in deltas.iter() {
                self.egui.update_texture(&self.device, &self.queue, *id, d);
            }
        }
        let sd = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [size.0, size.1],
            pixels_per_point: out.pixels_per_point,
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
        for id in &out.textures_delta.free {
            self.egui.free_texture(id);
        }
        let again = out
            .viewport_output
            .values()
            .next()
            .is_some_and(|v| v.repaint_delay.is_zero());
        (again, out.platform_output.cursor_icon)
    }

    /// What the shell asked of the host: the page's file picker, a save's download.
    fn asks(&mut self) {
        for a in std::mem::take(&mut self.shell.out) {
            match a {
                Action::Open => page_pick(),
                Action::SaveAs => {
                    if let Some(d) = &mut self.shell.dwgdoc {
                        d.save();
                    }
                }
                _ => {}
            }
        }
        if let Some((name, bytes)) = self.shell.dwgdoc.as_mut().and_then(|d| d.download.take()) {
            page_save(&name, &bytes);
        }
    }

    /// A window event as egui's input; true if it wants a frame.
    fn event(&mut self, e: &WindowEvent, ppp: f32) -> bool {
        let m = self.modifiers;
        match e {
            WindowEvent::CursorMoved { position, .. } => {
                self.pointer = egui::pos2(position.x as f32 / ppp, position.y as f32 / ppp);
                self.events.push(egui::Event::PointerMoved(self.pointer));
            }
            WindowEvent::CursorLeft { .. } => self.events.push(egui::Event::PointerGone),
            WindowEvent::MouseInput { state, button, .. } => {
                let button = match button {
                    MouseButton::Left => egui::PointerButton::Primary,
                    MouseButton::Right => egui::PointerButton::Secondary,
                    MouseButton::Middle => egui::PointerButton::Middle,
                    MouseButton::Back => egui::PointerButton::Extra1,
                    MouseButton::Forward => egui::PointerButton::Extra2,
                    MouseButton::Other(_) => return false,
                };
                self.events.push(egui::Event::PointerButton {
                    pos: self.pointer,
                    button,
                    pressed: *state == ElementState::Pressed,
                    modifiers: m,
                });
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (unit, delta) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (egui::MouseWheelUnit::Line, egui::vec2(*x, *y)),
                    MouseScrollDelta::PixelDelta(p) => (
                        egui::MouseWheelUnit::Point,
                        egui::vec2(p.x as f32 / ppp, p.y as f32 / ppp),
                    ),
                };
                self.events.push(egui::Event::MouseWheel {
                    unit,
                    delta,
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Wheel,
                    modifiers: m,
                });
            }
            WindowEvent::ModifiersChanged(s) => {
                let s = s.state();
                self.modifiers = egui::Modifiers {
                    alt: s.alt_key(),
                    ctrl: s.control_key(),
                    shift: s.shift_key(),
                    mac_cmd: false,
                    command: s.control_key() || s.super_key(),
                };
                self.events.push(egui::Event::ModifiersChanged(self.modifiers));
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == ElementState::Pressed;
                if let Some(key) = key_of(&event.logical_key) {
                    self.events.push(egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed,
                        repeat: event.repeat,
                        modifiers: m,
                    });
                }
                if pressed
                    && !m.ctrl
                    && !m.command
                    && let Some(t) = &event.text
                    && !t.chars().any(char::is_control)
                {
                    self.events.push(egui::Event::Text(t.to_string()));
                }
            }
            WindowEvent::Focused(f) => self.events.push(egui::Event::WindowFocused(*f)),
            _ => return false,
        }
        true
    }
}

fn key_of(k: &Key) -> Option<egui::Key> {
    use egui::Key as K;
    Some(match k {
        Key::Named(n) => match n {
            NamedKey::Enter => K::Enter,
            NamedKey::Space => K::Space,
            NamedKey::Backspace => K::Backspace,
            NamedKey::Delete => K::Delete,
            NamedKey::Escape => K::Escape,
            NamedKey::Tab => K::Tab,
            NamedKey::ArrowUp => K::ArrowUp,
            NamedKey::ArrowDown => K::ArrowDown,
            NamedKey::ArrowLeft => K::ArrowLeft,
            NamedKey::ArrowRight => K::ArrowRight,
            NamedKey::Home => K::Home,
            NamedKey::End => K::End,
            NamedKey::PageUp => K::PageUp,
            NamedKey::PageDown => K::PageDown,
            NamedKey::F1 => K::F1,
            NamedKey::F2 => K::F2,
            NamedKey::F3 => K::F3,
            NamedKey::F4 => K::F4,
            NamedKey::F5 => K::F5,
            NamedKey::F6 => K::F6,
            NamedKey::F7 => K::F7,
            NamedKey::F8 => K::F8,
            NamedKey::F9 => K::F9,
            NamedKey::F10 => K::F10,
            NamedKey::F11 => K::F11,
            NamedKey::F12 => K::F12,
            _ => return None,
        },
        Key::Character(s) => return egui::Key::from_name(&s.to_uppercase()),
        _ => return None,
    })
}

/// The canvas, its surface and the shell on it.
struct Host {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    ui: Ui,
    /// Frames drawn, to say when the first is.
    frames: u64,
}

enum Msg {
    Ready(Box<Host>),
    Wake,
}

struct App {
    proxy: EventLoopProxy<Msg>,
    host: Option<Box<Host>>,
    started: bool,
}

async fn open_host(window: Arc<Window>) -> Result<Host, String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let surface = instance
        .create_surface(window.clone())
        .map_err(|e| e.to_string())?;
    let (device, queue, adapter) = open_device(&instance, Some(&surface)).await?;
    let size = window.inner_size();
    let mut config = surface
        .get_default_config(&adapter, size.width.max(320), size.height.max(240))
        .ok_or("no surface format")?;
    // egui blends in sRGB, as a browser does
    config.format = config.format.remove_srgb_suffix();
    surface.configure(&device, &config);
    let format = config.format;
    Ok(Host {
        window,
        surface,
        config,
        ui: Ui::new(device, queue, format),
        frames: 0,
    })
}

impl Host {
    fn draw(&mut self) {
        use wgpu::CurrentSurfaceTexture as T;
        let frame = match self.surface.get_current_texture() {
            T::Success(f) | T::Suboptimal(f) => f,
            T::Outdated | T::Lost => {
                self.surface.configure(&self.ui.device, &self.config);
                self.window.request_redraw();
                return;
            }
            _ => return,
        };
        let view = frame.texture.create_view(&Default::default());
        let ppp = self.window.scale_factor() as f32;
        let (again, cursor) = self.ui.frame(&view, (self.config.width, self.config.height), ppp);
        self.window.pre_present_notify();
        self.ui.queue.present(frame);
        self.frames += 1;
        if self.frames == 2 {
            say("forge-render: drawn in the canvas");
        }
        self.cursor(cursor);
        if again {
            self.window.request_redraw();
        }
    }

    fn cursor(&self, c: egui::CursorIcon) {
        use winit::window::CursorIcon as W;
        if c == egui::CursorIcon::None {
            self.window.set_cursor_visible(false);
            return;
        }
        self.window.set_cursor_visible(true);
        self.window.set_cursor(match c {
            egui::CursorIcon::PointingHand => W::Pointer,
            egui::CursorIcon::Grab => W::Grab,
            egui::CursorIcon::Grabbing => W::Grabbing,
            egui::CursorIcon::Text => W::Text,
            egui::CursorIcon::ResizeHorizontal => W::EwResize,
            egui::CursorIcon::ResizeVertical => W::NsResize,
            _ => W::Default,
        });
    }

    fn take_inbox(&mut self) {
        for (name, bytes) in INBOX.with(|b| std::mem::take(&mut *b.borrow_mut())) {
            self.ui.open(&name, &bytes);
        }
    }
}

impl ApplicationHandler<Msg> for App {
    fn resumed(&mut self, on_loop: &ActiveEventLoop) {
        if std::mem::replace(&mut self.started, true) {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("ForgeCAD")
            .with_append(true);
        let window = Arc::new(on_loop.create_window(attrs).expect("a canvas"));
        let proxy = self.proxy.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match open_host(window).await {
                Ok(h) => {
                    let _ = proxy.send_event(Msg::Ready(Box::new(h)));
                }
                Err(e) => say(&format!("forge-render: {e}")),
            }
        });
    }

    fn user_event(&mut self, _: &ActiveEventLoop, m: Msg) {
        if let Msg::Ready(h) = m {
            self.host = Some(h);
        }
        if let Some(h) = &mut self.host {
            h.take_inbox();
            h.window.request_redraw();
        }
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(h) = &mut self.host else { return };
        match &event {
            WindowEvent::RedrawRequested => h.draw(),
            WindowEvent::Resized(s) if s.width > 0 && s.height > 0 => {
                h.config.width = s.width;
                h.config.height = s.height;
                h.surface.configure(&h.ui.device, &h.config);
                h.window.request_redraw();
            }
            e => {
                let ppp = h.window.scale_factor() as f32;
                if h.ui.event(e, ppp) {
                    h.window.request_redraw();
                }
            }
        }
    }
}

fn hand_over(name: String, bytes: Uint8Array) {
    INBOX.with(|b| b.borrow_mut().push((name, bytes.to_vec())));
    WAKE.with(|w| {
        if let Some(p) = &*w.borrow() {
            let _ = p.send_event(Msg::Wake);
        }
    });
}

/// Shows a DWG file's bytes in a canvas on the page, AutoCAD's layout round it.
#[wasm_bindgen]
pub fn start_dwg(name: String, bytes: Uint8Array) -> Result<(), JsValue> {
    let on_loop = EventLoop::with_user_event().build().map_err(|e| e.to_string())?;
    let proxy = on_loop.create_proxy();
    WAKE.with(|w| *w.borrow_mut() = Some(proxy.clone()));
    hand_over(name, bytes);
    on_loop.spawn_app(App {
        proxy,
        host: None,
        started: false,
    });
    Ok(())
}

/// Another file for the canvas `start_dwg` made: it takes the place of the one shown.
#[wasm_bindgen]
pub fn open_dwg(name: String, bytes: Uint8Array) {
    hand_over(name, bytes);
}

/// Waits for a callback the GPU makes from the browser's loop.
async fn called_back(start: impl FnOnce(Box<dyn FnOnce()>)) {
    let mut go = None;
    let promise = Promise::new(&mut |resolve, _| go = Some(resolve));
    let resolve = go.expect("a promise runs this at once");
    start(Box::new(move || {
        let _ = resolve.call0(&JsValue::NULL);
    }));
    let _ = JsFuture::from(promise).await;
}

/// The DWG document's window drawn once to a texture, `clicks` (x, y in points, pairs) clicked
/// first, `typed` typed into the command line: RGBA, for where the browser can't show a WebGPU
/// canvas.
#[wasm_bindgen]
pub async fn dwg_offscreen(
    name: String,
    bytes: Uint8Array,
    width: u32,
    height: u32,
    space: usize,
    clicks: Vec<f32>,
    typed: String,
) -> Result<Uint8Array, JsValue> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let (device, queue, _) = open_device(&instance, None).await?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("window"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let mut ui = Ui::new(device.clone(), queue.clone(), format);
    let t0 = now_ms();
    ui.open(&name, &bytes.to_vec());
    say(&format!(
        "forge-render: {name} read and made ready in {:.1} ms",
        now_ms() - t0
    ));
    if let Some(d) = &mut ui.shell.dwgdoc {
        d.switch_space(space);
    }
    let size = (width, height);
    for _ in 0..3 {
        ui.frame(&view, size, 1.0);
    }
    for p in clicks.as_chunks::<2>().0 {
        let at = egui::pos2(p[0], p[1]);
        ui.events.push(egui::Event::PointerMoved(at));
        ui.frame(&view, size, 1.0);
        for pressed in [true, false] {
            ui.events.push(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            });
            ui.frame(&view, size, 1.0);
        }
    }
    for line in typed.split(';').filter(|l| !l.is_empty()) {
        ui.events.push(egui::Event::Text(line.to_string()));
        ui.events.push(egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        });
        ui.frame(&view, size, 1.0);
    }
    // the pan of a drag, ten frames of it
    let (mut all, mut shell) = (0.0, 0.0);
    for k in 0..10 {
        if let Some(v) = ui.shell.dwgdoc.as_mut().and_then(|d| d.view.as_mut()) {
            v.center[0] += (k as f64 - 4.5) / v.scale;
        }
        let t1 = now_ms();
        ui.frame(&view, size, 1.0);
        all += now_ms() - t1;
        shell += ui.ui_ms;
    }
    say(&format!(
        "forge-render: a frame in {:.1} ms, the shell's part {:.1} ms (mean of 10)",
        all / 10.0,
        shell / 10.0
    ));
    let row = (width * 4).div_ceil(256) * 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("read back"),
        size: (row * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&Default::default());
    enc.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([enc.finish()]);
    called_back(|done| buffer.slice(..).map_async(wgpu::MapMode::Read, move |_| done())).await;
    let data = buffer.slice(..).get_mapped_range().expect("it was waited for");
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height as usize {
        let at = y * row as usize;
        rgba.extend_from_slice(&data[at..at + width as usize * 4]);
    }
    for p in rgba.as_chunks_mut::<4>().0 {
        p[3] = 255;
    }
    Ok(Uint8Array::from(&rgba[..]))
}
