//! A plain list of placed parts, and what of it a frame draws: frustum culling and level of detail.

use crate::camera::View;

/// Rows of a rigid transform: row major, translation in the last column.
pub type Rows = [[f32; 4]; 3];
/// Bytes per placement's rows on the GPU.
pub const ROWS: usize = 48;

/// A placed part and its colour.
#[derive(Clone, Copy, Debug)]
pub struct Placement {
    pub part: u32,
    pub rows: Rows,
    pub tint: [f32; 4],
}

impl Placement {
    pub fn at(part: u32, pos: [f32; 3]) -> Placement {
        Placement {
            part,
            rows: [
                [1.0, 0.0, 0.0, pos[0]],
                [0.0, 1.0, 0.0, pos[1]],
                [0.0, 0.0, 1.0, pos[2]],
            ],
            tint: crate::palette::GREY,
        }
    }
}

/// One draw: placements `first..first + count` of the frame's list, all one part at one level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Draw {
    pub part: u32,
    pub lod: u32,
    pub first: u32,
    pub count: u32,
}

/// Each part's chords (finest first), its sphere, and every placement's sphere in columns: the
/// frame's one loop over the placements runs over these.
pub struct Scene {
    /// Where each is drawn: its pose, moved outward by exploding.
    pub placements: Vec<Placement>,
    pose: Vec<Rows>,
    /// How far exploding moves each, in full (see `set_explode`).
    outward: Vec<[f32; 3]>,
    explode: f32,
    /// Each placement's move from its pose besides: an assembly's exploded view.
    offset: Vec<[f32; 3]>,
    /// Each part's box in its own space, when known (an assembly's): what bounds measures by.
    boxes: Vec<crate::explode::Aabb>,
    /// Squashed to nothing (hidden): left out of every draw.
    gone: Vec<bool>,
    /// Drawn only as a ghost (`Overlay::face_ghosts`): left out of the draws and picks.
    ghost: Vec<bool>,
    /// Every placement's rows as the GPU holds them, and the bytes changed since the last upload.
    pub rows_bytes: Vec<u8>,
    dirty: Option<(usize, usize)>,
    centers: Vec<[f32; 3]>,
    chords: Vec<Vec<f32>>,
    x: Vec<f32>,
    y: Vec<f32>,
    z: Vec<f32>,
    r: Vec<f32>,
    /// Filled by `cull`: the visible placements' indices, grouped as `draws` says.
    pub visible: Vec<[u8; 4]>,
    pub draws: Vec<Draw>,
    /// Those a cutting plane crosses, whose insides show: indexed in `visible` too.
    pub cut_draws: Vec<Draw>,
    picked: Vec<(u32, u32)>,
    counts: Vec<u32>,
    /// Each placement's level as of the last `cull`; `HIDDEN` if culled.
    level: Vec<u8>,
}

const HIDDEN: u8 = u8::MAX;

/// A pose squashed to nothing: how a hidden placement is put out of sight.
fn squashed(rows: &Rows) -> bool {
    rows.iter().all(|r| r[0] == 0.0 && r[1] == 0.0 && r[2] == 0.0)
}

impl Scene {
    /// Whether a placement is left out of the draws (hidden).
    /// Every shown placement, not culled, each part at the level `lod` gives it for its chords:
    /// what a view that doesn't move with the eye (the floor's shadow, the sun's) draws.
    pub fn all_shown(&self, lod: impl Fn(&[f32]) -> u32, list: &mut Vec<[u8; 4]>, draws: &mut Vec<Draw>) {
        list.clear();
        draws.clear();
        let mut by_part: Vec<(u32, u32)> = (0..self.placements.len())
            .filter(|&i| !self.gone[i] && !self.ghost[i])
            .map(|i| (self.placements[i].part, i as u32))
            .collect();
        by_part.sort_unstable();
        for group in by_part.chunk_by(|a, b| a.0 == b.0) {
            let part = group[0].0;
            draws.push(Draw {
                part,
                lod: lod(&self.chords[part as usize]),
                first: list.len() as u32,
                count: group.len() as u32,
            });
            list.extend(group.iter().map(|&(_, i)| i.to_le_bytes()));
        }
    }

