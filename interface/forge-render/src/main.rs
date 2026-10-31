use forge_render::bench::{self, Path};
use forge_render::camera::Camera;
use forge_render::mesh::PartMesh;
use forge_render::render::{self, Offscreen, Overlay, Renderer, part_info};
use forge_render::scene::{self, Motion, Placement, Scene};
use forge_render::viewer::{Bench, Viewer, now_ms};
use std::fmt::Write;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

const USAGE: &str =
    "forge-render [PART | file.fc] [--stress N] [--moving N] [--bench] [--hover] [--window] [--frames N]
  [--size WxH] [--lod-px T] [--no-lines] [--no-cache] [--shot FILE.ppm] [--yaw R --pitch R --dist X]
  [--export FILE] [--levels]
PART: a bench part (default crank). --stress: all 15 bench parts placed N times. --moving N: N of
them swing on joints every frame. --hover: --bench also picks under a wandering pixel each frame. --bench: a fixed
orbit, frame times on stdout, headless unless --window. --shot: one headless frame.";

struct Args {
    part: String,
    stress: usize,
    moving: usize,
    hover: bool,
    bench: bool,
    window: bool,
    frames: usize,
    size: (u32, u32),
    lod_px: f32,
    lines: bool,
    cache: bool,
    shot: Option<String>,
    export: Option<String>,
    levels: bool,
    yaw: Option<f32>,
    pitch: Option<f32>,
    dist: Option<f32>,
}

fn args() -> Result<Args, String> {
    let mut a = Args {
        part: "crank".into(),
        stress: 0,
        moving: 0,
        hover: false,
        bench: false,
        window: false,
        frames: 1200,
        size: (1920, 1080),
        lod_px: render::LOD_PX,
        lines: true,
        cache: true,
        shot: None,
        export: None,
        levels: false,
        yaw: None,
        pitch: None,
        dist: None,
    };
    let mut it = std::env::args().skip(1);
    fn num<T: std::str::FromStr>(v: Option<String>, flag: &str) -> Result<T, String> {
        v.and_then(|x| x.parse().ok())
            .ok_or(format!("{flag} takes a number\n{USAGE}"))
    }
    while let Some(k) = it.next() {
        match k.as_str() {
            "--stress" => a.stress = num(it.next(), &k)?,
            "--moving" => a.moving = num(it.next(), &k)?,
            "--hover" => a.hover = true,
            "--bench" => a.bench = true,
            "--window" => a.window = true,
            "--frames" => a.frames = num(it.next(), &k)?,
            "--lod-px" => a.lod_px = num(it.next(), &k)?,
            "--no-lines" => a.lines = false,
            "--no-cache" => a.cache = false,
            "--shot" => a.shot = it.next(),
            "--export" => a.export = it.next(),
            "--levels" => a.levels = true,
            "--yaw" => a.yaw = Some(num(it.next(), &k)?),
            "--pitch" => a.pitch = Some(num(it.next(), &k)?),
            "--dist" => a.dist = Some(num(it.next(), &k)?),
            "--size" => {
                let v = it.next().unwrap_or_default();
                let (w, h) = v.split_once('x').ok_or(USAGE)?;
                a.size = (num(Some(w.into()), &k)?, num(Some(h.into()), &k)?);
            }
            s if s.starts_with("--") => return Err(USAGE.into()),
            part => a.part = part.into(),
        }
    }
    Ok(a)
}

struct Stage {
    parts: Vec<PartMesh>,
    scene: Scene,
    camera: Camera,
    path: Path,
    spent_ms: f64,
    hits: usize,
}

