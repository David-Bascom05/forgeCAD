//! The renderer: all meshes in one vertex, index and segment buffer, drawn instanced, part by
//! part and level by level, from the frame's list of visible placements.

use crate::camera::Camera;
use crate::mesh::{PartMesh, SEGMENT, VERTEX};
use crate::scene::{Placement, Scene};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub const SAMPLES: u32 = 4;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// A frame keeps its GPU time this many frames at most before it is read.
const TIMERS: usize = 4;
const HUD_CHARS: usize = 256;
const GLOBALS: usize = 64 + 16 + 16 + HUD_CHARS;
const PLACEMENT: u64 = 64;
const BACKGROUND: wgpu::Color = wgpu::Color {
    r: 0.72,
    g: 0.75,
    b: 0.79,
    a: 1.0,
};

#[derive(Clone, Copy)]
struct LodDraw {
    first_index: u32,
    indices: u32,
    base_vertex: i32,
    first_segment: u32,
    segments: u32,
}

/// What a frame drew.
#[derive(Clone, Copy, Debug, Default)]
pub struct Drawn {
    pub placements: u32,
    pub tris: u64,
    pub segments: u64,
    pub draws: u32,
}

struct Timer {
    read: wgpu::Buffer,
    ready: Arc<AtomicBool>,
    busy: bool,
}

pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter: wgpu::AdapterInfo,
    format: wgpu::TextureFormat,
    surface: wgpu::RenderPipeline,
    line: wgpu::RenderPipeline,
    hud: wgpu::RenderPipeline,
    binds: wgpu::BindGroup,
    globals: wgpu::Buffer,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    instances: wgpu::Buffer,
    lods: Vec<Vec<LodDraw>>,
    size: (u32, u32),
    colour: wgpu::TextureView,
    depth: wgpu::TextureView,
    timer_set: Option<wgpu::QuerySet>,
    timer_resolve: wgpu::Buffer,
    timers: Vec<Timer>,
    timer_at: usize,
    period_ns: f32,
    /// The last GPU time read, in ms.
    pub gpu_ms: Option<f32>,
    /// Coarsest level whose chord stays under this many pixels.
    pub lod_px: f32,
    pub lines: bool,
}

/// What every part's chords and sphere are, for `Scene::new`.
pub fn part_info(parts: &[PartMesh]) -> Vec<([f32; 3], f32, Vec<f32>)> {
    parts
        .iter()
        .map(|p| (p.center, p.radius, p.lods.iter().map(|l| l.chord).collect()))
        .collect()
}

fn filled(
    device: &wgpu::Device,
    label: &str,
    size: u64,
    usage: wgpu::BufferUsages,
    data: Vec<&[u8]>,
) -> wgpu::Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: size.max(16),
        usage,
        mapped_at_creation: true,
    });
    let mut map = buffer.slice(..).get_mapped_range_mut().expect("just mapped");
    let mut at = 0;
    for d in data {
        map.slice(at..at + d.len()).copy_from_slice(d);
        at += d.len();
    }
    drop(map);
    buffer.unmap();
    buffer
}

/// wgpu's native futures finish without waiting on anything.
#[cfg(not(target_arch = "wasm32"))]
pub fn block_on<F: Future>(f: F) -> F::Output {
    let mut f = std::pin::pin!(f);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        if let std::task::Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}

/// The native GPU instance: Vulkan only.
#[cfg(not(target_arch = "wasm32"))]
pub fn native_instance() -> wgpu::Instance {
    wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    })
}

/// The adapter and device, the fast GPU first. `surface` is what it must draw to, if a window.
pub async fn open_device(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
) -> Result<(wgpu::Device, wgpu::Queue, wgpu::Adapter), String> {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: surface,
            apply_limit_buckets: false,
        })
        .await
        .map_err(|e| format!("no GPU: {e}"))?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: adapter.features() & wgpu::Features::TIMESTAMP_QUERY,
            required_limits: adapter.limits(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .await
        .map_err(|e| format!("no device: {e}"))?;
    Ok((device, queue, adapter))
}

