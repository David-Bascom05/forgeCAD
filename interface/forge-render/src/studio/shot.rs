//! A render without a window: a part or an assembly, its appearances, the lighting, the camera,
//! to PNG files (one, or a turntable's frames). `forge-studio` is its command line.

use super::appearance::Appearances;
use super::bsdf::{Bsdf, Lut};
use super::env::{self, Env};
use super::gpu::Tracer;
use super::scene::{Built, Instance};
use super::{Backdrop, Settings, View};
use crate::camera::Camera;
use crate::mesh::PartMesh;
use super::v3::{V3, v3};

/// What to render.
pub struct Stage {
    pub parts: Vec<PartMesh>,
    /// Each placement: its part, its pose, its appearance.
    pub placed: Vec<(u32, [[f32; 4]; 3], Bsdf)>,
}

impl Stage {
    /// One part, or an assembly (`.fg` with occurrences) exploded `explode` of the way, each
    /// placement given the next of `looks` in turn.
    pub fn open(what: &str, explode: f32, looks: &[Bsdf]) -> Result<Stage, String> {
        let looks = if looks.is_empty() { vec![Bsdf::default()] } else { looks.to_vec() };
        if what.ends_with(".fg")
            && let Ok(mut a) = crate::asmshot::stage(what)
        {
            a.explode(explode);
            let n = a.scene.placements.len();
            let placed = (0..n)
                .map(|i| {
                    let mut r = a.scene.rows_of(i);
                    let off = a.scene.offset_of(i);
                    for k in 0..3 {
                        r[k][3] += off[k];
                    }
                    let part = a.scene.placements[i].part;
                    (part, r, looks[part as usize % looks.len()])
                })
                .collect();
            return Ok(Stage { parts: a.parts, placed });
        }
        let (mesh, _) = crate::load::open(what, false)?;
        let rows = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]];
        Ok(Stage {
            parts: vec![mesh],
            placed: vec![(0, rows, looks[0])],
        })
    }

    /// The tracer's scene, its appearances (the floor's last), and its bounds.
    pub fn build(&self, floor: Bsdf) -> (Built, Vec<Bsdf>) {
        let mut mats: Vec<Bsdf> = Vec::new();
        let inst: Vec<Instance> = self
            .placed
            .iter()
            .map(|(part, rows, b)| {
                let m = mats.iter().position(|x| x == b).unwrap_or_else(|| {
                    mats.push(*b);
                    mats.len() - 1
                });
                Instance {
                    part: *part,
                    rows: *rows,
                    material: m as u32,
                }
            })
            .collect();
        mats.push(floor);
        (Built::new(&self.parts, &inst), mats)
    }
}

/// The lighting: a Radiance file or the drawn studio.
pub fn lighting(hdri: Option<&str>) -> Result<Env, String> {
    match hdri {
        Some(path) => {
            let bytes = forge_dirs::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
            let (w, h, rgb) = env::read_hdr(&bytes).map_err(|e| format!("{path}: {e}"))?;
            Ok(Env::new(w, h, rgb))
        }
        None => Ok(Env::new(512, 256, env::studio(512, 256, 1.0))),
    }
}

#[derive(Clone, Debug)]
pub struct Shot {
    pub size: (u32, u32),
    pub spp: u32,
    pub yaw: f32,
    pub pitch: f32,
    /// The camera's distance over the fitting one.
    pub zoom: f32,
    pub focal_mm: f32,
    pub f_number: Option<f32>,
    pub parallel: bool,
    pub settings: Settings,
    pub floor: bool,
    pub denoise: bool,
}

impl Default for Shot {
    fn default() -> Shot {
        Shot {
            size: (1280, 800),
            spp: 256,
            yaw: 35f32.to_radians(),
            pitch: 22f32.to_radians(),
            zoom: 1.0,
            focal_mm: 85.0,
            f_number: None,
            parallel: false,
            settings: Settings {
                exposure: -0.5,
                clamp: 16.0,
                ..Settings::default()
            },
            floor: true,
            denoise: true,
        }
    }
}

