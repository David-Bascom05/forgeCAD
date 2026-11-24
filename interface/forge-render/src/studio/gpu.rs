//! The path tracer on the GPU: its buffers, the trace and show passes, and the picture back.
//! The same on Vulkan, Metal, DX12 and WebGPU: compute shaders, storage buffers within WebGPU's
//! default limits (seven per stage), no extensions.

use super::bsdf::{Bsdf, Lut};
use super::env::Env;
use super::scene::Built;
use super::{Backdrop, Settings, View};
use wgpu::util::DeviceExt;

const TRACE: &str = include_str!("trace.wgsl");
const SHOW: &str = include_str!("show.wgsl");

/// WebGPU's guaranteed largest storage binding: a scene past it is refused, not truncated.
pub const MAX_BINDING: u64 = 128 << 20;

pub struct Tracer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    trace: wgpu::ComputePipeline,
    show: wgpu::ComputePipeline,
    params: wgpu::Buffer,
    show_params: wgpu::Buffer,
    nodes: wgpu::Buffer,
    tris: wgpu::Buffer,
    instances: wgpu::Buffer,
    materials: wgpu::Buffer,
    env: wgpu::Buffer,
    accum: wgpu::Buffer,
    aov: wgpu::Buffer,
    pub picture: wgpu::Texture,
    pub picture_view: wgpu::TextureView,
    trace_group: Option<wgpu::BindGroup>,
    show_group: Option<wgpu::BindGroup>,
    pub size: (u32, u32),
    /// Samples summed so far.
    pub samples: u32,
    top: u32,
    placements: u32,
    lut_at: u32,
    env_size: (u32, u32),
}

fn storage(device: &wgpu::Device, label: &str, words: &[u8], write: bool) -> wgpu::Buffer {
    let mut usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
    if write {
        usage |= wgpu::BufferUsages::COPY_SRC;
    }
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: words,
        usage,
    })
}

fn bytes<T: Copy>(v: &[T]) -> Vec<u8>
where
    T: ToBytes,
{
    v.iter().flat_map(|x| x.le()).collect()
}

trait ToBytes {
    fn le(&self) -> [u8; 4];
}
impl ToBytes for u32 {
    fn le(&self) -> [u8; 4] {
        self.to_le_bytes()
    }
}
impl ToBytes for f32 {
    fn le(&self) -> [u8; 4] {
        self.to_le_bytes()
    }
}

fn picture(device: &wgpu::Device, size: (u32, u32)) -> (wgpu::Texture, wgpu::TextureView) {
    let t = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("studio picture"),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let v = t.create_view(&Default::default());
    (t, v)
}

/// The materials' records (4 vec4s each) and the tables after them; where the tables start.
pub fn material_words(materials: &[Bsdf], lut: &Lut) -> (Vec<f32>, u32) {
    let mut v = Vec::new();
    for m in materials {
        v.extend([m.base.x, m.base.y, m.base.z, m.metal]);
        v.extend([m.f82.x, m.f82.y, m.f82.z, m.rough]);
        v.extend([m.ior, m.transmission, m.coat, m.coat_rough]);
        v.extend([0.0; 4]);
    }
    let at = (v.len() / 4) as u32;
    v.extend(lut.flat());
    while v.len() % 4 != 0 {
        v.push(0.0);
    }
    (v, at)
}

