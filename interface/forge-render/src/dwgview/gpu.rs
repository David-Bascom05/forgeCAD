//! A `Geom` drawn in the egui frame, as a paint callback: its buffers go to the GPU once (kept
//! in the egui renderer's callback resources, by the geometry's id), and each frame writes only
//! the view's uniform, and the layers' and the selection's words when they changed.

use super::{FILL_BYTES, Geom, LINE_BYTES};
use std::sync::Arc;

/// The format egui draws to: the host puts it in the egui renderer's callback resources, so
/// the pipelines match its pass.
pub struct Target(pub wgpu::TextureFormat);

/// How the space is seen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    /// The world point at the canvas centre.
    pub center: [f64; 2],
    /// Points (logical pixels) a world unit.
    pub scale: f64,
    /// Points a millimetre of lineweight; 0: every line one device pixel (LWDISPLAY off).
    pub lw_per_mm: f32,
    /// A layout's sheet: white drawn black.
    pub paper: bool,
    pub selected: [u8; 3],
    pub hovered: [u8; 3],
}

/// Per-item or per-layer words and their version; `since` the items changed after an older
/// version, for a write of just those.
#[derive(Clone)]
pub struct Words {
    pub words: Arc<Vec<u32>>,
    pub version: u64,
    /// (the version it was, first word, one past the last) changed since.
    pub since: Option<(u64, u32, u32)>,
}

/// One frame's draw of a geometry in a rectangle of the egui frame.
pub struct Paint {
    pub geom: Arc<Geom>,
    pub look: Look,
    /// Where, in points.
    pub rect: egui::Rect,
    /// A word a layer: bit 0 shown.
    pub layers: Words,
    /// A word an item: bit 0 selected, bit 1 hovered.
    pub flags: Words,
}

struct Pipes {
    lines: wgpu::RenderPipeline,
    fills: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}

struct OnGpu {
    id: u64,
    used: u64,
    /// The segments' instances and the fills' vertices, in buffers no bigger than the GPU takes:
    /// each with how many it holds.
    lines: Vec<(wgpu::Buffer, u32)>,
    fills: Vec<(wgpu::Buffer, u32)>,
    uniform: wgpu::Buffer,
    layers: wgpu::Buffer,
    layers_version: u64,
    flags: wgpu::Buffer,
    flags_version: u64,
    bind: wgpu::BindGroup,
}

/// What lives in the callback resources.
struct Store {
    pipes: Pipes,
    scenes: Vec<OnGpu>,
    frame: u64,
}

/// How many scenes keep their buffers (tabs, spaces gone back to).
const KEEP: usize = 3;

fn pipes(device: &wgpu::Device, format: wgpu::TextureFormat) -> Pipes {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("dwgview"),
        source: wgpu::ShaderSource::Wgsl(include_str!("dwgview.wgsl").into()),
    });
    let storage = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("dwgview"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            storage(1),
            storage(2),
        ],
    });
    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("dwgview"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let line_attrs = wgpu::vertex_attr_array![
        0 => Float32x2, 1 => Float32x2, 2 => Uint32, 3 => Float32, 4 => Uint32, 5 => Uint32
    ];
    let fill_attrs = wgpu::vertex_attr_array![0 => Float32x2, 1 => Uint32, 2 => Uint32, 3 => Uint32];
    let make = |entry: &str, buffers: &[Option<wgpu::VertexBufferLayout>]| {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("dwgview"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    };
    Pipes {
        lines: make(
            "vs_line",
            &[Some(wgpu::VertexBufferLayout {
                array_stride: LINE_BYTES as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &line_attrs,
            })],
        ),
        fills: make(
            "vs_fill",
            &[Some(wgpu::VertexBufferLayout {
                array_stride: FILL_BYTES as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &fill_attrs,
            })],
        ),
        layout,
    }
}

fn bytes(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|w| w.to_le_bytes()).collect()
}

fn buffer(device: &wgpu::Device, label: &str, data: &[u8], usage: wgpu::BufferUsages) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;
    // never empty: a binding of nothing is refused
    let mut padded;
    let data = if data.len() < 16 {
        padded = data.to_vec();
        padded.resize(16, 0);
        &padded[..]
    } else {
        data
    };
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: data,
        usage,
    })
}

const UNIFORM_BYTES: usize = 64;

/// The largest vertex buffer made, short of the GPU's own limit.
const CHUNK_BYTES: u64 = 1 << 27;

/// `data`, records of `stride` bytes, in buffers of whole groups of `group` records.
fn chunks(
    device: &wgpu::Device,
    label: &str,
    data: &[u8],
    stride: usize,
    group: usize,
) -> Vec<(wgpu::Buffer, u32)> {
    let cap = device.limits().max_buffer_size.min(CHUNK_BYTES) as usize;
    let per = (cap / (stride * group)).max(1) * stride * group;
    data.chunks(per)
        .map(|c| {
            (
                buffer(device, label, c, wgpu::BufferUsages::VERTEX),
                (c.len() / stride) as u32,
            )
        })
        .collect()
}