/// The camera for a shot of these bounds, turned `extra_yaw` more: as close as fits the box's
/// corners with a margin (`zoom` over that).
pub fn view_of(shot: &Shot, built: &Built, extra_yaw: f32) -> View {
    let b = built.bounds.expect("something to see");
    let centre = (b.min + b.max) * 0.5;
    let radius = (b.max - b.min).len() * 0.5;
    let mut cam = Camera::fit(centre.arr(), radius);
    cam.yaw = shot.yaw + extra_yaw;
    cam.pitch = shot.pitch;
    cam.fovy = 2.0 * (super::FRAME_MM * 0.5 / shot.focal_mm).atan();
    let (f, r, u) = cam.axes();
    let (f, r, u) = (V3::from(f), V3::from(r), V3::from(u));
    let ty = (cam.fovy * 0.5).tan();
    let tx = ty * shot.size.0 as f32 / shot.size.1 as f32;
    let corners = (0..8).map(|k| {
        v3(
            if k & 1 == 0 { b.min.x } else { b.max.x },
            if k & 2 == 0 { b.min.y } else { b.max.y },
            if k & 4 == 0 { b.min.z } else { b.max.z },
        ) - centre
    });
    let mut dist: f32 = 0.0;
    let mut half_h: f32 = 0.0;
    for c in corners {
        dist = dist.max((c.dot(r).abs() / tx).max(c.dot(u).abs() / ty) - c.dot(f));
        half_h = half_h.max(c.dot(u).abs()).max(c.dot(r).abs() * ty / tx);
    }
    cam.dist = dist * 1.12 * shot.zoom;
    if shot.parallel {
        cam.set_parallel(true, radius);
        // the same margin, square to the box's own extent
        let mut v = View::of(&cam);
        v.parallel = half_h * 1.12 * shot.zoom;
        return v;
    }
    View::of(&cam).with_lens(shot.focal_mm, shot.f_number, cam.dist)
}

/// Renders `frames` pictures (a turntable when more than one), calling `done` with each one's
/// RGBA bytes; returns the samples per second achieved.
pub fn render(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    stage: &Stage,
    env: &Env,
    shot: &Shot,
    frames: u32,
    mut done: impl FnMut(u32, Vec<u8>),
) -> Result<f64, String> {
    let apps = Appearances::builtin();
    let floor = apps.find("Studio floor").map(|a| a.bsdf).unwrap_or_default();
    let (built, mats) = stage.build(floor);
    let Some(b) = built.bounds else {
        return Err("nothing to render".into());
    };
    let mut settings = shot.settings;
    if shot.floor && settings.ground.is_none() {
        settings.ground = Some((b.min.z, (mats.len() - 1) as u32));
    }
    let lut = Lut::new(512);
    let mut t = Tracer::new(device, queue, shot.size);
    t.set_scene(&built, &mats, &lut)?;
    t.set_env(env);
    let start = std::time::Instant::now();
    for f in 0..frames.max(1) {
        let turn = if frames > 1 { f as f32 / frames as f32 * std::f32::consts::TAU } else { 0.0 };
        let view = view_of(shot, &built, turn);
        t.reset();
        // in batches, so a slow GPU's watchdog never sees one long submission
        let mut left = shot.spp;
        while left > 0 {
            let n = left.min(16);
            t.step(&view, &settings, n);
            t.wait();
            left -= n;
        }
        t.show(settings.exposure, shot.denoise);
        done(f, t.read_rgba());
    }
    let secs = start.elapsed().as_secs_f64();
    Ok((shot.size.0 * shot.size.1) as f64 * shot.spp as f64 * frames.max(1) as f64 / secs.max(1e-9))
}

pub use super::save_png;

pub const USAGE: &str = "forge-studio PART.fg|ASSEMBLY.fg|BENCH -o OUT.png [options]
  --look NAME[;NAME..]   appearances (data/appearances.toml), one per part in turn
  --hdri FILE.hdr        lighting from a Radiance map (default: the drawn studio)
  --turn DEG --strength X --exposure EV
  --size WxH --spp N --seed N --bounces N
  --yaw DEG --pitch DEG --zoom X --focal MM --fstop N --parallel
  --explode T            an assembly's parts T of the way apart
  --no-floor --backdrop env|transparent|RRGGBB --no-denoise
  --turntable N          N frames about z: OUT_0000.png ..";

