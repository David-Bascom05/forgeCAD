//! The renderer: all meshes in one vertex, index and segment buffer, drawn instanced, part by
//! part and level by level, from the frame's list of visible placements.

use crate::camera::{Camera, View};
use crate::mesh::{EdgeRange, FaceRange, PartMesh, SEGMENT, VERTEX};
use crate::scene::Scene;
use crate::viewer::now_ms;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub const SAMPLES: u32 = 4;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// A frame keeps its GPU time this many frames at most before it is read.
const TIMERS: usize = 4;
const HUD_CHARS: usize = 256;
/// Where the overlay's text starts in the globals (see render.wgsl).
const TEXT_AT: usize = 64 + 16 + 16 + 3 * 16 + 16 + DOTS * 16;
/// Picked vertices a frame can mark.
const DOTS: usize = 8;
/// Picked faces and edges a frame can mark.
const MARKS: usize = 32;
const SELECTED_BIT: u32 = 1 << 31;
const GLOBALS: usize = TEXT_AT + HUD_CHARS;
/// A pick draws this many pixels around the cursor.
const PICK_PX: u32 = 11;
const PICK_ID: wgpu::TextureFormat = wgpu::TextureFormat::Rg32Uint;
/// Bytes per row of a pick's read-back (WebGPU's alignment).
const PICK_ROW: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
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
    /// How many of the segments are B-rep edges.
    edges: u32,
}

/// Where a level's B-rep entities are.
struct PickIndex {
    faces: Vec<FaceRange>,
    edges: Vec<EdgeRange>,
    first_segment: u32,
}

/// What a frame drew.
#[derive(Clone, Copy, Debug, Default)]
pub struct Drawn {
    pub placements: u32,
    pub tris: u64,
    pub segments: u64,
    pub draws: u32,
    /// CPU time in the frame: culling, uploading what moved and what's visible, encoding.
    pub cull_ms: f32,
    pub upload_ms: f32,
    pub encode_ms: f32,
}

/// What's on the screen overlay of a frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct Overlay<'a> {
    pub hud: Option<&'a str>,
    /// The view cube's square (left, top, side in pixels) and its region under the cursor.
    pub cube: Option<((f32, f32), f32, [i8; 3])>,
    /// What's hovered or selected, at most `MARKS` of them.
    pub marks: &'a [Mark],
}

/// A B-rep entity of a part.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Entity {
    Face(u32),
    Edge(u32),
    Vertex(u32),
}

/// An entity of a placement drawn marked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mark {
    pub placement: u32,
    pub entity: Entity,
    pub selected: bool,
}

/// What was under a pixel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickHit {
    pub placement: u32,
    pub entity: Entity,
    /// The point on the surface.
    pub point: [f32; 3],
    /// How far ahead of the eye, along the view.
    pub depth: f32,
}

struct PickState {
    id: wgpu::Texture,
    id_view: wgpu::TextureView,
    depth: wgpu::Texture,
    depth_view: wgpu::TextureView,
    id_read: wgpu::Buffer,
    depth_read: wgpu::Buffer,
    globals: wgpu::Buffer,
    binds: wgpu::BindGroup,
    surface: wgpu::RenderPipeline,
    line: wgpu::RenderPipeline,
    ready: Arc<AtomicBool>,
    /// The pixel asked about, the camera it was asked with, when.
    busy: Option<((f32, f32), Camera, f64)>,
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
    cube: wgpu::RenderPipeline,
    face_mark: wgpu::RenderPipeline,
    edge_mark: wgpu::RenderPipeline,
    dot: wgpu::RenderPipeline,
    marks: wgpu::Buffer,
    pick: PickState,
    binds: wgpu::BindGroup,
    globals: wgpu::Buffer,
    placements: wgpu::Buffer,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    instances: wgpu::Buffer,
    lods: Vec<Vec<LodDraw>>,
    /// Each part's faces and edges, by level, for telling what a pick hit.
    index: Vec<Vec<PickIndex>>,
    vertex_faces: wgpu::Buffer,
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
                layout_entry(3, wgpu::ShaderStages::VERTEX, read),
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

