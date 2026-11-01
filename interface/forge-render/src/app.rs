//! The desktop app: a winit window, with no title bar of the system's, driving `Gui`. It sleeps
//! until something happens: input, a timer egui asked for, or the worker saying it has an answer.

use crate::gui::{Gui, Waker};
use crate::render;
use crate::shell::{Action, Edge};
use crate::viewer::now_ms;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::{ResizeDirection, Window, WindowId};

/// `--times` says what a frame longer than this spent (ms).
const SLOW_MS: f64 = 16.0;

struct Live {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    gui: Gui,
    egui: egui_winit::State,
    /// Input to pixels: when the oldest input not yet on screen came.
    input_at: Option<f64>,
    presenter: Presenter,
    /// The window changed size while a frame was going out.
    configure: bool,
}

/// Presents on its own thread. A present can wait a long time on the compositor, and wgpu holds its
/// submit lock for all of it: so the loop neither presents nor submits while one's out, and the frame
/// it would have drawn is dropped for a later one that shows everything.
struct Presenter {
    send: Option<std::sync::mpsc::Sender<(wgpu::SurfaceTexture, Option<f64>)>>,
    thread: Option<std::thread::JoinHandle<()>>,
    out: Arc<PresentState>,
}

#[derive(Default)]
struct PresentState {
    busy: AtomicBool,
    /// The loop wanted to draw while it was busy.
    wanted: AtomicBool,
    log: Mutex<PresentLog>,
}

#[derive(Default)]
struct PresentLog {
    /// How long each present call took.
    calls: Vec<f64>,
    /// Input to the frame showing it having gone out.
    inputs: Vec<f64>,
    dropped: usize,
}

impl Presenter {
    fn start(queue: wgpu::Queue, wake: Waker, times: bool) -> Presenter {
        let (send, recv) = std::sync::mpsc::channel::<(wgpu::SurfaceTexture, Option<f64>)>();
        let out = Arc::new(PresentState::default());
        let state = out.clone();
        let thread = std::thread::Builder::new()
            .name("present".into())
            .spawn(move || {
                while let Ok((frame, input_at)) = recv.recv() {
                    let t = now_ms();
                    queue.present(frame);
                    let done = now_ms();
                    {
                        let mut log = state.log.lock().unwrap_or_else(|e| e.into_inner());
                        log.calls.push(done - t);
                        if let Some(at) = input_at {
                            log.inputs.push(done - at);
                        }
                    }
                    if times && done - t > SLOW_MS {
                        eprintln!("forge-app: slow present {:.1} ms", done - t);
                    }
                    state.busy.store(false, Ordering::SeqCst);
                    if state.wanted.swap(false, Ordering::SeqCst) {
                        wake();
                    }
                }
            })
            .expect("a thread");
        Presenter {
            send: Some(send),
            thread: Some(thread),
            out,
        }
    }

    /// Is one out? If so, the loop's woken when it's back.
    fn busy(&self) -> bool {
        if !self.out.busy.load(Ordering::SeqCst) {
            return false;
        }
        self.out.wanted.store(true, Ordering::SeqCst);
        if self.out.busy.load(Ordering::SeqCst) {
            return true;
        }
        self.out.wanted.store(false, Ordering::SeqCst);
        false
    }

    fn present(&self, frame: wgpu::SurfaceTexture, input_at: Option<f64>) {
        self.out.busy.store(true, Ordering::SeqCst);
        if let Some(s) = &self.send {
            let _ = s.send((frame, input_at));
        }
    }

    fn dropped(&self) {
        self.out.log.lock().unwrap_or_else(|e| e.into_inner()).dropped += 1;
    }