    pub fn is_gone(&self, i: usize) -> bool {
        self.gone[i]
    }

    /// Shows a placement only as a ghost (its listed faces, see-through), or as itself again.
    pub fn set_ghost(&mut self, i: usize, on: bool) {
        if let Some(g) = self.ghost.get_mut(i) {
            *g = on;
        }
    }
}

/// A level per part at most (tessellated and clustered).
pub const LODS: usize = 8;
/// How far apart fully exploded is: each placement moves out by this many times its distance
/// from the scene's middle.
const EXPLODE: f32 = 1.5;

impl Scene {
    pub fn new(parts: &[([f32; 3], f32, Vec<f32>)], placements: Vec<Placement>) -> Scene {
        let (mut x, mut y, mut z, mut r) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for p in &placements {
            let (c, radius, _) = &parts[p.part as usize];
            let at = |row: [f32; 4]| row[0] * c[0] + row[1] * c[1] + row[2] * c[2] + row[3];
            x.push(at(p.rows[0]));
            y.push(at(p.rows[1]));
            z.push(at(p.rows[2]));
            r.push(*radius);
        }
        let n = placements.len();
        let mut rows_bytes = vec![0; n * ROWS];
        for (p, out) in placements.iter().zip(rows_bytes.as_chunks_mut::<ROWS>().0) {
            write_rows(&p.rows, out);
        }
        let count = x.len().max(1) as f32;
        let mid = [
            x.iter().sum::<f32>() / count,
            y.iter().sum::<f32>() / count,
            z.iter().sum::<f32>() / count,
        ];
        let outward = (0..n)
            .map(|i| {
                [
                    (x[i] - mid[0]) * EXPLODE,
                    (y[i] - mid[1]) * EXPLODE,
                    (z[i] - mid[2]) * EXPLODE,
                ]
            })
            .collect();
        Scene {
            pose: placements.iter().map(|p| p.rows).collect(),
            outward,
            explode: 0.0,
            offset: vec![[0.0; 3]; n],
            boxes: Vec::new(),
            gone: placements.iter().map(|p| squashed(&p.rows)).collect(),
            ghost: vec![false; placements.len()],
            placements,
            rows_bytes,
            dirty: None,
            centers: parts.iter().map(|p| p.0).collect(),
            chords: parts.iter().map(|p| p.2.clone()).collect(),
            x,
            y,
            z,
            r,
            visible: Vec::new(),
            draws: Vec::new(),
            cut_draws: Vec::new(),
            picked: Vec::new(),
            counts: vec![0; parts.len() * LODS],
            level: vec![HIDDEN; n],
        }
    }

    /// A placement's pose as it stands.
    pub fn rows_of(&self, i: usize) -> Rows {
        self.pose[i]
    }

    /// Moves a placement to a new pose: its rows, its sphere, and what the next frame uploads.
    pub fn set_rows(&mut self, i: usize, rows: &Rows) {
        self.pose[i] = *rows;
        self.place(i);
    }

    /// A part changed shape: its middle, its reach and its levels' chords, finest first.
    pub fn set_part(&mut self, part: u32, center: [f32; 3], radius: f32, chords: Vec<f32>) {
        self.centers[part as usize] = center;
        self.chords[part as usize] = chords;
        for i in 0..self.placements.len() {
            if self.placements[i].part == part {
                let rows = self.placements[i].rows;
                let at =
                    |row: [f32; 4]| row[0] * center[0] + row[1] * center[1] + row[2] * center[2] + row[3];
                self.x[i] = at(rows[0]);
                self.y[i] = at(rows[1]);
                self.z[i] = at(rows[2]);
                self.r[i] = radius;
            }
        }
    }

