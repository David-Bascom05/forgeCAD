//! The browser's app: the desktop's `Gui` (every tool, the sketcher, Measure, the issues,
//! assemblies, sheet metal, drawings, versions) on a canvas, the page's input fed to egui as
//! `webinput` translates it. Files live in the page's filesystem (`forge_dirs::fs`): the page
//! fills it from where the project is kept and is told of every change to keep it there
//! (web/storage.js). The kernel runs here, bit for bit as natively.
//!
//! The page's side (web/app.js) gives `window.forge_fs_changed(path, bytes | null)`,
//! `window.forge_pick()` (File > Open) and `window.forge_save_as(name)` (File > Save As, which
//! answers with `app_open`), and calls the exports below.

use crate::gui::Gui;
use crate::render::open_device;
use crate::shell::Action;
use crate::viewer::{now_ms, say};
use crate::webinput::Input;
use forge_agent::json::Json;
use js_sys::Uint8Array;
use std::cell::RefCell;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::PhysicalKey;
use winit::platform::web::{EventLoopExtWebSys, WindowAttributesExtWebSys};
use winit::window::{Window, WindowId};

#[wasm_bindgen]
extern "C" {
    /// The page's: a file of the project written (its bytes) or removed (null), to keep.
    #[wasm_bindgen(js_namespace = window, js_name = forge_fs_changed)]
    fn page_changed(path: &str, bytes: Option<Uint8Array>);
    /// The page's: File > Open (the project's files, or one from the computer).
    #[wasm_bindgen(js_namespace = window, js_name = forge_pick)]
    fn page_pick();
    /// The page's: File > Save As, a name asked for.
    #[wasm_bindgen(js_namespace = window, js_name = forge_save_as)]
    fn page_save_as(name: &str);
    #[wasm_bindgen(js_namespace = performance, js_name = now)]
    fn perf_now() -> f64;
    #[wasm_bindgen(js_namespace = Date, js_name = now)]
    fn date_now() -> f64;
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn console_error(s: &str);
}

/// What the page asked of the app between frames.
enum Ask {
    Open(String),
    SaveAs(String),
    Save,
    /// Files arrived behind the first frame: the open part built again (what it read may be new).
    Rebuild,
    Apply(forge_lang::error::Op),
    /// The next frame's pixels, for the page (`app_pixels`).
    Pixels(js_sys::Function),
    /// A file the app wrote that the page's storage refused or lost: (project path, why).
    StoreFailed(String, String),
    /// Words for the toolpath simulation (`app_camsim`).
    CamSim(String),
    /// A click on what the shell noted by this name (`app_click`): the tests' hand. The button
    /// (0 primary, 1 secondary) and whether a double click.
    Click(String, u8, bool),
    /// Text typed into what has the focus, then Enter if asked (`app_type`).
    Type(String, bool),
    /// A Manufacture workspace command, as words (`app_cam`).
    Cam(String),
    /// A Machine Builder command (`Gui::machine_command`).
    Machine(String),
}

thread_local! {
    static INBOX: RefCell<Vec<Ask>> = const { RefCell::new(Vec::new()) };
    static WAKE: RefCell<Option<EventLoopProxy<Msg>>> = const { RefCell::new(None) };
    /// The app's state as the page reads it (`app_state`), made after each frame.
    static STATE: RefCell<String> = RefCell::new("{\"ready\":false}".into());
    /// Frames drawn.
    static FRAMES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    /// When the app's first frame was drawn (`performance.now()`, ms from the page's start).
    static FIRST_FRAME: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
    /// The page's own changes are not echoed back to it.
    static LOADING: RefCell<bool> = const { RefCell::new(false) };
}

fn wake() {
    let here = WAKE.with(|w| {
        w.borrow().as_ref().map(|p| {
            let _ = p.send_event(Msg::Wake);
        })
    });
    // a worker's thread (the threaded build): the page is told through the worker's messages
    #[cfg(target_feature = "atomics")]
    if here.is_none() && !on_page() {
        wake_page();
    }
    #[cfg(not(target_feature = "atomics"))]
    let _ = here;
}

#[cfg(target_feature = "atomics")]
#[wasm_bindgen]
extern "C" {
    /// The worker's (web/worker.js): a message to the page, which wakes the app.
    #[wasm_bindgen(js_namespace = globalThis, js_name = forge_wake_page)]
    fn wake_page();
}

/// The threads the page's machine has (`navigator.hardwareConcurrency`, in a worker too), at
/// most 8, as natively.
#[cfg(target_feature = "atomics")]
fn hardware_threads() -> usize {
    let n = js_sys::Reflect::get(&js_sys::global(), &"navigator".into())
        .and_then(|nav| js_sys::Reflect::get(&nav, &"hardwareConcurrency".into()))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);
    (n as usize).clamp(1, 8)
}

/// The page's: a worker said its thread has news (the threaded build).
#[wasm_bindgen]
pub fn app_wake() {
    wake();
}

enum Msg {
    Ready(Box<Host>),
    Wake,
}

