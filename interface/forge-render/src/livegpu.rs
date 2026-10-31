//! A part being dragged on the GPU: each B-rep face in a slot of its own, so an edit writes only
//! the faces it changed. The same on both targets; `live` makes the faces natively.

use crate::mesh::{SEGMENT, VERTEX};
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

/// A face ready for the GPU: indices count from its own first vertex.
#[derive(Debug, Default)]
pub struct FaceGpu {
    pub vertices: Vec<u8>,
    pub indices: Vec<u32>,
    pub segments: Vec<u8>,
    /// Its box.
    pub lo: [f32; 3],
    pub hi: [f32; 3],
}

/// What changed since the last delta: faces by the key of what shaped them.
#[derive(Debug, Default)]
pub struct Delta {
    pub gone: Vec<u64>,
    pub added: Vec<(u64, Arc<FaceGpu>)>,
    /// The whole part's sphere and how many faces it has now.
    pub center: [f32; 3],
    pub radius: f32,
    pub chord: f32,
    pub faces: usize,
    /// Faces meshed for this delta; `added` may be fewer, or more, where one was reused.
    pub meshed: usize,
}

/// Spans of one buffer, in its units: freed ones are used again, neighbours merged.
#[derive(Debug, Default)]
struct Pool {
    /// Free spans (start, length), by start.
    free: Vec<(u32, u32)>,
    /// Where the never-used part starts.
    top: u32,
    cap: u32,
}

impl Pool {
    fn alloc(&mut self, n: u32) -> Option<u32> {
        if n == 0 {
            return Some(0);
        }
        if let Some(k) = self.free.iter().position(|f| f.1 >= n) {
            let (at, len) = self.free[k];
            if len == n {
                self.free.remove(k);
            } else {
                self.free[k] = (at + n, len - n);
            }
            return Some(at);
        }
        if self.top + n > self.cap {
            return None;
        }
        self.top += n;
        Some(self.top - n)
    }

    fn release(&mut self, at: u32, n: u32) {
        if n == 0 {
            return;
        }
        let k = self.free.partition_point(|f| f.0 < at);
        self.free.insert(k, (at, n));
        if k + 1 < self.free.len() && self.free[k].0 + self.free[k].1 == self.free[k + 1].0 {
            self.free[k].1 += self.free[k + 1].1;
            self.free.remove(k + 1);
        }
        if k > 0 && self.free[k - 1].0 + self.free[k - 1].1 == self.free[k].0 {
            self.free[k - 1].1 += self.free[k].1;
            self.free.remove(k);
        }
        if let Some(&(a, l)) = self.free.last()
            && a + l == self.top
        {
            self.top = a;
            self.free.pop();
        }
    }
}

/// Where a face is.
#[derive(Clone, Debug)]
struct Slot {
    face: Arc<FaceGpu>,
    vertex: u32,
    index: u32,
    segment: u32,
}

impl Slot {
    fn counts(&self) -> (u32, u32, u32) {
        (
            (self.face.vertices.len() / VERTEX) as u32,
            self.face.indices.len() as u32,
            (self.face.segments.len() / SEGMENT) as u32,
        )
    }
}

pub struct LiveGpu {
    pub shown: bool,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    segments: wgpu::Buffer,
    line_indices: wgpu::Buffer,
    binds: wgpu::BindGroup,
    slots: HashMap<u64, Slot>,
    pools: [Pool; 3],
    /// What a frame draws: contiguous spans of indices and of line indices.
    runs: Vec<Range<u32>>,
    line_runs: Vec<Range<u32>>,
    tris: u64,
    segs: u64,
}

const COPY_INDEX: wgpu::BufferUsages = wgpu::BufferUsages::INDEX.union(wgpu::BufferUsages::COPY_DST);
const COPY_VERTEX: wgpu::BufferUsages = wgpu::BufferUsages::VERTEX.union(wgpu::BufferUsages::COPY_DST);
const COPY_STORAGE: wgpu::BufferUsages = wgpu::BufferUsages::STORAGE.union(wgpu::BufferUsages::COPY_DST);