/// `forge-studio`'s arguments to a render; the output path and frames.
pub fn main_args(args: &[String]) -> Result<(), String> {
    let mut it = args.iter();
    let mut what = None;
    let mut out = None;
    let mut looks = Vec::new();
    let mut hdri = None;
    let mut shot = Shot::default();
    let mut explode = 0.0;
    let mut frames = 1;
    let num = |v: Option<&String>, what: &str| -> Result<f32, String> {
        v.and_then(|s| s.parse().ok()).ok_or(format!("{what} wants a number"))
    };
    let apps = Appearances::builtin();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-o" => out = it.next().cloned(),
            "--look" => {
                for n in it.next().ok_or("--look wants names")?.split(';') {
                    let a = apps.find(n).ok_or_else(|| {
                        let all: Vec<&str> = apps.all().map(|a| a.name.as_str()).collect();
                        format!("no appearance {n}; there are: {}", all.join(", "))
                    })?;
                    looks.push(a.bsdf);
                }
            }
            "--hdri" => hdri = it.next().cloned(),
            "--turn" => shot.settings.env_turn = num(it.next(), a)?.to_radians(),
            "--strength" => shot.settings.env_strength = num(it.next(), a)?,
            "--exposure" => shot.settings.exposure = num(it.next(), a)?,
            "--size" => {
                let s = it.next().ok_or("--size wants WxH")?;
                let (w, h) = s.split_once('x').ok_or("--size wants WxH")?;
                shot.size = (w.parse().map_err(|_| "width")?, h.parse().map_err(|_| "height")?);
            }
            "--spp" => shot.spp = num(it.next(), a)? as u32,
            "--seed" => shot.settings.seed = num(it.next(), a)? as u32,
            "--bounces" => shot.settings.bounces = num(it.next(), a)? as u32,
            "--yaw" => shot.yaw = num(it.next(), a)?.to_radians(),
            "--pitch" => shot.pitch = num(it.next(), a)?.to_radians(),
            "--zoom" => shot.zoom = num(it.next(), a)?,
            "--focal" => shot.focal_mm = num(it.next(), a)?,
            "--fstop" => shot.f_number = Some(num(it.next(), a)?),
            "--parallel" => shot.parallel = true,
            "--explode" => explode = num(it.next(), a)?,
            "--no-floor" => shot.floor = false,
            "--no-denoise" => shot.denoise = false,
            "--backdrop" => {
                shot.settings.backdrop = match it.next().map(String::as_str) {
                    Some("env") => Backdrop::Environment,
                    Some("transparent") => Backdrop::Transparent,
                    Some(hex) => {
                        let v = u32::from_str_radix(hex, 16).map_err(|_| "--backdrop env|transparent|RRGGBB")?;
                        Backdrop::Colour(crate::viewcube::rgb(v))
                    }
                    None => return Err("--backdrop env|transparent|RRGGBB".into()),
                }
            }
            "--turntable" => frames = num(it.next(), a)? as u32,
            "-h" | "--help" => return Err(USAGE.into()),
            s if what.is_none() && !s.starts_with('-') => what = Some(s.to_string()),
            s => return Err(format!("{s}?\n{USAGE}")),
        }
    }
    let what = what.ok_or(USAGE)?;
    let out = out.ok_or(USAGE)?;
    let stage = Stage::open(&what, explode, &looks)?;
    let env = lighting(hdri.as_deref())?;
    let instance = crate::render::native_instance();
    let (device, queue, adapter) = crate::render::block_on(crate::render::open_device(&instance, None))?;
    eprintln!("forge-studio: {} on {}", what, adapter.get_info().name);
    let size = shot.size;
    let rate = render(&device, &queue, &stage, &env, &shot, frames, |f, rgba| {
        let path = if frames > 1 {
            let stem = out.trim_end_matches(".png");
            format!("{stem}_{f:04}.png")
        } else {
            out.clone()
        };
        if let Err(e) = save_png(&path, size, &rgba) {
            eprintln!("forge-studio: {e}");
        }
    })?;
    eprintln!("forge-studio: {:.2} M samples/s", rate / 1e6);
    Ok(())
}