/// The canvas, its surface and the app on it. Offscreen (`?offscreen=WxH`: where a WebGPU canvas
/// can't show, a headless browser's software adapter) there is no surface: frames are drawn to a
/// texture, and the canvas only takes the input.
struct Host {
    /// Kept for the page's life: the browser ends a dropped instance's callbacks (a buffer's
    /// mapping among them).
    _instance: wgpu::Instance,
    window: Arc<Window>,
    surface: Option<wgpu::Surface<'static>>,
    config: wgpu::SurfaceConfiguration,
    /// Offscreen, what the frames are drawn to.
    target: Option<wgpu::Texture>,
    gui: Gui,
    input: Input,
    /// A test's click off the view (`app_click`): the view's pick it asks is dropped after
    /// the frame (nothing of the view was clicked; the offscreen adapter's pick may not finish).
    unpick: bool,
    /// Frames submitted whose GPU work isn't done yet. At most `IN_FLIGHT` are: a frame asked for
    /// past that waits for one to finish (`draw`), as the desktop's presenter waits for its last
    /// present. Unpaced, a slow adapter fell ever further behind: after a second's orbit the
    /// screen showed what the mouse did 13 s before (27 s on a large assembly).
    in_flight: std::sync::Arc<Pace>,
    /// When the last frame was submitted (`now_ms`): past a second, a frame is drawn whatever is
    /// out (a done callback that never came must not stop the page drawing).
    submitted: f64,
    frames: u64,
    /// A frame the app asked for later (a tooltip's delay), as `now_ms`.
    next: Option<f64>,
}

struct App {
    proxy: EventLoopProxy<Msg>,
    host: Option<Box<Host>>,
    started: bool,
    first: Option<String>,
    offscreen: Option<(u32, u32)>,
}

async fn open_host(
    window: Arc<Window>,
    first: Option<String>,
    offscreen: Option<(u32, u32)>,
) -> Result<Host, String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let surface = match offscreen {
        Some(_) => None,
        None => Some(
            instance
                .create_surface(window.clone())
                .map_err(|e| e.to_string())?,
        ),
    };
    let (device, queue, adapter) = open_device(&instance, surface.as_ref()).await?;
    let info = adapter.get_info();
    say(&format!(
        "forge-app: {} {:?} {}",
        info.name, info.device_type, info.backend
    ));
    // the canvas's pixels: its CSS size times the device's pixel ratio (winit's first size can
    // come before the page's layout)
    let size = backing_size(&window).unwrap_or_else(|| window.inner_size());
    let (config, target) = match (&surface, offscreen) {
        (Some(surface), _) => {
            let mut config = surface
                .get_default_config(&adapter, size.width.max(320), size.height.max(240))
                .ok_or("no surface format")?;
            // the chrome blends in sRGB, as on the desktop
            config.format = config.format.remove_srgb_suffix();
            set_backing(&window, (config.width, config.height));
            surface.configure(&device, &config);
            (config, None)
        }
        (None, size) => {
            // WxH in CSS pixels, drawn at the device's pixel ratio as a canvas would be
            let scale = window.scale_factor();
            let (width, height) = size.map_or((1280, 800), |(w, h)| {
                (
                    (w as f64 * scale).round() as u32,
                    (h as f64 * scale).round() as u32,
                )
            });
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                format: wgpu::TextureFormat::Rgba8Unorm,
                width,
                height,
                present_mode: wgpu::PresentMode::Fifo,
                desired_maximum_frame_latency: 2,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                color_space: wgpu::SurfaceColorSpace::Auto,
                view_formats: vec![],
            };
            let target = offscreen_texture(&device, &config);
            (config, Some(target))
        }
    };
    device.set_device_lost_callback(|why, msg| {
        say(&format!("forge-app: the GPU device was lost ({why:?}): {msg}"))
    });
    let scale = window.scale_factor() as f32;
    let mut gui = Gui::new(device, queue, info, config.format, Arc::new(wake), scale);
    gui.load_settings();
    if let Some(f) = first {
        gui.open(&f);
    }
    Ok(Host {
        _instance: instance,
        window,
        surface,
        config,
        target,
        gui,
        input: Input::default(),
        unpick: false,
        in_flight: Default::default(),
        submitted: 0.0,
        frames: 0,
        next: None,
    })
}

fn offscreen_texture(device: &wgpu::Device, c: &wgpu::SurfaceConfiguration) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("offscreen window"),
        size: wgpu::Extent3d {
            width: c.width,
            height: c.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: c.format,
        usage: c.usage,
        view_formats: &[],
    })
}

/// The canvas's size in device pixels: its CSS size times the pixel ratio.
fn backing_size(window: &Window) -> Option<winit::dpi::PhysicalSize<u32>> {
    use winit::platform::web::WindowExtWebSys;
    let c = window.canvas()?;
    let dpr = web_sys::window()?.device_pixel_ratio();
    let (w, h) = (c.client_width() as f64 * dpr, c.client_height() as f64 * dpr);
    (w >= 1.0 && h >= 1.0).then(|| winit::dpi::PhysicalSize::new(w.round() as u32, h.round() as u32))
}

