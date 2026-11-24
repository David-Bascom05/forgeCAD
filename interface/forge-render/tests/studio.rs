//! The Render Studio's tracer on the GPU against its CPU mirror, and against a golden picture.
//! A machine with no GPU (and no software Vulkan) has nothing to test here and says so.

use forge_render::camera::Camera;
use forge_render::render;
use forge_render::studio::bsdf::{Bsdf, Lut};
use forge_render::studio::env::{self, Env};
use forge_render::studio::gpu::Tracer;
use forge_render::studio::scene::{Built, Instance};
use forge_render::studio::v3::{V3, v3};
use forge_render::studio::{Settings, View, cpu};

fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = render::native_instance();
    match render::block_on(render::open_device(&instance, None)) {
        Ok((d, q, a)) => {
            eprintln!("on {:?}", a.get_info().name);
            Some((d, q))
        }
        Err(e) => {
            eprintln!("skipped: {e}");
            None
        }
    }
}

struct Fixture {
    built: Built,
    materials: Vec<Bsdf>,
    lut: Lut,
    env: Env,
    view: View,
    settings: Settings,
}

fn fixture() -> Fixture {
    let crank = forge_render::load::open("crank", false).unwrap().0;
    let rows = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]];
    let built = Built::new(&[crank.clone()], &[Instance { part: 0, rows, material: 0 }]);
    let b = built.bounds.unwrap();
    let centre = (b.min + b.max) * 0.5;
    let radius = (b.max - b.min).len() * 0.5;
    let materials = vec![
        Bsdf {
            base: v3(0.913, 0.922, 0.924),
            f82: v3(0.94, 0.94, 0.95),
            metal: 1.0,
            rough: 0.3,
            ..Bsdf::default()
        },
        Bsdf {
            base: V3::splat(0.6),
            rough: 0.7,
            ..Bsdf::default()
        },
    ];
    let (w, h) = (128, 64);
    let mut px = env::studio(w, h, 1.0);
    if std::env::var("STUDIO_FLAT").is_ok() {
        px = vec![[1.0; 3]; w * h];
    }
    let env = Env::new(w, h, px);
    let mut cam = Camera::fit(centre.arr(), radius);
    cam.dist *= 0.6;
    let view = View::of(&cam);
    let settings = Settings {
        ground: Some((b.min.z, 1)),
        env_turn: 0.3,
        bounces: std::env::var("STUDIO_BOUNCES").ok().and_then(|b| b.parse().ok()).unwrap_or(6),
        ..Settings::default()
    };
    Fixture {
        built,
        materials,
        lut: Lut::new(256),
        env,
        view,
        settings,
    }
}

/// Means over 8x8 blocks: what two pictures of different noise should agree on.
fn blocks(mean: &[[f32; 4]], size: (u32, u32)) -> Vec<f32> {
    let (w, h) = size;
    let mut out = Vec::new();
    for by in 0..h / 8 {
        for bx in 0..w / 8 {
            let mut s = 0.0;
            for y in 0..8 {
                for x in 0..8 {
                    let p = mean[((by * 8 + y) * w + bx * 8 + x) as usize];
                    s += 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2];
                }
            }
            out.push(s / 64.0);
        }
    }
    out
}

pub fn save(path: &str, size: (u32, u32), rgba: &[u8]) {
    let file = std::fs::File::create(path).unwrap();
    let mut e = png::Encoder::new(std::io::BufWriter::new(file), size.0, size.1);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header().unwrap().write_image_data(rgba).unwrap();
}

#[test]
fn the_gpu_traces_what_the_cpu_traces() {
    let Some((device, queue)) = device() else {
        return;
    };
    let f = fixture();
    let size = (64, 48);
    let mut t = Tracer::new(&device, &queue, size);
    t.set_scene(&f.built, &f.materials, &f.lut).unwrap();
    t.set_env(&f.env);
    let spp = 256;
    t.step(&f.view, &f.settings, spp);
    t.wait();
    let gpu = t.read_mean();
    let scene = cpu::Scene {
        built: &f.built,
        materials: &f.materials,
        lut: &f.lut,
        env: &f.env,
    };
    let cpu = cpu::render(&scene, &f.view, &f.settings, size, spp);
    if let Ok(dir) = std::env::var("STUDIO_DUMP") {
        for (name, m) in [("gpu", &gpu), ("cpu", &cpu)] {
            save(&format!("{dir}/{name}.png"), size, &cpu::to_rgba8(m, 0.0));
        }
    }
    let (a, b) = (blocks(&gpu, size), blocks(&cpu, size));
    eprintln!("gpu {:?}\ncpu {:?}", &a[..8], &b[..8]);
    let mut worst = 0.0f32;
    for (k, (x, y)) in a.iter().zip(&b).enumerate() {
        let rel = (x - y).abs() / (0.5 * (x + y)).max(0.02);
        worst = worst.max(rel);
        assert!(rel < 0.08, "block {k}: gpu {x} cpu {y}");
    }
    eprintln!("worst block difference {:.2} %", worst * 100.0);
    let lum = |v: &[f32]| v.iter().sum::<f32>() / v.len() as f32;
    assert!(lum(&a) > 0.05, "a picture, not black: {}", lum(&a));
}

#[test]
fn a_fixed_seed_gives_the_same_picture() {
    let Some((device, queue)) = device() else {
        return;
    };
    let f = fixture();
    let size = (48, 32);
    let run = || {
        let mut t = Tracer::new(&device, &queue, size);
        t.set_scene(&f.built, &f.materials, &f.lut).unwrap();
        t.set_env(&f.env);
        t.step(&f.view, &f.settings, 8);
        t.show(0.0, false);
        t.read_rgba()
    };
    assert_eq!(run(), run());
}