    /// Moves every placement out from the scene's middle: 0 home, 1 fully apart.
    pub fn set_explode(&mut self, t: f32) {
        self.explode = t;
        for i in 0..self.pose.len() {
            self.place(i);
        }
    }

    /// Moves each placement this far from its pose (an assembly's exploded view); none moved again.
    pub fn set_offsets(&mut self, offsets: &[[f32; 3]]) {
        for (i, o) in offsets.iter().enumerate().take(self.pose.len()) {
            if self.offset[i] != *o {
                self.offset[i] = *o;
                self.place(i);
            }
        }
    }

    pub fn offset_of(&self, i: usize) -> [f32; 3] {
        self.offset[i]
    }

    fn place(&mut self, i: usize) {
        let mut rows = self.pose[i];
        for (k, row) in rows.iter_mut().enumerate() {
            row[3] += self.outward[i][k] * self.explode + self.offset[i][k];
        }
        self.gone[i] = squashed(&rows);
        let p = &mut self.placements[i];
        p.rows = rows;
        let c = self.centers[p.part as usize];
        let at = |row: [f32; 4]| row[0] * c[0] + row[1] * c[1] + row[2] * c[2] + row[3];
        self.x[i] = at(rows[0]);
        self.y[i] = at(rows[1]);
        self.z[i] = at(rows[2]);
        let start = i * ROWS;
        write_rows(&rows, &mut self.rows_bytes[start..start + ROWS]);
        self.dirty = Some(match self.dirty {
            Some((lo, hi)) => (lo.min(start), hi.max(start + ROWS)),
            None => (start, start + ROWS),
        });
    }

    /// The span of `rows_bytes` to upload, once.
    pub fn take_dirty(&mut self) -> Option<(usize, usize)> {
        self.dirty.take()
    }

    /// The level a placement drew at in the last `cull`, if it wasn't culled.
    pub fn lod_on_screen(&self, i: u32) -> Option<u32> {
        let l = self.level[i as usize];
        (l != HIDDEN).then_some(l as u32)
    }

    /// A placement's sphere.
    pub fn sphere(&self, i: usize) -> ([f32; 3], f32) {
        ([self.x[i], self.y[i], self.z[i]], self.r[i])
    }

    /// Each part's box in its own space: bounds then hold what's shown, tightly.
    pub fn set_part_boxes(&mut self, boxes: Vec<crate::explode::Aabb>) {
        self.boxes = boxes;
    }

    /// The box of what's shown, by the parts' boxes (none without them, or with nothing shown).
    pub fn shown_box(&self) -> Option<crate::explode::Aabb> {
        if self.boxes.is_empty() {
            return None;
        }
        let mut all: Option<crate::explode::Aabb> = None;
        for p in &self.placements {
            // a hidden placement is squashed to nothing
            if squashed(&p.rows) {
                continue;
            }
            let local = self.boxes.get(p.part as usize)?;
            let b = crate::explode::world_box(&p.rows, local);
            all = Some(all.map_or(b, |a| {
                (
                    [0, 1, 2].map(|k| a.0[k].min(b.0[k])),
                    [0, 1, 2].map(|k| a.1[k].max(b.1[k])),
                )
            }));
        }
        all
    }

    /// A sphere holding the whole scene.
    pub fn bounds(&self) -> ([f32; 3], f32) {
        if let Some((lo, hi)) = self.shown_box() {
            let c = [0, 1, 2].map(|k| (lo[k] + hi[k]) * 0.5);
            let r = (0..3).map(|k| (hi[k] - lo[k]).powi(2)).sum::<f32>().sqrt() * 0.5;
            return (c, r.max(1e-3));
        }
        let n = self.x.len().max(1) as f32;
        let c = [
            self.x.iter().sum::<f32>() / n,
            self.y.iter().sum::<f32>() / n,
            self.z.iter().sum::<f32>() / n,
        ];
        let mut r = 0f32;
        for i in 0..self.x.len() {
            let (a, b, d) = (self.x[i] - c[0], self.y[i] - c[1], self.z[i] - c[2]);
            r = r.max((a * a + b * b + d * d).sqrt() + self.r[i]);
        }
        (c, r)
    }