impl Tracer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, size: (u32, u32)) -> Tracer {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("studio trace"),
            source: wgpu::ShaderSource::Wgsl(TRACE.into()),
        });
        let show_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("studio show"),
            source: wgpu::ShaderSource::Wgsl(SHOW.into()),
        });
        let pipeline = |m: &wgpu::ShaderModule, entry: &str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: None,
                module: m,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let trace = pipeline(&module, "trace_main");
        let show = pipeline(&show_module, "show_main");
        let uniform = |label: &str, n: u64| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: n,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let empty = |label: &str| storage(device, label, &[0u8; 64], false);
        let (picture, picture_view) = picture(device, size);
        let px = (size.0 * size.1) as usize;
        Tracer {
            device: device.clone(),
            queue: queue.clone(),
            trace,
            show,
            params: uniform("studio params", 160),
            show_params: uniform("studio show params", 16),
            nodes: empty("studio nodes"),
            tris: empty("studio tris"),
            instances: empty("studio instances"),
            materials: empty("studio materials"),
            env: empty("studio env"),
            accum: storage(device, "studio accum", &vec![0u8; px * 16], true),
            aov: storage(device, "studio aov", &vec![0u8; px * 32], true),
            picture,
            picture_view,
            trace_group: None,
            show_group: None,
            size,
            samples: 0,
            top: 0,
            placements: 0,
            lut_at: 0,
            env_size: (1, 1),
        }
    }

    /// A new scene: its hierarchy and the appearances its placements name. Refused when a
    /// buffer would pass WebGPU's binding limit.
    pub fn set_scene(&mut self, built: &Built, materials: &[Bsdf], lut: &Lut) -> Result<(), String> {
        let (nodes, tris, inst) = built.words();
        for (what, n) in [("nodes", nodes.len()), ("triangles", tris.len()), ("placements", inst.len())] {
            if (n * 4) as u64 > MAX_BINDING {
                return Err(format!("the scene's {what} need {} MB, past the GPU's 128 MB", n * 4 >> 20));
            }
        }
        let (mats, at) = material_words(materials, lut);
        self.nodes = storage(&self.device, "studio nodes", &bytes(&nodes), false);
        self.tris = storage(&self.device, "studio tris", &bytes(&tris), false);
        self.instances = storage(&self.device, "studio instances", &bytes(&inst), false);
        self.materials = storage(&self.device, "studio materials", &bytes(&mats), false);
        self.top = built.top;
        self.placements = built.instances.len() as u32;
        self.lut_at = at;
        self.trace_group = None;
        self.reset();
        Ok(())
    }

    /// Only the appearances changed: the same hierarchy.
    pub fn set_materials(&mut self, materials: &[Bsdf], lut: &Lut) {
        let (mats, at) = material_words(materials, lut);
        self.materials = storage(&self.device, "studio materials", &bytes(&mats), false);
        self.lut_at = at;
        self.trace_group = None;
        self.reset();
    }

    pub fn set_env(&mut self, env: &Env) {
        self.env = storage(&self.device, "studio env", &bytes(&env.words()), false);
        self.env_size = (env.w as u32, env.h as u32);
        self.trace_group = None;
        self.reset();
    }

    pub fn resize(&mut self, size: (u32, u32)) {
        if size == self.size {
            return;
        }
        let px = (size.0 * size.1) as usize;
        self.accum = storage(&self.device, "studio accum", &vec![0u8; px * 16], true);
        self.aov = storage(&self.device, "studio aov", &vec![0u8; px * 32], true);
        (self.picture, self.picture_view) = picture(&self.device, size);
        self.size = size;
        self.samples = 0;
        self.trace_group = None;
        self.show_group = None;
    }

    /// Starts the sum again: anything about the picture changed.
    pub fn reset(&mut self) {
        self.samples = 0;
        let mut e = self.device.create_command_encoder(&Default::default());
        e.clear_buffer(&self.accum, 0, None);
        e.clear_buffer(&self.aov, 0, None);
        self.queue.submit([e.finish()]);
    }

    fn group(&mut self) {
        if self.trace_group.is_none() {
            let layout = self.trace.get_bind_group_layout(0);
            let entries: Vec<wgpu::BindGroupEntry> = [
                &self.params,
                &self.nodes,
                &self.tris,
                &self.instances,
                &self.materials,
                &self.env,
                &self.accum,
                &self.aov,
            ]
            .iter()
            .enumerate()
            .map(|(k, b)| wgpu::BindGroupEntry {
                binding: k as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
            self.trace_group = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("studio trace"),
                layout: &layout,
                entries: &entries,
            }));
        }
        if self.show_group.is_none() {
            let layout = self.show.get_bind_group_layout(0);
            self.show_group = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("studio show"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.show_params.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.accum.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&self.picture_view),
                    },
                ],
            }));
        }
    }

    fn params(&self, view: &View, s: &Settings) -> Vec<u8> {
        let (w, h) = self.size;
        let aspect = w as f32 / h as f32;
        let ty = (view.fovy * 0.5).tan();
        let tx = ty * aspect;
        let sc = |v: [f32; 3], k: f32| [v[0] * k, v[1] * k, v[2] * k];
        let right = sc(view.right, tx);
        let up = sc(view.up, ty);
        let (ground_z, ground_on, ground_mat) = match s.ground {
            Some((z, m)) => (z, 1.0, m as f32),
            None => (0.0, 0.0, 0.0),
        };
        let (back_mode, back) = match s.backdrop {
            Backdrop::Environment => (0.0, [0.0; 3]),
            Backdrop::Colour(c) => (1.0, c),
            Backdrop::Transparent => (2.0, [0.0; 3]),
        };
        let mut v: Vec<u32> = vec![w, h, self.samples, s.seed];
        v.extend([self.top, self.env_size.0, self.env_size.1, s.bounces]);
        v.extend([self.lut_at, self.placements, 1, 0]);
        let f = |x: f32| x.to_bits();
        v.extend([f(view.eye[0]), f(view.eye[1]), f(view.eye[2]), f(view.parallel)]);
        v.extend([f(right[0]), f(right[1]), f(right[2]), 0]);
        v.extend([f(up[0]), f(up[1]), f(up[2]), 0]);
        v.extend([f(view.forward[0]), f(view.forward[1]), f(view.forward[2]), f(view.aperture)]);
        v.extend([f(view.focus), f(s.env_turn), f(s.env_strength), f(s.clamp)]);
        v.extend([f(ground_z), f(ground_on), f(ground_mat), f(back_mode)]);
        v.extend([f(back[0]), f(back[1]), f(back[2]), 0]);
        bytes(&v)
    }

    /// `n` more samples per pixel.
    pub fn step(&mut self, view: &View, settings: &Settings, n: u32) {
        self.group();
        let (w, h) = self.size;
        for _ in 0..n {
            self.queue.write_buffer(&self.params, 0, &self.params(view, settings));
            let mut e = self.device.create_command_encoder(&Default::default());
            {
                let mut pass = e.begin_compute_pass(&Default::default());
                pass.set_pipeline(&self.trace);
                pass.set_bind_group(0, self.trace_group.as_ref(), &[]);
                pass.dispatch_workgroups(w.div_ceil(8), h.div_ceil(8), 1);
            }
            self.queue.submit([e.finish()]);
            self.samples += 1;
        }
    }

    /// The sum so far into `picture`, exposed by `ev` stops.
    pub fn show(&mut self, ev: f32) {
        self.group();
        let v = [self.samples as f32, 2f32.powf(ev), 0.0, 0.0];
        self.queue.write_buffer(&self.show_params, 0, &bytes(&v));
        let (w, h) = self.size;
        let mut e = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = e.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.show);
            pass.set_bind_group(0, self.show_group.as_ref(), &[]);
            pass.dispatch_workgroups(w.div_ceil(8), h.div_ceil(8), 1);
        }
        self.queue.submit([e.finish()]);
    }

    pub fn wait(&self) {
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the GPU answers");
    }

    fn read_buffer(&self, src: &wgpu::Buffer) -> Vec<f32> {
        let size = src.size();
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut e = self.device.create_command_encoder(&Default::default());
        e.copy_buffer_to_buffer(src, 0, &staging, 0, size);
        self.queue.submit([e.finish()]);
        staging.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        self.wait();
        let map = staging.slice(..).get_mapped_range().expect("it was waited for");
        map.as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes(*b)).collect()
    }

    /// The running mean, linear RGBA per pixel (native only: it waits).
    pub fn read_mean(&self) -> Vec<[f32; 4]> {
        let n = self.samples.max(1) as f32;
        self.read_buffer(&self.accum)
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| p.map(|v| v / n))
            .collect()
    }

    /// First-hit albedo + depth, normal + coverage, per pixel, averaged.
    pub fn read_aov(&self) -> Vec<[f32; 8]> {
        let n = self.samples.max(1) as f32;
        self.read_buffer(&self.aov)
            .as_chunks::<8>()
            .0
            .iter()
            .map(|p| p.map(|v| v / n))
            .collect()
    }

    /// The shown picture as RGBA8, rows from the top (native only: it waits).
    pub fn read_rgba(&self) -> Vec<u8> {
        let (w, h) = self.size;
        let row = (w * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (row * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut e = self.device.create_command_encoder(&Default::default());
        e.copy_texture_to_buffer(
            self.picture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: None,
                },
            },
            self.picture.size(),
        );
        self.queue.submit([e.finish()]);
        buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        self.wait();
        let map = buffer.slice(..).get_mapped_range().expect("it was waited for");
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h as usize {
            out.extend_from_slice(&map[y * row as usize..][..w as usize * 4]);
        }
        out
    }
}
