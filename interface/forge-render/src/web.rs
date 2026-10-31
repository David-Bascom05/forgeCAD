//! The browser entry: the page hands over a mesh file (see web/index.html), a canvas takes the place of a window.

use crate::bench::Path;
use crate::camera::Camera;
use crate::mesh::PartMesh;
use crate::render::{Renderer, open_device, part_info};
use crate::scene::{Placement, Scene};
use crate::viewer::{Bench, Viewer, say};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::platform::web::{EventLoopExtWebSys, WindowAttributesExtWebSys};
use winit::window::{Window, WindowId};

struct App {
    bench: usize,
    mesh: Option<PartMesh>,
    proxy: EventLoopProxy<Viewer>,
    viewer: Option<Viewer>,
}

async fn open(window: Arc<Window>, mesh: PartMesh, frames: usize) -> Result<Viewer, String> {
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
    let parts = [mesh];
    let scene = Scene::new(&part_info(&parts), vec![Placement::at(0, [0.0; 3])]);
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
    if frames > 0 {
        v.bench = Some(Bench {
            path: Path {
                center,
                far: camera.dist,
                near: camera.dist * 0.35,
                frames,
            },
            frame: 0,
            times: Vec::new(),
            gpu: Vec::new(),
            motion_ms: Vec::new(),
            drawn: Vec::new(),
        });
    }
    Ok(v)
}

impl ApplicationHandler<Viewer> for App {
    fn resumed(&mut self, on_loop: &ActiveEventLoop) {
        let Some(mesh) = self.mesh.take() else { return };
        let attrs = Window::default_attributes()
            .with_title("ForgeCAD")
            .with_append(true);
        let window = Arc::new(on_loop.create_window(attrs).expect("a canvas"));
        let proxy = self.proxy.clone();
        let frames = self.bench;
        wasm_bindgen_futures::spawn_local(async move {
            match open(window, mesh, frames).await {
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
}

/// Starts drawing the mesh file's part in a canvas on the page; with `bench` > 0, that many frames
/// of a fixed orbit, the times on the console.
#[wasm_bindgen]
pub fn start(file: &[u8], bench: usize) -> Result<(), JsValue> {
    let mesh = PartMesh::from_bytes(file).ok_or("not a mesh file")?;
    let on_loop = EventLoop::with_user_event().build().map_err(|e| e.to_string())?;
    let proxy = on_loop.create_proxy();
    on_loop.spawn_app(App {
        bench,
        mesh: Some(mesh),
        proxy,
        viewer: None,
    });
    Ok(())
}