    /// Works out the frame's draws: finest levels first, which draws near things first.
    /// A level is the coarsest whose chord stays under `max_px` pixels on screen.
    /// `second`: another cutting plane, and whether what either cuts goes (a quarter is left),
    /// else only what both cut (three quarters are left).
    pub fn cull(
        &mut self,
        v: &View,
        max_px: f32,
        section: Option<[f32; 4]>,
        second: Option<([f32; 4], bool)>,
    ) {
        let parts = self.chords.len();
        self.picked.clear();
        self.counts.fill(0);
        self.level.fill(HIDDEN);
        let [a, b, c, d, e] = v.planes;
        // inside the view's planes, and not wholly in what the cut takes away
        let away = |p: [f32; 4], x: f32, y: f32, z: f32, r: f32| p[0] * x + p[1] * y + p[2] * z + p[3] > r;
        let gone = |x: f32, y: f32, z: f32, r: f32| match (section, second) {
            (None, _) => false,
            (Some(a), None) => away(a, x, y, z, r),
            (Some(a), Some((b, false))) => away(a, x, y, z, r) && away(b, x, y, z, r),
            (Some(a), Some((b, true))) => away(a, x, y, z, r) || away(b, x, y, z, r),
        };
        for i in 0..self.x.len() {
            if self.gone[i] || self.ghost[i] {
                continue;
            }
            let (x, y, z, r) = (self.x[i], self.y[i], self.z[i], -self.r[i]);
            let side = |p: [f32; 4]| p[0] * x + p[1] * y + p[2] * z + p[3] < r;
            if side(a) | side(b) | side(c) | side(d) | side(e) | gone(x, y, z, -r) {
                continue;
            }
            let (dx, dy, dz) = (x - v.eye[0], y - v.eye[1], z - v.eye[2]);
            let closest = ((dx * dx + dy * dy + dz * dz).sqrt() + r).max(v.near);
            let allowed = max_px * closest / v.px_mm;
            let part = self.placements[i].part as usize;
            let chords = &self.chords[part];
            let lod = chords.partition_point(|&c| c <= allowed).saturating_sub(1);
            self.level[i] = lod as u8;
            let bin = lod * parts + part;
            self.counts[bin] += 1;
            self.picked.push((bin as u32, i as u32));
        }
        self.draws.clear();
        let mut at = 0;
        for (bin, count) in self.counts.iter_mut().enumerate() {
            if *count > 0 {
                let n = *count;
                self.draws.push(Draw {
                    part: (bin % parts) as u32,
                    lod: (bin / parts) as u32,
                    first: at,
                    count: n,
                });
                *count = at;
                at += n;
            }
        }
        self.visible.resize(at as usize, [0; 4]);
        for &(bin, i) in &self.picked {
            let slot = &mut self.counts[bin as usize];
            self.visible[*slot as usize] = i.to_le_bytes();
            *slot += 1;
        }
        // those the plane crosses again after them, by bin, for their insides
        self.cut_draws.clear();
        if let Some(s) = section {
            let crossing = |&&(_, i): &&(u32, u32)| {
                let i = i as usize;
                let d = |s: [f32; 4]| s[0] * self.x[i] + s[1] * self.y[i] + s[2] * self.z[i] + s[3];
                d(s).abs() < self.r[i] || second.is_some_and(|b| d(b.0).abs() < self.r[i])
            };
            let mut cut: Vec<(u32, u32)> = self.picked.iter().filter(crossing).copied().collect();
            cut.sort_unstable();
            for group in cut.chunk_by(|a, b| a.0 == b.0) {
                let bin = group[0].0 as usize;
                self.cut_draws.push(Draw {
                    part: (bin % parts) as u32,
                    lod: (bin / parts) as u32,
                    first: self.visible.len() as u32,
                    count: group.len() as u32,
                });
                self.visible.extend(group.iter().map(|&(_, i)| i.to_le_bytes()));
            }
        }
    }
}