/// The canvas's own pixel count (its `width` and `height`): what a WebGPU canvas draws at. winit
/// follows the canvas's size but leaves these as they are.
fn set_backing(window: &Window, (w, h): (u32, u32)) {
    use winit::platform::web::WindowExtWebSys;
    if let Some(c) = window.canvas() {
        if c.width() != w {
            c.set_width(w);
        }
        if c.height() != h {
            c.set_height(h);
        }
    }
}

impl Host {
    /// The canvas's pixels and the surface to `size` (device pixels; none: measured now).
    /// The canvas as drawn: the pixel ratio, its CSS size, its own pixels, what the frames are
    /// drawn at.
    fn canvas_json(&self) -> Json {
        use winit::platform::web::WindowExtWebSys;
        let pair = |a: f64, b: f64| Json::Arr(vec![num(a), num(b)]);
        let dpr = web_sys::window().map_or(1.0, |w| w.device_pixel_ratio());
        let (css, own) = self.window.canvas().map_or((Json::Null, Json::Null), |c| {
            (
                pair(c.client_width() as f64, c.client_height() as f64),
                pair(c.width() as f64, c.height() as f64),
            )
        });
        Json::obj(vec![
            ("dpr", num(dpr)),
            ("css", css),
            ("backing", own),
            ("drawn", pair(self.config.width as f64, self.config.height as f64)),
            ("offscreen", self.surface.is_none().into()),
        ])
    }

    fn fit(&mut self, size: Option<(u32, u32)>) {
        if let Some(surface) = &self.surface {
            let s = size.or_else(|| backing_size(&self.window).map(|s| (s.width, s.height)));
            if let Some((w, h)) = s.filter(|s| s.0 > 0 && s.1 > 0) {
                self.config.width = w;
                self.config.height = h;
                set_backing(&self.window, (w, h));
                surface.configure(&self.gui.device(), &self.config);
            }
        }
        self.window.request_redraw();
    }