fn upload(device: &wgpu::Device, pipes: &Pipes, p: &Paint) -> OnGpu {
    use wgpu::BufferUsages as B;
    let g = &p.geom;
    let uniform = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("dwgview uniform"),
        size: UNIFORM_BYTES as u64,
        usage: B::UNIFORM | B::COPY_DST,
        mapped_at_creation: false,
    });
    let layers = buffer(
        device,
        "dwgview layers",
        &bytes(&p.layers.words),
        B::STORAGE | B::COPY_DST,
    );
    let flags = buffer(
        device,
        "dwgview flags",
        &bytes(&p.flags.words),
        B::STORAGE | B::COPY_DST,
    );
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("dwgview"),
        layout: &pipes.layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: layers.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: flags.as_entire_binding(),
            },
        ],
    });
    OnGpu {
        id: g.id,
        used: 0,
        lines: chunks(device, "dwgview lines", &g.lines, LINE_BYTES, 1),
        fills: chunks(device, "dwgview fills", &g.fills, FILL_BYTES, 3),
        uniform,
        layers,
        layers_version: p.layers.version,
        flags,
        flags_version: p.flags.version,
        bind,
    }
}

/// The words changed since the GPU's version: just those, or all of them.
fn update(queue: &wgpu::Queue, buf: &wgpu::Buffer, have: &mut u64, w: &Words) {
    if *have == w.version {
        return;
    }
    match w.since {
        Some((from, lo, hi)) if from == *have && lo < hi && hi as usize <= w.words.len() => {
            queue.write_buffer(buf, lo as u64 * 4, &bytes(&w.words[lo as usize..hi as usize]));
        }
        _ if !w.words.is_empty() => queue.write_buffer(buf, 0, &bytes(&w.words)),
        _ => {}
    }
    *have = w.version;
}

fn rgba(c: [u8; 3]) -> [f32; 4] {
    [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, 1.0]
}

impl egui_wgpu::CallbackTrait for Paint {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        res: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if res.get::<Store>().is_none() {
            let Some(Target(format)) = res.get::<Target>() else {
                return vec![];
            };
            let store = Store {
                pipes: pipes(device, *format),
                scenes: vec![],
                frame: 0,
            };
            res.insert(store);
        }
        let Some(store) = res.get_mut::<Store>() else {
            return vec![];
        };
        store.frame += 1;
        let frame = store.frame;
        let k = match store.scenes.iter().position(|s| s.id == self.geom.id) {
            Some(k) => k,
            None => {
                if store.scenes.len() >= KEEP
                    && let Some(old) = (0..store.scenes.len()).min_by_key(|&k| store.scenes[k].used)
                {
                    store.scenes.swap_remove(old);
                }
                let s = upload(device, &store.pipes, self);
                store.scenes.push(s);
                store.scenes.len() - 1
            }
        };
        let s = &mut store.scenes[k];
        s.used = frame;
        update(queue, &s.layers, &mut s.layers_version, &self.layers);
        update(queue, &s.flags, &mut s.flags_version, &self.flags);
        // the viewport as egui-wgpu sets it, rounded to device pixels
        let ppp = screen.pixels_per_point;
        let vp = egui::epaint::ViewportInPixels::from_points(&self.rect, ppp, screen.size_in_pixels);
        let half = [vp.width_px as f32 * 0.5, vp.height_px as f32 * 0.5];
        // the world point at the viewport's centre: the rect's, less what rounding moved it
        let scale = self.look.scale * ppp as f64;
        let moved = [
            (vp.left_px as f64 + half[0] as f64) - self.rect.center().x as f64 * ppp as f64,
            (vp.top_px as f64 + half[1] as f64) - self.rect.center().y as f64 * ppp as f64,
        ];
        let center = [
            self.look.center[0] + moved[0] / scale - self.geom.origin[0],
            self.look.center[1] - moved[1] / scale - self.geom.origin[1],
        ];
        let mut u = Vec::with_capacity(UNIFORM_BYTES);
        for f in [
            center[0] as f32,
            center[1] as f32,
            half[0],
            half[1],
            scale as f32,
            self.look.lw_per_mm * ppp,
        ] {
            u.extend_from_slice(&f.to_le_bytes());
        }
        u.extend_from_slice(&(self.look.paper as u32).to_le_bytes());
        u.extend_from_slice(&0u32.to_le_bytes());
        for c in [rgba(self.look.selected), rgba(self.look.hovered)] {
            for f in c {
                u.extend_from_slice(&f.to_le_bytes());
            }
        }
        queue.write_buffer(&s.uniform, 0, &u);
        vec![]
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        res: &egui_wgpu::CallbackResources,
    ) {
        let Some(store) = res.get::<Store>() else { return };
        let Some(s) = store.scenes.iter().find(|s| s.id == self.geom.id) else {
            return;
        };
        pass.set_bind_group(0, &s.bind, &[]);
        pass.set_pipeline(&store.pipes.fills);
        for (b, n) in &s.fills {
            pass.set_vertex_buffer(0, b.slice(..));
            pass.draw(0..*n, 0..1);
        }
        pass.set_pipeline(&store.pipes.lines);
        for (b, n) in &s.lines {
            pass.set_vertex_buffer(0, b.slice(..));
            pass.draw(0..6, 0..*n);
        }
    }
}
