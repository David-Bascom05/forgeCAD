//! A DWG drawing's space (`forge_dwg::scene::Scene`) on the GPU and under the pointer: its
//! segments and filled triangles made once into vertex data (`Geom`), drawn by `gpu::Paint` in
//! the egui frame with pan and zoom in the shader, and picked on the CPU through a grid of its
//! segments. Native and in the browser alike.

pub mod gpu;
pub mod pick;
pub mod tri;

use forge_dwg::scene::{Prim, Scene};
use pick::{Grid, seg_dist};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Bytes of a segment's instance: a, b, rgb, weight, layer, item.
pub const LINE_BYTES: usize = 32;
/// Bytes of a fill's vertex: p, rgb, layer, item.
pub const FILL_BYTES: usize = 20;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// A scene made ready to draw and to pick from.
pub struct Geom {
    /// Told apart from every other on the GPU by this.
    pub id: u64,
    pub scene: Arc<Scene>,
    /// What every coordinate is less, so the f32s keep their digits near the drawing.
    pub origin: [f64; 2],
    pub lines: Vec<u8>,
    pub n_lines: u32,
    pub fills: Vec<u8>,
    pub n_fill_verts: u32,
    grid: Grid,
    /// Each item's box, less the origin: lo x, lo y, hi x, hi y.
    boxes: Vec<[f32; 4]>,
    /// Item k's segments are instances `item_segs[k]..item_segs[k + 1]` of `lines`.
    item_segs: Vec<u32>,
    /// The fills' items, in drawing order.
    fill_items: Vec<u32>,
    /// Each entity's items.
    pub by_entity: BTreeMap<u32, Vec<u32>>,
}

fn pack(rgb: [u8; 3]) -> u32 {
    rgb[0] as u32 | (rgb[1] as u32) << 8 | (rgb[2] as u32) << 16 | 0xFF00_0000
}