    fn draw(&mut self) {
        use std::sync::atomic::Ordering::Relaxed;
        use wgpu::CurrentSurfaceTexture as T;
        // paced: drawn again once a frame's GPU work is done (its callback wakes the loop and asks
        // for the frame); input meanwhile waits in `input`, taken by the frame that's drawn
        if self.in_flight.out.load(Relaxed) >= IN_FLIGHT && now_ms() - self.submitted < 1000.0 {
            self.in_flight.skipped.store(true, Relaxed);
            return;
        }
        // the canvas's pixels checked each frame: not every change of the pixel ratio comes as an
        // event (a page zoomed, a window dragged to another monitor)
        if self.surface.is_some()
            && let Some(s) = backing_size(&self.window)
            && (s.width, s.height) != (self.config.width, self.config.height)
        {
            self.fit(Some((s.width, s.height)));
        }
        let ppp = self.window.scale_factor() as f32;
        let size = (self.config.width, self.config.height);
        let out = match (&self.surface, &self.target) {
            (Some(surface), _) => {
                let frame = match surface.get_current_texture() {
                    T::Success(f) | T::Suboptimal(f) => f,
                    T::Outdated | T::Lost => {
                        surface.configure(&self.gui.device(), &self.config);
                        self.window.request_redraw();
                        return;
                    }
                    _ => return,
                };
                let view = frame.texture.create_view(&Default::default());
                let raw = self.input.raw(size, ppp, now_ms() / 1000.0);
                let out = self.gui.frame(raw, &view, size);
                self.window.pre_present_notify();
                self.gui.queue().present(frame);
                out
            }
            (None, Some(t)) => {
                let view = t.create_view(&Default::default());
                let raw = self.input.raw(size, ppp, now_ms() / 1000.0);
                self.gui.frame(raw, &view, size)
            }
            (None, None) => return,
        };
        if std::mem::take(&mut self.unpick)
            && let Some(v) = &mut self.gui.view
        {
            v.nav.take_pick();
        }
        // input held back for its own frame (a click apart from typing): drawn next
        if self.input.waiting() {
            self.window.request_redraw();
        }
        self.in_flight.out.fetch_add(1, Relaxed);
        self.submitted = now_ms();
        let pace = self.in_flight.clone();
        self.gui.queue().on_submitted_work_done(move || {
            pace.out.fetch_sub(1, Relaxed);
            // a frame was asked for while this one was out: drawn now
            if pace.skipped.swap(false, Relaxed) {
                wake();
            }
        });
        self.frames += 1;
        FRAMES.with(|f| f.set(self.frames));
        if self.frames == 1 {
            FIRST_FRAME.with(|f| f.set(perf_now()));
        }
        if self.frames == 2 {
            say("forge-app: drawn in the canvas");
        }

        self.cursor(out.platform.cursor_icon);
        for c in &out.platform.commands {
            if let egui::OutputCommand::CopyText(t) = c
                && let Some(w) = web_sys::window()
            {
                let _ = w.navigator().clipboard().write_text(t);
            }
        }
        self.next = None;
        self.input.settling = out.again_in.is_some_and(|d| d.is_zero());
        match out.again_in {
            Some(d) if d.is_zero() => self.window.request_redraw(),
            Some(d) => {
                let at = now_ms() + d.as_secs_f64() * 1e3;
                self.next = Some(at);
                later(d.as_secs_f64() * 1e3);
            }
            None => {}
        }
        for a in out.host {
            match a {
                Action::Open | Action::Sheet(crate::shell::sheet::SheetCmd::NewPart) => page_pick(),
                Action::SaveAs => page_save_as(&self.gui.shell.name),
                _ => {}
            }
        }
        STATE.with(|s| *s.borrow_mut() = state_json(&self.gui, self.canvas_json()));
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
            egui::CursorIcon::Crosshair => W::Crosshair,
            egui::CursorIcon::Move | egui::CursorIcon::AllScroll => W::Move,
            egui::CursorIcon::ResizeHorizontal | egui::CursorIcon::ResizeColumn => W::EwResize,
            egui::CursorIcon::ResizeVertical | egui::CursorIcon::ResizeRow => W::NsResize,
            egui::CursorIcon::NotAllowed => W::NotAllowed,
            egui::CursorIcon::Wait | egui::CursorIcon::Progress => W::Wait,
            _ => W::Default,
        });
    }

    fn take_inbox(&mut self) {
        pass_on();
        for a in INBOX.with(|b| std::mem::take(&mut *b.borrow_mut())) {
            match a {
                Ask::Open(p) => self.gui.open(&p),
                Ask::SaveAs(p) => self.gui.save_as(&p),
                Ask::Save => self.gui.fg_save(),
                Ask::Machine(c) => {
                    let _ = self.gui.machine_command(&c);
                }
                Ask::Rebuild => {
                    if let Some(s) = &mut self.gui.fg {
                        s.rebuild();
                    }
                    // a machining document reads its parts: run again with what arrived
                    self.gui.camsim_reload();
                }
                Ask::Apply(op) => {
                    if let Err(e) = self.gui.fg_apply(op) {
                        self.gui.shell.status = e;
                    }
                }
                Ask::Pixels(f) => self.pixels(f),
                Ask::StoreFailed(path, why) => self.gui.store_failed(&path, &why),
                Ask::CamSim(c) => {
                    if let Err(e) = self.gui.camsim_command(&c) {
                        self.gui.shell.status = e;
                    }
                }
                Ask::Click(name, button, double) => match self.gui.shell.place(&name) {
                    Some(r) => {
                        let at = r.center();
                        let b = if button == 1 {
                            egui::PointerButton::Secondary
                        } else {
                            egui::PointerButton::Primary
                        };
                        let ev = &mut self.input.events;
                        // no hover move first: a press carries its place, and a hover over the
                        // view starts a pick the offscreen software adapter may never finish
                        for _ in 0..if double { 2 } else { 1 } {
                            for pressed in [true, false] {
                                ev.push(egui::Event::PointerButton {
                                    pos: at,
                                    button: b,
                                    pressed,
                                    modifiers: egui::Modifiers::NONE,
                                });
                            }
                        }
                        ev.push(egui::Event::PointerGone);
                        self.unpick = !self.gui.shell.view_rect.contains(at);
                    }
                    None => console_error(&format!("app_click: nothing noted as `{name}`")),
                },
                Ask::Cam(words) => {
                    if let Err(e) = cam_words(&mut self.gui, &words) {
                        console_error(&format!("app_cam: {e}"));
                    }
                }
                Ask::Type(text, enter) => {
                    self.input.events.push(egui::Event::Text(text));
                    if enter {
                        for pressed in [true, false] {
                            self.input.events.push(egui::Event::Key {
                                key: egui::Key::Enter,
                                physical_key: None,
                                pressed,
                                repeat: false,
                                modifiers: egui::Modifiers::NONE,
                            });
                        }
                    }
                }
            }
        }
    }
}