    /// Waits for what's out; nothing's presented after.
    fn stop(&mut self) {
        self.send = None;
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn spread(mut v: Vec<f64>) -> String {
    v.sort_by(f64::total_cmp);
    let at = |p: f64| v[((v.len() - 1) as f64 * p) as usize];
    format!(
        "{} of them, p50 {:.2} p95 {:.2} p99 {:.2} max {:.2} ms",
        v.len(),
        at(0.5),
        at(0.95),
        at(0.99),
        at(1.0)
    )
}

/// How frames go to the compositor: what was asked for if the window can, else one that never waits
/// on its frame callbacks, else the one every window has.
fn present_mode(asked: Option<&str>, has: &[wgpu::PresentMode]) -> wgpu::PresentMode {
    use wgpu::PresentMode as P;
    let named = asked.and_then(|a| match a {
        "fifo" => Some(P::Fifo),
        "mailbox" => Some(P::Mailbox),
        "immediate" => Some(P::Immediate),
        _ => None,
    });
    named
        .into_iter()
        .chain([P::Mailbox, P::Fifo])
        .find(|m| has.contains(m))
        .unwrap_or(P::Fifo)
}

pub struct App {
    file: Option<String>,
    size: (f64, f64),
    proxy: EventLoopProxy<()>,
    live: Option<Live>,
    mods: ModifiersState,
    next: Option<Instant>,
    /// Report input-to-pixels on exit (`--times`).
    times: bool,
    dialog: Option<std::sync::mpsc::Receiver<Option<String>>>,
    /// `--present`: how frames go out.
    present: Option<String>,
}

fn resize_direction(e: Edge) -> ResizeDirection {
    match e {
        Edge::N => ResizeDirection::North,
        Edge::S => ResizeDirection::South,
        Edge::E => ResizeDirection::East,
        Edge::W => ResizeDirection::West,
        Edge::NE => ResizeDirection::NorthEast,
        Edge::NW => ResizeDirection::NorthWest,
        Edge::SE => ResizeDirection::SouthEast,
        Edge::SW => ResizeDirection::SouthWest,
    }
}

impl App {
    fn draw(&mut self, on_loop: &ActiveEventLoop) {
        let Some(l) = &mut self.live else { return };
        if l.presenter.busy() {
            l.presenter.dropped();
            return;
        }
        if std::mem::take(&mut l.configure) {
            let device = l.gui.device();
            l.surface.configure(&device, &l.config);
        }
        use wgpu::CurrentSurfaceTexture as T;
        let began = now_ms();
        let frame = match l.surface.get_current_texture() {
            T::Success(f) | T::Suboptimal(f) => f,
            T::Outdated | T::Lost => {
                let device = l.gui.device();
                l.surface.configure(&device, &l.config);
                l.window.request_redraw();
                return;
            }
            _ => return,
        };
        let acquired = now_ms();
        let view = frame.texture.create_view(&Default::default());
        let raw = l.egui.take_egui_input(&l.window);
        let out = l.gui.frame(raw, &view, (l.config.width, l.config.height));
        let drawn = now_ms();
        l.egui.handle_platform_output(&l.window, out.platform);
        let output_ms = now_ms() - drawn;
        l.window.pre_present_notify();
        l.presenter.present(frame, l.input_at.take());
        let done = now_ms();
        if self.times && done - began > SLOW_MS {
            let t = out.times;
            eprintln!(
                "forge-app: slow frame {:.1} ms: acquire {:.1}, ui {:.1}, view {:.1} (drawn {}), tessellate {:.1}, draw {:.1}, platform {output_ms:.1}",
                done - began,
                acquired - began,
                t.ui,
                t.view,
                t.view_drawn,
                t.tess,
                t.draw,
            );
        }
        self.next = None;
        match out.again_in {
            Some(d) if d.is_zero() => l.window.request_redraw(),
            Some(d) => self.next = Some(Instant::now() + d),
            None => {}
        }
        for a in out.host {
            match a {
                Action::Close => on_loop.exit(),
                Action::Minimize => l.window.set_minimized(true),
                Action::Maximize => l.window.set_maximized(!l.window.is_maximized()),
                Action::DragWindow => {
                    let _ = l.window.drag_window();
                }
                Action::ResizeWindow(e) => {
                    let _ = l.window.drag_resize_window(resize_direction(e));
                }
                Action::New | Action::Open => {
                    let (send, recv) = std::sync::mpsc::channel();
                    self.dialog = Some(recv);
                    let proxy = self.proxy.clone();
                    let new = matches!(a, Action::New);
                    std::thread::spawn(move || {
                        let chosen = if new {
                            crate::app::new_dialog()
                        } else {
                            crate::app::open_dialog()
                        };
                        let _ = send.send(chosen);
                        let _ = proxy.send_event(());
                    });
                }
                _ => {}
            }
        }
        l.gui.shell.view.maximized = l.window.is_maximized();
    }
}

/// The system's dialog for where a new part goes; it starts as an empty file.
fn new_dialog() -> Option<String> {
    let f = rfd::FileDialog::new()
        .add_filter("Parts", &["fc"])
        .set_file_name("Part.fc")
        .save_file()?;
    let f = if f.extension().is_some() {
        f
    } else {
        f.with_extension("fc")
    };
    std::fs::write(&f, "").ok()?;
    Some(f.to_string_lossy().into_owned())
}

/// The system's file dialog.
fn open_dialog() -> Option<String> {
    let f = rfd::FileDialog::new().add_filter("Parts", &["fc"]).pick_file()?;
    Some(f.to_string_lossy().into_owned())
}

impl ApplicationHandler for App {
    fn resumed(&mut self, on_loop: &ActiveEventLoop) {
        if self.live.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("ForgeCAD")
            .with_decorations(false)
            .with_inner_size(LogicalSize::new(self.size.0, self.size.1))
            .with_min_inner_size(LogicalSize::new(640.0, 400.0));
        let window = Arc::new(on_loop.create_window(attrs).expect("a window"));
        let instance = render::native_instance();
        let surface = instance.create_surface(window.clone()).expect("a surface");
        let (device, queue, adapter) =
            render::block_on(render::open_device(&instance, Some(&surface))).expect("a GPU");
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .expect("the GPU draws to this window");
        let modes = surface.get_capabilities(&adapter).present_modes;
        config.present_mode = present_mode(self.present.as_deref(), &modes);
        eprintln!(
            "forge-app: present modes {modes:?}, using {:?}",
            config.present_mode
        );
        // the chrome blends in sRGB, as a browser does
        config.format = config.format.remove_srgb_suffix();
        config.desired_maximum_frame_latency = 1;
        surface.configure(&device, &config);
        let info = adapter.get_info();
        eprintln!(
            "forge-app: {} ({:?}), window {}x{} at scale {}, {:?} {:?}",
            info.name,
            info.backend,
            config.width,
            config.height,
            window.scale_factor(),
            config.format,
            config.present_mode
        );
        let proxy = self.proxy.clone();
        let waker: Waker = Arc::new(move || {
            let _ = proxy.send_event(());
        });
        let scale = window.scale_factor() as f32;
        let presenter = Presenter::start(queue.clone(), waker.clone(), self.times);
        let mut gui = Gui::new(device, queue, adapter.get_info(), config.format, waker, scale);
        if let Some(f) = self.file.take() {
            gui.open(&f);
        }
        let egui = egui_winit::State::new(
            gui.ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            window.theme(),
            None,
        );
        window.request_redraw();
        self.live = Some(Live {
            window,
            surface,
            config,
            gui,
            egui,
            input_at: None,
            presenter,
            configure: false,
        });
    }

    fn user_event(&mut self, _: &ActiveEventLoop, _: ()) {
        if let Some(d) = &self.dialog
            && let Ok(got) = d.try_recv()
        {
            self.dialog = None;
            if let (Some(path), Some(l)) = (got, &mut self.live) {
                l.gui.open(&path);
            }
        }
        if let Some(l) = &self.live {
            l.window.request_redraw();
        }
    }

    fn window_event(&mut self, on_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if let WindowEvent::RedrawRequested = event {
            self.draw(on_loop);
            return;
        }
        let Some(l) = &mut self.live else { return };
        let response = l.egui.on_window_event(&l.window, &event);
        l.gui.view_event(&event);
        if matches!(
            event,
            WindowEvent::MouseInput { .. } | WindowEvent::KeyboardInput { .. }
        ) && l.input_at.is_none()
        {
            l.input_at = Some(now_ms());
        }
        let mut redraw = response.repaint;
        match &event {
            WindowEvent::CloseRequested => on_loop.exit(),
            WindowEvent::Resized(PhysicalSize { width, height }) if *width > 0 && *height > 0 => {
                l.config.width = *width;
                l.config.height = *height;
                l.configure = true;
                redraw = true;
            }
            WindowEvent::ModifiersChanged(m) => self.mods = m.state(),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    let ctrl = self.mods.control_key();
                    let shift = self.mods.shift_key();
                    let action = match code {
                        KeyCode::KeyZ if ctrl && shift => Some(Action::Redo),
                        KeyCode::KeyZ if ctrl => Some(Action::Undo),
                        KeyCode::KeyY if ctrl => Some(Action::Redo),
                        KeyCode::KeyO if ctrl => Some(Action::Open),
                        _ => None,
                    };
                    if let Some(a) = action {
                        if !l.gui.ctx.egui_wants_keyboard_input() {
                            l.gui.shell.out.push(a);
                            redraw = true;
                        }
                    } else if !ctrl {
                        redraw |= l.gui.view_key(code);
                    }
                }
            }
            _ => {}
        }
        if l.gui.view_dirty() {
            redraw = true;
        }
        if redraw {
            l.window.request_redraw();
        }
    }