impl Renderer {
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        adapter: wgpu::AdapterInfo,
        format: wgpu::TextureFormat,
        size: (u32, u32),
        parts: &[PartMesh],
        scene: &Scene,
    ) -> Renderer {
        let shader = device.create_shader_module(wgpu::include_wgsl!("render.wgsl"));
        let layout_entry = |binding, visibility, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let all = wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT;
        let read = wgpu::BufferBindingType::Storage { read_only: true };
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                layout_entry(0, all, wgpu::BufferBindingType::Uniform),
                layout_entry(1, wgpu::ShaderStages::VERTEX, read),
                layout_entry(2, wgpu::ShaderStages::VERTEX, read),
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });

        let instance_at = |location| wgpu::VertexBufferLayout {
            array_stride: 4,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: match location {
                2 => &wgpu::vertex_attr_array![2 => Uint32],
                _ => &wgpu::vertex_attr_array![0 => Uint32],
            },
        };
        let target = [Some(wgpu::ColorTargetState {
            format,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let multisample = wgpu::MultisampleState {
            count: SAMPLES,
            ..Default::default()
        };
        let depth = |write| {
            Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: Some(write),
                depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })
        };
        let surface = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("surface"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("surface_vs"),
                compilation_options: Default::default(),
                buffers: &[
                    Some(wgpu::VertexBufferLayout {
                        array_stride: VERTEX as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Snorm16x2],
                    }),
                    Some(instance_at(2)),
                ],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: depth(true),
            multisample,
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("surface_fs"),
                compilation_options: Default::default(),
                targets: &target,
            }),
            multiview_mask: None,
            cache: None,
        });
        let line = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("line"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("line_vs"),
                compilation_options: Default::default(),
                buffers: &[Some(instance_at(0))],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: depth(false),
            multisample,
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("line_fs"),
                compilation_options: Default::default(),
                targets: &target,
            }),
            multiview_mask: None,
            cache: None,
        });
        let hud = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hud"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("hud_vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("hud_fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let (mut vbytes, mut ibytes, mut sbytes) = (0u64, 0u64, 0u64);
        let mut lods = Vec::with_capacity(parts.len());
        for p in parts {
            let mut row = Vec::with_capacity(p.lods.len());
            for l in &p.lods {
                row.push(LodDraw {
                    first_index: (ibytes / 4) as u32,
                    indices: (l.indices.len() / 4) as u32,
                    base_vertex: (vbytes / VERTEX as u64) as i32,
                    first_segment: (sbytes / SEGMENT as u64) as u32,
                    segments: (l.segments.len() / SEGMENT) as u32,
                });
                vbytes += l.vertices.len() as u64;
                ibytes += l.indices.len() as u64;
                sbytes += l.segments.len() as u64;
            }
            lods.push(row);
        }
        let all_lods = || parts.iter().flat_map(|p| &p.lods);
        let v: Vec<&[u8]> = all_lods().map(|l| &l.vertices[..]).collect();
        let i: Vec<&[u8]> = all_lods().map(|l| &l.indices[..]).collect();
        let s: Vec<&[u8]> = all_lods().map(|l| &l.segments[..]).collect();
        let vertices = filled(&device, "vertices", vbytes, wgpu::BufferUsages::VERTEX, v);
        let indices = filled(&device, "indices", ibytes, wgpu::BufferUsages::INDEX, i);
        let segments = filled(&device, "segments", sbytes, wgpu::BufferUsages::STORAGE, s);

        let mut placed = Vec::with_capacity(scene.placements.len() * PLACEMENT as usize);
        for p in &scene.placements {
            write_placement(&mut placed, p);
        }
        let placements = filled(
            &device,
            "placements",
            placed.len() as u64,
            wgpu::BufferUsages::STORAGE,
            vec![&placed],
        );
        let instances = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("visible"),
            size: scene.placements.len().max(1) as u64 * 4,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: GLOBALS as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let binds = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: globals.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: placements.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: segments.as_entire_binding(),
                },
            ],
        });

        let has_timer = device.features().contains(wgpu::Features::TIMESTAMP_QUERY);
        let timer_set = has_timer.then(|| {
            device.create_query_set(&wgpu::QuerySetDescriptor {
                label: None,
                ty: wgpu::QueryType::Timestamp,
                count: 2,
            })
        });
        let timer_resolve = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 16,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let timers = (0..if has_timer { TIMERS } else { 0 })
            .map(|_| Timer {
                read: device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size: 16,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                ready: Arc::new(AtomicBool::new(false)),
                busy: false,
            })
            .collect();
        let period_ns = queue.get_timestamp_period();
        let (colour, depth_view) = targets(&device, format, size);
        Renderer {
            device,
            queue,
            adapter,
            format,
            surface,
            line,
            hud,
            binds,
            globals,
            vertices,
            indices,
            instances,
            lods,
            size,
            colour,
            depth: depth_view,
            timer_set,
            timer_resolve,
            timers,
            timer_at: 0,
            period_ns,
            gpu_ms: None,
            lod_px: LOD_PX,
            lines: true,
        }
    }

    pub fn resize(&mut self, size: (u32, u32)) {
        if size != self.size && size.0 > 0 && size.1 > 0 {
            self.size = size;
            (self.colour, self.depth) = targets(&self.device, self.format, size);
        }
    }

    /// Waits for the GPU to finish what it was given.
    pub fn wait(&self) {
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the GPU answers");
    }

    /// Reads back any GPU times the GPU has finished.
    fn read_timers(&mut self) {
        for t in &mut self.timers {
            if t.busy && t.ready.swap(false, Ordering::AcqRel) {
                let map = t.read.slice(..).get_mapped_range().expect("it said ready");
                let stamp = |k: usize| u64::from_le_bytes(map[k..k + 8].try_into().expect("8 bytes"));
                let ns = stamp(8).saturating_sub(stamp(0)) as f32 * self.period_ns;
                self.gpu_ms = Some(ns * 1e-6);
                drop(map);
                t.read.unmap();
                t.busy = false;
            }
        }
    }

    /// Culls, draws into `target` and submits, the numbers overlay on top if given.
    pub fn frame(
        &mut self,
        target: &wgpu::TextureView,
        scene: &mut Scene,
        camera: &Camera,
        hud: Option<&str>,
    ) -> Drawn {
        let view = camera.view(self.size.0, self.size.1);
        scene.cull(&view, self.lod_px);
        self.queue
            .write_buffer(&self.instances, 0, scene.visible.as_flattened());

        let mut bytes = [0u8; GLOBALS];
        for (k, v) in view.matrix.iter().enumerate() {
            bytes[k * 4..k * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        let extra = [
            view.eye[0],
            view.eye[1],
            view.eye[2],
            0.0,
            2.0 / self.size.0 as f32,
            2.0 / self.size.1 as f32,
            LINE_HALF_PX,
            0.0,
        ];
        for (k, v) in extra.iter().enumerate() {
            bytes[64 + k * 4..68 + k * 4].copy_from_slice(&v.to_le_bytes());
        }
        let text = &mut bytes[96..];
        text.fill(255);
        for (c, b) in text.iter_mut().zip(hud.unwrap_or("").bytes()) {
            *c = glyph(b);
        }
        self.queue.write_buffer(&self.globals, 0, &bytes);

        self.read_timers();
        let slot = self.timer_at;
        let timed = self.timers.get(slot).is_some_and(|t| !t.busy);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let mut drawn = Drawn {
            placements: scene.visible.len() as u32,
            ..Drawn::default()
        };
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.colour,
                    depth_slice: None,
                    resolve_target: Some(target),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(BACKGROUND),
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: self.timer_set.as_ref().filter(|_| timed).map(|set| {
                    wgpu::RenderPassTimestampWrites {
                        query_set: set,
                        beginning_of_pass_write_index: Some(0),
                        end_of_pass_write_index: Some(1),
                    }
                }),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.binds, &[]);
            pass.set_pipeline(&self.surface);
            pass.set_vertex_buffer(0, self.vertices.slice(..));
            pass.set_vertex_buffer(1, self.instances.slice(..));
            pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
            for d in &scene.draws {
                let l = self.lods[d.part as usize][d.lod as usize];
                pass.draw_indexed(
                    l.first_index..l.first_index + l.indices,
                    l.base_vertex,
                    d.first..d.first + d.count,
                );
                drawn.tris += l.indices as u64 / 3 * d.count as u64;
                drawn.draws += 1;
            }
            if self.lines {
                pass.set_pipeline(&self.line);
                pass.set_vertex_buffer(0, self.instances.slice(..));
                for d in &scene.draws {
                    let l = self.lods[d.part as usize][d.lod as usize];
                    if l.segments > 0 {
                        pass.draw(
                            l.first_segment * 6..(l.first_segment + l.segments) * 6,
                            d.first..d.first + d.count,
                        );
                        drawn.segments += l.segments as u64 * d.count as u64;
                        drawn.draws += 1;
                    }
                }
            }
        }
        if hud.is_some() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("hud"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.binds, &[]);
            pass.set_pipeline(&self.hud);
            pass.draw(0..3, 0..1);
        }
        if timed && let Some(set) = &self.timer_set {
            encoder.resolve_query_set(set, 0..2, &self.timer_resolve, 0);
            encoder.copy_buffer_to_buffer(&self.timer_resolve, 0, &self.timers[slot].read, 0, 16);
        }
        self.queue.submit([encoder.finish()]);
        if timed {
            let t = &mut self.timers[slot];
            t.busy = true;
            let ready = t.ready.clone();
            t.read.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                if r.is_ok() {
                    ready.store(true, Ordering::Release);
                }
            });
            self.timer_at = (slot + 1) % TIMERS;
        }
        drawn
    }
}