fn stage(a: &Args) -> Result<Stage, String> {
    let t = now_ms();
    let names: Vec<&str> = if a.stress > 0 {
        forge_agent::bench::NAMES.to_vec()
    } else {
        vec![a.part.as_str()]
    };
    let (parts, hits) = forge_render::load::open_all(&names, a.cache)?;
    let placements = if a.stress > 0 {
        // a spacing the largest part about fills, so neighbours overlap little and hide much
        let r = parts.iter().map(|p| p.radius).sum::<f32>() / parts.len() as f32;
        scene::stress(parts.len(), a.stress, r * 2.5)
    } else {
        vec![Placement::at(0, [0.0; 3])]
    };
    let scene = Scene::new(&part_info(&parts), placements);
    let (center, radius) = scene.bounds();
    let mut camera = Camera::fit(center, radius);
    camera.yaw = a.yaw.unwrap_or(camera.yaw);
    camera.pitch = a.pitch.unwrap_or(camera.pitch);
    camera.dist = a.dist.unwrap_or(camera.dist);
    let near = if a.stress > 0 {
        radius * 0.25
    } else {
        camera.dist * 0.35
    };
    let path = Path {
        center,
        far: camera.dist,
        near,
        frames: a.frames,
    };
    Ok(Stage {
        parts,
        scene,
        camera,
        path,
        spent_ms: now_ms() - t,
        hits,
    })
}

fn headless(a: &Args, mut s: Stage) -> Result<(), String> {
    let t = now_ms();
    let instance = render::native_instance();
    let (device, queue, adapter) = render::block_on(render::open_device(&instance, None))?;
    let device_ms = now_ms() - t;
    let mut r = Renderer::new(
        device,
        queue,
        adapter.get_info(),
        render::OFFSCREEN,
        a.size,
        &s.parts,
        &s.scene,
    );
    r.lod_px = a.lod_px;
    r.lines = a.lines;
    let bytes: usize = s
        .parts
        .iter()
        .flat_map(|p| &p.lods)
        .map(|l| l.vertices.len() + l.indices.len() + l.segments.len())
        .sum();
    let target = Offscreen::new(&r.device, a.size);
    r.frame(&target.view, &mut s.scene, &s.camera, Overlay::default());
    r.wait();
    println!(
        "open: {:.0} ms meshes ({} of {} from cache), {device_ms:.0} ms GPU driver, {:.0} ms upload and first frame, {:.0} MB on the GPU",
        s.spent_ms,
        s.hits,
        s.parts.len(),
        now_ms() - t - device_ms,
        bytes as f64 / 1e6
    );

    if let Some(path) = &a.shot {
        // a few frames, so the overlay shows real numbers
        let mut hud = String::new();
        let mut drawn = Default::default();
        for _ in 0..4 {
            let t = now_ms();
            let (w, side) = (a.size.0 as f32, 120.0);
            let overlay = Overlay {
                hud: Some(&hud),
                cube: Some(((w - side - 12.0, 12.0), side, [0; 3])),
            };
            drawn = r.frame(&target.view, &mut s.scene, &s.camera, overlay);
            r.wait();
            hud = format!(
                "frame {:.1} ms gpu {:.1} ms",
                now_ms() - t,
                r.gpu_ms.unwrap_or(0.0)
            );
        }
        println!("{} {:?}", r.adapter.name, drawn);
        let mut out = format!("P6\n{} {}\n255\n", a.size.0, a.size.1).into_bytes();
        out.extend(target.read_rgb(&r));
        std::fs::write(path, out).map_err(|e| e.to_string())?;
    }
    if a.bench {
        let motion = (a.moving > 0).then(|| Motion::new(&s.scene, a.moving));
        let (mut times, mut gpu, mut motion_ms, mut all) = (Vec::new(), Vec::new(), vec![0.0], Vec::new());
        let mut pick_ms = Vec::new();
        let mut heaviest = (0.0f64, 0, Default::default());
        let mut hud = String::new();
        for frame in 0..a.frames {
            let cam = s.path.at(frame);
            let t = now_ms();
            if let Some(m) = &motion {
                m.apply(frame, &mut s.scene);
                motion_ms.push(now_ms() - t);
            }
            let overlay = Overlay {
                hud: Some(&hud),
                cube: Some(((a.size.0 as f32 - 132.0, 12.0), 120.0, [0; 3])),
            };
            if a.hover {
                let p = now_ms();
                r.pick_start(&mut s.scene, &cam, bench::hover_at(frame, a.size));
                r.wait();
                r.pick_poll();
                pick_ms.push(now_ms() - p);
            }
            let d = r.frame(&target.view, &mut s.scene, &cam, overlay);
            r.wait();
            let ms = now_ms() - t;
            hud.clear();
            let _ = write!(hud, "frame {ms:.1} ms gpu {:.1} ms", r.gpu_ms.unwrap_or(0.0));
            if ms > heaviest.0 {
                heaviest = (ms, frame, d);
            }
            times.push(ms);
            gpu.push(r.gpu_ms.unwrap_or(0.0) as f64);
            all.push(d);
        }
        let mode = format!("headless {}x{}", a.size.0, a.size.1);
        println!("{}", bench::report(&mode, &times, &gpu, &r.adapter));
        println!("{}", bench::report_cpu(a.moving, &motion_ms, &all));
        if !pick_ms.is_empty() {
            println!("{}", bench::report_pick(&pick_ms));
        }
        println!("slowest frame {} of {}: {:?}", heaviest.1, a.frames, heaviest.2);
    }
    Ok(())
}