        let cube = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cube"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("cube_vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            // convex and culled: no depth needed, over everything
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample,
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("cube_fs"),
                compilation_options: Default::default(),
                targets: &target,
            }),
            multiview_mask: None,
            cache: None,
        });
        let pick_surface = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pick surface"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("surface_pick_vs"),
                compilation_options: Default::default(),
                buffers: &[
                    Some(wgpu::VertexBufferLayout {
                        array_stride: VERTEX as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                    }),
                    Some(instance_at(2)),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 4,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![3 => Uint32],
                    }),
                ],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: depth(true),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("pick_fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: PICK_ID,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let pick_target_state = [Some(wgpu::ColorTargetState {
            format: PICK_ID,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let pick_line = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pick line"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("line_pick_vs"),
                compilation_options: Default::default(),
                buffers: &[Some(instance_at(0))],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: depth(true),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("pick_fs"),
                compilation_options: Default::default(),
                targets: &pick_target_state,
            }),
            multiview_mask: None,
            cache: None,
        });
        let over = [Some(wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let face_mark = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("face mark"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("face_mark_vs"),
                compilation_options: Default::default(),
                buffers: &[
                    Some(wgpu::VertexBufferLayout {
                        array_stride: VERTEX as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                    }),
                    Some(instance_at(2)),
                ],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: depth(false),
            multisample,
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("mark_fs"),
                compilation_options: Default::default(),
                targets: &over,
            }),
            multiview_mask: None,
            cache: None,
        });
        let edge_mark = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("edge mark"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("edge_mark_vs"),
                compilation_options: Default::default(),
                buffers: &[Some(instance_at(0))],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: depth(false),
            multisample,
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("mark_fs"),
                compilation_options: Default::default(),
                targets: &over,
            }),
            multiview_mask: None,
            cache: None,
        });
        let dot = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("dot"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("dot_vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: depth(false),
            multisample,
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("dot_fs"),
                compilation_options: Default::default(),
                targets: &over,
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
                    edges: l.fins,
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
        // which face each vertex is on, for picks: whole buffers line up with the vertices
        let mut face_bytes = vec![0u8; (vbytes / VERTEX as u64) as usize * 4];
        let mut base = 0usize;
        for l in all_lods() {
            for f in &l.faces {
                let from = (base + f.first_vertex as usize) * 4;
                let to = from + f.vertices as usize * 4;
                for out in face_bytes[from..to].as_chunks_mut::<4>().0 {
                    *out = f.face.to_le_bytes();
                }
            }
            base += l.vertices.len() / VERTEX;
        }
        let vertex_faces = filled(
            &device,
            "vertex faces",
            face_bytes.len() as u64,
            wgpu::BufferUsages::VERTEX,
            vec![&face_bytes],
        );
        let index: Vec<Vec<PickIndex>> = parts
            .iter()
            .zip(&lods)
            .map(|(p, row)| {
                p.lods
                    .iter()
                    .zip(row)
                    .map(|(l, d)| PickIndex {
                        faces: l.faces.clone(),
                        edges: l.edges.clone(),
                        first_segment: d.first_segment,
                    })
                    .collect()
            })
            .collect();
        let vertices = filled(&device, "vertices", vbytes, wgpu::BufferUsages::VERTEX, v);
        let indices = filled(&device, "indices", ibytes, wgpu::BufferUsages::INDEX, i);
        let segments = filled(&device, "segments", sbytes, wgpu::BufferUsages::STORAGE, s);

        let placements = filled(
            &device,
            "placements",
            scene.rows_bytes.len() as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            vec![&scene.rows_bytes],
        );
        let mut tint_bytes = Vec::with_capacity(scene.placements.len() * 16);
        for v in scene.placements.iter().flat_map(|p| &p.tint) {
            tint_bytes.extend_from_slice(&v.to_le_bytes());
        }
        let tints = filled(
            &device,
            "tints",
            tint_bytes.len() as u64,
            wgpu::BufferUsages::STORAGE,
            vec![&tint_bytes],
        );
        let instances = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("visible"),
            size: scene.placements.len().max(1) as u64 * 4,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let marks = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("marks"),
            size: MARKS as u64 * 4,
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
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: tints.as_entire_binding(),
                },
            ],
        });

        let pick_target = |format, usage, label| {
            let t = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: PICK_PX,
                    height: PICK_PX,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            });
            let v = t.create_view(&Default::default());
            (t, v)
        };
        let copied = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC;
        let (pick_id, pick_id_view) = pick_target(PICK_ID, copied, "pick id");
        let (pick_depth, pick_depth_view) = pick_target(DEPTH, copied, "pick depth");
        let read_buffer = |label| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: (PICK_ROW * PICK_PX) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            })
        };
        let pick_globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pick globals"),
            size: GLOBALS as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let pick_binds = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: pick_globals.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: placements.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: segments.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: tints.as_entire_binding(),
                },
            ],
        });
        let pick = PickState {
            id: pick_id,
            id_view: pick_id_view,
            depth: pick_depth,
            depth_view: pick_depth_view,
            id_read: read_buffer("pick id read"),
            depth_read: read_buffer("pick depth read"),
            globals: pick_globals,
            binds: pick_binds,
            surface: pick_surface,
            line: pick_line,
            ready: Arc::new(AtomicBool::new(false)),
            busy: None,
        };

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
            cube,
            face_mark,
            edge_mark,
            dot,
            marks,
            pick,
            binds,
            globals,
            placements,
            vertices,
            indices,
            instances,
            lods,
            index,
            vertex_faces,
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

    fn upload_moves(&mut self, scene: &mut Scene) {
        if let Some((lo, hi)) = scene.take_dirty() {
            self.queue
                .write_buffer(&self.placements, lo as u64, &scene.rows_bytes[lo..hi]);
        }
    }

    /// Whether a pick is under way: one at a time.
    pub fn pick_busy(&self) -> bool {
        self.pick.busy.is_some()
    }

    /// Draws what's around a pixel, only the placements there, and asks for the answer. It uses
    /// the frame's list of what's visible, so the frame after culls again.
    pub fn pick_start(&mut self, scene: &mut Scene, camera: &Camera, px: (f32, f32)) {
        let size = PICK_PX as f32;
        let view = camera.view_window(self.size.0, self.size.1, px, (size, size));
        scene.cull(&view, self.lod_px);
        self.upload_moves(scene);
        self.queue
            .write_buffer(&self.instances, 0, scene.visible.as_flattened());
        // the window is PICK_PX pixels across, a pixel of it one of the screen's
        let mut bytes = globals_bytes(&view, (PICK_PX, PICK_PX), [0; 3], "");
        bytes[88..92].copy_from_slice(&PICK_EDGE_PX.to_le_bytes());
        self.queue.write_buffer(&self.pick.globals, 0, &bytes);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("pick"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.pick.id_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.pick.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.pick.binds, &[]);
            pass.set_pipeline(&self.pick.surface);
            pass.set_vertex_buffer(0, self.vertices.slice(..));
            pass.set_vertex_buffer(1, self.instances.slice(..));
            pass.set_vertex_buffer(2, self.vertex_faces.slice(..));
            pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
            for d in &scene.draws {
                let l = self.lods[d.part as usize][self.pick_lod(d.part, d.lod)];
                pass.draw_indexed(
                    l.first_index..l.first_index + l.indices,
                    l.base_vertex,
                    d.first..d.first + d.count,
                );
            }
            pass.set_pipeline(&self.pick.line);
            pass.set_vertex_buffer(0, self.instances.slice(..));
            for d in &scene.draws {
                let l = self.lods[d.part as usize][self.pick_lod(d.part, d.lod)];
                pass.draw(
                    l.first_segment * 6..(l.first_segment + l.edges) * 6,
                    d.first..d.first + d.count,
                );
            }
        }
        let mut copy = |t: &wgpu::Texture, b: &wgpu::Buffer, aspect| {
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: t,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: b,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(PICK_ROW),
                        rows_per_image: None,
                    },
                },
                t.size(),
            );
        };
        copy(&self.pick.id, &self.pick.id_read, wgpu::TextureAspect::All);
        copy(
            &self.pick.depth,
            &self.pick.depth_read,
            wgpu::TextureAspect::DepthOnly,
        );
        self.queue.submit([encoder.finish()]);
        // the two buffers map together: when the second is ready so is the first
        self.pick.id_read.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        self.pick.ready.store(false, Ordering::Release);
        let ready = self.pick.ready.clone();
        self.pick
            .depth_read
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| {
                if r.is_ok() {
                    ready.store(true, Ordering::Release);
                }
            });
        self.pick.busy = Some((px, *camera, now_ms()));
    }

    /// The answer to the pick under way, once it's back: the pixel asked about, what's there,
    /// and how long it took in ms.
    pub fn pick_poll(&mut self, scene: &Scene) -> Option<((f32, f32), Option<PickHit>, f64)> {
        self.pick.busy?;
        let _ = self.device.poll(wgpu::PollType::Poll);
        if !self.pick.ready.swap(false, Ordering::AcqRel) {
            return None;
        }
        let (px, camera, asked) = self.pick.busy.take()?;
        let n = PICK_PX as usize;
        let mid = n as i32 / 2;
        let ids = self
            .pick
            .id_read
            .slice(..)
            .get_mapped_range()
            .expect("it said ready");
        let depths = self
            .pick
            .depth_read
            .slice(..)
            .get_mapped_range()
            .expect("it said ready");
        let word = |b: &[u8], at: usize| u32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"));
        // the nearest edge to the middle, or else the nearest of anything
        let mut best: Option<(bool, i32, usize, usize)> = None;
        for y in 0..n {
            for x in 0..n {
                let at = y * PICK_ROW as usize + x * 8;
                if word(&ids, at) == 0 {
                    continue;
                }
                let edge = word(&ids, at + 4) & SELECTED_BIT != 0;
                let (dx, dy) = (x as i32 - mid, y as i32 - mid);
                let far = dx * dx + dy * dy;
                if best.is_none_or(|b| (!edge, far) < (!b.0, b.1)) {
                    best = Some((edge, far, x, y));
                }
            }
        }
        let hit = best.map(|(edge, _, x, y)| {
            let at = y * PICK_ROW as usize + x * 8;
            let placement = word(&ids, at) - 1;
            let id = word(&ids, at + 4);
            // reversed z: depth is near / distance
            let d = f32::from_bits(word(&depths, y * PICK_ROW as usize + x * 4));
            let depth = camera.near() / d;
            let on = (px.0 + x as f32 - mid as f32, px.1 + y as f32 - mid as f32);
            let entity = if edge {
                self.edge_or_vertex(scene, placement, id & !SELECTED_BIT, &camera, px)
            } else {
                Entity::Face(id)
            };
            PickHit {
                placement,
                entity,
                point: camera.point_at(on, depth, self.size.0, self.size.1),
                depth,
            }
        });
        drop(ids);
        drop(depths);
        self.pick.id_read.unmap();
        self.pick.depth_read.unmap();
        Some((px, hit, now_ms() - asked))
    }

    /// The level a pick and a mark use for a level on screen: one that knows its faces and edges.
    fn pick_lod(&self, part: u32, lod: u32) -> usize {
        let row = &self.lods[part as usize];
        let known = row.iter().rposition(|l| l.segments > 0).unwrap_or(0);
        (lod as usize).min(known)
    }

    /// A segment drawn at a pick, as what it belongs to: its B-rep edge, or the vertex at the end
    /// of it if the cursor is that close.
    fn edge_or_vertex(
        &self,
        scene: &Scene,
        placement: u32,
        segment: u32,
        camera: &Camera,
        px: (f32, f32),
    ) -> Entity {
        let p = &scene.placements[placement as usize];
        let levels = &self.index[p.part as usize];
        let level = levels
            .iter()
            .rfind(|l| l.first_segment <= segment && !l.edges.is_empty())
            .expect("a segment is on a level");
        let local = segment - level.first_segment;
        let i = level.edges.partition_point(|e| e.first + e.count <= local);
        let e = &level.edges[i];
        let to_px = |q: [f32; 3]| camera.pixel_of(place(&p.rows, q), self.size.0, self.size.1);
        for (v, at) in [(e.start, e.start_at), (e.end, e.end_at)] {
            if let Some(o) = to_px(at) {
                let (dx, dy) = (o.0 - px.0, o.1 - px.1);
                if dx * dx + dy * dy <= PICK_VERTEX_PX * PICK_VERTEX_PX {
                    return Entity::Vertex(v);
                }
            }
        }
        Entity::Edge(e.edge)
    }

    /// Where picked vertices are this frame, into the uniforms.
    fn write_dots(&self, bytes: &mut [u8; GLOBALS], scene: &Scene, marks: &[Mark]) {
        let mut k = 0;
        for m in marks {
            let Entity::Vertex(v) = m.entity else { continue };
            let p = &scene.placements[m.placement as usize];
            let found = self.index[p.part as usize][0].edges.iter().find_map(|e| {
                (e.start == v)
                    .then_some(e.start_at)
                    .or((e.end == v).then_some(e.end_at))
            });
            if let Some(at) = found
                && k < DOTS
            {
                let [x, y, z] = place(&p.rows, at);
                let w = if m.selected { 2.0f32 } else { 1.0 };
                for (j, value) in [x, y, z, w].iter().enumerate() {
                    let o = DOTS_AT + k * 16 + j * 4;
                    bytes[o..o + 4].copy_from_slice(&value.to_le_bytes());
                }
                k += 1;
            }
        }
    }

    /// Draws what's hovered and selected, over what's there.
    fn draw_marks(&self, pass: &mut wgpu::RenderPass<'_>, scene: &Scene, marks: &[Mark]) {
        if marks.is_empty() {
            return;
        }
        pass.set_vertex_buffer(1, self.marks.slice(..));
        pass.set_pipeline(&self.face_mark);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        for (k, m) in marks.iter().take(MARKS).enumerate() {
            let k = k as u32;
            let part = scene.placements[m.placement as usize].part;
            let Some(lod) = scene.lod_on_screen(m.placement) else {
                continue;
            };
            let lod = self.pick_lod(part, lod);
            if let Entity::Face(f) = m.entity
                && let Some(r) = self.index[part as usize][lod].faces.iter().find(|r| r.face == f)
            {
                let l = self.lods[part as usize][lod];
                pass.draw_indexed(
                    l.first_index + r.first_tri * 3..l.first_index + (r.first_tri + r.tris) * 3,
                    l.base_vertex,
                    k..k + 1,
                );
            }
        }
        pass.set_pipeline(&self.edge_mark);
        pass.set_vertex_buffer(0, self.marks.slice(..));
        for (k, m) in marks.iter().take(MARKS).enumerate() {
            let k = k as u32;
            let part = scene.placements[m.placement as usize].part;
            let Some(lod) = scene.lod_on_screen(m.placement) else {
                continue;
            };
            let lod = self.pick_lod(part, lod);
            if let Entity::Edge(e) = m.entity
                && let Some(r) = self.index[part as usize][lod].edges.iter().find(|r| r.edge == e)
            {
                let first = self.lods[part as usize][lod].first_segment + r.first;
                pass.draw(first * 6..(first + r.count) * 6, k..k + 1);
            }
        }
        pass.set_pipeline(&self.dot);
        pass.draw(0..DOTS as u32 * 6, 0..1);
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
        overlay: Overlay,
    ) -> Drawn {
        let hud = overlay.hud;
        let start = now_ms();
        let view = camera.view(self.size.0, self.size.1);
        scene.cull(&view, self.lod_px);
        let culled = now_ms();
        self.upload_moves(scene);
        self.queue
            .write_buffer(&self.instances, 0, scene.visible.as_flattened());

        let hover = overlay.cube.map_or([0; 3], |c| c.2);
        let mut bytes = globals_bytes(&view, self.size, hover, hud.unwrap_or(""));
        self.write_dots(&mut bytes, scene, overlay.marks);
        let marked: Vec<[u8; 4]> = overlay
            .marks
            .iter()
            .take(MARKS)
            .map(|m| (m.placement | if m.selected { SELECTED_BIT } else { 0 }).to_le_bytes())
            .collect();
        if !marked.is_empty() {
            self.queue.write_buffer(&self.marks, 0, marked.as_flattened());
        }
        self.queue.write_buffer(&self.globals, 0, &bytes);

        let uploaded = now_ms();
        self.read_timers();
        let slot = self.timer_at;
        let timed = self.timers.get(slot).is_some_and(|t| !t.busy);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let mut drawn = Drawn {
            placements: scene.visible.len() as u32,
            cull_ms: (culled - start) as f32,
            upload_ms: (uploaded - culled) as f32,
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
            self.draw_marks(&mut pass, scene, overlay.marks);
            if let Some((at, side, _)) = overlay.cube {
                pass.set_viewport(at.0, at.1, side, side, 0.0, 1.0);
                pass.set_pipeline(&self.cube);
                pass.draw(0..36, 0..1);
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
        drawn.encode_ms = (now_ms() - uploaded) as f32;
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

fn place(rows: &crate::scene::Rows, q: [f32; 3]) -> [f32; 3] {
    rows.map(|r| r[0] * q[0] + r[1] * q[1] + r[2] * q[2] + r[3])
}

/// The shader's uniforms for a view.
fn globals_bytes(view: &View, size: (u32, u32), hover: [i8; 3], hud: &str) -> [u8; GLOBALS] {
    let mut bytes = [0u8; GLOBALS];
    let vec3 = |v: [f32; 3]| [v[0], v[1], v[2], 0.0];
    let words = view
        .matrix
        .into_iter()
        .chain(vec3(view.eye))
        .chain([
            2.0 / size.0 as f32,
            2.0 / size.1 as f32,
            LINE_HALF_PX,
            MARK_HALF_PX,
        ])
        .chain(vec3(view.right))
        .chain(vec3(view.up))
        .chain(vec3(view.forward))
        .chain(vec3(hover.map(|v| v as f32)));
    for (out, v) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(words) {
        *out = v.to_le_bytes();
    }
    let text = &mut bytes[TEXT_AT..];
    text.fill(255);
    for (c, b) in text.iter_mut().zip(hud.bytes()) {
        *c = glyph(b);
    }
    bytes
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

/// Half a marked edge's width in pixels.
const MARK_HALF_PX: f32 = 1.5;
/// How far from a pixel an edge is picked, and a vertex at its end (pixels).
const PICK_EDGE_PX: f32 = 4.0;
const PICK_VERTEX_PX: f32 = 7.0;
/// Where the picked vertices start in the globals.
const DOTS_AT: usize = TEXT_AT - DOTS * 16;
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