fn buffer(device: &wgpu::Device, label: &str, size: u64, usage: wgpu::BufferUsages) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: size.max(16),
        usage,
        mapped_at_creation: false,
    })
}

/// The two triangles of each of `cap` segments.
fn line_pattern(device: &wgpu::Device, queue: Option<&wgpu::Queue>, cap: u32) -> wgpu::Buffer {
    let b = buffer(device, "live line indices", cap as u64 * 24, COPY_INDEX);
    if let Some(q) = queue {
        let mut pattern = Vec::with_capacity(cap as usize * 24);
        for k in 0..cap {
            for v in [0, 1, 2, 2, 1, 3] {
                pattern.extend_from_slice(&(k * 4 + v).to_le_bytes());
            }
        }
        q.write_buffer(&b, 0, &pattern);
    }
    b
}

impl LiveGpu {
    pub fn new(device: &wgpu::Device, bind: &dyn Fn(&wgpu::Buffer) -> wgpu::BindGroup) -> LiveGpu {
        let segments = buffer(device, "live segments", 16, COPY_STORAGE);
        let binds = bind(&segments);
        LiveGpu {
            shown: false,
            vertices: buffer(device, "live vertices", 16, COPY_VERTEX),
            indices: buffer(device, "live indices", 16, COPY_INDEX),
            line_indices: line_pattern(device, None, 0),
            segments,
            binds,
            slots: HashMap::new(),
            pools: Default::default(),
            runs: Vec::new(),
            line_runs: Vec::new(),
            tris: 0,
            segs: 0,
        }
    }

    pub fn tris(&self) -> u64 {
        self.tris
    }

    pub fn segments(&self) -> u64 {
        self.segs
    }

    /// Takes the delta in, writing only its faces (everything, if a buffer had to grow). The bytes
    /// written.
    pub fn apply(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        delta: &Delta,
        bind: &dyn Fn(&wgpu::Buffer) -> wgpu::BindGroup,
    ) -> u64 {
        for key in &delta.gone {
            if let Some(s) = self.slots.remove(key) {
                let (v, i, g) = s.counts();
                self.pools[0].release(s.vertex, v);
                self.pools[1].release(s.index, i);
                self.pools[2].release(s.segment, g);
            }
        }
        let mut write: Vec<u64> = Vec::with_capacity(delta.added.len());
        let mut grow = false;
        for (key, face) in &delta.added {
            let mut slot = Slot {
                face: face.clone(),
                vertex: 0,
                index: 0,
                segment: 0,
            };
            let (v, i, g) = slot.counts();
            let spans = (
                self.pools[0].alloc(v),
                self.pools[1].alloc(i),
                self.pools[2].alloc(g),
            );
            let (Some(a), Some(b), Some(c)) = spans else {
                // no room: lay everything out afresh in bigger buffers, below
                grow = true;
                self.slots.insert(*key, slot);
                continue;
            };
            (slot.vertex, slot.index, slot.segment) = (a, b, c);
            self.slots.insert(*key, slot);
            write.push(*key);
        }
        if grow {
            self.grow(device, queue, bind);
            write = self.slots.keys().copied().collect();
        }
        let mut bytes = 0;
        let mut ix = Vec::new();
        for key in &write {
            let s = &self.slots[key];
            let f = &s.face;
            queue.write_buffer(&self.vertices, s.vertex as u64 * VERTEX as u64, &f.vertices);
            ix.clear();
            ix.reserve(f.indices.len() * 4);
            for i in &f.indices {
                ix.extend_from_slice(&(i + s.vertex).to_le_bytes());
            }
            queue.write_buffer(&self.indices, s.index as u64 * 4, &ix);
            if !f.segments.is_empty() {
                queue.write_buffer(&self.segments, s.segment as u64 * SEGMENT as u64, &f.segments);
            }
            bytes += (f.vertices.len() + ix.len() + f.segments.len()) as u64;
        }
        if !delta.gone.is_empty() || !delta.added.is_empty() {
            self.lay_out_runs();
        }
        bytes
    }