impl Geom {
    pub fn new(scene: Arc<Scene>) -> Geom {
        let origin = scene.extents.map_or([0.0; 2], |(lo, hi)| {
            [0.5 * (lo[0] + hi[0]), 0.5 * (lo[1] + hi[1])]
        });
        let local = |p: [f64; 2]| [(p[0] - origin[0]) as f32, (p[1] - origin[1]) as f32];
        let mut lines = vec![];
        let mut fills = vec![];
        let mut boxes = Vec::with_capacity(scene.items.len());
        let mut item_segs = Vec::with_capacity(scene.items.len() + 1);
        let mut fill_items = vec![];
        let mut by_entity: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        let put = |out: &mut Vec<u8>, words: &[u32]| {
            for w in words {
                out.extend_from_slice(&w.to_le_bytes());
            }
        };
        for (i, it) in scene.items.iter().enumerate() {
            let item = i as u32;
            by_entity.entry(it.entity).or_default().push(item);
            item_segs.push((lines.len() / LINE_BYTES) as u32);
            let rgb = pack(it.rgb);
            let layer = it.layer as u32;
            let mut bx = [f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY];
            let mut grow = |p: [f32; 2]| {
                bx[0] = bx[0].min(p[0]);
                bx[1] = bx[1].min(p[1]);
                bx[2] = bx[2].max(p[0]);
                bx[3] = bx[3].max(p[1]);
            };
            match &it.prim {
                Prim::Path { pts, closed } => {
                    let pts: Vec<[f32; 2]> = pts
                        .iter()
                        .filter(|p| p[0].is_finite() && p[1].is_finite())
                        .map(|&p| local(p))
                        .collect();
                    pts.iter().for_each(|&p| grow(p));
                    let mut add = |a: [f32; 2], b: [f32; 2]| {
                        put(
                            &mut lines,
                            &[
                                a[0].to_bits(),
                                a[1].to_bits(),
                                b[0].to_bits(),
                                b[1].to_bits(),
                                rgb,
                                it.weight.to_bits(),
                                layer,
                                item,
                            ],
                        );
                    };
                    match pts.len() {
                        0 => {}
                        // a point: a dot
                        1 => add(pts[0], pts[0]),
                        _ => {
                            for w in pts.windows(2) {
                                add(w[0], w[1]);
                            }
                            if *closed && pts.len() > 2 && pts[0] != pts[pts.len() - 1] {
                                add(pts[pts.len() - 1], pts[0]);
                            }
                        }
                    }
                }
                Prim::Segments { pts } => {
                    let mut it2 = pts.chunks_exact(2);
                    for ab in &mut it2 {
                        if !(ab.iter().all(|p| p[0].is_finite() && p[1].is_finite())) {
                            continue;
                        }
                        let (a, b) = (local(ab[0]), local(ab[1]));
                        grow(a);
                        grow(b);
                        put(
                            &mut lines,
                            &[
                                a[0].to_bits(),
                                a[1].to_bits(),
                                b[0].to_bits(),
                                b[1].to_bits(),
                                rgb,
                                it.weight.to_bits(),
                                layer,
                                item,
                            ],
                        );
                    }
                }
                Prim::Fill { rings } | Prim::Mask { rings } => {
                    // a mask is drawn in the canvas's colour: alpha 0 says so to the shader
                    let rgb = if matches!(it.prim, Prim::Mask { .. }) { rgb & 0x00FF_FFFF } else { rgb };
                    fill_items.push(item);
                    rings.iter().flatten().for_each(|&p| {
                        if p[0].is_finite() && p[1].is_finite() {
                            grow(local(p));
                        }
                    });
                    // triangulated about the fill's own first point, for its digits
                    let at = rings.iter().flatten().next().copied().unwrap_or([0.0; 2]);
                    let shifted: Vec<Vec<[f64; 2]>> = rings
                        .iter()
                        .map(|r| r.iter().map(|p| [p[0] - at[0], p[1] - at[1]]).collect())
                        .collect();
                    for t in tri::even_odd(&shifted) {
                        for p in t {
                            let q = local([p[0] + at[0], p[1] + at[1]]);
                            put(&mut fills, &[q[0].to_bits(), q[1].to_bits(), rgb, layer, item]);
                        }
                    }
                }
            }
            boxes.push(bx);
        }
        item_segs.push((lines.len() / LINE_BYTES) as u32);
        let n_lines = (lines.len() / LINE_BYTES) as u32;
        let n_fill_verts = (fills.len() / FILL_BYTES) as u32;
        let (lo, hi) = boxes.iter().filter(|b| b[0] <= b[2]).fold(
            ([f32::INFINITY; 2], [f32::NEG_INFINITY; 2]),
            |(lo, hi), b| {
                (
                    [lo[0].min(b[0]), lo[1].min(b[1])],
                    [hi[0].max(b[2]), hi[1].max(b[3])],
                )
            },
        );
        let (lo, hi) = if lo[0] <= hi[0] {
            (lo, hi)
        } else {
            ([0.0; 2], [1.0; 2])
        };
        let grid = Grid::new(n_lines as usize, |i| seg_of(&lines, i).1, lo, hi);
        Geom {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            scene,
            origin,
            lines,
            n_lines,
            fills,
            n_fill_verts,
            grid,
            boxes,
            item_segs,
            fill_items,
            by_entity,
        }
    }

    pub fn n_items(&self) -> usize {
        self.boxes.len()
    }

    pub fn local(&self, p: [f64; 2]) -> [f32; 2] {
        [(p[0] - self.origin[0]) as f32, (p[1] - self.origin[1]) as f32]
    }

    /// The entity whose item is nearest the point (world units) within `tol`: a path's within
    /// reach first, else the fill on top that holds the point. `shown` says which layers count.
    pub fn pick(&self, p: [f64; 2], tol: f64, shown: &dyn Fn(usize) -> bool) -> Option<u32> {
        let q = self.local(p);
        let items = &self.scene.items;
        let ok = |i: u32| shown(items[i as usize].layer);
        let r = tol as f32;
        let mut best: Option<(u32, f32)> = None;
        self.grid.near(q, r, |s| {
            let (item, (a, b)) = seg_of(&self.lines, s as usize);
            let d = seg_dist(q, a, b);
            // the later item is drawn on top: it wins a tie
            if d <= r && best.is_none_or(|(i, e)| d < e || (d == e && item > i)) && ok(item) {
                best = Some((item, d));
            }
        });
        if let Some((i, _)) = best {
            return Some(items[i as usize].entity);
        }
        for &i in self.fill_items.iter().rev() {
            let b = self.boxes[i as usize];
            if q[0] < b[0] || q[0] > b[2] || q[1] < b[1] || q[1] > b[3] || !ok(i) {
                continue;
            }
            if let Prim::Fill { rings } = &items[i as usize].prim
                && inside(rings, p)
            {
                return Some(items[i as usize].entity);
            }
        }
        None
    }