struct App {
    stage: Option<Stage>,
    args: Args,
    viewer: Option<Viewer>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, on_loop: &ActiveEventLoop) {
        let Some(s) = self.stage.take() else { return };
        let attrs = Window::default_attributes()
            .with_title("ForgeCAD")
            .with_inner_size(winit::dpi::PhysicalSize::new(self.args.size.0, self.args.size.1));
        let window = Arc::new(on_loop.create_window(attrs).expect("a window"));
        let instance = render::native_instance();
        let surface = instance.create_surface(window.clone()).expect("a surface");
        let (device, queue, adapter) =
            render::block_on(render::open_device(&instance, Some(&surface))).expect("a GPU");
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .expect("the GPU draws to this window");
        config.format = config.format.remove_srgb_suffix();
        config.view_formats = vec![config.format.add_srgb_suffix()];
        config.desired_maximum_frame_latency = 1;
        if self.args.bench {
            config.present_mode = wgpu::PresentMode::AutoNoVsync;
        }
        surface.configure(&device, &config);
        let mut renderer = Renderer::new(
            device,
            queue,
            adapter.get_info(),
            config.format.add_srgb_suffix(),
            (config.width, config.height),
            &s.parts,
            &s.scene,
        );
        renderer.lod_px = self.args.lod_px;
        renderer.lines = self.args.lines;
        let mut v = Viewer::new(window.clone(), surface, config, renderer, s.scene, s.camera);
        v.motion = (self.args.moving > 0).then(|| Motion::new(&v.scene, self.args.moving));
        if self.args.bench {
            v.bench = Some(Bench {
                path: s.path,
                frame: 0,
                times: Vec::new(),
                gpu: Vec::new(),
                motion_ms: Vec::new(),
                drawn: Vec::new(),
                hover: self.args.hover,
                pick_ms: Vec::new(),
            });
        }
        window.request_redraw();
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

fn main() -> Result<(), String> {
    let a = args()?;
    if let Some(path) = &a.export {
        let (mesh, _) = forge_render::load::open(&a.part, a.cache)?;
        return std::fs::write(path, mesh.to_bytes()).map_err(|e| e.to_string());
    }
    let s = stage(&a)?;
    if a.levels {
        for p in &s.parts {
            print!("r {:6.1}", p.radius);
            for l in &p.lods {
                print!(
                    "  {}:{}/{}",
                    l.chord,
                    l.tris(),
                    l.segments.len() / forge_render::mesh::SEGMENT
                );
            }
            println!();
        }
        return Ok(());
    }
    if a.shot.is_some() || (a.bench && !a.window) {
        return headless(&a, s);
    }
    let on_loop = EventLoop::new().map_err(|e| e.to_string())?;
    let mut app = App {
        stage: Some(s),
        args: a,
        viewer: None,
    };
    on_loop.run_app(&mut app).map_err(|e| e.to_string())
}
