//! The browser entry: the page hands over a mesh file (see web/index.html), a canvas takes the place of a window.

use crate::bench::{Path, hover_at, report, report_cpu, report_pick};
use crate::camera::Camera;
use crate::mesh::PartMesh;
use crate::render::{OFFSCREEN, Offscreen, Overlay, Renderer, open_device, part_info};
use crate::scene::{Motion, Placement, Scene, stress_of};
use crate::viewer::{Bench, Viewer, now_ms, say};
use js_sys::{Array, Promise, Uint8Array};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::platform::web::{EventLoopExtWebSys, WindowAttributesExtWebSys};
use winit::window::{Window, WindowId};

/// What the page asked for.
#[derive(Clone, Copy)]
struct Asked {
    /// Placements of the stress scene; 0 for the first part alone.
    stress: usize,
    moving: usize,
    /// Frames of a fixed orbit, picking as it goes, the times on the console.
    bench: usize,
}

struct App {
    asked: Asked,
    meshes: Option<Vec<PartMesh>>,
    proxy: EventLoopProxy<Viewer>,
    viewer: Option<Viewer>,
}

async fn open(window: Arc<Window>, parts: Vec<PartMesh>, asked: Asked) -> Result<Viewer, String> {
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
        "forge-render: {} {:?} {}",
        info.name, info.device_type, info.backend
    ));
    let size = window.inner_size();
    let (w, h) = (size.width.max(320), size.height.max(240));
    let mut config = surface
        .get_default_config(&adapter, w, h)
        .ok_or("no surface format")?;
    config.format = config.format.remove_srgb_suffix();
    config.view_formats = vec![config.format.add_srgb_suffix()];
    surface.configure(&device, &config);
    let placements = if asked.stress > 0 {
        stress_of(&parts, asked.stress)
    } else {
        vec![Placement::at(0, [0.0; 3])]
    };
    let scene = Scene::new(&part_info(&parts), placements);
    let (center, radius) = scene.bounds();
    let renderer = Renderer::new(
        device,
        queue,
        info,
        config.format.add_srgb_suffix(),
        (w, h),
        &parts,
        &scene,
    );
    let camera = Camera::fit(center, radius);
    let mut v = Viewer::new(window, surface, config, renderer, scene, camera);
    v.motion = (asked.moving > 0).then(|| Motion::new(&v.scene, asked.moving));
    if asked.bench > 0 {
        let near = if asked.stress > 0 {
            radius * 0.25
        } else {
            camera.dist * 0.35
        };
        v.bench = Some(Bench {
            path: Path {
                center,
                far: camera.dist,
                near,
                frames: asked.bench,
            },
            frame: 0,
            times: Vec::new(),
            gpu: Vec::new(),
            motion_ms: Vec::new(),
            drawn: Vec::new(),
            hover: true,
            pick_ms: Vec::new(),
        });
    }
    Ok(v)
}

impl ApplicationHandler<Viewer> for App {
    fn resumed(&mut self, on_loop: &ActiveEventLoop) {
        let Some(meshes) = self.meshes.take() else { return };
        let attrs = Window::default_attributes()
            .with_title("ForgeCAD")
            .with_append(true);
        let window = Arc::new(on_loop.create_window(attrs).expect("a canvas"));
        let proxy = self.proxy.clone();
        let asked = self.asked;
        wasm_bindgen_futures::spawn_local(async move {
            match open(window, meshes, asked).await {
                Ok(v) => {
                    let _ = proxy.send_event(v);
                }
                Err(e) => say(&format!("forge-render: {e}")),
            }
        });
    }

    fn user_event(&mut self, _: &ActiveEventLoop, v: Viewer) {
        v.window.request_redraw();
        self.viewer = Some(v);
    }

    fn window_event(&mut self, on_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if let Some(v) = &mut self.viewer {
            v.event(event, on_loop);
        }
    }

    fn about_to_wait(&mut self, on_loop: &ActiveEventLoop) {
        if let Some(v) = &mut self.viewer {
            v.about_to_wait(on_loop);
        }
    }
}