    /// AutoCAD's window (wholly inside the box) or crossing (inside or touching it) selection,
    /// between two corners in world units: the entities, each once.
    pub fn select_box(
        &self,
        a: [f64; 2],
        b: [f64; 2],
        crossing: bool,
        shown: &dyn Fn(usize) -> bool,
    ) -> Vec<u32> {
        let (qa, qb) = (self.local(a), self.local(b));
        let r = [
            qa[0].min(qb[0]),
            qa[1].min(qb[1]),
            qa[0].max(qb[0]),
            qa[1].max(qb[1]),
        ];
        let items = &self.scene.items;
        let mut out = vec![];
        for (&e, its) in &self.by_entity {
            let mut any_shown = false;
            let mut all_in = true;
            let mut touches = false;
            for &i in its {
                if !shown(items[i as usize].layer) {
                    continue;
                }
                any_shown = true;
                let bx = self.boxes[i as usize];
                if bx[0] > bx[2] {
                    continue;
                }
                let within = bx[0] >= r[0] && bx[1] >= r[1] && bx[2] <= r[2] && bx[3] <= r[3];
                all_in &= within;
                if crossing && !touches {
                    touches = within || self.item_touches(i, r);
                }
            }
            if any_shown && if crossing { touches } else { all_in } {
                out.push(e);
            }
        }
        out
    }

