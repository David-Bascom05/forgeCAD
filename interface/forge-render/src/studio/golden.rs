//! The scene the studio's tests render, the same natively and in the browser: the crank bench
//! part in aluminium on a grey floor under the drawn studio, a fixed camera and seed.

use super::bsdf::{Bsdf, Lut};
use super::env::{self, Env};
use super::scene::{Built, Instance};
use super::v3::{V3, v3};
use super::{Settings, View};
use crate::camera::Camera;

pub struct Golden {
    pub built: Built,
    pub materials: Vec<Bsdf>,
    pub lut: Lut,
    pub env: Env,
    pub view: View,
    pub settings: Settings,
}

/// The golden picture's size and samples.
pub const SIZE: (u32, u32) = (96, 64);
pub const SPP: u32 = 128;

/// The crank built from its text and its font, the same in the browser (the page's in-memory
/// files) as natively (the bench parts' folder).
fn crank() -> Result<crate::mesh::PartMesh, String> {
    let src = crate::load::bench_src("crank").ok_or("no bench crank")?;
    #[cfg(target_arch = "wasm32")]
    let dir = {
        let d = std::path::PathBuf::from("/studio-golden");
        forge_dirs::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
        forge_dirs::fs::write(d.join(forge_lang::bench::FONT), forge_lang::bench::font_bytes())
            .map_err(|e| e.to_string())?;
        d
    };
    #[cfg(not(target_arch = "wasm32"))]
    let dir = crate::load::bench_dir()?;
    let opts = forge_lang::Options {
        dir: Some(dir),
        checks: false,
        expects: false,
        ..forge_lang::Options::default()
    };
    let (run, _) = forge_lang::run(src, opts, Default::default());
    if let Some(e) = run.errors.first() {
        return Err(format!("crank: {}: {}", e.kind(), e.text()));
    }
    let body = run.body.ok_or("crank: nothing built")?;
    crate::tess::build_part(&body).map_err(|e| format!("crank: {e:?}"))
}

pub fn scene() -> Result<Golden, String> {
    let crank = crank()?;
    let rows = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]];
    let built = Built::new(&[crank], &[Instance { part: 0, rows, material: 0 }]);
    let b = built.bounds.ok_or("the crank has no triangles")?;
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
    let env = Env::new(w, h, env::studio(w, h, 1.0));
    let mut cam = Camera::fit(centre.arr(), radius);
    cam.dist *= 0.6;
    let settings = Settings {
        ground: Some((b.min.z, 1)),
        env_turn: 0.3,
        bounces: 6,
        ..Settings::default()
    };
    Ok(Golden {
        built,
        materials,
        lut: Lut::new(256),
        env,
        view: View::of(&cam),
        settings,
    })
}

/// How far a picture may stray from the golden one and still be it, on another GPU: the RMSE and
/// the 99th percentile of the per-channel difference, in 8-bit levels.
pub fn compare(got: &[u8], want: &[u8]) -> (f64, i32) {
    let mut d: Vec<i32> = got
        .iter()
        .zip(want)
        .enumerate()
        .filter(|(k, _)| k % 4 != 3)
        .map(|(_, (a, b))| (*a as i32 - *b as i32).abs())
        .collect();
    let rmse = (d.iter().map(|x| (x * x) as f64).sum::<f64>() / d.len().max(1) as f64).sqrt();
    d.sort_unstable();
    (rmse, d.get(d.len() * 99 / 100).copied().unwrap_or(0))
}

pub const MAX_RMSE: f64 = 6.0;
pub const MAX_P99: i32 = 40;

/// The golden picture in the browser: a promise of its RGBA bytes (tone-mapped on the CPU from
/// the GPU's sum, as `cpu::to_rgba8`).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn studio_golden() -> js_sys::Promise {
    use wasm_bindgen::JsValue;
    wasm_bindgen_futures::future_to_promise(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let (device, queue, _) = crate::render::open_device(&instance, None)
            .await
            .map_err(|e| JsValue::from_str(&e))?;
        let g = scene().map_err(|e| JsValue::from_str(&e))?;
        let mut t = super::gpu::Tracer::new(&device, &queue, SIZE);
        t.set_scene(&g.built, &g.materials, &g.lut).map_err(|e| JsValue::from_str(&e))?;
        t.set_env(&g.env);
        t.step(&g.view, &g.settings, SPP);
        let floats = t.read_mean_async().await.map_err(|e| JsValue::from_str(&e))?;
        let rgba = super::cpu::to_rgba8(&floats, 0.0);
        Ok(js_sys::Uint8Array::from(&rgba[..]).into())
    })
}