impl Host {
    /// A frame drawn to a texture and read back, handed to `f` as `{width, height, rgba}`: what
    /// the canvas shows, for where a WebGPU canvas can't be captured (a headless browser).
    fn pixels(&mut self, f: js_sys::Function) {
        let (width, height) = (self.config.width, self.config.height);
        let device = self.gui.device();
        let format = self.config.format;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("pixels"),
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
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let view = texture.create_view(&Default::default());
        let ppp = self.window.scale_factor() as f32;
        let raw = self.input.raw((width, height), ppp, now_ms() / 1000.0);
        let _ = self.gui.frame(raw, &view, (width, height));
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
        self.gui.queue().submit([enc.finish()]);
        let failed = scope.pop();
        wasm_bindgen_futures::spawn_local(async move {
            if let Some(e) = failed.await {
                say(&format!("forge-app: the frame's read back: {e}"));
            }
        });
        let bgra = matches!(
            format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );
        // the texture and the buffer kept till the copy is read (a dropped one is destroyed)
        wasm_bindgen_futures::spawn_local(async move {
            let _texture = texture;
            let mut go = None;
            let promise = js_sys::Promise::new(&mut |resolve, _| go = Some(resolve));
            let resolve = go.expect("a promise runs this at once");
            buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                let _ = resolve.call1(&JsValue::NULL, &JsValue::from_bool(r.is_ok()));
            });
            let mapped = wasm_bindgen_futures::JsFuture::from(promise).await;
            let data = match (mapped, buffer.slice(..).get_mapped_range()) {
                (Ok(ok), Ok(data)) if ok.as_bool() == Some(true) => data,
                _ => {
                    say("forge-app: the frame's pixels didn't map");
                    let _ = f.call1(&JsValue::NULL, &JsValue::NULL);
                    return;
                }
            };
            let mut rgba = Vec::with_capacity((width * height * 4) as usize);
            for y in 0..height as usize {
                let at = y * row as usize;
                rgba.extend_from_slice(&data[at..at + width as usize * 4]);
            }
            for p in rgba.as_chunks_mut::<4>().0 {
                if bgra {
                    p.swap(0, 2);
                }
                p[3] = 255;
            }
            let o = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&o, &"width".into(), &width.into());
            let _ = js_sys::Reflect::set(&o, &"height".into(), &height.into());
            let _ = js_sys::Reflect::set(&o, &"rgba".into(), &Uint8Array::from(&rgba[..]));
            let _ = f.call1(&JsValue::NULL, &o);
        });
        self.window.request_redraw();
    }
}

