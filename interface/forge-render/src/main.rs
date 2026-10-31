use forge_render::bench::{self, Path};
use forge_render::camera::Camera;
use forge_render::mesh::PartMesh;
use forge_render::render::{self, LineMode, Mark, Offscreen, Overlay, Renderer, part_info};
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
  [--size WxH] [--lod-px T] [--lines near|far|none] [--no-ao] [--section x|y|z] [--explode T] [--no-cache] [--shot FILE.ppm [--select X Y]..] [--yaw R --pitch R --dist X]
  [--export FILE] [--levels] [--dims] [--drag NAME [--drag-demo]] [--drag-bench [--drag-parts A,B] [--ticks N]]
PART: a bench part (default crank). --stress: all 15 bench parts placed N times. --moving N: N of
them swing on joints every frame. --hover: --bench also picks under a wandering pixel each frame. --bench: a fixed
orbit, frame times on stdout, headless unless --window. --shot: one headless frame.

In the window: left drag orbits about what's under the cursor, middle or right drag pans, the wheel
zooms at the cursor. Click selects, ctrl or shift + click adds; one or two selected on a part are
measured. Home or F6 home, A fit all, F fit selection, Esc clear. View cube: click to go, drag to
orbit. L lines near, far, none. O occlusion. S section x, y, z, off; ctrl + wheel moves it. E explode.

--dims lists the numbers a part has. --drag NAME: left drag changes that number live, release commits
it (a file is rewritten; the line prints), Esc cancels, D swaps the left button back to orbiting.
--drag-demo: the same with no mouse, there and back, then exits. --drag-bench: drags over every bench part, value change to frame, headless.";

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
    lines: LineMode,
    ao: bool,
    section: Option<usize>,
    explode: f32,
    cache: bool,
    shot: Option<String>,
    select: Vec<(f32, f32)>,
    export: Option<String>,
    levels: bool,
    yaw: Option<f32>,
    pitch: Option<f32>,
    dist: Option<f32>,
    dims: bool,
    drag: Option<String>,
    drag_demo: bool,
    drag_bench: bool,
    drag_parts: Vec<String>,
    ticks: usize,
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
        lines: LineMode::Near,
        ao: true,
        section: None,
        explode: 0.0,
        cache: true,
        shot: None,
        select: Vec::new(),
        export: None,
        levels: false,
        yaw: None,
        pitch: None,
        dist: None,
        dims: false,
        drag: None,
        drag_demo: false,
        drag_bench: false,
        drag_parts: Vec::new(),
        ticks: 180,
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
            "--lines" => {
                a.lines = it.next().as_deref().and_then(LineMode::from_name).ok_or(USAGE)?;
            }
            "--no-ao" => a.ao = false,
            "--explode" => a.explode = num(it.next(), &k)?,
            "--section" => {
                a.section = match it.next().as_deref() {
                    Some("x") => Some(0),
                    Some("y") => Some(1),
                    Some("z") => Some(2),
                    _ => return Err(USAGE.into()),
                }
            }
            "--no-cache" => a.cache = false,
            "--dims" => a.dims = true,
            "--drag" => a.drag = it.next(),
            "--drag-demo" => a.drag_demo = true,
            "--drag-bench" => a.drag_bench = true,
            "--drag-parts" => {
                a.drag_parts = it
                    .next()
                    .unwrap_or_default()
                    .split(',')
                    .map(String::from)
                    .collect();
            }
            "--ticks" => a.ticks = num(it.next(), &k)?,
            "--shot" => a.shot = it.next(),
            "--select" => {
                let at = (num(it.next(), &k)?, num(it.next(), &k)?);
                a.select.push(at);
            }
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
    names: Vec<String>,
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
        scene::stress_of(&parts, a.stress)
    } else {
        vec![Placement::at(0, [0.0; 3])]
    };
    let mut scene = Scene::new(&part_info(&parts), placements);
    let (center, radius) = scene.bounds();
    scene.set_explode(a.explode);
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
        names: names.iter().map(|n| n.to_string()).collect(),
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
    r.line_mode = a.lines;
    r.ao = a.ao;
    r.section = a.section.map(|axis| {
        let (c, _) = s.scene.bounds();
        let mut p = [0.0; 4];
        p[axis] = 1.0;
        p[3] = -c[axis];
        p
    });
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
        let mut marks = Vec::new();
        for &px in &a.select {
            r.pick_start(&mut s.scene, &s.camera, px);
            r.wait();
            if let Some((_, Some(h), ms)) = r.pick_poll(&s.scene) {
                println!(
                    "pick at {px:?} in {ms:.2} ms: {:?} of placement {}",
                    h.entity, h.placement
                );
                marks.push(Mark {
                    placement: h.placement,
                    entity: h.entity,
                    selected: true,
                });
            }
        }
        // a few frames, so the overlay shows real numbers
        let mut hud = String::new();
        let mut drawn = Default::default();
        for _ in 0..4 {
            let t = now_ms();
            let (w, side) = (a.size.0 as f32, 120.0);
            let overlay = Overlay {
                hud: Some(&hud),
                cube: Some(((w - side - 12.0, 12.0), side, [0; 3])),
                marks: &marks,
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
                marks: &[],
            };
            if a.hover {
                let p = now_ms();
                r.pick_start(&mut s.scene, &cam, bench::hover_at(frame, a.size));
                r.wait();
                r.pick_poll(&s.scene);
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
    drag: Option<(forge_agent::tools::Part, String)>,
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
        renderer.line_mode = self.args.lines;
        renderer.ao = self.args.ao;
        let mut v = Viewer::new(window.clone(), surface, config, renderer, s.scene, s.camera);
        v.measure = forge_render::measure::Measurer::new(s.names);
        if let Some((part, name)) = self.drag.take() {
            let (_, radius) = v.scene.bounds();
            let drag = forge_render::live::Drag::new(&part, &name, radius).expect("checked");
            v.drag = Some(forge_render::viewer::LiveDrag {
                live: forge_render::live::start(part),
                drag,
                on: true,
                cursor_x: 0.0,
                from_x: 0.0,
                waiting: 0,
                last_seq: 0,
                demo: self.args.drag_demo.then_some(120),
                demo_done: false,
            });
        }
        let (c, _) = v.scene.bounds();
        v.nav.section = self.args.section.map(|axis| (axis, c[axis]));
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

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let a = args()?;
    if let Some(path) = &a.export {
        let (mesh, _) = forge_render::load::open(&a.part, a.cache)?;
        return std::fs::write(path, mesh.to_bytes()).map_err(|e| e.to_string());
    }
    if a.dims {
        for d in forge_render::load::tool_part(&a.part)?.dims() {
            println!("{} = {}", d.name, forge_agent::short(d.value));
        }
        return Ok(());
    }
    if a.drag_bench {
        return forge_render::dragbench::run(&a.drag_parts, a.ticks, a.size);
    }
    let drag = match &a.drag {
        Some(name) => {
            let part = forge_render::load::tool_part(&a.part)?;
            forge_render::live::Drag::new(&part, name, 1.0)?;
            Some((part, name.clone()))
        }
        None => None,
    };
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
        drag,
    };
    on_loop.run_app(&mut app).map_err(|e| e.to_string())
}
