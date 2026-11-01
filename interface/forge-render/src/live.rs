//! Live editing, natively: meshes each changed face for the GPU and drives a value through the
//! worker in `forge_agent::live`. A drag draws a coarser level; the commit brings the full mesh.

use crate::livegpu::{Delta, FaceGpu};
use crate::mesh::{PartMesh, VERTEX};
use crate::tess;
use forge_agent::facemesh;
use forge_agent::live::{Live, Stage};
use forge_agent::tools::Part;
use forge_mesh::Quality;
use forge_topo::{Body, FaceId};
use std::collections::HashMap;
use std::sync::Arc;

/// What the worker hands the screen.
pub enum Out {
    Delta(Delta),
    Whole(Box<PartMesh>),
}

/// Threads meshing changed faces, when enough have changed to be worth them.
const THREADS: usize = 4;
const THREADS_FROM: usize = 8;

/// Faces meshed so far, by the key of what shapes them: a face no edit reached is never meshed
/// again, its triangles stay where they are on the GPU.
#[derive(Default)]
pub struct Mesher {
    quality: Option<Quality>,
    faces: HashMap<u64, Arc<FaceGpu>>,
}

/// The ladder's coarsest level still under about half a pixel at a window's fit of the part.
fn quality_for(radius: f64) -> Quality {
    let (chord, turn) = tess::chords()
        .iter()
        .rev()
        .find(|c| c.0 <= radius / 200.0)
        .copied()
        .unwrap_or(tess::chords()[0]);
    Quality {
        chord,
        max_angle: turn * core::f64::consts::PI / 180.0,
    }
}

fn box_radius(body: &Body) -> f64 {
    let mut bb = forge_math::Box3::EMPTY;
    for f in body.face_ids() {
        bb = bb.hull(&body.face_box(f));
    }
    let d = |r: forge_math::Interval| r.hi - r.lo;
    (d(bb.x).powi(2) + d(bb.y).powi(2) + d(bb.z).powi(2)).sqrt() / 2.0
}

fn mesh_face(body: &Body, f: FaceId, q: &Quality) -> Arc<FaceGpu> {
    let alone = facemesh::alone(body, f);
    let lod = tess::build_lod(&alone, q).or_else(|_| {
        // too coarse for this face: the next level down
        tess::build_lod(&alone, &Quality::default())
    });
    let Ok(lod) = lod else {
        return Arc::new(FaceGpu::default());
    };
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for v in lod.vertices.as_chunks::<VERTEX>().0 {
        for k in 0..3 {
            let x = f32::from_le_bytes(v[k * 4..k * 4 + 4].try_into().expect("4 bytes"));
            lo[k] = lo[k].min(x);
            hi[k] = hi[k].max(x);
        }
    }
    let indices = lod
        .indices
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b))
        .collect();
    let segments = tess::keep_own_edges(body, f, &alone, &lod);
    Arc::new(FaceGpu {
        vertices: lod.vertices,
        indices,
        segments,
        lo,
        hi,
    })
}

impl Mesher {
    /// What changed since last time.
    pub fn delta(&mut self, body: &Body) -> Delta {
        let q = *self.quality.get_or_insert_with(|| quality_for(box_radius(body)));
        let keys: Vec<(FaceId, u64)> = body.face_ids().map(|f| (f, facemesh::key(body, f).0)).collect();
        let new: Vec<(FaceId, u64)> = keys
            .iter()
            .filter(|(_, k)| !self.faces.contains_key(k))
            .copied()
            .collect();
        let meshed = mesh_all(body, &new, &q);
        let mut d = Delta {
            meshed: new.len(),
            faces: keys.len(),
            chord: q.chord as f32,
            ..Delta::default()
        };
        let mut next = HashMap::with_capacity(keys.len());
        for &(_, k) in &keys {
            if let Some(f) = self.faces.remove(&k) {
                next.insert(k, f);
            }
        }
        d.gone = self.faces.keys().copied().collect();
        for (k, f) in meshed {
            if next.insert(k, f.clone()).is_none() {
                d.added.push((k, f));
            }
        }
        self.faces = next;
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for f in self.faces.values().filter(|f| !f.vertices.is_empty()) {
            for k in 0..3 {
                lo[k] = lo[k].min(f.lo[k]);
                hi[k] = hi[k].max(f.hi[k]);
            }
        }
        d.center = [0, 1, 2].map(|k| (lo[k] + hi[k]) * 0.5);
        let half: f32 = (0..3).map(|k| (hi[k] - lo[k]).powi(2)).sum();
        d.radius = half.sqrt() * 0.5;
        d
    }
}

fn mesh_all(body: &Body, faces: &[(FaceId, u64)], q: &Quality) -> Vec<(u64, Arc<FaceGpu>)> {
    if faces.len() < THREADS_FROM {
        return faces.iter().map(|&(f, k)| (k, mesh_face(body, f, q))).collect();
    }
    let at = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|s| {
        let workers: Vec<_> = (0..THREADS)
            .map(|_| {
                s.spawn(|| {
                    let mut got = Vec::new();
                    loop {
                        let i = at.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(&(f, k)) = faces.get(i) else { return got };
                        got.push((k, mesh_face(body, f, q)));
                    }
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|w| w.join().expect("a mesher panicked"))
            .collect()
    })
}

/// The worker for this part: faces while dragging, the full mesh once committed.
pub fn start(part: Part) -> Live<Out> {
    let mut m = Mesher::default();
    Live::start(part, move |body, stage| match stage {
        Stage::Warm | Stage::Drag => Out::Delta(m.delta(body)),
        Stage::Committed => match tess::build_part(body) {
            Ok(mesh) => Out::Whole(Box::new(mesh)),
            Err(_) => Out::Delta(Delta::default()),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_faces_an_edit_reaches_are_meshed_again() {
        let mut p = Part::new(
            "base: extrude 20 rect -50 -50 100 100\nhole 8 -xy at -30 -30 20 thru\nhole 8 -xy at 30 30 20 thru\n",
        );
        let mut m = Mesher::default();
        let whole = m.delta(p.model.body().unwrap());
        assert_eq!(whole.meshed, whole.faces);
        assert_eq!(whole.added.len(), whole.faces);
        assert!(whole.gone.is_empty());
        // the same part: nothing
        let same = m.delta(p.model.body().unwrap());
        assert_eq!((same.meshed, same.added.len(), same.gone.len()), (0, 0, 0));
        // one hole wider: its wall, and the top and bottom it cuts; the block's sides and the other
        // hole's wall stay where they are
        let d = p.dims().into_iter().find(|d| d.name == "hole2.1").unwrap();
        p.drag(d.at.op("9".into())).unwrap();
        let moved = m.delta(p.model.body().unwrap());
        assert_eq!(moved.faces, whole.faces);
        assert!(moved.meshed <= 3, "{} of {}", moved.meshed, moved.faces);
        assert_eq!(moved.gone.len(), moved.added.len());
    }

    #[test]
    fn every_edge_is_drawn_once() {
        let p = Part::new("extrude 20 rect 0 0 100 100\n");
        let d = Mesher::default().delta(p.model.body().unwrap());
        let segs: usize = d
            .added
            .iter()
            .map(|(_, f)| f.segments.len() / crate::mesh::SEGMENT)
            .sum();
        assert_eq!(segs, 12);
    }
}