/// A frame `ms` from now (the app asked for one later).
fn later(ms: f64) {
    let Some(w) = web_sys::window() else { return };
    let f = Closure::once_into_js(wake);
    let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(f.unchecked_ref(), ms.ceil() as i32);
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
        let (first, offscreen) = (self.first.take(), self.offscreen);
        wasm_bindgen_futures::spawn_local(async move {
            match open_host(window, first, offscreen).await {
                Ok(h) => {
                    let _ = proxy.send_event(Msg::Ready(Box::new(h)));
                }
                Err(e) => say(&format!("forge-app: {e}")),
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
            WindowEvent::RedrawRequested => {
                h.take_inbox();
                h.draw();
                return;
            }
            // the canvas's CSS size or the device's pixel ratio changed (a resize, another
            // monitor, the page zoomed): its pixels made to match
            WindowEvent::Resized(s) if s.width > 0 && s.height > 0 => {
                h.fit(Some((s.width, s.height)));
                return;
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                h.fit(None);
                return;
            }
            _ => {}
        }
        let ppp = h.window.scale_factor() as f32;
        let mut redraw = h.input.event(&event, ppp);
        h.gui.view_event(&event);
        if let WindowEvent::KeyboardInput { event: k, .. } = &event
            && let PhysicalKey::Code(c) = k.physical_key
        {
            h.gui.held_key(c, k.state == ElementState::Pressed);
        }
        redraw |= h.gui.view_dirty() || h.gui.picks();
        if redraw {
            h.window.request_redraw();
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        let Some(h) = &mut self.host else { return };
        // a pick in flight is not drawn for: its read back wakes the loop when it is in (the
        // renderer's pick waker), and is taken here. Drawn for, as it was, a headless page's
        // loop spun at 60 fps, the software adapter's read back starved behind the frames,
        // and an answer that came after the loop went idle was never taken: it drew for ever
        if h.gui.picks() || h.next.is_some_and(|n| now_ms() >= n) {
            h.next = None;
            h.window.request_redraw();
        }
    }
}

fn num(x: f64) -> Json {
    Json::Num(x)
}

/// What the page (and the tests) read of the app: the open document, its last build's word,
/// exact (the volume's bits too, to compare with a native build's).
fn state_json(g: &Gui, canvas: Json) -> String {
    let doc = g.fg.as_ref();
    let built = g.last_built.as_ref().map(|b| {
        let mut f = vec![
            ("seq", num(b.seq as f64)),
            ("checked", b.checked.into()),
            ("ok", b.ok.into()),
            (
                "issues",
                Json::Arr(b.issues.iter().map(|i| i.clone().into()).collect()),
            ),
        ];
        if let Some(m) = &b.measure {
            f.extend(crate::parity::measure_fields(m));
        }
        Json::obj(f)
    });
    Json::obj(vec![
        ("ready", true.into()),
        ("first_frame_ms", num(FIRST_FRAME.with(|f| f.get()))),
        // frames drawn so far: a test waits on this, not on pictures, for a change to be drawn
        ("frames", num(FRAMES.with(|f| f.get()) as f64)),
        ("canvas", canvas),
        (
            "path",
            doc.and_then(|s| s.path.clone()).map_or(Json::Null, Json::from),
        ),
        ("seq", doc.map_or(Json::Null, |s| num(s.seq as f64))),
        ("done", doc.map_or(Json::Null, |s| num(s.done as f64))),
        ("dirty", doc.is_some_and(|s| s.dirty()).into()),
        // the Versions panel's word: history kept or not, and why not
        (
            "versions",
            Json::obj(vec![
                ("off", g.shell.versions.off.into()),
                ("unsigned", g.shell.versions.unsigned.into()),
                ("note", g.shell.versions.note.clone().into()),
                ("lost", g.shell.versions.lost.clone().into()),
            ]),
        ),
        // what's selected in the view: a test of a click or a selection window reads it
        (
            "selected",
            num(g.view.as_ref().map_or(0, |v| v.nav.selected.len()) as f64),
        ),
        ("status", g.shell.status.clone().into()),
        // the last build's time in the worker, ms
        ("build_ms", doc.map_or(Json::Null, |s| num(s.build_ms))),
        ("built", built.unwrap_or(Json::Null)),
        ("camsim", g.camsim_state().unwrap_or(Json::Null)),
        ("cam", cam_json(g)),
        // a pick of the view in flight (the host draws on until it is answered)
        ("pick_busy", g.pick_busy().into()),
        // where the shell's named things are, for the tests' clicks (`app_click`)
        (
            "places",
            Json::Arr(
                g.shell
                    .places
                    .iter()
                    .filter(|(n, _)| n.starts_with("cam:") || n.starts_with("tab:") || n.starts_with("tool:"))
                    .map(|(n, r)| {
                        Json::obj(vec![
                            ("name", n.clone().into()),
                            (
                                "rect",
                                Json::Arr(vec![
                                    num(r.left().into()),
                                    num(r.top().into()),
                                    num(r.width().into()),
                                    num(r.height().into()),
                                ]),
                            ),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
    .to_string()
}

/// A workspace command as words, for the tests to drive it without a pointer: `select ID`,
/// `edit ID`, `new-op FUNC`, `new-setup`, `tab K` (the open dialog's), `set ARG TEXT` (the op
/// dialog's), `post`, `tools`, `checks`, `generate`, `close` (every window).
fn cam_words(g: &mut Gui, words: &str) -> Result<(), String> {
    use crate::shell::cam::Cmd;
    let w = g.shell.cam.as_mut().ok_or("no machining document open")?;
    let (verb, rest) = words.split_once(' ').unwrap_or((words, ""));
    match verb {
        "select" => w.command(Cmd::Select(rest.into())),
        "edit" => w.command(Cmd::Edit(rest.into())),
        "new-op" => {
            let f = crate::shell::cam::schema::spec(rest).ok_or("no such op")?.func;
            w.command(Cmd::NewOp(f));
        }
        "new-setup" => w.command(Cmd::NewSetup),
        "tab" => {
            let k: usize = rest.parse().map_err(|_| "tab K")?;
            if let Some(d) = &mut w.op_dlg {
                d.tab = k;
            } else if let Some(d) = &mut w.setup_dlg {
                d.tab = k;
            }
        }
        "set" => {
            let (k, v) = rest.split_once(' ').unwrap_or((rest, ""));
            w.op_dlg.as_mut().ok_or("no op dialog")?.set(k, v.into());
        }
        "post" => w.command(Cmd::PostProcess),
        "tools" => w.command(Cmd::ToolLibrary),
        "checks" => w.command(Cmd::Checks),
        "generate" => w.command(Cmd::Generate),
        "close" => {
            w.op_dlg = None;
            w.setup_dlg = None;
            w.post_dlg = None;
            w.tools_open = false;
            w.checks_open = false;
            w.text_open = false;
        }
        _ => return Err(format!("unknown `{verb}`")),
    }
    Ok(())
}

/// The Manufacture workspace as the page's tests read it: the document's ops and their states.
fn cam_json(g: &Gui) -> Json {
    let Some(w) = &g.shell.cam else { return Json::Null };
    Json::obj(vec![
        ("path", w.path.clone().into()),
        ("generating", (w.running.is_some() || w.want_run).into()),
        ("generated", w.run.is_some().into()),
        (
            "certification",
            w.run
                .as_ref()
                .map_or(Json::Null, |r| r.certification.clone().into()),
        ),
        ("selected", w.selected.clone().map_or(Json::Null, Json::from)),
        ("dirty", w.dirty().into()),
        ("text", w.text.clone().into()),
        (
            "ops",
            Json::Arr(
                w.stmts
                    .iter()
                    .filter(|s| s.is_op())
                    .map(|s| {
                        Json::obj(vec![
                            ("name", s.name.clone().into()),
                            ("kind", s.func.clone().into()),
                            ("status", w.status(&s.name).to_string().into()),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "dialogs",
            Json::Arr(
                [
                    ("op", w.op_dlg.is_some()),
                    ("setup", w.setup_dlg.is_some()),
                    ("post", w.post_dlg.is_some()),
                    ("tools", w.tools_open),
                    ("checks", w.checks_open),
                    ("text", w.text_open),
                ]
                .into_iter()
                .filter(|x| x.1)
                .map(|x| x.0.to_string().into())
                .collect(),
            ),
        ),
    ])
}

/// Says a panic on the console (the page's tests watch for it) before the trap.
fn panic_hook() {
    std::panic::set_hook(Box::new(|p| console_error(&format!("forge-app panicked: {p}"))));
}

/// Changes made on a worker's thread (the threaded build), for the page's thread to pass on.
type Pending = Vec<(String, Option<Vec<u8>>)>;
static PENDING: std::sync::Mutex<Pending> = std::sync::Mutex::new(Vec::new());

thread_local! {
    /// Set on the page's own thread by `app_init` (a worker's thread never calls it).
    static PAGE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether this is the page's own thread.
fn on_page() -> bool {
    PAGE.with(|p| p.get())
}

/// The changes made on other threads, to the page, in order.
fn pass_on() {
    let all = std::mem::take(&mut *crate::plat::lock(&PENDING));
    for (p, b) in all {
        page_changed(&p, b.map(|b| Uint8Array::from(&b[..])));
    }
}

/// The filesystem's changes to the page, but not those it made itself.
fn tell_page(path: &std::path::Path, c: forge_dirs::fs::mem::Change<'_>) {
    if LOADING.with(|l| *l.borrow()) {
        return;
    }
    let p = path.to_string_lossy().into_owned();
    let bytes = match c {
        forge_dirs::fs::mem::Change::Wrote(b) => Some(b),
        forge_dirs::fs::mem::Change::Removed => None,
    };
    if on_page() {
        pass_on();
        page_changed(&p, bytes.map(Uint8Array::from));
    } else {
        crate::plat::lock(&PENDING).push((p, bytes.map(<[u8]>::to_vec)));
        wake();
    }
}

/// Set up once before anything else: the clock, the panic hook, the filesystem's hook.
#[wasm_bindgen]
pub fn app_init() {
    PAGE.with(|p| p.set(true));
    // the threaded build: the kernel's parallel paths (n-ary booleans, `forge_math::par`) on Web
    // Workers too; on the page's thread, which may not wait, each runs where it is spawned
    #[cfg(target_feature = "atomics")]
    forge_par::set_spawner(forge_par::Spawner {
        spawn: crate::plat::threads::spawn,
        can_block: || !on_page(),
        threads: hardware_threads,
    });
    panic_hook();
    forge_math::clock::set(perf_now, date_now);
    forge_dirs::fs::mem::set_hook(Some(tell_page));
}

/// Starts the app on a canvas appended to the page, with `path` open if given.
#[wasm_bindgen]
/// `offscreen`: "WxH", no WebGPU canvas (see `Host`).
pub fn start_app(path: Option<String>, offscreen: Option<String>) -> Result<(), JsValue> {
    let offscreen = offscreen.and_then(|s| {
        let (w, h) = s.split_once('x')?;
        Some((w.parse().ok()?, h.parse().ok()?))
    });
    app_init();
    let on_loop = EventLoop::with_user_event().build().map_err(|e| e.to_string())?;
    let proxy = on_loop.create_proxy();
    WAKE.with(|w| *w.borrow_mut() = Some(proxy.clone()));
    on_loop.spawn_app(App {
        proxy,
        host: None,
        started: false,
        first: path,
        offscreen,
    });
    Ok(())
}

/// Frames the GPU may be behind by: two, so the CPU makes the next while the GPU draws one.
const IN_FLIGHT: u32 = 2;

/// The page's frames out on the GPU (`Host::in_flight`), and whether one was asked for meanwhile.
#[derive(Default)]
struct Pace {
    out: std::sync::atomic::AtomicU32,
    skipped: std::sync::atomic::AtomicBool,
}

fn ask(a: Ask) {
    INBOX.with(|b| b.borrow_mut().push(a));
    wake();
}

/// Opens a file of the page's filesystem in its own tab.
#[wasm_bindgen]
pub fn app_open(path: String) {
    ask(Ask::Open(path));
}

/// Saves the document under this path (File > Save As's answer).
#[wasm_bindgen]
pub fn app_save_as(path: String) {
    ask(Ask::SaveAs(path));
}

/// The page's files have all arrived: the open part built again, through its warm stages.
#[wasm_bindgen]
pub fn app_rebuild() {
    ask(Ask::Rebuild);
}

#[wasm_bindgen]
pub fn app_save() {
    ask(Ask::Save);
}

/// An edit to the open part, as the tools' JSON gives it (`{"op":"SetParam",..}`), as the panel
/// would make it: one undo step, rebuilt live.
#[wasm_bindgen]
pub fn app_apply(op: &str) -> Result<(), JsValue> {
    let j = Json::parse(op).map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
    let op = forge_lang::error::Op::from_json(&j).ok_or("not an op")?;
    ask(Ask::Apply(op));
    Ok(())
}

/// Words for the open machining document's simulation (`Gui::camsim_command`): `t=12.5`,
/// `play`, `colour=compare`, ...
#[wasm_bindgen]
pub fn app_camsim(cmd: &str) {
    ask(Ask::CamSim(cmd.to_string()));
}

/// A click on what the shell noted as `name` (the tests' hand): `button` 0 primary, 1
/// secondary; `double` a double click.
#[wasm_bindgen]
pub fn app_click(name: String, button: u8, double: bool) {
    ask(Ask::Click(name, button, double));
}

/// A Manufacture workspace command as words (`cam_words`), without a pointer.
#[wasm_bindgen]
pub fn app_cam(words: String) {
    ask(Ask::Cam(words));
}

/// Text typed into what has the focus; Enter after it if `enter`.
#[wasm_bindgen]
pub fn app_type(text: String, enter: bool) {
    ask(Ask::Type(text, enter));
}

/// The page's storage could not keep a file the app wrote (`forgecad web` refused it, the
/// browser's store failed): said in the status, and in Versions for the history's files.
#[wasm_bindgen]
pub fn app_store_failed(path: String, why: String) {
    ask(Ask::StoreFailed(path, why));
}

/// The app's next frame as `{width, height, rgba}`, through a callback (see `Host::pixels`).
#[wasm_bindgen]
pub fn app_pixels(then: js_sys::Function) {
    ask(Ask::Pixels(then));
}

/// A Machine Builder command (`Gui::machine_command`: open, template KIND NAME, jog J V, check,
/// save project, ...); its answer is the state's `status`, prefixed `machine:`.
#[wasm_bindgen]
pub fn app_machine(cmd: String) {
    ask(Ask::Machine(cmd));
}

/// The app's state as JSON: see `state_json`.
#[wasm_bindgen]
pub fn app_state() -> String {
    STATE.with(|s| s.borrow().clone())
}

/// Puts the page's files in the filesystem (without telling it back).
#[wasm_bindgen]
pub fn fs_load(path: &str, bytes: &[u8]) {
    LOADING.with(|l| *l.borrow_mut() = true);
    forge_dirs::fs::mem::load(path, bytes.to_vec());
    LOADING.with(|l| *l.borrow_mut() = false);
}

#[wasm_bindgen]
pub fn fs_unload(prefix: &str) {
    forge_dirs::fs::mem::unload(prefix);
}

#[wasm_bindgen]
pub fn fs_read(path: &str) -> Option<Uint8Array> {
    forge_dirs::fs::read(path).ok().map(|b| Uint8Array::from(&b[..]))
}

/// Writes a file as the app would (the page is told, to keep it).
#[wasm_bindgen]
pub fn fs_write(path: &str, bytes: &[u8]) -> Result<(), JsValue> {
    forge_dirs::fs::write(path, bytes).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Every file under `prefix`.
#[wasm_bindgen]
pub fn fs_list(prefix: &str) -> js_sys::Array {
    forge_dirs::fs::mem::files_under(prefix)
        .into_iter()
        .map(|(p, _)| JsValue::from_str(&p.to_string_lossy()))
        .collect()
}

/// A file of the page's filesystem built as `forgecad build F --json` builds it, its measures
/// exact: `parity::build_report`, the same natively.
#[wasm_bindgen]
pub fn build_json(path: &str) -> String {
    app_init();
    crate::parity::build_report(path)
}

/// Builds handed to a worker (`build_start`), by number, and their reports once done.
static BUILDS: std::sync::Mutex<Vec<(u32, Option<String>)>> = std::sync::Mutex::new(Vec::new());
static NEXT_BUILD: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

/// `build_json` on a worker's thread (the threaded build: the kernel's parallel paths on more
/// workers; the single-threaded build: at once): its number, for `build_result`.
#[wasm_bindgen]
pub fn build_start(path: String) -> u32 {
    app_init();
    let id = NEXT_BUILD.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    crate::plat::lock(&BUILDS).push((id, None));
    crate::plat::spawn("forge-build", move || {
        let r = crate::parity::build_report(&path);
        if let Some(b) = crate::plat::lock(&BUILDS).iter_mut().find(|b| b.0 == id) {
            b.1 = Some(r);
        }
    });
    id
}

/// Build `id`'s report once it is done (then forgotten), else undefined.
#[wasm_bindgen]
pub fn build_result(id: u32) -> Option<String> {
    let mut all = crate::plat::lock(&BUILDS);
    let k = all.iter().position(|b| b.0 == id && b.1.is_some())?;
    all.swap_remove(k).1
}

/// What a test that timed out reports: the builds handed out and not yet taken (by number,
/// whether done), the memory's size and, in the threaded build, the worker pool
/// (`plat::threads::state`).
#[wasm_bindgen]
pub fn pool_state() -> String {
    let builds: Vec<String> = crate::plat::lock(&BUILDS)
        .iter()
        .map(|b| format!("[{},{}]", b.0, b.1.is_some()))
        .collect();
    #[cfg(target_feature = "atomics")]
    let pool = crate::plat::threads::state();
    #[cfg(not(target_feature = "atomics"))]
    let pool = "null".to_string();
    let mib = core::arch::wasm32::memory_size(0) / 16;
    format!(
        r#"{{"builds":[{}],"pool":{pool},"memory_mib":{mib}}}"#,
        builds.join(",")
    )
}

/// The bench parts (and their font) written into `dir`; their paths.
#[wasm_bindgen]
pub fn bench_load(dir: &str) -> js_sys::Array {
    crate::parity::bench_files()
        .into_iter()
        .filter_map(|(name, bytes)| {
            let p = format!("{dir}/{name}");
            fs_load(&p, &bytes);
            name.ends_with(".fg").then(|| JsValue::from_str(&p))
        })
        .collect()
}