fn meshes_of(files: Array) -> Result<Vec<PartMesh>, JsValue> {
    let got: Result<Vec<_>, _> = files
        .iter()
        .map(|f| PartMesh::from_bytes(&Uint8Array::new(&f).to_vec()).ok_or("not a mesh file"))
        .collect();
    Ok(got?)
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

/// The bench with no canvas: for where the browser can't show WebGPU on this GPU (research/render).
/// Each frame's time runs from its start to the GPU finishing it, as native headless. Gives back
/// the last frame as RGBA.
#[wasm_bindgen]
pub async fn bench_offscreen(
    files: Array,
    stress: usize,
    moving: usize,
    frames: usize,
    width: u32,
    height: u32,
) -> Result<Uint8Array, JsValue> {
    let parts = meshes_of(files)?;
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let (device, queue, adapter) = open_device(&instance, None).await?;
    let placements = if stress > 0 {
        stress_of(&parts, stress)
    } else {
        vec![Placement::at(0, [0.0; 3])]
    };
    let mut scene = Scene::new(&part_info(&parts), placements);
    let (center, radius) = scene.bounds();
    let camera = Camera::fit(center, radius);
    let size = (width, height);
    let mut r = Renderer::new(device, queue, adapter.get_info(), OFFSCREEN, size, &parts, &scene);
    let target = Offscreen::new(&r.device, size);
    let fence = r.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("fence"),
        size: 4,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let stamp = r.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("fence stamp"),
        size: 4,
        usage: wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let path = Path {
        center,
        far: camera.dist,
        near: if stress > 0 {
            radius * 0.25
        } else {
            camera.dist * 0.35
        },
        frames,
    };
    let motion = (moving > 0).then(|| Motion::new(&scene, moving));
    let (mut times, mut gpu, mut pick_ms, mut all) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut motion_ms = vec![0.0];
    let mut hud = String::new();
    for frame in 0..frames {
        let cam = path.at(frame);
        let t = now_ms();
        if let Some(m) = &motion {
            m.apply(frame, &mut scene);
            motion_ms.push(now_ms() - t);
        }
        if let Some((_, _, ms)) = r.pick_poll(&scene) {
            pick_ms.push(ms);
        }
        if !r.pick_busy() {
            r.pick_start(&mut scene, &cam, hover_at(frame, size));
        }
        let overlay = Overlay {
            hud: Some(&hud),
            cube: Some((crate::nav::Nav::new(cam, size, 1.0).cube_place(), None)),
            marks: &[],
            world: &[],
        };
        all.push(r.frame(&target.view, &mut scene, &cam, overlay));
        // a buffer the GPU has just been told to write maps once it has got that far
        let mut encoder = r.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&stamp, 0, &fence, 0, 4);
        r.queue.submit([encoder.finish()]);
        called_back(|done| fence.slice(..).map_async(wgpu::MapMode::Read, move |_| done())).await;
        fence.unmap();
        let ms = now_ms() - t;
        hud = format!("frame {ms:.1} ms gpu {:.1} ms", r.gpu_ms.unwrap_or(0.0));
        times.push(ms);
        gpu.push(r.gpu_ms.unwrap_or(0.0) as f64);
    }
    say(&report(
        &format!("browser {width}x{height}"),
        &times,
        &gpu,
        &r.adapter,
    ));
    say(&report_cpu(moving, &motion_ms, &all));
    if !pick_ms.is_empty() {
        say(&report_pick(&pick_ms));
    }
    let (buffer, row) = target.copy_out(&r);
    called_back(|done| buffer.slice(..).map_async(wgpu::MapMode::Read, move |_| done())).await;
    let rgb = target.rgb_of(&buffer, row);
    let rgba: Vec<u8> = rgb
        .as_chunks::<3>()
        .0
        .iter()
        .flat_map(|p| [p[0], p[1], p[2], 255])
        .collect();
    Ok(Uint8Array::from(&rgba[..]))
}

/// Starts drawing in a canvas on the page: the mesh files' parts, the first alone unless `stress`
/// places them all that many times; `moving` of them swing on joints; with `bench` > 0, that many
/// frames of a fixed orbit, picking as it goes, the times on the console.
#[wasm_bindgen]
pub fn start(files: Array, stress: usize, moving: usize, bench: usize) -> Result<(), JsValue> {
    let meshes = meshes_of(files)?;
    let on_loop = EventLoop::with_user_event().build().map_err(|e| e.to_string())?;
    let proxy = on_loop.create_proxy();
    on_loop.spawn_app(App {
        asked: Asked {
            stress,
            moving,
            bench,
        },
        meshes: Some(meshes),
        proxy,
        viewer: None,
    });
    Ok(())
}