    /// Does the item meet the box r (lo x, lo y, hi x, hi y)?
    fn item_touches(&self, i: u32, r: [f32; 4]) -> bool {
        let bx = self.boxes[i as usize];
        if bx[0] > r[2] || bx[2] < r[0] || bx[1] > r[3] || bx[3] < r[1] {
            return false;
        }
        let (s, e) = (
            self.item_segs[i as usize] as usize,
            self.item_segs[i as usize + 1] as usize,
        );
        if (s..e).any(|k| {
            let (a, b) = seg_of(&self.lines, k).1;
            seg_meets_box(a, b, r)
        }) {
            return true;
        }
        if let Prim::Fill { rings } = &self.scene.items[i as usize].prim {
            // a corner of the box in the fill, or an edge of the fill across the box
            let corner = [r[0] as f64 + self.origin[0], r[1] as f64 + self.origin[1]];
            if inside(rings, corner) {
                return true;
            }
            for ring in rings {
                for k in 0..ring.len() {
                    let (a, b) = (self.local(ring[k]), self.local(ring[(k + 1) % ring.len()]));
                    if seg_meets_box(a, b, r) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// The box round the shown items, world units.
    pub fn shown_extents(&self, shown: &dyn Fn(usize) -> bool) -> Option<([f64; 2], [f64; 2])> {
        let mut lo = [f32::INFINITY; 2];
        let mut hi = [f32::NEG_INFINITY; 2];
        for (it, b) in self.scene.items.iter().zip(&self.boxes) {
            if b[0] <= b[2] && shown(it.layer) {
                lo = [lo[0].min(b[0]), lo[1].min(b[1])];
                hi = [hi[0].max(b[2]), hi[1].max(b[3])];
            }
        }
        (lo[0] <= hi[0]).then(|| {
            (
                [lo[0] as f64 + self.origin[0], lo[1] as f64 + self.origin[1]],
                [hi[0] as f64 + self.origin[0], hi[1] as f64 + self.origin[1]],
            )
        })
    }
}

/// Segment `i` of the instance bytes: its item, and its ends.
fn seg_of(lines: &[u8], i: usize) -> (u32, ([f32; 2], [f32; 2])) {
    let w = &lines[i * LINE_BYTES..(i + 1) * LINE_BYTES];
    let f = |k: usize| f32::from_le_bytes([w[k * 4], w[k * 4 + 1], w[k * 4 + 2], w[k * 4 + 3]]);
    let item = u32::from_le_bytes([w[28], w[29], w[30], w[31]]);
    (item, ([f(0), f(1)], [f(2), f(3)]))
}

/// Even-odd: is p inside the rings?
pub fn inside(rings: &[Vec<[f64; 2]>], p: [f64; 2]) -> bool {
    let mut odd = false;
    for ring in rings {
        let n = ring.len();
        for k in 0..n {
            let (a, b) = (ring[k], ring[(k + 1) % n]);
            if (a[1] > p[1]) != (b[1] > p[1]) {
                let x = a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0]);
                if p[0] < x {
                    odd = !odd;
                }
            }
        }
    }
    odd
}

/// Does the segment ab meet the box r (lo x, lo y, hi x, hi y)? Liang and Barsky's clip.
pub fn seg_meets_box(a: [f32; 2], b: [f32; 2], r: [f32; 4]) -> bool {
    let d = [b[0] - a[0], b[1] - a[1]];
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for (p, q) in [
        (-d[0], a[0] - r[0]),
        (d[0], r[2] - a[0]),
        (-d[1], a[1] - r[1]),
        (d[1], r[3] - a[1]),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return false;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
            if t0 > t1 {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_dwg::scene::{Item, SceneLayer};

    fn path(entity: u32, layer: usize, pts: &[[f64; 2]]) -> Item {
        Item {
            entity,
            layer,
            rgb: [255, 255, 255],
            weight: 0.0,
            prim: Prim::Path {
                pts: pts.to_vec(),
                closed: false,
            },
        }
    }

    fn layer(name: &str) -> SceneLayer {
        SceneLayer {
            handle: 0x10,
            name: name.into(),
            rgb: [255; 3],
            on: true,
            frozen: false,
            locked: false,
            plot: true,
        }
    }

    fn scene() -> Scene {
        let mut s = Scene {
            layers: vec![layer("0"), layer("walls")],
            items: vec![
                path(0x20, 0, &[[1000.0, 1000.0], [1010.0, 1000.0]]),
                path(0x21, 1, &[[1000.0, 1005.0], [1010.0, 1005.0], [1010.0, 1020.0]]),
                // an entity of two items, as an INSERT's pieces
                path(0x22, 0, &[[1020.0, 1000.0], [1022.0, 1002.0]]),
                path(0x22, 0, &[[1024.0, 1000.0], [1026.0, 1002.0]]),
                Item {
                    entity: 0x23,
                    layer: 0,
                    rgb: [255, 0, 0],
                    weight: 0.5,
                    prim: Prim::Fill {
                        rings: vec![vec![
                            [1030.0, 1000.0],
                            [1040.0, 1000.0],
                            [1040.0, 1010.0],
                            [1030.0, 1010.0],
                        ]],
                    },
                },
            ],
            extents: None,
        };
        s.grow_extents();
        s
    }

    #[test]
    fn its_buffers() {
        let g = Geom::new(Arc::new(scene()));
        assert_eq!(g.n_lines, 1 + 2 + 1 + 1);
        assert_eq!(g.lines.len(), g.n_lines as usize * LINE_BYTES);
        // a square: two triangles
        assert_eq!(g.n_fill_verts, 6);
        assert_eq!(g.by_entity[&0x22], vec![2, 3]);
    }

    #[test]
    fn picks_the_nearest_and_the_fill_under() {
        let g = Geom::new(Arc::new(scene()));
        let all = |_: usize| true;
        assert_eq!(g.pick([1005.0, 1000.4], 1.0, &all), Some(0x20));
        assert_eq!(g.pick([1005.0, 1004.7], 1.0, &all), Some(0x21));
        assert_eq!(g.pick([1035.0, 1005.0], 1.0, &all), Some(0x23));
        assert_eq!(g.pick([1005.0, 1002.5], 1.0, &all), None);
        // the walls layer off: not there to pick
        assert_eq!(g.pick([1005.0, 1004.7], 1.0, &|l| l != 1), None);
    }

    #[test]
    fn window_and_crossing() {
        let g = Geom::new(Arc::new(scene()));
        let all = |_: usize| true;
        // round the first line and the first half of the INSERT
        let w = g.select_box([999.0, 999.0], [1023.0, 1003.0], false, &all);
        assert_eq!(w, vec![0x20]);
        let c = g.select_box([999.0, 999.0], [1023.0, 1003.0], true, &all);
        assert_eq!(c, vec![0x20, 0x22]);
        // crossing inside the fill, touching no edge of it
        assert_eq!(
            g.select_box([1033.0, 1003.0], [1034.0, 1004.0], true, &all),
            vec![0x23]
        );
        // the polyline's corner box crossed by its segment only
        assert_eq!(
            g.select_box([1009.0, 1010.0], [1011.0, 1011.0], true, &all),
            vec![0x21]
        );
    }

    /// A grid of rooms: 100k items, built and picked from fast enough for a frame's hover.
    #[test]
    fn a_hundred_thousand_items() {
        let mut items = vec![];
        for i in 0..250 {
            for j in 0..200 {
                let (x, y) = (i as f64 * 10.0, j as f64 * 10.0);
                let e = (i * 200 + j) as u32 * 2 + 0x100;
                items.push(Item {
                    entity: e,
                    layer: 0,
                    rgb: [255; 3],
                    weight: 0.25,
                    prim: Prim::Path {
                        pts: vec![[x, y], [x + 8.0, y], [x + 8.0, y + 8.0], [x, y + 8.0]],
                        closed: true,
                    },
                });
                items.push(Item {
                    entity: e + 1,
                    layer: 1,
                    rgb: [255, 0, 0],
                    weight: 0.0,
                    prim: Prim::Fill {
                        rings: vec![vec![
                            [x + 3.0, y + 3.0],
                            [x + 5.0, y + 3.0],
                            [x + 5.0, y + 5.0],
                            [x + 3.0, y + 5.0],
                        ]],
                    },
                });
            }
        }
        let mut s = Scene {
            layers: vec![layer("0"), layer("fills")],
            items,
            extents: None,
        };
        s.grow_extents();
        let t = std::time::Instant::now();
        let g = Geom::new(Arc::new(s));
        let built = t.elapsed();
        assert_eq!(g.n_items(), 100_000);
        assert_eq!(g.n_lines, 200_000);
        assert_eq!(g.n_fill_verts, 300_000);
        let t = std::time::Instant::now();
        let all = |_: usize| true;
        // room (37, 81): its left wall, its fill
        assert_eq!(
            g.pick([370.1, 814.0], 0.3, &all),
            Some((37 * 200 + 81) * 2 + 0x100)
        );
        assert_eq!(
            g.pick([374.0, 814.0], 0.3, &all),
            Some((37 * 200 + 81) * 2 + 0x101)
        );
        assert_eq!(g.pick([379.0, 819.0], 0.3, &all), None);
        let picked = t.elapsed();
        eprintln!("100k items: built in {built:?}, three picks in {picked:?}");
    }

    /// `FORGE_DWG_STRESS=FILE.dwg cargo test write_a_stress_drawing -- --ignored`: a plan of
    /// 150,000 entities on eight layers, to open in the app.
    #[test]
    #[ignore]
    fn write_a_stress_drawing() {
        let Ok(path) = std::env::var("FORGE_DWG_STRESS") else {
            return;
        };
        use forge_dwg::{Doc, Entity, Layer, Shape, Weight};
        let mut doc = Doc {
            layers: (0..8)
                .map(|k| Layer {
                    name: format!("A-{k}"),
                    colour: k as i16 + 1,
                    line_type: "Continuous".into(),
                    weight: Weight::Mm100([0, 13, 25, 35, 50, 70, 100, 200][k]),
                })
                .collect(),
            font: "txt".into(),
            limits: ([0.0, 0.0], [420.0, 297.0]),
            ..Default::default()
        };
        for i in 0..250 {
            for j in 0..200 {
                let (x, y) = (i as f64 * 1000.0, j as f64 * 800.0);
                let layer = format!("A-{}", (i + j) % 8);
                doc.entities.push(Entity {
                    layer: layer.clone(),
                    shape: Shape::Polyline {
                        points: vec![[x, y], [x + 900.0, y], [x + 900.0, y + 700.0], [x, y + 700.0]],
                        closed: true,
                    },
                });
                // a door's swing (the builder's chords are fine yet: a quarter circle is many)
                doc.entities.push(Entity {
                    layer: layer.clone(),
                    shape: Shape::Polyline {
                        points: vec![
                            [x + 100.0, y],
                            [x + 100.0, y + 250.0],
                            [x + 280.0, y + 180.0],
                            [x + 350.0, y],
                        ],
                        closed: false,
                    },
                });
                doc.entities.push(Entity {
                    layer,
                    shape: Shape::Line {
                        a: [x + 100.0, y + 100.0],
                        b: [x + 800.0, y + 600.0],
                    },
                });
            }
        }
        std::fs::write(&path, forge_dwg::write(&doc)).expect("written");
    }
}