    fn about_to_wait(&mut self, on_loop: &ActiveEventLoop) {
        let Some(l) = &mut self.live else { return };
        // nothing can be drawn till it's back, and it wakes the loop
        if l.presenter.busy() {
            on_loop.set_control_flow(ControlFlow::Wait);
            return;
        }
        if l.gui.picks() {
            l.window.request_redraw();
        }
        if let Some(n) = self.next
            && Instant::now() >= n
        {
            self.next = None;
            l.window.request_redraw();
        }
        on_loop.set_control_flow(if l.gui.pick_busy() {
            ControlFlow::Poll
        } else if let Some(n) = self.next {
            ControlFlow::WaitUntil(n)
        } else {
            ControlFlow::Wait
        });
    }

    fn exiting(&mut self, _: &ActiveEventLoop) {
        if let Some(l) = &mut self.live {
            l.presenter.stop();
            if self.times {
                let log = l.presenter.out.log.lock().unwrap_or_else(|e| e.into_inner());
                if !log.inputs.is_empty() {
                    println!("input to present: {}", spread(log.inputs.clone()));
                }
                if !log.calls.is_empty() {
                    println!("present calls: {}", spread(log.calls.clone()));
                }
                println!("frames dropped while one was out: {}", log.dropped);
            }
        }
        // while the display connection is still open: what holds it goes first (the clipboard), then
        // the worker and the GPU's work, then the swapchain, and the window last. After this the
        // event loop drops its connection with nothing left to use it.
        if let Some(l) = self.live.take() {
            let Live {
                window,
                surface,
                gui,
                egui,
                ..
            } = l;
            drop(egui);
            let device = gui.device();
            let _ = device.poll(wgpu::PollType::wait_indefinitely());
            drop(surface);
            drop(gui);
            drop(device);
            drop(window);
        }
    }
}

/// Runs the app, on this file if there is one.
pub fn run(
    file: Option<String>,
    size: (f64, f64),
    times: bool,
    present: Option<String>,
) -> Result<(), String> {
    let on_loop = EventLoop::with_user_event().build().map_err(|e| e.to_string())?;
    let proxy = on_loop.create_proxy();
    let mut app = App {
        file,
        size,
        proxy,
        live: None,
        mods: ModifiersState::empty(),
        next: None,
        times,
        dialog: None,
        present,
    };
    on_loop.run_app(&mut app).map_err(|e| e.to_string())
}