fn write_rows(rows: &Rows, out: &mut [u8]) {
    for (v, o) in rows.as_flattened().iter().zip(out.as_chunks_mut::<4>().0) {
        *o = v.to_le_bytes();
    }
}

/// What joints will do: placements swinging about a hinge of their own, each at its own pace. A
/// hinge is off the part's centre, so its sphere moves too.
pub struct Motion {
    joints: Vec<Joint>,
}

struct Joint {
    index: u32,
    base: Rows,
    hinge: [f32; 3],
    pace: f32,
    phase: f32,
}

impl Motion {
    /// `n` joints spread evenly over the scene's placements.
    pub fn new(scene: &Scene, n: usize) -> Motion {
        let total = scene.placements.len();
        let n = n.min(total);
        let joints = (0..n)
            .map(|k| {
                let index = k * total / n;
                let p = &scene.placements[index];
                let c = scene.centers[p.part as usize];
                let r = scene.r[index];
                Joint {
                    index: index as u32,
                    base: scene.pose[index],
                    hinge: [c[0] + r * 0.5, c[1], c[2]],
                    pace: 0.02 + 0.03 * (k % 17) as f32 / 17.0,
                    phase: k as f32 * 0.37,
                }
            })
            .collect();
        Motion { joints }
    }

    pub fn len(&self) -> usize {
        self.joints.len()
    }

    pub fn is_empty(&self) -> bool {
        self.joints.is_empty()
    }

    /// Every joint at this frame: its part turned about its hinge's local z.
    pub fn apply(&self, frame: usize, scene: &mut Scene) {
        for j in &self.joints {
            let (s, c) = ((frame as f32 * j.pace + j.phase).sin() * 0.6).sin_cos();
            let h = j.hinge;
            // the local point that lands where the hinge did: h - R h, with R about z
            let off = [h[0] - (c * h[0] - s * h[1]), h[1] - (s * h[0] + c * h[1]), 0.0];
            let row = |r: [f32; 4]| {
                [
                    r[0] * c + r[1] * s,
                    r[1] * c - r[0] * s,
                    r[2],
                    r[3] + r[0] * off[0] + r[1] * off[1],
                ]
            };
            let rows = [row(j.base[0]), row(j.base[1]), row(j.base[2])];
            scene.set_rows(j.index as usize, &rows);
        }
    }
}

/// The stress scene over these parts: a spacing the average part about fills, so neighbours
/// overlap little and hide much.
pub fn stress_of(parts: &[crate::mesh::PartMesh], n: usize) -> Vec<Placement> {
    let r = parts.iter().map(|p| p.radius).sum::<f32>() / parts.len() as f32;
    stress(parts.len(), n, r * 2.5)
}

