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
    Apply(forge_lang::error::Op),
    /// The next frame's pixels, for the page (`app_pixels`).
    Pixels(js_sys::Function),
}

thread_local! {
    static INBOX: RefCell<Vec<Ask>> = const { RefCell::new(Vec::new()) };
    static WAKE: RefCell<Option<EventLoopProxy<Msg>>> = const { RefCell::new(None) };
    /// The app's state as the page reads it (`app_state`), made after each frame.
    static STATE: RefCell<String> = RefCell::new("{\"ready\":false}".into());
    /// The page's own changes are not echoed back to it.
    static LOADING: RefCell<bool> = const { RefCell::new(false) };
}

fn wake() {
    WAKE.with(|w| {
        if let Some(p) = &*w.borrow() {
            let _ = p.send_event(Msg::Wake);
        }
    });
}

enum Msg {
    Ready(Box<Host>),
    Wake,
}

/// The canvas, its surface and the app on it.
struct Host {
    /// Kept for the page's life: the browser ends a dropped instance's callbacks (a buffer's
    /// mapping among them).
    _instance: wgpu::Instance,
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    gui: Gui,
    input: Input,
    frames: u64,
    /// A frame the app asked for later (a tooltip's delay), as `now_ms`.
    next: Option<f64>,
}

struct App {
    proxy: EventLoopProxy<Msg>,
    host: Option<Box<Host>>,
    started: bool,
    first: Option<String>,
}

async fn open_host(window: Arc<Window>, first: Option<String>) -> Result<Host, String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let surface = instance
        .create_surface(window.clone())
        .map_err(|e| e.to_string())?;
    let (device, queue, adapter) = open_device(&instance, Some(&surface)).await?;
    let info = adapter.get_info();
    say(&format!(
        "forge-app: {} {:?} {}",
        info.name, info.device_type, info.backend
    ));
    let size = window.inner_size();
    let mut config = surface
        .get_default_config(&adapter, size.width.max(320), size.height.max(240))
        .ok_or("no surface format")?;
    // the chrome blends in sRGB, as on the desktop
    config.format = config.format.remove_srgb_suffix();
    surface.configure(&device, &config);
    device.set_device_lost_callback(|why, msg| say(&format!("forge-app: the GPU device was lost ({why:?}): {msg}")));
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
        gui,
        input: Input::default(),
        frames: 0,
        next: None,
    })
}

impl Host {
    fn draw(&mut self) {
        use wgpu::CurrentSurfaceTexture as T;
        let frame = match self.surface.get_current_texture() {
            T::Success(f) | T::Suboptimal(f) => f,
            T::Outdated | T::Lost => {
                self.surface.configure(&self.gui.device(), &self.config);
                self.window.request_redraw();
                return;
            }
            _ => return,
        };
        let view = frame.texture.create_view(&Default::default());
        let ppp = self.window.scale_factor() as f32;
        let size = (self.config.width, self.config.height);
        let raw = self.input.raw(size, ppp, now_ms() / 1000.0);
        let out = self.gui.frame(raw, &view, size);
        self.window.pre_present_notify();
        self.gui.queue().present(frame);
        self.frames += 1;
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
        STATE.with(|s| *s.borrow_mut() = state_json(&self.gui));
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
        for a in INBOX.with(|b| std::mem::take(&mut *b.borrow_mut())) {
            match a {
                Ask::Open(p) => self.gui.open(&p),
                Ask::SaveAs(p) => self.gui.save_as(&p),
                Ask::Save => self.gui.fg_save(),
                Ask::Apply(op) => {
                    if let Err(e) = self.gui.fg_apply(op) {
                        self.gui.shell.status = e;
                    }
                }
                Ask::Pixels(f) => self.pixels(f),
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
        let first = self.first.take();
        wasm_bindgen_futures::spawn_local(async move {
            match open_host(window, first).await {
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
            WindowEvent::Resized(s) if s.width > 0 && s.height > 0 => {
                h.config.width = s.width;
                h.config.height = s.height;
                h.surface.configure(&h.gui.device(), &h.config);
                h.window.request_redraw();
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
        if h.gui.pick_busy() || h.gui.picks() || h.next.is_some_and(|n| now_ms() >= n) {
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
fn state_json(g: &Gui) -> String {
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
        (
            "path",
            doc.and_then(|s| s.path.clone()).map_or(Json::Null, Json::from),
        ),
        ("seq", doc.map_or(Json::Null, |s| num(s.seq as f64))),
        ("done", doc.map_or(Json::Null, |s| num(s.done as f64))),
        ("dirty", doc.is_some_and(|s| s.dirty()).into()),
        ("status", g.shell.status.clone().into()),
        ("built", built.unwrap_or(Json::Null)),
    ])
    .to_string()
}

/// Says a panic on the console (the page's tests watch for it) before the trap.
fn panic_hook() {
    std::panic::set_hook(Box::new(|p| console_error(&format!("forge-app panicked: {p}"))));
}

/// The filesystem's changes to the page, but not those it made itself.
fn tell_page(path: &std::path::Path, c: forge_dirs::fs::mem::Change<'_>) {
    if LOADING.with(|l| *l.borrow()) {
        return;
    }
    let p = path.to_string_lossy();
    match c {
        forge_dirs::fs::mem::Change::Wrote(b) => page_changed(&p, Some(Uint8Array::from(b))),
        forge_dirs::fs::mem::Change::Removed => page_changed(&p, None),
    }
}

/// Set up once before anything else: the clock, the panic hook, the filesystem's hook.
#[wasm_bindgen]
pub fn app_init() {
    panic_hook();
    forge_math::clock::set(perf_now, date_now);
    forge_dirs::fs::mem::set_hook(Some(tell_page));
}

/// Starts the app on a canvas appended to the page, with `path` open if given.
#[wasm_bindgen]
pub fn start_app(path: Option<String>) -> Result<(), JsValue> {
    app_init();
    let on_loop = EventLoop::with_user_event().build().map_err(|e| e.to_string())?;
    let proxy = on_loop.create_proxy();
    WAKE.with(|w| *w.borrow_mut() = Some(proxy.clone()));
    on_loop.spawn_app(App {
        proxy,
        host: None,
        started: false,
        first: path,
    });
    Ok(())
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

/// The app's next frame as `{width, height, rgba}`, through a callback (see `Host::pixels`).
#[wasm_bindgen]
pub fn app_pixels(then: js_sys::Function) {
    ask(Ask::Pixels(then));
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