    /// Room for twice what there is, every face placed again.
    fn grow(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind: &dyn Fn(&wgpu::Buffer) -> wgpu::BindGroup,
    ) {
        let total = self
            .slots
            .values()
            .map(Slot::counts)
            .fold((0u32, 0u32, 0u32), |a, c| (a.0 + c.0, a.1 + c.1, a.2 + c.2));
        let cap = [total.0, total.1, total.2].map(|n| (n * 2).max(1024));
        self.pools = cap.map(|cap| Pool {
            cap,
            ..Pool::default()
        });
        self.vertices = buffer(
            device,
            "live vertices",
            cap[0] as u64 * VERTEX as u64,
            COPY_VERTEX,
        );
        self.indices = buffer(device, "live indices", cap[1] as u64 * 4, COPY_INDEX);
        self.segments = buffer(
            device,
            "live segments",
            cap[2] as u64 * SEGMENT as u64,
            COPY_STORAGE,
        );
        self.line_indices = line_pattern(device, Some(queue), cap[2]);
        self.binds = bind(&self.segments);
        for s in self.slots.values_mut() {
            let (v, i, g) = s.counts();
            s.vertex = self.pools[0].alloc(v).expect("room for all");
            s.index = self.pools[1].alloc(i).expect("room for all");
            s.segment = self.pools[2].alloc(g).expect("room for all");
        }
    }

    fn lay_out_runs(&mut self) {
        let merged = |mut spans: Vec<Range<u32>>| {
            spans.sort_by_key(|s| s.start);
            let mut out: Vec<Range<u32>> = Vec::new();
            for s in spans.into_iter().filter(|s| !s.is_empty()) {
                match out.last_mut() {
                    Some(l) if l.end == s.start => l.end = s.end,
                    _ => out.push(s),
                }
            }
            out
        };
        let counts = |s: &Slot| s.counts();
        self.runs = merged(
            self.slots
                .values()
                .map(|s| s.index..s.index + counts(s).1)
                .collect(),
        );
        self.line_runs = merged(
            self.slots
                .values()
                .map(|s| s.segment * 6..(s.segment + counts(s).2) * 6)
                .collect(),
        );
        self.tris = self.runs.iter().map(|r| r.len() as u64 / 3).sum();
        self.segs = self.line_runs.iter().map(|r| r.len() as u64 / 6).sum();
    }

    /// The faces, and their lines if given a pipeline for them.
    pub fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        surface: &'a wgpu::RenderPipeline,
        line: Option<&'a wgpu::RenderPipeline>,
        instances: Range<u32>,
        visible: &'a wgpu::Buffer,
    ) {
        pass.set_bind_group(0, &self.binds, &[]);
        pass.set_pipeline(surface);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_vertex_buffer(1, visible.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        for r in &self.runs {
            pass.draw_indexed(r.clone(), 0, instances.clone());
        }
        if let Some(line) = line {
            pass.set_pipeline(line);
            pass.set_vertex_buffer(0, visible.slice(..));
            pass.set_index_buffer(self.line_indices.slice(..), wgpu::IndexFormat::Uint32);
            for r in &self.line_runs {
                pass.draw_indexed(r.clone(), 0, instances.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_come_back_merged() {
        let mut p = Pool {
            cap: 100,
            ..Pool::default()
        };
        let (a, b, c) = (p.alloc(10), p.alloc(10), p.alloc(10));
        assert_eq!((a, b, c), (Some(0), Some(10), Some(20)));
        p.release(0, 10);
        p.release(10, 10);
        assert_eq!(p.free, [(0, 20)]);
        assert_eq!(p.alloc(5), Some(0));
        assert_eq!(p.free, [(5, 15)]);
        assert_eq!(p.alloc(200), None);
        p.release(20, 10);
        assert_eq!((p.top, p.free.len()), (5, 0), "the end goes back to the top");
        p.release(0, 5);
        assert_eq!((p.top, p.free.len()), (0, 0));
    }
}