/// `n` placements of `parts` kinds in a cube, `spacing` apart on a jittered grid, turned at
/// random (fixed seed, so every run sees the same scene).
pub fn stress(parts: usize, n: usize, spacing: f32) -> Vec<Placement> {
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 40) as f32 / (1u64 << 24) as f32
    };
    let side = (n as f32).cbrt().ceil() as usize;
    let half = side as f32 * spacing * 0.5;
    (0..n)
        .map(|i| {
            let (ix, iy, iz) = (i % side, i / side % side, i / side / side);
            let jitter = |g: usize, r: f32| g as f32 * spacing - half + (r - 0.5) * spacing * 0.6;
            let pos = [jitter(ix, next()), jitter(iy, next()), jitter(iz, next())];
            // a uniform random rotation (Shoemake)
            let (u1, u2, u3) = (
                next(),
                next() * core::f32::consts::TAU,
                next() * core::f32::consts::TAU,
            );
            let (a, b) = ((1.0 - u1).sqrt(), u1.sqrt());
            let (w, x, y, z) = (a * u2.sin(), a * u2.cos(), b * u3.sin(), b * u3.cos());
            let part = (next() * parts as f32) as u32 % parts as u32;
            let hue = part as f32 / parts as f32 * core::f32::consts::TAU;
            let tint = [
                0.62 + 0.10 * hue.cos(),
                0.64 + 0.04 * hue.sin(),
                0.68 - 0.10 * hue.cos(),
                // every third kind of part bare metal, the rest painted
                if part.is_multiple_of(3) { 1.0 } else { 0.0 },
            ];
            Placement {
                part,
                rows: [
                    [
                        1.0 - 2.0 * (y * y + z * z),
                        2.0 * (x * y - w * z),
                        2.0 * (x * z + w * y),
                        pos[0],
                    ],
                    [
                        2.0 * (x * y + w * z),
                        1.0 - 2.0 * (x * x + z * z),
                        2.0 * (y * z - w * x),
                        pos[1],
                    ],
                    [
                        2.0 * (x * z - w * y),
                        2.0 * (y * z + w * x),
                        1.0 - 2.0 * (x * x + y * y),
                        pos[2],
                    ],
                ],
                tint,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Camera;

    fn scene(places: Vec<Placement>) -> Scene {
        Scene::new(&[([0.0; 3], 1.0, vec![0.01, 0.1, 1.0])], places)
    }

    #[test]
    fn a_part_behind_the_camera_is_culled_and_one_ahead_kept() {
        let cam = Camera::fit([0.0; 3], 1.0);
        let v = cam.view(800, 600);
        let behind = [0, 1, 2].map(|k| v.eye[k] * 3.0);
        let mut s = scene(vec![Placement::at(0, behind), Placement::at(0, [0.0; 3])]);
        s.cull(&v, 1.0, None, None);
        assert_eq!(s.visible, vec![1u32.to_le_bytes()]);
        assert_eq!(s.draws.len(), 1);
    }

    #[test]
    fn farther_parts_take_coarser_levels() {
        let cam = Camera::fit([0.0; 3], 1.0);
        let v = cam.view(800, 600);
        let dir = [0, 1, 2].map(|k| -v.eye[k]);
        let at = |t: f32| Placement::at(0, dir.map(|d| d * t));
        let mut s = scene(vec![at(1.0), at(1000.0)]);
        s.cull(&v, 1.0, None, None);
        assert_eq!(s.draws.len(), 2);
        assert!(s.draws[0].lod < s.draws[1].lod);
    }

    #[test]
    fn a_joint_keeps_its_hinge_still_and_its_part_rigid_and_marks_what_changed() {
        let mut s = Scene::new(
            &[([1.0, 2.0, 3.0], 4.0, vec![1.0])],
            vec![Placement::at(0, [0.0; 3]), Placement::at(0, [100.0, 0.0, 0.0])],
        );
        assert_eq!(s.take_dirty(), None);
        let m = Motion::new(&s, 1);
        m.apply(20, &mut s);
        assert_eq!(s.take_dirty(), Some((0, ROWS)));
        let p = s.placements[0].rows;
        let at = |q: [f32; 3], r: [f32; 4]| r[0] * q[0] + r[1] * q[1] + r[2] * q[2] + r[3];
        let hinge = [3.0, 2.0, 3.0];
        for k in 0..3 {
            assert!((at(hinge, p[k]) - hinge[k]).abs() < 1e-5);
            let unit: f32 = p[k][..3].iter().map(|v| v * v).sum();
            assert!((unit - 1.0).abs() < 1e-5);
        }
        // the sphere went with it
        assert!((s.x[0] - at([1.0, 2.0, 3.0], p[0])).abs() < 1e-6);
        assert_eq!(s.rows_bytes[..4], p[0][0].to_le_bytes());
        // the other one stayed
        assert_eq!(s.placements[1].rows[0][3], 100.0);
        assert_eq!(s.take_dirty(), None);
    }

    #[test]
    fn exploding_moves_each_out_from_the_middle_and_home_again() {
        let mut s = scene(vec![
            Placement::at(0, [-10.0, 0.0, 0.0]),
            Placement::at(0, [10.0, 0.0, 0.0]),
        ]);
        s.set_explode(1.0);
        assert_eq!(s.placements[0].rows[0][3], -25.0);
        assert_eq!(s.placements[1].rows[0][3], 25.0);
        assert_eq!(s.sphere(1).0[0], 25.0);
        // a pose set while apart keeps its place apart
        s.set_rows(1, &Placement::at(0, [20.0, 0.0, 0.0]).rows);
        assert_eq!(s.placements[1].rows[0][3], 35.0);
        s.set_explode(0.0);
        assert_eq!(s.placements[1].rows[0][3], 20.0);
    }

    #[test]
    fn a_cut_culls_what_is_all_on_its_far_side_and_lists_what_it_crosses() {
        let cam = Camera::fit([0.0; 3], 100.0);
        let v = cam.view(800, 600);
        let mut s = scene(vec![
            Placement::at(0, [-3.0, 0.0, 0.0]),
            Placement::at(0, [0.0, 0.0, 0.0]),
            Placement::at(0, [3.0, 0.0, 0.0]),
        ]);
        // cut away x > 0: radius 1, so 3 is gone, 0 crosses, -3 is whole
        s.cull(&v, 1.0, Some([1.0, 0.0, 0.0, 0.0]), None);
        let whole: u32 = s.draws.iter().map(|d| d.count).sum();
        let cut: u32 = s.cut_draws.iter().map(|d| d.count).sum();
        assert_eq!((whole, cut), (2, 1));
        assert_eq!(s.visible.len(), 3);
        let first = s.cut_draws[0].first as usize;
        assert_eq!(s.visible[first], 1u32.to_le_bytes());
    }

    #[test]
    fn two_planes_cut_a_quarter_or_three_quarters() {
        let cam = Camera::fit([0.0; 3], 100.0);
        let v = cam.view(800, 600);
        // one placement in each quarter of the xy plane, and one where the planes meet
        let at = [[3.0, 3.0], [-3.0, 3.0], [-3.0, -3.0], [3.0, -3.0], [0.0, 0.0]];
        let mut s = scene(at.iter().map(|p| Placement::at(0, [p[0], p[1], 0.0])).collect());
        let (x, y) = ([1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0]);
        let count = |s: &Scene| -> (u32, u32) {
            (
                s.draws.iter().map(|d| d.count).sum(),
                s.cut_draws.iter().map(|d| d.count).sum(),
            )
        };
        // what both cut goes (x > 0 and y > 0): three quarters and the middle are left
        s.cull(&v, 1.0, Some(x), Some((y, false)));
        assert_eq!(count(&s), (4, 1));
        // what either cuts goes: only x < 0, y < 0 and the middle are left
        s.cull(&v, 1.0, Some(x), Some((y, true)));
        assert_eq!(count(&s), (2, 1));
    }

    #[test]
    fn stress_rotations_are_rigid_and_the_scene_is_repeatable() {
        let a = stress(15, 100, 300.0);
        let b = stress(15, 100, 300.0);
        for (p, q) in a.iter().zip(&b) {
            assert_eq!(p.rows, q.rows);
            let dot = |i: usize, j: usize| (0..3).map(|k| p.rows[i][k] * p.rows[j][k]).sum::<f32>();
            assert!((dot(0, 0) - 1.0).abs() < 1e-4 && dot(0, 1).abs() < 1e-4 && dot(1, 2).abs() < 1e-4);
        }
    }
}