/// Somewhere to draw with no window.
pub struct Offscreen {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    size: (u32, u32),
}

pub const OFFSCREEN: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

impl Offscreen {
    pub fn new(device: &wgpu::Device, size: (u32, u32)) -> Offscreen {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OFFSCREEN,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        Offscreen { texture, view, size }
    }

    /// The picture, RGB, row by row from the top.
    pub fn read_rgb(&self, r: &Renderer) -> Vec<u8> {
        let (w, h) = self.size;
        let row = (w * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = r.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (row * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = r.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            self.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: None,
                },
            },
            self.texture.size(),
        );
        r.queue.submit([encoder.finish()]);
        buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        r.wait();
        let map = buffer.slice(..).get_mapped_range().expect("it was waited for");
        let mut out = Vec::with_capacity((w * h * 3) as usize);
        for y in 0..h as usize {
            for px in map[y * row as usize..][..w as usize * 4].as_chunks::<4>().0 {
                out.extend_from_slice(&px[..3]);
            }
        }
        out
    }
}

/// Half a line's width in pixels.
const LINE_HALF_PX: f32 = 0.7;
/// A level's chord may be this many pixels on screen (research/render).
pub const LOD_PX: f32 = 1.0;

fn targets(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    size: (u32, u32),
) -> (wgpu::TextureView, wgpu::TextureView) {
    let make = |format, label| {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: SAMPLES,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&Default::default())
    };
    (make(format, "colour"), make(DEPTH, "depth"))
}

fn write_placement(out: &mut Vec<u8>, p: &Placement) {
    for v in p.rows.iter().flatten().chain(&p.tint) {
        out.extend_from_slice(&v.to_le_bytes());
    }
}

/// The overlay's glyph number of a byte (see GLYPHS in render.wgsl).
fn glyph(b: u8) -> u8 {
    match b.to_ascii_uppercase() {
        c @ b'0'..=b'9' => c - b'0',
        c @ b'A'..=b'Z' => c - b'A' + 10,
        b'.' => 36,
        b'/' => 38,
        b':' => 39,
        _ => 37,
    }
}