#[test]
fn the_denoiser_brings_a_few_samples_closer_to_many() {
    let Some((device, queue)) = device() else {
        return;
    };
    let f = fixture();
    let size = (96, 64);
    let mut t = Tracer::new(&device, &queue, size);
    t.set_scene(&f.built, &f.materials, &f.lut).unwrap();
    t.set_env(&f.env);
    let settings = Settings { clamp: 16.0, ..f.settings };
    t.step(&f.view, &settings, 1024);
    t.wait();
    let truth = t.read_mean();
    let mut t = Tracer::new(&device, &queue, size);
    t.set_scene(&f.built, &f.materials, &f.lut).unwrap();
    t.set_env(&f.env);
    let few = Settings { seed: 7, ..settings };
    t.step(&f.view, &few, 16);
    t.show(0.0, true);
    t.wait();
    let raw = t.read_mean();
    let clean = t.read_denoised();
    // compared as shown: tone-mapped, so a bright outlier counts as it looks
    let shown = |v: &[[f32; 4]]| cpu::to_rgba8(v, 0.0);
    let (truth, raw, clean) = (shown(&truth), shown(&raw), shown(&clean));
    let rmse = |a: &[u8]| {
        let s: f64 = a.iter().zip(&truth).enumerate().filter(|(k, _)| k % 4 != 3).map(|(_, (x, y))| (*x as f64 - *y as f64).powi(2)).sum();
        (s / (a.len() as f64 * 0.75)).sqrt()
    };
    let (r, c) = (rmse(&raw), rmse(&clean));
    eprintln!("16 spp against 1024: raw {r:.2}, denoised {c:.2} (8-bit levels)");
    assert!(c < 0.6 * r, "denoised {c} vs raw {r}");
}

#[test]
fn the_queue_writes_a_turntable_frame_by_frame() {
    use forge_render::studio::live::{Job, Output, Queue};
    let Some((device, queue)) = device() else {
        return;
    };
    let f = fixture();
    let mut q = Queue::default();
    let b = f.built.bounds.unwrap();
    q.push(Job {
        output: Output::Turntable { stem: "t.png".into(), frames: 3 },
        size: (32, 24),
        spp: 12,
        denoise: true,
        view: f.view,
        settings: f.settings,
        pivot: ((b.min + b.max) * 0.5).arr(),
    });
    let mut written = Vec::new();
    for _ in 0..20 {
        let scene = || Some((f.built.clone(), f.materials.clone(), Lut::new(64)));
        q.tick(&device, &queue, scene, &f.env, |path, size, rgba| {
            assert_eq!(rgba.len() as u32, size.0 * size.1 * 4);
            written.push((path.to_string(), rgba));
            Ok(())
        });
    }
    let names: Vec<&str> = written.iter().map(|w| w.0.as_str()).collect();
    assert_eq!(names, ["t_0000.png", "t_0001.png", "t_0002.png"]);
    // the part turned: the frames differ
    assert_ne!(written[0].1, written[1].1);
    assert!(q.jobs.is_empty() && q.log.iter().all(|l| l.is_ok()));
}

/// The golden scene's picture, as the GPU shows it: 96x64, 128 samples, seed 0, not denoised.
fn golden_picture(device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
    let f = fixture();
    let size = (96, 64);
    let mut t = Tracer::new(device, queue, size);
    t.set_scene(&f.built, &f.materials, &f.lut).unwrap();
    t.set_env(&f.env);
    t.step(&f.view, &f.settings, 128);
    t.show(0.0, false);
    t.read_rgba()
}

const GOLDEN: &str = "tests/fixtures/studio_golden.png";

fn read_png(path: &str) -> ((u32, u32), Vec<u8>) {
    let d = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
    let mut r = d.read_info().unwrap();
    let mut buf = vec![0; r.output_buffer_size().unwrap()];
    let info = r.next_frame(&mut buf).unwrap();
    ((info.width, info.height), buf[..(info.width * info.height * 4) as usize].to_vec())
}

#[test]
fn the_golden_scene_looks_as_it_did() {
    let Some((device, queue)) = device() else {
        return;
    };
    let got = golden_picture(&device, &queue);
    if std::env::var("STUDIO_BLESS").is_ok() {
        save(GOLDEN, (96, 64), &got);
        return;
    }
    let (size, want) = read_png(GOLDEN);
    assert_eq!(size, (96, 64));
    // another GPU rounds differently and its noise lands elsewhere: the picture must agree on
    // the whole (RMSE) and nearly everywhere (99th percentile of the per-channel difference)
    let mut d: Vec<i32> = got.iter().zip(&want).enumerate().filter(|(k, _)| k % 4 != 3).map(|(_, (a, b))| (*a as i32 - *b as i32).abs()).collect();
    let rmse = (d.iter().map(|x| (x * x) as f64).sum::<f64>() / d.len() as f64).sqrt();
    d.sort();
    let p99 = d[d.len() * 99 / 100];
    eprintln!("golden: rmse {rmse:.2}, p99 {p99} (8-bit levels)");
    if rmse > 1e-9 {
        save("target/studio_golden_got.png", (96, 64), &got);
    }
    assert!(rmse < 6.0 && p99 < 40, "rmse {rmse}, p99 {p99}: see target/studio_golden_got.png");
}
